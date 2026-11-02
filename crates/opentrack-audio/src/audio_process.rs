// SPDX-License-Identifier: Apache-2.0

//! Offline source processing that writes new media instead of editing in place.
//!
//! These operations are deliberately separate from the playback graph: they read
//! the source file, produce a new WAV in the project, and let the project point an
//! event at it. Nothing here runs on the audio callback.

use std::path::Path;

/// Largest sample payload one operation may buffer, matching the import limit.
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
    let range = read_range(source, offset_seconds, duration_seconds)?;
    let mut samples = Vec::with_capacity(range.samples.len());
    for frame in (0..range.frames as usize).rev() {
        let base = frame * usize::from(range.channels);
        for channel in 0..usize::from(range.channels) {
            samples.push(range.samples[base + channel]);
        }
    }
    write_float_wav(destination, range.channels, range.sample_rate, &samples)?;
    Ok(range.frames)
}

/// Subtracts each channel's mean from the used range and writes new media.
///
/// Returns the frame count and the removed offset per channel (left, right; the
/// right value repeats the left for mono), so the caller can report what was
/// removed.
pub fn remove_dc_offset_wav_range(
    source: &Path,
    offset_seconds: f64,
    duration_seconds: f64,
    destination: &Path,
) -> Result<(u64, [f32; 2]), String> {
    let range = read_range(source, offset_seconds, duration_seconds)?;
    let channels = usize::from(range.channels);
    let mut sums = [0.0_f64; 2];
    for frame in range.samples.chunks_exact(channels) {
        for channel in 0..channels {
            sums[channel] += f64::from(frame[channel]);
        }
    }
    let mut offsets = [0.0_f32; 2];
    for channel in 0..channels {
        offsets[channel] = (sums[channel] / range.frames as f64) as f32;
    }
    if channels == 1 {
        offsets[1] = offsets[0];
    }
    let mut samples = Vec::with_capacity(range.samples.len());
    for frame in range.samples.chunks_exact(channels) {
        for channel in 0..channels {
            samples.push((frame[channel] - offsets[channel]).clamp(-1.0, 1.0));
        }
    }
    write_float_wav(destination, range.channels, range.sample_rate, &samples)?;
    Ok((range.frames, offsets))
}

struct WavRange {
    channels: u16,
    sample_rate: u32,
    frames: u64,
    samples: Vec<f32>,
}

/// Reads only the frames an event uses, bounded and format-normalized.
fn read_range(
    source: &Path,
    offset_seconds: f64,
    duration_seconds: f64,
) -> Result<WavRange, String> {
    if !offset_seconds.is_finite()
        || offset_seconds < 0.0
        || !duration_seconds.is_finite()
        || duration_seconds <= 0.0
    {
        return Err("the processing range is not usable".to_owned());
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
        return Err("the event has no audio inside the source file".to_owned());
    }
    let sample_count = frames
        .checked_mul(channels as u64)
        .ok_or_else(|| "the processing range is too large".to_owned())?;
    if sample_count
        .checked_mul(4)
        .is_none_or(|bytes| bytes > MAX_PROCESS_BYTES)
    {
        return Err("the processing range is larger than the 512 MiB limit".to_owned());
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
    Ok(WavRange {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        frames,
        samples,
    })
}

fn write_float_wav(
    destination: &Path,
    channels: u16,
    sample_rate: u32,
    samples: &[f32],
) -> Result<(), String> {
    if let Some(parent) = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let output = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(destination, output)
        .map_err(|error| format!("could not create the processed WAV: {error}"))?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|error| format!("could not write the processed WAV: {error}"))?;
    }
    writer
        .finalize()
        .map_err(|error| format!("could not finish the processed WAV: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(path: &Path, channels: u16, frames: usize, sample: impl Fn(usize, usize) -> f32) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("writer");
        for frame in 0..frames {
            for channel in 0..usize::from(channels) {
                writer.write_sample(sample(frame, channel)).expect("sample");
            }
        }
        writer.finalize().expect("finalize");
    }

    fn write_ramp(path: &Path, frames: usize) {
        write_wav(path, 1, frames, |frame, _| frame as f32 / frames as f32);
    }

    fn read(path: &Path) -> (u16, Vec<f32>) {
        let mut reader = hound::WavReader::open(path).expect("reader");
        let channels = reader.spec().channels;
        (
            channels,
            reader
                .samples::<f32>()
                .map(|s| s.expect("sample"))
                .collect(),
        )
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
        let (_, samples) = read(&destination);
        assert_eq!(samples.len(), 96);
        assert!(samples[0] > samples[95], "the ramp now falls");
        let (_, original) = read(&source);
        assert!((samples[0] - original[191]).abs() < 1e-6);
        assert!((samples[95] - original[96]).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn removing_dc_offset_centres_each_channel_of_the_used_range() {
        let root = std::env::temp_dir().join(format!("opentrack-dc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let source = root.join("dc.wav");
        let destination = root.join("centred.wav");
        // Left sits at +0.25 with a small ramp, right at -0.5.
        write_wav(&source, 2, 4_800, |frame, channel| {
            let ripple = (frame % 10) as f32 * 0.001;
            if channel == 0 {
                0.25 + ripple
            } else {
                -0.5 + ripple
            }
        });

        let (frames, offsets) =
            remove_dc_offset_wav_range(&source, 0.0, 0.05, &destination).expect("dc");
        assert_eq!(frames, 2_400);
        assert!((offsets[0] - 0.2545).abs() < 0.01, "{offsets:?}");
        assert!((offsets[1] + 0.4955).abs() < 0.01, "{offsets:?}");
        let (channels, samples) = read(&destination);
        assert_eq!(channels, 2);
        let frames = samples.len() / 2;
        let left_mean = samples
            .iter()
            .step_by(2)
            .map(|s| f64::from(*s))
            .sum::<f64>()
            / frames as f64;
        let right_mean = samples
            .iter()
            .skip(1)
            .step_by(2)
            .map(|s| f64::from(*s))
            .sum::<f64>()
            / frames as f64;
        assert!(left_mean.abs() < 1e-3, "left mean {left_mean}");
        assert!(right_mean.abs() < 1e-3, "right mean {right_mean}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn source_processing_refuses_unusable_ranges_and_missing_files() {
        let root =
            std::env::temp_dir().join(format!("opentrack-process-errors-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let source = root.join("ramp.wav");
        write_ramp(&source, 480);
        let destination = root.join("out.wav");

        for result in [
            reverse_wav_range(&source, 0.0, 0.0, &destination),
            reverse_wav_range(&source, -1.0, 0.1, &destination),
            reverse_wav_range(&source, 20.0, 0.1, &destination),
            reverse_wav_range(&root.join("missing.wav"), 0.0, 0.1, &destination),
        ] {
            assert!(result.is_err());
        }
        assert!(remove_dc_offset_wav_range(&source, 0.0, 0.0, &destination).is_err());
        assert!(
            remove_dc_offset_wav_range(&root.join("missing.wav"), 0.0, 0.1, &destination).is_err()
        );
        assert!(!destination.exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
