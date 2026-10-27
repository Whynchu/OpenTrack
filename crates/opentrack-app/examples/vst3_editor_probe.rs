//! Standalone native editor diagnostic, independent of eframe/OpenGL and projects.
//! Build with `cargo build -p opentrack --example vst3_editor_probe`.

#[path = "../src/vst3_host.rs"]
#[allow(dead_code)]
mod vst3_host;

#[cfg(windows)]
fn main() {
    use opentrack_plugins::PluginClassId;
    use opentrack_vst3::Vst3Effect;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
    };

    let module = std::env::var_os("OPENTRACK_VST3_PROBE_MODULE").unwrap_or_else(|| {
        r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3".into()
    });
    let class = std::env::var("OPENTRACK_VST3_PROBE_CLASS")
        .unwrap_or_else(|_| "375453564336634D6C612D3736206D6F".to_owned());
    let effect = Vst3Effect::load(
        std::path::Path::new(&module),
        PluginClassId::from_hex(&class).expect("valid class ID"),
        48_000,
        64,
    )
    .expect("load installed effect");
    let window = vst3_host::EditorWindow::create("OpenTrack standalone VST3 probe", 0)
        .expect("create editor host");
    effect
        .open_editor(window.hwnd() as *mut std::ffi::c_void)
        .expect("attach native editor");
    window.focus();
    let start = std::time::Instant::now();
    while window.is_alive() && start.elapsed() < std::time::Duration::from_secs(180) {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    effect.close_editor();
    drop(window);
}

#[cfg(not(windows))]
fn main() {
    eprintln!("This diagnostic currently requires Windows native VST3 editors.");
}
