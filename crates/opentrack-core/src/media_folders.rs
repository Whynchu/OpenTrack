//! Named Media Pool folders.
//!
//! Folders are pool organisation, not file-system folders: they group the
//! sources a project already references. The pool stays the authoritative list of
//! sources, so creating, renaming or deleting a folder never touches media or
//! events. Each source sits in at most one folder, and unfiled sources stay in
//! the pool root.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Bound on named pool folders in one project.
pub const MAX_MEDIA_FOLDERS: usize = 32;
/// Bound on one folder name, in characters.
pub const MAX_MEDIA_FOLDER_NAME_CHARS: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaFolders {
    /// Folder names in creation order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// Which folder each source sits in. Unlisted sources are unfiled.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assignments: BTreeMap<PathBuf, String>,
}

impl MediaFolders {
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.iter().any(|candidate| candidate == name)
    }

    /// The folder a source sits in, if any.
    pub fn folder_of(&self, path: &Path) -> Option<&str> {
        self.assignments.get(path).map(String::as_str)
    }

    /// Sources filed in `name`, in pool order once intersected with the pool.
    pub fn paths_in(&self, name: &str) -> impl Iterator<Item = &Path> {
        self.assignments
            .iter()
            .filter(move |(_, folder)| folder.as_str() == name)
            .map(|(path, _)| path.as_path())
    }

    /// Creates an empty folder and returns its trimmed name.
    ///
    /// A blank name, one over the length bound, one containing a path separator,
    /// and a duplicate are refused rather than silently renamed.
    pub fn create(&mut self, name: &str) -> Result<String, String> {
        let name = normalize(name)?;
        if self.contains(&name) {
            return Err(format!("A folder named {name} already exists"));
        }
        if self.names.len() >= MAX_MEDIA_FOLDERS {
            return Err(format!(
                "A project can have at most {MAX_MEDIA_FOLDERS} pool folders"
            ));
        }
        self.names.push(name.clone());
        Ok(name)
    }

    /// Renames a folder and its assignments, reporting how many sources moved.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<usize, String> {
        let to = normalize(to)?;
        let Some(index) = self.names.iter().position(|name| name == from) else {
            return Err(format!("There is no folder named {from}"));
        };
        if to != from && self.contains(&to) {
            return Err(format!("A folder named {to} already exists"));
        }
        self.names[index] = to.clone();
        let mut moved = 0;
        for folder in self.assignments.values_mut() {
            if folder == from {
                *folder = to.clone();
                moved += 1;
            }
        }
        Ok(moved)
    }

    /// Deletes a folder, leaving its sources in the pool root.
    pub fn remove(&mut self, name: &str) -> usize {
        let before = self.names.len();
        self.names.retain(|candidate| candidate != name);
        if self.names.len() == before {
            return 0;
        }
        let mut cleared = 0;
        self.assignments.retain(|_, folder| {
            if folder == name {
                cleared += 1;
                false
            } else {
                true
            }
        });
        cleared
    }

    /// Files `paths` into `folder` (or the root when `folder` is `None`).
    pub fn assign(&mut self, paths: &[PathBuf], folder: Option<&str>) -> Result<usize, String> {
        if let Some(folder) = folder
            && !self.contains(folder)
        {
            return Err(format!("There is no folder named {folder}"));
        }
        let mut changed = 0;
        for path in paths {
            let current = self.assignments.get(path).map(String::as_str);
            match folder {
                Some(folder) if current != Some(folder) => {
                    self.assignments.insert(path.clone(), folder.to_owned());
                    changed += 1;
                }
                None if current.is_some() => {
                    self.assignments.remove(path);
                    changed += 1;
                }
                _ => {}
            }
        }
        Ok(changed)
    }

    /// Drops assignments for sources the pool no longer holds.
    pub fn retain_paths(&mut self, keep: &BTreeSet<PathBuf>) {
        self.assignments.retain(|path, _| keep.contains(path));
    }

    pub fn is_valid(&self) -> Result<(), &'static str> {
        if self.names.len() > MAX_MEDIA_FOLDERS {
            return Err("too many media pool folders");
        }
        let mut seen = BTreeSet::new();
        for name in &self.names {
            if normalize(name).as_deref() != Ok(name.as_str()) {
                return Err("a media pool folder name is not usable");
            }
            if !seen.insert(name) {
                return Err("media pool folder names must be unique");
            }
        }
        if self
            .assignments
            .values()
            .any(|folder| !seen.contains(folder))
        {
            return Err("a media pool source points at a missing folder");
        }
        Ok(())
    }
}

/// Trims and checks one folder name.
fn normalize(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A folder needs a name".to_owned());
    }
    if name.chars().count() > MAX_MEDIA_FOLDER_NAME_CHARS {
        return Err(format!(
            "A folder name can be at most {MAX_MEDIA_FOLDER_NAME_CHARS} characters"
        ));
    }
    if name.contains(['/', '\\']) || name.chars().any(char::is_control) {
        return Err("A folder name cannot contain a path separator".to_owned());
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> PathBuf {
        PathBuf::from(format!("/media/{name}.wav"))
    }

    #[test]
    fn folders_are_named_bounded_and_unique() {
        let mut folders = MediaFolders::default();
        assert_eq!(folders.create("  Drums  ").as_deref(), Ok("Drums"));
        assert!(folders.contains("Drums"));
        assert!(folders.create("Drums").is_err(), "duplicates are refused");
        assert!(folders.create("   ").is_err());
        assert!(folders.create("a/b").is_err());
        assert!(
            folders
                .create(&"x".repeat(MAX_MEDIA_FOLDER_NAME_CHARS + 1))
                .is_err()
        );
        assert!(folders.is_valid().is_ok());
    }

    #[test]
    fn assigning_renaming_and_removing_keep_the_pool_authoritative() {
        let mut folders = MediaFolders::default();
        folders.create("Drums").expect("drums");
        folders.create("Vocals").expect("vocals");
        let kick = path("kick");
        let snare = path("snare");
        let vox = path("vox");

        assert_eq!(
            folders.assign(&[kick.clone(), snare.clone()], Some("Drums")),
            Ok(2)
        );
        assert_eq!(folders.assign(&[kick.clone()], Some("Drums")), Ok(0));
        assert_eq!(folders.folder_of(&kick), Some("Drums"));
        assert_eq!(
            folders.paths_in("Drums").collect::<Vec<_>>(),
            vec![kick.as_path(), snare.as_path()]
        );
        // An unknown folder is refused instead of inventing one.
        assert!(folders.assign(&[vox.clone()], Some("Missing")).is_err());

        // Renaming carries the assignments with it.
        assert_eq!(folders.rename("Drums", "Kit"), Ok(2));
        assert_eq!(folders.folder_of(&snare), Some("Kit"));
        assert!(!folders.contains("Drums"));
        assert!(folders.rename("Kit", "Vocals").is_err());

        // Removing a folder unfiles its sources without touching them.
        assert_eq!(folders.remove("Kit"), 2);
        assert_eq!(folders.folder_of(&kick), None);
        assert!(folders.is_valid().is_ok());
        assert_eq!(folders.remove("Kit"), 0);

        // Unfiling is explicit and counted.
        folders
            .assign(&[vox.clone()], Some("Vocals"))
            .expect("file");
        assert_eq!(folders.assign(&[vox.clone()], None), Ok(1));
        assert_eq!(folders.assign(&[vox], None), Ok(0));
    }

    #[test]
    fn validation_rejects_duplicates_and_stale_assignments() {
        let mut folders = MediaFolders {
            names: vec!["Kit".to_owned(), "Kit".to_owned()],
            ..MediaFolders::default()
        };
        assert!(folders.is_valid().is_err());
        folders.names = vec!["Kit".to_owned()];
        folders.assignments.insert(path("kick"), "Gone".to_owned());
        assert!(folders.is_valid().is_err());
        folders.assignments.clear();
        folders.assignments.insert(path("kick"), "Kit".to_owned());
        assert!(folders.is_valid().is_ok());

        // Pruning drops assignments for sources the pool no longer holds.
        folders.retain_paths(&BTreeSet::new());
        assert!(folders.assignments.is_empty());
        assert_eq!(folders.names, vec!["Kit".to_owned()]);
    }
}
