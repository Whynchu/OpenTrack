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
/// Recovery copies kept per project, newest first.
///
/// More than one point lets the user go back past a bad edit that was itself
/// autosaved; five is enough for that without turning the folder into a log.
pub(super) const MAX_RECOVERY_POINTS: usize = 5;
const RECOVERY_PREFIX: &str = "recovery-";

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

/// Creates the recovery folder and returns the path for one more point.
pub(super) fn prepare_point(
    project_root: &Path,
    written_at_unix_ms: u64,
) -> Result<PathBuf, String> {
    let directory = recovery_directory(project_root);
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    Ok(directory.join(format!("{RECOVERY_PREFIX}{written_at_unix_ms:013}.opr")))
}

/// Drops the oldest recovery points once more than [`MAX_RECOVERY_POINTS`] exist.
pub(super) fn prune(project_root: &Path) -> usize {
    let mut points = all_recovery_files(project_root);
    let mut removed = 0;
    while points.len() > MAX_RECOVERY_POINTS {
        let oldest = points.pop().expect("more than one point");
        if std::fs::remove_file(&oldest.path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Every recovery file on disk, newest first, without the kept-count limit.
fn all_recovery_files(project_root: &Path) -> Vec<PendingAutosave> {
    let directory = recovery_directory(project_root);
    let mut points = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !(name.starts_with(RECOVERY_PREFIX) || name == AUTOSAVE_FILE) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.len() == 0 {
                continue;
            }
            points.push(PendingAutosave {
                path,
                written: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                bytes: metadata.len(),
            });
        }
    }
    points.sort_by(|left, right| right.written.cmp(&left.written));
    points
}

/// The newest [`MAX_RECOVERY_POINTS`] recovery points, newest first.
fn recovery_points(project_root: &Path) -> Vec<PendingAutosave> {
    let mut points = all_recovery_files(project_root);
    points.truncate(MAX_RECOVERY_POINTS);
    points
}

/// Every recovery point, newest first, with the legacy single autosave included.
/// Every recovery point a project has, newest first.
pub(super) fn pending_points(project_root: &Path) -> Vec<PendingAutosave> {
    recovery_points(project_root)
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
    recovery_points(project_root).into_iter().next()
}

/// Removes the recovery copy after it was applied or declined.
pub(super) fn clear(project_root: &Path) -> Result<(), String> {
    let mut failure = None;
    let mut paths = recovery_points(project_root)
        .into_iter()
        .map(|point| point.path)
        .collect::<Vec<_>>();
    // `recovery_points` truncates to the kept count, so collect the leftovers too.
    if let Ok(entries) = std::fs::read_dir(recovery_directory(project_root)) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if (name.starts_with(RECOVERY_PREFIX) || name == AUTOSAVE_FILE)
                && !paths.contains(&path)
            {
                paths.push(path);
            }
        }
    }
    for path in paths {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => failure = Some(format!("could not remove {}: {error}", path.display())),
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
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

    #[test]
    fn several_recovery_points_are_kept_newest_first_and_pruned() {
        let root = temp_root("points");
        let directory = recovery_directory(&root);
        std::fs::create_dir_all(&directory).expect("dir");
        // Write more points than the limit, one per millisecond.
        for index in 0..(MAX_RECOVERY_POINTS + 3) {
            let path = prepare_point(&root, 1_000 + index as u64).expect("path");
            std::fs::write(&path, b"{}").expect("write");
        }
        // The listing never shows more than the kept count, even before pruning.
        let points = pending_points(&root);
        assert_eq!(points.len(), MAX_RECOVERY_POINTS);
        assert!(
            points[0].path > points[1].path,
            "the newest point sorts first by name"
        );
        assert_eq!(
            std::fs::read_dir(&directory).expect("dir").count(),
            MAX_RECOVERY_POINTS + 3,
            "every point is on disk before pruning"
        );

        let removed = prune(&root);
        assert_eq!(removed, 3);
        assert_eq!(
            std::fs::read_dir(&directory).expect("dir").count(),
            MAX_RECOVERY_POINTS,
            "pruning deletes the oldest points from disk"
        );
        let points = pending_points(&root);
        assert_eq!(points.len(), MAX_RECOVERY_POINTS);
        assert!(
            points
                .windows(2)
                .all(|pair| pair[0].written >= pair[1].written),
            "kept points are ordered newest first"
        );

        // Clearing removes every point, not just the newest.
        clear(&root).expect("clear");
        assert!(pending_points(&root).is_empty());
        assert_eq!(
            std::fs::read_dir(&directory).expect("dir").count(),
            0,
            "no recovery file is left behind"
        );
    }

    #[test]
    fn an_older_single_autosave_is_still_offered_as_a_point() {
        let root = temp_root("legacy-point");
        let path = prepare(&root).expect("prepare");
        std::fs::write(&path, b"{}").expect("write");
        let points = pending_points(&root);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].path, path);
        clear(&root).expect("clear");
        assert!(pending_points(&root).is_empty());
    }
}
