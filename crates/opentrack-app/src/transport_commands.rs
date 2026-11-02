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
}
