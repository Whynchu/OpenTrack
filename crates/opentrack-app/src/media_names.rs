//! Reversible source-file names, shared by project Undo/Redo and discard.
use super::*;
#[derive(Default, Serialize, Deserialize)]
pub(super) struct Names {
    groups: Vec<Group>,
    #[serde(skip)]
    journal: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reopening_saved_project_recovers_an_unsaved_file_rename_after_process_loss() {
        let root = std::env::temp_dir().join(format!(
            "opentrack-name-journal-{}-{}",
            std::process::id(),
            NEXT_AUDIO_IMPORT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("fixture.opr");
        let old = root.join("take.wav");
        let new = root.join("renamed.wav");
        fs::write(&old, b"original recording").unwrap();
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_audio_clip(0, old.clone(), 0.0, 1.0);
        let mut geometry = serde_json::to_value(DockState::new(vec![PanelTab::Project])).unwrap();
        for rect in ["rect", "viewport"] {
            for edge in ["min", "max"] {
                for axis in ["x", "y"] {
                    geometry["surfaces"][0]["Main"]["nodes"][0]["Leaf"][rect][edge][axis] =
                        0.0.into();
                }
            }
        }
        let dock: DockState<PanelTab> = serde_json::from_value(geometry).unwrap();
        let document = ProjectFile::new(
            project.clone(),
            initial_workspace_state(&dock).unwrap(),
            dock,
            true,
            0,
            false,
        );
        document.write(&path).unwrap();
        let mut names = Names::default();
        names.journal = Some(journal_path(&path));
        names
            .rename(&old, &new, &HashSet::from([old.clone()]))
            .unwrap();
        drop(names);
        assert!(!old.exists() && new.exists());
        let restored = ProjectFile::read(&path).unwrap();
        assert_eq!(restored.project.tracks[0].clips[0].source_path, old);
        assert!(old.exists() && !new.exists());
        assert_eq!(fs::read(&old).unwrap(), b"original recording");
        fs::remove_file(journal_path(&path)).unwrap();
        fs::remove_dir(root.join("Metadata")).unwrap();
        fs::remove_file(old).unwrap();
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn file_names_follow_project_undo_redo_and_discard_without_overwriting_collisions() {
        let directory = std::env::temp_dir().join(format!(
            "opentrack-names-{}",
            NEXT_AUDIO_IMPORT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let old = directory.join("take.wav");
        let new = directory.join("vocal.wav");
        fs::write(&old, b"unique original audio bytes").unwrap();
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_audio_clip(0, old.clone(), 0.0, 1.0);
        let mut history = ProjectHistory::default();
        history.saved(&project);
        history.begin(&project);
        history
            .names
            .rename(&old, &new, &history.saved_paths)
            .unwrap();
        relink_audio_source_path(&mut project, &old, &new);
        assert!(history.finish_frame(&project, true));
        assert!(new.exists() && !old.exists());
        let undone = history.undo(&project).unwrap();
        assert!(old.exists() && !new.exists());
        let redone = history.redo(&undone).unwrap();
        assert!(new.exists() && !old.exists());
        fs::write(&old, b"unrelated occupant").unwrap();
        assert!(history.undo(&redone).is_none());
        assert_eq!(fs::read(&old).unwrap(), b"unrelated occupant");
        fs::remove_file(&old).unwrap();
        history.discard_recordings().unwrap();
        assert!(old.exists() && !new.exists());
        assert_eq!(fs::read(&old).unwrap(), b"unique original audio bytes");
        fs::remove_file(old).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Group {
    aliases: Vec<PathBuf>,
    current: PathBuf,
    saved: Option<PathBuf>,
    identity: u64,
}
fn identity(path: &Path) -> Result<u64, String> {
    use std::{
        hash::{Hash, Hasher},
        io::Read,
    };
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        buffer[..count].hash(&mut hash);
    }
    Ok(hash.finish())
}
impl Names {
    fn persist(&self) -> Result<(), String> {
        if let Some(path) = &self.journal {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            crate::project_file::atomic_write(
                path,
                &serde_json::to_vec(self).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn same_source(&self, a: &Path, b: &Path) -> bool {
        a == b
            || self
                .groups
                .iter()
                .any(|g| g.aliases.iter().any(|p| p == a) && g.aliases.iter().any(|p| p == b))
    }
    pub fn rename(
        &mut self,
        old: &Path,
        new: &Path,
        saved_paths: &HashSet<PathBuf>,
    ) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        let fingerprint = identity(old)?;
        if new.exists() {
            return Err("A file with that name already exists. Choose another name.".into());
        }
        let before = self.groups.clone();
        if let Some(group) = self.groups.iter_mut().find(|g| g.current == old) {
            if !group.aliases.iter().any(|p| p == new) {
                group.aliases.push(new.to_path_buf());
            }
            group.current = new.to_path_buf();
        } else {
            self.groups.push(Group {
                aliases: vec![old.to_path_buf(), new.to_path_buf()],
                current: new.to_path_buf(),
                saved: saved_paths.contains(old).then(|| old.to_path_buf()),
                identity: fingerprint,
            });
        }
        if let Err(error) = self
            .persist()
            .and_then(|_| fs::rename(old, new).map_err(|e| format!("Could not rename audio: {e}")))
        {
            self.groups = before;
            let _ = self.persist();
            return Err(error);
        }
        Ok(())
    }
    pub fn saved(&mut self, paths: &mut HashSet<PathBuf>) {
        for group in &mut self.groups {
            group.saved = paths
                .contains(&group.current)
                .then(|| group.current.clone());
            if group.saved.is_some() {
                paths.extend(group.aliases.iter().cloned());
            }
        }
    }
    pub fn restore(&mut self, project: Option<&Project>) -> Result<(), String> {
        let paths = project.map(|p| p.media_paths());
        let mut moves = Vec::new();
        for (i, g) in self.groups.iter().enumerate() {
            let target = if let Some(paths) = &paths {
                g.aliases
                    .iter()
                    .find(|p| paths.contains(*p))
                    .unwrap_or(&g.aliases[0])
            } else {
                g.saved.as_ref().unwrap_or(&g.aliases[0])
            };
            if target == &g.current && g.current.exists() {
                continue;
            }
            let source = if g.current.exists() {
                if identity(&g.current)? != g.identity {
                    return Err("The renamed audio was replaced externally; its filename was left unchanged.".into());
                }
                Some(&g.current)
            } else {
                g.aliases
                    .iter()
                    .find(|p| p.exists() && identity(p).ok() == Some(g.identity))
            };
            if let Some(source) = source
                && source != target
            {
                if target.exists() {
                    return Err(format!(
                        "Cannot restore audio name: {} already exists.",
                        target.display()
                    ));
                }
                moves.push((i, source.clone(), target.clone()));
            }
        }
        let mut completed: Vec<(PathBuf, PathBuf)> = Vec::new();
        if !moves.is_empty() {
            self.persist()?;
        }
        for (_, from, to) in &moves {
            if let Err(error) = fs::rename(from, to) {
                for (a, b) in completed.iter().rev() {
                    let _ = fs::rename(b, a);
                }
                return Err(format!("Could not restore audio filename: {error}"));
            }
            completed.push((from.clone(), to.clone()));
        }
        for (i, _, to) in moves {
            self.groups[i].current = to;
        }
        Ok(())
    }
}

impl OpenTrackApp {
    pub(super) fn rename_event_source(
        &mut self,
        track: u64,
        id: u64,
        name: &str,
    ) -> Result<bool, String> {
        let name = name.trim();
        if name.is_empty()
            || name.ends_with('.')
            || name.ends_with(' ')
            || name
                .chars()
                .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        {
            return Err("Use a nonempty audio name without Windows filename punctuation.".into());
        }
        let clip = self
            .data
            .project
            .tracks
            .iter()
            .find(|t| t.id == track)
            .and_then(|t| t.clips.iter().find(|c| c.id == id))
            .cloned()
            .ok_or("Audio event is no longer available.")?;
        let old = clip.source_path.clone();
        self.project_history.names.journal = self.project_path.as_deref().map(journal_path);
        let extension = old.extension().and_then(|s| s.to_str()).unwrap_or("wav");
        let stem = name.strip_suffix(&format!(".{extension}")).unwrap_or(name);
        let new = old.with_file_name(format!("{stem}.{extension}"));
        if old == new && clip.name == stem {
            return Ok(false);
        }
        self.project_history
            .names
            .rename(&old, &new, &self.project_history.saved_paths)?;
        for t in &mut self.data.project.tracks {
            for c in &mut t.clips {
                if c.source_path == old {
                    c.source_path = new.clone();
                    let independent = c.pitch_time.as_ref().is_some_and(|e| {
                        e.notes.iter().any(|n| {
                            n.shift != 0.0
                                || n.straighten != 0.0
                                || n.start != n.source_start
                                || n.end != n.source_end
                        })
                    });
                    if c.id == id || !independent {
                        c.name = stem.into();
                    }
                }
            }
        }
        if self.data.project.media_pool.remove(&old) {
            self.data.project.media_pool.insert(new.clone());
        }
        if let Some(source) = self.data.audio_sources.remove(&old) {
            if !source.is_streamed() {
                self.data.audio_sources.insert(new, source);
            } else {
                self.data
                    .streaming_readers
                    .retain(|_, (path, _)| path != &old);
            }
        }
        self.status_message = format!("Renamed audio to {stem}");
        Ok(true)
    }
}

fn journal_path(project: &Path) -> PathBuf {
    project
        .parent()
        .unwrap_or(Path::new("."))
        .join("Metadata")
        .join(format!(
            "Renames-{}.json",
            project.file_name().unwrap_or_default().to_string_lossy()
        ))
}
pub(crate) fn recover(project_path: &Path, project: &Project) -> Result<(), String> {
    let path = journal_path(project_path);
    if !path.exists() {
        return Ok(());
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("Audio rename journal is too large.".into());
    }
    let mut names: Names = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let media = project.media_paths();
    names
        .groups
        .retain(|g| g.aliases.iter().any(|p| media.contains(p)));
    if names
        .groups
        .iter()
        .any(|g| g.aliases.is_empty() || g.aliases.iter().any(|p| p.parent() != g.current.parent()))
    {
        return Err("Audio rename journal contains an invalid directory change.".into());
    }
    names.journal = Some(path);
    names.restore(Some(project))
}
