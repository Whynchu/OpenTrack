//! The application's native, version-independent project domain model.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A track's content and editing domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Audio,
    Group,
    Instrument,
    Midi,
}

/// Maximum persisted channel gain, approximately +6 dB.
pub const MAX_CHANNEL_GAIN: f32 = 2.0;
/// Maximum number of user-visible send slots on one channel.
pub const MAX_SEND_SLOTS: usize = 24;
/// Upper bound on one Repeat/Fill Loop command, so a single gesture cannot
/// flood the timeline with events.
pub const MAX_REPEAT_COUNT: u32 = 64;
/// Latest beat a repetition may reach; mirrors the loop timeline bound.
const MAX_CLIP_BEAT: f64 = 1_000_000.0;
/// Stable identities for device buses, separate from arrange tracks.
pub const INPUT_BUS_ID: u64 = u64::MAX - 1;
pub const OUTPUT_BUS_ID: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SendTap {
    PreFader,
    #[default]
    PostFader,
}

/// Persisted parameters for one optional send slot.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SendParameters {
    /// Send level in dB, from silence (-90 dB) to +12 dB.
    pub level_db: f32,
    /// Stereo balance from full left (-1) to full right (+1).
    pub pan: f32,
    /// Tap position relative to the source channel fader.
    pub tap: SendTap,
    /// A new send is inactive until the user enables it.
    pub enabled: bool,
}

impl Default for SendParameters {
    fn default() -> Self {
        Self {
            level_db: 0.0,
            pan: 0.0,
            tap: SendTap::PostFader,
            enabled: false,
        }
    }
}

pub use crate::automation::{
    AutomationLane, AutomationPoint, AutomationTarget, MAX_AUTOMATION_BEAT,
    MAX_AUTOMATION_LANES_PER_TRACK, MAX_AUTOMATION_POINTS_PER_LANE,
    MAX_AUTOMATION_POINTS_PER_PROJECT,
};

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
            parent_group_id: None,
            group_collapsed: false,
            group_cut_beats: Vec::new(),
            color_index: None,
            audio_channels: 2,
            muted: false,
            solo: false,
            solo_defeat: false,
            listen: false,
            armed: false,
            monitor: false,
            monitor_mode: MonitorMode::Off,
            automation_read: false,
            automation_write: false,
            automation_lanes: Vec::new(),
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
            group_output_auto: false,
            send_routing: Vec::new(),
            send_parameters: Vec::new(),
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
    /// Owning group track, if this lane is nested in a group.
    #[serde(default)]
    pub parent_group_id: Option<u64>,
    /// Whether Project hides a group's child rows. Persisted with the group.
    #[serde(default)]
    pub group_collapsed: bool,
    /// Shared edit boundaries for the virtual event shown by a collapsed group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub group_cut_beats: Vec<f64>,
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
    /// Legacy boolean monitoring flag, kept so older projects decode. Reading it
    /// directly is deprecated: use [`Self::monitors_input`].
    #[serde(default)]
    pub monitor: bool,
    /// How this channel monitors its input: never, only while armed, or always.
    #[serde(default)]
    pub monitor_mode: MonitorMode,
    /// Automation Read control state; processing remains a separate feature.
    #[serde(default)]
    pub automation_read: bool,
    /// Automation Write control state; processing remains a separate feature.
    #[serde(default)]
    pub automation_write: bool,
    /// Track-specific automation curves; missing in legacy projects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub automation_lanes: Vec<AutomationLane>,
    /// Input gain in dB, persisted for the channel input control.
    #[serde(default)]
    pub input_gain_db: f32,
    /// Polarity switch for the channel input.
    #[serde(default)]
    pub input_phase_inverted: bool,
    /// Linear channel gain in the range 0.0 to 2.0 (about +6 dB).
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
    /// True when group membership assigned this output automatically.
    /// Explicit routing edits clear this so later group moves preserve them.
    #[serde(default)]
    pub group_output_auto: bool,
    /// Up to 24 optional send destinations. Empty entries represent No Bus.
    #[serde(default)]
    pub send_routing: Vec<Option<String>>,
    /// Slot-aligned settings. An empty vector means legacy defaults for all
    /// destinations; shorter vectors use defaults for the remaining slots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub send_parameters: Vec<SendParameters>,
    /// Selected built-in voice for an Instrument track.
    #[serde(default)]
    pub instrument_waveform: InstrumentWaveform,
    /// Arrangeable MIDI parts on MIDI and instrument tracks.
    #[serde(default)]
    pub midi_parts: Vec<MidiPart>,
}

/// How a channel passes its live input to its meters and the mix.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorMode {
    /// Input is never monitored.
    #[default]
    Off,
    /// Input is monitored while the track is record-enabled.
    Auto,
    /// Input is monitored whether or not the track is armed.
    On,
}

impl MonitorMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Auto, Self::On];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Auto => "Auto",
            Self::On => "On",
        }
    }

    /// The next mode in the Off → Auto → On cycle.
    pub const fn next(self) -> Self {
        match self {
            Self::Off => Self::Auto,
            Self::Auto => Self::On,
            Self::On => Self::Off,
        }
    }
}

impl Track {
    /// True when this channel should pass its live input to the mix right now.
    pub fn monitors_input(&self) -> bool {
        match self.monitor_mode {
            MonitorMode::Off => false,
            MonitorMode::Auto => self.armed,
            MonitorMode::On => true,
        }
    }
}

fn default_audio_channels() -> u8 {
    2
}

/// Shifts the volume points inside each `(track, start, end, delta)` span.
///
/// A lane is edited all-or-nothing: if any point would leave the timeline or the
/// shifted curve would collide with a neighbouring point, that lane keeps its
/// original points so the curve never becomes ambiguous.
fn shift_volume_automation_within_spans(
    project: &mut Project,
    spans: &[(u64, f64, f64, f64)],
) -> usize {
    let mut shifted_lanes = 0;
    for track in project.tracks.iter_mut() {
        let track_spans = spans
            .iter()
            .filter(|(track_id, ..)| *track_id == track.id)
            .map(|(_, start, end, delta)| (*start, *end, *delta))
            .collect::<Vec<_>>();
        if track_spans.is_empty() {
            continue;
        }
        for lane in track
            .automation_lanes
            .iter_mut()
            .filter(|lane| lane.target == AutomationTarget::TrackVolume)
        {
            let original = lane.points.clone();
            let mut changed = false;
            for point in lane.points.iter_mut() {
                // A point exactly on the boundary belongs to the neighbouring
                // region, so only the interior travels with the event.
                let hit = track_spans
                    .iter()
                    .find(|(start, end, _)| point.beat > *start + 1e-9 && point.beat < *end - 1e-9);
                let Some((_, _, delta)) = hit else {
                    continue;
                };
                let beat = point.beat + delta;
                if beat < 0.0 {
                    changed = false;
                    lane.points.clone_from(&original);
                    break;
                }
                point.beat = beat;
                changed = true;
            }
            if !changed {
                continue;
            }
            lane.points
                .sort_by(|left, right| left.beat.total_cmp(&right.beat));
            if !lane.is_valid() {
                lane.points = original;
                continue;
            }
            shifted_lanes += 1;
        }
    }
    shifted_lanes
}

/// Shortest edge trim, so a nudge can never collapse an event to nothing.
pub const MIN_CLIP_EDGE_SECONDS: f64 = 0.001;

/// Which edge of an audio event a nudge moves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipEdge {
    Start,
    End,
}

/// The geometry of `clip` after one edge nudge, or `None` when nothing can move.
///
/// This is pure so the keymap, a future drag target and the tests all agree on
/// the same rules: the start cannot pass the source's first sample, the end
/// cannot pass its last, and both keep at least `min_seconds`.
pub fn nudged_clip_edge(
    clip: &AudioClip,
    edge: ClipEdge,
    delta_beats: f64,
    tempo_bpm: f64,
    source_duration_seconds: f64,
    min_seconds: f64,
) -> Option<AudioClip> {
    if !delta_beats.is_finite() || tempo_bpm <= 0.0 {
        return None;
    }
    let seconds_per_beat = 60.0 / tempo_bpm;
    let mut next = clip.clone();
    match edge {
        ClipEdge::Start => {
            let end = clip.start_beats + clip.duration_seconds / seconds_per_beat;
            let earliest =
                (clip.start_beats - clip.source_offset_seconds / seconds_per_beat).max(0.0);
            let latest = (end - min_seconds / seconds_per_beat).max(earliest);
            let start = (clip.start_beats + delta_beats).clamp(earliest, latest);
            let applied_seconds = (start - clip.start_beats) * seconds_per_beat;
            let duration = clip.duration_seconds - applied_seconds;
            if duration < min_seconds {
                return None;
            }
            next.start_beats = start;
            next.source_offset_seconds = (clip.source_offset_seconds + applied_seconds).max(0.0);
            next.duration_seconds = duration;
            next.fade_in_seconds = clip.fade_in_seconds.min(duration);
            next.fade_out_seconds = clip
                .fade_out_seconds
                .min((duration - next.fade_in_seconds).max(0.0));
        }
        ClipEdge::End => {
            let source_limit =
                if source_duration_seconds.is_finite() && source_duration_seconds > 0.0 {
                    (source_duration_seconds - clip.source_offset_seconds).max(min_seconds)
                } else {
                    clip.duration_seconds
                };
            let duration = (clip.duration_seconds + delta_beats * seconds_per_beat)
                .clamp(min_seconds, source_limit);
            if (duration - clip.duration_seconds).abs() <= 1e-9 {
                return None;
            }
            next.duration_seconds = duration;
            next.fade_in_seconds = clip.fade_in_seconds.min(duration);
            next.fade_out_seconds = clip
                .fade_out_seconds
                .min((duration - next.fade_in_seconds).max(0.0));
        }
    }
    if (next.start_beats - clip.start_beats).abs() <= 1e-9
        && (next.duration_seconds - clip.duration_seconds).abs() <= 1e-9
        && (next.source_offset_seconds - clip.source_offset_seconds).abs() <= 1e-9
    {
        return None;
    }
    Some(next)
}

fn bounded_marker_name(name: String) -> String {
    let trimmed = name.trim();
    if trimmed.chars().count() <= MAX_MARKER_NAME_CHARS {
        return trimmed.to_owned();
    }
    trimmed.chars().take(MAX_MARKER_NAME_CHARS).collect()
}

fn rename_clip(project: &mut Project, track_id: u64, from: u64, to: u64) {
    if let Some(track) = project.tracks.iter_mut().find(|track| track.id == track_id)
        && let Some(clip) = track.clips.iter_mut().find(|clip| clip.id == from)
    {
        clip.id = to;
    }
}

fn remove_clip(project: &mut Project, track_id: u64, clip_id: u64) {
    if let Some(track) = project.tracks.iter_mut().find(|track| track.id == track_id) {
        track.clips.retain(|clip| clip.id != clip_id);
    }
}

fn track_belongs_to_group(tracks: &[Track], track_id: u64, group_id: u64) -> bool {
    let mut parent_id = tracks
        .iter()
        .find(|track| track.id == track_id)
        .and_then(|track| track.parent_group_id);
    for _ in 0..2 {
        let Some(parent) = parent_id else {
            return false;
        };
        if parent == group_id {
            return true;
        }
        parent_id = tracks
            .iter()
            .find(|track| track.id == parent)
            .and_then(|track| track.parent_group_id);
    }
    false
}

fn sync_group_default_output(track: &mut Track, parent_group_id: Option<u64>) -> bool {
    if track.group_output_auto {
        let route = Some(parent_group_id.map_or_else(
            || "Stereo Out".to_owned(),
            |group_id| format!("Group:{group_id}"),
        ));
        let changed =
            track.output_routing != route || track.group_output_auto != parent_group_id.is_some();
        track.output_routing = route;
        track.group_output_auto = parent_group_id.is_some();
        return changed;
    }
    if parent_group_id.is_some()
        && track
            .output_routing
            .as_deref()
            .is_none_or(|route| route == "Stereo Out")
    {
        let route = parent_group_id.map(|group_id| format!("Group:{group_id}"));
        let changed = track.output_routing != route || !track.group_output_auto;
        track.output_routing = route;
        track.group_output_auto = true;
        return changed;
    }
    false
}

/// Persistent built-in channel EQ with room for up to 32 bands. The original
/// four fields retain their legacy serialized layout; added bands default
/// neutrally when loading older projects.
pub const MAX_EQ_BANDS: usize = 32;
const EXTENDED_EQ_BANDS: usize = MAX_EQ_BANDS - 8;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChannelEqualizer {
    pub bypassed: bool,
    pub bands: [EqualizerBand; 4],
    /// Additional built-in EQ bands. Missing in older project files, where the
    /// default supplies four neutral disabled bands.
    #[serde(default = "default_additional_equalizer_bands")]
    pub additional_bands: [EqualizerBand; 4],
    /// Optional dynamic behavior is separate from static band shape/gain.
    #[serde(default = "default_dynamic_equalizer_bands")]
    pub dynamic_bands: [DynamicEqBand; 8],
    /// Bands beyond the original eight slots, up to `MAX_EQ_BANDS`.
    #[serde(default = "default_extended_equalizer_bands")]
    pub extended_bands: [EqualizerBand; EXTENDED_EQ_BANDS],
    /// Dynamic settings parallel to `extended_bands`.
    #[serde(default = "default_extended_dynamic_equalizer_bands")]
    pub extended_dynamic_bands: [DynamicEqBand; EXTENDED_EQ_BANDS],
    #[serde(default)]
    pub tonal_target: Option<TonalTarget>,
}

/// Compact, project-embedded tonal reference profile. Values are normalized
/// band powers indexed across logarithmic frequency regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TonalTarget {
    pub name: String,
    pub analysis_version: u32,
    pub sample_rate: u32,
    /// 0 = pre channel EQ/inserts, 1 = post channel EQ/inserts, 2 = imported file.
    #[serde(default)]
    pub source_stage: u8,
    /// Basename or channel label only; projects never depend on the source file.
    #[serde(default)]
    pub source_name: String,
    /// Duration represented by this profile, in seconds.
    #[serde(default)]
    pub analysis_seconds: f32,
    /// Tonal profiles use 4096 points in versions 1–2 and 8192 points in v3.
    #[serde(default = "default_tonal_fft_size")]
    pub fft_size: u32,
    pub band_powers: Vec<f32>,
}

fn default_tonal_fft_size() -> u32 {
    4096
}

fn default_additional_equalizer_bands() -> [EqualizerBand; 4] {
    [EqualizerBand::default(); 4]
}

fn default_dynamic_equalizer_bands() -> [DynamicEqBand; 8] {
    [DynamicEqBand::default(); 8]
}

fn default_extended_equalizer_bands() -> [EqualizerBand; EXTENDED_EQ_BANDS] {
    [EqualizerBand::default(); EXTENDED_EQ_BANDS]
}

fn default_extended_dynamic_equalizer_bands() -> [DynamicEqBand; EXTENDED_EQ_BANDS] {
    [DynamicEqBand::default(); EXTENDED_EQ_BANDS]
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DynamicEqBand {
    pub enabled: bool,
    pub threshold_db: f32,
    /// Maximum gain change in dB once the detector is above threshold.
    pub range_db: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

impl Default for DynamicEqBand {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -24.0,
            range_db: -6.0,
            attack_ms: 10.0,
            release_ms: 100.0,
        }
    }
}

impl ChannelEqualizer {
    pub fn all_bands(&self) -> impl Iterator<Item = &EqualizerBand> {
        self.bands
            .iter()
            .chain(self.additional_bands.iter())
            .chain(self.extended_bands.iter())
    }

    pub fn all_bands_mut(&mut self) -> impl Iterator<Item = &mut EqualizerBand> {
        self.bands
            .iter_mut()
            .chain(self.additional_bands.iter_mut())
            .chain(self.extended_bands.iter_mut())
    }

    pub fn all_dynamic_bands(&self) -> impl Iterator<Item = &DynamicEqBand> {
        self.dynamic_bands
            .iter()
            .chain(self.extended_dynamic_bands.iter())
    }

    pub fn band(&self, index: usize) -> Option<&EqualizerBand> {
        match index {
            0..4 => self.bands.get(index),
            4..8 => self.additional_bands.get(index - 4),
            8..MAX_EQ_BANDS => self.extended_bands.get(index - 8),
            _ => None,
        }
    }

    pub fn band_mut(&mut self, index: usize) -> Option<&mut EqualizerBand> {
        match index {
            0..4 => self.bands.get_mut(index),
            4..8 => self.additional_bands.get_mut(index - 4),
            8..MAX_EQ_BANDS => self.extended_bands.get_mut(index - 8),
            _ => None,
        }
    }

    pub fn dynamic_band(&self, index: usize) -> Option<&DynamicEqBand> {
        if index < 8 {
            self.dynamic_bands.get(index)
        } else {
            self.extended_dynamic_bands.get(index - 8)
        }
    }

    pub fn dynamic_band_mut(&mut self, index: usize) -> Option<&mut DynamicEqBand> {
        if index < 8 {
            self.dynamic_bands.get_mut(index)
        } else {
            self.extended_dynamic_bands.get_mut(index - 8)
        }
    }

    pub fn band_and_dynamic_mut(
        &mut self,
        index: usize,
    ) -> Option<(&mut EqualizerBand, &mut DynamicEqBand)> {
        match index {
            0..4 => Some((&mut self.bands[index], &mut self.dynamic_bands[index])),
            4..8 => Some((
                &mut self.additional_bands[index - 4],
                &mut self.dynamic_bands[index],
            )),
            8..MAX_EQ_BANDS => Some((
                &mut self.extended_bands[index - 8],
                &mut self.extended_dynamic_bands[index - 8],
            )),
            _ => None,
        }
    }
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
            additional_bands: default_additional_equalizer_bands(),
            dynamic_bands: default_dynamic_equalizer_bands(),
            extended_bands: default_extended_equalizer_bands(),
            extended_dynamic_bands: default_extended_dynamic_equalizer_bands(),
            tonal_target: None,
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
    Notch,
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
    LowPassII,
    LowShelfSoft,
    LowShelfSteep,
    LowShelfResonant,
    HighShelfSoft,
    HighShelfSteep,
    HighShelfResonant,
}

impl FilterType {
    pub fn supports_dynamic_eq(self) -> bool {
        matches!(
            self,
            Self::Bell
                | Self::ParametricI
                | Self::ParametricII
                | Self::LowShelf
                | Self::LowShelfI
                | Self::LowShelfII
                | Self::LowShelfIII
                | Self::LowShelfIV
                | Self::HighShelf
                | Self::LowShelfSoft
                | Self::LowShelfSteep
                | Self::LowShelfResonant
                | Self::HighShelfSoft
                | Self::HighShelfSteep
                | Self::HighShelfResonant
        )
    }
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
    /// Powered state. A powered-off VST3 insert is unloaded, not just skipped.
    pub enabled: bool,
    /// Bypassed inserts stay loaded but are removed from the audio chain.
    #[serde(default)]
    pub bypassed: bool,
    pub kind: InsertEffectKind,
}

/// Effects persisted in an insert slot. Reusable effects are VST3 plug-ins;
/// `LegacyUnavailableCompressor` only preserves projects that used the removed
/// OpenTrack built-in compressor.
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
    /// Legacy project data only. This keeps older project files readable but
    /// is not offered in the UI and has no audio processor.
    #[serde(rename = "compressor")]
    LegacyUnavailableCompressor {
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        #[serde(default = "default_legacy_compressor_hold_ms")]
        hold_ms: f32,
        release_ms: f32,
        #[serde(default = "default_legacy_compressor_analysis")]
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
    /// Third-party VST3 effect in a normal insert slot.
    ///
    /// Identity is the class ID plus module path hint. Component/controller
    /// state is opaque and versioned only by the plug-in itself.
    Vst3 {
        /// Canonical 32-character lowercase class ID.
        class_id: String,
        /// Module path at insertion time; used as a discovery hint.
        module_path: PathBuf,
        /// Display names retained for missing-plug-in UI.
        #[serde(default)]
        name: String,
        #[serde(default)]
        vendor: String,
        /// Opaque component (processor) state.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        component_state: Vec<u8>,
        /// Opaque controller state.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        controller_state: Vec<u8>,
    },
}

fn default_legacy_compressor_hold_ms() -> f32 {
    1.0
}

fn default_legacy_compressor_analysis() -> f32 {
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
/// Source-anchored fades retained when an event is split. Original media stays intact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceFade {
    pub offset_seconds: f64,
    pub duration_seconds: f64,
    pub fade_in_seconds: f64,
    pub fade_out_seconds: f64,
}
impl SourceFade {
    pub fn gain_at(&self, source_seconds: f64) -> f32 {
        let t = source_seconds - self.offset_seconds;
        if t < 0.0 || t >= self.duration_seconds {
            return 0.0;
        }
        let up = if self.fade_in_seconds > 0.0 {
            (t / self.fade_in_seconds).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let down = if self.fade_out_seconds > 0.0 {
            ((self.duration_seconds - t) / self.fade_out_seconds).clamp(0.0, 1.0)
        } else {
            1.0
        };
        (up * down) as f32
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioClip {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitch_time: Option<crate::pitch_time::PitchTimeEdit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_fades: Vec<SourceFade>,
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
    /// A muted event keeps its place, fades and gain but produces no audio.
    #[serde(default)]
    pub muted: bool,
    /// A locked event keeps playing but refuses move, trim, split and crop.
    #[serde(default)]
    pub locked: bool,
    /// Events sharing a group id move together, like one object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u64>,
    /// Loop-recording passes that belong together, one clip per pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take_group: Option<u64>,
    /// False for a take that is kept but not played; one take per group is active.
    #[serde(default = "default_take_active")]
    pub take_active: bool,
}

fn default_take_active() -> bool {
    true
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
    #[serde(default)]
    pub media_pool: BTreeSet<PathBuf>,
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
    /// Next loop-recording take group id.
    #[serde(default = "first_marker_id")]
    pub next_take_group: u64,
    /// Seconds added to the timeline when the ruler shows timecode. Setting the
    /// timecode at the cursor changes this offset, so zero means timecode starts
    /// at 00:00:00 at beat zero.
    #[serde(default)]
    pub timecode_offset_seconds: f64,
    /// Free-form project notes, kept with the project.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notepad: String,
    /// Timeline markers, ordered by beat. Older projects have none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    #[serde(default = "first_marker_id")]
    pub next_marker_id: u64,
}

/// A named position on the project timeline, used for navigation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub id: u64,
    pub beat: f64,
    pub name: String,
    /// Fixed marker slot 1-9 used by the numeric shortcuts, if this marker has
    /// one. Older projects and ordinary markers leave it empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u8>,
}

/// The marker slots reachable from the numeric shortcuts.
pub const MARKER_SLOTS: std::ops::RangeInclusive<u8> = 1..=9;

/// Largest linear gain an audio event may carry (about +12 dB).
pub const MAX_AUDIO_CLIP_GAIN: f64 = 4.0;

/// Largest timecode offset a project accepts, in seconds (99:59:59).
pub const MAX_TIMECODE_OFFSET_SECONDS: f64 = 359_999.0;

/// Bound on the project notepad, in characters.
pub const MAX_NOTEPAD_CHARS: usize = 100_000;

/// Bound on markers per project, so a runaway script cannot grow the file.
pub const MAX_MARKERS: usize = 1024;
/// Bound on a marker name, in characters.
pub const MAX_MARKER_NAME_CHARS: usize = 96;

fn first_marker_id() -> u64 {
    1
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
            media_pool: BTreeSet::new(),
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
            timecode_offset_seconds: 0.0,
            notepad: String::new(),
            markers: Vec::new(),
            next_marker_id: first_marker_id(),
            next_take_group: 1,
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
            TrackKind::Group => "Group",
            TrackKind::Instrument => "Instrument",
            TrackKind::Midi => "MIDI",
        };

        let audio_channels = if matches!(kind, TrackKind::Audio | TrackKind::Group) {
            audio_channels.clamp(1, 2)
        } else {
            2
        };
        self.tracks.push(Track {
            id,
            name: format!("{prefix} {:02}", self.next_track_number),
            kind,
            parent_group_id: None,
            group_collapsed: false,
            group_cut_beats: Vec::new(),
            color_index: None,
            audio_channels,
            muted: false,
            solo: false,
            solo_defeat: false,
            listen: false,
            armed: false,
            monitor: false,
            monitor_mode: MonitorMode::Off,
            automation_read: false,
            automation_write: false,
            automation_lanes: Vec::new(),
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
            group_output_auto: false,
            send_routing: Vec::new(),
            send_parameters: Vec::new(),
            instrument_waveform: InstrumentWaveform::default(),
            midi_parts: Vec::new(),
        });
        self.selected_track = u32::try_from(self.tracks.len() - 1).ok();
        self.selected_midi_part_id = None;
    }

    /// Duplicates the requested arrange tracks, including their audio/MIDI
    /// events, channel settings, EQ, and insert state. Duplicated groups carry
    /// their descendants, with group and event identities remapped. Audio
    /// clips keep referring to the same media files; those files are not copied.
    /// Returns the IDs of all created tracks in arrange order.
    pub fn duplicate_tracks(&mut self, track_ids: &[u64]) -> Option<Vec<u64>> {
        if track_ids.is_empty() {
            return None;
        }

        let requested = track_ids.iter().copied().collect::<BTreeSet<_>>();
        if requested.is_empty()
            || requested
                .iter()
                .any(|id| !self.tracks.iter().any(|track| track.id == *id))
        {
            return None;
        }

        let mut roots = requested.clone();
        for id in &requested {
            let mut parent_id = self
                .tracks
                .iter()
                .find(|track| track.id == *id)
                .and_then(|track| track.parent_group_id);
            while let Some(parent) = parent_id {
                if requested.contains(&parent) {
                    roots.remove(id);
                    break;
                }
                parent_id = self
                    .tracks
                    .iter()
                    .find(|track| track.id == parent)
                    .and_then(|track| track.parent_group_id);
            }
        }

        let mut families = Vec::new();
        for root in self.tracks.iter().filter(|track| roots.contains(&track.id)) {
            let mut family_ids = BTreeSet::from([root.id]);
            loop {
                let previous_len = family_ids.len();
                for track in &self.tracks {
                    if track
                        .parent_group_id
                        .is_some_and(|parent_id| family_ids.contains(&parent_id))
                    {
                        family_ids.insert(track.id);
                    }
                }
                if family_ids.len() == previous_len {
                    break;
                }
            }
            let family_tracks = self
                .tracks
                .iter()
                .filter(|track| family_ids.contains(&track.id))
                .cloned()
                .collect::<Vec<_>>();
            let last_source_index = self
                .tracks
                .iter()
                .rposition(|track| family_ids.contains(&track.id))?;
            families.push((last_source_index, family_tracks));
        }
        if families.is_empty() {
            return None;
        }

        let mut candidate = self.clone();
        let mut used_names = candidate
            .tracks
            .iter()
            .map(|track| track.name.clone())
            .collect::<BTreeSet<_>>();
        let mut duplicates_by_end = std::collections::BTreeMap::new();
        let mut duplicated_ids = Vec::new();

        for (last_source_index, source_tracks) in families {
            let mut id_map = std::collections::BTreeMap::new();
            for source in &source_tracks {
                let id = candidate.next_track_id;
                candidate.next_track_id = candidate.next_track_id.checked_add(1)?;
                if id == 0 || id >= INPUT_BUS_ID {
                    return None;
                }
                id_map.insert(source.id, id);
            }

            let mut duplicates = Vec::with_capacity(source_tracks.len());
            for source in source_tracks {
                let mut duplicate = source.clone();
                duplicate.id = *id_map.get(&source.id)?;
                duplicate.name = unique_duplicate_track_name(&source.name, &mut used_names);
                if let Some(parent_id) = source.parent_group_id
                    && let Some(duplicate_parent_id) = id_map.get(&parent_id)
                {
                    duplicate.parent_group_id = Some(*duplicate_parent_id);
                    if duplicate.group_output_auto {
                        sync_group_default_output(&mut duplicate, Some(*duplicate_parent_id));
                    }
                }

                for clip in &mut duplicate.clips {
                    clip.id = candidate.next_clip_id;
                    candidate.next_clip_id = candidate.next_clip_id.checked_add(1)?;
                }
                for part in &mut duplicate.midi_parts {
                    part.id = candidate.next_midi_part_id;
                    candidate.next_midi_part_id = candidate.next_midi_part_id.checked_add(1)?;
                    for note in &mut part.notes {
                        note.id = candidate.next_midi_note_id;
                        candidate.next_midi_note_id = candidate.next_midi_note_id.checked_add(1)?;
                    }
                    for controller in &mut part.controller_events {
                        controller.id = candidate.next_midi_controller_id;
                        candidate.next_midi_controller_id =
                            candidate.next_midi_controller_id.checked_add(1)?;
                    }
                }
                candidate.next_track_number = candidate.next_track_number.checked_add(1)?;
                duplicated_ids.push(duplicate.id);
                duplicates.push(duplicate);
            }
            duplicates_by_end.insert(last_source_index, duplicates);
        }

        let mut tracks = Vec::with_capacity(candidate.tracks.len() + duplicated_ids.len());
        for (index, track) in candidate.tracks.drain(..).enumerate() {
            tracks.push(track);
            if let Some(duplicates) = duplicates_by_end.remove(&index) {
                tracks.extend(duplicates);
            }
        }
        candidate.tracks = tracks;
        candidate.selected_track = candidate
            .tracks
            .iter()
            .position(|track| Some(track.id) == duplicated_ids.first().copied())
            .and_then(|index| u32::try_from(index).ok());
        candidate.selected_midi_part_id = None;
        candidate.next_track_number = candidate
            .next_track_number
            .max(candidate.tracks.len() as u32);
        candidate.validate().ok()?;
        *self = candidate;
        Some(duplicated_ids)
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

    /// Assigns existing tracks to a group without changing their timeline
    /// content. Moving a group also keeps its descendants together in the row
    /// order. A group may contain groups only when it is itself top-level,
    /// which caps nesting at two levels.
    pub fn assign_tracks_to_group(&mut self, group_id: u64, track_ids: &[u64]) -> bool {
        if track_ids.is_empty() {
            return false;
        }
        let Some(group_index) = self
            .tracks
            .iter()
            .position(|track| track.id == group_id && track.kind == TrackKind::Group)
        else {
            return false;
        };
        let group_is_nested = self.tracks[group_index].parent_group_id.is_some();
        let mut unique = BTreeSet::new();
        for id in track_ids {
            if *id == group_id || !unique.insert(*id) {
                return false;
            }
            if !self.tracks.iter().any(|track| track.id == *id) {
                return false;
            }
        }

        let mut root_ids = unique.clone();
        for id in &unique {
            let mut parent_id = self
                .tracks
                .iter()
                .find(|track| track.id == *id)
                .and_then(|track| track.parent_group_id);
            while let Some(parent) = parent_id {
                if unique.contains(&parent) {
                    root_ids.remove(id);
                    break;
                }
                parent_id = self
                    .tracks
                    .iter()
                    .find(|track| track.id == parent)
                    .and_then(|track| track.parent_group_id);
            }
        }
        let mut grouped_ids = root_ids.clone();
        loop {
            let previous_len = grouped_ids.len();
            for track in &self.tracks {
                if track
                    .parent_group_id
                    .is_some_and(|parent_id| grouped_ids.contains(&parent_id))
                {
                    grouped_ids.insert(track.id);
                }
            }
            if grouped_ids.len() == previous_len {
                break;
            }
        }
        if grouped_ids.contains(&group_id)
            || (group_is_nested
                && grouped_ids.iter().any(|id| {
                    self.tracks
                        .iter()
                        .any(|track| track.id == *id && track.kind == TrackKind::Group)
                }))
        {
            return false;
        }

        let mut candidate = self.clone();
        let selected_track_id = candidate
            .selected_track
            .and_then(|index| candidate.tracks.get(index as usize))
            .map(|track| track.id);
        for track in &mut candidate.tracks {
            if root_ids.contains(&track.id) {
                track.parent_group_id = Some(group_id);
                sync_group_default_output(track, Some(group_id));
            }
        }
        let mut grouped_tracks = Vec::with_capacity(grouped_ids.len());
        let mut remaining = Vec::with_capacity(candidate.tracks.len());
        for track in candidate.tracks.drain(..) {
            if grouped_ids.contains(&track.id) {
                grouped_tracks.push(track);
            } else {
                remaining.push(track);
            }
        }
        let group_index = remaining
            .iter()
            .position(|track| track.id == group_id)
            .expect("group must remain in the project");
        remaining.splice(group_index + 1..group_index + 1, grouped_tracks);
        candidate.tracks = remaining;
        candidate.selected_track = selected_track_id
            .and_then(|id| candidate.tracks.iter().position(|track| track.id == id))
            .and_then(|index| u32::try_from(index).ok());
        if candidate.validate().is_err() {
            return false;
        }
        *self = candidate;
        true
    }

    /// Removes the direct children of `group_id` from that group.
    pub fn ungroup_tracks(&mut self, group_id: u64) -> bool {
        if !self
            .tracks
            .iter()
            .any(|track| track.id == group_id && track.kind == TrackKind::Group)
        {
            return false;
        }
        let mut changed = false;
        for track in &mut self.tracks {
            if track.parent_group_id == Some(group_id) {
                track.parent_group_id = None;
                sync_group_default_output(track, None);
                changed = true;
            }
        }
        changed
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
        self.register_media();
        let removed_id = self.tracks[track_index].id;
        let removed_parent_id = self.tracks[track_index].parent_group_id;
        for track in &mut self.tracks {
            if track.parent_group_id == Some(removed_id) {
                track.parent_group_id = removed_parent_id;
            }
        }
        let selected_track_id = self
            .selected_track
            .and_then(|index| self.tracks.get(index as usize))
            .map(|track| track.id);
        let removed = self.tracks.remove(track_index);
        for track in &mut self.tracks {
            if track.parent_group_id == Some(removed_id) {
                sync_group_default_output(track, removed_parent_id);
            }
        }

        self.selected_track = selected_track_id
            .and_then(|id| self.tracks.iter().position(|track| track.id == id))
            .or_else(|| (!self.tracks.is_empty()).then(|| track_index.min(self.tracks.len() - 1)))
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

    /// Reorders selected tracks as one stable block at an insertion index.
    pub fn move_tracks_to(&mut self, track_ids: &[u64], insertion_index: usize) -> bool {
        if track_ids.is_empty() {
            return false;
        }
        let ids = track_ids.iter().copied().collect::<BTreeSet<_>>();
        if ids.len() != track_ids.len()
            || ids
                .iter()
                .any(|id| !self.tracks.iter().any(|t| t.id == *id))
        {
            return false;
        }
        let mut root_ids = ids.clone();
        for id in &ids {
            let mut parent_id = self
                .tracks
                .iter()
                .find(|track| track.id == *id)
                .and_then(|track| track.parent_group_id);
            while let Some(parent) = parent_id {
                if ids.contains(&parent) {
                    root_ids.remove(id);
                    break;
                }
                parent_id = self
                    .tracks
                    .iter()
                    .find(|track| track.id == parent)
                    .and_then(|track| track.parent_group_id);
            }
        }

        // A moved group carries its descendants in the ordered track list.
        // Their parent links remain intact; only the dragged roots can join a
        // different group at the destination.
        let mut moved_ids = root_ids.clone();
        loop {
            let previous_len = moved_ids.len();
            for track in &self.tracks {
                if track
                    .parent_group_id
                    .is_some_and(|parent_id| moved_ids.contains(&parent_id))
                {
                    moved_ids.insert(track.id);
                }
            }
            if moved_ids.len() == previous_len {
                break;
            }
        }

        let insertion_index = insertion_index.min(self.tracks.len());
        let moved_before_insertion = self.tracks[..insertion_index]
            .iter()
            .filter(|track| moved_ids.contains(&track.id))
            .count();
        let selected_track_id = self
            .selected_track
            .and_then(|index| self.tracks.get(index as usize))
            .map(|track| track.id);
        let original_order = self.tracks.iter().map(|track| track.id).collect::<Vec<_>>();
        let moved_order = self
            .tracks
            .iter()
            .filter(|track| moved_ids.contains(&track.id))
            .map(|track| track.id)
            .collect::<Vec<_>>();
        let mut moved = Vec::with_capacity(moved_ids.len());
        let mut remaining = Vec::with_capacity(self.tracks.len() - moved_ids.len());
        for track in self.tracks.iter().cloned() {
            if moved_ids.contains(&track.id) {
                moved.push(track);
            } else {
                remaining.push(track);
            }
        }
        let destination = insertion_index
            .saturating_sub(moved_before_insertion)
            .min(remaining.len());
        let before_parent = destination
            .checked_sub(1)
            .and_then(|index| remaining.get(index))
            .and_then(|track| track.parent_group_id);
        let after_parent = remaining
            .get(destination)
            .and_then(|track| track.parent_group_id);
        let destination_parent = match (before_parent, after_parent) {
            (Some(before), Some(after)) if before == after => Some(before),
            (Some(parent), None) | (None, Some(parent)) => Some(parent),
            _ => None,
        };
        let mut membership_changed = false;
        for track in &mut moved {
            if root_ids.contains(&track.id) && track.parent_group_id != destination_parent {
                track.parent_group_id = destination_parent;
                membership_changed = true;
            }
            if root_ids.contains(&track.id) {
                membership_changed |= sync_group_default_output(track, destination_parent);
            }
        }
        let mut target_order = remaining.iter().map(|track| track.id).collect::<Vec<_>>();
        target_order.splice(destination..destination, moved_order);
        if target_order == original_order && !membership_changed {
            return false;
        }
        remaining.splice(destination..destination, moved);
        self.tracks = remaining;
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
        // A locked event is visible and audible but cannot be rewritten: move,
        // trim and every other geometry edit arrive here.
        if self.tracks[source].clips[index].locked && self.tracks[source].clips[index] != edited {
            return false;
        }
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

    /// Move several audio events as one validated project mutation.
    /// The batch is rejected in full if any identity, destination or edited clip is invalid.
    pub fn move_audio_clips(&mut self, edits: &[(u64, u64, AudioClip)]) -> bool {
        self.move_audio_clips_with_automation(edits, false)
    }

    /// Moves events, and when `follow_automation` is set carries the volume
    /// automation under each moved event along with it.
    ///
    /// This is the "Automation follows Events" behavior: the curve keeps its
    /// shape relative to the event instead of staying at its timeline position.
    /// A clip that changes track still shifts the curve on the track it came
    /// from, because that is where the points were written.
    pub fn move_audio_clips_with_automation(
        &mut self,
        edits: &[(u64, u64, AudioClip)],
        follow_automation: bool,
    ) -> bool {
        if edits.is_empty() {
            return false;
        }
        // A grouped event drags its members with it. The delta comes from the
        // edited member, so the group keeps its internal spacing even when only
        // one member is passed in.
        let mut grouped = edits.to_vec();
        let mut grouped_ids = edits
            .iter()
            .map(|(_, _, clip)| clip.id)
            .collect::<BTreeSet<_>>();
        for (source_track_id, _, edited) in edits {
            let Some(original) = self
                .tracks
                .iter()
                .find(|track| track.id == *source_track_id)
                .and_then(|track| track.clips.iter().find(|clip| clip.id == edited.id))
            else {
                continue;
            };
            let Some(group) = original.group else {
                continue;
            };
            let delta = edited.start_beats - original.start_beats;
            if delta == 0.0 {
                continue;
            }
            let members = self
                .tracks
                .iter()
                .filter(|track| track.kind == TrackKind::Audio)
                .flat_map(|track| {
                    track
                        .clips
                        .iter()
                        .filter(|clip| clip.group == Some(group) && clip.id != edited.id)
                        .map(move |clip| (track.id, clip.clone()))
                })
                .collect::<Vec<_>>();
            for (track_id, member) in members {
                if member.locked || !grouped_ids.insert(member.id) {
                    continue;
                }
                let mut moved = member;
                moved.start_beats += delta;
                grouped.push((track_id, track_id, moved));
            }
        }

        // The spans are read before anything moves: a point belongs to the span
        // the event occupied when the gesture started.
        let followed = if follow_automation {
            grouped
                .iter()
                .filter_map(|(source_track_id, _, edited)| {
                    let original = self
                        .tracks
                        .iter()
                        .find(|track| track.id == *source_track_id)?
                        .clips
                        .iter()
                        .find(|clip| clip.id == edited.id)?;
                    let delta = edited.start_beats - original.start_beats;
                    if delta.abs() <= f64::EPSILON {
                        return None;
                    }
                    let end =
                        original.start_beats + original.duration_seconds * self.tempo_bpm / 60.0;
                    Some((*source_track_id, original.start_beats, end, delta))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let mut ids = BTreeSet::new();
        let mut removals = Vec::with_capacity(grouped.len());
        for (source_track_id, target_track_id, edited) in &grouped {
            if !ids.insert(edited.id) {
                return false;
            }
            let Some(source) = self
                .tracks
                .iter()
                .position(|track| track.id == *source_track_id)
            else {
                return false;
            };
            let Some(target) = self
                .tracks
                .iter()
                .position(|track| track.id == *target_track_id)
            else {
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
                .position(|clip| clip.id == edited.id && clip.source_path == edited.source_path)
            else {
                return false;
            };
            if self.tracks[source].clips[index].locked {
                return false;
            }
            removals.push((source, index));
        }
        let edits = &grouped;

        let mut candidate = self.clone();
        removals.sort_unstable_by(|a, b| b.cmp(a));
        for (track_index, clip_index) in removals {
            candidate.tracks[track_index].clips.remove(clip_index);
        }
        for (_, target_track_id, edited) in edits {
            let Some(target) = candidate
                .tracks
                .iter()
                .position(|track| track.id == *target_track_id)
            else {
                return false;
            };
            candidate.tracks[target].clips.push(edited.clone());
        }
        if candidate.validate().is_err() {
            return false;
        }
        if !followed.is_empty() {
            shift_volume_automation_within_spans(&mut candidate, &followed);
        }
        *self = candidate;
        true
    }

    /// Sample-aligned, non-destructive split; retains source identity, gain and original fades.
    pub fn split_audio_clip(&mut self, track_id: u64, clip_id: u64, at_beats: f64) -> Option<u64> {
        if !at_beats.is_finite() || self.next_clip_id == u64::MAX {
            return None;
        }
        let track = self
            .tracks
            .iter()
            .position(|t| t.id == track_id && t.kind == TrackKind::Audio)?;
        let index = self.tracks[track]
            .clips
            .iter()
            .position(|c| c.id == clip_id)?;
        let original = &self.tracks[track].clips[index];
        if original.locked {
            return None;
        }
        let cut = original.cut_offset_seconds(at_beats, self.tempo_bpm, self.sample_rate_hz)?;
        let mut left = original.clone();
        if left.fade_in_seconds > 0.0 || left.fade_out_seconds > 0.0 {
            if left.source_fades.len() >= 32 {
                return None;
            }
            left.source_fades.push(SourceFade {
                offset_seconds: left.source_offset_seconds,
                duration_seconds: left.duration_seconds,
                fade_in_seconds: left.fade_in_seconds,
                fade_out_seconds: left.fade_out_seconds,
            });
        }
        left.fade_in_seconds = 0.0;
        left.fade_out_seconds = 0.0;
        let mut right = left.clone();
        let id = self.next_clip_id;
        right.id = id;
        right.source_offset_seconds += cut;
        right.start_beats += cut * self.tempo_bpm / 60.0;
        right.duration_seconds -= cut;
        left.duration_seconds = cut;
        let mut candidate = self.clone();
        candidate.next_clip_id += 1;
        candidate.tracks[track].clips[index] = left;
        candidate.tracks[track].clips.insert(index + 1, right);
        if candidate.validate().is_err() {
            return None;
        }
        *self = candidate;
        Some(id)
    }

    /// Copies the given events directly after the selection's span.
    ///
    /// The copy keeps the source, trim, fades, gain and mute state, so it is a
    /// real event rather than a reference to the original. A single event is
    /// duplicated immediately after itself; several are shifted by the whole
    /// selection's length, which keeps their spacing.
    pub fn duplicate_audio_clips(&mut self, ids: &[u64]) -> Vec<u64> {
        if ids.is_empty() {
            return Vec::new();
        }
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut start = f64::INFINITY;
        let mut end = f64::NEG_INFINITY;
        let mut targets = Vec::new();
        for (track_index, track) in self.tracks.iter().enumerate() {
            if track.kind != TrackKind::Audio {
                continue;
            }
            for (clip_index, clip) in track.clips.iter().enumerate() {
                if !selected.contains(&clip.id) {
                    continue;
                }
                start = start.min(clip.start_beats);
                end = end.max(clip.start_beats + clip.duration_seconds * self.tempo_bpm / 60.0);
                targets.push((track_index, clip_index));
            }
        }
        if targets.is_empty() {
            return Vec::new();
        }
        let offset = end - start;
        if !offset.is_finite() || offset <= 0.0 {
            return Vec::new();
        }
        if self
            .next_clip_id
            .checked_add(targets.len() as u64)
            .is_none()
        {
            return Vec::new();
        }
        let mut candidate = self.clone();
        let mut created = Vec::with_capacity(targets.len());
        for (track_index, clip_index) in targets {
            let mut copy = candidate.tracks[track_index].clips[clip_index].clone();
            copy.id = candidate.next_clip_id;
            candidate.next_clip_id += 1;
            copy.start_beats += offset;
            created.push(copy.id);
            candidate.tracks[track_index].clips.push(copy);
        }
        if candidate.validate().is_err() {
            return Vec::new();
        }
        *self = candidate;
        created
    }

    /// Repeats the selection `count` more times directly after it.
    ///
    /// Each repetition is shifted by the selection's own span, so the spacing of
    /// a multi-event selection is preserved. Returns the ids of the new events.
    pub fn repeat_audio_clips(&mut self, ids: &[u64], count: u32) -> Vec<u64> {
        let count = count.min(MAX_REPEAT_COUNT);
        if ids.is_empty() || count == 0 {
            return Vec::new();
        }
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut start = f64::INFINITY;
        let mut end = f64::NEG_INFINITY;
        let mut targets = Vec::new();
        for (track_index, track) in self.tracks.iter().enumerate() {
            if track.kind != TrackKind::Audio {
                continue;
            }
            for (clip_index, clip) in track.clips.iter().enumerate() {
                if !selected.contains(&clip.id) {
                    continue;
                }
                start = start.min(clip.start_beats);
                end = end.max(clip.start_beats + clip.duration_seconds * self.tempo_bpm / 60.0);
                targets.push((track_index, clip_index));
            }
        }
        if targets.is_empty() {
            return Vec::new();
        }
        let span = end - start;
        if !span.is_finite() || span <= 0.0 {
            return Vec::new();
        }
        // Stop before any repetition would leave the supported timeline or the
        // id space, rather than writing a partial repeat the caller cannot see.
        let mut usable = count;
        while usable > 0 {
            let farthest = end + span * f64::from(usable - 1);
            if farthest <= MAX_CLIP_BEAT
                && self
                    .next_clip_id
                    .checked_add(targets.len() as u64 * u64::from(usable))
                    .is_some()
            {
                break;
            }
            usable -= 1;
        }
        if usable == 0 {
            return Vec::new();
        }
        let mut candidate = self.clone();
        let mut created = Vec::with_capacity(targets.len() * usable as usize);
        for repetition in 1..=usable {
            let offset = span * f64::from(repetition);
            for (track_index, clip_index) in &targets {
                let mut copy = candidate.tracks[*track_index].clips[*clip_index].clone();
                copy.id = candidate.next_clip_id;
                candidate.next_clip_id += 1;
                copy.start_beats += offset;
                created.push(copy.id);
                candidate.tracks[*track_index].clips.push(copy);
            }
        }
        if candidate.validate().is_err() {
            return Vec::new();
        }
        *self = candidate;
        created
    }

    /// Repeats the selection until it reaches the end of the loop range.
    ///
    /// The loop is the range the user already drew, so this is repeat with a
    /// count derived from the range rather than typed by hand.
    pub fn fill_loop_with_audio_clips(
        &mut self,
        ids: &[u64],
        loop_start: f64,
        loop_end: f64,
    ) -> Vec<u64> {
        if !loop_start.is_finite() || !loop_end.is_finite() || loop_end <= loop_start {
            return Vec::new();
        }
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut start = f64::INFINITY;
        let mut end = f64::NEG_INFINITY;
        for track in self.tracks.iter().filter(|t| t.kind == TrackKind::Audio) {
            for clip in &track.clips {
                if selected.contains(&clip.id) {
                    start = start.min(clip.start_beats);
                    end = end.max(clip.start_beats + clip.duration_seconds * self.tempo_bpm / 60.0);
                }
            }
        }
        let span = end - start;
        if !span.is_finite() || span <= 0.0 || !start.is_finite() {
            return Vec::new();
        }
        // How many direct repetitions land inside the loop range.
        let count = ((loop_end.min(MAX_CLIP_BEAT) - end) / span).floor();
        if !count.is_finite() || count <= 0.0 {
            return Vec::new();
        }
        let count = count.min(f64::from(MAX_REPEAT_COUNT)) as u32;
        self.repeat_audio_clips(ids, count)
    }

    /// Trims the given events to the half-open range and reports how many changed.
    ///
    /// Material outside the range is removed through the ordinary split path, so
    /// source trims and fades keep their lineage. Events that lie entirely
    /// outside are left untouched: cropping a range must not silently delete
    /// material the user did not point at.
    pub fn crop_audio_clips_to_range(&mut self, ids: &[u64], start: f64, end: f64) -> usize {
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
            return 0;
        }
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        if selected.is_empty() {
            return 0;
        }
        let mut candidate = self.clone();
        let mut trimmed = 0;
        for track_id in candidate
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
            .map(|track| track.id)
            .collect::<Vec<_>>()
        {
            let targets = candidate
                .tracks
                .iter()
                .find(|track| track.id == track_id)
                .map(|track| {
                    track
                        .clips
                        .iter()
                        .filter(|clip| selected.contains(&clip.id))
                        .map(|clip| {
                            (
                                clip.id,
                                clip.start_beats,
                                clip.start_beats
                                    + clip.duration_seconds * candidate.tempo_bpm / 60.0,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for (clip_id, clip_start, clip_end) in targets {
                if clip_end <= start + 1e-9 || clip_start >= end - 1e-9 {
                    continue;
                }
                if candidate
                    .tracks
                    .iter()
                    .find(|track| track.id == track_id)
                    .and_then(|track| track.clips.iter().find(|clip| clip.id == clip_id))
                    .is_some_and(|clip| clip.locked)
                {
                    continue;
                }
                let mut changed = false;
                // Trim the tail first so the head split keeps the shortened event.
                if clip_end > end + 1e-9
                    && let Some(right) = candidate.split_audio_clip(track_id, clip_id, end)
                {
                    remove_clip(&mut candidate, track_id, right);
                    changed = true;
                }
                if clip_start < start - 1e-9
                    && let Some(right) = candidate.split_audio_clip(track_id, clip_id, start)
                {
                    // The surviving part keeps the event's identity, so a
                    // selection that pointed at it still points at it.
                    remove_clip(&mut candidate, track_id, clip_id);
                    rename_clip(&mut candidate, track_id, right, clip_id);
                    changed = true;
                }
                if changed {
                    trimmed += 1;
                }
            }
        }
        if trimmed == 0 || candidate.validate().is_err() {
            return 0;
        }
        *self = candidate;
        trimmed
    }

    /// Adds a marker at `beat` and returns its id.
    ///
    /// A marker is a navigation bookmark, so duplicates at the same beat are
    /// allowed but bounded in count and name length.
    pub fn add_marker(&mut self, beat: f64, name: impl Into<String>) -> Option<u64> {
        if !beat.is_finite() || beat < 0.0 || beat > MAX_LOOP_BEATS {
            return None;
        }
        if self.markers.len() >= MAX_MARKERS || self.next_marker_id == u64::MAX {
            return None;
        }
        let id = self.next_marker_id;
        self.next_marker_id += 1;
        self.markers.push(Marker {
            id,
            beat,
            name: bounded_marker_name(name.into()),
            slot: None,
        });
        self.markers
            .sort_by(|left, right| left.beat.total_cmp(&right.beat));
        Some(id)
    }

    /// Renames a marker and reports whether it changed.
    pub fn rename_marker(&mut self, id: u64, name: impl Into<String>) -> bool {
        let Some(marker) = self.markers.iter_mut().find(|marker| marker.id == id) else {
            return false;
        };
        let name = bounded_marker_name(name.into());
        if marker.name == name {
            return false;
        }
        marker.name = name;
        true
    }

    /// Moves a marker to another beat and keeps the list ordered.
    pub fn move_marker(&mut self, id: u64, beat: f64) -> bool {
        if !beat.is_finite() || beat < 0.0 || beat > MAX_LOOP_BEATS {
            return false;
        }
        let Some(marker) = self.markers.iter_mut().find(|marker| marker.id == id) else {
            return false;
        };
        if marker.beat == beat {
            return false;
        }
        marker.beat = beat;
        self.markers
            .sort_by(|left, right| left.beat.total_cmp(&right.beat));
        true
    }

    /// Removes a marker and reports whether it existed.
    pub fn remove_marker(&mut self, id: u64) -> bool {
        let before = self.markers.len();
        self.markers.retain(|marker| marker.id != id);
        self.markers.len() != before
    }

    /// Sets the marker in a fixed 1-9 slot, creating or moving it.
    ///
    /// Slots are how the numeric shortcuts address markers, so a slot always
    /// names at most one marker and the marker keeps its identity when the slot
    /// moves. Returns the marker id, or `None` for a slot outside 1-9.
    pub fn set_marker_slot(&mut self, slot: u8, beat: f64, name: impl Into<String>) -> Option<u64> {
        if !MARKER_SLOTS.contains(&slot) || !beat.is_finite() || beat < 0.0 || beat > MAX_LOOP_BEATS
        {
            return None;
        }
        if let Some(marker) = self
            .markers
            .iter_mut()
            .find(|marker| marker.slot == Some(slot))
        {
            marker.beat = beat;
            marker.name = bounded_marker_name(name.into());
            let id = marker.id;
            self.markers
                .sort_by(|left, right| left.beat.total_cmp(&right.beat));
            return Some(id);
        }
        if self.markers.len() >= MAX_MARKERS || self.next_marker_id == u64::MAX {
            return None;
        }
        let id = self.next_marker_id;
        self.next_marker_id += 1;
        self.markers.push(Marker {
            id,
            beat,
            name: bounded_marker_name(name.into()),
            slot: Some(slot),
        });
        self.markers
            .sort_by(|left, right| left.beat.total_cmp(&right.beat));
        Some(id)
    }

    /// The marker in a fixed slot, if one is set.
    pub fn marker_in_slot(&self, slot: u8) -> Option<&Marker> {
        self.markers.iter().find(|marker| marker.slot == Some(slot))
    }

    /// The next marker strictly after `beat`, or the first one when wrapping.
    pub fn next_marker_after(&self, beat: f64, wrap: bool) -> Option<&Marker> {
        self.markers
            .iter()
            .find(|marker| marker.beat > beat + 1e-9)
            .or_else(|| wrap.then(|| self.markers.first()).flatten())
    }

    /// The previous marker strictly before `beat`, or the last one when wrapping.
    pub fn previous_marker_before(&self, beat: f64, wrap: bool) -> Option<&Marker> {
        self.markers
            .iter()
            .rev()
            .find(|marker| marker.beat < beat - 1e-9)
            .or_else(|| wrap.then(|| self.markers.last()).flatten())
    }

    /// Splits a MIDI part at an absolute project beat.
    ///
    /// Notes before the cut stay, notes after it move to the new part, and a note
    /// that crosses the cut is itself split so no played note is lost. Controller
    /// events follow the same rule. The cut must land inside the part by at least
    /// a 64th note, so a split cannot produce a part too short to edit.
    pub fn split_midi_part(&mut self, track_id: u64, part_id: u64, beat: f64) -> Option<u64> {
        let track_index = self.tracks.iter().position(|track| track.id == track_id)?;
        let part_index = self.tracks[track_index]
            .midi_parts
            .iter()
            .position(|part| part.id == part_id)?;
        let original = self.tracks[track_index].midi_parts[part_index].clone();
        let split_offset = beat - original.start_beats;
        if !split_offset.is_finite()
            || split_offset < 0.0625
            || split_offset > original.duration_beats - 0.0625
        {
            return None;
        }
        let new_part_id = self.add_midi_part(
            track_index,
            original.start_beats + split_offset,
            original.duration_beats - split_offset,
        )?;
        if let Some(new_part) = self.tracks[track_index]
            .midi_parts
            .iter_mut()
            .find(|part| part.id == new_part_id)
        {
            new_part.name = format!("{} (split)", original.name);
        }
        // The right part is filled first: the helpers number notes and controller
        // events monotonically, so the left part's retained ids stay untouched.
        for event in original.controller_events.iter() {
            if event.position_beats >= split_offset {
                let _ = self.add_midi_controller_event_on_channel(
                    track_index,
                    new_part_id,
                    event.position_beats - split_offset,
                    event.controller,
                    event.channel,
                    event.value,
                );
            }
        }
        for note in original.notes.iter() {
            let end = note.start_beats + note.duration_beats;
            if end <= split_offset {
                continue;
            }
            let start = (note.start_beats - split_offset).max(0.0);
            let duration = end.min(original.duration_beats) - split_offset.max(note.start_beats);
            if duration > 0.0 {
                let _ = self.add_midi_note_on_channel(
                    track_index,
                    new_part_id,
                    start,
                    duration,
                    note.pitch,
                    note.channel,
                    note.velocity,
                );
            }
        }
        let left = &mut self.tracks[track_index].midi_parts[part_index];
        left.duration_beats = split_offset;
        left.notes.retain_mut(|note| {
            if note.start_beats >= split_offset - 1e-9 {
                return false;
            }
            note.duration_beats = note.duration_beats.min(split_offset - note.start_beats);
            note.duration_beats > 1e-6
        });
        left.controller_events
            .retain(|event| event.position_beats < split_offset - 1e-9);
        if self.validate().is_err() {
            return None;
        }
        Some(new_part_id)
    }

    /// Merges MIDI parts on one track into a single part.
    ///
    /// The merged part spans from the earliest start to the latest end and keeps
    /// every note and controller event at its musical position. The earliest part
    /// keeps its identity, so a selection that pointed at it still does.
    pub fn merge_midi_parts(&mut self, track_id: u64, part_ids: &[u64]) -> Option<u64> {
        if part_ids.len() < 2 {
            return None;
        }
        let track_index = self.tracks.iter().position(|track| track.id == track_id)?;
        let selected: BTreeSet<u64> = part_ids.iter().copied().collect();
        let parts = self.tracks[track_index]
            .midi_parts
            .iter()
            .filter(|part| selected.contains(&part.id))
            .cloned()
            .collect::<Vec<_>>();
        if parts.len() != selected.len() {
            return None;
        }
        let start = parts
            .iter()
            .map(|part| part.start_beats)
            .fold(f64::INFINITY, f64::min);
        let end = parts
            .iter()
            .map(|part| part.start_beats + part.duration_beats)
            .fold(f64::NEG_INFINITY, f64::max);
        if !start.is_finite() || !end.is_finite() || end <= start {
            return None;
        }
        let mut keeper = parts
            .iter()
            .min_by(|left, right| left.id.cmp(&right.id))?
            .clone();
        keeper.start_beats = start;
        keeper.duration_beats = end - start;
        keeper.notes.clear();
        keeper.controller_events.clear();

        // Notes are re-added through the helpers so ids stay unique across the
        // merged part; controller events merge the same way.
        let remove = selected.clone();
        self.tracks[track_index]
            .midi_parts
            .retain(|part| !remove.contains(&part.id));
        self.tracks[track_index].midi_parts.push(keeper.clone());
        let note_sources = parts
            .iter()
            .flat_map(|part| {
                part.notes
                    .iter()
                    .map(move |note| (part.start_beats - start + note.start_beats, note.clone()))
            })
            .collect::<Vec<_>>();
        let event_sources = parts
            .iter()
            .flat_map(|part| {
                part.controller_events.iter().map(move |event| {
                    (
                        part.start_beats - start + event.position_beats,
                        event.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();
        for (offset, note) in note_sources {
            let _ = self.add_midi_note_on_channel(
                track_index,
                keeper.id,
                offset,
                note.duration_beats,
                note.pitch,
                note.channel,
                note.velocity,
            );
        }
        for (offset, event) in event_sources {
            let _ = self.add_midi_controller_event_on_channel(
                track_index,
                keeper.id,
                offset,
                event.controller,
                event.channel,
                event.value,
            );
        }
        // A selection on a removed part follows the merged one, so the project
        // never points at a part that no longer exists.
        if self
            .selected_midi_part_id
            .is_some_and(|selected_id| remove.contains(&selected_id))
        {
            self.selected_midi_part_id = Some(keeper.id);
        }
        if self.validate().is_err() {
            return None;
        }
        Some(keeper.id)
    }

    /// Removes `[start, end)` from the timeline and closes the gap.
    ///
    /// With an empty `track_ids` the whole project is edited and global markers
    /// move with it; with track ids only those tracks lose the time, leaving
    /// markers alone. Events and MIDI parts crossing either boundary are split so
    /// the surviving material keeps its audio, then everything later shifts left
    /// by the removed length. A locked event overlapping the range refuses the
    /// whole edit; a locked event after it moves with the timeline.
    pub fn remove_time_range(&mut self, start: f64, end: f64, track_ids: &[u64]) -> Option<usize> {
        if !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || end <= start
            || end > MAX_LOOP_BEATS
        {
            return None;
        }
        let gap = end - start;
        let all_tracks = track_ids.is_empty();
        let scope = if all_tracks {
            self.tracks.iter().map(|track| track.id).collect::<Vec<_>>()
        } else {
            track_ids.to_vec()
        };
        let mut candidate = self.clone();
        let mut changed = 0;
        for track_id in scope {
            let track_index = candidate
                .tracks
                .iter()
                .position(|track| track.id == track_id)?;
            let tempo = candidate.tempo_bpm.max(1.0);
            let end_of = |clip: &AudioClip| clip.start_beats + clip.duration_seconds * tempo / 60.0;
            if candidate.tracks[track_index].clips.iter().any(|clip| {
                clip.locked && clip.start_beats < end - 1e-9 && end_of(clip) > start + 1e-9
            }) {
                return None;
            }
            // Split at the range start, then at the range end, so the pieces that
            // remain are ordinary events with intact audio.
            let crosses_start = candidate.tracks[track_index]
                .clips
                .iter()
                .filter(|clip| clip.start_beats < start - 1e-9 && end_of(clip) > start + 1e-9)
                .map(|clip| clip.id)
                .collect::<Vec<_>>();
            for clip_id in crosses_start {
                candidate.split_audio_clip(track_id, clip_id, start)?;
                changed += 1;
            }
            let crosses_end = candidate.tracks[track_index]
                .clips
                .iter()
                .filter(|clip| {
                    clip.start_beats >= start - 1e-9
                        && clip.start_beats < end - 1e-9
                        && end_of(clip) > end + 1e-9
                })
                .map(|clip| clip.id)
                .collect::<Vec<_>>();
            for clip_id in crosses_end {
                candidate.split_audio_clip(track_id, clip_id, end)?;
                changed += 1;
            }
            let track = &mut candidate.tracks[track_index];
            let before = track.clips.len();
            // Everything that starts inside the range goes; pieces that begin at
            // the end boundary are kept and shifted below.
            track
                .clips
                .retain(|clip| clip.start_beats < start - 1e-9 || clip.start_beats >= end - 1e-9);
            changed += before - track.clips.len();
            for clip in track.clips.iter_mut() {
                if clip.start_beats >= end - 1e-9 {
                    clip.start_beats = (clip.start_beats - gap).max(0.0);
                    changed += 1;
                }
            }

            // MIDI parts follow the same rules; they are not individually locked.
            let part_crosses_start = track
                .midi_parts
                .iter()
                .filter(|part| {
                    part.start_beats < start - 1e-9
                        && part.start_beats + part.duration_beats > start + 1e-9
                })
                .map(|part| part.id)
                .collect::<Vec<_>>();
            for part_id in part_crosses_start {
                candidate.split_midi_part(track_id, part_id, start)?;
                changed += 1;
            }
            let track = &mut candidate.tracks[track_index];
            let part_crosses_end = track
                .midi_parts
                .iter()
                .filter(|part| {
                    part.start_beats >= start - 1e-9
                        && part.start_beats < end - 1e-9
                        && part.start_beats + part.duration_beats > end + 1e-9
                })
                .map(|part| part.id)
                .collect::<Vec<_>>();
            for part_id in part_crosses_end {
                candidate.split_midi_part(track_id, part_id, end)?;
                changed += 1;
            }
            let track = &mut candidate.tracks[track_index];
            let before = track.midi_parts.len();
            track
                .midi_parts
                .retain(|part| part.start_beats < start - 1e-9 || part.start_beats >= end - 1e-9);
            changed += before - track.midi_parts.len();
            for part in track.midi_parts.iter_mut() {
                if part.start_beats >= end - 1e-9 {
                    part.start_beats = (part.start_beats - gap).max(0.0);
                    changed += 1;
                }
            }

            for lane in track.automation_lanes.iter_mut() {
                let before = lane.points.len();
                lane.points
                    .retain(|point| point.beat < start - 1e-9 || point.beat >= end - 1e-9);
                changed += before - lane.points.len();
                for point in lane.points.iter_mut() {
                    if point.beat >= end - 1e-9 {
                        point.beat = (point.beat - gap).max(0.0);
                        changed += 1;
                    }
                }
            }
        }
        if all_tracks {
            let before = candidate.markers.len();
            candidate
                .markers
                .retain(|marker| marker.beat < start - 1e-9 || marker.beat >= end - 1e-9);
            changed += before - candidate.markers.len();
            for marker in candidate.markers.iter_mut() {
                if marker.beat >= end - 1e-9 {
                    marker.beat = (marker.beat - gap).max(0.0);
                    changed += 1;
                }
            }
        }
        if changed == 0 || candidate.validate().is_err() {
            return None;
        }
        *self = candidate;
        Some(changed)
    }

    /// Fits the selected events' fades to the locator range.
    ///
    /// An event crossing the range keeps its material inside the range at full
    /// level: the fade-in covers whatever precedes the left locator and the
    /// fade-out whatever follows the right one. An event fully inside the range
    /// loses its fades, because the range already bounds it. Locked events and
    /// events outside the range are left alone.
    pub fn fit_audio_clip_fades_to_range(
        &mut self,
        ids: &[u64],
        start_beats: f64,
        end_beats: f64,
    ) -> usize {
        if !start_beats.is_finite()
            || !end_beats.is_finite()
            || start_beats < 0.0
            || end_beats <= start_beats
        {
            return 0;
        }
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let tempo = self.tempo_bpm.max(1.0);
        let mut changed = 0;
        for track in self.tracks.iter_mut() {
            for clip in track.clips.iter_mut() {
                if !selected.contains(&clip.id) || clip.locked {
                    continue;
                }
                let clip_end = clip.start_beats + clip.duration_seconds * tempo / 60.0;
                if clip_end <= start_beats + 1e-9 || clip.start_beats >= end_beats - 1e-9 {
                    continue;
                }
                let seconds_per_beat = 60.0 / tempo;
                let fade_in = ((start_beats - clip.start_beats) * seconds_per_beat)
                    .max(0.0)
                    .min(clip.duration_seconds);
                let fade_out = ((clip_end - end_beats) * seconds_per_beat)
                    .max(0.0)
                    .min((clip.duration_seconds - fade_in).max(0.0));
                if (clip.fade_in_seconds - fade_in).abs() > 1e-9
                    || (clip.fade_out_seconds - fade_out).abs() > 1e-9
                {
                    clip.fade_in_seconds = fade_in;
                    clip.fade_out_seconds = fade_out;
                    changed += 1;
                }
            }
        }
        changed
    }

    /// Splits a MIDI part into one new MIDI track per MIDI channel.
    ///
    /// Notes and controller events keep their own channel, so a part recorded on
    /// several channels becomes one editable track per channel. A channel that
    /// carries only controller data still gets a part, so no controller data is
    /// dropped. The channel count is bounded and the whole edit runs on a
    /// candidate, so a refusal leaves the project unchanged.
    pub fn explode_midi_part_by_channel(
        &mut self,
        track_id: u64,
        part_id: u64,
        max_channels: usize,
    ) -> Result<Vec<u64>, String> {
        let track_index = self
            .tracks
            .iter()
            .position(|track| track.id == track_id)
            .ok_or_else(|| "The MIDI track no longer exists".to_owned())?;
        let part_index = self.tracks[track_index]
            .midi_parts
            .iter()
            .position(|part| part.id == part_id)
            .ok_or_else(|| "The MIDI part no longer exists".to_owned())?;
        let original = self.tracks[track_index].midi_parts[part_index].clone();
        let mut channels = original
            .notes
            .iter()
            .map(|note| note.channel)
            .chain(original.controller_events.iter().map(|event| event.channel))
            .collect::<BTreeSet<_>>();
        if channels.is_empty() {
            return Err("The MIDI part has no notes or controller data to explode".to_owned());
        }
        if channels.len() > max_channels.max(1) {
            return Err(format!(
                "The part uses {} channels; exploding supports at most {}",
                channels.len(),
                max_channels.max(1)
            ));
        }
        let track_name = self.tracks[track_index].name.clone();
        let mut candidate = self.clone();
        candidate.tracks[track_index].midi_parts.remove(part_index);
        if candidate.selected_midi_part_id == Some(part_id) {
            candidate.selected_midi_part_id = None;
        }
        let mut parts_by_channel = BTreeMap::<u8, (usize, u64)>::new();
        let mut created = Vec::with_capacity(channels.len());
        for channel in channels.iter().copied() {
            candidate.add_track(TrackKind::Midi);
            let index = candidate.tracks.len() - 1;
            candidate.tracks[index].name = format!("{track_name} - channel {}", channel + 1);
            let new_part = candidate
                .add_midi_part(index, original.start_beats, original.duration_beats)
                .ok_or_else(|| "Could not create an exploded part".to_owned())?;
            if let Some(part) = candidate.tracks[index]
                .midi_parts
                .iter_mut()
                .find(|part| part.id == new_part)
            {
                part.name = format!("{} (channel {})", original.name, channel + 1);
            }
            for note in original.notes.iter().filter(|note| note.channel == channel) {
                candidate
                    .add_midi_note_on_channel(
                        index,
                        new_part,
                        note.start_beats,
                        note.duration_beats,
                        note.pitch,
                        note.channel,
                        note.velocity,
                    )
                    .ok_or_else(|| "Could not copy an exploded note".to_owned())?;
            }
            parts_by_channel.insert(channel, (index, new_part));
            created.push(new_part);
        }
        for event in &original.controller_events {
            // Every controller channel has a part, because the channel set is
            // built from the controller data too.
            let Some((index, part)) = parts_by_channel.get(&event.channel).copied() else {
                return Err("Could not place a controller event".to_owned());
            };
            candidate
                .add_midi_controller_event_on_channel(
                    index,
                    part,
                    event.position_beats,
                    event.controller,
                    event.channel,
                    event.value,
                )
                .ok_or_else(|| "Could not copy a controller event".to_owned())?;
        }
        candidate
            .validate()
            .map_err(|error| format!("The exploded part would not be valid: {error}"))?;
        *self = candidate;
        Ok(created)
    }

    /// Splits a MIDI part into one new MIDI track per distinct pitch.
    ///
    /// Each new track receives a copy of the part containing only that pitch's
    /// notes, and the original part is removed. Controller events have no pitch,
    /// so they are copied once — into the lowest-pitched exploded part — rather
    /// than lost or duplicated. The pitch count is bounded so a dense drum part
    /// cannot create an unbounded number of tracks; the whole edit is refused
    /// rather than left half-applied.
    pub fn explode_midi_part_by_pitch(
        &mut self,
        track_id: u64,
        part_id: u64,
        max_pitches: usize,
    ) -> Result<Vec<u64>, String> {
        let track_index = self
            .tracks
            .iter()
            .position(|track| track.id == track_id)
            .ok_or_else(|| "The MIDI track no longer exists".to_owned())?;
        let part_index = self.tracks[track_index]
            .midi_parts
            .iter()
            .position(|part| part.id == part_id)
            .ok_or_else(|| "The MIDI part no longer exists".to_owned())?;
        let original = self.tracks[track_index].midi_parts[part_index].clone();
        let pitches = original
            .notes
            .iter()
            .map(|note| note.pitch)
            .collect::<BTreeSet<_>>();
        if pitches.is_empty() {
            return Err("The MIDI part has no notes to explode".to_owned());
        }
        if pitches.len() > max_pitches.max(1) {
            return Err(format!(
                "The part uses {} pitches; exploding supports at most {}",
                pitches.len(),
                max_pitches.max(1)
            ));
        }
        let track_name = self.tracks[track_index].name.clone();
        let notes = original.notes.clone();
        let controllers = original.controller_events.clone();
        let lowest = *pitches.iter().next().expect("non-empty");
        let mut candidate = self.clone();
        candidate.tracks[track_index].midi_parts.remove(part_index);
        if candidate.selected_midi_part_id == Some(part_id) {
            candidate.selected_midi_part_id = None;
        }
        let mut created = Vec::with_capacity(pitches.len());
        for pitch in pitches.iter().copied() {
            candidate.add_track(TrackKind::Midi);
            let index = candidate.tracks.len() - 1;
            candidate.tracks[index].name = format!("{track_name} - pitch {pitch}");
            let new_part = candidate
                .add_midi_part(index, original.start_beats, original.duration_beats)
                .ok_or_else(|| "Could not create an exploded part".to_owned())?;
            if let Some(part) = candidate.tracks[index]
                .midi_parts
                .iter_mut()
                .find(|part| part.id == new_part)
            {
                part.name = format!("{} ({pitch})", original.name);
            }
            for note in notes.iter().filter(|note| note.pitch == pitch) {
                candidate
                    .add_midi_note_on_channel(
                        index,
                        new_part,
                        note.start_beats,
                        note.duration_beats,
                        note.pitch,
                        note.channel,
                        note.velocity,
                    )
                    .ok_or_else(|| "Could not copy an exploded note".to_owned())?;
            }
            if pitch == lowest {
                for event in &controllers {
                    candidate
                        .add_midi_controller_event_on_channel(
                            index,
                            new_part,
                            event.position_beats,
                            event.controller,
                            event.channel,
                            event.value,
                        )
                        .ok_or_else(|| "Could not copy a controller event".to_owned())?;
                }
            }
            created.push(new_part);
        }
        candidate
            .validate()
            .map_err(|error| format!("The exploded part would not be valid: {error}"))?;
        *self = candidate;
        Ok(created)
    }

    /// Inserts `beats` of silence at `beat` and reports how many items moved.
    ///
    /// Events and MIDI parts that cross the insertion point are split first, so
    /// their right-hand audio moves with the rest of the timeline instead of
    /// being stretched. Markers and automation points at or after the point move
    /// too. A locked event crossing the point refuses the whole edit: inserting
    /// half a gap under a locked event would silently change what it plays.
    pub fn insert_silence_at(&mut self, beat: f64, beats: f64) -> Option<usize> {
        if !beat.is_finite()
            || beat < 0.0
            || !beats.is_finite()
            || beats <= 0.0
            || beats > MAX_LOOP_BEATS
        {
            return None;
        }
        let mut candidate = self.clone();
        let mut changed = 0;
        let track_ids = candidate
            .tracks
            .iter()
            .map(|track| track.id)
            .collect::<Vec<_>>();
        for track_id in track_ids {
            let track_index = candidate
                .tracks
                .iter()
                .position(|track| track.id == track_id)?;
            // Split every crossing event so no event spans the inserted gap.
            let crossing = candidate.tracks[track_index]
                .clips
                .iter()
                .filter(|clip| {
                    let end = clip.start_beats + clip.duration_seconds * candidate.tempo_bpm / 60.0;
                    clip.start_beats < beat - 1e-9 && end > beat + 1e-9
                })
                .map(|clip| (clip.id, clip.locked))
                .collect::<Vec<_>>();
            if crossing.iter().any(|(_, locked)| *locked) {
                return None;
            }
            for (clip_id, _) in crossing {
                candidate.split_audio_clip(track_id, clip_id, beat)?;
            }
            let crossing_parts = candidate.tracks[track_index]
                .midi_parts
                .iter()
                .filter(|part| {
                    part.start_beats < beat - 1e-9
                        && part.start_beats + part.duration_beats > beat + 1e-9
                })
                .map(|part| part.id)
                .collect::<Vec<_>>();
            for part_id in crossing_parts {
                candidate.split_midi_part(track_id, part_id, beat)?;
            }

            let track = &mut candidate.tracks[track_index];
            for clip in track.clips.iter_mut() {
                if clip.start_beats >= beat - 1e-9 {
                    clip.start_beats += beats;
                    changed += 1;
                }
            }
            for part in track.midi_parts.iter_mut() {
                if part.start_beats >= beat - 1e-9 {
                    part.start_beats += beats;
                    changed += 1;
                }
            }
            for lane in track.automation_lanes.iter_mut() {
                for point in lane.points.iter_mut() {
                    if point.beat >= beat - 1e-9 {
                        point.beat += beats;
                        changed += 1;
                    }
                }
            }
        }
        for marker in candidate.markers.iter_mut() {
            if marker.beat >= beat - 1e-9 {
                marker.beat += beats;
                changed += 1;
            }
        }
        candidate
            .markers
            .sort_by(|left, right| left.beat.total_cmp(&right.beat));
        if changed == 0 || candidate.validate().is_err() {
            return None;
        }
        *self = candidate;
        Some(changed)
    }

    /// Moves one selected event's start or end by `delta_beats`.
    ///
    /// Start moves carry the source offset with them, so the audio under the
    /// remaining part does not slide; an end move only changes the duration. The
    /// result stays inside the source media, above the minimum length and inside
    /// the project, and a locked event is refused.
    pub fn nudge_audio_clip_edge(
        &mut self,
        clip_id: u64,
        edge: ClipEdge,
        delta_beats: f64,
        source_duration_seconds: f64,
        min_seconds: f64,
    ) -> bool {
        let tempo = self.tempo_bpm.max(1.0);
        let seconds_per_beat = 60.0 / tempo;
        if !delta_beats.is_finite() || delta_beats == 0.0 || seconds_per_beat <= 0.0 {
            return false;
        }
        let min_seconds = if min_seconds.is_finite() && min_seconds > 0.0 {
            min_seconds
        } else {
            MIN_CLIP_EDGE_SECONDS
        };
        for track in self.tracks.iter_mut() {
            for clip in track.clips.iter_mut() {
                if clip.id != clip_id {
                    continue;
                }
                if clip.locked {
                    return false;
                }
                let Some(next) = nudged_clip_edge(
                    clip,
                    edge,
                    delta_beats,
                    tempo,
                    source_duration_seconds,
                    min_seconds,
                ) else {
                    return false;
                };
                *clip = next;
                return true;
            }
        }
        false
    }

    /// Sets the selected events' gain from a decibel value.
    ///
    /// The linear result is clamped to the event-gain range (0 to 4.0, about
    /// +12 dB), -60 dB and below is silence, and locked events are skipped, so a
    /// gain edit cannot make the project invalid.
    pub fn set_audio_clip_gain_db(&mut self, ids: &[u64], db: f64) -> usize {
        if !db.is_finite() {
            return 0;
        }
        let linear = if db <= -60.0 {
            0.0
        } else {
            10.0_f64.powf(db / 20.0).clamp(0.0, MAX_AUDIO_CLIP_GAIN) as f32
        };
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut changed = 0;
        for track in self.tracks.iter_mut() {
            for clip in track.clips.iter_mut() {
                if !selected.contains(&clip.id) || clip.locked {
                    continue;
                }
                if (clip.gain - linear).abs() > 1e-6 {
                    clip.gain = linear;
                    changed += 1;
                }
            }
        }
        changed
    }

    /// Applies a fade to the selected events and reports how many changed.
    ///
    /// The fade is clamped to the event and to whatever room the opposite fade
    /// leaves, so a short event never becomes invalid. A locked event is skipped.
    pub fn fade_audio_clips(&mut self, ids: &[u64], fade_in: bool, seconds: f64) -> usize {
        if !seconds.is_finite() || seconds <= 0.0 {
            return 0;
        }
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut changed = 0;
        for track in self.tracks.iter_mut() {
            for clip in track.clips.iter_mut() {
                if !selected.contains(&clip.id) || clip.locked {
                    continue;
                }
                let other = if fade_in {
                    clip.fade_out_seconds
                } else {
                    clip.fade_in_seconds
                };
                let next = seconds
                    .min(clip.duration_seconds)
                    .min((clip.duration_seconds - other).max(0.0));
                let target = if fade_in {
                    &mut clip.fade_in_seconds
                } else {
                    &mut clip.fade_out_seconds
                };
                if (*target - next).abs() > 1e-9 {
                    *target = next;
                    changed += 1;
                }
            }
        }
        changed
    }

    /// Silences the selected events by setting their gain to zero.
    ///
    /// The media and the event stay, so Undo and the pool both keep the audio;
    /// only the channel amplification changes.
    pub fn silence_audio_clips(&mut self, ids: &[u64]) -> usize {
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut changed = 0;
        for track in self.tracks.iter_mut() {
            for clip in track.clips.iter_mut() {
                if selected.contains(&clip.id) && !clip.locked && clip.gain != 0.0 {
                    clip.gain = 0.0;
                    changed += 1;
                }
            }
        }
        changed
    }

    /// The earliest event start strictly after `beat`, across audio and MIDI.
    ///
    /// "Event" means the beginning of an audio event or a MIDI part, which is
    /// what a locate command moves to. The search is strict, so pressing the key
    /// twice always advances instead of landing on the same place.
    pub fn next_event_start_after(&self, beat: f64) -> Option<f64> {
        if !beat.is_finite() {
            return None;
        }
        self.tracks
            .iter()
            .flat_map(|track| {
                track
                    .clips
                    .iter()
                    .map(|clip| clip.start_beats)
                    .chain(track.midi_parts.iter().map(|part| part.start_beats))
            })
            .filter(|start| start.is_finite() && *start > beat + 1e-9)
            .min_by(f64::total_cmp)
    }

    /// The latest event start strictly before `beat`, across audio and MIDI.
    pub fn previous_event_start_before(&self, beat: f64) -> Option<f64> {
        if !beat.is_finite() {
            return None;
        }
        self.tracks
            .iter()
            .flat_map(|track| {
                track
                    .clips
                    .iter()
                    .map(|clip| clip.start_beats)
                    .chain(track.midi_parts.iter().map(|part| part.start_beats))
            })
            .filter(|start| start.is_finite() && *start < beat - 1e-9)
            .max_by(f64::total_cmp)
    }

    /// Expands or collapses every group track and reports how many changed.
    pub fn set_all_groups_collapsed(&mut self, collapsed: bool) -> usize {
        let mut changed = 0;
        for track in self.tracks.iter_mut() {
            if track.kind == TrackKind::Group && track.group_collapsed != collapsed {
                track.group_collapsed = collapsed;
                changed += 1;
            }
        }
        changed
    }

    /// Replaces events on one track with a single rendered event.
    ///
    /// Bounce in place renders a range of a track through its own chain, so the
    /// result is one event that sounds like the replaced ones. The replaced events
    /// leave the arrangement (their files stay in the pool, so normal deletion
    /// rules still protect the media) and the new event covers their span.
    pub fn replace_audio_clips_with_bounce(
        &mut self,
        track_id: u64,
        replaced: &[u64],
        source_path: PathBuf,
        start_beats: f64,
        duration_seconds: f64,
    ) -> Option<u64> {
        if replaced.is_empty() {
            return None;
        }
        let track_index = self
            .tracks
            .iter()
            .position(|track| track.id == track_id && track.kind == TrackKind::Audio)?;
        let selected: BTreeSet<u64> = replaced.iter().copied().collect();
        let found = self.tracks[track_index]
            .clips
            .iter()
            .filter(|clip| selected.contains(&clip.id))
            .count();
        if found != selected.len() {
            return None;
        }
        if self.tracks[track_index]
            .clips
            .iter()
            .any(|clip| selected.contains(&clip.id) && clip.locked)
        {
            // A locked event is a promise not to change it, so bouncing refuses
            // instead of quietly unlocking and replacing it.
            return None;
        }
        let id = self.add_audio_clip(track_index, source_path, start_beats, duration_seconds)?;
        let track = &mut self.tracks[track_index];
        track
            .clips
            .retain(|clip| !selected.contains(&clip.id) || clip.id == id);
        track
            .clips
            .sort_by(|left, right| left.start_beats.total_cmp(&right.start_beats));
        if self.validate().is_err() {
            return None;
        }
        Some(id)
    }

    /// Splits one continuous loop recording into one clip per cycle pass.
    ///
    /// The recorder writes one file for the whole take, so each pass is a clip
    /// whose source offset points at that pass's audio and whose timeline start is
    /// the loop start again. The last pass is left active, which is what a user
    /// expects after recording several loops.
    ///
    /// Returns `None` when the take did not cross a loop boundary, so the caller
    /// keeps the ordinary single-clip behavior.
    pub fn add_loop_take_clips(
        &mut self,
        track_index: usize,
        source_path: PathBuf,
        start_beats: f64,
        duration_seconds: f64,
        loop_start_beats: f64,
        loop_end_beats: f64,
    ) -> Option<Vec<u64>> {
        let pass_length = loop_end_beats - loop_start_beats;
        if !pass_length.is_finite() || pass_length <= 1e-6 || self.next_take_group == u64::MAX {
            return None;
        }
        let beats_to_seconds = 60.0 / self.tempo_bpm.max(1.0);
        let take_end = start_beats + duration_seconds / beats_to_seconds;
        // Nothing to split when the take never reached the next pass.
        if take_end <= loop_end_beats + 1e-6 || start_beats >= loop_end_beats - 1e-6 {
            return None;
        }
        let mut segments = Vec::new();
        let mut source_offset = 0.0;
        let mut remaining = duration_seconds;
        // Pass one starts where the recording started and runs to the loop end.
        let first_beats = (loop_end_beats - start_beats).max(0.0);
        let mut segment_beats = first_beats.min(remaining / beats_to_seconds);
        loop {
            if segment_beats <= 1e-6 || remaining <= 1e-6 {
                break;
            }
            let segment_seconds = (segment_beats * beats_to_seconds).min(remaining);
            segments.push((source_offset, segment_seconds));
            source_offset += segment_seconds;
            remaining -= segment_seconds;
            segment_beats = pass_length;
        }
        if segments.len() < 2 {
            return None;
        }
        let group = self.next_take_group;
        self.next_take_group += 1;
        let last = segments.len() - 1;
        let mut created = Vec::with_capacity(segments.len());
        for (index, (offset, seconds)) in segments.iter().enumerate() {
            let start = if index == 0 {
                start_beats
            } else {
                loop_start_beats
            };
            let id = self.add_audio_clip(track_index, source_path.clone(), start, *seconds)?;
            let track = self.tracks.get_mut(track_index)?;
            let clip = track.clips.iter_mut().find(|clip| clip.id == id)?;
            clip.source_offset_seconds = *offset;
            clip.take_group = Some(group);
            clip.take_active = index == last;
            created.push(id);
        }
        if self.validate().is_err() {
            return None;
        }
        Some(created)
    }

    /// Makes one take of a loop group active and reports whether it changed.
    pub fn set_active_take(&mut self, clip_id: u64) -> bool {
        let group = self
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .find(|clip| clip.id == clip_id)
            .and_then(|clip| clip.take_group);
        let Some(group) = group else {
            return false;
        };
        let mut changed = false;
        for clip in self
            .tracks
            .iter_mut()
            .flat_map(|track| track.clips.iter_mut())
        {
            if clip.take_group != Some(group) {
                continue;
            }
            let active = clip.id == clip_id;
            if clip.take_active != active {
                clip.take_active = active;
                changed = true;
            }
        }
        changed
    }

    /// Locks or unlocks the given events and reports how many changed.
    ///
    /// A locked event keeps its audio and its place but refuses move, trim,
    /// split and crop, so a finished take cannot be nudged by accident.
    pub fn set_audio_clips_locked(&mut self, ids: &[u64], locked: bool) -> usize {
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut changed = 0;
        for track in self
            .tracks
            .iter_mut()
            .filter(|track| track.kind == TrackKind::Audio)
        {
            for clip in track.clips.iter_mut() {
                if selected.contains(&clip.id) && clip.locked != locked {
                    clip.locked = locked;
                    changed += 1;
                }
            }
        }
        changed
    }

    /// Binds the given events into one group and returns its id.
    ///
    /// Two events are the minimum: a group of one has nothing to move with it,
    /// and the user can already select one event.
    pub fn group_audio_clips(&mut self, ids: &[u64]) -> Option<u64> {
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let members = self.member_clip_ids(&selected);
        if members.len() < 2 {
            return None;
        }
        // Continue after the highest id in use so a removed group is not reused
        // by a later one while its old members still carry the number.
        let next = self
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .filter_map(|clip| clip.group)
            .max()
            .unwrap_or(0)
            .checked_add(1)?;
        let mut changed = false;
        for track in self
            .tracks
            .iter_mut()
            .filter(|track| track.kind == TrackKind::Audio)
        {
            for clip in track.clips.iter_mut() {
                if members.contains(&clip.id) && clip.group != Some(next) {
                    clip.group = Some(next);
                    changed = true;
                }
            }
        }
        changed.then_some(next)
    }

    /// Removes the given events from their groups and reports how many changed.
    ///
    /// Selecting one member ungroups the whole group, because a group that lost
    /// half its members would move unpredictably.
    pub fn ungroup_audio_clips(&mut self, ids: &[u64]) -> usize {
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let groups = self
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .filter(|clip| selected.contains(&clip.id))
            .filter_map(|clip| clip.group)
            .collect::<BTreeSet<_>>();
        if groups.is_empty() {
            return 0;
        }
        let mut changed = 0;
        for track in self
            .tracks
            .iter_mut()
            .filter(|track| track.kind == TrackKind::Audio)
        {
            for clip in track.clips.iter_mut() {
                if clip.group.is_some_and(|group| groups.contains(&group)) {
                    clip.group = None;
                    changed += 1;
                }
            }
        }
        changed
    }

    /// The given events plus every member of their groups.
    ///
    /// Selection uses this so clicking a grouped event selects the whole object.
    pub fn expand_selection_to_groups(&self, ids: &[u64]) -> Vec<u64> {
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        self.member_clip_ids(&selected).into_iter().collect()
    }

    fn member_clip_ids(&self, selected: &BTreeSet<u64>) -> BTreeSet<u64> {
        let groups = self
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .filter(|clip| selected.contains(&clip.id))
            .filter_map(|clip| clip.group)
            .collect::<BTreeSet<_>>();
        let mut members = selected.clone();
        for track in self.tracks.iter().filter(|t| t.kind == TrackKind::Audio) {
            for clip in &track.clips {
                if clip.group.is_some_and(|group| groups.contains(&group)) {
                    members.insert(clip.id);
                }
            }
        }
        members
    }

    /// Mutes or unmutes the given events and reports how many changed.
    pub fn set_audio_clips_muted(&mut self, ids: &[u64], muted: bool) -> usize {
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut changed = 0;
        for track in self
            .tracks
            .iter_mut()
            .filter(|track| track.kind == TrackKind::Audio)
        {
            for clip in track.clips.iter_mut() {
                if selected.contains(&clip.id) && clip.muted != muted {
                    clip.muted = muted;
                    changed += 1;
                }
            }
        }
        changed
    }

    /// A copy of this project's structure with no content in it.
    ///
    /// A template keeps what a user sets up once — tracks, groups, routing, gains,
    /// EQ, inserts, sends and automation lanes — and drops what belongs to a
    /// particular song: events, MIDI parts, markers, the media pool and the
    /// selection. Timeline positions reset so a new song starts at bar one.
    pub fn as_template(&self) -> Project {
        let mut template = self.clone();
        for track in template.tracks.iter_mut() {
            track.clips.clear();
            track.midi_parts.clear();
            track.armed = false;
            track.monitor = false;
        }
        template.media_pool.clear();
        template.markers.clear();
        template.next_marker_id = first_marker_id();
        template.next_clip_id = 1;
        template.next_midi_part_id = 1;
        template.next_midi_note_id = 1;
        template.next_midi_controller_id = 1;
        template.selected_track = None;
        template.selected_midi_part_id = None;
        template.loop_start_beats = default_loop_start_beats();
        template.loop_end_beats = default_loop_end_beats();
        template.cycle_mode = CycleMode::Loop;
        template.name = "Untitled Project".to_owned();
        template
    }

    /// Sources the project references that are not on disk.
    ///
    /// Sorted and de-duplicated, so a caller can present and relink them once.
    pub fn missing_media(&self) -> Vec<PathBuf> {
        self.media_paths()
            .into_iter()
            .filter(|path| !path.as_os_str().is_empty() && !path.exists())
            .collect()
    }

    /// Points every reference from `from` to `to`, returning how many events moved.
    ///
    /// The media pool follows the same mapping, so a relinked project has no
    /// dangling entry left behind.
    pub fn relink_media(&mut self, mapping: &BTreeMap<PathBuf, PathBuf>) -> usize {
        if mapping.is_empty() {
            return 0;
        }
        let mut relinked = 0;
        for track in self.tracks.iter_mut() {
            for clip in track.clips.iter_mut() {
                if let Some(replacement) = mapping.get(&clip.source_path) {
                    clip.source_path = replacement.clone();
                    relinked += 1;
                }
            }
        }
        self.media_pool = self
            .media_pool
            .iter()
            .map(|path| mapping.get(path).cloned().unwrap_or_else(|| path.clone()))
            .collect();
        relinked
    }

    /// Joins contiguous pieces of one source into a single event.
    ///
    /// Glue only claims what it can do without rendering: the pieces must sit on
    /// one track, share the source file and gain, be contiguous in both timeline
    /// beats and source seconds, and carry no Pitch & Time edit (whose analysis is
    /// tied to the piece). Anything else is refused, because joining two different
    /// sources would need a bounce, which is a separate command.
    pub fn glue_audio_clips(&mut self, ids: &[u64]) -> Option<u64> {
        if ids.len() < 2 {
            return None;
        }
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        let mut track_index = None;
        let mut pieces = Vec::new();
        for (index, track) in self.tracks.iter().enumerate() {
            if track.kind != TrackKind::Audio {
                continue;
            }
            for clip in &track.clips {
                if !selected.contains(&clip.id) {
                    continue;
                }
                if track_index.is_some_and(|found| found != index) {
                    return None;
                }
                track_index = Some(index);
                pieces.push(clip.clone());
            }
        }
        let track_index = track_index?;
        if pieces.len() != ids.len() || pieces.iter().any(|clip| clip.locked) {
            return None;
        }
        let source = pieces[0].source_path.clone();
        let gain = pieces[0].gain;
        if pieces.iter().any(|clip| {
            clip.source_path != source
                || clip.gain != gain
                || clip.pitch_time.is_some()
                // A split records the event's fades in the source-fade lineage;
                // merging across a boundary would need that lineage recomputed,
                // so faded pieces are refused instead of silently losing fades.
                || !clip.source_fades.is_empty()
                || clip.fade_in_seconds > 0.0
                || clip.fade_out_seconds > 0.0
        }) {
            return None;
        }
        pieces.sort_by(|left, right| left.start_beats.total_cmp(&right.start_beats));
        let tempo = self.tempo_bpm;
        let beats = |clip: &AudioClip| clip.duration_seconds * tempo / 60.0;
        for pair in pieces.windows(2) {
            let (left, right) = (&pair[0], &pair[1]);
            let contiguous_beats =
                (left.start_beats + beats(left) - right.start_beats).abs() < 1e-6;
            let contiguous_source = (left.source_offset_seconds + left.duration_seconds
                - right.source_offset_seconds)
                .abs()
                < 1e-6;
            if !contiguous_beats || !contiguous_source {
                return None;
            }
        }
        let first = pieces.first()?.clone();
        let last = pieces.last()?;
        let mut merged = first.clone();
        merged.duration_seconds =
            (last.start_beats + beats(last) - first.start_beats) * 60.0 / tempo;
        // The outer fades belong to the joined event; inner ones disappear with
        // the boundary they softened.
        merged.fade_in_seconds = 0.0;
        merged.fade_out_seconds = 0.0;
        merged.source_fades.clear();
        let keep = merged.id;
        let track = &mut self.tracks[track_index];
        track.clips.retain(|clip| !selected.contains(&clip.id));
        track.clips.push(merged);
        track
            .clips
            .sort_by(|left, right| left.start_beats.total_cmp(&right.start_beats));
        if self.validate().is_err() {
            return None;
        }
        Some(keep)
    }

    /// Removes the material inside `[start, end)` from the given events.
    ///
    /// Each crossing event is split at both boundaries and the middle piece is
    /// discarded, leaving a gap rather than closing it — deleting a range is not
    /// a ripple edit. An event fully inside the range is removed entirely.
    pub fn delete_range_from_audio_clips(&mut self, ids: &[u64], start: f64, end: f64) -> usize {
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
            return 0;
        }
        let selected: BTreeSet<u64> = ids.iter().copied().collect();
        if selected.is_empty() {
            return 0;
        }
        let mut changed = 0;
        for track_id in self
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
            .map(|track| track.id)
            .collect::<Vec<_>>()
        {
            let targets = self
                .tracks
                .iter()
                .find(|track| track.id == track_id)
                .map(|track| {
                    track
                        .clips
                        .iter()
                        .filter(|clip| selected.contains(&clip.id))
                        .map(|clip| {
                            (
                                clip.id,
                                clip.start_beats,
                                clip.start_beats + clip.duration_seconds * self.tempo_bpm / 60.0,
                                clip.locked,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for (clip_id, clip_start, clip_end, locked) in targets {
                if locked || clip_end <= start + 1e-9 || clip_start >= end - 1e-9 {
                    continue;
                }
                // Fully inside: the event goes away.
                if clip_start >= start - 1e-9 && clip_end <= end + 1e-9 {
                    remove_clip(self, track_id, clip_id);
                    changed += 1;
                    continue;
                }
                // Trim the tail beyond `end` first so the ids stay stable.
                if clip_end > end + 1e-9
                    && let Some(right) = self.split_audio_clip(track_id, clip_id, end)
                    && clip_start >= start - 1e-9
                {
                    // The event starts inside: keep the surviving right part and
                    // drop the head that reaches into the range.
                    remove_clip(self, track_id, clip_id);
                    rename_clip(self, track_id, right, clip_id);
                    changed += 1;
                    continue;
                }
                if clip_start < start - 1e-9 {
                    match self.split_audio_clip(track_id, clip_id, start) {
                        Some(middle) => {
                            // `middle` is the piece starting at `start`. Cut it at
                            // `end` when it reaches past the range, then drop it.
                            if clip_end > end + 1e-9 {
                                self.split_audio_clip(track_id, middle, end);
                            }
                            remove_clip(self, track_id, middle);
                            changed += 1;
                        }
                        None => continue,
                    }
                }
            }
        }
        changed
    }

    /// Splits every audio event crossing either locator.
    ///
    /// The loop range is the target, so this is the "Split Loop" command: each
    /// crossing event becomes two, and an event that spans the whole loop becomes
    /// three. Locked events are skipped by `split_audio_clip` itself.
    pub fn split_audio_clips_at_loop(&mut self) -> (usize, Vec<u64>) {
        let mut boundaries = [self.loop_start_beats, self.loop_end_beats];
        boundaries.sort_by(f64::total_cmp);
        let mut split = 0;
        let mut created = Vec::new();
        for track_id in self
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
            .map(|track| track.id)
            .collect::<Vec<_>>()
        {
            for boundary in boundaries {
                if !boundary.is_finite() || boundary <= 0.0 {
                    continue;
                }
                // Re-read the clips each pass: a previous split produced new ids.
                let targets = self
                    .tracks
                    .iter()
                    .find(|track| track.id == track_id)
                    .map(|track| {
                        track
                            .clips
                            .iter()
                            .filter(|clip| {
                                let end = clip.start_beats
                                    + clip.duration_seconds * self.tempo_bpm / 60.0;
                                clip.start_beats < boundary - 1e-9 && end > boundary + 1e-9
                            })
                            .map(|clip| clip.id)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                for clip_id in targets {
                    if let Some(id) = self.split_audio_clip(track_id, clip_id, boundary) {
                        split += 1;
                        created.push(id);
                    }
                }
            }
        }
        (split, created)
    }

    /// Split every descendant audio event crossing one timeline position and
    /// retain the shared virtual-event boundary on the group channel.
    pub fn split_group_audio_clips(&mut self, group_id: u64, at_beats: f64) -> (bool, Vec<u64>) {
        if !at_beats.is_finite()
            || !self
                .tracks
                .iter()
                .any(|track| track.id == group_id && track.kind == TrackKind::Group)
        {
            return (false, Vec::new());
        }
        let mut candidate = self.clone();
        let mut start = f64::INFINITY;
        let mut end = f64::NEG_INFINITY;
        for track in candidate.tracks.iter().filter(|track| {
            track.kind == TrackKind::Audio
                && track_belongs_to_group(&candidate.tracks, track.id, group_id)
        }) {
            for clip in &track.clips {
                start = start.min(clip.start_beats);
                end =
                    end.max(clip.start_beats + clip.duration_seconds * candidate.tempo_bpm / 60.0);
            }
        }
        if !start.is_finite() || at_beats <= start || at_beats >= end {
            return (false, Vec::new());
        }
        let Some(group_index) = candidate
            .tracks
            .iter()
            .position(|track| track.id == group_id && track.kind == TrackKind::Group)
        else {
            return (false, Vec::new());
        };
        if candidate.tracks[group_index]
            .group_cut_beats
            .iter()
            .any(|beat| (*beat - at_beats).abs() < 1e-6)
        {
            return (false, Vec::new());
        }
        let targets = candidate
            .tracks
            .iter()
            .filter(|track| {
                track.kind == TrackKind::Audio
                    && track_belongs_to_group(&candidate.tracks, track.id, group_id)
            })
            .flat_map(|track| {
                track.clips.iter().filter_map(move |clip| {
                    let end = clip.start_beats + clip.duration_seconds * candidate.tempo_bpm / 60.0;
                    (at_beats > clip.start_beats && at_beats < end).then_some((track.id, clip.id))
                })
            })
            .collect::<Vec<_>>();
        let mut right_ids = Vec::with_capacity(targets.len());
        for (track_id, clip_id) in targets {
            let Some(right_id) = candidate.split_audio_clip(track_id, clip_id, at_beats) else {
                return (false, Vec::new());
            };
            right_ids.push(right_id);
        }
        let Some(group) = candidate
            .tracks
            .iter_mut()
            .find(|track| track.id == group_id)
        else {
            return (false, Vec::new());
        };
        group.group_cut_beats.push(at_beats);
        group.group_cut_beats.sort_by(f64::total_cmp);
        if candidate.validate().is_err() {
            return (false, Vec::new());
        }
        *self = candidate;
        (true, right_ids)
    }
    /// Linear overlap crossfade. Does not extend events or alter source media.
    pub fn crossfade_audio_clips(&mut self, first: u64, second: u64) -> bool {
        if first == second {
            return false;
        }
        let Some(t) = self.tracks.iter().position(|t| {
            t.kind == TrackKind::Audio
                && t.clips.iter().any(|c| c.id == first)
                && t.clips.iter().any(|c| c.id == second)
        }) else {
            return false;
        };
        let mut indices = [
            self.tracks[t]
                .clips
                .iter()
                .position(|c| c.id == first)
                .unwrap(),
            self.tracks[t]
                .clips
                .iter()
                .position(|c| c.id == second)
                .unwrap(),
        ];
        indices.sort_by(|a, b| {
            self.tracks[t].clips[*a]
                .start_beats
                .total_cmp(&self.tracks[t].clips[*b].start_beats)
        });
        let a = &self.tracks[t].clips[indices[0]];
        let b = &self.tracks[t].clips[indices[1]];
        let start_delta = (b.start_beats - a.start_beats) * 60.0 / self.tempo_bpm;
        let overlap = a.duration_seconds - start_delta;
        if start_delta <= 0.0
            || overlap < 1.0 / self.sample_rate_hz.max(1) as f64
            || overlap > b.duration_seconds
        {
            return false;
        }
        let mut candidate = self.clone();
        let a = &mut candidate.tracks[t].clips[indices[0]];
        a.fade_out_seconds = overlap;
        a.fade_in_seconds = a.fade_in_seconds.min(a.duration_seconds - overlap);
        let b = &mut candidate.tracks[t].clips[indices[1]];
        b.fade_in_seconds = overlap;
        b.fade_out_seconds = b.fade_out_seconds.min(b.duration_seconds - overlap);
        if candidate == *self || candidate.validate().is_err() {
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
        self.media_pool.insert(source_path.clone());
        track.clips.push(AudioClip {
            pitch_time: None,
            source_fades: Vec::new(),
            id,
            name,
            source_path,
            start_beats,
            duration_seconds,
            source_offset_seconds: 0.0,
            fade_in_seconds: 0.0,
            fade_out_seconds: 0.0,
            gain: 1.0,
            muted: false,
            locked: false,
            group: None,
            take_group: None,
            take_active: true,
        });
        Some(id)
    }

    /// Pastes cloned audio events into the supplied audio-track indices.
    /// Event IDs are reassigned and source media is retained in the pool.
    /// The operation is atomic: invalid destinations or clips leave the project unchanged.
    pub fn paste_audio_clips(&mut self, clips: &[(usize, AudioClip)]) -> Option<Vec<u64>> {
        if clips.is_empty() {
            return None;
        }
        let mut candidate = self.clone();
        let mut new_ids = Vec::with_capacity(clips.len());
        for (track_index, source) in clips {
            if source.source_path.as_os_str().is_empty() {
                return None;
            }
            let track = candidate.tracks.get(*track_index)?;
            if track.kind != TrackKind::Audio || track.audio_channels == 0 {
                return None;
            }
            let id = candidate.next_clip_id;
            candidate.next_clip_id = candidate.next_clip_id.checked_add(1)?;
            let mut clip = source.clone();
            clip.id = id;
            candidate.media_pool.insert(clip.source_path.clone());
            candidate.tracks[*track_index].clips.push(clip);
            new_ids.push(id);
        }
        if candidate.validate().is_err() {
            return None;
        }
        *self = candidate;
        Some(new_ids)
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
    /// Checks every channel's routing against the project it belongs to.
    ///
    /// [`Self::validate`] keeps its historical short messages; routing needs to
    /// say *which* route is wrong, so it reports the parsed model's error. A
    /// project that fails this cannot play what it stores, and the save path
    /// refuses it instead of letting playback silently fall back to the master.
    pub fn validate_routing(&self) -> Result<(), crate::routing::RoutingError> {
        use crate::routing::{InputRoute, OutputRoute, RoutingError};

        let group_ids = self
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Group)
            .map(|track| track.id)
            .collect::<BTreeSet<_>>();

        // A destination may not be the channel itself or anything it already
        // feeds, which is the same rule the routing menu uses.
        let feeds_cycle = |track_id: u64, target: u64| -> bool {
            let mut pending = vec![target];
            let mut visited = BTreeSet::new();
            while let Some(current) = pending.pop() {
                if current == track_id {
                    return true;
                }
                if !visited.insert(current) {
                    continue;
                }
                let Some(track) = self.tracks.iter().find(|track| track.id == current) else {
                    continue;
                };
                let destinations = track
                    .output_routing
                    .as_deref()
                    .and_then(|route| OutputRoute::parse(route).ok())
                    .into_iter()
                    .chain(
                        track
                            .send_routing
                            .iter()
                            .flatten()
                            .filter_map(|route| OutputRoute::parse(route).ok()),
                    );
                for destination in destinations {
                    if let Some(group) = destination.group() {
                        pending.push(group);
                    }
                }
            }
            false
        };

        for channel in self
            .tracks
            .iter()
            .chain([&self.input_channel.strip, &self.output_channel.strip])
        {
            let is_strip = channel.id >= INPUT_BUS_ID;
            if let Some(route) = channel.input_routing.as_deref() {
                let input = InputRoute::parse(route)?;
                if !is_strip {
                    input.validate(crate::routing::MAX_INPUT_CHANNELS)?;
                }
            }
            if let Some(route) = channel.output_routing.as_deref() {
                let output = OutputRoute::parse(route)?;
                if let Some(group) = output.group() {
                    if group == channel.id {
                        return Err(RoutingError::SelfGroup { group });
                    }
                    if !group_ids.contains(&group) {
                        return Err(RoutingError::MissingGroup { group });
                    }
                    if !is_strip && feeds_cycle(channel.id, group) {
                        return Err(RoutingError::RoutingCycle { group });
                    }
                }
            }
            for route in channel.send_routing.iter().flatten() {
                let Some(group) = OutputRoute::parse(route)?.group() else {
                    continue;
                };
                if group == channel.id {
                    return Err(RoutingError::SelfGroup { group });
                }
                if !group_ids.contains(&group) {
                    return Err(RoutingError::MissingGroup { group });
                }
                if !is_strip && feeds_cycle(channel.id, group) {
                    return Err(RoutingError::RoutingCycle { group });
                }
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.media_pool.iter().any(|p| p.as_os_str().is_empty()) {
            return Err("media pool contains an empty source path");
        }
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
        if !self.timecode_offset_seconds.is_finite()
            || self.timecode_offset_seconds.abs() > MAX_TIMECODE_OFFSET_SECONDS
        {
            return Err("timecode offset must be finite and within 99:59:59");
        }
        if self.notepad.chars().count() > MAX_NOTEPAD_CHARS {
            return Err("project notepad is too long");
        }
        {
            let mut slots = BTreeSet::new();
            for marker in &self.markers {
                if let Some(slot) = marker.slot
                    && (!MARKER_SLOTS.contains(&slot) || !slots.insert(slot))
                {
                    return Err("marker slots must be unique and between 1 and 9");
                }
            }
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
                || !bus.strip.automation_lanes.is_empty()
            {
                return Err(
                    "device buses must be stereo channel strips without arrange media or recording controls",
                );
            }
            // Only the strip is checked, so a large project is not cloned twice
            // just to validate two device channels.
            let mut strip = bus.strip.clone();
            strip.id = 1;
            // The strip has no media, so the project's id counters do not
            // describe it; only its shape is checked.
            self.validate_track_data(std::slice::from_ref(&strip), false)?;
        }
        self.validate_track_data(&self.tracks, true)?;
        // The selection is a property of the whole project, so it is checked
        // here rather than while validating one track list (or a bus strip).
        if self.selected_midi_part_id.is_some_and(|id| {
            !self
                .tracks
                .iter()
                .flat_map(|track| track.midi_parts.iter())
                .any(|part| part.id == id)
        }) {
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

    fn validate_track_data(
        &self,
        tracks: &[Track],
        check_counters: bool,
    ) -> Result<(), &'static str> {
        let mut track_ids = BTreeSet::new();
        let mut clip_ids = BTreeSet::new();
        let mut midi_part_ids = BTreeSet::new();
        let mut midi_note_ids = BTreeSet::new();
        let mut midi_controller_ids = BTreeSet::new();
        let track_kinds = tracks
            .iter()
            .map(|track| (track.id, (track.kind, track.parent_group_id)))
            .collect::<std::collections::BTreeMap<_, _>>();
        // One pass builds the parents that contain a group, so the nesting check
        // below is a set lookup instead of a scan of every track per group.
        let groups_with_group_children = tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Group)
            .filter_map(|track| track.parent_group_id)
            .collect::<BTreeSet<u64>>();
        let mut automation_points = 0usize;
        for track in tracks {
            if track.id == 0 || track.id >= INPUT_BUS_ID || !track_ids.insert(track.id) {
                return Err("track IDs must be nonzero and unique");
            }
            if track.name.trim().is_empty() {
                return Err("track name is empty");
            }
            if !matches!(track.audio_channels, 1 | 2) {
                return Err("track audio channel width must be mono or stereo");
            }
            if (track.kind != TrackKind::Group && !track.group_cut_beats.is_empty())
                || track
                    .group_cut_beats
                    .iter()
                    .any(|beat| !beat.is_finite() || *beat < 0.0)
                || track
                    .group_cut_beats
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
            {
                return Err("group edit boundaries must be ordered nonnegative beats");
            }
            if let Some(parent_id) = track.parent_group_id {
                let Some((TrackKind::Group, parent_parent)) = track_kinds.get(&parent_id) else {
                    return Err("a grouped track must reference an existing group track");
                };
                if track.id == parent_id
                    || (track.kind == TrackKind::Group && parent_parent.is_some())
                {
                    return Err("group nesting cannot exceed two levels");
                }
                if track.kind == TrackKind::Group && groups_with_group_children.contains(&track.id)
                {
                    return Err("a nested group cannot contain another group");
                }
            }
            if track.automation_lanes.len() > MAX_AUTOMATION_LANES_PER_TRACK {
                return Err("a track has too many automation lanes");
            }
            let mut automation_targets = BTreeSet::new();
            for lane in &track.automation_lanes {
                if !automation_targets.insert(lane.target) {
                    return Err("a track cannot contain duplicate automation targets");
                }
                if lane.points.len() > MAX_AUTOMATION_POINTS_PER_LANE {
                    return Err("an automation lane has too many points");
                }
                automation_points = automation_points
                    .checked_add(lane.points.len())
                    .ok_or("project has too many automation points")?;
                if automation_points > MAX_AUTOMATION_POINTS_PER_PROJECT {
                    return Err("project has too many automation points");
                }
                if !lane.is_valid() {
                    return Err(
                        "automation points must have ordered finite positions and valid gain",
                    );
                }
            }
            if track.send_routing.len() > MAX_SEND_SLOTS
                || track.send_parameters.len() > MAX_SEND_SLOTS
            {
                return Err("a channel cannot have more than 24 send slots");
            }
            if track.send_parameters.iter().any(|send| {
                !send.level_db.is_finite()
                    || !(-90.0..=12.0).contains(&send.level_db)
                    || !send.pan.is_finite()
                    || !(-1.0..=1.0).contains(&send.pan)
            }) {
                return Err("send level or pan is outside its supported range");
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
            if !track.gain.is_finite() || !(0.0..=MAX_CHANNEL_GAIN).contains(&track.gain) {
                return Err("channel gain must be between 0.0 and 2.0");
            }
            for band in track.equalizer.all_bands() {
                if !band.frequency_hz.is_finite()
                    || !(20.0..=20_000.0).contains(&band.frequency_hz)
                    || !band.gain_db.is_finite()
                    || !(-24.0..=24.0).contains(&band.gain_db)
                    || !band.q.is_finite()
                    || !(0.1..=10.0).contains(&band.q)
                {
                    return Err("equalizer parameters are outside the supported range");
                }
            }
            for dynamic in track.equalizer.all_dynamic_bands() {
                if !dynamic.threshold_db.is_finite()
                    || !(-90.0..=0.0).contains(&dynamic.threshold_db)
                    || !dynamic.range_db.is_finite()
                    || !(-18.0..=18.0).contains(&dynamic.range_db)
                    || !dynamic.attack_ms.is_finite()
                    || !(0.1..=2000.0).contains(&dynamic.attack_ms)
                    || !dynamic.release_ms.is_finite()
                    || !(1.0..=5000.0).contains(&dynamic.release_ms)
                {
                    return Err("dynamic EQ parameters are outside the supported range");
                }
            }
            if let Some(target) = &track.equalizer.tonal_target
                && (!matches!(
                    (target.analysis_version, target.band_powers.len()),
                    (1, 24) | (2, 48) | (3, 96)
                ) || target.sample_rate == 0
                    || target.source_stage > 2
                    || target.name.chars().count() > 128
                    || target.source_name.chars().count() > 256
                    || !target.analysis_seconds.is_finite()
                    || !(0.0..=86_400.0).contains(&target.analysis_seconds)
                    || !matches!(
                        (target.analysis_version, target.fft_size),
                        (1 | 2, 4096) | (3, 8192)
                    )
                    || !matches!(target.band_powers.len(), 24 | 48 | 96)
                    || target
                        .band_powers
                        .iter()
                        .any(|power| !power.is_finite() || *power < 0.0)
                    || {
                        let total = target
                            .band_powers
                            .iter()
                            .map(|power| f64::from(*power))
                            .sum::<f64>();
                        !total.is_finite() || (total - 1.0).abs() > 1.0e-3
                    })
            {
                return Err("tonal reference target is invalid");
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
                    InsertEffectKind::Vst3 {
                        ref class_id,
                        ref module_path,
                        ..
                    } if class_id.len() != 32
                        || !class_id.bytes().all(|byte| byte.is_ascii_hexdigit())
                        || module_path.as_os_str().is_empty() =>
                    {
                        return Err("VST3 insert identity is incomplete");
                    }
                    InsertEffectKind::LegacyUnavailableCompressor {
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
                        return Err("legacy compressor parameters are outside the supported range");
                    }
                    _ => {}
                }
            }
            if !matches!(track.kind, TrackKind::Audio) && !track.clips.is_empty() {
                return Err("audio clips must be stored on audio tracks");
            }
            if !matches!(track.kind, TrackKind::Midi | TrackKind::Instrument)
                && !track.midi_parts.is_empty()
            {
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
                if clip.source_fades.len() > 32
                    || clip.source_fades.iter().any(|f| {
                        !f.offset_seconds.is_finite()
                            || f.offset_seconds < 0.0
                            || !f.duration_seconds.is_finite()
                            || f.duration_seconds <= 0.0
                            || !f.fade_in_seconds.is_finite()
                            || f.fade_in_seconds < 0.0
                            || !f.fade_out_seconds.is_finite()
                            || f.fade_out_seconds < 0.0
                            || f.fade_in_seconds + f.fade_out_seconds > f.duration_seconds + 1e-8
                    })
                {
                    return Err("source fade lineage is invalid");
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
                if let Some(edit) = &clip.pitch_time {
                    if !edit.validate(edit.analysis.source_duration_seconds) {
                        return Err("pitch/time note data is invalid");
                    }
                }
            }
        }
        let mut group_routes = std::collections::BTreeMap::new();
        for track in tracks {
            let Some(group_id) = track
                .output_routing
                .as_deref()
                .and_then(|route| route.strip_prefix("Group:"))
            else {
                continue;
            };
            let group_id = group_id
                .parse::<u64>()
                .map_err(|_| "group output route has an invalid ID")?;
            if track.id == group_id
                || !tracks
                    .iter()
                    .any(|candidate| candidate.id == group_id && candidate.kind == TrackKind::Group)
            {
                return Err("group output route must target another group channel");
            }
            group_routes.insert(track.id, group_id);
        }
        for track in tracks {
            let mut visited = BTreeSet::new();
            let mut current = track.id;
            while let Some(next) = group_routes.get(&current).copied() {
                if !visited.insert(current) {
                    return Err("group output routing cannot contain cycles");
                }
                current = next;
            }
        }
        // Validate the complete route graph before sends are connected to DSP.
        // Disabled sends also participate so enabling one cannot create feedback.
        let mut route_edges: std::collections::BTreeMap<u64, Vec<u64>> =
            std::collections::BTreeMap::new();
        for (&source, &destination) in &group_routes {
            route_edges.entry(source).or_default().push(destination);
        }
        for track in tracks {
            for route in track.send_routing.iter().flatten() {
                let Some(id) = route.strip_prefix("Group:") else {
                    continue;
                };
                let destination = id
                    .parse::<u64>()
                    .map_err(|_| "group send route has an invalid ID")?;
                if track.id == destination
                    || !self.tracks.iter().any(|candidate| {
                        candidate.id == destination && candidate.kind == TrackKind::Group
                    })
                {
                    return Err("group send must target another existing group channel");
                }
                route_edges.entry(track.id).or_default().push(destination);
            }
        }
        fn visit_route(
            id: u64,
            edges: &std::collections::BTreeMap<u64, Vec<u64>>,
            visiting: &mut BTreeSet<u64>,
            done: &mut BTreeSet<u64>,
        ) -> bool {
            if done.contains(&id) {
                return true;
            }
            if !visiting.insert(id) {
                return false;
            }
            if let Some(destinations) = edges.get(&id) {
                for &destination in destinations {
                    if !visit_route(destination, edges, visiting, done) {
                        return false;
                    }
                }
            }
            visiting.remove(&id);
            done.insert(id);
            true
        }
        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        for &id in route_edges.keys() {
            if !visit_route(id, &route_edges, &mut visiting, &mut done) {
                return Err("send and output routing cannot contain cycles");
            }
        }
        if check_counters
            && track_ids
                .last()
                .is_some_and(|maximum_id| self.next_track_id <= *maximum_id)
        {
            return Err("next track ID must be greater than every existing track ID");
        }
        if check_counters
            && clip_ids
                .last()
                .is_some_and(|maximum_id| self.next_clip_id <= *maximum_id)
        {
            return Err("next clip ID must be greater than every existing clip ID");
        }
        if check_counters
            && midi_part_ids
                .last()
                .is_some_and(|maximum_id| self.next_midi_part_id <= *maximum_id)
        {
            return Err("next MIDI part ID must be greater than every existing part ID");
        }
        if check_counters
            && midi_note_ids
                .last()
                .is_some_and(|maximum_id| self.next_midi_note_id <= *maximum_id)
        {
            return Err("next MIDI note ID must be greater than every existing note ID");
        }
        if check_counters
            && midi_controller_ids
                .last()
                .is_some_and(|maximum_id| self.next_midi_controller_id <= *maximum_id)
        {
            return Err(
                "next MIDI controller event ID must be greater than every existing event ID",
            );
        }

        Ok(())
    }
}

fn unique_duplicate_track_name(source: &str, used_names: &mut BTreeSet<String>) -> String {
    let base = format!("{source} Copy");
    if used_names.insert(base.clone()) {
        return base;
    }
    for suffix in 2_u32.. {
        let name = format!("{source} Copy {suffix}");
        if used_names.insert(name.clone()) {
            return name;
        }
    }
    unreachable!("u32 name suffix space exhausted")
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
    fn volume_automation_interpolates_and_holds_endpoints() {
        let lane = AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![
                AutomationPoint {
                    beat: 2.0,
                    value: 0.0,
                },
                AutomationPoint {
                    beat: 6.0,
                    value: 2.0,
                },
                AutomationPoint {
                    beat: 8.0,
                    value: 1.0,
                },
            ],
        };
        for (beat, expected) in [
            (-1.0, 0.0),
            (2.0, 0.0),
            (4.0, 1.0),
            (6.0, 2.0),
            (7.0, 1.5),
            (10.0, 1.0),
        ] {
            assert_eq!(lane.value_at(beat), Some(expected));
        }
        assert_eq!(lane.value_at(f64::NAN), None);
        assert_eq!(
            AutomationLane {
                target: AutomationTarget::TrackVolume,
                points: Vec::new()
            }
            .value_at(1.0),
            None
        );
    }

    #[test]
    fn cached_automation_matches_random_access_after_seeks_and_loops() {
        let lane = AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![
                AutomationPoint {
                    beat: 1.0,
                    value: 0.0,
                },
                AutomationPoint {
                    beat: 2.0,
                    value: 2.0,
                },
                AutomationPoint {
                    beat: 4.0,
                    value: 0.5,
                },
            ],
        };
        let mut cursor = usize::MAX;
        for beat in [0.0, 1.0, 1.5, 2.0, 3.0, 5.0, 1.5, 0.0, 4.0, 2.0, 100.0] {
            assert_eq!(
                lane.value_at_cached(beat, &mut cursor),
                lane.value_at(beat),
                "beat {beat}"
            );
        }
    }

    #[test]
    fn automation_round_trips_and_legacy_tracks_have_no_lanes() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].automation_read = true;
        project.tracks[0].automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![AutomationPoint {
                beat: 1.25,
                value: 0.7,
            }],
        });
        assert!(project.validate().is_ok());
        let mut encoded = serde_json::to_value(&project).unwrap();
        let decoded: Project = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(
            decoded.tracks[0].automation_lanes,
            project.tracks[0].automation_lanes
        );
        encoded["tracks"][0]
            .as_object_mut()
            .unwrap()
            .remove("automation_lanes");
        let legacy: Project = serde_json::from_value(encoded).unwrap();
        assert!(legacy.tracks[0].automation_lanes.is_empty());
        assert!(legacy.validate().is_ok());
        assert!(serde_json::from_str::<AutomationTarget>("\"unknown_parameter\"").is_err());
    }

    #[test]
    fn automation_validation_rejects_invalid_points_duplicates_and_unsupported_bus_lanes() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let lane = AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![AutomationPoint {
                beat: 1.0,
                value: 1.0,
            }],
        };
        project.tracks[0].automation_lanes = vec![lane.clone(), lane.clone()];
        assert_eq!(
            project.validate(),
            Err("a track cannot contain duplicate automation targets")
        );
        project.tracks[0].automation_lanes = vec![lane.clone()];
        for point in [
            AutomationPoint {
                beat: f64::NAN,
                value: 1.0,
            },
            AutomationPoint {
                beat: -1.0,
                value: 1.0,
            },
            AutomationPoint {
                beat: 1.0,
                value: f32::INFINITY,
            },
            AutomationPoint {
                beat: 1.0,
                value: 2.1,
            },
        ] {
            project.tracks[0].automation_lanes[0].points = vec![point];
            assert!(project.validate().is_err());
        }
        project.tracks[0].automation_lanes[0].points = vec![lane.points[0], lane.points[0]];
        assert!(project.validate().is_err());
        project.tracks[0].automation_lanes[0].points =
            vec![lane.points[0]; MAX_AUTOMATION_POINTS_PER_LANE + 1];
        assert_eq!(
            project.validate(),
            Err("an automation lane has too many points")
        );
        project.tracks[0].automation_lanes.clear();
        project.input_channel.strip.automation_lanes.push(lane);
        assert!(project.validate().is_err());
    }

    #[test]
    fn project_automation_budget_is_enforced_across_tracks() {
        let mut project = Project::default();
        let points: Vec<_> = (0..MAX_AUTOMATION_POINTS_PER_LANE)
            .map(|i| AutomationPoint {
                beat: i as f64,
                value: 1.0,
            })
            .collect();
        for _ in 0..10 {
            project.add_track(TrackKind::Audio);
            project
                .tracks
                .last_mut()
                .unwrap()
                .automation_lanes
                .push(AutomationLane {
                    target: AutomationTarget::TrackVolume,
                    points: points.clone(),
                });
        }
        assert!(project.validate().is_ok());
        project.add_track(TrackKind::Audio);
        project
            .tracks
            .last_mut()
            .unwrap()
            .automation_lanes
            .push(AutomationLane {
                target: AutomationTarget::TrackVolume,
                points: vec![AutomationPoint {
                    beat: 0.0,
                    value: 1.0,
                }],
            });
        assert_eq!(
            project.validate(),
            Err("project has too many automation points")
        );
    }

    #[test]
    fn send_parameters_round_trip_and_legacy_projects_default_to_inactive() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].send_routing = vec![Some("Stereo Out".to_owned())];
        project.tracks[0].send_parameters = vec![SendParameters {
            level_db: -6.0,
            pan: 0.5,
            tap: SendTap::PreFader,
            enabled: true,
        }];
        assert!(project.validate().is_ok());
        let encoded = serde_json::to_value(&project).unwrap();
        let decoded: Project = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(
            decoded.tracks[0].send_parameters,
            project.tracks[0].send_parameters
        );
        let mut legacy = encoded;
        legacy["tracks"][0]
            .as_object_mut()
            .unwrap()
            .remove("send_parameters");
        let decoded: Project = serde_json::from_value(legacy).unwrap();
        assert!(decoded.tracks[0].send_parameters.is_empty());
        assert!(!SendParameters::default().enabled);
        assert!(decoded.validate().is_ok());
    }

    #[test]
    fn send_parameter_validation_rejects_invalid_values_and_excess_slots() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].send_parameters = vec![SendParameters::default()];
        for value in [f32::NAN, f32::INFINITY, -90.1, 12.1] {
            project.tracks[0].send_parameters[0].level_db = value;
            assert!(project.validate().is_err());
        }
        project.tracks[0].send_parameters[0].level_db = 0.0;
        for value in [f32::NAN, -1.01, 1.01] {
            project.tracks[0].send_parameters[0].pan = value;
            assert!(project.validate().is_err());
        }
        project.tracks[0].send_parameters = vec![SendParameters::default(); MAX_SEND_SLOTS];
        assert!(project.validate().is_ok());
        project.tracks[0]
            .send_parameters
            .push(SendParameters::default());
        assert!(project.validate().is_err());
        project.tracks[0].send_parameters.clear();
        project.tracks[0].send_routing = vec![None; MAX_SEND_SLOTS + 1];
        assert!(project.validate().is_err());
    }

    #[test]
    fn send_routes_reject_feedback_across_sends_and_group_outputs() {
        let mut project = Project::default();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Group);
        let first = project.tracks[0].id;
        let second = project.tracks[1].id;
        project.tracks[0].send_routing = vec![Some(format!("Group:{second}"))];
        assert!(project.validate().is_ok());
        project.tracks[1].output_routing = Some(format!("Group:{first}"));
        assert_eq!(
            project.validate(),
            Err("send and output routing cannot contain cycles")
        );
        project.tracks[1].output_routing = Some("Stereo Out".to_owned());
        project.tracks[1].send_routing = vec![Some(format!("Group:{first}"))];
        assert_eq!(
            project.validate(),
            Err("send and output routing cannot contain cycles")
        );
        project.tracks[1].send_routing = vec![Some(format!("Group:{second}"))];
        assert_eq!(
            project.validate(),
            Err("group send must target another existing group channel")
        );
        project.tracks[1].send_routing = vec![Some("Group:9999".to_owned())];
        assert_eq!(
            project.validate(),
            Err("group send must target another existing group channel")
        );
    }

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
    fn any_builtin_eq_band_accepts_any_supported_filter_shape() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].equalizer.bands[1].filter = FilterType::LowShelf;
        project.tracks[0].equalizer.additional_bands[0].filter = FilterType::Notch;
        assert!(project.validate().is_ok());
    }

    #[test]
    fn thirty_second_eq_band_has_parallel_dynamic_state_and_boundaries() {
        let mut equalizer = ChannelEqualizer::default();
        assert_eq!(equalizer.all_bands().count(), MAX_EQ_BANDS);
        assert_eq!(equalizer.all_dynamic_bands().count(), MAX_EQ_BANDS);
        let band = equalizer.band_mut(MAX_EQ_BANDS - 1).unwrap();
        band.enabled = true;
        band.frequency_hz = 18_000.0;
        let (band, dynamic) = equalizer.band_and_dynamic_mut(MAX_EQ_BANDS - 1).unwrap();
        dynamic.enabled = true;
        dynamic.range_db = 9.0;
        assert!(band.enabled);
        assert_eq!(band.frequency_hz, 18_000.0);
        assert_eq!(
            equalizer.dynamic_band(MAX_EQ_BANDS - 1).unwrap().range_db,
            9.0
        );
        assert!(equalizer.band(MAX_EQ_BANDS).is_none());
        assert!(equalizer.band_and_dynamic_mut(MAX_EQ_BANDS).is_none());
    }

    #[test]
    fn legacy_four_band_equalizer_loads_with_neutral_additional_bands() {
        let legacy = serde_json::json!({
            "bypassed": false,
            "bands": [
                {"enabled": true, "filter":"low_shelf", "frequency_hz":91.0, "gain_db":2.5, "q":0.8},
                {"enabled": false, "filter":"bell", "frequency_hz":800.0, "gain_db":0.0, "q":0.7},
                {"enabled": true, "filter":"bell", "frequency_hz":2400.0, "gain_db":-1.5, "q":1.2},
                {"enabled": false, "filter":"high_shelf", "frequency_hz":12000.0, "gain_db":0.0, "q":0.8}
            ]
        });
        let loaded: ChannelEqualizer = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.bands[0].frequency_hz, 91.0);
        assert_eq!(loaded.bands[0].gain_db, 2.5);
        assert_eq!(loaded.bands[2].gain_db, -1.5);
        assert!(
            loaded.additional_bands.iter().all(|band| {
                !band.enabled && band.gain_db == 0.0 && band.frequency_hz == 1_000.0
            })
        );
        assert!(loaded.extended_bands.iter().all(|band| !band.enabled));
        assert!(
            loaded
                .extended_dynamic_bands
                .iter()
                .all(|band| !band.enabled)
        );
    }

    #[test]
    fn tonal_target_provenance_roundtrips_and_older_targets_get_defaults() {
        let target = TonalTarget {
            name: "Imported Reference".into(),
            analysis_version: 1,
            sample_rate: 48_000,
            source_stage: 2,
            source_name: "mix.wav".into(),
            analysis_seconds: 92.5,
            fft_size: 4096,
            band_powers: vec![1.0 / 24.0; 24],
        };
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].equalizer.tonal_target = Some(target.clone());
        let encoded = serde_json::to_vec(&project).unwrap();
        let decoded: Project = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            decoded.tracks[0].equalizer.tonal_target,
            Some(target.clone())
        );
        assert!(decoded.validate().is_ok());

        let mut legacy = serde_json::to_value(ChannelEqualizer::default()).unwrap();
        legacy["tonal_target"] = serde_json::json!({
            "name": "Older Target",
            "analysis_version": 1,
            "sample_rate": 44100,
            "source_stage": 1,
            "band_powers": vec![1.0 / 24.0; 24]
        });
        let migrated: ChannelEqualizer = serde_json::from_value(legacy).unwrap();
        let migrated_target = migrated.tonal_target.unwrap();
        assert_eq!(migrated_target.source_name, "");
        assert_eq!(migrated_target.analysis_seconds, 0.0);
        assert_eq!(migrated_target.fft_size, 4096);
    }

    #[test]
    fn tonal_target_v3_accepts_96_bands_with_the_fine_fft_and_rejects_mismatches() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].equalizer.tonal_target = Some(TonalTarget {
            name: "Fine Reference".into(),
            analysis_version: 3,
            sample_rate: 48_000,
            source_stage: 2,
            source_name: "reference.wav".into(),
            analysis_seconds: 30.0,
            fft_size: 8192,
            band_powers: vec![1.0 / 96.0; 96],
        });
        assert!(project.validate().is_ok());

        project.tracks[0]
            .equalizer
            .tonal_target
            .as_mut()
            .unwrap()
            .fft_size = 4096;
        assert_eq!(project.validate(), Err("tonal reference target is invalid"));
        let target = project.tracks[0].equalizer.tonal_target.as_mut().unwrap();
        target.fft_size = 8192;
        target.band_powers.pop();
        assert_eq!(project.validate(), Err("tonal reference target is invalid"));
    }

    #[test]
    fn imported_tonal_target_metadata_is_project_validated() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].equalizer.tonal_target = Some(TonalTarget {
            name: "File Reference".into(),
            analysis_version: 1,
            sample_rate: 48_000,
            source_stage: 2,
            source_name: "reference.wav".into(),
            analysis_seconds: 30.0,
            fft_size: 4096,
            band_powers: vec![1.0 / 24.0; 24],
        });
        assert!(project.validate().is_ok());
        project.tracks[0]
            .equalizer
            .tonal_target
            .as_mut()
            .unwrap()
            .band_powers[0] = f32::MAX;
        assert_eq!(project.validate(), Err("tonal reference target is invalid"));
        project.tracks[0]
            .equalizer
            .tonal_target
            .as_mut()
            .unwrap()
            .band_powers = vec![1.0; 24];
        assert_eq!(project.validate(), Err("tonal reference target is invalid"));
        project.tracks[0]
            .equalizer
            .tonal_target
            .as_mut()
            .unwrap()
            .band_powers = vec![1.0 / 24.0; 24];
        assert!(project.validate().is_ok());
        project.tracks[0]
            .equalizer
            .tonal_target
            .as_mut()
            .unwrap()
            .fft_size = 2048;
        assert_eq!(project.validate(), Err("tonal reference target is invalid"));
    }

    #[test]
    fn insert_parameters_are_validated_before_audio_plan_construction() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].inserts.push(InsertEffect {
            enabled: true,
            bypassed: false,
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

        project.tracks[0].inserts[0].kind = InsertEffectKind::LegacyUnavailableCompressor {
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
            Err("legacy compressor parameters are outside the supported range")
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
        project.input_channel.strip.color_index = Some(5);
        project.output_channel.strip.equalizer.bands[0].enabled = true;
        project.output_channel.strip.color_index = Some(8);
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
    fn vst3_insert_identity_and_state_round_trip() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].inserts.push(InsertEffect {
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::Vst3 {
                class_id: "0de7aef2de004e4f41414d6137325247".to_owned(),
                module_path: PathBuf::from(r"C:\Program Files\Common Files\VST3\Gateway.vst3"),
                name: "Gateway".to_owned(),
                vendor: "Retro".to_owned(),
                component_state: vec![1, 2, 3],
                controller_state: vec![4, 5],
            },
        });
        assert!(project.validate().is_ok());
        let json = serde_json::to_string(&project).unwrap();
        let loaded: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, project);
        project.tracks[0].inserts[0].kind = InsertEffectKind::Vst3 {
            class_id: "not-a-class".to_owned(),
            module_path: PathBuf::from("Gateway.vst3"),
            name: String::new(),
            vendor: String::new(),
            component_state: Vec::new(),
            controller_state: Vec::new(),
        };
        assert!(project.validate().is_err());
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

    #[test]
    fn batch_event_move_is_atomic_and_preserves_relative_positions_and_media() {
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        p.add_track(TrackKind::Audio);
        p.add_track_with_audio_channels(TrackKind::Audio, 1);
        let first = p
            .add_audio_clip(0, PathBuf::from("one.wav"), 1.0, 2.0)
            .unwrap();
        let second = p
            .add_audio_clip(2, PathBuf::from("two.wav"), 5.0, 3.0)
            .unwrap();
        p.tracks[0].clips[0].source_offset_seconds = 0.25;
        p.tracks[0].clips[0].fade_in_seconds = 0.1;
        let before = p.clone();
        let mut moved_first = p.tracks[0].clips[0].clone();
        moved_first.start_beats += 2.0;
        let mut moved_second = p.tracks[2].clips[0].clone();
        moved_second.start_beats += 2.0;
        assert!(p.move_audio_clips(&[
            (p.tracks[0].id, p.tracks[0].id, moved_first.clone()),
            (p.tracks[2].id, p.tracks[2].id, moved_second.clone()),
        ]));
        assert_eq!(p.tracks[0].clips[0], moved_first);
        assert_eq!(p.tracks[2].clips[0], moved_second);
        assert_eq!(
            p.tracks[0].clips[0].start_beats - p.tracks[2].clips[0].start_beats,
            -4.0
        );
        assert_eq!(p.media_pool, before.media_pool);

        let committed = p.clone();
        let mut invalid = p.tracks[2].clips[0].clone();
        invalid.start_beats = f64::NAN;
        assert!(!p.move_audio_clips(&[
            (p.tracks[0].id, p.tracks[0].id, p.tracks[0].clips[0].clone()),
            (p.tracks[2].id, p.tracks[2].id, invalid),
        ]));
        assert_eq!(p, committed);

        let mut incompatible = moved_first;
        incompatible.start_beats += 1.0;
        assert!(!p.move_audio_clips(&[
            (p.tracks[0].id, p.tracks[2].id, incompatible),
            (p.tracks[2].id, p.tracks[2].id, p.tracks[2].clips[0].clone()),
        ]));
        assert_eq!(p, committed);
        assert_ne!(first, second);
    }
}

#[cfg(test)]
mod channel_headroom_tests {
    use super::*;
    #[test]
    fn track_and_bus_positive_gain_persists_and_excess_is_rejected() {
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        p.tracks[0].gain = 2.0;
        p.input_channel.strip.gain = 2.0;
        p.output_channel.strip.gain = 2.0;
        assert!(p.validate().is_ok());
        let restored: Project = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
        assert_eq!(restored, p);
        p.tracks[0].gain = 2.01;
        assert!(p.validate().is_err());
        p.tracks[0].gain = 0.001;
        assert!(p.validate().is_ok());
    }
}

#[cfg(test)]
mod sample_edit_tests {
    use super::*;
    #[test]
    fn cuts_keep_source_offsets_and_original_fades_even_inside_fades_and_repeated_cuts() {
        let mut p = Project::default();
        p.tempo_bpm = 120.0;
        p.add_track(TrackKind::Audio);
        let id = p.add_audio_clip(0, "source.wav".into(), 2.0, 4.0).unwrap();
        let track = p.tracks[0].id;
        p.tracks[0].clips[0].source_offset_seconds = 0.5;
        p.tracks[0].clips[0].fade_in_seconds = 2.0;
        p.tracks[0].clips[0].fade_out_seconds = 1.0;
        let saved = p.clone();
        let right = p.split_audio_clip(track, id, 3.0).unwrap();
        assert_eq!(p.tracks[0].clips[0].duration_seconds, 0.5);
        assert_eq!(p.tracks[0].clips[1].source_offset_seconds, 1.0);
        assert_ne!(right, id);
        assert!(p.split_audio_clip(track, right, 5.0).is_some());
        for i in 0..4000 {
            let t = i as f64 / 1000.0;
            let beat = 2.0 + t * 2.0;
            let c = p.tracks[0]
                .clips
                .iter()
                .find(|c| beat >= c.start_beats && beat < c.start_beats + c.duration_seconds * 2.0)
                .unwrap();
            let source_time = c.source_offset_seconds + (beat - c.start_beats) / 2.0;
            let actual = c
                .source_fades
                .iter()
                .map(|f| f.gain_at(source_time))
                .product::<f32>();
            let expected = SourceFade {
                offset_seconds: 0.5,
                duration_seconds: 4.0,
                fade_in_seconds: 2.0,
                fade_out_seconds: 1.0,
            }
            .gain_at(t + 0.5);
            assert!((actual - expected).abs() < 0.00001);
        }
        let before = p.clone();
        assert_eq!(p.split_audio_clip(track, id, 2.0), None);
        assert_eq!(p, before);
        let encoded = serde_json::to_string(&p).unwrap();
        let loaded: Project = serde_json::from_str(&encoded).unwrap();
        assert_eq!(loaded, p);
        assert!(loaded.validate().is_ok());
        assert_eq!(saved.tracks[0].clips.len(), 1);
    }
    #[test]
    fn crossfade_uses_existing_overlap_and_rejects_disjoint_or_cross_track_events() {
        let mut p = Project::default();
        p.tempo_bpm = 120.0;
        p.add_track(TrackKind::Audio);
        p.add_track(TrackKind::Audio);
        let a = p.add_audio_clip(0, "a.wav".into(), 0.0, 2.0).unwrap();
        let b = p.add_audio_clip(0, "b.wav".into(), 3.0, 2.0).unwrap();
        let other = p.add_audio_clip(1, "b.wav".into(), 3.0, 2.0).unwrap();
        assert!(!p.crossfade_audio_clips(a, other));
        assert!(p.crossfade_audio_clips(b, a));
        assert_eq!(p.tracks[0].clips[0].fade_out_seconds, 0.5);
        assert_eq!(p.tracks[0].clips[1].fade_in_seconds, 0.5);
        assert!(!p.crossfade_audio_clips(a, b));
        p.tracks[0].clips[1].start_beats = 6.0;
        let before = p.clone();
        assert!(!p.crossfade_audio_clips(a, b));
        assert_eq!(p, before);
    }
    #[test]
    fn duplicating_one_event_places_the_copy_immediately_after_it() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, PathBuf::from("guitar.wav"), 2.0, 4.0)
            .unwrap();
        project.tracks[0].clips[0].gain = 0.5;
        project.tracks[0].clips[0].fade_in_seconds = 0.25;
        project.tracks[0].clips[0].muted = true;

        let created = project.duplicate_audio_clips(&[id]);
        assert_eq!(created.len(), 1);
        let copy = project.tracks[0]
            .clips
            .iter()
            .find(|clip| clip.id == created[0])
            .expect("copy");
        // 4 seconds at 120 BPM is 8 beats, so the copy starts at beat 10.
        assert!((copy.start_beats - 10.0).abs() < 1e-9);
        assert_eq!(copy.source_path, PathBuf::from("guitar.wav"));
        assert_eq!(copy.gain, 0.5);
        assert_eq!(copy.fade_in_seconds, 0.25);
        assert!(copy.muted, "a duplicate keeps the original's mute state");
        assert_eq!(project.tracks[0].clips.len(), 2);
    }

    #[test]
    fn duplicating_a_selection_shifts_every_event_by_the_selection_span() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let second = project
            .add_audio_clip(1, PathBuf::from("b.wav"), 6.0, 2.0)
            .unwrap();

        let created = project.duplicate_audio_clips(&[first, second]);
        assert_eq!(created.len(), 2);
        // The selection spans beats 0 through 10, so both copies shift by 10.
        let starts = project
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .filter(|clip| created.contains(&clip.id))
            .map(|clip| clip.start_beats)
            .collect::<Vec<_>>();
        assert_eq!(starts.len(), 2);
        assert!(starts.iter().any(|start| (*start - 10.0).abs() < 1e-9));
        assert!(starts.iter().any(|start| (*start - 16.0).abs() < 1e-9));
    }

    #[test]
    fn duplicating_nothing_or_an_unknown_event_changes_nothing() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let before = project.clone();
        assert!(project.duplicate_audio_clips(&[]).is_empty());
        assert!(project.duplicate_audio_clips(&[9_999]).is_empty());
        assert_eq!(project, before);
    }

    #[test]
    fn muting_events_is_reversible_and_reports_changes() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let second = project
            .add_audio_clip(0, PathBuf::from("b.wav"), 4.0, 2.0)
            .unwrap();

        assert_eq!(project.set_audio_clips_muted(&[first], true), 1);
        assert_eq!(project.set_audio_clips_muted(&[first], true), 0);
        assert!(project.tracks[0].clips[0].muted);
        assert!(!project.tracks[0].clips[1].muted);
        assert!(project.validate().is_ok());

        // Only the muted event changes, so unmuting reports one edit.
        assert_eq!(project.set_audio_clips_muted(&[first, second], false), 1);
        assert!(project.tracks[0].clips.iter().all(|clip| !clip.muted));
    }

    #[test]
    fn automation_follows_a_moved_event_only_when_asked() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        // 2 s at 120 BPM is 4 beats, so the event spans beats 0 through 4.
        let track_id = project.tracks[0].id;
        for (beat, value) in [(1.0, 0.5), (3.0, 0.9), (8.0, 0.2)] {
            crate::automation::set_volume_point(
                &mut project,
                track_id,
                crate::automation::AutomationPoint { beat, value },
            )
            .unwrap();
        }

        // Without following, the curve stays where it was written.
        let mut moved = project.tracks[0].clips[0].clone();
        moved.start_beats = 4.0;
        assert!(project.move_audio_clips(&[(project.tracks[0].id, project.tracks[0].id, moved)]));
        let beats = |project: &Project| {
            project.tracks[0].automation_lanes[0]
                .points
                .iter()
                .map(|point| point.beat)
                .collect::<Vec<_>>()
        };
        assert_eq!(beats(&project), vec![1.0, 3.0, 8.0]);

        // With following, the interior points travel with the event.
        let mut moved = project.tracks[0].clips[0].clone();
        moved.start_beats = 2.0;
        assert!(project.move_audio_clips_with_automation(
            &[(project.tracks[0].id, project.tracks[0].id, moved)],
            true
        ));
        // The event started at 4 and now starts at 2, so the points at 1.0 and
        // 3.0 were outside its span; only points inside 4..8 move. The point at
        // 8.0 is on the boundary and stays.
        assert_eq!(beats(&project), vec![1.0, 3.0, 8.0]);
    }

    #[test]
    fn following_automation_moves_the_points_under_the_event() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let track_id = project.tracks[0].id;
        for (beat, value) in [(0.0, 0.25), (2.0, 0.5), (4.0, 0.75), (6.0, 0.1)] {
            crate::automation::set_volume_point(
                &mut project,
                track_id,
                crate::automation::AutomationPoint { beat, value },
            )
            .unwrap();
        }
        let mut moved = project.tracks[0].clips[0].clone();
        moved.start_beats = 1.0;
        assert!(project.move_audio_clips_with_automation(
            &[(project.tracks[0].id, project.tracks[0].id, moved)],
            true
        ));
        let points = &project.tracks[0].automation_lanes[0].points;
        let beats = points.iter().map(|point| point.beat).collect::<Vec<_>>();
        // The event spanned 0..4, so 2.0 moves to 3.0. The boundary points at
        // 0.0 and 4.0 and the one outside at 6.0 stay put.
        assert_eq!(beats, vec![0.0, 3.0, 4.0, 6.0]);
        assert_eq!(points[1].value, 0.5, "values travel with their points");
        assert!(project.validate().is_ok());
    }

    #[test]
    fn following_automation_keeps_the_curve_valid_when_it_would_collide() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        // Moving the interior point onto the fixed point at 6.0 would create a
        // duplicate beat, which is not a valid curve.
        let track_id = project.tracks[0].id;
        for (beat, value) in [(0.0, 0.2), (5.0, 0.4), (6.0, 0.6)] {
            crate::automation::set_volume_point(
                &mut project,
                track_id,
                crate::automation::AutomationPoint { beat, value },
            )
            .unwrap();
        }
        let before = project.tracks[0].automation_lanes[0].clone();
        let mut moved = project.tracks[0].clips[0].clone();
        moved.start_beats = 1.0;
        assert!(project.move_audio_clips_with_automation(
            &[(project.tracks[0].id, project.tracks[0].id, moved)],
            true
        ));
        assert_eq!(
            project.tracks[0].automation_lanes[0], before,
            "an ambiguous curve is left untouched rather than made invalid"
        );
        assert_eq!(
            project.tracks[0].clips[0].start_beats, 1.0,
            "the event still moves"
        );
    }

    #[test]
    fn following_automation_uses_the_track_the_event_came_from() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let track_id = project.tracks[0].id;
        crate::automation::set_volume_point(
            &mut project,
            track_id,
            crate::automation::AutomationPoint {
                beat: 2.0,
                value: 0.5,
            },
        )
        .unwrap();
        let mut moved = project.tracks[0].clips[0].clone();
        moved.start_beats = 2.0;
        assert!(project.move_audio_clips_with_automation(
            &[(project.tracks[0].id, project.tracks[1].id, moved)],
            true
        ));
        assert_eq!(project.tracks[0].automation_lanes[0].points[0].beat, 4.0);
        assert_eq!(project.tracks[0].clips.len(), 0);
        assert_eq!(project.tracks[1].clips[0].id, id);
    }

    #[test]
    fn missing_media_and_relinking_follow_the_pool_and_the_events() {
        let root = std::env::temp_dir().join(format!("opentrack-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let present = root.join("here.wav");
        std::fs::write(&present, b"x").expect("write");
        let absent = root.join("gone.wav");

        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let kept = project
            .add_audio_clip(0, present.clone(), 0.0, 1.0)
            .unwrap();
        let broken = project.add_audio_clip(0, absent.clone(), 2.0, 1.0).unwrap();

        assert_eq!(project.missing_media(), vec![absent.clone()]);

        let replacement = root.join("found.wav");
        std::fs::write(&replacement, b"y").expect("write");
        let mapping = BTreeMap::from([(absent.clone(), replacement.clone())]);
        assert_eq!(project.relink_media(&mapping), 1);

        assert_eq!(project.tracks[0].clips[0].id, kept);
        assert_eq!(project.tracks[0].clips[0].source_path, present);
        assert_eq!(project.tracks[0].clips[1].id, broken);
        assert_eq!(project.tracks[0].clips[1].source_path, replacement);
        assert_eq!(project.missing_media(), Vec::<PathBuf>::new());
        assert!(!project.media_pool.contains(&absent), "the pool followed");
        assert!(project.validate().is_ok());

        // An empty mapping is a no-op.
        assert_eq!(project.relink_media(&BTreeMap::new()), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_notepad_round_trips_is_bounded_and_defaults_empty() {
        use crate::project::MAX_NOTEPAD_CHARS;

        let mut project = Project::default();
        assert!(project.notepad.is_empty());
        assert!(project.set_notepad("Mix notes: vocal bright at 3k"));
        assert!(
            !project.set_notepad("Mix notes: vocal bright at 3k"),
            "no change"
        );

        let encoded = serde_json::to_string(&project).unwrap();
        let restored: Project = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.notepad, "Mix notes: vocal bright at 3k");

        // An older project has no notepad field and reads as empty.
        let mut legacy: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        legacy.as_object_mut().expect("object").remove("notepad");
        let legacy: Project = serde_json::from_value(legacy).unwrap();
        assert!(legacy.notepad.is_empty());

        // The text is bounded rather than refused.
        assert!(project.set_notepad("x".repeat(MAX_NOTEPAD_CHARS * 2)));
        assert_eq!(project.notepad.chars().count(), MAX_NOTEPAD_CHARS);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn unused_pool_entries_are_detected_and_removed_without_touching_files() {
        let root =
            std::env::temp_dir().join(format!("opentrack-unused-media-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        let used = root.join("used.wav");
        let unused = root.join("unused.wav");
        let missing = root.join("missing.wav");
        std::fs::write(&used, b"x").expect("used");
        std::fs::write(&unused, b"y").expect("unused");

        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, used.clone(), 0.0, 2.0)
            .expect("clip");
        // The event's source is referenced; the other two are only in the pool.
        project.media_pool.insert(unused.clone());
        project.media_pool.insert(missing.clone());

        assert_eq!(project.referenced_media(), BTreeSet::from([used.clone()]));
        assert_eq!(
            project.unused_media(),
            vec![missing.clone(), unused.clone()]
        );
        assert_eq!(
            project.missing_unreferenced_media(),
            vec![missing.clone()],
            "an existing unused file is not missing"
        );

        assert_eq!(project.remove_unused_media(), vec![missing, unused.clone()]);
        assert!(
            project.media_pool.contains(&used),
            "the referenced file stays"
        );
        assert_eq!(project.media_pool.len(), 1);
        assert!(unused.exists(), "the file itself is untouched");
        assert_eq!(project.remove_unused_media(), Vec::<PathBuf>::new());
        assert!(project.validate().is_ok());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_missing_source_an_event_references_is_kept_for_relinking() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let gone = PathBuf::from("/nowhere/gone.wav");
        project
            .add_audio_clip(0, gone.clone(), 0.0, 2.0)
            .expect("clip");
        project.register_media();
        assert!(project.media_pool.contains(&gone));
        assert!(project.missing_unreferenced_media().is_empty());
        assert!(project.remove_unused_media().is_empty());
        assert!(
            project.media_pool.contains(&gone),
            "the event still needs it"
        );
    }

    #[test]
    fn fades_fit_the_locator_range_without_touching_outside_events() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // 8 s at 120 BPM is 16 beats, crossing the 4..8 range on both sides.
        let crossing = project
            .add_audio_clip(0, PathBuf::from("cross.wav"), 0.0, 8.0)
            .unwrap();
        // Fully inside the range: its fades should clear.
        let inside = project
            .add_audio_clip(0, PathBuf::from("inside.wav"), 5.0, 1.0)
            .unwrap();
        // Fully outside: untouched.
        let outside = project
            .add_audio_clip(0, PathBuf::from("outside.wav"), 20.0, 2.0)
            .unwrap();
        project.tracks[0].clips[1].fade_in_seconds = 0.4;
        project.tracks[0].clips[2].fade_in_seconds = 0.3;

        assert_eq!(
            project.fit_audio_clip_fades_to_range(&[crossing, inside, outside], 4.0, 8.0),
            2
        );
        let clips = &project.tracks[0].clips;
        // 4 beats is two seconds before the left locator.
        assert!((clips[0].fade_in_seconds - 2.0).abs() < 1e-9);
        // 8 beats after the right locator is another four seconds.
        assert!((clips[0].fade_out_seconds - 4.0).abs() < 1e-9);
        assert_eq!(clips[1].fade_in_seconds, 0.0, "inside the range: no fades");
        assert_eq!(clips[1].fade_out_seconds, 0.0);
        assert_eq!(clips[2].fade_in_seconds, 0.3, "outside is untouched");
        assert!(clips[0].fade_in_seconds + clips[0].fade_out_seconds <= 8.0);
        assert!(project.validate().is_ok());

        // Locked events are skipped and nothing changes twice.
        project.set_audio_clips_locked(&[crossing], true);
        project.tracks[0].clips[0].fade_in_seconds = 0.0;
        assert_eq!(
            project.fit_audio_clip_fades_to_range(&[crossing], 4.0, 8.0),
            0
        );
        project.set_audio_clips_locked(&[crossing], false);
        assert_eq!(
            project.fit_audio_clip_fades_to_range(&[crossing, inside, outside], 4.0, 8.0),
            1,
            "only the event whose fade was cleared changes again"
        );
        assert_eq!(
            project.fit_audio_clip_fades_to_range(&[crossing, inside, outside], 4.0, 4.0),
            0,
            "an invalid range does nothing"
        );
    }

    #[test]
    fn removing_a_time_range_splits_crossers_and_closes_the_gap() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Midi);
        // 8 s at 120 BPM spans 16 beats and crosses the 4..8 range on both ends.
        project
            .add_audio_clip(0, PathBuf::from("long.wav"), 0.0, 8.0)
            .unwrap();
        // Fully inside the range: removed.
        project
            .add_audio_clip(0, PathBuf::from("inside.wav"), 4.0, 1.0)
            .unwrap();
        // Fully after: shifts left by the four-beat gap.
        project
            .add_audio_clip(0, PathBuf::from("after.wav"), 16.0, 1.0)
            .unwrap();
        let midi = project.add_midi_part(1, 0.0, 16.0).unwrap();
        project.add_marker(2.0, "Intro").unwrap();
        project.add_marker(6.0, "Inside").unwrap();
        project.add_marker(10.0, "After").unwrap();
        project.tracks[0].automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![
                AutomationPoint {
                    beat: 2.0,
                    value: 0.5,
                },
                AutomationPoint {
                    beat: 6.0,
                    value: 0.6,
                },
                AutomationPoint {
                    beat: 10.0,
                    value: 0.7,
                },
            ],
        });

        let changed = project
            .remove_time_range(4.0, 8.0, &[])
            .expect("time removed");
        assert!(changed > 0);

        let spans = project.tracks[0]
            .clips
            .iter()
            .map(|clip| {
                (
                    clip.start_beats,
                    clip.start_beats + clip.duration_seconds * 2.0,
                    clip.source_path.to_string_lossy().into_owned(),
                )
            })
            .collect::<Vec<_>>();
        // The long event survives as 0..4 and 4..12 (its post-gap half moved
        // left by the four-beat gap); the inside event is gone; the after event
        // moved from 16..18 to 12..14.
        assert_eq!(spans.len(), 3);
        assert!(spans.iter().any(|(start, end, path)| path.contains("long")
            && (*start - 0.0).abs() < 1e-9
            && (*end - 4.0).abs() < 1e-9));
        assert!(spans.iter().any(|(start, end, path)| path.contains("long")
            && (*start - 4.0).abs() < 1e-9
            && (*end - 12.0).abs() < 1e-9));
        assert!(spans.iter().any(|(start, end, path)| path.contains("after")
            && (*start - 12.0).abs() < 1e-9
            && (*end - 14.0).abs() < 1e-9));
        assert!(!spans.iter().any(|(_, _, path)| path.contains("inside")));

        // The MIDI part was cut and its right half moved into the gap.
        let parts = project.tracks[1]
            .midi_parts
            .iter()
            .map(|part| (part.start_beats, part.start_beats + part.duration_beats))
            .collect::<Vec<_>>();
        assert!(parts.contains(&(0.0, 4.0)));
        assert!(parts.contains(&(4.0, 12.0)), "{parts:?}");
        assert_eq!(
            project.tracks[1].midi_parts[0].id, midi,
            "the left half keeps the original identity"
        );

        // Markers inside the range go, later markers move.
        assert_eq!(
            project
                .markers
                .iter()
                .map(|marker| (marker.name.as_str(), marker.beat))
                .collect::<Vec<_>>(),
            vec![("Intro", 2.0), ("After", 6.0)]
        );
        // Automation follows the same rule.
        let points = project.tracks[0].automation_lanes[0]
            .points
            .iter()
            .map(|point| point.beat)
            .collect::<Vec<_>>();
        assert_eq!(points, vec![2.0, 6.0]);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn removing_time_honours_locks_scopes_and_invalid_ranges() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let inside = project
            .add_audio_clip(0, PathBuf::from("inside.wav"), 4.0, 1.0)
            .unwrap();
        let after = project
            .add_audio_clip(0, PathBuf::from("after.wav"), 16.0, 1.0)
            .unwrap();
        let other = project
            .add_audio_clip(1, PathBuf::from("other.wav"), 16.0, 1.0)
            .unwrap();

        // A locked event inside the range refuses the edit.
        project.set_audio_clips_locked(&[inside], true);
        let before = project.clone();
        assert_eq!(project.remove_time_range(4.0, 8.0, &[]), None);
        assert_eq!(project, before);
        project.set_audio_clips_locked(&[inside], false);

        // A scoped removal edits one track and leaves the other alone, and does
        // not move global markers.
        project.add_marker(10.0, "After").unwrap();
        let track_id = project.tracks[0].id;
        assert!(project.remove_time_range(4.0, 8.0, &[track_id]).is_some());
        assert!(!project.tracks[0].clips.iter().any(|clip| clip.id == inside));
        assert_eq!(
            project.tracks[0]
                .clips
                .iter()
                .find(|clip| clip.id == after)
                .expect("after")
                .start_beats,
            12.0
        );
        assert_eq!(
            project.tracks[1]
                .clips
                .iter()
                .find(|clip| clip.id == other)
                .expect("other")
                .start_beats,
            16.0,
            "the other track keeps its timing"
        );
        assert_eq!(project.markers[0].beat, 10.0, "scoped edits keep markers");

        // Invalid ranges are refused.
        assert_eq!(project.remove_time_range(8.0, 8.0, &[]), None);
        assert_eq!(project.remove_time_range(-1.0, 4.0, &[]), None);
        assert_eq!(project.remove_time_range(0.0, 4.0, &[9_999]), None);
    }

    #[test]
    fn inserting_silence_splits_crossing_events_and_moves_the_rest() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Midi);
        // 4 s at 120 BPM is 8 beats, so this event crosses beat 4.
        project
            .add_audio_clip(0, PathBuf::from("long.wav"), 0.0, 4.0)
            .unwrap();
        project
            .add_audio_clip(0, PathBuf::from("after.wav"), 16.0, 1.0)
            .unwrap();
        project.add_midi_part(1, 0.0, 16.0).unwrap();
        project.add_marker(8.0, "Chorus").unwrap();
        project.add_marker(2.0, "Intro").unwrap();
        project.tracks[0].automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![AutomationPoint {
                beat: 10.0,
                value: 0.5,
            }],
        });

        assert_eq!(project.insert_silence_at(4.0, 4.0), Some(5));
        // The spanning event became two: the left half stays, the right half
        // starts after the inserted gap.
        let starts = project.tracks[0]
            .clips
            .iter()
            .map(|clip| clip.start_beats)
            .collect::<Vec<_>>();
        assert_eq!(starts, vec![0.0, 8.0, 20.0]);
        // Both MIDI halves moved too.
        let part_starts = project.tracks[1]
            .midi_parts
            .iter()
            .map(|part| part.start_beats)
            .collect::<Vec<_>>();
        assert_eq!(part_starts, vec![0.0, 8.0]);
        // Markers after the point moved, the one before stayed.
        assert_eq!(
            project
                .markers
                .iter()
                .map(|marker| (marker.name.as_str(), marker.beat))
                .collect::<Vec<_>>(),
            vec![("Intro", 2.0), ("Chorus", 12.0)]
        );
        // The automation point moved with the timeline.
        assert_eq!(project.tracks[0].automation_lanes[0].points[0].beat, 14.0);
        assert!(project.validate().is_ok());

        // Nothing at or after the point means nothing to insert.
        assert_eq!(project.insert_silence_at(1_000.0, 4.0), None);
        // Invalid lengths are refused.
        assert_eq!(project.insert_silence_at(4.0, 0.0), None);
        assert_eq!(project.insert_silence_at(-1.0, 4.0), None);
    }

    #[test]
    fn inserting_silence_refuses_when_a_locked_event_crosses_the_point() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let clip = project
            .add_audio_clip(0, PathBuf::from("long.wav"), 0.0, 4.0)
            .unwrap();
        project
            .add_audio_clip(0, PathBuf::from("after.wav"), 16.0, 1.0)
            .unwrap();
        project.set_audio_clips_locked(&[clip], true);
        let before = project.clone();
        assert_eq!(project.insert_silence_at(4.0, 4.0), None);
        assert_eq!(project, before, "a refused insert changes nothing");

        // A locked event after the point is simply shifted with the timeline.
        project.set_audio_clips_locked(&[clip], false);
        let after = project.tracks[0].clips[1].id;
        project.set_audio_clips_locked(&[after], true);
        assert!(project.insert_silence_at(4.0, 4.0).is_some());
        assert_eq!(
            project.tracks[0]
                .clips
                .iter()
                .find(|candidate| candidate.id == after)
                .expect("after")
                .start_beats,
            20.0
        );
    }

    #[test]
    fn nudging_an_event_start_carries_the_source_offset_and_stops_at_the_source() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // Two beats at 120 BPM is one second; the media is ten seconds long.
        let clip = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 4.0, 2.0)
            .unwrap();

        // Later start: trims two beats in, so the source offset advances.
        assert!(project.nudge_audio_clip_edge(
            clip,
            ClipEdge::Start,
            2.0,
            10.0,
            MIN_CLIP_EDGE_SECONDS
        ));
        let current = &project.tracks[0].clips[0];
        assert_eq!(current.start_beats, 6.0);
        assert!((current.source_offset_seconds - 1.0).abs() < 1e-9);
        assert!((current.duration_seconds - 1.0).abs() < 1e-9);

        // Earlier start: the source only reaches back one second, so it stops.
        assert!(project.nudge_audio_clip_edge(
            clip,
            ClipEdge::Start,
            -8.0,
            10.0,
            MIN_CLIP_EDGE_SECONDS
        ));
        let current = &project.tracks[0].clips[0];
        assert_eq!(current.start_beats, 4.0);
        assert!((current.source_offset_seconds - 0.0).abs() < 1e-9);
        assert!((current.duration_seconds - 2.0).abs() < 1e-9);

        // At the earliest point there is nothing more to give.
        assert!(!project.nudge_audio_clip_edge(
            clip,
            ClipEdge::Start,
            -2.0,
            10.0,
            MIN_CLIP_EDGE_SECONDS
        ));
        assert!(project.validate().is_ok());
    }

    #[test]
    fn nudging_an_event_end_stays_inside_the_media_and_keeps_the_minimum() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let clip = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 0.0, 1.0)
            .unwrap();

        // Longer, up to the source's remaining 2.5 seconds.
        assert!(project.nudge_audio_clip_edge(
            clip,
            ClipEdge::End,
            6.0,
            2.5,
            MIN_CLIP_EDGE_SECONDS
        ));
        assert!((project.tracks[0].clips[0].duration_seconds - 2.5).abs() < 1e-9);
        // Beyond the media it reports no change rather than reading past the end.
        assert!(!project.nudge_audio_clip_edge(
            clip,
            ClipEdge::End,
            1.0,
            2.5,
            MIN_CLIP_EDGE_SECONDS
        ));
        // Shorter keeps the minimum length.
        assert!(project.nudge_audio_clip_edge(
            clip,
            ClipEdge::End,
            -50.0,
            2.5,
            MIN_CLIP_EDGE_SECONDS
        ));
        assert!((project.tracks[0].clips[0].duration_seconds - MIN_CLIP_EDGE_SECONDS).abs() < 1e-9);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn locked_events_refuse_edge_nudges_and_fades_stay_inside_the_event() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let clip = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 4.0, 2.0)
            .unwrap();
        project.tracks[0].clips[0].fade_in_seconds = 1.5;
        project.tracks[0].clips[0].fade_out_seconds = 0.5;

        // Shortening the end clamps the fades into the new length.
        assert!(project.nudge_audio_clip_edge(
            clip,
            ClipEdge::End,
            -2.0,
            10.0,
            MIN_CLIP_EDGE_SECONDS
        ));
        let current = &project.tracks[0].clips[0];
        assert!(current.fade_in_seconds <= current.duration_seconds);
        assert!(
            current.fade_in_seconds + current.fade_out_seconds <= current.duration_seconds + 1e-9
        );
        assert!(project.validate().is_ok());

        project.set_audio_clips_locked(&[clip], true);
        let before = project.tracks[0].clips[0].clone();
        assert!(!project.nudge_audio_clip_edge(
            clip,
            ClipEdge::Start,
            2.0,
            10.0,
            MIN_CLIP_EDGE_SECONDS
        ));
        assert_eq!(project.tracks[0].clips[0], before);
    }

    #[test]
    fn timecode_at_the_cursor_sets_a_bounded_offset() {
        use crate::project::MAX_TIMECODE_OFFSET_SECONDS;

        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        // Two beats is one second; asking for 01:00:00 there offsets by 3599 s.
        assert!(project.set_timecode_at_cursor(2.0, 3600.0));
        assert!((project.timecode_offset_seconds - 3599.0).abs() < 1e-9);
        assert!((project.timecode_seconds_at(2.0) - 3600.0).abs() < 1e-9);
        assert!((project.timecode_seconds_at(4.0) - 3601.0).abs() < 1e-9);

        // A refused value leaves the offset untouched.
        let before = project.timecode_offset_seconds;
        assert!(!project.set_timecode_at_cursor(0.0, MAX_TIMECODE_OFFSET_SECONDS + 1.0));
        assert_eq!(project.timecode_offset_seconds, before);
        assert!(!project.set_timecode_at_cursor(0.0, f64::NAN));
        assert!(!project.set_timecode_at_cursor(-1.0, 0.0));
        assert!(project.validate().is_ok());

        // An out-of-range offset loaded from a file is rejected.
        project.timecode_offset_seconds = MAX_TIMECODE_OFFSET_SECONDS * 2.0;
        assert!(project.validate().is_err());
    }

    #[test]
    fn exploding_channels_keeps_each_channel_together() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let part = project.add_midi_part(0, 0.0, 8.0).expect("part");
        for (start, pitch, channel) in [(0.0, 60, 0), (1.0, 64, 2), (2.0, 67, 2)] {
            project
                .add_midi_note_on_channel(0, part, start, 0.5, pitch, channel, 100)
                .expect("note");
        }
        project
            .add_midi_controller_event_on_channel(0, part, 1.0, 11, 0, 64)
            .expect("controller");
        project
            .add_midi_controller_event_on_channel(0, part, 1.5, 11, 2, 90)
            .expect("controller");
        // A controller on a channel with no notes must still be kept.
        project
            .add_midi_controller_event_on_channel(0, part, 2.5, 64, 5, 127)
            .expect("controller");
        let track_id = project.tracks[0].id;

        let exploded = project
            .explode_midi_part_by_channel(track_id, part, 16)
            .expect("explode");
        assert_eq!(exploded.len(), 3, "channels 0, 2 and 5");
        assert!(project.tracks[0].midi_parts.is_empty());
        let summary = project
            .tracks
            .iter()
            .flat_map(|track| track.midi_parts.iter())
            .map(|part| {
                (
                    part.name.clone(),
                    part.notes.len(),
                    part.controller_events.len(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            vec![
                ("MIDI Part 01 (channel 1)".to_owned(), 1, 1),
                ("MIDI Part 01 (channel 3)".to_owned(), 2, 1),
                ("MIDI Part 01 (channel 6)".to_owned(), 0, 1),
            ],
            "each channel keeps its own notes and controller data"
        );
        // Every note keeps its own channel.
        let channels = project
            .tracks
            .iter()
            .flat_map(|track| track.midi_parts.iter())
            .flat_map(|part| part.notes.iter())
            .map(|note| note.channel)
            .collect::<Vec<_>>();
        assert_eq!(channels, vec![0, 2, 2]);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn exploding_channels_refuses_empty_or_too_wide_parts() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let empty = project.add_midi_part(0, 0.0, 4.0).expect("part");
        let track_id = project.tracks[0].id;
        let before = project.clone();
        assert!(
            project
                .explode_midi_part_by_channel(track_id, empty, 16)
                .is_err()
        );
        assert_eq!(project, before);

        let part = project.add_midi_part(0, 4.0, 4.0).expect("part");
        for channel in [0, 1, 2] {
            project
                .add_midi_note_on_channel(0, part, 0.0, 0.5, 60 + channel, channel, 100)
                .expect("note");
        }
        let before = project.clone();
        assert!(
            project
                .explode_midi_part_by_channel(track_id, part, 2)
                .is_err()
        );
        assert_eq!(project, before);
        assert!(
            project
                .explode_midi_part_by_channel(track_id, part, 3)
                .is_ok()
        );
    }

    #[test]
    fn exploding_pitches_makes_one_track_per_pitch_and_keeps_controllers_once() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Midi);
        let part = project.add_midi_part(0, 0.0, 8.0).expect("part");
        for (start, pitch) in [(0.0, 60), (1.0, 64), (2.0, 60), (3.0, 67)] {
            project
                .add_midi_note_on_channel(0, part, start, 0.5, pitch, 0, 100)
                .expect("note");
        }
        project
            .add_midi_controller_event_on_channel(0, part, 1.0, 11, 0, 64)
            .expect("controller");
        let track_id = project.tracks[0].id;

        let exploded = project
            .explode_midi_part_by_pitch(track_id, part, 16)
            .expect("explode");
        assert_eq!(exploded.len(), 3, "three distinct pitches");
        assert_eq!(project.tracks.len(), 4);
        assert!(
            project.tracks[0].midi_parts.is_empty(),
            "the source part was removed"
        );
        let pitches = project
            .tracks
            .iter()
            .flat_map(|track| track.midi_parts.iter())
            .map(|part| part.notes[0].pitch)
            .collect::<Vec<_>>();
        assert_eq!(pitches, vec![60, 64, 67]);
        let counts = project
            .tracks
            .iter()
            .flat_map(|track| track.midi_parts.iter())
            .map(|part| part.notes.len())
            .collect::<Vec<_>>();
        assert_eq!(
            counts,
            vec![2, 1, 1],
            "the duplicated pitch keeps both notes"
        );
        // Controller events live on the lowest-pitched part only.
        let controller_parts = project
            .tracks
            .iter()
            .flat_map(|track| track.midi_parts.iter())
            .filter(|part| !part.controller_events.is_empty())
            .count();
        assert_eq!(controller_parts, 1);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn exploding_refuses_empty_or_too_wide_parts_without_half_applying() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let empty = project.add_midi_part(0, 0.0, 4.0).expect("part");
        let track_id = project.tracks[0].id;
        let before = project.clone();
        assert!(
            project
                .explode_midi_part_by_pitch(track_id, empty, 16)
                .is_err()
        );
        assert_eq!(project, before, "a refused explode changes nothing");

        let part = project.add_midi_part(0, 4.0, 4.0).expect("part");
        for pitch in [60, 62, 64] {
            project
                .add_midi_note_on_channel(0, part, 0.0, 0.5, pitch, 0, 100)
                .expect("note");
        }
        let before = project.clone();
        assert!(
            project
                .explode_midi_part_by_pitch(track_id, part, 2)
                .is_err()
        );
        assert_eq!(project, before);
        assert!(
            project
                .explode_midi_part_by_pitch(track_id, part, 3)
                .is_ok()
        );
    }

    #[test]
    fn event_gain_is_set_from_decibels_and_clamped() {
        use crate::project::MAX_AUDIO_CLIP_GAIN;

        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let clip = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let locked = project
            .add_audio_clip(0, PathBuf::from("b.wav"), 4.0, 2.0)
            .unwrap();
        project.set_audio_clips_locked(&[locked], true);

        // +6 dB is about 1.995 linear.
        assert_eq!(project.set_audio_clip_gain_db(&[clip, locked], 6.0), 1);
        assert!((project.tracks[0].clips[0].gain - 1.995_262).abs() < 1e-4);
        assert_eq!(project.tracks[0].clips[1].gain, 1.0, "locked stays put");
        assert_eq!(
            project.set_audio_clip_gain_db(&[clip], 6.0),
            0,
            "no change twice"
        );

        // The range is clamped instead of producing an invalid project.
        assert_eq!(project.set_audio_clip_gain_db(&[clip], 40.0), 1);
        assert!((project.tracks[0].clips[0].gain as f64 - MAX_AUDIO_CLIP_GAIN).abs() < 1e-6);
        assert_eq!(project.set_audio_clip_gain_db(&[clip], -80.0), 1);
        assert_eq!(project.tracks[0].clips[0].gain, 0.0);
        assert_eq!(project.set_audio_clip_gain_db(&[clip], f64::NAN), 0);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn fades_apply_clamp_and_skip_locked_events() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let long = project
            .add_audio_clip(0, PathBuf::from("long.wav"), 0.0, 4.0)
            .unwrap();
        let short = project
            .add_audio_clip(0, PathBuf::from("short.wav"), 8.0, 0.5)
            .unwrap();
        let locked = project
            .add_audio_clip(0, PathBuf::from("locked.wav"), 16.0, 4.0)
            .unwrap();
        project.set_audio_clips_locked(&[locked], true);

        assert_eq!(
            project.fade_audio_clips(&[long, short, locked], true, 0.25),
            2
        );
        let clips = &project.tracks[0].clips;
        assert_eq!(clips[0].fade_in_seconds, 0.25);
        assert_eq!(clips[1].fade_in_seconds, 0.25, "clamped to the clip length");
        assert_eq!(clips[2].fade_in_seconds, 0.0, "locked stays untouched");

        assert_eq!(project.fade_audio_clips(&[long], false, 1.0), 1);
        assert_eq!(project.tracks[0].clips[0].fade_out_seconds, 1.0);
        // The sum can never exceed the event, so a long fade in gives way.
        assert_eq!(project.fade_audio_clips(&[long], true, 10.0), 1);
        assert_eq!(project.tracks[0].clips[0].fade_in_seconds, 3.0);
        assert!(
            (project.tracks[0].clips[0].fade_in_seconds
                + project.tracks[0].clips[0].fade_out_seconds
                - 4.0)
                .abs()
                < 1e-9
        );
        assert!(project.validate().is_ok());
        assert_eq!(project.fade_audio_clips(&[long], true, f64::NAN), 0);
        assert_eq!(project.fade_audio_clips(&[long], true, 0.0), 0);
    }

    #[test]
    fn silence_zeroes_gain_and_keeps_the_media() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let clip = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let locked = project
            .add_audio_clip(0, PathBuf::from("b.wav"), 4.0, 2.0)
            .unwrap();
        project.set_audio_clips_locked(&[locked], true);

        assert_eq!(project.silence_audio_clips(&[clip, locked]), 1);
        assert_eq!(project.tracks[0].clips[0].gain, 0.0);
        assert_eq!(project.tracks[0].clips[1].gain, 1.0);
        assert!(project.media_pool.contains(&PathBuf::from("a.wav")));
        assert_eq!(project.silence_audio_clips(&[clip]), 0, "already silent");
        assert!(project.validate().is_ok());
    }

    #[test]
    fn event_navigation_is_strict_and_spans_audio_and_midi() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Midi);
        project
            .add_audio_clip(0, PathBuf::from("a.wav"), 4.0, 2.0)
            .unwrap();
        project
            .add_audio_clip(0, PathBuf::from("b.wav"), 12.0, 2.0)
            .unwrap();
        project.add_midi_part(1, 8.0, 4.0).unwrap();

        assert_eq!(project.next_event_start_after(0.0), Some(4.0));
        assert_eq!(project.next_event_start_after(4.0), Some(8.0), "strict");
        assert_eq!(project.next_event_start_after(9.5), Some(12.0));
        assert_eq!(project.next_event_start_after(12.0), None);

        assert_eq!(project.previous_event_start_before(12.0), Some(8.0));
        assert_eq!(project.previous_event_start_before(8.0), Some(4.0));
        assert_eq!(project.previous_event_start_before(4.0), None);
        assert_eq!(project.previous_event_start_before(0.0), None);
        assert_eq!(project.next_event_start_after(f64::NAN), None);
    }

    #[test]
    fn folding_groups_expands_or_collapses_only_group_tracks() {
        let mut project = Project::default();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        assert_eq!(project.set_all_groups_collapsed(true), 2);
        assert!(project.tracks[0].group_collapsed && project.tracks[1].group_collapsed);
        assert!(
            !project.tracks[2].group_collapsed,
            "audio tracks do not fold"
        );
        assert_eq!(
            project.set_all_groups_collapsed(true),
            0,
            "already collapsed"
        );
        assert_eq!(project.set_all_groups_collapsed(false), 2);
        assert!(
            project.tracks[..2]
                .iter()
                .all(|track| !track.group_collapsed)
        );
        assert!(project.validate().is_ok());
    }

    #[test]
    fn bouncing_replaces_the_selected_events_with_one_rendered_event() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let track_id = project.tracks[0].id;
        let first = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let second = project
            .add_audio_clip(0, PathBuf::from("b.wav"), 4.0, 2.0)
            .unwrap();
        // A third event outside the selection must survive.
        let untouched = project
            .add_audio_clip(0, PathBuf::from("c.wav"), 20.0, 2.0)
            .unwrap();

        let bounced = project
            .replace_audio_clips_with_bounce(
                track_id,
                &[first, second],
                PathBuf::from("bounce.wav"),
                0.0,
                6.0,
            )
            .expect("bounce");
        let ids = project.tracks[0]
            .clips
            .iter()
            .map(|clip| clip.id)
            .collect::<Vec<_>>();
        assert!(ids.contains(&bounced));
        assert!(ids.contains(&untouched));
        assert!(!ids.contains(&first) && !ids.contains(&second));
        let clip = project.tracks[0]
            .clips
            .iter()
            .find(|clip| clip.id == bounced)
            .expect("bounced clip");
        assert_eq!(clip.start_beats, 0.0);
        assert_eq!(clip.duration_seconds, 6.0);
        assert_eq!(clip.source_path, PathBuf::from("bounce.wav"));
        assert!(project.media_pool.contains(&PathBuf::from("bounce.wav")));
        // The replaced audio stays available for recovery.
        assert!(project.media_pool.contains(&PathBuf::from("a.wav")));
        assert!(project.validate().is_ok());
    }

    #[test]
    fn bouncing_refuses_a_locked_event_or_a_foreign_selection() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let track_id = project.tracks[0].id;
        let clip = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let elsewhere = project
            .add_audio_clip(1, PathBuf::from("b.wav"), 0.0, 2.0)
            .unwrap();

        project.set_audio_clips_locked(&[clip], true);
        assert!(
            project
                .replace_audio_clips_with_bounce(
                    track_id,
                    &[clip],
                    PathBuf::from("bounce.wav"),
                    0.0,
                    2.0
                )
                .is_none()
        );

        project.set_audio_clips_locked(&[clip], false);
        // A selection that includes an event on another track is refused.
        assert!(
            project
                .replace_audio_clips_with_bounce(
                    track_id,
                    &[clip, elsewhere],
                    PathBuf::from("bounce.wav"),
                    0.0,
                    2.0
                )
                .is_none()
        );
        assert!(
            project
                .replace_audio_clips_with_bounce(track_id, &[], PathBuf::from("b.wav"), 0.0, 2.0)
                .is_none()
        );
        assert_eq!(project.tracks[0].clips.len(), 1);
    }

    #[test]
    fn loop_recording_creates_one_take_per_pass_with_the_last_active() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.set_loop_range(0.0, 8.0);
        // 2 s at 120 BPM is 4 beats. Recording from beat 0 for 12 s covers three
        // passes of the 4-beat loop.
        let created = project
            .add_loop_take_clips(0, PathBuf::from("take.wav"), 0.0, 12.0, 0.0, 8.0)
            .expect("three passes");

        assert_eq!(created.len(), 3, "one clip per loop pass");
        let clips = &project.tracks[0].clips;
        assert!(clips.iter().all(|clip| clip.take_group == Some(1)));
        assert_eq!(clips.iter().filter(|clip| clip.take_active).count(), 1);
        assert!(
            clips.last().is_some_and(|clip| clip.take_active),
            "the newest pass is the active take"
        );
        // Every pass starts at the loop start, and each one reads further into
        // the recorded file.
        let offsets = clips
            .iter()
            .map(|clip| (clip.start_beats, clip.source_offset_seconds))
            .collect::<Vec<_>>();
        assert_eq!(offsets[0], (0.0, 0.0));
        assert_eq!(offsets[1].0, 0.0);
        assert!((offsets[1].1 - 4.0).abs() < 1e-9);
        assert!((offsets[2].1 - 8.0).abs() < 1e-9);
        assert!(project.validate().is_ok());

        // Picking another take moves the active flag within the group.
        assert!(project.set_active_take(created[0]));
        let clips = &project.tracks[0].clips;
        assert_eq!(clips.iter().filter(|clip| clip.take_active).count(), 1);
        assert!(clips[0].take_active);
        assert!(!project.set_active_take(created[0]), "already active");
    }

    #[test]
    fn a_take_that_never_crosses_the_loop_boundary_stays_a_single_clip() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.set_loop_range(0.0, 8.0);
        // 1.5 s is 3 beats: still inside the first pass.
        assert!(
            project
                .add_loop_take_clips(0, PathBuf::from("take.wav"), 0.0, 1.5, 0.0, 8.0)
                .is_none()
        );
        // An empty loop cannot be split either.
        assert!(
            project
                .add_loop_take_clips(0, PathBuf::from("take.wav"), 0.0, 12.0, 4.0, 4.0)
                .is_none()
        );
        // A recording that starts after the loop end is not a loop take.
        assert!(
            project
                .add_loop_take_clips(0, PathBuf::from("take.wav"), 9.0, 12.0, 0.0, 8.0)
                .is_none()
        );
        assert!(project.tracks[0].clips.is_empty());
    }

    #[test]
    fn take_fields_round_trip_and_older_projects_read_as_active() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        project.tracks[0].clips[0].take_group = Some(4);
        project.tracks[0].clips[0].take_active = false;
        let encoded = serde_json::to_string(&project).unwrap();
        let restored: Project = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.tracks[0].clips[0].take_group, Some(4));
        assert!(!restored.tracks[0].clips[0].take_active);

        // A clip written before takes existed is an active, ungrouped take.
        let legacy = encoded
            .replace(",\"take_active\":false", "")
            .replace(",\"take_group\":4", "");
        let legacy: Project = serde_json::from_str(&legacy).unwrap();
        assert!(legacy.tracks[0].clips[0].take_active);
        assert_eq!(legacy.tracks[0].clips[0].take_group, None);
    }

    #[test]
    fn monitoring_modes_decide_whether_live_input_reaches_the_mix() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let track = &mut project.tracks[0];

        assert_eq!(track.monitor_mode, MonitorMode::Off);
        assert!(!track.monitors_input(), "off never monitors");

        track.monitor_mode = MonitorMode::Auto;
        assert!(!track.monitors_input(), "auto waits for record enable");
        track.armed = true;
        assert!(track.monitors_input());

        track.monitor_mode = MonitorMode::On;
        track.armed = false;
        assert!(track.monitors_input(), "on monitors without arming");

        // The strip's one control cycles through the three states.
        assert_eq!(MonitorMode::Off.next(), MonitorMode::Auto);
        assert_eq!(MonitorMode::Auto.next(), MonitorMode::On);
        assert_eq!(MonitorMode::On.next(), MonitorMode::Off);

        // The mode round-trips and an older project without it reads as off.
        project.tracks[0].monitor_mode = MonitorMode::Auto;
        let encoded = serde_json::to_string(&project).unwrap();
        let restored: Project = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.tracks[0].monitor_mode, MonitorMode::Auto);
        let mut legacy: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        legacy["tracks"][0]
            .as_object_mut()
            .expect("track object")
            .remove("monitor_mode");
        let legacy: Project = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy.tracks[0].monitor_mode, MonitorMode::Off);
    }

    #[test]
    fn splitting_a_midi_part_keeps_every_note_on_one_side_of_the_cut() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let track_id = project.tracks[0].id;
        let part = project.add_midi_part(0, 0.0, 8.0).expect("part");
        // Two notes before the cut at beat 4, one crossing it, one after.
        for (start, duration, pitch) in [
            (0.0, 1.0, 60),
            (2.0, 1.0, 62),
            (3.5, 1.0, 64),
            (5.0, 1.0, 65),
        ] {
            project
                .add_midi_note_on_channel(0, part, start, duration, pitch, 0, 100)
                .expect("note");
        }

        let right = project
            .split_midi_part(track_id, part, 4.0)
            .expect("split at beat 4");
        assert_ne!(right, part);

        let left_part = project.tracks[0]
            .midi_parts
            .iter()
            .find(|candidate| candidate.id == part)
            .expect("left part");
        let right_part = project.tracks[0]
            .midi_parts
            .iter()
            .find(|candidate| candidate.id == right)
            .expect("right part");
        assert_eq!(left_part.start_beats, 0.0);
        assert_eq!(left_part.duration_beats, 4.0);
        assert_eq!(right_part.start_beats, 4.0);
        assert_eq!(right_part.duration_beats, 4.0);

        // The crossing note is cut rather than dropped: 64 ends at the cut and a
        // 64 starts the right part at its own zero.
        assert!(left_part.notes.iter().any(|note| note.pitch == 64));
        assert!(right_part.notes.iter().any(|note| note.pitch == 64));
        assert!(right_part.notes.iter().any(|note| note.pitch == 65));
        assert!(!right_part.notes.iter().any(|note| note.pitch == 60));
        assert!(project.validate().is_ok());

        // A cut outside the part is refused.
        assert!(project.split_midi_part(track_id, part, 0.0).is_none());
        assert!(project.split_midi_part(track_id, part, 99.0).is_none());
    }

    #[test]
    fn merging_parts_keeps_every_note_at_its_musical_position() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let track_id = project.tracks[0].id;
        let first = project.add_midi_part(0, 0.0, 4.0).expect("part");
        let second = project.add_midi_part(0, 4.0, 4.0).expect("part");
        project
            .add_midi_note_on_channel(0, first, 1.0, 1.0, 60, 0, 100)
            .expect("note");
        project
            .add_midi_note_on_channel(0, second, 0.5, 1.0, 64, 0, 100)
            .expect("note");
        project
            .add_midi_controller_event_on_channel(0, second, 1.0, 11, 0, 64)
            .expect("event");

        let merged = project
            .merge_midi_parts(track_id, &[first, second])
            .expect("merge");
        assert_eq!(merged, first, "the earliest part keeps its identity");
        assert_eq!(project.tracks[0].midi_parts.len(), 1);
        let part = &project.tracks[0].midi_parts[0];
        assert_eq!(part.start_beats, 0.0);
        assert_eq!(part.duration_beats, 8.0);
        // The second part's note was at its own 0.5, so it lands at 4.5 here.
        let positions = part
            .notes
            .iter()
            .map(|note| (note.start_beats, note.pitch))
            .collect::<Vec<_>>();
        assert!(
            positions
                .iter()
                .any(|(start, pitch)| (*start - 1.0).abs() < 1e-9 && *pitch == 60)
        );
        assert!(
            positions
                .iter()
                .any(|(start, pitch)| (*start - 4.5).abs() < 1e-9 && *pitch == 64)
        );
        assert_eq!(part.controller_events.len(), 1);
        assert!((part.controller_events[0].position_beats - 5.0).abs() < 1e-9);
        assert!(project.validate().is_ok());

        // Merging needs two existing parts on the track.
        assert!(project.merge_midi_parts(track_id, &[first]).is_none());
        assert!(
            project
                .merge_midi_parts(track_id, &[first, 9_999])
                .is_none()
        );
    }

    #[test]
    fn gluing_contiguous_pieces_restores_one_event() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // 4 s at 120 BPM is 8 beats; split it in the middle.
        let original = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 0.0, 4.0)
            .unwrap();
        let track_id = project.tracks[0].id;
        let right = project
            .split_audio_clip(track_id, original, 4.0)
            .expect("split");
        assert_eq!(project.tracks[0].clips.len(), 2);

        let merged = project
            .glue_audio_clips(&[original, right])
            .expect("contiguous pieces glue");
        assert_eq!(merged, original, "the earliest piece keeps the identity");
        assert_eq!(project.tracks[0].clips.len(), 1);
        let clip = &project.tracks[0].clips[0];
        assert!((clip.start_beats - 0.0).abs() < 1e-9);
        assert!((clip.duration_seconds - 4.0).abs() < 1e-9);
        assert!((clip.source_offset_seconds - 0.0).abs() < 1e-9);
        assert_eq!(clip.source_path, PathBuf::from("take.wav"));
        assert!(project.validate().is_ok());
    }

    #[test]
    fn glue_refuses_pieces_it_cannot_join_without_rendering() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        // A gap between the pieces.
        let first = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let apart = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 8.0, 2.0)
            .unwrap();
        assert_eq!(project.glue_audio_clips(&[first, apart]), None);

        // Different sources.
        let other = project
            .add_audio_clip(0, PathBuf::from("b.wav"), 4.0, 2.0)
            .unwrap();
        assert_eq!(project.glue_audio_clips(&[first, other]), None);

        // Different tracks.
        let elsewhere = project
            .add_audio_clip(1, PathBuf::from("a.wav"), 4.0, 2.0)
            .unwrap();
        assert_eq!(project.glue_audio_clips(&[first, elsewhere]), None);

        // One piece is not a glue.
        assert_eq!(project.glue_audio_clips(&[first]), None);

        // A locked piece blocks the join.
        let right = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 4.0, 2.0)
            .unwrap();
        project.set_audio_clips_locked(&[right], true);
        assert_eq!(project.glue_audio_clips(&[first, right]), None);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn glue_refuses_faded_pieces_and_mismatched_gain() {
        // A split of a faded event leaves its fades in the source-fade lineage,
        // which glue cannot recompute, so it refuses instead of dropping them.
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let original = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 0.0, 4.0)
            .unwrap();
        project.tracks[0].clips[0].fade_in_seconds = 0.25;
        project.tracks[0].clips[0].fade_out_seconds = 0.5;
        let track_id = project.tracks[0].id;
        let right = project
            .split_audio_clip(track_id, original, 4.0)
            .expect("split");
        assert_eq!(project.glue_audio_clips(&[original, right]), None);

        // The same split without fades glues, and the result carries no fades.
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let plain = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 0.0, 4.0)
            .unwrap();
        let track_id = project.tracks[0].id;
        let right = project
            .split_audio_clip(track_id, plain, 4.0)
            .expect("split");
        let merged = project.glue_audio_clips(&[plain, right]).expect("glue");
        let clip = project.tracks[0]
            .clips
            .iter()
            .find(|clip| clip.id == merged)
            .expect("merged");
        assert_eq!(clip.fade_in_seconds, 0.0);
        assert_eq!(clip.fade_out_seconds, 0.0);
        assert!(clip.source_fades.is_empty());

        // A gain difference would need rendering, so it is refused.
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let a = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 0.0, 2.0)
            .unwrap();
        let b = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 4.0, 2.0)
            .unwrap();
        project.tracks[0].clips[1].gain = 0.5;
        project.tracks[0].clips[1].source_offset_seconds = 2.0;
        assert_eq!(project.glue_audio_clips(&[a, b]), None);
    }

    #[test]
    fn deleting_a_range_leaves_a_gap_and_keeps_the_outer_parts() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // 8 s at 120 BPM is 16 beats, so this event spans the whole range.
        let spanning = project
            .add_audio_clip(0, PathBuf::from("long.wav"), 0.0, 8.0)
            .unwrap();

        assert_eq!(
            project.delete_range_from_audio_clips(&[spanning], 4.0, 8.0),
            1
        );
        let mut spans = project.tracks[0]
            .clips
            .iter()
            .map(|clip| {
                (
                    clip.start_beats,
                    clip.start_beats + clip.duration_seconds * 120.0 / 60.0,
                )
            })
            .collect::<Vec<_>>();
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        assert_eq!(spans, vec![(0.0, 4.0), (8.0, 16.0)], "only the gap is gone");
        assert!(project.validate().is_ok());
        assert!(
            project.tracks[0]
                .clips
                .iter()
                .all(|clip| clip.source_path == PathBuf::from("long.wav"))
        );
    }

    #[test]
    fn deleting_a_range_removes_events_inside_and_skips_those_outside() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // 1 s at 120 BPM is 2 beats.
        let inside = project
            .add_audio_clip(0, PathBuf::from("inside.wav"), 4.0, 1.0)
            .unwrap();
        let before = project
            .add_audio_clip(0, PathBuf::from("before.wav"), 0.0, 1.0)
            .unwrap();
        let after = project
            .add_audio_clip(0, PathBuf::from("after.wav"), 12.0, 1.0)
            .unwrap();
        let locked = project
            .add_audio_clip(0, PathBuf::from("locked.wav"), 5.0, 1.0)
            .unwrap();
        project.set_audio_clips_locked(&[locked], true);

        let removed =
            project.delete_range_from_audio_clips(&[inside, before, after, locked], 3.0, 9.0);
        assert_eq!(removed, 1, "only the unlocked event inside the range goes");
        let remaining = project.tracks[0]
            .clips
            .iter()
            .map(|clip| clip.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(remaining, BTreeSet::from([before, after, locked]));
        assert!(!project.tracks[0].clips.iter().any(|clip| clip.id == inside));

        // An event spanning the range keeps both outer parts.
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let spanning = project
            .add_audio_clip(0, PathBuf::from("span.wav"), 2.0, 4.0)
            .unwrap();
        assert_eq!(
            project.delete_range_from_audio_clips(&[spanning], 3.0, 6.0),
            1
        );
        let mut spans = project.tracks[0]
            .clips
            .iter()
            .map(|clip| {
                (
                    clip.start_beats,
                    clip.start_beats + clip.duration_seconds * 120.0 / 60.0,
                )
            })
            .collect::<Vec<_>>();
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        assert_eq!(spans, vec![(2.0, 3.0), (6.0, 10.0)]);

        // An event that starts inside the range and ends after it keeps only its
        // tail, under the same id.
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let head = project
            .add_audio_clip(0, PathBuf::from("head.wav"), 3.0, 4.0)
            .unwrap();
        assert_eq!(project.delete_range_from_audio_clips(&[head], 3.0, 6.0), 1);
        let clip = &project.tracks[0].clips[0];
        assert_eq!(clip.id, head, "the surviving tail keeps the event identity");
        assert!((clip.start_beats + clip.duration_seconds * 120.0 / 60.0 - 11.0).abs() < 1e-9);
    }

    #[test]
    fn split_loop_cuts_every_event_at_both_locators() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        // 8 s at 120 BPM is 16 beats, so this event spans the whole loop 4..8.
        let spanning = project
            .add_audio_clip(0, PathBuf::from("long.wav"), 0.0, 8.0)
            .unwrap();
        // 4 s at 120 BPM is 8 beats, so this one spans 2..10 and crosses both
        // locators as well.
        project
            .add_audio_clip(1, PathBuf::from("left.wav"), 2.0, 4.0)
            .unwrap();
        project.set_loop_range(4.0, 8.0);
        let track_id = project.tracks[0].id;
        project.set_audio_clips_locked(&[spanning], false);

        let (split, created) = project.split_audio_clips_at_loop();
        assert_eq!(split, 4, "each event crosses both locators");
        assert_eq!(created.len(), 4);
        let beats = project.tracks[0]
            .clips
            .iter()
            .map(|clip| clip.start_beats)
            .collect::<Vec<_>>();
        assert_eq!(
            beats,
            vec![0.0, 4.0, 8.0],
            "the spanning event became three"
        );
        assert!(project.validate().is_ok());
        // The split copies keep the source identity.
        assert!(
            project.tracks[0]
                .clips
                .iter()
                .all(|clip| clip.source_path == PathBuf::from("long.wav"))
        );
        assert_eq!(project.tracks[1].clips.len(), 3);
        assert_eq!(project.tracks[0].id, track_id);
    }

    #[test]
    fn split_loop_skips_locked_events_and_reports_when_nothing_crosses() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 8.0)
            .unwrap();
        project.set_loop_range(4.0, 8.0);
        project.set_audio_clips_locked(&[id], true);
        assert_eq!(
            project.split_audio_clips_at_loop().0,
            0,
            "locked stays whole"
        );

        project.set_audio_clips_locked(&[id], false);
        project.set_loop_range(20.0, 24.0);
        assert_eq!(
            project.split_audio_clips_at_loop().0,
            0,
            "nothing crosses locators past the event"
        );
    }

    #[test]
    fn markers_add_rename_move_remove_and_navigate_in_order() {
        let mut project = Project::default();
        let intro = project.add_marker(0.0, "Intro").expect("intro");
        let chorus = project.add_marker(16.0, "Chorus").expect("chorus");
        let verse = project.add_marker(8.0, "Verse").expect("verse");
        // The list stays ordered by beat however the markers were added.
        assert_eq!(
            project
                .markers
                .iter()
                .map(|marker| marker.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Intro", "Verse", "Chorus"]
        );

        assert!(project.rename_marker(verse, "Verse 1"));
        assert!(!project.rename_marker(verse, "Verse 1"), "no change twice");
        assert!(project.move_marker(chorus, 4.0));
        assert_eq!(
            project
                .markers
                .iter()
                .map(|marker| marker.beat)
                .collect::<Vec<_>>(),
            vec![0.0, 4.0, 8.0]
        );

        // Navigation is strict: a marker exactly at the cursor is not "next".
        assert_eq!(
            project
                .next_marker_after(0.0, false)
                .map(|marker| marker.id),
            Some(chorus)
        );
        assert_eq!(
            project
                .previous_marker_before(8.0, false)
                .map(|marker| marker.id),
            Some(chorus)
        );
        assert_eq!(project.next_marker_after(16.0, false), None);
        assert_eq!(
            project
                .next_marker_after(16.0, true)
                .map(|marker| marker.id),
            Some(intro),
            "wrapping goes back to the first marker"
        );
        assert_eq!(
            project
                .previous_marker_before(0.0, true)
                .map(|marker| marker.id),
            Some(verse),
            "wrapping goes to the last marker"
        );

        assert!(project.remove_marker(verse));
        assert!(!project.remove_marker(verse));
        assert_eq!(project.markers.len(), 2);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn marker_slots_create_move_and_stay_unique() {
        let mut project = Project::default();
        let first = project.set_marker_slot(3, 8.0, "Marker 3").expect("slot 3");
        assert_eq!(
            project
                .markers
                .iter()
                .find(|marker| marker.id == first)
                .and_then(|marker| marker.slot),
            Some(3)
        );
        assert_eq!(
            project.marker_in_slot(3).map(|marker| marker.name.as_str()),
            Some("Marker 3")
        );
        assert!(project.marker_in_slot(1).is_none());

        // Moving the slot keeps the marker's identity.
        let moved = project.set_marker_slot(3, 16.0, "Marker 3").expect("moved");
        assert_eq!(moved, first);
        assert_eq!(project.markers.len(), 1);
        assert_eq!(
            project.marker_in_slot(3).map(|marker| marker.beat),
            Some(16.0)
        );

        // A plain marker has no slot and can coexist.
        let plain = project.add_marker(4.0, "Intro").expect("plain");
        assert!(
            project
                .markers
                .iter()
                .find(|marker| marker.id == plain)
                .is_some_and(|marker| marker.slot.is_none())
        );

        // Slots outside 1-9 are refused.
        assert!(project.set_marker_slot(0, 1.0, "bad").is_none());
        assert!(project.set_marker_slot(10, 1.0, "bad").is_none());

        // Duplicate slots are invalid, so decoding rejects such a file.
        project.markers[0].slot = Some(4);
        project.markers[1].slot = Some(4);
        assert_eq!(
            project.validate(),
            Err("marker slots must be unique and between 1 and 9")
        );
        assert!(project.validate().is_err());
    }

    #[test]
    fn markers_are_bounded_and_round_trip_with_a_legacy_default() {
        use crate::project::{MAX_MARKER_NAME_CHARS, MAX_MARKERS};

        let mut project = Project::default();
        let id = project
            .add_marker(4.0, "x".repeat(MAX_MARKER_NAME_CHARS * 3))
            .expect("marker");
        assert_eq!(
            project.markers[0].name.chars().count(),
            MAX_MARKER_NAME_CHARS,
            "the name is bounded"
        );
        assert!(project.add_marker(f64::NAN, "bad").is_none());
        assert!(project.add_marker(-1.0, "bad").is_none());
        assert!(project.add_marker(1_000_001.0, "bad").is_none());

        let encoded = serde_json::to_string(&project).unwrap();
        let restored: Project = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.markers, project.markers);
        assert_eq!(restored.next_marker_id, project.next_marker_id);
        assert_eq!(restored.markers[0].id, id);

        // A project saved before markers existed loads with none.
        let mut legacy: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        legacy.as_object_mut().expect("object").remove("markers");
        legacy
            .as_object_mut()
            .expect("object")
            .remove("next_marker_id");
        let legacy: Project = serde_json::from_value(legacy).unwrap();
        assert!(legacy.markers.is_empty());
        assert_eq!(legacy.next_marker_id, 1, "ids restart safely");
        // A project without markers does not store the empty list.
        assert!(
            !serde_json::to_string(&legacy)
                .unwrap()
                .contains("\"markers\"")
        );

        // The count is bounded.
        let mut full = Project::default();
        for index in 0..MAX_MARKERS {
            assert!(full.add_marker(index as f64, "m").is_some());
        }
        assert!(full.add_marker(0.0, "overflow").is_none());
    }

    #[test]
    fn a_locked_event_refuses_move_trim_split_and_crop() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, PathBuf::from("take.wav"), 0.0, 2.0)
            .unwrap();
        assert_eq!(project.set_audio_clips_locked(&[id], true), 1);
        assert_eq!(
            project.set_audio_clips_locked(&[id], true),
            0,
            "locking twice is not a change"
        );

        // Move to another track.
        let mut moved = project.tracks[0].clips[0].clone();
        moved.start_beats = 4.0;
        assert!(
            !project.move_audio_clips(&[(project.tracks[0].id, project.tracks[1].id, moved)]),
            "a locked event does not move"
        );
        // Trim in place.
        let mut trimmed = project.tracks[0].clips[0].clone();
        trimmed.duration_seconds = 1.0;
        assert!(
            !project.edit_audio_clip(project.tracks[0].id, project.tracks[0].id, trimmed),
            "a locked event does not trim"
        );
        // Split.
        assert_eq!(
            project.split_audio_clip(project.tracks[0].id, id, 1.0),
            None
        );
        // Crop.
        assert_eq!(project.crop_audio_clips_to_range(&[id], 0.0, 1.0), 0);

        // Unlocking restores every operation.
        assert_eq!(project.set_audio_clips_locked(&[id], false), 1);
        assert!(
            project
                .split_audio_clip(project.tracks[0].id, id, 1.0)
                .is_some()
        );
        assert_eq!(project.tracks[0].clips.len(), 2);
    }

    #[test]
    fn grouped_events_move_together_and_ungrouping_releases_them() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 1.0)
            .unwrap();
        let second = project
            .add_audio_clip(1, PathBuf::from("b.wav"), 4.0, 1.0)
            .unwrap();

        assert_eq!(
            project.group_audio_clips(&[first]),
            None,
            "one event is not a group"
        );
        let group = project
            .group_audio_clips(&[first, second])
            .expect("two events group");
        assert_eq!(project.tracks[0].clips[0].group, Some(group));
        assert_eq!(project.tracks[1].clips[0].group, Some(group));
        assert_eq!(
            project.expand_selection_to_groups(&[first]),
            vec![first, second],
            "selecting one member selects the group"
        );

        // Moving one member by hand moves the other by the same delta.
        let mut moved = project.tracks[0].clips[0].clone();
        moved.start_beats = 2.0;
        assert!(project.move_audio_clips(&[(project.tracks[0].id, project.tracks[0].id, moved)]));
        assert!((project.tracks[1].clips[0].start_beats - 6.0).abs() < 1e-9);

        // Ungrouping one member releases the whole group.
        assert_eq!(project.ungroup_audio_clips(&[second]), 2);
        assert!(
            project
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter())
                .all(|clip| clip.group.is_none())
        );
        assert_eq!(project.ungroup_audio_clips(&[second]), 0);
    }

    #[test]
    fn a_locked_group_member_stays_put_while_the_rest_moves() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 1.0)
            .unwrap();
        let second = project
            .add_audio_clip(0, PathBuf::from("b.wav"), 4.0, 1.0)
            .unwrap();
        project.group_audio_clips(&[first, second]).expect("group");
        project.set_audio_clips_locked(&[second], true);

        let mut moved = project.tracks[0].clips[0].clone();
        moved.start_beats = 2.0;
        assert!(project.move_audio_clips(&[(project.tracks[0].id, project.tracks[0].id, moved)]));
        let locked = project.tracks[0]
            .clips
            .iter()
            .find(|clip| clip.id == second)
            .expect("locked member");
        assert_eq!(
            locked.start_beats, 4.0,
            "a locked member keeps its place even inside a group move"
        );
    }

    #[test]
    fn lock_and_group_survive_a_round_trip_and_default_for_older_projects() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        project.tracks[0].clips[0].locked = true;
        project.tracks[0].clips[0].group = Some(3);
        let encoded = serde_json::to_string(&project).unwrap();
        let restored: Project = serde_json::from_str(&encoded).unwrap();
        assert!(restored.tracks[0].clips[0].locked);
        assert_eq!(restored.tracks[0].clips[0].group, Some(3));

        // A clip written before these fields existed is unlocked and ungrouped.
        let legacy = encoded
            .replace(",\"locked\":true", "")
            .replace(",\"group\":3", "");
        let legacy: Project = serde_json::from_str(&legacy).unwrap();
        assert!(!legacy.tracks[0].clips[0].locked);
        assert_eq!(legacy.tracks[0].clips[0].group, None);
        // An ungrouped clip does not store the field at all.
        assert!(!encoded.contains("\"group\":null"));
    }

    #[test]
    fn repeat_places_each_copy_one_selection_span_later() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // 2 s at 120 BPM is 4 beats, so the selection spans beats 0 through 4.
        let id = project
            .add_audio_clip(0, PathBuf::from("loop.wav"), 0.0, 2.0)
            .unwrap();

        let created = project.repeat_audio_clips(&[id], 3);
        assert_eq!(created.len(), 3);
        let mut starts = project.tracks[0]
            .clips
            .iter()
            .map(|clip| clip.start_beats)
            .collect::<Vec<_>>();
        starts.sort_by(f64::total_cmp);
        assert_eq!(starts, vec![0.0, 4.0, 8.0, 12.0]);
    }

    #[test]
    fn repeat_keeps_the_spacing_of_a_multi_event_selection() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 1.0)
            .unwrap();
        let second = project
            .add_audio_clip(1, PathBuf::from("b.wav"), 3.0, 1.0)
            .unwrap();

        let created = project.repeat_audio_clips(&[first, second], 1);
        assert_eq!(created.len(), 2);
        // The selection spans beats 0 through 5, so both copies move by 5.
        assert!(
            project.tracks[0].clips.iter().any(|clip| {
                created.contains(&clip.id) && (clip.start_beats - 5.0).abs() < 1e-9
            })
        );
        assert!(
            project.tracks[1].clips.iter().any(|clip| {
                created.contains(&clip.id) && (clip.start_beats - 8.0).abs() < 1e-9
            })
        );
    }

    #[test]
    fn repeat_is_bounded_and_leaves_the_project_alone_when_it_cannot_run() {
        use crate::project::MAX_REPEAT_COUNT;

        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 1.0)
            .unwrap();
        let before = project.clone();
        assert!(project.repeat_audio_clips(&[id], 0).is_empty());
        assert!(project.repeat_audio_clips(&[], 4).is_empty());
        assert!(project.repeat_audio_clips(&[9_999], 4).is_empty());
        assert_eq!(project, before);

        // One command cannot exceed the cap even if asked for more.
        let created = project.repeat_audio_clips(&[id], 1_000);
        assert_eq!(created.len(), MAX_REPEAT_COUNT as usize);
    }

    #[test]
    fn fill_loop_repeats_until_the_loop_end() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // One bar of 4/4 at 120 BPM is 8 beats; the event is half a bar.
        let id = project
            .add_audio_clip(0, PathBuf::from("half.wav"), 0.0, 2.0)
            .unwrap();
        project.set_loop_range(0.0, 16.0);

        let created = project.fill_loop_with_audio_clips(&[id], 0.0, 16.0);
        // Beats 0-4 are original, so 4-8, 8-12 and 12-16 are added.
        assert_eq!(created.len(), 3);
        let farthest = project.tracks[0]
            .clips
            .iter()
            .map(|clip| clip.start_beats)
            .fold(0.0_f64, f64::max);
        assert!((farthest - 12.0).abs() < 1e-9);

        let before = project.clone();
        assert!(
            project
                .fill_loop_with_audio_clips(&[id], 8.0, 8.0)
                .is_empty()
        );
        assert_eq!(project, before);
    }

    #[test]
    fn crop_trims_crossing_events_and_leaves_outside_events_alone() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // 4 s at 120 BPM is 8 beats: one event crosses both edges, one sits
        // entirely before the range and one entirely after it.
        let crossing = project
            .add_audio_clip(0, PathBuf::from("long.wav"), 2.0, 4.0)
            .unwrap();
        let before = project
            .add_audio_clip(0, PathBuf::from("early.wav"), 0.0, 0.5)
            .unwrap();
        let after = project
            .add_audio_clip(0, PathBuf::from("late.wav"), 20.0, 0.5)
            .unwrap();

        let trimmed = project.crop_audio_clips_to_range(&[crossing, before, after], 4.0, 8.0);
        assert_eq!(trimmed, 1, "only the crossing event is trimmed");
        let clips = &project.tracks[0].clips;
        let cropped = clips
            .iter()
            .find(|clip| clip.id == crossing)
            .expect("cropped event keeps its id");
        assert!((cropped.start_beats - 4.0).abs() < 1e-9);
        assert!(
            (cropped.start_beats + cropped.duration_seconds * 120.0 / 60.0 - 8.0).abs() < 1e-9,
            "the tail is trimmed to the range end"
        );
        assert!(clips.iter().any(|clip| clip.id == before));
        assert!(clips.iter().any(|clip| clip.id == after));
        assert!(project.validate().is_ok());
    }

    #[test]
    fn crop_trims_each_edge_independently_and_rejects_empty_ranges() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        // 3 s at 120 BPM is 6 beats: beats 1 through 7.
        let id = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 1.0, 3.0)
            .unwrap();

        let before = project.clone();
        assert_eq!(project.crop_audio_clips_to_range(&[id], 4.0, 4.0), 0);
        assert_eq!(project.crop_audio_clips_to_range(&[id], 0.0, 1.0), 0);
        assert_eq!(project, before);

        // Only the head is trimmed when the range ends after the event.
        assert_eq!(project.crop_audio_clips_to_range(&[id], 3.0, 40.0), 1);
        let clip = &project.tracks[0].clips[0];
        assert!((clip.start_beats - 3.0).abs() < 1e-9);
        assert!(
            (clip.start_beats + clip.duration_seconds * 120.0 / 60.0 - 7.0).abs() < 1e-9,
            "the tail is untouched"
        );
    }

    #[test]
    fn routing_validation_accepts_every_route_the_menus_offer() {
        use crate::routing::{InputRoute, OutputRoute};

        let mut project = Project::default();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        let group_id = project.tracks[0].id;
        project.tracks[1].parent_group_id = Some(group_id);
        project.tracks[1].input_routing = Some(InputRoute::MonoIn { channel: 2 }.persisted());
        project.tracks[1].output_routing = Some(OutputRoute::Group(group_id).persisted());
        assert_eq!(project.validate_routing(), Ok(()));

        // Speaker assignments and No Bus are legal too.
        for route in [
            OutputRoute::Left,
            OutputRoute::Right,
            OutputRoute::StereoOut,
            OutputRoute::NoBus,
        ] {
            project.tracks[1].output_routing = Some(route.persisted());
            project.tracks[1].group_output_auto = false;
            assert_eq!(project.validate_routing(), Ok(()), "{route:?}");
        }
        for route in [
            InputRoute::StereoIn,
            InputRoute::MonoIn { channel: 1 },
            InputRoute::NoBus,
        ] {
            project.tracks[1].input_routing = Some(route.persisted());
            assert_eq!(project.validate_routing(), Ok(()), "{route:?}");
        }
    }

    #[test]
    fn routing_validation_reports_a_group_that_does_not_exist() {
        use crate::routing::RoutingError;

        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].output_routing = Some("Group:999".to_owned());
        assert_eq!(
            project.validate_routing(),
            Err(RoutingError::MissingGroup { group: 999 })
        );

        // A send to a missing group is caught the same way.
        project.tracks[0].output_routing = Some("Stereo Out".to_owned());
        project.tracks[0].send_routing = vec![Some("Group:999".to_owned())];
        assert_eq!(
            project.validate_routing(),
            Err(RoutingError::MissingGroup { group: 999 })
        );
    }

    #[test]
    fn routing_validation_rejects_unknown_routes_and_narrow_inputs() {
        use crate::routing::RoutingError;

        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].output_routing = Some("Surround 5.1".to_owned());
        assert_eq!(
            project.validate_routing(),
            Err(RoutingError::UnknownRoute("Surround 5.1".to_owned()))
        );

        project.tracks[0].output_routing = Some("Stereo Out".to_owned());
        project.tracks[0].input_routing = Some("Input 3".to_owned());
        assert_eq!(
            project.validate_routing(),
            Err(RoutingError::UnsupportedInputChannel {
                channel: 3,
                available: 2
            })
        );
    }

    #[test]
    fn routing_validation_rejects_self_routing_and_feedback_loops() {
        use crate::routing::RoutingError;

        let mut project = Project::default();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Group);
        let (first, second) = (project.tracks[0].id, project.tracks[1].id);

        project.tracks[0].output_routing = Some(format!("Group:{first}"));
        assert_eq!(
            project.validate_routing(),
            Err(RoutingError::SelfGroup { group: first })
        );

        project.tracks[0].output_routing = Some(format!("Group:{second}"));
        project.tracks[1].output_routing = Some(format!("Group:{first}"));
        assert_eq!(
            project.validate_routing(),
            Err(RoutingError::RoutingCycle { group: second })
        );

        // Breaking the loop makes the project routable again.
        project.tracks[1].output_routing = Some("Stereo Out".to_owned());
        assert_eq!(project.validate_routing(), Ok(()));
    }

    #[test]
    fn muted_events_round_trip_and_older_projects_default_to_unmuted() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        project.tracks[0].clips[0].muted = true;
        let encoded = serde_json::to_string(&project).unwrap();
        let restored: Project = serde_json::from_str(&encoded).unwrap();
        assert!(restored.tracks[0].clips[0].muted);

        // A clip written before the field existed decodes as audible.
        let legacy = encoded.replace(",\"muted\":true", "");
        let legacy: Project = serde_json::from_str(&legacy).unwrap();
        assert!(!legacy.tracks[0].clips[0].muted);
    }
}

impl Project {
    pub fn media_paths(&self) -> BTreeSet<PathBuf> {
        self.media_pool
            .iter()
            .cloned()
            .chain(
                self.tracks
                    .iter()
                    .flat_map(|t| t.clips.iter().map(|c| c.source_path.clone())),
            )
            .collect()
    }
    /// The timecode shown at a timeline beat, in seconds.
    pub fn timecode_seconds_at(&self, beats: f64) -> f64 {
        if !beats.is_finite() {
            return self.timecode_offset_seconds.max(0.0);
        }
        beats * 60.0 / self.tempo_bpm.max(1.0) + self.timecode_offset_seconds
    }

    /// Makes `timecode_seconds` the timecode shown at `cursor_beats`.
    ///
    /// The offset is bounded so an absurd entry cannot make the ruler unusable or
    /// the project invalid; a refused value leaves the project unchanged.
    pub fn set_timecode_at_cursor(&mut self, cursor_beats: f64, timecode_seconds: f64) -> bool {
        if !cursor_beats.is_finite() || cursor_beats < 0.0 || !timecode_seconds.is_finite() {
            return false;
        }
        let cursor_seconds = cursor_beats * 60.0 / self.tempo_bpm.max(1.0);
        let offset = timecode_seconds - cursor_seconds;
        if !offset.is_finite() || offset.abs() > MAX_TIMECODE_OFFSET_SECONDS {
            return false;
        }
        self.timecode_offset_seconds = offset;
        true
    }

    /// Replaces the project notepad, bounded to [`MAX_NOTEPAD_CHARS`].
    pub fn set_notepad(&mut self, text: impl Into<String>) -> bool {
        let mut text = text.into();
        if text.chars().count() > MAX_NOTEPAD_CHARS {
            text = text.chars().take(MAX_NOTEPAD_CHARS).collect();
        }
        if self.notepad == text {
            return false;
        }
        self.notepad = text;
        true
    }

    /// Sources an event actually references, ignoring the pool.
    pub fn referenced_media(&self) -> BTreeSet<PathBuf> {
        self.tracks
            .iter()
            .flat_map(|track| track.clips.iter().map(|clip| clip.source_path.clone()))
            .collect()
    }

    /// Pool entries no event references, sorted.
    ///
    /// These are the files the pool keeps after their last event was removed;
    /// removing them from the pool is reversible and never touches the file.
    pub fn unused_media(&self) -> Vec<PathBuf> {
        let referenced = self.referenced_media();
        self.media_pool
            .iter()
            .filter(|path| !referenced.contains(*path))
            .cloned()
            .collect()
    }

    /// Drops the unused pool entries and returns them.
    pub fn remove_unused_media(&mut self) -> Vec<PathBuf> {
        let unused = self.unused_media();
        for path in &unused {
            self.media_pool.remove(path);
        }
        unused
    }

    /// Pool entries whose file is gone and that no event references, sorted.
    ///
    /// A missing file that an event still references is deliberately kept: the
    /// event needs it for playback and the relink command resolves it.
    pub fn missing_unreferenced_media(&self) -> Vec<PathBuf> {
        self.unused_media()
            .into_iter()
            .filter(|path| !path.exists())
            .collect()
    }

    /// Drops the missing, unreferenced pool entries and returns them.
    pub fn remove_missing_unreferenced_media(&mut self) -> Vec<PathBuf> {
        let missing = self.missing_unreferenced_media();
        for path in &missing {
            self.media_pool.remove(path);
        }
        missing
    }

    pub fn register_media(&mut self) {
        self.media_pool = self.media_paths();
    }
}

impl AudioClip {
    /// Event-relative sample position used by preview and cut commit.
    pub fn cut_offset_seconds(&self, beat: f64, tempo: f64, rate: u32) -> Option<f64> {
        if !beat.is_finite() || !tempo.is_finite() || tempo <= 0.0 {
            return None;
        }
        let rate = rate.max(1) as f64;
        let seconds = (((beat - self.start_beats) * 60.0 / tempo) * rate).round() / rate;
        if seconds < 1.0 / rate || self.duration_seconds - seconds < 1.0 / rate {
            None
        } else {
            Some(seconds)
        }
    }
}
