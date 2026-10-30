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

pub(crate) fn install_resource_icons(hwnd: Option<isize>) {
    #[cfg(windows)]
    if let Some(hwnd) = hwnd {
        platform::install_resource_icons(hwnd);
    }
    #[cfg(not(windows))]
    let _ = hwnd;
}

/// Reasserts the Transport viewport's Win32 z-order after it has been mapped.
/// Winit also requests this level, but an explicit HWND_TOPMOST raises the
/// borderless viewport above sibling owned tool windows when it is pinned.
pub(crate) fn apply_transport_pin(root: Option<isize>, pinned: bool) -> bool {
    #[cfg(windows)]
    {
        root.is_some_and(|root| platform::apply_transport_pin(root, pinned))
    }
    #[cfg(not(windows))]
    {
        let _ = (root, pinned);
        false
    }
}

/// Raise the main viewport within the normal application window band so an
/// egui popup in it stays above OpenTrack's unpinned sibling viewports.
pub(crate) fn raise_root_for_popup(root: Option<isize>) {
    #[cfg(windows)]
    if let Some(root) = root {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
        };
        // SAFETY: root is the live main viewport owned by this app. No other
        // process window is enumerated or changed, and this does not set the
        // global topmost style.
        unsafe {
            SetWindowPos(
                root as _,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
    #[cfg(not(windows))]
    let _ = root;
}

/// Raise the unpinned Transport viewport in the normal z-order band while its
/// own context menu is open.
pub(crate) fn raise_transport_for_popup(root: Option<isize>) {
    #[cfg(windows)]
    if let Some(root) = root {
        platform::raise_named_window(root, "Transport Panel");
    }
    #[cfg(not(windows))]
    let _ = root;
}

pub(crate) fn raise_media_for_popup(root: Option<isize>) {
    #[cfg(windows)]
    if let Some(root) = root {
        platform::raise_named_window(root, "Media Pool");
    }
    #[cfg(not(windows))]
    let _ = root;
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

    pub(super) fn install_resource_icons(root: isize) {
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_SHARED, LoadImageW, SM_CXICON, SM_CXSMICON,
            SM_CYICON, SM_CYSMICON, SendMessageW, WM_SETICON,
        };

        let module = unsafe { GetModuleHandleW(std::ptr::null()) };
        let resource_id = 1usize as *const u16;
        let small = unsafe {
            LoadImageW(
                module,
                resource_id,
                IMAGE_ICON,
                GetSystemMetrics(SM_CXSMICON),
                GetSystemMetrics(SM_CYSMICON),
                LR_SHARED,
            )
        };
        let large = unsafe {
            LoadImageW(
                module,
                resource_id,
                IMAGE_ICON,
                GetSystemMetrics(SM_CXICON),
                GetSystemMetrics(SM_CYICON),
                LR_SHARED,
            )
        };
        unsafe {
            if !small.is_null() {
                SendMessageW(root as _, WM_SETICON, ICON_SMALL as usize, small as isize);
            }
            if !large.is_null() {
                SendMessageW(root as _, WM_SETICON, ICON_BIG as usize, large as isize);
            }
        }
    }

    pub(super) fn apply_transport_pin(root: isize, pinned: bool) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            WS_EX_TOPMOST,
        };

        struct Search {
            root: HWND,
            pinned: bool,
            found: bool,
        }

        unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> i32 {
            let search = unsafe { &mut *(data as *mut Search) };
            if window == search.root {
                return 1;
            }
            let mut title = [0u16; 256];
            let length = unsafe { GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32) };
            if length <= 0
                || String::from_utf16_lossy(&title[..length as usize]) != "Transport Panel"
            {
                return 1;
            }
            search.found = true;
            let is_topmost =
                unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) & WS_EX_TOPMOST as isize != 0 };
            if search.pinned {
                // Reinsert even when WS_EX_TOPMOST is already set. This raises
                // Transport above other topmost/owned windows in its z-order band.
                unsafe {
                    SetWindowPos(
                        window,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            } else if is_topmost {
                unsafe {
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
            }
            0
        }

        let root = root as HWND;
        let mut search = Search {
            root,
            pinned,
            found: false,
        };
        unsafe {
            let thread = GetWindowThreadProcessId(root, std::ptr::null_mut());
            if thread != 0 {
                EnumThreadWindows(thread, Some(visit), &mut search as *mut Search as LPARAM);
            }
        }
        search.found
    }

    pub(super) fn raise_named_window(root: isize, target_title: &'static str) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
        };
        struct Search {
            root: HWND,
            title: &'static str,
        }
        unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> i32 {
            let search = unsafe { &*(data as *const Search) };
            if window == search.root {
                return 1;
            }
            let mut title = [0u16; 512];
            let length = unsafe { GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32) };
            if length > 0 && String::from_utf16_lossy(&title[..length as usize]) == search.title {
                unsafe {
                    SetWindowPos(
                        window,
                        HWND_TOP,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
                return 0;
            }
            1
        }
        let root = root as HWND;
        let search = Search {
            root,
            title: target_title,
        };
        unsafe {
            let thread = GetWindowThreadProcessId(root, std::ptr::null_mut());
            if thread != 0 {
                EnumThreadWindows(thread, Some(visit), &search as *const Search as LPARAM);
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
