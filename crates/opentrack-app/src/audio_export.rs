use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
};

use eframe::egui;
use opentrack_audio::{
    AudioSource, OfflineRenderer, PlaybackSettingsSnapshot, WavExportReport, write_mixdown_wav,
};
use opentrack_core::project::{Project, RecordingFormat};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    Pcm16,
    Pcm24,
    Float32,
}

impl Format {
    fn recording_format(self) -> RecordingFormat {
        match self {
            Self::Pcm16 => RecordingFormat::Pcm16,
            Self::Pcm24 => RecordingFormat::Pcm24,
            Self::Float32 => RecordingFormat::Float32,
        }
    }
}

pub(super) struct Dialog {
    start_beats: f64,
    end_beats: f64,
    format: Format,
    sample_rate: u32,
    open: bool,
    /// Render one WAV per track into a folder instead of one mixdown file.
    stems: bool,
}

impl Dialog {
    pub(super) fn for_project(project: &Project) -> Self {
        Self {
            start_beats: 0.0,
            end_beats: project_end_beats(project).max(4.0),
            format: Format::Float32,
            sample_rate: project.sample_rate_hz.max(1),
            open: true,
            stems: false,
        }
    }

    pub(super) fn for_stems(project: &Project) -> Self {
        Self {
            stems: true,
            ..Self::for_project(project)
        }
    }
}

pub(super) struct Task {
    /// The mixdown file, or the folder the stems were written into.
    destination: PathBuf,
    /// How many files the task is expected to write.
    expected_files: usize,
    result: Receiver<Result<Vec<(PathBuf, WavExportReport)>, String>>,
    progress: Arc<Mutex<(u64, u64)>>,
    cancel: Arc<AtomicBool>,
}

impl Drop for Task {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl super::OpenTrackApp {
    pub(super) fn open_audio_export_dialog(&mut self) {
        self.audio_export_dialog = Some(Dialog::for_project(&self.data.project));
    }

    pub(super) fn poll_audio_export_task(&mut self) {
        let result = match self.audio_export_task.as_ref() {
            Some(task) => match task.result.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("audio export worker stopped unexpectedly".to_owned()))
                }
            },
            None => return,
        };
        if let Some(result) = result {
            let task = self
                .audio_export_task
                .take()
                .expect("export task is present");
            self.status_message = match result {
                Ok(reports) if reports.len() == 1 => {
                    let (path, report) = &reports[0];
                    format!(
                        "Exported {} frames at {} Hz to {} (peak {:.1} dBFS)",
                        report.frames,
                        report.sample_rate,
                        path.display(),
                        if report.peak_linear > 0.0 {
                            20.0 * report.peak_linear.log10()
                        } else {
                            f32::NEG_INFINITY
                        },
                    )
                }
                Ok(reports) if reports.len() < task.expected_files => format!(
                    "Exported {} of {} expected stem(s) to {}; the rest had no audio",
                    reports.len(),
                    task.expected_files,
                    task.destination.display()
                ),
                Ok(reports) => format!(
                    "Exported {} stem(s) to {}",
                    reports.len(),
                    task.destination.display()
                ),
                Err(error) => format!("Audio export failed: {error}"),
            };
        }
    }

    pub(super) fn show_audio_export(&mut self, ctx: &egui::Context) {
        if let Some(task) = self.bounce_task.as_ref() {
            let (done, total) = task
                .progress
                .lock()
                .map(|progress| *progress)
                .unwrap_or_default();
            egui::Window::new("Bounce Selection")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Rendering the selected range through this track's channel...");
                    let fraction = if total == 0 {
                        0.0
                    } else {
                        done as f32 / total as f32
                    };
                    ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).show_percentage());
                    if ui.button("Cancel Bounce").clicked() {
                        task.cancel.store(true, Ordering::Release);
                    }
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
            return;
        }

        if self.audio_export_task.is_some() {
            let (done, total) = self
                .audio_export_task
                .as_ref()
                .and_then(|task| task.progress.lock().ok().map(|progress| *progress))
                .unwrap_or_default();
            egui::Window::new("Audio Mixdown")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Rendering the project playback graph...");
                    let fraction = if total == 0 {
                        0.0
                    } else {
                        done as f32 / total as f32
                    };
                    ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).show_percentage());
                    ui.label(format!("{done} / {total} frames"));
                    if ui.button("Cancel Export").clicked() {
                        if let Some(task) = &self.audio_export_task {
                            task.cancel.store(true, Ordering::Release);
                        }
                    }
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
            return;
        }

        let Some(dialog) = self.audio_export_dialog.as_mut() else {
            return;
        };
        let mut begin = false;
        let mut close = false;
        egui::Window::new("Audio Mixdown")
            .collapsible(false)
            .resizable(false)
            .open(&mut dialog.open)
            .show(ctx, |ui| {
                ui.label(if dialog.stems {
                    "Render every audible track to its own stereo WAV inside a folder."
                } else {
                    "Render the project timeline to a stereo WAV file."
                });
                ui.checkbox(
                    &mut dialog.stems,
                    "Stems: one WAV per track (choose a folder)",
                );
                ui.horizontal(|ui| {
                    ui.label("Start (beats)");
                    ui.add(
                        egui::DragValue::new(&mut dialog.start_beats)
                            .speed(0.25)
                            .range(0.0..=1_000_000.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("End (beats)");
                    ui.add(
                        egui::DragValue::new(&mut dialog.end_beats)
                            .speed(0.25)
                            .range(0.0..=1_000_000.0),
                    );
                });
                egui::ComboBox::from_label("WAV format")
                    .selected_text(match dialog.format {
                        Format::Pcm16 => "PCM 16-bit",
                        Format::Pcm24 => "PCM 24-bit",
                        Format::Float32 => "32-bit float",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut dialog.format, Format::Pcm16, "PCM 16-bit");
                        ui.selectable_value(&mut dialog.format, Format::Pcm24, "PCM 24-bit");
                        ui.selectable_value(&mut dialog.format, Format::Float32, "32-bit float");
                    });
                egui::ComboBox::from_label("Sample rate")
                    .selected_text(format!("{} Hz", dialog.sample_rate))
                    .show_ui(ui, |ui| {
                        for rate in [44_100, 48_000, 88_200, 96_000, 176_400, 192_000] {
                            ui.selectable_value(
                                &mut dialog.sample_rate,
                                rate,
                                format!("{rate} Hz"),
                            );
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button("Render...").clicked() {
                        begin = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        if close || !dialog.open {
            self.audio_export_dialog = None;
            return;
        }
        if begin {
            let dialog = self.audio_export_dialog.take().expect("dialog is present");
            if !dialog.start_beats.is_finite()
                || !dialog.end_beats.is_finite()
                || dialog.start_beats < 0.0
                || dialog.end_beats <= dialog.start_beats
            {
                self.status_message = "Choose a finite, non-empty beat range".to_owned();
                self.audio_export_dialog = Some(dialog);
                return;
            }
            if dialog.stems {
                let Some(folder) = rfd::FileDialog::new()
                    .set_title("Choose a folder for the stems")
                    .pick_folder()
                else {
                    self.audio_export_dialog = Some(dialog);
                    return;
                };
                if let Err(error) = self.start_audio_export(
                    folder,
                    dialog.start_beats,
                    dialog.end_beats,
                    dialog.format.recording_format(),
                    dialog.sample_rate,
                    true,
                ) {
                    self.status_message = format!("Could not start stem export: {error}");
                }
                return;
            }
            let Some(mut destination) = rfd::FileDialog::new()
                .add_filter("WAV audio", &["wav"])
                .set_file_name(format!("{}.wav", self.data.project.name))
                .save_file()
            else {
                self.audio_export_dialog = Some(dialog);
                return;
            };
            destination.set_extension("wav");
            if let Err(error) = self.start_audio_export(
                destination,
                dialog.start_beats,
                dialog.end_beats,
                dialog.format.recording_format(),
                dialog.sample_rate,
                false,
            ) {
                self.status_message = format!("Could not start audio export: {error}");
            }
        }
    }

    /// Renders the selected events' range through their track chain into one new
    /// event on that track.
    ///
    /// The replaced events leave the arrangement but keep their files (the media
    /// pool still lists them), so a bounce is recoverable beyond Undo.
    pub(super) fn start_bounce_in_place(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before bouncing a selection".to_owned();
            return;
        }
        if self.bounce_task.is_some() {
            self.status_message = "Wait for the running bounce to finish".to_owned();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to bounce".to_owned();
            return;
        }
        let selected = ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut target = None;
        for (index, track) in self.data.project.tracks.iter().enumerate() {
            let found = track
                .clips
                .iter()
                .filter(|clip| selected.contains(&clip.id))
                .count();
            if found == 0 {
                continue;
            }
            if target.is_some() {
                self.status_message = "Bounce works on events from one track at a time".to_owned();
                return;
            }
            target = Some((index, found));
        }
        let Some((track_index, found)) = target else {
            self.status_message = "The selection no longer contains audio events".to_owned();
            return;
        };
        if found != selected.len() {
            self.status_message =
                "Bounce works on audio events from one track at a time".to_owned();
            return;
        }
        let tempo = self.data.project.tempo_bpm.max(1.0);
        let (start_beats, end_beats) = self.data.project.tracks[track_index]
            .clips
            .iter()
            .filter(|clip| selected.contains(&clip.id))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(start, end), clip| {
                (
                    start.min(clip.start_beats),
                    end.max(clip.start_beats + clip.duration_seconds * tempo / 60.0),
                )
            });
        if !start_beats.is_finite() || !end_beats.is_finite() || end_beats <= start_beats {
            self.status_message = "The selected events have no usable range".to_owned();
            return;
        }
        if !self.project_folder_managed {
            self.begin_project_folder_setup();
            self.status_message = "Set up this project's folder before bouncing".to_owned();
            return;
        }
        let Some(root) = self.project_root() else {
            self.status_message = "Save this project into its folder before bouncing".to_owned();
            return;
        };
        let Some(snapshot) = self.offline_playback_settings.clone() else {
            self.status_message =
                "The project playback graph is not ready yet; try again in a moment".to_owned();
            return;
        };
        let mut project = self.data.project.clone();
        self.vst3_runtime.capture_states(&mut project);
        let track_id = project.tracks[track_index].id;
        let track_name = project.tracks[track_index].name.clone();
        let destination = match unique_wav_path(&root, &format!("{track_name} bounce")) {
            Ok(path) => path,
            Err(error) => {
                self.status_message = format!("Could not choose a bounce file: {error}");
                return;
            }
        };
        let duration_seconds = (end_beats - start_beats) * 60.0 / tempo;
        let sample_rate = self.data.project.sample_rate_hz.max(1);
        let progress = Arc::new(Mutex::new((0, 0)));
        let cancel = Arc::new(AtomicBool::new(false));
        let (sender, result) = mpsc::channel();
        let worker_progress = Arc::clone(&progress);
        let worker_cancel = Arc::clone(&cancel);
        let worker_destination = destination.clone();
        thread::Builder::new()
            .name("opentrack-bounce".to_owned())
            .spawn(move || {
                // The bounce is one track's channel, exactly like its stem.
                let Some(settings) = stem_snapshot(&snapshot, track_index) else {
                    let _ = sender.send(Err("the track has no playback settings".to_owned()));
                    return;
                };
                let mut bounce_project = project;
                bounce_project.tracks.retain(|track| track.id == track_id);
                let outcome = render_export(
                    bounce_project,
                    settings,
                    worker_destination,
                    start_beats,
                    end_beats,
                    sample_rate,
                    RecordingFormat::Float32,
                    &worker_cancel,
                    &worker_progress,
                );
                let _ = sender.send(outcome);
            })
            .map_err(|error| error.to_string())
            .ok();
        self.status_message = format!(
            "Bouncing {:.2} s of {} through its channel...",
            duration_seconds, track_name
        );
        self.bounce_task = Some(BounceTask {
            destination,
            track_id,
            replaced: ids,
            start_beats,
            duration_seconds,
            result,
            progress,
            cancel,
        });
    }

    /// Finishes a bounce: places the rendered event as one undo step.
    pub(super) fn poll_bounce_task(&mut self) {
        let result = match self.bounce_task.as_ref() {
            Some(task) => match task.result.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("bounce worker stopped unexpectedly".to_owned()))
                }
            },
            None => return,
        };
        let Some(outcome) = result else {
            return;
        };
        let task = self.bounce_task.take().expect("bounce task is present");
        let report = match outcome {
            Ok(report) => report,
            Err(error) => {
                let _ = std::fs::remove_file(&task.destination);
                self.status_message = format!("Bounce failed: {error}");
                return;
            }
        };
        let mut candidate = self.data.project.clone();
        let Some(new_id) = candidate.replace_audio_clips_with_bounce(
            task.track_id,
            &task.replaced,
            task.destination.clone(),
            task.start_beats,
            task.duration_seconds,
        ) else {
            let _ = std::fs::remove_file(&task.destination);
            self.status_message =
                "The selected events could not be replaced (a locked event blocks the bounce)"
                    .to_owned();
            return;
        };
        match AudioSource::open(&task.destination) {
            Ok(source) => {
                self.data
                    .audio_sources
                    .insert(task.destination.clone(), source);
            }
            Err(error) => {
                let _ = std::fs::remove_file(&task.destination);
                self.status_message = format!("The bounced file could not be opened: {error}");
                return;
            }
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot bounce: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Bounce in Place");
        let track_index = candidate
            .tracks
            .iter()
            .position(|track| track.id == task.track_id);
        self.data.project = candidate;
        if let Some(track_index) = track_index {
            self.data.project.selected_track = Some(track_index as u32);
        }
        self.arrange_view_state.audio_events.select_many([new_id]);
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Bounced {} event(s) into {}; Undo restores the originals",
            task.replaced.len(),
            task.destination.display()
        );
    }

    fn start_audio_export(
        &mut self,
        destination: PathBuf,
        start_beats: f64,
        end_beats: f64,
        format: RecordingFormat,
        sample_rate: u32,
        stems: bool,
    ) -> Result<(), String> {
        if let Some(path) =
            super::unresolved_audio_source_paths(&self.data.project, &self.data.audio_sources)
                .into_iter()
                .next()
        {
            return Err(format!("media source is unavailable: {}", path.display()));
        }
        let snapshot = self.offline_playback_settings.clone().ok_or_else(|| {
            "the project playback graph is not ready yet; wait for playback setup to finish"
                .to_owned()
        })?;
        let mut project = self.data.project.clone();
        self.vst3_runtime.capture_states(&mut project);
        let progress = Arc::new(Mutex::new((0, 0)));
        let cancel = Arc::new(AtomicBool::new(false));
        let (sender, result) = mpsc::channel();
        let worker_progress = Arc::clone(&progress);
        let worker_cancel = Arc::clone(&cancel);
        let worker_destination = destination.clone();
        let task_destination = destination.clone();
        // The same rule `render_stems` uses, so the count the window reports is
        // the count the folder receives.
        let expected_files = if stems {
            project
                .tracks
                .iter()
                .filter(|track| {
                    track
                        .clips
                        .iter()
                        .any(|clip| !clip.muted && clip.take_active)
                        || track.midi_parts.iter().any(|part| !part.notes.is_empty())
                })
                .count()
        } else {
            1
        };
        thread::Builder::new()
            .name("opentrack-audio-export".to_owned())
            .spawn(move || {
                let report_path = worker_destination.clone();
                let outcome = if stems {
                    render_stems(
                        project,
                        snapshot,
                        worker_destination,
                        start_beats,
                        end_beats,
                        sample_rate,
                        format,
                        &worker_cancel,
                        &worker_progress,
                    )
                } else {
                    render_export(
                        project,
                        snapshot,
                        worker_destination,
                        start_beats,
                        end_beats,
                        sample_rate,
                        format,
                        &worker_cancel,
                        &worker_progress,
                    )
                    .map(|report| vec![(report_path, report)])
                };
                let _ = sender.send(outcome);
            })
            .map_err(|error| error.to_string())?;
        self.audio_export_task = Some(Task {
            destination: task_destination,
            expected_files,
            result,
            progress,
            cancel,
        });
        Ok(())
    }
}

/// A bounce-in-place render running in the background.
///
/// Bounce keeps the range and the replaced events with the task, so a finished
/// render becomes one undo step rather than a file the user has to place by hand.
pub(super) struct BounceTask {
    destination: PathBuf,
    track_id: u64,
    replaced: Vec<u64>,
    start_beats: f64,
    duration_seconds: f64,
    result: Receiver<Result<WavExportReport, String>>,
    progress: Arc<Mutex<(u64, u64)>>,
    cancel: Arc<AtomicBool>,
}

/// A free file name for rendered audio inside the project's `Audio/Imports`.
pub(super) fn unique_wav_path(root: &std::path::Path, name: &str) -> Result<PathBuf, String> {
    let directory = root.join("Audio").join("Imports");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let base = stem_file_name(name, 0)
        .trim_end_matches(".wav")
        .trim_start_matches("001 ")
        .to_owned();
    let candidate = directory.join(format!("{base}.wav"));
    if !candidate.exists() {
        return Ok(candidate);
    }
    for index in 2..=99_999 {
        let candidate = directory.join(format!("{base} ({index}).wav"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("no unused name is available in Audio/Imports".to_owned())
}

/// One stem's playback settings: only the chosen track, routed to the master.
///
/// A stem is the channel's own signal, so its sends are dropped (a send belongs
/// to the mix, not to the stem) and its output is forced to the stereo master
/// instead of a group. Automation, EQ, inserts and hosted inserts are untouched,
/// because they are part of the channel.
pub(super) fn stem_snapshot(
    base: &PlaybackSettingsSnapshot,
    track_index: usize,
) -> Option<PlaybackSettingsSnapshot> {
    let mut settings = base.tracks.get(track_index)?.clone();
    settings.sends.clear();
    settings.output_route = opentrack_audio::PlaybackOutputRoute::Stereo;
    Some(PlaybackSettingsSnapshot {
        tempo_bpm: base.tempo_bpm,
        tracks: vec![settings],
    })
}

/// File name for one stem, without characters a file system would refuse.
pub(super) fn stem_file_name(track_name: &str, index: usize) -> String {
    let cleaned = track_name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, ' ' | '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    let base = if cleaned.is_empty() {
        format!("{:03} Track {:03}", index + 1, index + 1)
    } else {
        format!("{:03} {}", index + 1, cleaned)
    };
    format!("{base}.wav")
}

/// Renders each track of the project to its own WAV inside `folder`.
///
/// Every stem goes through the same offline renderer and settings as a mixdown,
/// with only that track's channel in the graph, so the stem equals what the
/// track contributes to the mix rather than a re-creation of it.
#[allow(clippy::too_many_arguments)]
fn render_stems(
    project: Project,
    snapshot: PlaybackSettingsSnapshot,
    folder: PathBuf,
    start_beats: f64,
    end_beats: f64,
    sample_rate: u32,
    format: RecordingFormat,
    cancel: &AtomicBool,
    progress: &Mutex<(u64, u64)>,
) -> Result<Vec<(PathBuf, WavExportReport)>, String> {
    std::fs::create_dir_all(&folder)
        .map_err(|error| format!("could not create {}: {error}", folder.display()))?;
    let mut reports = Vec::new();
    for (index, track) in project.tracks.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Err("export cancelled".to_owned());
        }
        let audible = track
            .clips
            .iter()
            .any(|clip| !clip.muted && clip.take_active)
            || track.midi_parts.iter().any(|part| !part.notes.is_empty());
        if !audible {
            continue;
        }
        let Some(stem) = stem_snapshot(&snapshot, index) else {
            continue;
        };
        let destination = folder.join(stem_file_name(&track.name, index));
        // Per-stem VST3 instances keep one track's plug-ins from seeing another
        // track's timeline, exactly like the mixdown path.
        let mut stem_project = project.clone();
        stem_project
            .tracks
            .retain(|candidate| candidate.id == track.id);
        let report = render_export(
            stem_project,
            stem,
            destination.clone(),
            start_beats,
            end_beats,
            sample_rate,
            format,
            cancel,
            progress,
        )?;
        reports.push((destination, report));
    }
    Ok(reports)
}

#[allow(clippy::too_many_arguments)]
fn render_export(
    project: Project,
    mut snapshot: PlaybackSettingsSnapshot,
    destination: PathBuf,
    start_beats: f64,
    end_beats: f64,
    sample_rate: u32,
    format: RecordingFormat,
    cancel: &AtomicBool,
    progress: &Mutex<(u64, u64)>,
) -> Result<WavExportReport, String> {
    let block_size = 1024;
    let mut runtime = super::vst3_host::Runtime::default();
    runtime.set_format(sample_rate, block_size);
    runtime.sync(&project);
    for (settings, track) in snapshot.tracks.iter_mut().zip(&project.tracks) {
        settings.hosted_inserts = runtime.hosted_inserts(track);
    }
    let output = &project.output_channel.strip;
    let output_hosted = runtime.hosted_inserts(output);
    let mut renderer = OfflineRenderer::new(
        snapshot.tempo_bpm,
        snapshot.tracks,
        sample_rate,
        start_beats,
        end_beats,
        output,
        output_hosted,
    )
    .map_err(|error| error.to_string())?;
    if let Ok(mut value) = progress.lock() {
        *value = (0, renderer.total_frames());
    }
    write_mixdown_wav(
        &mut renderer,
        &destination,
        format,
        cancel,
        |done, total| {
            if let Ok(mut value) = progress.lock() {
                *value = (done, total);
            }
        },
    )
    .map_err(|error| error.to_string())
}

fn project_end_beats(project: &Project) -> f64 {
    let tempo = project.tempo_bpm;
    project
        .tracks
        .iter()
        .flat_map(|track| {
            let audio = track
                .clips
                .iter()
                .map(move |clip| clip.start_beats + clip.duration_seconds * tempo / 60.0);
            let midi = track
                .midi_parts
                .iter()
                .map(|part| part.start_beats + part.duration_beats);
            audio.chain(midi)
        })
        .filter(|end| end.is_finite())
        .fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use opentrack_audio::{StereoMeter, oscillator_instrument_factory};
    use opentrack_core::project::{ChannelEqualizer, InstrumentWaveform};

    /// A plain stereo track distinguished from its neighbours by its gain.
    fn track(gain: f32) -> opentrack_audio::PlaybackTrackSettings {
        opentrack_audio::PlaybackTrackSettings {
            is_group: false,
            audio_channels: 2,
            clips: Vec::new(),
            midi_notes: Vec::new(),
            midi_controllers: Vec::new(),
            instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
            meter_peak: Arc::new(StereoMeter::default()),
            gain,
            pan: 0.0,
            input_gain_db: 0.0,
            input_phase_inverted: false,
            track_delay_ms: 0.0,
            muted: false,
            solo: false,
            solo_defeat: false,
            equalizer: ChannelEqualizer::default(),
            inserts: Vec::new(),
            inserts_bypassed: false,
            hosted_inserts: Vec::new(),
            monitor_input: None,
            volume_automation: None,
            sends: Vec::new(),
            output_route: opentrack_audio::PlaybackOutputRoute::Stereo,
        }
    }

    #[test]
    fn a_stem_contains_only_its_own_track_routed_to_the_master() {
        let base = PlaybackSettingsSnapshot {
            tempo_bpm: 120.0,
            tracks: vec![track(0.25), track(0.75)],
        };
        let stem = stem_snapshot(&base, 1).expect("stem");
        assert_eq!(stem.tracks.len(), 1, "one track per stem");
        assert_eq!(
            stem.tracks[0].gain, 0.75,
            "the second track is the one kept"
        );
        assert_eq!(
            stem.tracks[0].output_route,
            opentrack_audio::PlaybackOutputRoute::Stereo,
            "a stem goes to the master, not into a group"
        );
        assert!(stem.tracks[0].sends.is_empty(), "sends belong to the mix");
        assert_eq!(stem.tempo_bpm, base.tempo_bpm);
        assert!(stem_snapshot(&base, 9).is_none());
    }

    #[test]
    fn a_group_stem_keeps_its_channel_but_loses_its_sends() {
        let mut bus = track(1.0);
        bus.is_group = true;
        bus.sends = vec![opentrack_audio::PlaybackSend {
            destination: opentrack_audio::PlaybackOutputRoute::Stereo,
            tap: opentrack_core::project::SendTap::PostFader,
            gain: 0.5,
            pan: 0.0,
        }];
        let base = PlaybackSettingsSnapshot {
            tempo_bpm: 96.0,
            tracks: vec![bus],
        };
        let stem = stem_snapshot(&base, 0).expect("stem");
        assert!(stem.tracks[0].is_group);
        assert!(stem.tracks[0].sends.is_empty());
        assert_eq!(
            stem.tracks[0].output_route,
            opentrack_audio::PlaybackOutputRoute::Stereo
        );
    }

    /// Writes a one-second 16-bit mono WAV at a constant level.
    fn write_test_wav(path: &std::path::Path, level: f32) {
        const FRAMES: u32 = 48_000;
        const RATE: u32 = 48_000;
        let mut bytes = Vec::with_capacity(44 + (FRAMES as usize) * 2);
        let data_bytes = FRAMES * 2;
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&RATE.to_le_bytes());
        bytes.extend_from_slice(&(RATE * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_bytes.to_le_bytes());
        let sample = (level.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        for _ in 0..FRAMES {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        std::fs::write(path, bytes).expect("write wav");
    }

    #[test]
    fn every_audible_track_renders_its_own_wav_in_the_chosen_folder() {
        use opentrack_audio::{DecodedWav, PlaybackClip, PlaybackSource};
        use opentrack_core::project::TrackKind;
        use std::sync::atomic::AtomicBool;

        let root = std::env::temp_dir().join(format!("opentrack-stems-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("stems");
        std::fs::create_dir_all(&root).expect("root");
        let drums_path = root.join("drums.wav");
        let bass_path = root.join("bass.wav");
        write_test_wav(&drums_path, 0.25);
        write_test_wav(&bass_path, 0.5);

        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.sample_rate_hz = 48_000;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        project.tracks[0].name = "Drums".to_owned();
        project.tracks[1].name = "Bass".to_owned();
        project
            .add_audio_clip(0, drums_path.clone(), 0.0, 1.0)
            .expect("clip");
        project
            .add_audio_clip(1, bass_path.clone(), 0.0, 1.0)
            .expect("clip");

        let with_source = |mut settings: opentrack_audio::PlaybackTrackSettings,
                           path: &std::path::Path| {
            settings.clips = vec![PlaybackClip {
                source_fades: Vec::new(),
                source: PlaybackSource::Decoded(Arc::new(
                    DecodedWav::open(path).expect("open wav"),
                )),
                start_beats: 0.0,
                source_offset_seconds: 0.0,
                duration_seconds: 1.0,
                fade_in_seconds: 0.0,
                fade_out_seconds: 0.0,
                gain: 1.0,
                muted: false,
                solo: false,
            }];
            settings
        };
        let snapshot = PlaybackSettingsSnapshot {
            tempo_bpm: 120.0,
            tracks: vec![
                with_source(track(1.0), &drums_path),
                with_source(track(0.5), &bass_path),
            ],
        };

        let cancel = AtomicBool::new(false);
        let progress = Mutex::new((0, 0));
        let reports = render_stems(
            project,
            snapshot,
            folder.clone(),
            0.0,
            2.0,
            48_000,
            RecordingFormat::Float32,
            &cancel,
            &progress,
        )
        .expect("stems render");

        assert_eq!(reports.len(), 2, "one file per audible track");
        assert_eq!(reports[0].0.file_name().unwrap(), "001 Drums.wav");
        assert_eq!(reports[1].0.file_name().unwrap(), "002 Bass.wav");
        for (path, report) in &reports {
            assert!(path.exists(), "{} exists", path.display());
            assert_eq!(report.sample_rate, 48_000);
            assert!(report.frames > 0, "{} has frames", path.display());
        }
        // The stems are separate files of the same length, not one concatenation.
        assert_eq!(reports[0].1.frames, reports[1].1.frames);

        // A silent track is skipped instead of writing a blank file.
        let quiet_project = Project::default();
        let empty_snapshot = PlaybackSettingsSnapshot {
            tempo_bpm: 120.0,
            tracks: vec![track(1.0)],
        };
        assert!(
            render_stems(
                quiet_project,
                empty_snapshot,
                root.join("empty"),
                0.0,
                1.0,
                48_000,
                RecordingFormat::Float32,
                &cancel,
                &progress,
            )
            .expect("nothing to render")
            .is_empty(),
            "a track with no media writes no stem"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn bouncing_a_selection_places_one_rendered_event_as_one_undo_step() {
        use opentrack_audio::{DecodedWav, PlaybackClip, PlaybackSource};
        use opentrack_core::project::TrackKind;

        let root = std::env::temp_dir().join(format!("opentrack-bounce-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        super::super::create_project_directory_layout(&root).expect("project layout");
        let first_path = root.join("one.wav");
        let second_path = root.join("two.wav");
        write_test_wav(&first_path, 0.25);
        write_test_wav(&second_path, 0.25);

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = super::super::OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.tempo_bpm = 120.0;
        app.data.project.sample_rate_hz = 48_000;
        app.data.project.add_track(TrackKind::Audio);
        // At 120 BPM one second is two beats: the events cover 0..2 and 4..6
        // beats, so the bounce spans three seconds.
        let first = app
            .data
            .project
            .add_audio_clip(0, first_path.clone(), 0.0, 1.0)
            .expect("first");
        let second = app
            .data
            .project
            .add_audio_clip(0, second_path.clone(), 4.0, 1.0)
            .expect("second");
        app.project_history.clear();

        let clip = |path: &std::path::Path, start: f64| PlaybackClip {
            source_fades: Vec::new(),
            source: PlaybackSource::Decoded(Arc::new(DecodedWav::open(path).expect("open wav"))),
            start_beats: start,
            source_offset_seconds: 0.0,
            duration_seconds: 1.0,
            fade_in_seconds: 0.0,
            fade_out_seconds: 0.0,
            gain: 1.0,
            muted: false,
            solo: false,
        };
        let mut settings = track(1.0);
        settings.clips = vec![clip(&first_path, 0.0), clip(&second_path, 4.0)];
        app.offline_playback_settings = Some(PlaybackSettingsSnapshot {
            tempo_bpm: 120.0,
            tracks: vec![settings],
        });

        app.arrange_view_state
            .audio_events
            .select_many([first, second]);
        app.start_bounce_in_place();
        assert!(app.bounce_task.is_some(), "the render started");
        assert!(app.status_message.contains("Bouncing"));

        // Wait for the worker, then let the app finish the placement.
        for _ in 0..500 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            app.poll_bounce_task();
            if app.bounce_task.is_none() {
                break;
            }
        }
        assert!(app.bounce_task.is_none(), "the bounce finished");

        let bounced = {
            let clips = &app.data.project.tracks[0].clips;
            assert_eq!(clips.len(), 1, "the two events became one rendered event");
            clips[0].clone()
        };
        assert!((bounced.start_beats - 0.0).abs() < 1e-9);
        assert!((bounced.duration_seconds - 3.0).abs() < 1e-6);
        assert!(
            bounced
                .source_path
                .starts_with(root.join("Audio").join("Imports")),
            "the render lives in the project: {}",
            bounced.source_path.display()
        );
        assert!(bounced.source_path.exists());
        assert!(app.status_message.contains("Bounced 2 event(s)"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Bounce in Place");

        // A selection spanning two tracks is refused.
        app.data.project.add_track(TrackKind::Audio);
        let elsewhere = app
            .data
            .project
            .add_audio_clip(1, first_path.clone(), 0.0, 2.0)
            .expect("other");
        app.arrange_view_state
            .audio_events
            .select_many([bounced.id, elsewhere]);
        app.start_bounce_in_place();
        assert!(app.status_message.contains("one track at a time"));
        assert!(app.bounce_task.is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn stem_names_are_ordered_and_file_safe() {
        assert_eq!(stem_file_name("Drums", 0), "001 Drums.wav");
        assert_eq!(stem_file_name("Bass / DI", 3), "004 Bass _ DI.wav");
        assert_eq!(stem_file_name("   ", 5), "006 Track 006.wav");
        assert_eq!(stem_file_name("..", 0), "001 Track 001.wav");
        // A leading number keeps a sorted folder in track order.
        assert_eq!(stem_file_name("1 Kick", 1), "002 1 Kick.wav");
    }

    #[test]
    fn the_stems_dialog_starts_from_the_mixdown_settings() {
        let mut project = Project::default();
        project.add_track(opentrack_core::project::TrackKind::Audio);
        project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 4.0)
            .expect("clip");
        let dialog = Dialog::for_stems(&project);
        assert!(dialog.stems);
        assert_eq!(dialog.format, Format::Float32);
        assert_eq!(dialog.sample_rate, project.sample_rate_hz);
        assert!(dialog.end_beats >= 4.0);
        assert!(
            !Dialog::for_project(&project).stems,
            "mixdown is the default"
        );
    }
}
