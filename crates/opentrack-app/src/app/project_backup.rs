//! Whole-project backup: a self-contained copy of the project folder.
//!
//! The backup is a plain directory copy, so it can be opened by any build and
//! does not depend on the project format. The copy is bounded and refuses to
//! write into the project it is reading, and a failed copy removes the partial
//! destination instead of leaving a half backup behind.

use super::*;

/// Most files one backup may copy.
const MAX_BACKUP_FILES: usize = 20_000;
/// Most bytes one backup may copy (8 GiB).
const MAX_BACKUP_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// Copies `source` into a new directory `target`, returning `(files, bytes)`.
pub(super) fn copy_project_tree(source: &Path, target: &Path) -> Result<(usize, u64), String> {
    if !source.is_dir() {
        return Err("the project folder is not available".to_owned());
    }
    if target.exists() {
        return Err(format!("{} already exists", target.display()));
    }
    if target.starts_with(source) || source.starts_with(target) {
        return Err("choose a backup folder outside the project".to_owned());
    }
    let mut files = 0usize;
    let mut bytes = 0u64;
    let result = copy_directory(source, target, &mut files, &mut bytes);
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(target);
        return Err(error);
    }
    Ok((files, bytes))
}

fn copy_directory(
    source: &Path,
    target: &Path,
    files: &mut usize,
    bytes: &mut u64,
) -> Result<(), String> {
    std::fs::create_dir_all(target)
        .map_err(|error| format!("could not create {}: {error}", target.display()))?;
    let entries = std::fs::read_dir(source)
        .map_err(|error| format!("could not read {}: {error}", source.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("could not read {}: {error}", source.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        // Symlinks are skipped: a backup stays inside the project folder.
        if file_type.is_symlink() {
            continue;
        }
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&path, &destination, files, bytes)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let length = entry
            .metadata()
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?
            .len();
        *files += 1;
        *bytes = bytes.saturating_add(length);
        if *files > MAX_BACKUP_FILES || *bytes > MAX_BACKUP_BYTES {
            return Err(format!(
                "the project is larger than the backup limit ({MAX_BACKUP_FILES} files or 8 GiB)"
            ));
        }
        std::fs::copy(&path, &destination).map_err(|error| {
            format!(
                "could not copy {} to {}: {error}",
                path.display(),
                destination.display()
            )
        })?;
    }
    Ok(())
}

impl OpenTrackApp {
    /// Asks where to write a full copy of the project folder.
    pub(super) fn back_up_project(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project before backing it up".to_owned();
            return;
        }
        if !self.project_folder_managed {
            self.status_message = "Set up this project's folder before backing it up".to_owned();
            return;
        }
        let Some(destination) = rfd::FileDialog::new()
            .set_title("Back up OpenTrack Project")
            .pick_folder()
        else {
            return;
        };
        self.back_up_project_to(&destination);
    }

    /// Copies the managed project folder into `destination`.
    pub(super) fn back_up_project_to(&mut self, destination: &Path) {
        if !self.project_open {
            self.status_message = "Open a project before backing it up".to_owned();
            return;
        }
        if !self.project_folder_managed {
            self.status_message = "Set up this project's folder before backing it up".to_owned();
            return;
        }
        let Some(path) = self.project_path.clone() else {
            self.status_message = "Save the project before backing it up".to_owned();
            return;
        };
        let Some(root) = path.parent() else {
            self.status_message = "The project file has no folder".to_owned();
            return;
        };
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Project".to_owned());
        let target = unique_backup_path(destination, &stem);
        match copy_project_tree(root, &target) {
            Ok((files, bytes)) => {
                self.status_message = format!(
                    "Backed up {files} file(s) ({:.1} MB) to {}",
                    bytes as f64 / (1024.0 * 1024.0),
                    target.display()
                );
            }
            Err(error) => {
                self.status_message = format!("Could not back up the project: {error}");
            }
        }
    }
}

/// `"<stem> backup"`, then `"<stem> backup 2"`, and so on.
fn unique_backup_path(destination: &Path, stem: &str) -> PathBuf {
    let mut target = destination.join(format!("{stem} backup"));
    let mut suffix = 2;
    while target.exists() {
        target = destination.join(format!("{stem} backup {suffix}"));
        suffix += 1;
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(label: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("opentrack-backup-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Audio").join("Imports")).expect("imports");
        std::fs::write(root.join("Song.opr"), b"{\"format\":\"x\"}").expect("project");
        std::fs::write(
            root.join("Audio").join("Imports").join("take.wav"),
            b"RIFFdata",
        )
        .expect("media");
        root
    }

    #[test]
    fn a_backup_copies_the_whole_folder_and_leaves_the_source_alone() {
        let root = fixture("copy");
        let destination = root
            .parent()
            .expect("parent")
            .join(format!("opentrack-backup-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&destination);
        std::fs::create_dir_all(&destination).expect("destination");
        let target = unique_backup_path(&destination, "Song");

        let (files, bytes) = copy_project_tree(&root, &target).expect("copy");
        assert_eq!(files, 2);
        assert!(bytes > 0);
        assert_eq!(
            std::fs::read(target.join("Song.opr")).expect("project copy"),
            b"{\"format\":\"x\"}"
        );
        assert_eq!(
            std::fs::read(target.join("Audio").join("Imports").join("take.wav"))
                .expect("media copy"),
            b"RIFFdata"
        );
        // The source still holds its own files and the next backup takes a new name.
        assert!(root.join("Song.opr").exists());
        std::fs::create_dir_all(target.join("marker")).expect("taken");
        let second = unique_backup_path(&destination, "Song");
        assert!(second.ends_with("Song backup 2"));

        // Refusals: an existing target and a destination inside the source.
        assert!(copy_project_tree(&root, &target).is_err());
        assert!(copy_project_tree(&root, &root.join("inside")).is_err());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&destination);
    }

    #[test]
    fn the_command_backs_up_a_managed_project() {
        let root = fixture("command");
        create_project_directory_layout(&root).expect("layout");
        let path = root.join("Song.opr");
        let dock = egui_dock::DockState::new(vec![PanelTab::Project]);
        let workspace = crate::project_file::initial_workspace_state(&dock).expect("workspace");
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let mut document =
            crate::project_file::ProjectFile::new(project, workspace, dock, true, 0, false);
        document.project_folder_layout = 1;
        document.write(&path).expect("write project");

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(path);
        let destination = root
            .parent()
            .expect("parent")
            .join(format!("opentrack-backup-dest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&destination);
        std::fs::create_dir_all(&destination).expect("destination");

        app.back_up_project_to(&destination);
        assert!(
            app.status_message.contains("Backed up"),
            "{}",
            app.status_message
        );
        let backup = destination.join("Song backup");
        assert!(backup.join("Song.opr").exists());
        assert!(backup.join("Audio").join("Recordings").is_dir());

        // A second backup takes the next name instead of overwriting.
        app.back_up_project_to(&destination);
        assert!(destination.join("Song backup 2").join("Song.opr").exists());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&destination);
    }
}
