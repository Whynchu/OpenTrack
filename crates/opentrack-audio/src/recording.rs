//! Selected-input capture with a bounded single-producer/single-consumer queue.
//!
//! The capture callback only normalizes samples and copies into preallocated
//! storage. A worker thread drains the queue, converts the project sample rate,
//! and writes the selected WAV encoding.

use opentrack_core::project::RecordingFormat;
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[cfg(not(windows))]
use cpal::traits::DeviceTrait;
#[cfg(not(windows))]
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig, traits::StreamTrait};

const QUEUE_CAPACITY: usize = 1 << 17;
const QUEUE_MASK: usize = QUEUE_CAPACITY - 1;
const DRAIN_BUFFER_FRAMES: usize = 16 * 1024;
const MONITOR_CAPACITY: usize = 1 << 11;
const MONITOR_MASK: u64 = MONITOR_CAPACITY as u64 - 1;
static NEXT_MONITOR_ID: AtomicU64 = AtomicU64::new(1);

/// Summary for a finalized recording file.
#[derive(Clone, Debug)]
pub struct RecordingSummary {
    pub path: PathBuf,
    pub frames_written: u64,
    pub dropped_frames: u64,
    pub monitor_dropped_frames: u64,
    pub input_stream_failed: bool,
    pub input_discontinuities: u64,
    pub input_timestamp_errors: u64,
    pub capture_start_timestamp_hns: u64,
}

/// Physical input bus captured into one armed track's take.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioRecordRoute {
    Stereo,
    Left,
    Right,
}

/// Lock-free input-to-output bridge used for live monitoring during capture.
pub struct AudioMonitorBuffer {
    bus: crate::bus::InputBusProcessor,
    id: u64,
    sample_rate: u32,
    enabled: AtomicBool,
    monitor_start_frame: AtomicU64,
    monitor_epoch: AtomicU64,
    read_index: AtomicU64,
    write_index: AtomicU64,
    dropped_frames: AtomicU64,
    samples: Box<[AtomicU32]>,
}

impl AudioMonitorBuffer {
    pub(crate) fn new(sample_rate: u32) -> Self {
        let samples = (0..MONITOR_CAPACITY * 2)
            .map(|_| AtomicU32::new(0.0_f32.to_bits()))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            bus: crate::bus::InputBusProcessor::new(sample_rate),
            id: NEXT_MONITOR_ID.fetch_add(1, Ordering::Relaxed),
            sample_rate,
            enabled: AtomicBool::new(false),
            monitor_start_frame: AtomicU64::new(0),
            monitor_epoch: AtomicU64::new(0),
            read_index: AtomicU64::new(0),
            write_index: AtomicU64::new(0),
            dropped_frames: AtomicU64::new(0),
            samples,
        }
    }

    pub fn set_input_channel(&self, settings: &opentrack_core::project::Track) {
        self.bus.set(settings);
    }
    pub fn meter_tap(&self) -> Arc<crate::StereoMeter> {
        self.bus.meter_tap()
    }
    pub fn take_bus_peaks(&self) -> [f32; 2] {
        self.bus.take_peaks()
    }
    pub(crate) fn process_input(&self, left: f32, right: f32) -> (f32, f32) {
        self.bus.process(left, right)
    }
    pub fn set_enabled(&self, enabled: bool) {
        if enabled {
            if !self.enabled.load(Ordering::Acquire) {
                self.monitor_start_frame
                    .store(self.write_index.load(Ordering::Acquire), Ordering::Release);
                self.monitor_epoch.fetch_add(1, Ordering::Release);
            }
            self.enabled.store(true, Ordering::Release);
        } else {
            self.enabled.store(false, Ordering::Release);
        }
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Acquire)
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub(crate) fn available_from(&self, frame: u64) -> u64 {
        self.write_index
            .load(Ordering::Acquire)
            .saturating_sub(frame)
            .min(MONITOR_CAPACITY as u64)
    }

    pub(crate) fn read_index(&self) -> u64 {
        self.read_index.load(Ordering::Relaxed)
    }

    pub(crate) fn monitor_start_frame(&self) -> u64 {
        self.monitor_start_frame.load(Ordering::Acquire)
    }

    pub(crate) fn monitor_epoch(&self) -> u64 {
        self.monitor_epoch.load(Ordering::Acquire)
    }

    pub(crate) fn push_frame(&self, left: f32, right: f32) -> bool {
        let write = self.write_index.load(Ordering::Relaxed);
        let read = self.read_index.load(Ordering::Acquire);
        if write.wrapping_sub(read) >= MONITOR_CAPACITY as u64 {
            return false;
        }
        let slot = (write & MONITOR_MASK) as usize * 2;
        self.samples[slot].store(left.to_bits(), Ordering::Relaxed);
        self.samples[slot + 1].store(right.to_bits(), Ordering::Relaxed);
        self.write_index
            .store(write.wrapping_add(1), Ordering::Release);
        true
    }

    pub(crate) fn add_dropped_frames(&self, frames: u64) {
        if frames > 0 {
            self.dropped_frames.fetch_add(frames, Ordering::Relaxed);
        }
    }

    pub(crate) fn sample_frame(&self, frame: u64) -> Option<(f32, f32)> {
        let read = self.read_index.load(Ordering::Relaxed);
        let write = self.write_index.load(Ordering::Acquire);
        if frame < read || frame >= write {
            return None;
        }
        let slot = (frame & MONITOR_MASK) as usize * 2;
        Some((
            f32::from_bits(self.samples[slot].load(Ordering::Relaxed)),
            f32::from_bits(self.samples[slot + 1].load(Ordering::Relaxed)),
        ))
    }

    pub(crate) fn advance_read_to(&self, frame: u64) {
        let write = self.write_index.load(Ordering::Acquire);
        let current = self.read_index.load(Ordering::Relaxed);
        let next = frame.min(write);
        if next > current {
            self.read_index.store(next, Ordering::Release);
        }
    }
}

/// An input device available for recording. The CPAL identifier is stable
/// across launches where the audio backend can provide a persistent ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioInputDevice {
    pub id: String,
    pub name: String,
}

/// Default capture parameters reported for an input device, without opening a stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioInputFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

#[cfg(all(windows, feature = "local-asio"))]
pub(crate) struct AsioCapture {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub period_frames: AtomicU32,
    peaks: Arc<[AtomicU32]>,
    monitor: Arc<AudioMonitorBuffer>,
    pub failed: Arc<AtomicBool>,
    recording: arc_swap::ArcSwapOption<AsioRecording>,
    callback_reads: AtomicUsize,
}

#[cfg(all(windows, feature = "local-asio"))]
struct AsioRecording {
    ring: Arc<SpscAudioRing>,
    enabled: Arc<AtomicBool>,
    callbacks: Arc<AtomicUsize>,
}

#[cfg(all(windows, feature = "local-asio"))]
impl AsioCapture {
    pub(crate) fn new(
        device_name: String,
        sample_rate: u32,
        channels: u16,
        _output_failed: Arc<AtomicBool>,
    ) -> Self {
        Self {
            device_name,
            sample_rate,
            channels,
            period_frames: AtomicU32::new(0),
            peaks: (0..channels)
                .map(|_| AtomicU32::new(0))
                .collect::<Vec<_>>()
                .into(),
            monitor: Arc::new(AudioMonitorBuffer::new(sample_rate)),
            failed: Arc::new(AtomicBool::new(false)),
            recording: arc_swap::ArcSwapOption::empty(),
            callback_reads: AtomicUsize::new(0),
        }
    }
    pub(crate) fn push(&self, samples: &mut [f32]) {
        self.callback_reads.fetch_add(1, Ordering::AcqRel);
        record_probe_samples(samples, self.channels, &self.peaks, None);
        for frame in samples.chunks_exact_mut(self.channels as usize) {
            let left = if frame[0].is_finite() { frame[0] } else { 0.0 };
            let right = frame
                .get(1)
                .copied()
                .filter(|s| s.is_finite())
                .unwrap_or(left);
            let (left, right) = self.monitor.process_input(left, right);
            frame[0] = left;
            if frame.len() > 1 {
                frame[1] = right;
            }
            if self.monitor.is_enabled() && !self.monitor.push_frame(left, right) {
                self.monitor.add_dropped_frames(1);
            }
        }
        if let Some(recording) = self.recording.load().as_ref() {
            push_if_enabled(
                &recording.ring,
                None,
                &recording.enabled,
                &recording.callbacks,
                samples,
                self.channels,
                None,
                |sample| {
                    if sample.is_finite() { sample } else { 0.0 }
                },
            );
        }
        self.callback_reads.fetch_sub(1, Ordering::Release);
    }
}

/// Independent input test stream used by Device Setup. It opens capture and
/// reports per-channel peaks without creating a recording file or enabling
/// software monitoring.
pub struct AudioInputProbe {
    #[cfg(not(windows))]
    _stream: cpal::Stream,
    #[cfg(windows)]
    _stream: Option<crate::wasapi_backend::WasapiInputStream>,
    #[cfg(all(windows, feature = "local-asio"))]
    _asio_capture: Option<Arc<AsioCapture>>,
    peaks: Arc<[AtomicU32]>,
    monitor_buffer: Arc<AudioMonitorBuffer>,
    failed: Arc<AtomicBool>,
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub input_mode: crate::AudioDeviceMode,
    pub input_period_frames: u32,
    pub input_buffer_frames: u32,
    pub input_mode_detail: String,
}

impl AudioInputProbe {
    pub fn open(device_id: Option<&str>) -> Result<Self, String> {
        Self::open_with_mode(device_id, crate::AudioDeviceMode::SharedLowLatency)
    }

    pub fn open_with_mode(
        device_id: Option<&str>,
        mode: crate::AudioDeviceMode,
    ) -> Result<Self, String> {
        #[cfg(all(windows, feature = "local-asio"))]
        if let Some(id) = device_id.filter(|id| id.starts_with("asio:")) {
            let capture = crate::asio_backend::capture(id)?;
            for peak in capture.peaks.iter() {
                peak.store(0, Ordering::Relaxed);
            }
            return Ok(Self {
                _stream: None,
                _asio_capture: Some(Arc::clone(&capture)),
                peaks: Arc::clone(&capture.peaks),
                monitor_buffer: Arc::clone(&capture.monitor),
                failed: Arc::clone(&capture.failed),
                device_name: capture.device_name.clone(),
                sample_rate: capture.sample_rate,
                channels: capture.channels,
                input_mode: crate::AudioDeviceMode::Exclusive,
                input_period_frames: capture.period_frames.load(Ordering::Acquire),
                input_buffer_frames: capture.period_frames.load(Ordering::Acquire),
                input_mode_detail: "ASIO full duplex".into(),
            });
        }
        #[cfg(not(windows))]
        let device = crate::devices::input_device(device_id)?;
        #[cfg(not(windows))]
        if !device.supports_input() {
            return Err("the selected endpoint does not expose audio capture".to_owned());
        }
        #[cfg(not(windows))]
        let device_name = device
            .description()
            .map(|description| description.name().to_owned())
            .unwrap_or_else(|_| device.to_string());
        #[cfg(windows)]
        let endpoint_id = match device_id {
            Some(id) => crate::wasapi_backend::endpoint_id_from_cpal(id)?,
            None => crate::wasapi_backend::default_endpoint_id(wasapi::Direction::Capture)?,
        };
        #[cfg(windows)]
        {
            const MAX_METER_CHANNELS: usize = 64;
            let peaks: Arc<[AtomicU32]> = (0..MAX_METER_CHANNELS)
                .map(|_| AtomicU32::new(0.0_f32.to_bits()))
                .collect::<Vec<_>>()
                .into();
            let monitor_cell = Arc::new(OnceLock::new());
            let stream_peaks = Arc::clone(&peaks);
            let stream_monitor = Arc::clone(&monitor_cell);
            let failed = Arc::new(AtomicBool::new(false));
            let stream_failed = Arc::clone(&failed);
            let stream = crate::wasapi_backend::WasapiInputStream::open(
                endpoint_id,
                mode,
                None,
                None,
                stream_failed,
                move |samples, channels, _packet| {
                    record_probe_samples(
                        samples,
                        channels,
                        &stream_peaks,
                        stream_monitor.get().map(Arc::as_ref),
                    );
                },
            )?;
            if stream.info.channels as usize > MAX_METER_CHANNELS {
                return Err(format!(
                    "the selected input exposes {} channels; meters support at most {MAX_METER_CHANNELS}",
                    stream.info.channels
                ));
            }
            let monitor_buffer = Arc::new(AudioMonitorBuffer::new(stream.info.sample_rate));
            let _ = monitor_cell.set(Arc::clone(&monitor_buffer));
            let info = stream.info.clone();
            Ok(Self {
                _stream: Some(stream),
                #[cfg(feature = "local-asio")]
                _asio_capture: None,
                peaks,
                monitor_buffer,
                failed,
                device_name: info.device_name,
                sample_rate: info.sample_rate,
                channels: info.channels,
                input_mode: info.mode,
                input_period_frames: info.period_frames,
                input_buffer_frames: info.buffer_frames,
                input_mode_detail: info.detail,
            })
        }
        #[cfg(not(windows))]
        {
            let supported = device
                .default_input_config()
                .map_err(|error| format!("could not open the input format: {error}"))?;
            let sample_format = supported.sample_format();
            let config: StreamConfig = supported.into();
            let channels = config.channels;
            let sample_rate = config.sample_rate;
            if channels == 0 || sample_rate == 0 {
                return Err(format!(
                    "the selected endpoint reported an invalid format ({channels} channels at {sample_rate} Hz)"
                ));
            }
            let peaks = (0..channels)
                .map(|_| AtomicU32::new(0.0_f32.to_bits()))
                .collect::<Vec<_>>()
                .into();
            let monitor_buffer = Arc::new(AudioMonitorBuffer::new(sample_rate));
            let failed = Arc::new(AtomicBool::new(false));
            let stream_peaks = Arc::clone(&peaks);
            let stream_monitor = Arc::clone(&monitor_buffer);
            let stream_failed = Arc::clone(&failed);
            let stream = match sample_format {
                SampleFormat::F32 => build_probe_stream::<f32>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::F64 => build_probe_stream::<f64>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::I8 => build_probe_stream::<i8>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::I16 => build_probe_stream::<i16>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::I24 => build_probe_stream::<cpal::I24>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::I32 => build_probe_stream::<i32>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::I64 => build_probe_stream::<i64>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::U8 => build_probe_stream::<u8>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::U16 => build_probe_stream::<u16>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::U24 => build_probe_stream::<cpal::U24>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::U32 => build_probe_stream::<u32>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::U64 => build_probe_stream::<u64>(
                    &device,
                    config,
                    stream_peaks,
                    stream_monitor,
                    stream_failed,
                ),
                SampleFormat::DsdU8 | SampleFormat::DsdU16 | SampleFormat::DsdU32 => {
                    return Err(format!(
                        "input sample format {sample_format:?} is not supported by the meter"
                    ));
                }
                _ => {
                    return Err(format!(
                        "input sample format {sample_format:?} is not supported by the meter"
                    ));
                }
            }
            .map_err(|error| format!("could not open capture on {device_name}: {error}"))?;
            stream
                .play()
                .map_err(|error| format!("could not start capture on {device_name}: {error}"))?;
            Ok(Self {
                _stream: stream,
                peaks,
                monitor_buffer,
                failed,
                device_name,
                sample_rate,
                channels,
                input_mode: crate::AudioDeviceMode::SharedCompatible,
                input_period_frames: 0,
                input_buffer_frames: 0,
                input_mode_detail: "CPAL platform default".to_owned(),
            })
        }
    }

    /// Returns and resets the peak level for each physical input channel.
    pub fn take_channel_peaks(&self) -> Vec<f32> {
        self.peaks
            .iter()
            .take(usize::from(self.channels))
            .map(|peak| f32::from_bits(peak.swap(0.0_f32.to_bits(), Ordering::AcqRel)))
            .collect()
    }

    pub fn take_stream_error(&self) -> bool {
        self.failed.swap(false, Ordering::AcqRel)
    }

    pub fn monitor_buffer(&self) -> Arc<AudioMonitorBuffer> {
        Arc::clone(&self.monitor_buffer)
    }
}

#[cfg(windows)]
fn record_probe_samples(
    samples: &[f32],
    channels: u16,
    peaks: &[AtomicU32],
    monitor: Option<&AudioMonitorBuffer>,
) {
    let channel_count = usize::from(channels);
    if channel_count == 0 {
        return;
    }
    for frame in samples.chunks_exact(channel_count) {
        let mut left = 0.0_f32;
        let mut right = 0.0_f32;
        for (channel, sample) in frame.iter().enumerate() {
            let value = if sample.is_finite() {
                sample.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            if let Some(peak) = peaks.get(channel) {
                peak.fetch_max(value.abs().to_bits(), Ordering::Relaxed);
            }
            match channel {
                0 => left = value,
                1 => right = value,
                _ => {}
            }
        }
        if let Some(monitor) = monitor {
            if channel_count == 1 {
                right = left;
            }
            let (left, right) = monitor.process_input(left, right);
            if monitor.is_enabled() && !monitor.push_frame(left, right) {
                monitor.add_dropped_frames(1);
            }
        }
    }
}

#[cfg(not(windows))]
fn build_probe_stream<T: SizedSample>(
    device: &cpal::Device,
    config: StreamConfig,
    peaks: Arc<[AtomicU32]>,
    monitor: Arc<AudioMonitorBuffer>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, cpal::Error>
where
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    device.build_input_stream(
        config,
        move |samples: &[T], _| {
            for frame in samples.chunks_exact(channels) {
                let mut left = 0.0_f32;
                let mut right = 0.0_f32;
                for (channel, sample) in frame.iter().enumerate() {
                    let value = (*sample).to_sample::<f32>();
                    if value.is_finite() {
                        let value = value.clamp(-1.0, 1.0);
                        peaks[channel].fetch_max(value.abs().to_bits(), Ordering::Relaxed);
                        if channel == 0 {
                            left = value;
                        } else if channel == 1 {
                            right = value;
                        }
                    }
                }
                if channels == 1 {
                    right = left;
                }
                let (left, right) = monitor.process_input(left, right);
                if monitor.is_enabled() {
                    if !monitor.push_frame(left, right) {
                        monitor.add_dropped_frames(1);
                    }
                }
            }
        },
        move |_| failed.store(true, Ordering::Release),
        Some(Duration::from_secs(2)),
    )
}

/// Active default-device input stream plus the background WAV writer.
pub struct AudioRecorder {
    #[cfg(not(windows))]
    stream: Option<cpal::Stream>,
    #[cfg(windows)]
    stream: Option<crate::wasapi_backend::WasapiInputStream>,
    #[cfg(all(windows, feature = "local-asio"))]
    asio_capture: Option<Arc<AsioCapture>>,
    ring: Arc<SpscAudioRing>,
    monitor_buffer: Arc<AudioMonitorBuffer>,
    stop: Arc<AtomicBool>,
    capture_enabled: Arc<AtomicBool>,
    callbacks_active: Arc<AtomicUsize>,
    input_stream_failed: Arc<AtomicBool>,
    input_discontinuities: Arc<AtomicU64>,
    input_timestamp_errors: Arc<AtomicU64>,
    capture_start_timestamp_hns: Arc<AtomicU64>,
    worker: Option<JoinHandle<Result<Vec<u64>, String>>>,
    pub path: PathBuf,
    paths: Vec<PathBuf>,
    pub device_name: String,
    /// Sample rate stored in the resulting WAV files.
    pub sample_rate: u32,
    /// Rate negotiated with the capture device.
    pub capture_sample_rate: u32,
    pub channels: u16,
    pub recording_format: RecordingFormat,
    pub input_mode: crate::AudioDeviceMode,
    pub input_period_frames: u32,
    pub input_buffer_frames: u32,
    pub input_mode_detail: String,
}

impl AudioRecorder {
    pub fn live_waveform(&self) -> Arc<crate::Waveform> {
        self.ring.waveform.load_full()
    }

    /// Reads the device's default input format without starting capture.
    pub fn input_format(device_id: Option<&str>) -> Result<AudioInputFormat, String> {
        #[cfg(all(windows, feature = "local-asio"))]
        if let Some(id) = device_id.filter(|id| id.starts_with("asio:")) {
            let capture = crate::asio_backend::capture(id)?;
            return Ok(AudioInputFormat {
                sample_rate: capture.sample_rate,
                channels: capture.channels,
            });
        }
        #[cfg(windows)]
        {
            let endpoint_id = match device_id {
                Some(id) => crate::wasapi_backend::endpoint_id_from_cpal(id)?,
                None => crate::wasapi_backend::default_endpoint_id(wasapi::Direction::Capture)?,
            };
            let (sample_rate, channels) =
                crate::wasapi_backend::query_input_mix_format(endpoint_id)?;
            if sample_rate == 0 || channels == 0 {
                return Err(format!(
                    "the WASAPI input format is invalid ({channels} channels at {sample_rate} Hz)"
                ));
            }
            return Ok(AudioInputFormat {
                sample_rate,
                channels,
            });
        }
        #[cfg(not(windows))]
        {
            let device = crate::devices::input_device(device_id)?;
            let config = device
                .default_input_config()
                .map_err(|error| format!("could not read the input format: {error}"))?;
            let format = AudioInputFormat {
                sample_rate: config.sample_rate(),
                channels: config.channels(),
            };
            if format.sample_rate == 0 || format.channels == 0 {
                return Err(format!(
                    "the default input format is invalid ({channels} channels at {sample_rate} Hz)",
                    channels = format.channels,
                    sample_rate = format.sample_rate
                ));
            }
            Ok(format)
        }
    }

    /// Resolves the selected or system-default capture endpoint's display name.
    pub fn input_device_name(device_id: Option<&str>) -> Result<String, String> {
        #[cfg(all(windows, feature = "local-asio"))]
        if let Some(id) = device_id.filter(|id| id.starts_with("asio:")) {
            return crate::asio_backend::driver_name(id);
        }
        #[cfg(windows)]
        {
            let endpoint_id = match device_id {
                Some(id) => crate::wasapi_backend::endpoint_id_from_cpal(id)?,
                None => crate::wasapi_backend::default_endpoint_id(wasapi::Direction::Capture)?,
            };
            crate::wasapi_backend::endpoint_friendly_name(endpoint_id)
        }
        #[cfg(not(windows))]
        {
            let device = crate::devices::input_device(device_id)?;
            Ok(device
                .description()
                .map(|description| description.name().to_owned())
                .unwrap_or_else(|_| device.to_string()))
        }
    }

    /// Lists input-capable devices without opening audio streams.
    pub fn available_input_devices() -> Result<Vec<AudioInputDevice>, String> {
        Ok(crate::devices::available_audio_devices()?
            .into_iter()
            .filter(|device| device.supports_input)
            .map(|device| AudioInputDevice {
                id: device.id,
                name: device.name,
            })
            .collect())
    }

    /// Returns and clears the highest normalized input sample since the last read.
    pub fn take_input_peak(&self) -> f32 {
        self.ring.take_input_peak()
    }

    /// Returns peak levels for the captured channels since the last read.
    /// Mono inputs populate the left value and leave the right value at zero.
    pub fn take_channel_peaks(&self) -> [f32; 2] {
        self.ring.take_channel_peaks(self.channels)
    }

    /// Returns the live-monitor bridge for the active capture stream.
    pub fn monitor_buffer(&self) -> Arc<AudioMonitorBuffer> {
        Arc::clone(&self.monitor_buffer)
    }

    /// Opens and starts recording from the default mono/stereo input device.
    /// Existing output files are never overwritten.
    pub fn start_default_input(path: &Path) -> Result<Self, String> {
        Self::start_input(path, None)
    }

    /// Opens and starts recording from a selected input device, or the system
    /// default when `device_id` is `None`. Existing output files are never
    /// overwritten.
    pub fn start_input(path: &Path, device_id: Option<&str>) -> Result<Self, String> {
        let paths = [path.to_path_buf()];
        Self::start_input_paths(&paths, device_id)
    }

    /// Captures one selected input stream into multiple synchronized WAV files.
    /// The input callback and bounded queue are shared; a single writer thread
    /// fans each captured frame out to the requested take files.
    pub fn start_input_paths(paths: &[PathBuf], device_id: Option<&str>) -> Result<Self, String> {
        let routes = vec![AudioRecordRoute::Stereo; paths.len()];
        Self::start_input_paths_routed(paths, &routes, device_id)
    }

    #[cfg(windows)]
    pub fn start_input_paths_routed(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        device_id: Option<&str>,
    ) -> Result<Self, String> {
        Self::start_input_paths_routed_mode(
            paths,
            routes,
            device_id,
            crate::AudioDeviceMode::SharedLowLatency,
        )
    }

    #[cfg(windows)]
    pub fn start_input_paths_routed_mode(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        device_id: Option<&str>,
        mode: crate::AudioDeviceMode,
    ) -> Result<Self, String> {
        Self::start_input_paths_routed_mode_with_format(
            paths,
            routes,
            device_id,
            mode,
            None,
            RecordingFormat::Float32,
        )
    }

    #[cfg(windows)]
    pub fn start_input_paths_routed_mode_with_format(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        device_id: Option<&str>,
        mode: crate::AudioDeviceMode,
        target_sample_rate: Option<u32>,
        recording_format: RecordingFormat,
    ) -> Result<Self, String> {
        Self::start_input_paths_routed_mode_with_channel(
            paths,
            routes,
            device_id,
            mode,
            target_sample_rate,
            recording_format,
            &opentrack_core::project::BusChannel::input().strip,
        )
    }

    #[cfg(windows)]
    pub fn start_input_paths_routed_mode_with_channel(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        device_id: Option<&str>,
        mode: crate::AudioDeviceMode,
        target_sample_rate: Option<u32>,
        recording_format: RecordingFormat,
        input_channel: &opentrack_core::project::Track,
    ) -> Result<Self, String> {
        #[cfg(all(windows, feature = "local-asio"))]
        if let Some(id) = device_id.filter(|id| id.starts_with("asio:")) {
            return Self::start_asio(
                paths,
                routes,
                id,
                target_sample_rate,
                recording_format,
                input_channel,
            );
        }
        if paths.is_empty() {
            return Err("at least one output path is required for recording".to_owned());
        }
        if paths.len() != routes.len() {
            return Err("each recording path must have one input route".to_owned());
        }
        let paths = paths.to_vec();
        let path = &paths[0];
        let endpoint_id = match device_id {
            Some(id) => crate::wasapi_backend::endpoint_id_from_cpal(id)?,
            None => crate::wasapi_backend::default_endpoint_id(wasapi::Direction::Capture)?,
        };
        let ring = Arc::new(SpscAudioRing::new());
        let monitor_cell = Arc::new(OnceLock::new());
        let input_stream_failed = Arc::new(AtomicBool::new(false));
        let input_discontinuities = Arc::new(AtomicU64::new(0));
        let input_timestamp_errors = Arc::new(AtomicU64::new(0));
        let capture_start_timestamp_hns = Arc::new(AtomicU64::new(0));
        let capture_enabled = Arc::new(AtomicBool::new(false));
        let callbacks_active = Arc::new(AtomicUsize::new(0));
        let stream_ring = Arc::clone(&ring);
        let stream_monitor = Arc::clone(&monitor_cell);
        let stream_failed = Arc::clone(&input_stream_failed);
        let stream_discontinuities = Arc::clone(&input_discontinuities);
        let stream_timestamp_errors = Arc::clone(&input_timestamp_errors);
        let stream_start_timestamp = Arc::clone(&capture_start_timestamp_hns);
        let stream_enabled = Arc::clone(&capture_enabled);
        let stream_callbacks = Arc::clone(&callbacks_active);
        let stream = crate::wasapi_backend::WasapiInputStream::open(
            endpoint_id,
            mode,
            None,
            None,
            Arc::clone(&input_stream_failed),
            move |samples, channels, packet| {
                if channels == 0 || channels > 2 {
                    stream_failed.store(true, Ordering::Release);
                    return;
                }
                if stream_enabled.load(Ordering::Acquire) && packet.discontinuity {
                    stream_discontinuities.fetch_add(1, Ordering::Relaxed);
                }
                if stream_enabled.load(Ordering::Acquire) {
                    if packet.timestamp_error {
                        stream_timestamp_errors.fetch_add(1, Ordering::Relaxed);
                    }
                    if packet.timestamp_hns > 0 {
                        let _ = stream_start_timestamp.compare_exchange(
                            0,
                            packet.timestamp_hns,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        );
                    }
                }
                push_if_enabled(
                    &stream_ring,
                    stream_monitor.get().map(Arc::as_ref),
                    &stream_enabled,
                    &stream_callbacks,
                    samples,
                    channels,
                    Some(packet.device_position),
                    |sample| {
                        if sample.is_finite() {
                            sample.clamp(-1.0, 1.0)
                        } else {
                            0.0
                        }
                    },
                );
            },
        )?;
        let info = stream.info.clone();
        let channels = info.channels;
        let capture_sample_rate = info.sample_rate;
        let sample_rate = target_sample_rate.unwrap_or(capture_sample_rate);
        if !(1..=2).contains(&channels) || sample_rate == 0 {
            return Err(format!(
                "the selected input format must be mono or stereo with a valid sample rate (reported {channels} channels at {sample_rate} Hz)"
            ));
        }
        if channels == 1 && routes.contains(&AudioRecordRoute::Right) {
            return Err(
                "the selected input exposes no second channel for Right routing".to_owned(),
            );
        }
        let monitor_buffer = Arc::new(AudioMonitorBuffer::new(capture_sample_rate));
        monitor_buffer.set_input_channel(input_channel);
        let _ = monitor_cell.set(Arc::clone(&monitor_buffer));

        let writer_paths = paths.clone();
        let writer_routes = routes.to_vec();
        let worker_ring = Arc::clone(&ring);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_capture_enabled = Arc::clone(&capture_enabled);
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let worker = thread::Builder::new()
            .name("opentrack-wav-writer".to_owned())
            .spawn(move || {
                write_recording_files(
                    &writer_paths,
                    &writer_routes,
                    channels,
                    capture_sample_rate,
                    sample_rate,
                    recording_format,
                    &worker_ring,
                    &worker_stop,
                    &worker_capture_enabled,
                    ready_tx,
                )
            })
            .map_err(|error| format!("could not start recording writer: {error}"))?;

        match ready_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = worker.join();
                return Err(error);
            }
            Err(error) => {
                let _ = worker.join();
                return Err(format!("recording writer did not start: {error}"));
            }
        }
        capture_enabled.store(true, Ordering::Release);

        Ok(Self {
            stream: Some(stream),
            #[cfg(feature = "local-asio")]
            asio_capture: None,
            ring,
            monitor_buffer,
            stop,
            capture_enabled,
            callbacks_active,
            input_stream_failed,
            input_discontinuities,
            input_timestamp_errors,
            capture_start_timestamp_hns,
            worker: Some(worker),
            path: path.clone(),
            paths,
            device_name: info.device_name,
            sample_rate,
            capture_sample_rate,
            channels,
            recording_format,
            input_mode: info.mode,
            input_period_frames: info.period_frames,
            input_buffer_frames: info.buffer_frames,
            input_mode_detail: info.detail,
        })
    }

    /// Captures the selected physical input bus into each synchronized take.
    /// `routes` must have one entry per output path. Stereo takes preserve the
    /// input pair; Left and Right produce isolated mono takes.
    #[cfg(not(windows))]
    pub fn start_input_paths_routed(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        device_id: Option<&str>,
    ) -> Result<Self, String> {
        Self::start_input_paths_routed_mode(
            paths,
            routes,
            device_id,
            crate::AudioDeviceMode::SharedCompatible,
        )
    }

    #[cfg(not(windows))]
    pub fn start_input_paths_routed_mode(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        device_id: Option<&str>,
        mode: crate::AudioDeviceMode,
    ) -> Result<Self, String> {
        Self::start_input_paths_routed_mode_with_format(
            paths,
            routes,
            device_id,
            mode,
            None,
            RecordingFormat::Float32,
        )
    }

    #[cfg(not(windows))]
    pub fn start_input_paths_routed_mode_with_format(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        device_id: Option<&str>,
        _mode: crate::AudioDeviceMode,
        target_sample_rate: Option<u32>,
        recording_format: RecordingFormat,
    ) -> Result<Self, String> {
        Self::start_input_paths_routed_mode_with_channel(
            paths,
            routes,
            device_id,
            _mode,
            target_sample_rate,
            recording_format,
            &opentrack_core::project::BusChannel::input().strip,
        )
    }

    #[cfg(not(windows))]
    pub fn start_input_paths_routed_mode_with_channel(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        device_id: Option<&str>,
        _mode: crate::AudioDeviceMode,
        target_sample_rate: Option<u32>,
        recording_format: RecordingFormat,
        input_channel: &opentrack_core::project::Track,
    ) -> Result<Self, String> {
        if paths.is_empty() {
            return Err("at least one output path is required for recording".to_owned());
        }
        if paths.len() != routes.len() {
            return Err("each recording path must have one input route".to_owned());
        }
        let paths = paths.to_vec();
        let path = &paths[0];
        let device = crate::devices::input_device(device_id)?;
        if !device.supports_input() {
            return Err("the selected device does not support audio input".to_owned());
        }
        let device_name = device
            .description()
            .map(|description| description.name().to_owned())
            .unwrap_or_else(|_| device.to_string());
        let supported_config = device
            .default_input_config()
            .map_err(|error| format!("could not read the input format: {error}"))?;
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();
        let channels = config.channels;
        let capture_sample_rate = config.sample_rate;
        let sample_rate = target_sample_rate.unwrap_or(capture_sample_rate);
        if !(1..=2).contains(&channels) || sample_rate == 0 {
            return Err(format!(
                "the selected input format must be mono or stereo with a valid sample rate (reported {channels} channels at {sample_rate} Hz)"
            ));
        }
        if channels == 1 && routes.contains(&AudioRecordRoute::Right) {
            return Err(
                "the selected input exposes no second channel for Right routing".to_owned(),
            );
        }
        if !matches!(
            sample_format,
            SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16
        ) {
            return Err(format!(
                "the default input sample format {sample_format:?} is not supported yet"
            ));
        }

        let ring = Arc::new(SpscAudioRing::new());
        let monitor_buffer = Arc::new(AudioMonitorBuffer::new(capture_sample_rate));
        monitor_buffer.set_input_channel(input_channel);
        let input_stream_failed = Arc::new(AtomicBool::new(false));
        let capture_enabled = Arc::new(AtomicBool::new(false));
        let callbacks_active = Arc::new(AtomicUsize::new(0));
        let input_discontinuities = Arc::new(AtomicU64::new(0));
        let input_timestamp_errors = Arc::new(AtomicU64::new(0));
        let capture_start_timestamp_hns = Arc::new(AtomicU64::new(0));
        let stream_error_flag = Arc::clone(&input_stream_failed);
        let stream_ring = Arc::clone(&ring);
        let stream_monitor = Arc::clone(&monitor_buffer);
        let stream_capture_enabled = Arc::clone(&capture_enabled);
        let stream_callbacks_active = Arc::clone(&callbacks_active);
        let error_callback = move |_| {
            stream_error_flag.store(true, Ordering::Release);
        };
        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                config,
                move |samples: &[f32], _| {
                    push_if_enabled(
                        &stream_ring,
                        Some(&stream_monitor),
                        &stream_capture_enabled,
                        &stream_callbacks_active,
                        samples,
                        channels,
                        None,
                        |sample| {
                            if sample.is_finite() {
                                sample.clamp(-1.0, 1.0)
                            } else {
                                0.0
                            }
                        },
                    )
                },
                error_callback,
                Some(Duration::from_secs(5)),
            ),
            SampleFormat::I16 => device.build_input_stream(
                config,
                move |samples: &[i16], _| {
                    push_if_enabled(
                        &stream_ring,
                        Some(&stream_monitor),
                        &stream_capture_enabled,
                        &stream_callbacks_active,
                        samples,
                        channels,
                        None,
                        |sample| f32::from(sample) / 32_768.0,
                    )
                },
                error_callback,
                Some(Duration::from_secs(5)),
            ),
            SampleFormat::U16 => device.build_input_stream(
                config,
                move |samples: &[u16], _| {
                    push_if_enabled(
                        &stream_ring,
                        Some(&stream_monitor),
                        &stream_capture_enabled,
                        &stream_callbacks_active,
                        samples,
                        channels,
                        None,
                        |sample| f32::from(sample) / 32_767.5 - 1.0,
                    )
                },
                error_callback,
                Some(Duration::from_secs(5)),
            ),
            _ => unreachable!("sample format validated above"),
        }
        .map_err(|error| format!("could not open the default input stream: {error}"))?;

        let writer_paths = paths.clone();
        let writer_routes = routes.to_vec();
        let worker_ring = Arc::clone(&ring);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_capture_enabled = Arc::clone(&capture_enabled);
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let worker = thread::Builder::new()
            .name("opentrack-wav-writer".to_owned())
            .spawn(move || {
                write_recording_files(
                    &writer_paths,
                    &writer_routes,
                    channels,
                    capture_sample_rate,
                    sample_rate,
                    recording_format,
                    &worker_ring,
                    &worker_stop,
                    &worker_capture_enabled,
                    ready_tx,
                )
            })
            .map_err(|error| format!("could not start recording writer: {error}"))?;

        match ready_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = worker.join();
                return Err(error);
            }
            Err(error) => {
                let _ = worker.join();
                return Err(format!("recording writer did not start: {error}"));
            }
        }

        capture_enabled.store(true, Ordering::Release);
        if let Err(error) = stream.play() {
            capture_enabled.store(false, Ordering::Release);
            stop.store(true, Ordering::Release);
            let _ = worker.join();
            for path in &paths {
                let _ = fs::remove_file(path);
            }
            return Err(format!("could not start the default input stream: {error}"));
        }

        Ok(Self {
            stream: Some(stream),
            ring,
            monitor_buffer,
            stop,
            capture_enabled,
            callbacks_active,
            input_stream_failed,
            input_discontinuities,
            input_timestamp_errors,
            capture_start_timestamp_hns,
            worker: Some(worker),
            path: path.clone(),
            paths,
            device_name,
            sample_rate,
            capture_sample_rate,
            channels,
            recording_format,
            input_mode: crate::AudioDeviceMode::SharedCompatible,
            input_period_frames: 0,
            input_buffer_frames: 0,
            input_mode_detail: "CPAL platform default".to_owned(),
        })
    }

    #[cfg(all(windows, feature = "local-asio"))]
    fn start_asio(
        paths: &[PathBuf],
        routes: &[AudioRecordRoute],
        id: &str,
        target_rate: Option<u32>,
        recording_format: RecordingFormat,
        input_channel: &opentrack_core::project::Track,
    ) -> Result<Self, String> {
        if paths.is_empty() || paths.len() != routes.len() {
            return Err("Each recording path must have one input route".into());
        }
        let capture = crate::asio_backend::capture(id)?;
        if capture.recording.load().is_some() {
            return Err("ASIO is already recording".into());
        }
        if target_rate == Some(0) {
            return Err("Recording sample rate must be positive".into());
        }
        if capture.failed.load(Ordering::Acquire) {
            return Err("The ASIO capture session stopped; reopen the driver".into());
        }
        if capture.channels == 1 && routes.contains(&AudioRecordRoute::Right) {
            return Err("No second input channel is available".into());
        }
        capture.monitor.set_input_channel(input_channel);
        let ring = Arc::new(SpscAudioRing::new());
        let stop = Arc::new(AtomicBool::new(false));
        let capture_enabled = Arc::new(AtomicBool::new(false));
        let callbacks_active = Arc::new(AtomicUsize::new(0));
        let sample_rate = target_rate.unwrap_or(capture.sample_rate);
        let (tx, rx) = mpsc::sync_channel(1);
        let writer_paths = paths.to_vec();
        let writer_routes = routes.to_vec();
        let wr = Arc::clone(&ring);
        let ws = Arc::clone(&stop);
        let we = Arc::clone(&capture_enabled);
        let capture_rate = capture.sample_rate;
        let channels = capture.channels;
        let worker = thread::Builder::new()
            .name("opentrack-wav-writer".into())
            .spawn(move || {
                write_recording_files(
                    &writer_paths,
                    &writer_routes,
                    channels,
                    capture_rate,
                    sample_rate,
                    recording_format,
                    &wr,
                    &ws,
                    &we,
                    tx,
                )
            })
            .map_err(|e| e.to_string())?;
        match rx.recv() {
            Ok(Ok(())) => {}
            other => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                return Err(format!("Could not prepare recording files: {other:?}"));
            }
        }
        capture.recording.store(Some(Arc::new(AsioRecording {
            ring: Arc::clone(&ring),
            enabled: Arc::clone(&capture_enabled),
            callbacks: Arc::clone(&callbacks_active),
        })));
        capture_enabled.store(true, Ordering::Release);
        Ok(Self {
            stream: None,
            asio_capture: Some(Arc::clone(&capture)),
            ring,
            monitor_buffer: Arc::clone(&capture.monitor),
            stop,
            capture_enabled,
            callbacks_active,
            input_stream_failed: Arc::clone(&capture.failed),
            input_discontinuities: Arc::new(AtomicU64::new(0)),
            input_timestamp_errors: Arc::new(AtomicU64::new(0)),
            capture_start_timestamp_hns: Arc::new(AtomicU64::new(0)),
            worker: Some(worker),
            path: paths[0].clone(),
            paths: paths.to_vec(),
            device_name: capture.device_name.clone(),
            sample_rate,
            capture_sample_rate: capture.sample_rate,
            channels,
            recording_format,
            input_mode: crate::AudioDeviceMode::Exclusive,
            input_period_frames: capture.period_frames.load(Ordering::Acquire),
            input_buffer_frames: capture.period_frames.load(Ordering::Acquire),
            input_mode_detail: "ASIO full duplex".into(),
        })
    }

    /// Stops capture, drains queued samples, and finalizes the WAV header.
    pub fn finish(self) -> Result<RecordingSummary, String> {
        self.finish_all()?
            .into_iter()
            .next()
            .ok_or_else(|| "recording produced no output summaries".to_owned())
    }

    /// Stops capture and finalizes each output file in the same frame order.
    pub fn finish_all(mut self) -> Result<Vec<RecordingSummary>, String> {
        self.capture_enabled.store(false, Ordering::Release);
        #[cfg(all(windows, feature = "local-asio"))]
        if let Some(capture) = self.asio_capture.take() {
            let detached = capture.recording.swap(None);
            while capture.callback_reads.load(Ordering::Acquire) != 0 {
                thread::yield_now();
            }
            drop(detached);
        }
        self.stream.take();
        while self.callbacks_active.load(Ordering::Acquire) != 0 {
            thread::yield_now();
        }
        while self.ring.producer_active.load(Ordering::Acquire) {
            thread::yield_now();
        }
        self.stop.store(true, Ordering::Release);
        let worker = self
            .worker
            .take()
            .ok_or_else(|| "recording writer is unavailable".to_owned())?;
        let frames_written = worker
            .join()
            .map_err(|_| "recording writer stopped unexpectedly".to_owned())??;
        if frames_written.len() != self.paths.len() {
            return Err("recording writer returned an unexpected take count".to_owned());
        }
        let dropped_frames = self.ring.dropped_frames();
        let monitor_dropped_frames = self.monitor_buffer.dropped_frames();
        let input_stream_failed = self.input_stream_failed.load(Ordering::Acquire);
        let input_discontinuities = self.input_discontinuities.load(Ordering::Acquire);
        let input_timestamp_errors = self.input_timestamp_errors.load(Ordering::Acquire);
        let capture_start_timestamp_hns = self.capture_start_timestamp_hns.load(Ordering::Acquire);
        Ok(self
            .paths
            .iter()
            .cloned()
            .zip(frames_written)
            .map(|(path, frames_written)| RecordingSummary {
                path,
                frames_written,
                dropped_frames,
                monitor_dropped_frames,
                input_stream_failed,
                input_discontinuities,
                input_timestamp_errors,
                capture_start_timestamp_hns,
            })
            .collect())
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        self.capture_enabled.store(false, Ordering::Release);
        #[cfg(all(windows, feature = "local-asio"))]
        if let Some(capture) = self.asio_capture.take() {
            let detached = capture.recording.swap(None);
            while capture.callback_reads.load(Ordering::Acquire) != 0 {
                thread::yield_now();
            }
            drop(detached);
        }
        self.stream.take();
        while self.callbacks_active.load(Ordering::Acquire) != 0 {
            thread::yield_now();
        }
        while self.ring.producer_active.load(Ordering::Acquire) {
            thread::yield_now();
        }
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
fn write_recording_file(
    path: &Path,
    channels: u16,
    sample_rate: u32,
    ring: &SpscAudioRing,
    stop: &AtomicBool,
    capture_enabled: &AtomicBool,
    ready_tx: mpsc::SyncSender<Result<(), String>>,
) -> Result<u64, String> {
    let routes = [AudioRecordRoute::Stereo];
    write_recording_files(
        &[path.to_path_buf()],
        &routes,
        channels,
        sample_rate,
        sample_rate,
        RecordingFormat::Float32,
        ring,
        stop,
        capture_enabled,
        ready_tx,
    )?
    .into_iter()
    .next()
    .ok_or_else(|| "recording writer returned no frame count".to_owned())
}

fn write_recording_files(
    paths: &[PathBuf],
    routes: &[AudioRecordRoute],
    channels: u16,
    capture_sample_rate: u32,
    sample_rate: u32,
    recording_format: RecordingFormat,
    ring: &SpscAudioRing,
    stop: &AtomicBool,
    capture_enabled: &AtomicBool,
    ready_tx: mpsc::SyncSender<Result<(), String>>,
) -> Result<Vec<u64>, String> {
    if paths.is_empty() || paths.len() != routes.len() {
        let message =
            "recording paths and input routes must have matching nonzero counts".to_owned();
        capture_enabled.store(false, Ordering::Release);
        let _ = ready_tx.send(Err(message.clone()));
        return Err(message);
    }
    if capture_sample_rate == 0 || sample_rate == 0 {
        let message = "recording sample rates must be greater than zero".to_owned();
        capture_enabled.store(false, Ordering::Release);
        let _ = ready_tx.send(Err(message.clone()));
        return Err(message);
    }

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(file) => files.push(file),
            Err(error) => {
                let message = format!(
                    "could not create recording file {}: {error}",
                    path.display()
                );
                capture_enabled.store(false, Ordering::Release);
                let created_count = files.len();
                drop(files);
                for created_path in paths.iter().take(created_count) {
                    let _ = fs::remove_file(created_path);
                }
                let _ = ready_tx.send(Err(message.clone()));
                return Err(message);
            }
        }
    }

    let mut writers = Vec::with_capacity(paths.len());
    let output_channels = routes
        .iter()
        .map(|route| match route {
            AudioRecordRoute::Stereo => channels,
            AudioRecordRoute::Left | AudioRecordRoute::Right => 1,
        })
        .collect::<Vec<_>>();
    for ((file, path), file_channels) in files.into_iter().zip(paths).zip(&output_channels) {
        let spec = hound::WavSpec {
            channels: *file_channels,
            sample_rate,
            bits_per_sample: recording_format.bits_per_sample(),
            sample_format: if recording_format == RecordingFormat::Float32 {
                hound::SampleFormat::Float
            } else {
                hound::SampleFormat::Int
            },
        };
        match hound::WavWriter::new(file, spec) {
            Ok(writer) => writers.push(writer),
            Err(error) => {
                let message = format!(
                    "could not initialize recording WAV {}: {error}",
                    path.display()
                );
                capture_enabled.store(false, Ordering::Release);
                drop(writers);
                for created_path in paths {
                    let _ = fs::remove_file(created_path);
                }
                let _ = ready_tx.send(Err(message.clone()));
                return Err(message);
            }
        }
    }

    if ready_tx.send(Ok(())).is_err() {
        for writer in writers {
            let _ = writer.finalize();
        }
        return Err("recording startup was cancelled".to_owned());
    }

    let mut scratch = vec![CapturedFrame::default(); DRAIN_BUFFER_FRAMES];
    let mut resampler = LinearRecordingResampler::new(capture_sample_rate, sample_rate);
    let mut next_input_frame = 0_u64;
    let mut waveform = crate::waveform::WaveformBuilder::new(capture_sample_rate, channels);
    let mut published = std::time::Instant::now();
    loop {
        let count = ring.pop(&mut scratch);
        if count > 0 {
            for frame in &scratch[..count] {
                if frame.sequence < next_input_frame {
                    capture_enabled.store(false, Ordering::Release);
                    return Err("input frames arrived out of order".to_owned());
                }
                while next_input_frame < frame.sequence {
                    waveform.push(0.0, 0.0);
                    if let Err(error) = resampler.push(
                        next_input_frame,
                        CapturedFrame::default(),
                        &mut writers,
                        routes,
                        &output_channels,
                        recording_format,
                    ) {
                        capture_enabled.store(false, Ordering::Release);
                        return Err(error);
                    }
                    next_input_frame += 1;
                }
                waveform.push(
                    frame.left,
                    if channels == 1 {
                        frame.left
                    } else {
                        frame.right
                    },
                );
                if let Err(error) = resampler.push(
                    frame.sequence,
                    *frame,
                    &mut writers,
                    routes,
                    &output_channels,
                    recording_format,
                ) {
                    capture_enabled.store(false, Ordering::Release);
                    return Err(error);
                }
                next_input_frame = frame.sequence.saturating_add(1);
            }
            if published.elapsed() >= Duration::from_millis(33) {
                ring.waveform.store(Arc::new(waveform.snapshot()));
                published = std::time::Instant::now();
            }
        } else if stop.load(Ordering::Acquire) {
            break;
        } else {
            thread::sleep(Duration::from_millis(2));
        }
    }

    let input_frames = ring.input_frames();
    while next_input_frame < input_frames {
        waveform.push(0.0, 0.0);
        if let Err(error) = resampler.push(
            next_input_frame,
            CapturedFrame::default(),
            &mut writers,
            routes,
            &output_channels,
            recording_format,
        ) {
            capture_enabled.store(false, Ordering::Release);
            return Err(error);
        }
        next_input_frame += 1;
    }
    ring.waveform.store(Arc::new(waveform.snapshot()));
    if let Err(error) = resampler.finish(
        input_frames,
        &mut writers,
        routes,
        &output_channels,
        recording_format,
    ) {
        capture_enabled.store(false, Ordering::Release);
        return Err(error);
    }

    let mut frame_counts = Vec::with_capacity(writers.len());
    for writer in writers {
        if let Err(error) = writer.finalize() {
            capture_enabled.store(false, Ordering::Release);
            return Err(format!("could not finalize recording WAV: {error}"));
        }
        frame_counts.push(resampler.output_frames);
    }
    Ok(frame_counts)
}

struct LinearRecordingResampler {
    source_rate: u32,
    output_rate: u32,
    next_output_frame: u64,
    output_frames: u64,
    previous: Option<(u64, CapturedFrame)>,
}

impl LinearRecordingResampler {
    fn new(source_rate: u32, output_rate: u32) -> Self {
        Self {
            source_rate,
            output_rate,
            next_output_frame: 0,
            output_frames: 0,
            previous: None,
        }
    }

    fn push(
        &mut self,
        source_index: u64,
        current: CapturedFrame,
        writers: &mut [hound::WavWriter<fs::File>],
        routes: &[AudioRecordRoute],
        output_channels: &[u16],
        recording_format: RecordingFormat,
    ) -> Result<(), String> {
        let Some((previous_index, previous)) = self.previous else {
            if source_index == 0 {
                write_project_recording_frame(
                    writers,
                    routes,
                    output_channels,
                    current,
                    recording_format,
                )?;
                self.next_output_frame = 1;
                self.output_frames = 1;
            }
            self.previous = Some((source_index, current));
            return Ok(());
        };
        if source_index != previous_index.saturating_add(1) {
            return Err("recording resampler received a discontinuous frame sequence".to_owned());
        }

        loop {
            let output_position = u128::from(self.next_output_frame) * u128::from(self.source_rate);
            let current_position = u128::from(source_index) * u128::from(self.output_rate);
            if output_position > current_position {
                break;
            }
            let previous_position = u128::from(previous_index) * u128::from(self.output_rate);
            let fraction = if current_position == previous_position {
                1.0
            } else {
                ((output_position - previous_position) as f64
                    / (current_position - previous_position) as f64)
                    .clamp(0.0, 1.0) as f32
            };
            let interpolated = CapturedFrame {
                sequence: self.next_output_frame,
                left: previous.left + (current.left - previous.left) * fraction,
                right: previous.right + (current.right - previous.right) * fraction,
            };
            write_project_recording_frame(
                writers,
                routes,
                output_channels,
                interpolated,
                recording_format,
            )?;
            self.next_output_frame = self.next_output_frame.saturating_add(1);
            self.output_frames = self.output_frames.saturating_add(1);
        }
        self.previous = Some((source_index, current));
        Ok(())
    }

    fn finish(
        &mut self,
        input_frames: u64,
        writers: &mut [hound::WavWriter<fs::File>],
        routes: &[AudioRecordRoute],
        output_channels: &[u16],
        recording_format: RecordingFormat,
    ) -> Result<(), String> {
        if input_frames == 0 {
            return Ok(());
        }
        let target_frames = (u128::from(input_frames) * u128::from(self.output_rate)
            + u128::from(self.source_rate) / 2)
            / u128::from(self.source_rate);
        let target_frames = u64::try_from(target_frames)
            .map_err(|_| "recording is too long to represent its output frame count".to_owned())?;
        let last = self
            .previous
            .map(|(_, frame)| frame)
            .ok_or_else(|| "recording has input frames but no samples".to_owned())?;
        while self.next_output_frame < target_frames {
            let frame = CapturedFrame {
                sequence: self.next_output_frame,
                ..last
            };
            write_project_recording_frame(
                writers,
                routes,
                output_channels,
                frame,
                recording_format,
            )?;
            self.next_output_frame = self.next_output_frame.saturating_add(1);
            self.output_frames = self.output_frames.saturating_add(1);
        }
        Ok(())
    }
}

fn write_project_recording_frame(
    writers: &mut [hound::WavWriter<fs::File>],
    routes: &[AudioRecordRoute],
    output_channels: &[u16],
    frame: CapturedFrame,
    recording_format: RecordingFormat,
) -> Result<(), String> {
    for ((writer, route), channels) in writers.iter_mut().zip(routes).zip(output_channels) {
        let (left, right) = match route {
            AudioRecordRoute::Stereo => (frame.left, frame.right),
            AudioRecordRoute::Left => (frame.left, 0.0),
            AudioRecordRoute::Right => (frame.right, 0.0),
        };
        write_recording_sample(writer, left, recording_format)?;
        if *channels == 2 {
            write_recording_sample(writer, right, recording_format)?;
        }
    }
    Ok(())
}

fn write_recording_sample<W: std::io::Write + std::io::Seek>(
    writer: &mut hound::WavWriter<W>,
    sample: f32,
    recording_format: RecordingFormat,
) -> Result<(), String> {
    let sample = if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let result = match recording_format {
        RecordingFormat::Float32 => writer.write_sample::<f32>(sample),
        RecordingFormat::Pcm16 => writer.write_sample::<i16>(quantize_pcm(sample, 16) as i16),
        RecordingFormat::Pcm24 => writer.write_sample::<i32>(quantize_pcm(sample, 24)),
    };
    result.map_err(|error| format!("could not write recording samples: {error}"))
}

fn quantize_pcm(sample: f32, bits: u16) -> i32 {
    let scale = (1_i64 << (bits - 1)) as f64;
    let minimum = -scale;
    let maximum = scale - 1.0;
    ((f64::from(sample) * scale).round().clamp(minimum, maximum)) as i32
}

struct SpscAudioRing {
    waveform: arc_swap::ArcSwap<crate::Waveform>,
    frames: Box<[std::cell::UnsafeCell<CapturedFrame>]>,
    read_index: AtomicUsize,
    write_index: AtomicUsize,
    input_frames: AtomicU64,
    source_position_origin: AtomicU64,
    producer_active: AtomicBool,
    dropped_frames: AtomicU64,
    input_peak: AtomicU32,
    input_channel_peaks: [AtomicU32; 2],
}

#[derive(Clone, Copy, Debug, Default)]
struct CapturedFrame {
    sequence: u64,
    left: f32,
    right: f32,
}

// SAFETY: one CPAL input callback is the sole producer and one writer thread
// is the sole consumer. Slot access is synchronized by the published indices.
unsafe impl Send for SpscAudioRing {}
// SAFETY: the SPSC protocol prevents simultaneous access to any occupied slot.
unsafe impl Sync for SpscAudioRing {}

impl SpscAudioRing {
    fn new() -> Self {
        let frames = (0..QUEUE_CAPACITY)
            .map(|_| std::cell::UnsafeCell::new(CapturedFrame::default()))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            waveform: arc_swap::ArcSwap::from_pointee(crate::Waveform::default()),
            frames,
            read_index: AtomicUsize::new(0),
            write_index: AtomicUsize::new(0),
            input_frames: AtomicU64::new(0),
            source_position_origin: AtomicU64::new(u64::MAX),
            producer_active: AtomicBool::new(false),
            dropped_frames: AtomicU64::new(0),
            input_peak: AtomicU32::new(0),
            input_channel_peaks: std::array::from_fn(|_| AtomicU32::new(0)),
        }
    }

    fn push<T: Copy>(
        &self,
        input: &[T],
        channels: u16,
        monitor: Option<&AudioMonitorBuffer>,
        source_position: Option<u64>,
        convert: impl Fn(T) -> f32,
    ) {
        let channels = usize::from(channels);
        let input_frame_count = input.len() / channels;
        if self
            .producer_active
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            self.reserve_sequence(input_frame_count, source_position);
            self.dropped_frames
                .fetch_add(input_frame_count as u64, Ordering::Relaxed);
            if let Some(monitor) = monitor.filter(|monitor| monitor.is_enabled()) {
                monitor.add_dropped_frames(input_frame_count as u64);
            }
            return;
        }
        let first_sequence = self.reserve_sequence(input_frame_count, source_position);
        let write = self.write_index.load(Ordering::Relaxed);
        let read = self.read_index.load(Ordering::Acquire);
        let occupied = write.wrapping_sub(read).min(QUEUE_CAPACITY - 1);
        let available = QUEUE_CAPACITY - 1 - occupied;
        let count = input_frame_count.min(available);
        let input_bus = monitor;
        let monitor = monitor.filter(|monitor| monitor.is_enabled());
        let mut monitor_dropped_frames = 0_u64;
        let mut peak = 0.0_f32;
        let mut channel_peaks = [0.0_f32; 2];
        for offset in 0..count {
            let input_index = offset * channels;
            let left = convert(input[input_index]);
            let right = if channels == 2 {
                convert(input[input_index + 1])
            } else {
                0.0
            };
            let (left, right) = input_bus
                .map(|bus| bus.process_input(left, if channels == 1 { left } else { right }))
                .unwrap_or((left, right));
            peak = peak.max(left.abs()).max(right.abs());
            channel_peaks[0] = channel_peaks[0].max(left.abs());
            if channels > 1 {
                channel_peaks[1] = channel_peaks[1].max(right.abs());
            }
            if let Some(monitor) = monitor
                && !monitor.push_frame(left, if channels == 2 { right } else { left })
            {
                monitor_dropped_frames += 1;
            }
            let slot = &self.frames[write.wrapping_add(offset) & QUEUE_MASK];
            // SAFETY: the producer owns each free ring slot until publishing
            // the new write index below; the consumer cannot read it earlier.
            unsafe {
                *slot.get() = CapturedFrame {
                    sequence: first_sequence + offset as u64,
                    left,
                    right,
                }
            };
        }
        for offset in count..input_frame_count {
            let input_index = offset * channels;
            let left = convert(input[input_index]);
            let right = if channels == 2 {
                convert(input[input_index + 1])
            } else {
                left
            };
            let (left, right) = input_bus
                .map(|bus| bus.process_input(left, if channels == 1 { left } else { right }))
                .unwrap_or((left, right));
            peak = peak.max(left.abs()).max(right.abs());
            channel_peaks[0] = channel_peaks[0].max(left.abs());
            if channels > 1 {
                channel_peaks[1] = channel_peaks[1].max(right.abs());
            }
            if let Some(monitor) = monitor
                && !monitor.push_frame(left, right)
            {
                monitor_dropped_frames += 1;
            }
        }
        if peak.is_finite() && peak > 0.0 {
            self.input_peak
                .fetch_max(peak.min(4.0).to_bits(), Ordering::Relaxed);
        }
        for (published, channel_peak) in self.input_channel_peaks.iter().zip(channel_peaks) {
            if channel_peak.is_finite() && channel_peak > 0.0 {
                published.fetch_max(channel_peak.min(4.0).to_bits(), Ordering::Relaxed);
            }
        }
        if let Some(monitor) = monitor {
            monitor.add_dropped_frames(monitor_dropped_frames);
        }
        self.write_index
            .store(write.wrapping_add(count), Ordering::Release);
        if count < input_frame_count {
            self.dropped_frames
                .fetch_add((input_frame_count - count) as u64, Ordering::Relaxed);
        }
        self.producer_active.store(false, Ordering::Release);
    }

    fn reserve_sequence(&self, frame_count: usize, source_position: Option<u64>) -> u64 {
        let frame_count = frame_count as u64;
        let Some(source_position) = source_position else {
            return self.input_frames.fetch_add(frame_count, Ordering::Relaxed);
        };
        let mut origin = self.source_position_origin.load(Ordering::Relaxed);
        if origin == u64::MAX {
            match self.source_position_origin.compare_exchange(
                u64::MAX,
                source_position,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => origin = source_position,
                Err(existing) => origin = existing,
            }
        }
        let current = self.input_frames.load(Ordering::Relaxed);
        let first = source_position.saturating_sub(origin).max(current);
        let end = first.saturating_add(frame_count);
        self.input_frames.fetch_max(end, Ordering::Relaxed);
        first
    }

    fn pop(&self, output: &mut [CapturedFrame]) -> usize {
        let read = self.read_index.load(Ordering::Relaxed);
        let write = self.write_index.load(Ordering::Acquire);
        let count = output.len().min(write.wrapping_sub(read));
        for (offset, sample) in output[..count].iter_mut().enumerate() {
            let slot = &self.frames[read.wrapping_add(offset) & QUEUE_MASK];
            // SAFETY: the acquire load observes the producer's published
            // samples; the producer cannot reuse them until read_index advances.
            *sample = unsafe { *slot.get() };
        }
        self.read_index
            .store(read.wrapping_add(count), Ordering::Release);
        count
    }

    fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Acquire)
    }

    fn input_frames(&self) -> u64 {
        self.input_frames.load(Ordering::Acquire)
    }

    fn take_input_peak(&self) -> f32 {
        f32::from_bits(self.input_peak.swap(0, Ordering::Relaxed))
    }

    fn take_channel_peaks(&self, channels: u16) -> [f32; 2] {
        std::array::from_fn(|index| {
            if index < usize::from(channels).min(2) {
                f32::from_bits(self.input_channel_peaks[index].swap(0, Ordering::AcqRel))
            } else {
                0.0
            }
        })
    }
}

fn push_if_enabled<T: Copy>(
    ring: &SpscAudioRing,
    monitor: Option<&AudioMonitorBuffer>,
    enabled: &AtomicBool,
    callbacks_active: &AtomicUsize,
    samples: &[T],
    channels: u16,
    source_position: Option<u64>,
    convert: impl Fn(T) -> f32,
) {
    callbacks_active.fetch_add(1, Ordering::AcqRel);
    if enabled.load(Ordering::Acquire) {
        ring.push(samples, channels, monitor, source_position, convert);
    }
    callbacks_active.fetch_sub(1, Ordering::Release);
}

#[cfg(test)]
fn write_captured_frame<W: std::io::Write + std::io::Seek>(
    writer: &mut hound::WavWriter<W>,
    frame: CapturedFrame,
    next_frame: &mut u64,
    route: AudioRecordRoute,
    channels: u16,
) -> Result<(), String> {
    if frame.sequence < *next_frame {
        return Err("input frames arrived out of order".to_owned());
    }
    write_silence_until(writer, next_frame, frame.sequence, channels)?;
    let (left, right) = match route {
        AudioRecordRoute::Stereo => (frame.left, frame.right),
        AudioRecordRoute::Left => (frame.left, frame.left),
        AudioRecordRoute::Right => (frame.right, frame.right),
    };
    writer
        .write_sample(left)
        .map_err(|error| format!("could not write recording samples: {error}"))?;
    if channels == 2 {
        writer
            .write_sample(right)
            .map_err(|error| format!("could not write recording samples: {error}"))?;
    }
    *next_frame = frame.sequence.saturating_add(1);
    Ok(())
}

#[cfg(test)]
fn write_silence_until<W: std::io::Write + std::io::Seek>(
    writer: &mut hound::WavWriter<W>,
    next_frame: &mut u64,
    target_frame: u64,
    channels: u16,
) -> Result<(), String> {
    while *next_frame < target_frame {
        writer
            .write_sample(0.0_f32)
            .map_err(|error| format!("could not write dropout silence: {error}"))?;
        if channels == 2 {
            writer
                .write_sample(0.0_f32)
                .map_err(|error| format!("could not write dropout silence: {error}"))?;
        }
        *next_frame = next_frame.saturating_add(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn write_test_take(
        samples: &[f32],
        channels: u16,
        capture_rate: u32,
        project_rate: u32,
        recording_format: RecordingFormat,
    ) -> (PathBuf, Vec<u64>) {
        let path = std::env::temp_dir().join(format!(
            "opentrack-recording-format-{}-{}.wav",
            std::process::id(),
            TEST_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let ring = SpscAudioRing::new();
        ring.push(samples, channels, None, None, |sample| sample);
        let stop = AtomicBool::new(true);
        let capture_enabled = AtomicBool::new(true);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let counts = write_recording_files(
            std::slice::from_ref(&path),
            &[AudioRecordRoute::Stereo],
            channels,
            capture_rate,
            project_rate,
            recording_format,
            &ring,
            &stop,
            &capture_enabled,
            ready_tx,
        )
        .unwrap();
        assert_eq!(ready_rx.recv().unwrap(), Ok(()));
        (path, counts)
    }

    #[cfg(windows)]
    #[test]
    fn writer_publishes_growing_signed_stereo_waveform_before_recording_finishes() {
        let path =
            std::env::temp_dir().join(format!("opentrack-live-wave-{}.wav", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let ring = Arc::new(SpscAudioRing::new());
        let stop = Arc::new(AtomicBool::new(false));
        let enabled = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::sync_channel(1);
        let worker_ring = Arc::clone(&ring);
        let worker_stop = Arc::clone(&stop);
        let worker_enabled = Arc::clone(&enabled);
        let worker_path = path.clone();
        let worker = thread::spawn(move || {
            write_recording_file(
                &worker_path,
                2,
                48000,
                &worker_ring,
                &worker_stop,
                &worker_enabled,
                tx,
            )
        });
        rx.recv().unwrap().unwrap();
        ring.push(&vec![0.25_f32, -0.75].repeat(2400), 2, None, None, |s| s);
        thread::sleep(Duration::from_millis(40));
        ring.push(&[0.8_f32, -0.1].repeat(100), 2, None, None, |s| s);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while ring.waveform.load().frames == 0 && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(2));
        }
        let preview = ring.waveform.load_full();
        assert!(preview.frames >= 2400);
        assert_eq!(preview.channels, 2);
        assert!(!stop.load(Ordering::Acquire));
        assert!(
            preview
                .columns(0.0, preview.duration_seconds(), 50, 0)
                .iter()
                .any(|p| p[1] >= 0.25)
        );
        assert!(
            preview
                .columns(0.0, preview.duration_seconds(), 50, 1)
                .iter()
                .any(|p| p[0] <= -0.75)
        );
        stop.store(true, Ordering::Release);
        assert_eq!(worker.join().unwrap().unwrap(), 2500);
        assert_eq!(ring.waveform.load().frames, 2500);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn input_bus_processing_reaches_recorded_and_monitored_samples_exactly_once() {
        let ring = SpscAudioRing::new();
        let monitor = AudioMonitorBuffer::new(48000);
        let mut bus = opentrack_core::project::BusChannel::input().strip;
        bus.gain = 0.5;
        bus.input_phase_inverted = true;
        monitor.set_input_channel(&bus);
        monitor.set_enabled(true);
        ring.push(&[0.8_f32, -0.4], 2, Some(&monitor), None, |s| s);
        let mut frames = [CapturedFrame::default(); 1];
        assert_eq!(ring.pop(&mut frames), 1);
        assert_eq!((frames[0].left, frames[0].right), (-0.4, 0.2));
        assert_eq!(monitor.sample_frame(0), Some((-0.4, 0.2)));
        monitor.set_enabled(false);
        ring.push(&[0.8_f32, -0.4], 2, Some(&monitor), None, |s| s);
        assert_eq!(ring.pop(&mut frames), 1);
        assert_eq!((frames[0].left, frames[0].right), (-0.4, 0.2));
        assert_eq!(monitor.take_bus_peaks(), [0.4, 0.2]);
    }
    #[cfg(all(windows, feature = "local-asio"))]
    #[test]
    fn asio_bus_processing_is_shared_by_monitor_and_recording_without_doubling() {
        let capture = AsioCapture::new("test".into(), 48000, 2, Arc::new(AtomicBool::new(false)));
        let ring = Arc::new(SpscAudioRing::new());
        capture.recording.store(Some(Arc::new(AsioRecording {
            ring: Arc::clone(&ring),
            enabled: Arc::new(AtomicBool::new(true)),
            callbacks: Arc::new(AtomicUsize::new(0)),
        })));
        let mut bus = opentrack_core::project::BusChannel::input().strip;
        bus.gain = 0.5;
        capture.monitor.set_input_channel(&bus);
        capture.monitor.set_enabled(true);
        capture.push(&mut [0.8, -0.4]);
        let mut frames = [CapturedFrame::default(); 1];
        assert_eq!(ring.pop(&mut frames), 1);
        assert_eq!((frames[0].left, frames[0].right), (0.4, -0.2));
        assert_eq!(capture.monitor.sample_frame(0), Some((0.4, -0.2)));
    }

    #[test]
    fn input_bus_meter_keeps_channels_separate_with_monitoring_disabled() {
        let monitor = AudioMonitorBuffer::new(48000);
        let peaks = [AtomicU32::new(0), AtomicU32::new(0)];
        record_probe_samples(&[0.25, -0.75, -0.5, 0.125], 2, &peaks, Some(&monitor));
        assert_eq!(f32::from_bits(peaks[0].load(Ordering::Relaxed)), 0.5);
        assert_eq!(f32::from_bits(peaks[1].load(Ordering::Relaxed)), 0.75);
        assert_eq!(monitor.sample_frame(0), None);
    }

    #[cfg(all(windows, feature = "local-asio"))]
    #[test]
    fn shared_asio_capture_feeds_one_recording_and_monitor_from_the_same_frames() {
        let capture = AsioCapture::new("Fake".into(), 48000, 2, Arc::new(AtomicBool::new(false)));
        let ring = Arc::new(SpscAudioRing::new());
        let enabled = Arc::new(AtomicBool::new(true));
        capture.recording.store(Some(Arc::new(AsioRecording {
            ring: Arc::clone(&ring),
            enabled: Arc::clone(&enabled),
            callbacks: Arc::new(AtomicUsize::new(0)),
        })));
        capture.monitor.set_enabled(true);
        capture.push(&mut [0.25, -0.75, 0.5, -0.125]);
        let mut frames = [CapturedFrame::default(); 4];
        assert_eq!(ring.pop(&mut frames), 2);
        assert_eq!((frames[0].left, frames[0].right), (0.25, -0.75));
        assert_eq!(capture.monitor.sample_frame(0), Some((0.25, -0.75)));
        assert_eq!(
            f32::from_bits(capture.peaks[0].load(Ordering::Relaxed)),
            0.5
        );
        assert_eq!(
            f32::from_bits(capture.peaks[1].load(Ordering::Relaxed)),
            0.75
        );
        enabled.store(false, Ordering::Release);
        capture.push(&mut [1.0, 1.0]);
        assert_eq!(ring.pop(&mut frames), 0);
        assert_eq!(capture.callback_reads.load(Ordering::Acquire), 0);
    }

    #[test]
    fn queue_preserves_interleaved_samples_across_wraparound() {
        let ring = SpscAudioRing::new();
        let monitor = AudioMonitorBuffer::new(48_000);
        let wrap_start = QUEUE_CAPACITY - 2;
        ring.read_index.store(wrap_start, Ordering::Relaxed);
        ring.write_index.store(wrap_start, Ordering::Relaxed);

        ring.push(
            &[0.25_f32, -0.25, 0.5, -0.5],
            2,
            Some(&monitor),
            None,
            |sample| sample,
        );
        let mut output = [CapturedFrame::default(); 2];
        assert_eq!(ring.pop(&mut output), 2);
        assert_eq!(output[0].sequence, 0);
        assert_eq!((output[0].left, output[0].right), (0.25, -0.25));
        assert_eq!(output[1].sequence, 1);
        assert_eq!((output[1].left, output[1].right), (0.5, -0.5));
        assert_eq!(ring.dropped_frames(), 0);
    }

    #[test]
    fn capture_fans_out_to_the_monitor_ring_when_enabled() {
        let ring = SpscAudioRing::new();
        let monitor = AudioMonitorBuffer::new(48_000);
        monitor.set_enabled(true);

        ring.push(
            &[0.25_f32, -0.25, 0.5, -0.5],
            2,
            Some(&monitor),
            None,
            |sample| sample,
        );

        assert_eq!(monitor.sample_frame(0), Some((0.25, -0.25)));
        assert_eq!(monitor.sample_frame(1), Some((0.5, -0.5)));
        assert_eq!(monitor.dropped_frames(), 0);
    }

    #[test]
    fn queue_overflow_drops_whole_stereo_frames() {
        let ring = SpscAudioRing::new();
        let monitor = AudioMonitorBuffer::new(48_000);
        let mut input = vec![0.125_f32; QUEUE_CAPACITY * 2];
        input[(QUEUE_CAPACITY - 1) * 2] = 0.9;
        ring.push(&input, 2, Some(&monitor), None, |sample| sample);
        assert_eq!(ring.dropped_frames(), 1);
        assert!((ring.take_input_peak() - 0.9).abs() < f32::EPSILON);

        let mut output = vec![CapturedFrame::default(); QUEUE_CAPACITY];
        assert_eq!(ring.pop(&mut output), QUEUE_CAPACITY - 1);
        assert!(
            output[..QUEUE_CAPACITY - 1]
                .iter()
                .all(|frame| frame.left == 0.125 && frame.right == 0.125)
        );

        ring.push(&[0.75_f32, -0.75], 2, Some(&monitor), None, |sample| sample);
        let mut after_gap = [CapturedFrame::default(); 1];
        assert_eq!(ring.pop(&mut after_gap), 1);
        assert_eq!(after_gap[0].sequence, QUEUE_CAPACITY as u64);
    }

    #[test]
    fn writer_inserts_silence_at_missing_input_frame_positions() {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let path = std::env::temp_dir().join(format!(
            "opentrack-recording-gap-{}-{}.wav",
            std::process::id(),
            TEST_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        let mut next_frame = 0;
        write_captured_frame(
            &mut writer,
            CapturedFrame {
                sequence: 0,
                left: 0.25,
                right: 0.0,
            },
            &mut next_frame,
            AudioRecordRoute::Left,
            1,
        )
        .unwrap();
        write_captured_frame(
            &mut writer,
            CapturedFrame {
                sequence: 2,
                left: 0.75,
                right: 0.0,
            },
            &mut next_frame,
            AudioRecordRoute::Left,
            1,
        )
        .unwrap();
        write_silence_until(&mut writer, &mut next_frame, 4, 1).unwrap();
        writer.finalize().unwrap();
        let mut reader = hound::WavReader::open(&path).unwrap();
        let samples: Vec<f32> = reader.samples::<f32>().map(Result::unwrap).collect();
        drop(reader);
        let _ = std::fs::remove_file(path);
        assert_eq!(samples, [0.25, 0.0, 0.75, 0.0]);
    }

    #[test]
    fn writer_encodes_selected_pcm_and_float_wav_formats() {
        let source = [1.0_f32, -1.0, 0.5, -0.5];
        for format in [
            RecordingFormat::Pcm16,
            RecordingFormat::Pcm24,
            RecordingFormat::Float32,
        ] {
            let (path, frame_counts) = write_test_take(&source, 2, 48_000, 48_000, format);
            assert_eq!(frame_counts, [2]);
            let mut reader = hound::WavReader::open(&path).unwrap();
            let spec = reader.spec();
            assert_eq!(spec.channels, 2);
            assert_eq!(spec.sample_rate, 48_000);
            assert_eq!(spec.bits_per_sample, format.bits_per_sample());
            assert_eq!(
                spec.sample_format,
                if format == RecordingFormat::Float32 {
                    hound::SampleFormat::Float
                } else {
                    hound::SampleFormat::Int
                }
            );
            match format {
                RecordingFormat::Pcm16 => {
                    let values: Vec<i16> = reader.samples().map(Result::unwrap).collect();
                    assert_eq!(
                        values,
                        source
                            .map(|sample| quantize_pcm(sample, 16) as i16)
                            .to_vec()
                    );
                }
                RecordingFormat::Pcm24 => {
                    let values: Vec<i32> = reader.samples().map(Result::unwrap).collect();
                    assert_eq!(
                        values,
                        source.map(|sample| quantize_pcm(sample, 24)).to_vec()
                    );
                }
                RecordingFormat::Float32 => {
                    let values: Vec<f32> = reader.samples().map(Result::unwrap).collect();
                    assert_eq!(values, source);
                }
            }
            drop(reader);
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn writer_resamples_to_project_rate_with_an_exact_rounded_frame_count() {
        let source = (0..441).flat_map(|_| [0.25_f32, -0.25]).collect::<Vec<_>>();
        let (path, frame_counts) =
            write_test_take(&source, 2, 44_100, 48_000, RecordingFormat::Pcm24);
        assert_eq!(frame_counts, [480]);
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().sample_rate, 48_000);
        assert_eq!(reader.spec().bits_per_sample, 24);
        assert_eq!(reader.duration(), 480);
        drop(reader);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writer_worker_finalizes_a_readable_float_wav_from_queued_audio() {
        let path = std::env::temp_dir().join(format!(
            "opentrack-recording-pipeline-{}-{}.wav",
            std::process::id(),
            TEST_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let ring = Arc::new(SpscAudioRing::new());
        let monitor = AudioMonitorBuffer::new(48_000);
        let stop = Arc::new(AtomicBool::new(false));
        let capture_enabled = Arc::new(AtomicBool::new(true));
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker_path = path.clone();
        let worker_ring = Arc::clone(&ring);
        let worker_stop = Arc::clone(&stop);
        let worker_capture_enabled = Arc::clone(&capture_enabled);
        let worker = thread::spawn(move || {
            write_recording_file(
                &worker_path,
                2,
                48_000,
                &worker_ring,
                &worker_stop,
                &worker_capture_enabled,
                ready_tx,
            )
        });

        assert_eq!(ready_rx.recv().unwrap(), Ok(()));
        ring.push(
            &[0.25_f32, -0.25, 0.5, -0.5],
            2,
            Some(&monitor),
            None,
            |sample| sample,
        );
        stop.store(true, Ordering::Release);
        assert_eq!(worker.join().unwrap().unwrap(), 2);

        let mut reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.bits_per_sample, 32);
        assert_eq!(spec.sample_format, hound::SampleFormat::Float);
        assert_eq!(reader.duration(), 2);
        let samples: Vec<f32> = reader.samples::<f32>().map(Result::unwrap).collect();
        drop(reader);
        let decoded = Arc::new(crate::DecodedWav::open(&path).unwrap());
        let _ = std::fs::remove_file(path);
        assert_eq!(samples, [0.25, -0.25, 0.5, -0.5]);

        let plan = crate::PlaybackPlan::new(
            120.0,
            vec![crate::PlaybackTrackSettings {
                clips: vec![crate::PlaybackClip {
                    source_fades: Vec::new(),
                    source: crate::PlaybackSource::Decoded(decoded),
                    start_beats: 0.0,
                    source_offset_seconds: 0.0,
                    duration_seconds: f64::MAX,
                    fade_in_seconds: 0.0,
                    fade_out_seconds: 0.0,
                    gain: 1.0,
                    muted: false,
                    solo: false,
                }],
                midi_notes: Vec::new(),
                midi_controllers: Vec::new(),
                instrument_factory: crate::oscillator_instrument_factory(
                    opentrack_core::project::InstrumentWaveform::Sine,
                ),
                meter_peak: Arc::new(crate::StereoMeter::default()),
                gain: 1.0,
                pan: 0.0,
                track_delay_ms: 0.0,
                muted: false,
                solo: false,
                solo_defeat: false,
                equalizer: opentrack_core::project::ChannelEqualizer::default(),
                inserts: Vec::new(),
                inserts_bypassed: false,
                monitor_input: None,
                output_route: crate::PlaybackOutputRoute::Stereo,
            }],
            1,
            48_000,
            None,
        );
        let (left, right) = crate::mix_frame(&plan, 0, 48_000, (0.0, 0.0), true);
        assert_eq!((left, right), (0.25, -0.25));
    }
}
