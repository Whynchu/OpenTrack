// SPDX-License-Identifier: Apache-2.0

//! Musical quantize for MIDI notes.
//!
//! Quantize moves notes toward a grid, but never has to move them all the way:
//! *strength* keeps part of the performance's timing, and *swing* offsets every
//! second grid line so the grid itself breathes. Both are bounded, and the part's
//! own length is the limit — a note cannot be pushed outside the part that holds
//! it.

use serde::{Deserialize, Serialize};

use crate::project::MidiPart;

/// How far note lengths may be stretched in one command.
pub const MAX_QUANTIZE_GRID_BEATS: f64 = 64.0;

/// Quantize settings, as a user would set them.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct QuantizeOptions {
    /// Distance between grid lines, in quarter-note beats.
    pub grid_beats: f64,
    /// 0 keeps the recorded timing, 1 moves each note onto the grid.
    pub strength: f32,
    /// 0 is a straight grid; 1 delays every second line by half a step.
    pub swing: f32,
    /// Also snap note lengths to the grid.
    pub quantize_lengths: bool,
}

impl Default for QuantizeOptions {
    fn default() -> Self {
        Self {
            grid_beats: 0.25,
            strength: 1.0,
            swing: 0.0,
            quantize_lengths: false,
        }
    }
}

impl QuantizeOptions {
    /// Clamps the settings into a range the command can honor.
    ///
    /// Returns `None` when the grid is unusable, which the caller reports instead
    /// of quantizing everything to beat zero.
    pub fn sanitized(self) -> Option<Self> {
        if !self.grid_beats.is_finite() || self.grid_beats <= 0.0 {
            return None;
        }
        let strength = if self.strength.is_finite() {
            self.strength.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let swing = if self.swing.is_finite() {
            self.swing.clamp(0.0, 1.0)
        } else {
            0.0
        };
        Some(Self {
            grid_beats: self.grid_beats.min(MAX_QUANTIZE_GRID_BEATS),
            strength,
            swing,
            quantize_lengths: self.quantize_lengths,
        })
    }

    /// The grid line a position belongs to, including the swing offset.
    fn grid_line(self, position: f64) -> f64 {
        let step = self.grid_beats;
        let index = (position / step).round();
        // Every second line is an "off" subdivision, which swing delays.
        let offset = if (index as i64).rem_euclid(2) == 1 {
            self.swing as f64 * step * 0.5
        } else {
            0.0
        };
        index * step + offset
    }
}

/// Quantizes the given notes of `part` and reports how many changed.
///
/// A note that is already on the grid is left untouched, so repeated commands do
/// not keep nudging notes by the strength fraction.
pub fn quantize_notes(part: &mut MidiPart, ids: &[u64], options: QuantizeOptions) -> usize {
    let Some(options) = options.sanitized() else {
        return 0;
    };
    if options.strength <= 0.0 && !options.quantize_lengths {
        return 0;
    }
    let selected = ids.is_empty();
    let limit = part.duration_beats.max(options.grid_beats);
    let mut changed = 0;
    for note in part.notes.iter_mut() {
        if !selected && !ids.contains(&note.id) {
            continue;
        }
        let mut moved = false;
        if options.strength > 0.0 {
            let target = options.grid_line(note.start_beats);
            let start =
                note.start_beats + (target - note.start_beats) * f64::from(options.strength);
            let start = start.clamp(0.0, (limit - note.duration_beats.min(limit)).max(0.0));
            if (start - note.start_beats).abs() > 1e-9 {
                note.start_beats = start;
                moved = true;
            }
        }
        if options.quantize_lengths {
            let lines = (note.duration_beats / options.grid_beats).round().max(1.0);
            let duration = (lines * options.grid_beats)
                .min((limit - note.start_beats).max(options.grid_beats).max(1e-6));
            if (duration - note.duration_beats).abs() > 1e-9 {
                note.duration_beats = duration;
                moved = true;
            }
        }
        if moved {
            changed += 1;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{MidiNote, Project, TrackKind};

    fn part_with_starts(starts: &[f64]) -> (Project, u64, u64) {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let track_id = project.tracks[0].id;
        let part_id = 1;
        project.next_midi_part_id = part_id + 1;
        project.next_midi_note_id = starts.len() as u64 + 1;
        project.tracks[0].midi_parts.push(MidiPart {
            id: part_id,
            name: "Part".to_owned(),
            start_beats: 0.0,
            duration_beats: 16.0,
            notes: starts
                .iter()
                .enumerate()
                .map(|(index, start)| MidiNote {
                    id: index as u64 + 1,
                    start_beats: *start,
                    duration_beats: 0.5,
                    pitch: 60,
                    channel: 0,
                    velocity: 100,
                })
                .collect(),
            controller_events: Vec::new(),
        });
        (project, track_id, part_id)
    }

    fn starts(project: &Project) -> Vec<f64> {
        project.tracks[0].midi_parts[0]
            .notes
            .iter()
            .map(|note| note.start_beats)
            .collect()
    }

    #[test]
    fn full_strength_puts_every_note_on_the_grid() {
        let (mut project, _, _) = part_with_starts(&[0.02, 0.24, 0.51, 1.01]);
        let changed = quantize_notes(
            &mut project.tracks[0].midi_parts[0],
            &[],
            QuantizeOptions::default(),
        );
        assert_eq!(changed, 4);
        assert_eq!(starts(&project), vec![0.0, 0.25, 0.5, 1.0]);
    }

    #[test]
    fn strength_keeps_part_of_the_performance_timing() {
        let (mut project, _, _) = part_with_starts(&[0.1]);
        // Halfway between the recorded 0.1 and the grid 0.0.
        quantize_notes(
            &mut project.tracks[0].midi_parts[0],
            &[],
            QuantizeOptions {
                strength: 0.5,
                ..QuantizeOptions::default()
            },
        );
        assert!((starts(&project)[0] - 0.05).abs() < 1e-9);
    }

    #[test]
    fn swing_delays_the_off_grid_lines() {
        let (mut project, _, _) = part_with_starts(&[0.25, 0.5, 0.75, 1.0]);
        quantize_notes(
            &mut project.tracks[0].midi_parts[0],
            &[],
            QuantizeOptions {
                swing: 0.5,
                ..QuantizeOptions::default()
            },
        );
        // Odd-indexed lines move later by half a step times the swing; the
        // even ones stay where they are.
        assert_eq!(starts(&project), vec![0.3125, 0.5, 0.8125, 1.0]);
    }

    #[test]
    fn lengths_quantize_to_whole_steps_and_stay_inside_the_part() {
        // The note fits the part before quantizing (15.4 + 0.5 = 15.9 of 16).
        let (mut project, _, _) = part_with_starts(&[15.4]);
        quantize_notes(
            &mut project.tracks[0].midi_parts[0],
            &[],
            QuantizeOptions {
                quantize_lengths: true,
                ..QuantizeOptions::default()
            },
        );
        let note = &project.tracks[0].midi_parts[0].notes[0];
        // The note cannot start inside the part and still last a whole step, so
        // it is clamped to what is left.
        assert!(note.start_beats <= 16.0);
        assert!(note.duration_beats > 0.0);
        assert!(note.start_beats + note.duration_beats <= 16.0 + 1e-6);
        assert_eq!(project.validate(), Ok(()));
    }

    #[test]
    fn an_already_quantized_part_reports_no_change() {
        let (mut project, _, _) = part_with_starts(&[0.0, 0.25, 0.5]);
        assert_eq!(
            quantize_notes(
                &mut project.tracks[0].midi_parts[0],
                &[],
                QuantizeOptions::default()
            ),
            0
        );
    }

    #[test]
    fn only_the_selected_notes_move() {
        let (mut project, _, _) = part_with_starts(&[0.02, 0.24]);
        let changed = quantize_notes(
            &mut project.tracks[0].midi_parts[0],
            &[2],
            QuantizeOptions::default(),
        );
        assert_eq!(changed, 1);
        assert!(
            (starts(&project)[0] - 0.02).abs() < 1e-9,
            "the first note stays"
        );
        assert!((starts(&project)[1] - 0.25).abs() < 1e-9);
    }

    #[test]
    fn unusable_settings_change_nothing() {
        let (mut project, _, _) = part_with_starts(&[0.02]);
        let before = project.clone();
        assert_eq!(
            quantize_notes(
                &mut project.tracks[0].midi_parts[0],
                &[],
                QuantizeOptions {
                    grid_beats: 0.0,
                    ..QuantizeOptions::default()
                }
            ),
            0
        );
        assert_eq!(
            quantize_notes(
                &mut project.tracks[0].midi_parts[0],
                &[],
                QuantizeOptions {
                    strength: 0.0,
                    ..QuantizeOptions::default()
                }
            ),
            0
        );
        assert_eq!(project, before);
        // Clamping keeps the settings usable rather than refusing them.
        let clamped = QuantizeOptions {
            grid_beats: 1_000.0,
            strength: 2.0,
            swing: -1.0,
            quantize_lengths: false,
        }
        .sanitized()
        .expect("usable");
        assert_eq!(clamped.grid_beats, MAX_QUANTIZE_GRID_BEATS);
        assert_eq!(clamped.strength, 1.0);
        assert_eq!(clamped.swing, 0.0);
    }
}
