//! One visible/editable MIDI controller lane.
//!
//! This is a view configuration, not automation content. The lane number is
//! persisted with the project's workspace view state; choosing a lane never
//! creates controller events, never touches the project history and never dirties
//! project content. Editing the lane reuses the existing core controller-event
//! commands, which remain undoable.
//!
//! Deliberately not VST parameter automation and not a multi-lane editor: the
//! user-approved scope is one selected CC lane at a time.

use super::*;

/// Named controllers shown in the setup dialog and lane selector. Everything
/// else still uses its number, because all CC 0-127 are valid lane targets.
pub(super) const NAMED_CONTROLLERS: [(u8, &str); 25] = [
    (0, "Bank Select"),
    (1, "Modulation"),
    (2, "Breath"),
    (4, "Foot Controller"),
    (5, "Portamento Time"),
    (7, "Volume"),
    (8, "Balance"),
    (10, "Pan"),
    (11, "Expression"),
    (64, "Sustain Pedal"),
    (65, "Portamento"),
    (66, "Sostenuto"),
    (67, "Soft Pedal"),
    (71, "Resonance"),
    (72, "Release Time"),
    (73, "Attack Time"),
    (74, "Brightness"),
    (75, "Decay Time"),
    (84, "Portamento Control"),
    (91, "Reverb Send"),
    (93, "Chorus Send"),
    (95, "Phaser Send"),
    (120, "All Sound Off"),
    (121, "Reset All Controllers"),
    (123, "All Notes Off"),
];

/// A readable lane label for any valid controller number.
pub(super) fn controller_label(controller: u8) -> String {
    NAMED_CONTROLLERS
        .iter()
        .find(|(number, _)| *number == controller)
        .map_or_else(
            || format!("Controller (CC {controller})"),
            |(number, name)| format!("{name} (CC {number})"),
        )
}

pub(super) fn default_value(controller: u8) -> u8 {
    match controller {
        7 | 11 => 127,
        10 => 64,
        _ => 0,
    }
}

/// The selected lane's points, ordered for drawing and hit testing. Other
/// controllers stay in the project untouched and reappear when selected.
pub(super) fn lane_events(
    part: &opentrack_core::project::MidiPart,
    controller: u8,
) -> Vec<(u64, f64, u8)> {
    let mut events: Vec<_> = part
        .controller_events
        .iter()
        .filter(|event| event.controller == controller)
        .map(|event| (event.id, event.position_beats, event.value))
        .collect();
    events.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    events
}

/// Draft for the setup dialog. Apply publishes the lane number to the project
/// view state; Cancel leaves both the view and content unchanged.
pub(super) struct CcAutomationSetup {
    pub controller: u8,
}

impl Default for CcAutomationSetup {
    fn default() -> Self {
        Self { controller: 11 }
    }
}

impl OpenTrackApp {
    pub(super) fn open_cc_automation_setup(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to configure CC automation lanes".to_owned();
            return;
        }
        self.cc_automation_setup = Some(CcAutomationSetup {
            controller: self.arrange_view_state.cc_lane_controller.min(127),
        });
    }

    /// Publish the lane choice. This is view state: no history entry, no content
    /// change, no dirty project content.
    pub(super) fn apply_cc_lane(&mut self, controller: u8) {
        if controller > 127 {
            self.status_message = "The MIDI controller must be 0 through 127".to_owned();
            return;
        }
        self.arrange_view_state.cc_lane_controller = controller;
        self.status_message = format!(
            "CC lane set to {}; this view choice is saved with the project",
            controller_label(controller)
        );
    }

    pub(super) fn show_cc_automation_setup(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.cc_automation_setup.take() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let response = egui::Window::new("CC Automation Setup")
            .id(Id::new("cc-automation-setup-dialog"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(360.0);
                ui.label("Visible controller lane");
                ui.horizontal(|ui| {
                    let mut number = i32::from(dialog.controller);
                    if ui
                        .add(egui::DragValue::new(&mut number).range(0..=127))
                        .changed()
                    {
                        dialog.controller = number.clamp(0, 127) as u8;
                    }
                    ui.label(controller_label(dialog.controller));
                });
                ui.separator();
                ui.label("Common controllers");
                egui::ScrollArea::vertical().max_height(210.0).show(ui, |ui| {
                    for (number, _) in NAMED_CONTROLLERS {
                        ui.selectable_value(
                            &mut dialog.controller,
                            number,
                            controller_label(number),
                        );
                    }
                });
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "One controller lane is visible and editable at a time. Choosing a lane shows the existing CC data for that controller; it does not create controller events. This view choice is saved with the project and does not create an undo step. Editing CC points remains undoable.",
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
            Id::new("cc-automation-setup-dialog"),
            &mut open,
            response.as_ref(),
        );
        if apply {
            self.apply_cc_lane(dialog.controller);
        } else if !cancel && open {
            self.cc_automation_setup = Some(dialog);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_cc() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Midi);
        let part = app.data.project.add_midi_part(0, 0.0, 8.0).expect("part");
        app.data.project.selected_midi_part_id = Some(part);
        for (position, controller, value) in [(1.0, 11, 90), (2.0, 74, 40), (3.0, 74, 110)] {
            app.data
                .project
                .add_midi_controller_event(0, part, position, controller, value)
                .expect("cc");
        }
        app.project_history.clear();
        app
    }

    #[test]
    fn selecting_a_lane_is_view_only_and_keeps_other_controller_data() {
        let mut app = app_with_cc();
        let content_before = app.data.project.clone();
        let history_before = app.project_history.counts();
        let dirty_before = app.modified;

        app.apply_cc_lane(74);

        assert_eq!(app.arrange_view_state.cc_lane_controller, 74);
        assert_eq!(app.data.project, content_before);
        assert_eq!(app.project_history.counts(), history_before);
        assert_eq!(app.modified, dirty_before);
        assert!(app.status_message.contains("CC 74"));

        let part = &app.data.project.tracks[0].midi_parts[0];
        let events = lane_events(part, 74);
        assert_eq!(
            events
                .iter()
                .map(|(_, position, value)| (*position, *value))
                .collect::<Vec<_>>(),
            vec![(2.0, 40), (3.0, 110)]
        );
        assert_eq!(part.controller_events.len(), 3, "other CC data is kept");
        assert_eq!(lane_events(part, 11).len(), 1);
        assert!(lane_events(part, 1).is_empty(), "no invented lane data");
    }

    #[test]
    fn a_cc_content_edit_is_one_undoable_step() {
        let mut app = app_with_cc();
        app.apply_cc_lane(74);
        app.project_history.begin(&app.data.project, app.modified);

        let part = &mut app.data.project.tracks[0].midi_parts[0];
        let event = part
            .controller_events
            .iter_mut()
            .find(|event| event.controller == 74 && event.value == 40)
            .expect("cc 74");
        event.value = 100;
        app.project_history.label_next("Edit MIDI Controller");
        assert!(app.project_history.finish_frame(&app.data.project, true));

        assert_eq!(app.project_history.counts().0, 1);
        app.undo_project();
        let restored = &app.data.project.tracks[0].midi_parts[0];
        assert_eq!(
            restored
                .controller_events
                .iter()
                .find(|event| event.controller == 74 && event.position_beats == 2.0)
                .expect("cc 74")
                .value,
            40
        );
    }

    #[test]
    fn labels_cover_named_and_unnamed_controllers() {
        assert_eq!(controller_label(11), "Expression (CC 11)");
        assert_eq!(controller_label(74), "Brightness (CC 74)");
        assert_eq!(controller_label(42), "Controller (CC 42)");
        assert!((0..=127).all(|controller| !controller_label(controller).is_empty()));
    }
}
