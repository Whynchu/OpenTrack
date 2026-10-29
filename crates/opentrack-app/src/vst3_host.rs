//! Control-thread VST3 instance registry for the existing insert slots.
//!
//! Plug-ins are created and edited here. The audio callback receives only the
//! format-neutral `HostedProcessor` trait. Instances are retained by
//! (track, insert slot, class ID) so playback-plan rebuilds never recreate them.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use opentrack_audio::HostedInsert;
use opentrack_core::project::{InsertEffectKind, Project, Track};
use opentrack_plugins::{HostedProcessor, PluginClassId};
use opentrack_vst3::{Vst3Effect, Vst3ParameterInfo, Vst3State};

/// Dispatch thread-level Windows timers while a plug-in editor is open.
///
/// Some iPlug2 processors (including NeuralAmpModeler/Gateway) queue meter
/// data from `process()` and transfer it to their editor from `OnIdle()`,
/// which iPlug2 schedules with a thread timer. The VST3 component is created
/// on OpenTrack's UI thread, so pumping timers only on the detached editor
/// thread leaves the processor's idle callback stranded.
#[cfg(windows)]
pub fn pump_ui_thread_timers() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_TIMER,
    };

    let mut message = MSG::default();
    loop {
        // Restrict dispatch to thread-level WM_TIMER messages. Window-bound
        // timers and the app's regular message queue remain under winit.
        let available = unsafe {
            PeekMessageW(
                &mut message,
                std::ptr::null_mut(),
                WM_TIMER,
                WM_TIMER,
                PM_REMOVE,
            )
        };
        if available == 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[cfg(not(windows))]
pub fn pump_ui_thread_timers() {}

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
    instances: HashMap<(u64, usize), Instance>,
    failures: HashMap<(u64, usize), String>,
    failed_class: HashMap<(u64, usize), String>,
}

/// Owner-lifetime native window used as the parent of a VST3 editor view.
#[cfg(windows)]
pub struct EditorWindow {
    frame: isize,
    hwnd: isize,
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
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(self.frame as _) != 0
        }
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
            let _ = thread.join();
        }
        self.alive
            .store(false, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(windows)]
impl Drop for EditorHost {
    fn drop(&mut self) {
        let _ = self.commands.send(EditorCommand::Close);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
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
        let editor = effect.open_detached_editor(window.hwnd() as *mut std::ffi::c_void)?;
        window.restore_position(restore_bounds.as_ref());
        window.show();
        if let Ok(mut current) = bounds.lock() {
            *current = window.bounds();
        }
        window.focus();
        alive.store(true, Ordering::Release);
        let _ = ready.send(Ok(()));

        loop {
            let mut close = false;
            while let Ok(command) = commands.try_recv() {
                match command {
                    EditorCommand::Focus => window.focus(),
                    EditorCommand::SetOwner(owner) => window.set_owner(owner),
                    EditorCommand::Close => close = true,
                }
            }
            if close || !window.is_alive() {
                break;
            }
            window.pump_messages();
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

#[cfg(not(windows))]
pub struct EditorHost;

#[cfg(not(windows))]
impl EditorHost {
    pub fn open(
        _title: &str,
        _restore_bounds: Option<opentrack_core::workspaces::WindowBounds>,
        _effect: Arc<Vst3Effect>,
    ) -> Result<Self, String> {
        Err("native VST3 editors are only supported on Windows".to_owned())
    }

    pub fn is_alive(&self) -> bool {
        false
    }

    pub fn focus(&self) {}

    pub fn set_owner(&self, _owner: isize) {}

    pub fn bounds(&self) -> Option<opentrack_core::workspaces::WindowBounds> {
        None
    }

    pub fn close(self) {}
}

#[cfg(not(windows))]
pub struct EditorWindow;

#[cfg(not(windows))]
impl EditorWindow {
    pub fn create(_title: &str, _owner: isize) -> Result<Self, String> {
        Err("native VST3 editors are only supported on Windows".to_owned())
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
            && project.tracks.iter().find(|track| track.id == track_id)
                .and_then(|track| track.inserts.get(slot))
                .is_some_and(|insert| matches!(&insert.kind, InsertEffectKind::Vst3 { class_id, .. }
                    if self.instances.get(&(track_id, slot)).is_some_and(|instance| instance.class_id == *class_id)))
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
        let mut wanted = HashSet::new();
        let mut changed = false;
        for track in project
            .tracks
            .iter()
            .chain(std::iter::once(&project.output_channel.strip))
        {
            for (slot, insert) in track.inserts.iter().enumerate() {
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
                let key = (track.id, slot);
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
                match Vst3Effect::load(module_path, class_id, self.sample_rate, self.max_block) {
                    Ok(effect) => {
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
        changed
    }

    /// Load or restore one VST3 insert. Project loading uses this one-at-a-time
    /// entry point to keep the UI responsive between plug-in instances.
    pub fn load_insert(&mut self, track: &Track, slot: usize) -> bool {
        if self.sample_rate == 0 || self.max_block == 0 {
            return false;
        }
        let Some(insert) = track.inserts.get(slot) else {
            return false;
        };
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
        let key = (track.id, slot);
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
                let instance = self.instances.get(&(track.id, slot))?;
                let processor: Arc<dyn HostedProcessor> = instance.effect.clone();
                Some(HostedInsert {
                    processor,
                    enabled: insert.enabled && !track.inserts_bypassed,
                })
            })
            .collect()
    }

    pub fn instance(&self, track_id: u64, slot: usize) -> Option<Arc<Vst3Effect>> {
        self.instances
            .get(&(track_id, slot))
            .map(|instance| Arc::clone(&instance.effect))
    }

    pub fn parameters(&self, track_id: u64, slot: usize) -> Option<Vec<Vst3ParameterInfo>> {
        self.instance(track_id, slot)
            .map(|effect| effect.parameters())
    }

    pub fn sync_editor_parameters(&self, track_id: u64, slot: usize) {
        if let Some(effect) = self.instance(track_id, slot) {
            effect.sync_output_parameters();
        }
    }

    pub fn set_parameter(&self, track_id: u64, slot: usize, id: u32, value: f64) {
        if let Some(effect) = self.instance(track_id, slot) {
            effect.set_parameter(id, value);
        }
    }

    pub fn failure(&self, track_id: u64, slot: usize) -> Option<&str> {
        self.failures.get(&(track_id, slot)).map(String::as_str)
    }

    pub fn retry(&mut self, track_id: u64, slot: usize) {
        self.failures.remove(&(track_id, slot));
        self.failed_class.remove(&(track_id, slot));
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
                let Some(instance) = self.instances.get(&(track.id, slot)) else {
                    continue;
                };
                if instance.class_id != *class_id {
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
        let window = EditorWindow::create("OpenTrack compatibility probe", None).unwrap();
        first
            .open_editor(window.hwnd() as *mut std::ffi::c_void)
            .expect("native editor");
        pump_editor_messages();
        if let Ok(seconds) = std::env::var("OPENTRACK_VST3_PROBE_VIEW_SECONDS") {
            let started = std::time::Instant::now();
            while started.elapsed().as_secs() < seconds.parse::<u64>().unwrap_or(0).min(120) {
                pump_editor_messages();
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
        pump_editor_messages();
        eprintln!("stage: save/restore");
        let state = first.save_state().unwrap();
        first.load_state(&state).unwrap();
        first.close_editor();
        drop(window);
        eprintln!("stage: reopen editor");
        let window = EditorWindow::create("OpenTrack compatibility reopen", None).unwrap();
        first
            .open_editor(window.hwnd() as *mut std::ffi::c_void)
            .unwrap();
        pump_editor_messages();
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

    fn pump_editor_messages() {
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
            enabled: true,
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
                enabled: true,
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
            enabled: true,
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
            enabled: true,
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
        let window = EditorWindow::create("OpenTrack VST3 editor test", None).expect("editor host");
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
            enabled: true,
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
            enabled: true,
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
