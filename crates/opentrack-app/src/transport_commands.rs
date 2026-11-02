//! Shared transport selection and roll-target resolution.
//!
//! Steps 4-6 add pre/post-roll around the selection boundaries, so the boundary
//! resolution must be one function used by the existing play-from/until and
//! locate commands and by the upcoming roll commands. It is pure project state:
//! selections are stable event/part IDs, not screen coordinates.

use opentrack_core::project::Project;
use std::collections::BTreeSet;

/// The musical span covered by the resolved selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SelectionSpan {
    pub start_beats: f64,
    pub end_beats: f64,
}

/// Longest accepted pre/post-roll, in beats.
pub(crate) const MAX_ROLL_BEATS: f64 = 64.0;
/// Default roll length when a preference has never been stored.
pub(crate) const DEFAULT_ROLL_BEATS: f64 = 4.0;

/// Persisted pre/post-roll lengths, independent of each other.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RollLengths {
    pub pre_beats: f64,
    pub post_beats: f64,
}

impl Default for RollLengths {
    fn default() -> Self {
        Self {
            pre_beats: DEFAULT_ROLL_BEATS,
            post_beats: DEFAULT_ROLL_BEATS,
        }
    }
}

impl RollLengths {
    /// Loads a stored length, replacing non-finite values and clamping to the
    /// accepted range.
    pub(crate) fn from_stored(value: f64, fallback: f64) -> f64 {
        if value.is_finite() {
            value.clamp(0.0, MAX_ROLL_BEATS)
        } else {
            fallback
        }
    }

    /// Builds a pair from stored/edited values, keeping them independent.
    pub(crate) fn sanitized(pre_beats: f64, post_beats: f64) -> Self {
        Self {
            pre_beats: Self::from_stored(pre_beats, DEFAULT_ROLL_BEATS),
            post_beats: Self::from_stored(post_beats, DEFAULT_ROLL_BEATS),
        }
    }

    pub(crate) fn for_roll(self, post: bool) -> f64 {
        if post {
            self.post_beats
        } else {
            self.pre_beats
        }
    }
}

/// A scheduled roll: where transport starts and the beat it must stop at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RollPlan {
    pub start_beats: f64,
    pub stop_beats: f64,
}

/// Reasons a roll cannot be scheduled. The app maps these to status text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RollRefusal {
    MissingSelection,
    ZeroLengthSelection,
    Recording,
    CycleActive,
    ZeroRoll,
    NoRoomBeforeTarget,
}

/// Plans a pre-roll (ending at `target_beats`) or post-roll (starting there).
///
/// A pre-roll clamps its start at the project start; a target at beat zero has
/// no room to roll into. The caller refuses recording and active cycle before
/// using this, because neither interaction is supported yet.
pub(crate) fn plan_roll(
    lengths: RollLengths,
    post: bool,
    span: SelectionSpan,
    edge_end: bool,
) -> Result<RollPlan, RollRefusal> {
    if !span.start_beats.is_finite()
        || !span.end_beats.is_finite()
        || span.end_beats <= span.start_beats
    {
        return Err(RollRefusal::ZeroLengthSelection);
    }
    let length = lengths.for_roll(post).clamp(0.0, MAX_ROLL_BEATS);
    if length <= 0.0 {
        return Err(RollRefusal::ZeroRoll);
    }
    let target = if edge_end {
        span.end_beats
    } else {
        span.start_beats
    };
    let (start_beats, stop_beats) = if post {
        (target, target + length)
    } else {
        let start = (target - length).max(0.0);
        if target <= 0.0 {
            return Err(RollRefusal::NoRoomBeforeTarget);
        }
        (start, target)
    };
    if !start_beats.is_finite() || !stop_beats.is_finite() || stop_beats <= start_beats {
        return Err(RollRefusal::ZeroRoll);
    }
    Ok(RollPlan {
        start_beats,
        stop_beats,
    })
}

/// The span of the selected audio events and selected MIDI part.
///
/// Returns `None` when nothing is selected or the resolved span has no length,
/// so a caller can report a reason instead of scheduling a zero-length target.
pub(crate) fn selected_event_span(
    project: &Project,
    selected_audio: &BTreeSet<u64>,
    selected_midi_part: Option<u64>,
) -> Option<SelectionSpan> {
    let tempo = project.tempo_bpm.max(1.0);
    let mut start = f64::INFINITY;
    let mut end = f64::NEG_INFINITY;
    for track in &project.tracks {
        for clip in &track.clips {
            if selected_audio.contains(&clip.id) {
                start = start.min(clip.start_beats);
                end = end.max(clip.start_beats + clip.duration_seconds * tempo / 60.0);
            }
        }
        for part in &track.midi_parts {
            if selected_midi_part == Some(part.id) {
                start = start.min(part.start_beats);
                end = end.max(part.start_beats + part.duration_beats);
            }
        }
    }
    (start.is_finite() && end.is_finite() && end > start).then_some(SelectionSpan {
        start_beats: start,
        end_beats: end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::project::TrackKind;
    use std::path::PathBuf;

    fn project_with_audio_clip(start: f64, duration: f64) -> (Project, u64) {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, PathBuf::from("take.wav"), start, duration)
            .expect("clip");
        (project, id)
    }

    #[test]
    fn no_selection_resolves_to_nothing() {
        let project = Project::default();
        assert_eq!(selected_event_span(&project, &BTreeSet::new(), None), None);
    }

    #[test]
    fn audio_spans_convert_seconds_at_the_project_tempo() {
        let (project, id) = project_with_audio_clip(1.0, 2.0);
        assert!((project.tempo_bpm - 120.0).abs() < 1e-9);
        assert_eq!(
            selected_event_span(&project, &BTreeSet::from([id]), None),
            Some(SelectionSpan {
                start_beats: 1.0,
                end_beats: 5.0,
            })
        );
    }

    #[test]
    fn a_midi_part_joins_the_same_span() {
        let (mut project, clip) = project_with_audio_clip(4.0, 1.0);
        project.add_track(TrackKind::Midi);
        let part = project.add_midi_part(1, 0.0, 2.0).expect("part");
        assert_eq!(
            selected_event_span(&project, &BTreeSet::from([clip]), Some(part)),
            Some(SelectionSpan {
                start_beats: 0.0,
                end_beats: 6.0,
            })
        );
    }

    #[test]
    fn a_stale_selection_is_not_a_target() {
        let (project, _) = project_with_audio_clip(2.0, 1.0);
        assert_eq!(
            selected_event_span(&project, &BTreeSet::from([u64::MAX]), None),
            None
        );
    }

    #[test]
    fn roll_lengths_are_independent_and_bounded() {
        let lengths = RollLengths::sanitized(2.0, 8.0);
        assert_eq!(lengths.pre_beats, 2.0);
        assert_eq!(lengths.post_beats, 8.0);
        assert_eq!(
            RollLengths::sanitized(-3.0, 1000.0),
            RollLengths {
                pre_beats: 0.0,
                post_beats: MAX_ROLL_BEATS,
            }
        );
        assert_eq!(
            RollLengths::sanitized(f64::NAN, 3.0).post_beats,
            3.0,
            "a non-finite stored value falls back, not to zero"
        );
    }

    #[test]
    fn pre_roll_clamps_at_the_project_start_and_ends_at_the_target() {
        let span = SelectionSpan {
            start_beats: 2.0,
            end_beats: 6.0,
        };
        let plan = plan_roll(
            RollLengths {
                pre_beats: 4.0,
                post_beats: 4.0,
            },
            false,
            span,
            false,
        )
        .expect("pre-roll");
        assert_eq!(plan.start_beats, 0.0, "clamped at the project start");
        assert_eq!(plan.stop_beats, 2.0);

        let near_start = plan_roll(
            RollLengths::default(),
            false,
            SelectionSpan {
                start_beats: 0.0,
                end_beats: 4.0,
            },
            false,
        );
        assert_eq!(near_start, Err(RollRefusal::NoRoomBeforeTarget));
    }

    #[test]
    fn post_roll_starts_at_the_named_edge_and_uses_its_own_length() {
        let span = SelectionSpan {
            start_beats: 2.0,
            end_beats: 6.0,
        };
        let lengths = RollLengths {
            pre_beats: 1.0,
            post_beats: 8.0,
        };
        let from_start = plan_roll(lengths, true, span, false).expect("post start");
        assert_eq!(
            from_start,
            RollPlan {
                start_beats: 2.0,
                stop_beats: 10.0,
            }
        );
        let from_end = plan_roll(lengths, true, span, true).expect("post end");
        assert_eq!(
            from_end,
            RollPlan {
                start_beats: 6.0,
                stop_beats: 14.0,
            }
        );
    }

    #[test]
    fn a_zero_length_roll_or_selection_is_refused() {
        let span = SelectionSpan {
            start_beats: 1.0,
            end_beats: 2.0,
        };
        assert_eq!(
            plan_roll(
                RollLengths {
                    pre_beats: 0.0,
                    post_beats: 0.0,
                },
                false,
                span,
                false
            ),
            Err(RollRefusal::ZeroRoll)
        );
        assert_eq!(
            plan_roll(
                RollLengths::default(),
                true,
                SelectionSpan {
                    start_beats: 1.0,
                    end_beats: 1.0,
                },
                false
            ),
            Err(RollRefusal::ZeroLengthSelection)
        );
    }
}
