//! Recovery autosave for unsaved project changes.
//!
//! eframe cannot reliably intercept a window close, so unsaved work can be lost
//! without any prompt. This module keeps a recovery copy of the project inside
//! the project's own folder while it has unsaved changes, and reports it at the
//! next open so the user decides whether to take it back or drop it.
//!
//! The copy is a normal project document written through the same validated,
//! atomic path as a save, so a half-written autosave can never replace a good
//! one and it stays readable by [`crate::project_file::ProjectFile::read`].

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

pub(super) const AUTOSAVE_FILE: &str = "autosave.opr";

pub(super) fn recovery_directory(project_root: &Path) -> PathBuf {
    project_root.join("Metadata").join("Recovery")
}

pub(super) fn autosave_path(project_root: &Path) -> PathBuf {
    recovery_directory(project_root).join(AUTOSAVE_FILE)
}

/// Creates the recovery folder so a save can stage its temporary file there.
pub(super) fn prepare(project_root: &Path) -> Result<PathBuf, String> {
    let directory = recovery_directory(project_root);
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    Ok(autosave_path(project_root))
}

/// An autosave that has not been reconciled with the project file yet.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PendingAutosave {
    pub path: PathBuf,
    pub written: SystemTime,
    pub bytes: u64,
}

/// Reports an autosave left by an earlier session.
///
/// The project file itself is never compared here: an autosave only exists while
/// the project had unsaved changes, and it is removed on save, so its presence
/// always means the earlier session did not finish saving.
pub(super) fn pending(project_root: &Path) -> Option<PendingAutosave> {
    let path = autosave_path(project_root);
    let metadata = std::fs::metadata(&path).ok()?;
    if metadata.len() == 0 {
        return None;
    }
    let written = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    Some(PendingAutosave {
        path,
        written,
        bytes: metadata.len(),
    })
}

/// Removes the recovery copy after it was applied or declined.
pub(super) fn clear(project_root: &Path) -> Result<(), String> {
    let path = autosave_path(project_root);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not remove {}: {error}", path.display())),
    }
}

/// "12 minutes ago"-style description of when the autosave was written.
pub(super) fn describe_age(written: SystemTime) -> String {
    let elapsed = SystemTime::now()
        .duration_since(written)
        .unwrap_or_default()
        .as_secs();
    match elapsed {
        0..=59 => "less than a minute ago".to_owned(),
        60..=119 => "about a minute ago".to_owned(),
        120..=3599 => format!("about {} minutes ago", elapsed / 60),
        3600..=7199 => "about an hour ago".to_owned(),
        7200..=86_399 => format!("about {} hours ago", elapsed / 3600),
        _ => format!("about {} days ago", elapsed / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("opentrack-autosave-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Metadata")).expect("metadata");
        root
    }

    #[test]
    fn autosave_lives_in_the_projects_own_recovery_folder() {
        let root = temp_root("path");
        let path = autosave_path(&root);
        assert_eq!(
            path,
            root.join("Metadata").join("Recovery").join("autosave.opr")
        );
        assert_eq!(prepare(&root).expect("prepare"), path);
        assert!(path.parent().expect("parent").is_dir());
    }

    #[test]
    fn pending_ignores_missing_and_empty_autosaves() {
        let root = temp_root("pending");
        assert!(pending(&root).is_none(), "no autosave yet");

        let path = prepare(&root).expect("prepare");
        std::fs::write(&path, b"").expect("empty");
        assert!(pending(&root).is_none(), "an empty file holds no project");

        std::fs::write(&path, b"{}").expect("write");
        let found = pending(&root).expect("pending");
        assert_eq!(found.path, path);
        assert_eq!(found.bytes, 2);

        clear(&root).expect("clear");
        assert!(pending(&root).is_none());
        clear(&root).expect("clearing a missing autosave is not an error");
    }

    #[test]
    fn age_description_is_readable_at_every_scale() {
        let now = SystemTime::now();
        assert_eq!(describe_age(now), "less than a minute ago");
        assert_eq!(
            describe_age(now - std::time::Duration::from_secs(90)),
            "about a minute ago"
        );
        assert_eq!(
            describe_age(now - std::time::Duration::from_secs(600)),
            "about 10 minutes ago"
        );
        assert_eq!(
            describe_age(now - std::time::Duration::from_secs(7200)),
            "about 2 hours ago"
        );
        assert_eq!(
            describe_age(now - std::time::Duration::from_secs(3 * 86_400)),
            "about 3 days ago"
        );
    }
}
