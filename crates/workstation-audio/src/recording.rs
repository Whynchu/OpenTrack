//! Selected-input capture with a bounded single-producer/single-consumer queue.
//!
//! The CPAL callback only normalizes samples and copies into preallocated
//! storage. A worker thread drains the queue and writes a floating-point WAV.

use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use cpal::{
    DeviceId, SampleFormat, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

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
}

/// Lock-free input-to-output bridge used for live monitoring during capture.
pub struct AudioMonitorBuffer {
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

/// Active default-device input stream plus the background WAV writer.
pub struct AudioRecorder {
    stream: Option<cpal::Stream>,
    ring: Arc<SpscAudioRing>,
    monitor_buffer: Arc<AudioMonitorBuffer>,
    stop: Arc<AtomicBool>,
    capture_enabled: Arc<AtomicBool>,
    callbacks_active: Arc<AtomicUsize>,
    input_stream_failed: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Vec<u64>, String>>>,
    pub path: PathBuf,
    paths: Vec<PathBuf>,
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioRecorder {
    /// Reads the device's default input format without starting capture.
    pub fn input_format(device_id: Option<&str>) -> Result<AudioInputFormat, String> {
        let host = cpal::default_host();
        let device = if let Some(device_id) = device_id {
            let device_id = DeviceId::from_str(device_id)
                .map_err(|error| format!("invalid audio input device identifier: {error}"))?;
            host.device_by_id(&device_id).ok_or_else(|| {
                "the selected audio input device is no longer available".to_owned()
            })?
        } else {
            host.default_input_device()
                .ok_or_else(|| "no default audio input device is available".to_owned())?
        };
        let config = device
            .default_input_config()
            .map_err(|error| format!("could not read the input format: {error}"))?;
        let format = AudioInputFormat {
            sample_rate: config.sample_rate(),
            channels: config.channels(),
        };
        if format.sample_rate == 0 || !(1..=2).contains(&format.channels) {
            return Err(format!(
                "the default input format is not supported ({channels} channels at {sample_rate} Hz)",
                channels = format.channels,
                sample_rate = format.sample_rate
            ));
        }
        Ok(format)
    }

    /// Lists input-capable devices without opening audio streams.
    pub fn available_input_devices() -> Result<Vec<AudioInputDevice>, String> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|error| format!("could not list audio input devices: {error}"))?;
        devices
            .map(|device| {
                let id = device
                    .id()
                    .map_err(|error| format!("could not identify an input device: {error}"))?;
                let name = device
                    .description()
                    .map_err(|error| format!("could not read an input device name: {error}"))?
                    .name()
                    .to_owned();
                Ok(AudioInputDevice {
                    id: id.to_string(),
                    name,
                })
            })
            .collect()
    }

    /// Returns and clears the highest normalized input sample since the last read.
    pub fn take_input_peak(&self) -> f32 {
        self.ring.take_input_peak()
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
        if paths.is_empty() {
            return Err("at least one output path is required for recording".to_owned());
        }
        let paths = paths.to_vec();
        let path = &paths[0];
        let host = cpal::default_host();
        let device = if let Some(device_id) = device_id {
            let device_id = DeviceId::from_str(device_id)
                .map_err(|error| format!("invalid audio input device identifier: {error}"))?;
            host.device_by_id(&device_id).ok_or_else(|| {
                "the selected audio input device is no longer available".to_owned()
            })?
        } else {
            host.default_input_device()
                .ok_or_else(|| "no default audio input device is available".to_owned())?
        };
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
        let sample_rate = config.sample_rate;
        if !(1..=2).contains(&channels) || sample_rate == 0 {
            return Err(format!(
                "the selected input format must be mono or stereo with a valid sample rate (reported {channels} channels at {sample_rate} Hz)"
            ));
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
        let monitor_buffer = Arc::new(AudioMonitorBuffer::new(sample_rate));
        let input_stream_failed = Arc::new(AtomicBool::new(false));
        let capture_enabled = Arc::new(AtomicBool::new(true));
        let callbacks_active = Arc::new(AtomicUsize::new(0));
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
                        &stream_monitor,
                        &stream_capture_enabled,
                        &stream_callbacks_active,
                        samples,
                        channels,
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
                        &stream_monitor,
                        &stream_capture_enabled,
                        &stream_callbacks_active,
                        samples,
                        channels,
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
                        &stream_monitor,
                        &stream_capture_enabled,
                        &stream_callbacks_active,
                        samples,
                        channels,
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
                    channels,
                    sample_rate,
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
            worker: Some(worker),
            path: path.clone(),
            paths,
            device_name,
            sample_rate,
            channels,
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
            })
            .collect())
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        self.capture_enabled.store(false, Ordering::Release);
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
    write_recording_files(
        &[path.to_path_buf()],
        channels,
        sample_rate,
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
    channels: u16,
    sample_rate: u32,
    ring: &SpscAudioRing,
    stop: &AtomicBool,
    capture_enabled: &AtomicBool,
    ready_tx: mpsc::SyncSender<Result<(), String>>,
) -> Result<Vec<u64>, String> {
    if paths.is_empty() {
        let message = "at least one output path is required for recording".to_owned();
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

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writers = Vec::with_capacity(paths.len());
    for (file, path) in files.into_iter().zip(paths) {
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
    let mut next_frames = vec![0_u64; writers.len()];
    loop {
        let count = ring.pop(&mut scratch);
        if count > 0 {
            for frame in &scratch[..count] {
                for (writer, next_frame) in writers.iter_mut().zip(&mut next_frames) {
                    if let Err(error) = write_captured_frame(writer, *frame, next_frame, channels) {
                        capture_enabled.store(false, Ordering::Release);
                        return Err(error);
                    }
                }
            }
        } else if stop.load(Ordering::Acquire) {
            break;
        } else {
            thread::sleep(Duration::from_millis(2));
        }
    }

    let input_frames = ring.input_frames();
    for (writer, next_frame) in writers.iter_mut().zip(&mut next_frames) {
        if let Err(error) = write_silence_until(writer, next_frame, input_frames, channels) {
            capture_enabled.store(false, Ordering::Release);
            return Err(error);
        }
    }

    let mut frame_counts = Vec::with_capacity(writers.len());
    for (writer, frame_count) in writers.into_iter().zip(next_frames) {
        if let Err(error) = writer.finalize() {
            capture_enabled.store(false, Ordering::Release);
            return Err(format!("could not finalize recording WAV: {error}"));
        }
        frame_counts.push(frame_count);
    }
    Ok(frame_counts)
}

struct SpscAudioRing {
    frames: Box<[std::cell::UnsafeCell<CapturedFrame>]>,
    read_index: AtomicUsize,
    write_index: AtomicUsize,
    input_frames: AtomicU64,
    producer_active: AtomicBool,
    dropped_frames: AtomicU64,
    input_peak: AtomicU32,
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
            frames,
            read_index: AtomicUsize::new(0),
            write_index: AtomicUsize::new(0),
            input_frames: AtomicU64::new(0),
            producer_active: AtomicBool::new(false),
            dropped_frames: AtomicU64::new(0),
            input_peak: AtomicU32::new(0),
        }
    }

    fn push<T: Copy>(
        &self,
        input: &[T],
        channels: u16,
        monitor: &AudioMonitorBuffer,
        convert: impl Fn(T) -> f32,
    ) {
        let channels = usize::from(channels);
        let input_frame_count = input.len() / channels;
        if self
            .producer_active
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            self.input_frames
                .fetch_add(input_frame_count as u64, Ordering::Relaxed);
            self.dropped_frames
                .fetch_add(input_frame_count as u64, Ordering::Relaxed);
            if monitor.is_enabled() {
                monitor.add_dropped_frames(input_frame_count as u64);
            }
            return;
        }
        let first_sequence = self
            .input_frames
            .fetch_add(input_frame_count as u64, Ordering::Relaxed);
        let write = self.write_index.load(Ordering::Relaxed);
        let read = self.read_index.load(Ordering::Acquire);
        let occupied = write.wrapping_sub(read).min(QUEUE_CAPACITY - 1);
        let available = QUEUE_CAPACITY - 1 - occupied;
        let count = input_frame_count.min(available);
        let monitor_enabled = monitor.is_enabled();
        let mut monitor_dropped_frames = 0_u64;
        let mut peak = 0.0_f32;
        for offset in 0..count {
            let input_index = offset * channels;
            let left = convert(input[input_index]);
            let right = if channels == 2 {
                convert(input[input_index + 1])
            } else {
                0.0
            };
            peak = peak.max(left.abs()).max(right.abs());
            if monitor_enabled
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
            peak = peak.max(left.abs()).max(right.abs());
            if monitor_enabled && !monitor.push_frame(left, right) {
                monitor_dropped_frames += 1;
            }
        }
        if peak.is_finite() && peak > 0.0 {
            self.input_peak
                .fetch_max(peak.min(4.0).to_bits(), Ordering::Relaxed);
        }
        monitor.add_dropped_frames(monitor_dropped_frames);
        self.write_index
            .store(write.wrapping_add(count), Ordering::Release);
        if count < input_frame_count {
            self.dropped_frames
                .fetch_add((input_frame_count - count) as u64, Ordering::Relaxed);
        }
        self.producer_active.store(false, Ordering::Release);
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
}

fn push_if_enabled<T: Copy>(
    ring: &SpscAudioRing,
    monitor: &AudioMonitorBuffer,
    enabled: &AtomicBool,
    callbacks_active: &AtomicUsize,
    samples: &[T],
    channels: u16,
    convert: impl Fn(T) -> f32,
) {
    callbacks_active.fetch_add(1, Ordering::AcqRel);
    if enabled.load(Ordering::Acquire) {
        ring.push(samples, channels, monitor, convert);
    }
    callbacks_active.fetch_sub(1, Ordering::Release);
}

fn write_captured_frame<W: std::io::Write + std::io::Seek>(
    writer: &mut hound::WavWriter<W>,
    frame: CapturedFrame,
    next_frame: &mut u64,
    channels: u16,
) -> Result<(), String> {
    if frame.sequence < *next_frame {
        return Err("input frames arrived out of order".to_owned());
    }
    write_silence_until(writer, next_frame, frame.sequence, channels)?;
    writer
        .write_sample(frame.left)
        .map_err(|error| format!("could not write recording samples: {error}"))?;
    if channels == 2 {
        writer
            .write_sample(frame.right)
            .map_err(|error| format!("could not write recording samples: {error}"))?;
    }
    *next_frame = frame.sequence.saturating_add(1);
    Ok(())
}

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

    #[test]
    fn queue_preserves_interleaved_samples_across_wraparound() {
        let ring = SpscAudioRing::new();
        let monitor = AudioMonitorBuffer::new(48_000);
        let wrap_start = QUEUE_CAPACITY - 2;
        ring.read_index.store(wrap_start, Ordering::Relaxed);
        ring.write_index.store(wrap_start, Ordering::Relaxed);

        ring.push(&[0.25_f32, -0.25, 0.5, -0.5], 2, &monitor, |sample| sample);
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

        ring.push(&[0.25_f32, -0.25, 0.5, -0.5], 2, &monitor, |sample| sample);

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
        ring.push(&input, 2, &monitor, |sample| sample);
        assert_eq!(ring.dropped_frames(), 1);
        assert!((ring.take_input_peak() - 0.9).abs() < f32::EPSILON);

        let mut output = vec![CapturedFrame::default(); QUEUE_CAPACITY];
        assert_eq!(ring.pop(&mut output), QUEUE_CAPACITY - 1);
        assert!(
            output[..QUEUE_CAPACITY - 1]
                .iter()
                .all(|frame| frame.left == 0.125 && frame.right == 0.125)
        );

        ring.push(&[0.75_f32, -0.75], 2, &monitor, |sample| sample);
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
        ring.push(&[0.25_f32, -0.25, 0.5, -0.5], 2, &monitor, |sample| sample);
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
                    source: crate::PlaybackSource::Decoded(decoded),
                    start_beats: 0.0,
                    gain: 1.0,
                    muted: false,
                    solo: false,
                }],
                midi_notes: Vec::new(),
                midi_controllers: Vec::new(),
                instrument_factory: crate::oscillator_instrument_factory(
                    workstation_core::project::InstrumentWaveform::Sine,
                ),
                meter_peak: Arc::new(AtomicU32::new(0)),
                gain: 1.0,
                pan: 0.0,
                track_delay_ms: 0.0,
                muted: false,
                solo: false,
                solo_defeat: false,
                equalizer: workstation_core::project::ChannelEqualizer::default(),
                inserts: Vec::new(),
                inserts_bypassed: false,
            }],
            1,
            48_000,
            None,
        );
        let (left, right) = crate::mix_frame(&plan, 0, 48_000);
        assert_eq!((left, right), (0.25, -0.25));
    }
}
