//! Control-thread VST3 instance registry for the existing insert slots.
//!
//! Plug-ins are created and edited here. The audio callback receives only the
//! format-neutral `HostedProcessor` trait. Instances are retained by
//! (track, insert slot, class ID) so playback-plan rebuilds never recreate them.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

use opentrack_audio::HostedInsert;
use opentrack_core::project::{InsertEffectKind, Project, Track};
use opentrack_plugins::{HostedProcessor, PluginClassId};
use opentrack_vst3::{Vst3Effect, Vst3ParameterInfo, Vst3State};

struct Instance {
    class_id: String,
    effect: Arc<Vst3Effect>,
}

/// Resolves a file or bundle-folder selection to the loadable module file.
pub fn resolve_module_selection(selection: &Path) -> Result<PathBuf, String> {
    if selection.is_file() {
        return selection
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("vst3"))
            .then(|| selection.to_path_buf())
            .ok_or_else(|| "Select a .vst3 module file or bundle folder".to_owned());
    }
    if selection.is_dir() {
        if let Some(module) = opentrack_plugin_scan::resolve_vst3_bundle(selection)? {
            return Ok(module);
        }
        return Err(format!(
            "No native VST3 module found under {}",
            selection.display()
        ));
    }
    Err(format!("{} does not exist", selection.display()))
}

#[derive(Default)]
pub struct Runtime {
    sample_rate: u32,
    max_block: u32,
    instances: HashMap<(u64, u64), Instance>,
    failures: HashMap<(u64, u64), String>,
    failed_class: HashMap<(u64, u64), String>,
    slot_ids: HashMap<(u64, usize), u64>,
}

/// Owner-lifetime native window used as the parent of a VST3 editor view.
#[cfg(windows)]
pub struct EditorWindow {
    frame: isize,
    hwnd: isize,
}

#[cfg(target_os = "linux")]
pub struct EditorWindow {
    connection: x11rb::rust_connection::RustConnection,
    window: u32,
    plug_parent: u32,
    protocols_atom: u32,
    delete_atom: u32,
    xembed_info_atom: u32,
    xembed_atom: u32,
    embedded_children: HashMap<u32, u32>,
    alive: bool,
}

#[cfg(windows)]
const VST_EDITOR_ALWAYS_ON_TOP_COMMAND: usize = 0xF100;

// Keep the parent alive until IPlugView::removed has run on the UI thread.
#[cfg(windows)]
unsafe extern "system" fn editor_window_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CheckMenuItem, DefWindowProcW, GetMenuState, GetSystemMenu, HWND_BOTTOM, HWND_TOP,
        MF_BYCOMMAND, MF_CHECKED, MF_UNCHECKED, SW_HIDE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SetWindowPos, ShowWindow, WM_CLOSE, WM_SYSCOMMAND,
    };
    if message == WM_CLOSE {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
        return 0;
    }
    if message == WM_SYSCOMMAND && wparam & 0xFFF0 == VST_EDITOR_ALWAYS_ON_TOP_COMMAND {
        unsafe {
            let menu = GetSystemMenu(hwnd, 0);
            let was_checked =
                GetMenuState(menu, VST_EDITOR_ALWAYS_ON_TOP_COMMAND as u32, MF_BYCOMMAND)
                    & MF_CHECKED
                    != 0;
            let now_checked = !was_checked;
            let check = MF_BYCOMMAND
                | if now_checked {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                };
            CheckMenuItem(menu, VST_EDITOR_ALWAYS_ON_TOP_COMMAND as u32, check);
            // Stay in the ordinary application z-order band so dialogs and
            // other applications can still cover the editor.
            SetWindowPos(
                hwnd,
                if now_checked { HWND_TOP } else { HWND_BOTTOM },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        return 0;
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

// The plug-in owns the drawing inside this container. Unlike the system
// STATIC control, it must not paint a white background over the native view.
#[cfg(windows)]
unsafe extern "system" fn editor_surface_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::{
        Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
        UI::WindowsAndMessaging::{DefWindowProcW, WM_ERASEBKGND, WM_PAINT},
    };
    match message {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            unsafe {
                BeginPaint(hwnd, &mut paint);
                EndPaint(hwnd, &paint);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

#[cfg(windows)]
impl EditorWindow {
    pub fn create(
        title: &str,
        restore_bounds: Option<&opentrack_core::workspaces::WindowBounds>,
    ) -> Result<Self, String> {
        Self::create_with_visibility(title, restore_bounds, true)
    }

    fn create_hidden(
        title: &str,
        restore_bounds: Option<&opentrack_core::workspaces::WindowBounds>,
    ) -> Result<Self, String> {
        Self::create_with_visibility(title, restore_bounds, false)
    }

    fn create_with_visibility(
        title: &str,
        restore_bounds: Option<&opentrack_core::workspaces::WindowBounds>,
        visible: bool,
    ) -> Result<Self, String> {
        use windows_sys::Win32::{
            Foundation::{ERROR_CLASS_ALREADY_EXISTS, GetLastError},
            System::LibraryLoader::GetModuleHandleW,
            UI::WindowsAndMessaging::{
                AppendMenuW, CW_USEDEFAULT, CreateWindowExW, GetSystemMenu, MF_SEPARATOR,
                MF_STRING, RegisterClassW, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN,
                WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_SYSMENU, WS_VISIBLE,
            },
        };
        static REGISTERED: OnceLock<()> = OnceLock::new();
        let class: Vec<u16> = "OpenTrackVst3Editor\0".encode_utf16().collect();
        let surface_class: Vec<u16> = "OpenTrackVst3Surface\0".encode_utf16().collect();
        REGISTERED.get_or_init(|| unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(editor_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class.as_ptr(),
            };
            let registered = RegisterClassW(&class);
            if registered == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
                // The CreateWindow call below will report the concrete failure.
            }
            let surface = WNDCLASSW {
                lpfnWndProc: Some(editor_surface_proc),
                lpszClassName: surface_class.as_ptr(),
                ..class
            };
            let _ = RegisterClassW(&surface);
        });
        let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        let (x, y) = restore_bounds
            .filter(|bounds| bounds.left.is_finite() && bounds.top.is_finite())
            .map_or((CW_USEDEFAULT, CW_USEDEFAULT), |bounds| {
                (bounds.left.round() as i32, bounds.top.round() as i32)
            });
        let visibility = if visible { WS_VISIBLE } else { 0 };
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class.as_ptr(),
                title.as_ptr(),
                WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN | visibility,
                x,
                y,
                960,
                640,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err(format!(
                "could not create the VST3 editor window (error {})",
                unsafe { GetLastError() }
            ));
        }
        let menu = unsafe { GetSystemMenu(hwnd, 0) };
        let pin_label: Vec<u16> = "Always on Top\0".encode_utf16().collect();
        if !menu.is_null() {
            unsafe {
                AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                AppendMenuW(
                    menu,
                    MF_STRING,
                    VST_EDITOR_ALWAYS_ON_TOP_COMMAND,
                    pin_label.as_ptr(),
                );
            }
        }
        // VST3 hosts normally attach the view to a child of their frame, not
        // directly to the top-level window. Waves editors are sensitive to this.
        let child = unsafe {
            CreateWindowExW(
                0,
                surface_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS | visibility,
                0,
                0,
                900,
                600,
                hwnd,
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null_mut(),
            )
        };
        if child.is_null() {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            }
            return Err(format!(
                "could not create the VST3 editor child window (error {})",
                unsafe { GetLastError() }
            ));
        }
        Ok(Self {
            frame: hwnd as isize,
            hwnd: child as isize,
        })
    }

    pub fn show(&self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_SHOW, ShowWindow};
        unsafe {
            ShowWindow(self.hwnd as _, SW_SHOW);
            ShowWindow(self.frame as _, SW_SHOW);
        }
    }

    pub fn set_visible(&self, visible: bool) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow};
        unsafe {
            ShowWindow(
                self.frame as _,
                if visible { SW_SHOWNOACTIVATE } else { SW_HIDE },
            );
        }
    }

    pub fn hwnd(&self) -> isize {
        self.hwnd
    }

    pub fn focus(&self) {
        use windows_sys::Win32::UI::{
            Input::KeyboardAndMouse::SetFocus,
            WindowsAndMessaging::{
                GW_CHILD, GetWindow, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
                SetForegroundWindow, SetWindowPos,
            },
        };
        unsafe {
            SetWindowPos(
                self.frame as _,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            SetForegroundWindow(self.frame as _);
            let child = GetWindow(self.hwnd as _, GW_CHILD);
            if !child.is_null() {
                SetFocus(child);
            }
        }
    }

    pub fn set_owner(&self, owner: isize) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GWLP_HWNDPARENT, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowLongPtrW,
            SetWindowPos,
        };
        unsafe {
            SetWindowLongPtrW(self.frame as _, GWLP_HWNDPARENT, owner);
            SetWindowPos(
                self.frame as _,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    pub fn restore_position(&self, bounds: Option<&opentrack_core::workspaces::WindowBounds>) {
        let Some(bounds) =
            bounds.filter(|bounds| bounds.left.is_finite() && bounds.top.is_finite())
        else {
            return;
        };
        use windows_sys::Win32::{
            Foundation::{HWND, RECT},
            UI::WindowsAndMessaging::{GetWindowRect, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos},
        };
        let mut rect = RECT::default();
        unsafe {
            if GetWindowRect(self.frame as HWND, &mut rect) != 0 {
                let _ = SetWindowPos(
                    self.frame as HWND,
                    std::ptr::null_mut(),
                    bounds.left.round() as i32,
                    bounds.top.round() as i32,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
            }
        }
    }

    pub fn bounds(&self) -> Option<opentrack_core::workspaces::WindowBounds> {
        use windows_sys::Win32::{
            Foundation::{HWND, RECT},
            UI::WindowsAndMessaging::GetWindowRect,
        };
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(self.frame as HWND, &mut rect) } == 0 {
            return None;
        }
        Some(opentrack_core::workspaces::WindowBounds {
            left: f64::from(rect.left),
            top: f64::from(rect.top),
            width: f64::from(rect.right - rect.left),
            height: f64::from(rect.bottom - rect.top),
            inner_width: None,
            inner_height: None,
            monitor_id: None,
        })
    }

    /// Dispatch the dedicated editor thread's full queue, including
    /// Thread-level WM_TIMER callbacks are required by some animated views.
    /// moving independently of the application's redraw cadence.
    pub fn pump_messages(&self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
        };
        let mut message = MSG::default();
        loop {
            // This dedicated thread owns only the native plug-in editor and its
            // children. A null HWND includes thread-level WM_TIMER messages.
            let available =
                unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) };
            if available == 0 {
                break;
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(self.frame as _) != 0 }
    }
}

#[cfg(target_os = "linux")]
impl EditorWindow {
    fn create_hidden(
        title: &str,
        restore_bounds: Option<&opentrack_core::workspaces::WindowBounds>,
    ) -> Result<Self, String> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{
            AtomEnum, ConnectionExt, CreateWindowAux, EventMask, PropMode, WindowClass,
        };
        use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

        let (connection, screen_index) = x11rb::connect(None)
            .map_err(|error| format!("could not connect to X11 for VST3 editor: {error}"))?;
        let screen = &connection.setup().roots[screen_index];
        let window = connection
            .generate_id()
            .map_err(|error| format!("could not allocate VST3 editor XID: {error}"))?;
        let (x, y, width, height) = restore_bounds
            .filter(|bounds| bounds.left.is_finite() && bounds.top.is_finite())
            .map(|bounds| {
                (
                    bounds.left.round() as i16,
                    bounds.top.round() as i16,
                    bounds.width.round().clamp(240.0, 16_384.0) as u16,
                    bounds.height.round().clamp(160.0, 16_384.0) as u16,
                )
            })
            .unwrap_or((80, 80, 800, 600));
        connection
            .create_window(
                0,
                window,
                screen.root,
                x,
                y,
                width,
                height,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &CreateWindowAux::new()
                    .background_pixel(screen.black_pixel)
                    .event_mask(
                        EventMask::STRUCTURE_NOTIFY
                            | EventMask::EXPOSURE
                            | EventMask::SUBSTRUCTURE_NOTIFY,
                    ),
            )
            .map_err(|error| format!("could not create VST3 editor X11 window: {error}"))?;
        let protocols = connection
            .intern_atom(false, b"WM_PROTOCOLS")
            .map_err(|error| format!("could not intern WM_PROTOCOLS: {error}"))?
            .reply()
            .map_err(|error| format!("could not read WM_PROTOCOLS atom: {error}"))?
            .atom;
        let delete_atom = connection
            .intern_atom(false, b"WM_DELETE_WINDOW")
            .map_err(|error| format!("could not intern WM_DELETE_WINDOW: {error}"))?
            .reply()
            .map_err(|error| format!("could not read WM_DELETE_WINDOW atom: {error}"))?
            .atom;
        // XEmbed hosts advertise a two-CARDINAL _XEMBED_INFO structure on the
        // parent window. Flags start clear until the host maps the editor.
        let xembed_atom = connection
            .intern_atom(false, b"_XEMBED")
            .map_err(|error| format!("could not intern _XEMBED: {error}"))?
            .reply()
            .map_err(|error| format!("could not read _XEMBED atom: {error}"))?
            .atom;
        let xembed_info_atom = connection
            .intern_atom(false, b"_XEMBED_INFO")
            .map_err(|error| format!("could not intern _XEMBED_INFO: {error}"))?
            .reply()
            .map_err(|error| format!("could not read _XEMBED_INFO atom: {error}"))?
            .atom;
        let plug_parent = connection
            .generate_id()
            .map_err(|error| format!("could not allocate VST3 plug-in parent XID: {error}"))?;
        connection
            .create_window(
                0,
                plug_parent,
                window,
                0,
                0,
                width,
                height,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &CreateWindowAux::new()
                    .event_mask(EventMask::SUBSTRUCTURE_NOTIFY | EventMask::PROPERTY_CHANGE),
            )
            .map_err(|error| format!("could not create VST3 plug-in parent: {error}"))?;
        connection
            .change_property32(
                PropMode::REPLACE,
                window,
                protocols,
                AtomEnum::ATOM,
                &[delete_atom],
            )
            .map_err(|error| format!("could not set VST3 editor close protocol: {error}"))?;
        let utf8 = connection
            .intern_atom(false, b"UTF8_STRING")
            .map_err(|error| format!("could not intern UTF8_STRING: {error}"))?
            .reply()
            .map_err(|error| format!("could not read UTF8_STRING atom: {error}"))?
            .atom;
        let net_name = connection
            .intern_atom(false, b"_NET_WM_NAME")
            .map_err(|error| format!("could not intern _NET_WM_NAME: {error}"))?
            .reply()
            .map_err(|error| format!("could not read _NET_WM_NAME atom: {error}"))?
            .atom;
        connection
            .change_property8(PropMode::REPLACE, window, net_name, utf8, title.as_bytes())
            .map_err(|error| format!("could not set VST3 editor title: {error}"))?;
        connection
            .change_property8(
                PropMode::REPLACE,
                window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                title.as_bytes(),
            )
            .map_err(|error| format!("could not set VST3 editor legacy title: {error}"))?;
        connection
            .map_window(plug_parent)
            .map_err(|error| format!("could not map VST3 plug-in parent: {error}"))?;
        connection
            .flush()
            .map_err(|error| format!("could not flush VST3 editor X11 window: {error}"))?;
        Ok(Self {
            connection,
            window,
            plug_parent,
            protocols_atom: protocols,
            delete_atom,
            xembed_info_atom,
            xembed_atom,
            embedded_children: HashMap::new(),
            alive: true,
        })
    }

    fn show(&mut self) {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::ConnectionExt;
        let _ = self.connection.map_window(self.window);
        let _ = self.connection.flush();
    }

    fn set_visible(&self, visible: bool) {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::ConnectionExt;
        let _ = if visible {
            self.connection.map_window(self.window)
        } else {
            self.connection.unmap_window(self.window)
        };
        self.send_xembed_message(if visible { 1 } else { 2 });
        let _ = self.connection.flush();
    }

    fn hwnd(&self) -> isize {
        self.plug_parent as isize
    }

    fn focus(&self) {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{ConnectionExt, InputFocus};
        let focus_window = self
            .embedded_children
            .keys()
            .next()
            .copied()
            .unwrap_or(self.plug_parent);
        let _ =
            self.connection
                .set_input_focus(InputFocus::PARENT, focus_window, x11rb::CURRENT_TIME);
        self.send_xembed_message(4);
        let _ = self.connection.flush();
    }

    fn send_xembed_message(&self, opcode: u32) {
        use x11rb::protocol::xproto::{
            ClientMessageData, ClientMessageEvent, ConnectionExt, EventMask,
        };
        for (child, version) in &self.embedded_children {
            let event = ClientMessageEvent::new(
                32,
                *child,
                self.xembed_atom,
                ClientMessageData::from([
                    x11rb::CURRENT_TIME,
                    opcode,
                    0,
                    self.plug_parent,
                    *version,
                ]),
            );
            let _ = self
                .connection
                .send_event(false, *child, EventMask::NO_EVENT, event);
        }
    }

    fn set_owner(&self, owner: isize) {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
        use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;
        if owner == 0 {
            return;
        }
        if let Some(atom) = self
            .connection
            .intern_atom(false, b"WM_TRANSIENT_FOR")
            .ok()
            .and_then(|cookie| cookie.reply().ok())
        {
            let _ = self.connection.change_property32(
                PropMode::REPLACE,
                self.window,
                atom.atom,
                AtomEnum::WINDOW,
                &[owner as u32],
            );
            let _ = self.connection.flush();
        }
    }

    fn restore_position(&self, bounds: Option<&opentrack_core::workspaces::WindowBounds>) {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt};
        let Some(bounds) =
            bounds.filter(|bounds| bounds.left.is_finite() && bounds.top.is_finite())
        else {
            return;
        };
        let _ = self.connection.configure_window(
            self.window,
            &ConfigureWindowAux::new()
                .x(bounds.left.round() as i32)
                .y(bounds.top.round() as i32)
                .width(bounds.width.round().clamp(240.0, 16_384.0) as u32)
                .height(bounds.height.round().clamp(160.0, 16_384.0) as u32),
        );
        let _ = self.connection.configure_window(
            self.plug_parent,
            &ConfigureWindowAux::new()
                .x(0)
                .y(0)
                .width(bounds.width.round().clamp(240.0, 16_384.0) as u32)
                .height(bounds.height.round().clamp(160.0, 16_384.0) as u32),
        );
        let _ = self.connection.flush();
    }

    fn bounds(&self) -> Option<opentrack_core::workspaces::WindowBounds> {
        use x11rb::protocol::xproto::ConnectionExt;
        let geometry = self
            .connection
            .get_geometry(self.window)
            .ok()?
            .reply()
            .ok()?;
        Some(opentrack_core::workspaces::WindowBounds {
            left: f64::from(geometry.x),
            top: f64::from(geometry.y),
            width: f64::from(geometry.width),
            height: f64::from(geometry.height),
            inner_width: None,
            inner_height: None,
            monitor_id: None,
        })
    }

    fn pump_messages(&mut self) {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{ConnectionExt, EventMask};
        const XEMBED_EMBEDDED_NOTIFY: u32 = 0;
        const XEMBED_WINDOW_ACTIVATE: u32 = 1;
        const XEMBED_FOCUS_IN: u32 = 4;
        while let Ok(Some(event)) = self.connection.poll_for_event() {
            match event {
                x11rb::protocol::Event::CreateNotify(created)
                    if created.parent == self.plug_parent
                        && !self.embedded_children.contains_key(&created.window) =>
                {
                    let version = self
                        .connection
                        .get_property(
                            false,
                            created.window,
                            self.xembed_info_atom,
                            x11rb::protocol::xproto::AtomEnum::CARDINAL,
                            0,
                            2,
                        )
                        .ok()
                        .and_then(|cookie| cookie.reply().ok())
                        .and_then(|reply| reply.value32().and_then(|mut values| values.next()))
                        .unwrap_or(0);
                    self.embedded_children.insert(created.window, version);
                    for opcode in [
                        XEMBED_EMBEDDED_NOTIFY,
                        XEMBED_WINDOW_ACTIVATE,
                        XEMBED_FOCUS_IN,
                    ] {
                        let message = x11rb::protocol::xproto::ClientMessageEvent::new(
                            32,
                            created.window,
                            self.xembed_atom,
                            x11rb::protocol::xproto::ClientMessageData::from([
                                x11rb::CURRENT_TIME,
                                opcode,
                                0,
                                self.plug_parent,
                                version,
                            ]),
                        );
                        let _ = self.connection.send_event(
                            false,
                            created.window,
                            EventMask::NO_EVENT,
                            message,
                        );
                    }
                    let _ = self.connection.map_window(created.window);
                }
                x11rb::protocol::Event::ClientMessage(message) => {
                    if message.type_ == self.protocols_atom
                        && message.data.as_data32()[0] == self.delete_atom
                    {
                        self.alive = false;
                    } else if message.type_ == self.xembed_atom && message.data.as_data32()[1] == 3
                    {
                        self.focus();
                    }
                }
                x11rb::protocol::Event::DestroyNotify(destroyed)
                    if destroyed.window == self.window =>
                {
                    self.alive = false;
                }
                x11rb::protocol::Event::DestroyNotify(destroyed) => {
                    self.embedded_children.remove(&destroyed.window);
                }
                _ => {}
            }
        }
        let _ = self.connection.flush();
    }

    fn is_alive(&self) -> bool {
        self.alive
    }
}

#[cfg(target_os = "linux")]
impl Drop for EditorWindow {
    fn drop(&mut self) {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::ConnectionExt;
        let _ = self.connection.destroy_window(self.window);
        let _ = self.connection.flush();
    }
}

#[cfg(windows)]
impl Drop for EditorWindow {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(self.frame as _);
        }
    }
}

#[cfg(windows)]
enum EditorCommand {
    Focus,
    SetVisible(bool),
    SetOwner(isize),
    Close,
}

/// Native editor hosted on a dedicated STA thread.
///
/// The view is created from the live audio effect's controller, so meters and
/// processing displays remain connected to the actual processor. The editor
/// thread owns its own window message loop, keeping plug-in input and paint
/// independent from OpenTrack's main UI cadence.
#[cfg(windows)]
pub struct EditorHost {
    commands: std::sync::mpsc::Sender<EditorCommand>,
    thread: Option<std::thread::JoinHandle<()>>,
    alive: Arc<std::sync::atomic::AtomicBool>,
    bounds: Arc<Mutex<Option<opentrack_core::workspaces::WindowBounds>>>,
}

#[cfg(windows)]
impl EditorHost {
    pub fn open(
        title: &str,
        restore_bounds: Option<opentrack_core::workspaces::WindowBounds>,
        effect: Arc<Vst3Effect>,
    ) -> Result<Self, String> {
        use std::sync::atomic::AtomicBool;
        use std::sync::mpsc;
        use std::time::Duration;

        let title = title.to_owned();
        let alive = Arc::new(AtomicBool::new(false));
        let thread_alive = Arc::clone(&alive);
        let bounds = Arc::new(Mutex::new(None));
        let thread_bounds = Arc::clone(&bounds);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (command_tx, command_rx) = mpsc::channel();

        let thread = std::thread::Builder::new()
            .name("opentrack-vst3-editor".to_owned())
            .spawn(move || {
                let result = editor_thread(
                    title,
                    restore_bounds,
                    effect,
                    thread_alive,
                    thread_bounds,
                    command_rx,
                    ready_tx,
                );
                if let Err(error) = result {
                    eprintln!("VST3 editor thread: {error}");
                }
            })
            .map_err(|error| format!("could not start the VST3 editor thread: {error}"))?;

        match ready_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => Ok(Self {
                commands: command_tx,
                thread: Some(thread),
                alive,
                bounds,
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(error) => {
                let _ = command_tx.send(EditorCommand::Close);
                let _ = thread.join();
                Err(format!("the VST3 editor did not become ready: {error}"))
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(std::sync::atomic::Ordering::Acquire)
            && self
                .thread
                .as_ref()
                .is_some_and(|thread| !thread.is_finished())
    }

    pub fn focus(&self) {
        let _ = self.commands.send(EditorCommand::Focus);
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self.commands.send(EditorCommand::SetVisible(visible));
    }

    pub fn set_owner(&self, owner: isize) {
        if owner != 0 {
            let _ = self.commands.send(EditorCommand::SetOwner(owner));
        }
    }

    pub fn bounds(&self) -> Option<opentrack_core::workspaces::WindowBounds> {
        self.bounds.lock().ok().and_then(|bounds| bounds.clone())
    }

    pub fn close(mut self) {
        let _ = self.commands.send(EditorCommand::Close);
        if let Some(thread) = self.thread.take() {
            join_bounded(thread);
        }
        self.alive
            .store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Waits briefly for the editor thread. A plug-in stuck in its own modal loop
/// must not freeze the application, so a thread that does not exit is detached.
#[cfg(windows)]
fn join_bounded(thread: std::thread::JoinHandle<()>) {
    use std::time::Duration;
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("opentrack-vst3-editor-join".to_owned())
        .spawn(move || {
            let _ = thread.join();
            let _ = done_tx.send(());
        });
    if done_rx.recv_timeout(Duration::from_secs(2)).is_err() {
        crate::crash_log::session("editor thread did not exit within 2 s; detached");
    }
}

#[cfg(windows)]
impl Drop for EditorHost {
    fn drop(&mut self) {
        let _ = self.commands.send(EditorCommand::Close);
        if let Some(thread) = self.thread.take() {
            join_bounded(thread);
        }
        self.alive
            .store(false, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(windows)]
fn editor_thread(
    title: String,
    restore_bounds: Option<opentrack_core::workspaces::WindowBounds>,
    effect: Arc<Vst3Effect>,
    alive: Arc<std::sync::atomic::AtomicBool>,
    bounds: Arc<Mutex<Option<opentrack_core::workspaces::WindowBounds>>>,
    commands: std::sync::mpsc::Receiver<EditorCommand>,
    ready: std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let result = (|| -> Result<(), String> {
        let window = EditorWindow::create_hidden(&title, restore_bounds.as_ref())?;
        crate::crash_log::session(&format!("editor thread: opening \"{title}\""));
        let editor = effect.open_detached_editor(window.hwnd() as *mut std::ffi::c_void)?;
        crate::crash_log::session(&format!("editor thread: opened \"{title}\""));
        window.restore_position(restore_bounds.as_ref());
        window.show();
        if let Ok(mut current) = bounds.lock() {
            *current = window.bounds();
        }
        window.focus();
        alive.store(true, Ordering::Release);
        let _ = ready.send(Ok(()));

        let mut visible = true;
        let mut beats = 0_u32;
        loop {
            let mut close = false;
            while let Ok(command) = commands.try_recv() {
                let label = match &command {
                    EditorCommand::Focus => "focus",
                    EditorCommand::SetVisible(_) => "set-visible",
                    EditorCommand::SetOwner(_) => "set-owner",
                    EditorCommand::Close => "close",
                };
                crate::crash_log::session(&format!("editor thread: command {label}"));
                match command {
                    EditorCommand::Focus => {
                        if visible {
                            window.focus();
                        }
                    }
                    EditorCommand::SetVisible(value) => {
                        visible = value;
                        window.set_visible(value);
                    }
                    EditorCommand::SetOwner(owner) => window.set_owner(owner),
                    EditorCommand::Close => close = true,
                }
            }
            if close || !window.is_alive() {
                crate::crash_log::session("editor thread: loop ended");
                break;
            }
            window.pump_messages();
            // Processor-produced parameter changes must reach the controller on
            // the same thread that owns the editor view; calling this from the
            // main thread can deadlock inside a plug-in modal dialog.
            effect.sync_output_parameters();
            beats = beats.wrapping_add(1);
            if beats % 500 == 0 {
                crate::crash_log::session("editor thread: alive");
            }
            if let Ok(mut current) = bounds.lock() {
                *current = window.bounds();
            }
            std::thread::sleep(Duration::from_millis(4));
        }
        drop(editor);
        drop(window);
        Ok(())
    })();

    alive.store(false, Ordering::Release);
    if result.is_err() {
        let _ = ready.send(result.clone());
    }
    result
}

#[cfg(target_os = "linux")]
enum LinuxEditorCommand {
    Focus,
    SetVisible(bool),
    SetOwner(isize),
    Close,
}

#[cfg(target_os = "linux")]
pub struct EditorHost {
    commands: std::sync::mpsc::Sender<LinuxEditorCommand>,
    thread: Option<std::thread::JoinHandle<()>>,
    alive: Arc<std::sync::atomic::AtomicBool>,
    bounds: Arc<Mutex<Option<opentrack_core::workspaces::WindowBounds>>>,
}

#[cfg(target_os = "linux")]
fn linux_editor_unavailable_reason(display: Option<&std::ffi::OsStr>) -> Option<&'static str> {
    display
        .filter(|display| !display.is_empty())
        .is_none()
        .then_some(
            "Linux VST3 editors currently require X11 or XWayland; native Wayland-only embedding is unavailable",
        )
}

#[cfg(target_os = "linux")]
impl EditorHost {
    pub fn open(
        title: &str,
        restore_bounds: Option<opentrack_core::workspaces::WindowBounds>,
        effect: Arc<Vst3Effect>,
    ) -> Result<Self, String> {
        use std::sync::atomic::AtomicBool;
        use std::sync::mpsc;
        use std::time::Duration;

        if let Some(reason) =
            linux_editor_unavailable_reason(std::env::var_os("DISPLAY").as_deref())
        {
            return Err(String::from(reason));
        }

        let title = title.to_owned();
        let alive = Arc::new(AtomicBool::new(false));
        let thread_alive = Arc::clone(&alive);
        let bounds = Arc::new(Mutex::new(None));
        let thread_bounds = Arc::clone(&bounds);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (command_tx, command_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("opentrack-vst3-editor".to_owned())
            .spawn(move || {
                let result = linux_editor_thread(
                    title,
                    restore_bounds,
                    effect,
                    thread_alive,
                    thread_bounds,
                    command_rx,
                    ready_tx,
                );
                if let Err(error) = result {
                    eprintln!("VST3 editor thread: {error}");
                }
            })
            .map_err(|error| format!("could not start the VST3 editor thread: {error}"))?;
        match ready_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => Ok(Self {
                commands: command_tx,
                thread: Some(thread),
                alive,
                bounds,
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(error) => {
                let _ = command_tx.send(LinuxEditorCommand::Close);
                let _ = thread.join();
                Err(format!(
                    "the Linux VST3 editor did not become ready: {error}"
                ))
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(std::sync::atomic::Ordering::Acquire)
            && self
                .thread
                .as_ref()
                .is_some_and(|thread| !thread.is_finished())
    }
    pub fn focus(&self) {
        let _ = self.commands.send(LinuxEditorCommand::Focus);
    }
    pub fn set_visible(&self, visible: bool) {
        let _ = self.commands.send(LinuxEditorCommand::SetVisible(visible));
    }
    pub fn set_owner(&self, owner: isize) {
        let _ = self.commands.send(LinuxEditorCommand::SetOwner(owner));
    }
    pub fn bounds(&self) -> Option<opentrack_core::workspaces::WindowBounds> {
        self.bounds.lock().ok().and_then(|bounds| bounds.clone())
    }
    pub fn close(mut self) {
        let _ = self.commands.send(LinuxEditorCommand::Close);
        if let Some(thread) = self.thread.take() {
            join_bounded(thread);
        }
        self.alive
            .store(false, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(target_os = "linux")]
impl Drop for EditorHost {
    fn drop(&mut self) {
        let _ = self.commands.send(LinuxEditorCommand::Close);
        if let Some(thread) = self.thread.take() {
            join_bounded(thread);
        }
        self.alive
            .store(false, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(target_os = "linux")]
fn join_bounded(thread: std::thread::JoinHandle<()>) {
    use std::time::Duration;
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("opentrack-vst3-editor-join".into())
        .spawn(move || {
            let _ = thread.join();
            let _ = done_tx.send(());
        });
    if done_rx.recv_timeout(Duration::from_secs(2)).is_err() {
        crate::crash_log::session("Linux editor thread did not exit within 2 s; detached");
    }
}

#[cfg(target_os = "linux")]
fn linux_editor_thread(
    title: String,
    restore_bounds: Option<opentrack_core::workspaces::WindowBounds>,
    effect: Arc<Vst3Effect>,
    alive: Arc<std::sync::atomic::AtomicBool>,
    bounds: Arc<Mutex<Option<opentrack_core::workspaces::WindowBounds>>>,
    commands: std::sync::mpsc::Receiver<LinuxEditorCommand>,
    ready: std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    let result = (|| -> Result<(), String> {
        let mut window = EditorWindow::create_hidden(&title, restore_bounds.as_ref())?;
        let editor = effect.open_detached_editor(window.hwnd() as *mut std::ffi::c_void)?;
        // Attach may create the plug-in child before the outer window is
        // mapped. Process that X11 event first so initial focus targets the
        // child rather than the empty host container.
        window.pump_messages();
        window.restore_position(restore_bounds.as_ref());
        window.show();
        if let Ok(mut current) = bounds.lock() {
            *current = window.bounds();
        }
        window.focus();
        alive.store(true, Ordering::Release);
        let _ = ready.send(Ok(()));
        let mut visible = true;
        loop {
            let mut close = false;
            while let Ok(command) = commands.try_recv() {
                match command {
                    LinuxEditorCommand::Focus if visible => window.focus(),
                    LinuxEditorCommand::SetVisible(value) => {
                        visible = value;
                        window.set_visible(value);
                    }
                    LinuxEditorCommand::SetOwner(owner) => window.set_owner(owner),
                    LinuxEditorCommand::Close => close = true,
                    LinuxEditorCommand::Focus => {}
                }
            }
            if close || !window.is_alive() {
                break;
            }
            window.pump_messages();
            effect.pump_editor_run_loop();
            effect.sync_output_parameters();
            if let Ok(mut current) = bounds.lock() {
                *current = window.bounds();
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        drop(editor);
        drop(window);
        Ok(())
    })();
    alive.store(false, Ordering::Release);
    if let Err(error) = &result {
        let _ = ready.send(Err(error.clone()));
    }
    result
}

#[cfg(all(not(windows), not(target_os = "linux")))]
pub struct EditorHost;

#[cfg(all(not(windows), not(target_os = "linux")))]
impl EditorHost {
    pub fn open(
        _title: &str,
        _restore_bounds: Option<opentrack_core::workspaces::WindowBounds>,
        _effect: Arc<Vst3Effect>,
    ) -> Result<Self, String> {
        Err("native VST3 editors are only supported on Windows and Linux X11/XWayland".to_owned())
    }

    pub fn is_alive(&self) -> bool {
        false
    }

    pub fn focus(&self) {}

    pub fn set_visible(&self, _visible: bool) {}

    pub fn set_owner(&self, _owner: isize) {}

    pub fn bounds(&self) -> Option<opentrack_core::workspaces::WindowBounds> {
        None
    }

    pub fn close(self) {}
}

#[cfg(all(not(windows), not(target_os = "linux")))]
pub struct EditorWindow;

#[cfg(all(not(windows), not(target_os = "linux")))]
impl EditorWindow {
    pub fn create(_title: &str, _owner: Option<isize>) -> Result<Self, String> {
        Err("native VST3 editors are only supported on Windows and Linux X11/XWayland".to_owned())
    }

    pub fn hwnd(&self) -> isize {
        0
    }

    pub fn focus(&self) {}

    pub fn is_alive(&self) -> bool {
        false
    }
}

impl Runtime {
    pub fn matches_insert(
        &self,
        project: &Project,
        track_id: u64,
        slot: usize,
        rate: u32,
        block: u32,
    ) -> bool {
        self.sample_rate == rate && self.max_block == block
            && project.channel_by_id(track_id)
                .and_then(|track| track.inserts.get(slot))
                .is_some_and(|insert| matches!(&insert.kind, InsertEffectKind::Vst3 { class_id, .. }
                    if self.instances.get(&(track_id, insert.id)).is_some_and(|instance| instance.class_id == *class_id)))
    }

    pub fn take_dirty(&self) -> bool {
        self.instances.values().fold(false, |dirty, instance| {
            instance.effect.take_dirty() || dirty
        })
    }

    pub fn set_format(&mut self, sample_rate: u32, max_block: u32) -> bool {
        if self.sample_rate == sample_rate && self.max_block == max_block {
            return false;
        }
        self.sample_rate = sample_rate.max(1);
        self.max_block = max_block.max(1);
        // Recreate on the next sync with the new preparation.
        self.instances.clear();
        self.failures.clear();
        self.failed_class.clear();
        true
    }

    /// Reconciles instances with project inserts. Returns true when an instance
    /// was created or removed, which requires an audio-plan rebuild.
    pub fn sync(&mut self, project: &Project) -> bool {
        if self.sample_rate == 0 || self.max_block == 0 {
            return false;
        }
        let trace = std::env::var_os("OPENTRACK_VST3_TRACE").is_some();
        let sync_started = std::time::Instant::now();
        let mut wanted = HashSet::new();
        let mut changed = false;
        self.slot_ids.clear();
        for track in project
            .tracks
            .iter()
            .chain(std::iter::once(&project.output_channel.strip))
        {
            for (slot, insert) in track.inserts.iter().enumerate() {
                self.slot_ids.insert((track.id, slot), insert.id);
                let InsertEffectKind::Vst3 {
                    class_id,
                    module_path,
                    component_state,
                    controller_state,
                    ..
                } = &insert.kind
                else {
                    continue;
                };
                let key = (track.id, insert.id);
                // A powered-off plug-in is unloaded rather than left running.
                if !insert.enabled {
                    continue;
                }
                wanted.insert(key);
                if self
                    .failed_class
                    .get(&key)
                    .is_some_and(|failed| failed == class_id)
                {
                    continue;
                }
                let matches = self
                    .instances
                    .get(&key)
                    .is_some_and(|instance| instance.class_id == *class_id);
                if matches {
                    continue;
                }
                self.instances.remove(&key);
                self.failures.remove(&key);
                self.failed_class.remove(&key);
                let Ok(class_id) = PluginClassId::from_hex(class_id) else {
                    self.failures
                        .insert(key, "Saved VST3 class ID is invalid".to_owned());
                    changed = true;
                    continue;
                };
                let load_started = std::time::Instant::now();
                match Vst3Effect::load(module_path, class_id, self.sample_rate, self.max_block) {
                    Ok(effect) => {
                        if trace {
                            eprintln!(
                                "VST3 trace: {} instantiated in {} ms",
                                module_path.display(),
                                load_started.elapsed().as_millis()
                            );
                        }
                        if !component_state.is_empty() || !controller_state.is_empty() {
                            let state = Vst3State {
                                component: component_state.clone(),
                                controller: controller_state.clone(),
                            };
                            let state_started = std::time::Instant::now();
                            if let Err(error) = effect.load_state(&state) {
                                self.failures
                                    .insert(key, format!("VST3 state restore failed: {error}"));
                            }
                            if trace {
                                eprintln!(
                                    "VST3 trace: {} state restore in {} ms ({} bytes)",
                                    module_path.display(),
                                    state_started.elapsed().as_millis(),
                                    state.component.len() + state.controller.len()
                                );
                            }
                        }
                        self.instances.insert(
                            key,
                            Instance {
                                class_id: class_id.to_hex(),
                                effect: Arc::new(effect),
                            },
                        );
                    }
                    Err(error) => {
                        self.failures.insert(key, error);
                        self.failed_class.insert(key, class_id.to_hex());
                    }
                }
                changed = true;
            }
        }
        let stale = self
            .instances
            .keys()
            .copied()
            .filter(|key| !wanted.contains(key))
            .collect::<Vec<_>>();
        for key in stale {
            self.instances.remove(&key);
            self.failures.remove(&key);
            self.failed_class.remove(&key);
            changed = true;
        }
        if trace && changed {
            eprintln!(
                "VST3 trace: sync finished in {} ms",
                sync_started.elapsed().as_millis()
            );
        }
        changed
    }

    /// Load or restore one VST3 insert. Project loading uses this one-at-a-time
    /// entry point to keep the UI responsive between plug-in instances.
    pub fn load_insert(&mut self, track: &Track, slot: usize) -> bool {
        self.slot_ids.insert(
            (track.id, slot),
            track.inserts.get(slot).map_or(0, |insert| insert.id),
        );
        if self.sample_rate == 0 || self.max_block == 0 {
            return false;
        }
        let Some(insert) = track.inserts.get(slot) else {
            return false;
        };
        if !insert.enabled {
            return false;
        }
        let InsertEffectKind::Vst3 {
            class_id,
            module_path,
            component_state,
            controller_state,
            ..
        } = &insert.kind
        else {
            return false;
        };
        let key = (track.id, insert.id);
        if self
            .failed_class
            .get(&key)
            .is_some_and(|failed| failed == class_id)
            || self
                .instances
                .get(&key)
                .is_some_and(|instance| instance.class_id == *class_id)
        {
            return false;
        }
        self.instances.remove(&key);
        self.failures.remove(&key);
        self.failed_class.remove(&key);
        let Ok(parsed_class_id) = PluginClassId::from_hex(class_id) else {
            self.failures
                .insert(key, "Saved VST3 class ID is invalid".to_owned());
            return true;
        };
        match Vst3Effect::load(
            module_path,
            parsed_class_id,
            self.sample_rate,
            self.max_block,
        ) {
            Ok(effect) => {
                if std::env::var_os("OPENTRACK_VST3_TRACE").is_some() {
                    eprintln!(
                        "VST3 trace: {} instantiated while loading a project",
                        module_path.display()
                    );
                }
                if !component_state.is_empty() || !controller_state.is_empty() {
                    let state = Vst3State {
                        component: component_state.clone(),
                        controller: controller_state.clone(),
                    };
                    if let Err(error) = effect.load_state(&state) {
                        self.failures
                            .insert(key, format!("VST3 state restore failed: {error}"));
                    }
                }
                self.instances.insert(
                    key,
                    Instance {
                        class_id: parsed_class_id.to_hex(),
                        effect: Arc::new(effect),
                    },
                );
            }
            Err(error) => {
                self.failures.insert(key, error);
                self.failed_class.insert(key, parsed_class_id.to_hex());
            }
        }
        true
    }

    pub fn hosted_inserts(&self, track: &Track) -> Vec<HostedInsert> {
        track
            .inserts
            .iter()
            .enumerate()
            .filter_map(|(slot, insert)| {
                let InsertEffectKind::Vst3 { .. } = &insert.kind else {
                    return None;
                };
                let instance = self.instances.get(&(track.id, insert.id))?;
                let processor: Arc<dyn HostedProcessor> = instance.effect.clone();
                Some(HostedInsert {
                    processor,
                    enabled: insert.enabled && !insert.bypassed && !track.inserts_bypassed,
                    automation: track.automation_lanes.iter().filter(|lane| {
                        matches!(lane.target, opentrack_core::automation::AutomationTarget::PluginParameter { insert_id, .. } if insert_id == insert.id)
                            && lane.is_valid() && !lane.points.is_empty()
                    }).cloned().map(Arc::new).collect(),
                })
            })
            .collect()
    }

    pub fn instance(&self, track_id: u64, slot: usize) -> Option<Arc<Vst3Effect>> {
        self.instances
            .get(&(track_id, self.insert_id(track_id, slot)?))
            .map(|instance| Arc::clone(&instance.effect))
    }

    pub fn parameters(&self, track_id: u64, slot: usize) -> Option<Vec<Vst3ParameterInfo>> {
        self.instance(track_id, slot)
            .map(|effect| effect.parameters())
    }

    fn insert_id(&self, track_id: u64, slot: usize) -> Option<u64> {
        // Parameter/editor helpers are called with a project slot on the control
        // side; the stable-id map lookup is handled by retaining a slot map below.
        self.slot_ids.get(&(track_id, slot)).copied()
    }

    /// Abandons a queued editor retry; editor parameter sync now runs on the
    /// editor thread that owns the view.
    pub fn sync_editor_parameters(&self, _track_id: u64, _slot: usize) {}

    pub fn set_parameter(&self, track_id: u64, slot: usize, id: u32, value: f64) {
        if let Some(effect) = self.instance(track_id, slot) {
            effect.set_parameter(id, value);
        }
    }

    /// Every insert that failed to load, keyed by channel and slot.
    ///
    /// The rack shows these so a failed plug-in is visible where the user put
    /// it, not only in the parameter window they would have to open first.
    pub fn failures(&self) -> HashMap<(u64, usize), String> {
        self.failures
            .iter()
            .filter_map(|(&(track, insert_id), value)| {
                self.slot_ids.iter().find_map(|(&(owner, slot), &id)| {
                    (owner == track && id == insert_id).then_some(((track, slot), value.clone()))
                })
            })
            .collect()
    }

    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn failure(&self, track_id: u64, slot: usize) -> Option<&str> {
        self.failures
            .get(&(track_id, self.insert_id(track_id, slot)?))
            .map(String::as_str)
    }

    pub fn retry(&mut self, track_id: u64, slot: usize) {
        if let Some(id) = self.insert_id(track_id, slot) {
            self.failures.remove(&(track_id, id));
            self.failed_class.remove(&(track_id, id));
        }
    }

    pub fn open_editor(&self, track_id: u64, slot: usize, parent: isize) -> Result<(), String> {
        let effect = self
            .instance(track_id, slot)
            .ok_or_else(|| "VST3 instance is unavailable".to_owned())?;
        effect.open_editor(parent as *mut std::ffi::c_void)
    }

    pub fn close_editor(&self, track_id: u64, slot: usize) {
        if let Some(effect) = self.instance(track_id, slot) {
            effect.close_editor();
        }
    }

    pub fn has_editor(&self, track_id: u64, slot: usize) -> bool {
        self.instance(track_id, slot)
            .is_some_and(|effect| effect.has_editor())
    }

    /// Saves opaque component/controller chunks into the project before OPR
    /// serialization. No audio-thread work is involved.
    pub fn capture_states(&mut self, project: &mut Project) {
        for track in project
            .tracks
            .iter_mut()
            .chain(std::iter::once(&mut project.output_channel.strip))
        {
            for (slot, insert) in track.inserts.iter_mut().enumerate() {
                let InsertEffectKind::Vst3 {
                    class_id,
                    component_state,
                    controller_state,
                    ..
                } = &mut insert.kind
                else {
                    continue;
                };
                let Some(instance) = self.instances.get(&(track.id, insert.id)) else {
                    continue;
                };
                if instance.class_id != *class_id {
                    continue;
                }
                // Never query state while another thread owns this editor; the
                // live state is captured when the editor closes instead.
                if instance.effect.has_attached_editor() {
                    continue;
                }
                if let Ok(state) = instance.effect.save_state() {
                    *component_state = state.component;
                    *controller_state = state.controller;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::project::{InsertEffect, TrackKind};

    #[cfg(target_os = "linux")]
    #[test]
    fn native_editor_reports_wayland_only_limitation_without_x11_display() {
        assert_eq!(
            linux_editor_unavailable_reason(None),
            Some(
                "Linux VST3 editors currently require X11 or XWayland; native Wayland-only embedding is unavailable"
            )
        );
        assert_eq!(
            linux_editor_unavailable_reason(Some(std::ffi::OsStr::new(""))),
            Some(
                "Linux VST3 editors currently require X11 or XWayland; native Wayland-only embedding is unavailable"
            )
        );
        assert_eq!(
            linux_editor_unavailable_reason(Some(std::ffi::OsStr::new(":0"))),
            None
        );
    }

    /// Run one installed class per process so an ABI crash is attributable.
    #[test]
    #[ignore = "requires OPENTRACK_VST3_PROBE_MODULE and OPENTRACK_VST3_PROBE_CLASS"]
    fn installed_class_editor_lifecycle() {
        let module = PathBuf::from(std::env::var("OPENTRACK_VST3_PROBE_MODULE").expect("module"));
        let class =
            PluginClassId::from_hex(&std::env::var("OPENTRACK_VST3_PROBE_CLASS").expect("class"))
                .unwrap();
        eprintln!("stage: load first");
        let first = Vst3Effect::load(&module, class, 48_000, 64).expect("first instance");
        eprintln!("stage: open first editor");
        let mut window = probe_editor_window("OpenTrack compatibility probe");
        first
            .open_editor(window.hwnd() as *mut std::ffi::c_void)
            .expect("native editor");
        pump_editor_messages(&mut window, &first);
        #[cfg(target_os = "linux")]
        {
            use x11rb::protocol::xproto::ConnectionExt as _;

            let embedded_child = window
                .embedded_children
                .keys()
                .next()
                .copied()
                .expect("VST3 editor creates an XEmbed child");
            window.focus();
            let keyboard_focus = window
                .connection
                .get_input_focus()
                .expect("query X11 keyboard focus")
                .reply()
                .expect("read X11 keyboard focus")
                .focus;
            assert_eq!(keyboard_focus, embedded_child);
            eprintln!("stage: X11 keyboard focus routed to plug-in child");
            eprintln!("stage: request editor resize");
            eprintln!("resize result: {:?}", first.resize_editor(900, 600));
            pump_editor_messages(&mut window, &first);
            let before = first.parameters();
            // Opening an editor may legitimately publish initial state;
            // clear it so this assertion specifically covers the edit.
            let _ = first.take_dirty();
            inject_test_pointer_drag(&window, (140, 138), (140, 100))
                .expect("send synthetic pointer drag to plug-in child");
            pump_editor_messages(&mut window, &first);
            let after = first.parameters();
            assert!(
                before.iter().any(|old| {
                    after.iter().any(|new| {
                        old.id == new.id && (old.normalized - new.normalized).abs() > 1e-6
                    })
                }),
                "plug-in did not report a parameter edit after the X11 pointer drag"
            );
            assert!(
                first.take_dirty(),
                "native editor parameter edits must mark hosted state dirty"
            );
            eprintln!("stage: X11 pointer drag changed plug-in parameter");
            eprintln!("stage: native editor parameter edit marked hosted state dirty");
        }
        if let Ok(seconds) = std::env::var("OPENTRACK_VST3_PROBE_VIEW_SECONDS") {
            let started = std::time::Instant::now();
            while started.elapsed().as_secs() < seconds.parse::<u64>().unwrap_or(0).min(120) {
                pump_editor_messages(&mut window, &first);
            }
        }
        eprintln!("stage: load overlapping instance");
        let second = Vst3Effect::load(&module, class, 48_000, 64).expect("second instance");
        eprintln!("stage: process");
        use opentrack_plugins::HostedProcessor;
        let mut left = [0.01; 64];
        let mut right = [0.01; 64];
        first.process(&mut left, &mut right).unwrap();
        second.process(&mut left, &mut right).unwrap();
        assert!(left.iter().chain(&right).all(|v| v.is_finite()));
        eprintln!("stage: drop overlapping instance");
        drop(second);
        pump_editor_messages(&mut window, &first);
        eprintln!("stage: save/restore");
        let state = first.save_state().unwrap();
        first.load_state(&state).unwrap();
        first.close_editor();
        drop(window);
        eprintln!("stage: reopen editor");
        let mut window = probe_editor_window("OpenTrack compatibility reopen");
        first
            .open_editor(window.hwnd() as *mut std::ffi::c_void)
            .unwrap();
        pump_editor_messages(&mut window, &first);
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetParent, IsWindow, SendMessageW, WM_CLOSE,
            };
            let parent = GetParent(window.hwnd() as _);
            SendMessageW(parent, WM_CLOSE, 0, 0);
            assert!(!window.is_alive(), "close is observed by the app");
            assert!(
                IsWindow(window.hwnd() as _) != 0,
                "plugin parent survives until removed"
            );
        }
        first.close_editor();
        drop(window);
        drop(first);
        eprintln!("stage: complete");
    }

    fn probe_editor_window(title: &str) -> EditorWindow {
        #[cfg(windows)]
        {
            EditorWindow::create(title, None).expect("editor host")
        }
        #[cfg(target_os = "linux")]
        {
            let mut window = EditorWindow::create_hidden(title, None).expect("editor host");
            window.show();
            window
        }
    }

    #[cfg(target_os = "linux")]
    fn inject_test_pointer_drag(
        window: &EditorWindow,
        start: (i16, i16),
        end: (i16, i16),
    ) -> Result<(), String> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{
            ButtonPressEvent, ButtonReleaseEvent, ConnectionExt, EventMask, KeyButMask, Motion,
            MotionNotifyEvent,
        };
        use x11rb::x11_utils::Serialize;

        let child = window
            .embedded_children
            .keys()
            .next()
            .copied()
            .ok_or_else(|| String::from("plug-in has not created an XEmbed child window"))?;
        let attributes = window
            .connection
            .get_window_attributes(child)
            .map_err(|error| error.to_string())?
            .reply()
            .map_err(|error| error.to_string())?;
        eprintln!(
            "plug-in X11 child event mask: {:?}",
            attributes.all_event_masks
        );
        let root = window
            .connection
            .setup()
            .roots
            .first()
            .ok_or_else(|| String::from("X11 screen has no root window"))?
            .root;
        let root_position = |point: (i16, i16)| {
            window
                .connection
                .translate_coordinates(child, root, point.0, point.1)
                .map_err(|error| error.to_string())?
                .reply()
                .map(|reply| (reply.dst_x, reply.dst_y))
                .map_err(|error| error.to_string())
        };
        let start_root = root_position(start)?;
        let end_root = root_position(end)?;
        let press = ButtonPressEvent {
            response_type: x11rb::protocol::xproto::BUTTON_PRESS_EVENT,
            detail: 1,
            sequence: 0,
            time: x11rb::CURRENT_TIME,
            root,
            event: child,
            child: 0,
            root_x: start_root.0,
            root_y: start_root.1,
            event_x: start.0,
            event_y: start.1,
            state: KeyButMask::default(),
            same_screen: true,
        };
        let motion = MotionNotifyEvent {
            response_type: x11rb::protocol::xproto::MOTION_NOTIFY_EVENT,
            detail: Motion::NORMAL,
            sequence: 0,
            time: x11rb::CURRENT_TIME,
            root,
            event: child,
            child: 0,
            root_x: end_root.0,
            root_y: end_root.1,
            event_x: end.0,
            event_y: end.1,
            state: KeyButMask::BUTTON1,
            same_screen: true,
        };
        let release = ButtonReleaseEvent {
            response_type: x11rb::protocol::xproto::BUTTON_RELEASE_EVENT,
            state: KeyButMask::BUTTON1,
            ..press
        };
        window
            .connection
            .send_event(false, child, EventMask::BUTTON_PRESS, press.serialize())
            .map_err(|error| error.to_string())?;
        window
            .connection
            .send_event(false, child, EventMask::POINTER_MOTION, motion.serialize())
            .map_err(|error| error.to_string())?;
        window
            .connection
            .send_event(false, child, EventMask::BUTTON_RELEASE, release.serialize())
            .map_err(|error| error.to_string())?;
        window.connection.flush().map_err(|error| error.to_string())
    }

    fn pump_editor_messages(window: &mut EditorWindow, effect: &Vst3Effect) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
            };
            let started = std::time::Instant::now();
            while started.elapsed() < std::time::Duration::from_millis(250) {
                let mut message = MSG::default();
                while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) }
                    != 0
                {
                    unsafe {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        #[cfg(target_os = "linux")]
        {
            let started = std::time::Instant::now();
            while started.elapsed() < std::time::Duration::from_millis(250) {
                window.pump_messages();
                effect.pump_editor_run_loop();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }

    #[test]
    fn module_selection_accepts_a_single_module_file() {
        let path = std::env::temp_dir().join("opentrack-selection.vst3");
        std::fs::write(&path, b"not loaded by this test").unwrap();
        assert_eq!(resolve_module_selection(&path).unwrap(), path);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    #[ignore]
    fn installed_waveshell_child_loads_as_an_insert() {
        let module = Path::new(r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3");
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].inserts.push(InsertEffect {
            id: 0,
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::Vst3 {
                class_id: "37545356433663536c612d3736207374".to_owned(),
                module_path: module.to_path_buf(),
                name: "CLA-76 Stereo".to_owned(),
                vendor: "Waves".to_owned(),
                component_state: Vec::new(),
                controller_state: Vec::new(),
            },
        });
        let mut runtime = Runtime::default();
        runtime.set_format(48_000, 64);
        runtime.sync(&project);
        assert_eq!(runtime.failure(project.tracks[0].id, 0), None);
        assert!(
            !runtime
                .parameters(project.tracks[0].id, 0)
                .unwrap()
                .is_empty()
        );
        let hosted = runtime.hosted_inserts(&project.tracks[0]);
        let mut left = vec![0.1; 64];
        let mut right = vec![0.1; 64];
        hosted[0]
            .processor
            .process(&mut left, &mut right)
            .expect("WaveShell child processes a block");
        assert!(left.iter().chain(&right).all(|sample| sample.is_finite()));
    }

    #[test]
    #[ignore = "requires installed Waves CLA-76"]
    fn installed_cla76_passes_audio_and_respects_track_insert_bypass() {
        let module = Path::new(r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3");
        for (name, class) in [
            ("mono", "375453564336634D6C612D3736206D6F"),
            ("stereo", "37545356433663536C612D3736207374"),
        ] {
            let mut project = Project::default();
            project.add_track(TrackKind::Audio);
            project.tracks[0].inserts.push(InsertEffect {
                id: 0,
                enabled: true,
                bypassed: false,
                kind: InsertEffectKind::Vst3 {
                    class_id: class.to_owned(),
                    module_path: module.to_path_buf(),
                    name: format!("CLA-76 {name}"),
                    vendor: "Waves".to_owned(),
                    component_state: Vec::new(),
                    controller_state: Vec::new(),
                },
            });
            let mut runtime = Runtime::default();
            runtime.set_format(48_000, 64);
            runtime.sync(&project);
            assert_eq!(runtime.failure(project.tracks[0].id, 0), None);
            let hosted = runtime.hosted_inserts(&project.tracks[0]);
            assert!(hosted[0].enabled);
            let mut sum = 0.0_f64;
            let mut count = 0;
            for block in 0..512 {
                let mut left = [0.0_f32; 64];
                for (frame, sample) in left.iter_mut().enumerate() {
                    *sample = (std::f32::consts::TAU * 440.0 * (block * 64 + frame) as f32
                        / 48_000.0)
                        .sin()
                        * 0.1;
                }
                let mut right = left;
                hosted[0].processor.process(&mut left, &mut right).unwrap();
                if block >= 128 {
                    for sample in left.iter().chain(&right) {
                        assert!(sample.is_finite());
                        sum += f64::from(*sample) * f64::from(*sample);
                        count += 1;
                    }
                }
            }
            let rms = (sum / count as f64).sqrt();
            eprintln!("CLA-76 {name}: processed output RMS {rms:.6}");
            assert!(rms > 0.001, "CLA-76 {name} returned silence");
            project.tracks[0].inserts_bypassed = true;
            assert!(!runtime.hosted_inserts(&project.tracks[0])[0].enabled);
        }
    }

    #[test]
    #[ignore]
    fn installed_gateway_loads_despite_optional_host_context_failure() {
        let module = Path::new(
            r"C:\Program Files\Common Files\VST3\Gateway.vst3\Contents\x86_64-win\Gateway.vst3",
        );
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].inserts.push(InsertEffect {
            id: 0,
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::Vst3 {
                class_id: "0de7aef2de004e4f41414d6137325247".to_owned(),
                module_path: module.to_path_buf(),
                name: "Gateway".to_owned(),
                vendor: String::new(),
                component_state: Vec::new(),
                controller_state: Vec::new(),
            },
        });
        let mut runtime = Runtime::default();
        runtime.set_format(48_000, 64);
        runtime.sync(&project);
        assert_eq!(runtime.failure(project.tracks[0].id, 0), None);
        let hosted = runtime.hosted_inserts(&project.tracks[0]);
        let mut left = vec![0.1; 64];
        let mut right = vec![0.1; 64];
        hosted[0]
            .processor
            .process(&mut left, &mut right)
            .expect("Gateway block");
        assert!(left.iter().chain(&right).all(|sample| sample.is_finite()));
    }

    #[test]
    #[ignore]
    fn installed_waveshell_editor_opens_without_crashing() {
        let module = Path::new(r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3");
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].inserts.push(InsertEffect {
            id: 0,
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::Vst3 {
                class_id: "37545356433663536c612d3736207374".to_owned(),
                module_path: module.to_path_buf(),
                name: "CLA-76 Stereo".to_owned(),
                vendor: "Waves".to_owned(),
                component_state: Vec::new(),
                controller_state: Vec::new(),
            },
        });
        let track_id = project.tracks[0].id;
        let mut runtime = Runtime::default();
        runtime.set_format(48_000, 64);
        runtime.sync(&project);
        assert_eq!(runtime.failure(track_id, 0), None);
        let window = probe_editor_window("OpenTrack VST3 editor test");
        runtime
            .open_editor(track_id, 0, window.hwnd())
            .expect("CLA-76 editor opens");
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
            };
            let started = std::time::Instant::now();
            while started.elapsed() < std::time::Duration::from_millis(500) {
                let mut message = MSG::default();
                while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) }
                    != 0
                {
                    unsafe {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                // Match the full app's parameter window, which polls the
                // controller while the native editor is open.
                let _ = runtime.parameters(track_id, 0);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        runtime.close_editor(track_id, 0);
    }

    #[test]
    #[ignore]
    fn installed_waveshell_processes_on_a_separate_audio_thread() {
        let module = Path::new(r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3");
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].inserts.push(InsertEffect {
            id: 0,
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::Vst3 {
                class_id: "37545356433663536c612d3736207374".to_owned(),
                module_path: module.to_path_buf(),
                name: "CLA-76 Stereo".to_owned(),
                vendor: "Waves".to_owned(),
                component_state: Vec::new(),
                controller_state: Vec::new(),
            },
        });
        let mut runtime = Runtime::default();
        runtime.set_format(48_000, 64);
        runtime.sync(&project);
        assert_eq!(runtime.failure(project.tracks[0].id, 0), None);
        let hosted = runtime.hosted_inserts(&project.tracks[0]);
        std::thread::spawn(move || {
            for block in 0..256 {
                let mut left = (0..64)
                    .map(|frame| {
                        (std::f32::consts::TAU * 220.0 * (frame + block * 64) as f32 / 48_000.0)
                            .sin()
                            * 0.1
                    })
                    .collect::<Vec<_>>();
                let mut right = left.clone();
                hosted[0]
                    .processor
                    .process(&mut left, &mut right)
                    .expect("audio-thread block");
            }
        })
        .join()
        .expect("audio thread");
    }

    #[test]
    #[ignore]
    fn installed_tdr_nova_loads_in_an_existing_insert_slot() {
        let module = Path::new(r"C:\Program Files\Common Files\VST3\TDR Nova.vst3");
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].inserts.push(InsertEffect {
            id: 0,
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::Vst3 {
                class_id: "54545356356474616472206e6f766100".to_owned(),
                module_path: module.to_path_buf(),
                name: "TDR Nova".to_owned(),
                vendor: "Tokyo Dawn Labs".to_owned(),
                component_state: Vec::new(),
                controller_state: Vec::new(),
            },
        });
        let mut runtime = Runtime::default();
        runtime.set_format(48_000, 64);
        runtime.sync(&project);
        assert_eq!(
            runtime.failure(project.tracks[0].id, 0),
            None,
            "the installed TDR Nova must load"
        );
        let hosted = runtime.hosted_inserts(&project.tracks[0]);
        assert_eq!(hosted.len(), 1);
        assert_eq!(hosted[0].processor.latency_samples(), 184);
        assert!(
            !runtime
                .parameters(project.tracks[0].id, 0)
                .unwrap()
                .is_empty()
        );
        assert!(runtime.has_editor(project.tracks[0].id, 0));
        let mut left = (0..64)
            .map(|frame| (std::f32::consts::TAU * 220.0 * frame as f32 / 48_000.0).sin() * 0.1)
            .collect::<Vec<_>>();
        let mut right = left.clone();
        hosted[0]
            .processor
            .process(&mut left, &mut right)
            .expect("TDR Nova processes a block");
        assert!(left.iter().chain(&right).all(|sample| sample.is_finite()));
        let state = runtime
            .instance(project.tracks[0].id, 0)
            .unwrap()
            .save_state()
            .expect("TDR Nova state saves");
        eprintln!(
            "TDR Nova state: component={} controller={}",
            state.component.len(),
            state.controller.len()
        );
        assert!(!state.component.is_empty());
        runtime.capture_states(&mut project);
        let InsertEffectKind::Vst3 {
            component_state,
            controller_state,
            ..
        } = &project.tracks[0].inserts[0].kind
        else {
            panic!("VST3 insert changed kind");
        };
        assert!(!component_state.is_empty() || !controller_state.is_empty());

        let mut reopened = Runtime::default();
        reopened.set_format(48_000, 64);
        reopened.sync(&project);
        assert_eq!(reopened.failure(project.tracks[0].id, 0), None);
        assert!(
            !reopened
                .parameters(project.tracks[0].id, 0)
                .unwrap()
                .is_empty()
        );
        let restored = reopened
            .instance(project.tracks[0].id, 0)
            .unwrap()
            .save_state()
            .expect("restored state saves");
        assert!(!restored.component.is_empty());
    }

    #[test]
    #[ignore = "requires installed Waves CLA-76"]
    fn installed_waves_editor_runs_on_a_dedicated_sta_thread() {
        let module = Path::new(r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3");
        let effect = Arc::new(
            Vst3Effect::load(
                module,
                PluginClassId::from_hex("37545356433663536C612D3736207374").unwrap(),
                48_000,
                512,
            )
            .expect("load Waves"),
        );
        let host = EditorHost::open("OpenTrack dedicated editor test", None, effect)
            .expect("editor host starts");
        std::thread::sleep(std::time::Duration::from_millis(750));
        assert!(host.is_alive());
        host.focus();
        host.close();
    }
}
