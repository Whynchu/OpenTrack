//! Physical-key input that the GUI framework does not preserve.
//!
//! egui intentionally models a logical key, while the reference keymap uses
//! physical keypad keys separately from the top-row digits. On Windows this
//! module observes key-down messages sent to the application's own window.

use opentrack_core::commands::{ArrowKey, KeyChord, KeyCode, KeyModifiers};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeKeyEvent {
    pub chord: KeyChord,
}

#[cfg(windows)]
mod platform {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BLACK_BRUSH, FillRect, GetStockObject},
        UI::{
            Input::KeyboardAndMouse::{
                GetKeyState, VK_CONTROL, VK_MENU, VK_NUMPAD0, VK_NUMPAD9, VK_SHIFT,
            },
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{GetClientRect, WM_ERASEBKGND, WM_KEYDOWN, WM_SYSKEYDOWN},
        },
    };

    use super::{ArrowKey, KeyChord, KeyCode, KeyModifiers, NativeKeyEvent};

    type EventQueue = Mutex<VecDeque<NativeKeyEvent>>;
    const SUBCLASS_ID: usize = 0x5257_4157;

    pub(crate) struct NativeKeyCapture {
        hwnd: HWND,
        events: Arc<EventQueue>,
    }

    impl NativeKeyCapture {
        pub(crate) fn install(creation: &eframe::CreationContext<'_>) -> Result<Self, String> {
            let window_handle = creation
                .window_handle()
                .map_err(|error| format!("could not access the application window: {error:?}"))?;
            let RawWindowHandle::Win32(handle) = window_handle.as_raw() else {
                return Err("the physical-key adapter requires a Win32 window".to_owned());
            };
            let hwnd = handle.hwnd.get() as HWND;
            let events = Arc::new(Mutex::new(VecDeque::new()));
            let reference_data = Arc::as_ptr(&events) as usize;

            // SAFETY: `events` remains alive until Drop removes this subclass,
            // and the callback only runs on the owning window's UI thread.
            let installed = unsafe {
                SetWindowSubclass(hwnd, Some(capture_key_message), SUBCLASS_ID, reference_data)
            };
            if installed == 0 {
                return Err("Windows could not install the physical-key adapter".to_owned());
            }

            Ok(Self { hwnd, events })
        }

        pub(crate) fn take_events(&self) -> Vec<NativeKeyEvent> {
            let mut events = self
                .events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            events.drain(..).collect()
        }
    }

    impl Drop for NativeKeyCapture {
        fn drop(&mut self) {
            // SAFETY: This is the same live window and subclass callback used
            // at registration. Drop runs before the app releases its window.
            unsafe {
                RemoveWindowSubclass(self.hwnd, Some(capture_key_message), SUBCLASS_ID);
            }
        }
    }

    unsafe extern "system" fn capture_key_message(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _subclass_id: usize,
        reference_data: usize,
    ) -> LRESULT {
        if message == WM_ERASEBKGND {
            // SAFETY: live UI-thread HWND and Windows-supplied erase HDC;
            // the stock brush remains OS-owned. Never expose a white erase frame.
            unsafe {
                let mut rect = RECT::default();
                if GetClientRect(hwnd, &mut rect) != 0 {
                    FillRect(wparam as _, &rect, GetStockObject(BLACK_BRUSH) as _);
                    return 1;
                }
            }
        }
        if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
            let virtual_key = wparam as u16;
            let is_repeat = (lparam as u64 & (1 << 30)) != 0;
            if !is_repeat && let Some(key) = key_code(virtual_key, lparam) {
                let event = NativeKeyEvent {
                    chord: KeyChord::new(key, current_modifiers()),
                };
                // SAFETY: The pointer is the Arc-owned queue registered with
                // SetWindowSubclass and is valid until Drop removes the hook.
                let events = unsafe { &*(reference_data as *const EventQueue) };
                events
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push_back(event);
            }
        }

        // SAFETY: Forward the original message to the window's existing
        // procedure chain, as required by the Windows subclass API.
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }

    fn current_modifiers() -> KeyModifiers {
        KeyModifiers {
            control: key_is_down(VK_CONTROL),
            alt: key_is_down(VK_MENU),
            shift: key_is_down(VK_SHIFT),
            windows: false,
        }
    }

    fn key_is_down(virtual_key: u16) -> bool {
        // SAFETY: GetKeyState accepts a virtual-key code and is called while
        // processing keyboard messages on the window's input thread.
        unsafe { GetKeyState(virtual_key as i32) as u16 & 0x8000 != 0 }
    }

    fn key_code(virtual_key: u16, lparam: LPARAM) -> Option<KeyCode> {
        if let Some(digit) = numpad_digit(virtual_key, lparam) {
            return Some(KeyCode::NumpadDigit(digit));
        }

        let packed = lparam as u64;
        let is_extended = packed & (1 << 24) != 0;
        let scan_code = ((packed >> 16) & 0xff) as u8;
        if !is_extended && scan_code == 0x53 {
            return Some(KeyCode::NumpadDecimal);
        }

        match virtual_key {
            0x41..=0x5a => Some(KeyCode::Letter(char::from(virtual_key as u8))),
            0x30..=0x39 => Some(KeyCode::Digit((virtual_key - 0x30) as u8)),
            0x70..=0x7b => Some(KeyCode::Function((virtual_key - 0x6f) as u8)),
            0x25 => Some(KeyCode::Arrow(ArrowKey::Left)),
            0x26 => Some(KeyCode::Arrow(ArrowKey::Up)),
            0x27 => Some(KeyCode::Arrow(ArrowKey::Right)),
            0x28 => Some(KeyCode::Arrow(ArrowKey::Down)),
            0x0d => Some(KeyCode::Enter),
            0x1b => Some(KeyCode::Escape),
            0x20 => Some(KeyCode::Space),
            0x2d => Some(KeyCode::Insert),
            0x2e => Some(KeyCode::Delete),
            0x08 => Some(KeyCode::Backspace),
            0x24 => Some(KeyCode::Home),
            0x23 => Some(KeyCode::End),
            0xbb => Some(KeyCode::Plus),
            0xbc => Some(KeyCode::Comma),
            0xbd => Some(KeyCode::Minus),
            0xbe => Some(KeyCode::Period),
            0xbf => Some(KeyCode::Slash),
            0x6a => Some(KeyCode::NumpadMultiply),
            0x6b => Some(KeyCode::NumpadAdd),
            0x6c => Some(KeyCode::NumpadComma),
            0x6d => Some(KeyCode::NumpadSubtract),
            0x6e => Some(KeyCode::NumpadDecimal),
            0x6f => Some(KeyCode::NumpadDivide),
            _ => None,
        }
    }

    fn numpad_digit(virtual_key: u16, lparam: LPARAM) -> Option<u8> {
        if (VK_NUMPAD0..=VK_NUMPAD9).contains(&virtual_key) {
            return Some((virtual_key - VK_NUMPAD0) as u8);
        }

        let packed = lparam as u64;
        if packed & (1 << 24) != 0 {
            return None;
        }

        let scan_code = ((packed >> 16) & 0xff) as u8;
        match scan_code {
            0x52 => Some(0),
            0x4f => Some(1),
            0x50 => Some(2),
            0x51 => Some(3),
            0x4b => Some(4),
            0x4c => Some(5),
            0x4d => Some(6),
            0x47 => Some(7),
            0x48 => Some(8),
            0x49 => Some(9),
            _ => None,
        }
    }

    pub(crate) use NativeKeyCapture as Capture;
}

#[cfg(not(windows))]
mod platform {
    use opentrack_core::commands::{KeyChord, KeyCode, KeyModifiers};

    use super::NativeKeyEvent;

    pub(crate) struct NativeKeyCapture;

    impl NativeKeyCapture {
        pub(crate) fn install(_creation: &eframe::CreationContext<'_>) -> Result<Self, String> {
            Ok(Self)
        }

        pub(crate) fn take_events(&self) -> Vec<NativeKeyEvent> {
            Vec::<NativeKeyEvent>::new()
        }
    }

    pub(crate) use NativeKeyCapture as Capture;
}

pub(crate) use platform::Capture as NativeKeyCapture;
