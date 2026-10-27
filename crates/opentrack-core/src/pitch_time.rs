//! Durable monophonic analysis and non-destructive note edit intent.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DETECTOR_VERSION: u32 = 2;
const MIN_EDGE_PITCH_CONFIDENCE: f32 = 0.15;
const MAX_EDGE_PITCH_GAP_SECONDS: f64 = 0.025;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchPoint {
    pub source_time_seconds: f64,
    pub frequency_hz: Option<f32>,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchAnalysis {
    pub sample_rate_hz: u32,
    pub source_start_seconds: f64,
    pub source_duration_seconds: f64,
    pub points: Vec<PitchPoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchNote {
    pub id: u64,
    /// Source/event-local bounds are retained when the destination moves.
    pub source_start: f64,
    pub source_end: f64,
    pub start: f64,
    pub end: f64,
    pub detected_midi: f32,
    pub shift: f32,
    pub straighten: f32,
    /// Pitch offset at the left edge, in semitones from the note's base pitch.
    #[serde(default)]
    pub pitch_start_bend: f32,
    /// User-set anchor position within the destination note, from 0 through 1.
    #[serde(default = "default_pitch_anchor")]
    pub pitch_anchor: f64,
    /// Pitch offset at the anchor. Usually zero; retained when a bent note is split.
    #[serde(default)]
    pub pitch_anchor_bend: f32,
    /// Pitch offset at the right edge, in semitones from the note's base pitch.
    #[serde(default)]
    pub pitch_end_bend: f32,
}

fn default_pitch_anchor() -> f64 {
    0.5
}

impl PitchNote {
    pub fn pitch(&self, detected: f32) -> f32 {
        detected + self.shift + (self.detected_midi - detected) * self.straighten
    }
    pub fn time(&self, source_local: f64) -> f64 {
        self.start
            + (source_local - self.source_start) / (self.source_end - self.source_start)
                * (self.end - self.start)
    }

    pub fn pitch_at(&self, detected: f32, destination_time: f64) -> f32 {
        let base = self.pitch(detected);
        let fraction = ((destination_time - self.start) / (self.end - self.start)).clamp(0.0, 1.0);
        let anchor = self.pitch_anchor.clamp(0.0, 1.0);
        let bend = if fraction <= anchor && anchor > f64::EPSILON {
            let t = (fraction / anchor) as f32;
            self.pitch_start_bend * (1.0 - t) + self.pitch_anchor_bend * t
        } else if anchor < 1.0 {
            let t = ((fraction - anchor) / (1.0 - anchor)) as f32;
            self.pitch_anchor_bend * (1.0 - t) + self.pitch_end_bend * t
        } else {
            let t = fraction as f32;
            self.pitch_start_bend * (1.0 - t) + self.pitch_anchor_bend * t
        };
        base + bend
    }

    pub fn pitch_bend_at_fraction(&self, fraction: f64) -> f32 {
        let anchor = self.pitch_anchor.clamp(0.0, 1.0);
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction <= anchor && anchor > f64::EPSILON {
            let t = (fraction / anchor) as f32;
            self.pitch_start_bend * (1.0 - t) + self.pitch_anchor_bend * t
        } else if anchor < 1.0 {
            let t = ((fraction - anchor) / (1.0 - anchor)) as f32;
            self.pitch_anchor_bend * (1.0 - t) + self.pitch_end_bend * t
        } else {
            let t = fraction as f32;
            self.pitch_start_bend * (1.0 - t) + self.pitch_anchor_bend * t
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchTimeEdit {
    pub detector_version: u32,
    pub source_bytes: u64,
    pub source_modified_ns: u64,
    pub analysis: PitchAnalysis,
    pub notes: Vec<PitchNote>,
    pub rendered_source: Option<PathBuf>,
    pub enabled: bool,
}

impl PitchTimeEdit {
    /// Return the corrected pitch at a destination-local time, easing an edited
    /// note toward its touching neighbor over at most one sixteenth-note.
    /// Live rendering and the Sample Editor use this same contour.
    pub fn pitch_midi_at(
        &self,
        note_index: usize,
        detected_midi: f32,
        destination_time: f64,
        tempo_bpm: f64,
    ) -> f32 {
        let Some(note) = self.notes.get(note_index) else {
            return detected_midi;
        };
        let mut target = note.pitch_at(detected_midi, destination_time);

        let tempo = if tempo_bpm.is_finite() && tempo_bpm > 0.0 {
            tempo_bpm
        } else {
            120.0
        };
        let span = (15.0 / tempo).min((note.end - note.start).max(0.0) * 0.49);
        if span <= 0.0 {
            return target;
        }
        let tolerance = 1.0 / f64::from(self.analysis.sample_rate_hz.max(1));

        if let Some(previous_index) = note_index.checked_sub(1) {
            let previous = &self.notes[previous_index];
            if (previous.end - note.start).abs() <= tolerance
                && (note_has_pitch_edit(previous) || note_has_pitch_edit(note))
                && pitch_rules_differ(previous, note)
            {
                let boundary = boundary_pitch(self, previous, note);
                let progress = ((destination_time - note.start) / span).clamp(0.0, 1.0) as f32;
                let pull = 1.0 - smooth_pitch_blend(progress);
                target = target * (1.0 - pull) + boundary * pull;
            }
        }

        if let Some(next) = self.notes.get(note_index + 1) {
            if (note.end - next.start).abs() <= tolerance
                && (note_has_pitch_edit(note) || note_has_pitch_edit(next))
                && pitch_rules_differ(note, next)
            {
                let boundary = boundary_pitch(self, note, next);
                let progress = ((note.end - destination_time) / span).clamp(0.0, 1.0) as f32;
                let pull = 1.0 - smooth_pitch_blend(progress);
                target = target * (1.0 - pull) + boundary * pull;
            }
        }

        target
    }

    pub fn matches_range(&self, offset: f64, duration: f64) -> bool {
        self.detector_version == DETECTOR_VERSION
            && (self.analysis.source_start_seconds - offset).abs() < 0.0001
            && (self.analysis.source_duration_seconds - duration).abs() < 0.001
    }

    pub fn validate(&self, duration: f64) -> bool {
        if !duration.is_finite()
            || duration <= 0.0
            || duration > 3600.0
            || !self.analysis.source_start_seconds.is_finite()
            || self.analysis.source_start_seconds < 0.0
            || self.analysis.points.len() > 400_000
            || self.analysis.points.iter().any(|p| {
                !p.source_time_seconds.is_finite()
                    || p.source_time_seconds < self.analysis.source_start_seconds - 0.001
                    || p.source_time_seconds > self.analysis.source_start_seconds + duration + 0.001
                    || !p.confidence.is_finite()
                    || !(0.0..=1.0).contains(&p.confidence)
                    || p.frequency_hz
                        .is_some_and(|f| !f.is_finite() || !(20.0..=4000.0).contains(&f))
            })
        {
            return false;
        }
        let mut source_end = 0.0;
        let mut end = 0.0;
        let mut ids = std::collections::BTreeSet::new();
        for n in &self.notes {
            if ![n.start, n.end, n.source_start, n.source_end]
                .iter()
                .all(|v| v.is_finite())
                || ![
                    n.detected_midi,
                    n.shift,
                    n.straighten,
                    n.pitch_start_bend,
                    n.pitch_anchor_bend,
                    n.pitch_end_bend,
                ]
                .iter()
                .all(|v| v.is_finite())
                || n.source_start < source_end - 1e-6
                || n.start < end - 1e-6
                || n.source_end - n.source_start < 0.01
                || n.end - n.start < 0.01
                || n.source_end > duration + 0.001
                || n.end > duration + 0.001
                || !(-12.0..=12.0).contains(&n.shift)
                || !(0.0..=127.0).contains(&n.detected_midi)
                || !ids.insert(n.id)
                || !(0.0..=1.0).contains(&n.straighten)
                || !(0.0..=1.0).contains(&n.pitch_anchor)
                || !(-12.0..=12.0).contains(&n.pitch_start_bend)
                || !(-12.0..=12.0).contains(&n.pitch_anchor_bend)
                || !(-12.0..=12.0).contains(&n.pitch_end_bend)
            {
                return false;
            }
            source_end = n.source_end;
            end = n.end;
        }
        true
    }
}

fn note_has_pitch_edit(note: &PitchNote) -> bool {
    note.shift.abs() > 1e-6
        || note.straighten > 1e-6
        || note.pitch_start_bend.abs() > 1e-6
        || note.pitch_anchor_bend.abs() > 1e-6
        || note.pitch_end_bend.abs() > 1e-6
}

fn pitch_rules_differ(left: &PitchNote, right: &PitchNote) -> bool {
    (left.shift - right.shift).abs() > 1e-6
        || (left.straighten - right.straighten).abs() > 1e-6
        || (left.straighten > 1e-6 && (left.detected_midi - right.detected_midi).abs() > 1e-6)
        || (left.pitch_start_bend - right.pitch_start_bend).abs() > 1e-6
        || (left.pitch_anchor - right.pitch_anchor).abs() > 1e-6
        || (left.pitch_anchor_bend - right.pitch_anchor_bend).abs() > 1e-6
        || (left.pitch_end_bend - right.pitch_end_bend).abs() > 1e-6
}

fn boundary_pitch(edit: &PitchTimeEdit, left: &PitchNote, right: &PitchNote) -> f32 {
    let sample_period = 1.0 / f64::from(edit.analysis.sample_rate_hz.max(1));
    let left_midi = analyzed_midi_at(
        &edit.analysis,
        left.source_end - sample_period * 0.5,
        left.detected_midi,
    );
    let right_midi = analyzed_midi_at(
        &edit.analysis,
        right.source_start + sample_period * 0.5,
        right.detected_midi,
    );
    let left_target = left.pitch_at(left_midi, left.end);
    let right_target = right.pitch_at(right_midi, right.start);
    match (note_has_pitch_edit(left), note_has_pitch_edit(right)) {
        (false, true) => left_target,
        (true, false) => right_target,
        _ => (left_target + right_target) * 0.5,
    }
}

fn analyzed_midi_at(analysis: &PitchAnalysis, source_local_time: f64, fallback: f32) -> f32 {
    let points = &analysis.points;
    if points.is_empty() {
        return fallback;
    }
    let time = analysis.source_start_seconds + source_local_time;
    let right_index = points.partition_point(|point| point.source_time_seconds < time);
    let left = voiced_edge_point(points, time, (0..right_index.min(points.len())).rev());
    let right = voiced_edge_point(points, time, right_index.min(points.len())..points.len());
    let frequency = match (left, right) {
        (Some((left_time, left_hz)), Some((right_time, right_hz)))
            if right_time > left_time && right_time - left_time <= MAX_EDGE_PITCH_GAP_SECONDS =>
        {
            let fraction = ((time - left_time) / (right_time - left_time)).clamp(0.0, 1.0);
            (f64::from(left_hz).log2() * (1.0 - fraction) + f64::from(right_hz).log2() * fraction)
                .exp2() as f32
        }
        (Some((left_time, left_hz)), Some((right_time, right_hz))) => {
            if (time - left_time).abs() <= (right_time - time).abs() {
                left_hz
            } else {
                right_hz
            }
        }
        (Some((_, frequency)), None) | (None, Some((_, frequency))) => frequency,
        (None, None) => return fallback,
    };
    69.0 + 12.0 * (frequency / 440.0).log2()
}

fn voiced_edge_point(
    points: &[PitchPoint],
    time: f64,
    indices: impl Iterator<Item = usize>,
) -> Option<(f64, f32)> {
    indices
        .take_while(|index| {
            (points[*index].source_time_seconds - time).abs() <= MAX_EDGE_PITCH_GAP_SECONDS
        })
        .find_map(|index| {
            let point = &points[index];
            let frequency = point.frequency_hz?;
            (point.confidence >= MIN_EDGE_PITCH_CONFIDENCE
                && frequency.is_finite()
                && frequency > 0.0)
                .then_some((point.source_time_seconds, frequency))
        })
}

fn smooth_pitch_blend(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: u64, start: f64, end: f64, shift: f32) -> PitchNote {
        PitchNote {
            id,
            source_start: start,
            source_end: end,
            start,
            end,
            detected_midi: 57.0,
            shift,
            straighten: 0.0,
            pitch_start_bend: 0.0,
            pitch_anchor: 0.5,
            pitch_anchor_bend: 0.0,
            pitch_end_bend: 0.0,
        }
    }

    fn edit(notes: Vec<PitchNote>) -> PitchTimeEdit {
        PitchTimeEdit {
            detector_version: DETECTOR_VERSION,
            source_bytes: 0,
            source_modified_ns: 0,
            analysis: PitchAnalysis {
                sample_rate_hz: 16_000,
                source_start_seconds: 0.0,
                source_duration_seconds: 1.0,
                points: (0..=60)
                    .map(|index| PitchPoint {
                        source_time_seconds: index as f64 * 0.01,
                        frequency_hz: Some(220.0),
                        confidence: 1.0,
                    })
                    .collect(),
            },
            notes,
            rendered_source: None,
            enabled: true,
        }
    }

    #[test]
    fn boundary_pitch_prefers_the_unchanged_neighbor() {
        let unchanged_left = edit(vec![note(1, 0.1, 0.3, 0.0), note(2, 0.3, 0.6, 2.0)]);
        assert!((unchanged_left.pitch_midi_at(1, 57.0, 0.3, 120.0) - 57.0).abs() < 1e-5);
        assert!((unchanged_left.pitch_midi_at(1, 57.0, 0.425, 120.0) - 59.0).abs() < 1e-3);

        let unchanged_right = edit(vec![note(1, 0.1, 0.3, 2.0), note(2, 0.3, 0.6, 0.0)]);
        assert!((unchanged_right.pitch_midi_at(0, 57.0, 0.3, 120.0) - 57.0).abs() < 1e-5);
        assert!((unchanged_right.pitch_midi_at(0, 57.0, 0.175, 120.0) - 59.0).abs() < 1e-3);

        let both_edited = edit(vec![note(1, 0.1, 0.3, 2.0), note(2, 0.3, 0.6, 4.0)]);
        assert!((both_edited.pitch_midi_at(1, 57.0, 0.3, 120.0) - 60.0).abs() < 1e-5);
    }
}
