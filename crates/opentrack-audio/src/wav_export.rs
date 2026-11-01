//! Bounded-memory WAV output for the offline playback graph.

use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use hound::{SampleFormat, WavSpec, WavWriter};
use opentrack_core::project::RecordingFormat;

use crate::{OfflineRenderError, OfflineRenderer};

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(1);
const FRAMES_PER_CHUNK: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WavExportError {
    Cancelled,
    InvalidPath,
    TooLarge,
    NonFiniteSample { frame: u64, channel: u8 },
    Render(OfflineRenderError),
    Io(String),
    Wav(String),
}

impl std::fmt::Display for WavExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("mixdown was cancelled"),
            Self::InvalidPath => {
                f.write_str("mixdown destination must have a file name and parent folder")
            }
            Self::TooLarge => {
                f.write_str("mixdown exceeds the maximum size of a standard WAV file")
            }
            Self::NonFiniteSample { frame, channel } => {
                write!(
                    f,
                    "mixdown produced a non-finite sample at frame {frame}, channel {channel}"
                )
            }
            Self::Render(error) => write!(f, "offline audio rendering failed: {error}"),
            Self::Io(error) => write!(f, "mixdown file operation failed: {error}"),
            Self::Wav(error) => write!(f, "could not write WAV data: {error}"),
        }
    }
}

impl std::error::Error for WavExportError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WavExportReport {
    pub frames: u64,
    pub sample_rate: u32,
    pub format: RecordingFormat,
    pub peak_linear: f32,
}

struct TemporaryOutput {
    path: PathBuf,
    committed: bool,
}

impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Renders a stereo mix to a sibling temporary file and renames it into place
/// only after the WAV header is finalized. Progress callbacks run once per
/// bounded 256-frame chunk. Cancellation is checked between chunks.
pub fn write_mixdown_wav(
    renderer: &mut OfflineRenderer,
    destination: &Path,
    format: RecordingFormat,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<WavExportReport, WavExportError> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = destination.file_name().ok_or(WavExportError::InvalidPath)?;
    if !parent.is_dir() {
        return Err(WavExportError::Io(format!(
            "destination folder does not exist: {}",
            parent.display()
        )));
    }
    let bytes_per_sample = u64::from(format.bits_per_sample() / 8);
    let data_bytes = renderer
        .total_frames()
        .checked_mul(2)
        .and_then(|samples| samples.checked_mul(bytes_per_sample))
        .ok_or(WavExportError::TooLarge)?;
    if data_bytes > u64::from(u32::MAX).saturating_sub(36) {
        return Err(WavExportError::TooLarge);
    }

    let (temporary, file) = create_temporary_sibling(parent, file_name)?;
    let mut temporary = TemporaryOutput {
        path: temporary,
        committed: false,
    };
    let spec = WavSpec {
        channels: 2,
        sample_rate: renderer.sample_rate(),
        bits_per_sample: format.bits_per_sample(),
        sample_format: if format == RecordingFormat::Float32 {
            SampleFormat::Float
        } else {
            SampleFormat::Int
        },
    };
    let mut writer =
        WavWriter::new(file, spec).map_err(|error| WavExportError::Wav(error.to_string()))?;
    let mut samples = [0.0_f32; FRAMES_PER_CHUNK * 2];
    let mut peak_linear = 0.0_f32;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err(WavExportError::Cancelled);
        }
        let rendered = renderer
            .render_next(&mut samples)
            .map_err(WavExportError::Render)?;
        if rendered == 0 {
            break;
        }
        for (index, sample) in samples[..rendered * 2].iter().copied().enumerate() {
            if !sample.is_finite() {
                let frame = renderer.rendered_frames() - rendered as u64 + (index / 2) as u64;
                return Err(WavExportError::NonFiniteSample {
                    frame,
                    channel: (index % 2) as u8,
                });
            }
            peak_linear = peak_linear.max(sample.abs());
            match format {
                RecordingFormat::Pcm16 => {
                    let scaled = if sample <= -1.0 {
                        i16::MIN
                    } else {
                        (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
                    };
                    writer
                        .write_sample(scaled)
                        .map_err(|error| WavExportError::Wav(error.to_string()))?;
                }
                RecordingFormat::Pcm24 => {
                    const MAX: f32 = 8_388_607.0;
                    const MIN: f32 = -8_388_608.0;
                    let scaled = (sample.clamp(-1.0, 1.0) * MAX).round() as i32;
                    let scaled = if sample <= -1.0 { MIN as i32 } else { scaled };
                    writer
                        .write_sample(scaled)
                        .map_err(|error| WavExportError::Wav(error.to_string()))?;
                }
                RecordingFormat::Float32 => writer
                    .write_sample(sample)
                    .map_err(|error| WavExportError::Wav(error.to_string()))?,
            }
        }
        progress(renderer.rendered_frames(), renderer.total_frames());
    }
    if cancel.load(Ordering::Acquire) {
        return Err(WavExportError::Cancelled);
    }
    writer
        .finalize()
        .map_err(|error| WavExportError::Wav(error.to_string()))?;
    commit_temporary(&temporary.path, destination).map_err(|error| {
        WavExportError::Io(format!(
            "could not finish {}: {error}",
            destination.display()
        ))
    })?;
    temporary.committed = true;
    Ok(WavExportReport {
        frames: renderer.rendered_frames(),
        sample_rate: renderer.sample_rate(),
        format,
        peak_linear,
    })
}

#[cfg(windows)]
fn commit_temporary(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
    // SAFETY: both paths are NUL-terminated UTF-16 buffers that remain alive for the call.
    let result = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), flags) };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn commit_temporary(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

fn create_temporary_sibling(
    parent: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<(PathBuf, File), WavExportError> {
    for _ in 0..16 {
        let id = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let mut name = std::ffi::OsString::from(".");
        name.push(file_name);
        name.push(format!(".opentrack-{}-{id}.tmp", std::process::id()));
        let path = parent.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(WavExportError::Io(error.to_string())),
        }
    }
    Err(WavExportError::Io(
        "could not reserve a unique temporary file name".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChannelEqualizer, InstrumentWaveform, PlaybackMidiNote, PlaybackOutputRoute,
        PlaybackTrackSettings, StereoMeter, oscillator_instrument_factory,
    };
    use opentrack_core::project::{BusChannel, InsertEffect, RecordingFormat};
    use std::sync::Arc;

    fn renderer() -> OfflineRenderer {
        let track = PlaybackTrackSettings {
            is_group: false,
            audio_channels: 2,
            clips: Vec::new(),
            midi_notes: vec![PlaybackMidiNote {
                id: 1,
                start_beats: 0.0,
                duration_beats: 1.0,
                frequency_hz: 440.0,
                channel: 0,
                velocity: 1.0,
            }],
            midi_controllers: Vec::new(),
            instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
            meter_peak: Arc::new(StereoMeter::default()),
            gain: 1.0,
            pan: 0.0,
            input_gain_db: 0.0,
            input_phase_inverted: false,
            track_delay_ms: 0.0,
            muted: false,
            solo: false,
            solo_defeat: false,
            equalizer: ChannelEqualizer::default(),
            inserts: Vec::<InsertEffect>::new(),
            inserts_bypassed: false,
            hosted_inserts: Vec::new(),
            monitor_input: None,
            output_route: PlaybackOutputRoute::Stereo,
        };
        OfflineRenderer::new(
            120.0,
            vec![track],
            48_000,
            0.0,
            0.02,
            &BusChannel::output().strip,
            Vec::new(),
        )
        .unwrap()
    }

    fn destination(label: &str) -> PathBuf {
        let id = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("opentrack-{label}-{}-{id}.wav", std::process::id()))
    }

    #[test]
    fn writes_valid_stereo_wav_in_each_supported_encoding() {
        for format in [
            RecordingFormat::Pcm16,
            RecordingFormat::Pcm24,
            RecordingFormat::Float32,
        ] {
            let path = destination("encoding");
            let report = write_mixdown_wav(
                &mut renderer(),
                &path,
                format,
                &AtomicBool::new(false),
                |_, _| {},
            )
            .unwrap();
            assert_eq!(report.frames, 480);
            assert_eq!(report.sample_rate, 48_000);
            assert!(report.peak_linear > 0.0);
            let mut reader = hound::WavReader::open(&path).unwrap();
            let spec = reader.spec();
            assert_eq!(spec.channels, 2);
            assert_eq!(spec.sample_rate, 48_000);
            assert_eq!(spec.bits_per_sample, format.bits_per_sample());
            assert_eq!(
                spec.sample_format,
                if format == RecordingFormat::Float32 {
                    SampleFormat::Float
                } else {
                    SampleFormat::Int
                }
            );
            assert_eq!(reader.duration(), 480);
            let has_signal = match format {
                RecordingFormat::Pcm16 => reader
                    .samples::<i16>()
                    .any(|sample| sample.is_ok_and(|value| value != 0)),
                RecordingFormat::Pcm24 => reader
                    .samples::<i32>()
                    .any(|sample| sample.is_ok_and(|value| value != 0)),
                RecordingFormat::Float32 => reader
                    .samples::<f32>()
                    .any(|sample| sample.is_ok_and(|value| value.abs() > 1.0e-6)),
            };
            assert!(has_signal, "{format:?} output was silent");
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn cancellation_removes_temporary_output_and_leaves_destination_absent() {
        let path = destination("cancelled");
        let parent = path.parent().unwrap();
        let prefix = format!(
            ".{}.opentrack-",
            path.file_name().unwrap().to_string_lossy()
        );
        let existing: std::collections::HashSet<_> = fs::read_dir(parent)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        let cancel = AtomicBool::new(false);
        let result = write_mixdown_wav(
            &mut renderer(),
            &path,
            RecordingFormat::Float32,
            &cancel,
            |done, _| {
                if done > 0 {
                    cancel.store(true, Ordering::Release);
                }
            },
        );
        assert_eq!(result, Err(WavExportError::Cancelled));
        assert!(!path.exists());
        let remaining: Vec<_> = fs::read_dir(parent)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().starts_with(&prefix))
            .filter(|name| !existing.contains(name))
            .collect();
        assert!(
            remaining.is_empty(),
            "temporary files remain: {remaining:?}"
        );
    }

    #[test]
    fn completed_export_replaces_an_existing_destination() {
        let path = destination("replace");
        fs::write(&path, b"old destination").unwrap();
        write_mixdown_wav(
            &mut renderer(),
            &path,
            RecordingFormat::Pcm16,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().bits_per_sample, 16);
        assert_eq!(reader.duration(), 480);
        fs::remove_file(path).unwrap();
    }
}
