//! Tempo Track editor: the one place the project tempo and time signature change.
//!
//! The metronome accent pattern stays an application preference, so this editor
//! deliberately does not own it; it points at the existing metronome setup.

use super::*;

/// Editable draft of the project's tempo and time signature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Draft {
    pub bpm: f64,
    pub beats_per_bar: u8,
    pub beat_unit: u8,
}

impl Draft {
    pub(super) fn from_project(project: &Project) -> Self {
        Self {
            bpm: project.tempo_bpm,
            beats_per_bar: project.beats_per_bar,
            beat_unit: project.beat_unit,
        }
    }

    fn matches(self, project: &Project) -> bool {
        (self.bpm - project.tempo_bpm).abs() <= 1e-9
            && self.beats_per_bar == project.beats_per_bar
            && self.beat_unit == project.beat_unit
    }
}

impl OpenTrackApp {
    /// Opens the Tempo Track editor seeded from the project.
    pub(super) fn open_tempo_editor(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to edit its tempo".to_owned();
            return;
        }
        self.tempo_editor = Some(Draft::from_project(&self.data.project));
    }

    /// Applies a tempo and signature draft as one labelled history step.
    pub(super) fn apply_tempo_editor(&mut self, draft: Draft) {
        if !self.project_open {
            self.status_message = "Open a project to edit its tempo".to_owned();
            return;
        }
        if draft.matches(&self.data.project) {
            self.status_message = "Tempo and time signature are unchanged".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        if let Err(message) = candidate.set_tempo(draft.bpm) {
            self.status_message = message.to_owned();
            return;
        }
        if let Err(message) = candidate.set_time_signature(draft.beats_per_bar, draft.beat_unit) {
            self.status_message = message.to_owned();
            return;
        }
        if candidate.validate().is_err() {
            self.status_message =
                "That tempo and time signature would not form a valid project".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message =
                "Cannot change the tempo: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Tempo and Signature");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Tempo {:.3} BPM, {}/{}; Undo restores the previous values",
            draft.bpm, draft.beats_per_bar, draft.beat_unit
        );
    }

    pub(super) fn show_tempo_editor(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.tempo_editor.take() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let recording = self.is_recording();
        let response = egui::Window::new("Tempo Track")
            .id(Id::new("tempo-track-editor"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(340.0);
                ui.add_enabled_ui(!recording, |ui| {
                    egui::Grid::new("tempo-track-grid")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Tempo");
                            ui.add(
                                egui::DragValue::new(&mut draft.bpm)
                                    .range(
                                        opentrack_core::project::MIN_TEMPO_BPM
                                            ..=opentrack_core::project::MAX_TEMPO_BPM,
                                    )
                                    .speed(0.1)
                                    .suffix(" BPM"),
                            );
                            ui.end_row();
                            ui.label("Beats per bar");
                            ui.add(
                                egui::DragValue::new(&mut draft.beats_per_bar).range(
                                    opentrack_core::project::MIN_BEATS_PER_BAR
                                        ..=opentrack_core::project::MAX_BEATS_PER_BAR,
                                ),
                            );
                            ui.end_row();
                            ui.label("Beat unit");
                            egui::ComboBox::from_id_salt("tempo-track-beat-unit")
                                .selected_text(draft.beat_unit.to_string())
                                .show_ui(ui, |ui| {
                                    for unit in opentrack_core::project::BEAT_UNITS {
                                        ui.selectable_value(
                                            &mut draft.beat_unit,
                                            unit,
                                            unit.to_string(),
                                        );
                                    }
                                });
                            ui.end_row();
                        });
                });
                if recording {
                    ui.label("Stop recording to change the tempo.");
                }
                ui.label(
                    egui::RichText::new(format!(
                        "One tempo and one signature describe the project. Metronome accents are application settings, not project content; edit them in Transport > Metronome Setup. The current accent mask is 0x{:X}.",
                        self.metronome_accent_mask,
                    ))
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!recording, egui::Button::new("Apply"))
                        .clicked()
                        || (!recording && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("tempo-track-editor"),
            &mut open,
            response.as_ref(),
        );
        if apply {
            self.apply_tempo_editor(draft);
            return;
        }
        if cancel || !open || !self.project_open {
            return;
        }
        self.tempo_editor = Some(draft);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app
    }

    #[test]
    fn the_editor_seeds_and_applies_tempo_and_signature_as_one_undo_step() {
        let mut app = app();
        assert_eq!(
            Draft::from_project(&app.data.project),
            Draft {
                bpm: 120.0,
                beats_per_bar: 4,
                beat_unit: 4
            }
        );
        app.open_tempo_editor();
        assert!(app.tempo_editor.is_some());

        app.apply_tempo_editor(Draft {
            bpm: 96.5,
            beats_per_bar: 7,
            beat_unit: 8,
        });
        assert_eq!(app.data.project.tempo_bpm, 96.5);
        assert_eq!(
            (app.data.project.beats_per_bar, app.data.project.beat_unit),
            (7, 8)
        );
        assert!(app.status_message.contains("Tempo 96.500 BPM, 7/8"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Tempo and Signature");

        // A no-op writes no history.
        let steps = app.project_history.counts().0;
        app.apply_tempo_editor(Draft {
            bpm: 96.5,
            beats_per_bar: 7,
            beat_unit: 8,
        });
        assert!(app.status_message.contains("unchanged"));
        assert_eq!(app.project_history.counts().0, steps);

        // Invalid values are refused and leave the project alone.
        let before = app.data.project.clone();
        app.apply_tempo_editor(Draft {
            bpm: 0.0,
            beats_per_bar: 7,
            beat_unit: 8,
        });
        assert!(app.status_message.contains("tempo must be between"));
        app.apply_tempo_editor(Draft {
            bpm: 120.0,
            beats_per_bar: 0,
            beat_unit: 8,
        });
        assert!(app.status_message.contains("beats per bar"));
        app.apply_tempo_editor(Draft {
            bpm: 120.0,
            beats_per_bar: 4,
            beat_unit: 5,
        });
        assert!(app.status_message.contains("beat unit"));
        assert_eq!(app.data.project, before);
        assert_eq!(app.project_history.counts().0, steps);
    }

    #[test]
    fn the_tempo_window_renders_its_controls() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.tempo_editor = Some(Draft::from_project(&app.data.project));
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_tempo_editor(&ctx),
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
        assert!(joined.contains("Tempo Track"), "{joined}");
        assert!(joined.contains("Beats per bar"), "{joined}");
        assert!(joined.contains("Beat unit"), "{joined}");
        assert!(joined.contains("application settings"), "{joined}");
    }
}
