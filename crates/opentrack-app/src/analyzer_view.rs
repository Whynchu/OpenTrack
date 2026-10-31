//! Channel editor spectrum state. FFT work runs here/on the UI thread, never in callbacks.
use super::*;
use opentrack_audio::{
    SpectrumFrame, SpectrumResolution, SpectrumStage, StereoMeter, TONAL_FFT_SIZE,
    TONAL_PROFILE_BANDS, TonalProfile,
};
use opentrack_core::project::{ChannelEqualizer, TonalTarget};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
const TONAL_BANDS: usize = TONAL_PROFILE_BANDS;
const DEFAULT_ANALYZER_SAMPLE_RATE: u32 = 48_000;

struct ReferenceImportPayload {
    display_name: String,
    source_name: String,
    profile: TonalProfile,
}

struct ReferenceImportTask {
    owner: u64,
    initial_target: Option<TonalTarget>,
    cancel: Arc<AtomicBool>,
    result: Receiver<Result<ReferenceImportPayload, String>>,
}

#[derive(Clone)]
pub(super) struct Choice {
    pub id: u64,
    pub name: String,
    pub channel_count: u8,
    pub equalizer: opentrack_core::project::ChannelEqualizer,
}
#[derive(Default)]
pub(super) struct State {
    pub stage: SpectrumStage,
    tonal_stage: SpectrumStage,
    pub release_ms: u32,
    pub frozen: bool,
    pub peak_hold: bool,
    pub range_db: Option<i32>,
    pub tilt_db_per_octave: i8,
    pub resolution: SpectrumResolution,
    pub compare: Option<u64>,
    owner: Option<u64>,
    channel_count: Option<u8>,
    pub choices: Vec<Choice>,
    active: Vec<(u64, Arc<StereoMeter>)>,
    frames: HashMap<(u64, SpectrumStage), SpectrumFrame>,
    peak_bins: HashMap<(u64, SpectrumStage), Vec<f32>>,
    seen: HashMap<u64, (u64, Instant)>,
    last_poll: Option<Instant>,
    default_rate: u32,
    pub tonal_view: bool,
    pub tonal_difference: bool,
    pub capture_active: bool,
    pub capture_incomplete: bool,
    capture_seconds: f32,
    pub transport_active: bool,
    tonal_average: Vec<f64>,
    tonal_average_seconds: Option<u32>,
    tonal_average_elapsed: f32,
    tonal_average_held: bool,
    capture_sum: Vec<f64>,
    capture_frames: u64,
    capture_sequence: HashMap<u64, u64>,
    saved_target: Option<TonalTarget>,
    last_transport_position: Option<(f64, Instant)>,
    import_task: Option<ReferenceImportTask>,
    import_status: String,
}
impl State {
    pub fn controls_height(&self, width: f32) -> f32 {
        let rows = if self.tonal_view {
            if width < 400.0 {
                4
            } else if width < 440.0 {
                3
            } else if width < 880.0 {
                2
            } else {
                1
            }
        } else if width < 430.0 {
            3
        } else if width < 650.0 {
            2
        } else {
            1
        };
        rows as f32 * 24.0
    }

    fn active_stage(&self) -> SpectrumStage {
        if self.tonal_view {
            self.tonal_stage
        } else {
            self.stage
        }
    }

    fn active_resolution(&self) -> SpectrumResolution {
        if self.tonal_view {
            SpectrumResolution::Fine
        } else {
            self.resolution
        }
    }

    pub fn close(&mut self) {
        if let Some(task) = self.import_task.take() {
            task.cancel.store(true, Ordering::Release);
        }
        for (_, tap) in self.active.drain(..) {
            tap.enable_spectrum(false);
        }
        self.owner = None;
        self.channel_count = None;
        self.compare = None;
        self.stage = SpectrumStage::Post;
        self.tonal_stage = SpectrumStage::Post;
        self.resolution = SpectrumResolution::Balanced;
        self.frames.clear();
        self.peak_bins.clear();
        self.frozen = false;
        self.peak_hold = false;
        self.seen.clear();
        self.last_poll = None;
        self.tonal_average.clear();
        self.tonal_average_seconds = Some(30);
        self.tonal_average_elapsed = 0.0;
        self.tonal_average_held = false;
        self.capture_sum.clear();
        self.capture_frames = 0;
        self.capture_sequence.clear();
        self.capture_active = false;
        self.capture_incomplete = false;
        self.capture_seconds = 0.0;
        self.transport_active = false;
        self.last_transport_position = None;
        self.import_status.clear();
    }

    fn start_reference_import(&mut self, initial_target: Option<TonalTarget>) {
        let Some(owner) = self.owner else {
            self.import_status = "Select a channel before importing a reference".to_owned();
            return;
        };
        let Some(path) = FileDialog::new()
            .add_filter("WAV audio", &["wav", "wave"])
            .set_title("Import Tonal Reference WAV")
            .pick_file()
        else {
            return;
        };
        let source_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Reference WAV".to_owned());
        let display_name = path
            .file_stem()
            .map(|name| name.to_string_lossy().chars().take(128).collect())
            .unwrap_or_else(|| "Imported Reference".to_owned());
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let status = format!("Analyzing {source_name}…");
        let (sender, result) = mpsc::sync_channel(1);
        if let Err(error) = std::thread::Builder::new()
            .name("opentrack-tonal-reference-import".to_owned())
            .spawn(move || {
                let result = opentrack_audio::analyze_tonal_reference_wav(&path, &worker_cancel)
                    .map(|profile| ReferenceImportPayload {
                        display_name,
                        source_name,
                        profile,
                    });
                let _ = sender.send(result);
            })
        {
            self.import_status = format!("Could not start reference analysis: {error}");
            return;
        }
        self.import_status = status;
        self.import_task = Some(ReferenceImportTask {
            owner,
            initial_target,
            cancel,
            result,
        });
    }

    fn poll_reference_import(&mut self, eq: &mut ChannelEqualizer) -> bool {
        let result = match self.import_task.as_ref() {
            Some(task) => match task.result.try_recv() {
                Ok(result) => Some((task.owner, task.initial_target.clone(), result)),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some((
                    task.owner,
                    task.initial_target.clone(),
                    Err("reference analysis stopped before returning a result".to_owned()),
                )),
            },
            None => return false,
        };
        let Some((task_owner, initial_target, result)) = result else {
            return false;
        };
        self.import_task = None;
        match result {
            Ok(payload) if self.owner == Some(task_owner) && eq.tonal_target == initial_target => {
                let target = Some(TonalTarget {
                    name: payload.display_name,
                    analysis_version: 3,
                    sample_rate: payload.profile.sample_rate,
                    source_stage: 2,
                    source_name: payload.source_name,
                    analysis_seconds: payload.profile.duration_seconds as f32,
                    fft_size: TONAL_FFT_SIZE as u32,
                    band_powers: payload.profile.band_powers,
                });
                let changed = eq.tonal_target != target;
                eq.tonal_target = target;
                self.saved_target = eq.tonal_target.clone();
                self.tonal_difference = false;
                self.import_status = "Reference imported".to_owned();
                changed
            }
            Ok(_) => {
                self.import_status = if self.owner != Some(task_owner) {
                    "Reference discarded because the edited channel changed".into()
                } else {
                    "Reference discarded because the saved target changed during import".into()
                };
                false
            }
            Err(error) => {
                self.import_status = if error.contains("cancelled") {
                    "Reference import cancelled".to_owned()
                } else {
                    format!("Could not import reference: {error}")
                };
                false
            }
        }
    }
    pub fn update(
        &mut self,
        owner: Option<u64>,
        choices: Vec<Choice>,
        available: HashMap<u64, Arc<StereoMeter>>,
        rate: u32,
        transport_active: bool,
        position_seconds: f64,
    ) {
        if self.default_rate != 0 && self.default_rate != rate {
            self.invalidate_capture_for_signal_change();
        }
        if self.owner != owner {
            self.close();
            self.owner = owner;
        }
        self.choices = choices;
        let channel_count = owner
            .and_then(|id| self.choices.iter().find(|choice| choice.id == id))
            .map(|choice| choice.channel_count);
        if self.channel_count.is_some() && self.channel_count != channel_count {
            self.invalidate_capture_for_signal_change();
        }
        self.channel_count = channel_count;
        self.default_rate = rate;
        if self
            .compare
            .is_some_and(|id| Some(id) == owner || !self.choices.iter().any(|c| c.id == id))
        {
            self.compare = None;
        }
        let wanted = owner
            .into_iter()
            .chain(self.compare)
            .filter_map(|id| available.get(&id).map(|tap| (id, Arc::clone(tap))))
            .collect::<Vec<_>>();
        for (id, tap) in &self.active {
            if !wanted
                .iter()
                .any(|(other, current)| other == id && Arc::ptr_eq(tap, current))
            {
                tap.enable_spectrum(false);
                self.frames.retain(|(key, _), _| key != id);
                self.seen.remove(id);
            }
        }
        for (id, tap) in &wanted {
            if !self
                .active
                .iter()
                .any(|(old, previous)| old == id && Arc::ptr_eq(previous, tap))
            {
                tap.enable_spectrum(true);
                self.last_poll = None;
            }
        }
        self.active = wanted;
        let now = Instant::now();
        let jumped = self.last_transport_position.is_some_and(|(previous, at)| {
            let elapsed = now.duration_since(at).as_secs_f64();
            let moved = (position_seconds - previous).abs();
            if transport_active {
                moved > elapsed + 0.25
            } else {
                moved > 0.05
            }
        });
        if self.capture_active && (!transport_active || jumped) {
            self.capture_active = false;
            self.capture_incomplete = true;
        }
        self.transport_active = transport_active;
        self.last_transport_position = Some((position_seconds, now));
        if self
            .last_poll
            .is_some_and(|last| now.duration_since(last).as_millis() < 30)
        {
            return;
        }
        let dt = self
            .last_poll
            .map(|last| now.duration_since(last).as_secs_f32())
            .unwrap_or(0.03);
        self.last_poll = Some(now);
        let active_stage = self.active_stage();
        let active_resolution = self.active_resolution();
        let analysis_dt = dt.clamp(0.0, 0.1);
        for (id, tap) in &self.active {
            if let Some(mut frame) = tap.spectrum_frame_at(active_stage, active_resolution) {
                Self::observe_spectrum_sequence(
                    &mut self.seen,
                    self.owner,
                    &mut self.capture_active,
                    &mut self.capture_incomplete,
                    &mut self.capture_sum,
                    &mut self.capture_frames,
                    *id,
                    frame.sequence,
                    frame.sample_rate,
                    now,
                );
                if self.capture_sequence.get(id).copied() != Some(frame.sequence) {
                    let profile = frame.normalized_log_band_powers();
                    if profile.iter().any(|power| *power > 0.0) {
                        if self.tonal_view && Some(*id) == self.owner {
                            update_tonal_average(
                                &mut self.tonal_average,
                                &mut self.tonal_average_elapsed,
                                &profile,
                                analysis_dt,
                                self.tonal_average_seconds.unwrap_or(30),
                                self.tonal_average_held,
                            );
                        }
                        if self.capture_active && Some(*id) == self.owner {
                            if self.capture_sum.len() != TONAL_BANDS {
                                self.capture_sum = vec![0.0; TONAL_BANDS];
                            }
                            for (sum, value) in self.capture_sum.iter_mut().zip(&profile) {
                                *sum += f64::from(*value);
                            }
                            self.capture_frames += 1;
                            self.capture_seconds += analysis_dt;
                        }
                    }
                    self.capture_sequence.insert(*id, frame.sequence);
                }
                if let Some(old) = self.frames.get(&(*id, active_stage)) {
                    metering::smooth_spectrum(
                        &old.bins_dbfs,
                        &mut frame.bins_dbfs,
                        dt,
                        self.release_ms,
                    );
                }
                if self.peak_hold && !self.tonal_view && !self.frozen {
                    let peak = self
                        .peak_bins
                        .entry((*id, active_stage))
                        .or_insert_with(|| frame.bins_dbfs.clone());
                    if peak.len() != frame.bins_dbfs.len() {
                        *peak = frame.bins_dbfs.clone();
                    } else {
                        update_peak_bins(peak, &frame.bins_dbfs, dt * 12.0);
                    }
                }
                if self.tonal_view || !self.frozen {
                    self.frames.insert((*id, active_stage), frame);
                }
            } else if !self.frozen || self.tonal_view {
                self.frames.remove(&(*id, active_stage));
            }
        }
    }

    fn invalidate_capture_for_signal_change(&mut self) {
        let had_capture = self.capture_active || self.capture_frames > 0 || self.capture_incomplete;
        self.tonal_average.clear();
        self.tonal_average_elapsed = 0.0;
        self.tonal_average_held = false;
        self.capture_sum.clear();
        self.capture_frames = 0;
        self.capture_active = false;
        self.capture_incomplete = had_capture;
        if !had_capture {
            self.capture_seconds = 0.0;
        }
        self.capture_sequence.clear();
        self.last_transport_position = None;
    }

    fn invalidate_capture_for_tap_change(&mut self) {
        let had_capture = self.capture_active || self.capture_frames > 0 || self.capture_incomplete;
        self.capture_active = false;
        self.capture_incomplete = had_capture;
        self.capture_sum.clear();
        self.capture_frames = 0;
        if !had_capture {
            self.capture_seconds = 0.0;
        }
        self.capture_sequence.clear();
        self.tonal_average.clear();
        self.tonal_average_elapsed = 0.0;
        self.tonal_average_held = false;
    }

    pub fn controls(&mut self, ui: &mut egui::Ui, r: Rect, eq: &mut ChannelEqualizer) -> bool {
        let mut changed = self.poll_reference_import(eq);
        self.saved_target = eq.tonal_target.clone();
        if self.saved_target.is_none() {
            self.tonal_difference = false;
        }
        ui.painter()
            .rect_filled(r, 0.0, theme::palette(ui).surface_raised);
        let old_stage = self.stage;
        let old_tonal_stage = self.tonal_stage;
        let old_compare = self.compare;
        let old_tonal_view = self.tonal_view;
        let old_resolution = self.resolution;
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(r.shrink(2.0))
                .id_salt("spectrum-controls"),
            |ui| {
                ui.style_mut().override_font_id = Some(FontId::proportional(10.0));
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                ui.horizontal_wrapped(|ui| {
                    let selected_stage = if self.tonal_view {
                        &mut self.tonal_stage
                    } else {
                        &mut self.stage
                    };
                    egui::ComboBox::from_id_salt("analysis-stage")
                        .width(if self.tonal_view { 60.0 } else { 118.0 })
                        .selected_text(if *selected_stage == SpectrumStage::Pre {
                            if self.tonal_view {
                                "Pre"
                            } else {
                                "Pre Inserts/EQ"
                            }
                        } else {
                            if self.tonal_view {
                                "Post"
                            } else {
                                "Post Inserts/EQ"
                            }
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                selected_stage,
                                SpectrumStage::Pre,
                                if self.tonal_view {
                                    "Pre"
                                } else {
                                    "Pre Inserts/EQ"
                                },
                            );
                            ui.selectable_value(
                                selected_stage,
                                SpectrumStage::Post,
                                if self.tonal_view {
                                    "Post"
                                } else {
                                    "Post Inserts/EQ"
                                },
                            );
                        });
                    if !self.tonal_view {
                        ui.label("Compare");
                        let name = self
                            .compare
                            .and_then(|id| self.choices.iter().find(|c| c.id == id))
                            .map(|c| c.name.as_str())
                            .unwrap_or("None");
                        egui::ComboBox::from_id_salt("analysis-reference")
                            .width(100.0)
                            .selected_text(name)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.compare, None, "None");
                                for channel in &self.choices {
                                    if Some(channel.id) != self.owner {
                                        ui.selectable_value(
                                            &mut self.compare,
                                            Some(channel.id),
                                            &channel.name,
                                        );
                                    }
                                }
                            });
                    }
                    ui.separator();
                    ui.selectable_value(&mut self.tonal_view, false, "Spectrum");
                    ui.selectable_value(&mut self.tonal_view, true, "Tonal");
                    if self.tonal_view && self.saved_target.is_some() {
                        ui.selectable_value(&mut self.tonal_difference, false, "Profile");
                        ui.selectable_value(&mut self.tonal_difference, true, "Difference");
                    }
                    if self.tonal_view {
                        if let Some(target) = &self.saved_target {
                            let target_tap = tonal_target_tap_label(target.source_stage);
                            let current_tap = tonal_target_tap_label(if self.tonal_stage
                                == SpectrumStage::Pre
                            {
                                0
                            } else {
                                1
                            });
                            let tap_differs =
                                tonal_target_tap_differs(target.source_stage, self.tonal_stage);
                            let target_name = if target.name.is_empty() {
                                "Untitled target"
                            } else {
                                &target.name
                            };
                            let visible = if tap_differs {
                                format!("Target: {target_name} · {target_tap} · tap differs")
                            } else {
                                format!("Target: {target_name} · {target_tap}")
                            };
                            let mut label = egui::RichText::new(visible);
                            if tap_differs {
                                label = label.color(Color32::from_rgb(245, 155, 55));
                            }
                            ui.add(egui::Label::new(label).truncate())
                                .on_hover_text(format!(
                                    "Target: {}\nSource: {}\nTarget tap: {}\nCurrent tonal tap: {}\nAnalysis: {} Hz · {} point FFT · {:.1}s",
                                    target_name,
                                    if target.source_name.is_empty() {
                                        "Unknown source"
                                    } else {
                                        &target.source_name
                                    },
                                    target_tap,
                                    current_tap,
                                    target.sample_rate,
                                    target.fft_size,
                                    target.analysis_seconds,
                                ));
                        }
                        if let Some(task) = &self.import_task {
                            if ui.button("Cancel Import").clicked() {
                                task.cancel.store(true, Ordering::Release);
                                self.import_status = "Cancelling reference import…".to_owned();
                            }
                        } else if ui.button("Import WAV").clicked() {
                            self.start_reference_import(eq.tonal_target.clone());
                        }
                        let average_seconds = self.tonal_average_seconds.unwrap_or(30);
                        if ui
                            .selectable_label(self.tonal_average_held, "Hold")
                            .on_hover_text(
                                "Freeze the continuous tonal average. Explicit capture continues independently.",
                            )
                            .clicked()
                        {
                            self.tonal_average_held = !self.tonal_average_held;
                        }
                        egui::ComboBox::from_id_salt("tonal-average-duration")
                            .width(54.0)
                            .selected_text(format!("Avg {average_seconds}s"))
                            .show_ui(ui, |ui| {
                                for seconds in [10, 30, 60, 120] {
                                    ui.selectable_value(
                                        &mut self.tonal_average_seconds,
                                        Some(seconds),
                                        format!("{seconds}s"),
                                    );
                                }
                            })
                            .response
                            .on_hover_text(
                                "Continuous tonal average time constant. Warm-up reaches its settled indicator after three time constants.",
                            );
                    }
                    if !self.tonal_view {
                        if ui.selectable_label(self.frozen, "Freeze").clicked() {
                            self.frozen = !self.frozen;
                        }
                        if ui.selectable_label(self.peak_hold, "Peak").clicked() {
                            self.peak_hold = !self.peak_hold;
                            if !self.peak_hold {
                                self.peak_bins.clear();
                            }
                        }
                        egui::ComboBox::from_id_salt("spectrum-range")
                            .width(48.0)
                            .selected_text(format!("-{} dB", self.range_db.unwrap_or(90)))
                            .show_ui(ui, |ui| {
                                for range in [60, 90, 120] {
                                    ui.selectable_value(
                                        &mut self.range_db,
                                        Some(range),
                                        format!("-{range} dB"),
                                    );
                                }
                            });
                        egui::ComboBox::from_id_salt("spectrum-tilt")
                            .width(56.0)
                            .selected_text(format!("{} dB/oct", self.tilt_db_per_octave))
                            .show_ui(ui, |ui| {
                                for tilt in [0, 3, 6] {
                                    ui.selectable_value(
                                        &mut self.tilt_db_per_octave,
                                        tilt,
                                        format!("{tilt} dB/oct"),
                                    );
                                }
                            })
                            .response
                            .on_hover_text(
                                "Display tilt only; it does not change audio or tonal capture.",
                            );
                        egui::ComboBox::from_id_salt("spectrum-resolution")
                            .width(34.0)
                            .selected_text(format!("{}K", self.resolution.fft_size() / 1024))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.resolution,
                                    SpectrumResolution::Fast,
                                    "2K · fast",
                                );
                                ui.selectable_value(
                                    &mut self.resolution,
                                    SpectrumResolution::Balanced,
                                    "4K · balanced",
                                );
                                ui.selectable_value(
                                    &mut self.resolution,
                                    SpectrumResolution::Fine,
                                    "8K · fine",
                                );
                            })
                            .response
                            .on_hover_text(
                                "FFT size: larger windows resolve low frequencies more finely, with slower updates. Temporal release smoothing is adjustable in Preferences > Metering.",
                            );
                    }
                    if self.tonal_view
                        && ui
                            .add_enabled(
                                self.transport_active || self.capture_active,
                                egui::Button::new(if self.capture_active {
                                    "Stop Capture"
                                } else {
                                    "Capture"
                                }),
                            )
                            .on_hover_text("Start the transport before capturing a reference")
                            .clicked()
                    {
                        if self.capture_active {
                            self.capture_active = false;
                        } else {
                            self.capture_active = true;
                            self.capture_incomplete = false;
                            self.capture_sum = vec![0.0; TONAL_BANDS];
                            self.capture_frames = 0;
                            self.capture_seconds = 0.0;
                        }
                    }
                    if self.tonal_view {
                        let average_seconds = self.tonal_average_seconds.unwrap_or(30);
                        let settled_after = average_seconds * 3;
                        let average_label = if self.tonal_average_held {
                            format!("Held · {:.0}/{settled_after}s", self.tonal_average_elapsed)
                        } else if self.tonal_average.is_empty() {
                            "Waiting for signal".to_owned()
                        } else if self.tonal_average_elapsed < settled_after as f32 {
                            format!(
                                "Warming {:.0}/{settled_after}s",
                                self.tonal_average_elapsed
                            )
                        } else {
                            format!("Average ready · {average_seconds}s")
                        };
                        ui.label(average_label);
                        let capture_label = if self.capture_active {
                            format!("Capturing {:.0}s", self.capture_seconds)
                        } else if self.capture_incomplete {
                            format!("Incomplete · {:.0}s", self.capture_seconds)
                        } else if self.capture_frames > 0 {
                            format!("Captured {:.0}s", self.capture_seconds)
                        } else if !self.transport_active {
                            "Play to capture".to_owned()
                        } else {
                            "Capture ≥10s".to_owned()
                        };
                        ui.label(capture_label);
                        if !self.import_status.is_empty() {
                            ui.label(&self.import_status);
                        }
                    }
                    if self.tonal_view && ui.button("Reset").clicked() {
                        self.tonal_average.clear();
                        self.tonal_average_elapsed = 0.0;
                        self.tonal_average_held = false;
                        self.capture_sum.clear();
                        self.capture_frames = 0;
                        self.capture_seconds = 0.0;
                        self.capture_incomplete = false;
                    }
                    if self.tonal_view
                        && ui
                            .add_enabled(
                                self.capture_seconds >= 10.0 && !self.capture_incomplete,
                                egui::Button::new("Save"),
                            )
                            .clicked()
                        && self.capture_frames > 0
                    {
                        let powers = self
                            .capture_sum
                            .iter()
                            .map(|power| (power / self.capture_frames as f64) as f32)
                            .collect();
                        let target = Some(TonalTarget {
                            name: "Captured Reference".to_owned(),
                            analysis_version: 3,
                            sample_rate: self.default_rate,
                            source_stage: if self.tonal_stage == SpectrumStage::Pre {
                                0
                            } else {
                                1
                            },
                            source_name: self
                                .owner
                                .and_then(|id| self.choices.iter().find(|choice| choice.id == id))
                                .map(|choice| choice.name.clone())
                                .unwrap_or_else(|| "Channel capture".to_owned()),
                            analysis_seconds: self.capture_seconds,
                            fft_size: TONAL_FFT_SIZE as u32,
                            band_powers: powers,
                        });
                        changed |= eq.tonal_target != target;
                        eq.tonal_target = target;
                        self.saved_target = eq.tonal_target.clone();
                    }
                });
            },
        );
        if old_stage != self.stage || old_tonal_stage != self.tonal_stage {
            self.frames.clear();
            self.peak_bins.clear();
            self.frozen = false;
            self.capture_sequence.clear();
            self.last_poll = None;
            if old_tonal_stage != self.tonal_stage {
                self.invalidate_capture_for_tap_change();
            }
        }
        if old_resolution != self.resolution {
            self.frames.clear();
            self.peak_bins.clear();
            self.frozen = false;
            self.last_poll = None;
        }
        if old_compare != self.compare {
            self.peak_bins.clear();
            self.last_poll = None;
        }
        if old_tonal_view && !self.tonal_view {
            if self.capture_active {
                self.capture_active = false;
                self.capture_incomplete = true;
            }
        }
        if old_tonal_view != self.tonal_view {
            self.frozen = false;
            self.frames.clear();
            self.peak_bins.clear();
        }
        changed
    }
    pub fn rate(&self, id: Option<u64>) -> u32 {
        id.or(self.owner)
            .and_then(|id| self.frames.get(&(id, self.active_stage())))
            .map(|f| f.sample_rate)
            .unwrap_or(if self.default_rate == 0 {
                DEFAULT_ANALYZER_SAMPLE_RATE
            } else {
                self.default_rate
            })
    }

    pub fn dynamic_gains_db(
        &self,
        id: u64,
    ) -> Option<[f32; opentrack_core::project::MAX_EQ_BANDS]> {
        self.active
            .iter()
            .find(|(active_id, _)| *active_id == id)
            .map(|(_, meter)| meter.dynamic_eq_gains_db())
    }

    #[allow(clippy::too_many_arguments)]
    fn observe_spectrum_sequence(
        seen_sequences: &mut HashMap<u64, (u64, Instant)>,
        owner: Option<u64>,
        capture_active: &mut bool,
        capture_incomplete: &mut bool,
        capture_sum: &mut Vec<f64>,
        capture_frames: &mut u64,
        id: u64,
        sequence: u64,
        sample_rate: u32,
        now: Instant,
    ) {
        let Some(seen) = seen_sequences.get_mut(&id) else {
            seen_sequences.insert(id, (sequence, now));
            return;
        };
        if seen.0 == sequence {
            return;
        }
        let stream_gap =
            capture_stream_has_gap(seen.0, sequence, sample_rate, now.duration_since(seen.1));
        *seen = (sequence, now);
        if stream_gap && Some(id) == owner && *capture_active {
            *capture_active = false;
            *capture_incomplete = true;
            capture_sum.clear();
            *capture_frames = 0;
        }
    }

    pub fn paint(&self, ui: &egui::Ui, plot: Rect, own: Color32) {
        let painter = ui.painter().with_clip_rect(plot);
        let tonal_plot = if self.tonal_view {
            paint_tonal_regions(ui, plot)
        } else {
            plot
        };
        if !self.tonal_view {
            for (id, color, fill) in [
                (self.owner, own, true),
                (self.compare, Color32::from_rgb(245, 155, 55), false),
            ] {
                let Some(frame) = id.and_then(|id| self.frames.get(&(id, self.active_stage())))
                else {
                    continue;
                };
                // Aggregate bins per display column so narrow peaks survive zoom and resizing.
                let points = spectrum_points(
                    frame,
                    plot,
                    self.range_db.unwrap_or(90),
                    self.tilt_db_per_octave,
                );
                if points.len() > 1 {
                    if fill {
                        let mut mesh = egui::Mesh::default();
                        for (i, point) in points.iter().enumerate() {
                            mesh.colored_vertex(*point, color.gamma_multiply(0.18));
                            mesh.colored_vertex(
                                Pos2::new(point.x, plot.bottom()),
                                Color32::TRANSPARENT,
                            );
                            if i > 0 {
                                let n = (i * 2) as u32;
                                mesh.add_triangle(n - 2, n - 1, n);
                                mesh.add_triangle(n - 1, n + 1, n);
                            }
                        }
                        painter.add(egui::Shape::mesh(mesh));
                    }
                    painter.add(egui::Shape::line(
                        points,
                        Stroke::new(
                            if fill { 1.0 } else { 1.3 },
                            color.gamma_multiply(if fill { 0.65 } else { 0.9 }),
                        ),
                    ));
                }
                if fill && self.peak_hold {
                    if let Some(held) =
                        id.and_then(|id| self.peak_bins.get(&(id, self.active_stage())))
                    {
                        let held_frame = SpectrumFrame {
                            sequence: frame.sequence,
                            sample_rate: frame.sample_rate,
                            fft_size: frame.fft_size,
                            rms_dbfs: frame.rms_dbfs,
                            bins_dbfs: held.clone(),
                        };
                        let peak_points = spectrum_points(
                            &held_frame,
                            plot,
                            self.range_db.unwrap_or(90),
                            self.tilt_db_per_octave,
                        );
                        if peak_points.len() > 1 {
                            painter.add(egui::Shape::line(
                                peak_points,
                                Stroke::new(1.0, Color32::from_rgb(245, 155, 55)),
                            ));
                        }
                    }
                }
            }
        }
        if self.tonal_view {
            if self.tonal_difference {
                if let (Some(target), true) = (
                    self.saved_target.as_ref(),
                    self.tonal_average.len() == TONAL_BANDS,
                ) {
                    paint_tonal_difference(ui, tonal_plot, &self.tonal_average, target, own);
                }
            } else {
                if let Some(target) = &self.saved_target {
                    paint_tonal_profile(
                        ui,
                        tonal_plot,
                        &display_target_profile(target)
                            .iter()
                            .map(|v| f64::from(*v))
                            .collect::<Vec<_>>(),
                        Color32::from_rgb(245, 155, 55),
                        true,
                    );
                }
                if self.tonal_average.len() == TONAL_BANDS {
                    paint_tonal_profile(ui, tonal_plot, &self.tonal_average, own, false);
                }
            }
        }
        if self.tonal_view {
            let legend = Rect::from_min_size(
                plot.left_bottom() + Vec2::new(5.0, -33.0),
                Vec2::new(108.0, 28.0),
            );
            painter.rect_filled(legend, 2.0, theme::palette(ui).canvas.gamma_multiply(0.82));
            if self.tonal_difference {
                painter.line_segment(
                    [
                        legend.left_top() + Vec2::new(4.0, 14.0),
                        legend.left_top() + Vec2::new(15.0, 14.0),
                    ],
                    Stroke::new(1.8, own),
                );
                painter.text(
                    legend.left_top() + Vec2::new(19.0, 14.0),
                    Align2::LEFT_CENTER,
                    "Δ vs target · ±3 dB",
                    FontId::proportional(8.0),
                    theme::palette(ui).text,
                );
            } else if self.saved_target.is_some() {
                painter.line_segment(
                    [
                        legend.left_top() + Vec2::new(4.0, 8.0),
                        legend.left_top() + Vec2::new(15.0, 8.0),
                    ],
                    Stroke::new(1.8, own),
                );
                painter.text(
                    legend.left_top() + Vec2::new(19.0, 8.0),
                    Align2::LEFT_CENTER,
                    "Long average",
                    FontId::proportional(8.0),
                    theme::palette(ui).text,
                );
                painter.line_segment(
                    [
                        legend.left_top() + Vec2::new(4.0, 20.0),
                        legend.left_top() + Vec2::new(15.0, 20.0),
                    ],
                    Stroke::new(1.8, Color32::from_rgb(245, 155, 55)),
                );
                painter.text(
                    legend.left_top() + Vec2::new(19.0, 20.0),
                    Align2::LEFT_CENTER,
                    "Target ±3 dB",
                    FontId::proportional(8.0),
                    theme::palette(ui).text,
                );
            } else {
                painter.text(
                    legend.left_top() + Vec2::new(5.0, 14.0),
                    Align2::LEFT_CENTER,
                    "No target saved",
                    FontId::proportional(8.0),
                    theme::palette(ui).text,
                );
            }
        }
        if !self.tonal_view
            && let Some(reference) = self
                .compare
                .and_then(|id| self.choices.iter().find(|c| c.id == id))
        {
            let rate = self.rate(Some(reference.id));
            let curve = (0..=160)
                .map(|i| {
                    let hz = 20.0 * 1000.0_f32.powf(i as f32 / 160.0);
                    let db = opentrack_audio::channel_equalizer_response_db(
                        &reference.equalizer,
                        hz,
                        rate,
                    )
                    .clamp(-24.0, 24.0);
                    Pos2::new(
                        plot.left() + plot.width() * i as f32 / 160.0,
                        plot.center().y - db / 48.0 * plot.height(),
                    )
                })
                .collect();
            painter.add(egui::Shape::line(
                curve,
                Stroke::new(1.1, Color32::from_rgb(245, 155, 55)),
            ));
        }
        let short_range_db = self.range_db.unwrap_or(90);
        let axis_values: Vec<i32> = if self.tonal_view {
            vec![24, 12, 0, -12, -24]
        } else {
            vec![
                6,
                0,
                -short_range_db / 3,
                -short_range_db * 2 / 3,
                -short_range_db,
            ]
        };
        for db in axis_values {
            let y = if self.tonal_view {
                tonal_plot.center().y - db as f32 / 48.0 * tonal_plot.height()
            } else {
                plot.bottom()
                    - (db as f32 + short_range_db as f32) / (short_range_db as f32 + 6.0)
                        * plot.height()
            };
            ui.painter().text(
                Pos2::new(plot.right() + 2.0, y),
                Align2::LEFT_CENTER,
                format!("{db}"),
                FontId::proportional(8.0),
                theme::palette(ui).muted,
            );
        }
        ui.painter().text(
            plot.right_top() + Vec2::new(2.0, -10.0),
            Align2::LEFT_TOP,
            if self.tonal_view {
                "rel. dB"
            } else if self.tilt_db_per_octave == 0 {
                "dBFS"
            } else {
                "dBFS tilted"
            },
            FontId::proportional(8.0),
            theme::palette(ui).muted,
        );
    }
}

const TONAL_REGION_BOUNDARIES: [(f32, &str); 5] = [
    (20.0, "Low"),
    (200.0, "Low-Mid"),
    (1_000.0, "High-Mid"),
    (5_000.0, "High"),
    (20_000.0, ""),
];

fn tonal_frequency_x(plot: Rect, frequency_hz: f32) -> f32 {
    plot.left() + plot.width() * (frequency_hz.clamp(20.0, 20_000.0) / 20.0).ln() / 1_000.0_f32.ln()
}

fn paint_tonal_regions(ui: &egui::Ui, plot: Rect) -> Rect {
    let header_height = 16.0_f32.min(plot.height() * 0.12);
    let chart = Rect::from_min_max(
        Pos2::new(plot.left(), plot.top() + header_height),
        plot.right_bottom(),
    );
    let palette = theme::palette(ui);
    let header = Rect::from_min_max(plot.left_top(), Pos2::new(plot.right(), chart.top()));
    ui.painter()
        .rect_filled(header, 0.0, palette.surface_raised);
    for index in 0..TONAL_REGION_BOUNDARIES.len() - 1 {
        let (start_hz, label) = TONAL_REGION_BOUNDARIES[index];
        let end_hz = TONAL_REGION_BOUNDARIES[index + 1].0;
        let left = tonal_frequency_x(plot, start_hz);
        let right = tonal_frequency_x(plot, end_hz);
        let region = Rect::from_min_max(
            Pos2::new(left, chart.top()),
            Pos2::new(right, chart.bottom()),
        );
        ui.painter().text(
            Pos2::new((left + right) * 0.5, header.center().y),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(8.0),
            palette.muted,
        );
        if index % 2 == 1 {
            ui.painter()
                .rect_filled(region, 0.0, palette.surface_raised.gamma_multiply(0.09));
        }
        if index > 0 {
            ui.painter().line_segment(
                [
                    Pos2::new(left, chart.top()),
                    Pos2::new(left, chart.bottom()),
                ],
                Stroke::new(1.0, palette.grid.gamma_multiply(0.55)),
            );
        }
    }
    chart
}

fn update_peak_bins(held: &mut [f32], current: &[f32], decay_db: f32) {
    for (held, current) in held.iter_mut().zip(current) {
        *held = (*held - decay_db.max(0.0)).max(*current);
    }
}

fn tonal_target_tap_label(source_stage: u8) -> &'static str {
    match source_stage {
        0 => "Pre",
        1 => "Post",
        _ => "Imported",
    }
}

fn tonal_target_tap_differs(source_stage: u8, current_stage: SpectrumStage) -> bool {
    source_stage < 2
        && source_stage
            != if current_stage == SpectrumStage::Pre {
                0
            } else {
                1
            }
}

fn capture_stream_has_gap(
    previous_sequence: u64,
    current_sequence: u64,
    sample_rate: u32,
    wall_elapsed: Duration,
) -> bool {
    if sample_rate == 0 || current_sequence <= previous_sequence {
        return false;
    }
    let wall_elapsed = wall_elapsed.as_secs_f64();
    if wall_elapsed <= 0.5 {
        return false;
    }
    let audio_elapsed = (current_sequence - previous_sequence) as f64 / f64::from(sample_rate);
    audio_elapsed + 0.25 < wall_elapsed * 0.5
}

fn spectrum_points(
    frame: &SpectrumFrame,
    plot: Rect,
    range_db: i32,
    tilt_db_per_octave: i8,
) -> Vec<Pos2> {
    let width = plot.width().ceil().clamp(2.0, 2048.0) as usize;
    let mut levels = vec![-120.0_f32; width];
    for (index, db) in frame.bins_dbfs.iter().enumerate().skip(1) {
        let hz = frame.frequency_hz(index);
        if !(20.0..=20_000.0).contains(&hz) || !db.is_finite() || *db <= -119.0 {
            continue;
        }
        let column = ((hz / 20.0).ln() / 1000.0_f32.ln() * (width - 1) as f32).round() as usize;
        levels[column] = levels[column].max(tilt_compensated_db(*db, hz, tilt_db_per_octave));
    }
    levels
        .iter()
        .enumerate()
        .filter(|(_, db)| **db > -119.0)
        .map(|(index, db)| {
            Pos2::new(
                plot.left() + index as f32 / (width - 1) as f32 * plot.width(),
                plot.bottom()
                    - ((*db + range_db as f32) / (range_db as f32 + 6.0)).clamp(0.0, 1.0)
                        * plot.height(),
            )
        })
        .collect()
}

fn tilt_compensated_db(level_db: f32, frequency_hz: f32, db_per_octave: i8) -> f32 {
    level_db + f32::from(db_per_octave) * (1_000.0 / frequency_hz.max(1.0)).log2()
}

fn paint_tonal_profile(
    ui: &egui::Ui,
    plot: Rect,
    powers: &[f64],
    color: Color32,
    show_target_range: bool,
) {
    if powers.len() != TONAL_BANDS {
        return;
    }
    let mean = powers.iter().sum::<f64>() / TONAL_BANDS as f64;
    if mean <= 0.0 {
        return;
    }
    let relative_db = powers
        .iter()
        .map(|power| (10.0 * (power.max(mean * 1.0e-6) / mean).log10()) as f32)
        .collect::<Vec<_>>();
    let relative_db = smooth_tonal_display_curve(&relative_db);
    let point_at = |index: usize, db: f32| {
        let x = plot.left() + (index as f32 + 0.5) / TONAL_BANDS as f32 * plot.width();
        let y = plot.center().y
            - (db / 48.0 * plot.height()).clamp(-plot.height() / 2.0, plot.height() / 2.0);
        Pos2::new(x, y)
    };
    if show_target_range {
        const TARGET_TOLERANCE_DB: f32 = 3.0;
        let mut mesh = egui::Mesh::default();
        for index in 0..TONAL_BANDS - 1 {
            let lower_left = point_at(index, relative_db[index] - TARGET_TOLERANCE_DB);
            let upper_left = point_at(index, relative_db[index] + TARGET_TOLERANCE_DB);
            let lower_right = point_at(index + 1, relative_db[index + 1] - TARGET_TOLERANCE_DB);
            let upper_right = point_at(index + 1, relative_db[index + 1] + TARGET_TOLERANCE_DB);
            let base = mesh.vertices.len() as u32;
            let fill = color.gamma_multiply(0.16);
            for position in [lower_left, upper_left, lower_right, upper_right] {
                mesh.colored_vertex(position, fill);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 1, base + 3, base + 2);
        }
        ui.painter().add(egui::Shape::mesh(mesh));
    }
    let points = relative_db
        .iter()
        .enumerate()
        .map(|(index, db)| point_at(index, *db))
        .collect();
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.8, color)));
}

fn paint_tonal_difference(
    ui: &egui::Ui,
    plot: Rect,
    average: &[f64],
    target: &TonalTarget,
    color: Color32,
) {
    if average.len() != TONAL_BANDS {
        return;
    }
    let measured = average
        .iter()
        .map(|power| *power as f32)
        .collect::<Vec<_>>();
    let target_profile = display_target_profile(target);
    let Some(difference) =
        opentrack_audio::normalized_log_band_difference_db(&measured, &target_profile)
    else {
        return;
    };
    let difference = smooth_tonal_display_curve(&difference);
    const GUIDANCE_DB: f32 = 3.0;
    let guidance_y = |db: f32| plot.center().y - db / 48.0 * plot.height();
    let guidance = Rect::from_min_max(
        Pos2::new(plot.left(), guidance_y(GUIDANCE_DB)),
        Pos2::new(plot.right(), guidance_y(-GUIDANCE_DB)),
    );
    let palette = theme::palette(ui);
    ui.painter()
        .rect_filled(guidance, 0.0, color.gamma_multiply(0.1));
    for db in [-GUIDANCE_DB, 0.0, GUIDANCE_DB] {
        let y = guidance_y(db);
        ui.painter().line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(
                if db == 0.0 { 1.0 } else { 0.6 },
                if db == 0.0 {
                    palette.grid
                } else {
                    palette.muted.gamma_multiply(0.55)
                },
            ),
        );
    }
    let points = difference
        .iter()
        .enumerate()
        .map(|(index, difference)| {
            let x = plot.left() + (index as f32 + 0.5) / TONAL_BANDS as f32 * plot.width();
            let y = plot.center().y - *difference / 48.0 * plot.height();
            Pos2::new(x, y)
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.8, color)));
}

fn smooth_tonal_display_curve(values: &[f32]) -> Vec<f32> {
    const RADIUS: isize = 3;
    const SIGMA: f32 = 1.15;
    values
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let mut weighted_sum = 0.0;
            let mut total_weight = 0.0;
            for offset in -RADIUS..=RADIUS {
                let source_index = index as isize + offset;
                if !(0..values.len() as isize).contains(&source_index) {
                    continue;
                }
                let distance = offset as f32 / SIGMA;
                let weight = (-0.5 * distance * distance).exp();
                let value = values[source_index as usize];
                if value.is_finite() {
                    weighted_sum += value * weight;
                    total_weight += weight;
                }
            }
            if total_weight > 0.0 {
                weighted_sum / total_weight
            } else {
                0.0
            }
        })
        .collect()
}

fn display_target_profile(target: &TonalTarget) -> Vec<f32> {
    if target.band_powers.len() == TONAL_BANDS {
        return target.band_powers.clone();
    }
    if !matches!(target.band_powers.len(), 24 | 48) {
        return Vec::new();
    }
    let mut expanded = Vec::with_capacity(TONAL_BANDS);
    for power in &target.band_powers {
        let subdivisions = TONAL_BANDS / target.band_powers.len();
        expanded.extend(std::iter::repeat_n(
            *power / subdivisions as f32,
            subdivisions,
        ));
    }
    expanded
}

impl Drop for State {
    fn drop(&mut self) {
        self.close();
    }
}

fn update_tonal_average(
    average: &mut Vec<f64>,
    elapsed_seconds: &mut f32,
    profile: &[f32],
    dt: f32,
    time_constant_seconds: u32,
    held: bool,
) -> bool {
    if held
        || profile.len() != TONAL_BANDS
        || !dt.is_finite()
        || dt <= 0.0
        || profile
            .iter()
            .any(|power| !power.is_finite() || *power < 0.0)
        || profile.iter().all(|power| *power == 0.0)
    {
        return false;
    }
    if average.len() != TONAL_BANDS {
        *average = profile.iter().map(|power| f64::from(*power)).collect();
    } else {
        let time_constant = time_constant_seconds.max(1) as f32;
        let alpha = f64::from((dt / (time_constant + dt)).clamp(0.0, 1.0));
        for (average, power) in average.iter_mut().zip(profile) {
            *average += (f64::from(*power) - *average) * alpha;
        }
    }
    *elapsed_seconds += dt;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_channel_or_closing_resets_analysis_and_releases_taps() {
        let a = Arc::new(StereoMeter::default());
        let b = Arc::new(StereoMeter::default());
        let choices = vec![
            Choice {
                id: 1,
                name: "A".into(),
                channel_count: 2,
                equalizer: Default::default(),
            },
            Choice {
                id: 2,
                name: "B".into(),
                channel_count: 2,
                equalizer: Default::default(),
            },
        ];
        let available = HashMap::from([(1, a.clone()), (2, b.clone())]);
        let mut state = State::default();
        state.update(
            Some(1),
            choices.clone(),
            available.clone(),
            48000,
            false,
            0.0,
        );
        state.stage = SpectrumStage::Pre;
        state.compare = Some(2);
        state.update(
            Some(1),
            choices.clone(),
            available.clone(),
            48000,
            false,
            0.0,
        );
        assert!(a.spectrum_enabled() && b.spectrum_enabled());
        state.update(Some(2), choices, available, 44100, false, 0.0);
        assert_eq!(state.stage, SpectrumStage::Post);
        assert_eq!(state.compare, None);
        assert!(!a.spectrum_enabled());
        assert!(b.spectrum_enabled());
        assert_eq!(state.rate(None), 44100);
        state.stage = SpectrumStage::Pre;
        state.compare = Some(1);
        state.close();
        assert!(!b.spectrum_enabled());
        assert_eq!(state.stage, SpectrumStage::Post);
        assert_eq!(state.compare, None);
    }

    #[test]
    fn tonal_profile_aggregates_into_normalized_log_frequency_bands() {
        let mut bins = vec![-120.0; TONAL_FFT_SIZE / 2 + 1];
        bins[170] = -6.0;
        bins[1_024] = -12.0;
        let profile = SpectrumFrame {
            sequence: 1,
            sample_rate: 48_000,
            fft_size: TONAL_FFT_SIZE,
            rms_dbfs: -20.0,
            bins_dbfs: bins,
        }
        .normalized_log_band_powers();
        assert_eq!(profile.len(), TONAL_BANDS);
        let total: f32 = profile.iter().sum();
        assert!((total - 1.0).abs() < 1e-5);
        assert!(profile.iter().filter(|power| **power > 0.0).count() >= 2);
    }

    #[test]
    fn old_tonal_profiles_expand_to_the_fine_display_without_changing_total_power() {
        for (version, source_bands) in [(1, 24), (2, 48)] {
            let target = TonalTarget {
                name: "Legacy".into(),
                analysis_version: version,
                sample_rate: 48_000,
                source_stage: 2,
                source_name: "legacy.wav".into(),
                analysis_seconds: 30.0,
                fft_size: 4096,
                band_powers: (0..source_bands).map(|index| (index + 1) as f32).collect(),
            };
            let expanded = display_target_profile(&target);
            let subdivisions = TONAL_BANDS / source_bands;
            assert_eq!(expanded.len(), TONAL_BANDS);
            assert!(
                (expanded.iter().sum::<f32>() - target.band_powers.iter().sum::<f32>()).abs()
                    < 1e-4
            );
            for (source_index, source_power) in target.band_powers.iter().enumerate() {
                let start = source_index * subdivisions;
                assert!(
                    expanded[start..start + subdivisions]
                        .iter()
                        .all(|power| (*power - source_power / subdivisions as f32).abs() < 1e-6)
                );
            }
        }
    }

    #[test]
    fn tonal_plot_smoothing_reduces_isolated_spikes_without_changing_constant_profiles() {
        let mut spiked = vec![0.0; TONAL_BANDS];
        spiked[TONAL_BANDS / 2] = 18.0;
        let smoothed = smooth_tonal_display_curve(&spiked);
        assert!(smoothed[TONAL_BANDS / 2] < 18.0);
        assert!(smoothed[TONAL_BANDS / 2 - 1] > 0.0);
        assert!(smoothed[TONAL_BANDS / 2 + 1] > 0.0);
        assert_eq!(smoothed.len(), TONAL_BANDS);

        let constant = vec![-6.0; TONAL_BANDS];
        let smoothed_constant = smooth_tonal_display_curve(&constant);
        assert!(
            smoothed_constant
                .iter()
                .all(|value| (*value + 6.0).abs() < 1e-6)
        );
    }

    #[test]
    fn tonal_region_guides_follow_log_frequency_boundaries() {
        let plot = Rect::from_min_max(Pos2::ZERO, Pos2::new(300.0, 200.0));
        assert!((tonal_frequency_x(plot, 200.0) - 100.0).abs() < 0.01);
        assert!((tonal_frequency_x(plot, 1_000.0) - 169.8).abs() < 0.1);
        assert!((tonal_frequency_x(plot, 5_000.0) - 239.7).abs() < 0.1);
        let context = egui::Context::default();
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(plot),
                ..Default::default()
            },
            |ui| {
                ui.painter().rect_filled(plot, 0.0, Color32::BLACK);
                let chart = paint_tonal_regions(ui, plot);
                assert!(chart.top() > plot.top());
                assert_eq!(chart.bottom(), plot.bottom());
            },
        );
        for label in ["Low", "Low-Mid", "High-Mid", "High"] {
            fn has_label(shape: &egui::Shape, label: &str) -> bool {
                match shape {
                    egui::Shape::Text(text) => text.galley.text() == label,
                    egui::Shape::Vec(shapes) => shapes.iter().any(|shape| has_label(shape, label)),
                    _ => false,
                }
            }
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|shape| has_label(&shape.shape, label))
            );
        }
        output.textures_delta.clear();
    }

    #[test]
    fn sample_rate_or_channel_layout_change_marks_capture_incomplete() {
        let meter = Arc::new(StereoMeter::default());
        let available = HashMap::from([(1, meter)]);
        let choice = |channel_count| Choice {
            id: 1,
            name: "Audio".into(),
            channel_count,
            equalizer: Default::default(),
        };
        let mut state = State::default();
        state.update(
            Some(1),
            vec![choice(1)],
            available.clone(),
            48_000,
            false,
            0.0,
        );
        state.capture_active = true;
        state.capture_seconds = 7.0;
        state.capture_frames = 20;
        state.capture_sum = vec![1.0; TONAL_BANDS];

        state.update(Some(1), vec![choice(2)], available, 44_100, true, 0.0);
        assert!(!state.capture_active);
        assert!(state.capture_incomplete);
        assert_eq!(state.capture_seconds, 7.0);
        assert_eq!(state.capture_frames, 0);
        assert!(state.capture_sum.is_empty());
        assert!(state.tonal_average.is_empty());
        assert_eq!(state.channel_count, Some(2));
    }

    #[test]
    fn changing_tonal_tap_marks_an_active_capture_incomplete() {
        let mut state = State::default();
        state.capture_active = true;
        state.capture_seconds = 7.0;
        state.capture_frames = 20;
        state.capture_sum = vec![1.0; TONAL_BANDS];
        state.capture_sequence.insert(1, 20);
        state.tonal_average = vec![1.0 / TONAL_BANDS as f64; TONAL_BANDS];
        state.tonal_average_elapsed = 30.0;
        state.tonal_average_held = true;

        state.invalidate_capture_for_tap_change();

        assert!(!state.capture_active);
        assert!(state.capture_incomplete);
        assert_eq!(state.capture_seconds, 7.0);
        assert_eq!(state.capture_frames, 0);
        assert!(state.capture_sum.is_empty());
        assert!(state.capture_sequence.is_empty());
        assert!(state.tonal_average.is_empty());
        assert_eq!(state.tonal_average_elapsed, 0.0);
        assert!(!state.tonal_average_held);
    }

    #[test]
    fn completed_reference_import_does_not_overwrite_a_newer_saved_target() {
        let initial_target = TonalTarget {
            name: "Initial".into(),
            analysis_version: 3,
            sample_rate: 48_000,
            source_stage: 2,
            source_name: "initial.wav".into(),
            analysis_seconds: 20.0,
            fft_size: TONAL_FFT_SIZE as u32,
            band_powers: vec![1.0 / TONAL_BANDS as f32; TONAL_BANDS],
        };
        let mut newer_target = initial_target.clone();
        newer_target.name = "New Capture".into();
        newer_target.source_name = "Channel capture".into();
        let mut equalizer = ChannelEqualizer::default();
        equalizer.tonal_target = Some(newer_target.clone());

        let (sender, result) = mpsc::sync_channel(1);
        sender
            .send(Ok(ReferenceImportPayload {
                display_name: "Older Import".into(),
                source_name: "older.wav".into(),
                profile: TonalProfile {
                    sample_rate: 48_000,
                    duration_seconds: 20.0,
                    analyzed_windows: 100,
                    band_powers: vec![1.0 / TONAL_BANDS as f32; TONAL_BANDS],
                },
            }))
            .unwrap();

        let mut state = State::default();
        state.owner = Some(1);
        state.import_task = Some(ReferenceImportTask {
            owner: 1,
            initial_target: Some(initial_target),
            cancel: Arc::new(AtomicBool::new(false)),
            result,
        });
        assert!(!state.poll_reference_import(&mut equalizer));
        assert_eq!(equalizer.tonal_target, Some(newer_target));
        assert!(state.import_status.contains("target changed"));
    }

    #[test]
    fn tonal_and_short_analyzer_keep_independent_tap_settings() {
        let mut state = State::default();
        state.stage = SpectrumStage::Pre;
        state.tonal_stage = SpectrumStage::Post;
        state.resolution = SpectrumResolution::Fine;
        state.tonal_view = false;
        assert_eq!(state.active_stage(), SpectrumStage::Pre);
        assert_eq!(state.active_resolution(), SpectrumResolution::Fine);
        state.tonal_view = true;
        assert_eq!(state.active_stage(), SpectrumStage::Post);
        assert_eq!(state.active_resolution(), SpectrumResolution::Fine);
        assert_eq!(state.stage, SpectrumStage::Pre);
    }

    #[test]
    fn tonal_average_uses_selected_time_constant_and_hold_stops_elapsed_time() {
        let mut average = vec![0.0; TONAL_BANDS];
        average[0] = 1.0;
        let mut profile = vec![0.0; TONAL_BANDS];
        profile[1] = 1.0;
        let mut elapsed = 0.0;
        assert!(update_tonal_average(
            &mut average,
            &mut elapsed,
            &profile,
            1.0,
            10,
            false
        ));
        assert!((average[0] - 10.0 / 11.0).abs() < 1.0e-7);
        assert_eq!(elapsed, 1.0);

        let before = average.clone();
        assert!(!update_tonal_average(
            &mut average,
            &mut elapsed,
            &vec![1.0 / TONAL_BANDS as f32; TONAL_BANDS],
            5.0,
            10,
            true
        ));
        assert_eq!(average, before);
        assert_eq!(elapsed, 1.0);
    }

    #[test]
    fn tonal_average_ignores_silence_and_starts_from_first_valid_profile() {
        let mut average = Vec::new();
        let mut elapsed = 0.0;
        assert!(!update_tonal_average(
            &mut average,
            &mut elapsed,
            &[0.0; TONAL_BANDS],
            0.1,
            30,
            false
        ));
        assert!(average.is_empty());
        assert_eq!(elapsed, 0.0);

        let profile = vec![1.0 / TONAL_BANDS as f32; TONAL_BANDS];
        assert!(update_tonal_average(
            &mut average,
            &mut elapsed,
            &profile,
            0.1,
            30,
            false
        ));
        assert_eq!(average.len(), TONAL_BANDS);
        assert!((average.iter().sum::<f64>() - 1.0).abs() < 1.0e-6);
        assert!((elapsed - 0.1).abs() < 1.0e-6);
    }

    #[test]
    fn peak_hold_tracks_new_peaks_and_decays_at_a_bounded_rate() {
        let mut held = vec![-10.0, -20.0, -30.0];
        update_peak_bins(&mut held, &[-30.0, -5.0, -25.0], 1.0);
        assert_eq!(held, vec![-11.0, -5.0, -25.0]);
    }

    #[test]
    fn spectrum_tilt_changes_display_level_around_one_kilohertz() {
        assert_eq!(tilt_compensated_db(-24.0, 1_000.0, 6), -24.0);
        assert_eq!(tilt_compensated_db(-24.0, 500.0, 6), -18.0);
        assert_eq!(tilt_compensated_db(-24.0, 2_000.0, 6), -30.0);
    }

    #[test]
    fn capture_stream_gap_detection_distinguishes_gaps_from_live_progress() {
        assert!(!capture_stream_has_gap(
            48_000,
            96_000,
            48_000,
            Duration::from_secs(1)
        ));
        assert!(capture_stream_has_gap(
            48_000,
            52_800,
            48_000,
            Duration::from_secs(1)
        ));
        assert!(!capture_stream_has_gap(
            48_000,
            49_000,
            48_000,
            Duration::from_millis(200)
        ));
        assert!(!capture_stream_has_gap(
            48_000,
            48_000,
            48_000,
            Duration::from_secs(2)
        ));
    }

    #[test]
    fn detected_stream_gap_stops_capture_and_rejects_partial_target() {
        let now = Instant::now();
        let mut state = State::default();
        state.owner = Some(7);
        state.capture_active = true;
        state.capture_seconds = 8.0;
        state.capture_frames = 200;
        state.capture_sum = vec![1.0; TONAL_BANDS];
        state.seen.insert(7, (48_000, now - Duration::from_secs(1)));

        State::observe_spectrum_sequence(
            &mut state.seen,
            state.owner,
            &mut state.capture_active,
            &mut state.capture_incomplete,
            &mut state.capture_sum,
            &mut state.capture_frames,
            7,
            52_800,
            48_000,
            now,
        );

        assert!(!state.capture_active);
        assert!(state.capture_incomplete);
        assert_eq!(state.capture_seconds, 8.0);
        assert_eq!(state.capture_frames, 0);
        assert!(state.capture_sum.is_empty());
    }

    #[test]
    fn tonal_target_summary_maps_capture_and_import_taps() {
        assert_eq!(tonal_target_tap_label(0), "Pre");
        assert_eq!(tonal_target_tap_label(1), "Post");
        assert_eq!(tonal_target_tap_label(2), "Imported");
        assert!(!tonal_target_tap_differs(0, SpectrumStage::Pre));
        assert!(tonal_target_tap_differs(0, SpectrumStage::Post));
        assert!(!tonal_target_tap_differs(2, SpectrumStage::Pre));
        assert!(!tonal_target_tap_differs(2, SpectrumStage::Post));
    }

    #[test]
    fn tonal_toolbar_wraps_and_keeps_target_and_capture_controls_visible_when_narrow() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(460.0, 48.0));
        let mut state = State::default();
        state.tonal_view = true;
        state.tonal_stage = SpectrumStage::Post;
        state.transport_active = true;
        assert_eq!(state.controls_height(460.0), 48.0);
        assert_eq!(state.controls_height(800.0), 48.0);
        let mut equalizer = ChannelEqualizer::default();
        equalizer.tonal_target = Some(TonalTarget {
            name: "Mix Reference".into(),
            analysis_version: 3,
            sample_rate: 48_000,
            source_stage: 0,
            source_name: "Mix.wav".into(),
            analysis_seconds: 45.0,
            fft_size: TONAL_FFT_SIZE as u32,
            band_powers: vec![1.0 / TONAL_BANDS as f32; TONAL_BANDS],
        });
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ui| {
                state.controls(ui, rect, &mut equalizer);
            },
        );
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some((text.galley.text().to_owned(), text.pos.y)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(texts.iter().any(|(text, _)| text.contains("Mix Reference")));
        assert!(texts.iter().any(|(text, _)| text == "Import WAV"));
        assert!(texts.iter().any(|(text, _)| text == "Capture"));
        let min_y = texts.iter().map(|(_, y)| *y).fold(f32::INFINITY, f32::min);
        let max_y = texts
            .iter()
            .map(|(_, y)| *y)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max_y - min_y > 12.0,
            "toolbar items should wrap to rows: {texts:?}"
        );
        assert!(
            max_y < rect.bottom(),
            "wrapped controls must stay in the row area"
        );
        output.textures_delta.clear();
    }
}
