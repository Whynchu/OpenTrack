//! Drum Map Setup dialog: edit the project's presentation-only drum names.
//!
//! The draft is validated on Apply. Apply is one labelled project-history step;
//! Cancel or closing the window changes nothing. The map never changes pitch.

use super::*;

pub(super) struct DrumMapDraft {
    pub rows: Vec<(u8, String)>,
    pub error: Option<String>,
}

impl DrumMapDraft {
    pub(super) fn from_project(project: &opentrack_core::project::Project) -> Self {
        Self {
            rows: project
                .drum_map
                .rows()
                .iter()
                .map(|row| (row.pitch, row.name.clone()))
                .collect(),
            error: None,
        }
    }

    fn first_free_pitch(&self) -> Option<u8> {
        let used: std::collections::BTreeSet<u8> =
            self.rows.iter().map(|(pitch, _)| *pitch).collect();
        (0..=127u8).find(|pitch| !used.contains(pitch))
    }
}

impl OpenTrackApp {
    pub(super) fn open_drum_map_setup(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to edit its drum map".to_owned();
            return;
        }
        self.drum_map_setup = Some(DrumMapDraft::from_project(&self.data.project));
    }

    pub(super) fn apply_drum_map_setup(&mut self, rows: &[(u8, String)]) {
        let mut candidate = self.data.project.clone();
        if let Err(error) = candidate.drum_map.set_rows(rows) {
            if let Some(draft) = self.drum_map_setup.as_mut() {
                draft.error = Some(error.clone());
            }
            self.status_message = error;
            return;
        }
        if candidate.drum_map == self.data.project.drum_map {
            self.status_message = "The drum map already matches that".to_owned();
            self.drum_map_setup = None;
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message =
                "Cannot change the drum map: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Drum Map Setup");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.drum_map_setup = None;
        self.status_message =
            "Drum map applied; it changes displayed names and order, not sounding pitch".to_owned();
    }

    pub(super) fn show_drum_map_setup(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.drum_map_setup.take() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let mut changed = false;
        let response = egui::Window::new("Drum Map Setup")
            .id(Id::new("drum-map-setup-dialog"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_width(430.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(
                    "Names and row order are a display mapping for MIDI and Instrument tracks. \
                     Playback pitch is unchanged; an empty map shows MIDI note numbers.",
                );
                egui::ComboBox::from_label("Load preset")
                    .selected_text("Choose a map")
                    .show_ui(ui, |ui| {
                        if ui.button("General MIDI 1 Percussion").clicked() {
                            draft.rows = opentrack_core::drum_map::GENERAL_MIDI_1_PERCUSSION
                                .iter()
                                .map(|(pitch, name)| (*pitch, (*name).to_owned()))
                                .collect();
                            draft.error = None;
                            changed = true;
                        }
                    });
                if let Some(error) = &draft.error {
                    ui.colored_label(Color32::from_rgb(214, 88, 80), error);
                }
                ui.separator();
                if draft.rows.is_empty() {
                    ui.label("No mapped rows: the editor shows MIDI note numbers.");
                }
                let mut remove = None;
                let mut move_up = None;
                let mut move_down = None;
                let row_count = draft.rows.len();
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (index, (pitch, name)) in draft.rows.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(pitch)
                                        .range(0..=127)
                                        .speed(1.0)
                                        .prefix("MIDI "),
                                );
                                changed |= ui
                                    .add(
                                        egui::TextEdit::singleline(name)
                                            .desired_width(190.0)
                                            .hint_text("Name"),
                                    )
                                    .changed();
                                if ui.add_enabled(index > 0, egui::Button::new("Up")).clicked() {
                                    move_up = Some(index);
                                }
                                if ui
                                    .add_enabled(index + 1 < row_count, egui::Button::new("Down"))
                                    .clicked()
                                {
                                    move_down = Some(index);
                                }
                                if ui.button("Remove").clicked() {
                                    remove = Some(index);
                                }
                            });
                        }
                    });
                if let Some(index) = move_up {
                    draft.rows.swap(index, index - 1);
                    changed = true;
                }
                if let Some(index) = move_down {
                    draft.rows.swap(index, index + 1);
                    changed = true;
                }
                if let Some(index) = remove {
                    draft.rows.remove(index);
                    changed = true;
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            draft.rows.len() < opentrack_core::drum_map::MAX_DRUM_MAP_ROWS,
                            egui::Button::new("Add Row"),
                        )
                        .clicked()
                    {
                        if let Some(pitch) = draft.first_free_pitch() {
                            draft.rows.push((pitch, format!("Note {pitch}")));
                            changed = true;
                        }
                    }
                    if ui.button("Reset to Note Numbers").clicked() {
                        draft.rows.clear();
                        changed = true;
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
                let _ = changed;
            });
        super::dismiss_window_on_outside_click(
            ctx,
            Id::new("drum-map-setup-dialog"),
            &mut open,
            response.as_ref(),
        );
        if apply {
            self.apply_drum_map_setup(&draft.rows);
        } else if !cancel && open {
            self.drum_map_setup = Some(draft);
        }
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
    fn applying_a_drum_map_is_one_labelled_undo_step() {
        let mut app = app();
        app.open_drum_map_setup();
        assert!(app.drum_map_setup.is_some());
        app.apply_drum_map_setup(&[(36, "Kick".to_owned()), (38, "Snare".to_owned())]);
        assert!(app.drum_map_setup.is_none());
        assert_eq!(app.data.project.drum_map.rows().len(), 2);
        assert_eq!(app.data.project.drum_map.name_for(36), Some("Kick"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Drum Map Setup");

        app.undo_project();
        assert!(app.data.project.drum_map.is_empty());
        app.redo_project();
        assert_eq!(app.data.project.drum_map.rows().len(), 2);
    }

    #[test]
    fn duplicate_pitches_are_refused_without_changing_the_project() {
        let mut app = app();
        app.open_drum_map_setup();
        let before = app.data.project.clone();
        let history = app.project_history.counts();
        app.apply_drum_map_setup(&[(36, "Kick".to_owned()), (36, "Snare".to_owned())]);
        assert_eq!(app.data.project, before);
        assert_eq!(app.project_history.counts(), history);
        let draft = app.drum_map_setup.as_ref().expect("draft stays open");
        assert!(
            draft
                .error
                .as_deref()
                .unwrap_or("")
                .contains("more than once")
        );
    }

    #[test]
    fn resetting_to_note_numbers_and_reapplying_is_a_content_change() {
        let mut app = app();
        app.apply_drum_map_setup(&[(36, "Kick".to_owned())]);
        let steps = app.project_history.counts().0;
        app.apply_drum_map_setup(&[(36, "Kick".to_owned())]);
        assert_eq!(app.project_history.counts().0, steps);
        assert!(app.status_message.contains("already matches"));
        app.apply_drum_map_setup(&[]);
        assert!(app.data.project.drum_map.is_empty());
        assert_eq!(app.project_history.counts().0, steps + 1);
    }

    #[test]
    fn switching_tracks_while_open_keeps_the_draft() {
        let mut app = app();
        app.data.project.add_track(TrackKind::Midi);
        app.data.project.add_track(TrackKind::Midi);
        app.data.project.selected_track = Some(0);
        app.open_drum_map_setup();
        let draft = app.drum_map_setup.as_mut().expect("draft");
        draft.rows.push((36, "Kick".to_owned()));
        app.data.project.selected_track = Some(1);
        assert_eq!(
            app.drum_map_setup.as_ref().expect("draft").rows,
            vec![(36, "Kick".to_owned())]
        );
        app.apply_drum_map_setup(&[(36, "Kick".to_owned())]);
        assert_eq!(app.data.project.drum_map.name_for(36), Some("Kick"));
    }
}
