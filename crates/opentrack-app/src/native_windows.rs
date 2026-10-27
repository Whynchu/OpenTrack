//! Keep Channel Settings above its OpenTrack owner without global topmost status.
#[derive(Default)]
pub(crate) struct ChannelOwner {
    root: Option<isize>,
    dialog_open: bool,
}

pub(crate) fn creation_root(creation: &eframe::CreationContext<'_>) -> Option<isize> {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = creation.window_handle()
            && let RawWindowHandle::Win32(handle) = handle.as_raw()
        {
            return Some(handle.hwnd.get());
        }
    }
    let _ = creation;
    None
}

impl ChannelOwner {
    pub(crate) fn new(creation: &eframe::CreationContext<'_>) -> Self {
        if let Some(root) = creation_root(creation) {
            return Self {
                root: Some(root),
                dialog_open: false,
            };
        }
        Self::default()
    }

    /// Returns true when an application dialog needs the main window raised.
    pub(crate) fn apply(&mut self, pinned: bool, dialog_open: bool, modal: bool) -> bool {
        let raise_dialog = dialog_open && !self.dialog_open;
        self.dialog_open = dialog_open;
        #[cfg(windows)]
        if let Some(root) = self.root {
            platform::apply(root, pinned && !dialog_open, modal);
        }
        #[cfg(not(windows))]
        let _ = (self.root, pinned);
        raise_dialog
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM},
        UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
    };

    struct Search {
        root: HWND,
        pinned: bool,
        modal: bool,
    }

    pub(super) fn apply(root: isize, pinned: bool, modal: bool) {
        let mut search = Search {
            root: root as HWND,
            pinned,
            modal,
        };
        // SAFETY: enumerate only the main window's UI thread; callback data lives
        // until the synchronous enumeration returns. No foreign windows are changed.
        unsafe {
            let thread = GetWindowThreadProcessId(search.root, std::ptr::null_mut());
            if thread != 0 {
                EnumThreadWindows(thread, Some(visit), &mut search as *mut Search as LPARAM);
            }
        }
    }

    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> i32 {
        // SAFETY: `data` points to the live Search in the synchronous caller.
        let search = unsafe { &*(data as *const Search) };
        let mut title = [0u16; 512];
        let length = unsafe { GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32) };
        if window != search.root
            && length > 0
            && String::from_utf16_lossy(&title[..length as usize])
                .starts_with("VST Audio Channel Settings - ")
        {
            unsafe { apply_to_window(search.root, window, search.pinned, search.modal) };
        }
        1
    }

    unsafe fn apply_to_window(root: HWND, window: HWND, pinned: bool, modal: bool) {
        let owner = if pinned && !modal { root as isize } else { 0 };
        // SAFETY: these are live windows on this UI thread. Windows ownership
        // keeps the editor above its main window while other apps can cover both.
        unsafe {
            if GetWindowLongPtrW(window, GWLP_HWNDPARENT) != owner {
                SetWindowLongPtrW(window, GWLP_HWNDPARENT, owner);
            }
            if GetWindowLongPtrW(window, GWL_EXSTYLE) & WS_EX_TOPMOST as isize != 0 {
                SetWindowPos(
                    window,
                    HWND_NOTOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            EnableWindow(window, (!modal) as i32);
            if modal {
                // Detaching the owner alone leaves this window above the main
                // viewport, where it can cover an egui confirmation dialog.
                SetWindowPos(
                    window,
                    HWND_BOTTOM,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn pin_is_owner_relative_and_dialog_temporarily_releases_it() {
            let class = [83u16, 84, 65, 84, 73, 67, 0]; // Windows STATIC class
            unsafe {
                let create = || {
                    CreateWindowExW(
                        0,
                        class.as_ptr(),
                        class.as_ptr(),
                        WS_OVERLAPPEDWINDOW,
                        0,
                        0,
                        200,
                        100,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null(),
                    )
                };
                let root = create();
                let editor = create();
                assert!(!root.is_null() && !editor.is_null());
                apply_to_window(root, editor, true, false);
                assert_eq!(GetWindowLongPtrW(editor, GWLP_HWNDPARENT), root as isize);
                assert_eq!(
                    GetWindowLongPtrW(editor, GWL_EXSTYLE) & WS_EX_TOPMOST as isize,
                    0
                );
                apply_to_window(root, editor, true, true);
                assert_eq!(GetWindowLongPtrW(editor, GWLP_HWNDPARENT), 0);
                assert_eq!(
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(editor),
                    0
                );
                let mut below = GetWindow(root, GW_HWNDNEXT);
                while !below.is_null() && below != editor {
                    below = GetWindow(below, GW_HWNDNEXT);
                }
                assert_eq!(below, editor, "modal confirmation must be above the editor");
                apply_to_window(root, editor, true, false);
                assert_eq!(GetWindowLongPtrW(editor, GWLP_HWNDPARENT), root as isize);
                apply_to_window(root, editor, false, false);
                assert_eq!(GetWindowLongPtrW(editor, GWLP_HWNDPARENT), 0);
                DestroyWindow(editor);
                DestroyWindow(root);
            }
        }
    }
}
