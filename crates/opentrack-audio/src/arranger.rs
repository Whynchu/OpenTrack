//! Offline arranger sequence preparation.
//!
//! The source clips, MIDI and automation stay untouched. This module builds a new
//! immutable playback-timeline snapshot by copying each section's content into
//! the sequence order, clipping events to half-open section boundaries. Source
//! clips are never moved or rendered destructively.

use crate::{PlaybackClip, PlaybackMidiController, PlaybackMidiNote, PlaybackTrackSettings};
use opentrack_core::automation::{AutomationLane, AutomationPoint};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrangerSequenceSegment {
    pub start_beats: f64,
    pub end_beats: f64,
    pub repeats: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArrangerSequenceError {
    EmptySequence,
    InvalidSection,
    UnsupportedFades,
    TooManyFrames,
}

impl std::fmt::Display for ArrangerSequenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySequence => write!(formatter, "the arranger sequence is empty"),
            Self::InvalidSection => write!(formatter, "a section has an invalid beat range"),
            Self::UnsupportedFades => write!(
                formatter,
                "arranger rendering does not yet support clip fades or source envelopes"
            ),
            Self::TooManyFrames => write!(formatter, "the sequence is too long to render"),
        }
    }
}

pub struct ArrangerSequence {
    pub tracks: Vec<PlaybackTrackSettings>,
    pub total_beats: f64,
}

/// Maximum rendered sequence length in beats at the current tempo. Longer
/// sequences are refused instead of allocating an unbounded buffer.
const MAX_SEQUENCE_BEATS: f64 = 60.0 * 60.0; // one hour at 60 BPM

pub fn arrange_sequence(
    tempo_bpm: f64,
    tracks: &[PlaybackTrackSettings],
    segments: &[ArrangerSequenceSegment],
) -> Result<ArrangerSequence, ArrangerSequenceError> {
    if segments.is_empty() {
        return Err(ArrangerSequenceError::EmptySequence);
    }
    let tempo_bpm = if tempo_bpm.is_finite() && tempo_bpm > 0.0 {
        tempo_bpm
    } else {
        return Err(ArrangerSequenceError::InvalidSection);
    };
    let mut total_beats = 0.0;
    for segment in segments {
        if !segment.start_beats.is_finite()
            || !segment.end_beats.is_finite()
            || segment.start_beats < 0.0
            || segment.end_beats <= segment.start_beats
            || segment.repeats == 0
        {
            return Err(ArrangerSequenceError::InvalidSection);
        }
        total_beats += (segment.end_beats - segment.start_beats) * f64::from(segment.repeats);
        if !total_beats.is_finite() || total_beats > MAX_SEQUENCE_BEATS {
            return Err(ArrangerSequenceError::TooManyFrames);
        }
    }

    let mut expanded = Vec::with_capacity(tracks.len());
    for track in tracks {
        let mut output = track.clone();
        output.clips.clear();
        output.midi_notes.clear();
        output.midi_controllers.clear();
        output.volume_automation = None;
        let mut automation_points = Vec::new();
        let mut sequence_offset = 0.0;
        for segment in segments {
            let section_length = segment.end_beats - segment.start_beats;
            for _ in 0..segment.repeats {
                for clip in &track.clips {
                    let clip_end = clip.start_beats + clip.duration_seconds * tempo_bpm / 60.0;
                    let start = clip.start_beats.max(segment.start_beats);
                    let end = clip_end.min(segment.end_beats);
                    if end <= start + 1.0e-9 {
                        continue;
                    }
                    if clip.fade_in_seconds > 0.0
                        || clip.fade_out_seconds > 0.0
                        || !clip.source_fades.is_empty()
                    {
                        return Err(ArrangerSequenceError::UnsupportedFades);
                    }
                    let mut copy: PlaybackClip = clip.clone();
                    copy.start_beats = sequence_offset + (start - segment.start_beats);
                    copy.source_offset_seconds += (start - clip.start_beats) * 60.0 / tempo_bpm;
                    copy.duration_seconds = (end - start) * 60.0 / tempo_bpm;
                    copy.fade_in_seconds = 0.0;
                    copy.fade_out_seconds = 0.0;
                    output.clips.push(copy);
                }
                for note in &track.midi_notes {
                    let note_end = note.start_beats + note.duration_beats;
                    let start = note.start_beats.max(segment.start_beats);
                    let end = note_end.min(segment.end_beats);
                    if end <= start + 1.0e-9 {
                        continue;
                    }
                    output.midi_notes.push(PlaybackMidiNote {
                        start_beats: sequence_offset + (start - segment.start_beats),
                        duration_beats: end - start,
                        ..*note
                    });
                }
                for controller in &track.midi_controllers {
                    if controller.position_beats >= segment.start_beats
                        && controller.position_beats < segment.end_beats
                    {
                        output.midi_controllers.push(PlaybackMidiController {
                            position_beats: sequence_offset
                                + (controller.position_beats - segment.start_beats),
                            ..*controller
                        });
                    }
                }
                if let Some(lane) = &track.volume_automation {
                    if let Some(value) = lane.value_at(segment.start_beats) {
                        automation_points.push(AutomationPoint {
                            beat: sequence_offset,
                            value,
                        });
                    }
                    for point in &lane.points {
                        if point.beat >= segment.start_beats && point.beat < segment.end_beats {
                            automation_points.push(AutomationPoint {
                                beat: sequence_offset + (point.beat - segment.start_beats),
                                value: point.value,
                            });
                        }
                    }
                }
                sequence_offset += section_length;
            }
        }
        if !automation_points.is_empty() {
            automation_points.sort_by(|left, right| left.beat.total_cmp(&right.beat));
            output.volume_automation = Some(Arc::new(AutomationLane {
                target: opentrack_core::automation::AutomationTarget::TrackVolume,
                points: automation_points,
            }));
        }
        expanded.push(output);
    }
    Ok(ArrangerSequence {
        tracks: expanded,
        total_beats,
    })
}
