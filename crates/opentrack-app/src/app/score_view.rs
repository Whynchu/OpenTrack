//! Score view for the selected MIDI part.
//!
//! A first, bounded score surface: a grand staff with the part's notes placed by
//! pitch and length from the project's time signature, direct selection and
//! keyboard/button editing (move, transpose, delete), a page layout and zoom.
//! Engraving, symbols and beaming are not implemented.

use super::*;
use opentrack_core::midi_transform::ScoreAlign;
use opentrack_core::project::MidiPart;

/// Pitches at or above middle C read on the treble staff; lower ones on bass.
pub(super) const STAFF_SPLIT_PITCH: u8 = 60;
/// Default bars per system in page mode, and its bounds.
pub(super) const DEFAULT_SCORE_BARS_PER_SYSTEM: usize = 4;
pub(super) const MIN_SCORE_BARS_PER_SYSTEM: usize = 1;
pub(super) const MAX_SCORE_BARS_PER_SYSTEM: usize = 8;
/// Zoom bounds.
pub(super) const MIN_SCORE_ZOOM: f32 = 0.5;
pub(super) const MAX_SCORE_ZOOM: f32 = 2.0;

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

/// One edit requested by a button or key in the score window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScoreNoteFlag {
    Hidden,
    Flipped,
    Grace,
    Slur,
    Group,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ScoreAction {
    Move(f64),
    Transpose(i16),
    Delete,
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

/// Stem direction for a staff position, honouring the note's flip flag.
pub(super) fn stem_up(position: i32, flipped: bool) -> bool {
    let up = position < 4;
    if flipped { !up } else { up }
}

/// In page mode, the system and beat offset inside that system.
pub(super) fn page_system_for_beats(beats: f64, bar: f64, bars_per_system: usize) -> (usize, f64) {
    let per_system = bars_per_system.max(1);
    let bar_index = (beats / bar.max(0.25)).floor().max(0.0) as usize;
    let within = beats - bar_index as f64 * bar;
    (
        bar_index / per_system,
        (bar_index % per_system) as f64 * bar + within,
    )
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

    /// Opens the score for the selected MIDI part.
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
            "Score: click a note, then move/transpose/delete it from the buttons or the arrow keys and Delete"
                .to_owned();
    }

    /// Selects one note for the score (and the MIDI editor selection).
    pub(super) fn select_score_note(&mut self, id: u64) {
        self.selected_midi_note_ids.clear();
        self.selected_midi_note_ids.insert(id);
        self.selected_midi_note_id = Some(id);
        let label = self
            .selected_midi_part()
            .and_then(|part| {
                part.notes
                    .iter()
                    .find(|note| note.id == id)
                    .map(|note| (note.pitch, note.start_beats))
            })
            .map(|(pitch, start)| format!("{} at beat {start:.3}", pitch_label(pitch)))
            .unwrap_or_else(|| "note".to_owned());
        self.status_message = format!("Selected {label}");
    }

    /// Moves the selected notes in time, clamped inside the part, as one step.
    pub(super) fn move_selected_score_notes(&mut self, delta_beats: f64) {
        let Some(part_id) = self.data.project.selected_midi_part_id else {
            self.status_message = "Select a MIDI part before moving notes".to_owned();
            return;
        };
        if self.selected_midi_note_ids.is_empty() || !delta_beats.is_finite() {
            self.status_message = "Select a note to move".to_owned();
            return;
        }
        let ids = self
            .selected_midi_note_ids
            .iter()
            .copied()
            .collect::<Vec<_>>();
        let mut candidate = self.data.project.clone();
        let Some(part) = candidate
            .tracks
            .iter_mut()
            .flat_map(|track| track.midi_parts.iter_mut())
            .find(|part| part.id == part_id)
        else {
            self.status_message = "The selected MIDI part is no longer in the project".to_owned();
            return;
        };
        let limit = part.duration_beats;
        let mut changed = 0usize;
        for note in part.notes.iter_mut() {
            if !ids.contains(&note.id) {
                continue;
            }
            let maximum = (limit - note.duration_beats).max(0.0);
            let target = (note.start_beats + delta_beats).clamp(0.0, maximum);
            if (target - note.start_beats).abs() > 1e-9 {
                note.start_beats = target;
                changed += 1;
            }
        }
        if changed == 0 {
            self.status_message = "The note cannot move further in that direction".to_owned();
            return;
        }
        self.record_score_edit(candidate, "Move Notes");
        self.status_message = format!("Moved {changed} note(s) by {delta_beats:+.3} beat(s)");
    }

    /// Transposes the selected score notes through the MIDI transpose command.
    pub(super) fn transpose_selected_score_notes(&mut self, semitones: i16) {
        self.transpose_selected_notes(semitones);
    }

    /// Deletes the selected score notes through the MIDI delete command.
    pub(super) fn delete_selected_score_notes(&mut self) {
        self.delete_selected_midi_notes();
    }

    fn record_score_edit(&mut self, candidate: Project, label: &str) {
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = format!("Cannot {label}: undo snapshot is unavailable");
            return;
        }
        self.project_history.label_next(label);
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
    }

    /// Aligns the selected notes left/right/top/bottom as one undo step.
    pub(super) fn align_selected_score_notes(&mut self, align: ScoreAlign) {
        let Some(part_id) = self.data.project.selected_midi_part_id else {
            self.status_message = "Select a MIDI part before aligning notes".to_owned();
            return;
        };
        let ids = self
            .selected_midi_note_ids
            .iter()
            .copied()
            .collect::<Vec<_>>();
        if ids.len() < 2 {
            self.status_message = "Select at least two notes to align".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        let Some(part) = candidate
            .tracks
            .iter_mut()
            .flat_map(|track| track.midi_parts.iter_mut())
            .find(|part| part.id == part_id)
        else {
            self.status_message = "The selected MIDI part is no longer in the project".to_owned();
            return;
        };
        let changed = opentrack_core::midi_transform::align_notes(part, &ids, align);
        if changed == 0 {
            self.status_message = "The selected notes are already aligned".to_owned();
            return;
        }
        let (label, description) = match align {
            ScoreAlign::Left => ("Align Left", "starts to the earliest note"),
            ScoreAlign::Right => ("Align Right", "starts to the latest note"),
            ScoreAlign::Top => ("Align Top", "pitches to the highest note"),
            ScoreAlign::Bottom => ("Align Bottom", "pitches to the lowest note"),
            ScoreAlign::DistributeHorizontally => (
                "Distribute Horizontally",
                "starts evenly between the outer notes",
            ),
            ScoreAlign::DistributeVertically => (
                "Distribute Vertically",
                "pitches evenly between the outer notes",
            ),
        };
        self.record_score_edit(candidate, label);
        self.status_message = format!("Aligned {changed} note(s): {description}");
    }

    /// Analyses the selected notes (or the whole part) into chord symbols.
    pub(super) fn make_chord_symbols_for_selected_part(&mut self) {
        let Some(part_id) = self.data.project.selected_midi_part_id else {
            self.status_message = "Select a MIDI part before making chord symbols".to_owned();
            return;
        };
        let ids = self
            .selected_midi_note_ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut candidate = self.data.project.clone();
        let Some(part) = candidate
            .tracks
            .iter_mut()
            .flat_map(|track| track.midi_parts.iter_mut())
            .find(|part| part.id == part_id)
        else {
            self.status_message = "The selected MIDI part is no longer in the project".to_owned();
            return;
        };
        let notes = part
            .notes
            .iter()
            .filter(|note| ids.is_empty() || ids.contains(&note.id))
            .cloned()
            .collect::<Vec<_>>();
        let symbols = opentrack_core::chord_symbols::symbols_for_notes(&notes);
        if symbols.is_empty() {
            self.status_message =
                "No major, minor, diminished or augmented triads were found".to_owned();
            return;
        }
        let count = symbols.len();
        part.chord_symbols = symbols;
        if candidate.validate().is_err() {
            self.status_message = "The chord symbols would leave an invalid project".to_owned();
            return;
        }
        self.record_score_edit(candidate, "Make Chord Symbols");
        self.status_message = format!("Added {count} chord symbol(s) above the staff");
    }

    /// Toggles the score visibility of the selected notes as one undo step.
    pub(super) fn toggle_hidden_selected_score_notes(&mut self) {
        self.toggle_score_note_flag(ScoreNoteFlag::Hidden);
    }

    /// Toggles the stem direction of the selected notes as one undo step.
    pub(super) fn toggle_flipped_selected_score_notes(&mut self) {
        self.toggle_score_note_flag(ScoreNoteFlag::Flipped);
    }

    /// Converts the selected notes to grace notes (or back) as one undo step.
    pub(super) fn toggle_grace_selected_score_notes(&mut self) {
        self.toggle_score_note_flag(ScoreNoteFlag::Grace);
    }

    /// Toggles slurs from the selected notes to their next note.
    pub(super) fn toggle_slur_selected_score_notes(&mut self) {
        self.toggle_score_note_flag(ScoreNoteFlag::Slur);
    }

    /// Groups or ungroups the selected notes in the score.
    pub(super) fn toggle_group_selected_score_notes(&mut self) {
        self.toggle_score_note_flag(ScoreNoteFlag::Group);
    }

    fn toggle_score_note_flag(&mut self, flag: ScoreNoteFlag) {
        let Some(part_id) = self.data.project.selected_midi_part_id else {
            self.status_message = "Select a MIDI part before editing the score".to_owned();
            return;
        };
        if self.selected_midi_note_ids.is_empty() {
            self.status_message = "Select a note in the score first".to_owned();
            return;
        }
        let ids = self
            .selected_midi_note_ids
            .iter()
            .copied()
            .collect::<Vec<_>>();
        let mut candidate = self.data.project.clone();
        let Some(part) = candidate
            .tracks
            .iter_mut()
            .flat_map(|track| track.midi_parts.iter_mut())
            .find(|part| part.id == part_id)
        else {
            self.status_message = "The selected MIDI part is no longer in the project".to_owned();
            return;
        };
        let (changed, label) = match flag {
            ScoreNoteFlag::Hidden => (
                opentrack_core::midi_transform::toggle_hidden(part, &ids),
                "Hide/Show Notes",
            ),
            ScoreNoteFlag::Flipped => (
                opentrack_core::midi_transform::toggle_flipped(part, &ids),
                "Flip Notes",
            ),
            ScoreNoteFlag::Grace => (
                opentrack_core::midi_transform::toggle_grace(part, &ids),
                "Grace Notes",
            ),
            ScoreNoteFlag::Slur => (
                opentrack_core::midi_transform::toggle_slurs(part, &ids),
                "Slurs",
            ),
            ScoreNoteFlag::Group => (
                opentrack_core::midi_transform::toggle_note_group(part, &ids),
                "Group Notes",
            ),
        };
        if changed == 0 {
            self.status_message = "No selected note changed".to_owned();
            return;
        }
        self.record_score_edit(candidate, label);
        self.status_message = format!(
            "{} {changed} note(s); Undo restores them",
            match flag {
                ScoreNoteFlag::Hidden => "Toggled the score visibility of",
                ScoreNoteFlag::Flipped => "Toggled the stem direction of",
                ScoreNoteFlag::Grace => "Toggled grace notes for",
                ScoreNoteFlag::Slur => "Toggled slurs for",
                ScoreNoteFlag::Group => "Toggled grouping for",
            }
        );
    }

    pub(super) fn toggle_score_page_mode(&mut self) {
        self.score_page_mode = !self.score_page_mode;
        self.status_message = if self.score_page_mode {
            "Score page mode on".to_owned()
        } else {
            "Score continuous mode on".to_owned()
        };
    }

    pub(super) fn toggle_score_settings(&mut self) {
        self.score_settings_open = !self.score_settings_open;
    }

    pub(super) fn toggle_score_layout(&mut self) {
        self.score_layout_open = !self.score_layout_open;
    }

    /// Auto layout: four bars per system with page mode on.
    pub(super) fn auto_score_layout(&mut self) {
        self.score_bars_per_system = DEFAULT_SCORE_BARS_PER_SYSTEM;
        self.score_page_mode = true;
        self.status_message = "Auto layout: 4 bars per system, page mode on".to_owned();
    }

    /// Reset layout: the shipped defaults for bars, zoom and page mode.
    pub(super) fn reset_score_layout(&mut self) {
        self.score_bars_per_system = DEFAULT_SCORE_BARS_PER_SYSTEM;
        self.score_zoom = 1.0;
        self.score_page_mode = false;
        self.status_message =
            "Score layout reset: 4 bars per system, zoom 1.00, continuous mode".to_owned();
    }

    pub(super) fn show_score_layout(&mut self, ctx: &egui::Context) {
        if !self.score_layout_open {
            return;
        }
        let mut open = true;
        let mut bars = self.score_bars_per_system;
        let mut zoom = self.score_zoom;
        let mut page = self.score_page_mode;
        egui::Window::new("Score Layout")
            .id(Id::new("score-layout"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                egui::Grid::new("score-layout-grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Bars per system");
                        ui.add(
                            egui::DragValue::new(&mut bars)
                                .range(MIN_SCORE_BARS_PER_SYSTEM..=MAX_SCORE_BARS_PER_SYSTEM),
                        );
                        ui.end_row();
                        ui.label("Zoom");
                        ui.add(
                            egui::Slider::new(&mut zoom, MIN_SCORE_ZOOM..=MAX_SCORE_ZOOM)
                                .fixed_decimals(2),
                        );
                        ui.end_row();
                        ui.label("Page mode");
                        ui.checkbox(&mut page, "Stack systems as pages");
                        ui.end_row();
                    });
                ui.label(
                    egui::RichText::new(
                        "Layout settings are stored with the application, not in the project.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
            });
        self.score_bars_per_system =
            bars.clamp(MIN_SCORE_BARS_PER_SYSTEM, MAX_SCORE_BARS_PER_SYSTEM);
        self.score_zoom = zoom.clamp(MIN_SCORE_ZOOM, MAX_SCORE_ZOOM);
        self.score_page_mode = page;
        self.score_layout_open = open;
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
        let scale = self.score_zoom.clamp(MIN_SCORE_ZOOM, MAX_SCORE_ZOOM);
        let px_per_beat = 26.0 * scale;
        let page_mode = self.score_page_mode;
        let bars_per_system = self
            .score_bars_per_system
            .clamp(MIN_SCORE_BARS_PER_SYSTEM, MAX_SCORE_BARS_PER_SYSTEM);
        let systems = if page_mode {
            bars.div_ceil(bars_per_system).max(1)
        } else {
            1
        };
        let selected: BTreeSet<u64> = self.selected_midi_note_ids.iter().copied().collect();
        let mut open = true;
        let mut action = None;
        let mut clicked_note = None;
        let mut toggle_page = false;
        let mut settings = false;
        let mut hide_show = false;
        let mut flip = false;
        let mut grace = false;
        let mut slur = false;
        let mut group = false;
        let response = egui::Window::new("Score")
            .id(Id::new("score-view"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(600.0, 400.0))
            .open(&mut open)
            .show(ctx, |ui| {
                let slurs = part.notes.iter().filter(|note| note.slur_to_next).count();
                let groups = part
                    .notes
                    .iter()
                    .filter_map(|note| note.group)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len();
                let mut suffix = String::new();
                if page_mode {
                    suffix.push_str(&format!(
                        " · page mode, {bars_per_system} bar(s)/system"
                    ));
                }
                if slurs > 0 {
                    suffix.push_str(&format!(" · {slurs} slur(s)"));
                }
                if groups > 0 {
                    suffix.push_str(&format!(" · {groups} group(s)"));
                }
                if !part.chord_symbols.is_empty() {
                    suffix.push_str(&format!(" · {} chord(s)", part.chord_symbols.len()));
                }
                ui.label(format!(
                    "{} · {}/{} · {bars} bar(s) · {} note(s){suffix}",
                    part.name,
                    self.data.project.beats_per_bar,
                    self.data.project.beat_unit,
                    part.notes.len(),
                ));
                ui.label(
                    egui::RichText::new(
                        "Click a note to select it. Arrow keys move (left/right) or transpose (up/down); Shift makes a larger step. Delete removes it. Engraving and symbols are not implemented.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.horizontal_wrapped(|ui| {
                    if ui.button("◀ 1/4").clicked() {
                        action = Some(ScoreAction::Move(-0.25));
                    }
                    if ui.button("1/4 ▶").clicked() {
                        action = Some(ScoreAction::Move(0.25));
                    }
                    if ui.button("▲ +1").clicked() {
                        action = Some(ScoreAction::Transpose(1));
                    }
                    if ui.button("▼ -1").clicked() {
                        action = Some(ScoreAction::Transpose(-1));
                    }
                    if ui.button("Delete Note").clicked() {
                        action = Some(ScoreAction::Delete);
                    }
                    if ui.button("Hide/Show").clicked() {
                        hide_show = true;
                    }
                    if ui.button("Flip").clicked() {
                        flip = true;
                    }
                    if ui.button("Grace").clicked() {
                        grace = true;
                    }
                    if ui.button("Slur").clicked() {
                        slur = true;
                    }
                    if ui.button("Group").clicked() {
                        group = true;
                    }
                    if ui
                        .selectable_label(page_mode, "Page Mode")
                        .clicked()
                    {
                        toggle_page = true;
                    }
                    if ui.button("Settings...").clicked() {
                        settings = true;
                    }
                });
                let (up, down, left, right, shift, delete) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.key_pressed(egui::Key::ArrowLeft),
                        i.key_pressed(egui::Key::ArrowRight),
                        i.modifiers.shift,
                        i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                    )
                });
                if up {
                    action = Some(ScoreAction::Transpose(if shift { 12 } else { 1 }));
                } else if down {
                    action = Some(ScoreAction::Transpose(if shift { -12 } else { -1 }));
                } else if left {
                    action = Some(ScoreAction::Move(if shift { -1.0 } else { -0.25 }));
                } else if right {
                    action = Some(ScoreAction::Move(if shift { 1.0 } else { 0.25 }));
                } else if delete {
                    action = Some(ScoreAction::Delete);
                }
                let width = 80.0
                    + if page_mode {
                        bars_per_system as f32 * bar as f32 * px_per_beat
                    } else {
                        bars as f32 * bar as f32 * px_per_beat
                    }
                    + 24.0;
                let height = if page_mode {
                    systems as f32 * 170.0 * scale + 60.0
                } else {
                    300.0 * scale + 20.0
                };
                let mut note_centers: Vec<(u64, Pos2)> = Vec::new();
                let mut system_notes: Vec<(u64, f64, Pos2, bool, Option<u64>)> = Vec::new();
                egui::ScrollArea::both()
                    .id_salt("score-scroll")
                    .show(ui, |ui| {
                        let (rect, canvas) = ui.allocate_exact_size(
                            Vec2::new(width.max(560.0), height.max(240.0)),
                            Sense::click(),
                        );
                        let painter = ui.painter_at(rect);
                        let colors = theme::palette(ui);
                        let spacing = 10.0_f32 * scale;
                        let system_gap = 170.0 * scale;
                        for system in 0..systems {
                            let treble_top = rect.top() + 40.0 + system as f32 * system_gap;
                            let bass_top = treble_top + 150.0 * scale;
                            for (label, top) in [("Treble", treble_top), ("Bass", bass_top)] {
                                for line in 0..5 {
                                    let y = top + line as f32 * spacing;
                                    painter.line_segment(
                                        [
                                            Pos2::new(rect.left() + 56.0, y),
                                            Pos2::new(rect.right() - 8.0, y),
                                        ],
                                        Stroke::new(1.0, colors.grid),
                                    );
                                }
                                painter.text(
                                    Pos2::new(rect.left() + 8.0, top + 2.0 * spacing),
                                    Align2::LEFT_CENTER,
                                    label,
                                    FontId::proportional(12.0 * scale),
                                    colors.muted,
                                );
                            }
                            let bars_in_system = if page_mode {
                                bars_per_system
                            } else {
                                bars
                            };
                            for symbol in &part.chord_symbols {
                                let (symbol_system, x_beats) = if page_mode {
                                    page_system_for_beats(symbol.beat, bar, bars_per_system)
                                } else {
                                    (0, symbol.beat)
                                };
                                if symbol_system != system {
                                    continue;
                                }
                                let x = rect.left() + 56.0 + x_beats as f32 * px_per_beat;
                                painter.text(
                                    Pos2::new(x, treble_top - 10.0 * scale),
                                    Align2::LEFT_BOTTOM,
                                    &symbol.text,
                                    FontId::proportional(13.0 * scale),
                                    colors.accent,
                                );
                            }
                            for bar_index in 1..=bars_in_system {
                                let global_bar = if page_mode {
                                    system * bars_per_system + bar_index
                                } else {
                                    bar_index
                                };
                                if global_bar > bars {
                                    break;
                                }
                                let x = rect.left()
                                    + 56.0
                                    + bar_index as f32 * bar as f32 * px_per_beat;
                                if x > rect.right() - 8.0 {
                                    break;
                                }
                                painter.line_segment(
                                    [
                                        Pos2::new(x, treble_top),
                                        Pos2::new(x, bass_top + 4.0 * spacing),
                                    ],
                                    Stroke::new(1.0, colors.grid),
                                );
                            }
                            for note in &part.notes {
                                let (system_of_note, x_beats) = if page_mode {
                                    page_system_for_beats(
                                        note.start_beats.max(0.0),
                                        bar,
                                        bars_per_system,
                                    )
                                } else {
                                    (0, note.start_beats.max(0.0))
                                };
                                if system_of_note != system {
                                    continue;
                                }
                                let staff = staff_for_pitch(note.pitch);
                                let position = staff_position(note.pitch, staff);
                                let bottom = if staff == Staff::Treble {
                                    treble_top + 4.0 * spacing
                                } else {
                                    bass_top + 4.0 * spacing
                                };
                                let y = bottom - position as f32 * spacing * 0.5;
                                let x =
                                    rect.left() + 56.0 + x_beats as f32 * px_per_beat;
                                if x > rect.right() - 12.0 {
                                    continue;
                                }
                                note_centers.push((note.id, Pos2::new(x, y)));
                                system_notes.push((
                                    note.id,
                                    note.start_beats,
                                    Pos2::new(x, y),
                                    note.slur_to_next,
                                    note.group,
                                ));
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
                                let grace_scale = if note.grace { 0.62 } else { 1.0 };
                                let radius = Vec2::new(
                                    5.0 * scale * grace_scale,
                                    3.6 * scale * grace_scale,
                                );
                                let center = Pos2::new(x, y);
                                let is_selected = selected.contains(&note.id);
                                if is_selected {
                                    painter.circle_stroke(
                                        center,
                                        7.5 * scale,
                                        Stroke::new(1.5, colors.accent),
                                    );
                                }
                                if note.hidden {
                                    // A hidden note keeps its staff position as a
                                    // muted placeholder; it still sounds.
                                    painter.add(egui::Shape::ellipse_stroke(
                                        center,
                                        Vec2::new(3.4 * scale, 2.4 * scale),
                                        Stroke::new(1.0, colors.muted),
                                    ));
                                    painter.text(
                                        center,
                                        Align2::CENTER_CENTER,
                                        "×",
                                        FontId::proportional(9.0 * scale),
                                        colors.muted,
                                    );
                                    continue;
                                }
                                let filled = matches!(
                                    head,
                                    NoteHead::Quarter
                                        | NoteHead::Eighth
                                        | NoteHead::Sixteenth
                                        | NoteHead::Shorter
                                );
                                if filled {
                                    painter.add(egui::Shape::ellipse_filled(
                                        center, radius, colors.text,
                                    ));
                                } else {
                                    painter.add(egui::Shape::ellipse_stroke(
                                        center,
                                        radius,
                                        Stroke::new(1.4, colors.text),
                                    ));
                                }
                                if !matches!(head, NoteHead::Whole) {
                                    let up = stem_up(position, note.flipped);
                                    let stem_end = if up {
                                        Pos2::new(x + 4.5 * scale, y - 26.0 * scale * grace_scale)
                                    } else {
                                        Pos2::new(x + 4.5 * scale, y + 26.0 * scale * grace_scale)
                                    };
                                    painter.line_segment(
                                        [Pos2::new(x + 4.5 * scale, y), stem_end],
                                        Stroke::new(1.2, colors.text),
                                    );
                                    for flag in 0..note_flags(head) {
                                        let offset = (4.0 + flag as f32 * 4.0) * scale;
                                        let direction = if up { 1.0 } else { -1.0 };
                                        painter.line_segment(
                                            [
                                                stem_end,
                                                Pos2::new(
                                                    stem_end.x + 7.0 * scale,
                                                    stem_end.y + direction * offset,
                                                ),
                                            ],
                                            Stroke::new(1.2, colors.text),
                                        );
                                    }
                                }
                                if note.grace {
                                    painter.text(
                                        Pos2::new(x + 3.0 * scale, y - 5.0 * scale),
                                        Align2::CENTER_CENTER,
                                        "/",
                                        FontId::proportional(10.0 * scale),
                                        colors.text,
                                    );
                                }
                                if is_accidental(note.pitch) {
                                    painter.text(
                                        Pos2::new(x - 10.0 * scale, y),
                                        Align2::CENTER_CENTER,
                                        "#",
                                        FontId::proportional(12.0 * scale),
                                        colors.text,
                                    );
                                }
                            }
                            for (_, start, center, slurred, _) in &system_notes {
                                if !*slurred {
                                    continue;
                                }
                                let next = system_notes
                                    .iter()
                                    .filter(|(_, other, _, _, _)| other > start)
                                    .min_by(|left, right| left.1.total_cmp(&right.1));
                                let Some((_, _, next, _, _)) = next else {
                                    continue;
                                };
                                let top = center.y.min(next.y) - 14.0 * scale;
                                let control = Pos2::new((center.x + next.x) * 0.5, top);
                                let mut points = Vec::with_capacity(12);
                                for step in 0..=10 {
                                    let t = step as f32 / 10.0;
                                    let inverse = 1.0 - t;
                                    let px = inverse * inverse * center.x
                                        + 2.0 * inverse * t * control.x
                                        + t * t * next.x;
                                    let py = inverse * inverse * center.y
                                        + 2.0 * inverse * t * control.y
                                        + t * t * next.y;
                                    points.push(Pos2::new(px, py));
                                }
                                painter.add(egui::Shape::line(
                                    points,
                                    Stroke::new(1.0, colors.text),
                                ));
                            }
                            let mut group_ids = std::collections::BTreeSet::new();
                            for (_, _, _, _, group) in &system_notes {
                                if let Some(id) = group {
                                    group_ids.insert(*id);
                                }
                            }
                            for id in group_ids {
                                let members = system_notes
                                    .iter()
                                    .filter(|(_, _, _, _, group)| *group == Some(id))
                                    .collect::<Vec<_>>();
                                if members.len() < 2 {
                                    continue;
                                }
                                let left = members
                                    .iter()
                                    .map(|(_, _, center, _, _)| center.x)
                                    .fold(f32::INFINITY, f32::min);
                                let right = members
                                    .iter()
                                    .map(|(_, _, center, _, _)| center.x)
                                    .fold(f32::NEG_INFINITY, f32::max);
                                let top = members
                                    .iter()
                                    .map(|(_, _, center, _, _)| center.y)
                                    .fold(f32::INFINITY, f32::min)
                                    - 22.0 * scale;
                                let tick = 4.0 * scale;
                                painter.line_segment(
                                    [Pos2::new(left, top), Pos2::new(right, top)],
                                    Stroke::new(1.2, colors.accent),
                                );
                                painter.line_segment(
                                    [Pos2::new(left, top), Pos2::new(left, top + tick)],
                                    Stroke::new(1.2, colors.accent),
                                );
                                painter.line_segment(
                                    [Pos2::new(right, top), Pos2::new(right, top + tick)],
                                    Stroke::new(1.2, colors.accent),
                                );
                            }
                            system_notes.clear();
                        }
                        if canvas.clicked()
                            && let Some(pointer) = canvas.interact_pointer_pos()
                            && let Some((id, _)) = note_centers.iter().min_by(|left, right| {
                                left.1
                                    .distance(pointer)
                                    .total_cmp(&right.1.distance(pointer))
                            })
                            && note_centers
                                .iter()
                                .any(|(_, center)| center.distance(pointer) <= 14.0)
                        {
                            clicked_note = Some(*id);
                        }
                    });
            });
        dismiss_window_on_outside_click(ctx, Id::new("score-view"), &mut open, response.as_ref());
        if toggle_page {
            self.toggle_score_page_mode();
        }
        if hide_show {
            self.toggle_hidden_selected_score_notes();
        }
        if flip {
            self.toggle_flipped_selected_score_notes();
        }
        if grace {
            self.toggle_grace_selected_score_notes();
        }
        if slur {
            self.toggle_slur_selected_score_notes();
        }
        if group {
            self.toggle_group_selected_score_notes();
        }
        if settings {
            self.toggle_score_settings();
        }
        if let Some(id) = clicked_note {
            self.select_score_note(id);
        } else if let Some(action) = action {
            match action {
                ScoreAction::Move(delta) => self.move_selected_score_notes(delta),
                ScoreAction::Transpose(semitones) => self.transpose_selected_score_notes(semitones),
                ScoreAction::Delete => self.delete_selected_score_notes(),
            }
        }
        self.score_view_open = open;
        self.show_score_settings(ctx);
    }

    pub(super) fn show_score_settings(&mut self, ctx: &egui::Context) {
        if !self.score_settings_open {
            return;
        }
        let mut open = true;
        let mut zoom = self.score_zoom;
        egui::Window::new("Score Settings")
            .id(Id::new("score-settings"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                ui.label("Zoom");
                ui.add(
                    egui::Slider::new(&mut zoom, MIN_SCORE_ZOOM..=MAX_SCORE_ZOOM).fixed_decimals(2),
                );
                ui.label(
                    egui::RichText::new(
                        "Page mode and zoom are view settings; they are not part of the project.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
            });
        self.score_zoom = zoom.clamp(MIN_SCORE_ZOOM, MAX_SCORE_ZOOM);
        self.score_settings_open = open;
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
    fn time_signatures_note_shapes_and_paging_are_mapped() {
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

        // Page mode wraps every four bars onto the next system.
        assert_eq!(page_system_for_beats(9.0, 4.0, 4), (0, 9.0));
        assert_eq!(page_system_for_beats(16.0, 4.0, 4), (1, 0.0));
        assert_eq!(page_system_for_beats(17.5, 4.0, 4), (1, 1.5));
        // Two bars per system start a new system after bar 2.
        assert_eq!(page_system_for_beats(8.0, 4.0, 2), (1, 0.0));
        assert_eq!(page_system_for_beats(4.0, 4.0, 2), (0, 4.0));
    }

    #[test]
    fn pitches_map_to_a_staff_line_or_space() {
        assert_eq!(staff_for_pitch(59), Staff::Bass);
        assert_eq!(staff_for_pitch(60), Staff::Treble);
        assert_eq!(staff_position(64, Staff::Treble), 0);
        assert_eq!(staff_position(62, Staff::Treble), -1);
        assert_eq!(staff_position(60, Staff::Treble), -2);
        assert_eq!(staff_position(72, Staff::Treble), 5);
        assert_eq!(staff_position(43, Staff::Bass), 0);
        assert_eq!(staff_position(45, Staff::Bass), 1);
        assert_eq!(staff_position(60, Staff::Bass), 10);

        assert_eq!(pitch_label(60), "C4");
        assert_eq!(pitch_label(61), "C#4");
        assert_eq!(pitch_label(43), "G2");
        assert!(is_accidental(61) && !is_accidental(60));
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
    }

    #[test]
    fn notes_can_be_selected_moved_transposed_and_deleted() {
        let mut app = app_with_part();
        let part_id = app.data.project.selected_midi_part_id.expect("part");
        let notes = app.data.project.tracks[0]
            .midi_parts
            .iter()
            .find(|part| part.id == part_id)
            .expect("part")
            .notes
            .clone();
        let first = &notes[0];

        app.select_score_note(first.id);
        assert_eq!(app.selected_midi_note_id, Some(first.id));
        assert!(app.status_message.contains("C4 at beat 0.000"));

        // Moving one grid step is one undo step; moving left at zero clamps.
        app.move_selected_score_notes(0.25);
        let moved = app.data.project.tracks[0].midi_parts[0]
            .notes
            .iter()
            .find(|note| note.id == first.id)
            .expect("note");
        assert!((moved.start_beats - 0.25).abs() < 1e-9);
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Move Notes");
        app.move_selected_score_notes(-1.0);
        let clamped = app.data.project.tracks[0].midi_parts[0]
            .notes
            .iter()
            .find(|note| note.id == first.id)
            .expect("note");
        assert_eq!(clamped.start_beats, 0.0);

        // Transpose goes through the MIDI command and its own label.
        app.transpose_selected_score_notes(2);
        let transposed = app.data.project.tracks[0].midi_parts[0]
            .notes
            .iter()
            .find(|note| note.id == first.id)
            .expect("note");
        assert_eq!(transposed.pitch, 62);
        app.project_history.finish_frame(&app.data.project, true);
        assert!(
            app.project_history
                .steps()
                .iter()
                .any(|step| step.label == "Transpose Notes")
        );

        // Delete removes the selection through the MIDI delete command.
        app.select_score_note(first.id);
        app.delete_selected_score_notes();
        assert!(
            app.data.project.tracks[0].midi_parts[0]
                .notes
                .iter()
                .all(|note| note.id != first.id)
        );
        assert_eq!(app.data.project.tracks[0].midi_parts[0].notes.len(), 2);
    }

    #[test]
    fn layout_applies_auto_and_reset() {
        let mut app = app_with_part();
        app.score_bars_per_system = 3;
        app.score_zoom = 1.5;
        app.score_page_mode = true;
        app.auto_score_layout();
        assert_eq!(app.score_bars_per_system, DEFAULT_SCORE_BARS_PER_SYSTEM);
        assert!(app.score_page_mode);
        assert!((app.score_zoom - 1.5).abs() < 1e-6, "auto keeps the zoom");
        assert!(app.status_message.contains("4 bars per system"));

        app.reset_score_layout();
        assert_eq!(app.score_bars_per_system, DEFAULT_SCORE_BARS_PER_SYSTEM);
        assert!((app.score_zoom - 1.0).abs() < 1e-6);
        assert!(!app.score_page_mode);
        assert!(app.status_message.contains("reset"));
    }

    #[test]
    fn the_layout_dialog_draws_its_controls() {
        let ctx = egui::Context::default();
        let mut app = app_with_part();
        app.score_layout_open = true;
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_score_layout(&ctx),
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
        assert!(joined.contains("Score Layout"), "{joined}");
        assert!(joined.contains("Bars per system"), "{joined}");
        assert!(joined.contains("Zoom"), "{joined}");
        assert!(joined.contains("Page mode"), "{joined}");
    }

    #[test]
    fn hidden_and_flipped_notes_toggle_with_undo() {
        let mut app = app_with_part();
        let part_id = app.data.project.selected_midi_part_id.expect("part");
        let first = app.data.project.tracks[0].midi_parts[0].notes[0].id;
        let second = app.data.project.tracks[0].midi_parts[0].notes[1].id;

        app.select_score_note(first);
        app.toggle_hidden_selected_score_notes();
        assert!(app.data.project.tracks[0].midi_parts[0].notes[0].hidden);
        app.project_history.finish_frame(&app.data.project, true);
        assert!(
            app.project_history
                .steps()
                .iter()
                .any(|step| step.label == "Hide/Show Notes")
        );

        // Without a selection the command reports.
        app.selected_midi_note_ids.clear();
        app.toggle_hidden_selected_score_notes();
        assert!(app.status_message.contains("Select a note"));

        // Flip applies to the selected notes only.
        app.select_score_note(second);
        app.toggle_flipped_selected_score_notes();
        assert!(app.data.project.tracks[0].midi_parts[0].notes[1].flipped);
        assert!(!app.data.project.tracks[0].midi_parts[0].notes[0].flipped);
        app.project_history.finish_frame(&app.data.project, true);
        assert!(
            app.project_history
                .steps()
                .iter()
                .any(|step| step.label == "Flip Notes")
        );
        assert_eq!(
            part_id,
            app.data.project.selected_midi_part_id.expect("part")
        );
    }

    #[test]
    fn stem_direction_respects_the_flip_flag() {
        assert!(stem_up(0, false));
        assert!(!stem_up(4, false));
        assert!(!stem_up(0, true));
        assert!(stem_up(4, true));
    }

    #[test]
    fn selected_notes_align_with_labelled_undo_steps() {
        let mut app = app_with_part();
        app.selected_midi_note_ids.clear();
        for note in &app.data.project.tracks[0].midi_parts[0].notes {
            app.selected_midi_note_ids.insert(note.id);
        }
        app.align_selected_score_notes(ScoreAlign::Left);
        assert!(
            app.data.project.tracks[0].midi_parts[0]
                .notes
                .iter()
                .all(|note| note.start_beats == 0.0)
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert!(
            app.project_history
                .steps()
                .iter()
                .any(|step| step.label == "Align Left")
        );

        app.align_selected_score_notes(ScoreAlign::Top);
        assert!(
            app.data.project.tracks[0].midi_parts[0]
                .notes
                .iter()
                .all(|note| note.pitch == 64)
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert!(
            app.project_history
                .steps()
                .iter()
                .any(|step| step.label == "Align Top")
        );

        // Distribution spaces the middle note between the outer ones.
        app.data.project.tracks[0].midi_parts[0].notes[0].start_beats = 0.0;
        app.data.project.tracks[0].midi_parts[0].notes[1].start_beats = 0.5;
        app.data.project.tracks[0].midi_parts[0].notes[2].start_beats = 2.0;
        app.selected_midi_note_ids.clear();
        for note in &app.data.project.tracks[0].midi_parts[0].notes {
            app.selected_midi_note_ids.insert(note.id);
        }
        app.align_selected_score_notes(ScoreAlign::DistributeHorizontally);
        assert!((app.data.project.tracks[0].midi_parts[0].notes[1].start_beats - 1.0).abs() < 1e-9);
        app.project_history.finish_frame(&app.data.project, true);
        assert!(
            app.project_history
                .steps()
                .iter()
                .any(|step| step.label == "Distribute Horizontally")
        );

        app.data.project.tracks[0].midi_parts[0].notes[0].pitch = 60;
        app.data.project.tracks[0].midi_parts[0].notes[1].pitch = 61;
        app.data.project.tracks[0].midi_parts[0].notes[2].pitch = 72;
        app.align_selected_score_notes(ScoreAlign::DistributeVertically);
        assert_eq!(app.data.project.tracks[0].midi_parts[0].notes[1].pitch, 66);
        app.project_history.finish_frame(&app.data.project, true);
        assert!(
            app.project_history
                .steps()
                .iter()
                .any(|step| step.label == "Distribute Vertically")
        );

        // One selected note is not an alignment.
        let first = app.data.project.tracks[0].midi_parts[0].notes[0].id;
        app.selected_midi_note_ids.clear();
        app.selected_midi_note_ids.insert(first);
        app.align_selected_score_notes(ScoreAlign::Bottom);
        assert!(app.status_message.contains("at least two notes"));
    }

    #[test]
    fn chord_symbols_are_made_from_triads_with_undo() {
        let mut app = app_with_part();
        // The fixture has single notes at 0, 1 and 2 beats: no triads.
        app.make_chord_symbols_for_selected_part();
        assert!(
            app.status_message.contains("No major, minor"),
            "{}",
            app.status_message
        );

        // Add a C major triad at beat 0 and an A minor triad at beat 1.
        let part_id = app.data.project.selected_midi_part_id.expect("part");
        app.data.project.tracks[0].midi_parts[0].notes.clear();
        for (index, (beat, pitch)) in [
            (0.0, 60),
            (0.0, 64),
            (0.0, 67),
            (1.0, 57),
            (1.0, 60),
            (1.0, 64),
        ]
        .into_iter()
        .enumerate()
        {
            app.data
                .project
                .add_midi_note(0, part_id, beat, 1.0, pitch, 100)
                .expect("note");
            let _ = index;
        }
        app.project_history.clear();
        app.make_chord_symbols_for_selected_part();
        let symbols = &app.data.project.tracks[0].midi_parts[0].chord_symbols;
        assert_eq!(symbols.len(), 2, "{symbols:?}");
        assert_eq!(symbols[0].text, "C");
        assert_eq!(symbols[1].text, "Am");
        app.project_history.finish_frame(&app.data.project, true);
        assert!(
            app.project_history
                .steps()
                .iter()
                .any(|step| step.label == "Make Chord Symbols")
        );
        assert!(app.status_message.contains("2 chord symbol(s)"));
    }

    #[test]
    fn page_mode_and_zoom_are_view_settings() {
        let mut app = app_with_part();
        assert!(!app.score_page_mode);
        app.toggle_score_page_mode();
        assert!(app.score_page_mode);
        assert!(app.status_message.contains("page mode"));
        app.toggle_score_page_mode();
        assert!(!app.score_page_mode);

        app.toggle_score_settings();
        assert!(app.score_settings_open);
        app.toggle_score_settings();
        assert!(!app.score_settings_open);
    }

    #[test]
    fn the_window_draws_the_staves_and_note_count() {
        let ctx = egui::Context::default();
        let mut app = app_with_part();
        app.score_view_open = true;
        app.data.project.tracks[0].midi_parts[0].notes[0].hidden = true;
        app.data.project.tracks[0].midi_parts[0].notes[1].grace = true;
        app.data.project.tracks[0].midi_parts[0].notes[1].slur_to_next = true;
        app.data.project.tracks[0].midi_parts[0].notes[0].group = Some(1);
        app.data.project.tracks[0].midi_parts[0].chord_symbols.push(
            opentrack_core::project::ChordSymbol {
                beat: 0.0,
                text: "C".to_owned(),
            },
        );
        app.data.project.tracks[0].midi_parts[0].notes[1].group = Some(1);
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
        assert!(joined.contains("Page Mode"), "{joined}");
        assert!(joined.contains("×"), "hidden placeholder drawn: {joined}");
    }
}
