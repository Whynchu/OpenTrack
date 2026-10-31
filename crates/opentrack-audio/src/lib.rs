//! Audio file decoding and default-device output.
//!
//! WAV files are decoded before reaching the audio callback. Playback plans
//! are immutable snapshots; the callback only reads them and writes samples.

#[cfg(all(windows, feature = "local-asio"))]
mod asio_backend;
mod audition;
mod bus;
mod devices;
mod live_pitch;
mod metronome;
mod midi;
mod pitch_analysis;
mod pitch_render;
mod pitch_synthesis;
pub use audition::NoteAudition;
pub use metronome::MetronomeSettings;
pub use pitch_analysis::{PitchAnalysis, PitchPoint, analyze_pitch_file, segment_notes};
pub use pitch_render::{
    pitch_source_identity, render_pitch_time_file, render_pitch_time_file_at_tempo,
};
pub mod plugins;
mod recording;
mod spectrum;
mod streaming;
pub use spectrum::{
    SpectrumFrame, SpectrumResolution, SpectrumStage, TONAL_FFT_SIZE, TONAL_PROFILE_BANDS,
    TonalProfile, analyze_tonal_reference_wav, normalized_log_band_difference_db,
};
mod waveform;
pub use waveform::{WAVEFORM_BLOCK_FRAMES, Waveform};
#[cfg(windows)]
mod wasapi_backend;

pub use devices::{
    AudioDevice, AudioDeviceInventory, AudioOutputConfig, available_audio_devices,
    output_configurations, scan_audio_devices,
};
pub use midi::{MidiInputDevice, MidiInputEvent, MidiInputMessage, MidiInputStream};
pub use recording::{
    AudioInputDevice, AudioInputFormat, AudioInputProbe, AudioMonitorBuffer, AudioRecordRoute,
    AudioRecorder, RecordingSummary,
};
pub use streaming::{
    AudioFileMetadata, AudioSource, PlaybackSource, StreamedWav, StreamingPlayback,
    audio_file_metadata,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioDeviceMode {
    SharedLowLatency,
    SharedCompatible,
    Exclusive,
}

impl AudioDeviceMode {
    pub fn from_preference(value: u8) -> Self {
        match value {
            1 => Self::SharedCompatible,
            2 => Self::Exclusive,
            _ => Self::SharedLowLatency,
        }
    }

    pub fn preference_value(self) -> u8 {
        match self {
            Self::SharedLowLatency => 0,
            Self::SharedCompatible => 1,
            Self::Exclusive => 2,
        }
    }
}

use std::time::{Duration, Instant};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering},
    },
};

use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use opentrack_core::project::{
    ChannelEqualizer, CycleMode, EqualizerBand, FilterType, GeneratorWaveform, InsertEffect,
    InsertEffectKind, InstrumentWaveform,
};
use std::collections::HashMap;

use crate::plugins::test_generator::{TestGenerator, TestGeneratorParameters, TestWaveform};

#[cfg(not(windows))]
use cpal::traits::DeviceTrait;
#[cfg(not(windows))]
use cpal::traits::StreamTrait;
#[cfg(not(windows))]
use cpal::{BufferSize, SampleFormat};

const TEST_TONE_HZ: f32 = 440.0;
const TEST_TONE_LEVEL: f32 = 0.08;
const MAX_DECODED_WAV_BYTES: u64 = 512 * 1024 * 1024;
const PLAYBACK_PARAMETER_FADE_MS: u32 = 10;
static ACTIVE_OUTPUT_PERIOD_FRAMES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(usize::MAX);

/// Maximum internal hosted block used to preallocate callback scratch.
const HOSTED_BLOCK_CAPACITY: usize = 256;

/// Internal block size for the T058 0c attribution comparison. The environment
/// is read once off the callback; unsupported values retain the 64-frame
/// production default.
fn hosted_block_frames() -> usize {
    static BLOCK_FRAMES: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *BLOCK_FRAMES.get_or_init(|| match std::env::var("OPENTRACK_HOSTED_BLOCK_FRAMES") {
        Ok(value) if value == "32" => 32,
        Ok(value) if value == "128" => 128,
        Ok(value) if value == "256" => 256,
        _ => 64,
    })
}

fn effective_hosted_block_frames(configured: usize, callback_frames: usize) -> usize {
    configured.min(callback_frames.max(1))
}

/// Upper bound used when a plug-in reports latency for delay compensation.
///
/// Reported latency is untrusted input. Clamping it keeps every PDC ring
/// buffer allocation bounded; the clamped value is still what playback
/// compensates and what [`PlaybackPlan::hosted_latency_samples`] reports.
const HOSTED_MAX_LATENCY_SECONDS: u32 = 10;

/// A fully decoded mono or stereo WAV file held in memory.
#[derive(Clone, Debug)]
pub struct DecodedWav {
    samples: Arc<[f32]>,
    waveform_peaks: Arc<Vec<f32>>,
    waveform: Arc<Waveform>,
    channels: u16,
    sample_rate: u32,
}

impl DecodedWav {
    /// Decodes an integer PCM or IEEE float WAV on the calling thread.
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut reader = hound::WavReader::open(path)
            .map_err(|error| format!("could not open WAV file: {error}"))?;
        let spec = reader.spec();
        if !(1..=2).contains(&spec.channels) || spec.sample_rate == 0 {
            return Err("WAV must be mono or stereo and have a valid sample rate".to_owned());
        }

        let frame_count = u64::from(reader.duration());
        let sample_count = frame_count
            .checked_mul(u64::from(spec.channels))
            .ok_or_else(|| "WAV sample count is too large".to_owned())?;
        let decoded_size = sample_count
            .checked_mul(std::mem::size_of::<f32>() as u64)
            .ok_or_else(|| "WAV decoded size is too large".to_owned())?;
        if decoded_size == 0 {
            return Err("WAV file contains no audio frames".to_owned());
        }
        if decoded_size > MAX_DECODED_WAV_BYTES {
            return Err("WAV decodes to more than the 512 MiB import limit".to_owned());
        }

        let mut samples = Vec::with_capacity(sample_count as usize);
        match spec.sample_format {
            hound::SampleFormat::Int => {
                let scale = 2.0_f64.powi(i32::from(spec.bits_per_sample) - 1);
                for sample in reader.samples::<i32>() {
                    let sample =
                        sample.map_err(|error| format!("could not decode WAV samples: {error}"))?;
                    samples.push((f64::from(sample) / scale).clamp(-1.0, 1.0) as f32);
                }
            }
            hound::SampleFormat::Float => {
                for sample in reader.samples::<f32>() {
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
        if samples.len() != sample_count as usize {
            return Err("WAV sample data is shorter than its header reports".to_owned());
        }

        let waveform_peaks = Arc::new(make_waveform_peaks(&samples, spec.channels));
        let mut detail = waveform::WaveformBuilder::new(spec.sample_rate, spec.channels);
        for frame in samples.chunks_exact(spec.channels as usize) {
            detail.push(frame[0], frame.get(1).copied().unwrap_or(frame[0]));
        }
        let waveform = Arc::new(detail.snapshot());
        Ok(Self {
            samples: samples.into(),
            waveform_peaks: waveform_peaks.into(),
            waveform,
            channels: spec.channels,
            sample_rate: spec.sample_rate,
        })
    }

    pub fn duration_seconds(&self) -> f64 {
        (self.samples.len() / usize::from(self.channels)) as f64 / f64::from(self.sample_rate)
    }

    pub fn waveform_peaks(&self) -> &[f32] {
        self.waveform_peaks.as_slice()
    }
}

fn make_waveform_peaks(samples: &[f32], channels: u16) -> Vec<f32> {
    const MAX_BUCKETS: usize = 256;
    let channel_count = usize::from(channels);
    let frame_count = samples.len() / channel_count;
    let bucket_count = frame_count.min(MAX_BUCKETS);
    (0..bucket_count)
        .map(|bucket| {
            let start = bucket * frame_count / bucket_count;
            let end = ((bucket + 1) * frame_count / bucket_count).max(start + 1);
            samples[start * channel_count..end * channel_count]
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
        })
        .collect()
}

/// One decoded source placed in the project timeline.
#[derive(Clone, Debug)]
pub struct PlaybackClip {
    pub source_fades: Vec<opentrack_core::project::SourceFade>,
    pub source: PlaybackSource,
    pub start_beats: f64,
    pub source_offset_seconds: f64,
    pub duration_seconds: f64,
    pub fade_in_seconds: f64,
    pub fade_out_seconds: f64,
    pub gain: f32,
    pub muted: bool,
    pub solo: bool,
}

/// One scheduled note rendered by a Rust instrument processor.
#[derive(Clone, Copy, Debug)]
pub struct PlaybackMidiNote {
    pub id: u64,
    pub start_beats: f64,
    pub duration_beats: f64,
    pub frequency_hz: f32,
    pub channel: u8,
    pub velocity: f32,
}

/// One scheduled MIDI control-change event on the project beat timeline.
#[derive(Clone, Copy, Debug)]
pub struct PlaybackMidiController {
    pub position_beats: f64,
    pub channel: u8,
    pub controller: u8,
    pub value: u8,
}

/// Lock-free live values for MIDI data that can change without altering its
/// scheduled position. The UI is the producer; instrument processors read these
/// atomics on the audio thread.
pub struct MidiEventControls {
    note_velocity_indices: HashMap<u64, usize>,
    note_velocities: Box<[AtomicU32]>,
}

impl MidiEventControls {
    fn new(notes: &[PlaybackMidiNote]) -> Self {
        let note_velocity_indices = notes
            .iter()
            .enumerate()
            .map(|(index, note)| (note.id, index))
            .collect();
        let note_velocities = notes
            .iter()
            .map(|note| AtomicU32::new(note.velocity.to_bits()))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            note_velocity_indices,
            note_velocities,
        }
    }

    fn update_note_velocity(&self, note_id: u64, velocity: f32) -> bool {
        let Some(index) = self.note_velocity_indices.get(&note_id).copied() else {
            return false;
        };
        let Some(value) = self.note_velocities.get(index) else {
            return false;
        };
        value.store(velocity.to_bits(), Ordering::Release);
        true
    }

    fn note_velocity(&self, index: usize) -> f32 {
        self.note_velocities
            .get(index)
            .map_or(0.0, |value| f32::from_bits(value.load(Ordering::Acquire)))
    }
}

/// Audio-thread processor created from a track's scheduled MIDI notes.
/// Implementations must not allocate, block, or take contended locks in `next_frame`.
pub trait InstrumentProcessor: Send {
    fn next_frame(&mut self, output_frame: u64) -> (f32, f32);
}

/// Creates an instrument processor while the UI builds a new playback plan.
/// The factory may copy or schedule notes; creation never runs in the callback.
pub trait InstrumentFactory: Send + Sync {
    fn create_processor(
        &self,
        notes: &[PlaybackMidiNote],
        controllers: &[PlaybackMidiController],
        controls: Arc<MidiEventControls>,
        tempo_bpm: f64,
        sample_rate: u32,
    ) -> Box<dyn InstrumentProcessor>;
}

/// Creates the built-in oscillator instrument for a MIDI or Instrument track.
pub fn oscillator_instrument_factory(waveform: InstrumentWaveform) -> Arc<dyn InstrumentFactory> {
    Arc::new(OscillatorInstrumentFactory { waveform })
}

struct OscillatorInstrumentFactory {
    waveform: InstrumentWaveform,
}

impl InstrumentFactory for OscillatorInstrumentFactory {
    fn create_processor(
        &self,
        notes: &[PlaybackMidiNote],
        controllers: &[PlaybackMidiController],
        controls: Arc<MidiEventControls>,
        tempo_bpm: f64,
        sample_rate: u32,
    ) -> Box<dyn InstrumentProcessor> {
        Box::new(MidiVoice::new(
            notes,
            controllers,
            controls,
            tempo_bpm,
            sample_rate,
            self.waveform,
        ))
    }
}

/// Audio material and channel-strip state for one project track.
#[derive(Clone, Debug)]
pub struct PlaybackTrack {
    pub clips: Vec<PlaybackClip>,
    pub gain: f32,
    pub muted: bool,
    pub solo: bool,
    pub equalizer: ChannelEqualizer,
    pub inserts: Vec<InsertEffect>,
}

struct PlaybackTrackRuntime {
    settings: PlaybackTrackSettings,
    midi_controls: Arc<MidiEventControls>,
    processor: std::cell::UnsafeCell<ChannelProcessor>,
    mix: MixControl,
    equalizer_control: Arc<EqualizerControl>,
    meter_accumulator: std::cell::UnsafeCell<[f32; 2]>,
    hosted_latency_samples: u32,
    hosted_delay_samples: u32,
    hosted_buffers: std::cell::UnsafeCell<HostedTrackBuffers>,
    hosted_timing: Option<StageTimingTrace>,
}

struct MixControl {
    target_gain: AtomicU32,
    target_pan: AtomicU32,
    current: std::cell::UnsafeCell<(f32, f32)>,
    smoothing: f32,
}

impl MixControl {
    fn new(gain: f32, pan: f32, sample_rate: u32) -> Self {
        Self {
            target_gain: AtomicU32::new(gain.to_bits()),
            target_pan: AtomicU32::new(pan.to_bits()),
            current: std::cell::UnsafeCell::new((gain, pan)),
            smoothing: 1.0 - (-1.0 / (sample_rate.max(1) as f32 * 0.005)).exp(),
        }
    }

    fn set(&self, gain: f32, pan: f32) {
        self.target_gain.store(gain.to_bits(), Ordering::Relaxed);
        self.target_pan.store(pan.to_bits(), Ordering::Relaxed);
    }

    fn next(&self) -> (f32, f32) {
        // The audio callback is the only writer; UI updates only the atomic targets.
        let current = unsafe { &mut *self.current.get() };
        let gain = f32::from_bits(self.target_gain.load(Ordering::Relaxed));
        let pan = f32::from_bits(self.target_pan.load(Ordering::Relaxed));
        current.0 += (gain - current.0) * self.smoothing;
        current.1 += (pan - current.1) * self.smoothing;
        *current
    }
}

struct EqualizerControl {
    target: ArcSwap<ChannelEqualizer>,
    revision: AtomicU64,
    solo_band: AtomicU32,
    retired_targets: Mutex<Vec<Arc<ChannelEqualizer>>>,
}

impl EqualizerControl {
    fn new(target: ChannelEqualizer) -> Arc<Self> {
        Arc::new(Self {
            target: ArcSwap::from_pointee(target),
            revision: AtomicU64::new(0),
            solo_band: AtomicU32::new(0),
            retired_targets: Mutex::new(Vec::new()),
        })
    }

    fn set_solo_band(&self, band: Option<usize>) {
        self.solo_band.store(
            band.map_or(0, |index| index.saturating_add(1)) as u32,
            Ordering::Release,
        );
    }

    fn set(&self, target: &ChannelEqualizer) {
        let current = self.target.load();
        // Tonal targets are project/analyzer state, not audio parameters. A
        // save or rename in the tonal view must not reconfigure the channel
        // processor or advance its real-time parameter revision.
        let processing_changed = current.bypassed != target.bypassed
            || current.bands != target.bands
            || current.additional_bands != target.additional_bands
            || current.dynamic_bands != target.dynamic_bands
            || current.extended_bands != target.extended_bands
            || current.extended_dynamic_bands != target.extended_dynamic_bands;
        if processing_changed {
            drop(current);
            let previous = self.target.swap(Arc::new(target.clone()));
            self.retired_targets
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(previous);
            self.revision.fetch_add(1, Ordering::Release);
        }
    }

    fn reclaim_retired(&self) {
        self.retired_targets
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}

/// Preallocated planar scratch owned by one track for the hosted block path.
///
/// `delay_*` is the PDC ring. Its state belongs to this plan snapshot: a plan
/// rebuild creates a fresh ring and restarts the compensation delay. That is
/// deliberate because carrying delay state between snapshots would require
/// mutation or allocation on the audio thread when the plan changes.
struct HostedTrackBuffers {
    block_left: Vec<f32>,
    block_right: Vec<f32>,
    dry_left: Vec<f32>,
    dry_right: Vec<f32>,
    pre_left: Vec<f32>,
    pre_right: Vec<f32>,
    delay_left: Vec<f32>,
    delay_right: Vec<f32>,
    delay_cursor: usize,
    group_left: Vec<f32>,
    group_right: Vec<f32>,
}

impl HostedTrackBuffers {
    fn new(block_processing: bool, delay_frames: usize) -> Self {
        let block_frames = effective_hosted_block_frames(
            hosted_block_frames(),
            ACTIVE_OUTPUT_PERIOD_FRAMES.load(Ordering::Relaxed),
        );
        let block = |enabled: bool| {
            if enabled {
                vec![0.0; block_frames]
            } else {
                Vec::new()
            }
        };
        Self {
            block_left: block(block_processing),
            block_right: block(block_processing),
            dry_left: block(block_processing),
            dry_right: block(block_processing),
            pre_left: block(block_processing),
            pre_right: block(block_processing),
            delay_left: vec![0.0; delay_frames],
            delay_right: vec![0.0; delay_frames],
            delay_cursor: 0,
            group_left: vec![0.0; block_frames],
            group_right: vec![0.0; block_frames],
        }
    }

    /// Returns the compensated sample for this frame and stores the input.
    /// A zero-length ring is the no-added-delay fast path.
    fn delay_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        if self.delay_left.is_empty() {
            return (left, right);
        }
        let cursor = self.delay_cursor;
        let delayed = (self.delay_left[cursor], self.delay_right[cursor]);
        self.delay_left[cursor] = left;
        self.delay_right[cursor] = right;
        self.delay_cursor += 1;
        if self.delay_cursor == self.delay_left.len() {
            self.delay_cursor = 0;
        }
        delayed
    }
}

fn hosted_track_latency(inserts: &[HostedInsert], latency_cap: u32) -> u32 {
    inserts
        .iter()
        .filter(|insert| insert.enabled)
        .fold(0_u32, |total, insert| {
            total.saturating_add(insert.processor.latency_samples().min(latency_cap))
        })
        .min(latency_cap)
}

impl PlaybackTrackRuntime {
    fn new(
        mut settings: PlaybackTrackSettings,
        tempo_bpm: f64,
        sample_rate: u32,
        hosted_latency_samples: u32,
        hosted_max_latency_samples: u32,
        hosted_processing: bool,
    ) -> Self {
        // Offset the playback schedule, leaving arrange positions untouched.
        let offset_beats = f64::from(settings.track_delay_ms) * tempo_bpm / 60_000.0;
        for clip in &mut settings.clips {
            clip.start_beats += offset_beats;
        }
        settings.midi_notes.retain_mut(|note| {
            let end = note.start_beats + note.duration_beats + offset_beats;
            note.start_beats = (note.start_beats + offset_beats).max(0.0);
            note.duration_beats = end - note.start_beats;
            note.duration_beats > 0.0
        });
        for event in &mut settings.midi_controllers {
            event.position_beats = (event.position_beats + offset_beats).max(0.0);
        }
        let hosted_delay_samples =
            hosted_max_latency_samples.saturating_sub(hosted_latency_samples);
        let hosted_delay_frames = usize::try_from(hosted_delay_samples).unwrap_or(0);
        let midi_controls = Arc::new(MidiEventControls::new(&settings.midi_notes));
        let equalizer_control = EqualizerControl::new(settings.equalizer.clone());
        let processor = ChannelProcessor::new(
            &settings.equalizer,
            Arc::clone(&equalizer_control),
            &settings.inserts,
            settings.inserts_bypassed,
            &settings.midi_notes,
            &settings.midi_controllers,
            Arc::clone(&midi_controls),
            settings.instrument_factory.as_ref(),
            tempo_bpm,
            sample_rate,
        );
        let hosted_buffers = HostedTrackBuffers::new(hosted_processing, hosted_delay_frames);
        Self {
            mix: MixControl::new(settings.gain, settings.pan, sample_rate),
            settings,
            midi_controls,
            equalizer_control,
            processor: std::cell::UnsafeCell::new(processor),
            meter_accumulator: std::cell::UnsafeCell::new([0.0; 2]),
            hosted_latency_samples,
            hosted_delay_samples,
            hosted_buffers: std::cell::UnsafeCell::new(hosted_buffers),
            hosted_timing: hosted_processing.then(StageTimingTrace::new).flatten(),
        }
    }
}

/// Independent callback peak accumulators. The UI reads both once per update.
#[derive(Default)]
pub struct StereoMeter {
    peaks: [AtomicU32; 2],
    dynamic_eq_gain_db: [AtomicU32; opentrack_core::project::MAX_EQ_BANDS],
    spectrum: std::sync::OnceLock<spectrum::SpectrumTap>,
}

impl StereoMeter {
    /// Control-thread activation; allocation never occurs in record_spectrum().
    pub fn enable_spectrum(&self, enabled: bool) {
        if enabled {
            self.spectrum
                .get_or_init(spectrum::SpectrumTap::new)
                .enable(true);
        } else if let Some(tap) = self.spectrum.get() {
            tap.enable(false);
        }
    }
    pub fn spectrum_enabled(&self) -> bool {
        self.spectrum.get().is_some_and(|tap| tap.enabled())
    }
    /// Latest per-band dynamic gain movement, published by the audio callback.
    pub fn dynamic_eq_gains_db(&self) -> [f32; opentrack_core::project::MAX_EQ_BANDS] {
        self.dynamic_eq_gain_db
            .each_ref()
            .map(|gain| f32::from_bits(gain.load(Ordering::Relaxed)))
    }
    fn publish_dynamic_eq_gain(&self, band: usize, gain_db: f32) {
        if let Some(gain) = self.dynamic_eq_gain_db.get(band) {
            let gain_db = if gain_db.is_finite() {
                gain_db.clamp(-18.0, 18.0)
            } else {
                0.0
            };
            gain.store(gain_db.to_bits(), Ordering::Relaxed);
        }
    }
    pub fn spectrum_frame(&self, stage: SpectrumStage) -> Option<SpectrumFrame> {
        self.spectrum_frame_at(stage, SpectrumResolution::Balanced)
    }
    pub fn spectrum_frame_at(
        &self,
        stage: SpectrumStage,
        resolution: SpectrumResolution,
    ) -> Option<SpectrumFrame> {
        self.spectrum.get()?.frame(stage, resolution)
    }
    pub(crate) fn record_spectrum(&self, pre: (f32, f32), post: (f32, f32), rate: u32) {
        if let Some(tap) = self.spectrum.get() {
            tap.record(pre, post, rate);
        }
    }
    pub fn publish(&self, levels: [f32; 2]) {
        for (peak, level) in self.peaks.iter().zip(levels) {
            if level.is_finite() && level > 0.0 {
                peak.fetch_max(level.to_bits(), Ordering::Relaxed);
            }
        }
    }

    pub fn take(&self) -> [f32; 2] {
        self.peaks
            .each_ref()
            .map(|peak| f32::from_bits(peak.swap(0, Ordering::Relaxed)))
    }
}

/// One prepared hosted insert in a track's channel strip.
///
/// The processor is shared with the control side and must remain valid across
/// playback-plan snapshots. Disabled inserts are reported but never processed.
#[derive(Clone)]
pub struct HostedInsert {
    pub processor: Arc<dyn opentrack_plugins::HostedProcessor>,
    pub enabled: bool,
}

impl HostedInsert {
    /// Latency contributed to the track path by this insert. Disabled inserts
    /// contribute nothing.
    pub fn latency_samples(&self) -> u32 {
        if self.enabled {
            self.processor.latency_samples()
        } else {
            0
        }
    }

    /// Tail length contributed by this insert, for future offline render.
    pub fn tail_samples(&self) -> u32 {
        if self.enabled {
            self.processor.tail_samples()
        } else {
            0
        }
    }
}

/// Bounded, allocation-free report of hosted processing failures.
///
/// Only the most recent failure is retained. `last_error_code` is an FNV-1a
/// hash of the processor's error text so the callback can record a diagnostic
/// without storing or formatting strings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HostedProcessingDiagnostic {
    pub error_count: u64,
    pub last_track: Option<usize>,
    pub last_insert: Option<usize>,
    pub last_error_code: Option<u32>,
}

/// Immutable channel-strip data assembled by the UI thread before publication.
#[derive(Clone)]
pub struct PlaybackTrackSettings {
    pub is_group: bool,
    pub audio_channels: u8,
    pub clips: Vec<PlaybackClip>,
    pub midi_notes: Vec<PlaybackMidiNote>,
    pub midi_controllers: Vec<PlaybackMidiController>,
    pub instrument_factory: Arc<dyn InstrumentFactory>,
    /// Atomic peak accumulator read and cleared by the UI thread.
    pub meter_peak: Arc<StereoMeter>,
    pub gain: f32,
    pub pan: f32,
    pub track_delay_ms: f32,
    pub muted: bool,
    pub solo: bool,
    pub solo_defeat: bool,
    pub equalizer: ChannelEqualizer,
    pub inserts: Vec<InsertEffect>,
    pub inserts_bypassed: bool,
    /// Hosted (VST3-style) inserts, processed block-wise after `inserts`.
    pub hosted_inserts: Vec<HostedInsert>,
    pub monitor_input: Option<PlaybackInputChannel>,
    pub output_route: PlaybackOutputRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackInputChannel {
    Stereo,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackOutputRoute {
    Stereo,
    Left,
    Right,
    Disconnected,
    Group(usize),
}

// SAFETY: the only mutable access to the processor and peak accumulator is
// from the audio callback. The callback uses callback_active as a single-render
// gate, while the UI reads atomic peaks and replaces the ArcSwap snapshot.
unsafe impl Sync for PlaybackTrackRuntime {}

const NO_HOSTED_ERROR: u32 = u32::MAX;

fn hosted_error_code(message: &str) -> u32 {
    message.bytes().fold(2_166_136_261_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    })
}

/// Immutable audio state read by the device callback.
pub struct PlaybackPlan {
    metronome: MetronomeSettings,
    metronome_downbeat_frequency: f64,
    output_bus: bus::BusRuntime,
    output_hosted_inserts: Vec<HostedInsert>,
    output_stage_timing: Option<StageTimingTrace>,
    output_meter: Arc<StereoMeter>,
    tempo_bpm: f64,
    tracks: Vec<PlaybackTrackRuntime>,
    /// Group tracks ordered deepest-first so child buses feed their parents.
    group_order: Vec<usize>,
    any_solo: bool,
    loop_range_frames: Option<(u64, u64)>,
    cycle_mode: CycleMode,
    generation: u64,
    monitor_mute_mask: Vec<bool>,
    hosted_processing: bool,
    hosted_max_latency_samples: u32,
    hosted_error_count: AtomicU64,
    hosted_error_track: AtomicU32,
    hosted_error_insert: AtomicU32,
    hosted_error_code: AtomicU32,
    callback_metrics: Arc<CallbackMetrics>,
    transition_from: ArcSwapOption<PlaybackPlan>,
    transition_position: AtomicU32,
    transition_frames: u32,
    note_audition: Arc<ArcSwapOption<NoteAudition>>,
}

#[derive(Default)]
struct CallbackMetrics {
    load_ema: AtomicU32,
    overruns: AtomicU64,
}

const CALLBACK_TRACE_CAPACITY: usize = 2048;

/// Bounded callback timing samples. Writers run on the audio callback; readers
/// snapshot and calculate summaries on the UI/control thread.
struct CallbackTimingTrace {
    enabled: bool,
    write_index: AtomicU64,
    overwritten_samples: AtomicU64,
    samples: [TimingSampleSlot; CALLBACK_TRACE_CAPACITY],
}

/// Opt-in ASIO callback-thread MMCSS A/B state. Registration runs only on the
/// first driver callback; teardown is requested by the owner and completed by
/// that callback thread before the stream is stopped.
struct AsioMmcssTrace {
    enabled: bool,
    state: AtomicU8,
    teardown_requested: AtomicBool,
    teardown_complete: AtomicBool,
    first_setup_ns: AtomicU64,
    registered_thread_id: AtomicU32,
    priority_set: AtomicBool,
}

impl AsioMmcssTrace {
    const DISABLED: u8 = 0;
    const PENDING: u8 = 1;
    const SETUP: u8 = 2;
    const ENROLLED: u8 = 3;
    const SETUP_FAILED: u8 = 4;
    const REVERTED: u8 = 5;
    const REVERT_FAILED: u8 = 6;
    const TEARDOWN_TIMEOUT: u8 = 7;

    fn new() -> Self {
        let enabled = std::env::var_os("OPENTRACK_ASIO_MMCSS").is_some_and(|v| v == "1");
        Self {
            enabled,
            state: AtomicU8::new(if enabled {
                Self::PENDING
            } else {
                Self::DISABLED
            }),
            teardown_requested: AtomicBool::new(false),
            teardown_complete: AtomicBool::new(!enabled),
            first_setup_ns: AtomicU64::new(0),
            registered_thread_id: AtomicU32::new(0),
            priority_set: AtomicBool::new(false),
        }
    }

    fn begin_setup(&self) -> bool {
        self.enabled
            && !self.teardown_requested.load(Ordering::Acquire)
            && self
                .state
                .compare_exchange(
                    Self::PENDING,
                    Self::SETUP,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
    }

    fn finish_setup(&self, enrolled: bool, priority_set: bool, setup_ns: u64, thread_id: u32) {
        self.first_setup_ns.store(setup_ns, Ordering::Relaxed);
        if enrolled {
            self.registered_thread_id
                .store(thread_id, Ordering::Relaxed);
            self.priority_set.store(priority_set, Ordering::Relaxed);
            self.state.store(Self::ENROLLED, Ordering::Release);
        } else {
            self.state.store(Self::SETUP_FAILED, Ordering::Release);
            self.teardown_complete.store(true, Ordering::Release);
        }
    }

    fn request_teardown(&self) {
        self.teardown_requested.store(true, Ordering::Release);
        if self.state.load(Ordering::Acquire) == Self::PENDING {
            let _ = self.state.compare_exchange(
                Self::PENDING,
                Self::DISABLED,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            self.teardown_complete.store(true, Ordering::Release);
        }
    }

    fn finish_teardown(&self, reverted: bool) {
        self.state.store(
            if reverted {
                Self::REVERTED
            } else {
                Self::REVERT_FAILED
            },
            Ordering::Release,
        );
        self.teardown_complete.store(true, Ordering::Release);
    }

    fn summary(&self) -> String {
        if !self.enabled {
            return "ASIO MMCSS A/B disabled (set OPENTRACK_ASIO_MMCSS=1 to enable)".into();
        }
        match self.state.load(Ordering::Acquire) {
            Self::PENDING => "ASIO MMCSS A/B waiting for first callback".into(),
            Self::SETUP => "ASIO MMCSS A/B first callback setup in progress".into(),
            Self::ENROLLED => format!(
                "ASIO MMCSS enrolled on callback thread {} | high priority {} | first setup {} us | teardown pending",
                self.registered_thread_id.load(Ordering::Relaxed),
                if self.priority_set.load(Ordering::Relaxed) {
                    "set"
                } else {
                    "not set"
                },
                self.first_setup_ns.load(Ordering::Relaxed) / 1_000,
            ),
            Self::SETUP_FAILED => format!(
                "ASIO MMCSS registration failed | first attempt {} us",
                self.first_setup_ns.load(Ordering::Relaxed) / 1_000,
            ),
            Self::REVERTED => format!(
                "ASIO MMCSS enrolled then reverted on callback thread {} | high priority {} | first setup {} us",
                self.registered_thread_id.load(Ordering::Relaxed),
                if self.priority_set.load(Ordering::Relaxed) {
                    "set"
                } else {
                    "not set"
                },
                self.first_setup_ns.load(Ordering::Relaxed) / 1_000,
            ),
            Self::REVERT_FAILED => format!(
                "ASIO MMCSS teardown failed on callback thread {} | first setup {} us",
                self.registered_thread_id.load(Ordering::Relaxed),
                self.first_setup_ns.load(Ordering::Relaxed) / 1_000,
            ),
            Self::TEARDOWN_TIMEOUT => format!(
                "ASIO MMCSS teardown callback not observed | first setup {} us",
                self.first_setup_ns.load(Ordering::Relaxed) / 1_000,
            ),
            _ => "ASIO MMCSS A/B disabled".into(),
        }
    }
}

struct TimingSampleSlot {
    version: AtomicU64,
    sequence: AtomicU64,
    interval_ns: AtomicU64,
    duration_ns: AtomicU64,
}

impl TimingSampleSlot {
    fn new() -> Self {
        Self {
            version: AtomicU64::new(0),
            sequence: AtomicU64::new(u64::MAX),
            interval_ns: AtomicU64::new(0),
            duration_ns: AtomicU64::new(0),
        }
    }

    fn write(&self, sequence: u64, interval_ns: u64, duration_ns: u64) {
        self.version.fetch_add(1, Ordering::AcqRel);
        self.sequence.store(sequence, Ordering::Relaxed);
        self.interval_ns.store(interval_ns, Ordering::Relaxed);
        self.duration_ns.store(duration_ns, Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Release);
    }

    fn read(&self, sequence: u64) -> Option<(u64, u64)> {
        let before = self.version.load(Ordering::Acquire);
        if before % 2 != 0 || self.sequence.load(Ordering::Relaxed) != sequence {
            return None;
        }
        let interval = self.interval_ns.load(Ordering::Relaxed);
        let duration = self.duration_ns.load(Ordering::Relaxed);
        (self.version.load(Ordering::Acquire) == before).then_some((interval, duration))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CallbackTimingSummary {
    pub samples: usize,
    pub overwritten_samples: u64,
    pub interval_p95_us: u64,
    pub interval_max_us: u64,
    pub duration_p95_us: u64,
    pub duration_max_us: u64,
}

const STAGE_TRACE_CAPACITY: usize = 512;

/// Bounded per-stage duration samples. The callback only stores one atomic
/// duration; sorting and allocation happen when the control/UI side reads it.
struct StageTimingTrace {
    write_index: AtomicU64,
    samples: [TimingSampleSlot; STAGE_TRACE_CAPACITY],
}

#[derive(Clone, Copy, Debug)]
pub struct StageTimingSummary {
    pub samples: usize,
    pub overwritten_samples: u64,
    pub duration_p95_us: u64,
    pub duration_max_us: u64,
}

impl StageTimingTrace {
    fn new() -> Option<Self> {
        std::env::var_os("OPENTRACK_REALTIME_DIAGNOSTICS")?;
        Some(Self {
            write_index: AtomicU64::new(0),
            samples: std::array::from_fn(|_| TimingSampleSlot::new()),
        })
    }

    fn record(&self, started: Instant) {
        let duration = started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        let sequence = self.write_index.fetch_add(1, Ordering::Relaxed);
        self.samples[sequence as usize % STAGE_TRACE_CAPACITY].write(sequence, 0, duration);
    }

    fn summary(&self) -> Option<StageTimingSummary> {
        let end = self.write_index.load(Ordering::Acquire);
        let count = end.min(STAGE_TRACE_CAPACITY as u64) as usize;
        if count == 0 {
            return None;
        }
        let start = end.saturating_sub(STAGE_TRACE_CAPACITY as u64);
        let mut durations = Vec::with_capacity(count);
        for sequence in start..end {
            if let Some((_, value)) =
                self.samples[sequence as usize % STAGE_TRACE_CAPACITY].read(sequence)
            {
                if value > 0 {
                    durations.push(value);
                }
            }
        }
        if durations.is_empty() {
            return None;
        }
        durations.sort_unstable();
        let rank = durations
            .len()
            .saturating_mul(95)
            .div_ceil(100)
            .saturating_sub(1)
            .min(durations.len() - 1);
        Some(StageTimingSummary {
            samples: durations.len(),
            overwritten_samples: end.saturating_sub(STAGE_TRACE_CAPACITY as u64),
            duration_p95_us: durations[rank] / 1_000,
            duration_max_us: durations[durations.len() - 1] / 1_000,
        })
    }
}

impl CallbackTimingTrace {
    fn new() -> Self {
        Self {
            enabled: std::env::var_os("OPENTRACK_REALTIME_DIAGNOSTICS").is_some(),
            write_index: AtomicU64::new(0),
            overwritten_samples: AtomicU64::new(0),
            samples: std::array::from_fn(|_| TimingSampleSlot::new()),
        }
    }

    fn record(&self, interval: Option<Duration>, duration: Duration) {
        self.record_ns(
            interval.map_or(0, |value| value.as_nanos().min(u64::MAX as u128) as u64),
            duration.as_nanos().min(u64::MAX as u128) as u64,
        );
    }

    fn record_ns(&self, interval_ns: u64, duration_ns: u64) {
        let index = self.write_index.fetch_add(1, Ordering::Relaxed);
        self.samples[index as usize % CALLBACK_TRACE_CAPACITY].write(
            index,
            interval_ns,
            duration_ns,
        );
    }

    fn report_overwritten(&self, count: u64) {
        self.overwritten_samples.fetch_add(count, Ordering::Relaxed);
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn summary(&self) -> Option<CallbackTimingSummary> {
        if !self.enabled {
            return None;
        }
        let end = self.write_index.load(Ordering::Acquire);
        let count = end.min(CALLBACK_TRACE_CAPACITY as u64) as usize;
        if count < 2 {
            return None;
        }
        let start = end.saturating_sub(CALLBACK_TRACE_CAPACITY as u64);
        let mut intervals = Vec::with_capacity(count);
        let mut durations = Vec::with_capacity(count);
        for sequence in start..end {
            if let Some((interval, duration)) =
                self.samples[sequence as usize % CALLBACK_TRACE_CAPACITY].read(sequence)
            {
                if interval > 0 && duration > 0 {
                    intervals.push(interval);
                    durations.push(duration);
                }
            }
        }
        if intervals.is_empty() || durations.is_empty() {
            return None;
        }
        intervals.sort_unstable();
        durations.sort_unstable();
        let percentile = |values: &[u64], p: usize| -> u64 {
            let rank = values
                .len()
                .saturating_mul(p)
                .div_ceil(100)
                .saturating_sub(1);
            values[rank.min(values.len() - 1)] / 1_000
        };
        Some(CallbackTimingSummary {
            samples: durations.len(),
            overwritten_samples: self
                .overwritten_samples
                .load(Ordering::Relaxed)
                .saturating_add(end.saturating_sub(CALLBACK_TRACE_CAPACITY as u64)),
            interval_p95_us: percentile(&intervals, 95),
            interval_max_us: intervals[intervals.len() - 1] / 1_000,
            duration_p95_us: percentile(&durations, 95),
            duration_max_us: durations[durations.len() - 1] / 1_000,
        })
    }
}

#[cfg(test)]
mod callback_timing_tests {
    use super::*;

    #[test]
    fn callback_timing_summary_reports_percentiles_and_ring_overwrites() {
        let trace = CallbackTimingTrace {
            enabled: true,
            write_index: AtomicU64::new(0),
            overwritten_samples: AtomicU64::new(0),
            samples: std::array::from_fn(|_| TimingSampleSlot::new()),
        };
        for sample in 1..=CALLBACK_TRACE_CAPACITY + 2 {
            trace.record(
                Some(Duration::from_micros(sample as u64)),
                Duration::from_micros((sample * 2) as u64),
            );
        }
        let summary = trace.summary().unwrap();
        assert_eq!(summary.samples, CALLBACK_TRACE_CAPACITY);
        assert_eq!(summary.overwritten_samples, 2);
        assert!(summary.interval_p95_us > 0);
        assert!(summary.duration_p95_us >= summary.interval_p95_us);
        assert_eq!(
            summary.duration_max_us,
            ((CALLBACK_TRACE_CAPACITY + 2) * 2) as u64
        );
    }

    #[test]
    fn callback_timing_summary_is_disabled_by_default_without_samples() {
        let trace = CallbackTimingTrace {
            enabled: false,
            write_index: AtomicU64::new(0),
            overwritten_samples: AtomicU64::new(0),
            samples: std::array::from_fn(|_| TimingSampleSlot::new()),
        };
        assert!(trace.summary().is_none());
    }
}

#[cfg(test)]
mod hosted_block_size_tests {
    use super::effective_hosted_block_frames;

    #[test]
    fn effective_hosted_block_never_exceeds_callback_frames() {
        assert_eq!(effective_hosted_block_frames(128, 64), 64);
        assert_eq!(effective_hosted_block_frames(256, 64), 64);
        assert_eq!(effective_hosted_block_frames(32, 64), 32);
        assert_eq!(effective_hosted_block_frames(128, 128), 128);
    }
}

impl PlaybackPlan {
    fn empty() -> Self {
        Self {
            metronome: MetronomeSettings::default(),
            metronome_downbeat_frequency: MetronomeSettings::default().downbeat_frequency(),
            output_bus: bus::BusRuntime::new(
                &opentrack_core::project::BusChannel::output().strip,
                44100,
            ),
            output_hosted_inserts: Vec::new(),
            output_stage_timing: None,
            output_meter: Arc::new(StereoMeter::default()),
            tempo_bpm: 120.0,
            tracks: Vec::new(),
            group_order: Vec::new(),
            any_solo: false,
            loop_range_frames: None,
            cycle_mode: CycleMode::Loop,
            generation: 0,
            monitor_mute_mask: Vec::new(),
            hosted_processing: false,
            hosted_max_latency_samples: 0,
            hosted_error_count: AtomicU64::new(0),
            hosted_error_track: AtomicU32::new(NO_HOSTED_ERROR),
            hosted_error_insert: AtomicU32::new(NO_HOSTED_ERROR),
            hosted_error_code: AtomicU32::new(0),
            callback_metrics: Arc::new(CallbackMetrics::default()),
            transition_from: ArcSwapOption::empty(),
            transition_position: AtomicU32::new(0),
            transition_frames: 0,
            note_audition: Arc::new(ArcSwapOption::empty()),
        }
    }

    fn new(
        tempo_bpm: f64,
        tracks: Vec<PlaybackTrackSettings>,
        generation: u64,
        sample_rate: u32,
        loop_range_beats: Option<(f64, f64)>,
    ) -> Self {
        Self::new_with_output_hosted(
            tempo_bpm,
            tracks,
            generation,
            sample_rate,
            loop_range_beats,
            Vec::new(),
        )
    }

    fn new_with_output_hosted(
        tempo_bpm: f64,
        tracks: Vec<PlaybackTrackSettings>,
        generation: u64,
        sample_rate: u32,
        loop_range_beats: Option<(f64, f64)>,
        output_hosted_inserts: Vec<HostedInsert>,
    ) -> Self {
        let any_solo = tracks.iter().any(|track| track.solo);
        let tempo_bpm = if tempo_bpm.is_finite() && tempo_bpm > 0.0 {
            tempo_bpm
        } else {
            120.0
        };
        let loop_range_frames = loop_range_beats.and_then(|(start_beats, end_beats)| {
            if !start_beats.is_finite()
                || start_beats < 0.0
                || !end_beats.is_finite()
                || end_beats <= start_beats
            {
                return None;
            }
            let frames_per_beat = f64::from(sample_rate) * 60.0 / tempo_bpm;
            let start_frame = start_beats * frames_per_beat;
            let end_frame = end_beats * frames_per_beat;
            if !start_frame.is_finite()
                || !end_frame.is_finite()
                || start_frame < 0.0
                || end_frame <= start_frame
                || end_frame >= u64::MAX as f64
            {
                return None;
            }
            let range = (start_frame.round() as u64, end_frame.round() as u64);
            (range.1 > range.0).then_some(range)
        });
        let monitor_mute_mask = tracks
            .iter()
            .map(|track| track.monitor_input.is_some())
            .collect();
        let hosted_processing = !output_hosted_inserts.is_empty()
            || tracks.iter().any(|track| !track.hosted_inserts.is_empty());
        // Delay compensation is part of the published plan. Ring buffers are
        // sized here on the control thread; the callback never allocates.
        let latency_cap = sample_rate.saturating_mul(HOSTED_MAX_LATENCY_SECONDS);
        let track_latencies: Vec<u32> = tracks
            .iter()
            .map(|track| hosted_track_latency(&track.hosted_inserts, latency_cap))
            .collect();
        let hosted_max_latency_samples = track_latencies.iter().copied().max().unwrap_or(0);
        let group_order = tracks
            .iter()
            .enumerate()
            .filter_map(|(index, track)| track.is_group.then_some(index))
            .rev()
            .collect();
        Self {
            output_bus: bus::BusRuntime::new(
                &opentrack_core::project::BusChannel::output().strip,
                sample_rate,
            ),
            output_hosted_inserts,
            output_stage_timing: hosted_processing.then(StageTimingTrace::new).flatten(),
            output_meter: Arc::new(StereoMeter::default()),
            tempo_bpm,
            metronome: MetronomeSettings::default(),
            metronome_downbeat_frequency: MetronomeSettings::default().downbeat_frequency(),
            tracks: tracks
                .into_iter()
                .zip(track_latencies)
                .map(|(track, latency)| {
                    PlaybackTrackRuntime::new(
                        track,
                        tempo_bpm,
                        sample_rate,
                        latency,
                        hosted_max_latency_samples,
                        hosted_processing,
                    )
                })
                .collect(),
            group_order,
            any_solo,
            loop_range_frames,
            cycle_mode: CycleMode::Loop,
            generation,
            monitor_mute_mask,
            hosted_processing,
            hosted_max_latency_samples,
            hosted_error_count: AtomicU64::new(0),
            hosted_error_track: AtomicU32::new(NO_HOSTED_ERROR),
            hosted_error_insert: AtomicU32::new(NO_HOSTED_ERROR),
            hosted_error_code: AtomicU32::new(0),
            callback_metrics: Arc::new(CallbackMetrics::default()),
            transition_from: ArcSwapOption::empty(),
            transition_position: AtomicU32::new(0),
            transition_frames: (sample_rate.saturating_mul(PLAYBACK_PARAMETER_FADE_MS) / 1000)
                .max(1),
            note_audition: Arc::new(ArcSwapOption::empty()),
        }
    }

    fn begin_parameter_transition(&self, previous: Arc<PlaybackPlan>) {
        if self.hosted_processing || previous.hosted_processing {
            // Hosted processors are shared instances, not per-plan state. A
            // dual-plan crossfade would call each instance twice per callback
            // and would run two delay lines for the same track. Swap hosted
            // plans directly and restart PDC from the new snapshot instead.
            self.transition_position
                .store(self.transition_frames, Ordering::Relaxed);
            self.transition_from.store(None);
            return;
        }
        self.transition_position.store(0, Ordering::Relaxed);
        self.transition_from.store(Some(previous));
    }

    /// True when this plan uses the hosted block path. The legacy per-sample
    /// path remains in use for plans with no hosted inserts.
    pub fn hosted_processing_enabled(&self) -> bool {
        self.hosted_processing
    }

    /// Maximum hosted insert latency in samples used for playback compensation.
    pub fn hosted_latency_samples(&self) -> u32 {
        self.hosted_max_latency_samples
    }

    /// PDC delay applied to one track, in samples. Returns `None` for an
    /// unknown track index.
    pub fn track_hosted_delay_samples(&self, track_index: usize) -> Option<u32> {
        self.tracks
            .get(track_index)
            .map(|track| track.hosted_delay_samples)
    }

    /// Hosted latency reported by one track's enabled insert chain, in samples.
    pub fn track_hosted_latency_samples(&self, track_index: usize) -> Option<u32> {
        self.tracks
            .get(track_index)
            .map(|track| track.hosted_latency_samples)
    }

    /// Reads the bounded hosted-processing failure report.
    pub fn hosted_processing_diagnostic(&self) -> HostedProcessingDiagnostic {
        let track = self.hosted_error_track.load(Ordering::Relaxed);
        let insert = self.hosted_error_insert.load(Ordering::Relaxed);
        HostedProcessingDiagnostic {
            error_count: self.hosted_error_count.load(Ordering::Relaxed),
            last_track: (track != NO_HOSTED_ERROR).then_some(track as usize),
            last_insert: (insert != NO_HOSTED_ERROR).then_some(insert as usize),
            last_error_code: (track != NO_HOSTED_ERROR)
                .then(|| self.hosted_error_code.load(Ordering::Relaxed)),
        }
    }

    fn record_hosted_error(&self, track_index: usize, insert_index: usize, message: &str) {
        let _ = self
            .hosted_error_count
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                Some(count.saturating_add(1))
            });
        self.hosted_error_track.store(
            u32::try_from(track_index).unwrap_or(NO_HOSTED_ERROR),
            Ordering::Relaxed,
        );
        self.hosted_error_insert.store(
            u32::try_from(insert_index).unwrap_or(NO_HOSTED_ERROR),
            Ordering::Relaxed,
        );
        self.hosted_error_code
            .store(hosted_error_code(message), Ordering::Relaxed);
    }

    fn detach_completed_transition(&self) -> Option<Arc<PlaybackPlan>> {
        (self.transition_frames == 0
            || self.transition_position.load(Ordering::Acquire) >= self.transition_frames)
            .then(|| self.transition_from.swap(None))
            .flatten()
    }
}

/// Break completed snapshot links on the control thread so retired DSP state
/// is never deallocated by the real-time callback.
fn detach_transition_chain(plan: &PlaybackPlan, retired: &mut Vec<Arc<PlaybackPlan>>) {
    if let Some(previous) = plan.detach_completed_transition() {
        detach_transition_chain(&previous, retired);
        retired.push(previous);
    }
}

#[derive(Clone, Debug)]
pub struct AsioDeviceInfo {
    pub name: String,
    pub input_ports: Vec<String>,
    pub output_ports: Vec<String>,
    pub sample_rates: Vec<u32>,
    pub buffer_choices: Vec<u32>,
    pub input_latency_frames: u32,
    pub output_latency_frames: u32,
}

#[cfg(windows)]
#[allow(dead_code)]
enum WindowsOutputStream {
    Wasapi(wasapi_backend::WasapiOutputStream),
    #[cfg(feature = "local-asio")]
    Asio(asio_backend::AsioOutputStream),
}

pub fn open_asio_control_panel(id: String) -> Result<(), String> {
    #[cfg(all(windows, feature = "local-asio"))]
    return asio_backend::control_panel(id);
    #[cfg(not(all(windows, feature = "local-asio")))]
    {
        let _ = id;
        Err("ASIO is unavailable in this build".into())
    }
}

/// A running output stream for the system's default audio device.
pub struct AudioEngine {
    #[cfg(not(windows))]
    _stream: cpal::Stream,
    #[cfg(windows)]
    _windows_stream: WindowsOutputStream,
    test_tone_enabled: Arc<AtomicBool>,
    playing: Arc<AtomicBool>,
    playhead_frame: Arc<AtomicU64>,
    plan: Arc<ArcSwap<PlaybackPlan>>,
    input_monitor: Arc<ArcSwapOption<AudioMonitorBuffer>>,
    live_midi_queue: Arc<LiveMidiQueue>,
    live_midi_enabled: Arc<AtomicBool>,
    live_midi_waveform: Arc<AtomicU8>,
    live_midi_gain: Arc<AtomicU32>,
    live_midi_dropped: Arc<AtomicU64>,
    asio_overload_windows: Arc<AtomicU64>,
    asio_callback_collisions: Arc<AtomicU64>,
    asio_mmcss_trace: Arc<AsioMmcssTrace>,
    retired_input_monitors: Mutex<Vec<Arc<AudioMonitorBuffer>>>,
    retired_note_auditions: Mutex<Vec<Arc<NoteAudition>>>,
    callback_timing: Arc<CallbackTimingTrace>,
    asio_callback_timing: Arc<CallbackTimingTrace>,
    wasapi_callback_timing: Arc<CallbackTimingTrace>,
    callback_generation: Arc<AtomicU64>,
    callback_active: Arc<AtomicBool>,
    output_stream_failed: Arc<AtomicBool>,
    current_generation: u64,
    retired_plans: Vec<Arc<PlaybackPlan>>,
    pub device_name: String,
    pub device_id: String,
    pub asio_info: Option<AsioDeviceInfo>,
    pub sample_rate: u32,
    pub channels: u16,
    pub requested_buffer_frames: Option<u32>,
    pub output_mode: AudioDeviceMode,
    pub output_period_frames: u32,
    pub output_period_choices: Vec<u32>,
    pub output_buffer_frames: u32,
    pub output_mode_detail: String,
}

impl AudioEngine {
    /// Opens and starts a silent stream on the default output device.
    pub fn open_default_output() -> Result<Self, String> {
        Self::open_output_device_mode(None, None, None, AudioDeviceMode::SharedLowLatency)
    }

    /// Opens a selected output device with an optional sample rate and buffer size.
    pub fn open_output_device(
        device_id: Option<&str>,
        requested_sample_rate: Option<u32>,
        requested_buffer_frames: Option<u32>,
    ) -> Result<Self, String> {
        Self::open_output_device_mode(
            device_id,
            requested_sample_rate,
            requested_buffer_frames,
            AudioDeviceMode::SharedLowLatency,
        )
    }

    pub fn open_output_device_mode(
        device_id: Option<&str>,
        requested_sample_rate: Option<u32>,
        requested_buffer_frames: Option<u32>,
        output_mode: AudioDeviceMode,
    ) -> Result<Self, String> {
        // Resolve the experimental hosted block size before any real-time
        // callback can query it; callbacks never read environment variables.
        let _ = hosted_block_frames();
        #[cfg(windows)]
        let (device_id, endpoint_id) = if let Some(id) =
            device_id.filter(|id| id.starts_with("asio:"))
        {
            #[cfg(not(feature = "local-asio"))]
            {
                let _ = id;
                return Err("This build does not include the optional local ASIO backend".into());
            }
            #[cfg(feature = "local-asio")]
            {
                (id.to_owned(), None)
            }
        } else {
            let endpoint_id = match device_id {
                Some(id) => wasapi_backend::endpoint_id_from_cpal(id)?,
                None => wasapi_backend::default_endpoint_id(wasapi::Direction::Render)?,
            };
            (format!("wasapi:{endpoint_id}"), Some(endpoint_id))
        };
        #[cfg(not(windows))]
        let device = crate::devices::output_device(device_id)?;
        #[cfg(not(windows))]
        if !device.supports_output() {
            return Err("the selected device does not support audio output".to_owned());
        }
        #[cfg(not(windows))]
        let device_name = device
            .description()
            .map(|description| description.name().to_owned())
            .unwrap_or_else(|_| device.to_string());
        #[cfg(not(windows))]
        let device_id = device
            .id()
            .map_err(|error| format!("could not identify the output device: {error}"))?
            .to_string();
        #[cfg(not(windows))]
        let (sample_format, config) = {
            let default_config = device
                .default_output_config()
                .map_err(|error| format!("could not read the output format: {error}"))?;
            let supported_config = if cfg!(windows) {
                default_config
            } else if let Some(sample_rate) = requested_sample_rate {
                device
                    .supported_output_configs()
                    .map_err(|error| format!("could not read output configurations: {error}"))?
                    .filter(|config| {
                        matches!(
                            config.sample_format(),
                            SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16
                        ) && config.channels() > 0
                    })
                    .filter_map(|config| config.try_with_sample_rate(sample_rate))
                    .min_by_key(|config| {
                        (
                            config.sample_format() != default_config.sample_format(),
                            config.channels() != default_config.channels(),
                            config.channels(),
                        )
                    })
                    .ok_or_else(|| {
                        format!("the selected output does not support {sample_rate} Hz")
                    })?
            } else {
                default_config
            };
            let sample_format = supported_config.sample_format();
            let mut config: cpal::StreamConfig = supported_config.into();
            if !cfg!(windows)
                && let Some(buffer_frames) = requested_buffer_frames
            {
                let range = device
                    .supported_output_configs()
                    .map_err(|error| format!("could not read supported buffer sizes: {error}"))?
                    .find(|range| {
                        range.channels() == config.channels
                            && range.sample_format() == sample_format
                            && range.min_sample_rate() <= config.sample_rate
                            && config.sample_rate <= range.max_sample_rate()
                    })
                    .map(|range| *range.buffer_size());
                match range {
                    Some(cpal::SupportedBufferSize::Range { min, max })
                        if buffer_frames >= min && buffer_frames <= max => {}
                    Some(cpal::SupportedBufferSize::Range { min, max }) => {
                        return Err(format!(
                            "the selected output supports buffer sizes from {min} to {max} frames"
                        ));
                    }
                    Some(cpal::SupportedBufferSize::Unknown) | None => {
                        return Err(
                            "the selected output does not expose a configurable buffer range"
                                .to_owned(),
                        );
                    }
                }
                config.buffer_size = BufferSize::Fixed(buffer_frames);
            }
            if config.channels == 0 || config.sample_rate == 0 {
                return Err("the selected output device reports an invalid format".to_owned());
            }
            (sample_format, config)
        };

        #[cfg(not(windows))]
        let channels = usize::from(config.channels);
        #[cfg(not(windows))]
        let output_channels = config.channels;
        #[cfg(not(windows))]
        let sample_rate = config.sample_rate;
        let test_tone_enabled = Arc::new(AtomicBool::new(false));
        let playing = Arc::new(AtomicBool::new(false));
        let playhead_frame = Arc::new(AtomicU64::new(0));
        let plan = Arc::new(ArcSwap::from_pointee(PlaybackPlan::empty()));
        let input_monitor = Arc::new(ArcSwapOption::empty());
        let live_midi_queue = Arc::new(LiveMidiQueue::new());
        let live_midi_enabled = Arc::new(AtomicBool::new(false));
        let live_midi_waveform = Arc::new(AtomicU8::new(0));
        let live_midi_gain = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
        let live_midi_dropped = Arc::new(AtomicU64::new(0));
        let asio_overload_windows = Arc::new(AtomicU64::new(0));
        let asio_callback_collisions = Arc::new(AtomicU64::new(0));
        let asio_mmcss_trace = Arc::new(AsioMmcssTrace::new());
        let callback_timing = Arc::new(CallbackTimingTrace::new());
        let asio_callback_timing = Arc::new(CallbackTimingTrace::new());
        let wasapi_callback_timing = Arc::new(CallbackTimingTrace::new());
        let callback_generation = Arc::new(AtomicU64::new(0));
        let callback_active = Arc::new(AtomicBool::new(false));
        let output_stream_failed = Arc::new(AtomicBool::new(false));

        #[cfg(windows)]
        {
            let render_tone = Arc::clone(&test_tone_enabled);
            let render_playing = Arc::clone(&playing);
            let render_playhead = Arc::clone(&playhead_frame);
            let render_plan = Arc::clone(&plan);
            let render_monitor = Arc::clone(&input_monitor);
            let render_midi_queue = Arc::clone(&live_midi_queue);
            let render_midi_enabled = Arc::clone(&live_midi_enabled);
            let render_midi_waveform = Arc::clone(&live_midi_waveform);
            let render_midi_gain = Arc::clone(&live_midi_gain);
            let render_generation = Arc::clone(&callback_generation);
            let render_active = Arc::clone(&callback_active);
            let worker_error = Arc::clone(&output_stream_failed);
            let render_timing = Arc::clone(&callback_timing);
            let make_renderer =
                move |actual_rate, actual_channels| -> Box<dyn FnMut(&mut [f32]) + Send> {
                    let channels = usize::from(actual_channels);
                    let mut phase = 0.0_f32;
                    let mut monitor_reader = AudioMonitorReader::default();
                    let mut live_midi = LiveMidiRenderer::new(
                        Arc::clone(&render_midi_queue),
                        Arc::clone(&render_midi_enabled),
                        Arc::clone(&render_midi_waveform),
                        Arc::clone(&render_midi_gain),
                    );
                    let (tone, playing, playhead, plan, generation, active) = callback_state(
                        &render_tone,
                        &render_playing,
                        &render_playhead,
                        &render_plan,
                        &render_generation,
                        &render_active,
                    );
                    let timing = Arc::clone(&render_timing);
                    let mut last_entry = None;
                    Box::new(move |samples: &mut [f32]| {
                        let started = timing.enabled.then(Instant::now);
                        let interval = started
                            .zip(last_entry)
                            .map(|(now, previous)| now.duration_since(previous));
                        if let Some(now) = started {
                            last_entry = Some(now);
                        }
                        render_f32_with_midi(
                            samples,
                            channels,
                            actual_rate,
                            &tone,
                            &playing,
                            &playhead,
                            &plan,
                            &generation,
                            &active,
                            &render_monitor,
                            &mut monitor_reader,
                            &mut phase,
                            Some(&mut live_midi),
                        );
                        if let Some(started) = started {
                            timing.record(interval, started.elapsed());
                        }
                    })
                };
            let (windows_stream, info, asio_info) = if let Some(endpoint_id) = endpoint_id {
                let stream = wasapi_backend::WasapiOutputStream::open(
                    endpoint_id,
                    output_mode,
                    requested_sample_rate,
                    requested_buffer_frames,
                    worker_error,
                    Arc::clone(&wasapi_callback_timing),
                    make_renderer,
                )?;
                let info = stream.info.clone();
                (WindowsOutputStream::Wasapi(stream), info, None)
            } else {
                #[cfg(feature = "local-asio")]
                {
                    let stream = asio_backend::AsioOutputStream::open(
                        device_id.clone(),
                        requested_sample_rate,
                        requested_buffer_frames,
                        worker_error,
                        Arc::clone(&asio_overload_windows),
                        Arc::clone(&asio_callback_collisions),
                        Arc::clone(&asio_mmcss_trace),
                        Arc::clone(&asio_callback_timing),
                        make_renderer,
                    )?;
                    let info = stream.info.clone();
                    let driver_info = stream.driver_info.clone();
                    (WindowsOutputStream::Asio(stream), info, Some(driver_info))
                }
                #[cfg(not(feature = "local-asio"))]
                {
                    return Err("ASIO is unavailable in this build".into());
                }
            };
            ACTIVE_OUTPUT_PERIOD_FRAMES.store(info.period_frames as usize, Ordering::Release);
            return Ok(Self {
                _windows_stream: windows_stream,
                test_tone_enabled,
                playing,
                playhead_frame,
                plan,
                input_monitor,
                live_midi_queue,
                live_midi_enabled,
                live_midi_waveform,
                live_midi_gain,
                live_midi_dropped,
                asio_overload_windows,
                asio_callback_collisions,
                asio_mmcss_trace,
                retired_input_monitors: Mutex::new(Vec::new()),
                retired_note_auditions: Mutex::new(Vec::new()),
                callback_timing,
                asio_callback_timing,
                wasapi_callback_timing,
                callback_generation,
                callback_active,
                output_stream_failed,
                current_generation: 0,
                retired_plans: Vec::new(),
                asio_info,
                device_name: info.device_name,
                device_id,
                sample_rate: info.sample_rate,
                channels: info.channels,
                requested_buffer_frames,
                output_mode: info.mode,
                output_period_frames: info.period_frames,
                output_period_choices: info.period_choices,
                output_buffer_frames: info.buffer_frames,
                output_mode_detail: info.detail,
            });
        }

        #[cfg(not(windows))]
        {
            let stream = match sample_format {
                cpal::SampleFormat::F32 => {
                    let mut phase = 0.0_f32;
                    let mut monitor_reader = AudioMonitorReader::default();
                    let mut live_midi = LiveMidiRenderer::new(
                        Arc::clone(&live_midi_queue),
                        Arc::clone(&live_midi_enabled),
                        Arc::clone(&live_midi_waveform),
                        Arc::clone(&live_midi_gain),
                    );
                    let input_monitor = Arc::clone(&input_monitor);
                    let stream_failed = Arc::clone(&output_stream_failed);
                    let (tone, playing, playhead, plan, generation, active) = callback_state(
                        &test_tone_enabled,
                        &playing,
                        &playhead_frame,
                        &plan,
                        &callback_generation,
                        &callback_active,
                    );
                    device.build_output_stream(
                        config,
                        move |samples: &mut [f32], _| {
                            render_f32_with_midi(
                                samples,
                                channels,
                                sample_rate,
                                &tone,
                                &playing,
                                &playhead,
                                &plan,
                                &generation,
                                &active,
                                &input_monitor,
                                &mut monitor_reader,
                                &mut phase,
                                Some(&mut live_midi),
                            );
                        },
                        move |_| {
                            stream_failed.store(true, Ordering::Release);
                        },
                        Some(Duration::from_secs(5)),
                    )
                }
                cpal::SampleFormat::I16 => {
                    let mut phase = 0.0_f32;
                    let mut monitor_reader = AudioMonitorReader::default();
                    let mut live_midi = LiveMidiRenderer::new(
                        Arc::clone(&live_midi_queue),
                        Arc::clone(&live_midi_enabled),
                        Arc::clone(&live_midi_waveform),
                        Arc::clone(&live_midi_gain),
                    );
                    let input_monitor = Arc::clone(&input_monitor);
                    let stream_failed = Arc::clone(&output_stream_failed);
                    let (tone, playing, playhead, plan, generation, active) = callback_state(
                        &test_tone_enabled,
                        &playing,
                        &playhead_frame,
                        &plan,
                        &callback_generation,
                        &callback_active,
                    );
                    device.build_output_stream(
                        config,
                        move |samples: &mut [i16], _| {
                            render_i16(
                                samples,
                                channels,
                                sample_rate,
                                &tone,
                                &playing,
                                &playhead,
                                &plan,
                                &generation,
                                &active,
                                &input_monitor,
                                &mut monitor_reader,
                                &mut phase,
                                Some(&mut live_midi),
                            );
                        },
                        move |_| {
                            stream_failed.store(true, Ordering::Release);
                        },
                        Some(Duration::from_secs(5)),
                    )
                }
                cpal::SampleFormat::U16 => {
                    let mut phase = 0.0_f32;
                    let mut monitor_reader = AudioMonitorReader::default();
                    let mut live_midi = LiveMidiRenderer::new(
                        Arc::clone(&live_midi_queue),
                        Arc::clone(&live_midi_enabled),
                        Arc::clone(&live_midi_waveform),
                        Arc::clone(&live_midi_gain),
                    );
                    let input_monitor = Arc::clone(&input_monitor);
                    let stream_failed = Arc::clone(&output_stream_failed);
                    let (tone, playing, playhead, plan, generation, active) = callback_state(
                        &test_tone_enabled,
                        &playing,
                        &playhead_frame,
                        &plan,
                        &callback_generation,
                        &callback_active,
                    );
                    device.build_output_stream(
                        config,
                        move |samples: &mut [u16], _| {
                            render_u16(
                                samples,
                                channels,
                                sample_rate,
                                &tone,
                                &playing,
                                &playhead,
                                &plan,
                                &generation,
                                &active,
                                &input_monitor,
                                &mut monitor_reader,
                                &mut phase,
                                Some(&mut live_midi),
                            );
                        },
                        move |_| {
                            stream_failed.store(true, Ordering::Release);
                        },
                        Some(Duration::from_secs(5)),
                    )
                }
                format => {
                    return Err(format!(
                        "the default output sample format {format:?} is not supported yet"
                    ));
                }
            }
            .map_err(|error| format!("could not open the selected output stream: {error}"))?;

            stream
                .play()
                .map_err(|error| format!("could not start the selected output stream: {error}"))?;

            Ok(Self {
                _stream: stream,
                test_tone_enabled,
                playing,
                playhead_frame,
                plan,
                input_monitor,
                live_midi_queue,
                live_midi_enabled,
                live_midi_waveform,
                live_midi_gain,
                live_midi_dropped,
                asio_overload_windows,
                asio_callback_collisions,
                asio_mmcss_trace,
                retired_input_monitors: Mutex::new(Vec::new()),
                retired_note_auditions: Mutex::new(Vec::new()),
                callback_generation,
                callback_active,
                output_stream_failed,
                current_generation: 0,
                retired_plans: Vec::new(),
                device_name,
                device_id,
                asio_info: None,
                callback_timing,
                asio_callback_timing,
                wasapi_callback_timing,
                sample_rate,
                channels: output_channels,
                requested_buffer_frames,
                output_mode,
                output_period_frames: requested_buffer_frames.unwrap_or(0),
                output_period_choices: Vec::new(),
                output_buffer_frames: requested_buffer_frames.unwrap_or(0),
                output_mode_detail: "CPAL output".to_owned(),
            })
        }
    }

    /// Returns and clears a stream failure reported asynchronously by CPAL.
    pub fn take_output_stream_error(&self) -> bool {
        self.output_stream_failed.swap(false, Ordering::AcqRel)
    }

    pub fn set_playback_plan(
        &mut self,
        tempo_bpm: f64,
        tracks: Vec<PlaybackTrackSettings>,
        loop_range_beats: Option<(f64, f64)>,
        cycle_mode: CycleMode,
    ) {
        self.set_playback_plan_with_output(
            tempo_bpm,
            tracks,
            loop_range_beats,
            cycle_mode,
            &opentrack_core::project::BusChannel::output().strip,
        );
    }

    pub fn set_playback_plan_with_output(
        &mut self,
        tempo_bpm: f64,
        tracks: Vec<PlaybackTrackSettings>,
        loop_range_beats: Option<(f64, f64)>,
        cycle_mode: CycleMode,
        output_channel: &opentrack_core::project::Track,
    ) {
        self.set_playback_plan_with_metronome(
            tempo_bpm,
            tracks,
            loop_range_beats,
            cycle_mode,
            output_channel,
            MetronomeSettings::default(),
        );
    }

    pub fn set_playback_plan_with_metronome(
        &mut self,
        tempo_bpm: f64,
        tracks: Vec<PlaybackTrackSettings>,
        loop_range_beats: Option<(f64, f64)>,
        cycle_mode: CycleMode,
        output_channel: &opentrack_core::project::Track,
        metronome: MetronomeSettings,
    ) {
        self.set_playback_plan_with_hosted_output(
            tempo_bpm,
            tracks,
            loop_range_beats,
            cycle_mode,
            output_channel,
            metronome,
            Vec::new(),
        );
    }

    pub fn set_playback_plan_with_hosted_output(
        &mut self,
        tempo_bpm: f64,
        tracks: Vec<PlaybackTrackSettings>,
        loop_range_beats: Option<(f64, f64)>,
        cycle_mode: CycleMode,
        output_channel: &opentrack_core::project::Track,
        metronome: MetronomeSettings,
        output_hosted_inserts: Vec<HostedInsert>,
    ) {
        self.current_generation = self.current_generation.wrapping_add(1).max(1);
        let previous = self.plan.load_full();
        let mut new_plan = PlaybackPlan::new_with_output_hosted(
            tempo_bpm,
            tracks,
            self.current_generation,
            self.sample_rate,
            loop_range_beats,
            output_hosted_inserts,
        );
        new_plan.callback_metrics = Arc::clone(&previous.callback_metrics);
        new_plan.output_meter = Arc::clone(&self.plan.load().output_meter);
        new_plan.cycle_mode = cycle_mode;
        new_plan.metronome = metronome;
        new_plan.metronome_downbeat_frequency = metronome.downbeat_frequency();
        new_plan.output_bus = bus::BusRuntime::new(output_channel, self.sample_rate);
        new_plan.note_audition = Arc::clone(&previous.note_audition);
        new_plan.begin_parameter_transition(previous);
        self.retired_plans.push(self.plan.swap(Arc::new(new_plan)));
        self.reclaim_retired_plans();
    }

    /// Updates live gain/pan and EQ targets without rebuilding clips or DSP
    /// graphs. The callback smooths the fader and retains biquad history.
    pub fn update_track_channel_parameters(
        &self,
        track_index: usize,
        gain: f32,
        pan: f32,
        equalizer: &ChannelEqualizer,
    ) -> bool {
        let plan = self.plan.load();
        let Some(track) = plan.tracks.get(track_index) else {
            return false;
        };
        track.mix.set(gain, pan);
        track.equalizer_control.set(equalizer);
        true
    }

    /// Starts or stops transient EQ-band audition without changing project EQ
    /// state or rebuilding the playback plan.
    pub fn update_track_eq_solo_band(&self, track_index: usize, band: Option<usize>) -> bool {
        let plan = self.plan.load();
        let Some(track) = plan.tracks.get(track_index) else {
            return false;
        };
        track
            .equalizer_control
            .set_solo_band(valid_eq_solo_band(band));
        true
    }

    /// Updates only the live fader target for a track. Project fader drags use
    /// this narrow path to avoid cloning equalizer state or rebuilding plans.
    pub fn update_track_gain(&self, track_index: usize, gain: f32) -> bool {
        let plan = self.plan.load();
        let Some(track) = plan.tracks.get(track_index) else {
            return false;
        };
        track.mix.set(gain, track.settings.pan);
        true
    }

    /// Updates live gain and pan targets without cloning EQ state or rebuilding
    /// the playback plan. The callback smooths both controls per sample.
    pub fn update_track_mix(&self, track_index: usize, gain: f32, pan: f32) -> bool {
        let plan = self.plan.load();
        let Some(track) = plan.tracks.get(track_index) else {
            return false;
        };
        track.mix.set(gain, pan);
        true
    }

    /// Updates one scheduled note's velocity without rebuilding its track's
    /// instrument or playback plan. The new value also affects an active voice.
    pub fn update_midi_note_velocity(
        &self,
        track_index: usize,
        note_id: u64,
        velocity: u8,
    ) -> bool {
        let plan = self.plan.load();
        let Some(track) = plan.tracks.get(track_index) else {
            return false;
        };
        track
            .midi_controls
            .update_note_velocity(note_id, f32::from(velocity.min(127)) / 127.0)
    }

    /// Releases retired playback plans after a live-control update, without
    /// requiring mutable access to the engine owner.
    pub fn reclaim_retired_plans_from_owner(&mut self) {
        self.reclaim_retired_plans();
    }

    pub fn update_output_channel_parameters(
        &self,
        gain: f32,
        pan: f32,
        equalizer: &ChannelEqualizer,
    ) {
        self.plan
            .load()
            .output_bus
            .set_channel_parameters(gain, pan, equalizer);
    }

    pub fn update_output_eq_solo_band(&self, band: Option<usize>) {
        self.plan
            .load()
            .output_bus
            .set_eq_solo_band(valid_eq_solo_band(band));
    }

    /// Releases replaced plans and callback snapshots after the audio callback
    /// has finished using them. This prevents final Arc drops for large monitor
    /// buffers and note-preview sources from running on the audio callback.
    pub fn reclaim_retired_plans(&mut self) {
        if self.callback_active.load(Ordering::SeqCst) {
            return;
        }
        if let Some(audition) = self.plan.load().note_audition.load_full() {
            audition.reclaim_retired();
        }
        self.retired_note_auditions
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        self.retired_input_monitors
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        let current = self.plan.load();
        // First detach any snapshots reachable only through transition links.
        // Their EQ retirement queues must be drained here as well, after the
        // callback gate confirms no plan is currently in use.
        detach_transition_chain(&current, &mut self.retired_plans);
        let mut index = 0;
        while index < self.retired_plans.len() {
            let plan = Arc::clone(&self.retired_plans[index]);
            detach_transition_chain(&plan, &mut self.retired_plans);
            index += 1;
        }
        current.output_bus.reclaim_retired();
        for track in &current.tracks {
            track.equalizer_control.reclaim_retired();
        }
        for plan in &self.retired_plans {
            plan.output_bus.reclaim_retired();
            for track in &plan.tracks {
                track.equalizer_control.reclaim_retired();
            }
        }
        let callback_generation = self.callback_generation.load(Ordering::Acquire);
        let mut index = 0;
        while index < self.retired_plans.len() {
            if self.retired_plans[index].generation <= callback_generation {
                self.retired_plans.swap_remove(index);
            } else {
                index += 1;
            }
        }
    }

    pub fn output_meter_tap(&self) -> Arc<StereoMeter> {
        Arc::clone(&self.plan.load().output_meter)
    }
    pub fn take_output_peaks(&self) -> [f32; 2] {
        self.plan.load().output_meter.take()
    }

    pub fn hosted_processing_diagnostic(&self) -> HostedProcessingDiagnostic {
        self.plan.load().hosted_processing_diagnostic()
    }

    /// Recent hosted processing durations, keyed by zero-based project track
    /// index. Percentiles are summarized on the calling control/UI thread.
    pub fn hosted_track_timing_summaries(&self) -> Vec<(usize, StageTimingSummary)> {
        self.plan
            .load()
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(index, track)| {
                track
                    .hosted_timing
                    .as_ref()
                    .and_then(StageTimingTrace::summary)
                    .map(|summary| (index, summary))
            })
            .collect()
    }

    /// Recent Stereo Out processing duration for one hosted render block.
    pub fn output_stage_timing_summary(&self) -> Option<StageTimingSummary> {
        self.plan
            .load()
            .output_stage_timing
            .as_ref()
            .and_then(StageTimingTrace::summary)
    }

    /// Exponentially smoothed render time as a percentage of the callback's
    /// audio deadline. Values above 100% indicate a callback overrun.
    pub fn callback_load_percent(&self) -> f32 {
        f32::from_bits(
            self.plan
                .load()
                .callback_metrics
                .load_ema
                .load(Ordering::Relaxed),
        ) * 100.0
    }

    /// Number of callbacks that exceeded their audio deadline or collided
    /// with another active renderer.
    pub fn callback_overrun_count(&self) -> u64 {
        self.plan
            .load()
            .callback_metrics
            .overruns
            .load(Ordering::Relaxed)
    }

    /// Number of 10 ms ASIO owner-loop polling windows that contained one or
    /// more driver-reported overload notifications. This is not an event count.
    pub fn asio_overload_window_count(&self) -> Option<u64> {
        self.asio_info.as_ref()?;
        Some(self.asio_overload_windows.load(Ordering::Relaxed))
    }

    /// Number of ASIO buffer-switch callbacks rejected because another
    /// callback was still using the stream's shared scratch and processor state.
    pub fn asio_callback_collision_count(&self) -> Option<u64> {
        self.asio_info.as_ref()?;
        Some(self.asio_callback_collisions.load(Ordering::Relaxed))
    }

    /// Summary of the opt-in ASIO callback-thread MMCSS A/B, including its
    /// first-callback setup cost and same-thread teardown result.
    pub fn asio_mmcss_summary(&self) -> Option<String> {
        self.asio_info.as_ref()?;
        Some(self.asio_mmcss_trace.summary())
    }

    pub fn callback_timing_summary(&self) -> Option<CallbackTimingSummary> {
        self.callback_timing.summary()
    }

    pub fn hosted_internal_block_frames(&self) -> (usize, usize) {
        let configured = hosted_block_frames();
        (
            configured,
            effective_hosted_block_frames(configured, self.output_period_frames as usize),
        )
    }

    pub fn asio_callback_timing_summary(&self) -> Option<CallbackTimingSummary> {
        self.asio_info.as_ref()?;
        self.asio_callback_timing.summary()
    }

    pub fn wasapi_callback_timing_summary(&self) -> Option<CallbackTimingSummary> {
        if self.asio_info.is_some() {
            return None;
        }
        self.wasapi_callback_timing.summary()
    }

    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
    }

    /// Starts or updates a note-drag preview without starting transport or
    /// rebuilding the playback plan. The previous preview fades out.
    pub fn set_note_audition(&self, audition: NoteAudition) {
        let plan = self.plan.load_full();
        if let Some(existing) = plan.note_audition.load_full() {
            existing.replace_with(audition);
        } else {
            plan.note_audition.store(Some(Arc::new(audition)));
        }
    }

    pub fn stop_note_audition(&self) {
        if let Some(audition) = self.plan.load_full().note_audition.load_full() {
            audition.stop();
        }
    }

    pub fn clear_note_audition(&self) {
        if let Some(audition) = self.plan.load_full().note_audition.swap(None) {
            self.retired_note_auditions
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(audition);
        }
    }

    /// Connects or disconnects the lock-free live input-monitor buffer.
    pub fn set_input_monitor(&self, monitor: Option<Arc<AudioMonitorBuffer>>) {
        let replacement = monitor.clone();
        if let Some(previous) = self.input_monitor.swap(monitor) {
            if !replacement
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(&previous, current))
            {
                self.retired_input_monitors
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(previous);
            }
        }
    }

    /// Enables the built-in live MIDI monitor on the default output stream.
    pub fn set_live_midi_enabled(&self, enabled: bool) {
        self.live_midi_enabled.store(enabled, Ordering::Release);
    }

    /// Selects the OpenTrack- starter waveform used by live MIDI monitoring.
    pub fn set_live_midi_waveform(&self, waveform: InstrumentWaveform) {
        self.live_midi_waveform
            .store(instrument_waveform_code(waveform), Ordering::Release);
    }

    /// Sets the live MIDI monitor's linear output gain.
    pub fn set_live_midi_gain(&self, gain: f32) {
        self.live_midi_gain.store(
            if gain.is_finite() {
                gain.clamp(0.0, opentrack_core::project::MAX_CHANNEL_GAIN)
                    .to_bits()
            } else {
                0.0_f32.to_bits()
            },
            Ordering::Release,
        );
    }

    /// Enqueues a note or controller event for the output callback.
    ///
    /// The callback never waits for this producer. `false` reports a full
    /// queue, and the dropped-message count can then be shown to the user.
    pub fn send_live_midi_message(&self, message: MidiInputMessage) -> bool {
        if self.live_midi_queue.push(message) {
            true
        } else {
            self.live_midi_dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    pub fn live_midi_dropped_messages(&self) -> u64 {
        self.live_midi_dropped.load(Ordering::Relaxed)
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn seek_seconds(&self, seconds: f64) {
        let frame = (seconds.max(0.0) * f64::from(self.sample_rate)).min(u64::MAX as f64) as u64;
        self.playhead_frame.store(frame, Ordering::Relaxed);
    }

    pub fn position_seconds(&self) -> f64 {
        self.playhead_frame.load(Ordering::Relaxed) as f64 / f64::from(self.sample_rate)
    }

    /// Enables or silences the diagnostic tone without changing project data.
    pub fn set_test_tone_enabled(&self, enabled: bool) {
        self.test_tone_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn test_tone_enabled(&self) -> bool {
        self.test_tone_enabled.load(Ordering::Relaxed)
    }
}

fn valid_eq_solo_band(band: Option<usize>) -> Option<usize> {
    band.filter(|index| *index < opentrack_core::project::MAX_EQ_BANDS)
}

const LIVE_MIDI_QUEUE_CAPACITY: usize = 2_048;
const LIVE_MIDI_QUEUE_MASK: u64 = (LIVE_MIDI_QUEUE_CAPACITY - 1) as u64;
const LIVE_MIDI_EVENT_MARKER: u64 = 1 << 63;
const LIVE_MIDI_EVENTS_PER_FRAME: usize = 64;

/// Single-producer/single-consumer fixed queue. The UI thread publishes
/// events and the output callback consumes them without locks or allocation.
struct LiveMidiQueue {
    events: Box<[AtomicU64]>,
    read_index: AtomicU64,
    write_index: AtomicU64,
}

impl LiveMidiQueue {
    fn new() -> Self {
        Self {
            events: (0..LIVE_MIDI_QUEUE_CAPACITY)
                .map(|_| AtomicU64::new(0))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            read_index: AtomicU64::new(0),
            write_index: AtomicU64::new(0),
        }
    }

    fn push(&self, message: MidiInputMessage) -> bool {
        let write = self.write_index.load(Ordering::Relaxed);
        let read = self.read_index.load(Ordering::Acquire);
        if write.wrapping_sub(read) >= LIVE_MIDI_QUEUE_CAPACITY as u64 {
            return false;
        }
        let packed = pack_live_midi_message(message);
        self.events[(write & LIVE_MIDI_QUEUE_MASK) as usize].store(packed, Ordering::Relaxed);
        self.write_index
            .store(write.wrapping_add(1), Ordering::Release);
        true
    }

    fn pop(&self) -> Option<MidiInputMessage> {
        let read = self.read_index.load(Ordering::Relaxed);
        let write = self.write_index.load(Ordering::Acquire);
        if read == write {
            return None;
        }
        let packed = self.events[(read & LIVE_MIDI_QUEUE_MASK) as usize].load(Ordering::Relaxed);
        self.read_index
            .store(read.wrapping_add(1), Ordering::Release);
        unpack_live_midi_message(packed)
    }
}

fn pack_live_midi_message(message: MidiInputMessage) -> u64 {
    let (kind, channel, first, second) = match message {
        MidiInputMessage::NoteOn {
            channel,
            key,
            velocity,
        } => (0_u64, channel, key, velocity),
        MidiInputMessage::NoteOff {
            channel,
            key,
            velocity,
        } => (1, channel, key, velocity),
        MidiInputMessage::ControlChange {
            channel,
            controller,
            value,
        } => (2, channel, controller, value),
    };
    LIVE_MIDI_EVENT_MARKER
        | (kind << 16)
        | (u64::from(channel & 0x0F) << 12)
        | (u64::from(first & 0x7F) << 5)
        | u64::from(second & 0x7F)
}

fn unpack_live_midi_message(packed: u64) -> Option<MidiInputMessage> {
    if packed & LIVE_MIDI_EVENT_MARKER == 0 {
        return None;
    }
    let kind = (packed >> 16) & 0xFF;
    let channel = ((packed >> 12) & 0x0F) as u8;
    let first = ((packed >> 5) & 0x7F) as u8;
    let second = (packed & 0x7F) as u8;
    match kind {
        0 => Some(MidiInputMessage::NoteOn {
            channel,
            key: first,
            velocity: second,
        }),
        1 => Some(MidiInputMessage::NoteOff {
            channel,
            key: first,
            velocity: second,
        }),
        2 => Some(MidiInputMessage::ControlChange {
            channel,
            controller: first,
            value: second,
        }),
        _ => None,
    }
}

struct LiveMidiRenderer {
    queue: Arc<LiveMidiQueue>,
    enabled: Arc<AtomicBool>,
    waveform: Arc<AtomicU8>,
    gain: Arc<AtomicU32>,
    synth: LiveMidiSynth,
}

impl LiveMidiRenderer {
    fn new(
        queue: Arc<LiveMidiQueue>,
        enabled: Arc<AtomicBool>,
        waveform: Arc<AtomicU8>,
        gain: Arc<AtomicU32>,
    ) -> Self {
        Self {
            queue,
            enabled,
            waveform,
            gain,
            synth: LiveMidiSynth::new(),
        }
    }

    fn next_frame(&mut self, sample_rate: u32) -> (f32, f32) {
        let mut received = 0;
        while received < LIVE_MIDI_EVENTS_PER_FRAME {
            let Some(message) = self.queue.pop() else {
                break;
            };
            self.synth.apply(message, sample_rate);
            received += 1;
        }
        if !self.enabled.load(Ordering::Acquire) {
            self.synth.reset();
            return (0.0, 0.0);
        }
        self.synth.set_waveform(instrument_waveform_from_code(
            self.waveform.load(Ordering::Acquire),
        ));
        let gain = f32::from_bits(self.gain.load(Ordering::Acquire))
            .clamp(0.0, opentrack_core::project::MAX_CHANNEL_GAIN);
        let (left, right) = self.synth.next_frame(sample_rate);
        (left * gain, right * gain)
    }
}

#[derive(Clone, Copy)]
struct LiveMidiVoice {
    channel: u8,
    key: u8,
    velocity: f32,
    frequency_hz: f32,
    phase: f32,
    age_frames: u32,
    release_frames: Option<u32>,
}

struct LiveMidiSynth {
    voices: [Option<LiveMidiVoice>; 64],
    controller_values: [[u8; 128]; 16],
    waveform: InstrumentWaveform,
}

impl LiveMidiSynth {
    fn new() -> Self {
        let mut controller_values = [[0; 128]; 16];
        for channel in &mut controller_values {
            channel[7] = 127;
            channel[10] = 64;
            channel[11] = 127;
        }
        Self {
            voices: [None; 64],
            controller_values,
            waveform: InstrumentWaveform::Sine,
        }
    }

    fn set_waveform(&mut self, waveform: InstrumentWaveform) {
        self.waveform = waveform;
    }

    fn reset(&mut self) {
        self.voices.fill(None);
    }

    fn apply(&mut self, message: MidiInputMessage, sample_rate: u32) {
        match message {
            MidiInputMessage::NoteOn {
                channel,
                key,
                velocity,
            } if velocity > 0 => {
                let channel = channel.min(15);
                let key = key.min(127);
                let velocity = f32::from(velocity.min(127)) / 127.0;
                let voice = LiveMidiVoice {
                    channel,
                    key,
                    velocity,
                    frequency_hz: 440.0_f32 * 2.0_f32.powf((f32::from(key) - 69.0) / 12.0),
                    phase: 0.0,
                    age_frames: 0,
                    release_frames: None,
                };
                let slot = self
                    .voices
                    .iter()
                    .position(Option::is_none)
                    .unwrap_or_else(|| {
                        self.voices
                            .iter()
                            .enumerate()
                            .filter_map(|(index, voice)| {
                                voice.map(|voice| (index, voice.age_frames))
                            })
                            .max_by_key(|(_, age)| *age)
                            .map(|(index, _)| index)
                            .unwrap_or(0)
                    });
                self.voices[slot] = Some(voice);
            }
            MidiInputMessage::NoteOn { .. } => {}
            MidiInputMessage::NoteOff { channel, key, .. } => {
                let release = (sample_rate.max(1) / 30).max(1);
                if let Some(voice) = self
                    .voices
                    .iter_mut()
                    .flatten()
                    .find(|voice| voice.channel == channel.min(15) && voice.key == key.min(127))
                {
                    voice.release_frames = Some(release);
                }
            }
            MidiInputMessage::ControlChange {
                channel,
                controller,
                value,
            } => {
                let channel = usize::from(channel.min(15));
                let controller = usize::from(controller.min(127));
                self.controller_values[channel][controller] = value.min(127);
                if matches!(controller, 120 | 123) {
                    let release = (sample_rate.max(1) / 30).max(1);
                    for voice in self.voices.iter_mut().flatten() {
                        if usize::from(voice.channel) == channel {
                            voice.release_frames = Some(release);
                        }
                    }
                }
            }
        }
    }

    fn next_frame(&mut self, sample_rate: u32) -> (f32, f32) {
        let sample_rate = sample_rate.max(1);
        let attack_frames = (sample_rate / 200).max(1) as f32;
        let release_total = (sample_rate / 30).max(1);
        let mut left = 0.0;
        let mut right = 0.0;
        for slot in &mut self.voices {
            let Some(voice) = slot.as_mut() else {
                continue;
            };
            let release_gain = match voice.release_frames {
                Some(remaining) if remaining == 0 => {
                    *slot = None;
                    continue;
                }
                Some(remaining) => {
                    voice.release_frames = Some(remaining - 1);
                    remaining as f32 / release_total as f32
                }
                None => 1.0,
            };
            let attack = (voice.age_frames as f32 / attack_frames).clamp(0.0, 1.0);
            let amplitude = oscillator_sample(self.waveform, voice.phase)
                * voice.velocity
                * MIDI_VOICE_LEVEL
                * attack
                * release_gain;
            let channel = self.controller_values[usize::from(voice.channel)];
            let volume = f32::from(channel[7]) / 127.0;
            let expression = f32::from(channel[11]) / 127.0;
            let pan_value = channel[10];
            let pan = if pan_value <= 64 {
                -(f32::from(64 - pan_value) / 64.0)
            } else {
                f32::from(pan_value - 64) / 63.0
            };
            let left_gain = if pan > 0.0 { 1.0 - pan } else { 1.0 };
            let right_gain = if pan < 0.0 { 1.0 + pan } else { 1.0 };
            let channel_sample = amplitude * volume * expression;
            left += channel_sample * left_gain;
            right += channel_sample * right_gain;
            voice.phase = (voice.phase
                + std::f32::consts::TAU * voice.frequency_hz / sample_rate as f32)
                .rem_euclid(std::f32::consts::TAU);
            voice.age_frames = voice.age_frames.saturating_add(1);
        }
        (left, right)
    }
}

fn instrument_waveform_code(waveform: InstrumentWaveform) -> u8 {
    match waveform {
        InstrumentWaveform::Sine => 0,
        InstrumentWaveform::Square => 1,
        InstrumentWaveform::Sawtooth => 2,
        InstrumentWaveform::Triangle => 3,
    }
}

fn instrument_waveform_from_code(code: u8) -> InstrumentWaveform {
    match code {
        1 => InstrumentWaveform::Square,
        2 => InstrumentWaveform::Sawtooth,
        3 => InstrumentWaveform::Triangle,
        _ => InstrumentWaveform::Sine,
    }
}

fn callback_state(
    tone: &Arc<AtomicBool>,
    playing: &Arc<AtomicBool>,
    playhead: &Arc<AtomicU64>,
    plan: &Arc<ArcSwap<PlaybackPlan>>,
    generation: &Arc<AtomicU64>,
    active: &Arc<AtomicBool>,
) -> (
    Arc<AtomicBool>,
    Arc<AtomicBool>,
    Arc<AtomicU64>,
    Arc<ArcSwap<PlaybackPlan>>,
    Arc<AtomicU64>,
    Arc<AtomicBool>,
) {
    (
        Arc::clone(tone),
        Arc::clone(playing),
        Arc::clone(playhead),
        Arc::clone(plan),
        Arc::clone(generation),
        Arc::clone(active),
    )
}

#[derive(Default)]
struct AudioMonitorReader {
    buffer_id: Option<u64>,
    monitor_epoch: u64,
    cursor: Option<f64>,
    rate_correction: f64,
    was_enabled: bool,
}

impl AudioMonitorReader {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn next_frame(&mut self, buffer: &AudioMonitorBuffer, output_rate: u32) -> (f32, f32) {
        if !buffer.is_enabled() {
            self.buffer_id = Some(buffer.id());
            self.cursor = None;
            self.was_enabled = false;
            return (0.0, 0.0);
        }

        let monitor_epoch = buffer.monitor_epoch();
        if self.buffer_id != Some(buffer.id()) || self.monitor_epoch != monitor_epoch {
            let start = buffer.monitor_start_frame();
            buffer.advance_read_to(start);
            self.cursor = Some(start as f64);
        } else if !self.was_enabled {
            let start = buffer.monitor_start_frame();
            buffer.advance_read_to(start);
            self.cursor = Some(start as f64);
        }
        self.buffer_id = Some(buffer.id());
        self.monitor_epoch = monitor_epoch;
        self.was_enabled = true;

        let mut cursor = self.cursor.unwrap_or_else(|| buffer.read_index() as f64);
        let read = buffer.read_index();
        if cursor < read as f64 {
            cursor = read as f64;
        }
        let first_frame = cursor.floor() as u64;
        let Some((first_left, first_right)) = buffer.sample_frame(first_frame) else {
            self.cursor = Some(cursor);
            return (0.0, 0.0);
        };
        let Some((second_left, second_right)) = buffer.sample_frame(first_frame.saturating_add(1))
        else {
            self.cursor = Some(cursor);
            return (0.0, 0.0);
        };
        let fraction = (cursor - first_frame as f64) as f32;
        let target_fill = 256.0_f64;
        let fill_error = (buffer.available_from(first_frame) as f64 - target_fill) / target_fill;
        let desired_correction = (1.0 + fill_error * 0.001).clamp(0.998, 1.002);
        if self.rate_correction == 0.0 {
            self.rate_correction = 1.0;
        }
        self.rate_correction += (desired_correction - self.rate_correction) * 0.0002;
        let input_step =
            f64::from(buffer.sample_rate()) / f64::from(output_rate.max(1)) * self.rate_correction;
        let next_cursor = cursor + input_step;
        self.cursor = Some(next_cursor);
        buffer.advance_read_to(next_cursor.floor() as u64);
        (
            first_left + (second_left - first_left) * fraction,
            first_right + (second_right - first_right) * fraction,
        )
    }
}

#[cfg(test)]
fn render_f32(
    samples: &mut [f32],
    channels: usize,
    sample_rate: u32,
    tone: &AtomicBool,
    playing: &AtomicBool,
    playhead: &AtomicU64,
    plan: &ArcSwap<PlaybackPlan>,
    callback_generation: &AtomicU64,
    callback_active: &AtomicBool,
    input_monitor: &Arc<ArcSwapOption<AudioMonitorBuffer>>,
    monitor_reader: &mut AudioMonitorReader,
    phase: &mut f32,
) {
    render_f32_with_midi(
        samples,
        channels,
        sample_rate,
        tone,
        playing,
        playhead,
        plan,
        callback_generation,
        callback_active,
        input_monitor,
        monitor_reader,
        phase,
        None,
    );
}

fn render_f32_with_midi(
    samples: &mut [f32],
    channels: usize,
    sample_rate: u32,
    tone: &AtomicBool,
    playing: &AtomicBool,
    playhead: &AtomicU64,
    plan: &ArcSwap<PlaybackPlan>,
    callback_generation: &AtomicU64,
    callback_active: &AtomicBool,
    input_monitor: &Arc<ArcSwapOption<AudioMonitorBuffer>>,
    monitor_reader: &mut AudioMonitorReader,
    phase: &mut f32,
    midi: Option<&mut LiveMidiRenderer>,
) {
    render_frames(
        samples,
        channels,
        sample_rate,
        tone,
        playing,
        playhead,
        plan,
        callback_generation,
        callback_active,
        input_monitor,
        monitor_reader,
        phase,
        |sample, channel, left, right| {
            *sample = match channel {
                0 if channels == 1 => (left + right) * 0.5,
                0 => left,
                1 => right,
                _ => 0.0,
            }
            .clamp(-1.0, 1.0);
        },
        midi,
    );
}

#[cfg(not(windows))]
fn render_i16(
    samples: &mut [i16],
    channels: usize,
    sample_rate: u32,
    tone: &AtomicBool,
    playing: &AtomicBool,
    playhead: &AtomicU64,
    plan: &ArcSwap<PlaybackPlan>,
    callback_generation: &AtomicU64,
    callback_active: &AtomicBool,
    input_monitor: &Arc<ArcSwapOption<AudioMonitorBuffer>>,
    monitor_reader: &mut AudioMonitorReader,
    phase: &mut f32,
    midi: Option<&mut LiveMidiRenderer>,
) {
    render_frames(
        samples,
        channels,
        sample_rate,
        tone,
        playing,
        playhead,
        plan,
        callback_generation,
        callback_active,
        input_monitor,
        monitor_reader,
        phase,
        |sample, channel, left, right| {
            let value = match channel {
                0 if channels == 1 => (left + right) * 0.5,
                0 => left,
                1 => right,
                _ => 0.0,
            }
            .clamp(-1.0, 1.0);
            *sample = (value * i16::MAX as f32) as i16;
        },
        midi,
    );
}

#[cfg(not(windows))]
fn render_u16(
    samples: &mut [u16],
    channels: usize,
    sample_rate: u32,
    tone: &AtomicBool,
    playing: &AtomicBool,
    playhead: &AtomicU64,
    plan: &ArcSwap<PlaybackPlan>,
    callback_generation: &AtomicU64,
    callback_active: &AtomicBool,
    input_monitor: &Arc<ArcSwapOption<AudioMonitorBuffer>>,
    monitor_reader: &mut AudioMonitorReader,
    phase: &mut f32,
    midi: Option<&mut LiveMidiRenderer>,
) {
    render_frames(
        samples,
        channels,
        sample_rate,
        tone,
        playing,
        playhead,
        plan,
        callback_generation,
        callback_active,
        input_monitor,
        monitor_reader,
        phase,
        |sample, channel, left, right| {
            let value = match channel {
                0 if channels == 1 => (left + right) * 0.5,
                0 => left,
                1 => right,
                _ => 0.0,
            }
            .clamp(-1.0, 1.0);
            *sample = ((value + 1.0) * 0.5 * u16::MAX as f32) as u16;
        },
        midi,
    );
}

fn render_frames<T>(
    samples: &mut [T],
    channels: usize,
    sample_rate: u32,
    tone: &AtomicBool,
    playing: &AtomicBool,
    playhead: &AtomicU64,
    plan: &ArcSwap<PlaybackPlan>,
    callback_generation: &AtomicU64,
    callback_active: &AtomicBool,
    input_monitor: &Arc<ArcSwapOption<AudioMonitorBuffer>>,
    monitor_reader: &mut AudioMonitorReader,
    phase: &mut f32,
    mut write_sample: impl FnMut(&mut T, usize, f32, f32),
    mut live_midi: Option<&mut LiveMidiRenderer>,
) {
    if callback_active
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        .is_err()
    {
        plan.load()
            .callback_metrics
            .overruns
            .fetch_add(1, Ordering::Relaxed);
        for frame in samples.chunks_mut(channels) {
            for (channel, sample) in frame.iter_mut().enumerate() {
                write_sample(sample, channel, 0.0, 0.0);
            }
        }
        return;
    }
    let callback_started_at = Instant::now();
    let plan_snapshot = plan.load_full();
    let monitor_snapshot = input_monitor.load_full();
    let audition_snapshot = plan_snapshot.note_audition.load_full();
    let generation = plan_snapshot.generation;
    let playing = playing.load(Ordering::Relaxed);
    let mut output_frame = wrap_playhead_frame(
        playhead.load(Ordering::Relaxed),
        plan_snapshot.loop_range_frames,
        plan_snapshot.cycle_mode,
    );
    let mut output_peaks = [0.0_f32; 2];
    if plan_snapshot.hosted_processing {
        render_hosted_blocks(
            samples,
            channels,
            sample_rate,
            tone,
            &plan_snapshot,
            monitor_snapshot.as_deref(),
            monitor_reader,
            audition_snapshot.as_deref(),
            playing,
            &mut output_frame,
            &mut output_peaks,
            phase,
            live_midi.as_deref_mut(),
            &mut write_sample,
        );
    } else {
        for frame in samples.chunks_mut(channels) {
            let (monitor_left, monitor_right) = if let Some(monitor) = monitor_snapshot.as_ref() {
                monitor_reader.next_frame(monitor, sample_rate)
            } else {
                monitor_reader.reset();
                (0.0, 0.0)
            };
            let click = plan_snapshot.metronome.sample_with_frequency(
                output_frame,
                sample_rate,
                plan_snapshot.tempo_bpm,
                playing,
                plan_snapshot.metronome_downbeat_frequency,
            );
            let mut external_input = (0.0, 0.0);
            if tone.load(Ordering::Relaxed) {
                let tone_sample = (*phase * std::f32::consts::TAU).sin() * TEST_TONE_LEVEL;
                external_input.0 += tone_sample;
                external_input.1 += tone_sample;
            }
            *phase += TEST_TONE_HZ / sample_rate as f32;
            if *phase >= 1.0 {
                *phase -= 1.0;
            }
            if let Some(midi) = live_midi.as_mut() {
                let (midi_left, midi_right) = midi.next_frame(sample_rate);
                external_input.0 += midi_left;
                external_input.1 += midi_right;
            }
            let (bus_left, bus_right) = render_plan_frame(
                &plan_snapshot,
                output_frame,
                sample_rate,
                (monitor_left, monitor_right),
                external_input,
                playing,
                &plan_snapshot.monitor_mute_mask,
                true,
            );
            // The metronome and note audition are direct monitor/output feeds. They
            // bypass Stereo Out channel processing and its meter while remaining
            // audible even when the transport is stopped.
            let (audition_left, audition_right) = audition_snapshot
                .as_ref()
                .map_or((0.0, 0.0), |audition| audition.next_frame(sample_rate));
            let left = bus_left + click + audition_left;
            let right = bus_right + click + audition_right;
            output_peaks[0] = output_peaks[0].max(bus_left.abs());
            output_peaks[1] = output_peaks[1].max(bus_right.abs());
            for (channel, sample) in frame.iter_mut().enumerate() {
                write_sample(sample, channel, left, right);
            }
            if playing {
                output_frame = wrap_playhead_frame(
                    output_frame.saturating_add(1),
                    plan_snapshot.loop_range_frames,
                    plan_snapshot.cycle_mode,
                );
            }
        }
    }
    if playing {
        playhead.store(output_frame, Ordering::Relaxed);
    }
    plan_snapshot.output_meter.publish(output_peaks);
    publish_track_meters(&plan_snapshot);
    let frame_count = samples.len() / channels;
    if frame_count > 0 && sample_rate > 0 {
        let budget_seconds = frame_count as f32 / sample_rate as f32;
        let load = callback_started_at.elapsed().as_secs_f32() / budget_seconds;
        let metrics = &plan_snapshot.callback_metrics;
        let previous = f32::from_bits(metrics.load_ema.load(Ordering::Relaxed));
        let smoothed = if previous.is_finite() {
            previous + (load - previous) * 0.1
        } else {
            load
        };
        metrics
            .load_ema
            .store(smoothed.to_bits(), Ordering::Relaxed);
        if load > 1.0 {
            metrics.overruns.fetch_add(1, Ordering::Relaxed);
        }
    }
    drop(plan_snapshot);
    callback_generation.store(generation, Ordering::Release);
    callback_active.store(false, Ordering::SeqCst);
}

/// Hosted block renderer used only when a plan contains hosted inserts.
///
/// Every callback is split into [`HOSTED_BLOCK_CAPACITY`] blocks. Each track is
/// rendered into its preallocated planar scratch, its built-in channel strip is
/// applied per sample, then enabled hosted inserts run once per block in order.
/// All buffers are plan-owned or stack arrays; failures fall back to the
/// post-built-in dry block and are reported through the plan's bounded atomics.
#[allow(clippy::too_many_arguments)]
fn render_hosted_blocks<T>(
    samples: &mut [T],
    channels: usize,
    sample_rate: u32,
    tone: &AtomicBool,
    plan: &PlaybackPlan,
    monitor: Option<&AudioMonitorBuffer>,
    monitor_reader: &mut AudioMonitorReader,
    audition: Option<&NoteAudition>,
    playing: bool,
    output_frame: &mut u64,
    output_peaks: &mut [f32; 2],
    phase: &mut f32,
    mut live_midi: Option<&mut LiveMidiRenderer>,
    write_sample: &mut impl FnMut(&mut T, usize, f32, f32),
) {
    let frame_count = samples.len() / channels;
    let mut monitor_left = [0.0_f32; HOSTED_BLOCK_CAPACITY];
    let mut monitor_right = [0.0_f32; HOSTED_BLOCK_CAPACITY];
    let mut frames = [0_u64; HOSTED_BLOCK_CAPACITY];
    let mut block_start = 0;
    while block_start < frame_count {
        let count = (frame_count - block_start).min(hosted_block_frames());
        for frame in frames[..count].iter_mut() {
            *frame = *output_frame;
            if playing {
                *output_frame = wrap_playhead_frame(
                    (*output_frame).saturating_add(1),
                    plan.loop_range_frames,
                    plan.cycle_mode,
                );
            }
        }
        if let Some(monitor) = monitor {
            for index in 0..count {
                let (left, right) = monitor_reader.next_frame(monitor, sample_rate);
                monitor_left[index] = left;
                monitor_right[index] = right;
            }
        } else {
            monitor_reader.reset();
            monitor_left[..count].fill(0.0);
            monitor_right[..count].fill(0.0);
        }

        for (track_index, track) in plan.tracks.iter().enumerate() {
            if track.settings.is_group {
                continue;
            }
            let inaudible = track.settings.muted
                || (plan.any_solo && !track.settings.solo && !track.settings.solo_defeat);
            if inaudible && !track.settings.meter_peak.spectrum_enabled() {
                continue;
            }
            let stage_started = track.hosted_timing.as_ref().map(|_| Instant::now());
            render_hosted_track_block(
                plan,
                track,
                track_index,
                &frames[..count],
                &monitor_left[..count],
                &monitor_right[..count],
                playing,
                sample_rate,
            );
            if let (Some(started), Some(trace)) = (stage_started, track.hosted_timing.as_ref()) {
                trace.record(started);
            }
        }

        let mut master_left = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let mut master_right = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        for &group_index in &plan.group_order {
            // SAFETY: the callback is the sole writer of these plan-owned buffers.
            let buffers = unsafe { &mut *plan.tracks[group_index].hosted_buffers.get() };
            buffers.group_left[..count].fill(0.0);
            buffers.group_right[..count].fill(0.0);
        }
        for track in &plan.tracks {
            if track.settings.is_group {
                continue;
            }
            let inaudible = track.settings.muted
                || (plan.any_solo && !track.settings.solo && !track.settings.solo_defeat);
            // SAFETY: callback_active admits exactly one writer of the block buffers.
            let buffers = unsafe { &mut *track.hosted_buffers.get() };
            if buffers.block_left.len() < count {
                continue;
            }
            for index in 0..count {
                let pre = (buffers.pre_left[index], buffers.pre_right[index]);
                let post = (buffers.block_left[index], buffers.block_right[index]);
                track
                    .settings
                    .meter_peak
                    .record_spectrum(pre, post, sample_rate);
                if inaudible {
                    continue;
                }
                let (track_left, track_right) = buffers.delay_frame(post.0, post.1);
                let (gain, pan) = track.mix.next();
                let output_left = track_left * gain * (1.0 - pan.max(0.0));
                let output_right = track_right * gain * (1.0 + pan.min(0.0));
                // SAFETY: the same serialized callback owns this accumulator.
                let accumulator = unsafe { &mut *track.meter_accumulator.get() };
                accumulator[0] = accumulator[0].max(output_left.abs());
                accumulator[1] = accumulator[1].max(output_right.abs());
                route_group_or_output(
                    plan,
                    track.settings.output_route,
                    output_left,
                    output_right,
                    index,
                    &mut master_left[index],
                    &mut master_right[index],
                );
            }
        }
        for &group_index in &plan.group_order {
            let track = &plan.tracks[group_index];
            let stage_started = track.hosted_timing.as_ref().map(|_| Instant::now());
            render_hosted_group_block(
                plan,
                group_index,
                count,
                sample_rate,
                &mut master_left,
                &mut master_right,
            );
            if let (Some(started), Some(trace)) = (stage_started, track.hosted_timing.as_ref()) {
                trace.record(started);
            }
        }

        let mut output_left = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let mut output_right = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let mut dry_output_left = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let mut dry_output_right = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let mut click_left = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let mut click_right = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let mut audition_left = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let mut audition_right = [0.0_f32; HOSTED_BLOCK_CAPACITY];
        let output_stage_started = plan.output_stage_timing.as_ref().map(|_| Instant::now());
        for index in 0..count {
            let output_frame = frames[index];
            let left = master_left[index];
            let right = master_right[index];

            let click = plan.metronome.sample_with_frequency(
                output_frame,
                sample_rate,
                plan.tempo_bpm,
                playing,
                plan.metronome_downbeat_frequency,
            );
            let mut external_input = (0.0, 0.0);
            if tone.load(Ordering::Relaxed) {
                let tone_sample = (*phase * std::f32::consts::TAU).sin() * TEST_TONE_LEVEL;
                external_input.0 += tone_sample;
                external_input.1 += tone_sample;
            }
            *phase += TEST_TONE_HZ / sample_rate as f32;
            if *phase >= 1.0 {
                *phase -= 1.0;
            }
            if let Some(midi) = live_midi.as_mut() {
                let (midi_left, midi_right) = midi.next_frame(sample_rate);
                external_input.0 += midi_left;
                external_input.1 += midi_right;
            }
            let (bus_left, bus_right) = finish_plan_frame(
                plan,
                output_frame,
                left,
                right,
                sample_rate,
                (monitor_left[index], monitor_right[index]),
                external_input,
                playing,
                &plan.monitor_mute_mask,
                true,
            );
            let (audition_l, audition_r) =
                audition.map_or((0.0, 0.0), |audition| audition.next_frame(sample_rate));
            output_left[index] = bus_left;
            output_right[index] = bus_right;
            dry_output_left[index] = bus_left;
            dry_output_right[index] = bus_right;
            click_left[index] = click;
            click_right[index] = click;
            audition_left[index] = audition_l;
            audition_right[index] = audition_r;
        }
        for (insert_index, insert) in plan.output_hosted_inserts.iter().enumerate() {
            if !insert.enabled {
                continue;
            }
            if let Err(error) = insert
                .processor
                .process(&mut output_left[..count], &mut output_right[..count])
            {
                plan.record_hosted_error(plan.tracks.len(), insert_index, &error);
                output_left[..count].copy_from_slice(&dry_output_left[..count]);
                output_right[..count].copy_from_slice(&dry_output_right[..count]);
                break;
            }
        }
        if let (Some(started), Some(trace)) =
            (output_stage_started, plan.output_stage_timing.as_ref())
        {
            trace.record(started);
        }
        for index in 0..count {
            output_peaks[0] = output_peaks[0].max(output_left[index].abs());
            output_peaks[1] = output_peaks[1].max(output_right[index].abs());
            let left = output_left[index] + click_left[index] + audition_left[index];
            let right = output_right[index] + click_right[index] + audition_right[index];
            let sample_start = (block_start + index) * channels;
            for (channel, sample) in samples[sample_start..sample_start + channels]
                .iter_mut()
                .enumerate()
            {
                write_sample(sample, channel, left, right);
            }
        }
        block_start += count;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_hosted_track_block(
    plan: &PlaybackPlan,
    track: &PlaybackTrackRuntime,
    track_index: usize,
    frames: &[u64],
    monitor_left: &[f32],
    monitor_right: &[f32],
    playing: bool,
    sample_rate: u32,
) {
    // SAFETY: callback_active admits exactly one renderer, which is the only
    // writer of this track's processor, block buffers, and delay ring.
    let buffers = unsafe { &mut *track.hosted_buffers.get() };
    if buffers.block_left.len() < frames.len() {
        return;
    }
    let processor = unsafe { &mut *track.processor.get() };
    let monitor_only = track.settings.monitor_input.is_some()
        || plan
            .monitor_mute_mask
            .get(track_index)
            .copied()
            .unwrap_or(false);
    for (index, &output_frame) in frames.iter().enumerate() {
        let timeline_beats =
            output_frame as f64 * plan.tempo_bpm / (f64::from(sample_rate.max(1)) * 60.0);
        let (mut track_left, mut track_right) = if playing && !monitor_only {
            mix_track_frame(plan, track, timeline_beats)
        } else {
            (0.0, 0.0)
        };
        if let Some(input) = track.settings.monitor_input {
            match input {
                PlaybackInputChannel::Stereo => {
                    track_left += monitor_left[index];
                    track_right += monitor_right[index];
                }
                PlaybackInputChannel::Left => {
                    track_left += monitor_left[index];
                    track_right += monitor_left[index];
                }
                PlaybackInputChannel::Right => {
                    track_left += monitor_right[index];
                    track_right += monitor_right[index];
                }
            }
        }
        let (instrument_left, instrument_right) = if playing && !monitor_only {
            processor.instrument.next_frame(output_frame)
        } else {
            (0.0, 0.0)
        };
        track_left += instrument_left;
        track_right += instrument_right;
        buffers.pre_left[index] = track_left;
        buffers.pre_right[index] = track_right;
    }

    for index in 0..frames.len() {
        let mut left = buffers.pre_left[index];
        let mut right = buffers.pre_right[index];
        processor.process(&mut left, &mut right);
        processor.publish_dynamic_updates(&track.settings.meter_peak);
        buffers.block_left[index] = left;
        buffers.block_right[index] = right;
        // The post-built-in signal is the dry fallback if a hosted insert fails.
        buffers.dry_left[index] = left;
        buffers.dry_right[index] = right;
    }

    let count = frames.len();
    for (insert_index, insert) in track.settings.hosted_inserts.iter().enumerate() {
        if !insert.enabled {
            continue;
        }
        if let Err(error) = insert.processor.process(
            &mut buffers.block_left[..count],
            &mut buffers.block_right[..count],
        ) {
            plan.record_hosted_error(track_index, insert_index, &error);
            buffers.block_left[..count].copy_from_slice(&buffers.dry_left[..count]);
            buffers.block_right[..count].copy_from_slice(&buffers.dry_right[..count]);
            break;
        }
    }
}

fn render_hosted_group_block(
    plan: &PlaybackPlan,
    group_index: usize,
    count: usize,
    sample_rate: u32,
    master_left: &mut [f32; HOSTED_BLOCK_CAPACITY],
    master_right: &mut [f32; HOSTED_BLOCK_CAPACITY],
) {
    let track = &plan.tracks[group_index];
    let inaudible = track.settings.muted
        || (plan.any_solo && !track.settings.solo && !track.settings.solo_defeat);
    // SAFETY: callback_active admits exactly one writer to this group's bus,
    // processor, and output buffers.
    let buffers = unsafe { &mut *track.hosted_buffers.get() };
    if buffers.block_left.len() < count {
        return;
    }
    let processor = unsafe { &mut *track.processor.get() };
    for index in 0..count {
        let (mut left, mut right) = (buffers.group_left[index], buffers.group_right[index]);
        buffers.group_left[index] = 0.0;
        buffers.group_right[index] = 0.0;
        if track.settings.audio_channels == 1 {
            let mono = (left + right) * 0.5;
            left = mono;
            right = mono;
        }
        buffers.pre_left[index] = left;
        buffers.pre_right[index] = right;
        processor.process(&mut left, &mut right);
        processor.publish_dynamic_updates(&track.settings.meter_peak);
        buffers.block_left[index] = left;
        buffers.block_right[index] = right;
        buffers.dry_left[index] = left;
        buffers.dry_right[index] = right;
    }
    for (insert_index, insert) in track.settings.hosted_inserts.iter().enumerate() {
        if !insert.enabled {
            continue;
        }
        if let Err(error) = insert.processor.process(
            &mut buffers.block_left[..count],
            &mut buffers.block_right[..count],
        ) {
            plan.record_hosted_error(group_index, insert_index, &error);
            buffers.block_left[..count].copy_from_slice(&buffers.dry_left[..count]);
            buffers.block_right[..count].copy_from_slice(&buffers.dry_right[..count]);
            break;
        }
    }
    for index in 0..count {
        let pre = (buffers.pre_left[index], buffers.pre_right[index]);
        let post = (buffers.block_left[index], buffers.block_right[index]);
        track
            .settings
            .meter_peak
            .record_spectrum(pre, post, sample_rate);
        if inaudible {
            continue;
        }
        let (left, right) = buffers.delay_frame(post.0, post.1);
        let (gain, pan) = track.mix.next();
        let output_left = left * gain * (1.0 - pan.max(0.0));
        let output_right = right * gain * (1.0 + pan.min(0.0));
        // SAFETY: this renderer is the only writer of the meter accumulator.
        let accumulator = unsafe { &mut *track.meter_accumulator.get() };
        accumulator[0] = accumulator[0].max(output_left.abs());
        accumulator[1] = accumulator[1].max(output_right.abs());
        route_group_or_output(
            plan,
            track.settings.output_route,
            output_left,
            output_right,
            index,
            &mut master_left[index],
            &mut master_right[index],
        );
    }
}

fn publish_track_meters(plan: &PlaybackPlan) {
    for track in &plan.tracks {
        // SAFETY: render_frames is the only caller and callback_active admits
        // only one callback that can mutate each accumulator.
        let peak = unsafe {
            let accumulator = &mut *track.meter_accumulator.get();
            let peak = *accumulator;
            *accumulator = [0.0; 2];
            peak
        };
        track.settings.meter_peak.publish(peak);
    }
}

fn wrap_playhead_frame(frame: u64, loop_range: Option<(u64, u64)>, mode: CycleMode) -> u64 {
    let Some((start_frame, end_frame)) = loop_range.filter(|(start, end)| end > start) else {
        return frame;
    };
    match mode {
        CycleMode::Loop if frame >= end_frame => {
            start_frame + (frame - end_frame) % (end_frame - start_frame)
        }
        CycleMode::Skip if frame >= start_frame && frame < end_frame => end_frame,
        _ => frame,
    }
}

#[cfg(test)]
fn mix_frame(
    plan: &PlaybackPlan,
    output_frame: u64,
    output_rate: u32,
    monitor_input: (f32, f32),
    playing: bool,
) -> (f32, f32) {
    mix_frame_with_policy(
        plan,
        output_frame,
        output_rate,
        monitor_input,
        playing,
        true,
        &[],
    )
}

fn mix_frame_with_policy(
    plan: &PlaybackPlan,
    output_frame: u64,
    output_rate: u32,
    monitor_input: (f32, f32),
    playing: bool,
    analyze: bool,
    monitor_mute_mask: &[bool],
) -> (f32, f32) {
    let timeline_beats = output_frame as f64 * plan.tempo_bpm / (f64::from(output_rate) * 60.0);
    let mut left = 0.0;
    let mut right = 0.0;
    for &group_index in &plan.group_order {
        // SAFETY: this renderer is serialized by callback_active.
        let buffers = unsafe { &mut *plan.tracks[group_index].hosted_buffers.get() };
        buffers.group_left[0] = 0.0;
        buffers.group_right[0] = 0.0;
    }
    for (track_index, track) in plan.tracks.iter().enumerate() {
        if track.settings.is_group {
            continue;
        }
        let inaudible = track.settings.muted
            || (plan.any_solo && !track.settings.solo && !track.settings.solo_defeat);
        if inaudible && !track.settings.meter_peak.spectrum_enabled() {
            continue;
        }
        let monitor_only = track.settings.monitor_input.is_some()
            || monitor_mute_mask.get(track_index).copied().unwrap_or(false);
        let (mut track_left, mut track_right) = if playing && !monitor_only {
            mix_track_frame(plan, track, timeline_beats)
        } else {
            (0.0, 0.0)
        };
        if let Some(input) = track.settings.monitor_input {
            match input {
                PlaybackInputChannel::Stereo => {
                    track_left += monitor_input.0;
                    track_right += monitor_input.1;
                }
                PlaybackInputChannel::Left => {
                    track_left += monitor_input.0;
                    track_right += monitor_input.0;
                }
                PlaybackInputChannel::Right => {
                    track_left += monitor_input.1;
                    track_right += monitor_input.1;
                }
            }
        }
        // SAFETY: callback_active permits exactly one callback to mutate the
        // processor state; the UI never accesses this cell.
        let processor = unsafe { &mut *track.processor.get() };
        let (instrument_left, instrument_right) = if playing && !monitor_only {
            processor.instrument.next_frame(output_frame)
        } else {
            (0.0, 0.0)
        };
        track_left += instrument_left;
        track_right += instrument_right;
        let pre = (track_left, track_right);
        processor.process(&mut track_left, &mut track_right);
        processor.publish_dynamic_updates(&track.settings.meter_peak);
        if analyze {
            track
                .settings
                .meter_peak
                .record_spectrum(pre, (track_left, track_right), output_rate);
        }
        if inaudible {
            continue;
        }
        // Linear stereo balance preserves unity at center, with the opposite
        // side attenuated toward either edge. Exact reference pan law is unmeasured.
        let (gain, pan) = track.mix.next();
        let output_left = track_left * gain * (1.0 - pan.max(0.0));
        let output_right = track_right * gain * (1.0 + pan.min(0.0));
        // SAFETY: mix_frame only runs in the serialized audio callback.
        let accumulator = unsafe { &mut *track.meter_accumulator.get() };
        accumulator[0] = accumulator[0].max(output_left.abs());
        accumulator[1] = accumulator[1].max(output_right.abs());
        route_group_or_output(
            plan,
            track.settings.output_route,
            output_left,
            output_right,
            0,
            &mut left,
            &mut right,
        );
    }
    for &group_index in &plan.group_order {
        let track = &plan.tracks[group_index];
        let inaudible = track.settings.muted
            || (plan.any_solo && !track.settings.solo && !track.settings.solo_defeat);
        // SAFETY: the group buffer and processor are owned by this callback.
        let buffers = unsafe { &mut *track.hosted_buffers.get() };
        let (mut track_left, mut track_right) = (buffers.group_left[0], buffers.group_right[0]);
        buffers.group_left[0] = 0.0;
        buffers.group_right[0] = 0.0;
        if track.settings.audio_channels == 1 {
            let mono = (track_left + track_right) * 0.5;
            track_left = mono;
            track_right = mono;
        }
        // SAFETY: callback_active permits exactly one writer of this processor.
        let processor = unsafe { &mut *track.processor.get() };
        let pre = (track_left, track_right);
        processor.process(&mut track_left, &mut track_right);
        processor.publish_dynamic_updates(&track.settings.meter_peak);
        if analyze {
            track
                .settings
                .meter_peak
                .record_spectrum(pre, (track_left, track_right), output_rate);
        }
        if inaudible {
            continue;
        }
        let (gain, pan) = track.mix.next();
        let output_left = track_left * gain * (1.0 - pan.max(0.0));
        let output_right = track_right * gain * (1.0 + pan.min(0.0));
        // SAFETY: mix_frame only runs in the serialized audio callback.
        let accumulator = unsafe { &mut *track.meter_accumulator.get() };
        accumulator[0] = accumulator[0].max(output_left.abs());
        accumulator[1] = accumulator[1].max(output_right.abs());
        route_group_or_output(
            plan,
            track.settings.output_route,
            output_left,
            output_right,
            0,
            &mut left,
            &mut right,
        );
    }
    (left, right)
}

fn route_group_or_output(
    plan: &PlaybackPlan,
    route: PlaybackOutputRoute,
    left_sample: f32,
    right_sample: f32,
    frame: usize,
    left: &mut f32,
    right: &mut f32,
) {
    match route {
        PlaybackOutputRoute::Stereo => {
            *left += left_sample;
            *right += right_sample;
        }
        PlaybackOutputRoute::Left => *left += (left_sample + right_sample) * 0.5,
        PlaybackOutputRoute::Right => *right += (left_sample + right_sample) * 0.5,
        PlaybackOutputRoute::Disconnected => {}
        PlaybackOutputRoute::Group(index) => {
            let Some(group) = plan.tracks.get(index) else {
                return;
            };
            // SAFETY: the serialized audio renderer is the only writer of the
            // preallocated group accumulator for this frame.
            let buffers = unsafe { &mut *group.hosted_buffers.get() };
            if frame < buffers.group_left.len() {
                buffers.group_left[frame] += left_sample;
                buffers.group_right[frame] += right_sample;
            }
        }
    }
}

fn render_plan_frame(
    plan: &PlaybackPlan,
    output_frame: u64,
    output_rate: u32,
    monitor_input: (f32, f32),
    external_input: (f32, f32),
    playing: bool,
    monitor_mute_mask: &[bool],
    analyze: bool,
) -> (f32, f32) {
    let (left, right) = mix_frame_with_policy(
        plan,
        output_frame,
        output_rate,
        monitor_input,
        playing,
        analyze,
        monitor_mute_mask,
    );
    finish_plan_frame(
        plan,
        output_frame,
        left,
        right,
        output_rate,
        monitor_input,
        external_input,
        playing,
        monitor_mute_mask,
        analyze,
    )
}

/// Applies external input, the output bus, and any parameter crossfade to an
/// already mixed stereo frame. The hosted block path uses this after summing
/// its block-rendered tracks, so output-bus processing and metering stay
/// identical to the legacy per-sample path.
#[allow(clippy::too_many_arguments)]
fn finish_plan_frame(
    plan: &PlaybackPlan,
    output_frame: u64,
    mut left: f32,
    mut right: f32,
    output_rate: u32,
    monitor_input: (f32, f32),
    external_input: (f32, f32),
    playing: bool,
    monitor_mute_mask: &[bool],
    analyze: bool,
) -> (f32, f32) {
    left += external_input.0;
    right += external_input.1;
    if analyze {
        (left, right) =
            plan.output_bus
                .process_analyzed(left, right, &plan.output_meter, output_rate);
    } else {
        (left, right) = plan.output_bus.process_unanalyzed(left, right);
    }

    let total = plan.transition_frames;
    if total == 0 || plan.transition_position.load(Ordering::Relaxed) >= total {
        return (left, right);
    }
    let previous = plan.transition_from.load();
    let Some(previous) = previous.as_ref() else {
        return (left, right);
    };
    let position = plan
        .transition_position
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            Some(value.saturating_add(1))
        })
        .unwrap_or_else(|value| value);
    if position >= total {
        return (left, right);
    }
    let (old_left, old_right) = render_plan_frame(
        previous,
        output_frame,
        output_rate,
        monitor_input,
        external_input,
        playing,
        monitor_mute_mask,
        false,
    );
    let blend = (position + 1) as f32 / total.max(1) as f32;
    (
        old_left * (1.0 - blend) + left * blend,
        old_right * (1.0 - blend) + right * blend,
    )
}

/// Linear event fades; the source media is never rewritten.
pub fn event_envelope(time: f64, duration: f64, fade_in: f64, fade_out: f64) -> f32 {
    if time < 0.0 || time >= duration {
        return 0.0;
    }
    let up = if fade_in > 0.0 {
        (time / fade_in).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let down = if fade_out > 0.0 {
        ((duration - time) / fade_out).clamp(0.0, 1.0)
    } else {
        1.0
    };
    (up * down) as f32
}

fn mix_track_frame(
    plan: &PlaybackPlan,
    track: &PlaybackTrackRuntime,
    timeline_beats: f64,
) -> (f32, f32) {
    let Some(top_index) =
        track.settings.clips.iter().rposition(|clip| {
            !clip.muted && clip_event_seconds(plan, clip, timeline_beats).is_some()
        })
    else {
        return (0.0, 0.0);
    };
    let top = &track.settings.clips[top_index];
    // Retain the intentional X crossfade pair; all other overlaps use the
    // newest event as the audible take layer.
    let crossfade_index = track.settings.clips[..top_index]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, clip)| {
            let overlaps =
                clip.start_beats + clip.duration_seconds * plan.tempo_bpm / 60.0 > top.start_beats;
            (overlaps && clip.fade_out_seconds > 0.0 && top.fade_in_seconds > 0.0).then_some(index)
        });
    let mut left = 0.0;
    let mut right = 0.0;
    for (index, clip) in track.settings.clips.iter().enumerate() {
        if index != top_index && Some(index) != crossfade_index {
            continue;
        }
        let Some(event_seconds) = clip_event_seconds(plan, clip, timeline_beats) else {
            continue;
        };
        let source_frame =
            (event_seconds + clip.source_offset_seconds) * f64::from(clip.source.sample_rate());
        let source_frames = clip.source.frame_count();
        if source_frame >= source_frames as f64 {
            continue;
        }
        let first = source_frame as usize;
        let second = (first + 1).min(source_frames as usize - 1);
        let fraction = (source_frame - first as f64) as f32;
        let Some((first_left, first_right)) = clip.source.sample_frame(first as u64) else {
            continue;
        };
        let Some((second_left, second_right)) = clip.source.sample_frame(second as u64) else {
            continue;
        };
        let envelope = event_envelope(
            event_seconds,
            clip.duration_seconds,
            clip.fade_in_seconds,
            clip.fade_out_seconds,
        ) * clip.gain
            * clip
                .source_fades
                .iter()
                .map(|f| f.gain_at(event_seconds + clip.source_offset_seconds))
                .product::<f32>();
        left += (first_left * (1.0 - fraction) + second_left * fraction) * envelope;
        right += (first_right * (1.0 - fraction) + second_right * fraction) * envelope;
    }
    (left, right)
}

fn clip_event_seconds(
    plan: &PlaybackPlan,
    clip: &PlaybackClip,
    timeline_beats: f64,
) -> Option<f64> {
    let seconds = (timeline_beats - clip.start_beats) * 60.0 / plan.tempo_bpm;
    (seconds >= 0.0 && seconds < clip.duration_seconds).then_some(seconds)
}

const MAX_ACTIVE_MIDI_VOICES: usize = 64;
const MAX_MIDI_NOTE_EVENTS_PER_FRAME: usize = 64;
const MIDI_CONTROLLER_CHECKPOINT_INTERVAL: usize = 256;
const MIDI_ENVELOPE_FRAMES: u64 = 240;
const MIDI_VOICE_LEVEL: f32 = 0.12;

#[derive(Clone, Copy)]
struct ScheduledMidiNote {
    start_frame: u64,
    end_frame: u64,
    frequency_hz: f32,
    channel: u8,
    velocity_index: usize,
}

#[derive(Clone, Copy)]
struct ActiveMidiVoice {
    start_frame: u64,
    end_frame: u64,
    frequency_hz: f32,
    channel: u8,
    velocity: f32,
    velocity_index: usize,
    phase: f32,
}

#[derive(Clone, Copy)]
struct ScheduledMidiController {
    frame: u64,
    channel: u8,
    controller: u8,
    value: u8,
}

struct MidiControllerCheckpoint {
    frame: u64,
    next_event: usize,
    values: [[u8; 128]; 16],
}

struct MidiVoice {
    notes: Vec<ScheduledMidiNote>,
    next_note: usize,
    controllers: Vec<ScheduledMidiController>,
    controller_checkpoints: Vec<MidiControllerCheckpoint>,
    next_controller: usize,
    controller_values: [[u8; 128]; 16],
    active: [Option<ActiveMidiVoice>; MAX_ACTIVE_MIDI_VOICES],
    last_frame: Option<u64>,
    sample_rate: f32,
    velocity_smoothing: f32,
    waveform: InstrumentWaveform,
    controls: Arc<MidiEventControls>,
}

impl MidiVoice {
    fn new(
        notes: &[PlaybackMidiNote],
        controllers: &[PlaybackMidiController],
        controls: Arc<MidiEventControls>,
        tempo_bpm: f64,
        sample_rate: u32,
        waveform: InstrumentWaveform,
    ) -> Self {
        let mut scheduled = Vec::with_capacity(notes.len());
        let sample_rate_f64 = f64::from(sample_rate.max(1));
        for note in notes {
            if !note.start_beats.is_finite()
                || note.start_beats < 0.0
                || !note.duration_beats.is_finite()
                || note.duration_beats <= 0.0
                || !note.frequency_hz.is_finite()
                || note.frequency_hz <= 0.0
                || note.channel > 15
                || !note.velocity.is_finite()
            {
                continue;
            }
            let start_seconds = note.start_beats * 60.0 / tempo_bpm;
            let end_seconds = (note.start_beats + note.duration_beats) * 60.0 / tempo_bpm;
            if !start_seconds.is_finite() || !end_seconds.is_finite() {
                continue;
            }
            let start_frame = (start_seconds * sample_rate_f64).ceil() as u64;
            let end_frame = (end_seconds * sample_rate_f64).ceil() as u64;
            if end_frame > start_frame {
                scheduled.push(ScheduledMidiNote {
                    start_frame,
                    end_frame,
                    frequency_hz: note.frequency_hz,
                    channel: note.channel,
                    velocity_index: controls
                        .note_velocity_indices
                        .get(&note.id)
                        .copied()
                        .unwrap_or(usize::MAX),
                });
            }
        }
        scheduled.sort_by_key(|note| note.start_frame);

        let mut scheduled_controllers = Vec::with_capacity(controllers.len());
        for event in controllers {
            if event.channel > 15
                || event.controller > 127
                || event.value > 127
                || !event.position_beats.is_finite()
                || event.position_beats < 0.0
            {
                continue;
            }
            let event_seconds = event.position_beats * 60.0 / tempo_bpm;
            if !event_seconds.is_finite() {
                continue;
            }
            scheduled_controllers.push(ScheduledMidiController {
                frame: (event_seconds * sample_rate_f64).ceil() as u64,
                channel: event.channel,
                controller: event.controller,
                value: event.value,
            });
        }
        scheduled_controllers.sort_by_key(|event| (event.frame, event.channel, event.controller));
        let mut coalesced_controllers: Vec<ScheduledMidiController> =
            Vec::with_capacity(scheduled_controllers.len());
        for event in scheduled_controllers {
            if let Some(previous) = coalesced_controllers.last_mut()
                && previous.frame == event.frame
                && previous.channel == event.channel
                && previous.controller == event.controller
            {
                *previous = event;
            } else {
                coalesced_controllers.push(event);
            }
        }
        let mut controller_values = initial_midi_controller_values();
        let mut controller_checkpoints = Vec::new();
        for (index, event) in coalesced_controllers.iter().enumerate() {
            controller_values[usize::from(event.channel)][usize::from(event.controller)] =
                event.value;
            if (index + 1) % MIDI_CONTROLLER_CHECKPOINT_INTERVAL == 0 {
                controller_checkpoints.push(MidiControllerCheckpoint {
                    frame: event.frame,
                    next_event: index + 1,
                    values: controller_values,
                });
            }
        }
        Self {
            notes: scheduled,
            next_note: 0,
            controllers: coalesced_controllers,
            controller_checkpoints,
            next_controller: 0,
            controller_values: initial_midi_controller_values(),
            active: std::array::from_fn(|_| None),
            last_frame: None,
            sample_rate: sample_rate_f64 as f32,
            velocity_smoothing: 1.0 - (-1.0 / (sample_rate.max(1) as f32 * 0.005)).exp(),
            waveform,
            controls,
        }
    }

    #[cfg(test)]
    fn next_sample(&mut self, frame: u64) -> f32 {
        let (left, right) = self.next_stereo_sample(frame);
        (left + right) * 0.5
    }

    fn next_stereo_sample(&mut self, frame: u64) -> (f32, f32) {
        let discontinuity = match self.last_frame {
            Some(last_frame) => frame != last_frame.saturating_add(1),
            None => frame > 0,
        };
        if discontinuity {
            self.next_note = 0;
            self.active.fill(None);
            self.next_note = self.notes.partition_point(|note| note.start_frame < frame);
            self.restore_controller_state(frame);
        }
        self.last_frame = Some(frame);

        let mut applied_this_frame = 0;
        while self
            .controllers
            .get(self.next_controller)
            .is_some_and(|event| event.frame <= frame)
            && applied_this_frame < 128
        {
            let event = self.controllers[self.next_controller];
            self.controller_values[usize::from(event.channel)][usize::from(event.controller)] =
                event.value;
            self.next_controller += 1;
            applied_this_frame += 1;
        }

        let mut started_this_frame = 0;
        while self
            .notes
            .get(self.next_note)
            .is_some_and(|note| note.start_frame <= frame)
            && started_this_frame < MAX_MIDI_NOTE_EVENTS_PER_FRAME
        {
            let note = self.notes[self.next_note];
            self.next_note += 1;
            started_this_frame += 1;
            if note.end_frame <= frame {
                continue;
            }
            let elapsed_frames = frame - note.start_frame;
            let phase_cycles = (elapsed_frames as f64 * f64::from(note.frequency_hz)
                / f64::from(self.sample_rate))
            .rem_euclid(1.0);
            let voice = ActiveMidiVoice {
                start_frame: note.start_frame,
                end_frame: note.end_frame,
                frequency_hz: note.frequency_hz,
                channel: note.channel,
                velocity: self.controls.note_velocity(note.velocity_index),
                velocity_index: note.velocity_index,
                phase: phase_cycles as f32 * std::f32::consts::TAU,
            };
            let free_voice = self.active.iter().position(Option::is_none);
            let voice_index = free_voice.unwrap_or_else(|| {
                self.active
                    .iter()
                    .enumerate()
                    .filter_map(|(index, voice)| voice.map(|voice| (index, voice.end_frame)))
                    .min_by_key(|(_, end_frame)| *end_frame)
                    .map(|(index, _)| index)
                    .unwrap_or(0)
            });
            self.active[voice_index] = Some(voice);
        }

        let mut left = 0.0;
        let mut right = 0.0;
        for slot in &mut self.active {
            let Some(voice) = slot.as_mut() else {
                continue;
            };
            if frame >= voice.end_frame {
                *slot = None;
                continue;
            }
            let elapsed_frames = frame - voice.start_frame;
            let remaining_frames = voice.end_frame - frame;
            let attack = (elapsed_frames as f32 / MIDI_ENVELOPE_FRAMES as f32).clamp(0.0, 1.0);
            let release = (remaining_frames as f32 / MIDI_ENVELOPE_FRAMES as f32).clamp(0.0, 1.0);
            let controller_values = self.controller_values[usize::from(voice.channel)];
            let velocity_target = self.controls.note_velocity(voice.velocity_index);
            voice.velocity += (velocity_target - voice.velocity) * self.velocity_smoothing;
            let voice_sample = oscillator_sample(self.waveform, voice.phase)
                * voice.velocity
                * MIDI_VOICE_LEVEL
                * attack.min(release)
                * (f32::from(controller_values[7]) / 127.0)
                * (f32::from(controller_values[11]) / 127.0);
            let pan_value = controller_values[10];
            let pan = if pan_value <= 64 {
                -(f32::from(64 - pan_value) / 64.0)
            } else {
                f32::from(pan_value - 64) / 63.0
            };
            let left_gain = if pan > 0.0 { 1.0 - pan } else { 1.0 };
            let right_gain = if pan < 0.0 { 1.0 + pan } else { 1.0 };
            left += voice_sample * left_gain;
            right += voice_sample * right_gain;
            let modulation = f32::from(controller_values[1]) / 127.0;
            let modulated_frequency = if modulation > 0.0 {
                let lfo_phase = (std::f64::consts::TAU * 5.0 * frame as f64
                    / f64::from(self.sample_rate))
                .rem_euclid(std::f64::consts::TAU) as f32;
                voice.frequency_hz * (1.0 + modulation * 0.01 * lfo_phase.sin())
            } else {
                voice.frequency_hz
            };
            voice.phase = (voice.phase
                + std::f32::consts::TAU * modulated_frequency / self.sample_rate)
                .rem_euclid(std::f32::consts::TAU);
        }
        (left, right)
    }

    fn restore_controller_state(&mut self, frame: u64) {
        if let Some(checkpoint) = self
            .controller_checkpoints
            .get(
                self.controller_checkpoints
                    .partition_point(|checkpoint| checkpoint.frame < frame)
                    .saturating_sub(1),
            )
            .filter(|checkpoint| checkpoint.frame < frame)
        {
            self.controller_values = checkpoint.values;
            self.next_controller = checkpoint.next_event;
        } else {
            self.controller_values = initial_midi_controller_values();
            self.next_controller = 0;
        }
        while self
            .controllers
            .get(self.next_controller)
            .is_some_and(|event| event.frame < frame)
        {
            let event = self.controllers[self.next_controller];
            self.controller_values[usize::from(event.channel)][usize::from(event.controller)] =
                event.value;
            self.next_controller += 1;
        }
    }
}

fn initial_midi_controller_values() -> [[u8; 128]; 16] {
    let mut values = [[0; 128]; 16];
    for channel in &mut values {
        channel[7] = 127;
        channel[10] = 64;
        channel[11] = 127;
    }
    values
}

impl InstrumentProcessor for MidiVoice {
    fn next_frame(&mut self, output_frame: u64) -> (f32, f32) {
        self.next_stereo_sample(output_frame)
    }
}

fn oscillator_sample(waveform: InstrumentWaveform, phase: f32) -> f32 {
    let cycle = (phase / std::f32::consts::TAU).rem_euclid(1.0);
    match waveform {
        InstrumentWaveform::Sine => phase.sin(),
        InstrumentWaveform::Square => {
            if cycle < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        InstrumentWaveform::Sawtooth => cycle * 2.0 - 1.0,
        InstrumentWaveform::Triangle => 1.0 - 4.0 * (cycle - 0.5).abs(),
    }
}

struct ChannelProcessor {
    equalizer_bypassed: bool,
    equalizer_wet: f32,
    equalizer_wet_target: f32,
    equalizer_wet_step: f32,
    equalizer_control: Arc<EqualizerControl>,
    equalizer_revision: u64,
    sample_rate: u32,
    inserts_bypassed: bool,
    equalizer: [Biquad; opentrack_core::project::MAX_EQ_BANDS],
    equalizer_second: [Biquad; opentrack_core::project::MAX_EQ_BANDS],
    solo_filter: Biquad,
    solo_band: u32,
    solo_mix: f32,
    solo_mix_target: f32,
    solo_mix_step: f32,
    eq_bands: [EqualizerBand; opentrack_core::project::MAX_EQ_BANDS],
    dynamic_bands: [opentrack_core::project::DynamicEqBand; opentrack_core::project::MAX_EQ_BANDS],
    dynamic_detectors: [DynamicEqDetector; opentrack_core::project::MAX_EQ_BANDS],
    dynamic_gain_db: [f32; opentrack_core::project::MAX_EQ_BANDS],
    dynamic_meter_dirty: u32,
    inserts: Vec<InsertProcessor>,
    instrument: Box<dyn InstrumentProcessor>,
}

impl ChannelProcessor {
    fn new(
        equalizer: &ChannelEqualizer,
        equalizer_control: Arc<EqualizerControl>,
        inserts: &[InsertEffect],
        inserts_bypassed: bool,
        midi_notes: &[PlaybackMidiNote],
        midi_controllers: &[PlaybackMidiController],
        controls: Arc<MidiEventControls>,
        instrument_factory: &dyn InstrumentFactory,
        tempo_bpm: f64,
        sample_rate: u32,
    ) -> Self {
        let bands = std::array::from_fn(|index| {
            Biquad::from_band(equalizer.band(index).unwrap(), sample_rate)
        });
        let second_bands = std::array::from_fn(|index| {
            let band = equalizer.band(index).unwrap();
            let second = equalizer_band_sections(band).1.unwrap_or_else(|| {
                let mut disabled = *band;
                disabled.enabled = false;
                disabled
            });
            Biquad::from_band(&second, sample_rate)
        });
        let eq_bands = std::array::from_fn(|index| *equalizer.band(index).unwrap());
        let solo_filter = Biquad::from_bandpass(&eq_bands[0], sample_rate);
        let dynamic_bands = std::array::from_fn(|index| *equalizer.dynamic_band(index).unwrap());
        let dynamic_detectors = std::array::from_fn(|index| {
            DynamicEqDetector::new(&eq_bands[index], dynamic_bands[index], sample_rate)
        });
        let inserts = inserts
            .iter()
            .map(|insert| InsertProcessor::new(insert, sample_rate))
            .collect();
        Self {
            equalizer_bypassed: equalizer.bypassed,
            equalizer_wet: if equalizer.bypassed { 0.0 } else { 1.0 },
            equalizer_wet_target: if equalizer.bypassed { 0.0 } else { 1.0 },
            equalizer_wet_step: 0.0,
            equalizer_control,
            equalizer_revision: 0,
            sample_rate: sample_rate.max(1),
            inserts_bypassed,
            equalizer: bands,
            equalizer_second: second_bands,
            solo_filter,
            solo_band: 0,
            solo_mix: 0.0,
            solo_mix_target: 0.0,
            solo_mix_step: 0.0,
            eq_bands,
            dynamic_bands,
            dynamic_detectors,
            dynamic_gain_db: [0.0; opentrack_core::project::MAX_EQ_BANDS],
            dynamic_meter_dirty: 0,
            inserts,
            instrument: instrument_factory.create_processor(
                midi_notes,
                midi_controllers,
                controls,
                tempo_bpm,
                sample_rate,
            ),
        }
    }

    fn process(&mut self, left: &mut f32, right: &mut f32) {
        let revision = self.equalizer_control.revision.load(Ordering::Acquire);
        if revision != self.equalizer_revision {
            let target = self.equalizer_control.target.load_full();
            self.equalizer_bypassed = target.bypassed;
            self.equalizer_wet_target = if target.bypassed { 0.0 } else { 1.0 };
            self.equalizer_wet_step = (self.equalizer_wet_target - self.equalizer_wet)
                / (self.sample_rate / 200).max(1) as f32;
            for (index, band) in target.all_bands().enumerate() {
                let filter = &mut self.equalizer[index];
                filter.update_band(band, self.sample_rate);
                let second = equalizer_band_sections(band).1.unwrap_or_else(|| {
                    let mut disabled = *band;
                    disabled.enabled = false;
                    disabled
                });
                self.equalizer_second[index].update_band(&second, self.sample_rate);
            }
            for (index, band) in target.all_bands().enumerate() {
                let target_dynamic = *target.dynamic_band(index).unwrap();
                let was_dynamic = self.dynamic_bands[index].enabled
                    && self.eq_bands[index].enabled
                    && self.eq_bands[index].filter.supports_dynamic_eq();
                let is_dynamic =
                    target_dynamic.enabled && band.enabled && band.filter.supports_dynamic_eq();
                self.eq_bands[index] = *band;
                self.dynamic_detectors[index].configure(band, target_dynamic, self.sample_rate);
                if was_dynamic && !is_dynamic {
                    self.dynamic_detectors[index].reset();
                    self.dynamic_gain_db[index] = 0.0;
                    self.dynamic_meter_dirty |= 1u32 << index;
                }
            }
            if self.solo_band > 0 {
                self.solo_filter.update_bandpass(
                    &self.eq_bands[(self.solo_band - 1) as usize],
                    self.sample_rate,
                );
            }
            self.dynamic_bands = std::array::from_fn(|index| *target.dynamic_band(index).unwrap());
            self.equalizer_revision = revision;
        }
        let dry = (*left, *right);
        let requested_solo = self.equalizer_control.solo_band.load(Ordering::Acquire);
        if requested_solo != self.solo_band {
            let was_inactive = self.solo_band == 0 && requested_solo > 0;
            self.solo_band = requested_solo.min(opentrack_core::project::MAX_EQ_BANDS as u32);
            if self.solo_band > 0 {
                if was_inactive {
                    self.solo_filter.reset_state();
                }
                self.solo_filter.update_bandpass(
                    &self.eq_bands[(self.solo_band - 1) as usize],
                    self.sample_rate,
                );
            }
            self.solo_mix_target = if self.solo_band > 0 { 1.0 } else { 0.0 };
            self.solo_mix_step =
                (self.solo_mix_target - self.solo_mix) / (self.sample_rate / 200).max(1) as f32;
        }
        let mut solo_output = dry;
        if self.solo_band > 0 || self.solo_mix > 0.0 {
            self.solo_filter
                .process(&mut solo_output.0, &mut solo_output.1);
        }
        for index in 0..self.equalizer.len() {
            if self.dynamic_bands[index].enabled
                && self.eq_bands[index].enabled
                && self.eq_bands[index].filter.supports_dynamic_eq()
            {
                let dynamic_gain = self.dynamic_detectors[index].process(*left, *right);
                if self.dynamic_detectors[index].take_coefficients_due() {
                    let mut band = self.eq_bands[index];
                    band.gain_db = (band.gain_db + dynamic_gain).clamp(-24.0, 24.0);
                    self.equalizer[index].update_band(&band, self.sample_rate);
                    self.dynamic_gain_db[index] = dynamic_gain;
                    self.dynamic_meter_dirty |= 1u32 << index;
                }
            }
            if self.equalizer[index].is_processing() {
                self.equalizer[index].process(left, right);
            }
            if self.equalizer_second[index].is_processing() {
                self.equalizer_second[index].process(left, right);
            }
        }
        if self.equalizer_wet != self.equalizer_wet_target {
            self.equalizer_wet += self.equalizer_wet_step;
            if (self.equalizer_wet - self.equalizer_wet_target).abs() < 1e-4 {
                self.equalizer_wet = self.equalizer_wet_target;
            }
        }
        *left = dry.0 * (1.0 - self.equalizer_wet) + *left * self.equalizer_wet;
        *right = dry.1 * (1.0 - self.equalizer_wet) + *right * self.equalizer_wet;
        if self.solo_mix != self.solo_mix_target {
            self.solo_mix += self.solo_mix_step;
            if (self.solo_mix - self.solo_mix_target).abs() < 1e-4 {
                self.solo_mix = self.solo_mix_target;
            }
        }
        *left = *left * (1.0 - self.solo_mix) + solo_output.0 * self.solo_mix;
        *right = *right * (1.0 - self.solo_mix) + solo_output.1 * self.solo_mix;
        if !self.inserts_bypassed {
            for insert in &mut self.inserts {
                if insert.enabled() {
                    insert.process(left, right);
                }
            }
        }
    }

    fn publish_dynamic_updates(&mut self, meter: &StereoMeter) {
        let dirty = std::mem::replace(&mut self.dynamic_meter_dirty, 0);
        for band in 0..opentrack_core::project::MAX_EQ_BANDS {
            if dirty & (1 << band) != 0 {
                meter.publish_dynamic_eq_gain(band, self.dynamic_gain_db[band]);
            }
        }
    }
}

/// Magnitude of OpenTrack's actual channel EQ filters, for the channel editor curve.
pub fn channel_equalizer_response_db(
    equalizer: &opentrack_core::project::ChannelEqualizer,
    frequency_hz: f32,
    sample_rate: u32,
) -> f32 {
    if equalizer.bypassed {
        return 0.0;
    }
    equalizer
        .all_bands()
        .filter(|band| band.enabled)
        .map(|band| equalizer_band_response_db(band, frequency_hz, sample_rate))
        .sum()
}

/// Magnitude of one actual channel EQ filter, using its configured Q and shape.
pub fn equalizer_band_response_db(
    band: &EqualizerBand,
    frequency_hz: f32,
    sample_rate: u32,
) -> f32 {
    let (first, second) = equalizer_band_sections(band);
    let response = single_equalizer_band_response_db(&first, frequency_hz, sample_rate);
    second.map_or(response, |second| {
        response + single_equalizer_band_response_db(&second, frequency_hz, sample_rate)
    })
}

fn equalizer_band_sections(band: &EqualizerBand) -> (EqualizerBand, Option<EqualizerBand>) {
    let base_filter = match band.filter {
        FilterType::LowPassII => FilterType::LowPass,
        FilterType::HighPassII => FilterType::HighPassI,
        _ => return (*band, None),
    };
    // A 24 dB/octave Butterworth pair uses section Q values 0.5412 and
    // 1.3066. The band's Q scales both sections around the familiar 0.707
    // single-pole-pair reference while preserving the requested order.
    let q_scale = band.q.clamp(0.1, 10.0) / std::f32::consts::FRAC_1_SQRT_2;
    let mut first = *band;
    first.filter = base_filter;
    first.q = (0.541_196_1 * q_scale).clamp(0.1, 10.0);
    let mut second = first;
    second.q = (1.306_563 * q_scale).clamp(0.1, 10.0);
    (first, Some(second))
}

fn single_equalizer_band_response_db(
    band: &EqualizerBand,
    frequency_hz: f32,
    sample_rate: u32,
) -> f32 {
    let f = Biquad::from_band(band, sample_rate);
    let omega = std::f32::consts::TAU * frequency_hz / sample_rate.max(1) as f32;
    let (sin, cos) = omega.sin_cos();
    let (sin2, cos2) = (2.0 * omega).sin_cos();
    let numerator = (f.b0 + f.b1 * cos + f.b2 * cos2).powi(2) + (f.b1 * sin + f.b2 * sin2).powi(2);
    let denominator = (1.0 + f.a1 * cos + f.a2 * cos2).powi(2) + (f.a1 * sin + f.a2 * sin2).powi(2);
    10.0 * (numerator.max(1.0e-20) / denominator.max(1.0e-20)).log10()
}

struct Biquad {
    enabled: bool,
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    target_b0: f32,
    target_b1: f32,
    target_b2: f32,
    target_a1: f32,
    target_a2: f32,
    step_b0: f32,
    step_b1: f32,
    step_b2: f32,
    step_a1: f32,
    step_a2: f32,
    coefficient_ramp_remaining: u32,
    enabled_mix: f32,
    enabled_target: f32,
    enabled_step: f32,
    enabled_ramp_remaining: u32,
    left_z1: f32,
    left_z2: f32,
    right_z1: f32,
    right_z2: f32,
}

struct DynamicEqDetector {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    left_z1: f32,
    left_z2: f32,
    left_y1: f32,
    left_y2: f32,
    right_z1: f32,
    right_z2: f32,
    right_y1: f32,
    right_y2: f32,
    envelope: f32,
    attack_coefficient: f32,
    release_coefficient: f32,
    threshold_db: f32,
    range_db: f32,
    gain_db: f32,
    target_gain_db: f32,
    samples_since_update: u8,
    control_update_due: bool,
    previous_gain_db: f32,
}

impl DynamicEqDetector {
    fn new(
        band: &EqualizerBand,
        settings: opentrack_core::project::DynamicEqBand,
        sample_rate: u32,
    ) -> Self {
        let mut detector = Self {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            left_z1: 0.0,
            left_z2: 0.0,
            left_y1: 0.0,
            left_y2: 0.0,
            right_z1: 0.0,
            right_z2: 0.0,
            right_y1: 0.0,
            right_y2: 0.0,
            envelope: 0.0,
            attack_coefficient: 0.0,
            release_coefficient: 0.0,
            threshold_db: -24.0,
            range_db: 0.0,
            gain_db: 0.0,
            target_gain_db: 0.0,
            samples_since_update: 0,
            control_update_due: false,
            previous_gain_db: 0.0,
        };
        detector.configure(band, settings, sample_rate);
        detector
    }

    fn configure(
        &mut self,
        band: &EqualizerBand,
        settings: opentrack_core::project::DynamicEqBand,
        sample_rate: u32,
    ) {
        let sr = sample_rate.max(1) as f32;
        let omega = std::f32::consts::TAU * band.frequency_hz.clamp(20.0, sr * 0.49) / sr;
        let (sin, cos) = omega.sin_cos();
        let alpha = sin / (2.0 * band.q.clamp(0.1, 10.0));
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b1 = 0.0;
        self.b2 = -alpha / a0;
        self.a1 = -2.0 * cos / a0;
        self.a2 = (1.0 - alpha) / a0;
        let coefficient =
            |time_ms: f32| (-1.0 / ((time_ms.clamp(0.1, 2000.0) * 0.001 * sr).max(1.0))).exp();
        self.attack_coefficient = coefficient(settings.attack_ms);
        self.release_coefficient = coefficient(settings.release_ms);
        self.threshold_db = settings.threshold_db.clamp(-90.0, 0.0);
        self.range_db = settings.range_db.clamp(-18.0, 18.0);
    }

    fn reset(&mut self) {
        self.left_z1 = 0.0;
        self.left_z2 = 0.0;
        self.left_y1 = 0.0;
        self.left_y2 = 0.0;
        self.right_z1 = 0.0;
        self.right_z2 = 0.0;
        self.right_y1 = 0.0;
        self.right_y2 = 0.0;
        self.envelope = 0.0;
        self.gain_db = 0.0;
        self.target_gain_db = 0.0;
        self.samples_since_update = 0;
        self.control_update_due = false;
        self.previous_gain_db = 0.0;
    }

    fn process(&mut self, left: f32, right: f32) -> f32 {
        let l = self.b0 * left + self.b1 * self.left_z1 + self.b2 * self.left_z2
            - self.a1 * self.left_y1
            - self.a2 * self.left_y2;
        self.left_z2 = self.left_z1;
        self.left_z1 = left;
        self.left_y2 = self.left_y1;
        self.left_y1 = l;
        let r = self.b0 * right + self.b1 * self.right_z1 + self.b2 * self.right_z2
            - self.a1 * self.right_y1
            - self.a2 * self.right_y2;
        self.right_z2 = self.right_z1;
        self.right_z1 = right;
        self.right_y2 = self.right_y1;
        self.right_y1 = r;
        let detected = ((l * l + r * r) * 0.5).sqrt().max(1.0e-8);
        let coefficient = if detected > self.envelope {
            self.attack_coefficient
        } else {
            self.release_coefficient
        };
        self.envelope = coefficient * self.envelope + (1.0 - coefficient) * detected;
        self.samples_since_update = self.samples_since_update.saturating_add(1);
        if self.samples_since_update >= 32 {
            self.samples_since_update = 0;
            let level_db = 20.0 * self.envelope.max(1.0e-8).log10();
            let over = ((level_db - self.threshold_db) / 12.0).clamp(0.0, 1.0);
            self.target_gain_db = self.range_db * over;
            self.control_update_due = true;
        }
        self.gain_db += (self.target_gain_db - self.gain_db) * 0.05;
        self.gain_db
    }

    fn take_coefficients_due(&mut self) -> bool {
        if !self.control_update_due {
            return false;
        }
        self.control_update_due = false;
        if (self.gain_db - self.previous_gain_db).abs() < 0.03 {
            return false;
        }
        self.previous_gain_db = self.gain_db;
        true
    }
}

impl Biquad {
    fn reset_state(&mut self) {
        self.left_z1 = 0.0;
        self.left_z2 = 0.0;
        self.right_z1 = 0.0;
        self.right_z2 = 0.0;
    }

    fn from_bandpass(band: &EqualizerBand, sample_rate: u32) -> Self {
        let mut band = *band;
        band.enabled = true;
        let mut filter = Self::from_band(&band, sample_rate);
        let coefficients = Self::bandpass_coefficients(&band, sample_rate);
        [
            &mut filter.b0,
            &mut filter.b1,
            &mut filter.b2,
            &mut filter.a1,
            &mut filter.a2,
        ]
        .into_iter()
        .zip(coefficients)
        .for_each(|(slot, value)| *slot = value);
        filter.target_b0 = filter.b0;
        filter.target_b1 = filter.b1;
        filter.target_b2 = filter.b2;
        filter.target_a1 = filter.a1;
        filter.target_a2 = filter.a2;
        filter.enabled_mix = 1.0;
        filter.enabled_target = 1.0;
        filter
    }

    fn bandpass_coefficients(band: &EqualizerBand, sample_rate: u32) -> [f32; 5] {
        let sr = sample_rate.max(1) as f32;
        let omega = std::f32::consts::TAU * band.frequency_hz.clamp(20.0, sr * 0.49) / sr;
        let q = band.q.clamp(0.1, 10.0);
        let (sin, cos) = omega.sin_cos();
        let alpha = sin / (2.0 * q);
        let inverse = 1.0 / (1.0 + alpha);
        [
            alpha * inverse,
            0.0,
            -alpha * inverse,
            -2.0 * cos * inverse,
            (1.0 - alpha) * inverse,
        ]
    }

    fn from_band(band: &EqualizerBand, sample_rate: u32) -> Self {
        let (band, _) = equalizer_band_sections(band);
        let band = &band;
        let sr = sample_rate.max(1) as f32;
        let frequency = band.frequency_hz.clamp(1.0, sr * 0.49);
        let base_q = band.q.clamp(0.1, 10.0);
        // Parametric II is the proportional-Q bell flavor: its bandwidth
        // tightens as the requested boost/cut grows. Parametric I retains the
        // classic constant-Q response. This is also used by the response
        // renderer, so the graph and audio path share the same curve.
        let q = match band.filter {
            FilterType::ParametricII => {
                base_q * 10.0_f32.powf(band.gain_db.abs().clamp(0.0, 24.0) / 40.0)
            }
            FilterType::LowShelfSoft | FilterType::HighShelfSoft => base_q * 0.62,
            FilterType::LowShelfSteep | FilterType::HighShelfSteep => base_q * 1.65,
            FilterType::LowShelfResonant | FilterType::HighShelfResonant => base_q * 2.6,
            _ => base_q,
        }
        .clamp(0.1, 10.0);
        let omega = std::f32::consts::TAU * frequency / sr;
        let (sin, cos) = omega.sin_cos();
        let a = 10.0_f32.powf(band.gain_db / 40.0);
        let alpha = sin / (2.0 * q);
        let (b0, b1, b2, a0, a1, a2) = match band.filter {
            FilterType::Bell | FilterType::ParametricI | FilterType::ParametricII => (
                1.0 + alpha * a,
                -2.0 * cos,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos,
                1.0 - alpha / a,
            ),
            FilterType::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            FilterType::LowPass | FilterType::LowPassII => (
                (1.0 - cos) * 0.5,
                1.0 - cos,
                (1.0 - cos) * 0.5,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterType::HighPass | FilterType::HighPassI | FilterType::HighPassII => (
                (1.0 + cos) * 0.5,
                -(1.0 + cos),
                (1.0 + cos) * 0.5,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterType::LowShelf
            | FilterType::LowShelfI
            | FilterType::LowShelfII
            | FilterType::LowShelfIII
            | FilterType::LowShelfIV
            | FilterType::LowShelfSoft
            | FilterType::LowShelfSteep
            | FilterType::LowShelfResonant
            | FilterType::HighShelf
            | FilterType::HighShelfSoft
            | FilterType::HighShelfSteep
            | FilterType::HighShelfResonant => {
                let slope = (1.0 / q).clamp(0.1, 10.0);
                let shelf_alpha =
                    sin * 0.5 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).max(0.0).sqrt();
                let beta = 2.0 * a.sqrt() * shelf_alpha;
                if matches!(
                    band.filter,
                    FilterType::LowShelf
                        | FilterType::LowShelfI
                        | FilterType::LowShelfII
                        | FilterType::LowShelfIII
                        | FilterType::LowShelfIV
                        | FilterType::LowShelfSoft
                        | FilterType::LowShelfSteep
                        | FilterType::LowShelfResonant
                ) {
                    (
                        a * ((a + 1.0) - (a - 1.0) * cos + beta),
                        2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                        a * ((a + 1.0) - (a - 1.0) * cos - beta),
                        (a + 1.0) + (a - 1.0) * cos + beta,
                        -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                        (a + 1.0) + (a - 1.0) * cos - beta,
                    )
                } else {
                    (
                        a * ((a + 1.0) + (a - 1.0) * cos + beta),
                        -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                        a * ((a + 1.0) + (a - 1.0) * cos - beta),
                        (a + 1.0) - (a - 1.0) * cos + beta,
                        2.0 * ((a - 1.0) - (a + 1.0) * cos),
                        (a + 1.0) - (a - 1.0) * cos - beta,
                    )
                }
            }
        };
        Self {
            enabled: band.enabled,
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            target_b0: b0 / a0,
            target_b1: b1 / a0,
            target_b2: b2 / a0,
            target_a1: a1 / a0,
            target_a2: a2 / a0,
            step_b0: 0.0,
            step_b1: 0.0,
            step_b2: 0.0,
            step_a1: 0.0,
            step_a2: 0.0,
            coefficient_ramp_remaining: 0,
            enabled_mix: if band.enabled { 1.0 } else { 0.0 },
            enabled_target: if band.enabled { 1.0 } else { 0.0 },
            enabled_step: 0.0,
            enabled_ramp_remaining: 0,
            left_z1: 0.0,
            left_z2: 0.0,
            right_z1: 0.0,
            right_z2: 0.0,
        }
    }

    fn update_band(&mut self, band: &EqualizerBand, sample_rate: u32) {
        let coefficients = Self::from_band(band, sample_rate);
        self.update_targets(
            [
                coefficients.b0,
                coefficients.b1,
                coefficients.b2,
                coefficients.a1,
                coefficients.a2,
            ],
            sample_rate,
        );
        if !self.enabled && band.enabled {
            self.left_z1 = 0.0;
            self.left_z2 = 0.0;
            self.right_z1 = 0.0;
            self.right_z2 = 0.0;
        }
        if self.enabled != band.enabled {
            self.enabled_target = if band.enabled { 1.0 } else { 0.0 };
            self.enabled_step =
                (self.enabled_target - self.enabled_mix) / (sample_rate.max(1) / 200).max(1) as f32;
            self.enabled_ramp_remaining = (sample_rate.max(1) / 200).max(1);
        }
        self.enabled = coefficients.enabled;
    }

    fn update_bandpass(&mut self, band: &EqualizerBand, sample_rate: u32) {
        self.update_targets(Self::bandpass_coefficients(band, sample_rate), sample_rate);
    }

    fn update_targets(&mut self, coefficients: [f32; 5], sample_rate: u32) {
        let ramp = (sample_rate / 200).max(1);
        [
            &mut self.target_b0,
            &mut self.target_b1,
            &mut self.target_b2,
            &mut self.target_a1,
            &mut self.target_a2,
        ]
        .into_iter()
        .zip(coefficients)
        .for_each(|(slot, value)| *slot = value);
        self.step_b0 = (self.target_b0 - self.b0) / ramp as f32;
        self.step_b1 = (self.target_b1 - self.b1) / ramp as f32;
        self.step_b2 = (self.target_b2 - self.b2) / ramp as f32;
        self.step_a1 = (self.target_a1 - self.a1) / ramp as f32;
        self.step_a2 = (self.target_a2 - self.a2) / ramp as f32;
        self.coefficient_ramp_remaining = ramp;
    }

    fn is_processing(&self) -> bool {
        self.enabled || self.enabled_mix > 0.0 || self.enabled_ramp_remaining > 0
    }

    fn process(&mut self, left: &mut f32, right: &mut f32) {
        if self.coefficient_ramp_remaining > 0 {
            self.b0 += self.step_b0;
            self.b1 += self.step_b1;
            self.b2 += self.step_b2;
            self.a1 += self.step_a1;
            self.a2 += self.step_a2;
            self.coefficient_ramp_remaining -= 1;
            if self.coefficient_ramp_remaining == 0 {
                self.b0 = self.target_b0;
                self.b1 = self.target_b1;
                self.b2 = self.target_b2;
                self.a1 = self.target_a1;
                self.a2 = self.target_a2;
            }
        }
        if self.enabled_ramp_remaining > 0 {
            self.enabled_mix += self.enabled_step;
            self.enabled_ramp_remaining -= 1;
            if self.enabled_ramp_remaining == 0 {
                self.enabled_mix = self.enabled_target;
            }
        }
        let dry_left = *left;
        let dry_right = *right;
        let input_left = *left;
        let output_left = self.b0 * input_left + self.left_z1;
        self.left_z1 = self.b1 * input_left - self.a1 * output_left + self.left_z2;
        self.left_z2 = self.b2 * input_left - self.a2 * output_left;
        let input_right = *right;
        let output_right = self.b0 * input_right + self.right_z1;
        self.right_z1 = self.b1 * input_right - self.a1 * output_right + self.right_z2;
        self.right_z2 = self.b2 * input_right - self.a2 * output_right;
        *left = dry_left + (output_left - dry_left) * self.enabled_mix;
        *right = dry_right + (output_right - dry_right) * self.enabled_mix;
    }
}

enum InsertProcessor {
    TestGenerator {
        enabled: bool,
        generator: TestGenerator,
    },
    StereoDelay {
        enabled: bool,
        left: Vec<f32>,
        right: Vec<f32>,
        cursor: usize,
        feedback: f32,
        mix: f32,
    },
    /// Preserved project slot for the removed OpenTrack compressor. It has no
    /// processor and deliberately leaves audio unchanged.
    UnavailableLegacy,
    /// A project-level VST3 slot that is not attached to a hosted runtime.
    /// Hosted audio runs through [`HostedInsert`] instead; this arm keeps the
    /// built-in chain neutral until the project slot is bound.
    Vst3Passthrough { enabled: bool },
}

#[cfg(test)]
mod processing_tests {
    use super::*;

    #[test]
    fn eq_solo_routes_accept_every_band_and_reject_out_of_range_indices() {
        assert_eq!(valid_eq_solo_band(None), None);
        assert_eq!(valid_eq_solo_band(Some(0)), Some(0));
        assert_eq!(valid_eq_solo_band(Some(7)), Some(7));
        assert_eq!(
            valid_eq_solo_band(Some(opentrack_core::project::MAX_EQ_BANDS - 1)),
            Some(opentrack_core::project::MAX_EQ_BANDS - 1)
        );
        assert_eq!(
            valid_eq_solo_band(Some(opentrack_core::project::MAX_EQ_BANDS)),
            None
        );
    }

    #[test]
    fn split_fade_lineage_preserves_rendered_audio_and_crossfades_sum_to_unity() {
        use opentrack_core::project::{Project, TrackKind};
        fn plan(p: &Project) -> PlaybackPlan {
            let source = Arc::new(DecodedWav {
                samples: vec![1.0; 8000].into(),
                waveform_peaks: Arc::new(vec![]),
                waveform: Arc::new(Waveform::default()),
                channels: 1,
                sample_rate: 1000,
            });
            PlaybackPlan::new(
                p.tempo_bpm,
                vec![PlaybackTrackSettings {
                    is_group: false,
                    audio_channels: 2,
                    clips: p.tracks[0]
                        .clips
                        .iter()
                        .map(|c| PlaybackClip {
                            source_fades: c.source_fades.clone(),
                            source: PlaybackSource::Decoded(source.clone()),
                            start_beats: c.start_beats,
                            source_offset_seconds: c.source_offset_seconds,
                            duration_seconds: c.duration_seconds,
                            fade_in_seconds: c.fade_in_seconds,
                            fade_out_seconds: c.fade_out_seconds,
                            gain: c.gain,
                            muted: false,
                            solo: false,
                        })
                        .collect(),
                    midi_notes: vec![],
                    midi_controllers: vec![],
                    instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
                    meter_peak: Arc::new(StereoMeter::default()),
                    gain: 1.0,
                    pan: 0.0,
                    track_delay_ms: 0.0,
                    muted: false,
                    solo: false,
                    solo_defeat: false,
                    equalizer: ChannelEqualizer::default(),
                    inserts: vec![],
                    inserts_bypassed: false,
                    hosted_inserts: Vec::new(),
                    monitor_input: None,
                    output_route: PlaybackOutputRoute::Stereo,
                }],
                1,
                48000,
                None,
            )
        }
        let mut p = Project::default();
        p.tempo_bpm = 120.0;
        p.add_track(TrackKind::Audio);
        let id = p.add_audio_clip(0, "signal.wav".into(), 0.0, 4.0).unwrap();
        let track = p.tracks[0].id;
        p.tracks[0].clips[0].source_offset_seconds = 0.5;
        p.tracks[0].clips[0].fade_in_seconds = 2.0;
        p.tracks[0].clips[0].fade_out_seconds = 1.0;
        let before = plan(&p);
        let right = p.split_audio_clip(track, id, 1.0).unwrap();
        assert!(p.split_audio_clip(track, right, 3.0).is_some());
        let after = plan(&p);
        for i in 0..4000 {
            let beat = i as f64 / 500.0;
            let a = mix_track_frame(&before, &before.tracks[0], beat);
            let b = mix_track_frame(&after, &after.tracks[0], beat);
            assert!((a.0 - b.0).abs() < 1e-6);
            assert_eq!(b.0, b.1);
        }
        p.tracks[0].clips.clear();
        let a = p.add_audio_clip(0, "a.wav".into(), 0.0, 2.0).unwrap();
        let b = p.add_audio_clip(0, "b.wav".into(), 3.0, 2.0).unwrap();
        assert!(p.crossfade_audio_clips(a, b));
        let crossfade = plan(&p);
        for i in 1500..2000 {
            let out = mix_track_frame(&crossfade, &crossfade.tracks[0], i as f64 / 500.0);
            assert!((out.0 - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn newest_overlapping_recording_is_the_audible_layer_except_for_crossfades() {
        fn source(value: f32) -> PlaybackSource {
            PlaybackSource::Decoded(Arc::new(DecodedWav {
                samples: vec![value; 100_000].into(),
                waveform_peaks: Arc::new(Vec::new()),
                waveform: Arc::new(Waveform::default()),
                channels: 1,
                sample_rate: 48_000,
            }))
        }
        let plan = PlaybackPlan::new(
            120.0,
            vec![PlaybackTrackSettings {
                is_group: false,
                audio_channels: 2,
                clips: vec![
                    PlaybackClip {
                        source_fades: Vec::new(),
                        source: source(0.25),
                        start_beats: 0.0,
                        source_offset_seconds: 0.0,
                        duration_seconds: 2.0,
                        fade_in_seconds: 0.0,
                        fade_out_seconds: 0.0,
                        gain: 1.0,
                        muted: false,
                        solo: false,
                    },
                    PlaybackClip {
                        source_fades: Vec::new(),
                        source: source(0.75),
                        start_beats: 0.0,
                        source_offset_seconds: 0.0,
                        duration_seconds: 1.0,
                        fade_in_seconds: 0.0,
                        fade_out_seconds: 0.0,
                        gain: 1.0,
                        muted: false,
                        solo: false,
                    },
                ],
                midi_notes: Vec::new(),
                midi_controllers: Vec::new(),
                instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
                meter_peak: Arc::new(StereoMeter::default()),
                gain: 1.0,
                pan: 0.0,
                track_delay_ms: 0.0,
                muted: false,
                solo: false,
                solo_defeat: false,
                equalizer: ChannelEqualizer::default(),
                inserts: Vec::new(),
                inserts_bypassed: false,
                hosted_inserts: Vec::new(),
                monitor_input: None,
                output_route: PlaybackOutputRoute::Stereo,
            }],
            1,
            48_000,
            None,
        );
        assert_eq!(mix_track_frame(&plan, &plan.tracks[0], 0.0), (0.75, 0.75));
        assert_eq!(mix_track_frame(&plan, &plan.tracks[0], 3.0), (0.25, 0.25));
    }

    #[test]
    fn playback_parameter_snapshots_crossfade_without_an_output_step() {
        fn plan(gain: f32) -> Arc<PlaybackPlan> {
            Arc::new(PlaybackPlan::new(
                120.0,
                vec![PlaybackTrackSettings {
                    is_group: false,
                    audio_channels: 2,
                    clips: Vec::new(),
                    midi_notes: Vec::new(),
                    midi_controllers: Vec::new(),
                    instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
                    meter_peak: Arc::new(StereoMeter::default()),
                    gain,
                    pan: 0.0,
                    track_delay_ms: 0.0,
                    muted: false,
                    solo: false,
                    solo_defeat: false,
                    equalizer: ChannelEqualizer::default(),
                    inserts: Vec::new(),
                    inserts_bypassed: false,
                    hosted_inserts: Vec::new(),
                    monitor_input: Some(PlaybackInputChannel::Stereo),
                    output_route: PlaybackOutputRoute::Stereo,
                }],
                1,
                48_000,
                None,
            ))
        }
        let previous = plan(1.0);
        let current = plan(0.0);
        current.begin_parameter_transition(previous);
        let mut values = Vec::new();
        for frame in 0..current.transition_frames {
            values.push(
                render_plan_frame(
                    &current,
                    frame as u64,
                    48_000,
                    (1.0, 1.0),
                    (0.0, 0.0),
                    true,
                    &current.monitor_mute_mask,
                    true,
                )
                .0,
            );
        }
        assert!(values.first().copied().unwrap() > 0.99);
        assert!(values.last().copied().unwrap().abs() < 1.0e-6);
        assert!(
            values
                .windows(2)
                .all(|pair| (pair[1] - pair[0]).abs() < 0.003)
        );
    }
    #[test]
    fn mixed_rate_source_playback_preserves_pitch_and_duration_at_project_output_rates() {
        for rate in [44100, 48000, 96000] {
            let source = Arc::new(DecodedWav {
                samples: (0..44100)
                    .map(|i| (i as f32 * std::f32::consts::TAU * 440.0 / 44100.0).sin())
                    .collect::<Vec<_>>()
                    .into(),
                waveform_peaks: Arc::new(vec![]),
                waveform: Arc::new(Waveform::default()),
                channels: 1,
                sample_rate: 44100,
            });
            let plan = PlaybackPlan::new(
                120.0,
                vec![PlaybackTrackSettings {
                    is_group: false,
                    audio_channels: 2,
                    clips: vec![PlaybackClip {
                        source_fades: vec![],
                        source: PlaybackSource::Decoded(source),
                        start_beats: 2.0,
                        source_offset_seconds: 0.0,
                        duration_seconds: 1.0,
                        fade_in_seconds: 0.0,
                        fade_out_seconds: 0.0,
                        gain: 1.0,
                        muted: false,
                        solo: false,
                    }],
                    midi_notes: vec![],
                    midi_controllers: vec![],
                    instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
                    meter_peak: Arc::new(StereoMeter::default()),
                    gain: 1.0,
                    pan: 0.0,
                    track_delay_ms: 0.0,
                    muted: false,
                    solo: false,
                    solo_defeat: false,
                    equalizer: ChannelEqualizer::default(),
                    inserts: vec![],
                    inserts_bypassed: false,
                    hosted_inserts: Vec::new(),
                    monitor_input: None,
                    output_route: PlaybackOutputRoute::Stereo,
                }],
                1,
                rate,
                None,
            );
            for frame in (0..rate).step_by(97) {
                let seconds = frame as f64 / rate as f64;
                let actual = mix_track_frame(&plan, &plan.tracks[0], 2.0 + seconds * 2.0).0;
                let expected = (seconds * std::f64::consts::TAU * 440.0).sin() as f32;
                assert!((actual - expected).abs() < 0.0015, "{rate} frame {frame}");
            }
            assert_eq!(mix_track_frame(&plan, &plan.tracks[0], 4.0), (0.0, 0.0));
        }
    }
    #[test]
    fn live_midi_gain_above_unity_reaches_audio_instead_of_being_clamped() {
        fn renderer(gain: f32) -> LiveMidiRenderer {
            let queue = Arc::new(LiveMidiQueue::new());
            assert!(queue.push(MidiInputMessage::NoteOn {
                channel: 0,
                key: 69,
                velocity: 100
            }));
            LiveMidiRenderer::new(
                queue,
                Arc::new(AtomicBool::new(true)),
                Arc::new(AtomicU8::new(0)),
                Arc::new(AtomicU32::new(gain.to_bits())),
            )
        }
        let mut unity = renderer(1.0);
        let mut boosted = renderer(2.0);
        let mut heard = false;
        for _ in 0..512 {
            let a = unity.next_frame(48000);
            let b = boosted.next_frame(48000);
            assert!((b.0 - a.0 * 2.0).abs() < 0.000001);
            if a.0.abs() > 0.001 {
                heard = true;
            }
        }
        assert!(heard);
    }
    #[test]
    fn audio_event_trim_gain_and_fades_change_rendered_samples_and_stop_at_event_end() {
        let source = Arc::new(DecodedWav {
            samples: vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7].into(),
            waveform_peaks: Arc::new(vec![]),
            waveform: Arc::new(Waveform::default()),
            channels: 1,
            sample_rate: 2,
        });
        let plan = PlaybackPlan::new(
            120.0,
            vec![PlaybackTrackSettings {
                is_group: false,
                audio_channels: 2,
                clips: vec![PlaybackClip {
                    source_fades: Vec::new(),
                    source: PlaybackSource::Decoded(source),
                    start_beats: 1.0,
                    source_offset_seconds: 1.0,
                    duration_seconds: 2.0,
                    fade_in_seconds: 0.5,
                    fade_out_seconds: 0.5,
                    gain: 2.0,
                    muted: false,
                    solo: false,
                }],
                midi_notes: vec![],
                midi_controllers: vec![],
                instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
                meter_peak: Arc::new(StereoMeter::default()),
                gain: 1.0,
                pan: 0.0,
                track_delay_ms: 0.0,
                muted: false,
                solo: false,
                solo_defeat: false,
                equalizer: ChannelEqualizer::default(),
                inserts: vec![],
                inserts_bypassed: false,
                hosted_inserts: Vec::new(),
                monitor_input: None,
                output_route: PlaybackOutputRoute::Stereo,
            }],
            1,
            48000,
            None,
        );
        for (seconds, expected) in [
            (-0.25, 0.0),
            (0.0, 0.0),
            (0.25, 0.25),
            (1.0, 0.8),
            (1.75, 0.55),
            (2.0, 0.0),
            (3.0, 0.0),
        ] {
            let (l, r) = mix_track_frame(&plan, &plan.tracks[0], 1.0 + seconds * 2.0);
            assert!((l - expected).abs() < 0.000001, "at {seconds}: {l}");
            assert_eq!(l, r);
        }
    }
    #[test]
    fn metronome_is_block_independent_through_cycle_wrap_and_master_gain() {
        fn render(gain: f32, enabled: bool, playing: bool, block: usize) -> (Vec<f32>, [f32; 2]) {
            let mut p = PlaybackPlan::new(120.0, Vec::new(), 1, 48_000, Some((0.0, 2.0)));
            p.metronome = MetronomeSettings {
                enabled,
                ..Default::default()
            };
            let mut bus = opentrack_core::project::BusChannel::output().strip;
            bus.gain = gain;
            p.output_bus = bus::BusRuntime::new(&bus, 48_000);
            let plan = ArcSwap::from(Arc::new(p));
            let tone = AtomicBool::new(false);
            let playing = AtomicBool::new(playing);
            let playhead = AtomicU64::new(47_980);
            let generation = AtomicU64::new(0);
            let active = AtomicBool::new(false);
            let monitor = Arc::new(ArcSwapOption::empty());
            let mut reader = AudioMonitorReader::default();
            let mut phase = 0.0;
            let mut samples = vec![0.0; 4096];
            for chunk in samples.chunks_mut(block * 2) {
                render_f32(
                    chunk,
                    2,
                    48_000,
                    &tone,
                    &playing,
                    &playhead,
                    &plan,
                    &generation,
                    &active,
                    &monitor,
                    &mut reader,
                    &mut phase,
                );
            }
            let stereo_out_peak = plan.load().output_meter.take();
            (samples, stereo_out_peak)
        }
        let (full, meter) = render(1.0, true, true, 2048);
        assert!(full[..40].iter().all(|v| *v == 0.0));
        assert!(full[42..2000].iter().any(|v| v.abs() > 0.01));
        assert_eq!(meter, [0.0; 2], "click must bypass Stereo Out metering");
        for block in [7, 64, 127, 512] {
            assert_eq!(full, render(1.0, true, true, block).0);
        }
        assert_eq!(full, render(0.5, true, true, 127).0);
        assert_eq!(full, render(0.0, true, true, 127).0);
        for silent in [
            render(1.0, false, true, 127).0,
            render(1.0, true, false, 127).0,
        ] {
            assert!(silent.iter().all(|v| *v == 0.0));
        }
    }

    #[test]
    fn cycle_boundaries_apply_inside_one_audio_block() {
        for (mode, expected) in [(CycleMode::Loop, 2), (CycleMode::Skip, 8)] {
            // At 60 BPM and 1000 Hz these locators are exactly frames 2..4.
            let mut plan = PlaybackPlan::new(60.0, Vec::new(), 1, 1000, Some((0.002, 0.004)));
            plan.cycle_mode = mode;
            let plan = ArcSwap::from(Arc::new(plan));
            let tone = AtomicBool::new(false);
            let playing = AtomicBool::new(true);
            let playhead = AtomicU64::new(0);
            let generation = AtomicU64::new(0);
            let active = AtomicBool::new(false);
            let monitor = Arc::new(ArcSwapOption::empty());
            let mut reader = AudioMonitorReader::default();
            let mut phase = 0.0;
            let mut samples = [0.0; 12]; // six stereo frames in one callback
            render_f32(
                &mut samples,
                2,
                1000,
                &tone,
                &playing,
                &playhead,
                &plan,
                &generation,
                &active,
                &monitor,
                &mut reader,
                &mut phase,
            );
            assert_eq!(playhead.load(Ordering::Relaxed), expected, "{mode:?}");
        }
        assert_eq!(wrap_playhead_frame(1, Some((2, 4)), CycleMode::Skip), 1);
        assert_eq!(wrap_playhead_frame(2, Some((2, 4)), CycleMode::Skip), 4);
        assert_eq!(wrap_playhead_frame(4, Some((2, 4)), CycleMode::Skip), 4);
        assert_eq!(wrap_playhead_frame(10, Some((2, 4)), CycleMode::Loop), 2);
        assert_eq!(wrap_playhead_frame(3, None, CycleMode::Skip), 3);
    }

    #[test]
    fn editor_curve_uses_enabled_filter_response_and_channel_bypass() {
        let mut eq = opentrack_core::project::ChannelEqualizer::default();
        eq.bands[1] = EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 1000.0,
            gain_db: 6.0,
            q: 1.0,
        };
        assert!((channel_equalizer_response_db(&eq, 1000.0, 48000) - 6.0).abs() < 0.01);
        eq.bands[1].enabled = false;
        assert_eq!(channel_equalizer_response_db(&eq, 1000.0, 48000), 0.0);
        eq.bands[1].enabled = true;
        eq.bypassed = true;
        assert_eq!(channel_equalizer_response_db(&eq, 1000.0, 48000), 0.0);
    }

    #[test]
    fn tonal_target_edits_do_not_advance_audio_eq_revision() {
        let equalizer = ChannelEqualizer::default();
        let control = EqualizerControl::new(equalizer.clone());
        let original = control.target.load_full();

        let mut target_edit = equalizer.clone();
        target_edit.tonal_target = Some(opentrack_core::project::TonalTarget {
            name: "Reference".into(),
            analysis_version: 1,
            sample_rate: 48_000,
            source_stage: 1,
            source_name: "Main Vocal".into(),
            analysis_seconds: 30.0,
            fft_size: 4096,
            band_powers: vec![1.0 / 24.0; 24],
        });
        control.set(&target_edit);

        assert_eq!(control.revision.load(Ordering::Acquire), 0);
        assert!(Arc::ptr_eq(&original, &control.target.load_full()));

        target_edit.bands[1].gain_db = 3.0;
        control.set(&target_edit);
        assert_eq!(control.revision.load(Ordering::Acquire), 1);
        assert_eq!(control.target.load().bands[1].gain_db, 3.0);
    }

    #[test]
    fn additional_builtin_eq_band_contributes_to_audio_response() {
        let mut eq = ChannelEqualizer::default();
        eq.additional_bands[0] = EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 3_200.0,
            gain_db: -5.0,
            q: 1.1,
        };
        assert!((channel_equalizer_response_db(&eq, 3_200.0, 48_000) + 5.0).abs() < 0.02);
    }

    #[test]
    fn dynamic_eq_detector_moves_gain_after_crossing_threshold() {
        let band = EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
        };
        let settings = opentrack_core::project::DynamicEqBand {
            enabled: true,
            threshold_db: -36.0,
            range_db: -6.0,
            attack_ms: 1.0,
            release_ms: 100.0,
        };
        let mut detector = DynamicEqDetector::new(&band, settings, 48_000);
        let mut gain = 0.0;
        for index in 0..20_000 {
            let sample = (std::f32::consts::TAU * 1_000.0 * index as f32 / 48_000.0).sin() * 0.5;
            gain = detector.process(sample, sample);
        }
        assert!(gain < -4.0, "dynamic gain did not engage: {gain} dB");
    }

    #[test]
    fn dynamic_eq_detector_supports_positive_gain_range() {
        let band = EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
        };
        let settings = opentrack_core::project::DynamicEqBand {
            enabled: true,
            threshold_db: -36.0,
            range_db: 9.0,
            attack_ms: 1.0,
            release_ms: 100.0,
        };
        let mut detector = DynamicEqDetector::new(&band, settings, 48_000);
        let mut gain = 0.0;
        for index in 0..20_000 {
            let sample = (std::f32::consts::TAU * 1_000.0 * index as f32 / 48_000.0).sin() * 0.5;
            gain = detector.process(sample, sample);
        }
        assert!(gain > 6.0, "dynamic boost did not engage: {gain} dB");
    }

    #[test]
    fn dynamic_eq_detector_updates_level_control_every_32_samples() {
        let band = EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
        };
        let settings = opentrack_core::project::DynamicEqBand {
            enabled: true,
            threshold_db: -36.0,
            range_db: -6.0,
            attack_ms: 1.0,
            release_ms: 100.0,
        };
        let mut detector = DynamicEqDetector::new(&band, settings, 48_000);
        for _ in 0..31 {
            detector.process(0.5, 0.5);
            assert!(!detector.control_update_due);
        }
        detector.process(0.5, 0.5);
        assert!(detector.control_update_due);
        detector.take_coefficients_due();
        assert!(!detector.control_update_due);
    }

    #[test]
    #[ignore = "manual release-mode dynamic EQ 64/128-frame callback workload probe"]
    fn dynamic_eq_callback_cost_probe() {
        let sample_rate = 48_000;
        let settings = opentrack_core::project::DynamicEqBand {
            enabled: true,
            threshold_db: -36.0,
            range_db: -6.0,
            attack_ms: 1.0,
            release_ms: 100.0,
        };
        let frequencies =
            std::array::from_fn::<_, { opentrack_core::project::MAX_EQ_BANDS }, _>(|index| {
                80.0 * (10_000.0_f32 / 80.0)
                    .powf(index as f32 / (opentrack_core::project::MAX_EQ_BANDS - 1) as f32)
            });
        let signal = (0..sample_rate as usize)
            .map(|index| {
                let phase = std::f32::consts::TAU * 1_000.0 * index as f32 / sample_rate as f32;
                let left = phase.sin() * 0.5;
                let right = (phase * 0.997).sin() * 0.45;
                (left, right)
            })
            .collect::<Vec<_>>();
        for block_frames in [64, 128] {
            let mut bands = frequencies.map(|frequency_hz| EqualizerBand {
                enabled: true,
                filter: FilterType::Bell,
                frequency_hz,
                gain_db: 0.0,
                q: 0.707,
            });
            let mut detectors: [DynamicEqDetector; opentrack_core::project::MAX_EQ_BANDS] =
                std::array::from_fn(|index| {
                    DynamicEqDetector::new(&bands[index], settings, sample_rate)
                });
            let mut filters: [Biquad; opentrack_core::project::MAX_EQ_BANDS] =
                std::array::from_fn(|index| Biquad::from_band(&bands[index], sample_rate));
            let blocks = 30 * sample_rate as usize / block_frames;
            let mut callback_ns = Vec::with_capacity(blocks);
            let mut checksum = 0.0_f32;
            for block in 0..blocks {
                let start = std::time::Instant::now();
                for frame in 0..block_frames {
                    let (mut left, mut right) =
                        signal[(block * block_frames + frame) % signal.len()];
                    for index in 0..opentrack_core::project::MAX_EQ_BANDS {
                        let dynamic_gain = detectors[index].process(left, right);
                        if detectors[index].take_coefficients_due() {
                            bands[index].gain_db = dynamic_gain.clamp(-18.0, 18.0);
                            filters[index].update_band(&bands[index], sample_rate);
                        }
                        filters[index].process(&mut left, &mut right);
                    }
                    checksum += left + right;
                }
                callback_ns.push(start.elapsed().as_nanos());
            }
            callback_ns.sort_unstable();
            let percentile =
                |percentile: usize| callback_ns[(blocks * percentile / 100).min(blocks - 1)];
            let budget_ns = (block_frames as u128 * 1_000_000_000) / sample_rate as u128;
            eprintln!(
                "dynamic EQ probe: {} bands, 48 kHz, {block_frames} frames, 30 simulated seconds; p50={} ns, p95={} ns, p99={} ns, max={} ns, p99={:.2}% of callback budget, checksum={:.3}",
                opentrack_core::project::MAX_EQ_BANDS,
                percentile(50),
                percentile(95),
                percentile(99),
                callback_ns[blocks - 1],
                percentile(99) as f64 * 100.0 / budget_ns as f64,
                std::hint::black_box(checksum),
            );
        }
    }

    #[test]
    fn dynamic_eq_changes_the_channel_processor_audio_path() {
        let mut equalizer = ChannelEqualizer::default();
        let band_index = opentrack_core::project::MAX_EQ_BANDS - 1;
        *equalizer.band_mut(band_index).unwrap() = EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
        };
        *equalizer.dynamic_band_mut(band_index).unwrap() = opentrack_core::project::DynamicEqBand {
            enabled: true,
            threshold_db: -36.0,
            range_db: -6.0,
            attack_ms: 1.0,
            release_ms: 100.0,
        };
        let instrument_factory = oscillator_instrument_factory(InstrumentWaveform::Sine);
        let control = EqualizerControl::new(equalizer.clone());
        let mut processor = ChannelProcessor::new(
            &equalizer,
            Arc::clone(&control),
            &[],
            false,
            &[],
            &[],
            Arc::new(MidiEventControls::new(&[])),
            instrument_factory.as_ref(),
            120.0,
            48_000,
        );
        let meter = StereoMeter::default();
        let mut input_energy = 0.0_f64;
        let mut output_energy = 0.0_f64;
        for index in 0..24_000 {
            let input = (std::f32::consts::TAU * 1_000.0 * index as f32 / 48_000.0).sin() * 0.5;
            let (mut left, mut right) = (input, input);
            processor.process(&mut left, &mut right);
            processor.publish_dynamic_updates(&meter);
            if index >= 16_000 {
                input_energy += f64::from(input * input);
                output_energy += f64::from(left * left);
            }
        }
        let gain_db = 10.0 * (output_energy / input_energy).log10();
        assert!(
            gain_db < -2.0,
            "dynamic EQ audio change was {gain_db:.2} dB"
        );
        assert!(meter.dynamic_eq_gains_db()[band_index] < -4.0);

        let mut static_only = equalizer;
        static_only.dynamic_band_mut(band_index).unwrap().enabled = false;
        control.set(&static_only);
        let (mut left, mut right) = (0.0, 0.0);
        processor.process(&mut left, &mut right);
        processor.publish_dynamic_updates(&meter);
        assert_eq!(meter.dynamic_eq_gains_db()[band_index], 0.0);
    }

    #[test]
    fn eq_band_solo_isolates_selected_frequency_without_changing_eq_revision() {
        fn render(frequency_hz: f32, center_frequency: f32, band_index: usize) -> f64 {
            let mut equalizer = ChannelEqualizer::default();
            let band = equalizer.band_mut(band_index).unwrap();
            band.frequency_hz = center_frequency;
            band.q = 1.0;
            let control = EqualizerControl::new(equalizer.clone());
            control.set_solo_band(valid_eq_solo_band(Some(band_index)));
            assert_eq!(control.revision.load(Ordering::Acquire), 0);
            let instrument_factory = oscillator_instrument_factory(InstrumentWaveform::Sine);
            let mut processor = ChannelProcessor::new(
                &equalizer,
                control,
                &[],
                false,
                &[],
                &[],
                Arc::new(MidiEventControls::new(&[])),
                instrument_factory.as_ref(),
                120.0,
                48_000,
            );
            let mut energy = 0.0_f64;
            for index in 0..48_000 {
                let input =
                    (std::f32::consts::TAU * frequency_hz * index as f32 / 48_000.0).sin() * 0.5;
                let (mut left, mut right) = (input, input);
                processor.process(&mut left, &mut right);
                if index >= 24_000 {
                    energy += f64::from(left * left);
                }
            }
            energy
        }

        for (band_index, center_frequency) in [(0, 1_000.0), (31, 8_000.0)] {
            let center_energy = render(center_frequency, center_frequency, band_index);
            let off_band_energy = render(100.0, center_frequency, band_index);
            assert!(
                center_energy > off_band_energy * 20.0,
                "band {} solo isolation ratio was {:.1} dB",
                band_index + 1,
                10.0 * (center_energy / off_band_energy).log10()
            );
        }
    }

    #[test]
    fn bell_eq_applies_requested_gain_at_center_frequency() {
        let band = EqualizerBand {
            enabled: true,
            filter: FilterType::Bell,
            frequency_hz: 1_000.0,
            gain_db: 6.0,
            q: 1.0,
        };
        let mut biquad = Biquad::from_band(&band, 48_000);
        let mut input_energy = 0.0_f64;
        let mut output_energy = 0.0_f64;
        let sample_count = 48_000;
        for index in 0..sample_count {
            let input = (std::f32::consts::TAU * 1_000.0 * index as f32 / 48_000.0).sin();
            let mut left = input;
            let mut right = input;
            biquad.process(&mut left, &mut right);
            if index > sample_count / 2 {
                input_energy += f64::from(input * input);
                output_energy += f64::from(left * left);
            }
        }
        let gain = (output_energy / input_energy).sqrt();
        assert!((1.85..=2.15).contains(&gain), "measured gain was {gain}");
    }

    #[test]
    fn band_enable_and_parameter_changes_ramp_filter_state() {
        let mut band = EqualizerBand {
            enabled: false,
            filter: FilterType::Bell,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
        };
        let mut filter = Biquad::from_band(&band, 48_000);
        band.enabled = true;
        band.gain_db = 6.0;
        filter.update_band(&band, 48_000);

        assert_eq!(filter.enabled_mix, 0.0);
        assert_eq!(filter.enabled_ramp_remaining, 240);
        assert!(filter.coefficient_ramp_remaining > 0);
        let mut left = 0.5;
        let mut right = 0.5;
        filter.process(&mut left, &mut right);
        assert!((left - 0.5).abs() < 0.02);
        assert!(filter.enabled_mix > 0.0 && filter.enabled_mix < 1.0);

        for _ in 1..240 {
            let (mut left, mut right) = (0.25, 0.25);
            filter.process(&mut left, &mut right);
        }
        assert_eq!(filter.enabled_mix, 1.0);
        assert_eq!(filter.enabled_ramp_remaining, 0);
        assert_eq!(filter.coefficient_ramp_remaining, 0);
        assert_eq!(filter.b0, filter.target_b0);
    }

    #[test]
    fn low_and_high_pass_filters_attenuate_their_stop_bands() {
        let low_pass_pass = measure_filter_gain(FilterType::LowPass, 1_000.0, 100.0, 0.0);
        let low_pass_stop = measure_filter_gain(FilterType::LowPass, 1_000.0, 10_000.0, 0.0);
        let high_pass_stop = measure_filter_gain(FilterType::HighPass, 1_000.0, 100.0, 0.0);
        let high_pass_pass = measure_filter_gain(FilterType::HighPass, 1_000.0, 10_000.0, 0.0);
        assert!(
            low_pass_pass > 0.95,
            "low-pass pass-band gain was {low_pass_pass}"
        );
        assert!(
            low_pass_stop < 0.03,
            "low-pass stop-band gain was {low_pass_stop}"
        );
        assert!(
            high_pass_stop < 0.03,
            "high-pass stop-band gain was {high_pass_stop}"
        );
        assert!(
            high_pass_pass > 0.95,
            "high-pass pass-band gain was {high_pass_pass}"
        );
    }

    #[test]
    fn second_order_pass_slopes_are_distinct_in_response_and_channel_audio() {
        fn render_channel(high_pass: bool, second_order: bool) -> f64 {
            let mut equalizer = ChannelEqualizer::default();
            let band = equalizer.band_mut(0).unwrap();
            band.enabled = true;
            band.filter = match (high_pass, second_order) {
                (false, false) => FilterType::LowPass,
                (false, true) => FilterType::LowPassII,
                (true, false) => FilterType::HighPassI,
                (true, true) => FilterType::HighPassII,
            };
            band.frequency_hz = 1_000.0;
            band.gain_db = 0.0;
            band.q = 0.707;
            let control = EqualizerControl::new(equalizer.clone());
            let instrument_factory = oscillator_instrument_factory(InstrumentWaveform::Sine);
            let mut processor = ChannelProcessor::new(
                &equalizer,
                control,
                &[],
                false,
                &[],
                &[],
                Arc::new(MidiEventControls::new(&[])),
                instrument_factory.as_ref(),
                120.0,
                48_000,
            );
            let test_frequency = if high_pass { 500.0 } else { 2_000.0 };
            let mut input_energy = 0.0_f64;
            let mut output_energy = 0.0_f64;
            for index in 0..48_000 {
                let input =
                    (std::f32::consts::TAU * test_frequency * index as f32 / 48_000.0).sin();
                let (mut left, mut right) = (input, input);
                processor.process(&mut left, &mut right);
                if index >= 24_000 {
                    input_energy += f64::from(input * input);
                    output_energy += f64::from(left * left);
                }
            }
            10.0 * (output_energy / input_energy).log10()
        }

        let low_12 = render_channel(false, false);
        let low_24 = render_channel(false, true);
        let high_12 = render_channel(true, false);
        let high_24 = render_channel(true, true);
        assert!(
            low_24 < low_12 - 8.0,
            "low-pass slopes: {low_12} vs {low_24} dB"
        );
        assert!(
            high_24 < high_12 - 8.0,
            "high-pass slopes: {high_12} vs {high_24} dB"
        );
        for (filter, measured) in [
            (FilterType::LowPassII, low_24),
            (FilterType::HighPassII, high_24),
        ] {
            let expected = f64::from(equalizer_band_response_db(
                &EqualizerBand {
                    enabled: true,
                    filter,
                    frequency_hz: 1_000.0,
                    gain_db: 0.0,
                    q: 0.707,
                },
                if filter == FilterType::LowPassII {
                    2_000.0
                } else {
                    500.0
                },
                48_000,
            ));
            assert!(
                (measured - expected).abs() < 0.5,
                "{filter:?}: {measured} vs {expected} dB"
            );
        }
    }

    #[test]
    fn notch_filter_attenuates_its_center_frequency() {
        let center = measure_filter_gain(FilterType::Notch, 1_000.0, 1_000.0, 0.0);
        let lower_side = measure_filter_gain(FilterType::Notch, 1_000.0, 200.0, 0.0);
        let upper_side = measure_filter_gain(FilterType::Notch, 1_000.0, 5_000.0, 0.0);
        assert!(center < 0.01, "notch center gain was {center}");
        assert!(lower_side > 0.9, "lower side gain was {lower_side}");
        assert!(upper_side > 0.9, "upper side gain was {upper_side}");
    }

    #[test]
    fn shelf_filters_apply_gain_on_their_respective_sides_without_stereo_cross_talk() {
        let low_shelf_boost = measure_filter_gain(FilterType::LowShelf, 1_000.0, 100.0, 6.0);
        let low_shelf_high = measure_filter_gain(FilterType::LowShelf, 1_000.0, 10_000.0, 6.0);
        let high_shelf_low = measure_filter_gain(FilterType::HighShelf, 1_000.0, 100.0, 6.0);
        let high_shelf_boost = measure_filter_gain(FilterType::HighShelf, 1_000.0, 10_000.0, 6.0);
        assert!(
            low_shelf_boost > 1.8,
            "low-shelf gain was {low_shelf_boost}"
        );
        assert!((0.95..=1.05).contains(&low_shelf_high));
        assert!((0.95..=1.05).contains(&high_shelf_low));
        assert!(
            high_shelf_boost > 1.8,
            "high-shelf gain was {high_shelf_boost}"
        );

        let band = EqualizerBand {
            enabled: true,
            filter: FilterType::HighPass,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 0.707,
        };
        let mut biquad = Biquad::from_band(&band, 48_000);
        let mut left = 0.5;
        let mut right = 0.0;
        for _ in 0..128 {
            biquad.process(&mut left, &mut right);
            assert_eq!(right, 0.0);
        }
    }

    #[test]
    fn shelf_flavors_have_distinct_low_and_high_transition_curves() {
        for (classic, smooth, steep, resonant, frequency) in [
            (
                FilterType::LowShelf,
                FilterType::LowShelfSoft,
                FilterType::LowShelfSteep,
                FilterType::LowShelfResonant,
                700.0,
            ),
            (
                FilterType::HighShelf,
                FilterType::HighShelfSoft,
                FilterType::HighShelfSteep,
                FilterType::HighShelfResonant,
                1_400.0,
            ),
        ] {
            let response = |filter| {
                equalizer_band_response_db(
                    &EqualizerBand {
                        enabled: true,
                        filter,
                        frequency_hz: 1_000.0,
                        gain_db: 9.0,
                        q: 0.707,
                    },
                    frequency,
                    48_000,
                )
            };
            let values = [
                response(classic),
                response(smooth),
                response(steep),
                response(resonant),
            ];
            for left in 0..values.len() {
                for right in left + 1..values.len() {
                    assert!(
                        (values[left] - values[right]).abs() > 0.05,
                        "shelf responses should differ: {values:?}"
                    );
                }
            }
        }
    }

    fn measure_filter_gain(
        filter: FilterType,
        frequency_hz: f32,
        input_frequency_hz: f32,
        gain_db: f32,
    ) -> f64 {
        let sample_rate = 48_000.0_f32;
        let band = EqualizerBand {
            enabled: true,
            filter,
            frequency_hz,
            gain_db,
            q: 0.707,
        };
        let (first, second) = equalizer_band_sections(&band);
        let mut biquad = Biquad::from_band(&first, sample_rate as u32);
        let mut second_biquad = second.map(|band| Biquad::from_band(&band, sample_rate as u32));
        let mut input_energy = 0.0_f64;
        let mut output_energy = 0.0_f64;
        for index in 0..48_000 {
            let input =
                (std::f32::consts::TAU * input_frequency_hz * index as f32 / sample_rate).sin();
            let mut left = input;
            let mut right = input;
            biquad.process(&mut left, &mut right);
            if let Some(second_biquad) = &mut second_biquad {
                second_biquad.process(&mut left, &mut right);
            }
            if index >= 24_000 {
                input_energy += f64::from(input * input);
                output_energy += f64::from(left * left);
            }
        }
        (output_energy / input_energy).sqrt()
    }

    #[test]
    fn stereo_delay_returns_the_wet_impulse_at_the_configured_frame() {
        let effect = InsertEffect {
            enabled: true,
            kind: InsertEffectKind::StereoDelay {
                time_ms: 200.0,
                feedback: 0.0,
                mix: 0.5,
            },
        };
        let mut delay = InsertProcessor::new(&effect, 10);
        let mut output = Vec::new();
        for index in 0..4 {
            let mut left = if index == 0 { 1.0 } else { 0.0 };
            let mut right = 0.0;
            delay.process(&mut left, &mut right);
            output.push(left);
            assert_eq!(right, 0.0);
        }
        assert_eq!(output, [0.5, 0.0, 0.5, 0.0]);
    }

    #[test]
    fn test_generator_insert_emits_audio_on_an_empty_track() {
        let effect = InsertEffect {
            enabled: true,
            kind: InsertEffectKind::TestGenerator {
                waveform: GeneratorWaveform::Sine,
                frequency_hz: 1_000.0,
                gain_db: -6.0,
            },
        };
        let mut generator = InsertProcessor::new(&effect, 48_000);
        let mut left = 0.0;
        let mut right = 0.0;
        generator.process(&mut left, &mut right);
        assert_eq!(left, 0.0);
        assert_eq!(right, 0.0);
        generator.process(&mut left, &mut right);
        assert!(left > 0.0);
        assert_eq!(left, right);
    }

    #[test]
    fn disabled_eq_and_empty_insert_chain_leave_audio_unchanged() {
        let instrument_factory = oscillator_instrument_factory(InstrumentWaveform::Sine);
        let equalizer = ChannelEqualizer::default();
        let mut processor = ChannelProcessor::new(
            &equalizer,
            EqualizerControl::new(equalizer.clone()),
            &[],
            false,
            &[],
            &[],
            Arc::new(MidiEventControls::new(&[])),
            instrument_factory.as_ref(),
            120.0,
            48_000,
        );
        let mut left = -0.375;
        let mut right = 0.625;
        processor.process(&mut left, &mut right);
        assert_eq!(left, -0.375);
        assert_eq!(right, 0.625);
    }

    #[test]
    fn midi_voice_renders_notes_at_scheduled_frames_and_resets_after_seek() {
        let note = PlaybackMidiNote {
            id: 1,
            start_beats: 1.0,
            duration_beats: 1.0,
            frequency_hz: 1_000.0,
            channel: 0,
            velocity: 1.0,
        };
        let controls = Arc::new(MidiEventControls::new(&[note]));
        let mut voice = MidiVoice::new(
            &[note],
            &[],
            controls,
            120.0,
            48_000,
            InstrumentWaveform::Sine,
        );

        assert_eq!(voice.next_sample(23_999), 0.0);
        assert_eq!(voice.next_sample(24_000), 0.0);
        let mut rendered_sample = 0.0;
        for frame in 24_001..=24_252 {
            rendered_sample = voice.next_sample(frame);
        }
        assert!(rendered_sample > 0.1, "note sample was {rendered_sample}");
        for frame in 24_253..=48_000 {
            voice.next_sample(frame);
        }
        assert_eq!(voice.next_sample(48_000), 0.0);
        assert_eq!(voice.next_sample(24_000), 0.0);
    }

    #[test]
    fn midi_voice_caps_callback_polyphony() {
        let notes = vec![
            PlaybackMidiNote {
                id: 1,
                start_beats: 0.0,
                duration_beats: 4.0,
                frequency_hz: 440.0,
                channel: 0,
                velocity: 0.5,
            };
            MAX_ACTIVE_MIDI_VOICES + 1
        ];
        let controls = Arc::new(MidiEventControls::new(&notes));
        let mut voice = MidiVoice::new(
            &notes,
            &[],
            controls,
            120.0,
            48_000,
            InstrumentWaveform::Sine,
        );
        voice.next_sample(0);
        assert_eq!(
            voice.active.iter().flatten().count(),
            MAX_ACTIVE_MIDI_VOICES
        );
        voice.next_sample(1);
        assert_eq!(
            voice.active.iter().flatten().count(),
            MAX_ACTIVE_MIDI_VOICES
        );
    }

    #[test]
    fn live_velocity_change_smooths_a_sounding_midi_voice() {
        let note = PlaybackMidiNote {
            id: 41,
            start_beats: 0.0,
            duration_beats: 4.0,
            frequency_hz: 440.0,
            channel: 0,
            velocity: 1.0,
        };
        let controls = Arc::new(MidiEventControls::new(&[note]));
        let mut voice = MidiVoice::new(
            &[note],
            &[],
            Arc::clone(&controls),
            120.0,
            48_000,
            InstrumentWaveform::Sine,
        );
        voice.next_frame(0);
        assert_eq!(voice.active[0].unwrap().velocity, 1.0);

        assert!(controls.update_note_velocity(41, 0.0));
        voice.next_frame(1);
        let velocity_after_first_sample = voice.active[0].unwrap().velocity;
        assert!(velocity_after_first_sample < 1.0);
        assert!(velocity_after_first_sample > 0.9);

        for frame in 2..600 {
            voice.next_frame(frame);
        }
        assert!(voice.active[0].unwrap().velocity < 0.1);
    }

    #[test]
    fn midi_controllers_change_voice_volume_and_pan_and_restore_after_seek() {
        let note = PlaybackMidiNote {
            id: 1,
            start_beats: 0.0,
            duration_beats: 4.0,
            frequency_hz: 440.0,
            channel: 0,
            velocity: 1.0,
        };
        let controllers = [
            PlaybackMidiController {
                position_beats: 0.0,
                channel: 0,
                controller: 7,
                value: 0,
            },
            PlaybackMidiController {
                position_beats: 2.0,
                channel: 0,
                controller: 7,
                value: 127,
            },
            PlaybackMidiController {
                position_beats: 2.0,
                channel: 0,
                controller: 10,
                value: 127,
            },
        ];
        let controls = Arc::new(MidiEventControls::new(&[note]));
        let mut voice = MidiVoice::new(
            &[note],
            &controllers,
            controls,
            120.0,
            48_000,
            InstrumentWaveform::Sine,
        );

        let mut last_frame = (0.0, 0.0);
        for frame in 0..=48_001 {
            last_frame = voice.next_frame(frame);
        }
        let (left, right) = last_frame;
        assert_eq!(left, 0.0);
        assert!(right > 0.0);

        assert_eq!(voice.next_frame(24_001), (0.0, 0.0));
        assert_eq!(voice.controller_values[0][7], 0);
    }

    #[test]
    fn midi_controller_seek_restores_state_from_bounded_checkpoints() {
        let controllers: Vec<_> = (0..300_u32)
            .map(|index| PlaybackMidiController {
                position_beats: f64::from(index),
                channel: 0,
                controller: 11,
                value: (index % 128) as u8,
            })
            .collect();
        let mut voice = MidiVoice::new(
            &[],
            &controllers,
            Arc::new(MidiEventControls::new(&[])),
            120.0,
            48_000,
            InstrumentWaveform::Sine,
        );

        voice.next_sample(269 * 24_000 + 1);

        assert_eq!(voice.controller_values[0][11], 13);
        assert_eq!(voice.next_controller, 270);
    }

    #[test]
    fn same_frame_controller_changes_coalesce_to_the_last_value_per_control() {
        let controllers = [
            PlaybackMidiController {
                position_beats: 0.0,
                channel: 0,
                controller: 7,
                value: 12,
            },
            PlaybackMidiController {
                position_beats: 0.0,
                channel: 0,
                controller: 10,
                value: 0,
            },
            PlaybackMidiController {
                position_beats: 0.0,
                channel: 0,
                controller: 7,
                value: 108,
            },
            PlaybackMidiController {
                position_beats: 0.0,
                channel: 0,
                controller: 10,
                value: 127,
            },
        ];
        let mut voice = MidiVoice::new(
            &[],
            &controllers,
            Arc::new(MidiEventControls::new(&[])),
            120.0,
            48_000,
            InstrumentWaveform::Sine,
        );

        assert_eq!(voice.controllers.len(), 2);
        voice.next_sample(0);
        assert_eq!(voice.controller_values[0][7], 108);
        assert_eq!(voice.controller_values[0][10], 127);
    }

    #[test]
    fn live_input_monitor_resamples_stereo_frames_from_the_capture_ring() {
        let monitor = AudioMonitorBuffer::new(24_000);
        monitor.set_enabled(true);
        assert!(monitor.push_frame(0.0, 0.0));
        assert!(monitor.push_frame(1.0, -1.0));
        assert!(monitor.push_frame(0.0, 0.0));
        let mut reader = AudioMonitorReader::default();

        assert_eq!(reader.next_frame(&monitor, 48_000), (0.0, 0.0));
        let midpoint = reader.next_frame(&monitor, 48_000);
        assert!((midpoint.0 - 0.5).abs() < 1.0e-6);
        assert!((midpoint.1 + 0.5).abs() < 1.0e-6);
        let peak = reader.next_frame(&monitor, 48_000);
        assert!((peak.0 - 1.0).abs() < 1.0e-6);
        assert!((peak.1 + 1.0).abs() < 1.0e-6);

        monitor.set_enabled(false);
        assert_eq!(reader.next_frame(&monitor, 48_000), (0.0, 0.0));
        monitor.set_enabled(true);
        assert!(monitor.push_frame(-1.0, 1.0));
        assert!(monitor.push_frame(0.0, 0.0));
        let reset_frame = reader.next_frame(&monitor, 48_000);
        assert!((reset_frame.0 + 1.0).abs() < 1.0e-6);
        assert!((reset_frame.1 - 1.0).abs() < 1.0e-6);
        monitor.set_enabled(false);
        monitor.set_enabled(true);
        assert!(monitor.push_frame(0.5, 0.5));
        assert!(monitor.push_frame(0.75, 0.75));
        let reenabled_frame = reader.next_frame(&monitor, 48_000);
        assert!((reenabled_frame.0 - 0.5).abs() < 1.0e-6);
        assert!((reenabled_frame.1 - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn monitoring_during_playback_suppresses_recorded_audio_on_that_track() {
        let monitor = Arc::new(AudioMonitorBuffer::new(48_000));
        monitor.set_enabled(true);
        assert!(monitor.push_frame(0.25, -0.25));
        assert!(monitor.push_frame(0.5, -0.5));
        assert!(monitor.push_frame(0.75, -0.75));
        let input_monitor = Arc::new(ArcSwapOption::empty());
        input_monitor.store(Some(Arc::clone(&monitor)));
        let playback = PlaybackSource::Decoded(Arc::new(DecodedWav {
            samples: vec![0.9; 16].into(),
            waveform_peaks: Arc::new(Vec::new()),
            waveform: Arc::new(Waveform::default()),
            channels: 1,
            sample_rate: 48_000,
        }));
        let plan = ArcSwap::from(Arc::new(PlaybackPlan::new(
            120.0,
            vec![PlaybackTrackSettings {
                is_group: false,
                audio_channels: 2,
                clips: vec![PlaybackClip {
                    source_fades: Vec::new(),
                    source: playback,
                    start_beats: 0.0,
                    source_offset_seconds: 0.0,
                    duration_seconds: 1.0,
                    fade_in_seconds: 0.0,
                    fade_out_seconds: 0.0,
                    gain: 1.0,
                    muted: false,
                    solo: false,
                }],
                midi_notes: Vec::new(),
                midi_controllers: Vec::new(),
                instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
                meter_peak: Arc::new(StereoMeter::default()),
                gain: 1.0,
                pan: 0.0,
                track_delay_ms: 0.0,
                muted: false,
                solo: false,
                solo_defeat: false,
                equalizer: ChannelEqualizer::default(),
                inserts: Vec::new(),
                inserts_bypassed: false,
                hosted_inserts: Vec::new(),
                monitor_input: Some(PlaybackInputChannel::Stereo),
                output_route: PlaybackOutputRoute::Stereo,
            }],
            1,
            48_000,
            None,
        )));
        let tone = AtomicBool::new(false);
        let playing = AtomicBool::new(true);
        let playhead = AtomicU64::new(0);
        let callback_generation = AtomicU64::new(0);
        let callback_active = AtomicBool::new(false);
        let mut monitor_reader = AudioMonitorReader::default();
        let mut phase = 0.0;
        let mut samples = [0.0_f32; 4];

        render_f32(
            &mut samples,
            2,
            48_000,
            &tone,
            &playing,
            &playhead,
            &plan,
            &callback_generation,
            &callback_active,
            &input_monitor,
            &mut monitor_reader,
            &mut phase,
        );

        for (actual, expected) in samples.iter().zip([0.25, -0.25, 0.5, -0.5]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
    }

    #[test]
    fn note_audition_is_audible_with_stopped_transport_and_leaves_the_playhead_alone() {
        let source = PlaybackSource::Decoded(Arc::new(DecodedWav {
            samples: vec![0.5; 48_000].into(),
            waveform_peaks: Arc::new(Vec::new()),
            waveform: Arc::new(Waveform::default()),
            channels: 1,
            sample_rate: 48_000,
        }));
        let audition = NoteAudition::new(source, 0.0, 0.0, 1.0, 1.0).unwrap();
        let plan = ArcSwap::from_pointee(PlaybackPlan::new(120.0, Vec::new(), 1, 48_000, None));
        plan.load().note_audition.store(Some(Arc::new(audition)));
        let tone = AtomicBool::new(false);
        let playing = AtomicBool::new(false);
        let playhead = AtomicU64::new(0);
        let callback_generation = AtomicU64::new(0);
        let callback_active = AtomicBool::new(false);
        let input_monitor = Arc::new(ArcSwapOption::empty());
        let mut monitor_reader = AudioMonitorReader::default();
        let mut phase = 0.0;
        let mut samples = [0.0_f32; 960];
        render_f32(
            &mut samples,
            1,
            48_000,
            &tone,
            &playing,
            &playhead,
            &plan,
            &callback_generation,
            &callback_active,
            &input_monitor,
            &mut monitor_reader,
            &mut phase,
        );
        assert!(
            samples.iter().any(|sample| sample.abs() > 0.2),
            "the preview must reach the device while transport is stopped"
        );
        assert_eq!(playhead.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn track_stereo_meters_follow_pan_and_output_meter_follows_routing() {
        for (pan, route, expected) in [
            (-1.0, PlaybackOutputRoute::Stereo, [0.5, 0.0]),
            (0.0, PlaybackOutputRoute::Stereo, [0.5, 0.125]),
            (1.0, PlaybackOutputRoute::Stereo, [0.0, 0.125]),
            (0.0, PlaybackOutputRoute::Disconnected, [0.0, 0.0]),
        ] {
            let meter = Arc::new(StereoMeter::default());
            let plan = ArcSwap::from_pointee(PlaybackPlan::new(
                120.0,
                vec![PlaybackTrackSettings {
                    is_group: false,
                    audio_channels: 2,
                    clips: Vec::new(),
                    midi_notes: Vec::new(),
                    midi_controllers: Vec::new(),
                    instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
                    meter_peak: Arc::clone(&meter),
                    gain: 0.5,
                    pan,
                    track_delay_ms: 0.0,
                    muted: false,
                    solo: false,
                    solo_defeat: false,
                    equalizer: ChannelEqualizer::default(),
                    inserts: Vec::new(),
                    inserts_bypassed: false,
                    hosted_inserts: Vec::new(),
                    monitor_input: Some(PlaybackInputChannel::Stereo),
                    output_route: route,
                }],
                1,
                48000,
                None,
            ));
            let monitor = Arc::new(AudioMonitorBuffer::new(48000));
            monitor.set_enabled(true);
            for _ in 0..4 {
                assert!(monitor.push_frame(1.0, 0.25));
            }
            let bridge = Arc::new(ArcSwapOption::from(Some(monitor)));
            let mut samples = [0.0; 4];
            render_f32(
                &mut samples,
                2,
                48000,
                &AtomicBool::new(false),
                &AtomicBool::new(false),
                &AtomicU64::new(0),
                &plan,
                &AtomicU64::new(0),
                &AtomicBool::new(false),
                &bridge,
                &mut AudioMonitorReader::default(),
                &mut 0.0,
            );
            let master = plan.load().output_meter.take();
            for (actual, expected) in master.into_iter().zip(expected) {
                assert!((actual - expected).abs() < 1.0e-6);
            }
            let track = meter.take();
            let expected_track = [0.5 * (1.0 - pan.max(0.0)), 0.125 * (1.0 + pan.min(0.0))];
            for (actual, expected) in track.into_iter().zip(expected_track) {
                assert!((actual - expected).abs() < 1.0e-6);
            }
        }
    }

    #[test]
    fn playback_plan_uses_the_reusable_instrument_factory() {
        struct TestFactory;
        struct TestProcessor;

        impl InstrumentFactory for TestFactory {
            fn create_processor(
                &self,
                _notes: &[PlaybackMidiNote],
                _controllers: &[PlaybackMidiController],
                _controls: Arc<MidiEventControls>,
                _tempo_bpm: f64,
                _sample_rate: u32,
            ) -> Box<dyn InstrumentProcessor> {
                Box::new(TestProcessor)
            }
        }

        impl InstrumentProcessor for TestProcessor {
            fn next_frame(&mut self, _output_frame: u64) -> (f32, f32) {
                (0.25, -0.125)
            }
        }

        let meter_peak = Arc::new(StereoMeter::default());
        let plan = ArcSwap::from(Arc::new(PlaybackPlan::new(
            120.0,
            vec![PlaybackTrackSettings {
                is_group: false,
                audio_channels: 2,
                clips: Vec::new(),
                midi_notes: Vec::new(),
                midi_controllers: Vec::new(),
                instrument_factory: Arc::new(TestFactory),
                meter_peak: Arc::clone(&meter_peak),
                gain: 1.0,
                pan: 0.0,
                track_delay_ms: 0.0,
                muted: false,
                solo: false,
                solo_defeat: false,
                equalizer: ChannelEqualizer::default(),
                inserts: Vec::new(),
                inserts_bypassed: false,
                hosted_inserts: Vec::new(),
                monitor_input: None,
                output_route: PlaybackOutputRoute::Stereo,
            }],
            1,
            48_000,
            None,
        )));
        let tone = AtomicBool::new(false);
        let playing = AtomicBool::new(true);
        let playhead = AtomicU64::new(0);
        let callback_generation = AtomicU64::new(0);
        let callback_active = AtomicBool::new(false);
        let input_monitor = Arc::new(ArcSwapOption::empty());
        let mut monitor_reader = AudioMonitorReader::default();
        let mut phase = 0.0;
        let mut samples = [0.0_f32; 4];
        render_f32(
            &mut samples,
            2,
            48_000,
            &tone,
            &playing,
            &playhead,
            &plan,
            &callback_generation,
            &callback_active,
            &input_monitor,
            &mut monitor_reader,
            &mut phase,
        );

        assert_eq!(samples, [0.25, -0.125, 0.25, -0.125]);
        assert_eq!(meter_peak.take(), [0.25, 0.125]);
    }

    #[test]
    fn built_in_instrument_waveforms_have_distinct_oscillator_shapes() {
        let quarter_cycle = std::f32::consts::FRAC_PI_2;
        assert!((oscillator_sample(InstrumentWaveform::Sine, quarter_cycle) - 1.0).abs() < 1.0e-6);
        assert_eq!(oscillator_sample(InstrumentWaveform::Square, 0.0), 1.0);
        assert_eq!(
            oscillator_sample(InstrumentWaveform::Square, std::f32::consts::PI),
            -1.0
        );
        assert_eq!(oscillator_sample(InstrumentWaveform::Sawtooth, 0.0), -1.0);
        assert_eq!(
            oscillator_sample(InstrumentWaveform::Sawtooth, std::f32::consts::PI),
            0.0
        );
        assert_eq!(oscillator_sample(InstrumentWaveform::Triangle, 0.0), -1.0);
        assert_eq!(
            oscillator_sample(InstrumentWaveform::Triangle, std::f32::consts::PI),
            1.0
        );
    }
}

impl InsertProcessor {
    fn new(insert: &InsertEffect, sample_rate: u32) -> Self {
        match &insert.kind {
            InsertEffectKind::TestGenerator {
                waveform,
                frequency_hz,
                gain_db,
            } => {
                let parameters = TestGeneratorParameters {
                    waveform: match waveform {
                        GeneratorWaveform::Sine => TestWaveform::Sine,
                        GeneratorWaveform::Square => TestWaveform::Square,
                        GeneratorWaveform::Sawtooth => TestWaveform::Sawtooth,
                        GeneratorWaveform::Triangle => TestWaveform::Triangle,
                        GeneratorWaveform::WhiteNoise => TestWaveform::WhiteNoise,
                        GeneratorWaveform::BrownNoise => TestWaveform::BrownNoise,
                        GeneratorWaveform::PinkNoise => TestWaveform::PinkNoise,
                    },
                    frequency_hz: *frequency_hz,
                    gain_db: *gain_db,
                };
                let rate = sample_rate.max(1);
                let generator = TestGenerator::new(rate, parameters).unwrap_or_else(|_| {
                    TestGenerator::new(
                        rate,
                        TestGeneratorParameters {
                            waveform: TestWaveform::Sine,
                            frequency_hz: 440.0,
                            gain_db: -18.0,
                        },
                    )
                    .expect("fallback TestGenerator parameters are valid")
                });
                Self::TestGenerator {
                    enabled: insert.enabled,
                    generator,
                }
            }
            InsertEffectKind::StereoDelay {
                time_ms,
                feedback,
                mix,
            } => {
                let delay_frames = ((*time_ms * sample_rate as f32 / 1_000.0).round() as usize)
                    .clamp(1, sample_rate.max(1) as usize * 2);
                Self::StereoDelay {
                    enabled: insert.enabled,
                    left: vec![0.0; delay_frames],
                    right: vec![0.0; delay_frames],
                    cursor: 0,
                    feedback: *feedback,
                    mix: *mix,
                }
            }
            InsertEffectKind::LegacyUnavailableCompressor { .. } => Self::UnavailableLegacy,
            InsertEffectKind::Vst3 { .. } => Self::Vst3Passthrough {
                enabled: insert.enabled,
            },
        }
    }

    fn enabled(&self) -> bool {
        match self {
            Self::TestGenerator { enabled, .. }
            | Self::StereoDelay { enabled, .. }
            | Self::Vst3Passthrough { enabled } => *enabled,
            Self::UnavailableLegacy => false,
        }
    }

    fn process(&mut self, left: &mut f32, right: &mut f32) {
        match self {
            Self::TestGenerator { generator, .. } => {
                let sample = generator.next_sample();
                *left += sample;
                *right += sample;
            }
            Self::StereoDelay {
                left: delay_left,
                right: delay_right,
                cursor,
                feedback,
                mix,
                ..
            } => {
                let delayed_left = delay_left[*cursor];
                let delayed_right = delay_right[*cursor];
                delay_left[*cursor] = *left + delayed_left * *feedback;
                delay_right[*cursor] = *right + delayed_right * *feedback;
                *left = *left * (1.0 - *mix) + delayed_left * *mix;
                *right = *right * (1.0 - *mix) + delayed_right * *mix;
                *cursor += 1;
                if *cursor == delay_left.len() {
                    *cursor = 0;
                }
            }
            Self::UnavailableLegacy => {}
            Self::Vst3Passthrough { .. } => {}
        }
    }
}

#[cfg(test)]
mod hosted_processing_tests {
    use super::*;

    struct GainProcessor {
        gain: f32,
        offset: f32,
        latency: u32,
        calls: AtomicU64,
        fail: bool,
    }

    impl GainProcessor {
        fn new(gain: f32, offset: f32, latency: u32, fail: bool) -> Arc<Self> {
            Arc::new(Self {
                gain,
                offset,
                latency,
                calls: AtomicU64::new(0),
                fail,
            })
        }
    }

    impl opentrack_plugins::HostedProcessor for GainProcessor {
        fn process(&self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(left.len(), right.len());
            if self.fail {
                return Err("synthetic hosted failure".to_owned());
            }
            for (left, right) in left.iter_mut().zip(right.iter_mut()) {
                *left = *left * self.gain + self.offset;
                *right = *right * self.gain + self.offset;
            }
            Ok(())
        }

        fn latency_samples(&self) -> u32 {
            self.latency
        }

        fn tail_samples(&self) -> u32 {
            0
        }
    }

    struct DelayState {
        left: Vec<f32>,
        right: Vec<f32>,
        cursor: usize,
    }

    struct DelayProcessor {
        latency: u32,
        state: std::cell::UnsafeCell<DelayState>,
        calls: AtomicU64,
    }

    // SAFETY: the test processor is only called by one serialized test render.
    unsafe impl Sync for DelayProcessor {}

    impl DelayProcessor {
        fn new(latency: u32) -> Arc<Self> {
            Arc::new(Self {
                latency,
                state: std::cell::UnsafeCell::new(DelayState {
                    left: vec![0.0; latency as usize],
                    right: vec![0.0; latency as usize],
                    cursor: 0,
                }),
                calls: AtomicU64::new(0),
            })
        }
    }

    impl opentrack_plugins::HostedProcessor for DelayProcessor {
        fn process(&self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(left.len(), right.len());
            // SAFETY: one serialized renderer owns this test processor.
            let state = unsafe { &mut *self.state.get() };
            if state.left.is_empty() {
                return Ok(());
            }
            for (left, right) in left.iter_mut().zip(right.iter_mut()) {
                let cursor = state.cursor;
                let delayed_left = state.left[cursor];
                let delayed_right = state.right[cursor];
                state.left[cursor] = *left;
                state.right[cursor] = *right;
                state.cursor += 1;
                if state.cursor == state.left.len() {
                    state.cursor = 0;
                }
                *left = delayed_left;
                *right = delayed_right;
            }
            Ok(())
        }

        fn latency_samples(&self) -> u32 {
            self.latency
        }

        fn tail_samples(&self) -> u32 {
            0
        }
    }

    fn source(samples: Vec<f32>) -> PlaybackSource {
        PlaybackSource::Decoded(Arc::new(DecodedWav {
            samples: samples.into(),
            waveform_peaks: Arc::new(Vec::new()),
            waveform: Arc::new(Waveform::default()),
            channels: 1,
            sample_rate: 48_000,
        }))
    }

    fn constant_source(level: f32, frames: usize) -> PlaybackSource {
        source(vec![level; frames])
    }

    fn track(source: PlaybackSource, hosted_inserts: Vec<HostedInsert>) -> PlaybackTrackSettings {
        PlaybackTrackSettings {
            is_group: false,
            audio_channels: 2,
            clips: vec![PlaybackClip {
                source_fades: Vec::new(),
                source,
                start_beats: 0.0,
                source_offset_seconds: 0.0,
                duration_seconds: 2.0,
                fade_in_seconds: 0.0,
                fade_out_seconds: 0.0,
                gain: 1.0,
                muted: false,
                solo: false,
            }],
            midi_notes: Vec::new(),
            midi_controllers: Vec::new(),
            instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
            meter_peak: Arc::new(StereoMeter::default()),
            gain: 1.0,
            pan: 0.0,
            track_delay_ms: 0.0,
            muted: false,
            solo: false,
            solo_defeat: false,
            equalizer: ChannelEqualizer::default(),
            inserts: Vec::new(),
            inserts_bypassed: false,
            hosted_inserts,
            monitor_input: None,
            output_route: PlaybackOutputRoute::Stereo,
        }
    }

    fn render(plan: Arc<PlaybackPlan>, frames: usize) -> Vec<f32> {
        let plan = ArcSwap::from(plan);
        let tone = AtomicBool::new(false);
        let playing = AtomicBool::new(true);
        let playhead = AtomicU64::new(0);
        let generation = AtomicU64::new(0);
        let active = AtomicBool::new(false);
        let monitor = Arc::new(ArcSwapOption::empty());
        let mut reader = AudioMonitorReader::default();
        let mut phase = 0.0;
        let mut samples = vec![0.0_f32; frames * 2];
        render_f32(
            &mut samples,
            2,
            48_000,
            &tone,
            &playing,
            &playhead,
            &plan,
            &generation,
            &active,
            &monitor,
            &mut reader,
            &mut phase,
        );
        samples
    }

    #[test]
    fn hosted_insert_processes_each_block_once_and_applies_gain() {
        let gain = GainProcessor::new(2.0, 0.0, 0, false);
        let plan = Arc::new(PlaybackPlan::new(
            120.0,
            vec![track(
                constant_source(0.125, 256),
                vec![HostedInsert {
                    processor: gain.clone(),
                    enabled: true,
                }],
            )],
            1,
            48_000,
            None,
        ));
        let output = render(Arc::clone(&plan), 128);
        let expected_calls = ((128 + hosted_block_frames() - 1) / hosted_block_frames()) as u64;
        assert_eq!(gain.calls.load(Ordering::Relaxed), expected_calls);
        assert!(plan.hosted_processing_enabled());
        for sample in output {
            assert!((sample - 0.25).abs() < 1.0e-6, "sample was {sample}");
        }
    }

    #[test]
    fn hosted_plan_transitions_do_not_process_the_shared_instance_twice() {
        let gain = GainProcessor::new(2.0, 0.0, 0, false);
        let settings = || {
            vec![track(
                constant_source(0.125, 256),
                vec![HostedInsert {
                    processor: gain.clone(),
                    enabled: true,
                }],
            )]
        };
        let previous = Arc::new(PlaybackPlan::new(120.0, settings(), 1, 48_000, None));
        let current = Arc::new(PlaybackPlan::new(120.0, settings(), 2, 48_000, None));
        current.begin_parameter_transition(previous);
        let output = render(Arc::clone(&current), 64);
        let expected_calls = ((64 + hosted_block_frames() - 1) / hosted_block_frames()) as u64;
        assert_eq!(gain.calls.load(Ordering::Relaxed), expected_calls);
        for sample in output {
            assert!((sample - 0.25).abs() < 1.0e-6, "sample was {sample}");
        }
    }

    #[test]
    fn pdc_delays_the_lower_latency_track_to_align_an_impulse() {
        let mut impulse = vec![0.0_f32; 256];
        impulse[0] = 0.5;
        let delay = DelayProcessor::new(4);
        let plan = Arc::new(PlaybackPlan::new(
            120.0,
            vec![
                track(
                    source(impulse.clone()),
                    vec![HostedInsert {
                        processor: delay.clone(),
                        enabled: true,
                    }],
                ),
                track(source(impulse), Vec::new()),
            ],
            1,
            48_000,
            None,
        ));
        assert_eq!(plan.hosted_latency_samples(), 4);
        assert_eq!(plan.track_hosted_latency_samples(0), Some(4));
        assert_eq!(plan.track_hosted_latency_samples(1), Some(0));
        assert_eq!(plan.track_hosted_delay_samples(0), Some(0));
        assert_eq!(plan.track_hosted_delay_samples(1), Some(4));
        let output = render(Arc::clone(&plan), 16);
        for (frame, pair) in output.chunks_exact(2).enumerate() {
            let expected = if frame == 4 { 1.0 } else { 0.0 };
            assert!(
                (pair[0] - expected).abs() < 1.0e-6,
                "frame {frame} left was {}",
                pair[0]
            );
            assert!(
                (pair[1] - expected).abs() < 1.0e-6,
                "frame {frame} right was {}",
                pair[1]
            );
        }
    }

    #[test]
    fn plan_without_hosted_inserts_keeps_the_legacy_path_and_zero_delay() {
        let mut impulse = vec![0.0_f32; 256];
        impulse[0] = 0.5;
        let plan = Arc::new(PlaybackPlan::new(
            120.0,
            vec![track(source(impulse), Vec::new())],
            1,
            48_000,
            None,
        ));
        assert!(!plan.hosted_processing_enabled());
        assert_eq!(plan.hosted_latency_samples(), 0);
        assert_eq!(plan.track_hosted_delay_samples(0), Some(0));
        let output = render(plan, 16);
        assert!((output[0] - 0.5).abs() < 1.0e-6);
        assert!(output[2..].iter().all(|sample| sample.abs() < 1.0e-6));
    }

    #[test]
    fn disabled_hosted_insert_is_skipped() {
        let gain = GainProcessor::new(10.0, 0.0, 0, false);
        let plan = Arc::new(PlaybackPlan::new(
            120.0,
            vec![track(
                constant_source(0.125, 256),
                vec![HostedInsert {
                    processor: gain.clone(),
                    enabled: false,
                }],
            )],
            1,
            48_000,
            None,
        ));
        let output = render(plan, 128);
        assert_eq!(gain.calls.load(Ordering::Relaxed), 0);
        for sample in output {
            assert!((sample - 0.125).abs() < 1.0e-6, "sample was {sample}");
        }
    }

    #[test]
    fn hosted_processor_error_falls_back_to_dry_without_panicking() {
        let gain = GainProcessor::new(10.0, 0.0, 0, true);
        let plan = Arc::new(PlaybackPlan::new(
            120.0,
            vec![track(
                constant_source(0.125, 256),
                vec![HostedInsert {
                    processor: gain.clone(),
                    enabled: true,
                }],
            )],
            1,
            48_000,
            None,
        ));
        let output = render(Arc::clone(&plan), 128);
        let expected_calls = ((128 + hosted_block_frames() - 1) / hosted_block_frames()) as u64;
        assert_eq!(gain.calls.load(Ordering::Relaxed), expected_calls);
        for sample in output {
            assert!((sample - 0.125).abs() < 1.0e-6, "sample was {sample}");
        }
        let diagnostic = plan.hosted_processing_diagnostic();
        assert_eq!(diagnostic.error_count, expected_calls);
        assert_eq!(diagnostic.last_track, Some(0));
        assert_eq!(diagnostic.last_insert, Some(0));
        assert!(diagnostic.last_error_code.is_some());
    }
}
