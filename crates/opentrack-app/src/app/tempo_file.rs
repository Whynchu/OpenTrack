//! Tempo Track exchange: a small, documented file for tempo and time signature.
//!
//! This is OpenTrack's own format. It carries only what the project model holds -
//! one tempo and one time signature - and says so in the file, so a future
//! multi-tempo map is a new version rather than a silent reinterpretation.

use super::*;
use serde::{Deserialize, Serialize};

pub(super) const TEMPO_FILE_FORMAT: &str = "opentrack-tempo";
pub(super) const TEMPO_FILE_VERSION: u32 = 1;

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct TempoFile {
    pub format: String,
    pub version: u32,
    pub tempo_bpm: f64,
    pub beats_per_bar: u8,
    pub beat_unit: u8,
}

impl TempoFile {
    pub(super) fn from_project(project: &Project) -> Self {
        Self {
            format: TEMPO_FILE_FORMAT.to_owned(),
            version: TEMPO_FILE_VERSION,
            tempo_bpm: project.tempo_bpm,
            beats_per_bar: project.beats_per_bar,
            beat_unit: project.beat_unit,
        }
    }

    /// Serializes the project's tempo and signature as documented JSON.
    pub(super) fn to_text(project: &Project) -> String {
        serde_json::to_string_pretty(&Self::from_project(project))
            .unwrap_or_else(|_| String::from("{}"))
    }

    /// Parses and validates exchanged tempo data.
    ///
    /// The values are checked with the same contract the editor uses, so an
    /// imported file cannot put a project into a state the editor would refuse.
    pub(super) fn from_text(text: &str) -> Result<Self, String> {
        let file: Self = serde_json::from_str(text)
            .map_err(|error| format!("this is not a tempo file: {error}"))?;
        if file.format != TEMPO_FILE_FORMAT {
            return Err(format!(
                "unexpected tempo file format {} (expected {TEMPO_FILE_FORMAT})",
                file.format
            ));
        }
        if file.version != TEMPO_FILE_VERSION {
            return Err(format!(
                "unsupported tempo file version {} (this build reads version {TEMPO_FILE_VERSION})",
                file.version
            ));
        }
        let mut probe = Project::default();
        probe
            .set_tempo(file.tempo_bpm)
            .map_err(|message| message.to_owned())?;
        probe
            .set_time_signature(file.beats_per_bar, file.beat_unit)
            .map_err(|message| message.to_owned())?;
        Ok(file)
    }
}

impl OpenTrackApp {
    pub(super) fn export_tempo_file(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to export its tempo".to_owned();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenTrack tempo", &["json"])
            .set_file_name(format!("{} tempo.json", self.data.project.name))
            .set_title("Export Tempo Track")
            .save_file()
        else {
            return;
        };
        self.export_tempo_to(&path);
    }

    pub(super) fn export_tempo_to(&mut self, path: &Path) {
        let text = TempoFile::to_text(&self.data.project);
        match std::fs::write(path, text) {
            Ok(()) => {
                self.status_message = format!(
                    "Exported tempo {:.3} BPM and {}/{} to {}",
                    self.data.project.tempo_bpm,
                    self.data.project.beats_per_bar,
                    self.data.project.beat_unit,
                    path.display()
                )
            }
            Err(error) => {
                self.status_message = format!("Could not export the tempo: {error}");
            }
        }
    }

    pub(super) fn import_tempo_file(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to import a tempo".to_owned();
            return;
        }
        if self.is_recording() {
            self.status_message = "Stop recording before importing a tempo".to_owned();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenTrack tempo", &["json"])
            .set_title("Import Tempo Track")
            .pick_file()
        else {
            return;
        };
        self.import_tempo_from(&path);
    }

    pub(super) fn import_tempo_from(&mut self, path: &Path) {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                self.status_message = format!("Could not read the tempo file: {error}");
                return;
            }
        };
        let file = match TempoFile::from_text(&text) {
            Ok(file) => file,
            Err(message) => {
                self.status_message = format!("Tempo import failed: {message}");
                return;
            }
        };
        let draft = crate::app::tempo_editor::Draft {
            bpm: file.tempo_bpm,
            beats_per_bar: file.beats_per_bar,
            beat_unit: file.beat_unit,
        };
        self.apply_tempo_editor(draft);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app
    }

    #[test]
    fn tempo_files_round_trip_and_refuse_foreign_or_invalid_data() {
        let mut project = Project::default();
        project.set_tempo(96.5).expect("tempo");
        project.set_time_signature(7, 8).expect("signature");
        let text = TempoFile::to_text(&project);
        assert!(text.contains("\"format\": \"opentrack-tempo\""));
        assert!(text.contains("\"version\": 1"));
        let parsed = TempoFile::from_text(&text).expect("parse");
        assert_eq!(parsed.tempo_bpm, 96.5);
        assert_eq!((parsed.beats_per_bar, parsed.beat_unit), (7, 8));

        // Foreign files, unknown versions and invalid values are refused.
        assert!(TempoFile::from_text("{").is_err());
        assert!(TempoFile::from_text(
            r#"{"format":"cubase-tempo","version":1,"tempo_bpm":120,"beats_per_bar":4,"beat_unit":4}"#
        )
        .is_err());
        assert!(TempoFile::from_text(
            r#"{"format":"opentrack-tempo","version":2,"tempo_bpm":120,"beats_per_bar":4,"beat_unit":4}"#
        )
        .is_err());
        assert!(TempoFile::from_text(
            r#"{"format":"opentrack-tempo","version":1,"tempo_bpm":0,"beats_per_bar":4,"beat_unit":4}"#
        )
        .is_err());
        assert!(TempoFile::from_text(
            r#"{"format":"opentrack-tempo","version":1,"tempo_bpm":120,"beats_per_bar":4,"beat_unit":3}"#
        )
        .is_err());
    }

    #[test]
    fn exporting_and_importing_reaches_the_project_as_one_undo_step() {
        let root =
            std::env::temp_dir().join(format!("opentrack-tempo-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let path = root.join("tempo.json");

        let mut app = app();
        app.data.project.set_tempo(140.0).expect("tempo");
        app.data
            .project
            .set_time_signature(3, 4)
            .expect("signature");
        app.project_history.clear();
        app.export_tempo_to(&path);
        assert!(path.exists());
        assert!(
            app.status_message
                .contains("Exported tempo 140.000 BPM and 3/4")
        );

        // Change the project, then import the file back.
        app.data.project.set_tempo(90.0).expect("tempo");
        app.data
            .project
            .set_time_signature(4, 4)
            .expect("signature");
        app.project_history.clear();
        app.import_tempo_from(&path);
        assert_eq!(app.data.project.tempo_bpm, 140.0);
        assert_eq!(
            (app.data.project.beats_per_bar, app.data.project.beat_unit),
            (3, 4)
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Tempo and Signature");

        // A malformed file reports and changes nothing.
        let bad = root.join("bad.json");
        std::fs::write(&bad, b"{\"format\":\"other\"}").expect("bad");
        let before = app.data.project.clone();
        app.import_tempo_from(&bad);
        assert!(app.status_message.contains("Tempo import failed"));
        assert_eq!(app.data.project, before);
        let _ = std::fs::remove_dir_all(root);
    }
}
