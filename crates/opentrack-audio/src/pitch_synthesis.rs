//! Shared, allocation-free pitch sampling for live playback and offline output.
use opentrack_core::pitch_time::{PitchNote, PitchTimeEdit};
use std::sync::atomic::{AtomicBool, Ordering};

const MIN_VOICED_CONFIDENCE: f32 = 0.15;
const MAX_PITCH_GAP_SECONDS: f64 = 0.025;
const NOTE_SEAM_BLEND_SECONDS: f64 = 0.005;
const UNCONNECTED_EDGE_BLEND_SECONDS: f64 = 0.005;
const GRAIN_SEARCH_LIMIT: usize = 64;

#[derive(Clone, Debug)]
struct Grain {
    destination: f64,
    source: f64,
    radius: f64,
    voicing: f32,
}

#[derive(Debug)]
struct NotePlan {
    note: PitchNote,
    grains: Vec<Grain>,
    processed: bool,
    time_changed: bool,
    edge_only: bool,
    incoming_pitch_seconds: f64,
    outgoing_pitch_seconds: f64,
}

#[derive(Clone, Copy, Debug)]
struct NoteTransition {
    left: usize,
    right: usize,
    start_frame: f64,
    end_frame: f64,
}

#[derive(Debug)]
pub(crate) struct PitchSynthesis {
    edit: PitchTimeEdit,
    sample_rate: f64,
    channels: usize,
    frame_count: usize,
    plans: Vec<NotePlan>,
    transitions: Vec<NoteTransition>,
}

impl PitchSynthesis {
    pub(crate) fn new(
        samples: &[f32],
        sample_rate: u32,
        channels: usize,
        edit: &PitchTimeEdit,
        tempo_bpm: f64,
        cancel: &AtomicBool,
    ) -> Result<Self, String> {
        if !(1..=2).contains(&channels)
            || sample_rate == 0
            || samples.is_empty()
            || samples.len() % channels != 0
            || !edit.validate(edit.analysis.source_duration_seconds)
        {
            return Err("Invalid live pitch/time data".into());
        }

        let rate = f64::from(sample_rate);
        let offset = edit.analysis.source_start_seconds;
        let frame_count = samples.len() / channels;
        let tempo_bpm = if tempo_bpm.is_finite() && tempo_bpm > 0.0 {
            tempo_bpm
        } else {
            120.0
        };
        let available_seconds = frame_count as f64 / rate;
        if offset + edit.analysis.source_duration_seconds > available_seconds + 1.0 / rate {
            return Err("Pitch/time analysis range exceeds its audio source".into());
        }
        let mut transitions = Vec::new();
        let mut extend_left = vec![0.0_f64; edit.notes.len()];
        let mut extend_right = vec![0.0_f64; edit.notes.len()];
        let mut incoming_pitch_seconds = vec![0.0_f64; edit.notes.len()];
        let mut outgoing_pitch_seconds = vec![0.0_f64; edit.notes.len()];
        let mut edge_render = vec![false; edit.notes.len()];
        let sixteenth_seconds = 15.0 / tempo_bpm;

        for (left_index, pair) in edit.notes.windows(2).enumerate() {
            let left = &pair[0];
            let right = &pair[1];
            let tolerance = (1.0 / rate).max(1e-6);
            if (left.end - right.start).abs() > tolerance
                || (!note_is_processed(left) && !note_is_processed(right))
            {
                continue;
            }

            let has_pitch_transition = note_has_pitch_edit(left) || note_has_pitch_edit(right);
            let left_width = if has_pitch_transition {
                sixteenth_seconds.min((left.end - left.start) * 0.49)
            } else {
                NOTE_SEAM_BLEND_SECONDS
            };
            let right_width = if has_pitch_transition {
                sixteenth_seconds.min((right.end - right.start) * 0.49)
            } else {
                NOTE_SEAM_BLEND_SECONDS
            };
            if left_width <= 0.0 || right_width <= 0.0 {
                continue;
            }

            if has_pitch_transition {
                edge_render[left_index] = true;
                edge_render[left_index + 1] = true;
                outgoing_pitch_seconds[left_index] = left_width;
                incoming_pitch_seconds[left_index + 1] = right_width;
            }

            let boundary = (offset + left.end) * rate;
            transitions.push(NoteTransition {
                left: left_index,
                right: left_index + 1,
                start_frame: boundary - left_width * rate,
                end_frame: boundary + right_width * rate,
            });
            // Each note must supply audio across the entire transition window:
            // the left note reaches through the right edge, and vice versa.
            extend_right[left_index] = right_width;
            extend_left[left_index + 1] = left_width;
        }

        let mut plans = Vec::with_capacity(edit.notes.len());
        for (index, note) in edit.notes.iter().enumerate() {
            if cancel.load(Ordering::Acquire) {
                return Err("Pitch render canceled.".into());
            }
            let own_edit = note_is_processed(note);
            let processed = own_edit || edge_render[index];
            let grains = if processed {
                make_grains(
                    samples,
                    rate,
                    channels,
                    edit,
                    index,
                    note,
                    extend_left[index],
                    extend_right[index],
                    tempo_bpm,
                    cancel,
                )?
            } else {
                Vec::new()
            };
            plans.push(NotePlan {
                note: note.clone(),
                grains,
                processed,
                time_changed: note_time_changed(note),
                edge_only: !own_edit,
                incoming_pitch_seconds: incoming_pitch_seconds[index],
                outgoing_pitch_seconds: outgoing_pitch_seconds[index],
            });
        }

        Ok(Self {
            edit: edit.clone(),
            sample_rate: rate,
            channels,
            frame_count,
            plans,
            transitions,
        })
    }

    pub(crate) fn sample_frame(&self, samples: &[f32], frame: u64) -> Option<(f32, f32)> {
        let frame_index = usize::try_from(frame).ok()?;
        if frame_index >= self.frame_count || samples.len() < self.frame_count * self.channels {
            return None;
        }
        // Callback work is read-only scalar math over immutable prepared arrays.
        let frame = frame as f64;
        let offset = self.edit.analysis.source_start_seconds;
        let local_time = frame / self.sample_rate - offset;
        if local_time < 0.0 || local_time > self.edit.analysis.source_duration_seconds {
            return Some(self.at(samples, frame));
        }
        let base = self.base_at(samples, local_time);

        let transition_index = self
            .transitions
            .partition_point(|transition| transition.end_frame < frame);
        if let Some(transition) = self
            .transitions
            .get(transition_index)
            .filter(|transition| frame >= transition.start_frame && frame <= transition.end_frame)
        {
            let left = self.note_at(samples, transition.left, frame, false);
            let right = self.note_at(samples, transition.right, frame, false);
            let position = ((frame - transition.start_frame)
                / (transition.end_frame - transition.start_frame).max(1.0))
            .clamp(0.0, 1.0) as f32;
            let blend = smoothstep(position);
            return Some(mix(left, right, blend));
        }

        let note_index = self
            .plans
            .partition_point(|plan| plan.note.end < local_time);
        let plan = self
            .plans
            .get(note_index)
            .filter(|plan| local_time >= plan.note.start && local_time <= plan.note.end);
        let plan = plan.filter(|plan| {
            !plan.edge_only
                || (plan.incoming_pitch_seconds > 0.0
                    && local_time <= plan.note.start + plan.incoming_pitch_seconds)
                || (plan.outgoing_pitch_seconds > 0.0
                    && local_time >= plan.note.end - plan.outgoing_pitch_seconds)
        });
        Some(plan.map_or(base, |_| self.note_at(samples, note_index, frame, true)))
    }

    fn note_at(
        &self,
        samples: &[f32],
        index: usize,
        frame: f64,
        fade_unconnected_edge: bool,
    ) -> (f32, f32) {
        let Some(plan) = self.plans.get(index) else {
            return self.at(samples, frame);
        };
        let offset = self.edit.analysis.source_start_seconds;
        let local_time = frame / self.sample_rate - offset;
        let source_local = note_source_time(&plan.note, local_time);
        let base = self.at(samples, (offset + source_local) * self.sample_rate);
        if !plan.processed || plan.grains.is_empty() {
            return base;
        }

        let center = plan
            .grains
            .partition_point(|grain| grain.destination < frame);
        let first = center.saturating_sub(GRAIN_SEARCH_LIMIT);
        let last = (center + GRAIN_SEARCH_LIMIT).min(plan.grains.len());
        let mut sum = (0.0_f32, 0.0_f32);
        let mut voiced_weight = 0.0_f32;
        let mut total_weight = 0.0_f32;
        for grain in &plan.grains[first..last] {
            let delta = frame - grain.destination;
            if delta.abs() >= grain.radius {
                continue;
            }
            let window = (0.5 + 0.5 * (std::f64::consts::PI * delta / grain.radius).cos()) as f32;
            total_weight += window;
            let weight = window * grain.voicing;
            if weight <= 1e-7 {
                continue;
            }
            let sample = self.at(samples, grain.source + delta);
            sum.0 += weight * sample.0;
            sum.1 += weight * sample.1;
            voiced_weight += weight;
        }

        let mut result = if voiced_weight > 1e-6 {
            let wet = (sum.0 / voiced_weight, sum.1 / voiced_weight);
            let mut voice = (voiced_weight / total_weight.max(1e-6)).clamp(0.0, 1.0);
            // A time-compressed dry signal has a higher pitch than the source.
            // Mixing it equally with pitch-synchronous grains causes beating
            // and a bubbly tone, so favor the voiced grains during time edits.
            if plan.time_changed {
                voice = voice.sqrt();
            }
            mix(base, wet, voice)
        } else {
            base
        };

        if fade_unconnected_edge {
            let edge_seconds = (local_time - plan.note.start)
                .min(plan.note.end - local_time)
                .max(0.0);
            let position = (edge_seconds / UNCONNECTED_EDGE_BLEND_SECONDS).clamp(0.0, 1.0) as f32;
            result = mix(base, result, smoothstep(position));
        }
        result
    }

    fn base_at(&self, samples: &[f32], local_time: f64) -> (f32, f32) {
        let offset = self.edit.analysis.source_start_seconds;
        let index = self
            .plans
            .partition_point(|plan| plan.note.end < local_time);
        let mut previous = (0.0, 0.0);
        if index > 0 {
            let note = &self.plans[index - 1].note;
            previous = (note.end, note.source_end);
        }
        let next = self.plans.get(index).map_or(
            (
                self.edit.analysis.source_duration_seconds,
                self.edit.analysis.source_duration_seconds,
            ),
            |plan| (plan.note.start, plan.note.source_start),
        );
        let mapped =
            if let Some(plan) = self.plans.get(index).filter(|p| local_time >= p.note.start) {
                note_source_time(&plan.note, local_time)
            } else {
                previous.1
                    + ((local_time - previous.0) / (next.0 - previous.0).max(1e-9)).clamp(0.0, 1.0)
                        * (next.1 - previous.1)
            };
        self.at(samples, (offset + mapped) * self.sample_rate)
    }

    fn at(&self, samples: &[f32], frame: f64) -> (f32, f32) {
        let count = self.frame_count;
        if count == 0 {
            return (0.0, 0.0);
        }
        let frame = frame.clamp(0.0, (count - 1) as f64);
        let i = frame as usize;
        let t = (frame - i as f64) as f32;
        let read = |channel: usize| {
            let channel = channel.min(self.channels - 1);
            let a = samples[i * self.channels + channel];
            let b = samples[(i + 1).min(count - 1) * self.channels + channel];
            a * (1.0 - t) + b * t
        };
        let left = read(0);
        (left, if self.channels == 1 { left } else { read(1) })
    }
}

fn note_is_processed(note: &PitchNote) -> bool {
    note.shift.abs() > 1e-6
        || note.straighten > 1e-6
        || (note.start - note.source_start).abs() > 1e-8
        || (note.end - note.source_end).abs() > 1e-8
}

fn note_source_time(note: &PitchNote, destination_time: f64) -> f64 {
    note.source_start
        + ((destination_time - note.start) / (note.end - note.start))
            * (note.source_end - note.source_start)
}

fn make_grains(
    samples: &[f32],
    rate: f64,
    channels: usize,
    edit: &PitchTimeEdit,
    note_index: usize,
    note: &PitchNote,
    extend_left_seconds: f64,
    extend_right_seconds: f64,
    tempo_bpm: f64,
    cancel: &AtomicBool,
) -> Result<Vec<Grain>, String> {
    let offset = edit.analysis.source_start_seconds;
    let frame_count = samples.len() / channels;
    let source_per_destination =
        (note.source_end - note.source_start) / (note.end - note.start).max(1e-9);
    let render_start = note.start - extend_left_seconds;
    let render_end = note.end + extend_right_seconds;
    let source_start = (offset + note_source_time(note, render_start)).max(0.0) * rate;
    let source_end = (offset + note_source_time(note, render_end))
        .min((frame_count.saturating_sub(1)) as f64 / rate)
        * rate;
    if source_end <= source_start {
        return Ok(Vec::new());
    }

    let reference_channel = (0..channels)
        .max_by(|a, b| {
            let energy = |channel: usize| {
                (source_start.max(0.0) as usize..source_end.max(0.0) as usize)
                    .step_by(8)
                    .map(|frame| samples[frame * channels + channel].abs())
                    .sum::<f32>()
            };
            energy(*a).total_cmp(&energy(*b))
        })
        .unwrap_or(0);

    let fallback = midi_hz(note.detected_midi);
    let mut marks = Vec::<f64>::new();
    let mut position = source_start;
    let mut iterations = 0usize;
    let max_iterations =
        (((source_end - source_start) / rate * 1800.0).ceil() as usize).saturating_add(16);
    while position < source_end + rate / 55.0 && iterations < max_iterations {
        if cancel.load(Ordering::Acquire) {
            return Err("Pitch render canceled.".into());
        }
        iterations += 1;
        let (detected, _) = estimate_pitch(edit, position / rate, fallback);
        let period = rate / f64::from(detected);
        let radius = period * if marks.is_empty() { 0.5 } else { 0.2 };
        let low = (position - radius).floor().max(0.0) as usize;
        let high = (position + radius)
            .ceil()
            .max(low as f64 + 1.0)
            .min(frame_count as f64) as usize;
        let peak = (low..high)
            .max_by(|a, b| {
                samples[*a * channels + reference_channel]
                    .total_cmp(&samples[*b * channels + reference_channel])
            })
            .unwrap_or(position as usize) as f64;
        let mark = refine_positive_peak(samples, channels, reference_channel, peak, frame_count);
        if marks.last().is_none_or(|last| mark > *last + 1.0) {
            marks.push(mark);
        }
        position = peak + period;
    }
    if marks.is_empty() {
        return Ok(Vec::new());
    }

    let mut grains = Vec::new();
    let mut destination = (offset + render_start) * rate;
    let render_end_frame = (offset + render_end) * rate;
    let max_grains = ((render_end_frame - destination).max(0.0) / (rate / 27.5)).ceil() as usize
        + ((render_end_frame - destination).max(0.0) / (rate / 3200.0)).ceil() as usize;
    let max_grains = max_grains.min(2_000_000).max(1);
    while destination < render_end_frame && grains.len() < max_grains {
        if grains.len() % 2048 == 0 && cancel.load(Ordering::Acquire) {
            return Err("Pitch render canceled.".into());
        }
        let local_time = destination / rate - offset;
        let source_local = note_source_time(note, local_time);
        let source = (offset + source_local) * rate;
        let (detected, voicing) = estimate_pitch(edit, source / rate, fallback);
        let source_midi = 69.0 + 12.0 * (f64::from(detected) / 440.0).log2() as f32;
        let target_midi = edit.pitch_midi_at(note_index, source_midi, local_time, tempo_bpm);
        let target_hz = midi_hz(target_midi).clamp(27.5, 3200.0);
        let mark_index = marks
            .partition_point(|mark| *mark < source)
            .min(marks.len() - 1);
        let mark = if mark_index > 0
            && (marks[mark_index - 1] - source).abs() < (marks[mark_index] - source).abs()
        {
            marks[mark_index - 1]
        } else {
            marks[mark_index]
        };
        grains.push(Grain {
            destination,
            source: mark,
            // Keep neighboring grains overlapped as source time advances faster
            // than destination time. Without this, a compressed note can leave
            // audible holes between pitch-synchronous grains.
            radius: (rate / f64::from(detected)).max(
                (source_per_destination * rate / f64::from(target_hz) * 0.65)
                    .min(rate / f64::from(detected) * 4.0),
            ),
            voicing,
        });
        destination += rate / f64::from(target_hz);
    }
    Ok(grains)
}

fn note_has_pitch_edit(note: &PitchNote) -> bool {
    note.shift.abs() > 1e-6 || note.straighten > 1e-6
}

fn note_time_changed(note: &PitchNote) -> bool {
    (note.start - note.source_start).abs() > 1e-8 || (note.end - note.source_end).abs() > 1e-8
}

fn estimate_pitch(edit: &PitchTimeEdit, time: f64, fallback: f32) -> (f32, f32) {
    let points = &edit.analysis.points;
    if points.is_empty() {
        return (fallback, 0.0);
    }
    let right = points.partition_point(|point| point.source_time_seconds < time);
    let left_index = right.saturating_sub(1).min(points.len() - 1);
    let right_index = right.min(points.len() - 1);
    let left = voiced_point(&points[left_index]);
    let right = voiced_point(&points[right_index]);

    match (left, right) {
        (Some((left_hz, left_confidence)), Some((right_hz, right_confidence)))
            if left_index == right_index
                || points[right_index].source_time_seconds
                    - points[left_index].source_time_seconds
                    <= MAX_PITCH_GAP_SECONDS =>
        {
            let left_time = points[left_index].source_time_seconds;
            let right_time = points[right_index].source_time_seconds;
            let fraction = if right_index == left_index {
                0.0
            } else {
                ((time - left_time) / (right_time - left_time).max(1e-9)).clamp(0.0, 1.0) as f32
            };
            let hz = (left_hz.log2() * (1.0 - fraction) + right_hz.log2() * fraction).exp2();
            let confidence = left_confidence * (1.0 - fraction) + right_confidence * fraction;
            (hz, confidence.clamp(0.0, 1.0))
        }
        (Some((hz, confidence)), None) => {
            if left_index != right_index {
                let left_time = points[left_index].source_time_seconds;
                let right_time = points[right_index].source_time_seconds;
                let gap = right_time - left_time;
                if gap > 0.0 && gap <= MAX_PITCH_GAP_SECONDS {
                    let fraction = ((time - left_time) / gap).clamp(0.0, 1.0) as f32;
                    return (hz, confidence * (1.0 - fraction));
                }
            }
            let distance = (time - points[left_index].source_time_seconds).abs();
            let weight = (1.0 - distance / MAX_PITCH_GAP_SECONDS).clamp(0.0, 1.0) as f32;
            (hz, confidence * weight)
        }
        (None, Some((hz, confidence))) => {
            if left_index != right_index {
                let left_time = points[left_index].source_time_seconds;
                let right_time = points[right_index].source_time_seconds;
                let gap = right_time - left_time;
                if gap > 0.0 && gap <= MAX_PITCH_GAP_SECONDS {
                    let fraction = ((time - left_time) / gap).clamp(0.0, 1.0) as f32;
                    return (hz, confidence * fraction);
                }
            }
            let distance = (time - points[right_index].source_time_seconds).abs();
            let weight = (1.0 - distance / MAX_PITCH_GAP_SECONDS).clamp(0.0, 1.0) as f32;
            (hz, confidence * weight)
        }
        _ => (fallback, 0.0),
    }
}

fn voiced_point(point: &opentrack_core::pitch_time::PitchPoint) -> Option<(f32, f32)> {
    let hz = point.frequency_hz?;
    (point.confidence >= MIN_VOICED_CONFIDENCE).then_some((hz, point.confidence))
}

fn refine_positive_peak(
    samples: &[f32],
    channels: usize,
    channel: usize,
    peak: f64,
    frame_count: usize,
) -> f64 {
    let index = peak as usize;
    if index == 0 || index + 1 >= frame_count {
        return peak;
    }
    let left = samples[(index - 1) * channels + channel];
    let center = samples[index * channels + channel];
    let right = samples[(index + 1) * channels + channel];
    let denominator = left - 2.0 * center + right;
    if denominator.abs() < 1e-9 {
        return peak;
    }
    let fraction = (0.5 * (left - right) / denominator).clamp(-0.5, 0.5);
    index as f64 + f64::from(fraction)
}

fn midi_hz(midi: f32) -> f32 {
    440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn mix(a: (f32, f32), b: (f32, f32), amount: f32) -> (f32, f32) {
    (
        a.0 * (1.0 - amount) + b.0 * amount,
        a.1 * (1.0 - amount) + b.1 * amount,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::pitch_time::{PitchAnalysis, PitchPoint};

    #[test]
    fn estimate_pitch_interpolates_in_log_frequency_and_fades_unvoiced_frames() {
        let edit = PitchTimeEdit {
            detector_version: 2,
            source_bytes: 0,
            source_modified_ns: 0,
            analysis: PitchAnalysis {
                sample_rate_hz: 16000,
                source_start_seconds: 0.0,
                source_duration_seconds: 0.04,
                points: vec![
                    PitchPoint {
                        source_time_seconds: 0.0,
                        frequency_hz: Some(220.0),
                        confidence: 1.0,
                    },
                    PitchPoint {
                        source_time_seconds: 0.01,
                        frequency_hz: Some(440.0),
                        confidence: 1.0,
                    },
                    PitchPoint {
                        source_time_seconds: 0.02,
                        frequency_hz: None,
                        confidence: 0.0,
                    },
                    PitchPoint {
                        source_time_seconds: 0.03,
                        frequency_hz: None,
                        confidence: 0.0,
                    },
                ],
            },
            notes: Vec::new(),
            rendered_source: None,
            enabled: true,
        };
        let (middle, voice) = estimate_pitch(&edit, 0.005, 330.0);
        assert!((middle - (220.0_f32 * 440.0).sqrt()).abs() < 0.01);
        assert!(voice > 0.99);
        let (fade, fade_voice) = estimate_pitch(&edit, 0.015, 330.0);
        assert!((fade - 440.0).abs() < 0.01);
        assert!((0.0..1.0).contains(&fade_voice));
        let (_, unvoiced_edge) = estimate_pitch(&edit, 0.02, 330.0);
        assert_eq!(unvoiced_edge, 0.0);
        let (unvoiced, unvoiced_voice) = estimate_pitch(&edit, 0.025, 330.0);
        assert_eq!(unvoiced, 330.0);
        assert_eq!(unvoiced_voice, 0.0);
    }

    #[test]
    fn touching_changed_notes_get_bounded_crossfade_and_real_gaps_are_untouched() {
        let note = |id, start, end, shift| PitchNote {
            id,
            source_start: start,
            source_end: end,
            start,
            end,
            detected_midi: 60.0,
            shift,
            straighten: 0.0,
        };
        let make_edit = |notes| PitchTimeEdit {
            detector_version: 2,
            source_bytes: 0,
            source_modified_ns: 0,
            analysis: PitchAnalysis {
                sample_rate_hz: 16000,
                source_start_seconds: 0.0,
                source_duration_seconds: 1.0,
                points: vec![PitchPoint {
                    source_time_seconds: 0.0,
                    frequency_hz: Some(220.0),
                    confidence: 1.0,
                }],
            },
            notes,
            rendered_source: None,
            enabled: true,
        };
        let touched = make_edit(vec![note(1, 0.1, 0.3, 1.0), note(2, 0.3, 0.6, -1.0)]);
        let touching = PitchSynthesis::new(
            &vec![0.0; 16000],
            16000,
            1,
            &touched,
            120.0,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(touching.transitions.len(), 1);
        let transition = touching.transitions[0];
        assert!(((0.3 * 16000.0 - transition.start_frame) - 16000.0 * 0.098).abs() < 1e-6);
        assert!(((transition.end_frame - 0.3 * 16000.0) - 16000.0 * 0.125).abs() < 1e-6);
        assert!(touching.plans[0].grains.last().unwrap().destination > 0.3 * 16000.0);
        assert!(touching.plans[1].grains.first().unwrap().destination < 0.3 * 16000.0);

        let gap = make_edit(vec![note(1, 0.1, 0.29, 1.0), note(2, 0.31, 0.6, -1.0)]);
        let separated = PitchSynthesis::new(
            &vec![0.0; 16000],
            16000,
            1,
            &gap,
            120.0,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(separated.transitions.is_empty());
    }

    #[test]
    fn audio_outside_the_analyzed_event_passes_through_unchanged() {
        let rate = 16_000;
        let channels = 2;
        let samples: Vec<_> = (0..rate * 2)
            .flat_map(|frame| {
                let value = (std::f32::consts::TAU * 220.0 * frame as f32 / rate as f32).sin();
                [value, value * 0.37]
            })
            .collect();
        let edit = PitchTimeEdit {
            detector_version: 2,
            source_bytes: 0,
            source_modified_ns: 0,
            analysis: PitchAnalysis {
                sample_rate_hz: rate,
                source_start_seconds: 0.5,
                source_duration_seconds: 0.5,
                points: (0..50)
                    .map(|index| PitchPoint {
                        source_time_seconds: 0.5 + index as f64 * 0.01,
                        frequency_hz: Some(220.0),
                        confidence: 1.0,
                    })
                    .collect(),
            },
            notes: vec![PitchNote {
                id: 1,
                source_start: 0.1,
                source_end: 0.4,
                start: 0.1,
                end: 0.4,
                detected_midi: 57.0,
                shift: 3.0,
                straighten: 0.0,
            }],
            rendered_source: None,
            enabled: true,
        };
        let renderer = PitchSynthesis::new(
            &samples,
            rate,
            channels,
            &edit,
            120.0,
            &AtomicBool::new(false),
        )
        .unwrap();
        for frame in [100, 8_000, 20_000, 28_000] {
            let (left, right) = renderer.sample_frame(&samples, frame).unwrap();
            assert_eq!(left, samples[frame as usize * channels]);
            assert_eq!(right, samples[frame as usize * channels + 1]);
        }
    }
}
