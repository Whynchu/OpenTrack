//! Event-wide pulse tracking and a continuous synthesis clock. All work here
//! runs during preparation, never in the device callback.
use super::*;

fn sample(samples: &[f32], channels: usize, channel: usize, frame: f64) -> f64 {
    let count = samples.len() / channels;
    let frame = frame.clamp(0.0, count.saturating_sub(1) as f64);
    let index = frame as usize;
    let fraction = frame - index as f64;
    let left = f64::from(samples[index * channels + channel]);
    let right = f64::from(samples[(index + 1).min(count - 1) * channels + channel]);
    left + (right - left) * fraction
}

fn correlation(
    samples: &[f32],
    channels: usize,
    channel: usize,
    anchor: f64,
    lag: f64,
    period: f64,
) -> f64 {
    let stride = (period / 48.0).max(1.0);
    let mut offset = -period * 0.5;
    let (mut sx, mut sy, mut xx, mut yy, mut xy, mut count) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    while offset <= period * 0.5 {
        let x = sample(samples, channels, channel, anchor + offset);
        let y = sample(samples, channels, channel, anchor + lag + offset);
        sx += x;
        sy += y;
        xx += x * x;
        yy += y * y;
        xy += x * y;
        count += 1.0;
        offset += stride;
    }
    let denominator = ((xx - sx * sx / count).max(0.0) * (yy - sy * sy / count).max(0.0)).sqrt();
    if denominator < 1e-12 {
        return -1.0;
    }
    (xy - sx * sy / count) / denominator
}

/// Fraction of the cycle energy that cannot be represented by its fundamental
/// sinusoid. Harmonic-rich sources need shorter PSOLA windows to avoid
/// formant/comb coloration; nearly sinusoidal sources tolerate longer windows.
fn cycle_complexity(
    samples: &[f32],
    channels: usize,
    channel: usize,
    anchor: f64,
    period: f64,
) -> f64 {
    const POINTS: usize = 24;
    let values: [f64; POINTS] = std::array::from_fn(|i| {
        sample(
            samples,
            channels,
            channel,
            anchor + period * i as f64 / POINTS as f64,
        )
    });
    let mean = values.iter().sum::<f64>() / POINTS as f64;
    let mut sine = 0.0;
    let mut cosine = 0.0;
    let mut total = 0.0;
    for (i, value) in values.into_iter().enumerate() {
        let centered = value - mean;
        let angle = std::f64::consts::TAU * i as f64 / POINTS as f64;
        sine += centered * angle.sin();
        cosine += centered * angle.cos();
        total += centered * centered;
    }
    if total < 1e-16 {
        return 0.0;
    }
    (1.0 - (2.0 * (sine * sine + cosine * cosine) / POINTS as f64 / total)).clamp(0.0, 1.0)
}

#[cfg(test)]
pub(super) fn source_cycle_complexity(
    samples: &[f32],
    rate: f64,
    channels: usize,
    edit: &PitchTimeEdit,
) -> f64 {
    let offset = edit.analysis.source_start_seconds * rate;
    let (hz, _) = estimate_pitch(edit, edit.analysis.source_start_seconds, 220.0);
    cycle_complexity(
        samples,
        channels,
        0,
        offset,
        rate / f64::from(hz.clamp(27.5, 3200.0)),
    )
}

pub(super) fn source_marks(
    samples: &[f32],
    rate: f64,
    channels: usize,
    edit: &PitchTimeEdit,
    cancel: &AtomicBool,
) -> Result<(Vec<f64>, usize), String> {
    let frames = samples.len() / channels;
    let offset = edit.analysis.source_start_seconds * rate;
    let first_frame = (offset - rate * 0.04).max(0.0);
    let last_frame = (offset + edit.analysis.source_duration_seconds * rate + rate * 0.04)
        .min(frames.saturating_sub(1) as f64);
    let channel = (0..channels)
        .max_by(|a, b| {
            let energy = |channel: usize| {
                (first_frame as usize..last_frame as usize)
                    .step_by(16)
                    .map(|i| samples[i * channels + channel].abs())
                    .sum::<f32>()
            };
            energy(*a).total_cmp(&energy(*b))
        })
        .unwrap_or(0);
    let (hz, _) = estimate_pitch(edit, first_frame / rate, 220.0);
    let period = rate / f64::from(hz.clamp(27.5, 3200.0));
    let initial_end = (first_frame + period).min(last_frame) as usize;
    let first = (first_frame as usize..initial_end)
        .max_by(|a, b| {
            samples[*a * channels + channel].total_cmp(&samples[*b * channels + channel])
        })
        .unwrap_or(first_frame as usize);
    let mut marks = vec![refine_positive_peak(
        samples,
        channels,
        channel,
        first as f64,
        frames,
    )];
    let maximum =
        (((last_frame - first_frame) / rate * 3200.0).ceil() as usize + 32).min(2_000_000);
    while *marks.last().unwrap() < last_frame && marks.len() < maximum {
        if marks.len() % 256 == 0 && cancel.load(Ordering::Acquire) {
            return Err("Pitch render canceled.".into());
        }
        let previous = *marks.last().unwrap();
        let (hz, _) = estimate_pitch(edit, previous / rate, 220.0);
        let mut period = rate / f64::from(hz.clamp(27.5, 3200.0));
        // Once waveform cycles agree, follow their measured period. An octave
        // flip in the coarse F0 trace must not double/halve the synthesis clock.
        // Reacquire from F0 when the waveform no longer supports that period.
        if marks.len() > 1 && voicing_presence(edit, previous / rate) > 0.5 {
            let tracked = previous - marks[marks.len() - 2];
            if correlation(samples, channels, channel, previous, tracked, tracked) > 0.9 {
                period = tracked;
            }
        }
        let mut lag = period;
        if voicing_presence(edit, previous / rate) > 0.5 {
            let score = |lag: f64| {
                correlation(samples, channels, channel, previous, lag, period)
                    - 0.03 * ((lag - period) / period).powi(2)
            };
            let step = (period / 100.0).max(1.0);
            let mut candidate = period * 0.8;
            let mut best = f64::NEG_INFINITY;
            while candidate <= period * 1.2 {
                let value = score(candidate);
                if value > best {
                    best = value;
                    lag = candidate;
                }
                candidate += step;
            }
            let left = score(lag - 1.0);
            let center = score(lag);
            let right = score(lag + 1.0);
            let curvature = left - 2.0 * center + right;
            if curvature.abs() > 1e-10 {
                lag += (0.5 * (left - right) / curvature).clamp(-0.5, 0.5);
            }
        }
        let predicted = previous + lag.max(1.0);
        let next = if voicing_presence(edit, predicted / rate) > 0.5 {
            let search_radius = (period * 0.25).max(1.0);
            let bounded = predicted.clamp(0.0, frames.saturating_sub(1) as f64);
            let low = (bounded - search_radius).floor().max(0.0) as usize;
            let high = (bounded + search_radius)
                .ceil()
                .min(frames.saturating_sub(1) as f64) as usize;
            let peak = (low..=high)
                .max_by(|a, b| {
                    samples[*a * channels + channel].total_cmp(&samples[*b * channels + channel])
                })
                .unwrap_or(bounded as usize) as f64;
            if samples[peak as usize * channels + channel].abs() > 1e-5 {
                refine_positive_peak(samples, channels, channel, peak, frames)
            } else {
                predicted
            }
        } else {
            predicted
        };
        marks.push(next.max(previous + 1.0));
    }
    Ok((marks, channel))
}

/// Destination-local seconds to source-local seconds, including unchanged gaps.
pub(super) fn source_mapping(edit: &PitchTimeEdit, time: f64) -> (f64, Option<usize>) {
    if time < 0.0 || time > edit.analysis.source_duration_seconds {
        return (time, None);
    }
    let index = edit.notes.partition_point(|note| note.end < time);
    if let Some(note) = edit.notes.get(index).filter(|note| time >= note.start) {
        return (note_source_time(note, time), Some(index));
    }
    let previous = index
        .checked_sub(1)
        .map(|i| (edit.notes[i].end, edit.notes[i].source_end))
        .unwrap_or((0.0, 0.0));
    let next = edit
        .notes
        .get(index)
        .map(|note| (note.start, note.source_start))
        .unwrap_or((
            edit.analysis.source_duration_seconds,
            edit.analysis.source_duration_seconds,
        ));
    let fraction = ((time - previous.0) / (next.0 - previous.0).max(1e-9)).clamp(0.0, 1.0);
    (previous.1 + fraction * (next.1 - previous.1), None)
}

fn voicing_presence(edit: &PitchTimeEdit, time: f64) -> f32 {
    let points = &edit.analysis.points;
    if points.is_empty() {
        return 0.0;
    }
    let right = points
        .partition_point(|p| p.source_time_seconds < time)
        .min(points.len() - 1);
    let left = right.saturating_sub(1);
    let l = if voiced_point(&points[left]).is_some() {
        1.0
    } else {
        0.0
    };
    let r = if voiced_point(&points[right]).is_some() {
        1.0
    } else {
        0.0
    };
    let lt = points[left].source_time_seconds;
    let rt = points[right].source_time_seconds;
    if left != right && rt - lt <= MAX_PITCH_GAP_SECONDS && time >= lt && time <= rt {
        let amount = smoothstep(((time - lt) / (rt - lt).max(1e-9)).clamp(0.0, 1.0) as f32);
        return l * (1.0 - amount) + r * amount;
    }
    let (distance, value) = if (time - lt).abs() < (time - rt).abs() {
        ((time - lt).abs(), l)
    } else {
        ((time - rt).abs(), r)
    };
    value * smoothstep((1.0 - distance / MAX_PITCH_GAP_SECONDS).clamp(0.0, 1.0) as f32)
}

pub(super) fn make_event_grains(
    samples: &[f32],
    rate: f64,
    channels: usize,
    edit: &PitchTimeEdit,
    tempo: f64,
    cancel: &AtomicBool,
) -> Result<Vec<Grain>, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Pitch render canceled.".into());
    }
    let (marks, channel) = source_marks(samples, rate, channels, edit, cancel)?;
    if marks.len() < 2 {
        return Ok(Vec::new());
    }
    let offset = edit.analysis.source_start_seconds * rate;
    let unchanged = !edit.notes.iter().any(note_is_processed);
    let nearest_mark = |source: f64| {
        let right = marks
            .partition_point(|mark| *mark < source)
            .min(marks.len() - 1);
        if right > 0 && (marks[right - 1] - source).abs() < (marks[right] - source).abs() {
            right - 1
        } else {
            right
        }
    };
    // (output pulse, source pulse index, voiced/unvoiced presence). Confidence
    // chooses whether a source frame is voiced; it is never a permanent dry/wet
    // knob that leaves the original pitch sounding alongside a shifted vowel.
    let mut pulses = Vec::<(f64, usize, f32)>::new();
    if unchanged {
        pulses.extend(
            marks
                .iter()
                .enumerate()
                .map(|(i, mark)| (*mark, i, voicing_presence(edit, *mark / rate))),
        );
    } else {
        let mut destination = marks[0];
        let last = *marks.last().unwrap();
        let mut phase = 0.0;
        let source_phase = |source: f64| {
            let right = marks
                .partition_point(|mark| *mark <= source)
                .clamp(1, marks.len() - 1);
            (right - 1) as f64
                + (source - marks[right - 1]) / (marks[right] - marks[right - 1]).max(1.0)
        };
        let mut iterations = 0usize;
        pulses.push((destination, 0, voicing_presence(edit, destination / rate)));
        while destination < last {
            if iterations % 256 == 0 && cancel.load(Ordering::Acquire) {
                return Err("Pitch render canceled.".into());
            }
            iterations += 1;
            let local = (destination - offset) / rate;
            let (source_local, _) = source_mapping(edit, local);
            let source = offset + source_local * rate;
            let right = marks
                .partition_point(|mark| *mark <= source)
                .clamp(1, marks.len() - 1);
            let source_period = (marks[right] - marks[right - 1]).max(1.0);
            // Integrate measured source phase in short intervals. At most half
            // a millisecond and one eighth of a cycle keeps steep ramps resolved
            // without evaluating every source sample during a UI edit.
            let step = (source_period / 8.0)
                .min(rate * 0.0005)
                .max(1.0)
                .min(last - destination);
            let middle = (destination + step * 0.5 - offset) / rate;
            let (middle_source, index) = source_mapping(edit, middle);
            let middle_source = offset + middle_source * rate;
            let source_end =
                offset + source_mapping(edit, (destination + step - offset) / rate).0 * rate;
            let cycles = if (source_end - source).abs() > 1e-9 {
                (source_phase(source_end) - source_phase(source)) / (source_end - source) * step
            } else {
                step / source_period
            };
            let (detected, _) = estimate_pitch(edit, middle_source / rate, 220.0);
            let midi = 69.0 + 12.0 * (detected / 440.0).log2();
            let target = index.map_or(midi, |i| edit.pitch_midi_at(i, midi, middle, tempo));
            let ratio = 2.0_f64.powf(f64::from(target - midi) / 12.0);
            let increment = (ratio * cycles).clamp(1e-8, 0.95);
            if phase + increment >= 1.0 {
                let fraction = (1.0 - phase) / increment;
                let pulse = destination + fraction * step;
                let mapped = offset + source_mapping(edit, (pulse - offset) / rate).0 * rate;
                pulses.push((
                    pulse,
                    nearest_mark(mapped),
                    voicing_presence(edit, mapped / rate),
                ));
                phase = phase + increment - 1.0;
            } else {
                phase += increment;
            }
            destination += step;
        }
    }
    let complexity: Vec<f64> = marks
        .windows(2)
        .map(|pair| cycle_complexity(samples, channels, channel, pair[0], pair[1] - pair[0]))
        .collect();
    let mut grains = Vec::with_capacity(pulses.len());
    for (i, &(destination, source_index, voicing)) in pulses.iter().enumerate() {
        let source = marks[source_index];
        let sl = if source_index > 0 {
            source - marks[source_index - 1]
        } else {
            marks[1] - source
        };
        let sr = if source_index + 1 < marks.len() {
            marks[source_index + 1] - source
        } else {
            sl
        };
        let dl = if i > 0 {
            destination - pulses[i - 1].0
        } else {
            sl
        };
        let dr = if i + 1 < pulses.len() {
            pulses[i + 1].0 - destination
        } else {
            dl
        };
        let first = source_index.saturating_sub(8);
        let last = (source_index + 8).min(complexity.len().saturating_sub(1));
        let harmonic_content = if complexity.is_empty() {
            0.0
        } else {
            complexity[first..=last].iter().sum::<f64>() / (last - first + 1) as f64
        };
        let complex_amount = ((harmonic_content - 0.05) / 0.20).clamp(0.0, 1.0);
        let local_time = (destination - offset) / rate;
        let time_expanded = source_mapping(edit, local_time)
            .1
            .is_some_and(|note_index| {
                let note = &edit.notes[note_index];
                let source_length = note.source_end - note.source_start;
                let destination_length = note.end - note.start;
                destination_length > source_length + rate.recip() * 2.0
            });
        // When extending a source into a longer destination, adjacent target
        // pulses can reuse the same source cycle; keep those windows short to
        // avoid destructive overlap. Compression retains the longer adaptive
        // window, which suppresses modulation sidebands.
        let window_scale = if time_expanded {
            0.5
        } else {
            2.0 - complex_amount
        };
        grains.push(Grain {
            destination,
            source,
            left_radius: (window_scale * sl.max(dl)).max(1.0),
            right_radius: (window_scale * sr.max(dr)).max(1.0),
            voicing,
        });
    }
    Ok(grains)
}
