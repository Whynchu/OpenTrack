//! Focused drum editor over the existing MIDI part/note model.
//!
//! Rows come from the project drum map, followed by any sounding pitches that
//! are not mapped, so notes never disappear from the editor. Pitch stays the
//! note's real MIDI pitch; the map only names rows. Editing uses the same
//! selected part and note-ID selection as the Key and List editors.

use super::*;

fn drum_editor_rows(
    project: &opentrack_core::project::Project,
    part: &opentrack_core::project::MidiPart,
) -> Vec<u8> {
    let mut rows: Vec<u8> = project
        .drum_map
        .rows()
        .iter()
        .map(|row| row.pitch)
        .collect();
    let mut unmapped: Vec<u8> = part
        .notes
        .iter()
        .map(|note| note.pitch)
        .filter(|pitch| !rows.contains(pitch))
        .collect();
    unmapped.sort_unstable();
    unmapped.dedup();
    rows.extend(unmapped);
    rows
}

pub(super) struct DrumEditorDrag {
    start_pointer_beat: f64,
    origin: Vec<(u64, f64)>,
}

impl OpenTrackApp {
    pub(super) fn toggle_drum_editor(&mut self) {
        if self.drum_editor_open {
            self.drum_editor_open = false;
            self.drum_editor_drag = None;
            return;
        }
        if self
            .data
            .project
            .selected_midi_part_id
            .and_then(|id| {
                self.data
                    .project
                    .tracks
                    .iter()
                    .flat_map(|track| track.midi_parts.iter())
                    .find(|part| part.id == id)
            })
            .is_none()
        {
            self.status_message = "Select a MIDI part to open the drum editor".to_owned();
            return;
        }
        self.drum_editor_open = true;
        self.drum_editor_velocity = self
            .selected_midi_note_ids
            .iter()
            .next()
            .and_then(|id| self.selected_part_note(*id))
            .map(|note| note.velocity)
            .unwrap_or(self.midi_editor_settings.insert_velocity);
    }

    fn selected_part_note(&self, note_id: u64) -> Option<&opentrack_core::project::MidiNote> {
        let part_id = self.data.project.selected_midi_part_id?;
        self.data
            .project
            .tracks
            .iter()
            .flat_map(|track| track.midi_parts.iter())
            .find(|part| part.id == part_id)?
            .notes
            .iter()
            .find(|note| note.id == note_id)
    }

    fn drum_editor_apply<F>(&mut self, label: &str, mutate: F) -> bool
    where
        F: FnOnce(&mut opentrack_core::project::MidiPart) -> bool,
    {
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
            self.status_message = "No note changed".to_owned();
            return false;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot edit notes: undo snapshot is unavailable".to_owned();
            return false;
        }
        self.project_history.label_next(label);
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        true
    }

    fn drum_editor_add_note(&mut self, pitch: u8, beat: f64) {
        let Some(part_id) = self.data.project.selected_midi_part_id else {
            return;
        };
        let Some((track_index, part_duration)) = self
            .data
            .project
            .tracks
            .iter()
            .enumerate()
            .find_map(|(index, track)| {
                track
                    .midi_parts
                    .iter()
                    .find(|part| part.id == part_id)
                    .map(|part| (index, part.duration_beats))
            })
        else {
            return;
        };
        let grid = super::midi_editor_settings::position_quantize(
            self.midi_editor_settings.position_grid_beats,
            self.midi_editor_settings.snap_enabled,
        );
        let length = self.midi_editor_settings.length_grid_beats.clamp(grid, 4.0);
        let start = (beat / grid).round() * grid;
        if start < 0.0 || start + length > part_duration + 1e-9 {
            self.status_message = "The note would fall outside the part".to_owned();
            return;
        }
        let velocity = self.midi_editor_settings.insert_velocity.clamp(1, 127);
        let mut candidate = self.data.project.clone();
        let Some(note_id) = candidate.add_midi_note_on_channel(
            track_index,
            part_id,
            start,
            length,
            pitch,
            0,
            velocity,
        ) else {
            self.status_message = "Could not add the note".to_owned();
            return;
        };
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot add note: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Add Drum Note");
        self.data.project = candidate;
        self.selected_midi_note_ids.clear();
        self.selected_midi_note_ids.insert(note_id);
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!("Added MIDI note {pitch} at beat {start:.3}");
    }

    fn drum_editor_delete_selected(&mut self) {
        let ids: std::collections::BTreeSet<u64> = self.selected_midi_note_ids.clone();
        if ids.is_empty() {
            self.status_message = "Select drum notes to delete".to_owned();
            return;
        }
        if self.drum_editor_apply("Delete Drum Notes", |part| {
            let before = part.notes.len();
            part.notes.retain(|note| !ids.contains(&note.id));
            part.notes.len() != before
        }) {
            self.selected_midi_note_ids.clear();
            self.status_message = format!("Deleted {} drum note(s)", ids.len());
        }
    }

    fn drum_editor_set_velocity(&mut self, velocity: u8) {
        let ids = self.selected_midi_note_ids.clone();
        if ids.is_empty() {
            return;
        }
        if self.drum_editor_apply("Drum Note Velocity", |part| {
            let mut changed = false;
            for note in &mut part.notes {
                if ids.contains(&note.id) && note.velocity != velocity {
                    note.velocity = velocity;
                    changed = true;
                }
            }
            changed
        }) {
            self.status_message = format!("Set {} drum note(s) to velocity {velocity}", ids.len());
        }
    }

    fn drum_editor_move_selected(&mut self, delta_beats: f64) {
        let ids = self.selected_midi_note_ids.clone();
        if ids.is_empty() {
            return;
        }
        if self.drum_editor_apply("Move Drum Notes", |part| {
            let mut changed = false;
            for note in &mut part.notes {
                if ids.contains(&note.id) {
                    let maximum = (part.duration_beats - note.duration_beats).max(0.0);
                    let next = (note.start_beats + delta_beats).clamp(0.0, maximum);
                    if (next - note.start_beats).abs() > 1e-9 {
                        note.start_beats = next;
                        changed = true;
                    }
                }
            }
            changed
        }) {
            self.status_message = format!("Moved drum note(s) by {delta_beats:+.3} beats");
        }
    }

    pub(super) fn show_drum_editor(&mut self, ctx: &egui::Context) {
        if !self.drum_editor_open {
            return;
        }
        let Some(part_id) = self.data.project.selected_midi_part_id else {
            self.drum_editor_open = false;
            return;
        };
        let Some(part) = self
            .data
            .project
            .tracks
            .iter()
            .flat_map(|track| track.midi_parts.iter())
            .find(|part| part.id == part_id)
            .cloned()
        else {
            self.drum_editor_open = false;
            return;
        };

        let mut rows: Vec<u8> = drum_editor_rows(&self.data.project, &part);
        if rows.is_empty() {
            rows.push(36);
        }

        let mut open = self.drum_editor_open;
        let mut delete = false;
        let mut velocity = self.drum_editor_velocity;
        let mut apply_velocity = false;
        let mut close = false;
        let response = egui::Window::new("Drum Editor")
            .id(Id::new("drum-editor-window"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_width(720.0)
            .default_height(460.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "{} · {} mapped row(s) · {} note(s)",
                    part.name,
                    self.data.project.drum_map.rows().len(),
                    part.notes.len()
                ));
                ui.label(
                    egui::RichText::new(
                        "Click an empty cell to add, click a note to select (Ctrl adds), drag to move, Escape cancels a drag. Pitch is playback pitch; the map only names rows.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.horizontal(|ui| {
                    if ui.button("Delete Selected").clicked() {
                        delete = true;
                    }
                    ui.label("Velocity");
                    ui.add(egui::DragValue::new(&mut velocity).range(1..=127));
                    if ui.button("Set Velocity").clicked() {
                        apply_velocity = true;
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
                ui.separator();
                let row_height = 18.0;
                let width = ui.available_width().max(320.0);
                let height = (rows.len() as f32 * row_height).max(80.0);
                let (canvas, response) =
                    ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
                let painter = ui.painter_at(canvas);
                let header_width = 100.0;
                let grid_rect = Rect::from_min_max(
                    Pos2::new(canvas.left() + header_width, canvas.top()),
                    canvas.max,
                );
                let beat_width = grid_rect.width() / part.duration_beats.max(1e-6) as f32;
                for (index, pitch) in rows.iter().enumerate() {
                    let top = grid_rect.top() + index as f32 * row_height;
                    let row_rect =
                        Rect::from_min_size(Pos2::new(canvas.left(), top), Vec2::new(canvas.width(), row_height));
                    if index % 2 == 0 {
                        painter.rect_filled(
                            row_rect,
                            0.0,
                            theme::palette(ui).canvas_alternate,
                        );
                    }
                    let name = self
                        .data
                        .project
                        .drum_map
                        .name_for(*pitch)
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("Note {pitch}"));
                    painter.text(
                        Pos2::new(canvas.left() + 4.0, top + row_height / 2.0),
                        Align2::LEFT_CENTER,
                        format!("{name}  ({pitch})"),
                        FontId::proportional(10.0),
                        theme::palette(ui).text,
                    );
                }
                let grid_step = super::midi_editor_settings::position_quantize(
                    self.midi_editor_settings.position_grid_beats,
                    self.midi_editor_settings.snap_enabled,
                );
                let mut beat = 0.0;
                while beat <= part.duration_beats + 1e-9 {
                    let x = grid_rect.left() + (beat / part.duration_beats.max(1e-6)) as f32 * grid_rect.width();
                    painter.line_segment(
                        [Pos2::new(x, grid_rect.top()), Pos2::new(x, grid_rect.bottom())],
                        Stroke::new(1.0, theme::palette(ui).grid),
                    );
                    painter.text(
                        Pos2::new(x + 2.0, canvas.top() + 2.0),
                        Align2::LEFT_TOP,
                        format!("{beat:.2}"),
                        FontId::proportional(9.0),
                        theme::palette(ui).muted,
                    );
                    beat += grid_step.max(0.125);
                }
                let selected_color = theme::palette(ui).accent;
                let note_color = theme::palette(ui).text;
                for note in &part.notes {
                    let Some(index) = rows.iter().position(|pitch| *pitch == note.pitch) else {
                        continue;
                    };
                    let x = grid_rect.left() + (note.start_beats / part.duration_beats.max(1e-6)) as f32 * grid_rect.width();
                    let w = (note.duration_beats / part.duration_beats.max(1e-6) as f64) as f32 * grid_rect.width();
                    let rect = Rect::from_min_size(
                        Pos2::new(x, grid_rect.top() + index as f32 * row_height + 1.0),
                        Vec2::new(w.max(2.0), row_height - 2.0),
                    );
                    let selected = self.selected_midi_note_ids.contains(&note.id);
                    painter.rect_filled(
                        rect,
                        1.0,
                        if selected { selected_color } else { note_color },
                    );
                }
                if response.clicked() || response.drag_started() {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let index = ((pointer.y - grid_rect.top()) / row_height).floor();
                        let pitch = usize::try_from(index as i64)
                            .ok()
                            .and_then(|index| rows.get(index).copied());
                        if let Some(pitch) = pitch {
                            let beat = ((pointer.x - grid_rect.left()) / beat_width) as f64;
                            let hit = part.notes.iter().find(|note| {
                                note.pitch == pitch
                                    && beat >= note.start_beats
                                    && beat <= note.start_beats + note.duration_beats
                            });
                            if let Some(note) = hit {
                                if !ui.input(|input| input.modifiers.command)
                                    && !self.selected_midi_note_ids.contains(&note.id)
                                {
                                    self.selected_midi_note_ids.clear();
                                }
                                if !self.selected_midi_note_ids.insert(note.id)
                                    && ui.input(|input| input.modifiers.command)
                                {
                                    self.selected_midi_note_ids.remove(&note.id);
                                }
                                if response.drag_started() {
                                    let origin = part
                                        .notes
                                        .iter()
                                        .filter(|candidate| {
                                            self.selected_midi_note_ids.contains(&candidate.id)
                                        })
                                        .map(|candidate| (candidate.id, candidate.start_beats))
                                        .collect();
                                    self.drum_editor_drag = Some(DrumEditorDrag {
                                        start_pointer_beat: beat,
                                        origin,
                                    });
                                }
                            } else {
                                self.selected_midi_note_ids.clear();
                                if response.clicked() {
                                    self.drum_editor_add_note(pitch, beat);
                                }
                            }
                        }
                    }
                }
                if let Some(drag) = self.drum_editor_drag.as_ref() {
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        self.drum_editor_drag = None;
                        self.status_message = "Drum note move cancelled".to_owned();
                    } else if response.drag_stopped() {
                        let delta = response
                            .interact_pointer_pos()
                            .map(|pointer| {
                                let beat = ((pointer.x - grid_rect.left()) / beat_width) as f64;
                                let raw = beat - drag.start_pointer_beat;
                                (raw / grid_step).round() * grid_step
                            })
                            .unwrap_or(0.0);
                        let origin = drag.origin.clone();
                        self.drum_editor_drag = None;
                        if delta.abs() > 1e-9 {
                            if self.drum_editor_apply("Move Drum Notes", |part| {
                                let mut changed = false;
                                for note in &mut part.notes {
                                    if let Some((_, start)) =
                                        origin.iter().find(|(id, _)| *id == note.id)
                                    {
                                        let maximum =
                                            (part.duration_beats - note.duration_beats).max(0.0);
                                        let next = (start + delta).clamp(0.0, maximum);
                                        if (next - note.start_beats).abs() > 1e-9 {
                                            note.start_beats = next;
                                            changed = true;
                                        }
                                    }
                                }
                                changed
                            }) {
                                self.status_message =
                                    format!("Moved drum notes by {delta:+.3} beats");
                            }
                        }
                    }
                }
                if response.hovered()
                    && ui.input(|input| input.key_pressed(egui::Key::Delete))
                {
                    delete = true;
                }
            });
        self.drum_editor_open = open && !close;
        if !self.drum_editor_open {
            self.drum_editor_drag = None;
        }
        self.drum_editor_velocity = velocity;
        if apply_velocity {
            self.drum_editor_set_velocity(velocity);
        }
        if delete {
            self.drum_editor_delete_selected();
        }
        let _ = response;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_part() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Midi);
        let part = app.data.project.add_midi_part(0, 0.0, 8.0).expect("part");
        app.data.project.selected_midi_part_id = Some(part);
        app.data
            .project
            .add_midi_note(0, part, 1.0, 0.5, 60, 100)
            .expect("note");
        app.project_history.clear();
        app
    }

    #[test]
    fn rows_show_mapped_order_then_unmapped_sounding_pitches() {
        let mut app = app_with_part();
        app.data
            .project
            .drum_map
            .set_rows(&[(38, "Snare".to_owned())])
            .expect("map");
        let part = &app.data.project.tracks[0].midi_parts[0];
        assert_eq!(drum_editor_rows(&app.data.project, part), vec![38, 60]);
    }

    #[test]
    fn adding_moving_changing_velocity_and_deleting_are_labelled_steps() {
        let mut app = app_with_part();
        app.midi_editor_settings.snap_enabled = true;
        app.midi_editor_settings.position_grid_beats = 0.25;
        app.midi_editor_settings.length_grid_beats = 0.25;
        app.midi_editor_settings.insert_velocity = 100;
        app.drum_editor_open = true;
        app.drum_editor_add_note(36, 0.4);
        let note = app.data.project.tracks[0].midi_parts[0]
            .notes
            .iter()
            .find(|note| note.pitch == 36)
            .expect("added");
        assert_eq!(note.start_beats, 0.5);
        assert_eq!(note.duration_beats, 0.25);
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Add Drum Note");
        let id = note.id;
        app.selected_midi_note_ids.insert(id);
        app.drum_editor_move_selected(0.5);
        assert_eq!(
            app.data.project.tracks[0].midi_parts[0]
                .notes
                .iter()
                .find(|note| note.id == id)
                .unwrap()
                .start_beats,
            1.0
        );
        app.drum_editor_set_velocity(64);
        assert_eq!(
            app.data.project.tracks[0].midi_parts[0]
                .notes
                .iter()
                .find(|note| note.id == id)
                .unwrap()
                .velocity,
            64
        );
        app.drum_editor_delete_selected();
        assert!(
            !app.data.project.tracks[0].midi_parts[0]
                .notes
                .iter()
                .any(|note| note.id == id)
        );
    }

    #[test]
    fn closing_clears_the_drag_and_does_not_touch_the_project() {
        let mut app = app_with_part();
        app.drum_editor_open = true;
        app.drum_editor_drag = Some(DrumEditorDrag {
            start_pointer_beat: 1.0,
            origin: vec![(1, 1.0)],
        });
        app.toggle_drum_editor();
        assert!(!app.drum_editor_open);
        assert!(app.drum_editor_drag.is_none());
        assert_eq!(app.project_history.counts().0, 0);
    }
}
