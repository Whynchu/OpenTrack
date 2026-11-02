// SPDX-License-Identifier: Apache-2.0

//! Offline source processing that writes new media instead of editing in place.
//!
//! These operations are deliberately separate from the playback graph: they read
//! the source file, produce a new WAV in the project, and let the project point an
//! event at it. Nothing here runs on the audio callback.

use std::path::Path;

/// Largest sample payload one reversal may buffer, matching the import limit.
const MAX_PROCESS_BYTES: u64 = 512 * 1024 * 1024;

/// Writes the used range of `source`, reversed, to a 32-bit float WAV.
///
/// `offset_seconds` and `duration_seconds` describe the part of the source that
/// the event actually plays, so reversing a trimmed event reverses what is heard
/// rather than the whole file. The source is left untouched. Returns the frame
/// count written.
pub fn reverse_wav_range(
    source: &Path,
    offset_seconds: f64,
    duration_seconds: f64,
    destination: &Path,
) -> Result<u64, String> {
    if !offset_seconds.is_finite()
        || offset_seconds < 0.0
        || !duration_seconds.is_finite()
        || duration_seconds <= 0.0
    {
        return Err("the reverse range is not usable".to_owned());
    }
    let mut reader = hound::WavReader::open(source)
        .map_err(|error| format!("could not open the source WAV: {error}"))?;
    let spec = reader.spec();
    if !(1..=2).contains(&spec.channels) || spec.sample_rate == 0 {
        return Err("the source WAV must be mono or stereo with a valid rate".to_owned());
    }
    let channels = usize::from(spec.channels);
    let rate = f64::from(spec.sample_rate);
    let total_frames = u64::from(reader.duration());
    let start_frame = ((offset_seconds * rate).round() as u64).min(total_frames);
    let frames = ((duration_seconds * rate).round() as u64).min(total_frames - start_frame);
    if frames == 0 {
        return Err("the event has no audio inside the source file to reverse".to_owned());
    }
    let sample_count = frames
        .checked_mul(channels as u64)
        .ok_or_else(|| "the reversed range is too large".to_owned())?;
    if sample_count
        .checked_mul(4)
        .is_none_or(|bytes| bytes > MAX_PROCESS_BYTES)
    {
        return Err("the reversed range is larger than the 512 MiB process limit".to_owned());
    }

    let skip = (start_frame * channels as u64) as usize;
    let take = sample_count as usize;
    let mut samples = Vec::with_capacity(take);
    match spec.sample_format {
        hound::SampleFormat::Int => {
            let scale = 2.0_f64.powi(i32::from(spec.bits_per_sample) - 1);
            for sample in reader.samples::<i32>().skip(skip).take(take) {
                let sample =
                    sample.map_err(|error| format!("could not decode WAV samples: {error}"))?;
                samples.push((f64::from(sample) / scale).clamp(-1.0, 1.0) as f32);
            }
        }
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>().skip(skip).take(take) {
                let sample =
                    sample.map_err(|error| format!("could not decode WAV samples: {error}"))?;
                samples.push(if sample.is_finite() {
                    sample.clamp(-1.0, 1.0)
                } else {
                    0.0
                });
            }
        }
    }
    if samples.len() != take {
        return Err("the source WAV ended before the event did".to_owned());
    }

    if let Some(parent) = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let output = hound::WavSpec {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(destination, output)
        .map_err(|error| format!("could not create the reversed WAV: {error}"))?;
    for frame in (0..frames as usize).rev() {
        let base = frame * channels;
        for channel in 0..channels {
            writer
                .write_sample(samples[base + channel])
                .map_err(|error| format!("could not write the reversed WAV: {error}"))?;
        }
    }
    writer
        .finalize()
        .map_err(|error| format!("could not finish the reversed WAV: {error}"))?;
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_ramp(path: &Path, frames: usize) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("writer");
        for index in 0..frames {
            writer
                .write_sample(index as f32 / frames as f32)
                .expect("sample");
        }
        writer.finalize().expect("finalize");
    }

    fn read_mono(path: &Path) -> Vec<f32> {
        let mut reader = hound::WavReader::open(path).expect("reader");
        reader
            .samples::<f32>()
            .map(|s| s.expect("sample"))
            .collect()
    }

    #[test]
    fn reversing_writes_the_used_range_backwards() {
        let root = std::env::temp_dir().join(format!("opentrack-reverse-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let source = root.join("ramp.wav");
        let destination = root.join("reversed.wav");
        write_ramp(&source, 1_000);

        let frames = reverse_wav_range(&source, 0.002, 0.002, &destination).expect("reverse");
        assert_eq!(frames, 96, "the range is 96 frames at 48 kHz");
        let samples = read_mono(&destination);
        assert_eq!(samples.len(), 96);
        assert!(samples[0] > samples[95], "the ramp now falls");
        // The reversed data is the used window, not the whole file.
        let original = read_mono(&source);
        assert!((samples[0] - original[191]).abs() < 1e-6);
        assert!((samples[95] - original[96]).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reversing_refuses_unusable_ranges_and_missing_files() {
        let root =
            std::env::temp_dir().join(format!("opentrack-reverse-errors-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let source = root.join("ramp.wav");
        write_ramp(&source, 480);
        let destination = root.join("out.wav");

        assert!(reverse_wav_range(&source, 0.0, 0.0, &destination).is_err());
        assert!(reverse_wav_range(&source, -1.0, 0.1, &destination).is_err());
        // A range past the end has no frames.
        assert!(reverse_wav_range(&source, 20.0, 0.1, &destination).is_err());
        assert!(reverse_wav_range(&root.join("missing.wav"), 0.0, 0.1, &destination).is_err());
        assert!(!destination.exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
