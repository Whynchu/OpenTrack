//! Recovering takes that an abnormal exit left behind.
//!
//! A recording writes audio straight to disk, but only finalizes the WAV header
//! and adds the clip when the take ends. If OpenTrack exits while recording, the
//! samples survive but nothing in the project points at them.
//!
//! The journal below closes that gap. It is written before the recorder creates
//! its files and removed only after the takes are in the project, so its
//! presence at load time means a recording never completed. Each listed take is
//! then either restored (header repaired, clip placed) or discarded by the user,
//! with the decision recorded on disk.

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

const JOURNAL_VERSION: u32 = 1;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

/// One take that was being written when OpenTrack stopped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct JournalTake {
    /// Absolute when the project has no folder to be relative to.
    pub path: PathBuf,
    pub track_id: u64,
    pub track_name: String,
    pub start_beats: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Journal {
    version: u32,
    takes: Vec<JournalTake>,
}

/// A take the user can still act on: the file exists and the project does not
/// already reference it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PendingTake {
    pub take: JournalTake,
    pub file_name: String,
    /// Size on disk, so the dialog can say how much audio is at stake.
    pub bytes: u64,
}

pub(super) fn journal_path(project_root: &Path) -> PathBuf {
    project_root
        .join("Metadata")
        .join("Recording")
        .join("in-progress.json")
}

fn relative_to_root(project_root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(project_root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Records the takes a recording is about to write.
///
/// This runs before the recorder creates its files, so a crash between file
/// creation and the first sample still leaves a recoverable entry.
pub(super) fn begin(
    project_root: &Path,
    takes: &[(PathBuf, u64, String, f64)],
) -> Result<(), String> {
    if takes.is_empty() {
        return clear(project_root);
    }
    let journal = Journal {
        version: JOURNAL_VERSION,
        takes: takes
            .iter()
            .map(|(path, track_id, track_name, start_beats)| JournalTake {
                path: relative_to_root(project_root, path),
                track_id: *track_id,
                track_name: track_name.clone(),
                start_beats: *start_beats,
            })
            .collect(),
    };
    write_journal(project_root, &journal)
}

fn write_journal(project_root: &Path, journal: &Journal) -> Result<(), String> {
    let path = journal_path(project_root);
    let parent = path
        .parent()
        .ok_or_else(|| "the recording journal path has no parent".to_owned())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let encoded = serde_json::to_vec_pretty(journal)
        .map_err(|error| format!("could not encode the recording journal: {error}"))?;
    // A crash while writing the journal must not destroy the previous one, so
    // the replacement is staged and renamed.
    let temp = parent.join(format!(
        "in-progress-{}-{}.json",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&temp, encoded)
        .map_err(|error| format!("could not write {}: {error}", temp.display()))?;
    std::fs::rename(&temp, &path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        format!("could not replace {}: {error}", path.display())
    })
}

/// Removes the journal once its takes belong to the project.
pub(super) fn clear(project_root: &Path) -> Result<(), String> {
    let path = journal_path(project_root);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not remove {}: {error}", path.display())),
    }
}

fn read_journal(project_root: &Path) -> Result<Option<Journal>, String> {
    let path = journal_path(project_root);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("could not read {}: {error}", path.display())),
    };
    let journal: Journal = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "{} is not a readable recording journal: {error}",
            path.display()
        )
    })?;
    if journal.version != JOURNAL_VERSION {
        return Err(format!(
            "{} was written by a newer OpenTrack (version {})",
            path.display(),
            journal.version
        ));
    }
    Ok(Some(journal))
}

fn resolve(project_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

/// Lists the journaled takes that are still worth a decision.
///
/// A take that the project already references is not pending: it was committed
/// and the journal is simply stale. A take whose file is gone cannot be
/// recovered and is dropped from the journal by [`settle`].
pub(super) fn pending(
    project_root: &Path,
    project: &opentrack_core::project::Project,
) -> Result<Vec<PendingTake>, String> {
    let Some(journal) = read_journal(project_root)? else {
        return Ok(Vec::new());
    };
    let referenced = project.media_paths();
    Ok(journal
        .takes
        .into_iter()
        .filter_map(|take| {
            let absolute = resolve(project_root, &take.path);
            if referenced.contains(&absolute) {
                return None;
            }
            let bytes = std::fs::metadata(&absolute).ok()?.len();
            if bytes == 0 {
                return None;
            }
            Some(PendingTake {
                file_name: absolute
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| take.path.display().to_string()),
                take: JournalTake {
                    path: absolute,
                    ..take
                },
                bytes,
            })
        })
        .collect())
}

/// Drops one take from the journal after the user decided about it.
pub(super) fn settle(project_root: &Path, decided: &Path) -> Result<(), String> {
    let Some(mut journal) = read_journal(project_root)? else {
        return Ok(());
    };
    journal
        .takes
        .retain(|take| resolve(project_root, &take.path) != decided);
    if journal.takes.is_empty() {
        clear(project_root)
    } else {
        write_journal(project_root, &journal)
    }
}

/// Moves a discarded take out of the recording folder instead of destroying it.
///
/// The audio may be the only copy of a performance, so discarding keeps the
/// file under `Metadata/Recording/Discarded` where it can be recovered by hand.
pub(super) fn discard_file(project_root: &Path, path: &Path) -> Result<PathBuf, String> {
    let directory = project_root
        .join("Metadata")
        .join("Recording")
        .join("Discarded");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let name = path
        .file_name()
        .map(|name| name.to_os_string())
        .ok_or_else(|| "the take has no file name".to_owned())?;
    let mut destination = directory.join(&name);
    let mut suffix = 1_u32;
    while destination.exists() {
        let stem = Path::new(&name)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "take".to_owned());
        let extension = Path::new(&name)
            .extension()
            .map(|extension| format!(".{}", extension.to_string_lossy()))
            .unwrap_or_default();
        destination = directory.join(format!("{stem} ({suffix}){extension}"));
        suffix += 1;
    }
    match std::fs::rename(path, &destination) {
        Ok(()) => Ok(destination),
        Err(error) => Err(format!(
            "could not move {} out of the recording folder: {error}",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::project::{Project, TrackKind};

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("opentrack-journal-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Audio").join("Recordings")).expect("recordings dir");
        std::fs::create_dir_all(root.join("Metadata")).expect("metadata dir");
        root
    }

    fn take(root: &Path, name: &str, bytes: usize) -> PathBuf {
        let path = root.join("Audio").join("Recordings").join(name);
        std::fs::write(&path, vec![7_u8; bytes]).expect("write take");
        path
    }

    #[test]
    fn journal_round_trips_relative_paths_and_is_cleared_on_success() {
        let root = temp_root("round-trip");
        let path = take(&root, "Take 001.wav", 64);
        begin(&root, &[(path.clone(), 4, "Audio 1".to_owned(), 8.0)]).expect("begin");

        let journal = read_journal(&root).expect("read").expect("present");
        assert_eq!(journal.takes.len(), 1);
        assert_eq!(journal.takes[0].track_id, 4);
        assert_eq!(journal.takes[0].start_beats, 8.0);
        assert!(
            !journal.takes[0].path.is_absolute(),
            "the journal stores a portable path inside the project"
        );

        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let listed = pending(&root, &project).expect("pending");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].take.path, path);
        assert_eq!(listed[0].bytes, 64);

        clear(&root).expect("clear");
        assert!(read_journal(&root).expect("read").is_none());
        assert!(pending(&root, &project).expect("pending").is_empty());
    }

    #[test]
    fn takes_already_in_the_project_or_missing_are_not_pending() {
        let root = temp_root("skip");
        let committed = take(&root, "Take 001.wav", 32);
        let vanished = root.join("Audio").join("Recordings").join("Take 002.wav");
        let empty = take(&root, "Take 003.wav", 0);
        begin(
            &root,
            &[
                (committed.clone(), 1, "Audio 1".to_owned(), 0.0),
                (vanished, 1, "Audio 1".to_owned(), 4.0),
                (empty, 1, "Audio 1".to_owned(), 8.0),
            ],
        )
        .expect("begin");

        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.media_pool.insert(committed);
        assert!(
            pending(&root, &project).expect("pending").is_empty(),
            "committed, missing and empty takes need no decision"
        );
    }

    #[test]
    fn settling_one_take_keeps_the_rest_of_the_journal() {
        let root = temp_root("settle");
        let first = take(&root, "Take 001.wav", 16);
        let second = take(&root, "Take 002.wav", 16);
        begin(
            &root,
            &[
                (first.clone(), 1, "Audio 1".to_owned(), 0.0),
                (second.clone(), 1, "Audio 1".to_owned(), 4.0),
            ],
        )
        .expect("begin");

        settle(&root, &first).expect("settle");
        let project = Project::default();
        let remaining = pending(&root, &project).expect("pending");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].take.path, second);

        settle(&root, &second).expect("settle");
        assert!(read_journal(&root).expect("read").is_none());
    }

    #[test]
    fn discarding_moves_the_take_aside_without_overwriting() {
        let root = temp_root("discard");
        let first = take(&root, "Take 001.wav", 8);
        let moved = discard_file(&root, &first).expect("discard");
        assert!(!first.exists());
        assert_eq!(std::fs::read(&moved).expect("read"), vec![7_u8; 8]);
        assert!(
            moved.starts_with(root.join("Metadata").join("Recording").join("Discarded")),
            "the discarded take stays inside the project for manual recovery"
        );

        // A second take with the same name must not overwrite the first.
        let second = take(&root, "Take 001.wav", 4);
        let moved_again = discard_file(&root, &second).expect("discard");
        assert_ne!(moved, moved_again);
        assert_eq!(std::fs::read(&moved).expect("read").len(), 8);
        assert_eq!(std::fs::read(&moved_again).expect("read").len(), 4);
    }

    #[test]
    fn a_corrupt_journal_reports_an_error_instead_of_blocking_the_project() {
        let root = temp_root("corrupt");
        let path = journal_path(&root);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, b"{ not json").expect("write");
        assert!(read_journal(&root).is_err());
        assert!(pending(&root, &Project::default()).is_err());
    }
}
