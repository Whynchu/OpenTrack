//! Bounded, background WAV streaming for sources too large to decode in memory.

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use arc_swap::ArcSwap;

use super::{DecodedWav, MAX_DECODED_WAV_BYTES};

const STREAM_BUFFER_FRAMES: usize = 1 << 15;
const STREAM_PREFETCH_FRAMES: u64 = (STREAM_BUFFER_FRAMES / 2) as u64;
const STREAM_READ_BLOCK_FRAMES: usize = 2_048;
const WAVEFORM_BUCKETS: usize = 256;
const WAVEFORM_CANCEL_CHECK_FRAMES: u32 = 4_096;

/// An audio asset that is either decoded in memory or read through a bounded stream cache.
#[derive(Clone)]
pub enum AudioSource {
    Decoded(Arc<DecodedWav>),
    Streamed(Arc<StreamedWav>),
}

impl AudioSource {
    /// Opens a supported WAV, decoding smaller sources and streaming larger ones.
    pub fn open(path: &Path) -> Result<Self, String> {
        let reader = hound::WavReader::open(path)
            .map_err(|error| format!("could not open WAV file: {error}"))?;
        let spec = reader.spec();
        validate_spec(spec)?;
        let frame_count = reader.duration();
        if frame_count == 0 {
            return Err("WAV file contains no audio frames".to_owned());
        }
        let decoded_bytes = u64::from(frame_count)
            .checked_mul(u64::from(spec.channels))
            .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>() as u64))
            .ok_or_else(|| "WAV decoded size is too large".to_owned())?;
        drop(reader);

        if !should_stream(decoded_bytes) {
            Ok(Self::Decoded(Arc::new(DecodedWav::open(path)?)))
        } else {
            Ok(Self::Streamed(Arc::new(StreamedWav::open(path)?)))
        }
    }

    pub fn duration_seconds(&self) -> f64 {
        match self {
            Self::Decoded(source) => source.duration_seconds(),
            Self::Streamed(source) => source.duration_seconds(),
        }
    }

    /// Returns the current waveform snapshot. Large-file peaks arrive asynchronously.
    pub fn waveform_peaks(&self) -> Arc<Vec<f32>> {
        match self {
            Self::Decoded(source) => Arc::clone(&source.waveform_peaks),
            Self::Streamed(source) => source.waveform_peaks(),
        }
    }

    pub fn waveform_ready(&self) -> bool {
        match self {
            Self::Decoded(_) => true,
            Self::Streamed(source) => source.waveform_ready(),
        }
    }

    pub fn waveform_failed(&self) -> bool {
        match self {
            Self::Decoded(_) => false,
            Self::Streamed(source) => source.waveform_failed(),
        }
    }

    pub fn is_streamed(&self) -> bool {
        matches!(self, Self::Streamed(_))
    }
}

impl fmt::Debug for AudioSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decoded(source) => formatter.debug_tuple("Decoded").field(source).finish(),
            Self::Streamed(source) => formatter.debug_tuple("Streamed").field(source).finish(),
        }
    }
}

/// Metadata and asynchronously generated waveform for a large WAV file.
pub struct StreamedWav {
    path: PathBuf,
    spec: hound::WavSpec,
    channels: u16,
    sample_rate: u32,
    frame_count: u32,
    waveform: Arc<ArcSwap<Vec<f32>>>,
    waveform_ready: Arc<AtomicBool>,
    waveform_failed: Arc<AtomicBool>,
    waveform_stop: Arc<AtomicBool>,
    waveform_worker: Option<JoinHandle<()>>,
}

impl StreamedWav {
    /// Reads WAV metadata and starts a bounded-memory waveform scan in the background.
    pub fn open(path: &Path) -> Result<Self, String> {
        let reader = hound::WavReader::open(path)
            .map_err(|error| format!("could not open WAV file: {error}"))?;
        let spec = reader.spec();
        validate_spec(spec)?;
        let frame_count = reader.duration();
        if frame_count == 0 {
            return Err("WAV file contains no audio frames".to_owned());
        }
        drop(reader);

        let waveform = Arc::new(ArcSwap::from_pointee(Vec::new()));
        let waveform_ready = Arc::new(AtomicBool::new(false));
        let waveform_failed = Arc::new(AtomicBool::new(false));
        let waveform_stop = Arc::new(AtomicBool::new(false));
        let worker_waveform = Arc::clone(&waveform);
        let worker_ready = Arc::clone(&waveform_ready);
        let worker_failed = Arc::clone(&waveform_failed);
        let worker_stop = Arc::clone(&waveform_stop);
        let worker_path = path.to_path_buf();
        let waveform_worker = thread::Builder::new()
            .name("opentrack-waveform-scan".to_owned())
            .spawn(move || {
                match scan_waveform(&worker_path, frame_count, spec, &worker_stop) {
                    Ok(Some(peaks)) => worker_waveform.store(Arc::new(peaks)),
                    Ok(None) => {}
                    Err(()) => worker_failed.store(true, Ordering::Release),
                }
                worker_ready.store(true, Ordering::Release);
            })
            .map_err(|error| format!("could not start WAV waveform scan: {error}"))?;

        Ok(Self {
            path: path.to_path_buf(),
            spec,
            channels: spec.channels,
            sample_rate: spec.sample_rate,
            frame_count,
            waveform,
            waveform_ready,
            waveform_failed,
            waveform_stop,
            waveform_worker: Some(waveform_worker),
        })
    }

    pub fn duration_seconds(&self) -> f64 {
        f64::from(self.frame_count) / f64::from(self.sample_rate)
    }

    pub fn waveform_peaks(&self) -> Arc<Vec<f32>> {
        self.waveform.load_full()
    }

    pub fn waveform_ready(&self) -> bool {
        self.waveform_ready.load(Ordering::Acquire)
    }

    pub fn waveform_failed(&self) -> bool {
        self.waveform_failed.load(Ordering::Acquire)
    }

    pub fn open_playback(&self) -> Result<Arc<StreamingPlayback>, String> {
        StreamingPlayback::open(self.path.clone(), self.spec, self.frame_count)
    }
}

impl fmt::Debug for StreamedWav {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StreamedWav")
            .field("path", &self.path)
            .field("spec", &self.spec)
            .field("channels", &self.channels)
            .field("sample_rate", &self.sample_rate)
            .field("frame_count", &self.frame_count)
            .field("waveform_ready", &self.waveform_ready())
            .finish()
    }
}

impl Drop for StreamedWav {
    fn drop(&mut self) {
        self.waveform_stop.store(true, Ordering::Release);
        if let Some(worker) = self.waveform_worker.take() {
            let _ = worker.join();
        }
    }
}

/// Immutable source access used by one arranged clip in the callback.
#[derive(Clone)]
pub enum PlaybackSource {
    Decoded(Arc<DecodedWav>),
    Streamed {
        source: Arc<StreamedWav>,
        reader: Arc<StreamingPlayback>,
    },
}

impl PlaybackSource {
    pub fn from_asset(
        source: &AudioSource,
        streamed_reader: Option<Arc<StreamingPlayback>>,
    ) -> Result<Self, String> {
        match source {
            AudioSource::Decoded(source) => Ok(Self::Decoded(Arc::clone(source))),
            AudioSource::Streamed(source) => {
                let reader = match streamed_reader {
                    Some(reader) => reader,
                    None => source.open_playback()?,
                };
                Ok(Self::Streamed {
                    source: Arc::clone(source),
                    reader,
                })
            }
        }
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        match self {
            Self::Decoded(source) => source.sample_rate,
            Self::Streamed { source, .. } => source.sample_rate,
        }
    }

    pub(crate) fn frame_count(&self) -> u64 {
        match self {
            Self::Decoded(source) => (source.samples.len() / usize::from(source.channels)) as u64,
            Self::Streamed { source, .. } => u64::from(source.frame_count),
        }
    }

    pub(crate) fn sample_frame(&self, frame: u64) -> Option<(f32, f32)> {
        match self {
            Self::Decoded(source) => {
                let channels = usize::from(source.channels);
                let frame = usize::try_from(frame).ok()?;
                let index = frame.checked_mul(channels)?;
                if index + channels > source.samples.len() {
                    return None;
                }
                let left = source.samples[index];
                let right = if channels == 1 {
                    left
                } else {
                    source.samples[index + 1]
                };
                Some((left, right))
            }
            Self::Streamed { source, reader } => reader.sample_frame(frame, source.channels),
        }
    }
}

impl fmt::Debug for PlaybackSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decoded(source) => formatter.debug_tuple("Decoded").field(source).finish(),
            Self::Streamed { source, reader } => formatter
                .debug_struct("Streamed")
                .field("source", source)
                .field("reader", reader)
                .finish(),
        }
    }
}

/// A per-clip background reader with a fixed-size sample ring.
pub struct StreamingPlayback {
    buffer: Arc<StreamBuffer>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl StreamingPlayback {
    fn open(path: PathBuf, spec: hound::WavSpec, frame_count: u32) -> Result<Arc<Self>, String> {
        let buffer = Arc::new(StreamBuffer::new());
        let stop = Arc::new(AtomicBool::new(false));
        let worker_buffer = Arc::clone(&buffer);
        let worker_stop = Arc::clone(&stop);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("opentrack-wav-stream".to_owned())
            .spawn(move || {
                stream_wav(
                    &path,
                    spec,
                    frame_count,
                    &worker_buffer,
                    &worker_stop,
                    ready_tx,
                );
            })
            .map_err(|error| format!("could not start WAV stream reader: {error}"))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Arc::new(Self {
                buffer,
                stop,
                worker: Some(worker),
            })),
            Ok(Err(error)) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(format!("WAV stream reader did not start: {error}"))
            }
        }
    }

    fn sample_frame(&self, frame: u64, channels: u16) -> Option<(f32, f32)> {
        let (left, right) = self.buffer.sample_frame(frame)?;
        if channels == 1 {
            Some((left, left))
        } else {
            Some((left, right))
        }
    }

    pub fn underrun_count(&self) -> u64 {
        self.buffer.underruns.load(Ordering::Relaxed)
    }

    pub fn has_read_error(&self) -> bool {
        self.buffer.failed.load(Ordering::Acquire)
    }
}

impl fmt::Debug for StreamingPlayback {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StreamingPlayback")
            .field("underruns", &self.underrun_count())
            .field("read_error", &self.has_read_error())
            .finish()
    }
}

impl Drop for StreamingPlayback {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct StreamBuffer {
    slots: Box<[StreamSlot]>,
    requested_frame: AtomicU64,
    underruns: AtomicU64,
    failed: AtomicBool,
}

impl StreamBuffer {
    fn new() -> Self {
        let slots = (0..STREAM_BUFFER_FRAMES)
            .map(|_| StreamSlot::default())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            slots,
            requested_frame: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            failed: AtomicBool::new(false),
        }
    }

    fn publish(&self, frame: u64, left: f32, right: f32) {
        let slot = &self.slots[frame as usize & (STREAM_BUFFER_FRAMES - 1)];
        slot.sequence.store(0, Ordering::Relaxed);
        let samples = (u64::from(left.to_bits()) << 32) | u64::from(right.to_bits());
        slot.samples.store(samples, Ordering::Relaxed);
        slot.sequence.store(frame + 1, Ordering::Release);
    }

    fn sample_frame(&self, frame: u64) -> Option<(f32, f32)> {
        self.requested_frame.store(frame, Ordering::Relaxed);
        let slot = &self.slots[frame as usize & (STREAM_BUFFER_FRAMES - 1)];
        let expected = frame + 1;
        if slot.sequence.load(Ordering::Acquire) != expected {
            self.underruns.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let samples = slot.samples.load(Ordering::Relaxed);
        if slot.sequence.load(Ordering::Acquire) != expected {
            self.underruns.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let left = f32::from_bits((samples >> 32) as u32);
        let right = f32::from_bits(samples as u32);
        Some((left, right))
    }
}

#[derive(Default)]
struct StreamSlot {
    sequence: AtomicU64,
    samples: AtomicU64,
}

fn stream_wav(
    path: &Path,
    expected_spec: hound::WavSpec,
    frame_count: u32,
    buffer: &StreamBuffer,
    stop: &AtomicBool,
    ready_tx: mpsc::SyncSender<Result<(), String>>,
) {
    let mut reader = match hound::WavReader::open(path) {
        Ok(reader) => reader,
        Err(error) => {
            buffer.failed.store(true, Ordering::Release);
            let _ = ready_tx.send(Err(format!("could not open WAV stream: {error}")));
            return;
        }
    };
    let spec = reader.spec();
    if spec != expected_spec || reader.duration() != frame_count {
        buffer.failed.store(true, Ordering::Release);
        let _ = ready_tx.send(Err("WAV source changed after it was opened".to_owned()));
        return;
    }
    let mut ready_tx = Some(ready_tx);
    let mut next_frame = 0_u64;

    while !stop.load(Ordering::Acquire) {
        let request = buffer
            .requested_frame
            .load(Ordering::Relaxed)
            .min(u64::from(frame_count.saturating_sub(1)));
        let oldest_cached = next_frame.saturating_sub(STREAM_BUFFER_FRAMES as u64);
        let request_is_cached = request >= oldest_cached && request < next_frame;
        if !request_is_cached && request != next_frame {
            if reader.seek(request as u32).is_err() {
                buffer.failed.store(true, Ordering::Release);
                break;
            }
            next_frame = request;
        }

        let prefetch_end = request
            .saturating_add(STREAM_PREFETCH_FRAMES)
            .min(u64::from(frame_count));
        if next_frame >= prefetch_end {
            thread::sleep(Duration::from_millis(2));
            continue;
        }
        let count = (prefetch_end - next_frame).min(STREAM_READ_BLOCK_FRAMES as u64) as usize;
        let result = read_stream_chunk(&mut reader, next_frame, count, buffer);
        if let Err(error) = result {
            buffer.failed.store(true, Ordering::Release);
            if let Some(ready_tx) = ready_tx.take() {
                let _ = ready_tx.send(Err(error));
            }
            break;
        }
        next_frame += count as u64;
        if next_frame >= 4_096 || next_frame >= u64::from(frame_count) {
            if let Some(ready_tx) = ready_tx.take() {
                let _ = ready_tx.send(Ok(()));
            }
        }
    }
}

fn read_stream_chunk(
    reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
    start_frame: u64,
    frame_count: usize,
    buffer: &StreamBuffer,
) -> Result<(), String> {
    let spec = reader.spec();
    let channels = usize::from(spec.channels);
    match spec.sample_format {
        hound::SampleFormat::Int => {
            let scale = 2.0_f64.powi(i32::from(spec.bits_per_sample) - 1);
            let mut samples = reader.samples::<i32>();
            for offset in 0..frame_count {
                let left = samples
                    .next()
                    .transpose()
                    .map_err(|error| format!("could not decode WAV stream: {error}"))?
                    .ok_or_else(|| "WAV stream ended before its header".to_owned())?;
                let right = if channels == 2 {
                    samples
                        .next()
                        .transpose()
                        .map_err(|error| format!("could not decode WAV stream: {error}"))?
                        .ok_or_else(|| "WAV stream ended before its header".to_owned())?
                } else {
                    left
                };
                buffer.publish(
                    start_frame + offset as u64,
                    (f64::from(left) / scale).clamp(-1.0, 1.0) as f32,
                    (f64::from(right) / scale).clamp(-1.0, 1.0) as f32,
                );
            }
        }
        hound::SampleFormat::Float => {
            let mut samples = reader.samples::<f32>();
            for offset in 0..frame_count {
                let left = samples
                    .next()
                    .transpose()
                    .map_err(|error| format!("could not decode WAV stream: {error}"))?
                    .ok_or_else(|| "WAV stream ended before its header".to_owned())?;
                let right = if channels == 2 {
                    samples
                        .next()
                        .transpose()
                        .map_err(|error| format!("could not decode WAV stream: {error}"))?
                        .ok_or_else(|| "WAV stream ended before its header".to_owned())?
                } else {
                    left
                };
                buffer.publish(
                    start_frame + offset as u64,
                    sanitize_float(left),
                    sanitize_float(right),
                );
            }
        }
    }
    Ok(())
}

fn scan_waveform(
    path: &Path,
    frame_count: u32,
    expected_spec: hound::WavSpec,
    stop: &AtomicBool,
) -> Result<Option<Vec<f32>>, ()> {
    let mut reader = hound::WavReader::open(path).map_err(|_| ())?;
    let spec = reader.spec();
    if spec != expected_spec || reader.duration() != frame_count {
        return Err(());
    }
    let bucket_count = (frame_count as usize).min(WAVEFORM_BUCKETS);
    let mut peaks = vec![0.0_f32; bucket_count];
    let channel_count = usize::from(expected_spec.channels);

    match spec.sample_format {
        hound::SampleFormat::Int => {
            let scale = 2.0_f64.powi(i32::from(spec.bits_per_sample) - 1);
            let mut samples = reader.samples::<i32>();
            for frame in 0..frame_count {
                if frame % WAVEFORM_CANCEL_CHECK_FRAMES == 0 && stop.load(Ordering::Acquire) {
                    return Ok(None);
                }
                let bucket =
                    (u64::from(frame) * bucket_count as u64 / u64::from(frame_count)) as usize;
                for _ in 0..channel_count {
                    let sample = samples.next().transpose().map_err(|_| ())?.ok_or(())?;
                    peaks[bucket] = peaks[bucket]
                        .max((f64::from(sample) / scale).clamp(-1.0, 1.0).abs() as f32);
                }
            }
        }
        hound::SampleFormat::Float => {
            let mut samples = reader.samples::<f32>();
            for frame in 0..frame_count {
                if frame % WAVEFORM_CANCEL_CHECK_FRAMES == 0 && stop.load(Ordering::Acquire) {
                    return Ok(None);
                }
                let bucket =
                    (u64::from(frame) * bucket_count as u64 / u64::from(frame_count)) as usize;
                for _ in 0..channel_count {
                    let sample = samples.next().transpose().map_err(|_| ())?.ok_or(())?;
                    peaks[bucket] = peaks[bucket].max(sanitize_float(sample).abs());
                }
            }
        }
    }
    Ok(Some(peaks))
}

fn sanitize_float(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn validate_spec(spec: hound::WavSpec) -> Result<(), String> {
    if !(1..=2).contains(&spec.channels) || spec.sample_rate == 0 {
        return Err("WAV must be mono or stereo and have a valid sample rate".to_owned());
    }
    Ok(())
}

fn should_stream(decoded_bytes: u64) -> bool {
    decoded_bytes > MAX_DECODED_WAV_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    static TEST_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_wav_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "opentrack-{label}-{}-{}.wav",
            std::process::id(),
            TEST_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn write_ramp_wav(path: &Path, frame_count: u32) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for frame in 0..frame_count {
            let sample = frame as f32 / (frame_count - 1) as f32;
            writer.write_sample(sample).unwrap();
            writer.write_sample(-sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn write_large_header_wav(path: &Path) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        writer.write_sample(0.0_f32).unwrap();
        writer.write_sample(0.0_f32).unwrap();
        writer.finalize().unwrap();

        let mut contents = std::fs::read(path).unwrap();
        let data_header = contents
            .windows(4)
            .position(|window| window == b"data")
            .unwrap();
        let declared_data_bytes = (MAX_DECODED_WAV_BYTES + 8) as u32;
        contents[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        contents[data_header + 4..data_header + 8]
            .copy_from_slice(&declared_data_bytes.to_le_bytes());
        std::fs::write(path, contents).unwrap();
    }

    fn wait_for_frame(reader: &StreamingPlayback, frame: u64) -> (f32, f32) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(samples) = reader.sample_frame(frame, 2) {
                return samples;
            }
            assert!(
                Instant::now() < deadline,
                "stream did not cache frame {frame}"
            );
            thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn large_source_streams_with_bounded_cache_and_supports_seeks() {
        const FRAME_COUNT: u32 = 50_000;
        let path = temp_wav_path("streaming-test");
        write_ramp_wav(&path, FRAME_COUNT);

        let source = Arc::new(StreamedWav::open(&path).unwrap());
        let reader = source.open_playback().unwrap();
        assert_eq!(reader.buffer.slots.len(), STREAM_BUFFER_FRAMES);
        for frame in [0_u32, 1_000, 45_000, 10] {
            let (left, right) = wait_for_frame(&reader, u64::from(frame));
            let expected = frame as f32 / (FRAME_COUNT - 1) as f32;
            assert!((left - expected).abs() < f32::EPSILON);
            assert!((right + expected).abs() < f32::EPSILON);
        }

        let plan = super::super::PlaybackPlan::new(
            120.0,
            vec![super::super::PlaybackTrackSettings {
                clips: vec![super::super::PlaybackClip {
                    source: PlaybackSource::Streamed {
                        source: Arc::clone(&source),
                        reader: Arc::clone(&reader),
                    },
                    start_beats: 0.0,
                    gain: 1.0,
                    muted: false,
                    solo: false,
                }],
                midi_notes: Vec::new(),
                midi_controllers: Vec::new(),
                instrument_factory: super::super::oscillator_instrument_factory(
                    workstation_core::project::InstrumentWaveform::Sine,
                ),
                meter_peak: Arc::new(std::sync::atomic::AtomicU32::new(0)),
                gain: 1.0,
                muted: false,
                solo: false,
                solo_defeat: false,
                equalizer: super::super::ChannelEqualizer::default(),
                inserts: Vec::new(),
            }],
            1,
            48_000,
            None,
        );
        let (left, right) = super::super::mix_frame(&plan, 1_000, 48_000);
        let expected = 1_000.0_f32 / (FRAME_COUNT - 1) as f32;
        assert!((left - expected).abs() < 1.0e-5);
        assert!((right + expected).abs() < 1.0e-5);

        let deadline = Instant::now() + Duration::from_secs(2);
        while !source.waveform_ready() {
            assert!(Instant::now() < deadline, "waveform scan did not finish");
            thread::sleep(Duration::from_millis(2));
        }
        assert!(!source.waveform_failed());
        let peaks = source.waveform_peaks();
        assert_eq!(peaks.len(), WAVEFORM_BUCKETS);
        assert!(peaks[0] < 0.02);
        assert!(peaks[WAVEFORM_BUCKETS - 1] > 0.99);

        drop(reader);
        drop(source);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn streaming_threshold_only_selects_large_decoded_sources() {
        assert!(!should_stream(MAX_DECODED_WAV_BYTES));
        assert!(should_stream(MAX_DECODED_WAV_BYTES + 1));

        let path = temp_wav_path("large-header");
        write_large_header_wav(&path);
        let source = AudioSource::open(&path).unwrap();
        assert!(source.is_streamed());
        drop(source);
        std::fs::remove_file(path).unwrap();
    }
}
