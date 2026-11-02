use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use opentrack_core::{
    midi_file::{
        MidiExportReport, MidiExportScope, MidiImportReport, export_standard_midi,
        export_standard_midi_scope, import_standard_midi,
    },
    project::Project,
};

const MAX_MIDI_FILE_BYTES: u64 = 128 * 1024 * 1024;

/// What the Export MIDI Selection dialog is set to export.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExportChoice {
    /// The selected MIDI part only.
    SelectedPart,
    /// Every part on the selected MIDI track(s).
    SelectedTrack,
    /// Every MIDI track in the project, like the plain MIDI File command.
    WholeProject,
}

impl ExportChoice {
    pub(super) const ALL: [Self; 3] = [Self::SelectedPart, Self::SelectedTrack, Self::WholeProject];

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::SelectedPart => "Selected MIDI part",
            Self::SelectedTrack => "Selected MIDI track(s)",
            Self::WholeProject => "Whole project",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExportDialog {
    pub choice: ExportChoice,
    pub use_loop_range: bool,
}

pub(super) struct Task {
    receiver: Receiver<TaskResult>,
    base_project: Option<Project>,
    path: PathBuf,
}

enum TaskResult {
    Import(Result<(Project, MidiImportReport), String>),
    Export(Result<MidiExportReport, String>),
}

impl super::OpenTrackApp {
    pub(super) fn import_midi_file(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before importing a MIDI file".to_owned();
            return;
        }
        if self.midi_file_task.is_some() {
            self.status_message = "Wait for the current MIDI file operation to finish".to_owned();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Standard MIDI Files", &["mid", "midi"])
            .set_title("Import Standard MIDI File")
            .pick_file()
        else {
            return;
        };
        let base_project = self.data.project.clone();
        let worker_project = base_project.clone();
        let worker_path = path.clone();
        let (sender, receiver) = mpsc::channel();
        if let Err(error) = thread::Builder::new()
            .name("opentrack-midi-file-import".to_owned())
            .spawn(move || {
                let result = (|| {
                    let size = fs::metadata(&worker_path)
                        .map_err(|error| format!("could not read MIDI file information: {error}"))?
                        .len();
                    if size > MAX_MIDI_FILE_BYTES {
                        return Err("MIDI file exceeds the 128 MiB import limit".to_owned());
                    }
                    let bytes = fs::read(&worker_path)
                        .map_err(|error| format!("could not read MIDI file: {error}"))?;
                    let mut project = worker_project;
                    let report = import_standard_midi(&mut project, &bytes)
                        .map_err(|error| error.to_string())?;
                    Ok((project, report))
                })();
                let _ = sender.send(TaskResult::Import(result));
            })
        {
            self.status_message = format!("Could not start MIDI import: {error}");
            return;
        }
        self.midi_file_task = Some(Task {
            receiver,
            base_project: Some(base_project),
            path,
        });
        self.status_message = "Importing MIDI file...".to_owned();
    }

    /// Imports a partwise MusicXML score as MIDI tracks.
    ///
    /// The import runs synchronously because the document is bounded to 8 MiB and
    /// parsing is a single linear pass; a larger file is refused before parsing.
    pub(super) fn import_musicxml_file(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project before importing MusicXML".to_owned();
            return;
        }
        if self.is_recording() {
            self.status_message = "Stop recording before importing MusicXML".to_owned();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MusicXML", &["musicxml", "xml"])
            .set_title("Import MusicXML")
            .pick_file()
        else {
            return;
        };
        self.import_musicxml_from_path(&path);
    }

    /// Imports `path` as one undo step; split out so tests skip the dialog.
    pub(super) fn import_musicxml_from_path(&mut self, path: &std::path::Path) {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Could not read the MusicXML file: {error}");
                return;
            }
        };
        let import = match opentrack_core::musicxml::import_musicxml(&bytes) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("MusicXML import failed: {error}");
                return;
            }
        };
        let mut candidate = self.data.project.clone();
        let first_track = candidate.tracks.len();
        let mut first_part = None;
        let mut imported_parts = 0;
        let mut imported_notes = 0;
        for part in &import.parts {
            candidate.add_track(opentrack_core::project::TrackKind::Midi);
            let index = candidate.tracks.len() - 1;
            candidate.tracks[index].name = part.name.clone();
            let Some(part_id) =
                candidate.add_midi_part(index, part.start_beats, part.duration_beats)
            else {
                self.status_message = "The MusicXML parts could not be placed".to_owned();
                return;
            };
            first_part.get_or_insert(part_id);
            for note in &part.notes {
                if candidate
                    .add_midi_note_on_channel(
                        index,
                        part_id,
                        note.start_beats,
                        note.duration_beats,
                        note.pitch,
                        0,
                        note.velocity,
                    )
                    .is_none()
                {
                    self.status_message =
                        "A MusicXML note could not be placed in its part".to_owned();
                    return;
                }
                imported_notes += 1;
            }
            imported_parts += 1;
        }
        if candidate.validate().is_err() {
            self.status_message = "The imported score would not form a valid project".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot import MusicXML: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Import MusicXML");
        self.data.project = candidate;
        self.arrange_view_state.selected_track_ids.clear();
        if let Some(index) = self.data.project.tracks.get(first_track) {
            self.data.project.selected_track = Some(first_track as u32);
            self.arrange_view_state.selected_track_ids.insert(index.id);
            self.arrange_view_state.selection_anchor_track_id = Some(index.id);
        }
        self.data.project.selected_midi_part_id = first_part;
        self.selected_midi_note_ids.clear();
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        let mut status =
            format!("Imported {imported_parts} MusicXML part(s) with {imported_notes} note(s)");
        if let Some(tempo) = import.tempo_bpm {
            status.push_str(&format!(
                "; the score names {tempo:.0} BPM, project tempo unchanged"
            ));
        }
        if !import.unsupported.is_empty() {
            status.push_str(&format!(
                "; not imported: {}",
                import.unsupported.join(", ")
            ));
        }
        self.status_message = status;
    }

    pub(super) fn export_midi_file(&mut self) {
        self.start_midi_export(MidiExportScope::default(), "MIDI file");
    }

    /// Starts a scoped MIDI export after the user picks a destination.
    pub(super) fn start_midi_export(&mut self, scope: MidiExportScope, what: &str) {
        if self.is_recording() {
            self.status_message = "Stop recording before exporting a MIDI file".to_owned();
            return;
        }
        if self.midi_file_task.is_some() {
            self.status_message = "Wait for the current MIDI file operation to finish".to_owned();
            return;
        }
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("Standard MIDI Files", &["mid", "midi"])
            .set_file_name(format!("{}.mid", self.data.project.name))
            .set_title(format!("Export {what}"))
            .save_file()
        else {
            return;
        };
        path.set_extension("mid");
        let project = self.data.project.clone();
        let worker_path = path.clone();
        let (sender, receiver) = mpsc::channel();
        if let Err(error) = thread::Builder::new()
            .name("opentrack-midi-file-export".to_owned())
            .spawn(move || {
                let result = export_standard_midi_scope(&project, &scope)
                    .map_err(|error| error.to_string())
                    .and_then(|(bytes, report)| {
                        crate::project_file::atomic_write(&worker_path, &bytes)
                            .map(|()| report)
                            .map_err(|error| format!("could not write MIDI file: {error}"))
                    });
                let _ = sender.send(TaskResult::Export(result));
            })
        {
            self.status_message = format!("Could not start MIDI export: {error}");
            return;
        }
        self.midi_file_task = Some(Task {
            receiver,
            base_project: None,
            path,
        });
        self.status_message = format!("Exporting {what}...");
    }

    /// Builds the export scope the dialog is set to, or explains what is missing.
    pub(super) fn midi_export_scope(
        &self,
        dialog: ExportDialog,
    ) -> Result<MidiExportScope, String> {
        let project = &self.data.project;
        let mut scope = MidiExportScope::default();
        match dialog.choice {
            ExportChoice::SelectedPart => {
                let part_id = project
                    .selected_midi_part_id
                    .ok_or_else(|| "Select a MIDI part to export".to_owned())?;
                scope.part_ids = vec![part_id];
            }
            ExportChoice::SelectedTrack => {
                let mut track_ids = self
                    .arrange_view_state
                    .selected_track_ids
                    .iter()
                    .copied()
                    .collect::<Vec<_>>();
                if track_ids.is_empty()
                    && let Some(track) = project
                        .selected_track
                        .and_then(|index| project.tracks.get(index as usize))
                {
                    track_ids.push(track.id);
                }
                let midi_tracks = project
                    .tracks
                    .iter()
                    .filter(|track| {
                        track_ids.contains(&track.id)
                            && matches!(
                                track.kind,
                                opentrack_core::project::TrackKind::Midi
                                    | opentrack_core::project::TrackKind::Instrument
                            )
                            && !track.midi_parts.is_empty()
                    })
                    .map(|track| track.id)
                    .collect::<Vec<_>>();
                if midi_tracks.is_empty() {
                    return Err("Select a MIDI track with at least one part".to_owned());
                }
                scope.track_ids = midi_tracks;
            }
            ExportChoice::WholeProject => {}
        }
        if dialog.use_loop_range {
            let (start, end) = (project.loop_start_beats, project.loop_end_beats);
            if !start.is_finite() || !end.is_finite() || end <= start {
                return Err("Set the loop range before exporting it".to_owned());
            }
            scope.start_beats = Some(start);
            scope.end_beats = Some(end);
        }
        Ok(scope)
    }

    pub(super) fn show_midi_export_dialog(&mut self, ctx: &eframe::egui::Context) {
        let Some(mut dialog) = self.midi_export_dialog.take() else {
            return;
        };
        let mut open = true;
        let mut export = false;
        let mut cancel = false;
        let response = eframe::egui::Window::new("Export MIDI Selection")
            .id(eframe::egui::Id::new("midi-export-selection"))
            .order(eframe::egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.label("What to export");
                for choice in ExportChoice::ALL {
                    ui.radio_value(&mut dialog.choice, choice, choice.label());
                }
                ui.separator();
                ui.checkbox(&mut dialog.use_loop_range, "Limit to the loop range");
                ui.label(
                    eframe::egui::RichText::new(
                        "A note crossing the range is clipped to it, not dropped.",
                    )
                    .small()
                    .color(crate::theme::palette(ui).muted),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Export...").clicked() {
                        export = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        super::dismiss_window_on_outside_click(
            ctx,
            eframe::egui::Id::new("midi-export-selection"),
            &mut open,
            response.as_ref(),
        );
        if export {
            match self.midi_export_scope(dialog) {
                Ok(scope) => {
                    let name = match self.data.project.selected_midi_part_id {
                        Some(_) if dialog.choice == ExportChoice::SelectedPart => "MIDI part",
                        _ => "MIDI selection",
                    };
                    self.start_midi_export(scope, name);
                }
                Err(error) => self.status_message = error,
            }
        } else if !cancel && open {
            self.midi_export_dialog = Some(dialog);
        }
    }

    pub(super) fn poll_midi_file_task(&mut self) {
        let result = match self.midi_file_task.as_ref() {
            Some(task) => match task.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(TaskResult::Export(Err(
                    "MIDI file worker stopped before returning a result".to_owned(),
                ))),
            },
            None => return,
        };
        let Some(result) = result else {
            return;
        };
        let task = self
            .midi_file_task
            .take()
            .expect("MIDI file task is present");
        match result {
            TaskResult::Import(Ok((project, report))) => {
                let base_project = task
                    .base_project
                    .expect("import task retains its base project");
                let mut base_content = base_project.clone();
                base_content.selected_track = self.data.project.selected_track;
                base_content.selected_midi_part_id = self.data.project.selected_midi_part_id;
                if self.data.project != base_content {
                    self.status_message = "MIDI import was discarded because the project changed while the file was being read".to_owned();
                    return;
                }
                if report.tracks_added == 0 {
                    self.status_message = format!(
                        "No supported note or controller events found in {}{}",
                        task.path.display(),
                        ignored_summary(&report),
                    );
                    return;
                }
                if !self
                    .project_history
                    .begin(&self.data.project, self.modified)
                {
                    self.status_message =
                        "MIDI import could not be added to undo history".to_owned();
                    return;
                }
                self.data.project = project;
                self.project_history.commit_after_frame = true;
                self.mark_modified();
                self.rebuild_audio_plan();
                self.sync_arrange_track_selection();
                self.status_message = format!(
                    "Imported {} track(s), {} note(s), and {} controller event(s) from {}{}",
                    report.tracks_added,
                    report.notes_imported,
                    report.controllers_imported,
                    task.path.display(),
                    ignored_summary(&report),
                );
            }
            TaskResult::Import(Err(error)) | TaskResult::Export(Err(error)) => {
                self.status_message = format!("MIDI file operation failed: {error}");
            }
            TaskResult::Export(Ok(report)) => {
                self.status_message = format!(
                    "Exported {} MIDI track(s), {} note(s), and {} controller event(s) to {}",
                    report.tracks_written.saturating_sub(1),
                    report.notes_written,
                    report.controllers_written,
                    task.path.display(),
                );
            }
        }
    }
}

fn ignored_summary(report: &MidiImportReport) -> String {
    if report.ignored_events == 0 {
        String::new()
    } else {
        format!(
            "; ignored {} event(s): {} tempo, {} other metadata, {} SysEx, {} unsupported channel messages, {} unmatched notes, {} duplicate controller points",
            report.ignored_events,
            report.tempo_events_ignored,
            report.meta_events_ignored,
            report.sysex_events_ignored,
            report.unsupported_channel_events,
            report.unmatched_note_events,
            report.duplicate_controller_events,
        )
    }
}

#[cfg(test)]
mod musicxml_import_tests {
    use super::*;
    use crate::app::OpenTrackApp;

    const SCORE: &str = r#"<score-partwise version="4.0">
      <part-list><score-part id="P1"><part-name>Melody</part-name></score-part></part-list>
      <part id="P1">
        <measure>
          <attributes><divisions>1</divisions></attributes>
          <note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note>
          <note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note>
        </measure>
      </part>
    </score-partwise>"#;

    #[test]
    fn importing_a_score_adds_midi_tracks_and_is_one_undo_step() {
        let root =
            std::env::temp_dir().join(format!("opentrack-musicxml-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let path = root.join("score.musicxml");
        std::fs::write(&path, SCORE).expect("score");

        let ctx = eframe::egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_history.clear();

        app.import_musicxml_from_path(&path);
        assert_eq!(app.data.project.tracks.len(), 1);
        assert_eq!(app.data.project.tracks[0].name, "Melody");
        let part = &app.data.project.tracks[0].midi_parts[0];
        assert_eq!(part.notes.len(), 2);
        assert_eq!(part.notes[0].pitch, 60);
        assert_eq!(part.notes[1].start_beats, 1.0);
        assert_eq!(app.data.project.selected_track, Some(0));
        assert!(app.data.project.selected_midi_part_id.is_some());
        assert!(
            app.status_message
                .contains("1 MusicXML part(s) with 2 note(s)")
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Import MusicXML");

        // A malformed file reports and adds nothing.
        let bad = root.join("bad.musicxml");
        std::fs::write(&bad, b"<score-partwise><part id=\"P1\">").expect("bad");
        let before = app.data.project.tracks.len();
        let steps = app.project_history.counts().0;
        app.import_musicxml_from_path(&bad);
        assert!(app.status_message.contains("MusicXML import failed"));
        assert_eq!(app.data.project.tracks.len(), before);
        assert_eq!(app.project_history.counts().0, steps);
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod export_selection_tests {
    use super::*;
    use crate::app::OpenTrackApp;
    use opentrack_core::project::TrackKind;

    fn app_with_parts() -> OpenTrackApp {
        let ctx = eframe::egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Midi);
        app.data.project.add_track(TrackKind::Audio);
        let part = app.data.project.add_midi_part(0, 0.0, 4.0).expect("part");
        app.data
            .project
            .add_midi_note_on_channel(0, part, 0.0, 1.0, 60, 0, 100)
            .expect("note");
        app.data.project.selected_midi_part_id = Some(part);
        app.data.project.selected_track = Some(0);
        app
    }

    #[test]
    fn the_scope_builder_follows_the_dialog_choice() {
        let app = app_with_parts();
        let part = app.data.project.selected_midi_part_id.expect("part");

        let scope = app
            .midi_export_scope(ExportDialog {
                choice: ExportChoice::SelectedPart,
                use_loop_range: false,
            })
            .expect("part scope");
        assert_eq!(scope.part_ids, vec![part]);
        assert!(scope.track_ids.is_empty());
        assert!(!scope.is_whole_project());

        let scope = app
            .midi_export_scope(ExportDialog {
                choice: ExportChoice::SelectedTrack,
                use_loop_range: false,
            })
            .expect("track scope");
        assert_eq!(scope.track_ids, vec![app.data.project.tracks[0].id]);
        assert!(scope.part_ids.is_empty());

        let scope = app
            .midi_export_scope(ExportDialog {
                choice: ExportChoice::WholeProject,
                use_loop_range: false,
            })
            .expect("whole project");
        assert!(scope.is_whole_project());
    }

    #[test]
    fn the_loop_range_limits_the_scope_and_is_validated() {
        let mut app = app_with_parts();
        app.data.project.set_loop_range(1.0, 3.0);
        let scope = app
            .midi_export_scope(ExportDialog {
                choice: ExportChoice::WholeProject,
                use_loop_range: true,
            })
            .expect("range");
        assert_eq!(scope.start_beats, Some(1.0));
        assert_eq!(scope.end_beats, Some(3.0));

        app.data.project.loop_start_beats = 3.0;
        app.data.project.loop_end_beats = 3.0;
        let error = app
            .midi_export_scope(ExportDialog {
                choice: ExportChoice::WholeProject,
                use_loop_range: true,
            })
            .expect_err("an empty range is refused");
        assert!(error.contains("loop range"));
    }

    #[test]
    fn a_choice_without_a_target_is_explained() {
        let mut app = app_with_parts();
        app.data.project.selected_midi_part_id = None;
        let error = app
            .midi_export_scope(ExportDialog {
                choice: ExportChoice::SelectedPart,
                use_loop_range: false,
            })
            .expect_err("no part");
        assert!(error.contains("Select a MIDI part"));

        // Selecting only the audio track cannot export a MIDI track.
        app.data.project.selected_track = Some(1);
        app.arrange_view_state.selected_track_ids.clear();
        let error = app
            .midi_export_scope(ExportDialog {
                choice: ExportChoice::SelectedTrack,
                use_loop_range: false,
            })
            .expect_err("no midi track");
        assert!(error.contains("MIDI track"));
    }

    #[test]
    fn the_selection_dialog_lists_every_scope() {
        let ctx = eframe::egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = app_with_parts();
        app.midi_export_dialog = Some(ExportDialog {
            choice: ExportChoice::SelectedPart,
            use_loop_range: false,
        });

        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                eframe::egui::RawInput {
                    screen_rect: Some(eframe::egui::Rect::from_min_size(
                        eframe::egui::Pos2::ZERO,
                        eframe::egui::Vec2::new(900.0, 700.0),
                    )),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_midi_export_dialog(&ctx),
            );
            output.textures_delta.clear();
            if frame == 0 {
                continue;
            }
            fn collect(shape: &eframe::egui::Shape, texts: &mut Vec<String>) {
                match shape {
                    eframe::egui::Shape::Text(text) => texts.push(text.galley.text().to_owned()),
                    eframe::egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect(shape, texts);
                        }
                    }
                    _ => {}
                }
            }
            for shape in &output.shapes {
                collect(&shape.shape, &mut texts);
            }
        }
        let joined = texts.join("\n");
        assert!(joined.contains("Export MIDI Selection"), "{joined}");
        for choice in ExportChoice::ALL {
            assert!(joined.contains(choice.label()), "{joined}");
        }
        assert!(joined.contains("Limit to the loop range"), "{joined}");
    }
}
