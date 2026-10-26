//! Control-thread VST3 instance registry for the existing insert slots.
//!
//! Plug-ins are created and edited here. The audio callback receives only the
//! format-neutral `HostedProcessor` trait. Instances are retained by
//! (track, insert slot, class ID) so playback-plan rebuilds never recreate them.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

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
        let platform = selection.join("Contents").join("x86_64-win");
        for folder in [platform, selection.to_path_buf()] {
            let Ok(entries) = std::fs::read_dir(&folder) else {
                continue;
            };
            let mut candidates = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .extension()
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("vst3"))
                })
                .collect::<Vec<_>>();
            candidates.sort();
            if let Some(module) = candidates.into_iter().next() {
                return Ok(module);
            }
        }
        return Err(format!("No x64 module found under {}", selection.display()));
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
impl EditorWindow {
    pub fn create(title: &str, owner: isize) -> Result<Self, String> {
        use windows_sys::Win32::{
            Foundation::{ERROR_CLASS_ALREADY_EXISTS, GetLastError},
            System::LibraryLoader::GetModuleHandleW,
            UI::WindowsAndMessaging::{
                CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, RegisterClassW, WNDCLASSW,
                WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
            },
        };
        static REGISTERED: OnceLock<()> = OnceLock::new();
        let class: Vec<u16> = "OpenTrackVst3Editor\0".encode_utf16().collect();
        REGISTERED.get_or_init(|| unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(DefWindowProcW),
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
        });
        let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        let static_class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                960,
                640,
                owner as _,
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
        // VST3 hosts normally attach the view to a child of their frame, not
        // directly to the top-level window. Waves editors are sensitive to this.
        let child = unsafe {
            CreateWindowExW(
                0,
                static_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE,
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

    pub fn hwnd(&self) -> isize {
        self.hwnd
    }

    pub fn is_alive(&self) -> bool {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(self.frame as _) != 0 }
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

    pub fn is_alive(&self) -> bool {
        false
    }
}

impl Runtime {
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
        for track in &project.tracks {
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
                    enabled: insert.enabled,
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
        for track in &mut project.tracks {
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
        let window = EditorWindow::create("OpenTrack VST3 editor test", 0).expect("editor host");
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
}
