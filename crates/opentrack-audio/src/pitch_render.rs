//! Offline, pitch-synchronous overlap-add for monophonic note edits.
//! Original WAVs are read only; a completed float WAV is published atomically.
use opentrack_core::pitch_time::PitchTimeEdit;
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn pitch_source_identity(path: &Path) -> Result<(u64, u64), String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |t| t.as_nanos().min(u64::MAX as u128) as u64);
    Ok((meta.len(), modified))
}

pub fn render_pitch_time_file(
    path: &Path,
    edit: &PitchTimeEdit,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    render_pitch_time_file_at_tempo(path, edit, 120.0, output, cancel)
}

pub fn render_pitch_time_file_at_tempo(
    path: &Path,
    edit: &PitchTimeEdit,
    tempo_bpm: f64,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if pitch_source_identity(path)? != (edit.source_bytes, edit.source_modified_ns) {
        return Err("The source changed. Re-analyze before applying edits.".into());
    }
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if !(1..=2).contains(&spec.channels) || !edit.validate(edit.analysis.source_duration_seconds) {
        return Err("Invalid note bounds or unsupported source channels.".into());
    }
    if reader.len() > 32_000_000 {
        return Err(
            "This source exceeds the current offline edit memory limit (32 million samples)."
                .into(),
        );
    }
    let mut samples = Vec::with_capacity(reader.len() as usize);
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for (i, v) in reader.samples::<f32>().enumerate() {
                if i % 8192 == 0 && cancel.load(Ordering::Acquire) {
                    return Err("Pitch render canceled.".into());
                }
                samples.push(v.map_err(|e| e.to_string())?);
            }
        }
        hound::SampleFormat::Int => {
            let scale = 2.0_f32.powi(spec.bits_per_sample as i32 - 1);
            for (i, v) in reader.samples::<i32>().enumerate() {
                if i % 8192 == 0 && cancel.load(Ordering::Acquire) {
                    return Err("Pitch render canceled.".into());
                }
                samples.push(v.map_err(|e| e.to_string())? as f32 / scale);
            }
        }
    }
    let rendered = render_samples(
        &samples,
        spec.sample_rate,
        usize::from(spec.channels),
        edit,
        tempo_bpm,
        cancel,
    )?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = output.with_extension(format!("{}.part", std::process::id()));
    let result = (|| {
        let mut writer = hound::WavWriter::create(
            &temporary,
            hound::WavSpec {
                sample_rate: spec.sample_rate,
                channels: spec.channels,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .map_err(|e| e.to_string())?;
        for (i, sample) in rendered.iter().enumerate() {
            if i % 8192 == 0 && cancel.load(Ordering::Acquire) {
                return Err("Pitch render canceled.".into());
            }
            writer.write_sample(*sample).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
        if pitch_source_identity(path)? != (edit.source_bytes, edit.source_modified_ns) {
            return Err("The source changed during rendering.".into());
        }
        if output.exists() {
            std::fs::remove_file(&temporary).map_err(|e| e.to_string())?;
        } else {
            std::fs::rename(&temporary, output).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

fn render_samples(
    samples: &[f32],
    rate: u32,
    channels: usize,
    edit: &PitchTimeEdit,
    tempo_bpm: f64,
    cancel: &AtomicBool,
) -> Result<Vec<f32>, String> {
    let changed = edit.notes.iter().any(|note| {
        note.shift.abs() > 1e-6
            || note.straighten > 1e-6
            || (note.start - note.source_start).abs() > 1e-9
            || (note.end - note.source_end).abs() > 1e-9
    });
    if !changed {
        return Ok(samples.to_vec());
    }

    let renderer = crate::pitch_synthesis::PitchSynthesis::new(
        samples, rate, channels, edit, tempo_bpm, cancel,
    )?;
    let frame_count = samples.len() / channels;
    let mut output = samples.to_vec();
    for frame in 0..frame_count {
        if frame % 4096 == 0 && cancel.load(Ordering::Acquire) {
            return Err("Pitch render canceled.".into());
        }
        let (left, right) = renderer
            .sample_frame(samples, frame as u64)
            .ok_or_else(|| "Pitch renderer returned a frame outside its source".to_owned())?;
        let sample = frame * channels;
        output[sample] = left;
        if channels == 2 {
            output[sample + 1] = right;
        }
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::pitch_time::{DETECTOR_VERSION, PitchAnalysis, PitchNote, PitchPoint};
    #[test]
    fn wav_worker_publishes_readable_derived_audio_and_cancel_preserves_the_recording() {
        let (samples, mut edit) = fixture(false);
        let path =
            std::env::temp_dir().join(format!("opentrack-note-source-{}.wav", std::process::id()));
        let output = path.with_extension("corrected.wav");
        let cancelled_output = path.with_extension("cancelled.wav");
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                sample_rate: 16000,
                channels: 1,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for sample in samples {
            writer.write_sample((sample * 32767.0) as i16).unwrap();
        }
        writer.finalize().unwrap();
        let before = std::fs::read(&path).unwrap();
        let identity = pitch_source_identity(&path).unwrap();
        edit.source_bytes = identity.0;
        edit.source_modified_ns = identity.1;
        edit.notes[0].shift = 7.0;
        render_pitch_time_file(&path, &edit, &output, &AtomicBool::new(false)).unwrap();
        let mut reader = hound::WavReader::open(&output).unwrap();
        assert_eq!(reader.duration(), 16000);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        let values: Vec<_> = reader.samples::<f32>().map(Result::unwrap).collect();
        let analysis = crate::pitch_analysis::detect_pitch(&values, 0.0, &AtomicBool::new(false));
        let hz = analysis[40].frequency_hz.unwrap();
        assert!((1200.0 * (hz / 329.62756).log2()).abs() < 25.0);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(
            render_pitch_time_file(&path, &edit, &cancelled_output, &AtomicBool::new(true))
                .is_err()
        );
        assert!(!cancelled_output.exists());
        drop(reader);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(output).unwrap();
    }
    fn fixture(stereo: bool) -> (Vec<f32>, PitchTimeEdit) {
        let channels = if stereo { 2 } else { 1 };
        let samples = (0..16000)
            .flat_map(|i| {
                let t = i as f32 / 16000.0;
                let value = if (0.2..0.6).contains(&t) {
                    0.4 * (std::f32::consts::TAU * 220.0 * t).sin()
                } else {
                    0.0
                };
                (0..channels).map(move |ch| value * if ch == 0 { 1.0 } else { 0.37 })
            })
            .collect();
        let edit = PitchTimeEdit {
            detector_version: DETECTOR_VERSION,
            source_bytes: 0,
            source_modified_ns: 0,
            analysis: PitchAnalysis {
                sample_rate_hz: 16000,
                source_start_seconds: 0.0,
                source_duration_seconds: 1.0,
                points: (0..100)
                    .map(|i| PitchPoint {
                        source_time_seconds: i as f64 * 0.01,
                        frequency_hz: Some(220.0),
                        confidence: 1.0,
                    })
                    .collect(),
            },
            notes: vec![PitchNote {
                id: 1,
                source_start: 0.2,
                source_end: 0.6,
                start: 0.2,
                end: 0.6,
                detected_midi: 57.0,
                shift: 0.0,
                straighten: 0.0,
            }],
            rendered_source: None,
            enabled: true,
        };
        (samples, edit)
    }
    #[test]
    fn rendered_notes_change_pitch_and_duration_independently_preserving_stereo_and_original() {
        let (samples, mut edit) = fixture(true);
        let original = samples.clone();
        assert_eq!(
            render_samples(&samples, 16000, 2, &edit, &AtomicBool::new(false)).unwrap(),
            samples
        );
        for shift in [-5.0, 7.0] {
            edit.notes[0].shift = shift;
            edit.notes[0].start = 0.25;
            edit.notes[0].end = 0.9;
            let out = render_samples(&samples, 16000, 2, &edit, &AtomicBool::new(false)).unwrap();
            assert_eq!(out.len(), samples.len());
            assert_eq!(samples, original);
            assert!(
                out.chunks_exact(2)
                    .all(|s| (s[1] - 0.37 * s[0]).abs() < 1e-5)
            );
            let mono: Vec<_> = out.chunks_exact(2).map(|s| s[0]).collect();
            let pitches = crate::pitch_analysis::detect_pitch(&mono, 0.0, &AtomicBool::new(false));
            let expected = 220.0 * 2.0_f32.powf(shift / 12.0);
            let mut errors: Vec<_> = pitches
                .iter()
                .filter(|p| (0.4..0.75).contains(&p.source_time_seconds))
                .map(|p| {
                    1200.0
                        * (p.frequency_hz
                            .expect("rendered sustained note must remain voiced")
                            / expected)
                            .log2()
                            .abs()
                })
                .collect();
            errors.sort_by(f32::total_cmp);
            println!(
                "render shift {shift} st: median {:.3} cents",
                errors[errors.len() / 2]
            );
            assert!(errors[errors.len() / 2] < 25.0);
            assert!(
                mono[12800..13440].iter().any(|v| v.abs() > 0.1),
                "stretched note must be audible after its original end"
            );
            assert!(mono[..2000].iter().all(|v| v.abs() < 1e-6));
        }
        assert!(render_samples(&samples, 16000, 2, &edit, &AtomicBool::new(true)).is_err());
    }
}
