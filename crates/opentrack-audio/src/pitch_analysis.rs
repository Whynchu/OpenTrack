//! Explicit, offline monophonic pitch analysis for the Sample Editor.
//!
//! This is a Rust-native YIN-style detector, deliberately separate from the
//! audio callback. Results are estimates, not pitch correction or note edits.

use std::{
    io::{Read, Seek},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

const ANALYSIS_RATE: u32 = 16_000;
const WINDOW: usize = 1_536;
const HOP: usize = 160; // 10 ms, rather than the previous 50 ms.
const MIN_HZ: f32 = 55.0;
const MAX_HZ: f32 = 1_600.0;
const YIN_THRESHOLD: f32 = 0.15;
pub use opentrack_core::pitch_time::{PitchAnalysis, PitchPoint};

/// Analyze only the requested source interval. Call from a worker thread.
pub fn analyze_pitch_file(
    path: &Path,
    source_start_seconds: f64,
    duration_seconds: f64,
    cancelled: &AtomicBool,
) -> Result<PitchAnalysis, String> {
    if !source_start_seconds.is_finite()
        || !duration_seconds.is_finite()
        || source_start_seconds < 0.0
        || duration_seconds <= 0.0
        || duration_seconds > 3600.0
    {
        return Err("The audio event has an invalid source range.".to_owned());
    }
    let mut reader = hound::WavReader::open(path)
        .map_err(|error| format!("Could not open audio for pitch analysis: {error}"))?;
    let spec = reader.spec();
    if !(1..=2).contains(&spec.channels) || spec.sample_rate < 8_000 {
        return Err("Pitch analysis supports mono or stereo WAV sources.".to_owned());
    }
    let first_frame = (source_start_seconds * f64::from(spec.sample_rate)).floor() as u64;
    let requested_frames = (duration_seconds * f64::from(spec.sample_rate)).ceil() as u64;
    let end_frame = first_frame
        .saturating_add(requested_frames)
        .min(u64::from(reader.duration()));
    if first_frame >= end_frame {
        return Err("The audio event has no source frames to analyze.".to_owned());
    }
    let mono = match spec.sample_format {
        hound::SampleFormat::Int => read_resampled_mono::<_, i32>(
            &mut reader,
            first_frame,
            end_frame,
            spec.channels,
            spec.sample_rate,
            2.0_f64.powi(i32::from(spec.bits_per_sample) - 1),
            cancelled,
        )?,
        hound::SampleFormat::Float => read_resampled_mono::<_, f32>(
            &mut reader,
            first_frame,
            end_frame,
            spec.channels,
            spec.sample_rate,
            1.0,
            cancelled,
        )?,
    };
    if cancelled.load(Ordering::Acquire) {
        return Err("Pitch analysis canceled.".to_owned());
    }
    let points = detect_pitch(&mono, source_start_seconds, cancelled);
    if cancelled.load(Ordering::Acquire) {
        return Err("Pitch analysis canceled.".to_owned());
    }
    Ok(PitchAnalysis {
        sample_rate_hz: ANALYSIS_RATE,
        source_start_seconds,
        source_duration_seconds: (end_frame - first_frame) as f64 / f64::from(spec.sample_rate),
        points,
    })
}

fn read_resampled_mono<R, S>(
    reader: &mut hound::WavReader<R>,
    first_frame: u64,
    end_frame: u64,
    channels: u16,
    source_rate: u32,
    scale: f64,
    cancelled: &AtomicBool,
) -> Result<Vec<f32>, String>
where
    R: Read + Seek,
    S: hound::Sample + Into<f64>,
{
    let channel_count = usize::from(channels);
    let output_len = ((end_frame - first_frame) * u64::from(ANALYSIS_RATE) / u64::from(source_rate))
        .saturating_add(2)
        .min(ANALYSIS_RATE as u64 * 60 * 60) as usize;
    let mut sums = vec![0.0_f32; output_len];
    let mut counts = vec![0_u8; output_len];
    let mut samples = reader.samples::<S>();
    let skip = first_frame.saturating_mul(channels as u64);
    for sample_index in 0..skip {
        if sample_index % 8_192 == 0 && cancelled.load(Ordering::Acquire) {
            return Err("Pitch analysis canceled.".to_owned());
        }
        samples
            .next()
            .ok_or_else(|| "Audio file ended before the selected event.".to_owned())?
            .map_err(|error| format!("Could not decode audio for analysis: {error}"))?;
    }
    let frames = end_frame - first_frame;
    for local_frame in 0..frames {
        if local_frame % 4_096 == 0 && cancelled.load(Ordering::Acquire) {
            return Err("Pitch analysis canceled.".to_owned());
        }
        let mut mono = 0.0_f64;
        for _ in 0..channel_count {
            let sample: f64 = samples
                .next()
                .ok_or_else(|| "Audio file ended during pitch analysis.".to_owned())?
                .map_err(|error| format!("Could not decode audio for analysis: {error}"))?
                .into();
            mono += (sample / scale).clamp(-1.0, 1.0);
        }
        mono /= channel_count as f64;
        // Average source samples that land in each analysis-rate bin. This
        // supplies a bounded-rate, low-pass-like mono view before YIN.
        let out_index = (local_frame * u64::from(ANALYSIS_RATE) / u64::from(source_rate)) as usize;
        if out_index < output_len {
            sums[out_index] += mono as f32;
            counts[out_index] = counts[out_index].saturating_add(1);
        }
    }
    for (sum, count) in sums.iter_mut().zip(&counts) {
        *sum = if *count == 0 {
            0.0
        } else {
            *sum / f32::from(*count)
        };
    }
    // Older low-rate WAVs need interpolation, not zero-filled alternating bins.
    if source_rate < ANALYSIS_RATE {
        for index in 0..sums.len() {
            if counts[index] == 0 {
                let previous = index.saturating_sub(1);
                let next = (index + 1..sums.len())
                    .find(|i| counts[*i] > 0)
                    .unwrap_or(previous);
                sums[index] = if next > previous {
                    sums[previous]
                        + (sums[next] - sums[previous]) * (index - previous) as f32
                            / (next - previous) as f32
                } else {
                    sums[previous]
                };
            }
        }
    }
    Ok(sums)
}

pub(crate) fn detect_pitch(
    samples: &[f32],
    source_start_seconds: f64,
    cancelled: &AtomicBool,
) -> Vec<PitchPoint> {
    let min_lag = (ANALYSIS_RATE as f32 / MAX_HZ).floor() as usize;
    let max_lag = (ANALYSIS_RATE as f32 / MIN_HZ).ceil() as usize;
    let mut difference = vec![0.0_f32; max_lag + 2];
    let mut normalized = vec![1.0_f32; max_lag + 2];
    let mut result = Vec::with_capacity(samples.len() / HOP + 1);
    let half = WINDOW / 2;
    let mut padded = vec![0.0; half];
    padded.extend_from_slice(samples);
    padded.resize(padded.len() + half, 0.0);
    for center in (0..samples.len()).step_by(HOP) {
        if cancelled.load(Ordering::Acquire) {
            return Vec::new();
        }
        let frame = &padded[center..center + WINDOW];
        let rms = (frame.iter().map(|v| v * v).sum::<f32>() / WINDOW as f32).sqrt();
        let mut candidate = None;
        if rms >= 0.003 {
            // Equal integration length at every lag avoids a bias toward long periods.
            let integration = WINDOW - max_lag - 1;
            for lag in 1..=max_lag + 1 {
                difference[lag] = (0..integration)
                    .map(|i| {
                        let delta = frame[i] - frame[i + lag];
                        delta * delta
                    })
                    .sum();
            }
            let mut cumulative = 0.0;
            for lag in 1..=max_lag + 1 {
                cumulative += difference[lag];
                normalized[lag] = if cumulative > 1e-12 {
                    difference[lag] * lag as f32 / cumulative
                } else {
                    1.0
                };
            }
            let mut lag = min_lag;
            while lag <= max_lag {
                if normalized[lag] < YIN_THRESHOLD {
                    // The previous detector stopped on the leading slope of the trough.
                    // Select the local minimum, then interpolate that normalized trough.
                    while lag < max_lag && normalized[lag + 1] < normalized[lag] {
                        lag += 1;
                    }
                    candidate = Some(lag);
                    break;
                }
                lag += 1;
            }
            if candidate.is_none() {
                candidate = (min_lag..=max_lag)
                    .min_by(|a, b| normalized[*a].total_cmp(&normalized[*b]))
                    .filter(|lag| normalized[*lag] < 0.25);
            }
        }
        let (frequency_hz, confidence) = if let Some(lag) = candidate {
            let left = normalized[lag - 1];
            let middle = normalized[lag];
            let right = normalized[lag + 1];
            let denominator = left - 2.0 * middle + right;
            let correction = if denominator.abs() > 1e-12 {
                (0.5 * (left - right) / denominator).clamp(-0.5, 0.5)
            } else {
                0.0
            };
            let hz = ANALYSIS_RATE as f32 / (lag as f32 + correction);
            (
                Some(hz).filter(|v| (MIN_HZ..=MAX_HZ).contains(v)),
                (1.0 - middle).clamp(0.0, 1.0),
            )
        } else {
            (None, 0.0)
        };
        result.push(PitchPoint {
            source_time_seconds: source_start_seconds + center as f64 / ANALYSIS_RATE as f64,
            frequency_hz,
            confidence,
        });
    }
    result
}

/// Stable voiced regions; preserve each measured center pitch until explicitly edited.
pub fn segment_notes(analysis: &PitchAnalysis) -> Vec<opentrack_core::pitch_time::PitchNote> {
    use opentrack_core::pitch_time::PitchNote;
    fn median(values: &[f32]) -> f32 {
        let mut v = values.to_vec();
        v.sort_by(f32::total_cmp);
        v[v.len() / 2]
    }
    let mut notes = Vec::new();
    let mut run = Vec::<(f64, f32)>::new();
    let finish = |run: &mut Vec<(f64, f32)>, notes: &mut Vec<PitchNote>| {
        if run.len() < 5 {
            run.clear();
            return;
        }
        let raw: Vec<_> = run.iter().map(|(_, p)| *p).collect();
        let smooth: Vec<_> = (0..raw.len())
            .map(|i| median(&raw[i.saturating_sub(2)..(i + 3).min(raw.len())]))
            .collect();
        let mut boundaries = vec![0];
        let mut start = 0;
        let mut i = 5;
        while i + 6 < smooth.len() {
            let base = median(&smooth[start..i]);
            let future = &smooth[i..i + 6];
            let next = median(future);
            let spread = future.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - future.iter().copied().fold(f32::INFINITY, f32::min);
            // Detect a new stable pitch against the segment, not against a
            // following moving average (which absorbed gradual slides).
            if i - start >= 8 && (next - base).abs() >= 0.8 && spread <= 0.25 {
                let midway = (base + next) * 0.5;
                let direction = (next - base).signum();
                let boundary = (start + 5..=i)
                    .rev()
                    .find(|j| (smooth[*j] - midway) * direction <= 0.0)
                    .map_or(i, |j| j + 1);
                if boundary - start >= 5 {
                    boundaries.push(boundary);
                    start = boundary;
                    i = boundary + 5;
                    continue;
                }
            }
            i += 1;
        }
        boundaries.push(run.len());
        for pair in boundaries.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b - a < 5 {
                continue;
            }
            let start = (run[a].0 - 0.005).max(0.0);
            let end = (run[b - 1].0 + 0.005).min(analysis.source_duration_seconds);
            if end - start < 0.03 {
                continue;
            }
            notes.push(PitchNote {
                id: notes.len() as u64 + 1,
                source_start: start,
                source_end: end,
                start,
                end,
                detected_midi: median(&smooth[a..b]),
                shift: 0.0,
                straighten: 0.0,
            });
        }
        run.clear();
    };
    for point in &analysis.points {
        if let Some(hz) = point.frequency_hz.filter(|_| point.confidence >= 0.75) {
            run.push((
                point.source_time_seconds - analysis.source_start_seconds,
                69.0 + 12.0 * (hz / 440.0).log2(),
            ));
        } else {
            finish(&mut run, &mut notes);
        }
    }
    finish(&mut run, &mut notes);
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slides_between_three_stable_pitches_split_without_fragmenting_vibrato() {
        for direction in [-1.0, 1.0] {
            let points = (0..145)
                .map(|i| {
                    let t = i as f64 * 0.01;
                    let step = if t < 0.35 {
                        0.0
                    } else if t < 0.45 {
                        (t - 0.35) / 0.1 * 2.0
                    } else if t < 0.85 {
                        2.0
                    } else if t < 0.95 {
                        2.0 + (t - 0.85) / 0.1 * 3.0
                    } else {
                        5.0
                    };
                    let midi = 60.0 + direction * step as f32 + 0.04 * (i as f32 * 0.3).sin();
                    PitchPoint {
                        source_time_seconds: t,
                        frequency_hz: Some(440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)),
                        confidence: 0.99,
                    }
                })
                .collect();
            let analysis = PitchAnalysis {
                sample_rate_hz: 16000,
                source_start_seconds: 0.0,
                source_duration_seconds: 1.45,
                points,
            };
            let notes = segment_notes(&analysis);
            assert_eq!(notes.len(), 3, "{notes:?}");
            for (note, offset) in notes.iter().zip([0.0, 2.0, 5.0]) {
                assert!((note.detected_midi - (60.0 + direction * offset)).abs() < 0.15);
            }
        }
        let analysis = PitchAnalysis {
            sample_rate_hz: 16000,
            source_start_seconds: 0.0,
            source_duration_seconds: 1.5,
            points: (0..150)
                .map(|i| {
                    let midi = 60.0 + 0.45 * (std::f32::consts::TAU * 5.5 * i as f32 * 0.01).sin();
                    PitchPoint {
                        source_time_seconds: i as f64 * 0.01,
                        frequency_hz: Some(440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)),
                        confidence: 0.99,
                    }
                })
                .collect(),
        };
        assert_eq!(segment_notes(&analysis).len(), 1);
    }
    #[test]
    fn wav_source_rates_are_conformed_for_detection_without_rewriting_sources() {
        for rate in [8000, 44100, 48000] {
            let path = std::env::temp_dir().join(format!(
                "opentrack-yin-rate-{}-{rate}.wav",
                std::process::id()
            ));
            let mut wav = hound::WavWriter::create(
                &path,
                hound::WavSpec {
                    sample_rate: rate,
                    channels: 1,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap();
            for i in 0..rate {
                wav.write_sample(
                    (12000.0 * (std::f32::consts::TAU * 440.0 * i as f32 / rate as f32).sin())
                        as i16,
                )
                .unwrap();
            }
            wav.finalize().unwrap();
            let before = std::fs::read(&path).unwrap();
            let analysis = analyze_pitch_file(&path, 0.2, 0.6, &AtomicBool::new(false)).unwrap();
            let mut errors: Vec<_> = analysis
                .points
                .iter()
                .skip(10)
                .take(40)
                .map(|p| 1200.0 * (p.frequency_hz.unwrap() / 440.0).log2().abs())
                .collect();
            errors.sort_by(f32::total_cmp);
            assert!(errors[errors.len() / 2] < 10.0, "{rate}: {errors:?}");
            assert_eq!(analysis.source_start_seconds, 0.2);
            assert_eq!(analysis.sample_rate_hz, 16000);
            assert_eq!(std::fs::read(&path).unwrap(), before);
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn measured_tones_have_fine_time_resolution_and_low_absolute_pitch_error() {
        for harmonic in [false, true] {
            for hz in [65.4, 82.4, 110.0, 220.0, 440.0, 880.0, 1200.0] {
                let samples: Vec<_> = (0..16000)
                    .map(|i| {
                        let phase = std::f32::consts::TAU * hz * i as f32 / 16000.0;
                        if harmonic {
                            0.12 * phase.sin()
                                + 0.25 * (2.0 * phase).sin()
                                + 0.15 * (3.0 * phase).sin()
                        } else {
                            0.4 * phase.sin()
                        }
                    })
                    .collect();
                let result = detect_pitch(&samples, 3.0, &AtomicBool::new(false));
                assert!(
                    (result[1].source_time_seconds - result[0].source_time_seconds - 0.01).abs()
                        < 1e-9
                );
                let mut cents: Vec<_> = result
                    .iter()
                    .skip(10)
                    .take(80)
                    .map(|p| {
                        1200.0
                            * (p.frequency_hz.expect("steady voiced tone") / hz)
                                .log2()
                                .abs()
                    })
                    .collect();
                cents.sort_by(f32::total_cmp);
                println!(
                    "tone {hz:.1} Hz harmonics={harmonic}: median {:.3} cents, p95 {:.3}",
                    cents[40], cents[76]
                );
                assert!(cents[40] < 10.0 && cents[76] < 25.0, "{hz} Hz: {cents:?}");
            }
        }
        assert!(
            detect_pitch(&vec![0.0; 16000], 0.0, &AtomicBool::new(false))
                .iter()
                .all(|p| p.frequency_hz.is_none())
        );
    }

    #[test]
    fn segmentation_uses_actual_voiced_regions_and_preserves_untuned_pitch() {
        let samples: Vec<_> = (0..20000)
            .map(|i| {
                let t = i as f32 / 16000.0;
                let hz = if (0.1..0.5).contains(&t) {
                    220.0
                } else if (0.65..1.1).contains(&t) {
                    329.628
                } else {
                    return 0.0;
                };
                0.4 * (std::f32::consts::TAU * hz * t).sin()
            })
            .collect();
        let analysis = PitchAnalysis {
            sample_rate_hz: 16000,
            source_start_seconds: 0.0,
            source_duration_seconds: 1.25,
            points: detect_pitch(&samples, 0.0, &AtomicBool::new(false)),
        };
        let notes = segment_notes(&analysis);
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!((notes[0].detected_midi - 57.0).abs() < 0.1);
        assert!((notes[1].detected_midi - 64.0).abs() < 0.1);
        assert!(
            notes
                .iter()
                .all(|n| n.shift == 0.0 && n.start == n.source_start && n.end == n.source_end)
        );
    }
}
