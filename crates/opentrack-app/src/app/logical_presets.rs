//! Bounded named MIDI logical presets.
//!
//! Deliberately not a scripting engine. Selection presets only change the shared
//! transient note selection; the one content preset (set channel) is atomic and
//! leaves every other note and controller event untouched.

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LogicalPreset {
    SelectPitchRange,
    SelectVelocityRange,
    SetChannel,
}

impl LogicalPreset {
    pub(super) const ALL: [Self; 3] = [
        Self::SelectPitchRange,
        Self::SelectVelocityRange,
        Self::SetChannel,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::SelectPitchRange => "Select by pitch range",
            Self::SelectVelocityRange => "Select by velocity range",
            Self::SetChannel => "Set selected-note MIDI channel",
        }
    }

    fn is_selection_only(self) -> bool {
        !matches!(self, Self::SetChannel)
    }
}

pub(super) struct LogicalPresetDialog {
    pub preset: LogicalPreset,
    pub minimum: i32,
    pub maximum: i32,
    pub channel: u8,
}

impl Default for LogicalPresetDialog {
    fn default() -> Self {
        Self {
            preset: LogicalPreset::SelectPitchRange,
            minimum: 36,
            maximum: 51,
            channel: 0,
        }
    }
}

impl OpenTrackApp {
    pub(super) fn open_logical_presets(&mut self) {
        if self.data.project.selected_midi_part_id.is_none() {
            self.status_message = "Select a MIDI part to use a logical preset".to_owned();
            return;
        }
        self.logical_preset_dialog = Some(LogicalPresetDialog::default());
    }

    fn with_selected_part(
        &mut self,
        mutate: impl FnOnce(&mut opentrack_core::project::MidiPart) -> bool,
    ) -> bool {
        let Some(part_id) = self.data.project.selected_midi_part_id else {
            self.status_message = "Select a MIDI part first".to_owned();
            return false;
        };
        let mut candidate = self.data.project.clone();
        let changed = candidate
            .tracks
            .iter_mut()
            .flat_map(|track| track.midi_parts.iter_mut())
            .find(|part| part.id == part_id)
            .is_some_and(mutate);
        if !changed {
            self.status_message = "No note matched the preset".to_owned();
            return false;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot apply preset: undo snapshot is unavailable".to_owned();
            return false;
        }
        self.project_history.label_next("Logical Preset");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        true
    }

    pub(super) fn apply_logical_preset(
        &mut self,
        preset: LogicalPreset,
        minimum: i32,
        maximum: i32,
        channel: u8,
    ) {
        let Some(part_id) = self.data.project.selected_midi_part_id else {
            self.status_message = "Select a MIDI part to use a logical preset".to_owned();
            return;
        };
        if minimum > maximum {
            self.status_message = "The preset range is inverted".to_owned();
            return;
        }
        if preset.is_selection_only() {
            let Some(part) = self
                .data
                .project
                .tracks
                .iter()
                .flat_map(|track| track.midi_parts.iter())
                .find(|part| part.id == part_id)
            else {
                return;
            };
            let matched: Vec<u64> = part
                .notes
                .iter()
                .filter(|note| match preset {
                    LogicalPreset::SelectPitchRange => {
                        (i32::from(note.pitch)).clamp(0, 127) >= minimum
                            && i32::from(note.pitch) <= maximum
                    }
                    LogicalPreset::SelectVelocityRange => {
                        i32::from(note.velocity) >= minimum && i32::from(note.velocity) <= maximum
                    }
                    LogicalPreset::SetChannel => false,
                })
                .map(|note| note.id)
                .collect();
            if matched.is_empty() {
                self.status_message =
                    "No note matched the preset; the selection is unchanged".to_owned();
                return;
            }
            self.selected_midi_note_ids = matched.iter().copied().collect();
            self.status_message = format!(
                "{} matched {} note(s); project content is unchanged",
                preset.label(),
                matched.len()
            );
            return;
        }

        if !(0..=15).contains(&channel) {
            self.status_message = "The MIDI channel must be 0 through 15".to_owned();
            return;
        }
        let ids = self.selected_midi_note_ids.clone();
        if ids.is_empty() {
            self.status_message = "Select notes before setting their channel".to_owned();
            return;
        }
        let mut changed = 0;
        if self.with_selected_part(|part| {
            let mut changed_here = false;
            for note in &mut part.notes {
                if ids.contains(&note.id) && note.channel != channel {
                    note.channel = channel;
                    changed_here = true;
                    changed += 1;
                }
            }
            changed_here
        }) {
            self.status_message = format!(
                "{}: moved {changed} note(s) to channel {}; controller events are untouched",
                preset.label(),
                channel + 1
            );
        }
    }

    pub(super) fn show_logical_presets(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.logical_preset_dialog.take() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let response = egui::Window::new("Logical Presets")
            .id(Id::new("logical-presets-dialog"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(340.0);
                for preset in LogicalPreset::ALL {
                    ui.selectable_value(&mut dialog.preset, preset, preset.label());
                }
                ui.separator();
                match dialog.preset {
                    LogicalPreset::SelectPitchRange => {
                        ui.label("Pitch range (MIDI note numbers)");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut dialog.minimum).range(0..=127));
                            ui.label("to");
                            ui.add(egui::DragValue::new(&mut dialog.maximum).range(0..=127));
                        });
                    }
                    LogicalPreset::SelectVelocityRange => {
                        ui.label("Velocity range (1-127)");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut dialog.minimum).range(1..=127));
                            ui.label("to");
                            ui.add(egui::DragValue::new(&mut dialog.maximum).range(1..=127));
                        });
                    }
                    LogicalPreset::SetChannel => {
                        ui.label("MIDI channel (1-16)");
                        let mut shown = i32::from(dialog.channel) + 1;
                        if ui
                            .add(egui::DragValue::new(&mut shown).range(1..=16))
                            .changed()
                        {
                            dialog.channel = (shown - 1).clamp(0, 15) as u8;
                        }
                        ui.label("Applies to the current note selection only.");
                    }
                }
                ui.label(
                    egui::RichText::new(
                        "Selection presets change only the transient note selection. Set Channel is one undo step and does not alter CC events.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        super::dismiss_window_on_outside_click(
            ctx,
            Id::new("logical-presets-dialog"),
            &mut open,
            response.as_ref(),
        );
        if apply {
            self.apply_logical_preset(
                dialog.preset,
                dialog.minimum,
                dialog.maximum,
                dialog.channel,
            );
        } else if !cancel && open {
            self.logical_preset_dialog = Some(dialog);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_notes() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Midi);
        let part = app.data.project.add_midi_part(0, 0.0, 8.0).expect("part");
        app.data.project.selected_midi_part_id = Some(part);
        for (start, pitch, velocity, channel) in
            [(0.0, 36, 80, 0), (1.0, 38, 110, 0), (2.0, 60, 30, 1)]
        {
            app.data
                .project
                .add_midi_note_on_channel(0, part, start, 0.5, pitch, channel, velocity)
                .expect("note");
        }
        app.data
            .project
            .add_midi_controller_event(0, part, 0.0, 11, 100)
            .expect("cc");
        app.project_history.clear();
        app
    }

    #[test]
    fn selection_presets_change_only_the_transient_selection() {
        let mut app = app_with_notes();
        let before = app.data.project.clone();
        app.apply_logical_preset(LogicalPreset::SelectPitchRange, 36, 40, 0);
        assert_eq!(app.selected_midi_note_ids.len(), 2);
        assert_eq!(app.data.project, before);
        assert_eq!(app.project_history.counts().0, 0);
        assert!(app.status_message.contains("content is unchanged"));

        app.apply_logical_preset(LogicalPreset::SelectVelocityRange, 100, 127, 0);
        assert_eq!(app.selected_midi_note_ids.len(), 1);
        app.apply_logical_preset(LogicalPreset::SelectPitchRange, 90, 127, 0);
        assert!(app.status_message.contains("No note matched"));
        assert_eq!(
            app.selected_midi_note_ids.len(),
            1,
            "no match leaves selection"
        );
    }

    #[test]
    fn setting_channel_is_atomic_and_preserves_other_notes_and_cc() {
        let mut app = app_with_notes();
        app.selected_midi_note_ids = app.data.project.tracks[0].midi_parts[0]
            .notes
            .iter()
            .filter(|note| note.pitch == 36)
            .map(|note| note.id)
            .collect();
        app.apply_logical_preset(LogicalPreset::SetChannel, 0, 0, 4);
        let part = &app.data.project.tracks[0].midi_parts[0];
        let changed = part.notes.iter().find(|note| note.pitch == 36).unwrap();
        assert_eq!(changed.channel, 4);
        assert_eq!(
            part.notes
                .iter()
                .find(|note| note.pitch == 60)
                .unwrap()
                .channel,
            1
        );
        assert_eq!(part.controller_events[0].value, 100);
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Logical Preset");
        app.undo_project();
        assert_eq!(
            app.data.project.tracks[0].midi_parts[0]
                .notes
                .iter()
                .find(|note| note.pitch == 36)
                .unwrap()
                .channel,
            0
        );
    }

    #[test]
    fn setting_channel_without_a_selection_or_with_an_invalid_channel_is_refused() {
        let mut app = app_with_notes();
        app.selected_midi_note_ids.clear();
        app.apply_logical_preset(LogicalPreset::SetChannel, 0, 0, 2);
        assert!(app.status_message.contains("Select notes"));
        assert_eq!(app.project_history.counts().0, 0);
    }
}
