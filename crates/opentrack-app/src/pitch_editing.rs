//! Note-only gestures publish persistent intent directly to live playback.
use super::*;
use opentrack_core::{
    pitch_time::{PitchNote, PitchTimeEdit},
    project::AudioClip,
};
use std::collections::BTreeSet;

const MIN_NOTE_LENGTH_SECONDS: f64 = 0.01;
const FINE_PITCH_SENSITIVITY: f32 = 0.25;
const ATTACHED_NOTE_TOLERANCE_SECONDS: f64 = 1e-6;
const NOTE_COLORS: [Color32; 4] = [
    Color32::from_rgb(216, 125, 120),
    Color32::from_rgb(179, 146, 225),
    Color32::from_rgb(120, 189, 155),
    Color32::from_rgb(104, 170, 223),
];
/// Only used when no track color is supplied, such as in isolated editor tests.
const DEFAULT_SELECTED_NOTE_COLOR: Color32 = Color32::from_rgb(157, 202, 246);

/// The palette index follows the note's current rounded semitone center, not
/// the source pitch it was detected at. Invalid centers fall back to the first
/// row instead of wrapping unpredictably.
fn note_color_index(note: &PitchNote) -> usize {
    let center = note.detected_midi + note.shift;
    if !center.is_finite() {
        return 0;
    }
    (center.round().clamp(0.0, 127.0) as usize) % NOTE_COLORS.len()
}

fn note_color(note: &PitchNote) -> Color32 {
    NOTE_COLORS[note_color_index(note)]
}

fn center_pitch_cents(note: &PitchNote) -> f32 {
    let pitch = note.detected_midi + note.shift;
    (pitch - pitch.round()) * 100.0
}

#[derive(Clone)]
struct MarqueeGesture {
    origin: Pos2,
    additive: bool,
    selected_before: BTreeSet<u64>,
}

fn note_screen_rect(
    note: &PitchNote,
    wave: Rect,
    view: &PitchViewport,
    start: f64,
    end: f64,
) -> Rect {
    let x = |t: f64| wave.left() + ((t - start) / (end - start)) as f32 * wave.width();
    let h = (wave.height() / view.span * 0.80).max(12.0);
    Rect::from_min_max(
        Pos2::new(
            x(note.start),
            view.y(note.detected_midi + note.shift, wave) - h * 0.5,
        ),
        Pos2::new(
            x(note.end),
            view.y(note.detected_midi + note.shift, wave) + h * 0.5,
        ),
    )
}

fn reset_selected_notes(notes: &mut [PitchNote], selected: &BTreeSet<u64>, duration: f64) -> bool {
    let before = notes.to_vec();
    for note in notes.iter_mut().filter(|note| selected.contains(&note.id)) {
        note.shift = 0.0;
        note.straighten = 0.0;
        note.start = note.source_start;
        note.end = note.source_end;
    }

    for i in 1..notes.len() {
        let left_selected = selected.contains(&notes[i - 1].id);
        let right_selected = selected.contains(&notes[i].id);
        let was_attached =
            (before[i - 1].end - before[i].start).abs() <= ATTACHED_NOTE_TOLERANCE_SECONDS;
        if left_selected && !right_selected && (was_attached || notes[i - 1].end > notes[i].start) {
            let target = notes[i - 1].end;
            align_note_boundary(notes, i - 1, target);
        } else if !left_selected
            && right_selected
            && (was_attached || notes[i - 1].end > notes[i].start)
        {
            let target = notes[i].start;
            align_note_boundary(notes, i - 1, target);
        } else if left_selected && right_selected && notes[i - 1].end > notes[i].start + 1e-6 {
            notes.clone_from_slice(&before);
            return false;
        }
    }

    let valid = note_positions_valid(notes, duration);
    if !valid {
        notes.clone_from_slice(&before);
    }
    valid
}

fn note_positions_valid(notes: &[PitchNote], duration: f64) -> bool {
    notes.iter().all(|note| {
        note.start.is_finite()
            && note.end.is_finite()
            && note.end - note.start >= MIN_NOTE_LENGTH_SECONDS
            && note.start >= 0.0
            && note.end <= duration + 0.001
    }) && notes
        .windows(2)
        .all(|pair| pair[0].end <= pair[1].start + 1e-6)
}

fn align_note_boundary(notes: &mut [PitchNote], left_index: usize, target: f64) -> f64 {
    let right_index = left_index + 1;
    let low = notes[left_index].start + MIN_NOTE_LENGTH_SECONDS;
    let high = notes[right_index].end - MIN_NOTE_LENGTH_SECONDS;
    let current = notes[left_index].end.min(notes[right_index].start);
    let boundary = bounded_time(target, low, high, current);
    notes[left_index].end = boundary;
    notes[right_index].start = boundary;
    boundary
}

fn reset_note_edge(notes: &mut [PitchNote], index: usize, mode: u8, duration: f64) -> bool {
    let before = notes.to_vec();
    let target = if mode == 1 {
        notes[index].source_start
    } else {
        notes[index].source_end
    };
    let current = if mode == 1 {
        notes[index].start
    } else {
        notes[index].end
    };
    resize_note_edge(notes, index, mode, target - current, duration);
    if !note_positions_valid(notes, duration) {
        notes.clone_from_slice(&before);
        return false;
    }
    true
}

fn reset_note(
    notes: &mut [PitchNote],
    index: usize,
    duration: f64,
    analysis: &opentrack_audio::PitchAnalysis,
) -> bool {
    let before = notes.to_vec();
    notes[index].shift = 0.0;
    notes[index].straighten = 0.0;
    if let Some(center) =
        measured_center(analysis, notes[index].source_start, notes[index].source_end)
    {
        notes[index].detected_midi = center;
    }
    if !reset_note_edge(notes, index, 1, duration)
        || !reset_note_edge(notes, index, 2, duration)
        || !note_positions_valid(notes, duration)
    {
        notes.clone_from_slice(&before);
        return false;
    }
    true
}

fn resize_note_edge(
    notes: &mut [PitchNote],
    index: usize,
    mode: u8,
    delta_seconds: f64,
    duration: f64,
) {
    let old = notes[index].clone();
    if mode == 1 {
        let previous_attached = index > 0
            && (notes[index - 1].end - old.start).abs() <= ATTACHED_NOTE_TOLERANCE_SECONDS;
        let low = if index > 0 {
            notes[index - 1].start + MIN_NOTE_LENGTH_SECONDS
        } else {
            0.0
        };
        let new_start = bounded_time(
            old.start + delta_seconds,
            low,
            old.end - MIN_NOTE_LENGTH_SECONDS,
            old.start,
        );
        notes[index].start = new_start;
        if index > 0 && (previous_attached || new_start < notes[index - 1].end) {
            let boundary = align_note_boundary(notes, index - 1, new_start);
            notes[index].start = boundary;
        }
    } else {
        let next_attached = notes
            .get(index + 1)
            .is_some_and(|next| (next.start - old.end).abs() <= ATTACHED_NOTE_TOLERANCE_SECONDS);
        let high = notes
            .get(index + 1)
            .map_or(duration, |next| next.end - MIN_NOTE_LENGTH_SECONDS);
        let new_end = bounded_time(
            old.end + delta_seconds,
            old.start + MIN_NOTE_LENGTH_SECONDS,
            high,
            old.end,
        );
        notes[index].end = new_end;
        if index + 1 < notes.len() && (next_attached || new_end > notes[index + 1].start) {
            let boundary = align_note_boundary(notes, index, new_end);
            notes[index].end = boundary;
        }
    }
}

#[derive(Default)]
pub(super) struct NoteEditor {
    committed: Option<PitchTimeEdit>,
    pub draft: Option<PitchTimeEdit>,
    selected: BTreeSet<u64>,
    drag: Option<(u64, u8, Vec<PitchNote>)>,
    drag_origin: Option<Pos2>,
    drag_fine: bool,
    drag_pitch_active: bool,
    drag_pitch_base_shifts: Vec<f32>,
    marquee: Option<MarqueeGesture>,
    pub directory: Option<PathBuf>,
    pub message: String,
    pub audition_note: Option<u64>,
}

pub(crate) fn playback_path(clip: &AudioClip) -> &Path {
    &clip.source_path
}

impl NoteEditor {
    pub fn analysis_directory(&self) -> Option<PathBuf> {
        self.directory
            .as_deref()
            .and_then(Path::parent)
            .map(|p| p.join("Analysis"))
    }
    pub fn publish(&mut self) -> Option<PitchTimeEdit> {
        let draft = self.draft.as_mut()?;
        if self
            .committed
            .as_ref()
            .is_some_and(|c| c.notes == draft.notes && c.enabled == draft.enabled)
        {
            return None;
        }
        draft.rendered_source = None;
        let edit = draft.clone();
        self.committed = Some(edit.clone());
        Some(edit)
    }
    pub fn sync(&mut self, clip: &AudioClip) -> bool {
        if self.committed != clip.pitch_time {
            self.committed = clip.pitch_time.clone();
            self.draft = clip.pitch_time.clone();
            self.selected.clear();
            self.drag = None;
            self.marquee = None;
            return true;
        }
        false
    }
    pub fn selected_pitch_cents(&self) -> Option<f32> {
        let edit = self.draft.as_ref()?;
        edit.notes
            .iter()
            .find(|note| self.selected.contains(&note.id))
            .map(center_pitch_cents)
    }
    pub fn sidebar(
        &mut self,
        ui: &mut egui::Ui,
        clip: &AudioClip,
        enabled: bool,
    ) -> Option<PitchTimeEdit> {
        let Some(edit) = self.draft.as_mut() else {
            return None;
        };
        ui.label(format!("{} notes - 10 ms analysis", edit.notes.len()));
        let mut audition = None;

        ui.add_enabled_ui(enabled, |ui| {
            if let Some(index) = edit.notes.iter().position(|n| self.selected.contains(&n.id)) {
                let previous = if index > 0 { edit.notes[index-1].end } else { 0.0 };
                let next = edit.notes.get(index+1).map_or(clip.duration_seconds, |n| n.start);
                let source_previous = if index > 0 { edit.notes[index-1].source_end } else { 0.0 };
                let source_next = edit.notes.get(index+1).map_or(clip.duration_seconds, |n| n.source_start);
                {
                    let note = &mut edit.notes[index];
                    let key = (note.detected_midi + note.shift).round().clamp(0.0,127.0) as usize;
                    let names = ["C","C#","D","D#","E","F","F#","G","G#","A","A#","B"];
                    ui.label(format!("Note {}{}", names[key%12], key as i32/12-1));
                    ui.label(format!("Center {:+.1} cents", center_pitch_cents(note)));
                    ui.horizontal(|ui| {
                        note_value(ui, "Pitch", &mut note.shift, -12.0..=12.0, 0.01);
                        note_value(ui, "Straight", &mut note.straighten, 0.0..=1.0, 0.01);
                    });
                    let upper_start = note.end-0.01;
                    ui.horizontal(|ui| {
                        note_value(ui, "Start", &mut note.start, previous..=upper_start, 0.001);
                        let lower_end = note.start+0.01;
                        note_value(ui, "End", &mut note.end, lower_end..=next, 0.001);
                    });
                    ui.collapsing("Detection bounds", |ui| {
                        let upper_start = note.source_end-0.01;
                        note_value(ui, "From", &mut note.source_start, source_previous..=upper_start, 0.001);
                        let lower_end = note.source_start+0.01;
                        note_value(ui, "To", &mut note.source_end, lower_end..=source_next, 0.001);
                        if ui.small_button("Estimate center").clicked() {
                            if let Some(center) = measured_center(&edit.analysis,note.source_start,note.source_end) { note.detected_midi = center; }
                        }
                    });
                }
                let (quantize,reset)=ui.horizontal(|ui| (ui.small_button("Quantize pitch").clicked(),ui.small_button("Reset").clicked())).inner;
                let (join,remove)=ui.horizontal(|ui| (ui.add_enabled(self.selected.len()>1,egui::Button::new("Join").small()).clicked(),ui.small_button("Remove region").clicked())).inner;
                if quantize {
                    for n in &mut edit.notes {
                        if self.selected.contains(&n.id) { n.shift = ((n.detected_midi+n.shift).round()-n.detected_midi).clamp(-12.0,12.0); }
                    }
                }
                if reset {
                    if !reset_selected_notes(
                        &mut edit.notes,
                        &self.selected,
                        clip.duration_seconds,
                    ) {
                        self.message = "Selected notes could not be reset without violating their minimum length.".into();
                    } else {
                        self.message = "Reset selected notes.".into();
                    }
                }
                if join {
                    let indices: Vec<_> = edit.notes.iter().enumerate().filter(|(_,n)| self.selected.contains(&n.id)).map(|(i,_)| i).collect();
                    if let (Some(&a),Some(&b)) = (indices.first(),indices.last()) {
                        if b-a+1 == indices.len() {
                            let last = edit.notes[b].clone();
                            edit.notes[a].end=last.end;
                            edit.notes[a].source_end=last.source_end;
                            if let Some(center) = measured_center(&edit.analysis,edit.notes[a].source_start,last.source_end) { edit.notes[a].detected_midi=center; }
                            edit.notes.drain(a+1..=b);
                            self.selected=BTreeSet::from([edit.notes[a].id]);
                        } else { self.message="Join requires adjacent notes.".into(); }
                    }
                }
                if remove {
                    edit.notes.retain(|n| !self.selected.contains(&n.id));
                    self.selected.clear();
                }
            } else {
                ui.label("Select a note to edit.");
            }
            let (reset_all,original)=ui.horizontal(|ui| {
                let reset=ui.small_button("Reset all").clicked();
                let original=ui.small_button(if edit.enabled {"Hear original"} else {"Hear corrected"}).clicked();
                (reset,original)
            }).inner;
            if reset_all {edit.notes=opentrack_audio::segment_notes(&edit.analysis);self.selected.clear();}
            if original {edit.enabled=!edit.enabled;let mut changed=edit.clone();changed.rendered_source=None;audition=Some(changed);}

        });
        audition
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        wave: Rect,
        view: &PitchViewport,
        start: f64,
        end: f64,
        clip: &AudioClip,
        grid: Option<f64>,
        tempo: f64,
        tool: EditTool,
        enabled: bool,
        track_color: Option<Color32>,
    ) {
        self.audition_note = None;
        let Some(edit) = &mut self.draft else {
            return;
        };
        let x = |t: f64| wave.left() + ((t - start) / (end - start)) as f32 * wave.width();
        let painter = ui.painter().with_clip_rect(wave);
        let mut action = None;
        let mut reset_action = None;
        let mut fine_readout = None;
        let mut pressed_note = false;
        let interactive = enabled;
        let (pointer, pressed, down, released, double_clicked, modifiers) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.pointer.button_down(egui::PointerButton::Primary),
                i.pointer.button_released(egui::PointerButton::Primary),
                i.pointer
                    .button_double_clicked(egui::PointerButton::Primary),
                i.modifiers,
            )
        });
        for (index, note) in edit.notes.iter().enumerate() {
            let h = (wave.height() / view.span * 0.80).max(12.0);
            let note_rect = note_screen_rect(note, wave, view, start, end);
            let rect = note_rect;
            let captured = self.drag.as_ref().is_some_and(|(id, _, _)| *id == note.id);
            if !rect.intersects(wave) && !captured {
                continue;
            }
            let note_rect = rect;
            let rect = if rect.intersects(wave) {
                rect.intersect(wave)
            } else {
                rect
            };
            let edge_width = (note_rect.width() * 0.2).min(6.0);
            let fine_modifier = modifiers.shift;
            let trace_hit = pointer.is_some_and(|pos| {
                if !wave.contains(pos) || pos.x < rect.left() || pos.x > rect.right() {
                    return false;
                }
                let time =
                    start + (pos.x - wave.left()) as f64 / wave.width() as f64 * (end - start);
                let source = note.source_start
                    + (time - note.start) / (note.end - note.start)
                        * (note.source_end - note.source_start)
                    + edit.analysis.source_start_seconds;
                let i = edit
                    .analysis
                    .points
                    .partition_point(|p| p.source_time_seconds < source)
                    .min(edit.analysis.points.len().saturating_sub(1));
                edit.analysis
                    .points
                    .get(i)
                    .and_then(|p| p.frequency_hz)
                    .is_some_and(|f| {
                        let detected_midi = 69.0 + 12.0 * (f / 440.0).log2();
                        let corrected_midi = edit.pitch_midi_at(index, detected_midi, time, tempo);
                        (pos.y - view.y(corrected_midi, wave)).abs() < h * 0.5 + 3.0
                    })
            });
            let hovered = pointer
                .is_some_and(|p| wave.contains(p) && rect.expand2(Vec2::new(0.0, 4.0)).contains(p))
                || trace_hit;
            let dragging = down && (captured || (pressed && hovered));
            let hint = !fine_modifier
                && pointer.is_some_and(|p| {
                    wave.contains(p)
                        && p.x > rect.left() + 6.0
                        && p.x < rect.right() - 6.0
                        && p.y >= rect.bottom() - h * 0.5
                        && p.y <= rect.bottom() + 5.0
                });
            let cut_active = hovered
                && !fine_modifier
                && (tool == EditTool::Split
                    || (pointer.is_some_and(|p| {
                        rect.contains(p)
                            && p.y >= rect.bottom() - h * 0.25
                            && p.x > rect.left() + 6.0
                            && p.x < rect.right() - 6.0
                    })));
            let selected = self.selected.contains(&note.id);
            let color = if selected {
                track_color.unwrap_or(DEFAULT_SELECTED_NOTE_COLOR)
            } else {
                note_color(note)
            };
            painter.rect_filled(
                rect,
                0.0,
                color.gamma_multiply(if selected { 0.20 } else { 0.07 }),
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(if selected || hovered { 1.7 } else { 1.0 }, color),
                egui::StrokeKind::Inside,
            );
            if hint {
                let y = rect.bottom() - h * 0.25;
                painter.line_segment(
                    [
                        Pos2::new(x(note.start + 0.02).max(rect.left()), y),
                        Pos2::new(x(note.end - 0.02).min(rect.right()), y),
                    ],
                    Stroke::new(1.0, Color32::WHITE),
                );
            }
            if cut_active && let Some(pos) = pointer {
                let mut t =
                    start + (pos.x - wave.left()) as f64 / wave.width() as f64 * (end - start);
                if let Some(grid) = grid {
                    t = (crate::timeline::snap_beats_to_grid(
                        clip.start_beats + t * tempo / 60.0,
                        grid,
                    ) - clip.start_beats)
                        * 60.0
                        / tempo;
                }
                if t > note.start + 0.02 && t < note.end - 0.02 {
                    let guide = x(t);
                    if grid.is_some() {
                        painter.line_segment(
                            [
                                Pos2::new(guide, rect.top()),
                                Pos2::new(guide, rect.bottom()),
                            ],
                            Stroke::new(1.0, Color32::WHITE),
                        );
                        crate::system_tool_icons::tool_cursor_at(
                            ui,
                            ToolbarIcon::Split,
                            Some(Pos2::new(guide, pos.y)),
                        );
                    } else {
                        crate::system_tool_icons::tool_cursor_only_at(
                            ui,
                            ToolbarIcon::Split,
                            Some(pos),
                        );
                    }
                }
            }
            if hovered && !cut_active {
                let pos = pointer.unwrap();
                ui.ctx().set_cursor_icon(
                    if !fine_modifier
                        && ((pos.x - note_rect.left()).abs() < edge_width
                            || (pos.x - note_rect.right()).abs() < edge_width)
                    {
                        egui::CursorIcon::ResizeHorizontal
                    } else {
                        egui::CursorIcon::ResizeVertical
                    },
                );
            }
            if !interactive {
                continue;
            }
            if pressed && hovered {
                pressed_note = true;
                if cut_active && pressed {
                    if let Some(pos) = pointer {
                        let mut t = start
                            + (pos.x - wave.left()) as f64 / wave.width() as f64 * (end - start);
                        if let Some(grid) = grid {
                            let beat = clip.start_beats + t * tempo / 60.0;
                            t = (crate::timeline::snap_beats_to_grid(beat, grid)
                                - clip.start_beats)
                                * 60.0
                                / tempo;
                        }
                        if t > note.start + 0.02 && t < note.end - 0.02 {
                            action = Some((index, t));
                        }
                    }
                } else {
                    let multiple = ui.input(|i| i.modifiers.ctrl);
                    if !multiple && (fine_modifier || !selected) {
                        self.selected.clear();
                    }
                    self.selected.insert(note.id);
                }
            }
            if pressed && hovered && !cut_active {
                let pos = ui
                    .input(|i| i.pointer.press_origin())
                    .unwrap_or(rect.center());
                let mode = if fine_modifier {
                    0
                } else if (pos.x - note_rect.left()).abs() < edge_width {
                    1
                } else if (note_rect.right() - pos.x).abs() < edge_width {
                    2
                } else {
                    0
                };
                self.drag_origin = pointer;
                self.drag_fine = fine_modifier;
                self.drag_pitch_active = false;
                self.drag_pitch_base_shifts = edit.notes.iter().map(|note| note.shift).collect();
                self.drag = Some((note.id, mode, edit.notes.clone()));
            }
            if dragging && self.drag.as_ref().is_some_and(|(id, _, _)| *id == note.id) {
                self.audition_note = Some(note.id);
            }
            // Shift can enter at any point during a held body drag. Latch fine
            // pitch mode at that position so switching modes never jumps.
            if dragging
                && modifiers.shift
                && !self.drag_fine
                && self
                    .drag
                    .as_ref()
                    .is_some_and(|(id, mode, _)| *id == note.id && *mode == 0)
            {
                self.drag_fine = true;
                // Rebase from the pitch already visible under the pointer.
                // Treat the zero-distance transition frame as active so its
                // recalculation cannot restore the original press snapshot.
                self.drag_pitch_active = true;
                self.drag_origin = pointer;
                self.drag_pitch_base_shifts = edit.notes.iter().map(|note| note.shift).collect();
                self.selected.clear();
                self.selected.insert(note.id);
            }
            if dragging
                && let Some((id, mode, before)) = &self.drag
                && *id == note.id
            {
                let delta =
                    pointer.unwrap_or(rect.center()) - self.drag_origin.unwrap_or(rect.center());
                let mut dt = delta.x as f64 / wave.width() as f64 * (end - start);
                if let Some(grid) = grid {
                    let step = grid * 60.0 / tempo;
                    dt = (dt / step).round() * step;
                }
                let mut changed = before.clone();
                if *mode == 0 {
                    // Selection/clicks never change pitch. Body movement has no
                    // horizontal constraint or time mutation, even between touching notes.
                    self.drag_pitch_active |= delta.y.abs() >= 1.0;
                    if self.drag_pitch_active {
                        let pitch = -delta.y * view.span / wave.height().max(1.0);
                        for (i, note) in before
                            .iter()
                            .enumerate()
                            .filter(|(_, n)| self.selected.contains(&n.id))
                        {
                            let base_shift = self
                                .drag_pitch_base_shifts
                                .get(i)
                                .copied()
                                .unwrap_or(note.shift);
                            changed[i].shift = if self.drag_fine {
                                (base_shift + pitch * FINE_PITCH_SENSITIVITY).clamp(-12.0, 12.0)
                            } else {
                                let center =
                                    (note.detected_midi + base_shift + pitch).round().clamp(
                                        (note.detected_midi - 12.0).ceil(),
                                        (note.detected_midi + 12.0).floor(),
                                    );
                                center - note.detected_midi
                            };
                        }
                    }
                    if self.drag_fine {
                        fine_readout = Some((
                            pointer.unwrap_or(rect.center()),
                            center_pitch_cents(&changed[index]),
                        ));
                    }
                } else if *mode == 1 {
                    resize_note_edge(&mut changed, index, *mode, dt, clip.duration_seconds);
                } else {
                    resize_note_edge(&mut changed, index, *mode, dt, clip.duration_seconds);
                }
                // Publish after iteration to keep hit-testing stable during this frame.
                ui.data_mut(|d| d.insert_temp(ui.id().with("pitch-note-drag-result"), changed));
            }
            if released && captured {
                if double_clicked
                    && let Some((id, mode, _)) = &self.drag
                    && *id == note.id
                {
                    reset_action = Some((index, *mode));
                }
                self.drag = None;
            }
        }
        if interactive
            && pressed
            && tool == EditTool::Select
            && !pressed_note
            && pointer.is_some_and(|pos| wave.contains(pos))
        {
            let additive = modifiers.ctrl;
            let selected_before = self.selected.clone();
            if !additive {
                self.selected.clear();
            }
            self.marquee = Some(MarqueeGesture {
                origin: pointer.unwrap(),
                additive,
                selected_before,
            });
        }
        if let Some(marquee) = self.marquee.clone() {
            let current = pointer.unwrap_or(marquee.origin);
            let rect = Rect::from_two_pos(marquee.origin, current).intersect(wave);
            if down {
                painter.rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgb(108, 169, 225).gamma_multiply(0.16),
                );
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, Color32::from_rgb(157, 202, 246)),
                    egui::StrokeKind::Inside,
                );
            }
            if released {
                let mut selected = if marquee.additive {
                    marquee.selected_before
                } else {
                    BTreeSet::new()
                };
                for note in &edit.notes {
                    if note_screen_rect(note, wave, view, start, end).intersects(rect) {
                        selected.insert(note.id);
                    }
                }
                self.selected = selected;
                self.marquee = None;
            }
        }
        if let Some(changed) =
            ui.data_mut(|d| d.remove_temp::<Vec<PitchNote>>(ui.id().with("pitch-note-drag-result")))
        {
            edit.notes = changed;
        }
        if let Some((index, mode)) = reset_action {
            let success = if mode == 0 {
                reset_note(
                    &mut edit.notes,
                    index,
                    clip.duration_seconds,
                    &edit.analysis,
                )
            } else {
                reset_note_edge(&mut edit.notes, index, mode, clip.duration_seconds)
            };
            if success {
                self.selected = BTreeSet::from([edit.notes[index].id]);
                self.message = if mode == 0 {
                    "Note returned to its analyzed pitch and source timing.".into()
                } else {
                    "Note edge returned to its source boundary.".into()
                };
            } else {
                self.message = "Reset would make the neighboring note too short.".into();
            }
        }
        if let Some((pos, cents)) = fine_readout {
            ui.painter().with_clip_rect(wave).text(
                pos + Vec2::new(12.0, -12.0),
                Align2::LEFT_BOTTOM,
                format!("{cents:+.1} cents"),
                FontId::monospace(11.0),
                Color32::WHITE,
            );
        }
        if let Some((index, t)) = action {
            let mut right = edit.notes[index].clone();
            let cut = right.source_start
                + (t - right.start) / (right.end - right.start)
                    * (right.source_end - right.source_start);
            right.id = edit.notes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
            edit.notes[index].end = t;
            edit.notes[index].source_end = cut;
            right.start = t;
            right.source_start = cut;
            self.selected = BTreeSet::from([right.id]);
            edit.notes.insert(index + 1, right);
            for n in &mut edit.notes[index..=index + 1] {
                if let Some(center) = measured_center(&edit.analysis, n.source_start, n.source_end)
                {
                    n.detected_midi = center;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn touching_note_clicks_are_stable_body_drags_snap_centers_and_shift_drags_fine_tune() {
        let mut clip = clip();
        let middle = &mut clip.pitch_time.as_mut().unwrap().notes[0];
        middle.detected_midi = 69.23;
        let mut previous = middle.clone();
        previous.id = 2;
        previous.start = 0.0;
        previous.source_start = 0.0;
        previous.end = 0.2 + 1.7763568394002505e-15;
        previous.source_end = previous.end;
        let mut next = middle.clone();
        next.id = 3;
        next.start = 0.6;
        next.source_start = 0.6;
        next.end = 1.0;
        next.source_end = 1.0;
        clip.pitch_time.as_mut().unwrap().notes.insert(0, previous);
        clip.pitch_time.as_mut().unwrap().notes.push(next);
        let original = clip.pitch_time.as_ref().unwrap().notes.clone();
        let mut editor = NoteEditor::default();
        editor.sync(&clip);
        let view = PitchViewport {
            center: 69.0,
            span: 24.0,
        };
        let wave = Rect::from_min_max(Pos2::new(200.0, 20.0), Pos2::new(1000.0, 300.0));
        let ctx = egui::Context::default();
        let frame = |events: Vec<egui::Event>, editor: &mut NoteEditor| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 340.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(Rect::from_min_max(Pos2::ZERO, Pos2::new(190.0, 330.0))),
                        |ui| {
                            editor.sidebar(ui, &clip, true);
                        },
                    );
                    editor.draw(
                        ui,
                        wave,
                        &view,
                        0.0,
                        1.0,
                        &clip,
                        None,
                        120.0,
                        EditTool::Select,
                        true,
                        None,
                    );
                },
            )
            .textures_delta
            .clear();
        };
        let button = |pos, pressed, modifiers| {
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                },
            ]
        };
        frame(vec![], &mut editor);
        let press = Pos2::new(500.0, view.y(69.23, wave));
        frame(button(press, true, egui::Modifiers::NONE), &mut editor);
        frame(button(press, false, egui::Modifiers::NONE), &mut editor);
        assert_eq!(
            editor.draft.as_ref().unwrap().notes,
            original,
            "selection must not edit or panic on rounding-sized overlap"
        );
        assert!(editor.publish().is_none());
        frame(button(press, true, egui::Modifiers::NONE), &mut editor);
        let moved = press + Vec2::new(80.0, -wave.height() / view.span);
        frame(vec![egui::Event::PointerMoved(moved)], &mut editor);
        frame(button(moved, false, egui::Modifiers::NONE), &mut editor);
        let coarse = editor.draft.as_ref().unwrap().notes[1].clone();
        assert!(
            (coarse.detected_midi + coarse.shift - 70.0).abs() < 1e-5,
            "snap absolute center, not merely the shift offset"
        );
        assert_eq!((coarse.start, coarse.end), (0.2, 0.6));
        editor.selected = BTreeSet::from([1, 2, 3]);
        let fine_press = Pos2::new(500.0, view.y(70.0, wave));
        frame(
            button(fine_press, true, egui::Modifiers::SHIFT),
            &mut editor,
        );
        let fine_moved = fine_press + Vec2::new(130.0, -0.5 * wave.height() / view.span);
        frame(vec![egui::Event::PointerMoved(fine_moved)], &mut editor);
        frame(
            button(fine_moved, false, egui::Modifiers::SHIFT),
            &mut editor,
        );
        let fine = &editor.draft.as_ref().unwrap().notes[1];
        assert!((fine.detected_midi + fine.shift - 70.125).abs() < 0.001);
        assert!((editor.selected_pitch_cents().unwrap() - 12.5).abs() < 0.1);
        assert_eq!((fine.start, fine.end), (0.2, 0.6));
        assert_eq!(
            editor.selected,
            BTreeSet::from([1]),
            "Shift reserves fine pitch for the clicked note"
        );
        assert_eq!(editor.draft.as_ref().unwrap().notes[0], original[0]);
        assert_eq!(editor.draft.as_ref().unwrap().notes[2], original[2]);
    }

    #[test]
    fn shift_can_be_pressed_after_a_body_drag_starts_without_a_pitch_jump() {
        let mut clip = clip();
        clip.pitch_time.as_mut().unwrap().notes[0].detected_midi = 69.23;
        let mut editor = NoteEditor::default();
        editor.sync(&clip);
        let view = PitchViewport {
            center: 69.0,
            span: 24.0,
        };
        let wave = Rect::from_min_max(Pos2::new(200.0, 20.0), Pos2::new(1000.0, 300.0));
        let ctx = egui::Context::default();
        let mut time = 0.0;
        let mut frame = |events: Vec<egui::Event>, editor: &mut NoteEditor| {
            time += 0.05;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 340.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    editor.draw(
                        ui,
                        wave,
                        &view,
                        0.0,
                        1.0,
                        &clip,
                        None,
                        120.0,
                        EditTool::Select,
                        true,
                        None,
                    )
                },
            );
            output.textures_delta.clear();
            output
        };
        let button = |pos, pressed, modifiers| {
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                },
            ]
        };

        frame(vec![], &mut editor);
        let press = Pos2::new(500.0, view.y(69.23, wave));
        frame(button(press, true, egui::Modifiers::NONE), &mut editor);
        let one_semitone_up = press - Vec2::new(0.0, wave.height() / view.span);
        frame(
            vec![egui::Event::PointerMoved(one_semitone_up)],
            &mut editor,
        );
        let before_shift = editor.draft.as_ref().unwrap().notes[0].clone();
        assert!((before_shift.detected_midi + before_shift.shift - 70.0).abs() < 1e-4);

        frame(
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::SHIFT),
                egui::Event::PointerMoved(one_semitone_up),
            ],
            &mut editor,
        );
        let at_shift_transition = editor.draft.as_ref().unwrap().notes[0].clone();
        assert_eq!(
            at_shift_transition.shift, before_shift.shift,
            "entering fine mode must retain the current pitch"
        );

        let half_semitone_row = one_semitone_up - Vec2::new(0.0, 0.5 * wave.height() / view.span);
        frame(
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::SHIFT),
                egui::Event::PointerMoved(half_semitone_row),
            ],
            &mut editor,
        );
        frame(
            button(half_semitone_row, false, egui::Modifiers::SHIFT),
            &mut editor,
        );
        let final_note = &editor.draft.as_ref().unwrap().notes[0];
        assert!((final_note.detected_midi + final_note.shift - 70.125).abs() < 0.001);
        assert!((editor.selected_pitch_cents().unwrap() - 12.5).abs() < 0.1);
    }

    #[test]
    fn double_clicking_note_body_resets_note_and_double_clicking_edge_resets_only_edge() {
        let view = PitchViewport {
            center: 69.0,
            span: 24.0,
        };
        let wave = Rect::from_min_max(Pos2::new(200.0, 20.0), Pos2::new(1000.0, 300.0));
        let ctx = egui::Context::default();
        let mut time = 0.0;
        let mut editor = NoteEditor::default();
        let mut clip = clip();
        {
            let note = &mut clip.pitch_time.as_mut().unwrap().notes[0];
            note.start = 0.25;
            note.end = 0.55;
            note.shift = 1.0;
            note.straighten = 0.4;
        }
        let body_before = clip.pitch_time.clone().unwrap();
        editor.sync(&clip);
        let mut frame = |events: Vec<egui::Event>, editor: &mut NoteEditor, clip: &AudioClip| {
            time += 0.05;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 340.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    editor.draw(
                        ui,
                        wave,
                        &view,
                        0.0,
                        1.0,
                        clip,
                        None,
                        120.0,
                        EditTool::Select,
                        true,
                        None,
                    )
                },
            );
            output.textures_delta.clear();
            output
        };
        let click = |pos, pressed| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let body = Pos2::new(520.0, view.y(70.0, wave));
        frame(vec![], &mut editor, &clip);
        frame(click(body, true), &mut editor, &clip);
        frame(click(body, false), &mut editor, &clip);
        frame(click(body, true), &mut editor, &clip);
        frame(click(body, false), &mut editor, &clip);
        let body_reset = &editor.draft.as_ref().unwrap().notes[0];
        assert_eq!((body_reset.start, body_reset.end), (0.2, 0.6));
        assert_eq!((body_reset.shift, body_reset.straighten), (0.0, 0.0));
        let body_published = editor.publish().expect("reset publishes for Undo/save");
        assert!(
            editor.publish().is_none(),
            "the reset settles after publication"
        );
        let mut body_project = Project::default();
        body_project.add_track(TrackKind::Audio);
        body_project.add_audio_clip(0, clip.source_path.clone(), 0.0, 1.0);
        body_project.tracks[0].clips[0].pitch_time = Some(body_before.clone());
        let mut body_history = super::super::super::ProjectHistory::default();
        body_history.begin(&body_project);
        body_project.tracks[0].clips[0].pitch_time = Some(body_published);
        assert!(body_history.finish_frame(&body_project, true));
        let undone_body = body_history.undo(&body_project).unwrap();
        assert_eq!(
            undone_body.tracks[0].clips[0].pitch_time,
            Some(body_before),
            "one Undo restores the full body pitch/timing edit"
        );

        let mut edge_clip = clip.clone();
        {
            let note = &mut edge_clip.pitch_time.as_mut().unwrap().notes[0];
            note.start = 0.25;
            note.end = 0.55;
            note.shift = 1.0;
            note.straighten = 0.4;
        }
        let edge_before = edge_clip.pitch_time.clone().unwrap();
        editor = NoteEditor::default();
        editor.sync(&edge_clip);
        for _ in 0..8 {
            frame(vec![], &mut editor, &edge_clip);
        }
        let left_edge = Pos2::new(wave.left() + 0.25 * wave.width() + 2.0, view.y(70.0, wave));
        frame(vec![], &mut editor, &edge_clip);
        frame(click(left_edge, true), &mut editor, &edge_clip);
        frame(click(left_edge, false), &mut editor, &edge_clip);
        frame(click(left_edge, true), &mut editor, &edge_clip);
        frame(click(left_edge, false), &mut editor, &edge_clip);
        let edge_reset = &editor.draft.as_ref().unwrap().notes[0];
        assert_eq!(edge_reset.start, 0.2);
        assert_eq!(edge_reset.end, 0.55, "the opposite edge is untouched");
        assert_eq!(edge_reset.shift, 1.0, "edge reset does not alter pitch");
        assert_eq!(
            edge_reset.straighten, 0.4,
            "edge reset does not alter straighten"
        );
        let edge_published = editor
            .publish()
            .expect("edge reset publishes for Undo/save");
        let mut edge_project = Project::default();
        edge_project.add_track(TrackKind::Audio);
        edge_project.add_audio_clip(0, edge_clip.source_path.clone(), 0.0, 1.0);
        edge_project.tracks[0].clips[0].pitch_time = Some(edge_before.clone());
        let mut edge_history = super::super::super::ProjectHistory::default();
        edge_history.begin(&edge_project);
        edge_project.tracks[0].clips[0].pitch_time = Some(edge_published);
        assert!(edge_history.finish_frame(&edge_project, true));
        let undone_edge = edge_history.undo(&edge_project).unwrap();
        assert_eq!(
            undone_edge.tracks[0].clips[0].pitch_time,
            Some(edge_before),
            "one Undo restores the full edge timing edit"
        );
    }

    #[test]
    fn stretching_an_edge_pushes_only_the_neighbors_near_edge_and_keeps_minimum_lengths() {
        let mut notes = vec![
            PitchNote {
                id: 10,
                source_start: 0.0,
                source_end: 0.3,
                start: 0.0,
                end: 0.3,
                detected_midi: 60.0,
                shift: 0.0,
                straighten: 0.0,
            },
            PitchNote {
                id: 11,
                source_start: 0.3,
                source_end: 0.6,
                start: 0.3,
                end: 0.6,
                detected_midi: 62.0,
                shift: 0.0,
                straighten: 0.0,
            },
            PitchNote {
                id: 12,
                source_start: 0.6,
                source_end: 1.0,
                start: 0.6,
                end: 1.0,
                detected_midi: 64.0,
                shift: 0.0,
                straighten: 0.0,
            },
        ];
        let previous_far_edge = notes[0].start;
        let next_far_edge = notes[2].end;

        resize_note_edge(&mut notes, 1, 1, -0.2, 1.0);
        assert!((notes[1].start - 0.1).abs() < 1e-9);
        assert!((notes[0].end - 0.1).abs() < 1e-9);
        assert_eq!(notes[0].start, previous_far_edge);
        assert_eq!(notes[1].end, 0.6);

        resize_note_edge(&mut notes, 1, 2, 0.3, 1.0);
        assert!((notes[1].end - 0.9).abs() < 1e-9);
        assert!((notes[2].start - 0.9).abs() < 1e-9);
        assert_eq!(notes[2].end, next_far_edge);
        assert!(
            notes
                .iter()
                .all(|note| note.end - note.start >= MIN_NOTE_LENGTH_SECONDS)
        );

        // The previous and next note each reach their minimum duration and
        // keep their opposite edge fixed when the edited edge is pushed again.
        resize_note_edge(&mut notes, 1, 1, -1.0, 1.0);
        assert!((notes[0].end - notes[0].start - MIN_NOTE_LENGTH_SECONDS).abs() < 1e-9);
        assert!((notes[1].start - (notes[0].start + MIN_NOTE_LENGTH_SECONDS)).abs() < 1e-9);
        resize_note_edge(&mut notes, 1, 2, 1.0, 1.0);
        assert!((notes[2].end - notes[2].start - MIN_NOTE_LENGTH_SECONDS).abs() < 1e-9);
        assert!((notes[1].end - (notes[2].end - MIN_NOTE_LENGTH_SECONDS)).abs() < 1e-9);
    }

    #[test]
    fn moving_an_attached_edge_away_stretches_the_neighbor_and_gaps_stay_open() {
        let make_notes = || {
            vec![
                PitchNote {
                    id: 10,
                    source_start: 0.0,
                    source_end: 0.3,
                    start: 0.0,
                    end: 0.3,
                    detected_midi: 60.0,
                    shift: 0.0,
                    straighten: 0.0,
                },
                PitchNote {
                    id: 11,
                    source_start: 0.3,
                    source_end: 0.6,
                    start: 0.3,
                    end: 0.6,
                    detected_midi: 62.0,
                    shift: 0.0,
                    straighten: 0.0,
                },
                PitchNote {
                    id: 12,
                    source_start: 0.6,
                    source_end: 1.0,
                    start: 0.6,
                    end: 1.0,
                    detected_midi: 64.0,
                    shift: 0.0,
                    straighten: 0.0,
                },
            ]
        };

        let mut notes = make_notes();
        resize_note_edge(&mut notes, 1, 1, 0.1, 1.0);
        assert_eq!((notes[1].start, notes[0].end), (0.4, 0.4));
        assert_eq!((notes[0].start, notes[1].end), (0.0, 0.6));

        let mut notes = make_notes();
        resize_note_edge(&mut notes, 1, 2, -0.1, 1.0);
        assert_eq!((notes[1].end, notes[2].start), (0.5, 0.5));
        assert_eq!((notes[2].end, notes[1].start), (1.0, 0.3));

        let mut notes = make_notes();
        notes[0].end = 0.2;
        notes[1].start = 0.3;
        resize_note_edge(&mut notes, 1, 1, -0.05, 1.0);
        assert!((notes[1].start - 0.25).abs() < 1e-9);
        assert_eq!(notes[0].end, 0.2, "a pre-existing gap is not stretched");
    }

    #[test]
    fn pointer_edge_stretch_pushes_the_adjacent_region_without_moving_its_far_edge() {
        let mut clip = clip();
        let mut middle = clip.pitch_time.as_ref().unwrap().notes[0].clone();
        let mut previous = middle.clone();
        previous.id = 2;
        previous.source_start = 0.0;
        previous.source_end = 0.2;
        previous.start = 0.0;
        previous.end = 0.2;
        previous.detected_midi = 65.0;
        middle.id = 1;
        let mut next = middle.clone();
        next.id = 3;
        next.source_start = 0.6;
        next.source_end = 1.0;
        next.start = 0.6;
        next.end = 1.0;
        next.detected_midi = 73.0;
        let notes = &mut clip.pitch_time.as_mut().unwrap().notes;
        notes.clear();
        notes.extend([previous, middle, next]);

        let mut editor = NoteEditor::default();
        editor.sync(&clip);
        let view = PitchViewport {
            center: 69.0,
            span: 24.0,
        };
        let wave = Rect::from_min_max(Pos2::new(200.0, 20.0), Pos2::new(1000.0, 300.0));
        let ctx = egui::Context::default();
        let frame = |events: Vec<egui::Event>, editor: &mut NoteEditor| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 340.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    editor.draw(
                        ui,
                        wave,
                        &view,
                        0.0,
                        1.0,
                        &clip,
                        None,
                        120.0,
                        EditTool::Select,
                        true,
                        None,
                    )
                },
            );
            output.textures_delta.clear();
            output
        };
        let button = |pos, pressed| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        frame(vec![], &mut editor);
        let y = view.y(69.0, wave);
        let left = Pos2::new(361.0, y);
        let moved_left = left - Vec2::new(80.0, 0.0);
        frame(button(left, true), &mut editor);
        frame(vec![egui::Event::PointerMoved(moved_left)], &mut editor);
        frame(button(moved_left, false), &mut editor);
        let notes = &editor.draft.as_ref().unwrap().notes;
        assert!((notes[1].start - 0.1).abs() < 1e-6);
        assert!((notes[0].end - 0.1).abs() < 1e-6);
        assert_eq!(notes[0].start, 0.0);
        assert_eq!(notes[1].end, 0.6);

        let right = Pos2::new(679.0, y);
        let moved_right = right + Vec2::new(160.0, 0.0);
        frame(button(right, true), &mut editor);
        frame(vec![egui::Event::PointerMoved(moved_right)], &mut editor);
        frame(button(moved_right, false), &mut editor);
        let notes = &editor.draft.as_ref().unwrap().notes;
        assert!((notes[1].end - 0.8).abs() < 1e-6);
        assert!((notes[2].start - 0.8).abs() < 1e-6);
        assert_eq!(notes[2].end, 1.0);
        assert!(editor.draft.as_ref().unwrap().validate(1.0));
    }

    #[test]
    fn empty_grid_marquee_selects_notes_without_dirtying_and_selected_reset_is_one_undo() {
        let mut clip = clip();
        let mut second = clip.pitch_time.as_ref().unwrap().notes[0].clone();
        second.id = 2;
        second.source_start = 0.65;
        second.source_end = 0.9;
        second.start = 0.65;
        second.end = 0.9;
        second.detected_midi = 72.0;
        clip.pitch_time.as_mut().unwrap().notes.push(second);
        let mut editor = NoteEditor::default();
        editor.sync(&clip);
        let view = PitchViewport {
            center: 69.0,
            span: 24.0,
        };
        let wave = Rect::from_min_max(Pos2::new(200.0, 20.0), Pos2::new(1000.0, 300.0));
        let ctx = egui::Context::default();
        let frame = |events: Vec<egui::Event>, editor: &mut NoteEditor| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 340.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    editor.draw(
                        ui,
                        wave,
                        &view,
                        0.0,
                        1.0,
                        &clip,
                        None,
                        120.0,
                        EditTool::Select,
                        true,
                        None,
                    )
                },
            );
            output.textures_delta.clear();
            output
        };
        let button = |pos, pressed, modifiers| {
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                },
            ]
        };
        frame(vec![], &mut editor);
        let origin = Pos2::new(205.0, 100.0);
        let end = Pos2::new(940.0, 180.0);
        frame(button(origin, true, egui::Modifiers::NONE), &mut editor);
        let marquee_frame = frame(vec![egui::Event::PointerMoved(end)], &mut editor);
        assert!(marquee_frame.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Rect(rect)
                if rect.rect.width() > 100.0 && rect.rect.height() > 20.0
        )));
        frame(button(end, false, egui::Modifiers::NONE), &mut editor);
        assert_eq!(editor.selected, BTreeSet::from([1, 2]));
        assert!(
            editor.publish().is_none(),
            "selection is transient, not a project edit"
        );

        let mut reset_clip = clip.clone();
        reset_clip.pitch_time.as_mut().unwrap().notes[1].start = 0.5;
        reset_clip.pitch_time.as_mut().unwrap().notes[1].end = 0.72;
        let mut third = reset_clip.pitch_time.as_ref().unwrap().notes[1].clone();
        third.id = 3;
        third.source_start = 0.9;
        third.source_end = 1.0;
        third.start = 0.8;
        third.end = 1.0;
        third.detected_midi = 66.0;
        reset_clip.pitch_time.as_mut().unwrap().notes.push(third);
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_audio_clip(0, reset_clip.source_path.clone(), 0.0, 1.0);
        {
            let edited = project.tracks[0].clips[0]
                .pitch_time
                .get_or_insert_with(|| reset_clip.pitch_time.as_ref().unwrap().clone());
            edited.notes[0].shift = 1.0;
            edited.notes[0].straighten = 0.4;
            edited.notes[0].start = 0.1;
            edited.notes[0].end = 0.5;
            edited.notes[1].shift = -0.5;
            edited.notes[1].straighten = 0.2;
        }
        let saved_analysis = project.tracks[0].clips[0]
            .pitch_time
            .as_ref()
            .unwrap()
            .analysis
            .clone();
        let mut history = super::super::super::ProjectHistory::default();
        history.begin(&project);
        let selection = BTreeSet::from([1, 2]);
        assert!(reset_selected_notes(
            &mut project.tracks[0].clips[0]
                .pitch_time
                .as_mut()
                .unwrap()
                .notes,
            &selection,
            1.0
        ));
        assert!(history.finish_frame(&project, true));
        assert_eq!(
            project.tracks[0].clips[0]
                .pitch_time
                .as_ref()
                .unwrap()
                .analysis,
            saved_analysis
        );
        let reset = &project.tracks[0].clips[0]
            .pitch_time
            .as_ref()
            .unwrap()
            .notes;
        assert!(reset[..2].iter().all(|note| {
            note.shift == 0.0
                && note.straighten == 0.0
                && note.start == note.source_start
                && note.end == note.source_end
        }));
        assert_eq!(
            reset[2].start, 0.9,
            "only the neighbor's near edge is pushed"
        );
        assert_eq!(reset[2].end, 1.0, "the neighbor's far edge stays fixed");
        let undone = history.undo(&project).unwrap();
        assert_eq!(
            undone.tracks[0].clips[0].pitch_time.as_ref().unwrap().notes[0].shift,
            1.0
        );
        assert_eq!(
            undone.tracks[0].clips[0].pitch_time.as_ref().unwrap().notes[1].shift,
            -0.5
        );
        assert_eq!(
            undone.tracks[0].clips[0].pitch_time.as_ref().unwrap().notes[2].start,
            0.8
        );
    }

    #[test]
    fn inverted_time_bounds_disable_the_field_and_preserve_the_existing_edge() {
        assert_eq!(bounded_time(0.0, 1.7763568394002505e-15, 0.0, 0.0), 0.0);
        assert_eq!(bounded_time(0.3, 0.4, 0.4, 0.2), 0.4);
        let mut value = 0.0_f64;
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            note_value(ui, "Time", &mut value, 1.7763568394002505e-15..=0.0, 0.001)
        })
        .textures_delta
        .clear();
        assert_eq!(value, 0.0);
    }

    #[test]
    fn initial_analysis_is_retained_by_undo_while_each_note_gesture_is_reversible() {
        let analyzed = clip();
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_audio_clip(0, analyzed.source_path.clone(), 0.0, 1.0);
        let mut history = super::super::super::ProjectHistory::default();
        history.begin(&project);
        project.tracks[0].clips[0].pitch_time = analyzed.pitch_time.clone();
        history.retain_analysis(project.tracks[0].id, &project.tracks[0].clips[0]);
        assert!(!history.finish_frame(&project, true));
        assert!(!history.can_undo());
        history.begin(&project);
        project.tracks[0].clips[0]
            .pitch_time
            .as_mut()
            .unwrap()
            .notes[0]
            .shift = 2.0;
        assert!(history.finish_frame(&project, true));
        let undone = history.undo(&project).unwrap();
        assert_eq!(undone.tracks[0].clips[0].pitch_time, analyzed.pitch_time);
    }
    fn clip() -> AudioClip {
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        p.add_audio_clip(0, PathBuf::from("source.wav"), 0.0, 1.0);
        let mut clip = p.tracks[0].clips[0].clone();
        clip.pitch_time = Some(PitchTimeEdit {
            detector_version: opentrack_core::pitch_time::DETECTOR_VERSION,
            source_bytes: 0,
            source_modified_ns: 0,
            analysis: opentrack_audio::PitchAnalysis {
                sample_rate_hz: 16000,
                source_start_seconds: 0.0,
                source_duration_seconds: 1.0,
                points: Vec::new(),
            },
            notes: vec![PitchNote {
                id: 1,
                source_start: 0.2,
                source_end: 0.6,
                start: 0.2,
                end: 0.6,
                detected_midi: 69.0,
                shift: 0.0,
                straighten: 0.0,
            }],
            rendered_source: None,
            enabled: true,
        });
        clip
    }
    fn color_note(detected_midi: f32, shift: f32) -> PitchNote {
        PitchNote {
            id: 1,
            source_start: 0.0,
            source_end: 1.0,
            start: 0.0,
            end: 1.0,
            detected_midi,
            shift,
            straighten: 0.0,
        }
    }

    #[test]
    fn note_palette_follows_the_current_rounded_center_for_semitone_and_fine_edits() {
        let mut note = color_note(69.0, 0.0);
        assert_eq!(note_color_index(&note), 1);
        assert_eq!(note_color(&note), NOTE_COLORS[1]);

        note.shift = 2.0;
        assert_eq!(note_color_index(&note), 3);
        assert_ne!(note_color(&note), NOTE_COLORS[1]);

        note.shift = -2.0;
        assert_eq!(note_color_index(&note), 3);

        note.shift = 0.49;
        assert_eq!(note_color_index(&note), 1, "69.49 rounds down to row 69");
        note.shift = 0.51;
        assert_eq!(note_color_index(&note), 2, "69.51 rounds up to row 70");

        let mut invalid = color_note(f32::NAN, 0.0);
        assert_eq!(note_color_index(&invalid), 0);
        invalid = color_note(-20.0, -20.0);
        assert_eq!(note_color_index(&invalid), 0);
        invalid = color_note(200.0, 0.0);
        assert_eq!(note_color_index(&invalid), 127 % NOTE_COLORS.len());
    }

    #[test]
    fn time_only_edge_stretches_leave_note_colors_unchanged() {
        let mut notes = vec![
            color_note(60.0, 0.0),
            color_note(62.0, 0.0),
            color_note(64.0, 0.0),
        ];
        for (index, note) in notes.iter_mut().enumerate() {
            note.id = index as u64 + 10;
            note.source_start = index as f64 * 0.3;
            note.source_end = (index + 1) as f64 * 0.3;
            note.start = note.source_start;
            note.end = note.source_end;
        }
        notes[2].source_end = 1.0;
        notes[2].end = 1.0;
        let colors: Vec<_> = notes.iter().map(note_color_index).collect();
        let timing = (notes[1].start, notes[1].end);

        resize_note_edge(&mut notes, 1, 1, -0.2, 1.0);
        resize_note_edge(&mut notes, 1, 2, 0.3, 1.0);

        assert_ne!((notes[1].start, notes[1].end), timing);
        assert_eq!(
            notes.iter().map(note_color_index).collect::<Vec<_>>(),
            colors,
            "resizing source time uses must not recolor notes"
        );
    }

    #[test]
    fn reset_and_undo_restore_the_original_note_color() {
        let mut notes = vec![color_note(69.0, 2.0)];
        assert_eq!(note_color_index(&notes[0]), 3);
        assert!(reset_selected_notes(&mut notes, &BTreeSet::from([1]), 1.0));
        assert_eq!(note_color_index(&notes[0]), 1);

        let original = clip();
        let original_color = note_color_index(&original.pitch_time.as_ref().unwrap().notes[0]);
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_audio_clip(0, original.source_path.clone(), 0.0, 1.0);
        project.tracks[0].clips[0].pitch_time = original.pitch_time.clone();
        let mut history = super::super::super::ProjectHistory::default();
        history.begin(&project);
        project.tracks[0].clips[0]
            .pitch_time
            .as_mut()
            .unwrap()
            .notes[0]
            .shift = 2.0;
        assert_eq!(
            note_color_index(
                &project.tracks[0].clips[0]
                    .pitch_time
                    .as_ref()
                    .unwrap()
                    .notes[0]
            ),
            3
        );
        assert!(history.finish_frame(&project, true));
        let undone = history.undo(&project).unwrap();
        assert_eq!(
            note_color_index(&undone.tracks[0].clips[0].pitch_time.as_ref().unwrap().notes[0]),
            original_color,
            "Undo restores the analyzed center and therefore its palette color"
        );
    }

    #[test]
    fn multi_selected_drag_keeps_each_notes_own_center_color() {
        let mut clip = clip();
        let mut second = clip.pitch_time.as_ref().unwrap().notes[0].clone();
        second.id = 2;
        second.source_start = 0.65;
        second.source_end = 0.9;
        second.start = 0.65;
        second.end = 0.9;
        second.detected_midi = 72.0;
        clip.pitch_time.as_mut().unwrap().notes.push(second);
        let mut editor = NoteEditor::default();
        editor.sync(&clip);
        let view = PitchViewport {
            center: 69.0,
            span: 24.0,
        };
        let wave = Rect::from_min_max(Pos2::new(200.0, 20.0), Pos2::new(1000.0, 300.0));
        let ctx = egui::Context::default();
        let frame = |events: Vec<egui::Event>, editor: &mut NoteEditor| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 340.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    editor.draw(
                        ui,
                        wave,
                        &view,
                        0.0,
                        1.0,
                        &clip,
                        None,
                        120.0,
                        EditTool::Select,
                        true,
                        None,
                    )
                },
            );
            output.textures_delta.clear();
        };
        let button = |pos, pressed, modifiers| {
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                },
            ]
        };
        frame(vec![], &mut editor);
        let first = Pos2::new(500.0, view.y(69.0, wave));
        let second = Pos2::new(820.0, view.y(72.0, wave));
        frame(button(first, true, egui::Modifiers::NONE), &mut editor);
        frame(button(first, false, egui::Modifiers::NONE), &mut editor);
        frame(button(second, true, egui::Modifiers::CTRL), &mut editor);
        frame(button(second, false, egui::Modifiers::CTRL), &mut editor);
        assert_eq!(editor.selected, BTreeSet::from([1, 2]));

        frame(button(first, true, egui::Modifiers::NONE), &mut editor);
        let moved = first - Vec2::new(0.0, 2.0 * wave.height() / view.span);
        frame(vec![egui::Event::PointerMoved(moved)], &mut editor);
        frame(button(moved, false, egui::Modifiers::NONE), &mut editor);

        let notes = &editor.draft.as_ref().unwrap().notes;
        assert!((notes[0].detected_midi + notes[0].shift - 71.0).abs() < 1e-4);
        assert!((notes[1].detected_midi + notes[1].shift - 74.0).abs() < 1e-4);
        assert_eq!(note_color_index(&notes[0]), 3);
        assert_eq!(note_color_index(&notes[1]), 2);
        assert_ne!(
            note_color_index(&notes[0]),
            note_color_index(&notes[1]),
            "multi-selected notes use their own centers, not one shared color"
        );
    }

    #[test]
    fn selected_notes_use_the_supplied_track_color_and_a_default_without_one() {
        let clip = clip();
        let mut editor = NoteEditor::default();
        editor.sync(&clip);
        editor.selected = BTreeSet::from([1]);
        let view = PitchViewport {
            center: 69.0,
            span: 24.0,
        };
        let wave = Rect::from_min_max(Pos2::new(200.0, 20.0), Pos2::new(1000.0, 300.0));
        let ctx = egui::Context::default();
        let frame = |track_color: Option<Color32>, editor: &mut NoteEditor| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 340.0))),
                    ..Default::default()
                },
                |ui| {
                    editor.draw(
                        ui,
                        wave,
                        &view,
                        0.0,
                        1.0,
                        &clip,
                        None,
                        120.0,
                        EditTool::Select,
                        true,
                        track_color,
                    )
                },
            );
            output.textures_delta.clear();
            output
        };
        let track_color = Color32::from_rgb(12, 34, 56);
        let output = frame(Some(track_color), &mut editor);
        let expected_fill = track_color.gamma_multiply(0.20);
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.fill == expected_fill
            )),
            "the selected note fill uses the track color emphasis"
        );
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.stroke.color == track_color
            )),
            "the selected note stroke uses the track color"
        );
        assert!(
            output.shapes.iter().all(|shape| !matches!(
                &shape.shape,
                egui::Shape::Rect(rect)
                    if rect.stroke.color == Color32::from_rgb(240, 174, 91)
            )),
            "selected notes must not fall back to the old orange highlight"
        );

        let output = frame(None, &mut editor);
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.stroke.color == DEFAULT_SELECTED_NOTE_COLOR
            )),
            "a draw call without a track color keeps the default selection highlight"
        );

        editor.selected.clear();
        let expected = note_color(&clip.pitch_time.as_ref().unwrap().notes[0]);
        let output = frame(None, &mut editor);
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.stroke.color == expected
            )),
            "unselected notes use the palette color derived from their current center"
        );
    }

    #[test]
    fn actual_note_drag_resizes_and_splits_editable_regions_without_touching_original_audio() {
        let clip = clip();
        let mut state = NoteEditor::default();
        state.sync(&clip);
        let ctx = egui::Context::default();
        let wave = Rect::from_min_max(Pos2::new(200.0, 20.0), Pos2::new(1000.0, 300.0));
        let view = PitchViewport {
            center: 69.0,
            span: 24.0,
        };
        let frame = |events: Vec<egui::Event>, tool: EditTool, state: &mut NoteEditor| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 340.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    state.draw(
                        ui, wave, &view, 0.0, 1.0, &clip, None, 120.0, tool, true, None,
                    )
                },
            )
            .textures_delta
            .clear();
        };
        let button = |pos, pressed| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        frame(vec![], EditTool::Select, &mut state);
        frame(
            button(Pos2::new(500.0, 160.0), true),
            EditTool::Select,
            &mut state,
        );
        frame(
            vec![egui::Event::PointerMoved(Pos2::new(580.0, 148.0))],
            EditTool::Select,
            &mut state,
        );
        frame(
            button(Pos2::new(580.0, 148.0), false),
            EditTool::Select,
            &mut state,
        );
        let note = &state.draft.as_ref().unwrap().notes[0];
        assert!((note.start - 0.2).abs() < 1e-5 && (note.end - 0.6).abs() < 1e-5);
        assert_eq!(note.shift, 1.0);
        assert_eq!(note.source_start, 0.2);
        assert_eq!(note.source_end, 0.6);
        frame(vec![], EditTool::Select, &mut state);
        frame(
            button(Pos2::new(679.0, 148.0), true),
            EditTool::Select,
            &mut state,
        );
        frame(
            vec![egui::Event::PointerMoved(Pos2::new(719.0, 148.0))],
            EditTool::Select,
            &mut state,
        );
        frame(
            button(Pos2::new(719.0, 148.0), false),
            EditTool::Select,
            &mut state,
        );
        assert!((state.draft.as_ref().unwrap().notes[0].end - 0.65).abs() < 1e-5);
        frame(vec![], EditTool::Split, &mut state);
        frame(
            button(Pos2::new(600.0, 148.0), true),
            EditTool::Split,
            &mut state,
        );
        frame(
            button(Pos2::new(600.0, 148.0), false),
            EditTool::Split,
            &mut state,
        );
        let notes = &state.draft.as_ref().unwrap().notes;
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].end, notes[1].start);
        assert_eq!(notes[0].source_end, notes[1].source_start);
        assert_eq!(
            note_color_index(&notes[0]),
            note_color_index(&notes[1]),
            "split regions retain the same applicable center color"
        );
        assert!(state.draft.as_ref().unwrap().validate(1.0));
        assert_eq!(clip.pitch_time.as_ref().unwrap().notes.len(), 1);
    }

    #[test]
    fn committed_note_edits_round_trip_and_undo_redo_without_changing_source_identity() {
        let original = clip();
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_audio_clip(0, original.source_path.clone(), 0.0, 1.0);
        project.tracks[0].clips[0] = original.clone();
        let mut history = super::super::super::ProjectHistory::default();
        history.begin(&project);
        let edit = project.tracks[0].clips[0].pitch_time.as_mut().unwrap();
        edit.notes[0].shift = 3.0;
        edit.notes[0].end = 0.8;
        edit.rendered_source = Some(PathBuf::from("Metadata/PitchTime/derived.wav"));
        assert!(history.finish_frame(&project, true));
        let saved: Project =
            serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
        assert_eq!(saved, project);
        saved.validate().unwrap();
        let undone = history.undo(&project).unwrap();
        assert_eq!(undone.tracks[0].clips[0], original);
        assert_eq!(history.redo(&undone).unwrap(), saved);
    }
}

fn measured_center(analysis: &opentrack_audio::PitchAnalysis, start: f64, end: f64) -> Option<f32> {
    let mut pitches: Vec<_> = analysis
        .points
        .iter()
        .filter(|p| {
            let time = p.source_time_seconds - analysis.source_start_seconds;
            time >= start && time <= end && p.confidence >= 0.75
        })
        .filter_map(|p| p.frequency_hz)
        .map(|f| 69.0 + 12.0 * (f / 440.0).log2())
        .collect();
    pitches.sort_by(f32::total_cmp);
    pitches.get(pitches.len() / 2).copied()
}

fn note_value<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    speed: f64,
) {
    let (min, max) = range.into_inner();
    let valid = min.to_f64().is_finite() && max.to_f64().is_finite() && min <= max;
    let range = if valid { min..=max } else { *value..=*value };
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add_enabled_ui(valid, |ui| {
            ui.add_sized(
                [51.0, 18.0],
                egui::DragValue::new(value)
                    .range(range)
                    .clamp_existing_to_range(false)
                    .speed(speed),
            );
        });
    });
}

fn bounded_time(request: f64, min: f64, max: f64, previous: f64) -> f64 {
    if request.is_finite() && min.is_finite() && max.is_finite() && min <= max {
        request.clamp(min, max)
    } else {
        previous
    }
}
