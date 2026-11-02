//! Staged rendering for an event's offline processing chain.
//!
//! Every processor reads the previous stage's file and writes a new one, so a
//! failure at any point leaves the project untouched and only staged files are
//! removed. The event is repointed only after the last stage succeeds.

use super::*;
use opentrack_core::project::OfflineProcess;

/// Handles of the files a chain render created, so a failure can clean up.
struct Staging {
    files: Vec<PathBuf>,
    directory: PathBuf,
}

impl Staging {
    fn path_for(&mut self, root: &Path, subfolder: &str, name: &str) -> Result<PathBuf, String> {
        let path = super::audio_export::unique_wav_path(root, subfolder, name)?;
        self.files.push(path.clone());
        Ok(path)
    }

    fn clean(&self) {
        for file in &self.files {
            let _ = std::fs::remove_file(file);
        }
        let _ = &self.directory;
    }
}

/// The duration multiplier a chain applies (time stretch scales, resampling does not).
fn duration_factor(chain: &[OfflineProcess]) -> f64 {
    chain
        .iter()
        .map(|process| match process {
            OfflineProcess::TimeStretch { percent } => f64::from(*percent) / 100.0,
            _ => 1.0,
        })
        .product()
}

/// Reads a whole staged file's duration in seconds.
fn file_seconds(path: &Path) -> Result<f64, String> {
    let metadata = opentrack_audio::audio_file_metadata(path)?;
    if metadata.frames == 0 || metadata.sample_rate == 0 {
        return Err(format!("{} holds no audio", path.display()));
    }
    Ok(f64::from(metadata.frames) / f64::from(metadata.sample_rate))
}

impl OpenTrackApp {
    /// Bakes one event's offline chain into new media as one history step.
    pub(super) fn apply_offline_chain(&mut self, clip_id: u64) {
        if self.is_recording() {
            self.status_message = "Stop recording before processing offline".to_owned();
            return;
        }
        let Some(clip) = self
            .data
            .project
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .find(|clip| clip.id == clip_id)
        else {
            self.status_message = "The selected event is no longer in the project".to_owned();
            return;
        };
        if clip.locked {
            self.status_message = "A locked event refuses offline processing".to_owned();
            return;
        }
        if clip.offline_chain.is_empty() {
            self.status_message = "Add at least one processor before applying the chain".to_owned();
            return;
        }
        if !self.project_folder_managed {
            self.status_message =
                "Set up this project's folder before processing offline".to_owned();
            return;
        }
        let Some(root) = self.project_root() else {
            self.status_message =
                "Save this project into its folder before processing offline".to_owned();
            return;
        };
        let subfolder = self.pool_import_subfolder();
        let chain = clip.offline_chain.clone();
        let source = clip.source_path.clone();
        let offset = clip.source_offset_seconds;
        let duration = clip.duration_seconds;
        let name = clip.name.clone();
        let factor = duration_factor(&chain);
        let mut staging = Staging {
            files: Vec::new(),
            directory: root.clone(),
        };

        let mut current = source.clone();
        let mut current_offset = offset;
        let mut current_duration = duration;
        let mut staged_media = false;
        for (index, process) in chain.iter().enumerate() {
            let destination =
                match staging.path_for(&root, &subfolder, &format!("{name} step{}", index + 1)) {
                    Ok(path) => path,
                    Err(error) => {
                        staging.clean();
                        self.status_message = error;
                        return;
                    }
                };
            let step = if staged_media {
                let seconds = match file_seconds(&current) {
                    Ok(seconds) => seconds,
                    Err(error) => {
                        staging.clean();
                        self.status_message = error;
                        return;
                    }
                };
                (0.0, seconds)
            } else {
                (current_offset, current_duration)
            };
            let result = run_processor(process, &current, step.0, step.1, &destination, &staging);
            if let Err(error) = result {
                staging.clean();
                self.status_message =
                    format!("Offline processing failed at {}: {error}", process.label());
                return;
            }
            current = destination;
            current_offset = 0.0;
            current_duration = step.1;
            staged_media = true;
        }

        let audio = match AudioSource::open(&current) {
            Ok(audio) => audio,
            Err(error) => {
                staging.clean();
                self.status_message = format!("Could not open the processed audio: {error}");
                return;
            }
        };
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            staging.clean();
            self.status_message = "Cannot process offline: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Direct Offline Processing");
        if let Some(clip) = self
            .data
            .project
            .tracks
            .iter_mut()
            .flat_map(|track| track.clips.iter_mut())
            .find(|clip| clip.id == clip_id)
        {
            clip.source_path = current.clone();
            clip.source_offset_seconds = 0.0;
            clip.duration_seconds = duration * factor;
            clip.offline_chain.clear();
            clip.pitch_time = None;
            clip.source_fades.clear();
            clip.fade_in_seconds = clip.fade_in_seconds.min(clip.duration_seconds);
            clip.fade_out_seconds = clip
                .fade_out_seconds
                .min((clip.duration_seconds - clip.fade_in_seconds).max(0.0));
        }
        self.data.audio_sources.insert(current.clone(), audio);
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Applied {} offline processor(s) to {name}; Undo restores the original",
            chain.len()
        );
    }
}

/// Runs one processor stage, writing `destination`.
fn run_processor(
    process: &OfflineProcess,
    source: &Path,
    offset: f64,
    duration: f64,
    destination: &Path,
    _staging: &Staging,
) -> Result<(), String> {
    match process {
        OfflineProcess::GainDb { db } => {
            let gain = 10.0_f32.powf(db / 20.0).clamp(0.0, 4.0);
            opentrack_audio::apply_gain_envelope_wav_range(
                source,
                offset,
                duration,
                &[(0.0, gain), (1.0, gain)],
                destination,
            )?;
        }
        OfflineProcess::Normalize { target_dbfs } => {
            let outcome = opentrack_audio::normalize_wav_range(
                source,
                offset,
                duration,
                *target_dbfs,
                destination,
            )?;
            if !outcome.written {
                std::fs::copy(source, destination)
                    .map_err(|error| format!("could not stage the normalized audio: {error}"))?;
            }
        }
        OfflineProcess::Reverse => {
            opentrack_audio::reverse_wav_range(source, offset, duration, destination)?;
        }
        OfflineProcess::RemoveDcOffset => {
            opentrack_audio::remove_dc_offset_wav_range(source, offset, duration, destination)?;
        }
        OfflineProcess::StereoFlip => {
            opentrack_audio::flip_stereo_wav_range(source, offset, duration, destination)?;
        }
        OfflineProcess::Resample { target_rate } => {
            opentrack_audio::resample_wav_range(
                source,
                offset,
                duration,
                *target_rate,
                destination,
            )?;
        }
        OfflineProcess::TimeStretch { percent } => {
            opentrack_audio::stretch_wav_range(
                source,
                offset,
                duration,
                f64::from(*percent) / 100.0,
                destination,
            )?;
        }
    }
    Ok(())
}

/// Editing state for one event's chain, so a burst of edits is one history step.
pub(super) struct ChainDraft {
    pub clip_id: u64,
    pub chain: Vec<OfflineProcess>,
}

impl OpenTrackApp {
    /// Opens the Direct Offline Processing window for the first selected event.
    pub(super) fn open_offline_processing(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to process events offline".to_owned();
            return;
        }
        let Some(clip_id) = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .first()
            .copied()
        else {
            self.status_message = "Select an audio event to process offline".to_owned();
            return;
        };
        let Some(chain) = self.data.project.offline_chain_of(clip_id) else {
            self.status_message = "The selected event is no longer in the project".to_owned();
            return;
        };
        self.offline_chain_draft = Some(ChainDraft {
            clip_id,
            chain: chain.to_vec(),
        });
    }

    /// Commits the draft as one labelled history step.
    fn save_offline_chain_draft(&mut self, draft: &ChainDraft) -> bool {
        if self.data.project.offline_chain_of(draft.clip_id) == Some(draft.chain.as_slice()) {
            return true;
        }
        let mut candidate = self.data.project.clone();
        if let Err(message) = candidate.set_offline_chain(draft.clip_id, draft.chain.clone()) {
            self.status_message = message.to_owned();
            return false;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot edit the chain: undo snapshot is unavailable".to_owned();
            return false;
        }
        self.project_history.label_next("Edit Offline Chain");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        true
    }

    /// Appends one built-in processor to the selected event's chain.
    pub(super) fn append_offline_processor(&mut self, process: OfflineProcess) {
        if !self.project_open {
            self.status_message = "Open a project to process events offline".to_owned();
            return;
        }
        let Some(clip_id) = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .first()
            .copied()
        else {
            self.status_message = "Select an audio event before adding a processor".to_owned();
            return;
        };
        let Some(chain) = self.data.project.offline_chain_of(clip_id) else {
            self.status_message = "The selected event is no longer in the project".to_owned();
            return;
        };
        if chain.len() >= opentrack_core::project::MAX_OFFLINE_CHAIN {
            self.status_message = format!(
                "An offline chain holds at most {} processors",
                opentrack_core::project::MAX_OFFLINE_CHAIN
            );
            return;
        }
        let mut next = chain.to_vec();
        next.push(process);
        let draft = ChainDraft {
            clip_id,
            chain: next,
        };
        if self.save_offline_chain_draft(&draft) {
            self.status_message = format!("Added {} to the offline chain", process.label());
        }
    }

    /// Removes the last processor from the selected event's chain.
    pub(super) fn remove_last_offline_processor(&mut self) {
        let Some(clip_id) = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .first()
            .copied()
        else {
            self.status_message = "Select an audio event before removing a processor".to_owned();
            return;
        };
        let Some(chain) = self.data.project.offline_chain_of(clip_id) else {
            self.status_message = "The selected event is no longer in the project".to_owned();
            return;
        };
        if chain.is_empty() {
            self.status_message = "The selected event has no offline processors".to_owned();
            return;
        }
        let mut next = chain.to_vec();
        let removed = next.pop().expect("the chain is not empty");
        let draft = ChainDraft {
            clip_id,
            chain: next,
        };
        if self.save_offline_chain_draft(&draft) {
            self.status_message = format!("Removed {} from the offline chain", removed.label());
        }
    }

    pub(super) fn show_offline_processing(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.offline_chain_draft.take() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut apply = false;
        let name = self
            .data
            .project
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .find(|clip| clip.id == draft.clip_id)
            .map(|clip| clip.name.clone())
            .unwrap_or_else(|| "the selected event".to_owned());
        let response = egui::Window::new("Direct Offline Processing")
            .id(Id::new("offline-processing"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(460.0, 320.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!("Chain for {name}"));
                ui.label(
                    egui::RichText::new(
                        "Processors run top to bottom when applied. The chain is not baked until Apply.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.separator();
                let mut remove = None;
                let mut move_up = None;
                let mut move_down = None;
                for (index, process) in draft.chain.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}. {}", index + 1, process.label()));
                        match process {
                            OfflineProcess::GainDb { db } => {
                                ui.add(
                                    egui::DragValue::new(db)
                                        .range(-60.0..=12.0)
                                        .speed(0.1)
                                        .suffix(" dB"),
                                );
                            }
                            OfflineProcess::Normalize { target_dbfs } => {
                                ui.add(
                                    egui::DragValue::new(target_dbfs)
                                        .range(-60.0..=0.0)
                                        .speed(0.1)
                                        .suffix(" dBFS"),
                                );
                            }
                            OfflineProcess::Resample { target_rate } => {
                                ui.add(
                                    egui::DragValue::new(target_rate)
                                        .range(8_000..=192_000)
                                        .speed(100.0)
                                        .suffix(" Hz"),
                                );
                            }
                            OfflineProcess::TimeStretch { percent } => {
                                ui.add(
                                    egui::DragValue::new(percent)
                                        .range(25.0..=400.0)
                                        .speed(0.5)
                                        .suffix(" %"),
                                );
                            }
                            _ => {
                                ui.weak(process.description());
                            }
                        }
                        if ui.small_button("↑").clicked() && index > 0 {
                            move_up = Some(index);
                        }
                        if ui.small_button("↓").clicked() {
                            move_down = Some(index);
                        }
                        if ui.small_button("✕").clicked() {
                            remove = Some(index);
                        }
                    });
                }
                if draft.chain.is_empty() {
                    ui.weak("No processors yet.");
                }
                if let Some(index) = remove {
                    draft.chain.remove(index);
                }
                if let Some(index) = move_up {
                    draft.chain.swap(index, index - 1);
                }
                if let Some(index) = move_down
                    && index + 1 < draft.chain.len()
                {
                    draft.chain.swap(index, index + 1);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("offline-add-processor")
                        .selected_text("Add processor")
                        .show_ui(ui, |ui| {
                            for process in [
                                OfflineProcess::GainDb { db: -3.0 },
                                OfflineProcess::Normalize { target_dbfs: -0.1 },
                                OfflineProcess::Reverse,
                                OfflineProcess::RemoveDcOffset,
                                OfflineProcess::StereoFlip,
                                OfflineProcess::Resample { target_rate: 48_000 },
                                OfflineProcess::TimeStretch { percent: 100.0 },
                            ] {
                                if ui
                                    .selectable_label(false, process.label())
                                    .clicked()
                                {
                                    if draft.chain.len() < opentrack_core::project::MAX_OFFLINE_CHAIN {
                                        draft.chain.push(process);
                                    }
                                }
                            }
                        });
                    if ui.button("Save Chain").clicked() {
                        save = true;
                    }
                    if ui
                        .button("Apply Chain")
                        .on_hover_text("Bake the chain into new audio as one undo step")
                        .clicked()
                    {
                        apply = true;
                    }
                });
            });
        super::dismiss_window_on_outside_click(
            ctx,
            Id::new("offline-processing"),
            &mut open,
            response.as_ref(),
        );
        if apply {
            if self.save_offline_chain_draft(&draft) {
                self.apply_offline_chain(draft.clip_id);
            }
            return;
        }
        if save {
            if self.save_offline_chain_draft(&draft) {
                self.status_message = format!(
                    "Saved a {}-processor offline chain for {name}",
                    draft.chain.len()
                );
            }
            self.offline_chain_draft = Some(draft);
            return;
        }
        if open {
            self.offline_chain_draft = Some(draft);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_flat_wav(path: &std::path::Path, frames: u32, amplitude: f32) {
        let mut bytes = Vec::new();
        let data_bytes = frames * 2;
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&(48_000_u32 * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_bytes.to_le_bytes());
        let sample = (amplitude * f32::from(i16::MAX)) as i16;
        for _ in 0..frames {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        std::fs::write(path, bytes).expect("wav");
    }

    fn app_with_clip(
        source: &std::path::Path,
        duration: f64,
        label: &str,
    ) -> (OpenTrackApp, u64, PathBuf) {
        // A distinct root per test: the tests run in parallel and must not share
        // an imports folder.
        let root =
            std::env::temp_dir().join(format!("opentrack-offline-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        create_project_directory_layout(&root).expect("layout");
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        let clip = app
            .data
            .project
            .add_audio_clip(0, source.to_path_buf(), 0.0, duration)
            .expect("clip");
        app.project_history.clear();
        (app, clip, root)
    }

    #[test]
    fn the_menu_adds_and_removes_built_in_processors_undoably() {
        let source_root =
            std::env::temp_dir().join(format!("opentrack-offline-src4-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&source_root);
        std::fs::create_dir_all(&source_root).expect("src");
        let source = source_root.join("flat.wav");
        write_flat_wav(&source, 24_000, 0.25);
        let (mut app, clip, root) = app_with_clip(&source, 0.5, "menu");

        // No selection: nothing is added.
        app.append_offline_processor(OfflineProcess::Reverse);
        assert!(app.status_message.contains("Select an audio event"));

        app.arrange_view_state.audio_events.select_many([clip]);
        app.append_offline_processor(OfflineProcess::GainDb { db: -6.0 });
        app.append_offline_processor(OfflineProcess::Normalize { target_dbfs: -0.1 });
        let chain = app
            .data
            .project
            .offline_chain_of(clip)
            .expect("clip")
            .to_vec();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].label(), "Gain");
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Edit Offline Chain");

        // Removing drops the last processor.
        app.remove_last_offline_processor();
        let chain = app
            .data
            .project
            .offline_chain_of(clip)
            .expect("clip")
            .to_vec();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].label(), "Gain");
        assert!(app.status_message.contains("Removed Normalize"));

        // An empty chain reports instead of writing another step.
        app.remove_last_offline_processor();
        let steps = app.project_history.counts().0;
        app.remove_last_offline_processor();
        assert!(app.status_message.contains("no offline processors"));
        assert_eq!(app.project_history.counts().0, steps);

        // The eight-entry bound is enforced.
        for _ in 0..opentrack_core::project::MAX_OFFLINE_CHAIN {
            app.append_offline_processor(OfflineProcess::Reverse);
        }
        app.append_offline_processor(OfflineProcess::Reverse);
        assert!(app.status_message.contains("at most"));
        assert_eq!(
            app.data.project.offline_chain_of(clip).expect("clip").len(),
            opentrack_core::project::MAX_OFFLINE_CHAIN
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(source_root);
    }

    #[test]
    fn the_editor_saves_and_applies_a_draft_chain() {
        let source_root =
            std::env::temp_dir().join(format!("opentrack-offline-src3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&source_root);
        std::fs::create_dir_all(&source_root).expect("src");
        let source = source_root.join("flat.wav");
        write_flat_wav(&source, 24_000, 0.25);

        let (mut app, clip, root) = app_with_clip(&source, 0.5, "editor");
        // Without a selection the window refuses to open.
        app.open_offline_processing();
        assert!(app.status_message.contains("Select an audio event"));

        app.arrange_view_state.audio_events.select_many([clip]);
        app.open_offline_processing();
        assert_eq!(
            app.offline_chain_draft.as_ref().map(|d| d.chain.len()),
            Some(0)
        );

        // Editing the draft does not touch the project until it is saved.
        let preview = ChainDraft {
            clip_id: clip,
            chain: vec![OfflineProcess::GainDb { db: -6.0 }, OfflineProcess::Reverse],
        };
        assert!(
            app.data
                .project
                .offline_chain_of(clip)
                .expect("clip")
                .is_empty()
        );
        assert!(app.save_offline_chain_draft(&preview));
        assert_eq!(
            app.data.project.offline_chain_of(clip).expect("clip").len(),
            2
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Edit Offline Chain");

        // An invalid draft is refused without touching the project.
        let invalid = ChainDraft {
            clip_id: clip,
            chain: vec![OfflineProcess::GainDb { db: 99.0 }],
        };
        assert!(!app.save_offline_chain_draft(&invalid));
        assert_eq!(
            app.data.project.offline_chain_of(clip).expect("clip").len(),
            2
        );

        // Applying the saved chain bakes it as its own step.
        app.apply_offline_chain(clip);
        assert!(
            app.data
                .project
                .offline_chain_of(clip)
                .expect("clip")
                .is_empty()
        );
        assert!(
            app.status_message
                .contains("Applied 2 offline processor(s)")
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(source_root);
    }

    #[test]
    fn a_chain_bakes_in_order_and_records_one_undo_step() {
        let source_root =
            std::env::temp_dir().join(format!("opentrack-offline-src-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&source_root);
        std::fs::create_dir_all(&source_root).expect("src");
        let source = source_root.join("flat.wav");
        write_flat_wav(&source, 24_000, 0.25);

        let (mut app, clip, root) = app_with_clip(&source, 0.5, "order");
        app.data
            .project
            .set_offline_chain(
                clip,
                vec![OfflineProcess::GainDb { db: -6.0 }, OfflineProcess::Reverse],
            )
            .expect("chain");
        app.apply_offline_chain(clip);

        let processed = app.data.project.tracks[0].clips[0].clone();
        assert_ne!(processed.source_path, source);
        assert!(processed.source_path.exists());
        assert!(
            processed
                .source_path
                .starts_with(root.join("Audio").join("Imports"))
        );
        assert!(processed.offline_chain.is_empty(), "the chain is baked");
        assert!((processed.duration_seconds - 0.5).abs() < 1e-6);
        assert!(
            app.status_message
                .contains("Applied 2 offline processor(s)")
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(
            app.project_history.steps()[0].label,
            "Direct Offline Processing"
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(source_root);
    }

    #[test]
    fn a_stretch_scales_the_event_length_and_a_failed_stage_changes_nothing() {
        let source_root =
            std::env::temp_dir().join(format!("opentrack-offline-src2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&source_root);
        std::fs::create_dir_all(&source_root).expect("src");
        let source = source_root.join("flat.wav");
        write_flat_wav(&source, 24_000, 0.25);

        let (mut app, clip, root) = app_with_clip(&source, 0.5, "stretch");
        app.data
            .project
            .set_offline_chain(clip, vec![OfflineProcess::TimeStretch { percent: 200.0 }])
            .expect("chain");
        app.apply_offline_chain(clip);
        assert!((app.data.project.tracks[0].clips[0].duration_seconds - 1.0).abs() < 1e-6);

        // A mono event refuses Stereo Flip, and nothing is published.
        app.data
            .project
            .set_offline_chain(clip, vec![OfflineProcess::StereoFlip])
            .expect("chain");
        let before = app.data.project.clone();
        let imports_before = std::fs::read_dir(root.join("Audio").join("Imports"))
            .expect("imports")
            .count();
        app.apply_offline_chain(clip);
        assert!(app.status_message.contains("failed at Stereo Flip"));
        assert_eq!(app.data.project, before);
        let imports_after = std::fs::read_dir(root.join("Audio").join("Imports"))
            .expect("imports")
            .count();
        assert_eq!(imports_before, imports_after, "staged files are removed");

        // An empty chain is refused with a reason.
        app.data
            .project
            .set_offline_chain(clip, Vec::new())
            .expect("clear");
        app.apply_offline_chain(clip);
        assert!(app.status_message.contains("Add at least one processor"));
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(source_root);
    }
}
