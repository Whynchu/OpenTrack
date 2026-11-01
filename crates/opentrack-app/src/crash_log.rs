//! Keep Rust panic details locally when the app was opened without a console.
use std::sync::atomic::{AtomicBool, Ordering};

/// Set once the application has asked to close. winit's X11 backend queries
/// geometry on a window the window manager has already destroyed while closing,
/// which panics after the last frame has been drawn.
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub(crate) fn begin_shutdown() {
    SHUTTING_DOWN.store(true, Ordering::Relaxed);
}

fn is_shutting_down() -> bool {
    SHUTTING_DOWN.load(Ordering::Relaxed)
}

/// The close path in winit's X11 backend unwraps an `X11Error` raised against a
/// window that no longer exists. Nothing is left to tear down by then, so finish
/// quietly instead of aborting with a core dump and skipping the final save.
fn is_x11_teardown_panic(info: &std::panic::PanicHookInfo<'_>) -> bool {
    let location = info
        .location()
        .map(|location| location.file())
        .unwrap_or_default();
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
        .unwrap_or_default();
    location.contains("winit") && location.contains("x11") && message.contains("X11Error")
}

pub(crate) fn install() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if is_x11_teardown_panic(info) {
            // Closing the root window shuts the application down without setting
            // `close_requested`, so the shutdown marker cannot gate this. The
            // signature is specific to a window that no longer exists during
            // teardown, and there is nothing left to unwind.
            session(&format!(
                "ignored winit X11 teardown panic (explicit close: {})",
                is_shutting_down()
            ));
            std::process::exit(0);
        }
        append_log(
            "panic.log",
            &format!(
                "\nOpenTrack panic at {}: {info}\n{}",
                timestamp(),
                std::backtrace::Backtrace::force_capture()
            ),
        );
        default_hook(info);
    }));
    #[cfg(windows)]
    native::install();
}

fn log_root() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OpenTrack-")
        .join("logs")
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn append_log(file_name: &str, text: &str) {
    use std::io::Write;
    let root = log_root();
    if std::fs::create_dir_all(&root).is_ok()
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join(file_name))
    {
        let _ = write!(file, "{text}");
    }
}

/// Session tracing for hang diagnosis. Truncated once per launch, then appended
/// with timestamps by the UI, the editor thread and hang events.
pub(crate) fn reset_session_log() {
    let root = log_root();
    if std::fs::create_dir_all(&root).is_ok() {
        let _ = std::fs::remove_file(root.join("session.log"));
    }
}

pub(crate) fn session(text: &str) {
    append_log("session.log", &format!("[{:>10}] {text}\n", timestamp()));
}

/// Records native faults (access violations and similar) that never reach the
/// Rust panic hook, so a plug-in crash leaves a local trace.
#[cfg(windows)]
mod native {
    use windows_sys::Win32::Foundation::HMODULE;
    use windows_sys::Win32::System::Diagnostics::Debug::{
        EXCEPTION_CONTINUE_SEARCH, EXCEPTION_POINTERS, RtlCaptureStackBackTrace,
        SetUnhandledExceptionFilter,
    };
    use windows_sys::Win32::System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleFileNameW, GetModuleHandleExW,
    };

    pub(super) fn install() {
        // SAFETY: the filter only observes the exception record and returns
        // EXCEPTION_CONTINUE_SEARCH, leaving normal handling untouched.
        unsafe { SetUnhandledExceptionFilter(Some(filter)) };
    }

    unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
        let mut text = String::new();
        if !info.is_null() {
            // SAFETY: the OS supplies valid exception pointers for this call.
            let pointers = unsafe { &*info };
            if !pointers.ExceptionRecord.is_null() {
                // SAFETY: the record belongs to the exception being reported.
                let record = unsafe { &*pointers.ExceptionRecord };
                let mut frames = [std::ptr::null_mut::<core::ffi::c_void>(); 48];
                // SAFETY: the buffer and its length are provided by us.
                let count = unsafe {
                    RtlCaptureStackBackTrace(
                        1,
                        frames.len() as u32,
                        frames.as_mut_ptr(),
                        std::ptr::null_mut(),
                    )
                } as usize;
                text.push_str(&format!(
                    "\nOpenTrack native fault at {}: code 0x{:08x} in {}\n",
                    super::timestamp(),
                    record.ExceptionCode as u32,
                    module_name(record.ExceptionAddress),
                ));
                for (index, frame) in frames[..count.min(frames.len())].iter().enumerate() {
                    text.push_str(&format!("  #{index} {:p} {}\n", frame, module_name(*frame)));
                }
            }
        }
        if text.is_empty() {
            text.push_str("\nOpenTrack native fault (no exception record)\n");
        }
        super::append_log("exception.log", &text);
        EXCEPTION_CONTINUE_SEARCH
    }

    fn module_name(address: *mut core::ffi::c_void) -> String {
        if address.is_null() {
            return "unknown".to_owned();
        }
        let mut module: HMODULE = std::ptr::null_mut();
        // SAFETY: the address comes from the exception record or a backtrace.
        let ok = unsafe {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                address as *const u16,
                &mut module,
            )
        };
        if ok == 0 || module.is_null() {
            return "unknown".to_owned();
        }
        let mut buffer = [0u16; 512];
        // SAFETY: the module handle is valid and the buffer is ours.
        let length =
            unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) };
        if length == 0 {
            return "unknown".to_owned();
        }
        let name = String::from_utf16_lossy(&buffer[..length as usize]);
        name.rsplit(['\\', '/']).next().unwrap_or(&name).to_owned()
    }
}
