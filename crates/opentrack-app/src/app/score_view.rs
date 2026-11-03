//! Read-only score view for the selected MIDI part.
//!
//! This is a first, bounded score surface: a grand staff with the part's notes
//! placed by pitch and length from the project's time signature. It does not
//! edit, lay out or engrave, and the window says so.

use super::*;
use opentrack_core::project::MidiPart;

/// Pitches at or above middle C read on the treble staff; lower ones on bass.
pub(super) const STAFF_SPLIT_PITCH: u8 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Staff {
    Treble,
    Bass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NoteHead {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    Shorter,
}

pub(super) fn staff_for_pitch(pitch: u8) -> Staff {
    if pitch >= STAFF_SPLIT_PITCH {
        Staff::Treble
    } else {
        Staff::Bass
    }
}

/// Diatonic index (C0 = 0, D0 = 1, …) of a MIDI pitch.
fn diatonic_index(pitch: u8) -> i32 {
    let octave = i32::from(pitch / 12) - 1;
    let step = match pitch % 12 {
        0 | 1 => 0,
        2 | 3 => 1,
        4 => 2,
        5 | 6 => 3,
        7 | 8 => 4,
        9 | 10 => 5,
        _ => 6,
    };
    octave * 7 + step
}

/// Steps above the staff's bottom line; zero is the bottom line.
pub(super) fn staff_position(pitch: u8, staff: Staff) -> i32 {
    match staff {
        // Treble bottom line is E4.
        Staff::Treble => diatonic_index(pitch) - (4 * 7 + 2),
        // Bass bottom line is G2.
        Staff::Bass => diatonic_index(pitch) - (2 * 7 + 4),
    }
}

pub(super) fn pitch_label(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!(
        "{}{}",
        NAMES[(pitch % 12) as usize],
        i32::from(pitch / 12) - 1
    )
}

pub(super) fn is_accidental(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

/// Quarter-note beats in one bar for a time signature.
pub(super) fn bar_beats(beats_per_bar: u8, beat_unit: u8) -> f64 {
    (f64::from(beats_per_bar) * 4.0 / f64::from(beat_unit.max(1))).max(0.25)
}

/// Note shape for a length in quarter-note beats.
pub(super) fn note_head(duration_beats: f64) -> NoteHead {
    if duration_beats >= 3.5 {
        NoteHead::Whole
    } else if duration_beats >= 1.75 {
        NoteHead::Half
    } else if duration_beats >= 0.75 {
        NoteHead::Quarter
    } else if duration_beats >= 0.375 {
        NoteHead::Eighth
    } else if duration_beats >= 0.1875 {
        NoteHead::Sixteenth
    } else {
        NoteHead::Shorter
    }
}

pub(super) fn note_flags(head: NoteHead) -> u8 {
    match head {
        NoteHead::Eighth => 1,
        NoteHead::Sixteenth => 2,
        NoteHead::Shorter => 3,
        NoteHead::Whole | NoteHead::Half | NoteHead::Quarter => 0,
    }
}

impl OpenTrackApp {
    fn selected_midi_part(&self) -> Option<MidiPart> {
        let id = self.data.project.selected_midi_part_id?;
        self.data
            .project
            .tracks
            .iter()
            .flat_map(|track| track.midi_parts.iter())
            .find(|part| part.id == id)
            .cloned()
    }

    /// Opens the read-only score for the selected MIDI part.
    pub(super) fn open_score_view(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to see its score".to_owned();
            return;
        }
        if self.selected_midi_part().is_none() {
            self.status_message = "Select a MIDI part to open the score".to_owned();
            return;
        }
        self.score_view_open = true;
        self.status_message =
            "Score view is read-only; engraving and editing are not implemented".to_owned();
    }

    pub(super) fn show_score_view(&mut self, ctx: &egui::Context) {
        if !self.score_view_open {
            return;
        }
        let Some(part) = self.selected_midi_part() else {
            self.score_view_open = false;
            return;
        };
        let bar = bar_beats(self.data.project.beats_per_bar, self.data.project.beat_unit);
        let total = part.duration_beats.max(bar);
        let bars = (total / bar).ceil().max(1.0) as usize;
        let px_per_beat: f32 = 26.0;
        let mut open = true;
        let response = egui::Window::new("Score")
            .id(Id::new("score-view"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(760.0, 360.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "{} · {}/{} · {bars} bar(s) · {} note(s)",
                    part.name,
                    self.data.project.beats_per_bar,
                    self.data.project.beat_unit,
                    part.notes.len()
                ));
                ui.label(
                    egui::RichText::new(
                        "Read-only: notes are placed by pitch and length from the time signature. Engraving, layout and editing are not implemented.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                let width = 80.0 + bars as f32 * bar as f32 * px_per_beat + 24.0;
                egui::ScrollArea::horizontal()
                    .id_salt("score-scroll")
                    .show(ui, |ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(width.max(560.0), 300.0),
                            Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);
                        let colors = theme::palette(ui);
                        let spacing = 10.0_f32;
                        let treble_top = rect.top() + 40.0;
                        let bass_top = treble_top + 150.0;
                        for (label, top) in [("Treble", treble_top), ("Bass", bass_top)] {
                            for line in 0..5 {
                                let y = top + line as f32 * spacing;
                                painter.line_segment(
                                    [Pos2::new(rect.left() + 56.0, y), Pos2::new(rect.right() - 8.0, y)],
                                    Stroke::new(1.0, colors.grid),
                                );
                            }
                            painter.text(
                                Pos2::new(rect.left() + 8.0, top + 2.0 * spacing),
                                Align2::LEFT_CENTER,
                                label,
                                FontId::proportional(12.0),
                                colors.muted,
                            );
                        }
                        // Bar lines across both staves.
                        for bar_index in 1..=bars {
                            let x = rect.left()
                                + 56.0
                                + bar_index as f32 * bar as f32 * px_per_beat;
                            if x > rect.right() - 8.0 {
                                break;
                            }
                            painter.line_segment(
                                [Pos2::new(x, treble_top), Pos2::new(x, bass_top + 4.0 * spacing)],
                                Stroke::new(1.0, colors.grid),
                            );
                        }
                        for note in &part.notes {
                            let staff = staff_for_pitch(note.pitch);
                            let position = staff_position(note.pitch, staff);
                            let bottom = if staff == Staff::Treble {
                                treble_top + 4.0 * spacing
                            } else {
                                bass_top + 4.0 * spacing
                            };
                            let y = bottom - position as f32 * spacing * 0.5;
                            let x = rect.left()
                                + 56.0
                                + (note.start_beats.max(0.0) as f32) * px_per_beat;
                            if x > rect.right() - 12.0 {
                                continue;
                            }
                            // Ledger lines for notes outside the staff.
                            if position > 8 {
                                let mut ledger = 10;
                                while ledger <= position {
                                    let ly = bottom - ledger as f32 * spacing * 0.5;
                                    painter.line_segment(
                                        [Pos2::new(x - 7.0, ly), Pos2::new(x + 7.0, ly)],
                                        Stroke::new(1.0, colors.grid),
                                    );
                                    ledger += 2;
                                }
                            } else if position < 0 {
                                let mut ledger = -2;
                                while ledger >= position {
                                    let ly = bottom - ledger as f32 * spacing * 0.5;
                                    painter.line_segment(
                                        [Pos2::new(x - 7.0, ly), Pos2::new(x + 7.0, ly)],
                                        Stroke::new(1.0, colors.grid),
                                    );
                                    ledger -= 2;
                                }
                            }
                            let head = note_head(note.duration_beats);
                            let radius = Vec2::new(5.0, 3.6);
                            let center = Pos2::new(x, y);
                            let filled = matches!(
                                head,
                                NoteHead::Quarter | NoteHead::Eighth | NoteHead::Sixteenth | NoteHead::Shorter
                            );
                            if filled {
                                painter.add(egui::Shape::ellipse_filled(
                                    center,
                                    radius,
                                    colors.text,
                                ));
                            } else {
                                painter.add(egui::Shape::ellipse_stroke(
                                    center,
                                    radius,
                                    Stroke::new(1.4, colors.text),
                                ));
                            }
                            if !matches!(head, NoteHead::Whole) {
                                let up = position < 4;
                                let stem_end = if up {
                                    Pos2::new(x + 4.5, y - 26.0)
                                } else {
                                    Pos2::new(x + 4.5, y + 26.0)
                                };
                                painter.line_segment(
                                    [Pos2::new(x + 4.5, y), stem_end],
                                    Stroke::new(1.2, colors.text),
                                );
                                for flag in 0..note_flags(head) {
                                    let offset = 4.0 + flag as f32 * 4.0;
                                    let direction = if up { 1.0 } else { -1.0 };
                                    painter.line_segment(
                                        [
                                            stem_end,
                                            Pos2::new(stem_end.x + 7.0, stem_end.y + direction * offset),
                                        ],
                                        Stroke::new(1.2, colors.text),
                                    );
                                }
                            }
                            if is_accidental(note.pitch) {
                                painter.text(
                                    Pos2::new(x - 10.0, y),
                                    Align2::CENTER_CENTER,
                                    "#",
                                    FontId::proportional(12.0),
                                    colors.text,
                                );
                            }
                        }
                    });
            });
        dismiss_window_on_outside_click(ctx, Id::new("score-view"), &mut open, response.as_ref());
        self.score_view_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_signatures_and_note_shapes_are_mapped() {
        assert!((bar_beats(4, 4) - 4.0).abs() < 1e-9);
        assert!((bar_beats(6, 8) - 3.0).abs() < 1e-9);
        assert!((bar_beats(3, 4) - 3.0).abs() < 1e-9);
        assert!((bar_beats(2, 2) - 4.0).abs() < 1e-9);

        assert_eq!(note_head(4.0), NoteHead::Whole);
        assert_eq!(note_head(2.0), NoteHead::Half);
        assert_eq!(note_head(1.0), NoteHead::Quarter);
        assert_eq!(note_head(0.5), NoteHead::Eighth);
        assert_eq!(note_head(0.25), NoteHead::Sixteenth);
        assert_eq!(note_head(0.1), NoteHead::Shorter);
        assert_eq!(note_flags(NoteHead::Eighth), 1);
        assert_eq!(note_flags(NoteHead::Sixteenth), 2);
        assert_eq!(note_flags(NoteHead::Half), 0);
    }

    #[test]
    fn pitches_map_to_a_staff_line_or_space() {
        // Middle C splits the staves.
        assert_eq!(staff_for_pitch(59), Staff::Bass);
        assert_eq!(staff_for_pitch(60), Staff::Treble);
        // Treble bottom line is E4; D4 is the space below, C4 the ledger below.
        assert_eq!(staff_position(64, Staff::Treble), 0);
        assert_eq!(staff_position(62, Staff::Treble), -1);
        assert_eq!(staff_position(60, Staff::Treble), -2);
        assert_eq!(staff_position(72, Staff::Treble), 5);
        // Bass bottom line is G2.
        assert_eq!(staff_position(43, Staff::Bass), 0);
        assert_eq!(staff_position(45, Staff::Bass), 1);
        assert_eq!(staff_position(60, Staff::Bass), 10);

        assert_eq!(pitch_label(60), "C4");
        assert_eq!(pitch_label(61), "C#4");
        assert_eq!(pitch_label(43), "G2");
        assert!(is_accidental(61) && !is_accidental(60));
    }

    fn app_with_part() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Midi);
        let part = app.data.project.add_midi_part(0, 0.0, 8.0).expect("part");
        for (beat, pitch, duration) in [(0.0, 60, 1.0), (1.0, 64, 0.5), (2.0, 43, 2.0)] {
            app.data
                .project
                .add_midi_note(0, part, beat, duration, pitch, 100)
                .expect("note");
        }
        app.data.project.selected_midi_part_id = Some(part);
        app
    }

    #[test]
    fn the_score_opens_only_with_a_selected_part() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut empty = OpenTrackApp::new_internal(&creation, false);
        empty.project_open = true;
        empty.open_score_view();
        assert!(!empty.score_view_open);
        assert!(empty.status_message.contains("Select a MIDI part"));

        let mut app = app_with_part();
        app.open_score_view();
        assert!(app.score_view_open);
        assert!(app.status_message.contains("read-only"));
    }

    #[test]
    fn the_window_draws_the_staves_and_note_count() {
        let ctx = egui::Context::default();
        let mut app = app_with_part();
        app.score_view_open = true;
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 800.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_score_view(&ctx),
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
        assert!(joined.contains("Score"), "{joined}");
        assert!(joined.contains("Treble"), "{joined}");
        assert!(joined.contains("Bass"), "{joined}");
        assert!(joined.contains("3 note(s)"), "{joined}");
        assert!(joined.contains("Read-only"), "{joined}");
    }
}
