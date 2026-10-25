//! Keep Rust panic details locally when the app was opened without a console.
pub(crate) fn install() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        let root = std::env::var_os("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("OpenTrack-")
            .join("logs");
        if std::fs::create_dir_all(&root).is_ok()
            && let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(root.join("panic.log"))
        {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(
                file,
                "\nOpenTrack panic at {timestamp}: {info}\n{}",
                std::backtrace::Backtrace::force_capture()
            );
        }
        default_hook(info);
    }));
}
