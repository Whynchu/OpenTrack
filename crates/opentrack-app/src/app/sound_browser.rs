//! Browse Sounds: a bounded browser over a user-chosen folder of WAV files.
//!
//! The browser lists audio files from one persisted folder, shows the selected
//! file's format, and imports it through the ordinary WAV import path (which
//! copies it into the project and places it on the selected audio track). It
//! does not audition audio: the preview is the file's metadata, stated plainly.

use super::*;

/// Most files one scan returns.
pub(super) const MAX_SOUND_FILES: usize = 2_000;
/// How deep the scan walks below the chosen folder.
const MAX_SCAN_DEPTH: usize = 4;

/// Metadata shown for the selected file.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SoundFileInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub frames: u32,
}

impl SoundFileInfo {
    pub(super) fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            f64::from(self.frames) / f64::from(self.sample_rate)
        }
    }
}

/// True for the audio files this build imports.
pub(super) fn is_sound_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("wav") || extension.eq_ignore_ascii_case("wave")
        })
}

/// WAV files under `folder`, optionally filtered by file-name substring.
pub(super) fn scan_sound_folder(folder: &Path, filter: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_sound_files(folder, 0, &mut files)?;
    let filter = filter.trim().to_lowercase();
    files.retain(|path| {
        is_sound_file(path)
            && (filter.is_empty()
                || path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_lowercase().contains(&filter))
                    .unwrap_or(false))
    });
    files.sort_by(|left, right| {
        left.file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .cmp(
                &right
                    .file_name()
                    .map(|name| name.to_string_lossy().to_lowercase()),
            )
            .then_with(|| left.cmp(right))
    });
    files.truncate(MAX_SOUND_FILES);
    Ok(files)
}

fn collect_sound_files(
    folder: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if depth > MAX_SCAN_DEPTH {
        return Ok(());
    }
    let entries = std::fs::read_dir(folder)
        .map_err(|error| format!("could not read {}: {error}", folder.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            // A subfolder that cannot be read must not abort the whole scan.
            let _ = collect_sound_files(&path, depth + 1, files);
            continue;
        }
        if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// Format of a WAV file, or its error message.
pub(super) fn sound_file_info(path: &Path) -> Result<SoundFileInfo, String> {
    opentrack_audio::audio_file_metadata(path).map(|metadata| SoundFileInfo {
        sample_rate: metadata.sample_rate,
        channels: metadata.channels,
        bits_per_sample: metadata.bits_per_sample,
        frames: metadata.frames,
    })
}

impl OpenTrackApp {
    /// Opens the browser, scanning the persisted folder when there is one.
    fn open_sound_browser(&mut self) {
        self.sound_browser_open = true;
        if let Some(folder) = self.sound_browser_folder.clone() {
            self.rescan_sound_folder(&folder);
        }
    }

    fn choose_sound_folder(&mut self) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose a folder to browse for sounds")
            .pick_folder()
        else {
            return;
        };
        self.sound_browser_folder = Some(folder.clone());
        self.rescan_sound_folder(&folder);
    }

    pub(super) fn rescan_sound_folder(&mut self, folder: &Path) {
        match scan_sound_folder(folder, &self.sound_browser_filter) {
            Ok(files) => {
                let count = files.len();
                self.sound_browser_files = files;
                self.sound_browser_selected = (!self.sound_browser_files.is_empty()).then_some(0);
                self.status_message = format!("Browsing {}: {count} WAV file(s)", folder.display());
            }
            Err(error) => {
                self.sound_browser_files.clear();
                self.sound_browser_selected = None;
                self.status_message = format!("Could not scan the folder: {error}");
            }
        }
    }

    /// Imports the selected file through the ordinary WAV import path.
    fn import_selected_sound(&mut self) {
        if !self.sound_browser_open {
            return;
        }
        let Some(path) = self
            .sound_browser_selected
            .and_then(|index| self.sound_browser_files.get(index))
            .cloned()
        else {
            self.status_message = "Select a sound to import".to_owned();
            return;
        };
        self.import_audio_path(path);
    }

    pub(super) fn show_sound_browser(&mut self, ctx: &egui::Context) {
        if !self.sound_browser_open {
            return;
        }
        let folder = self.sound_browser_folder.clone();
        let files = self.sound_browser_files.clone();
        let mut selected = self.sound_browser_selected;
        let mut filter = self.sound_browser_filter.clone();
        let mut rescan = false;
        let mut choose = false;
        let mut import = false;
        let import_ready =
            self.project_open && self.project_folder_managed && self.audio_import_task.is_none();
        let mut open = true;
        let response = egui::Window::new("Browse Sounds")
            .id(Id::new("sound-browser"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(600.0, 440.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(560.0);
                ui.label(
                    egui::RichText::new(
                        "Browses WAV files in one folder. Import copies the file into the project and places it on the selected audio track. The preview is the file's format; this window does not audition audio.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.horizontal(|ui| {
                    if ui.button("Choose Folder...").clicked() {
                        choose = true;
                    }
                    ui.label(
                        folder
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| "No folder chosen".to_owned()),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Search");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut filter)
                            .hint_text("File name")
                            .desired_width(200.0),
                    );
                    if response.changed() {
                        rescan = true;
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("sound-browser-list")
                    .max_height(230.0)
                    .show(ui, |ui| {
                        if files.is_empty() {
                            ui.label(
                                egui::RichText::new(if folder.is_some() {
                                    "No WAV files match in this folder."
                                } else {
                                    "Choose a folder to list its WAV files."
                                })
                                .small()
                                .color(theme::palette(ui).muted),
                            );
                        }
                        for (index, path) in files.iter().enumerate() {
                            let label = path
                                .file_stem()
                                .map(|stem| stem.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.display().to_string());
                            if ui
                                .selectable_label(selected == Some(index), label)
                                .on_hover_text(path.display().to_string())
                                .clicked()
                            {
                                selected = Some(index);
                            }
                        }
                    });
                ui.separator();
                let Some(path) = selected.and_then(|index| files.get(index)) else {
                    ui.label("Select a file to see its format.");
                    return;
                };
                ui.label(
                    egui::RichText::new(
                        path.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string()),
                    )
                    .strong(),
                );
                match sound_file_info(path) {
                    Ok(info) => {
                        ui.label(format!(
                            "{:.2} s · {} Hz · {} ch · {} bit · {:.1} MB",
                            info.duration_seconds(),
                            info.sample_rate,
                            info.channels,
                            info.bits_per_sample,
                            std::fs::metadata(path)
                                .map(|metadata| metadata.len() as f64 / (1024.0 * 1024.0))
                                .unwrap_or(0.0),
                        ));
                    }
                    Err(error) => {
                        ui.colored_label(theme::palette(ui).muted, error);
                    }
                }
                if ui
                    .add_enabled(import_ready, egui::Button::new("Import Selected"))
                    .on_disabled_hover_text(
                        "Set up the project folder before importing",
                    )
                    .clicked()
                {
                    import = true;
                }
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("sound-browser"),
            &mut open,
            response.as_ref(),
        );
        self.sound_browser_filter = filter;
        if choose {
            self.choose_sound_folder();
        } else if rescan && let Some(folder) = self.sound_browser_folder.clone() {
            self.rescan_sound_folder(&folder);
        }
        self.sound_browser_selected = selected;
        if import {
            self.import_selected_sound();
        }
        self.sound_browser_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(label: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("opentrack-sounds-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Drums")).expect("drums");
        std::fs::create_dir_all(root.join("Loops")).expect("loops");
        std::fs::write(root.join("Kick.wav"), b"x").expect("kick");
        std::fs::write(root.join("Snare.WAVE"), b"x").expect("snare");
        std::fs::write(root.join("Notes.txt"), b"x").expect("txt");
        std::fs::write(root.join("Drums").join("HiHat.wav"), b"x").expect("hat");
        root
    }

    fn write_wav(path: &Path, frames: u32, rate: u32) {
        let mut bytes = Vec::new();
        let data_bytes = frames * 2;
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_bytes.to_le_bytes());
        bytes.resize(bytes.len() + (frames as usize) * 2, 0);
        std::fs::write(path, bytes).expect("wav");
    }

    #[test]
    fn the_scan_lists_wav_files_sorted_and_filtered() {
        let root = fixture("scan");
        let all = scan_sound_folder(&root, "").expect("scan");
        let names = all
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["HiHat.wav", "Kick.wav", "Snare.WAVE"]);

        let filtered = scan_sound_folder(&root, "kick").expect("scan");
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].ends_with("Kick.wav"));

        // A folder without WAVs is empty, not an error.
        std::fs::create_dir_all(root.join("Empty")).expect("empty");
        assert!(
            scan_sound_folder(&root.join("Empty"), "")
                .expect("scan")
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_selected_file_reports_its_format() {
        let root = fixture("info");
        let path = root.join("Tone.wav");
        write_wav(&path, 48_000, 48_000);
        let info = sound_file_info(&path).expect("info");
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(info.channels, 1);
        assert!((info.duration_seconds() - 1.0).abs() < 1e-6);
        assert!(sound_file_info(&root.join("Kick.wav")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn opening_and_importing_without_a_ready_project_reports() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.open_sound_browser();
        assert!(app.sound_browser_open);
        app.import_selected_sound();
        assert!(app.status_message.contains("Select a sound"));

        // A browsed folder fills the list; selection drives the import path.
        let root = fixture("app");
        app.sound_browser_folder = Some(root.clone());
        app.rescan_sound_folder(&root);
        assert_eq!(app.sound_browser_files.len(), 3);
        assert_eq!(app.sound_browser_selected, Some(0));
        app.import_selected_sound();
        assert!(
            app.status_message.contains("before importing")
                || app.status_message.contains("project")
                || app.status_message.contains("Import"),
            "{}",
            app.status_message
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn importing_the_selected_sound_lands_on_the_selected_track() {
        use std::time::{Duration, Instant};

        let root = fixture("import");
        create_project_directory_layout(&root).expect("layout");
        // The source lives outside the project, so the import has to copy it in.
        let source_root =
            std::env::temp_dir().join(format!("opentrack-sounds-source-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&source_root);
        std::fs::create_dir_all(&source_root).expect("source root");
        let source = source_root.join("Incoming.wav");
        write_wav(&source, 24_000, 48_000);
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.selected_track = Some(0);
        app.sound_browser_open = true;
        app.sound_browser_folder = Some(source_root.clone());
        app.rescan_sound_folder(&source_root);
        let index = app
            .sound_browser_files
            .iter()
            .position(|path| path.ends_with("Incoming.wav"))
            .expect("incoming");
        app.sound_browser_selected = Some(index);

        app.import_selected_sound();
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.audio_import_task.is_some() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
            app.poll_audio_import_task();
        }
        assert!(app.audio_import_task.is_none(), "{}", app.status_message);
        let clips = &app.data.project.tracks[0].clips;
        assert_eq!(clips.len(), 1, "{}", app.status_message);
        assert!(
            clips[0]
                .source_path
                .starts_with(root.join("Audio").join("Imports"))
        );
        assert!(app.data.project.media_pool.contains(&clips[0].source_path));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&source_root);
    }

    #[test]
    fn the_window_lists_files_and_the_selected_format() {
        let ctx = egui::Context::default();
        let root = fixture("window");
        let path = root.join("Tone.wav");
        write_wav(&path, 24_000, 48_000);
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.sound_browser_open = true;
        app.sound_browser_folder = Some(root.clone());
        app.rescan_sound_folder(&root);
        // The fixture files are not valid WAVs; select the valid tone by name.
        let tone = app
            .sound_browser_files
            .iter()
            .position(|candidate| candidate.ends_with("Tone.wav"))
            .expect("tone");
        app.sound_browser_selected = Some(tone);
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 760.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_sound_browser(&ctx),
            );
            output.textures_delta.clear();
            if frame == 0 {
                continue;
            }
            fn walk(shape: &egui::Shape, texts: &mut Vec<String>) {
                match shape {
                    egui::Shape::Text(text) => texts.push(text.galley.text().to_owned()),
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            walk(shape, texts);
                        }
                    }
                    _ => {}
                }
            }
            for shape in &output.shapes {
                walk(&shape.shape, &mut texts);
            }
        }
        let joined = texts.join("\n");
        assert!(joined.contains("Browse Sounds"), "{joined}");
        assert!(joined.contains("Tone"), "{joined}");
        assert!(joined.contains("48000 Hz"), "{joined}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
