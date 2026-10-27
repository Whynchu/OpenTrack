//! Headless reconstruction diagnostics; run against the same sampler as playback.
use super::*;
use opentrack_core::pitch_time::{PitchAnalysis, PitchPoint};

const RATE: u32 = 16_000;
const SECONDS: f64 = 2.0;

#[test]
#[ignore = "explicit read-only source and analysis paths required"]
fn recorded_vocal_matrix() {
    let source_path = std::env::var_os("OPENTRACK_PITCH_SOURCE").expect("source WAV path");
    let edit_path = std::env::var_os("OPENTRACK_PITCH_EDIT").expect("saved analysis path");
    let mut reader = hound::WavReader::open(source_path).unwrap();
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(Result::unwrap).collect(),
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.unwrap() as f32 / 2.0_f32.powi(i32::from(spec.bits_per_sample) - 1))
            .collect(),
    };
    let original: PitchTimeEdit =
        serde_json::from_slice(&std::fs::read(edit_path).unwrap()).unwrap();
    let mut rows = Vec::new();
    save_wav_at("vocal-source", &samples, spec.sample_rate, spec.channels);
    for shift in [0.0_f32, -0.2, 0.2, -2.0, 2.0] {
        let mut edit = original.clone();
        for note in &mut edit.notes {
            note.start = note.source_start;
            note.end = note.source_end;
            note.shift = shift;
            note.straighten = 0.0;
        }
        let started = std::time::Instant::now();
        let output = render_at(
            &samples,
            spec.sample_rate,
            usize::from(spec.channels),
            &edit,
            true,
        );
        let elapsed = started.elapsed();
        assert!(output.iter().all(|s| s.is_finite()));
        let source_difference = relative_error(&samples, &output);
        let mono_16k: Vec<f32> = output
            .chunks_exact(usize::from(spec.channels) * 3)
            .map(|block| {
                block
                    .chunks_exact(usize::from(spec.channels))
                    .map(|frame| {
                        frame
                            .iter()
                            .map(|sample| *sample / spec.channels as f32)
                            .sum::<f32>()
                    })
                    .sum::<f32>()
                    / 3.0
            })
            .collect();
        let rendered_pitch =
            crate::pitch_analysis::detect_pitch(&mono_16k, 0.0, &AtomicBool::new(false));
        let mut cents = Vec::new();
        for point in rendered_pitch {
            let Some(frequency) = point.frequency_hz.filter(|_| point.confidence >= 0.75) else {
                continue;
            };
            let local_time = point.source_time_seconds - edit.analysis.source_start_seconds;
            let note_index = edit.notes.partition_point(|note| note.end < local_time);
            let Some(note) = edit
                .notes
                .get(note_index)
                .filter(|note| local_time >= note.start)
            else {
                continue;
            };
            let fallback = 440.0 * 2.0_f32.powf((note.detected_midi - 69.0) / 12.0);
            let (detected, confidence) = estimate_pitch(&edit, point.source_time_seconds, fallback);
            if confidence < 0.5 {
                continue;
            }
            let detected_midi = 69.0 + 12.0 * (detected / 440.0).log2();
            let target = edit.pitch_midi_at(note_index, detected_midi, local_time, 120.0);
            let expected = 440.0 * 2.0_f32.powf((target - 69.0) / 12.0);
            cents.push(1200.0 * f64::from(frequency / expected).log2());
        }
        cents.sort_by(f64::total_cmp);
        assert!(
            cents.len() > 100,
            "not enough confident rendered-pitch frames: {}",
            cents.len()
        );
        let median_cents = cents[cents.len() / 2];
        let p90_cents = cents[cents.len() * 9 / 10];
        println!(
            "vocal shift {shift:+}: rendered pitch median {median_cents:.1} cents, p90 {p90_cents:.1} cents over {} frames",
            cents.len()
        );
        assert!(
            median_cents.abs() < 25.0,
            "median rendered pitch error {median_cents} cents"
        );
        assert!(
            p90_cents.abs() < 100.0,
            "p90 rendered pitch error {p90_cents} cents"
        );
        let max_step = output
            .windows(2)
            .map(|p| (p[1] - p[0]).abs())
            .fold(0.0_f32, f32::max);
        println!(
            "vocal shift {shift:+}: rendered pitch median {median_cents:.1} cents, p90 {p90_cents:.1} cents; source difference {source_difference:.2} dB; max step {max_step:.5}; elapsed {elapsed:?}"
        );
        rows.push(serde_json::json!({"shift":shift,"rendered_pitch_median_cents":median_cents,"rendered_pitch_p90_cents":p90_cents,"pitch_frames":cents.len(),"source_difference_db":source_difference,"maximum_sample_step":max_step,"elapsed_ms":elapsed.as_secs_f64()*1000.0}));
        save_wav_at(
            &format!("vocal-{shift:+}"),
            &output,
            spec.sample_rate,
            spec.channels,
        );
    }
    if let Some(directory) = std::env::var_os("OPENTRACK_PITCH_QUALITY_DIR") {
        std::fs::write(
            std::path::PathBuf::from(directory).join("vocal-metrics.json"),
            serde_json::to_vec_pretty(&rows).unwrap(),
        )
        .unwrap();
    }
}

fn fixture(shift: f32, confidence: f32, pieces: usize) -> PitchTimeEdit {
    PitchTimeEdit {
        detector_version: 2,
        source_bytes: 0,
        source_modified_ns: 0,
        analysis: PitchAnalysis {
            sample_rate_hz: RATE,
            source_start_seconds: 0.0,
            source_duration_seconds: SECONDS,
            points: (0..=200)
                .map(|i| PitchPoint {
                    source_time_seconds: i as f64 * 0.01,
                    frequency_hz: Some(220.0),
                    confidence,
                })
                .collect(),
        },
        notes: (0..pieces)
            .map(|i| PitchNote {
                id: i as u64 + 1,
                source_start: i as f64 * SECONDS / pieces as f64,
                source_end: (i + 1) as f64 * SECONDS / pieces as f64,
                start: i as f64 * SECONDS / pieces as f64,
                end: (i + 1) as f64 * SECONDS / pieces as f64,
                detected_midi: 57.0,
                shift,
                straighten: 0.0,
                pitch_start_bend: 0.0,
                pitch_anchor: 0.5,
                pitch_anchor_bend: 0.0,
                pitch_end_bend: 0.0,
            })
            .collect(),
        rendered_source: None,
        enabled: true,
    }
}

fn render(samples: &[f32], edit: &PitchTimeEdit, force: bool) -> Vec<f32> {
    render_at(samples, RATE, 1, edit, force)
}

fn render_at(
    samples: &[f32],
    rate: u32,
    channels: usize,
    edit: &PitchTimeEdit,
    force: bool,
) -> Vec<f32> {
    let cancel = AtomicBool::new(false);
    let mut synth = PitchSynthesis::new(samples, rate, channels, edit, 120.0, &cancel).unwrap();
    if force && synth.plans.iter().all(|plan| !plan.processed) {
        let grains: Arc<[Grain]> =
            make_event_grains(samples, f64::from(rate), channels, edit, 120.0, &cancel)
                .unwrap()
                .into();
        for plan in &mut synth.plans {
            plan.processed = true;
            plan.edge_only = false;
            plan.grains = Arc::clone(&grains);
        }
    }
    (0..samples.len() / channels)
        .flat_map(|i| {
            let (left, right) = synth.sample_frame(samples, i as u64).unwrap();
            [left, right].into_iter().take(channels)
        })
        .collect()
}

fn rms(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0), |(sum, n), value| (sum + value * value, n + 1));
    (sum / n.max(1) as f64).sqrt()
}

fn relative_error(reference: &[f32], result: &[f32]) -> f64 {
    let margin = RATE as usize / 5;
    let a = &reference[margin..reference.len() - margin];
    let b = &result[margin..result.len() - margin];
    20.0 * (rms(a.iter().zip(b).map(|(a, b)| f64::from(a - b)))
        / rms(a.iter().map(|v| f64::from(*v))).max(1e-15))
    .max(1e-15)
    .log10()
}

fn tone_residual_db(samples: &[f32], hz: f64) -> f64 {
    let margin = RATE as usize / 5;
    let mut ss = 0.0;
    let mut cc = 0.0;
    let mut sc = 0.0;
    let mut ys = 0.0;
    let mut yc = 0.0;
    for (i, y) in samples
        .iter()
        .enumerate()
        .take(samples.len() - margin)
        .skip(margin)
    {
        let (s, c) = (std::f64::consts::TAU * hz * i as f64 / f64::from(RATE)).sin_cos();
        ss += s * s;
        cc += c * c;
        sc += s * c;
        ys += f64::from(*y) * s;
        yc += f64::from(*y) * c;
    }
    let determinant = ss * cc - sc * sc;
    let a = (ys * cc - yc * sc) / determinant;
    let b = (yc * ss - ys * sc) / determinant;
    let error = rms(samples
        .iter()
        .enumerate()
        .take(samples.len() - margin)
        .skip(margin)
        .map(|(i, y)| {
            let (s, c) = (std::f64::consts::TAU * hz * i as f64 / f64::from(RATE)).sin_cos();
            f64::from(*y) - a * s - b * c
        }));
    20.0 * (error
        / rms(samples[margin..samples.len() - margin]
            .iter()
            .map(|v| f64::from(*v)))
        .max(1e-15))
    .max(1e-15)
    .log10()
}

fn save_wav(name: &str, samples: &[f32]) {
    save_wav_at(name, samples, RATE, 1);
}

fn save_wav_at(name: &str, samples: &[f32], rate: u32, channels: u16) {
    if let Some(directory) = std::env::var_os("OPENTRACK_PITCH_QUALITY_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let mut writer = hound::WavWriter::create(
            directory.join(format!("{name}.wav")),
            hound::WavSpec {
                channels,
                sample_rate: rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for value in samples {
            writer.write_sample(*value).unwrap();
        }
        writer.finalize().unwrap();
    }
}

#[test]
fn diagnostic_matrix() {
    let source: Vec<f32> = (0..(f64::from(RATE) * SECONDS) as usize)
        .map(|i| (0.45 * (std::f64::consts::TAU * 220.0 * i as f64 / f64::from(RATE)).sin()) as f32)
        .collect();
    assert!(
        pulses::source_cycle_complexity(&source, f64::from(RATE), 1, &fixture(0.0, 1.0, 1)) < 0.01
    );
    save_wav("source-sine", &source);
    let mut rows = Vec::new();
    for shift in [0.0_f32, -0.1, 0.1, -0.2, 0.2, -2.0, 2.0] {
        let output = render(&source, &fixture(shift, 1.0, 1), true);
        let residual = tone_residual_db(&output, 220.0 * 2.0_f64.powf(f64::from(shift) / 12.0));
        let identity_error = relative_error(&source, &output);
        let split = render(&source, &fixture(shift, 1.0, 4), true);
        let cut_error = relative_error(&output, &split);
        assert!(output.iter().all(|value| value.is_finite()));
        assert!(
            cut_error < -100.0,
            "identical cuts changed audio by {cut_error} dB"
        );
        if shift == 0.0 {
            assert!(
                identity_error < -110.0,
                "forced zero reconstruction error {identity_error} dB"
            );
        } else {
            let limit = if shift.abs() < 0.5 { -60.0 } else { -45.0 };
            assert!(
                residual < limit,
                "shift {shift}: unwanted energy {residual} dB exceeds {limit} dB"
            );
        }
        println!(
            "shift {shift:+}: tone residual {residual:.2} dB; identity {identity_error:.2} dB; cut difference {cut_error:.2} dB"
        );
        rows.push(serde_json::json!({"shift":shift,"confidence":1.0,"residual_db":residual,"identity_error_db":identity_error,"cut_error_db":cut_error}));
        save_wav(&format!("sine-{shift:+}"), &output);
    }
    let output = render(&source, &fixture(0.2, 0.55, 1), true);
    let residual = tone_residual_db(&output, 220.0 * 2.0_f64.powf(0.2 / 12.0));
    assert!(
        residual < -60.0,
        "medium confidence must not leave an original-pitch voice: {residual} dB"
    );
    println!("confidence 0.55, +0.2: tone residual {residual:.2} dB");
    rows.push(serde_json::json!({"shift":0.2,"confidence":0.55,"residual_db":residual}));
    save_wav("sine-confidence-055", &output);
    if let Some(directory) = std::env::var_os("OPENTRACK_PITCH_QUALITY_DIR") {
        std::fs::write(
            std::path::PathBuf::from(directory).join("synthetic-metrics.json"),
            serde_json::to_vec_pretty(&rows).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn harmonic_vibrato_zero_reconstruction_and_unvoiced_integrity() {
    let source: Vec<f32> = (0..(f64::from(RATE) * SECONDS) as usize)
        .map(|i| {
            let t = i as f64 / f64::from(RATE);
            let phase =
                std::f64::consts::TAU * 220.0 * t + 0.7 * (std::f64::consts::TAU * 5.0 * t).sin();
            let envelope = 0.7 + 0.3 * (std::f64::consts::TAU * 1.7 * t).sin();
            (envelope
                * (0.3 * phase.sin()
                    + 0.15 * (2.0 * phase + 0.4).sin()
                    + 0.08 * (3.0 * phase + 0.8).sin()
                    + 0.04 * (7.0 * phase).sin())) as f32
        })
        .collect();
    let edit = fixture(0.0, 0.7, 4);
    let output = render(&source, &edit, true);
    assert!(relative_error(&source, &output) < -110.0);
    let mut unvoiced = fixture(2.0, 0.0, 1);
    for point in &mut unvoiced.analysis.points {
        point.frequency_hz = None;
    }
    let output = render(&source, &unvoiced, false);
    let error = relative_error(&source, &output);
    assert!(error < -110.0, "unvoiced reconstruction error {error} dB");
}

#[test]
fn octave_glitches_in_analysis_do_not_fragment_a_stable_tone() {
    let source: Vec<f32> = (0..(f64::from(RATE) * SECONDS) as usize)
        .map(|i| (0.4 * (std::f64::consts::TAU * 220.0 * i as f64 / f64::from(RATE)).sin()) as f32)
        .collect();
    let mut edit = fixture(0.2, 1.0, 1);
    for (i, point) in edit.analysis.points.iter_mut().enumerate() {
        point.frequency_hz = Some(if (i / 5) % 2 == 0 { 220.0 } else { 110.0 });
    }
    let output = render(&source, &edit, false);
    let residual = tone_residual_db(&output, 220.0 * 2.0_f64.powf(0.2 / 12.0));
    println!("octave-glitch tone residual {residual:.2} dB");
    assert!(
        residual < -40.0,
        "octave analysis glitch fragments output: {residual} dB"
    );
}

#[test]
fn compression_keeps_pitch_and_stereo_phase() {
    let source: Vec<f32> = (0..(f64::from(RATE) * SECONDS) as usize)
        .map(|i| (0.4 * (std::f64::consts::TAU * 220.0 * i as f64 / f64::from(RATE)).sin()) as f32)
        .collect();
    let mut edit = fixture(0.2, 1.0, 1);
    edit.notes[0].source_start = 0.4;
    edit.notes[0].source_end = 1.6;
    edit.notes[0].start = 0.4;
    edit.notes[0].end = 1.2;
    let stereo: Vec<f32> = source.iter().flat_map(|v| [*v, *v * 0.37]).collect();
    let output = render_at(&stereo, RATE, 2, &edit, false);
    assert!(
        output
            .chunks_exact(2)
            .all(|frame| (frame[1] - frame[0] * 0.37).abs() < 1e-6)
    );
    let mono: Vec<f32> = output.chunks_exact(2).map(|frame| frame[0]).collect();
    let probe = &mono[(RATE as f64 * 0.5) as usize..(RATE as f64 * 1.1) as usize];
    // The tone fitter crops 0.2 seconds from either side, leaving the steady
    // interior of the retimed note, away from the intended transition ramps.
    let residual = tone_residual_db(probe, 220.0 * 2.0_f64.powf(0.2 / 12.0));
    println!("compressed tone residual {residual:.2} dB");
    assert!(
        residual < -45.0,
        "compression introduces sidebands: {residual} dB"
    );
}

#[test]
fn stretched_note_keeps_audio_past_its_original_source_tail() {
    let mut edit = fixture(7.0, 1.0, 1);
    edit.analysis.source_duration_seconds = 1.0;
    edit.analysis.points.truncate(101);
    edit.notes[0].source_start = 0.2;
    edit.notes[0].source_end = 0.6;
    edit.notes[0].start = 0.25;
    edit.notes[0].end = 0.9;
    let source: Vec<f32> = (0..RATE as usize)
        .map(|i| {
            let time = i as f64 / f64::from(RATE);
            if (0.2..0.6).contains(&time) {
                (0.4 * (std::f64::consts::TAU * 220.0 * time).sin()) as f32
            } else {
                0.0
            }
        })
        .collect();
    let output = render(&source, &edit, false);
    let tail = &output[(RATE as usize * 13 / 16)..(RATE as usize * 7 / 8)];
    let rms = rms(tail.iter().map(|v| f64::from(*v)));
    assert!(
        rms > 0.15,
        "stretched note lost level after original source end: {rms}"
    );
}

fn harmonic_amplitude(samples: &[f32], hz: f64) -> f64 {
    let margin = RATE as usize / 5;
    let count = samples.len() - 2 * margin;
    let (mut sine, mut cosine, mut weights) = (0.0, 0.0, 0.0);
    for (j, &value) in samples[margin..samples.len() - margin].iter().enumerate() {
        let window = 0.5 - 0.5 * (std::f64::consts::TAU * j as f64 / count as f64).cos();
        let (s, c) = (std::f64::consts::TAU * hz * (j + margin) as f64 / f64::from(RATE)).sin_cos();
        sine += f64::from(value) * s * window;
        cosine += f64::from(value) * c * window;
        weights += window;
    }
    2.0 * (sine * sine + cosine * cosine).sqrt() / weights.max(1e-15)
}

#[test]
fn vowel_spectral_envelope_and_consonant_are_preserved() {
    let envelope = |hz: f64| {
        0.025
            + (-0.5 * ((hz - 650.0) / 180.0).powi(2)).exp()
            + 0.6 * (-0.5 * ((hz - 1500.0) / 200.0).powi(2)).exp()
            + 0.3 * (-0.5 * ((hz - 2600.0) / 250.0).powi(2)).exp()
    };
    let fundamental = 110.0;
    let source: Vec<f32> = (0..(f64::from(RATE) * SECONDS) as usize)
        .map(|i| {
            (1..=55)
                .map(|h| {
                    0.035
                        * envelope(fundamental * f64::from(h))
                        * (std::f64::consts::TAU * fundamental * f64::from(h) * i as f64
                            / f64::from(RATE))
                        .sin()
                })
                .sum::<f64>() as f32
        })
        .collect();
    assert!(
        pulses::source_cycle_complexity(&source, f64::from(RATE), 1, &fixture(0.0, 1.0, 1)) > 0.5
    );
    let mut rows = Vec::new();
    for shift in [-2.0_f32, 2.0] {
        let mut edit = fixture(shift, 0.55, 1);
        for note in &mut edit.notes {
            note.detected_midi = 45.0;
        }
        for point in &mut edit.analysis.points {
            point.frequency_hz = Some(fundamental as f32);
        }
        let output = render(&source, &edit, false);
        let target = fundamental * 2.0_f64.powf(f64::from(shift) / 12.0);
        let output_pitch =
            crate::pitch_analysis::detect_pitch(&output, 0.0, &AtomicBool::new(false));
        let mut cents: Vec<f32> = output_pitch
            .iter()
            .filter_map(|point| {
                if !(0.2..1.8).contains(&point.source_time_seconds) || point.confidence < 0.5 {
                    return None;
                }
                point
                    .frequency_hz
                    .map(|hz| 1200.0 * f64::from(hz / target as f32).log2() as f32)
            })
            .collect();
        cents.sort_by(f32::total_cmp);
        let pitch_error = cents[cents.len() / 2];
        println!("vowel shift {shift:+}: detected F0 error {pitch_error:.1} cents");
        assert!(
            pitch_error.abs() < 25.0,
            "vowel fundamental error {pitch_error} cents"
        );
        let differences: Vec<f64> = (1..=40)
            .filter_map(|h| {
                let hz = target * f64::from(h);
                if !(300.0..=3500.0).contains(&hz) {
                    return None;
                }
                let expected = 0.035 * envelope(hz);
                Some(20.0 * (harmonic_amplitude(&output, hz).max(1e-12) / expected).log10())
            })
            .collect();
        let mean = differences.iter().sum::<f64>() / differences.len() as f64;
        let shape = rms(differences.iter().map(|value| value - mean));
        println!(
            "vowel shift {shift:+}: spectral-envelope shape error {shape:.3} dB; level offset {mean:.3} dB"
        );
        assert!(shape < 3.0, "vowel resonances changed by {shape} dB");
        assert!(mean.abs() < 3.0, "vowel level changed by {mean} dB");
        rows.push(serde_json::json!({"shift":shift,"spectral_envelope_shape_error_db":shape,"level_offset_db":mean}));
        save_wav(&format!("vowel-{shift:+}"), &output);
    }
    // A noise consonant and its sharp transient remain original samples, away
    // from the short voiced/unvoiced crossfade at either side.
    let mut mixed = source.clone();
    let mut rng = 17_u32;
    for sample in &mut mixed[(RATE as f64 * 0.7) as usize..(RATE as f64 * 1.3) as usize] {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        *sample = (rng as f64 / u32::MAX as f64 * 0.2 - 0.1) as f32;
    }
    mixed[RATE as usize] = 0.8;
    let mut edit = fixture(2.0, 1.0, 1);
    for point in &mut edit.analysis.points {
        if (0.7..=1.3).contains(&point.source_time_seconds) {
            point.frequency_hz = None;
            point.confidence = 0.0;
        }
    }
    let output = render(&mixed, &edit, false);
    let range = (RATE as f64 * 0.8) as usize..(RATE as f64 * 1.2) as usize;
    let maximum = mixed[range.clone()]
        .iter()
        .zip(&output[range])
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(maximum < 1e-6, "unvoiced transient changed by {maximum}");
    if let Some(directory) = std::env::var_os("OPENTRACK_PITCH_QUALITY_DIR") {
        std::fs::write(
            std::path::PathBuf::from(directory).join("vowel-metrics.json"),
            serde_json::to_vec_pretty(&rows).unwrap(),
        )
        .unwrap();
    }
}
