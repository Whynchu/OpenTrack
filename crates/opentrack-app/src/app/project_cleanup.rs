//! Reversible project-folder cleanup.
//!
//! Cleanup moves files under the project's `Audio` folder that no pool entry or
//! clip refers to into `Metadata/Cleanup Trash/<date>/`, keeping each file's
//! relative path. Nothing is deleted, so a wrong guess can be moved back by
//! hand, and metadata, presets and the project file are never touched.

use super::*;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

/// Media folder scanned by cleanup.
const AUDIO_FOLDER: &str = "Audio";
/// Where unreferenced files are moved.
const TRASH_FOLDER: &str = "Cleanup Trash";

/// Files a cleanup would move.
#[derive(Clone, Debug, Default)]
pub(super) struct CleanupPlan {
    /// `(absolute path, bytes)` for each unreferenced file, sorted by path.
    pub files: Vec<(PathBuf, u64)>,
    pub bytes: u64,
    /// Date label used for the trash folder.
    pub label: String,
}

impl CleanupPlan {
    pub(super) fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// What a completed cleanup did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CleanupReport {
    pub moved: usize,
    pub bytes: u64,
    pub trash: PathBuf,
}

/// The canonical form used to compare pool paths with files on disk.
pub(super) fn canonical_or_self(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Every source the project refers to: the media pool plus every clip path.
pub(super) fn referenced_media(project: &Project) -> HashSet<PathBuf> {
    let mut referenced = HashSet::new();
    for path in &project.media_pool {
        referenced.insert(canonical_or_self(path));
    }
    for track in project
        .tracks
        .iter()
        .chain(std::iter::once(&project.output_channel.strip))
    {
        for clip in &track.clips {
            referenced.insert(canonical_or_self(&clip.source_path));
        }
    }
    referenced
}

/// Scans `<root>/Audio` for files no pool entry or clip refers to.
///
/// Returns an empty plan when the folder does not exist, so a fresh project is
/// never an error.
pub(super) fn scan_project_audio(
    root: &Path,
    referenced: &HashSet<PathBuf>,
    label: &str,
) -> Result<CleanupPlan, String> {
    let audio_root = root.join(AUDIO_FOLDER);
    let mut plan = CleanupPlan {
        files: Vec::new(),
        bytes: 0,
        label: label.to_owned(),
    };
    if !audio_root.is_dir() {
        return Ok(plan);
    }
    collect_unreferenced(&audio_root, referenced, &mut plan)?;
    plan.files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(plan)
}

fn collect_unreferenced(
    directory: &Path,
    referenced: &HashSet<PathBuf>,
    plan: &mut CleanupPlan,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("could not read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("could not read {}: {error}", directory.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_unreferenced(&path, referenced, plan)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        if referenced.contains(&canonical_or_self(&path)) {
            continue;
        }
        let bytes = entry
            .metadata()
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?
            .len();
        plan.bytes = plan.bytes.saturating_add(bytes);
        plan.files.push((path, bytes));
    }
    Ok(())
}

/// Moves the planned files into `<root>/Metadata/Cleanup Trash/<label>/`.
///
/// Each file keeps its path relative to the project root, so restoring one is a
/// plain move back. A failed move stops the pass and names what already moved.
pub(super) fn execute_cleanup(root: &Path, plan: &CleanupPlan) -> Result<CleanupReport, String> {
    let trash = unique_trash_folder(root, &plan.label);
    std::fs::create_dir_all(&trash)
        .map_err(|error| format!("could not create {}: {error}", trash.display()))?;
    let mut moved = 0usize;
    let mut bytes = 0u64;
    for (path, length) in &plan.files {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let destination = trash.join(relative);
        if let Some(parent) = destination.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            return Err(format!(
                "moved {moved} file(s), then could not create {}: {error}",
                parent.display()
            ));
        }
        if let Err(error) = std::fs::rename(path, &destination) {
            return Err(format!(
                "moved {moved} file(s), then could not move {}: {error}",
                path.display()
            ));
        }
        moved += 1;
        bytes = bytes.saturating_add(*length);
    }
    Ok(CleanupReport {
        moved,
        bytes,
        trash,
    })
}

/// `<root>/Metadata/Cleanup Trash/<label>`, then `… <label> 2`, and so on.
fn unique_trash_folder(root: &Path, label: &str) -> PathBuf {
    let base = root.join("Metadata").join(TRASH_FOLDER);
    let mut target = base.join(label);
    let mut suffix = 2;
    while target.exists() {
        target = base.join(format!("{label} {suffix}"));
        suffix += 1;
    }
    target
}

/// `YYYY-MM-DD` for today, from the system clock.
pub(super) fn today_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    date_label(seconds)
}

/// `YYYY-MM-DD` for a UNIX timestamp in seconds.
pub(super) fn date_label(unix_seconds: u64) -> String {
    let (year, month, day) = civil_date_from_days((unix_seconds / 86_400) as i64);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Howard Hinnant's civil-from-days conversion, valid for positive years.
fn civil_date_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

impl OpenTrackApp {
    /// Scans the project folder and asks before moving unreferenced media.
    pub(super) fn request_project_cleanup(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project before cleaning its folder".to_owned();
            return;
        }
        if !self.project_folder_managed {
            self.status_message = "Set up this project's folder before cleaning it".to_owned();
            return;
        }
        let Some(path) = self.project_path.clone() else {
            self.status_message = "Save the project before cleaning its folder".to_owned();
            return;
        };
        let Some(root) = path.parent() else {
            self.status_message = "The project file has no folder".to_owned();
            return;
        };
        let referenced = referenced_media(&self.data.project);
        let label = today_label();
        let plan = match scan_project_audio(root, &referenced, &label) {
            Ok(plan) => plan,
            Err(error) => {
                self.status_message = format!("Could not scan the project folder: {error}");
                return;
            }
        };
        if plan.is_empty() {
            self.status_message = "No unreferenced files in the project's Audio folder".to_owned();
            return;
        }
        self.status_message = format!(
            "Cleanup ready: {} unreferenced file(s), {:.1} MB",
            plan.files.len(),
            plan.bytes as f64 / (1024.0 * 1024.0)
        );
        self.cleanup_prompt = Some(plan);
    }

    /// Moves the planned files and reports where they went.
    pub(super) fn execute_project_cleanup(&mut self) {
        let Some(plan) = self.cleanup_prompt.take() else {
            return;
        };
        let Some(root) = self
            .project_path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
        else {
            self.status_message = "Save the project before cleaning its folder".to_owned();
            return;
        };
        match execute_cleanup(&root, &plan) {
            Ok(report) => {
                self.status_message = format!(
                    "Moved {} unreferenced file(s) ({:.1} MB) to {}; move them back from there to undo",
                    report.moved,
                    report.bytes as f64 / (1024.0 * 1024.0),
                    report.trash.display()
                );
            }
            Err(error) => {
                self.status_message = format!("Cleanup stopped: {error}");
            }
        }
    }

    pub(super) fn show_cleanup_prompt(&mut self, ctx: &egui::Context) {
        let Some(plan) = self.cleanup_prompt.as_ref() else {
            return;
        };
        let mut clean = false;
        let mut cancel = false;
        egui::Window::new("Clean Up Project Folder")
            .id(Id::new("cleanup-prompt"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_width(400.0);
                ui.label(format!(
                    "Move {} unreferenced file(s) ({:.1} MB) from the project's Audio folder to Metadata/Cleanup Trash/{}?",
                    plan.files.len(),
                    plan.bytes as f64 / (1024.0 * 1024.0),
                    plan.label
                ));
                ui.label(
                    egui::RichText::new(
                        "Nothing is deleted: each file keeps its relative path under the trash folder, so it can be moved back.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.horizontal(|ui| {
                    if ui.button("Clean Up").clicked() {
                        clean = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if clean {
            self.execute_project_cleanup();
        } else if cancel {
            self.cleanup_prompt = None;
            self.status_message = "Cleanup cancelled".to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(label: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("opentrack-cleanup-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Audio").join("Imports")).expect("imports");
        std::fs::create_dir_all(root.join("Audio").join("Recordings")).expect("recordings");
        std::fs::create_dir_all(root.join("Metadata")).expect("metadata");
        std::fs::write(root.join("Song.opr"), b"{}").expect("project");
        std::fs::write(root.join("Metadata").join("state.json"), b"keep").expect("state");
        std::fs::write(root.join("Audio").join("Imports").join("used.wav"), b"used").expect("used");
        std::fs::write(
            root.join("Audio")
                .join("Recordings")
                .join("unused take.wav"),
            b"stale",
        )
        .expect("unused");
        root
    }

    fn project_with_used(root: &Path) -> Project {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(
                0,
                root.join("Audio").join("Imports").join("used.wav"),
                0.0,
                1.0,
            )
            .expect("clip");
        project
    }

    #[test]
    fn cleanup_moves_only_unreferenced_audio_and_can_be_moved_back() {
        let root = fixture("scan");
        let project = project_with_used(&root);
        let referenced = referenced_media(&project);
        let plan = scan_project_audio(&root, &referenced, "2026-11-02").expect("scan");
        assert_eq!(plan.files.len(), 1, "{:?}", plan.files);
        assert!(plan.files[0].0.ends_with("unused take.wav"));
        assert_eq!(plan.bytes, 5);

        let report = execute_cleanup(&root, &plan).expect("cleanup");
        assert_eq!(report.moved, 1);
        assert!(
            report.trash.ends_with(
                Path::new("Metadata")
                    .join("Cleanup Trash")
                    .join("2026-11-02")
            )
        );
        let moved = report
            .trash
            .join("Audio")
            .join("Recordings")
            .join("unused take.wav");
        assert!(moved.exists(), "the stale file keeps its relative path");
        assert!(
            !root
                .join("Audio")
                .join("Recordings")
                .join("unused take.wav")
                .exists()
        );

        // Referenced media and metadata are untouched.
        assert!(root.join("Audio").join("Imports").join("used.wav").exists());
        assert!(root.join("Song.opr").exists());
        assert!(root.join("Metadata").join("state.json").exists());

        // Moving the file back restores the project folder exactly.
        std::fs::rename(
            &moved,
            root.join("Audio")
                .join("Recordings")
                .join("unused take.wav"),
        )
        .expect("restore");
        assert!(
            scan_project_audio(&root, &referenced, "2026-11-02")
                .expect("rescan")
                .files
                .iter()
                .any(|(path, _)| path.ends_with("unused take.wav"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_second_cleanup_uses_a_new_trash_folder_and_dates_are_civil() {
        let root = fixture("again");
        let referenced = HashSet::new();
        let plan = scan_project_audio(&root, &referenced, "2026-11-02").expect("scan");
        assert_eq!(plan.files.len(), 2, "every audio file is unreferenced here");
        let first = execute_cleanup(&root, &plan).expect("first");
        assert!(first.trash.ends_with("2026-11-02"));

        std::fs::write(root.join("Audio").join("Imports").join("new.wav"), b"x").expect("new");
        let plan = scan_project_audio(&root, &referenced, "2026-11-02").expect("rescan");
        assert_eq!(plan.files.len(), 1);
        let second = execute_cleanup(&root, &plan).expect("second");
        assert!(second.trash.ends_with("2026-11-02 2"));
        assert!(first.trash.exists() && second.trash.exists());

        assert_eq!(date_label(0), "1970-01-01");
        assert_eq!(date_label(86_400), "1970-01-02");
        assert_eq!(date_label(1_790_000_000), "2026-09-21");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_command_refuses_an_unsaved_or_unmanaged_project() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.request_project_cleanup();
        assert!(app.status_message.contains("Open a project"));

        app.project_open = true;
        app.request_project_cleanup();
        assert!(app.status_message.contains("Set up this project's folder"));

        app.project_folder_managed = true;
        app.request_project_cleanup();
        assert!(app.status_message.contains("Save the project"));
        assert!(app.cleanup_prompt.is_none());
    }

    #[test]
    fn the_command_plans_then_moves_on_confirmation() {
        let root = fixture("command");
        let path = root.join("Song.opr");
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(path);
        app.data.project = project_with_used(&root);

        app.request_project_cleanup();
        assert!(app.cleanup_prompt.is_some(), "{}", app.status_message);
        assert!(app.status_message.contains("Cleanup ready: 1"));

        app.execute_project_cleanup();
        assert!(app.cleanup_prompt.is_none());
        assert!(
            app.status_message.contains("Moved 1 unreferenced file(s)"),
            "{}",
            app.status_message
        );
        assert!(root.join("Audio").join("Imports").join("used.wav").exists());
        assert!(
            !root
                .join("Audio")
                .join("Recordings")
                .join("unused take.wav")
                .exists()
        );

        // A second pass finds nothing.
        app.request_project_cleanup();
        assert!(app.status_message.contains("No unreferenced files"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
