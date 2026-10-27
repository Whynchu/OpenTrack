// SPDX-License-Identifier: Apache-2.0

//! Native VST3 editor attachment.
//!
//! The host does not create a window here: the application supplies an owner
//! HWND. This module creates the plug-in view, sets the host `IPlugFrame`,
//! resizes the supplied window to the view size and attaches the view with
//! `kPlatformTypeHWND`. Closing detaches the view and releases it.
//!
//! The editor is strictly a UI-thread object. It is never reached from the
//! audio callback.

use std::ffi::c_void;

use vst3::ComPtr;
use vst3::Steinberg::Vst::{IEditController, IEditControllerTrait};
use vst3::Steinberg::{
    FIDString, IPlugFrame, IPlugView, IPlugViewTrait, ViewRect, kPlatformTypeHWND, kResultOk,
};

/// One live editor view.
pub(crate) struct EditorSession {
    pub(crate) view: ComPtr<IPlugView>,
    pub(crate) hwnd: isize,
    pub(crate) width: u32,
    pub(crate) height: u32,
    _com: EditorApartment,
}

/// Creates and attaches the plug-in editor to `parent`.
pub(crate) fn open_session(
    controller: &ComPtr<IEditController>,
    frame: &ComPtr<IPlugFrame>,
    parent: *mut c_void,
) -> Result<EditorSession, String> {
    if parent.is_null() {
        return Err(String::from("editor parent window handle is null"));
    }
    let _com = EditorApartment::new();

    // SAFETY: `createView` is a control-thread call and "editor" is a
    // NUL-terminated VST3 view name.
    let raw_view = unsafe { controller.createView(c"editor".as_ptr() as FIDString) };
    let view = unsafe { ComPtr::<IPlugView>::from_raw(raw_view) }
        .ok_or_else(|| String::from("plug-in has no editor view"))?;

    // SAFETY: control-thread call on a live view.
    if unsafe { view.isPlatformTypeSupported(kPlatformTypeHWND) } != kResultOk {
        return Err(String::from(
            "plug-in editor does not support the Windows HWND platform type",
        ));
    }

    let mut rect = ViewRect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: control-thread call on a live view with a valid out pointer.
    let size_result = unsafe { view.getSize(&mut rect) };
    let (width, height) = if size_result == kResultOk {
        let width = rect.right.wrapping_sub(rect.left);
        let height = rect.bottom.wrapping_sub(rect.top);
        if width > 0 && height > 0 && width <= 16_384 && height <= 16_384 {
            (width as u32, height as u32)
        } else {
            (800, 600)
        }
    } else {
        (800, 600)
    };

    // SAFETY: control-thread call on a live view.
    if unsafe { view.setFrame(frame.as_ptr()) } != kResultOk {
        return Err(String::from("plug-in rejected the host IPlugFrame"));
    }

    let hwnd = parent as isize;
    if let Err(error) = resize_native_window(hwnd, width, height) {
        // SAFETY: detach the frame we just set before dropping the view.
        unsafe {
            let _ = view.setFrame(std::ptr::null_mut());
        }
        return Err(error);
    }

    // SAFETY: the parent is a valid HWND supplied by the application and the
    // view is alive for the duration of the call.
    if unsafe { view.attached(parent, kPlatformTypeHWND) } != kResultOk {
        // SAFETY: best-effort detachment after a failed attach.
        unsafe {
            let _ = view.setFrame(std::ptr::null_mut());
        }
        return Err(String::from("plug-in failed to attach its editor view"));
    }

    let mut attached_size = ViewRect {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    };
    unsafe {
        // Some views determine their final size only once attached.
        let _ = view.getSize(&mut attached_size);
    }
    let attached_width = attached_size.right.wrapping_sub(attached_size.left);
    let attached_height = attached_size.bottom.wrapping_sub(attached_size.top);
    if (1..=16_384).contains(&attached_width) && (1..=16_384).contains(&attached_height) {
        let _ = resize_native_window(hwnd, attached_width as u32, attached_height as u32);
        unsafe {
            let _ = view.onSize(&mut attached_size);
        }
    }
    // Let views that draw directly into the supplied HWND know that they own
    // focus. This also starts editor-side timers in some plug-ins.
    unsafe {
        let _ = view.onFocus(1);
    }
    redraw_editor_surface(hwnd);

    Ok(EditorSession {
        view,
        hwnd,
        width,
        height,
        _com,
    })
}

#[cfg(windows)]
fn redraw_editor_surface(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        RDW_ALLCHILDREN, RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow,
    };

    // Request an initial paint after attachment and sizing. Some views paint
    // asynchronously, so this request alone does not establish visual success.
    unsafe {
        let _ = RedrawWindow(
            Some(HWND(hwnd as *mut c_void)),
            None,
            None,
            RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW,
        );
    }
}

#[cfg(not(windows))]
fn redraw_editor_surface(_hwnd: isize) {}

/// Detaches and releases an attached view.
pub(crate) fn close_session(session: EditorSession) {
    // SAFETY: both calls are UI-thread calls on a live, attached view.
    unsafe {
        let _ = session.view.onFocus(0);
        let _ = session.view.removed();
        let _ = session.view.setFrame(std::ptr::null_mut());
    }
    // Dropping `session` releases the view and uninitializes the apartment when
    // this thread initialized it.
}

/// Resizes the editor and informs the plug-in.
pub(crate) fn resize_session(
    session: &mut EditorSession,
    width: u32,
    height: u32,
) -> Result<(), String> {
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
        return Err(format!("invalid editor size {width}x{height}"));
    }
    let mut rect = ViewRect {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    };
    // SAFETY: UI-thread call on a live view.
    // The plug-in may adjust the rectangle to its closest valid size.
    if unsafe { session.view.checkSizeConstraint(&mut rect) } == kResultOk {
        let constrained_width = rect.right.wrapping_sub(rect.left);
        let constrained_height = rect.bottom.wrapping_sub(rect.top);
        if constrained_width <= 0
            || constrained_height <= 0
            || constrained_width > 16_384
            || constrained_height > 16_384
        {
            return Err(String::from("plug-in returned an invalid editor size"));
        }
    }
    let target_width = rect.right.wrapping_sub(rect.left).max(1) as u32;
    let target_height = rect.bottom.wrapping_sub(rect.top).max(1) as u32;
    resize_native_window(session.hwnd, target_width, target_height)?;
    // SAFETY: UI-thread call on a live view with a valid rectangle.
    if unsafe { session.view.onSize(&mut rect) } != kResultOk {
        return Err(String::from("plug-in rejected the editor size"));
    }
    session.width = target_width;
    session.height = target_height;
    Ok(())
}

/// Resizes an existing HWND so its client area matches the requested size.
pub(crate) fn resize_native_window(hwnd: isize, width: u32, height: u32) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, RECT};
        use windows::Win32::UI::WindowsAndMessaging::{
            GWL_STYLE, GetClientRect, GetParent, GetWindowLongW, GetWindowRect, SWP_NOACTIVATE,
            SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, WS_CHILD,
        };

        if hwnd == 0 || width == 0 || height == 0 {
            return Err(String::from("invalid editor window or size"));
        }
        let hwnd = HWND(hwnd as *mut c_void);
        let mut client = RECT::default();
        let mut window = RECT::default();
        // Account for the window frame so the plug-in view fills the client
        // area rather than the outer window.
        let (frame_width, frame_height) = {
            // SAFETY: both calls only read window geometry for a caller-owned
            // HWND.
            let client_ok = unsafe { GetClientRect(hwnd, &mut client) }.is_ok();
            // SAFETY: as above.
            let window_ok = unsafe { GetWindowRect(hwnd, &mut window) }.is_ok();
            if client_ok && window_ok {
                (
                    (window.right - window.left - (client.right - client.left)).max(0),
                    (window.bottom - window.top - (client.bottom - client.top)).max(0),
                )
            } else {
                (0, 0)
            }
        };
        let outer_width = width as i32 + frame_width;
        let outer_height = height as i32 + frame_height;
        // SAFETY: `hwnd` is a caller-owned window handle; the flags keep the
        // editor from stealing focus or changing its z-order.
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                outer_width,
                outer_height,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        }
        .map_err(|error| format!("could not resize editor window: {error}"))?;
        // The supplied HWND is the view container. Resize its enclosing host
        // frame as well, preserving the plugin's requested native client size.
        if unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32 & WS_CHILD.0 != 0 {
            if let Ok(parent) = unsafe { GetParent(hwnd) } {
                resize_native_window(parent.0 as isize, width, height)?;
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, width, height);
        Err(String::from(
            "native VST3 editors are only supported on Windows",
        ))
    }
}

#[cfg(windows)]
struct EditorApartment {
    initialized: bool,
}

#[cfg(windows)]
impl EditorApartment {
    fn new() -> Self {
        // Apartment-threaded COM is the conventional editor apartment. If the
        // thread already initialized a different apartment (for example the
        // module loader's MTA), keep using it rather than failing the editor.
        let result = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            )
        };
        Self {
            initialized: result.is_ok(),
        }
    }
}

#[cfg(windows)]
impl Drop for EditorApartment {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: every successful CoInitializeEx on this thread is paired
            // with one CoUninitialize.
            unsafe { windows::Win32::System::Com::CoUninitialize() };
        }
    }
}

#[cfg(not(windows))]
struct EditorApartment;

#[cfg(not(windows))]
impl EditorApartment {
    fn new() -> Self {
        Self
    }
}
