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
    OfflineRenderer, PlaybackSettingsSnapshot, WavExportReport, write_mixdown_wav,
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
}

impl Dialog {
    pub(super) fn for_project(project: &Project) -> Self {
        Self {
            start_beats: 0.0,
            end_beats: project_end_beats(project).max(4.0),
            format: Format::Float32,
            sample_rate: project.sample_rate_hz.max(1),
            open: true,
        }
    }
}

pub(super) struct Task {
    destination: PathBuf,
    result: Receiver<Result<WavExportReport, String>>,
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
                Ok(report) => format!(
                    "Exported {} frames at {} Hz to {} (peak {:.1} dBFS)",
                    report.frames,
                    report.sample_rate,
                    task.destination.display(),
                    if report.peak_linear > 0.0 {
                        20.0 * report.peak_linear.log10()
                    } else {
                        f32::NEG_INFINITY
                    },
                ),
                Err(error) => format!("Audio export failed: {error}"),
            };
        }
    }

    pub(super) fn show_audio_export(&mut self, ctx: &egui::Context) {
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
                ui.label("Render the project timeline to a stereo WAV file.");
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
            ) {
                self.status_message = format!("Could not start audio export: {error}");
            }
        }
    }

    fn start_audio_export(
        &mut self,
        destination: PathBuf,
        start_beats: f64,
        end_beats: f64,
        format: RecordingFormat,
        sample_rate: u32,
    ) -> Result<(), String> {
        if let Some(path) =
            super::unresolved_audio_source_paths(&self.data.project, &self.data.audio_sources)
                .into_iter()
                .next()
        {
            return Err(format!("media source is unavailable: {}", path.display()));
        }
        self.rebuild_audio_plan();
        let snapshot = self
            .offline_playback_settings
            .clone()
            .ok_or_else(|| "the project playback graph is not ready".to_owned())?;
        let mut project = self.data.project.clone();
        self.vst3_runtime.capture_states(&mut project);
        let progress = Arc::new(Mutex::new((0, 0)));
        let cancel = Arc::new(AtomicBool::new(false));
        let (sender, result) = mpsc::channel();
        let worker_progress = Arc::clone(&progress);
        let worker_cancel = Arc::clone(&cancel);
        let worker_destination = destination.clone();
        thread::Builder::new()
            .name("opentrack-audio-export".to_owned())
            .spawn(move || {
                let outcome = render_export(
                    project,
                    snapshot,
                    worker_destination,
                    start_beats,
                    end_beats,
                    sample_rate,
                    format,
                    &worker_cancel,
                    &worker_progress,
                );
                let _ = sender.send(outcome);
            })
            .map_err(|error| error.to_string())?;
        self.audio_export_task = Some(Task {
            destination,
            result,
            progress,
            cancel,
        });
        Ok(())
    }
}

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
