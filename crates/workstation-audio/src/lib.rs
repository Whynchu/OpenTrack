//! Audio file decoding and default-device output.
//!
//! WAV files are decoded before reaching the audio callback. Playback plans
//! are immutable snapshots; the callback only reads them and writes samples.

mod midi;
pub mod plugins;
mod recording;
mod streaming;

pub use midi::{MidiInputDevice, MidiInputEvent, MidiInputMessage, MidiInputStream};
pub use recording::{AudioInputDevice, AudioMonitorBuffer, AudioRecorder, RecordingSummary};
pub use streaming::{AudioSource, PlaybackSource, StreamedWav, StreamingPlayback};

use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering},
    },
    time::Duration,
};

use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use workstation_core::project::{
    ChannelEqualizer, EqualizerBand, FilterType, GeneratorWaveform, InsertEffect, InsertEffectKind,
    InstrumentWaveform,
};

use crate::plugins::test_generator::{TestGenerator, TestGeneratorParameters, TestWaveform};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const TEST_TONE_HZ: f32 = 440.0;
const TEST_TONE_LEVEL: f32 = 0.08;
const MAX_DECODED_WAV_BYTES: u64 = 512 * 1024 * 1024;

/// A fully decoded mono or stereo WAV file held in memory.
#[derive(Clone, Debug)]
pub struct DecodedWav {
    samples: Arc<[f32]>,
    waveform_peaks: Arc<Vec<f32>>,
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
        Ok(Self {
            samples: samples.into(),
            waveform_peaks: waveform_peaks.into(),
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
    pub source: PlaybackSource,
    pub start_beats: f64,
    pub gain: f32,
    pub muted: bool,
    pub solo: bool,
}

/// One scheduled note rendered by a Rust instrument processor.
#[derive(Clone, Copy, Debug)]
pub struct PlaybackMidiNote {
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
        tempo_bpm: f64,
        sample_rate: u32,
    ) -> Box<dyn InstrumentProcessor> {
        Box::new(MidiVoice::new(
            notes,
            controllers,
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
    processor: std::cell::UnsafeCell<ChannelProcessor>,
    meter_accumulator: std::cell::UnsafeCell<f32>,
}

impl PlaybackTrackRuntime {
    fn new(mut settings: PlaybackTrackSettings, tempo_bpm: f64, sample_rate: u32) -> Self {
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
        let processor = ChannelProcessor::new(
            &settings.equalizer,
            &settings.inserts,
            &settings.midi_notes,
            &settings.midi_controllers,
            settings.instrument_factory.as_ref(),
            tempo_bpm,
            sample_rate,
        );
        Self {
            settings,
            processor: std::cell::UnsafeCell::new(processor),
            meter_accumulator: std::cell::UnsafeCell::new(0.0),
        }
    }
}

/// Immutable channel-strip data assembled by the UI thread before publication.
#[derive(Clone)]
pub struct PlaybackTrackSettings {
    pub clips: Vec<PlaybackClip>,
    pub midi_notes: Vec<PlaybackMidiNote>,
    pub midi_controllers: Vec<PlaybackMidiController>,
    pub instrument_factory: Arc<dyn InstrumentFactory>,
    /// Atomic peak accumulator read and cleared by the UI thread.
    pub meter_peak: Arc<AtomicU32>,
    pub gain: f32,
    pub pan: f32,
    pub track_delay_ms: f32,
    pub muted: bool,
    pub solo: bool,
    pub solo_defeat: bool,
    pub equalizer: ChannelEqualizer,
    pub inserts: Vec<InsertEffect>,
}

// SAFETY: the only mutable access to the processor and peak accumulator is
// from the audio callback. The callback uses callback_active as a single-render
// gate, while the UI reads atomic peaks and replaces the ArcSwap snapshot.
unsafe impl Sync for PlaybackTrackRuntime {}

/// Immutable audio state read by the device callback.
pub struct PlaybackPlan {
    tempo_bpm: f64,
    tracks: Vec<PlaybackTrackRuntime>,
    any_solo: bool,
    loop_range_frames: Option<(u64, u64)>,
    generation: u64,
}

impl PlaybackPlan {
    fn empty() -> Self {
        Self {
            tempo_bpm: 120.0,
            tracks: Vec::new(),
            any_solo: false,
            loop_range_frames: None,
            generation: 0,
        }
    }

    fn new(
        tempo_bpm: f64,
        tracks: Vec<PlaybackTrackSettings>,
        generation: u64,
        sample_rate: u32,
        loop_range_beats: Option<(f64, f64)>,
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
        Self {
            tempo_bpm,
            tracks: tracks
                .into_iter()
                .map(|track| PlaybackTrackRuntime::new(track, tempo_bpm, sample_rate))
                .collect(),
            any_solo,
            loop_range_frames,
            generation,
        }
    }
}

/// A running output stream for the system's default audio device.
pub struct AudioEngine {
    _stream: cpal::Stream,
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
    callback_generation: Arc<AtomicU64>,
    callback_active: Arc<AtomicBool>,
    current_generation: u64,
    retired_plans: Vec<Arc<PlaybackPlan>>,
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioEngine {
    /// Opens and starts a silent stream on the default output device.
    pub fn open_default_output() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no default audio output device is available".to_owned())?;
        let device_name = device
            .description()
            .map(|description| description.name().to_owned())
            .unwrap_or_else(|_| "Default output device".to_owned());
        let supported_config = device
            .default_output_config()
            .map_err(|error| format!("could not read the output format: {error}"))?;
        let sample_format = supported_config.sample_format();
        let config: cpal::StreamConfig = supported_config.into();
        if config.channels == 0 || config.sample_rate == 0 {
            return Err("the default output device reports an invalid format".to_owned());
        }

        let channels = usize::from(config.channels);
        let output_channels = config.channels;
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
        let callback_generation = Arc::new(AtomicU64::new(0));
        let callback_active = Arc::new(AtomicBool::new(false));
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
                        render_f32(
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
                        );
                        live_midi.mix_f32(samples, channels, sample_rate);
                    },
                    |_| {},
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
                        );
                        live_midi.mix_i16(samples, channels, sample_rate);
                    },
                    |_| {},
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
                        );
                        live_midi.mix_u16(samples, channels, sample_rate);
                    },
                    |_| {},
                    Some(Duration::from_secs(5)),
                )
            }
            format => {
                return Err(format!(
                    "the default output sample format {format:?} is not supported yet"
                ));
            }
        }
        .map_err(|error| format!("could not open the default output stream: {error}"))?;

        stream
            .play()
            .map_err(|error| format!("could not start the default output stream: {error}"))?;

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
            callback_generation,
            callback_active,
            current_generation: 0,
            retired_plans: Vec::new(),
            device_name,
            sample_rate,
            channels: output_channels,
        })
    }

    pub fn set_playback_plan(
        &mut self,
        tempo_bpm: f64,
        tracks: Vec<PlaybackTrackSettings>,
        loop_range_beats: Option<(f64, f64)>,
    ) {
        self.current_generation = self.current_generation.wrapping_add(1).max(1);
        let new_plan = Arc::new(PlaybackPlan::new(
            tempo_bpm,
            tracks,
            self.current_generation,
            self.sample_rate,
            loop_range_beats,
        ));
        self.retired_plans.push(self.plan.swap(new_plan));
        self.reclaim_retired_plans();
    }

    /// Releases replaced snapshots after the audio callback has finished using them.
    pub fn reclaim_retired_plans(&mut self) {
        if self.callback_active.load(Ordering::SeqCst) {
            return;
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

    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
    }

    /// Connects or disconnects the lock-free live input-monitor buffer.
    pub fn set_input_monitor(&self, monitor: Option<Arc<AudioMonitorBuffer>>) {
        self.input_monitor.store(monitor);
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
                gain.clamp(0.0, 1.0).to_bits()
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
        let gain = f32::from_bits(self.gain.load(Ordering::Acquire)).clamp(0.0, 1.0);
        let (left, right) = self.synth.next_frame(sample_rate);
        (left * gain, right * gain)
    }

    fn mix_f32(&mut self, samples: &mut [f32], channels: usize, sample_rate: u32) {
        if channels == 0 {
            return;
        }
        for frame in samples.chunks_mut(channels) {
            let (left, right) = self.next_frame(sample_rate);
            mix_live_midi_frame(frame, channels, left, right, |sample, value| {
                *sample = (*sample + value).clamp(-1.0, 1.0);
            });
        }
    }

    fn mix_i16(&mut self, samples: &mut [i16], channels: usize, sample_rate: u32) {
        if channels == 0 {
            return;
        }
        for frame in samples.chunks_mut(channels) {
            let (left, right) = self.next_frame(sample_rate);
            mix_live_midi_frame(frame, channels, left, right, |sample, value| {
                let normalized = f32::from(*sample) / f32::from(i16::MAX);
                *sample = ((normalized + value).clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
            });
        }
    }

    fn mix_u16(&mut self, samples: &mut [u16], channels: usize, sample_rate: u32) {
        if channels == 0 {
            return;
        }
        for frame in samples.chunks_mut(channels) {
            let (left, right) = self.next_frame(sample_rate);
            mix_live_midi_frame(frame, channels, left, right, |sample, value| {
                let normalized = f32::from(*sample) / f32::from(u16::MAX) * 2.0 - 1.0;
                *sample = (((normalized + value).clamp(-1.0, 1.0) + 1.0)
                    * 0.5
                    * f32::from(u16::MAX)) as u16;
            });
        }
    }
}

fn mix_live_midi_frame<T>(
    frame: &mut [T],
    channels: usize,
    left: f32,
    right: f32,
    mut add: impl FnMut(&mut T, f32),
) {
    for (channel, sample) in frame.iter_mut().enumerate() {
        let value = match channel {
            0 if channels == 1 => (left + right) * 0.5,
            0 => left,
            1 => right,
            _ => 0.0,
        };
        if value != 0.0 {
            add(sample, value);
        }
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
        let input_step = f64::from(buffer.sample_rate()) / f64::from(output_rate.max(1));
        let next_cursor = cursor + input_step;
        self.cursor = Some(next_cursor);
        buffer.advance_read_to(next_cursor.floor() as u64);
        (
            first_left + (second_left - first_left) * fraction,
            first_right + (second_right - first_right) * fraction,
        )
    }
}

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
    );
}

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
    );
}

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
) {
    if callback_active
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        .is_err()
    {
        for frame in samples.chunks_mut(channels) {
            for (channel, sample) in frame.iter_mut().enumerate() {
                write_sample(sample, channel, 0.0, 0.0);
            }
        }
        return;
    }
    let plan_snapshot = plan.load_full();
    let monitor_snapshot = input_monitor.load_full();
    let generation = plan_snapshot.generation;
    let playing = playing.load(Ordering::Relaxed);
    let mut output_frame = wrap_playhead_frame(
        playhead.load(Ordering::Relaxed),
        plan_snapshot.loop_range_frames,
    );
    for frame in samples.chunks_mut(channels) {
        let (mut left, mut right) = if playing {
            mix_frame(&plan_snapshot, output_frame, sample_rate)
        } else {
            (0.0, 0.0)
        };
        let (monitor_left, monitor_right) = if let Some(monitor) = monitor_snapshot.as_ref() {
            monitor_reader.next_frame(monitor, sample_rate)
        } else {
            monitor_reader.reset();
            (0.0, 0.0)
        };
        left += monitor_left;
        right += monitor_right;
        if tone.load(Ordering::Relaxed) {
            let tone_sample = (*phase * std::f32::consts::TAU).sin() * TEST_TONE_LEVEL;
            left += tone_sample;
            right += tone_sample;
        }
        *phase += TEST_TONE_HZ / sample_rate as f32;
        if *phase >= 1.0 {
            *phase -= 1.0;
        }
        for (channel, sample) in frame.iter_mut().enumerate() {
            write_sample(sample, channel, left, right);
        }
        if playing {
            output_frame = wrap_playhead_frame(
                output_frame.saturating_add(1),
                plan_snapshot.loop_range_frames,
            );
        }
    }
    if playing {
        playhead.store(output_frame, Ordering::Relaxed);
    }
    publish_track_meters(&plan_snapshot);
    drop(plan_snapshot);
    callback_generation.store(generation, Ordering::Release);
    callback_active.store(false, Ordering::SeqCst);
}

fn publish_track_meters(plan: &PlaybackPlan) {
    for track in &plan.tracks {
        // SAFETY: render_frames is the only caller and callback_active admits
        // only one callback that can mutate each accumulator.
        let peak = unsafe {
            let accumulator = &mut *track.meter_accumulator.get();
            let peak = *accumulator;
            *accumulator = 0.0;
            peak
        };
        if peak.is_finite() && peak > 0.0 {
            track
                .settings
                .meter_peak
                .fetch_max(peak.min(4.0).to_bits(), Ordering::Relaxed);
        }
    }
}

fn wrap_playhead_frame(frame: u64, loop_range: Option<(u64, u64)>) -> u64 {
    let Some((start_frame, end_frame)) = loop_range.filter(|(start, end)| end > start) else {
        return frame;
    };
    if frame < end_frame {
        return frame;
    }
    start_frame + (frame - end_frame) % (end_frame - start_frame)
}

fn mix_frame(plan: &PlaybackPlan, output_frame: u64, output_rate: u32) -> (f32, f32) {
    let timeline_beats = output_frame as f64 * plan.tempo_bpm / (f64::from(output_rate) * 60.0);
    let mut left = 0.0;
    let mut right = 0.0;
    for track in &plan.tracks {
        if track.settings.muted
            || (plan.any_solo && !track.settings.solo && !track.settings.solo_defeat)
        {
            continue;
        }
        let (mut track_left, mut track_right) = mix_track_frame(plan, track, timeline_beats);
        // SAFETY: callback_active permits exactly one callback to mutate the
        // processor state; the UI never accesses this cell.
        let processor = unsafe { &mut *track.processor.get() };
        let (instrument_left, instrument_right) = processor.instrument.next_frame(output_frame);
        track_left += instrument_left;
        track_right += instrument_right;
        processor.process(&mut track_left, &mut track_right);
        // Linear stereo balance preserves unity at center, with the opposite
        // side attenuated toward either edge. Exact reference pan law is unmeasured.
        let output_left = track_left * track.settings.gain * (1.0 - track.settings.pan.max(0.0));
        let output_right = track_right * track.settings.gain * (1.0 + track.settings.pan.min(0.0));
        let track_peak = output_left.abs().max(output_right.abs());
        if track_peak.is_finite() {
            // SAFETY: mix_frame only runs in the serialized audio callback.
            let accumulator = unsafe { &mut *track.meter_accumulator.get() };
            *accumulator = accumulator.max(track_peak);
        }
        left += output_left;
        right += output_right;
    }
    (left, right)
}

fn mix_track_frame(
    plan: &PlaybackPlan,
    track: &PlaybackTrackRuntime,
    timeline_beats: f64,
) -> (f32, f32) {
    let mut left = 0.0;
    let mut right = 0.0;
    for clip in &track.settings.clips {
        if clip.muted {
            continue;
        }
        let source_seconds = (timeline_beats - clip.start_beats) * 60.0 / plan.tempo_bpm;
        if source_seconds < 0.0 {
            continue;
        }
        let source_frame = source_seconds * f64::from(clip.source.sample_rate());
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
        left += (first_left * (1.0 - fraction) + second_left * fraction) * clip.gain;
        right += (first_right * (1.0 - fraction) + second_right * fraction) * clip.gain;
    }
    (left, right)
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
    velocity: f32,
}

#[derive(Clone, Copy)]
struct ActiveMidiVoice {
    start_frame: u64,
    end_frame: u64,
    frequency_hz: f32,
    channel: u8,
    velocity: f32,
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
    waveform: InstrumentWaveform,
}

impl MidiVoice {
    fn new(
        notes: &[PlaybackMidiNote],
        controllers: &[PlaybackMidiController],
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
                    velocity: note.velocity.clamp(0.0, 1.0),
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
            waveform,
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
                velocity: note.velocity,
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
    equalizer: [Biquad; 4],
    inserts: Vec<InsertProcessor>,
    instrument: Box<dyn InstrumentProcessor>,
}

impl ChannelProcessor {
    fn new(
        equalizer: &ChannelEqualizer,
        inserts: &[InsertEffect],
        midi_notes: &[PlaybackMidiNote],
        midi_controllers: &[PlaybackMidiController],
        instrument_factory: &dyn InstrumentFactory,
        tempo_bpm: f64,
        sample_rate: u32,
    ) -> Self {
        let bands =
            std::array::from_fn(|index| Biquad::from_band(&equalizer.bands[index], sample_rate));
        let inserts = inserts
            .iter()
            .map(|insert| InsertProcessor::new(insert, sample_rate))
            .collect();
        Self {
            equalizer_bypassed: equalizer.bypassed,
            equalizer: bands,
            inserts,
            instrument: instrument_factory.create_processor(
                midi_notes,
                midi_controllers,
                tempo_bpm,
                sample_rate,
            ),
        }
    }

    fn process(&mut self, left: &mut f32, right: &mut f32) {
        if !self.equalizer_bypassed {
            for band in &mut self.equalizer {
                if band.enabled {
                    band.process(left, right);
                }
            }
        }
        for insert in &mut self.inserts {
            if insert.enabled() {
                insert.process(left, right);
            }
        }
    }
}

struct Biquad {
    enabled: bool,
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    left_z1: f32,
    left_z2: f32,
    right_z1: f32,
    right_z2: f32,
}

impl Biquad {
    fn from_band(band: &EqualizerBand, sample_rate: u32) -> Self {
        let sr = sample_rate.max(1) as f32;
        let frequency = band.frequency_hz.clamp(1.0, sr * 0.49);
        let q = band.q.clamp(0.1, 10.0);
        let omega = std::f32::consts::TAU * frequency / sr;
        let (sin, cos) = omega.sin_cos();
        let a = 10.0_f32.powf(band.gain_db / 40.0);
        let alpha = sin / (2.0 * q);
        let (b0, b1, b2, a0, a1, a2) = match band.filter {
            FilterType::Bell => (
                1.0 + alpha * a,
                -2.0 * cos,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos,
                1.0 - alpha / a,
            ),
            FilterType::LowPass => (
                (1.0 - cos) * 0.5,
                1.0 - cos,
                (1.0 - cos) * 0.5,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterType::HighPass => (
                (1.0 + cos) * 0.5,
                -(1.0 + cos),
                (1.0 + cos) * 0.5,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterType::LowShelf | FilterType::HighShelf => {
                let slope = (1.0 / q).clamp(0.1, 10.0);
                let shelf_alpha =
                    sin * 0.5 * ((a + 1.0 / a) * (1.0 / slope - 1.0) + 2.0).max(0.0).sqrt();
                let beta = 2.0 * a.sqrt() * shelf_alpha;
                if band.filter == FilterType::LowShelf {
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
            left_z1: 0.0,
            left_z2: 0.0,
            right_z1: 0.0,
            right_z2: 0.0,
        }
    }

    fn process(&mut self, left: &mut f32, right: &mut f32) {
        let input_left = *left;
        let output_left = self.b0 * input_left + self.left_z1;
        self.left_z1 = self.b1 * input_left - self.a1 * output_left + self.left_z2;
        self.left_z2 = self.b2 * input_left - self.a2 * output_left;
        let input_right = *right;
        let output_right = self.b0 * input_right + self.right_z1;
        self.right_z1 = self.b1 * input_right - self.a1 * output_right + self.right_z2;
        self.right_z2 = self.b2 * input_right - self.a2 * output_right;
        *left = output_left;
        *right = output_right;
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
    Compressor {
        enabled: bool,
        threshold_db: f32,
        ratio: f32,
        makeup_gain: f32,
        attack_coefficient: f32,
        release_coefficient: f32,
        detector_envelope: f32,
    },
}

#[cfg(test)]
mod processing_tests {
    use super::*;

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
        let mut biquad = Biquad::from_band(&band, sample_rate as u32);
        let mut input_energy = 0.0_f64;
        let mut output_energy = 0.0_f64;
        for index in 0..48_000 {
            let input =
                (std::f32::consts::TAU * input_frequency_hz * index as f32 / sample_rate).sin();
            let mut left = input;
            let mut right = input;
            biquad.process(&mut left, &mut right);
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
    fn compressor_reduces_levels_and_preserves_stereo_balance() {
        let effect = InsertEffect {
            enabled: true,
            kind: InsertEffectKind::Compressor {
                threshold_db: -18.0,
                ratio: 4.0,
                attack_ms: 1.0,
                hold_ms: 1.0,
                release_ms: 100.0,
                analysis: 80.0,
                makeup_db: 0.0,
                soft_knee: true,
                auto_makeup: true,
                auto_release: false,
                rms_mode: false,
                live: true,
            },
        };
        let mut compressor = InsertProcessor::new(&effect, 48_000);
        let mut left = 0.5;
        let mut right = 0.25;
        for _ in 0..5_000 {
            left = 0.5;
            right = 0.25;
            compressor.process(&mut left, &mut right);
        }

        assert!(
            (0.175..=0.181).contains(&left),
            "compressed level was {left}"
        );
        assert!((left / right - 2.0).abs() < 1.0e-5);
    }

    #[test]
    fn disabled_eq_and_empty_insert_chain_leave_audio_unchanged() {
        let instrument_factory = oscillator_instrument_factory(InstrumentWaveform::Sine);
        let mut processor = ChannelProcessor::new(
            &ChannelEqualizer::default(),
            &[],
            &[],
            &[],
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
            start_beats: 1.0,
            duration_beats: 1.0,
            frequency_hz: 1_000.0,
            channel: 0,
            velocity: 1.0,
        };
        let mut voice = MidiVoice::new(&[note], &[], 120.0, 48_000, InstrumentWaveform::Sine);

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
                start_beats: 0.0,
                duration_beats: 4.0,
                frequency_hz: 440.0,
                channel: 0,
                velocity: 0.5,
            };
            MAX_ACTIVE_MIDI_VOICES + 1
        ];
        let mut voice = MidiVoice::new(&notes, &[], 120.0, 48_000, InstrumentWaveform::Sine);
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
    fn midi_controllers_change_voice_volume_and_pan_and_restore_after_seek() {
        let note = PlaybackMidiNote {
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
        let mut voice = MidiVoice::new(
            &[note],
            &controllers,
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
        let mut voice = MidiVoice::new(&[], &controllers, 120.0, 48_000, InstrumentWaveform::Sine);

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
        let mut voice = MidiVoice::new(&[], &controllers, 120.0, 48_000, InstrumentWaveform::Sine);

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
        assert_eq!(reader.next_frame(&monitor, 48_000), (0.5, -0.5));
        assert_eq!(reader.next_frame(&monitor, 48_000), (1.0, -1.0));

        monitor.set_enabled(false);
        assert_eq!(reader.next_frame(&monitor, 48_000), (0.0, 0.0));
        monitor.set_enabled(true);
        assert!(monitor.push_frame(-1.0, 1.0));
        assert!(monitor.push_frame(0.0, 0.0));
        assert_eq!(reader.next_frame(&monitor, 48_000), (-1.0, 1.0));
        monitor.set_enabled(false);
        monitor.set_enabled(true);
        assert!(monitor.push_frame(0.5, 0.5));
        assert!(monitor.push_frame(0.75, 0.75));
        assert_eq!(reader.next_frame(&monitor, 48_000), (0.5, 0.5));
    }

    #[test]
    fn output_callback_mixes_live_input_when_transport_is_stopped() {
        let monitor = Arc::new(AudioMonitorBuffer::new(48_000));
        monitor.set_enabled(true);
        assert!(monitor.push_frame(0.25, -0.25));
        assert!(monitor.push_frame(0.5, -0.5));
        assert!(monitor.push_frame(0.75, -0.75));
        let input_monitor = Arc::new(ArcSwapOption::empty());
        input_monitor.store(Some(Arc::clone(&monitor)));
        let plan = ArcSwap::from(Arc::new(PlaybackPlan::empty()));
        let tone = AtomicBool::new(false);
        let playing = AtomicBool::new(false);
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

        assert_eq!(samples, [0.25, -0.25, 0.5, -0.5]);
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

        let meter_peak = Arc::new(AtomicU32::new(0));
        let plan = ArcSwap::from(Arc::new(PlaybackPlan::new(
            120.0,
            vec![PlaybackTrackSettings {
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
        assert_eq!(f32::from_bits(meter_peak.swap(0, Ordering::Relaxed)), 0.25);
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
            InsertEffectKind::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                makeup_db,
                ..
            } => {
                let rate = sample_rate.max(1) as f32;
                Self::Compressor {
                    enabled: insert.enabled,
                    threshold_db: *threshold_db,
                    ratio: *ratio,
                    makeup_gain: 10.0_f32.powf(*makeup_db / 20.0),
                    attack_coefficient: (-1.0 / (rate * attack_ms * 0.001)).exp(),
                    release_coefficient: (-1.0 / (rate * release_ms * 0.001)).exp(),
                    detector_envelope: 0.0,
                }
            }
        }
    }

    fn enabled(&self) -> bool {
        match self {
            Self::TestGenerator { enabled, .. }
            | Self::StereoDelay { enabled, .. }
            | Self::Compressor { enabled, .. } => *enabled,
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
            Self::Compressor {
                threshold_db,
                ratio,
                makeup_gain,
                attack_coefficient,
                release_coefficient,
                detector_envelope,
                ..
            } => {
                // A single peak detector keeps the stereo image centered while
                // attack and release smoothing remain bounded and allocation-free.
                let input_peak = left.abs().max(right.abs());
                let coefficient = if input_peak > *detector_envelope {
                    *attack_coefficient
                } else {
                    *release_coefficient
                };
                *detector_envelope =
                    coefficient * *detector_envelope + (1.0 - coefficient) * input_peak;
                let level_db = 20.0 * detector_envelope.max(1.0e-9).log10();
                let reduction_db = if level_db > *threshold_db {
                    *threshold_db + (level_db - *threshold_db) / *ratio - level_db
                } else {
                    0.0
                };
                let gain = *makeup_gain * 10.0_f32.powf(reduction_db / 20.0);
                *left *= gain;
                *right *= gain;
            }
        }
    }
}
