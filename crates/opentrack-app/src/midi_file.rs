use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use opentrack_core::{
    midi_file::{MidiExportReport, MidiImportReport, export_standard_midi, import_standard_midi},
    project::Project,
};

const MAX_MIDI_FILE_BYTES: u64 = 128 * 1024 * 1024;

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

    pub(super) fn export_midi_file(&mut self) {
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
            .set_title("Export Standard MIDI File")
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
                let result = export_standard_midi(&project)
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
        self.status_message = "Exporting MIDI file...".to_owned();
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
