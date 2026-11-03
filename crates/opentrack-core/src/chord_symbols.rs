//! Bounded chord-name analysis for the score's Make Chord Symbols command.
//!
//! Only triads are recognised (major, minor, diminished, augmented) from the
//! pitches that start together, with a slash bass for an inversion. Anything
//! else is left unlabelled rather than guessed; seventh chords, extensions and
//! key-aware spelling are out of scope.

use crate::project::{ChordSymbol, MidiNote};
use std::collections::BTreeMap;

const NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// The chord name for a set of simultaneous pitches, or `None` when they do not
/// form a recognised triad.
pub fn chord_name(pitches: &[u8]) -> Option<String> {
    let mut classes = pitches.iter().map(|pitch| pitch % 12).collect::<Vec<_>>();
    classes.sort_unstable();
    classes.dedup();
    if classes.len() < 3 {
        return None;
    }
    let lowest = pitches.iter().copied().min()? % 12;
    let mut roots = vec![lowest];
    roots.extend(classes.iter().copied().filter(|class| *class != lowest));
    for root in roots {
        let has = |interval: u8| classes.contains(&((root + interval) % 12));
        let quality = if has(4) && has(7) {
            ""
        } else if has(3) && has(7) {
            "m"
        } else if has(3) && has(6) {
            "dim"
        } else if has(4) && has(8) {
            "aug"
        } else {
            continue;
        };
        let chord_tone = |class: u8| {
            [0u8, 3, 4, 6, 7, 8]
                .iter()
                .any(|interval| (root + interval) % 12 == class)
        };
        let mut text = format!("{}{quality}", NAMES[usize::from(root)]);
        if lowest != root && chord_tone(lowest) {
            text.push('/');
            text.push_str(NAMES[usize::from(lowest)]);
        }
        return Some(text);
    }
    None
}

/// Chord symbols for the notes that start together, sorted by beat.
pub fn symbols_for_notes(notes: &[MidiNote]) -> Vec<ChordSymbol> {
    let mut groups: BTreeMap<i64, Vec<u8>> = BTreeMap::new();
    for note in notes {
        let key = (note.start_beats.max(0.0) * 1_000.0).round() as i64;
        groups.entry(key).or_default().push(note.pitch);
    }
    groups
        .into_iter()
        .filter_map(|(key, pitches)| {
            chord_name(&pitches).map(|text| ChordSymbol {
                beat: key as f64 / 1_000.0,
                text,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triads_are_named_with_inversions_and_qualities() {
        assert_eq!(chord_name(&[60, 64, 67]).as_deref(), Some("C"));
        assert_eq!(chord_name(&[57, 60, 64]).as_deref(), Some("Am"));
        assert_eq!(chord_name(&[60, 63, 66]).as_deref(), Some("Cdim"));
        assert_eq!(chord_name(&[60, 64, 68]).as_deref(), Some("Caug"));
        // An inversion shows the bass.
        assert_eq!(chord_name(&[64, 67, 72]).as_deref(), Some("C/E"));
        // Octave doublings do not change the name.
        assert_eq!(chord_name(&[48, 60, 64, 67]).as_deref(), Some("C"));
        // Fewer than three pitch classes or no triad: no label.
        assert_eq!(chord_name(&[60, 64]), None);
        assert_eq!(chord_name(&[60, 61, 62]), None);
        assert_eq!(chord_name(&[]), None);
    }

    #[test]
    fn simultaneous_notes_group_into_symbols() {
        let note = |id, beat, pitch| MidiNote {
            id,
            start_beats: beat,
            duration_beats: 1.0,
            pitch,
            channel: 0,
            velocity: 100,
            hidden: false,
            flipped: false,
            grace: false,
            slur_to_next: false,
            group: None,
        };
        let notes = vec![
            note(1, 0.0, 60),
            note(2, 0.0, 64),
            note(3, 0.0, 67),
            note(4, 1.0, 57),
            note(5, 1.0, 60),
            note(6, 1.0, 64),
            note(7, 2.0, 60),
            note(8, 2.0, 61),
            note(9, 2.0, 62),
        ];
        let symbols = symbols_for_notes(&notes);
        assert_eq!(symbols.len(), 2, "{symbols:?}");
        assert_eq!(symbols[0].beat, 0.0);
        assert_eq!(symbols[0].text, "C");
        assert_eq!(symbols[1].beat, 1.0);
        assert_eq!(symbols[1].text, "Am");
    }
}
