//! Reproduce native editor attachment during an eframe OpenGL UI frame.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../src/vst3_host.rs"]
#[allow(dead_code)]
mod vst3_host;

#[cfg(windows)]
struct Probe {
    effect: Option<std::sync::Arc<opentrack_vst3::Vst3Effect>>,
    window: Option<vst3_host::EditorWindow>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    owner: isize,
    loaded: bool,
    opened: bool,
    started: std::time::Instant,
    _audio: Option<opentrack_audio::AudioEngine>,
}

#[cfg(windows)]
impl eframe::App for Probe {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().show_viewport_immediate(
            eframe::egui::ViewportId::from_hash_of("probe-channel-settings"),
            eframe::egui::ViewportBuilder::default()
                .with_title("Channel Settings diagnostic")
                .with_inner_size([500.0, 200.0]),
            |ctx, _| {
                eframe::egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Second native viewport, matching Channel Settings.");
                });
            },
        );
        ui.ctx().show_viewport_immediate(
            eframe::egui::ViewportId::from_hash_of("probe-transport"),
            eframe::egui::ViewportBuilder::default()
                .with_title("Transport Panel diagnostic")
                .with_inner_size([500.0, 120.0]),
            |ctx, _| {
                eframe::egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Third native viewport, matching Transport Panel.");
                });
            },
        );
        if !self.loaded {
            self.loaded = true;
            let effect = std::sync::Arc::new(
                opentrack_vst3::Vst3Effect::load(
                    std::path::Path::new(
                        r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3",
                    ),
                    opentrack_plugins::PluginClassId::from_hex("375453564336634D6C612D3736206D6F")
                        .unwrap(),
                    48_000,
                    64,
                )
                .expect("load Waves"),
            );
            let worker_effect = std::sync::Arc::clone(&effect);
            let stop = std::sync::Arc::clone(&self.stop);
            self.worker = Some(std::thread::spawn(move || {
                use opentrack_plugins::HostedProcessor;
                while !stop.load(std::sync::atomic::Ordering::Acquire) {
                    let mut left = [0.0_f32; 64];
                    let mut right = left;
                    worker_effect.process(&mut left, &mut right).unwrap();
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }));
            self.effect = Some(effect);
        }
        if self.loaded
            && !self.opened
            && self.started.elapsed() > std::time::Duration::from_secs(10)
        {
            self.opened = true;
            let effect = std::sync::Arc::clone(self.effect.as_ref().unwrap());
            let window =
                vst3_host::EditorWindow::create("OpenTrack egui Waves editor", self.owner).unwrap();
            effect
                .open_editor(window.hwnd() as *mut std::ffi::c_void)
                .unwrap();
            window.focus();
            self.window = Some(window);
        }
        ui.label("Native Waves editor opened from the OpenGL UI. Close the editor to finish.");
        if self.window.as_ref().is_some_and(|w| !w.is_alive())
            || self.started.elapsed() > std::time::Duration::from_secs(180)
        {
            ui.ctx()
                .send_viewport_cmd(eframe::egui::ViewportCommand::Close);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(30));
    }
}

#[cfg(windows)]
impl Drop for Probe {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
        if let Some(effect) = &self.effect {
            effect.close_editor();
        }
        self.window = None;
    }
}

#[cfg(windows)]
fn main() -> eframe::Result {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    // Match the full application's background in-process class enumeration.
    std::thread::spawn(|| {
        let module = opentrack_vst3::Vst3Module::open(std::path::Path::new(
            r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3",
        ))
        .unwrap();
        module.enumerate().unwrap();
    })
    .join()
    .unwrap();
    eframe::run_native(
        "OpenTrack renderer diagnostic",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default().with_inner_size([500.0, 120.0]),
            ..Default::default()
        },
        Box::new(|creation| {
            let RawWindowHandle::Win32(handle) = creation.window_handle().unwrap().as_raw() else {
                unreachable!()
            };
            let audio = if std::env::var_os("OPENTRACK_PROBE_AUDIO_ASIO").is_some() {
                let inventory = opentrack_audio::scan_audio_devices();
                let device = inventory
                    .devices
                    .iter()
                    .find(|device| device.id.starts_with("asio:"))
                    .expect("ASIO device");
                Some(
                    opentrack_audio::AudioEngine::open_output_device_mode(
                        Some(&device.id),
                        Some(48_000),
                        None,
                        opentrack_audio::AudioDeviceMode::SharedLowLatency,
                    )
                    .expect("open ASIO output"),
                )
            } else {
                None
            };
            Ok(Box::new(Probe {
                effect: None,
                window: None,
                stop: Default::default(),
                worker: None,
                owner: handle.hwnd.get(),
                loaded: false,
                opened: false,
                started: std::time::Instant::now(),
                _audio: audio,
            }))
        }),
    )
}

#[cfg(not(windows))]
fn main() {}
