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
/// One manifest import's result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ManifestImport {
    pub added: usize,
    pub already_present: usize,
    pub missing: Vec<PathBuf>,
}

/// A media manifest: a comment header, then one `path<TAB>event count` per source.
///
/// It is deliberately plain text so it can be read, diffed and edited by hand.
/// The event count is informational; import only uses the path.
pub(super) fn manifest_text(project: &Project) -> String {
    let mut text = String::from("# OpenTrack media manifest v1\n");
    for path in project.media_paths() {
        let uses = project
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .filter(|clip| clip.source_path == path)
            .count();
        text.push_str(&path.to_string_lossy());
        text.push('\t');
        text.push_str(&uses.to_string());
        text.push('\n');
    }
    text
}

/// Adds the manifest's existing files to the pool and reports what happened.
///
/// Lines starting with `#` and blank lines are skipped; the second column is
/// ignored so hand-written one-path-per-line lists also work. Missing files are
/// reported and never added, and the pool is not copied into the project.
pub(super) fn import_manifest(project: &mut Project, text: &str) -> ManifestImport {
    let mut report = ManifestImport::default();
    let mut seen = std::collections::BTreeSet::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let path = PathBuf::from(trimmed.split('\t').next().unwrap_or(trimmed));
        if path.as_os_str().is_empty() || !seen.insert(path.clone()) {
            continue;
        }
        if !path.exists() {
            report.missing.push(path);
        } else if project.media_pool.contains(&path) {
            report.already_present += 1;
        } else {
            project.media_pool.insert(path);
            report.added += 1;
        }
    }
    report
}

/// True when `path` matches a Media Pool search term.
///
/// A term matches the file name or any part of the full path, case-insensitively,
/// so both "kick" and "Samples/Kick" work.
pub(super) fn media_matches(path: &Path, search: &str) -> bool {
    let search = search.trim().to_lowercase();
    if search.is_empty() {
        return true;
    }
    path.file_name()
        .map(|name| name.to_string_lossy().to_lowercase().contains(&search))
        .unwrap_or(false)
        || path.to_string_lossy().to_lowercase().contains(&search)
}

pub(super) fn show(
    ui: &mut egui::Ui,
    project: &Project,
    revealed: Option<&Path>,
    search: &mut String,
    focus_search: &mut bool,
) -> Option<PathBuf> {
    let mut insert = None;
    ui.heading("Media Pool");
    ui.label("Delete retains source media here. Shift+Delete removes unused source files with undo recovery.");
    ui.horizontal(|ui| {
        ui.label("Search");
        let response = ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(220.0)
                .hint_text("Filter by name or path"),
        );
        if *focus_search {
            response.request_focus();
            *focus_search = false;
        }
        if !search.is_empty() && ui.button("✕").on_hover_text("Clear the filter").clicked() {
            search.clear();
        }
    });
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
                let mut shown = 0usize;
                for path in project
                    .media_paths()
                    .into_iter()
                    .filter(|path| media_matches(path, search))
                {
                    shown += 1;
                    let uses = project
                        .tracks
                        .iter()
                        .flat_map(|t| &t.clips)
                        .filter(|c| c.source_path == path)
                        .count();
                    let is_revealed = revealed.is_some_and(|shown| shown == path);
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let label = if is_revealed {
                        format!("▶ {name}")
                    } else {
                        name.into_owned()
                    };
                    let response = if is_revealed {
                        ui.colored_label(theme::palette(ui).accent, label)
                    } else {
                        ui.label(label)
                    };
                    if is_revealed {
                        // Bring the revealed source into view when the pool is
                        // longer than the window.
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
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
                if shown == 0 && !search.trim().is_empty() {
                    ui.weak("No media matches this search.");
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
    /// Writes the project's media manifest to a file the user chooses.
    pub(super) fn export_media_pool(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to export its media pool".to_owned();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Media manifest", &["txt"])
            .set_file_name(format!("{} media.txt", self.data.project.name))
            .set_title("Export Media Pool")
            .save_file()
        else {
            return;
        };
        self.export_media_pool_to(&path);
    }

    /// Writes the manifest to `path`; split out so tests do not need a dialog.
    pub(super) fn export_media_pool_to(&mut self, path: &Path) {
        let text = manifest_text(&self.data.project);
        match std::fs::write(path, text) {
            Ok(()) => {
                self.status_message = format!(
                    "Exported {} media path(s) to {}",
                    self.data.project.media_paths().len(),
                    path.display()
                )
            }
            Err(error) => self.status_message = format!("Could not export the media pool: {error}"),
        }
    }

    /// Adds the files named in a manifest the user chooses.
    pub(super) fn import_media_pool(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to import a media pool".to_owned();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Media manifest", &["txt"])
            .set_title("Import Media Pool")
            .pick_file()
        else {
            return;
        };
        self.import_media_pool_from(&path);
    }

    /// Imports the manifest at `path` as one undo step; split out for tests.
    pub(super) fn import_media_pool_from(&mut self, path: &Path) {
        if self.is_recording() {
            self.status_message = "Stop recording before importing a media pool".to_owned();
            return;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                self.status_message = format!("Could not read the manifest: {error}");
                return;
            }
        };
        let mut candidate = self.data.project.clone();
        let report = import_manifest(&mut candidate, &text);
        if report.added == 0 {
            self.status_message = format!(
                "Nothing to import: {} already in the pool, {} missing",
                report.already_present,
                report.missing.len()
            );
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message =
                "Cannot import the media pool: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Import Pool");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        let mut status = format!(
            "Added {} media path(s) to the pool ({} already there)",
            report.added, report.already_present
        );
        if !report.missing.is_empty() {
            status.push_str(&format!("; {} file(s) are missing", report.missing.len()));
        }
        self.status_message = status;
    }

    /// Drops pool entries no event references, keeping the files on disk.
    pub(super) fn remove_unused_media_from_pool(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before changing the media pool".to_owned();
            return;
        }
        let unused = self.data.project.unused_media();
        if unused.is_empty() {
            self.status_message = "The media pool has no unused entries".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot clean the pool: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Remove Unused Media");
        let removed = self.data.project.remove_unused_media();
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.status_message = format!(
            "Removed {} unused pool entr{}; the files stay on disk. Undo restores the entries",
            removed.len(),
            if removed.len() == 1 { "y" } else { "ies" }
        );
    }

    /// Drops pool entries whose file is gone and that no event references.
    pub(super) fn remove_missing_media_from_pool(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before changing the media pool".to_owned();
            return;
        }
        let missing = self.data.project.missing_unreferenced_media();
        if missing.is_empty() {
            self.status_message =
                "The media pool has no missing entries that no event references".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot clean the pool: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Remove Missing Files");
        let removed = self.data.project.remove_missing_unreferenced_media();
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.status_message = format!(
            "Removed {} missing pool entr{}. A missing file an event still uses is kept for relinking",
            removed.len(),
            if removed.len() == 1 { "y" } else { "ies" }
        );
    }

    /// Deletes unused source files that live inside the project, recoverably.
    ///
    /// OpenTrack has no separate trash, so "Empty Trash" means: drop every unused
    /// pool entry and delete the unused files that belong to this project. Files
    /// outside the project folder may be shared, so they are left on disk. Undo
    /// brings both the entries and the deleted files back.
    pub(super) fn empty_media_trash(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before emptying unused media".to_owned();
            return;
        }
        let unused = self.data.project.unused_media();
        if unused.is_empty() {
            self.status_message = "There are no unused sources to empty".to_owned();
            return;
        }
        let Some(root) = self.project_root() else {
            self.status_message =
                "Save this project into its folder before deleting unused files".to_owned();
            return;
        };
        let canonical_root = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        let files = unused
            .iter()
            .filter(|path| path.exists())
            .filter(|path| {
                fs::canonicalize(path)
                    .unwrap_or_else(|_| (*path).clone())
                    .starts_with(&canonical_root)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut candidate = self.data.project.clone();
        candidate.remove_unused_media();
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message =
                "Cannot empty unused media: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Empty Trash");
        if !files.is_empty() {
            let mut recovery = recording_undo::RecordingUndo::new(
                files.clone(),
                root.join("Metadata").join("Undo"),
            );
            if let Err(error) = recovery.remove(&candidate) {
                self.project_history.pending = None;
                self.status_message = format!("Could not delete unused media: {error}");
                return;
            }
            self.project_history.pending_deleted = Some(recovery);
        }
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.status_message = if files.is_empty() {
            format!(
                "Removed {} unused pool entr{}; files outside the project were kept",
                unused.len(),
                if unused.len() == 1 { "y" } else { "ies" }
            )
        } else {
            format!(
                "Deleted {} unused file(s) from the project and removed {} pool entr{}; Undo restores both",
                files.len(),
                unused.len(),
                if unused.len() == 1 { "y" } else { "ies" }
            )
        };
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

    /// Finds replacement files for the project's missing sources in `folder`.
    ///
    /// The search is bounded (depth and entry count) and matches by file name
    /// case-insensitively, so a project moved between machines can be relinked by
    /// pointing at the folder that still holds its audio.
    pub(super) fn find_relinked_media(
        &self,
        folder: &Path,
    ) -> std::collections::BTreeMap<PathBuf, PathBuf> {
        use std::collections::BTreeMap;
        let missing = self.data.project.missing_media();
        if missing.is_empty() || !folder.is_dir() {
            return BTreeMap::new();
        }
        // One index by lower-case name: the first match wins, which keeps the
        // result stable when several folders hold a copy.
        let mut by_name = std::collections::BTreeMap::<String, PathBuf>::new();
        let mut pending = vec![(folder.to_path_buf(), 0usize)];
        let mut visited = 0usize;
        const MAX_DEPTH: usize = 6;
        const MAX_ENTRIES: usize = 20_000;
        while let Some((directory, depth)) = pending.pop() {
            if depth > MAX_DEPTH || visited >= MAX_ENTRIES {
                break;
            }
            let Ok(entries) = fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                visited += 1;
                if visited >= MAX_ENTRIES {
                    break;
                }
                let path = entry.path();
                if path.is_dir() {
                    pending.push((path, depth + 1));
                    continue;
                }
                if let Some(name) = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_lowercase())
                {
                    by_name.entry(name).or_insert(path);
                }
            }
        }
        missing
            .into_iter()
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy().to_lowercase();
                let found = by_name.get(&name)?.clone();
                Some((path, found))
            })
            .collect()
    }

    /// Relinks missing sources from `folder` as one undo step.
    pub(super) fn relink_missing_media(&mut self, folder: &Path) {
        if self.is_recording() {
            self.status_message = "Stop recording before relinking media".to_owned();
            return;
        }
        let missing = self.data.project.missing_media();
        if missing.is_empty() {
            self.status_message = "This project has no missing media".to_owned();
            return;
        }
        let mapping = self.find_relinked_media(folder);
        if mapping.is_empty() {
            self.status_message = format!(
                "Found none of the {} missing source(s) in {}",
                missing.len(),
                folder.display()
            );
            return;
        }
        let mut candidate = self.data.project.clone();
        let relinked = candidate.relink_media(&mapping);
        if relinked == 0 {
            self.status_message = "No event referenced the files that were found".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot relink: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Relink Media");
        self.data.project = candidate;
        for (source, replacement) in &mapping {
            self.data.audio_sources.remove(source);
            if let Ok(audio) = AudioSource::open(replacement) {
                self.data.audio_sources.insert(replacement.clone(), audio);
            }
        }
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        let still_missing = self.data.project.missing_media().len();
        let mut status = format!(
            "Relinked {relinked} event(s) to {} source(s)",
            mapping.len()
        );
        if still_missing > 0 {
            status.push_str(&format!("; {still_missing} source(s) are still missing"));
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

    #[test]
    fn the_pool_search_matches_names_and_paths_case_insensitively() {
        assert!(media_matches(Path::new("/media/Kick.wav"), ""));
        assert!(media_matches(Path::new("/media/Kick.wav"), "kick"));
        assert!(media_matches(Path::new("/media/Kick.wav"), "KICK"));
        assert!(media_matches(Path::new("/media/Kick.wav"), "media/"));
        assert!(!media_matches(Path::new("/media/Kick.wav"), "snare"));
        // Whitespace-only is no filter at all.
        assert!(media_matches(Path::new("/media/Kick.wav"), "   "));
        // Whitespace around a term is ignored.
        assert!(media_matches(Path::new("/media/Kick.wav"), "  kick  "));
    }

    #[test]
    fn the_pool_filters_rows_by_the_search_term() {
        let mut project = Project::default();
        project.media_pool.insert(PathBuf::from("/media/kick.wav"));
        project.media_pool.insert(PathBuf::from("/media/snare.wav"));

        let ctx = egui::Context::default();
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut search = "kick".to_owned();
            let mut focus = false;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(800.0, 600.0),
                    )),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |ui| {
                    assert!(show(ui, &project, None, &mut search, &mut focus).is_none());
                },
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
        assert!(joined.contains("kick.wav"), "{joined}");
        assert!(
            !joined.contains("snare.wav"),
            "the search hides non-matching rows: {joined}"
        );
        assert!(joined.contains("Search"), "{joined}");
    }

    #[test]
    fn the_pool_marks_and_scrolls_to_a_revealed_source() {
        let mut project = Project::default();
        project.media_pool.insert(PathBuf::from("/media/kick.wav"));
        project.media_pool.insert(PathBuf::from("/media/snare.wav"));

        let ctx = egui::Context::default();
        let revealed = PathBuf::from("/media/kick.wav");
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(800.0, 600.0),
                    )),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |ui| {
                    let mut search = String::new();
                    let mut focus = false;
                    assert!(
                        show(
                            ui,
                            &project,
                            Some(revealed.as_path()),
                            &mut search,
                            &mut focus
                        )
                        .is_none()
                    );
                },
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
        assert!(joined.contains("▶ kick.wav"), "{joined}");
        assert!(
            !joined.contains("▶ snare.wav"),
            "only the revealed file is marked: {joined}"
        );
        assert!(
            joined.contains("snare.wav"),
            "the other file is still listed"
        );
    }

    #[test]
    fn the_manifest_lists_paths_with_usage_and_imports_them_back() {
        let root =
            std::env::temp_dir().join(format!("opentrack-manifest-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let used = root.join("used.wav");
        let spare = root.join("spare.wav");
        let absent = root.join("absent.wav");
        std::fs::write(&used, b"a").expect("used");
        std::fs::write(&spare, b"b").expect("spare");

        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, used.clone(), 0.0, 2.0)
            .expect("clip");
        project
            .add_audio_clip(0, used.clone(), 4.0, 2.0)
            .expect("clip");
        project.media_pool.insert(used.clone());
        project.media_pool.insert(spare.clone());

        let text = manifest_text(&project);
        assert!(text.starts_with("# OpenTrack media manifest v1"));
        assert!(text.contains(&format!("{}\t2", used.display())), "{text}");
        assert!(text.contains(&format!("{}\t0", spare.display())), "{text}");

        // Importing the exported manifest into a project with an empty pool
        // restores both existing paths and reports the absent one.
        let mut target = Project::default();
        let text = format!("{text}{}\t0\n", absent.display());
        let report = import_manifest(&mut target, &text);
        assert_eq!(report.added, 2);
        assert_eq!(report.already_present, 0);
        assert_eq!(report.missing, vec![absent.clone()]);
        assert!(target.media_pool.contains(&used));
        assert!(target.media_pool.contains(&spare));
        assert!(!target.media_pool.contains(&absent));

        // Comments, blank lines, duplicates and one-path-per-line lists are fine.
        let report = import_manifest(
            &mut target,
            &format!(
                "# comment\n\n{}\n{}\nnot-real.wav\n",
                used.display(),
                used.display()
            ),
        );
        assert_eq!(report.added, 0);
        assert_eq!(report.already_present, 1);
        assert_eq!(report.missing.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn exporting_and_importing_the_pool_are_files_and_one_undo_step() {
        let root =
            std::env::temp_dir().join(format!("opentrack-manifest-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        create_project_directory_layout(&root).expect("layout");
        let media = root.join("Audio").join("Imports").join("take.wav");
        std::fs::write(&media, b"take").expect("media");
        let manifest = root.join("pool media.txt");

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, media.clone(), 0.0, 2.0)
            .expect("clip");
        app.project_history.clear();

        app.export_media_pool_to(&manifest);
        assert!(manifest.exists());
        let exported = std::fs::read_to_string(&manifest).expect("manifest");
        assert!(exported.contains("take.wav"));
        assert!(app.status_message.contains("Exported 1 media path(s)"));

        // Import into a pool that has been emptied first.
        app.data.project.media_pool.clear();
        app.import_media_pool_from(&manifest);
        assert!(app.data.project.media_pool.contains(&media));
        assert!(app.status_message.contains("Added 1 media path(s)"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Import Pool");

        // A second import adds nothing and writes no step.
        let steps = app.project_history.counts().0;
        app.import_media_pool_from(&manifest);
        assert!(app.status_message.contains("Nothing to import"));
        assert_eq!(app.project_history.counts().0, steps);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_search_command_opens_the_media_window_with_the_field_focused() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.search_media();
        assert!(app.media_window.open);
        assert!(app.pool_search_focus);
        assert!(app.status_message.contains("Search the Media Pool"));

        // Without a project it reports instead.
        let mut closed = OpenTrackApp::new_internal(&creation, false);
        closed.search_media();
        assert!(closed.status_message.contains("Open a project"));
    }

    #[test]
    fn find_missing_files_reports_when_nothing_is_missing() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Audio);
        // No clips, so nothing can be missing and no picker is opened.
        app.find_missing_media_dialog();
        assert_eq!(app.status_message, "This project has no missing media");
    }

    #[test]
    fn removing_unused_pool_entries_is_one_undo_step_and_keeps_the_files() {
        let root =
            std::env::temp_dir().join(format!("opentrack-pool-clean-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        create_project_directory_layout(&root).expect("layout");
        let used = root.join("Audio").join("Imports").join("used.wav");
        let unused = root.join("Audio").join("Imports").join("unused.wav");
        std::fs::write(&used, b"used").expect("used");
        std::fs::write(&unused, b"unused").expect("unused");

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, used.clone(), 0.0, 2.0)
            .expect("clip");
        app.data.project.media_pool.insert(unused.clone());
        app.project_history.clear();

        app.remove_unused_media_from_pool();
        assert!(!app.data.project.media_pool.contains(&unused));
        assert!(app.data.project.media_pool.contains(&used));
        assert!(unused.exists(), "the file stays on disk");
        assert!(app.status_message.contains("Removed 1 unused pool entry"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Remove Unused Media");

        // Running it again reports there is nothing to do.
        app.remove_unused_media_from_pool();
        assert!(app.status_message.contains("no unused entries"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn removing_missing_pool_entries_keeps_the_ones_events_still_use() {
        let root =
            std::env::temp_dir().join(format!("opentrack-pool-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        create_project_directory_layout(&root).expect("layout");
        let absent_referenced = root.join("Audio").join("gone.wav");
        let absent_unused = root.join("Audio").join("also-gone.wav");

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, absent_referenced.clone(), 0.0, 2.0)
            .expect("clip");
        app.data.project.media_pool.insert(absent_unused.clone());
        app.project_history.clear();

        app.remove_missing_media_from_pool();
        assert!(!app.data.project.media_pool.contains(&absent_unused));
        assert!(
            app.data.project.media_pool.contains(&absent_referenced),
            "an event still needs it for relinking"
        );
        assert!(app.status_message.contains("Removed 1 missing pool entry"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Remove Missing Files");

        app.remove_missing_media_from_pool();
        assert!(app.status_message.contains("no missing entries"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn emptying_unused_media_deletes_only_project_files_and_undo_restores_them() {
        let root =
            std::env::temp_dir().join(format!("opentrack-pool-empty-{}", std::process::id()));
        let outside =
            std::env::temp_dir().join(format!("opentrack-pool-outside-{}.wav", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        create_project_directory_layout(&root).expect("layout");
        let inside_unused = root.join("Audio").join("Imports").join("take.wav");
        std::fs::write(&inside_unused, b"take").expect("inside");
        std::fs::write(&outside, b"shared").expect("outside");
        let used = root.join("Audio").join("Imports").join("used.wav");
        std::fs::write(&used, b"used").expect("used");

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, used.clone(), 0.0, 2.0)
            .expect("clip");
        app.data.project.media_pool.insert(inside_unused.clone());
        app.data.project.media_pool.insert(outside.clone());
        app.project_history.clear();

        app.empty_media_trash();
        assert!(!inside_unused.exists(), "the project's unused file is gone");
        assert!(
            outside.exists(),
            "a file outside the project is never deleted"
        );
        assert!(!app.data.project.media_pool.contains(&inside_unused));
        assert!(!app.data.project.media_pool.contains(&outside));
        assert!(app.data.project.media_pool.contains(&used));
        assert!(app.status_message.contains("Deleted 1 unused file(s)"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Empty Trash");

        // Undo brings the deleted file and the pool entries back.
        let restored = app
            .project_history
            .undo(&app.data.project)
            .expect("undo the cleanup");
        assert!(inside_unused.exists(), "Undo restores the deleted file");
        assert!(restored.media_pool.contains(&inside_unused));
        assert!(restored.media_pool.contains(&outside));
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_file(outside);
    }

    #[test]
    fn relinking_finds_named_sources_in_a_moved_project_folder() {
        let root =
            std::env::temp_dir().join(format!("opentrack-relink-{}-app", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Audio")).expect("audio");
        std::fs::create_dir_all(root.join("Metadata")).expect("metadata");
        // The media lives one level down, as it would after a project was copied
        // without its Audio folder flattening.
        std::fs::create_dir_all(root.join("Audio").join("Old")).expect("old");
        let found = root.join("Audio").join("Old").join("Kick.WAV");
        write_wav(&found, 480);

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Audio);
        // Two events reference a path that no longer exists.
        let missing = root.join("Audio").join("kick.wav");
        app.data
            .project
            .add_audio_clip(0, missing.clone(), 0.0, 1.0)
            .unwrap();
        app.data
            .project
            .add_audio_clip(0, missing.clone(), 4.0, 1.0)
            .unwrap();
        app.project_history.clear();

        assert_eq!(app.data.project.missing_media(), vec![missing.clone()]);
        app.relink_missing_media(&root);

        assert_eq!(
            app.data.project.tracks[0].clips[0].source_path, found,
            "the events point at the found file"
        );
        assert!(app.data.project.missing_media().is_empty());
        assert_eq!(
            app.data
                .project
                .media_pool
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![found.clone()],
            "the pool followed the relink"
        );
        assert!(
            app.data.audio_sources.contains_key(&found),
            "the relinked source is ready to play"
        );
        assert!(app.status_message.contains("Relinked 2 event(s)"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Relink Media");

        // A folder without the file reports instead of guessing.
        app.data.project.tracks[0].clips[0].source_path = root.join("Audio").join("gone.wav");
        app.data.project.register_media();
        let empty = root.join("Metadata");
        app.relink_missing_media(&empty);
        assert!(app.status_message.contains("Found none of the"));

        // A project with nothing missing says so.
        for clip in app.data.project.tracks[0].clips.iter_mut() {
            clip.source_path = found.clone();
        }
        app.data.project.media_pool.clear();
        app.data.project.register_media();
        app.relink_missing_media(&root);
        assert!(app.status_message.contains("no missing media"));
        let _ = std::fs::remove_dir_all(root);
    }
}
