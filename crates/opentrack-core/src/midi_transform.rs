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
                },
                MidiNote {
                    id: 2,
                    start_beats: 1.0,
                    duration_beats: 0.5,
                    pitch: 127,
                    channel: 0,
                    velocity: 1,
                },
                MidiNote {
                    id: 3,
                    start_beats: 2.0,
                    duration_beats: 0.5,
                    pitch: 0,
                    channel: 0,
                    velocity: 64,
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
}
