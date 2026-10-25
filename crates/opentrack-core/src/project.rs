//! The application's native, version-independent project domain model.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A track's content and editing domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Audio,
    Instrument,
    Midi,
}

/// Stable identities for the two device bus strips, separate from arrange tracks.
pub const INPUT_BUS_ID: u64 = u64::MAX - 1;
pub const OUTPUT_BUS_ID: u64 = u64::MAX;

/// Reuses the existing channel-strip settings and editor without adding an arrange lane.
/// Bus strips cannot contain media, arm recording, or monitor themselves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BusChannel {
    pub strip: Track,
}
impl BusChannel {
    pub fn input() -> Self {
        Self::new(INPUT_BUS_ID, "Stereo In")
    }
    pub fn output() -> Self {
        Self::new(OUTPUT_BUS_ID, "Stereo Out")
    }
    fn new(id: u64, name: &str) -> Self {
        let strip = Track {
            id,
            name: name.into(),
            kind: TrackKind::Audio,
            color_index: None,
            audio_channels: 2,
            muted: false,
            solo: false,
            solo_defeat: false,
            listen: false,
            armed: false,
            monitor: false,
            automation_read: false,
            automation_write: false,
            input_gain_db: 0.0,
            input_phase_inverted: false,
            gain: 1.0,
            pan: 0.0,
            track_delay_ms: 0.0,
            clips: Vec::new(),
            equalizer: ChannelEqualizer::default(),
            inserts: Vec::new(),
            inserts_bypassed: false,
            sends_bypassed: false,
            input_routing: None,
            output_routing: None,
            send_routing: Vec::new(),
            instrument_waveform: InstrumentWaveform::default(),
            midi_parts: Vec::new(),
        };
        Self { strip }
    }
}

/// One channel and arrange lane in a project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub name: String,
    pub kind: TrackKind,
    /// Original UI palette selection; absent in older RWP files.
    #[serde(default)]
    pub color_index: Option<u8>,
    /// Configured audio channel width. Non-audio tracks use two channels.
    #[serde(default = "default_audio_channels")]
    pub audio_channels: u8,
    pub muted: bool,
    pub solo: bool,
    /// Keeps the channel audible while other channels are soloed.
    #[serde(default)]
    pub solo_defeat: bool,
    /// Listen-state toggle presented in the mixer and channel strip.
    #[serde(default)]
    pub listen: bool,
    pub armed: bool,
    /// Input monitoring toggle presented by the audio channel strip.
    #[serde(default)]
    pub monitor: bool,
    /// Automation Read control state; processing remains a separate feature.
    #[serde(default)]
    pub automation_read: bool,
    /// Automation Write control state; processing remains a separate feature.
    #[serde(default)]
    pub automation_write: bool,
    /// Input gain in dB, persisted for the channel input control.
    #[serde(default)]
    pub input_gain_db: f32,
    /// Polarity switch for the channel input.
    #[serde(default)]
    pub input_phase_inverted: bool,
    /// Linear gain in the range 0.0 to 1.0.
    pub gain: f32,
    /// Stereo balance: -1 left, 0 center, 1 right.
    #[serde(default)]
    pub pan: f32,
    /// Playback offset in milliseconds; positive values play later.
    #[serde(default)]
    pub track_delay_ms: f32,
    /// Audio clips arranged on this track. Non-audio tracks keep this empty.
    #[serde(default)]
    pub clips: Vec<AudioClip>,
    /// Built-in four-band channel EQ state.
    #[serde(default)]
    pub equalizer: ChannelEqualizer,
    /// Ordered audio insert effects.
    #[serde(default)]
    pub inserts: Vec<InsertEffect>,
    /// Bypasses every insert on this channel without changing per-insert state.
    #[serde(default)]
    pub inserts_bypassed: bool,
    /// Send rack bypass state. Routing and send processing remain independent.
    #[serde(default)]
    pub sends_bypassed: bool,
    /// Selected channel input bus. `None` represents No Bus.
    #[serde(default)]
    pub input_routing: Option<String>,
    /// Selected channel output bus. `None` represents No Bus.
    #[serde(default)]
    pub output_routing: Option<String>,
    /// Eight optional send destinations. Empty entries represent No Bus.
    #[serde(default)]
    pub send_routing: Vec<Option<String>>,
    /// Selected built-in voice for an Instrument track.
    #[serde(default)]
    pub instrument_waveform: InstrumentWaveform,
    /// Arrangeable MIDI parts on MIDI and instrument tracks.
    #[serde(default)]
    pub midi_parts: Vec<MidiPart>,
}

fn default_audio_channels() -> u8 {
    2
}

/// Built-in channel EQ. The four-band layout follows the Cubase 5 manual;
/// parameter ranges and DSP behavior remain OpenTrack- decisions pending
/// reference measurements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChannelEqualizer {
    pub bypassed: bool,
    pub bands: [EqualizerBand; 4],
}

impl Default for ChannelEqualizer {
    fn default() -> Self {
        Self {
            bypassed: false,
            bands: [
                EqualizerBand::with_q(FilterType::LowShelf, 100.0, 7.0),
                EqualizerBand::with_q(FilterType::Bell, 800.0, 0.2),
                EqualizerBand::with_q(FilterType::Bell, 2_000.0, 0.2),
                EqualizerBand::with_q(FilterType::HighShelf, 12_000.0, 7.0),
            ],
        }
    }
}

/// One independently enabled filter in the built-in channel EQ.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EqualizerBand {
    pub enabled: bool,
    pub filter: FilterType,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl EqualizerBand {
    fn new(filter: FilterType, frequency_hz: f32) -> Self {
        Self::with_q(filter, frequency_hz, 0.707)
    }

    fn with_q(filter: FilterType, frequency_hz: f32, q: f32) -> Self {
        Self {
            enabled: false,
            filter,
            frequency_hz,
            gain_db: 0.0,
            q,
        }
    }
}

impl Default for EqualizerBand {
    fn default() -> Self {
        Self::new(FilterType::Bell, 1_000.0)
    }
}

/// Filter choices offered by the initial channel EQ implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterType {
    Bell,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    ParametricI,
    ParametricII,
    LowShelfI,
    LowShelfII,
    LowShelfIII,
    LowShelfIV,
    HighPassI,
    HighPassII,
}

/// Oscillator shape for OpenTrack-'s starter built-in instrument.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentWaveform {
    #[default]
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

/// Signal choices exposed by the TestGenerator insert.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratorWaveform {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    WhiteNoise,
    BrownNoise,
    PinkNoise,
}

/// A persisted built-in insert effect. These are OpenTrack- processors; a
/// Cubase plug-in name is used only after its implementation is measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InsertEffect {
    pub enabled: bool,
    pub kind: InsertEffectKind,
}

/// Effect implementations available in the first processing milestone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InsertEffectKind {
    TestGenerator {
        waveform: GeneratorWaveform,
        frequency_hz: f32,
        gain_db: f32,
    },
    StereoDelay {
        time_ms: f32,
        feedback: f32,
        mix: f32,
    },
    Compressor {
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        #[serde(default = "default_compressor_hold_ms")]
        hold_ms: f32,
        release_ms: f32,
        #[serde(default = "default_compressor_analysis")]
        analysis: f32,
        makeup_db: f32,
        #[serde(default = "default_true")]
        soft_knee: bool,
        #[serde(default = "default_true")]
        auto_makeup: bool,
        #[serde(default)]
        auto_release: bool,
        #[serde(default)]
        rms_mode: bool,
        #[serde(default = "default_true")]
        live: bool,
    },
}

fn default_compressor_hold_ms() -> f32 {
    1.0
}

fn default_compressor_analysis() -> f32 {
    80.0
}

fn default_true() -> bool {
    true
}

impl Default for InsertEffectKind {
    fn default() -> Self {
        Self::StereoDelay {
            time_ms: 300.0,
            feedback: 0.25,
            mix: 0.25,
        }
    }
}

/// One reference to external audio placed on an audio track.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioClip {
    pub id: u64,
    pub name: String,
    pub source_path: PathBuf,
    pub start_beats: f64,
    pub duration_seconds: f64,
    /// Non-destructive source trim and linear amplitude fades, in seconds.
    #[serde(default)]
    pub source_offset_seconds: f64,
    #[serde(default)]
    pub fade_in_seconds: f64,
    #[serde(default)]
    pub fade_out_seconds: f64,
    /// Linear event gain, 0.0 through 4.0 (approximately +12 dB).
    pub gain: f32,
}

/// A MIDI event container arranged on the project beat timeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiPart {
    pub id: u64,
    pub name: String,
    pub start_beats: f64,
    pub duration_beats: f64,
    pub notes: Vec<MidiNote>,
    /// Beat-based controller changes stored relative to this part's start.
    #[serde(default)]
    pub controller_events: Vec<MidiControllerEvent>,
}

/// A MIDI note event positioned relative to the start of its MIDI part.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiNote {
    pub id: u64,
    pub start_beats: f64,
    pub duration_beats: f64,
    /// MIDI key number in the range 0 through 127.
    pub pitch: u8,
    /// MIDI channel number in the range 0 through 15. Older projects use channel 0.
    #[serde(default)]
    pub channel: u8,
    /// MIDI velocity in the range 1 through 127; zero is reserved for note-off.
    pub velocity: u8,
}

/// A MIDI control-change event positioned relative to its MIDI part.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiControllerEvent {
    pub id: u64,
    pub position_beats: f64,
    /// MIDI channel number in the range 0 through 15. Older projects use channel 0.
    #[serde(default)]
    pub channel: u8,
    /// MIDI controller number in the range 0 through 127.
    pub controller: u8,
    /// MIDI controller value in the range 0 through 127.
    pub value: u8,
}

/// A project owned by this application. This is not a Cubase `.cpr` model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub tracks: Vec<Track>,
    #[serde(default = "BusChannel::input")]
    pub input_channel: BusChannel,
    #[serde(default = "BusChannel::output")]
    pub output_channel: BusChannel,
    pub selected_track: Option<u32>,
    pub next_track_number: u32,
    pub next_track_id: u64,
    #[serde(default = "first_clip_id")]
    pub next_clip_id: u64,
    #[serde(default = "first_midi_part_id")]
    pub next_midi_part_id: u64,
    #[serde(default)]
    pub selected_midi_part_id: Option<u64>,
    #[serde(default = "first_midi_note_id")]
    pub next_midi_note_id: u64,
    #[serde(default = "first_midi_controller_id")]
    pub next_midi_controller_id: u64,
    pub tempo_bpm: f64,
    /// Project timeline and recorded-media rate. Missing fields in legacy
    /// projects retain the historical 44.1 kHz interpretation.
    #[serde(default = "legacy_project_sample_rate")]
    pub sample_rate_hz: u32,
    /// Encoding for newly recorded WAV takes. Older projects retain the
    /// historical 32-bit float recording behavior.
    #[serde(default = "legacy_recording_format")]
    pub recording_format: RecordingFormat,
    pub beats_per_bar: u8,
    pub beat_unit: u8,
    #[serde(default = "default_loop_start_beats")]
    pub loop_start_beats: f64,
    #[serde(default = "default_loop_end_beats")]
    pub loop_end_beats: f64,
    #[serde(default)]
    pub cycle_mode: CycleMode,
}

/// The locator direction determines whether Cycle repeats or skips the range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CycleMode {
    #[default]
    Loop,
    Skip,
}

/// WAV sample representation used for new recordings in a project.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingFormat {
    Pcm16,
    Pcm24,
    #[default]
    Float32,
}

impl RecordingFormat {
    pub const fn bits_per_sample(self) -> u16 {
        match self {
            Self::Pcm16 => 16,
            Self::Pcm24 => 24,
            Self::Float32 => 32,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pcm16 => "16-bit PCM",
            Self::Pcm24 => "24-bit PCM",
            Self::Float32 => "32-bit float",
        }
    }
}

fn legacy_project_sample_rate() -> u32 {
    44_100
}

fn legacy_recording_format() -> RecordingFormat {
    RecordingFormat::Float32
}

fn default_new_project_sample_rate() -> u32 {
    48_000
}

fn default_loop_start_beats() -> f64 {
    0.0
}

fn default_loop_end_beats() -> f64 {
    16.0
}

const MAX_LOOP_BEATS: f64 = 1_000_000.0;

impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Untitled Project".to_owned(),
            tracks: Vec::new(),
            input_channel: BusChannel::input(),
            output_channel: BusChannel::output(),
            selected_track: None,
            next_track_number: 0,
            next_track_id: 1,
            next_clip_id: 1,
            next_midi_part_id: 1,
            selected_midi_part_id: None,
            next_midi_note_id: 1,
            next_midi_controller_id: 1,
            tempo_bpm: 120.0,
            sample_rate_hz: default_new_project_sample_rate(),
            recording_format: RecordingFormat::Pcm24,
            beats_per_bar: 4,
            beat_unit: 4,
            loop_start_beats: default_loop_start_beats(),
            loop_end_beats: default_loop_end_beats(),
            cycle_mode: CycleMode::Loop,
        }
    }
}

impl Project {
    pub fn channel_at(&self, index: usize) -> Option<&Track> {
        if index == self.tracks.len() {
            Some(&self.input_channel.strip)
        } else if index == self.tracks.len() + 1 {
            Some(&self.output_channel.strip)
        } else {
            self.tracks.get(index)
        }
    }
    pub fn channel_at_mut(&mut self, index: usize) -> Option<&mut Track> {
        if index == self.tracks.len() {
            Some(&mut self.input_channel.strip)
        } else if index == self.tracks.len() + 1 {
            Some(&mut self.output_channel.strip)
        } else {
            self.tracks.get_mut(index)
        }
    }
    pub fn channels(&self) -> impl Iterator<Item = &Track> {
        self.tracks
            .iter()
            .chain([&self.input_channel.strip, &self.output_channel.strip])
    }
    pub fn channel_by_id_mut(&mut self, id: u64) -> Option<&mut Track> {
        match id {
            INPUT_BUS_ID => Some(&mut self.input_channel.strip),
            OUTPUT_BUS_ID => Some(&mut self.output_channel.strip),
            _ => self.tracks.iter_mut().find(|t| t.id == id),
        }
    }
    /// Sets the persisted cycle locator range in quarter-note beats.
    pub fn set_loop_range(&mut self, start_beats: f64, end_beats: f64) -> bool {
        if !start_beats.is_finite()
            || start_beats < 0.0
            || !end_beats.is_finite()
            || end_beats > MAX_LOOP_BEATS
            || end_beats <= start_beats
        {
            return false;
        }
        self.loop_start_beats = start_beats;
        self.loop_end_beats = end_beats;
        self.cycle_mode = CycleMode::Loop;
        true
    }

    /// Commits a directional ruler range in quarter-note beats. Bounds are
    /// stored in ascending order; the mode preserves the drawing direction.
    pub fn set_cycle_range(&mut self, anchor_beats: f64, endpoint_beats: f64) -> bool {
        if !anchor_beats.is_finite() || !endpoint_beats.is_finite() {
            return false;
        }
        if !self.set_loop_range(
            anchor_beats.min(endpoint_beats),
            anchor_beats.max(endpoint_beats),
        ) {
            return false;
        }
        self.cycle_mode = if endpoint_beats > anchor_beats {
            CycleMode::Loop
        } else {
            CycleMode::Skip
        };
        true
    }

    /// Applies the active cycle to a running transport position. Skip uses
    /// the half-open interval [start, end), so playback resumes at end.
    pub fn cycle_position_beats(&self, position: f64) -> f64 {
        match self.cycle_mode {
            CycleMode::Loop if position >= self.loop_end_beats => {
                self.loop_start_beats
                    + (position - self.loop_end_beats)
                        .rem_euclid(self.loop_end_beats - self.loop_start_beats)
            }
            CycleMode::Skip
                if position >= self.loop_start_beats && position < self.loop_end_beats =>
            {
                self.loop_end_beats
            }
            _ => position,
        }
    }

    /// Adds a uniquely identified lane and selects it.
    pub fn add_track(&mut self, kind: TrackKind) {
        self.add_track_with_audio_channels(kind, 2);
    }

    /// Adds a uniquely identified lane with an explicit mono/stereo choice
    /// for audio tracks. Audio routing remains a separate engine concern.
    pub fn add_track_with_audio_channels(&mut self, kind: TrackKind, audio_channels: u8) {
        self.next_track_number = self.next_track_number.saturating_add(1);
        let id = self.next_track_id;
        self.next_track_id = self.next_track_id.saturating_add(1);

        let prefix = match kind {
            TrackKind::Audio => "Audio",
            TrackKind::Instrument => "Instrument",
            TrackKind::Midi => "MIDI",
        };

        let audio_channels = if kind == TrackKind::Audio {
            audio_channels.clamp(1, 2)
        } else {
            2
        };
        self.tracks.push(Track {
            id,
            name: format!("{prefix} {:02}", self.next_track_number),
            kind,
            color_index: None,
            audio_channels,
            muted: false,
            solo: false,
            solo_defeat: false,
            listen: false,
            armed: false,
            monitor: false,
            automation_read: false,
            automation_write: false,
            input_gain_db: 0.0,
            input_phase_inverted: false,
            gain: 1.0,
            pan: 0.0,
            track_delay_ms: 0.0,
            clips: Vec::new(),
            equalizer: ChannelEqualizer::default(),
            inserts: Vec::new(),
            inserts_bypassed: false,
            sends_bypassed: false,
            input_routing: if kind == TrackKind::Audio {
                Some(if audio_channels == 1 {
                    "Left - Stereo In".to_owned()
                } else {
                    "Stereo In".to_owned()
                })
            } else {
                None
            },
            output_routing: Some("Stereo Out".to_owned()),
            send_routing: Vec::new(),
            instrument_waveform: InstrumentWaveform::default(),
            midi_parts: Vec::new(),
        });
        self.selected_track = u32::try_from(self.tracks.len() - 1).ok();
        self.selected_midi_part_id = None;
    }

    /// Inserts a new track immediately below the selected track, or at the
    /// end when nothing is selected. Returns the inserted row index.
    pub fn insert_track_below_selection(&mut self, kind: TrackKind, audio_channels: u8) -> usize {
        let insertion_index = self
            .selected_track
            .map(|index| index as usize + 1)
            .unwrap_or(self.tracks.len())
            .min(self.tracks.len());
        self.add_track_with_audio_channels(kind, audio_channels);
        let track = self
            .tracks
            .pop()
            .expect("adding a track always appends one track");
        self.tracks.insert(insertion_index, track);
        self.selected_track = u32::try_from(insertion_index).ok();
        self.selected_midi_part_id = None;
        insertion_index
    }

    /// Renames a track after trimming surrounding whitespace.
    pub fn rename_track(&mut self, track_index: usize, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        let Some(track) = self.tracks.get_mut(track_index) else {
            return false;
        };
        if track.name == name {
            return false;
        }
        track.name = name.to_owned();
        true
    }

    /// Removes a track, selecting the nearest remaining track when needed.
    pub fn remove_track(&mut self, track_index: usize) -> Option<Track> {
        if track_index >= self.tracks.len() {
            return None;
        }
        let selected_track_id = self
            .selected_track
            .and_then(|index| self.tracks.get(index as usize))
            .map(|track| track.id);
        let removed = self.tracks.remove(track_index);

        self.selected_track = selected_track_id
            .and_then(|id| self.tracks.iter().position(|track| track.id == id))
            .or_else(|| (!self.tracks.is_empty()).then_some(track_index.min(self.tracks.len() - 1)))
            .and_then(|index| u32::try_from(index).ok());

        let selected_part_is_valid = self
            .selected_track
            .and_then(|index| self.tracks.get(index as usize))
            .is_some_and(|track| {
                self.selected_midi_part_id
                    .is_some_and(|part_id| track.midi_parts.iter().any(|part| part.id == part_id))
            });
        if !selected_part_is_valid {
            self.selected_midi_part_id = None;
        }
        Some(removed)
    }

    /// Reorders a track while preserving selection by stable track ID.
    pub fn move_track(&mut self, from: usize, to: usize) -> bool {
        if from >= self.tracks.len() || to >= self.tracks.len() || from == to {
            return false;
        }
        let selected_track_id = self
            .selected_track
            .and_then(|index| self.tracks.get(index as usize))
            .map(|track| track.id);
        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);
        self.selected_track = selected_track_id
            .and_then(|id| self.tracks.iter().position(|track| track.id == id))
            .and_then(|index| u32::try_from(index).ok());
        true
    }

    /// Commit an event edit without changing source identity or audio-track width.
    pub fn edit_audio_clip(
        &mut self,
        source_track_id: u64,
        target_track_id: u64,
        edited: AudioClip,
    ) -> bool {
        let Some(source) = self.tracks.iter().position(|t| t.id == source_track_id) else {
            return false;
        };
        let Some(target) = self.tracks.iter().position(|t| t.id == target_track_id) else {
            return false;
        };
        if self.tracks[source].kind != TrackKind::Audio
            || self.tracks[target].kind != TrackKind::Audio
            || self.tracks[source].audio_channels != self.tracks[target].audio_channels
        {
            return false;
        }
        let Some(index) = self.tracks[source]
            .clips
            .iter()
            .position(|c| c.id == edited.id && c.source_path == edited.source_path)
        else {
            return false;
        };
        if self.tracks[source].clips[index] == edited && source == target {
            return false;
        }
        let mut candidate = self.clone();
        candidate.tracks[source].clips.remove(index);
        candidate.tracks[target].clips.push(edited);
        if candidate.validate().is_err() {
            return false;
        }
        *self = candidate;
        true
    }

    /// Adds an external audio file to an audio track at the given beat.
    pub fn add_audio_clip(
        &mut self,
        track_index: usize,
        source_path: PathBuf,
        start_beats: f64,
        duration_seconds: f64,
    ) -> Option<u64> {
        if source_path.as_os_str().is_empty()
            || !start_beats.is_finite()
            || start_beats < 0.0
            || !duration_seconds.is_finite()
            || duration_seconds <= 0.0
            || self.next_clip_id == u64::MAX
        {
            return None;
        }
        let track = self.tracks.get_mut(track_index)?;
        if track.kind != TrackKind::Audio {
            return None;
        }

        let id = self.next_clip_id;
        self.next_clip_id += 1;
        let name = source_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| "Audio Clip".to_owned());
        track.clips.push(AudioClip {
            id,
            name,
            source_path,
            start_beats,
            duration_seconds,
            source_offset_seconds: 0.0,
            fade_in_seconds: 0.0,
            fade_out_seconds: 0.0,
            gain: 1.0,
        });
        Some(id)
    }

    /// Adds a MIDI part to a MIDI or instrument track.
    pub fn add_midi_part(
        &mut self,
        track_index: usize,
        start_beats: f64,
        duration_beats: f64,
    ) -> Option<u64> {
        if !start_beats.is_finite()
            || start_beats < 0.0
            || !duration_beats.is_finite()
            || duration_beats <= 0.0
            || !(start_beats + duration_beats).is_finite()
            || self.next_midi_part_id == u64::MAX
        {
            return None;
        }
        let track = self.tracks.get_mut(track_index)?;
        if !matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) {
            return None;
        }

        let id = self.next_midi_part_id;
        self.next_midi_part_id += 1;
        track.midi_parts.push(MidiPart {
            id,
            name: format!("MIDI Part {id:02}"),
            start_beats,
            duration_beats,
            notes: Vec::new(),
            controller_events: Vec::new(),
        });
        self.selected_track = u32::try_from(track_index).ok();
        self.selected_midi_part_id = Some(id);
        Some(id)
    }

    /// Adds a note event at a beat position relative to the start of a MIDI part.
    pub fn add_midi_note(
        &mut self,
        track_index: usize,
        part_id: u64,
        start_beats: f64,
        duration_beats: f64,
        pitch: u8,
        velocity: u8,
    ) -> Option<u64> {
        self.add_midi_note_on_channel(
            track_index,
            part_id,
            start_beats,
            duration_beats,
            pitch,
            0,
            velocity,
        )
    }

    /// Adds a note event on the specified MIDI channel, relative to its part.
    pub fn add_midi_note_on_channel(
        &mut self,
        track_index: usize,
        part_id: u64,
        start_beats: f64,
        duration_beats: f64,
        pitch: u8,
        channel: u8,
        velocity: u8,
    ) -> Option<u64> {
        if !start_beats.is_finite()
            || start_beats < 0.0
            || !duration_beats.is_finite()
            || duration_beats <= 0.0
            || !(start_beats + duration_beats).is_finite()
            || pitch > 127
            || channel > 15
            || !(1..=127).contains(&velocity)
            || self.next_midi_note_id == u64::MAX
        {
            return None;
        }
        let track = self.tracks.get_mut(track_index)?;
        if !matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) {
            return None;
        }
        let part = track
            .midi_parts
            .iter_mut()
            .find(|part| part.id == part_id)?;
        if start_beats + duration_beats > part.duration_beats {
            return None;
        }

        let id = self.next_midi_note_id;
        self.next_midi_note_id += 1;
        part.notes.push(MidiNote {
            id,
            start_beats,
            duration_beats,
            pitch,
            channel,
            velocity,
        });
        Some(id)
    }

    /// Adds a MIDI control-change event to a MIDI or instrument part.
    pub fn add_midi_controller_event(
        &mut self,
        track_index: usize,
        part_id: u64,
        position_beats: f64,
        controller: u8,
        value: u8,
    ) -> Option<u64> {
        self.add_midi_controller_event_on_channel(
            track_index,
            part_id,
            position_beats,
            controller,
            0,
            value,
        )
    }

    /// Adds a control-change event on the specified MIDI channel.
    pub fn add_midi_controller_event_on_channel(
        &mut self,
        track_index: usize,
        part_id: u64,
        position_beats: f64,
        controller: u8,
        channel: u8,
        value: u8,
    ) -> Option<u64> {
        if !position_beats.is_finite()
            || position_beats < 0.0
            || controller > 127
            || channel > 15
            || value > 127
        {
            return None;
        }
        let track = self.tracks.get_mut(track_index)?;
        if !matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) {
            return None;
        }
        let part = track
            .midi_parts
            .iter_mut()
            .find(|part| part.id == part_id)?;
        if position_beats > part.duration_beats {
            return None;
        }
        if let Some(event) = part.controller_events.iter_mut().find(|event| {
            event.channel == channel
                && event.controller == controller
                && event.position_beats == position_beats
        }) {
            event.value = value;
            return Some(event.id);
        }
        if self.next_midi_controller_id == u64::MAX {
            return None;
        }

        let id = self.next_midi_controller_id;
        self.next_midi_controller_id += 1;
        part.controller_events.push(MidiControllerEvent {
            id,
            position_beats,
            channel,
            controller,
            value,
        });
        Some(id)
    }

    /// Validates project data loaded from a file before replacing live state.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.name.trim().is_empty() {
            return Err("project name is empty");
        }
        if !self.tempo_bpm.is_finite() || !(1.0..=1000.0).contains(&self.tempo_bpm) {
            return Err("tempo must be between 1 and 1000 BPM");
        }
        if ![
            22_050, 32_000, 44_100, 48_000, 88_200, 96_000, 176_400, 192_000,
        ]
        .contains(&self.sample_rate_hz)
        {
            return Err("project sample rate is not one of the supported standard rates");
        }
        if !(1..=32).contains(&self.beats_per_bar) {
            return Err("beats per bar must be between 1 and 32");
        }
        if !(1..=32).contains(&self.beat_unit) || !self.beat_unit.is_power_of_two() {
            return Err("beat unit must be a power of two between 1 and 32");
        }
        if !self.loop_start_beats.is_finite()
            || self.loop_start_beats < 0.0
            || !self.loop_end_beats.is_finite()
            || self.loop_end_beats > MAX_LOOP_BEATS
            || self.loop_end_beats <= self.loop_start_beats
        {
            return Err("cycle loop must end after a nonnegative start and before beat 1,000,000");
        }
        if self
            .selected_track
            .is_some_and(|index| index as usize >= self.tracks.len())
        {
            return Err("selected track is outside the project track list");
        }
        if self.next_track_number < self.tracks.len() as u32 {
            return Err("next track number is lower than the existing track count");
        }
        if self.next_clip_id == 0 {
            return Err("next clip ID must be nonzero");
        }
        if self.next_midi_part_id == 0 {
            return Err("next MIDI part ID must be nonzero");
        }
        if self.next_midi_note_id == 0 {
            return Err("next MIDI note ID must be nonzero");
        }
        if self.next_midi_controller_id == 0 {
            return Err("next MIDI controller event ID must be nonzero");
        }
        if self.tracks.len() > 1_024 {
            return Err("project cannot contain more than 1024 tracks");
        }

        for (bus, id) in [
            (&self.input_channel, INPUT_BUS_ID),
            (&self.output_channel, OUTPUT_BUS_ID),
        ] {
            if bus.strip.id != id
                || bus.strip.kind != TrackKind::Audio
                || bus.strip.audio_channels != 2
                || !bus.strip.clips.is_empty()
                || !bus.strip.midi_parts.is_empty()
                || bus.strip.armed
                || bus.strip.monitor
            {
                return Err(
                    "device buses must be stereo channel strips without arrange media or recording controls",
                );
            }
            let mut validation = self.clone();
            validation.tracks = vec![bus.strip.clone()];
            validation.tracks[0].id = 1;
            validation.selected_track = None;
            validation.selected_midi_part_id = None;
            validation.next_track_number = 1;
            validation.next_track_id = 2;
            validation.validate_track_data()?;
        }
        self.validate_track_data()
    }

    fn validate_track_data(&self) -> Result<(), &'static str> {
        let mut track_ids = BTreeSet::new();
        let mut clip_ids = BTreeSet::new();
        let mut midi_part_ids = BTreeSet::new();
        let mut midi_note_ids = BTreeSet::new();
        let mut midi_controller_ids = BTreeSet::new();
        for track in &self.tracks {
            if track.id == 0 || track.id >= INPUT_BUS_ID || !track_ids.insert(track.id) {
                return Err("track IDs must be nonzero and unique");
            }
            if track.name.trim().is_empty() {
                return Err("track name is empty");
            }
            if !matches!(track.audio_channels, 1 | 2) {
                return Err("track audio channel width must be mono or stereo");
            }
            if !track.input_gain_db.is_finite() || !(-24.0..=24.0).contains(&track.input_gain_db) {
                return Err("channel input gain must be between -24 and 24 dB");
            }
            if !track.pan.is_finite() || !(-1.0..=1.0).contains(&track.pan) {
                return Err("Track pan must be between -1 and 1");
            }
            if !track.track_delay_ms.is_finite()
                || !(-2000.0..=2000.0).contains(&track.track_delay_ms)
            {
                return Err("Track delay must be between -2000 and 2000 ms");
            }
            if !track.gain.is_finite() || !(0.0..=1.0).contains(&track.gain) {
                return Err("track gain must be between 0.0 and 1.0");
            }
            for (band_index, band) in track.equalizer.bands.iter().enumerate() {
                if !band.frequency_hz.is_finite()
                    || !(20.0..=20_000.0).contains(&band.frequency_hz)
                    || !band.gain_db.is_finite()
                    || !(-24.0..=24.0).contains(&band.gain_db)
                    || !band.q.is_finite()
                    || !(0.1..=10.0).contains(&band.q)
                {
                    return Err("equalizer parameters are outside the supported range");
                }
                if matches!(band_index, 1 | 2)
                    && !matches!(
                        band.filter,
                        FilterType::Bell | FilterType::ParametricI | FilterType::ParametricII
                    )
                {
                    return Err("EQ bands 2 and 3 must use the parametric bell filter");
                }
            }
            if track.inserts.len() > 16 {
                return Err("a track cannot contain more than 16 insert effects");
            }
            for insert in &track.inserts {
                match insert.kind {
                    InsertEffectKind::TestGenerator {
                        frequency_hz,
                        gain_db,
                        ..
                    } if !frequency_hz.is_finite()
                        || !(1.0..=20_000.0).contains(&frequency_hz)
                        || !gain_db.is_finite()
                        || gain_db > 0.0 =>
                    {
                        return Err("TestGenerator parameters are outside the supported range");
                    }
                    InsertEffectKind::StereoDelay {
                        time_ms,
                        feedback,
                        mix,
                    } if !time_ms.is_finite()
                        || !(1.0..=2_000.0).contains(&time_ms)
                        || !feedback.is_finite()
                        || !(0.0..=0.95).contains(&feedback)
                        || !mix.is_finite()
                        || !(0.0..=1.0).contains(&mix) =>
                    {
                        return Err("stereo delay parameters are outside the supported range");
                    }
                    InsertEffectKind::Compressor {
                        threshold_db,
                        ratio,
                        attack_ms,
                        hold_ms,
                        release_ms,
                        analysis,
                        makeup_db,
                        ..
                    } if !threshold_db.is_finite()
                        || !(-60.0..=0.0).contains(&threshold_db)
                        || !ratio.is_finite()
                        || !(1.0..=20.0).contains(&ratio)
                        || !attack_ms.is_finite()
                        || !(0.1..=200.0).contains(&attack_ms)
                        || !hold_ms.is_finite()
                        || !(0.0..=500.0).contains(&hold_ms)
                        || !release_ms.is_finite()
                        || !(5.0..=2_000.0).contains(&release_ms)
                        || !analysis.is_finite()
                        || !(0.0..=100.0).contains(&analysis)
                        || !makeup_db.is_finite()
                        || !(0.0..=24.0).contains(&makeup_db) =>
                    {
                        return Err("compressor parameters are outside the supported range");
                    }
                    _ => {}
                }
            }
            if track.kind != TrackKind::Audio && !track.clips.is_empty() {
                return Err("audio clips must be stored on audio tracks");
            }
            if track.kind == TrackKind::Audio && !track.midi_parts.is_empty() {
                return Err("MIDI parts must be stored on MIDI or instrument tracks");
            }
            for part in &track.midi_parts {
                if part.id == 0 || !midi_part_ids.insert(part.id) {
                    return Err("MIDI part IDs must be nonzero and unique");
                }
                if part.name.trim().is_empty() {
                    return Err("MIDI part name is empty");
                }
                if !part.start_beats.is_finite()
                    || part.start_beats < 0.0
                    || !part.duration_beats.is_finite()
                    || part.duration_beats <= 0.0
                    || !(part.start_beats + part.duration_beats).is_finite()
                {
                    return Err("MIDI part start must be nonnegative and duration positive");
                }
                for note in &part.notes {
                    if note.id == 0 || !midi_note_ids.insert(note.id) {
                        return Err("MIDI note IDs must be nonzero and unique");
                    }
                    if !note.start_beats.is_finite() || note.start_beats < 0.0 {
                        return Err("MIDI note start beat must be finite and nonnegative");
                    }
                    if !note.duration_beats.is_finite()
                        || note.duration_beats <= 0.0
                        || !(note.start_beats + note.duration_beats).is_finite()
                        || note.start_beats + note.duration_beats > part.duration_beats
                    {
                        return Err("MIDI note duration must fit within its part");
                    }
                    if note.pitch > 127 {
                        return Err("MIDI note pitch must be between 0 and 127");
                    }
                    if note.channel > 15 {
                        return Err("MIDI note channel must be between 0 and 15");
                    }
                    if !(1..=127).contains(&note.velocity) {
                        return Err("MIDI note velocity must be between 1 and 127");
                    }
                }
                if part.controller_events.len() > 65_536 {
                    return Err("a MIDI part cannot contain more than 65,536 controller events");
                }
                for event in &part.controller_events {
                    if event.id == 0 || !midi_controller_ids.insert(event.id) {
                        return Err("MIDI controller event IDs must be nonzero and unique");
                    }
                    if event.channel > 15 {
                        return Err("MIDI controller channel must be between 0 and 15");
                    }
                    if !event.position_beats.is_finite()
                        || event.position_beats < 0.0
                        || event.position_beats > part.duration_beats
                    {
                        return Err("MIDI controller event position must fit within its part");
                    }
                    if event.controller > 127 || event.value > 127 {
                        return Err("MIDI controller number and value must be between 0 and 127");
                    }
                }
            }
            for clip in &track.clips {
                if clip.id == 0 || !clip_ids.insert(clip.id) {
                    return Err("audio clip IDs must be nonzero and unique");
                }
                if clip.name.trim().is_empty() {
                    return Err("audio clip name is empty");
                }
                if clip.source_path.as_os_str().is_empty() {
                    return Err("audio clip source path is empty");
                }
                if !clip.start_beats.is_finite() || clip.start_beats < 0.0 {
                    return Err("audio clip start beat must be finite and nonnegative");
                }
                if !clip.duration_seconds.is_finite() || clip.duration_seconds <= 0.0 {
                    return Err("audio clip duration must be finite and positive");
                }
                if !clip.source_offset_seconds.is_finite()
                    || clip.source_offset_seconds < 0.0
                    || !clip.fade_in_seconds.is_finite()
                    || clip.fade_in_seconds < 0.0
                    || !clip.fade_out_seconds.is_finite()
                    || clip.fade_out_seconds < 0.0
                    || clip.fade_in_seconds + clip.fade_out_seconds > clip.duration_seconds + 1e-8
                {
                    return Err("audio event trim/fade timing is invalid");
                }
                if !clip.gain.is_finite() || !(0.0..=4.0).contains(&clip.gain) {
                    return Err("audio event gain must be between 0.0 and 4.0");
                }
            }
        }
        if track_ids
            .last()
            .is_some_and(|maximum_id| self.next_track_id <= *maximum_id)
        {
            return Err("next track ID must be greater than every existing track ID");
        }
        if clip_ids
            .last()
            .is_some_and(|maximum_id| self.next_clip_id <= *maximum_id)
        {
            return Err("next clip ID must be greater than every existing clip ID");
        }
        if midi_part_ids
            .last()
            .is_some_and(|maximum_id| self.next_midi_part_id <= *maximum_id)
        {
            return Err("next MIDI part ID must be greater than every existing part ID");
        }
        if midi_note_ids
            .last()
            .is_some_and(|maximum_id| self.next_midi_note_id <= *maximum_id)
        {
            return Err("next MIDI note ID must be greater than every existing note ID");
        }
        if midi_controller_ids
            .last()
            .is_some_and(|maximum_id| self.next_midi_controller_id <= *maximum_id)
        {
            return Err(
                "next MIDI controller event ID must be greater than every existing event ID",
            );
        }
        if self
            .selected_midi_part_id
            .is_some_and(|id| !midi_part_ids.contains(&id))
        {
            return Err("selected MIDI part is outside the project");
        }
        if let Some(selected_part_id) = self.selected_midi_part_id {
            let selected_track = self
                .selected_track
                .and_then(|index| self.tracks.get(index as usize));
            if !selected_track.is_some_and(|track| {
                track
                    .midi_parts
                    .iter()
                    .any(|part| part.id == selected_part_id)
            }) {
                return Err("selected MIDI part is not on the selected track");
            }
        }

        Ok(())
    }
}

fn first_clip_id() -> u64 {
    1
}

fn first_midi_part_id() -> u64 {
    1
}

fn first_midi_note_id() -> u64 {
    1
}

fn first_midi_controller_id() -> u64 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_audio_track_has_valid_channel_defaults() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn recording_settings_default_for_new_and_legacy_projects() {
        let new_project = Project::default();
        assert_eq!(new_project.sample_rate_hz, 48_000);
        assert_eq!(new_project.recording_format, RecordingFormat::Pcm24);

        let mut legacy = serde_json::to_value(new_project).unwrap();
        let object = legacy.as_object_mut().unwrap();
        object.remove("sample_rate_hz");
        object.remove("recording_format");
        let legacy: Project = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy.sample_rate_hz, 44_100);
        assert_eq!(legacy.recording_format, RecordingFormat::Float32);
        assert!(legacy.validate().is_ok());
    }

    #[test]
    fn cycle_loop_range_requires_a_finite_end_after_a_nonnegative_start() {
        let mut project = Project::default();
        assert_eq!(
            (project.loop_start_beats, project.loop_end_beats),
            (0.0, 16.0)
        );
        assert!(project.set_loop_range(8.0, 24.0));
        assert_eq!(
            (project.loop_start_beats, project.loop_end_beats),
            (8.0, 24.0)
        );
        assert!(!project.set_loop_range(24.0, 8.0));
        assert!(!project.set_loop_range(f64::NAN, 32.0));
        assert_eq!(
            (project.loop_start_beats, project.loop_end_beats),
            (8.0, 24.0)
        );
        assert!(project.validate().is_ok());
    }

    #[test]
    fn directional_cycle_range_rejects_empty_or_invalid_edits() {
        let mut project = Project::default();
        assert!(project.set_cycle_range(8.0, 24.0));
        assert_eq!(project.cycle_mode, CycleMode::Loop);
        assert_eq!(project.cycle_position_beats(25.0), 9.0);
        assert!(project.set_cycle_range(24.0, 8.0));
        assert_eq!(project.cycle_mode, CycleMode::Skip);
        assert_eq!(
            (project.loop_start_beats, project.loop_end_beats),
            (8.0, 24.0)
        );
        assert_eq!(project.cycle_position_beats(7.0), 7.0);
        assert_eq!(project.cycle_position_beats(8.0), 24.0);
        assert_eq!(project.cycle_position_beats(24.0), 24.0);
        for (anchor, endpoint) in [
            (8.0, 8.0),
            (-1.0, 8.0),
            (f64::NAN, 8.0),
            (8.0, f64::INFINITY),
        ] {
            assert!(!project.set_cycle_range(anchor, endpoint));
            assert_eq!(
                (
                    project.loop_start_beats,
                    project.loop_end_beats,
                    project.cycle_mode
                ),
                (8.0, 24.0, CycleMode::Skip)
            );
        }
    }

    #[test]
    fn midi_tracks_accept_beat_based_notes_and_audio_tracks_reject_them() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        project.add_track(TrackKind::Audio);
        let part_id = project
            .add_midi_part(0, 4.0, 16.0)
            .expect("MIDI track should accept a MIDI part");
        let note_id = project
            .add_midi_note(0, part_id, 1.0, 0.5, 60, 96)
            .expect("MIDI part should accept a valid note");
        assert_eq!(note_id, 1);
        assert_eq!(project.selected_midi_part_id, Some(part_id));
        assert!(project.add_midi_part(1, 0.0, 4.0).is_none());
        assert!(
            project
                .add_midi_note(1, part_id, 0.0, 1.0, 60, 96)
                .is_none()
        );
        assert!(project.validate().is_ok());
        assert_eq!(project.tracks[0].midi_parts[0].notes[0].pitch, 60);
        assert_eq!(project.tracks[0].midi_parts[0].notes[0].channel, 0);
        assert_eq!(project.tracks[0].midi_parts[0].notes[0].velocity, 96);
    }

    #[test]
    fn midi_notes_reject_invalid_timing_and_velocity() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let part_id = project.add_midi_part(0, 0.0, 4.0).unwrap();
        assert!(
            project
                .add_midi_note(0, part_id, -1.0, 1.0, 60, 96)
                .is_none()
        );
        assert!(
            project
                .add_midi_note(0, part_id, 0.0, 0.0, 60, 96)
                .is_none()
        );
        assert!(project.add_midi_note(0, part_id, 0.0, 1.0, 60, 0).is_none());
        assert!(
            project
                .add_midi_note(0, part_id, 0.0, 1.0, 60, 128)
                .is_none()
        );
        assert!(
            project
                .add_midi_note(0, part_id, 3.5, 1.0, 60, 96)
                .is_none()
        );
        assert!(
            project
                .add_midi_note_on_channel(0, part_id, 0.0, 1.0, 60, 16, 96)
                .is_none()
        );
        project
            .add_midi_note_on_channel(0, part_id, 0.0, 1.0, 60, 15, 96)
            .expect("MIDI channel 15 should be accepted");
        assert_eq!(project.tracks[0].midi_parts[0].notes.len(), 1);
        assert_eq!(project.tracks[0].midi_parts[0].notes[0].channel, 15);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn midi_controller_events_are_part_relative_and_validated() {
        let mut project = Project::default();
        project.add_track(TrackKind::Instrument);
        let part_id = project.add_midi_part(0, 8.0, 4.0).unwrap();
        let event_id = project
            .add_midi_controller_event(0, part_id, 2.0, 11, 96)
            .expect("controller event should be added");
        assert_eq!(event_id, 1);
        assert_eq!(
            project.tracks[0].midi_parts[0].controller_events[0].position_beats,
            2.0
        );
        assert_eq!(
            project.tracks[0].midi_parts[0].controller_events[0].controller,
            11
        );
        assert_eq!(
            project.tracks[0].midi_parts[0].controller_events[0].value,
            96
        );
        assert_eq!(
            project.add_midi_controller_event(0, part_id, 2.0, 11, 80),
            Some(event_id)
        );
        assert_eq!(project.tracks[0].midi_parts[0].controller_events.len(), 1);
        assert_eq!(
            project.tracks[0].midi_parts[0].controller_events[0].value,
            80
        );
        assert!(
            project
                .add_midi_controller_event(0, part_id, -0.5, 11, 96)
                .is_none()
        );
        assert!(
            project
                .add_midi_controller_event(0, part_id, 4.5, 11, 96)
                .is_none()
        );
        assert!(
            project
                .add_midi_controller_event(0, part_id, 1.0, 128, 96)
                .is_none()
        );
        assert!(
            project
                .add_midi_controller_event(0, part_id, 1.0, 11, 128)
                .is_none()
        );
        assert!(project.validate().is_ok());
    }

    #[test]
    fn middle_eq_bands_reject_shelf_filters() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].equalizer.bands[1].filter = FilterType::LowShelf;
        assert_eq!(
            project.validate(),
            Err("EQ bands 2 and 3 must use the parametric bell filter")
        );
    }

    #[test]
    fn insert_parameters_are_validated_before_audio_plan_construction() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].inserts.push(InsertEffect {
            enabled: true,
            kind: InsertEffectKind::StereoDelay {
                time_ms: 3_000.0,
                feedback: 0.2,
                mix: 0.5,
            },
        });
        assert_eq!(
            project.validate(),
            Err("stereo delay parameters are outside the supported range")
        );

        project.tracks[0].inserts[0].kind = InsertEffectKind::Compressor {
            threshold_db: -18.0,
            ratio: 21.0,
            attack_ms: 10.0,
            hold_ms: 1.0,
            release_ms: 120.0,
            analysis: 80.0,
            makeup_db: 0.0,
            soft_knee: true,
            auto_makeup: true,
            auto_release: false,
            rms_mode: false,
            live: true,
        };
        assert_eq!(
            project.validate(),
            Err("compressor parameters are outside the supported range")
        );
    }
}

#[cfg(test)]
mod bus_tests {
    use super::*;
    #[test]
    fn bus_settings_roundtrip_and_legacy_projects_default_to_unity_without_lanes() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.input_channel.strip.gain = 0.25;
        project.output_channel.strip.equalizer.bands[0].enabled = true;
        let value = serde_json::to_value(&project).unwrap();
        let loaded: Project = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(loaded, project);
        assert!(loaded.validate().is_ok());
        assert_eq!(loaded.tracks.len(), 1);
        let mut legacy = value;
        legacy.as_object_mut().unwrap().remove("input_channel");
        legacy.as_object_mut().unwrap().remove("output_channel");
        let loaded: Project = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.input_channel, BusChannel::input());
        assert_eq!(loaded.output_channel, BusChannel::output());
        assert!(loaded.validate().is_ok());
    }
    #[test]
    fn invalid_bus_settings_and_reserved_track_ids_are_rejected() {
        let mut project = Project::default();
        project.input_channel.strip.gain = f32::NAN;
        assert!(project.validate().is_err());
        project.input_channel = BusChannel::input();
        project.output_channel.strip.armed = true;
        assert!(project.validate().is_err());
        project.output_channel = BusChannel::output();
        project.add_track(TrackKind::Audio);
        project.tracks[0].id = INPUT_BUS_ID;
        assert!(project.validate().is_err());
    }
}

#[cfg(test)]
mod audio_event_tests {
    use super::*;
    #[test]
    fn event_edits_move_only_between_matching_audio_widths_and_persist_non_destructive_settings() {
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        p.add_track(TrackKind::Audio);
        p.add_track_with_audio_channels(TrackKind::Audio, 1);
        p.add_audio_clip(0, PathBuf::from("original.wav"), 0.0, 4.0)
            .unwrap();
        let mut c = p.tracks[0].clips[0].clone();
        c.start_beats = 4.0;
        c.source_offset_seconds = 0.5;
        c.duration_seconds = 3.0;
        c.fade_in_seconds = 0.25;
        c.fade_out_seconds = 0.5;
        c.gain = 2.0;
        let ids = p.tracks.iter().map(|t| t.id).collect::<Vec<_>>();
        assert!(!p.edit_audio_clip(ids[0], ids[2], c.clone()));
        assert!(p.edit_audio_clip(ids[0], ids[1], c.clone()));
        assert_eq!(p.tracks[1].clips[0], c);
        assert!(p.tracks[0].clips.is_empty());
        let encoded = serde_json::to_value(&p).unwrap();
        let decoded: Project = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(p, decoded);
        let mut legacy = encoded;
        for field in [
            "source_offset_seconds",
            "fade_in_seconds",
            "fade_out_seconds",
        ] {
            legacy["tracks"][1]["clips"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
        }
        let old: Project = serde_json::from_value(legacy).unwrap();
        assert_eq!(old.tracks[1].clips[0].source_offset_seconds, 0.0);
        assert_eq!(old.tracks[1].clips[0].fade_in_seconds, 0.0);
        let before = p.clone();
        c.fade_out_seconds = 10.0;
        assert!(!p.edit_audio_clip(ids[1], ids[1], c));
        assert_eq!(p, before);
    }
}
