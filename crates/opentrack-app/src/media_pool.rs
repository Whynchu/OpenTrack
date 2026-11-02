//! Persistent pool and reversible Delete/Shift+Delete. Sources are never removed implicitly.
use super::*;
#[derive(Clone, Debug)]
pub(super) enum Target {
    Events(Vec<u64>),
    Track(u64),
}
fn key(path: &Path) -> PathBuf {
    let p = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if cfg!(windows) {
        PathBuf::from(p.to_string_lossy().to_lowercase())
    } else {
        p
    }
}
pub(super) fn selection_target(project: &Project, state: &audio_events::State) -> Option<Target> {
    let ids = state
        .selected_ids()
        .into_iter()
        .filter(|id| {
            project
                .tracks
                .iter()
                .any(|t| t.clips.iter().any(|c| c.id == *id))
        })
        .collect::<Vec<_>>();
    if !ids.is_empty() {
        Some(Target::Events(ids))
    } else if state.row_focus {
        project
            .selected_track
            .and_then(|i| project.tracks.get(i as usize))
            .map(|t| Target::Track(t.id))
    } else {
        None
    }
}
pub(super) fn prepare(
    project: &Project,
    target: Target,
    delete_files: bool,
) -> Result<(Project, Vec<PathBuf>), String> {
    let mut candidate = project.clone();
    candidate.register_media();
    let mut removed = Vec::new();
    match target {
        Target::Events(ids) => {
            for t in &mut candidate.tracks {
                t.clips.retain(|c| {
                    if ids.contains(&c.id) {
                        removed.push(c.source_path.clone());
                        false
                    } else {
                        true
                    }
                });
            }
            if removed.is_empty() {
                return Err("Select an audio event to delete".into());
            }
        }
        Target::Track(id) => {
            let index = candidate
                .tracks
                .iter()
                .position(|t| t.id == id)
                .ok_or("The selected track no longer exists")?;
            removed.extend(
                candidate.tracks[index]
                    .clips
                    .iter()
                    .map(|c| c.source_path.clone()),
            );
            candidate.remove_track(index);
        }
    }
    let mut files = Vec::new();
    if delete_files {
        let mut unique = HashMap::new();
        for path in removed {
            unique.entry(key(&path)).or_insert(path);
        }
        if candidate
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .any(|c| unique.contains_key(&key(&c.source_path)))
        {
            return Err("File is still used by another audio event. Nothing was deleted; select all its events or use Delete to retain the file.".into());
        }
        candidate
            .media_pool
            .retain(|p| !unique.contains_key(&key(p)));
        files = unique.into_values().collect();
        files.sort();
    }
    candidate.validate().map_err(str::to_owned)?;
    Ok((candidate, files))
}
pub(super) fn show(ui: &mut egui::Ui, project: &Project) -> Option<PathBuf> {
    let mut insert = None;
    ui.heading("Media Pool");
    ui.label("Delete retains source media here. Shift+Delete removes unused source files with undo recovery.");
    ui.separator();
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("project-media-pool")
            .striped(true)
            .spacing([16.0, 7.0])
            .show(ui, |ui| {
                for heading in ["File", "Usage", "Location", ""] {
                    ui.strong(heading);
                }
                ui.end_row();
                for path in project.media_paths() {
                    let uses = project
                        .tracks
                        .iter()
                        .flat_map(|t| &t.clips)
                        .filter(|c| c.source_path == path)
                        .count();
                    ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                    ui.label(if path.is_file() {
                        format!("{uses} event(s)")
                    } else {
                        "Missing file".into()
                    });
                    ui.label(path.display().to_string());
                    if ui
                        .add_enabled(path.is_file(), egui::Button::new("Add to Project"))
                        .clicked()
                    {
                        insert = Some(path.clone());
                    }
                    ui.end_row();
                }
            })
    });
    if project.media_paths().is_empty() {
        ui.label("No audio media in this project.");
    }
    insert
}
impl OpenTrackApp {
    pub(super) fn show_media_operation_error(&mut self, ctx: &egui::Context) {
        let Some(message) = self.media_operation_error.clone() else {
            return;
        };
        let mut open = true;
        let mut done = false;
        egui::Window::new("Audio File Operation")
            .id(Id::new("media-operation-error"))
            .collapsible(false)
            .resizable(false)
            .default_width(500.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        ui.label(message);
                    });
                if ui.button("OK").clicked() {
                    done = true;
                }
            });
        if done || !open {
            self.media_operation_error = None;
        }
    }
    pub(super) fn delete_audio_selection(&mut self, delete_files: bool) {
        if self.is_recording() {
            self.status_message = "Stop recording before deleting audio".into();
            return;
        }
        let Some(target) =
            selection_target(&self.data.project, &self.arrange_view_state.audio_events)
        else {
            if delete_files {
                return;
            }
            let Some(part_id) = self.data.project.selected_midi_part_id else {
                return;
            };
            let Some(track_index) = self.data.project.selected_track.map(|index| index as usize)
            else {
                return;
            };
            let Some(track) = self.data.project.tracks.get(track_index) else {
                return;
            };
            if !track.midi_parts.iter().any(|part| part.id == part_id) {
                return;
            }
            self.project_history.finish_frame(&self.data.project, true);
            if !self
                .project_history
                .begin(&self.data.project, self.modified)
            {
                self.status_message = "Cannot delete: undo snapshot is unavailable".into();
                return;
            }
            self.data.project.tracks[track_index]
                .midi_parts
                .retain(|part| part.id != part_id);
            self.data.project.selected_midi_part_id = None;
            self.midi_editor_open = false;
            self.selected_midi_note_id = None;
            self.midi_note_drag = None;
            self.midi_note_draw = None;
            self.midi_controller_drag = None;
            self.project_history.finish_frame(&self.data.project, true);
            self.rebuild_audio_plan();
            self.mark_modified();
            self.status_message = "Removed selected MIDI part; Undo restores it".into();
            return;
        };
        let (candidate, files) = match prepare(&self.data.project, target, delete_files) {
            Ok(v) => v,
            Err(e) => {
                self.media_operation_error = Some(e.clone());
                self.status_message = e;
                return;
            }
        };
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot delete: undo snapshot is unavailable".into();
            return;
        }
        if !files.is_empty() {
            let root = self
                .project_path
                .as_deref()
                .and_then(Path::parent)
                .map(|p| p.join("Metadata").join("Undo"))
                .unwrap_or_else(|| std::env::temp_dir().join("OpenTrack-Undo"));
            let mut recovery = recording_undo::RecordingUndo::new(files, root);
            if let Err(e) = recovery.remove(&candidate) {
                self.project_history.pending = None;
                self.status_message = format!("Could not delete media: {e}");
                self.media_operation_error = Some(self.status_message.clone());
                return;
            }
            self.project_history.pending_deleted = Some(recovery);
        }
        self.data.project = candidate;
        self.arrange_view_state.audio_events.clear_selection();
        self.project_history.finish_frame(&self.data.project, true);
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = if delete_files {
            "Removed selected content and unused source files; Undo restores both"
        } else {
            "Removed selected content; audio remains in Media Pool; Undo restores content"
        }
        .into();
    }
    /// Copies every referenced source into the project's own `Audio` folder.
    ///
    /// Consolidation makes the project self-contained: a file used from a shared
    /// library is copied once, every event that referenced it is repointed, and
    /// one undo step covers the whole change. Sources already inside the project
    /// are left alone, and a missing source is reported instead of silently
    /// dropped from the project.
    pub(super) fn consolidate_project_media(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before consolidating media".to_owned();
            return;
        }
        if !self.project_folder_managed {
            self.status_message =
                "Choose or create this project's folder before consolidating media".to_owned();
            return;
        }
        let Some(project_root) = self
            .project_path
            .as_deref()
            .and_then(Path::parent)
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
        else {
            self.status_message =
                "Save this project into its folder before consolidating media".to_owned();
            return;
        };

        let sources = self.data.project.media_paths();
        let mut moved = std::collections::BTreeMap::<PathBuf, PathBuf>::new();
        let mut already_local = 0;
        let mut missing = Vec::new();
        let mut issues = Vec::new();
        for source in sources {
            if !source.exists() {
                missing.push(source);
                continue;
            }
            match audio_import_destination(&source, &project_root) {
                Ok((_, false)) => already_local += 1,
                Ok((destination, true)) => match fs::copy(&source, &destination) {
                    Ok(_) => {
                        moved.insert(source, destination);
                    }
                    Err(error) => {
                        issues.push(format!("could not copy {}: {error}", source.display()))
                    }
                },
                Err(error) => issues.push(error),
            }
        }

        if moved.is_empty() {
            let mut status = if already_local > 0 {
                format!("All {already_local} source(s) already live in this project")
            } else {
                "There is no media to consolidate".to_owned()
            };
            if !missing.is_empty() {
                status.push_str(&format!("; {} source(s) are missing", missing.len()));
            }
            if !issues.is_empty() {
                status.push_str("; ");
                status.push_str(&issues.join(" · "));
            }
            self.status_message = status;
            return;
        }

        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            for destination in moved.values() {
                let _ = fs::remove_file(destination);
            }
            self.status_message = "Cannot consolidate: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Consolidate Media");
        let count = moved.len();
        for track in self.data.project.tracks.iter_mut() {
            for clip in track.clips.iter_mut() {
                if let Some(destination) = moved.get(&clip.source_path) {
                    clip.source_path = destination.clone();
                }
            }
        }
        self.data.project.media_pool = self
            .data
            .project
            .media_pool
            .iter()
            .map(|path| moved.get(path).cloned().unwrap_or_else(|| path.clone()))
            .collect();
        self.data.project.register_media();
        for (source, destination) in &moved {
            if let Some(audio) = self.data.audio_sources.remove(source) {
                self.data.audio_sources.insert(destination.clone(), audio);
            }
        }
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        let mut status = format!(
            "Consolidated {count} source(s) into the project's Audio folder; Undo restores the references"
        );
        if already_local > 0 {
            status.push_str(&format!("; {already_local} already inside the project"));
        }
        if !missing.is_empty() {
            status.push_str(&format!("; {} source(s) are missing", missing.len()));
        }
        if !issues.is_empty() {
            status.push_str("; ");
            status.push_str(&issues.join(" · "));
        }
        self.status_message = status;
    }

    pub(super) fn add_pool_media(&mut self, path: PathBuf) {
        if self.is_recording() {
            return;
        }
        let source = match AudioSource::open(&path) {
            Ok(s) => s,
            Err(e) => {
                self.status_message = e;
                return;
            }
        };
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            return;
        }
        let index = self
            .data
            .project
            .selected_track
            .and_then(|i| {
                self.data
                    .project
                    .tracks
                    .get(i as usize)
                    .filter(|t| {
                        t.kind == TrackKind::Audio && t.audio_channels == source.channels() as u8
                    })
                    .map(|_| i as usize)
            })
            .unwrap_or_else(|| {
                self.data
                    .project
                    .add_track_with_audio_channels(TrackKind::Audio, source.channels() as u8);
                self.data.project.tracks.len() - 1
            });
        self.data.project.add_audio_clip(
            index,
            path.clone(),
            self.data.transport.position_beats,
            source.duration_seconds(),
        );
        self.data.audio_sources.insert(path, source);
        self.project_history.finish_frame(&self.data.project, true);
        self.rebuild_audio_plan();
        self.mark_modified();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn apply(p: &mut Project, h: &mut ProjectHistory, id: u64, hard: bool, root: &Path) {
        let (candidate, files) = prepare(p, Target::Events(vec![id]), hard).unwrap();
        assert!(h.begin(p, false));
        if !files.is_empty() {
            let mut recovery = recording_undo::RecordingUndo::new(files, root.to_path_buf());
            recovery.remove(&candidate).unwrap();
            h.pending_deleted = Some(recovery);
        }
        *p = candidate;
        assert!(h.finish_frame(p, true));
    }
    #[test]
    fn row_and_event_selection_are_separate_and_deleting_event_never_selects_row_implicitly() {
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        let id = p.add_audio_clip(0, "a.wav".into(), 0.0, 1.0).unwrap();
        let track = p.tracks[0].id;
        let mut state = audio_events::State::default();
        state.select_one(id);
        assert!(matches!(selection_target(&p,&state),Some(Target::Events(ids)) if ids==vec![id]));
        state.clear_selection();
        assert!(selection_target(&p, &state).is_none());
        state.focus_row();
        assert!(matches!(selection_target(&p,&state),Some(Target::Track(t)) if t==track));
    }
    #[test]
    fn delete_retains_pool_while_shift_delete_undo_redo_and_discard_restore_exact_file() {
        let (path, _source) = sample_editor::tests::fixture_source(1);
        let bytes = fs::read(&path).unwrap();
        let root = path.with_extension("undo");
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        let id = p.add_audio_clip(0, path.clone(), 0.0, 1.0).unwrap();
        let saved = p.clone();
        let mut h = ProjectHistory::default();
        h.saved(&p);
        apply(&mut p, &mut h, id, false, &root);
        assert!(path.is_file());
        assert!(p.media_pool.contains(&path));
        assert!(p.tracks[0].clips.is_empty());
        p = h.undo(&p).unwrap();
        assert_eq!(p, saved);
        p = h.redo(&p).unwrap();
        assert!(p.media_pool.contains(&path));
        p = h.undo(&p).unwrap();
        apply(&mut p, &mut h, id, true, &root);
        assert!(!path.exists());
        assert!(!p.media_pool.contains(&path));
        fs::write(&path, b"another file").unwrap();
        let depth = h.undo.len();
        assert!(h.undo(&p).is_none());
        assert_eq!(h.undo.len(), depth);
        assert_eq!(fs::read(&path).unwrap(), b"another file");
        fs::remove_file(&path).unwrap();
        p = h.undo(&p).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(p, saved);
        p = h.redo(&p).unwrap();
        assert!(p.tracks[0].clips.is_empty());
        assert!(!path.exists());
        h.discard_recordings().unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        h.clear();
        fs::remove_file(path).unwrap();
        let _ = fs::remove_dir(root);
    }
    #[test]
    fn shared_source_blocks_hard_delete_and_row_delete_keeps_pool() {
        let (path, _) = sample_editor::tests::fixture_source(1);
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        let first = p.add_audio_clip(0, path.clone(), 0.0, 1.0).unwrap();
        p.add_audio_clip(0, path.clone(), 2.0, 1.0).unwrap();
        assert!(prepare(&p, Target::Events(vec![first]), true).is_err());
        assert!(path.exists());
        let (removed, files) = prepare(&p, Target::Track(p.tracks[0].id), false).unwrap();
        assert!(removed.tracks.is_empty());
        assert!(removed.media_pool.contains(&path));
        assert!(files.is_empty());
        let bytes = serde_json::to_vec(&removed).unwrap();
        let reopened: Project = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(reopened.media_pool, removed.media_pool);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn trimmed_deletion_history_keeps_saved_file_recovery_for_discard() {
        let (path, _) = sample_editor::tests::fixture_source(1);
        let bytes = fs::read(&path).unwrap();
        let root = path.with_extension("undo");
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        let id = p.add_audio_clip(0, path.clone(), 0.0, 1.0).unwrap();
        let mut h = ProjectHistory::default();
        h.saved(&p);
        apply(&mut p, &mut h, id, true, &root);
        for i in 0..40 {
            h.begin(&p, false);
            p.name = format!("edit {i}");
            h.finish_frame(&p, true);
        }
        assert!(!path.exists());
        assert!(!h.saved_recording_recovery.is_empty());
        h.discard_recordings().unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        h.clear();
        fs::remove_file(path).unwrap();
        let _ = fs::remove_dir(root);
    }

    /// A minimal readable WAV so a consolidated copy can be opened.
    fn write_wav(path: &Path, frames: usize) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + (frames * 8) as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&3_u16.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&(48_000_u32 * 8).to_le_bytes());
        bytes.extend_from_slice(&8_u16.to_le_bytes());
        bytes.extend_from_slice(&32_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&((frames * 8) as u32).to_le_bytes());
        for index in 0..frames {
            let value = (index as f32) / 48_000.0;
            bytes.extend_from_slice(&value.to_le_bytes());
            bytes.extend_from_slice(&(-value).to_le_bytes());
        }
        std::fs::write(path, bytes).expect("write wav");
    }

    fn project_folder(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "opentrack-consolidate-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Audio").join("Imports")).expect("imports");
        std::fs::create_dir_all(root.join("Metadata")).expect("metadata");
        root
    }

    #[test]
    fn consolidating_copies_shared_sources_once_and_repoints_every_event() {
        let root = project_folder("app");
        let shared_dir =
            std::env::temp_dir().join(format!("opentrack-consolidate-src-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&shared_dir);
        std::fs::create_dir_all(&shared_dir).expect("shared");
        let shared = shared_dir.join("kick.wav");
        write_wav(&shared, 480);

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, shared.clone(), 0.0, 1.0)
            .unwrap();
        app.data
            .project
            .add_audio_clip(0, shared.clone(), 4.0, 1.0)
            .unwrap();
        app.project_history.clear();

        app.consolidate_project_media();

        let destination = app.data.project.tracks[0].clips[0].source_path.clone();
        assert_ne!(destination, shared, "the event left the shared source");
        assert_eq!(
            destination.parent(),
            Some(root.join("Audio").join("Imports").as_path())
        );
        assert!(destination.exists(), "the copy exists");
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .all(|clip| clip.source_path == destination),
            "both events point at the one copy"
        );
        assert!(
            !app.data.project.media_pool.contains(&shared),
            "the media pool no longer references the outside file"
        );
        assert_eq!(
            app.data.project.media_pool.len(),
            1,
            "one pool entry for the one source"
        );
        assert!(app.status_message.contains("Consolidated 1 source(s)"));

        app.consolidate_project_media();
        assert!(app.status_message.contains("already live in this project"));
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(shared_dir);
    }

    #[test]
    fn consolidating_reports_missing_sources_without_dropping_them() {
        let root = project_folder("missing");
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, PathBuf::from("/nowhere/missing.wav"), 0.0, 1.0)
            .unwrap();

        app.consolidate_project_media();
        assert!(app.status_message.contains("1 source(s) are missing"));
        assert_eq!(app.data.project.tracks[0].clips.len(), 1);
        assert_eq!(
            app.data.project.tracks[0].clips[0].source_path,
            PathBuf::from("/nowhere/missing.wav")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
