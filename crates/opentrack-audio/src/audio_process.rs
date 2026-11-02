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
    let range = read_range(source, offset_seconds, duration_seconds, false)?;
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

/// What one trimming pass did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrimOutcome {
    /// Frames the used range covers.
    pub frames: u64,
    /// Frames the source file holds.
    pub source_frames: u64,
    /// False when the range already covers the whole file and nothing was written.
    pub written: bool,
}

/// Writes only the frames the event uses, so unused audio can be dropped.
///
/// Refuses a range that runs past the end of the source, because a shorter file
/// would silently truncate what the event plays. A range that already covers the
/// whole file reports `written: false` and creates no file.
pub fn trim_wav_range(
    source: &Path,
    offset_seconds: f64,
    duration_seconds: f64,
    destination: &Path,
) -> Result<TrimOutcome, String> {
    let range = read_range(source, offset_seconds, duration_seconds, true)?;
    if range.frames == range.source_frames {
        return Ok(TrimOutcome {
            frames: range.frames,
            source_frames: range.source_frames,
            written: false,
        });
    }
    write_float_wav(
        destination,
        range.channels,
        range.sample_rate,
        &range.samples,
    )?;
    Ok(TrimOutcome {
        frames: range.frames,
        source_frames: range.source_frames,
        written: true,
    })
}

/// What one normalization did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizeOutcome {
    pub frames: u64,
    /// Peak amplitude found in the used range.
    pub peak: f32,
    /// Linear gain applied (or that would be applied when already at target).
    pub gain: f32,
    /// False when the material was already at the target and no file was written.
    pub written: bool,
}

/// Scales the used range so its peak reaches `target_peak`, writing new media.
///
/// A silent range has no peak to normalize and is refused; material already at
/// the target is reported without writing a duplicate file.
pub fn normalize_wav_range(
    source: &Path,
    offset_seconds: f64,
    duration_seconds: f64,
    target_peak: f32,
    destination: &Path,
) -> Result<NormalizeOutcome, String> {
    if !target_peak.is_finite() || target_peak <= 0.0 || target_peak > 1.0 {
        return Err("the normalize target must be above 0 and at most 1.0".to_owned());
    }
    let range = read_range(source, offset_seconds, duration_seconds, false)?;
    let peak = range
        .samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    if peak <= 1e-7 {
        return Err("the event is silent; there is nothing to normalize".to_owned());
    }
    let gain = (target_peak / peak).clamp(0.0, 64.0);
    if (gain - 1.0).abs() <= 1e-3 {
        return Ok(NormalizeOutcome {
            frames: range.frames,
            peak,
            gain,
            written: false,
        });
    }
    let samples = range
        .samples
        .iter()
        .map(|sample| (sample * gain).clamp(-1.0, 1.0))
        .collect::<Vec<_>>();
    write_float_wav(destination, range.channels, range.sample_rate, &samples)?;
    Ok(NormalizeOutcome {
        frames: range.frames,
        peak,
        gain,
        written: true,
    })
}

/// Swaps the left and right channels of the used range into new media.
///
/// A mono source has nothing to flip and is refused rather than silently
/// duplicated; the remainder is written unchanged.
pub fn flip_stereo_wav_range(
    source: &Path,
    offset_seconds: f64,
    duration_seconds: f64,
    destination: &Path,
) -> Result<u64, String> {
    let range = read_range(source, offset_seconds, duration_seconds, false)?;
    if range.channels != 2 {
        return Err("Stereo Flip needs a stereo event; this source is mono".to_owned());
    }
    let mut samples = Vec::with_capacity(range.samples.len());
    for frame in range.samples.chunks_exact(2) {
        samples.push(frame[1]);
        samples.push(frame[0]);
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
    let range = read_range(source, offset_seconds, duration_seconds, false)?;
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
    source_frames: u64,
    samples: Vec<f32>,
}

/// Reads only the frames an event uses, bounded and format-normalized.
fn read_range(
    source: &Path,
    offset_seconds: f64,
    duration_seconds: f64,
    require_exact: bool,
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
    let requested = (duration_seconds * rate).round() as u64;
    let frames = requested.min(total_frames - start_frame);
    if frames == 0 {
        return Err("the event has no audio inside the source file".to_owned());
    }
    if require_exact && frames != requested {
        return Err("the event extends past the end of the source file".to_owned());
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
        source_frames: total_frames,
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
    fn normalizing_scales_the_peak_and_skips_already_matching_audio() {
        let root = std::env::temp_dir().join(format!("opentrack-normalize-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let source = root.join("quiet.wav");
        let destination = root.join("loud.wav");
        // A ramp reaching 0.25.
        write_wav(&source, 1, 4_800, |frame, _| {
            (frame as f32 / 4_799.0) * 0.25
        });

        let outcome = normalize_wav_range(&source, 0.0, 0.1, 1.0, &destination).expect("normalize");
        assert_eq!(outcome.frames, 4_800);
        assert!((outcome.peak - 0.25).abs() < 1e-4, "{outcome:?}");
        assert!((outcome.gain - 4.0).abs() < 1e-3, "{outcome:?}");
        assert!(outcome.written);
        let (_, samples) = read(&destination);
        let peak = samples.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()));
        assert!((peak - 1.0).abs() < 1e-4, "peak {peak}");

        // Already at the target: no duplicate file.
        let at_target = root.join("at-target.wav");
        std::fs::copy(&destination, &at_target).expect("copy");
        let second = root.join("second.wav");
        let outcome =
            normalize_wav_range(&at_target, 0.0, 0.1, 1.0, &second).expect("already at target");
        assert!(!outcome.written);
        assert!(!second.exists());

        // Silence has nothing to scale; bad targets are refused.
        let silent = root.join("silent.wav");
        write_wav(&silent, 1, 480, |_, _| 0.0);
        assert!(normalize_wav_range(&silent, 0.0, 0.1, 1.0, &second).is_err());
        assert!(normalize_wav_range(&source, 0.0, 0.1, 0.0, &second).is_err());
        assert!(normalize_wav_range(&source, 0.0, 0.1, 1.5, &second).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn trimming_keeps_only_the_used_frames() {
        let root = std::env::temp_dir().join(format!("opentrack-trim-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let source = root.join("full.wav");
        let destination = root.join("trimmed.wav");
        write_ramp(&source, 48_000);

        // 0.1 s at 48 kHz is 4,800 frames.
        let outcome = trim_wav_range(&source, 0.25, 0.1, &destination).expect("trim");
        assert_eq!(outcome.frames, 4_800);
        assert_eq!(outcome.source_frames, 48_000);
        assert!(outcome.written);
        let (channels, samples) = read(&destination);
        assert_eq!(channels, 1);
        assert_eq!(samples.len(), 4_800);
        let (_, original) = read(&source);
        assert!((samples[0] - original[12_000]).abs() < 1e-6);

        // A range that covers the whole file is reported, not rewritten.
        let whole = root.join("whole.wav");
        let outcome = trim_wav_range(&source, 0.0, 1.0, &whole).expect("whole");
        assert_eq!(outcome.frames, 48_000);
        assert!(!outcome.written);
        assert!(!whole.exists());

        // A range that runs past the end is refused rather than truncated.
        let too_long = trim_wav_range(&source, 0.9, 0.5, &root.join("no.wav"));
        assert!(too_long.is_err(), "the source would be truncated");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn stereo_flip_swaps_channels_and_refuses_mono() {
        let root = std::env::temp_dir().join(format!("opentrack-flip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let stereo = root.join("stereo.wav");
        let destination = root.join("flipped.wav");
        // Left is +0.5, right is -0.25, with one ramp frame at the end.
        write_wav(&stereo, 2, 4_800, |frame, channel| {
            if frame == 4_799 {
                if channel == 0 { 0.1 } else { 0.2 }
            } else if channel == 0 {
                0.5
            } else {
                -0.25
            }
        });

        let frames = flip_stereo_wav_range(&stereo, 0.0, 0.1, &destination).expect("flip");
        assert_eq!(frames, 4_800);
        let (channels, samples) = read(&destination);
        assert_eq!(channels, 2);
        assert!((samples[0] + 0.25).abs() < 1e-6, "left is the old right");
        assert!((samples[1] - 0.5).abs() < 1e-6, "right is the old left");
        let last = samples.len() - 2;
        assert!((samples[last] - 0.2).abs() < 1e-6);
        assert!((samples[last + 1] - 0.1).abs() < 1e-6);

        // Only the requested range is written.
        let short = root.join("short.wav");
        flip_stereo_wav_range(&stereo, 0.05, 0.01, &short).expect("short flip");
        let (_, short_samples) = read(&short);
        assert_eq!(short_samples.len(), 960);

        // Mono cannot be flipped.
        let mono = root.join("mono.wav");
        write_ramp(&mono, 480);
        assert!(flip_stereo_wav_range(&mono, 0.0, 0.1, &root.join("no.wav")).is_err());
        assert!(flip_stereo_wav_range(&stereo, 0.0, 0.0, &root.join("no.wav")).is_err());
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
