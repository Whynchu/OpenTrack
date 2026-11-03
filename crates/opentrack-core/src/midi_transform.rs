// SPDX-License-Identifier: Apache-2.0

//! Pitch and velocity edits for MIDI notes.
//!
//! Two bounded operations over a selection (or a whole part): transpose by whole
//! semitones, and offset velocity. Transpose never invents a pitch outside the
//! MIDI range — a note that would leave 0..=127 is left where it is rather than
//! clamped onto an octave the user did not ask for. Velocity is clamped, because a
//! softer or louder version of the same note is still the same note.

use crate::project::MidiPart;

/// Largest transposition allowed in one command.
pub const MAX_TRANSPOSE_SEMITONES: i16 = 48;
/// Largest velocity change allowed in one command.
pub const MAX_VELOCITY_DELTA: i16 = 127;

fn says_yes(ids: &[u64], id: u64) -> bool {
    ids.is_empty() || ids.contains(&id)
}

/// Transposes the selected notes and reports how many moved.
///
/// Notes that would leave the MIDI pitch range are skipped, and the caller can
/// report the difference between the requested and the changed count.
/// Toggles the score `hidden` flag on the given notes.
pub fn toggle_hidden(part: &mut MidiPart, ids: &[u64]) -> usize {
    let mut changed = 0;
    for note in part.notes.iter_mut() {
        if ids.contains(&note.id) {
            note.hidden = !note.hidden;
            changed += 1;
        }
    }
    changed
}

/// Toggles the score `grace` flag on the given notes.
pub fn toggle_grace(part: &mut MidiPart, ids: &[u64]) -> usize {
    let mut changed = 0;
    for note in part.notes.iter_mut() {
        if ids.contains(&note.id) {
            note.grace = !note.grace;
            changed += 1;
        }
    }
    changed
}

/// Toggles the score stem `flipped` flag on the given notes.
pub fn toggle_flipped(part: &mut MidiPart, ids: &[u64]) -> usize {
    let mut changed = 0;
    for note in part.notes.iter_mut() {
        if ids.contains(&note.id) {
            note.flipped = !note.flipped;
            changed += 1;
        }
    }
    changed
}

pub fn transpose_notes(part: &mut MidiPart, ids: &[u64], semitones: i16) -> usize {
    if semitones == 0 || semitones.abs() > MAX_TRANSPOSE_SEMITONES {
        return 0;
    }
    let mut changed = 0;
    for note in part.notes.iter_mut() {
        if !says_yes(ids, note.id) {
            continue;
        }
        let shifted = i16::from(note.pitch) + semitones;
        if !(0..=127).contains(&shifted) {
            continue;
        }
        note.pitch = shifted as u8;
        changed += 1;
    }
    changed
}

/// Offsets the selected notes' velocity and reports how many changed.
pub fn change_velocity(part: &mut MidiPart, ids: &[u64], delta: i16) -> usize {
    if delta == 0 || delta.abs() > MAX_VELOCITY_DELTA {
        return 0;
    }
    let mut changed = 0;
    for note in part.notes.iter_mut() {
        if !says_yes(ids, note.id) {
            continue;
        }
        let shifted = (i16::from(note.velocity) + delta).clamp(1, 127) as u8;
        if shifted != note.velocity {
            note.velocity = shifted;
            changed += 1;
        }
    }
    changed
}

/// Sets the selected notes' velocity to a fixed value and reports changes.
pub fn set_velocity(part: &mut MidiPart, ids: &[u64], velocity: u8) -> usize {
    let target = velocity.clamp(1, 127);
    let mut changed = 0;
    for note in part.notes.iter_mut() {
        if says_yes(ids, note.id) && note.velocity != target {
            note.velocity = target;
            changed += 1;
        }
    }
    changed
}

/// Rises or falls the selected notes' velocity across their time order.
///
/// The first selected note takes `start`, the last takes `end`, and the notes
/// between are spaced evenly by position, so a crescendo follows the performance
/// rather than the note order in the file.
pub fn ramp_velocity(part: &mut MidiPart, ids: &[u64], start: u8, end: u8) -> usize {
    let start = start.clamp(1, 127);
    let end = end.clamp(1, 127);
    let mut selected = part
        .notes
        .iter()
        .enumerate()
        .filter(|(_, note)| says_yes(ids, note.id))
        .map(|(index, note)| (note.start_beats, index))
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
    let count = selected.len();
    if count == 0 {
        return 0;
    }
    let mut changed = 0;
    for (rank, (_, index)) in selected.into_iter().enumerate() {
        let value = if count <= 1 {
            start
        } else {
            let fraction = rank as f64 / (count - 1) as f64;
            (f64::from(start) + (f64::from(end) - f64::from(start)) * fraction).round() as u8
        }
        .clamp(1, 127);
        if part.notes[index].velocity != value {
            part.notes[index].velocity = value;
            changed += 1;
        }
    }
    changed
}

/// Builds an evenly spaced tuplet from the selected notes.
///
/// The user picks the note count and the span; every selected note is placed on
/// its own slot from the earliest selected note and gets that slot's length. The
/// span is clamped to the part so the last note never crosses its end, and a
/// selection whose size does not match the requested count is refused rather
/// than silently dropping or adding notes.
pub fn build_tuplet(
    part: &mut MidiPart,
    ids: &[u64],
    count: u8,
    span_beats: f64,
) -> Result<usize, String> {
    if !(2..=16).contains(&count) {
        return Err("A tuplet needs between 2 and 16 notes".to_owned());
    }
    if !span_beats.is_finite() || span_beats <= 0.0 {
        return Err("The tuplet span must be a positive length".to_owned());
    }
    let mut selected = part
        .notes
        .iter()
        .enumerate()
        .filter(|(_, note)| says_yes(ids, note.id))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if selected.len() != usize::from(count) {
        return Err(format!(
            "Select exactly {count} note(s) to build this tuplet, or change the note count"
        ));
    }
    selected.sort_by(|left, right| {
        part.notes[*left]
            .start_beats
            .total_cmp(&part.notes[*right].start_beats)
            .then(left.cmp(right))
    });
    let anchor = part.notes[selected[0]].start_beats;
    let span = span_beats.min((part.duration_beats - anchor).max(0.0));
    let slot = span / f64::from(count);
    if slot <= 1e-6 {
        return Err("The part is too short for that tuplet".to_owned());
    }
    let mut changed = 0;
    for (rank, index) in selected.iter().enumerate() {
        let start = anchor + rank as f64 * slot;
        let duration = slot.min((part.duration_beats - start).max(1e-6));
        let note = &mut part.notes[*index];
        if (note.start_beats - start).abs() > 1e-9 || (note.duration_beats - duration).abs() > 1e-9
        {
            note.start_beats = start;
            note.duration_beats = duration;
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{MidiNote, Project, TrackKind};

    fn part() -> (Project, MidiPart) {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        project.next_midi_part_id = 2;
        project.next_midi_note_id = 4;
        let part = MidiPart {
            id: 1,
            name: "Part".to_owned(),
            start_beats: 0.0,
            duration_beats: 8.0,
            notes: vec![
                MidiNote {
                    id: 1,
                    start_beats: 0.0,
                    duration_beats: 0.5,
                    pitch: 60,
                    channel: 0,
                    velocity: 100,
                    hidden: false,
                    flipped: false,
                    grace: false,
                },
                MidiNote {
                    id: 2,
                    start_beats: 1.0,
                    duration_beats: 0.5,
                    pitch: 127,
                    channel: 0,
                    velocity: 1,
                    hidden: false,
                    flipped: false,
                    grace: false,
                },
                MidiNote {
                    id: 3,
                    start_beats: 2.0,
                    duration_beats: 0.5,
                    pitch: 0,
                    channel: 0,
                    velocity: 64,
                    hidden: false,
                    flipped: false,
                    grace: false,
                },
            ],
            controller_events: Vec::new(),
        };
        project.tracks[0].midi_parts.push(part.clone());
        (project, part)
    }

    #[test]
    fn transposing_moves_the_selection_and_skips_out_of_range_notes() {
        let (_project, mut part) = part();
        // +12 would push pitch 127 out of range, so only two notes move.
        assert_eq!(transpose_notes(&mut part, &[], 12), 2);
        assert_eq!(part.notes[0].pitch, 72);
        assert_eq!(part.notes[1].pitch, 127, "the top note stays put");
        assert_eq!(part.notes[2].pitch, 12);

        // Downwards: 72 and 127 both fit, the note at 12 does not.
        assert_eq!(transpose_notes(&mut part, &[], -48), 2);
        assert_eq!(part.notes[0].pitch, 24);
        assert_eq!(part.notes[1].pitch, 79);
        assert_eq!(part.notes[2].pitch, 12, "the low note stays put");
        assert_eq!(transpose_notes(&mut part, &[2], -1), 1);
        assert_eq!(part.notes[1].pitch, 78);
    }

    #[test]
    fn transposing_a_selection_leaves_the_rest_alone() {
        let (_project, mut part) = part();
        assert_eq!(transpose_notes(&mut part, &[3], 5), 1);
        assert_eq!(part.notes[0].pitch, 60);
        assert_eq!(part.notes[2].pitch, 5);
    }

    #[test]
    fn velocity_is_clamped_instead_of_overflowing() {
        let (_project, mut part) = part();
        // All three move: the 100 is clamped at the top, the 1 rises normally.
        assert_eq!(change_velocity(&mut part, &[], 40), 3);
        assert_eq!(part.notes[0].velocity, 127, "clamped at the top");
        assert_eq!(part.notes[1].velocity, 41);
        assert_eq!(part.notes[2].velocity, 104);

        // Everything drops to the floor, so all three notes change.
        assert_eq!(change_velocity(&mut part, &[], -127), 3);
        assert_eq!(part.notes[0].velocity, 1, "clamped at the bottom");
        assert_eq!(part.notes[1].velocity, 1);
        assert_eq!(part.notes[2].velocity, 1);
        assert_eq!(
            change_velocity(&mut part, &[], -1),
            0,
            "already at the floor"
        );
    }

    #[test]
    fn a_tuplet_evens_the_spacing_and_stays_inside_the_part() {
        let (_project, mut tuplet) = part();
        // The fixture has three notes at 0, 1 and 2 beats in an 8-beat part.
        // Three half-beat slots leave the first note where it already was.
        assert_eq!(build_tuplet(&mut tuplet, &[], 3, 1.5), Ok(2));
        let starts = tuplet
            .notes
            .iter()
            .map(|note| (note.start_beats, note.duration_beats))
            .collect::<Vec<_>>();
        assert_eq!(
            starts,
            vec![(0.0, 0.5), (0.5, 0.5), (1.0, 0.5)],
            "three slots of half a beat"
        );

        // A span that would leave the part is clamped instead of overflowing.
        let (_project, mut clamped) = part();
        // Clamped to the 8-beat part: three slots of about 2.67 beats.
        assert_eq!(build_tuplet(&mut clamped, &[], 3, 100.0), Ok(3));
        assert!(clamped
            .notes
            .iter()
            .all(|note| note.start_beats + note.duration_beats <= clamped.duration_beats + 1e-9));

        // A mismatched selection is refused rather than dropping notes.
        let (_project, mut mismatched) = part();
        assert!(build_tuplet(&mut mismatched, &[], 4, 1.0).is_err());
        assert!(build_tuplet(&mut mismatched, &[], 1, 1.0).is_err());
        assert!(build_tuplet(&mut mismatched, &[], 3, 0.0).is_err());
    }

    #[test]
    fn velocity_can_be_set_and_ramped_across_the_selection() {
        let (_project, mut part) = part();
        assert_eq!(set_velocity(&mut part, &[], 80), 3);
        assert!(part.notes.iter().all(|note| note.velocity == 80));
        assert_eq!(set_velocity(&mut part, &[], 80), 0, "no change twice");

        // A ramp follows time order: note 3 (beat 2) is between notes 1 (0) and
        // 2 (1), so the middle value goes to it.
        assert_eq!(ramp_velocity(&mut part, &[], 10, 110), 3);
        let by_start = |part: &MidiPart, beat: f64| {
            part.notes
                .iter()
                .find(|note| (note.start_beats - beat).abs() < 1e-9)
                .expect("note")
                .velocity
        };
        assert_eq!(by_start(&part, 0.0), 10);
        assert_eq!(by_start(&part, 1.0), 60);
        assert_eq!(by_start(&part, 2.0), 110);

        // A restricted selection only touches those notes.
        assert_eq!(ramp_velocity(&mut part, &[2], 5, 5), 1);
        assert_eq!(by_start(&part, 1.0), 5);
        assert_eq!(by_start(&part, 0.0), 10);
    }

    #[test]
    fn zero_and_unbounded_values_change_nothing() {
        let (project, mut part) = part();
        let before = part.clone();
        assert_eq!(transpose_notes(&mut part, &[], 0), 0);
        assert_eq!(
            transpose_notes(&mut part, &[], MAX_TRANSPOSE_SEMITONES + 1),
            0
        );
        assert_eq!(change_velocity(&mut part, &[], 0), 0);
        assert_eq!(change_velocity(&mut part, &[], MAX_VELOCITY_DELTA + 1), 0);
        assert_eq!(part, before);
        assert!(project.validate().is_ok());
    }
    #[test]
    fn score_flags_toggle_per_note() {
        let (mut project, part) = part();
        let mut part = project.tracks[0].midi_parts.remove(0);
        assert_eq!(toggle_hidden(&mut part, &[1]), 1);
        assert!(part.notes[0].hidden && !part.notes[1].hidden);
        assert_eq!(toggle_hidden(&mut part, &[1]), 1);
        assert!(!part.notes[0].hidden);
        assert_eq!(
            toggle_hidden(&mut part, &[9]),
            0,
            "unknown ids change nothing"
        );

        assert_eq!(toggle_flipped(&mut part, &[2, 3]), 2);
        assert!(part.notes[1].flipped && part.notes[2].flipped);
        assert!(!part.notes[0].flipped);
        assert_eq!(toggle_flipped(&mut part, &[2]), 1);
        assert!(!part.notes[1].flipped);

        assert_eq!(toggle_grace(&mut part, &[3]), 1);
        assert!(part.notes[2].grace && !part.notes[0].grace);
        assert_eq!(toggle_grace(&mut part, &[3]), 1);
        assert!(!part.notes[2].grace);
    }
}
