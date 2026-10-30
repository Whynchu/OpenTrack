#[path = "analyzer_view.rs"]
mod analyzer_view;
#[path = "audio_configuration.rs"]
mod audio_configuration;
#[path = "audio_events.rs"]
pub(crate) mod audio_events;
#[path = "channel_gain.rs"]
mod channel_gain;
#[path = "channel_settings.rs"]
mod channel_settings;
#[path = "media_names.rs"]
pub(crate) mod media_names;
#[path = "media_pool.rs"]
mod media_pool;
#[path = "metering.rs"]
mod metering;
#[path = "mixer.rs"]
mod mixer;
#[path = "preferences.rs"]
mod preferences;
#[path = "project_controls.rs"]
mod project_controls;
#[path = "project_inspector.rs"]
mod project_inspector;
#[path = "project_rate.rs"]
mod project_rate;
#[path = "recording_undo.rs"]
mod recording_undo;
#[path = "sample_editor.rs"]
mod sample_editor;
#[path = "transport_panel.rs"]
mod transport_panel;
#[path = "vst3_host.rs"]
mod vst3_host;
#[path = "waveform_view.rs"]
mod waveform_view;

use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    time::{Duration, Instant},
};

use eframe::egui::{self, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, Vec2};
use egui_dock::DockState;
use opentrack_audio::{
    AudioDevice, AudioDeviceMode, AudioEngine, AudioInputDevice, AudioInputFormat, AudioInputProbe,
    AudioOutputConfig, AudioRecordRoute, AudioRecorder, AudioSource, MidiInputDevice,
    MidiInputEvent, MidiInputMessage, MidiInputStream, PlaybackClip, PlaybackInputChannel,
    PlaybackMidiController, PlaybackMidiNote, PlaybackOutputRoute, PlaybackSource,
    PlaybackTrackSettings, StreamingPlayback, oscillator_instrument_factory, output_configurations,
    scan_audio_devices,
};
use opentrack_core::commands::{
    ArrowKey, BindingResolution, BindingScope, CommandId, KeyBinding, KeyChord, KeyCode,
    KeyModifiers, KeymapProfile,
};
use opentrack_core::project::{
    CycleMode, FilterType, GeneratorWaveform, InsertEffect, InsertEffectKind, InstrumentWaveform,
    Project, RecordingFormat, Track, TrackKind,
};
use opentrack_core::workspaces::{
    ProjectWorkspaceState, WindowBounds, WindowClass, Workspace, WorkspaceId, WorkspacePreferences,
    WorkspacePreset,
};
use opentrack_plugins::{PluginCategory, PluginDescriptor};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

use crate::icons::ToolbarIcon;
use crate::native_keys::NativeKeyCapture;
use crate::project_file::{
    ArrangeViewState, CHANNEL_SETTINGS_WINDOW_ID, MEDIA_WINDOW_ID, MIXER_WINDOW_ID,
    PROJECT_WINDOW_ID, ProjectFile, ProjectFileError, arrange_view_state_from_workspace,
    dock_state_from_workspace, initial_workspace_state, main_window_window_snapshot,
    midi_editor_window_snapshot, project_window_snapshot, store_active_dock_state,
    store_active_project_window_state, store_active_workspace_child_state,
};
use crate::theme;

static NEXT_AUDIO_IMPORT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub(crate) enum PanelTab {
    Project,
    Mixer,
    Media,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum EditTool {
    #[default]
    Select,
    Range,
    Split,
    Glue,
    Delete,
    Zoom,
    Mute,
    Draw,
    Play,
    Drumstick,
}

#[derive(Default)]
struct TransportState {
    playing: bool,
    recording: bool,
    cycle: bool,
    position_beats: f64,
    last_clock: Option<f64>,
}

#[derive(Default)]
struct NativeWindowState {
    channel_bus: Option<bool>,
    open: bool,
    bounds: Option<WindowBounds>,
    restore_bounds: Option<WindowBounds>,
    epoch: u64,
    viewport_live: bool,
    focus_requested: bool,
    /// Window level last sent to the compositor, so it is not re-applied per frame.
    window_level: Option<egui::viewport::WindowLevel>,
}

#[derive(Default)]
struct TransportWindowState {
    bounds: Option<WindowBounds>,
    viewport_live: bool,
    always_on_top: bool,
}

impl NativeWindowState {
    fn request_channel_settings(&mut self, bus: Option<bool>) {
        if !self.open || !self.viewport_live {
            self.epoch = self.epoch.wrapping_add(1);
            self.viewport_live = false;
            self.window_level = None;
        }
        self.channel_bus = bus;
        self.open = true;
        self.focus_requested = true;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum ChannelSettingsViewItem {
    Inserts,
    InsertRouting,
    Equalizers,
    EqualizersCurve,
    EqualizersCurveDials,
    Sends,
    SendRouting,
    StudioSends,
    StudioSendsRouting,
    ControlStrip,
    Fader,
}

impl ChannelSettingsViewItem {
    const ALL: [Self; 11] = [
        Self::Inserts,
        Self::InsertRouting,
        Self::Equalizers,
        Self::EqualizersCurve,
        Self::EqualizersCurveDials,
        Self::Sends,
        Self::SendRouting,
        Self::StudioSends,
        Self::StudioSendsRouting,
        Self::ControlStrip,
        Self::Fader,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Inserts => "Inserts",
            Self::InsertRouting => "Insert Routing",
            Self::Equalizers => "Equalizers",
            Self::EqualizersCurve => "Equalizers + Curve",
            Self::EqualizersCurveDials => "Equalizers + Curve + Dials",
            Self::Sends => "Sends",
            Self::SendRouting => "Send Routing",
            Self::StudioSends => "Studio Sends",
            Self::StudioSendsRouting => "Studio Sends Routing",
            Self::ControlStrip => "Control Strip",
            Self::Fader => "Fader",
        }
    }

    fn available_for_bus(self) -> bool {
        !matches!(
            self,
            Self::Sends | Self::SendRouting | Self::StudioSends | Self::StudioSendsRouting
        )
    }
    fn implemented(self) -> bool {
        !matches!(
            self,
            Self::InsertRouting | Self::StudioSends | Self::StudioSendsRouting
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChannelSettingsLayoutPreset {
    name: String,
    visible_items: Vec<ChannelSettingsViewItem>,
}

const CHANNEL_SETTINGS_PREFERENCES_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChannelSettingsPreferences {
    #[serde(default = "default_channel_settings_items")]
    visible_items: Vec<ChannelSettingsViewItem>,
    #[serde(default)]
    presets: Vec<ChannelSettingsLayoutPreset>,
    #[serde(default)]
    always_on_top: bool,
    /// Bumped when a default changes, so an existing file adopts it once.
    #[serde(default)]
    version: u32,
}

fn default_channel_settings_items() -> Vec<ChannelSettingsViewItem> {
    vec![
        ChannelSettingsViewItem::Inserts,
        ChannelSettingsViewItem::EqualizersCurve,
        ChannelSettingsViewItem::Sends,
        ChannelSettingsViewItem::Fader,
    ]
}

impl Default for ChannelSettingsPreferences {
    fn default() -> Self {
        let visible_items = default_channel_settings_items();
        Self {
            presets: vec![ChannelSettingsLayoutPreset {
                name: "Default".to_owned(),
                visible_items: visible_items.clone(),
            }],
            visible_items,
            always_on_top: true,
            version: CHANNEL_SETTINGS_PREFERENCES_VERSION,
        }
    }
}

#[derive(Clone, Copy)]
enum ChannelSettingsAction {
    SaveSelected,
    LoadSelected,
    SaveAll,
    LoadAll,
}

#[derive(Default)]
struct ChannelSettingsUiState {
    analyzer: analyzer_view::State,
    bus_context: bool,
    preferences: ChannelSettingsPreferences,
    setup_open: bool,
    selected_preset: Option<usize>,
    preset_name_draft: String,
    pending_action: Option<ChannelSettingsAction>,
    editing_track_id: Option<u64>,
    rename_draft: String,
    rename_focus_requested: bool,
    setup_selected_visible: Option<usize>,
    setup_selected_hidden: Option<ChannelSettingsViewItem>,
    // Per-slot Send activation is currently UI state only; the track/audio model has no Send DSP.
    enabled_sends: HashSet<(u64, usize)>,
    /// Insert slot whose VST3 parameters/editor should be shown.
    vst3_parameter_slot: Option<(u64, usize)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChannelSettingsPresetFile {
    version: u32,
    channels: Vec<ChannelSettingsSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChannelSettingsSnapshot {
    #[serde(default)]
    bus_id: Option<u64>,
    name: String,
    kind: TrackKind,
    audio_channels: u8,
    muted: bool,
    solo: bool,
    solo_defeat: bool,
    listen: bool,
    armed: bool,
    monitor: bool,
    automation_read: bool,
    automation_write: bool,
    input_gain_db: f32,
    input_phase_inverted: bool,
    gain: f32,
    pan: f32,
    track_delay_ms: f32,
    equalizer: opentrack_core::project::ChannelEqualizer,
    inserts: Vec<InsertEffect>,
    inserts_bypassed: bool,
    sends_bypassed: bool,
    input_routing: Option<String>,
    output_routing: Option<String>,
    send_routing: Vec<Option<String>>,
}

impl From<&Track> for ChannelSettingsSnapshot {
    fn from(track: &Track) -> Self {
        Self {
            bus_id: (track.id >= opentrack_core::project::INPUT_BUS_ID).then_some(track.id),
            name: track.name.clone(),
            kind: track.kind,
            audio_channels: track.audio_channels,
            muted: track.muted,
            solo: track.solo,
            solo_defeat: track.solo_defeat,
            listen: track.listen,
            armed: track.armed,
            monitor: track.monitor,
            automation_read: track.automation_read,
            automation_write: track.automation_write,
            input_gain_db: track.input_gain_db,
            input_phase_inverted: track.input_phase_inverted,
            gain: track.gain,
            pan: track.pan,
            track_delay_ms: track.track_delay_ms,
            equalizer: track.equalizer.clone(),
            inserts: track.inserts.clone(),
            inserts_bypassed: track.inserts_bypassed,
            sends_bypassed: track.sends_bypassed,
            input_routing: track.input_routing.clone(),
            output_routing: track.output_routing.clone(),
            send_routing: track.send_routing.clone(),
        }
    }
}

impl ChannelSettingsSnapshot {
    fn apply_to(&self, track: &mut Track) {
        track.muted = self.muted;
        track.solo = self.solo;
        track.solo_defeat = self.solo_defeat;
        track.listen = self.listen;
        let bus = track.id >= opentrack_core::project::INPUT_BUS_ID;
        track.armed = !bus && self.armed;
        track.monitor = !bus && self.monitor;
        track.automation_read = self.automation_read;
        track.automation_write = self.automation_write;
        track.input_gain_db = self.input_gain_db;
        track.input_phase_inverted = self.input_phase_inverted;
        track.gain = self.gain;
        track.pan = self.pan;
        track.track_delay_ms = self.track_delay_ms;
        track.equalizer = self.equalizer.clone();
        track.inserts = self.inserts.clone();
        track.inserts_bypassed = self.inserts_bypassed;
        track.sends_bypassed = self.sends_bypassed;
        track.input_routing = self.input_routing.clone();
        track.output_routing = self.output_routing.clone();
        track.group_output_auto = false;
        track.send_routing = self.send_routing.clone();
    }
}

fn channel_snapshot_matches_track(snapshot: &ChannelSettingsSnapshot, track: &Track) -> bool {
    snapshot.bus_id == (track.id >= opentrack_core::project::INPUT_BUS_ID).then_some(track.id)
        && snapshot.name == track.name
        && channel_snapshot_compatible(snapshot, track)
}

fn channel_snapshot_compatible(snapshot: &ChannelSettingsSnapshot, track: &Track) -> bool {
    snapshot.kind == track.kind && snapshot.audio_channels == track.audio_channels
}

#[derive(Default)]
struct ProjectWindowState {
    open: bool,
    bounds: Option<WindowBounds>,
    restore_bounds: Option<WindowBounds>,
    minimized: bool,
    maximized: bool,
    z_order: u32,
    epoch: u64,
    focus_requested: bool,
}

struct TrackAddDialog {
    kind: TrackKind,
    count: u32,
    audio_channels: u8,
    browse_presets: bool,
}

#[derive(Clone)]
struct TrackRenameDialog {
    track_id: u64,
    draft: String,
}

struct EventColorsDialog {
    slot_index: usize,
    draft_rgb: [u8; 3],
}

#[derive(Clone, Copy)]
struct RulerGestureState {
    origin: Pos2,
    start_zoom: f32,
    start_bar_width: f32,
    start_scroll_x: f32,
    edge_scroll_x: f32,
    active: bool,
}

#[derive(Clone, Copy)]
enum RulerRangeDragKind {
    Draw,
    Resize {
        left: bool,
        anchor: f64,
        endpoint: f64,
        pointer_start: f64,
    },
    Move {
        anchor: f64,
        endpoint: f64,
        pointer_start: f64,
    },
}

#[derive(Clone, Copy)]
struct RulerRangeGestureState {
    origin: Pos2,
    anchor_beats: f64,
    endpoint_beats: f64,
    cancelled: bool,
    kind: RulerRangeDragKind,
}

fn restore_native_window_state(
    workspace: Option<&Workspace>,
    id: &str,
    previous_epoch: u64,
) -> NativeWindowState {
    let snapshot = workspace.and_then(|workspace| project_window_snapshot(workspace, id));
    let bounds = snapshot.and_then(|window| window.bounds.clone());
    NativeWindowState {
        channel_bus: None,
        open: snapshot.is_some_and(|window| window.open),
        bounds: bounds.clone(),
        restore_bounds: bounds,
        epoch: previous_epoch.wrapping_add(1),
        viewport_live: false,
        focus_requested: false,
        window_level: None,
    }
}

fn audio_sample_rate_choices(configs: &[AudioOutputConfig]) -> Vec<u32> {
    let common_rates = [
        22_050, 32_000, 44_100, 48_000, 88_200, 96_000, 176_400, 192_000,
    ];
    let mut rates = Vec::new();
    for config in configs {
        if !config.sample_format_supported_by_engine() {
            continue;
        }
        rates.extend(
            common_rates
                .into_iter()
                .filter(|rate| (config.min_sample_rate..=config.max_sample_rate).contains(rate)),
        );
        rates.push(config.min_sample_rate);
        rates.push(config.max_sample_rate);
    }
    rates.sort_unstable();
    rates.dedup();
    rates
}

fn audio_buffer_choices(configs: &[AudioOutputConfig], sample_rate: Option<u32>) -> Vec<u32> {
    let presets = [64, 128, 256, 512, 1_024, 2_048, 4_096];
    let configured_rate =
        sample_rate.or_else(|| configs.first().map(|config| config.default_sample_rate));
    let matching = configs
        .iter()
        .filter(|config| {
            config.sample_format_supported_by_engine()
                && configured_rate.is_some_and(|rate| {
                    (config.min_sample_rate..=config.max_sample_rate).contains(&rate)
                })
        })
        .collect::<Vec<_>>();
    let best = matching
        .iter()
        .map(|config| {
            (
                config.sample_format != config.default_sample_format,
                config.channels != config.default_channels,
                config.channels,
            )
        })
        .min();
    let mut buffers = matching
        .into_iter()
        .filter(|config| {
            Some((
                config.sample_format != config.default_sample_format,
                config.channels != config.default_channels,
                config.channels,
            )) == best
        })
        .flat_map(|config| {
            presets.into_iter().filter(move |frames| {
                config
                    .min_buffer_frames
                    .zip(config.max_buffer_frames)
                    .is_some_and(|(min, max)| *frames >= min && *frames <= max)
            })
        })
        .collect::<Vec<_>>();
    buffers.sort_unstable();
    buffers.dedup();
    buffers
}

fn reset_native_window_state(previous_epoch: u64) -> NativeWindowState {
    NativeWindowState {
        epoch: previous_epoch.wrapping_add(1),
        ..Default::default()
    }
}

fn restore_project_window_state(
    workspace: Option<&Workspace>,
    previous_epoch: u64,
) -> ProjectWindowState {
    let snapshot =
        workspace.and_then(|workspace| project_window_snapshot(workspace, PROJECT_WINDOW_ID));
    if let Some(snapshot) = snapshot {
        ProjectWindowState {
            open: snapshot.open,
            bounds: snapshot.bounds.clone(),
            restore_bounds: snapshot.restore_bounds.clone(),
            minimized: false,
            maximized: false,
            z_order: snapshot.z_order,
            epoch: previous_epoch.wrapping_add(1),
            focus_requested: false,
        }
    } else {
        ProjectWindowState {
            open: true,
            epoch: previous_epoch.wrapping_add(1),
            ..Default::default()
        }
    }
}

fn native_window_bounds(ctx: &egui::Context) -> Option<WindowBounds> {
    ctx.input(|input| {
        let viewport = input.viewport();
        if viewport.minimized.unwrap_or(false) {
            return None;
        }
        let outer = viewport.outer_rect?;
        let inner = viewport.inner_rect;
        Some(WindowBounds {
            left: f64::from(outer.left()),
            top: f64::from(outer.top()),
            width: f64::from(outer.width()),
            height: f64::from(outer.height()),
            inner_width: inner.map(|rect| f64::from(rect.width())),
            inner_height: inner.map(|rect| f64::from(rect.height())),
            monitor_id: None,
        })
    })
}

fn viewport_builder(
    title: impl Into<String>,
    default_size: Vec2,
    minimum_size: Vec2,
    restore_bounds: Option<&WindowBounds>,
) -> egui::ViewportBuilder {
    let mut builder = egui::ViewportBuilder::default()
        .with_title(title)
        // Each child window is a separate Wayland/X11 toplevel, so it needs the
        // same application id as the main window for the desktop to resolve its
        // icon; otherwise it falls back to a generic placeholder.
        .with_app_id("opentrack")
        .with_inner_size(default_size)
        .with_min_inner_size(minimum_size)
        .with_resizable(true)
        .with_icon(crate::icons::opentrack_icon());
    if let Some(bounds) = restore_bounds {
        builder = builder
            .with_position(Pos2::new(bounds.left as f32, bounds.top as f32))
            .with_inner_size(Vec2::new(
                bounds.inner_width.unwrap_or(bounds.width) as f32,
                bounds.inner_height.unwrap_or(bounds.height) as f32,
            ));
    }
    builder
}

fn apply_viewport_bounds(ctx: &egui::Context, bounds: &WindowBounds) {
    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(
        bounds.left as f32,
        bounds.top as f32,
    )));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
        bounds.inner_width.unwrap_or(bounds.width) as f32,
        bounds.inner_height.unwrap_or(bounds.height) as f32,
    )));
}

fn viewport_interaction(ctx: &egui::Context) -> (bool, bool, bool, bool) {
    ctx.input(|input| {
        let mut pointer_event = false;
        let mut pointer_released = false;
        let mut key_pressed = false;
        for event in &input.events {
            match event {
                egui::Event::PointerButton { pressed, .. } => {
                    pointer_event = true;
                    pointer_released |= !pressed;
                }
                egui::Event::Key { pressed: true, .. } => key_pressed = true,
                _ => {}
            }
        }
        let pointer_down = [
            egui::PointerButton::Primary,
            egui::PointerButton::Secondary,
            egui::PointerButton::Middle,
        ]
        .into_iter()
        .any(|button| input.pointer.button_down(button));
        (pointer_event, pointer_released, key_pressed, pointer_down)
    })
}

#[derive(Default)]
struct AppData {
    project: Project,
    transport: TransportState,
    selected_media: usize,
    audio_sources: HashMap<PathBuf, AudioSource>,
    streaming_readers: HashMap<u64, (PathBuf, Arc<StreamingPlayback>)>,
    track_meter_peaks: HashMap<u64, Arc<opentrack_audio::StereoMeter>>,
    track_meter_levels: HashMap<u64, [f32; 2]>,
    peak_holds: HashMap<u64, metering::Hold>,
    input_channel_meter_levels: [f32; 2],
    output_channel_meter_levels: [f32; 2],
    reported_stream_errors: HashSet<u64>,
    reported_stream_underruns: HashSet<u64>,
    reported_waveform_errors: HashSet<PathBuf>,
}

pub struct OpenTrackApp {
    dock_state: DockState<PanelTab>,
    workspace_state: ProjectWorkspaceState,
    arrange_view_state: ArrangeViewState,
    audio_event_clipboard: Option<audio_events::Clipboard>,
    project_window: ProjectWindowState,
    workspace_preferences: WorkspacePreferences,
    data: AppData,
    last_meter_update: Option<Instant>,
    metering_preferences: metering::Preferences,
    preferences_ui: preferences::Window,
    sample_editor: sample_editor::State,
    midi_pitch_view: sample_editor::PitchViewport,
    sample_editor_active: bool,
    active_note_audition: Option<sample_editor::NoteAuditionRequest>,
    note_audition_stop_at: Option<Instant>,
    vst3_runtime: vst3_host::Runtime,
    vst3_plugin_folders: Vec<PathBuf>,
    vst3_plugin_catalog: Vec<PluginDescriptor>,
    plugin_information_open: bool,
    plugin_scan_task: Option<PluginScanTask>,
    plugin_scan_report: Option<opentrack_plugin_scan::ScanReport>,
    plugin_scan_status: String,
    plugin_scan_error: Option<String>,
    root_window_handle: Option<isize>,
    startup_visibility_pending: bool,
    vst3_parameter_window: Option<(u64, usize)>,
    vst3_editor_window: Option<(u64, usize)>,
    vst3_editor_host: Option<vst3_host::EditorHost>,
    vst3_editor_bounds: Option<WindowBounds>,
    main_window_bounds: Option<WindowBounds>,
    main_window_restore_bounds: Option<WindowBounds>,
    show_transport: bool,
    transport_window: TransportWindowState,
    metronome_enabled: bool,
    metronome_level: f32,
    metronome_pitch_midi: u8,
    metronome_settings_open: bool,
    tempo_signature_settings_open: bool,
    edit_tool: EditTool,
    keymap_profile: Option<KeymapProfile>,
    native_key_capture: Option<NativeKeyCapture>,
    key_commands_open: bool,
    key_command_category: Option<String>,
    key_capture_target: Option<KeyCaptureTarget>,
    pending_key_assignment: Option<PendingKeyAssignment>,
    audio_engine: Option<AudioEngine>,
    audio_devices: Vec<AudioDevice>,
    audio_input_devices: Vec<AudioInputDevice>,
    audio_output_devices: Vec<AudioDevice>,
    audio_output_configs: Vec<AudioOutputConfig>,
    audio_output_sample_rates: Vec<u32>,
    audio_output_buffer_choices: Vec<u32>,
    audio_device_diagnostics: Vec<String>,
    cached_input_format: Option<AudioInputFormat>,
    resolved_input_device_name: Option<String>,
    audio_input_format_error: Option<String>,
    audio_input_probe: Option<AudioInputProbe>,
    audio_input_probe_status: Option<String>,
    device_test_active: bool,
    device_setup_open: bool,
    /// True while a menu in the menu bar is open.
    menu_open: bool,
    device_setup_page: u8,
    vst_connections_open: bool,
    vst_connections_tab: usize,
    project_setup_open: bool,
    project_rate_change: Option<project_rate::Change>,
    media_operation_error: Option<String>,
    selected_input_device: Option<String>,
    selected_output_device: Option<String>,
    selected_output_sample_rate: Option<u32>,
    selected_output_buffer_frames: Option<u32>,
    audio_device_mode: AudioDeviceMode,
    release_exclusive_audio_when_idle: bool,
    input_monitoring: bool,
    midi_input_devices: Vec<MidiInputDevice>,
    selected_midi_input_device: Option<String>,
    midi_input_stream: Option<MidiInputStream>,
    midi_event_scratch: Vec<MidiInputEvent>,
    midi_reported_dropped_messages: u64,
    midi_thru_enabled: bool,
    midi_editor_open: bool,
    channel_settings_window: NativeWindowState,
    channel_settings_ui: ChannelSettingsUiState,
    channel_window_owner: crate::native_windows::ChannelOwner,
    mixer_window: NativeWindowState,
    mixer_track_control_cache: Vec<(f32, f32)>,
    media_window: NativeWindowState,
    track_rename_dialog: Option<TrackRenameDialog>,
    track_add_dialog: Option<TrackAddDialog>,
    event_colors_dialog: Option<EventColorsDialog>,
    pending_track_delete_id: Option<u64>,
    midi_position_quantize_beats: f64,
    midi_length_quantize_beats: f64,
    midi_insert_velocity: u8,
    midi_controller_number: u8,
    selected_midi_note_id: Option<u64>,
    selected_midi_note_ids: BTreeSet<u64>,
    midi_note_drag: Option<MidiNoteDrag>,
    midi_note_draw: Option<MidiNoteDraw>,
    midi_part_draw: Option<MidiPartDraw>,
    midi_controller_drag: Option<MidiControllerDrag>,
    midi_pending_audio_rebuild: bool,
    project_history: ProjectHistory,
    recording_session: Option<RecordingSession>,
    audio_import_task: Option<AudioImportTask>,
    project_load_task: Option<ProjectLoadTask>,
    midi_recording_session: Option<MidiRecordingSession>,
    project_path: Option<std::path::PathBuf>,
    native_window_title: Option<String>,
    project_open: bool,
    project_folder_managed: bool,
    project_folder_setup: Option<ProjectFolderSetup>,
    saved_project_fingerprint: Vec<u8>,
    saved_workspace_fingerprint: Vec<u8>,
    modified: bool,
    pending_project_action: Option<PendingProjectAction>,
    workspace_organizer_open: bool,
    about_open: bool,
    confirmed_close_request: bool,
    status_message: String,
    realtime_diagnostic_details: String,
    realtime_diagnostic_refreshed_at: Option<Instant>,
    realtime_diagnostics_open: bool,
    project_open_error: Option<String>,
}

struct RecordingSession {
    routes: Vec<AudioRecordRoute>,
    recorder: AudioRecorder,
    track_ids: Vec<u64>,
    start_beats: f64,
}

struct RecordingPreview {
    start_beats: f64,
    tracks: Vec<(u64, AudioRecordRoute)>,
    waveform: Arc<opentrack_audio::Waveform>,
}

struct AudioImportTask {
    result: Receiver<Result<(PathBuf, AudioSource, bool), String>>,
}

struct ProjectLoadTask {
    path: PathBuf,
    result: Receiver<Result<LoadedProject, String>>,
    progress: Arc<Mutex<ProjectLoadStatus>>,
    pending_project: Option<LoadedProject>,
    next_plugin_to_load: usize,
}

#[derive(Clone)]
struct ProjectLoadStatus {
    fraction: f32,
    phase: String,
}

impl Default for ProjectLoadStatus {
    fn default() -> Self {
        Self {
            fraction: 0.02,
            phase: "Starting project load".to_owned(),
        }
    }
}

struct LoadedProject {
    document: ProjectFile,
    audio_sources: HashMap<PathBuf, AudioSource>,
    vst3_runtime: vst3_host::Runtime,
    vst3_plugins: Vec<(u64, usize, String)>,
}

struct PluginScanTask {
    result: Receiver<Result<opentrack_plugin_scan::ScanReport, String>>,
}

struct MidiRecordingSession {
    tracks: Vec<MidiRecordingTrack>,
    start_beats: f64,
    captured_events: u64,
}

struct MidiRecordingTrack {
    track_id: u64,
    part_id: u64,
    open_notes: HashMap<(u8, u8), Vec<OpenMidiNote>>,
}

#[derive(Clone, Copy)]
struct OpenMidiNote {
    note_id: u64,
    start_beats: f64,
}

#[derive(Clone)]
struct KeyCaptureTarget {
    command: CommandId,
    scope: BindingScope,
    replacing: Option<KeyBinding>,
}

#[derive(Clone)]
struct PendingKeyAssignment {
    target: KeyCaptureTarget,
    chord: KeyChord,
    conflicts: Vec<KeyBinding>,
}

enum KeyCommandUiAction {
    BeginCapture(KeyCaptureTarget),
    Remove(KeyBinding),
    RestoreDefaults,
}

enum WorkspacePresetUiAction {
    SaveActiveGlobal,
    SaveActiveProject,
    CopyGlobalToProject,
    CreateGlobal(usize),
    RemoveGlobal(usize),
    CreateProject(usize),
    RemoveProject(usize),
}

#[derive(Clone)]
struct MidiNoteDrag {
    track_id: u64,
    part_id: u64,
    mode: MidiNoteDragMode,
    members: Vec<MidiNoteDragMember>,
    delta_beats: f64,
    delta_rows: f64,
}

#[derive(Clone, Copy)]
struct MidiNoteDragMember {
    note_id: u64,
    start_beats: f64,
    duration_beats: f64,
    pitch: u8,
}

#[derive(Clone, Copy)]
struct MidiNoteDraw {
    track_id: u64,
    part_id: u64,
    start_beats: f64,
    end_beats: f64,
    pitch: u8,
    velocity: u8,
}

#[derive(Clone, Copy)]
struct MidiNoteSelectionDrag {
    track_id: u64,
    part_id: u64,
    start: Pos2,
    current: Pos2,
}

#[derive(Clone, Copy)]
struct MidiPartDraw {
    track_id: u64,
    start_beats: f64,
    end_beats: f64,
}

#[derive(Clone, Copy)]
struct MidiControllerDrag {
    track_id: u64,
    part_id: u64,
    event_id: u64,
    start_position_beats: f64,
    start_value: u8,
    delta_beats: f64,
    delta_value: f64,
}

#[derive(Clone, Copy)]
enum MidiNoteDragMode {
    Move,
    ResizeStart,
    ResizeEnd,
}

enum PendingProjectAction {
    New(PathBuf),
    Open(std::path::PathBuf),
    Close,
}

enum ProjectFolderSetup {
    New {
        project_name: String,
        parent: PathBuf,
        error: Option<String>,
    },
    Legacy {
        source_path: PathBuf,
        error: Option<String>,
    },
}

enum ProjectFolderAction {
    CreateNew { name: String, parent: PathBuf },
    CreateLegacyFolder { parent: PathBuf },
    AssignLegacy { root: PathBuf, copy_project: bool },
}

fn menu_command(ui: &mut egui::Ui, label: &str, shortcut: &str, enabled: bool) -> egui::Response {
    let button = if shortcut.is_empty() {
        egui::Button::new(label)
    } else {
        egui::Button::new(label).shortcut_text(shortcut)
    };
    ui.add_enabled(enabled, button)
}

fn checked_menu_command(
    ui: &mut egui::Ui,
    label: &str,
    shortcut: &str,
    checked: bool,
    enabled: bool,
) -> egui::Response {
    let response = menu_command(ui, &format!("    {label}"), shortcut, enabled);
    if checked {
        let center = Pos2::new(response.rect.min.x + 12.0, response.rect.center().y);
        let stroke = Stroke::new(1.5, ui.visuals().text_color());
        ui.painter().line_segment(
            [center + Vec2::new(-4.0, 0.0), center + Vec2::new(-1.0, 3.0)],
            stroke,
        );
        ui.painter().line_segment(
            [center + Vec2::new(-1.0, 3.0), center + Vec2::new(5.0, -4.0)],
            stroke,
        );
    }
    response
}

fn toolbar_choice(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    toolbar_choice_with_sense(ui, label, width, Sense::hover())
}

fn toolbar_choice_with_sense(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    sense: Sense,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 22.0), sense);
    let p = theme::palette(ui);
    let ink = if sense == Sense::click() && ui.is_enabled() {
        if ui.visuals().dark_mode {
            Color32::from_gray(230)
        } else {
            Color32::from_gray(24)
        }
    } else {
        p.muted
    };
    ui.painter().rect_filled(rect, 1.0, p.canvas);
    ui.painter().rect_stroke(
        rect,
        1.0,
        Stroke::new(1.0, p.grid),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(5.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        ink,
    );
    crate::icon_system::paint_app_icon(
        ui,
        Rect::from_center_size(rect.right_center() - Vec2::new(7.0, 0.0), Vec2::splat(12.0)),
        crate::icon_system::IconId::ChevronDown,
        ink,
    );
    response
}

fn project_grid_choices(ui: &mut egui::Ui, view: &mut ArrangeViewState) {
    use crate::timeline::{GridType, QuantizeNote, QuantizeSetting, QuantizeVariation};
    let grid = toolbar_choice_with_sense(ui, view.grid_type.label(), 84.0, Sense::click())
        .on_hover_text("Grid Type");
    egui::Popup::menu(&grid).show(|ui| {
        theme::menu_popup_style(ui.style_mut());
        for value in GridType::ALL {
            if ui
                .selectable_value(&mut view.grid_type, value, value.label())
                .clicked()
            {
                ui.close();
            }
        }
    });
    ui.add_space(20.0);
    let quantize = toolbar_choice_with_sense(ui, &view.quantize.label(), 84.0, Sense::click())
        .on_hover_text("Quantize Type");
    fn note_options(
        ui: &mut egui::Ui,
        selected: &mut QuantizeSetting,
        variation: QuantizeVariation,
    ) {
        for note in QuantizeNote::ALL {
            let setting = QuantizeSetting { note, variation };
            if ui
                .selectable_value(selected, setting, setting.label())
                .clicked()
            {
                ui.close();
            }
        }
    }
    egui::Popup::menu(&quantize).show(|ui| {
        theme::menu_popup_style(ui.style_mut());
        note_options(ui, &mut view.quantize, QuantizeVariation::Straight);
        ui.separator();
        ui.menu_button("Triplet", |ui| {
            note_options(ui, &mut view.quantize, QuantizeVariation::Triplet)
        });
        ui.menu_button("Dotted", |ui| {
            note_options(ui, &mut view.quantize, QuantizeVariation::Dotted)
        });
    });
}

fn toolbar_icon_button(
    ui: &mut egui::Ui,
    icon: ToolbarIcon,
    tooltip: &str,
    selected: bool,
    enabled: bool,
) -> egui::Response {
    toolbar_icon_button_tinted(ui, icon, tooltip, selected, enabled, None)
}

fn toolbar_icon_button_tinted(
    ui: &mut egui::Ui,
    icon: ToolbarIcon,
    tooltip: &str,
    selected: bool,
    enabled: bool,
    selected_color: Option<Color32>,
) -> egui::Response {
    let enabled = enabled && ui.is_enabled();
    let width = match icon {
        ToolbarIcon::Previous | ToolbarIcon::Next => 18.0,
        ToolbarIcon::Cycle | ToolbarIcon::Stop | ToolbarIcon::Play | ToolbarIcon::Record => 34.0,
        ToolbarIcon::Inspector
        | ToolbarIcon::InfoLine
        | ToolbarIcon::Overview
        | ToolbarIcon::Pool
        | ToolbarIcon::Mixer => 22.0,
        ToolbarIcon::RootKey => 54.0,
        ToolbarIcon::Autoscroll => 34.0,
        ToolbarIcon::SnapType => 80.0,
        ToolbarIcon::Select
        | ToolbarIcon::Range
        | ToolbarIcon::Split
        | ToolbarIcon::Glue
        | ToolbarIcon::Erase
        | ToolbarIcon::Zoom
        | ToolbarIcon::Mute
        | ToolbarIcon::TimeWarp
        | ToolbarIcon::Draw
        | ToolbarIcon::Line
        | ToolbarIcon::PlayTool
        | ToolbarIcon::Color => 20.0,
        _ => 25.0,
    };
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 25.0), sense);
    let p = theme::palette(ui);
    let normal_fill = if selected {
        selected_color.unwrap_or(if matches!(icon, ToolbarIcon::Record) {
            theme::RECORD
        } else {
            p.accent
        })
    } else {
        theme::toolbar_button_fill(ui.visuals())
    };
    let fill = theme::paint_control(ui, rect, &response, normal_fill, enabled, false);
    let color = if !enabled {
        p.muted.gamma_multiply(0.62)
    } else if selected {
        theme::control_ink(fill, true)
    } else if matches!(icon, ToolbarIcon::Record) {
        theme::RECORD
    } else {
        theme::control_ink(fill, true)
    };
    crate::icons::paint_toolbar(ui, rect, icon, color);
    if tooltip.is_empty() {
        response
    } else {
        response.on_hover_text(tooltip)
    }
}

fn toolbar_window_toggle(
    ui: &mut egui::Ui,
    icon: ToolbarIcon,
    window: &mut NativeWindowState,
    open_tooltip: &str,
    close_tooltip: &str,
) -> egui::Response {
    let response = toolbar_icon_button(
        ui,
        icon,
        if window.open {
            close_tooltip
        } else {
            open_tooltip
        },
        window.open,
        true,
    );
    if response.clicked() {
        window.open = !window.open;
        window.focus_requested = window.open;
    }
    response
}

fn disabled_menu_commands(ui: &mut egui::Ui, commands: &[(&str, &str)]) {
    for (label, shortcut) in commands {
        menu_command(ui, label, shortcut, false);
    }
}

#[derive(Default)]
struct ProjectHistory {
    names: media_names::Names,
    undo: VecDeque<HistoryEntry>,
    redo: VecDeque<HistoryEntry>,
    pending_recording: Option<recording_undo::RecordingUndo>,
    pending_deleted: Option<recording_undo::RecordingUndo>,
    last_error: Option<String>,
    saved_paths: HashSet<PathBuf>,
    unsaved_recordings: HashSet<PathBuf>,
    saved_recording_recovery: Vec<recording_undo::RecordingUndo>,
    pending: Option<Vec<u8>>,
    skip_next_commit: bool,
    commit_after_frame: bool,
}

struct HistoryEntry {
    snapshot: Vec<u8>,
    recording: Option<recording_undo::RecordingUndo>,
    deleted: Option<recording_undo::RecordingUndo>,
}
impl HistoryEntry {
    fn len(&self) -> usize {
        self.snapshot.len()
    }
}

impl ProjectHistory {
    fn retain_analysis(&mut self, track_id: u64, clip: &opentrack_core::project::AudioClip) {
        let names = &self.names;
        let attach = |snapshot: &mut Vec<u8>| {
            let Ok(mut project) = serde_json::from_slice::<Project>(snapshot) else {
                return;
            };
            if let Some(c) = project
                .tracks
                .iter_mut()
                .find(|t| t.id == track_id)
                .and_then(|t| t.clips.iter_mut().find(|c| c.id == clip.id))
                && c.pitch_time.is_none()
                && names.same_source(&c.source_path, &clip.source_path)
                && c.source_offset_seconds == clip.source_offset_seconds
                && c.duration_seconds == clip.duration_seconds
            {
                c.pitch_time = clip.pitch_time.clone();
                if let Ok(bytes) = serde_json::to_vec(&project) {
                    *snapshot = bytes;
                }
            }
        };
        if let Some(snapshot) = &mut self.pending {
            attach(snapshot);
        }
        for entry in self.undo.iter_mut().chain(self.redo.iter_mut()) {
            attach(&mut entry.snapshot);
        }
    }
    const MAX_ENTRIES: usize = 32;
    const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
    const MAX_TOTAL_BYTES: usize = 128 * 1024 * 1024;

    fn begin(&mut self, project: &Project) -> bool {
        if self.pending.is_some() {
            return true;
        }
        let Ok(snapshot) = serde_json::to_vec(project) else {
            return false;
        };
        if snapshot.len() > Self::MAX_SNAPSHOT_BYTES {
            return false;
        }
        self.pending = Some(snapshot);
        true
    }

    fn finish_frame(&mut self, project: &Project, interaction_finished: bool) -> bool {
        if self.skip_next_commit {
            self.pending = None;
            self.pending_recording = None;
            self.pending_deleted = None;
            self.skip_next_commit = false;
            self.commit_after_frame = false;
            return false;
        }
        if !interaction_finished && !self.commit_after_frame {
            return false;
        }
        self.commit_after_frame = false;
        let Some(before) = self.pending.take() else {
            return false;
        };
        let Ok(after) = serde_json::to_vec(project) else {
            return false;
        };
        if before == after || after.len() > Self::MAX_SNAPSHOT_BYTES {
            return false;
        }
        while let Some(entry) = self.redo.pop_back() {
            self.dispose_entry(entry);
        }
        self.undo.push_back(HistoryEntry {
            snapshot: before,
            recording: self.pending_recording.take(),
            deleted: self.pending_deleted.take(),
        });
        self.trim();
        true
    }

    fn undo(&mut self, current: &Project) -> Option<Project> {
        self.last_error = None;
        let mut previous = self.undo.pop_back()?;
        let current = match serde_json::to_vec(current) {
            Ok(snapshot) if snapshot.len() <= Self::MAX_SNAPSHOT_BYTES => snapshot,
            _ => {
                self.undo.push_back(previous);
                return None;
            }
        };
        let restored = match serde_json::from_slice(&previous.snapshot) {
            Ok(project) => project,
            Err(_) => {
                self.undo.push_back(previous);
                return None;
            }
        };
        if let Some(deleted) = previous.deleted.as_mut() {
            if let Err(error) = deleted.restore() {
                self.last_error = Some(format!("Could not undo media deletion: {error}"));
                self.undo.push_back(previous);
                return None;
            }
        }
        if let Some(recording) = previous.recording.as_mut() {
            if let Err(error) = self.names.restore(Some(&restored)) {
                self.last_error = Some(error);
                self.undo.push_back(previous);
                return None;
            }
            if let Err(error) = recording.remove(&restored) {
                self.last_error = Some(format!("Could not undo recording: {error}"));
                self.undo.push_back(previous);
                return None;
            }
        }
        if let Err(error) = self.names.restore(Some(&restored)) {
            self.last_error = Some(error);
            self.undo.push_back(previous);
            return None;
        }
        self.redo.push_back(HistoryEntry {
            snapshot: current,
            recording: previous.recording.take(),
            deleted: previous.deleted.take(),
        });
        self.trim();
        Some(restored)
    }

    fn redo(&mut self, current: &Project) -> Option<Project> {
        self.last_error = None;
        let mut next = self.redo.pop_back()?;
        let current = match serde_json::to_vec(current) {
            Ok(snapshot) if snapshot.len() <= Self::MAX_SNAPSHOT_BYTES => snapshot,
            _ => {
                self.redo.push_back(next);
                return None;
            }
        };
        let restored = match serde_json::from_slice(&next.snapshot) {
            Ok(project) => project,
            Err(_) => {
                self.redo.push_back(next);
                return None;
            }
        };
        if let Some(deleted) = next.deleted.as_mut() {
            if let Err(error) = deleted.remove(&restored) {
                self.last_error = Some(format!("Could not redo media deletion: {error}"));
                self.redo.push_back(next);
                return None;
            }
        }
        if let Some(recording) = next.recording.as_mut() {
            if let Err(error) = recording.restore() {
                self.last_error = Some(format!("Could not redo recording: {error}"));
                self.redo.push_back(next);
                return None;
            }
        }
        if let Err(error) = self.names.restore(Some(&restored)) {
            self.last_error = Some(error);
            self.redo.push_back(next);
            return None;
        }
        self.undo.push_back(HistoryEntry {
            snapshot: current,
            recording: next.recording.take(),
            deleted: next.deleted.take(),
        });
        self.trim();
        Some(restored)
    }

    fn dispose_entry(&mut self, mut entry: HistoryEntry) {
        for value in [&mut entry.recording, &mut entry.deleted] {
            if value
                .as_ref()
                .is_some_and(|r| r.protects_saved(&self.saved_paths))
            {
                self.saved_recording_recovery.push(value.take().unwrap());
            }
        }
    }
    fn saved(&mut self, project: &Project) {
        self.unsaved_recordings.clear();
        self.saved_paths = project.media_paths().into_iter().collect();
        self.names.saved(&mut self.saved_paths);
        self.saved_recording_recovery
            .retain(|r| r.protects_saved(&self.saved_paths));
    }
    fn restore_saved_recordings(&mut self) -> Result<(), String> {
        for entry in self.undo.iter_mut().chain(self.redo.iter_mut()) {
            for r in [&mut entry.recording, &mut entry.deleted]
                .into_iter()
                .filter_map(Option::as_mut)
                .filter(|r| r.protects_saved(&self.saved_paths))
            {
                r.restore()?;
            }
        }
        for r in &mut self.saved_recording_recovery {
            r.restore()?;
        }
        Ok(())
    }
    fn reserved_recording_paths(&self) -> HashSet<PathBuf> {
        let mut paths = self
            .saved_paths
            .union(&self.unsaved_recordings)
            .cloned()
            .collect::<HashSet<_>>();
        for r in self
            .undo
            .iter()
            .chain(self.redo.iter())
            .flat_map(|e| e.recording.iter().chain(e.deleted.iter()))
            .chain(self.saved_recording_recovery.iter())
        {
            paths.extend(r.paths().iter().cloned());
        }
        paths
    }
    fn discard_recordings(&mut self) -> Result<(), String> {
        self.restore_saved_recordings()?;
        self.names.restore(None)?;
        let paths = self
            .unsaved_recordings
            .iter()
            .filter(|p| !self.saved_paths.contains(*p) && p.exists())
            .cloned()
            .collect::<Vec<_>>();
        if let Some(first) = paths.first() {
            let parent = first.parent().ok_or("recording path has no parent")?;
            let root = if parent.file_name().is_some_and(|n| n == "Recordings") {
                parent
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("Metadata").join("Undo"))
                    .ok_or("recording project root is unavailable")?
            } else {
                parent.join(".opentrack-undo")
            };
            let mut cleanup = recording_undo::RecordingUndo::new(paths, root);
            cleanup.remove(&Project::default())?;
        }
        self.unsaved_recordings.clear();
        Ok(())
    }
    fn clear(&mut self) {
        self.names = Default::default();
        self.unsaved_recordings.clear();
        self.saved_recording_recovery.clear();
        self.saved_paths.clear();
        self.undo.clear();
        self.redo.clear();
        self.pending_recording = None;
        self.pending_deleted = None;
        self.last_error = None;
        self.pending = None;
        self.skip_next_commit = false;
        self.commit_after_frame = false;
    }

    fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn trim(&mut self) {
        let mut total_bytes = self
            .undo
            .iter()
            .chain(self.redo.iter())
            .map(HistoryEntry::len)
            .sum::<usize>();
        while total_bytes > Self::MAX_TOTAL_BYTES
            || self.undo.len() + self.redo.len() > Self::MAX_ENTRIES
        {
            let removed = if let Some(snapshot) = self.undo.pop_front() {
                snapshot
            } else if let Some(snapshot) = self.redo.pop_front() {
                snapshot
            } else {
                break;
            };
            total_bytes = total_bytes.saturating_sub(removed.len());
            self.dispose_entry(removed);
        }
    }
}

enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

impl OpenTrackApp {
    pub fn new(creation: &eframe::CreationContext<'_>) -> Self {
        Self::new_internal(creation, true)
    }

    fn new_internal(creation: &eframe::CreationContext<'_>, open_audio: bool) -> Self {
        theme::install(&creation.egui_ctx);
        let dock_state = creation
            .storage
            .and_then(|storage| eframe::get_value(storage, "dock-state"))
            .unwrap_or_else(default_dock_state);
        let workspace_preferences = creation
            .storage
            .and_then(|storage| {
                eframe::get_value::<WorkspacePreferences>(storage, "workspace-preferences-v1")
            })
            .unwrap_or_default();
        let mut workspace_state = initial_workspace_state(&dock_state)
            .expect("the built-in dock layout must be serializable");
        let startup_instantiated_presets =
            workspace_state.instantiate_global_presets(&workspace_preferences);
        let mut startup_messages = Vec::new();
        if startup_instantiated_presets > 0 {
            startup_messages.push(format!(
                "Loaded {startup_instantiated_presets} workspace preset(s)"
            ));
        }
        let audio_inventory = scan_audio_devices();
        let audio_devices = audio_inventory.devices;
        let audio_device_diagnostics = audio_inventory.diagnostics;
        if audio_devices.is_empty() {
            startup_messages.push(if audio_device_diagnostics.is_empty() {
                "No audio endpoints were reported by the available backends".to_owned()
            } else {
                format!(
                    "Audio endpoint scan: {}",
                    audio_device_diagnostics.join("; ")
                )
            });
        }
        let audio_input_devices = audio_devices
            .iter()
            .filter(|device| device.supports_input)
            .map(|device| AudioInputDevice {
                id: device.id.clone(),
                name: device.name.clone(),
            })
            .collect::<Vec<_>>();
        let audio_output_devices = audio_devices
            .iter()
            .filter(|device| device.supports_output)
            .cloned()
            .collect::<Vec<_>>();
        let saved_input_device = creation
            .storage
            .and_then(|storage| eframe::get_value::<Option<String>>(storage, "audio-input-device"))
            .flatten();
        let selected_input_device = saved_input_device.clone();
        if let Some(saved_id) = selected_input_device.as_ref()
            && !audio_input_devices
                .iter()
                .any(|device| &device.id == saved_id)
        {
            startup_messages.push(
                "Saved audio input is unavailable; the saved endpoint remains selected".to_owned(),
            );
        }
        let (cached_input_format, audio_input_format_error) =
            match AudioRecorder::input_format(selected_input_device.as_deref()) {
                Ok(format) => (Some(format), None),
                Err(error) => {
                    startup_messages.push(format!("Audio input unavailable: {error}"));
                    (None, Some(error))
                }
            };
        let resolved_input_device_name =
            AudioRecorder::input_device_name(selected_input_device.as_deref()).ok();
        let saved_output_device = creation
            .storage
            .and_then(|storage| eframe::get_value::<Option<String>>(storage, "audio-output-device"))
            .flatten();
        let selected_output_device = saved_output_device.clone();
        let saved_asio = selected_output_device
            .as_deref()
            .is_some_and(|id| id.starts_with("asio:"));
        if saved_output_device
            .as_ref()
            .is_some_and(|id| !audio_output_devices.iter().any(|device| &device.id == id))
        {
            startup_messages
                .push("Saved audio output is unavailable; it remains selected".to_owned());
        }
        let mut audio_output_configs =
            match output_configurations(selected_output_device.as_deref()) {
                Ok(configs) => configs,
                Err(error) => {
                    startup_messages
                        .push(format!("Audio output configurations unavailable: {error}"));
                    Vec::new()
                }
            };
        let mut audio_output_sample_rates = audio_sample_rate_choices(&audio_output_configs);
        let saved_output_sample_rate = creation
            .storage
            .and_then(|storage| {
                eframe::get_value::<Option<u32>>(storage, "audio-output-sample-rate")
            })
            .flatten();
        let mut selected_output_sample_rate = saved_output_sample_rate
            .filter(|rate| saved_asio || audio_output_sample_rates.contains(rate));
        if saved_output_sample_rate.is_some() && selected_output_sample_rate.is_none() {
            startup_messages.push(
                "Saved output sample rate is unavailable; using the device default".to_owned(),
            );
        }
        let mut audio_output_buffer_choices =
            audio_buffer_choices(&audio_output_configs, selected_output_sample_rate);
        let saved_output_buffer_frames = creation
            .storage
            .and_then(|storage| {
                eframe::get_value::<Option<u32>>(storage, "audio-output-buffer-frames")
            })
            .flatten();
        let mut selected_output_buffer_frames = saved_output_buffer_frames
            .filter(|frames| saved_asio || audio_output_buffer_choices.contains(frames));
        if saved_output_buffer_frames.is_some() && selected_output_buffer_frames.is_none() {
            startup_messages.push(
                "Saved output buffer size is unavailable; using the device default".to_owned(),
            );
        }
        let audio_device_mode = AudioDeviceMode::from_preference(
            creation
                .storage
                .and_then(|storage| eframe::get_value::<u8>(storage, "audio-device-mode-v1"))
                .unwrap_or(0),
        );
        if !saved_asio
            && audio_device_mode == AudioDeviceMode::SharedLowLatency
            && selected_output_sample_rate.take().is_some()
        {
            startup_messages.push(
                "Shared low-latency mode follows the Windows output mix rate; the saved custom rate was cleared".to_owned(),
            );
        }
        let release_exclusive_audio_when_idle = creation
            .storage
            .and_then(|storage| {
                eframe::get_value::<bool>(storage, "release-exclusive-audio-when-idle")
            })
            .unwrap_or(false);
        let midi_input_devices = match MidiInputStream::available_devices() {
            Ok(devices) => devices,
            Err(error) => {
                startup_messages.push(format!("MIDI inputs unavailable: {error}"));
                Vec::new()
            }
        };
        let saved_midi_input = creation
            .storage
            .and_then(|storage| eframe::get_value::<Option<String>>(storage, "midi-input-device"))
            .flatten();
        let mut selected_midi_input_device = saved_midi_input.clone().filter(|saved_id| {
            midi_input_devices
                .iter()
                .any(|device| &device.id == saved_id)
        });
        let mut midi_input_stream = None;
        if let Some(saved_id) = selected_midi_input_device.as_deref() {
            match MidiInputStream::open(saved_id) {
                Ok(stream) => midi_input_stream = Some(stream),
                Err(error) => {
                    startup_messages.push(format!("Saved MIDI input could not open: {error}"));
                    selected_midi_input_device = None;
                }
            }
        }
        if saved_midi_input.is_some() && selected_midi_input_device.is_none() {
            startup_messages.push("Saved MIDI input is unavailable".to_owned());
        }
        let input_monitoring = creation
            .storage
            .and_then(|storage| eframe::get_value::<bool>(storage, "audio-input-monitoring"))
            .unwrap_or(false);
        let midi_thru_enabled = creation
            .storage
            .and_then(|storage| eframe::get_value::<bool>(storage, "midi-thru-enabled"))
            .unwrap_or(false);
        let midi_position_quantize_beats = creation
            .storage
            .and_then(|storage| eframe::get_value::<f64>(storage, "midi-position-quantize"))
            .filter(|value| is_midi_quantize_value(*value))
            .unwrap_or(1.0);
        let midi_length_quantize_beats = creation
            .storage
            .and_then(|storage| eframe::get_value::<f64>(storage, "midi-length-quantize"))
            .filter(|value| is_midi_quantize_value(*value))
            .unwrap_or(0.25);
        let midi_insert_velocity = creation
            .storage
            .and_then(|storage| eframe::get_value::<u8>(storage, "midi-insert-velocity"))
            .filter(|value| (1..=127).contains(value))
            .unwrap_or(96);
        let native_key_capture = match NativeKeyCapture::install(creation) {
            Ok(capture) => Some(capture),
            Err(error) => {
                startup_messages.push(format!("Physical keypad shortcuts unavailable: {error}"));
                None
            }
        };
        let audio_engine = if open_audio {
            match AudioEngine::open_output_device_mode(
                selected_output_device.as_deref(),
                selected_output_sample_rate,
                selected_output_buffer_frames,
                audio_device_mode,
            ) {
                Ok(engine) => Some(engine),
                Err(error) => {
                    startup_messages.push(format!(
                        "Saved audio output configuration could not open: {error}"
                    ));
                    None
                }
            }
        } else {
            None
        };
        if audio_device_mode == AudioDeviceMode::SharedLowLatency
            && let Some(engine) = audio_engine.as_ref()
            && engine.output_mode == AudioDeviceMode::SharedLowLatency
            && let Some(requested) = selected_output_buffer_frames
        {
            selected_output_buffer_frames = Some(
                engine
                    .output_period_choices
                    .iter()
                    .copied()
                    .min_by_key(|period| period.abs_diff(requested))
                    .unwrap_or(engine.output_period_frames),
            );
        }
        if selected_output_device.is_none() {
            audio_output_configs = output_configurations(None).unwrap_or_default();
            audio_output_sample_rates = audio_sample_rate_choices(&audio_output_configs);
            audio_output_buffer_choices = audio_buffer_choices(&audio_output_configs, None);
        }
        let saved_keymap_bindings = creation.storage.and_then(|storage| {
            eframe::get_value::<Vec<KeyBinding>>(storage, "reference-keymap-bindings-v1")
        });
        let keymap_profile = match KeymapProfile::windows_reference_default() {
            Ok(mut profile) => {
                if let Some(bindings) = saved_keymap_bindings {
                    let bindings = bindings
                        .into_iter()
                        .filter(|binding| profile.catalog.get(&binding.command).is_some())
                        .collect();
                    profile.keymap.replace_bindings(bindings);
                }
                Some(profile)
            }
            Err(error) => {
                startup_messages.push(format!("Reference keymap unavailable: {error}"));
                None
            }
        };
        let status_message = if startup_messages.is_empty() {
            "Ready".to_owned()
        } else {
            startup_messages.join(" · ")
        };
        let project_window = restore_project_window_state(
            workspace_state
                .active_index
                .and_then(|index| workspace_state.workspaces.get(index)),
            0,
        );
        let channel_settings_preferences = creation
            .storage
            .and_then(|storage| {
                eframe::get_value::<ChannelSettingsPreferences>(
                    storage,
                    "channel-settings-preferences-v1",
                )
            })
            .unwrap_or_default();
        let mut channel_settings_preferences = channel_settings_preferences;
        if channel_settings_preferences.version < CHANNEL_SETTINGS_PREFERENCES_VERSION {
            // Channel Settings starts pinned above its owner.
            channel_settings_preferences.always_on_top = true;
            channel_settings_preferences.version = CHANNEL_SETTINGS_PREFERENCES_VERSION;
        }
        let vst3_plugin_folders = creation
            .storage
            .and_then(|storage| {
                eframe::get_value::<Vec<PathBuf>>(storage, "vst3-plugin-folders-v1")
            })
            .unwrap_or_else(default_vst3_plugin_folders);
        let mut vst3_plugin_catalog = creation
            .storage
            .and_then(|storage| {
                eframe::get_value::<Vec<PluginDescriptor>>(storage, "vst3-plugin-catalog-v1")
            })
            .unwrap_or_default();
        sort_vst3_plugin_catalog(&mut vst3_plugin_catalog);
        let plugin_scan_status = if vst3_plugin_catalog.is_empty() {
            "No VST3 plug-ins are cached. Add a folder and scan.".to_owned()
        } else {
            format!(
                "{} cached VST3 plug-in(s). Rescan to refresh the catalog.",
                vst3_plugin_catalog.len()
            )
        };

        let mut app = Self {
            dock_state,
            workspace_state,
            arrange_view_state: ArrangeViewState::default(),
            audio_event_clipboard: None,
            project_window,
            workspace_preferences,
            data: AppData::default(),
            last_meter_update: None,
            metering_preferences: {
                let mut value = creation
                    .storage
                    .and_then(|s| {
                        eframe::get_value::<metering::Preferences>(s, "metering-preferences-v1")
                    })
                    .unwrap_or_default();
                value.sanitize();
                value
            },
            preferences_ui: preferences::Window::default(),
            sample_editor: {
                let mut state = sample_editor::State::default();
                state.snap = creation
                    .storage
                    .and_then(|storage| eframe::get_value(storage, "sample-editor-snap-v1"))
                    .unwrap_or_default();
                state.pitch_view = creation
                    .storage
                    .and_then(|storage| eframe::get_value(storage, "sample-editor-pitch-view-v1"))
                    .unwrap_or_default();
                state.pitch_view.sanitize();
                state
            },
            midi_pitch_view: {
                let mut view: sample_editor::PitchViewport = creation
                    .storage
                    .and_then(|storage| eframe::get_value(storage, "midi-editor-pitch-view-v1"))
                    .unwrap_or_default();
                view.sanitize();
                view
            },
            sample_editor_active: false,
            active_note_audition: None,
            note_audition_stop_at: None,
            vst3_runtime: vst3_host::Runtime::default(),
            vst3_plugin_folders,
            vst3_plugin_catalog,
            plugin_information_open: false,
            plugin_scan_task: None,
            plugin_scan_report: None,
            plugin_scan_status,
            plugin_scan_error: None,
            root_window_handle: crate::native_windows::creation_root(creation),
            startup_visibility_pending: true,
            vst3_parameter_window: None,
            vst3_editor_window: None,
            vst3_editor_host: None,
            vst3_editor_bounds: None,
            main_window_bounds: None,
            main_window_restore_bounds: None,
            show_transport: true,
            transport_window: TransportWindowState {
                bounds: creation
                    .storage
                    .and_then(|storage| eframe::get_value(storage, "transport-window-bounds-v1")),
                viewport_live: false,
                always_on_top: creation
                    .storage
                    .and_then(|storage| {
                        eframe::get_value(storage, "transport-window-always-on-top-v1")
                    })
                    .unwrap_or(false),
            },
            metronome_enabled: creation
                .storage
                .and_then(|s| eframe::get_value(s, "metronome-enabled"))
                .unwrap_or(false),
            metronome_level: creation
                .storage
                .and_then(|s| eframe::get_value::<f32>(s, "metronome-level"))
                .filter(|v| v.is_finite())
                .unwrap_or(0.35)
                .clamp(0.0, 1.0),
            metronome_pitch_midi: creation
                .storage
                .and_then(|s| eframe::get_value::<u8>(s, "metronome-pitch-midi"))
                .unwrap_or(87)
                .clamp(24, 108),
            metronome_settings_open: false,
            tempo_signature_settings_open: false,
            edit_tool: EditTool::Select,
            keymap_profile,
            native_key_capture,
            key_commands_open: false,
            key_command_category: None,
            key_capture_target: None,
            pending_key_assignment: None,
            audio_engine,
            audio_devices,
            audio_input_devices,
            audio_output_devices,
            audio_output_configs,
            audio_output_sample_rates,
            audio_output_buffer_choices,
            audio_device_diagnostics,
            cached_input_format,
            resolved_input_device_name,
            audio_input_format_error,
            audio_input_probe: None,
            audio_input_probe_status: None,
            device_test_active: false,
            device_setup_open: false,
            menu_open: false,
            device_setup_page: 0,
            vst_connections_open: false,
            vst_connections_tab: 0,
            project_setup_open: false,
            project_rate_change: None,
            media_operation_error: None,
            selected_input_device,
            selected_output_device,
            selected_output_sample_rate,
            selected_output_buffer_frames,
            audio_device_mode,
            release_exclusive_audio_when_idle,
            input_monitoring,
            midi_input_devices,
            selected_midi_input_device,
            midi_input_stream,
            midi_event_scratch: Vec::with_capacity(128),
            midi_reported_dropped_messages: 0,
            midi_thru_enabled,
            midi_editor_open: false,
            channel_settings_window: NativeWindowState::default(),
            channel_settings_ui: ChannelSettingsUiState {
                preferences: channel_settings_preferences,
                preset_name_draft: "Default".to_owned(),
                ..Default::default()
            },
            channel_window_owner: crate::native_windows::ChannelOwner::new(creation),
            mixer_window: NativeWindowState {
                open: true,
                ..NativeWindowState::default()
            },
            mixer_track_control_cache: Vec::new(),
            media_window: NativeWindowState::default(),
            track_rename_dialog: None,
            track_add_dialog: None,
            event_colors_dialog: None,
            pending_track_delete_id: None,
            midi_position_quantize_beats,
            midi_length_quantize_beats,
            midi_insert_velocity,
            midi_controller_number: 11,
            selected_midi_note_id: None,
            selected_midi_note_ids: BTreeSet::new(),
            midi_note_drag: None,
            midi_note_draw: None,
            midi_part_draw: None,
            midi_controller_drag: None,
            midi_pending_audio_rebuild: false,
            project_history: ProjectHistory::default(),
            recording_session: None,
            audio_import_task: None,
            project_load_task: None,
            midi_recording_session: None,
            project_path: None,
            native_window_title: None,
            project_open: false,
            project_folder_managed: false,
            project_folder_setup: None,
            saved_project_fingerprint: Vec::new(),
            saved_workspace_fingerprint: Vec::new(),
            modified: false,
            pending_project_action: None,
            workspace_organizer_open: false,
            about_open: false,
            confirmed_close_request: false,
            status_message,
            realtime_diagnostic_details: String::new(),
            realtime_diagnostic_refreshed_at: None,
            realtime_diagnostics_open: false,
            project_open_error: None,
        };
        if app
            .selected_output_device
            .as_deref()
            .is_some_and(|id| id.starts_with("asio:"))
        {
            app.selected_input_device = app.selected_output_device.clone();
        }
        app.refresh_audio_input_format();
        app.refresh_audio_output_configurations();
        app.mark_saved();
        app
    }

    fn project_file(&self) -> Result<ProjectFile, ProjectFileError> {
        let mut document = ProjectFile::new(
            self.data.project.clone(),
            self.workspace_state_for_save()?,
            self.dock_state.clone(),
            self.show_transport,
            self.data.selected_media,
            self.data.transport.cycle,
        );
        document.project_folder_layout = u8::from(self.project_folder_managed);
        Ok(document)
    }

    fn project_window_title(&self) -> String {
        if !self.project_open && self.project_path.is_none() {
            return "OpenTrack-".to_owned();
        }
        let name = self
            .project_path
            .as_deref()
            .and_then(Path::file_stem)
            .map(|stem| stem.to_string_lossy().into_owned())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| self.data.project.name.clone());
        format!("Project- {name}")
    }

    fn workspace_state_for_save(&self) -> Result<ProjectWorkspaceState, ProjectFileError> {
        let mut workspace_state = self.workspace_state.clone();
        let active_index = workspace_state.active_index.ok_or_else(|| {
            ProjectFileError::InvalidData("project has no active workspace".to_owned())
        })?;
        let active_workspace = workspace_state
            .workspaces
            .get(active_index)
            .ok_or_else(|| {
                ProjectFileError::InvalidData("active workspace index is invalid".to_owned())
            })?;
        if !active_workspace.locked {
            Self::store_active_workspace_windows(
                &mut workspace_state,
                &self.dock_state,
                self.main_window_bounds.clone(),
                &self.arrange_view_state,
                &self.project_window,
                self.midi_editor_open,
                self.vst3_editor_bounds.clone(),
                &self.channel_settings_window,
                &self.mixer_window,
                &self.media_window,
            )?;
        }
        Ok(workspace_state)
    }

    fn store_active_workspace_windows(
        workspace_state: &mut ProjectWorkspaceState,
        dock_state: &DockState<PanelTab>,
        main_window_bounds: Option<WindowBounds>,
        arrange_view_state: &ArrangeViewState,
        project_window: &ProjectWindowState,
        midi_editor_open: bool,
        vst3_editor_bounds: Option<WindowBounds>,
        channel_settings_window: &NativeWindowState,
        mixer_window: &NativeWindowState,
        media_window: &NativeWindowState,
    ) -> Result<(), ProjectFileError> {
        store_active_dock_state(
            workspace_state,
            dock_state,
            main_window_bounds,
            arrange_view_state,
        )?;
        store_active_workspace_child_state(
            workspace_state,
            PROJECT_WINDOW_ID,
            WindowClass::ProjectSpecific,
            project_window.open,
            project_window.bounds.clone(),
            project_window.restore_bounds.clone(),
            project_window.z_order,
            project_window.minimized,
            project_window.maximized,
        )?;
        store_active_workspace_child_state(
            workspace_state,
            MIXER_WINDOW_ID,
            WindowClass::ProjectSpecific,
            mixer_window.open,
            mixer_window.bounds.clone(),
            None,
            0,
            false,
            false,
        )?;
        store_active_project_window_state(workspace_state, "midi-editor", midi_editor_open, None)?;
        store_active_project_window_state(
            workspace_state,
            "vst3-editor",
            false,
            vst3_editor_bounds,
        )?;
        for (id, window) in [
            (CHANNEL_SETTINGS_WINDOW_ID, channel_settings_window),
            (MEDIA_WINDOW_ID, media_window),
        ] {
            store_active_project_window_state(
                workspace_state,
                id,
                window.open,
                window.bounds.clone(),
            )?;
        }
        Ok(())
    }

    fn workspace_fingerprint(&self) -> Result<Vec<u8>, ProjectFileError> {
        let mut workspace_state = self.workspace_state_for_save()?;
        crate::project_file::normalize_workspace_for_dirty_check(&mut workspace_state)?;
        Ok(serde_json::to_vec(&(
            workspace_state,
            &self.dock_state,
            self.show_transport,
            self.data.selected_media,
            self.data.transport.cycle,
        ))?)
    }

    fn has_unsaved_changes(&self) -> bool {
        self.is_recording()
            || self.modified
            || (self.project_path.is_some()
                && self
                    .workspace_fingerprint()
                    .map(|fingerprint| fingerprint != self.saved_workspace_fingerprint)
                    .unwrap_or(true))
    }

    fn mark_modified(&mut self) {
        self.modified = true;
        self.status_message = "Modified".to_owned();
    }

    fn sync_arrange_track_selection(&mut self) {
        let Some(track_id) = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize))
            .map(|track| track.id)
        else {
            return;
        };
        if self.arrange_view_state.selected_track_ids.len() == 1
            && self
                .arrange_view_state
                .selected_track_ids
                .contains(&track_id)
        {
            return;
        }
        self.arrange_view_state.selected_track_ids.clear();
        self.arrange_view_state.selected_track_ids.insert(track_id);
        self.arrange_view_state.selection_anchor_track_id = Some(track_id);
    }

    fn mark_saved(&mut self) {
        self.modified = false;
        self.saved_project_fingerprint = serde_json::to_vec(&self.data.project).unwrap_or_default();
        self.project_history.saved(&self.data.project);
        self.saved_workspace_fingerprint = self.workspace_fingerprint().unwrap_or_default();
    }

    fn refresh_modified_from_saved(&mut self) {
        self.modified = serde_json::to_vec(&self.data.project)
            .map(|fingerprint| fingerprint != self.saved_project_fingerprint)
            .unwrap_or(true);
        self.status_message = if self.modified {
            "Modified".to_owned()
        } else {
            "Restored saved project".to_owned()
        };
    }

    fn is_recording(&self) -> bool {
        self.recording_session.is_some() || self.midi_recording_session.is_some()
    }

    fn undo_project(&mut self) {
        if self.is_recording() {
            return;
        }

        if let Some(project) = self.project_history.undo(&self.data.project) {
            self.project_history.skip_next_commit = true;
            self.data.project = project;
            self.selected_midi_note_id = None;
            self.midi_note_drag = None;
            self.midi_note_draw = None;
            self.rebuild_audio_plan();
            self.refresh_modified_from_saved();
            self.status_message = "Undid project edit".to_owned();
        } else if let Some(error) = self.project_history.last_error.take() {
            self.media_operation_error = Some(error.clone());
            self.status_message = error;
        }
    }

    fn redo_project(&mut self) {
        if self.is_recording() {
            return;
        }

        if let Some(project) = self.project_history.redo(&self.data.project) {
            self.project_history.skip_next_commit = true;
            self.data.project = project;
            self.selected_midi_note_id = None;
            self.midi_note_drag = None;
            self.midi_note_draw = None;
            self.rebuild_audio_plan();
            self.refresh_modified_from_saved();
            self.status_message = "Redid project edit".to_owned();
        } else if let Some(error) = self.project_history.last_error.take() {
            self.media_operation_error = Some(error.clone());
            self.status_message = error;
        }
    }

    fn request_new_project(&mut self) {
        if self.audio_import_task.is_some() {
            self.status_message =
                "Wait for audio import to finish before changing projects".to_owned();
            return;
        }
        let Some(parent) = FileDialog::new()
            .set_title("Choose where to create the new project folder")
            .pick_folder()
        else {
            return;
        };
        self.request_project_action(PendingProjectAction::New(parent));
    }

    fn request_open_project(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("OpenTrack Projects", &["opr", "otp", "rwp"])
            .set_title("Open OpenTrack Project")
            .pick_file()
        else {
            return;
        };
        self.request_project_action(PendingProjectAction::Open(path));
    }

    fn import_audio_file(&mut self) {
        if !self.project_open {
            self.status_message = "Create or open a project before importing audio".to_owned();
            return;
        }
        if self.audio_import_task.is_some() {
            self.status_message = "Wait for the current audio import to finish".to_owned();
            return;
        }
        if !self.project_folder_managed {
            self.begin_project_folder_setup();
            self.status_message = "Set up this project's folder before importing audio".to_owned();
            return;
        }
        let Some(path) = FileDialog::new()
            .add_filter("WAV audio", &["wav", "wave"])
            .set_title("Import WAV Audio")
            .pick_file()
        else {
            return;
        };
        let Some(project_root) = self
            .project_path
            .as_deref()
            .and_then(Path::parent)
            .filter(|path| !path.as_os_str().is_empty())
        else {
            self.status_message =
                "Save this project into its folder before importing audio".to_owned();
            return;
        };
        let (destination, copied) = match audio_import_destination(&path, project_root) {
            Ok(destination) => destination,
            Err(error) => {
                self.status_message = format!("Could not prepare project audio folder: {error}");
                return;
            }
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        let import_source = path.clone();
        let worker_destination = destination.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("opentrack-project-audio-import".to_owned())
            .spawn(move || {
                let result = (|| {
                    if copied {
                        let temporary = audio_import_temporary_path(&worker_destination);
                        if let Err(error) = fs::copy(&import_source, &temporary) {
                            let _ = fs::remove_file(&temporary);
                            return Err(format!("could not copy audio into the project: {error}"));
                        }
                        if let Err(error) = fs::rename(&temporary, &worker_destination) {
                            let _ = fs::remove_file(&temporary);
                            return Err(format!("could not finish copying project audio: {error}"));
                        }
                    }
                    match AudioSource::open(&worker_destination) {
                        Ok(source) => Ok((worker_destination.clone(), source, copied)),
                        Err(error) => {
                            if copied {
                                let _ = fs::remove_file(&worker_destination);
                            }
                            Err(error)
                        }
                    }
                })();
                let _ = sender.send(result);
            })
        {
            self.status_message = format!("Could not start audio import: {error}");
            return;
        }
        self.audio_import_task = Some(AudioImportTask { result: receiver });
        self.status_message = if copied {
            format!("Copying {} into Audio/Imports…", project_file_name(&path))
        } else {
            format!("Loading {}…", project_file_name(&path))
        };
    }

    fn poll_audio_import_task(&mut self) {
        let result = match self.audio_import_task.as_ref() {
            Some(task) => match task.result.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(
                    "the audio import worker stopped before returning a result".to_owned(),
                )),
            },
            None => return,
        };
        let Some(result) = result else {
            return;
        };
        self.audio_import_task = None;
        match result {
            Ok((path, source, copied)) => self.finish_audio_import(path, source, copied),
            Err(error) => self.status_message = format!("Could not import WAV: {error}"),
        }
    }

    fn finish_audio_import(&mut self, path: PathBuf, source: AudioSource, copied: bool) {
        let streams_from_disk = source.is_streamed();

        let selected_audio_track = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize))
            .is_some_and(|track| track.kind == TrackKind::Audio);
        let track_index = if selected_audio_track {
            self.data.project.selected_track.unwrap_or_default() as usize
        } else {
            self.data.project.add_track(TrackKind::Audio);
            self.data.project.tracks.len() - 1
        };
        let duration = source.duration_seconds();
        if self
            .data
            .project
            .add_audio_clip(
                track_index,
                path.clone(),
                self.data.transport.position_beats,
                duration,
            )
            .is_none()
        {
            self.status_message = "Could not add the WAV to an audio track".to_owned();
            return;
        }
        let clip_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "WAV audio".to_owned());
        self.data.audio_sources.insert(path, source);
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = if streams_from_disk {
            format!(
                "Imported {clip_name}; {}streaming from disk",
                if copied {
                    "copied into project and "
                } else {
                    ""
                }
            )
        } else {
            format!(
                "Imported {clip_name}{}",
                if copied { " into project audio" } else { "" }
            )
        };
    }

    fn relink_unavailable_audio(&mut self) {
        let Some(missing_path) =
            unresolved_audio_source_paths(&self.data.project, &self.data.audio_sources)
                .into_iter()
                .next()
        else {
            self.status_message = "All project audio sources are available".to_owned();
            return;
        };
        let suggested_name = missing_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "audio.wav".to_owned());
        let mut dialog = FileDialog::new()
            .add_filter("WAV audio", &["wav", "wave"])
            .set_title("Relink Audio Source")
            .set_file_name(suggested_name);
        if let Some(directory) = missing_path.parent().filter(|path| path.is_dir()) {
            dialog = dialog.set_directory(directory);
        }
        let Some(replacement_path) = dialog.pick_file() else {
            return;
        };
        let source = match AudioSource::open(&replacement_path) {
            Ok(source) => source,
            Err(error) => {
                self.status_message = format!("Could not relink audio: {error}");
                return;
            }
        };
        let streams_from_disk = source.is_streamed();

        if !self.project_history.begin(&self.data.project) {
            self.status_message = "Undo history is unavailable for this project size".to_owned();
        } else {
            self.project_history.commit_after_frame = true;
        }
        let relinked_clips =
            relink_audio_source_path(&mut self.data.project, &missing_path, &replacement_path);
        if relinked_clips == 0 {
            self.status_message = "The unavailable audio reference was already changed".to_owned();
            return;
        }
        self.data
            .audio_sources
            .insert(replacement_path.clone(), source);
        self.rebuild_audio_plan();
        self.mark_modified();
        let remaining_sources =
            unresolved_audio_source_paths(&self.data.project, &self.data.audio_sources).len();
        let replacement_name = project_file_name(&replacement_path);
        self.status_message = if remaining_sources == 0 {
            if streams_from_disk {
                format!(
                    "Relinked {relinked_clips} clip(s) to {replacement_name}; streaming from disk"
                )
            } else {
                format!("Relinked {relinked_clips} clip(s) to {replacement_name}")
            }
        } else {
            format!(
                "Relinked {relinked_clips} clip(s) to {replacement_name}; {remaining_sources} source(s) still need relinking"
            )
        };
    }

    fn rebuild_audio_plan(&mut self) {
        self.midi_pending_audio_rebuild = false;
        let tempo_bpm = self.data.project.tempo_bpm;
        let (host_rate, host_block) = self.audio_engine.as_ref().map_or(
            (self.data.project.sample_rate_hz.max(1), 512),
            |engine| {
                (
                    engine.sample_rate.max(1),
                    engine.output_buffer_frames.max(64),
                )
            },
        );
        if self.vst3_editor_window.is_some_and(|(track_id, slot)| {
            !self.vst3_runtime.matches_insert(
                &self.data.project,
                track_id,
                slot,
                host_rate,
                host_block,
            )
        }) {
            self.close_vst3_editor();
        }
        self.vst3_runtime.set_format(host_rate, host_block);
        self.vst3_runtime.sync(&self.data.project);
        let loop_range_beats = self.data.transport.cycle.then_some((
            self.data.project.loop_start_beats,
            self.data.project.loop_end_beats,
        ));
        let active_track_ids: HashSet<_> = self
            .data
            .project
            .tracks
            .iter()
            .map(|track| track.id)
            .collect();
        self.data
            .track_meter_peaks
            .retain(|track_id, _| active_track_ids.contains(track_id));
        self.data
            .track_meter_levels
            .retain(|track_id, _| active_track_ids.contains(track_id));
        let project_tracks = &self.data.project.tracks;
        let track_meter_peaks = &mut self.data.track_meter_peaks;
        let vst3_runtime = &self.vst3_runtime;
        let track_descriptors: Vec<_> = project_tracks
            .iter()
            .map(|track| {
                let mut effective_muted = track.muted;
                let mut effective_solo = track.solo;
                let mut effective_solo_defeat = track.solo_defeat;
                if track.kind == TrackKind::Group {
                    effective_solo |= project_tracks.iter().any(|child| {
                        project_track_is_in_group(&self.data.project, child.id, track.id)
                            && (child.solo || child.solo_defeat)
                    });
                }
                let mut parent_id = track.parent_group_id;
                for _ in 0..2 {
                    let Some(id) = parent_id else { break };
                    let Some(parent) = project_tracks.iter().find(|parent| parent.id == id) else {
                        break;
                    };
                    effective_muted |= parent.muted;
                    effective_solo |= parent.solo;
                    effective_solo_defeat |= parent.solo_defeat;
                    parent_id = parent.parent_group_id;
                }
                let meter_peak = Arc::clone(
                    track_meter_peaks
                        .entry(track.id)
                        .or_insert_with(|| Arc::new(opentrack_audio::StereoMeter::default())),
                );
                (
                    meter_peak,
                    track.kind == TrackKind::Group,
                    track.audio_channels,
                    track.gain,
                    track.pan,
                    track.track_delay_ms,
                    effective_muted,
                    effective_solo,
                    effective_solo_defeat,
                    track.equalizer.clone(),
                    track.inserts.clone(),
                    track.inserts_bypassed,
                    vst3_runtime.hosted_inserts(track),
                    track.instrument_waveform,
                    track.monitor || (self.input_monitoring && track.armed),
                    track.input_routing.clone(),
                    track.output_routing.clone(),
                    track
                        .clips
                        .iter()
                        .map(|clip| {
                            (
                                clip.id,
                                sample_editor::playback_path(clip).to_path_buf(),
                                clip.start_beats,
                                clip.gain,
                                clip.source_offset_seconds,
                                clip.duration_seconds,
                                clip.fade_in_seconds,
                                clip.fade_out_seconds,
                                clip.source_fades.clone(),
                                clip.pitch_time.clone(),
                            )
                        })
                        .collect::<Vec<_>>(),
                    track
                        .midi_parts
                        .iter()
                        .flat_map(|part| {
                            part.notes.iter().map(|note| {
                                (
                                    note.id,
                                    part.start_beats + note.start_beats,
                                    note.duration_beats,
                                    440.0_f64 * 2.0_f64.powf((f64::from(note.pitch) - 69.0) / 12.0),
                                    note.channel,
                                    f32::from(note.velocity) / 127.0,
                                )
                            })
                        })
                        .map(
                            |(id, start_beats, duration_beats, frequency_hz, channel, velocity)| {
                                PlaybackMidiNote {
                                    id,
                                    start_beats,
                                    duration_beats,
                                    frequency_hz: frequency_hz as f32,
                                    channel,
                                    velocity,
                                }
                            },
                        )
                        .collect::<Vec<_>>(),
                    track
                        .midi_parts
                        .iter()
                        .flat_map(|part| {
                            part.controller_events
                                .iter()
                                .map(|event| PlaybackMidiController {
                                    position_beats: part.start_beats + event.position_beats,
                                    channel: event.channel,
                                    controller: event.controller,
                                    value: event.value,
                                })
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect();

        let mut playback_tracks = Vec::with_capacity(track_descriptors.len());
        let mut source_errors = Vec::new();
        let mut active_audio_paths: HashSet<_> = self
            .data
            .project
            .tracks
            .iter()
            .flat_map(|t| t.clips.iter().map(|c| c.source_path.clone()))
            .collect();
        // Keep originals available for waveform/analysis and pool operations;
        // playback may additionally load an event's worker-rendered variant.
        for path in &active_audio_paths {
            if !self.data.audio_sources.contains_key(path)
                && let Ok(source) = AudioSource::open(path)
            {
                self.data.audio_sources.insert(path.clone(), source);
            }
        }
        let mut active_stream_clip_ids = HashSet::new();
        for (
            meter_peak,
            is_group,
            audio_channels,
            gain,
            pan,
            track_delay_ms,
            muted,
            solo,
            solo_defeat,
            equalizer,
            inserts,
            inserts_bypassed,
            hosted_inserts,
            instrument_waveform,
            monitor_enabled,
            input_routing,
            output_routing,
            clips,
            midi_notes,
            midi_controllers,
        ) in track_descriptors
        {
            let mut playback_clips = Vec::with_capacity(clips.len());
            for (
                clip_id,
                source_path,
                start_beats,
                clip_gain,
                source_offset_seconds,
                duration_seconds,
                fade_in_seconds,
                fade_out_seconds,
                source_fades,
                pitch_edit,
            ) in clips
            {
                active_audio_paths.insert(source_path.clone());
                let source = if let Some(source) = self.data.audio_sources.get(&source_path) {
                    Some(source.clone())
                } else {
                    match AudioSource::open(&source_path) {
                        Ok(source) => {
                            self.data
                                .audio_sources
                                .insert(source_path.clone(), source.clone());
                            Some(source)
                        }
                        Err(error) => {
                            source_errors.push(format!("{}: {error}", source_path.display()));
                            None
                        }
                    }
                };
                if let Some(mut source) = source {
                    if matches!(source, AudioSource::Streamed(_))
                        && pitch_edit.as_ref().is_some_and(|e| {
                            e.enabled
                                && e.notes.iter().any(|n| {
                                    n.shift != 0.0
                                        || n.straighten != 0.0
                                        || n.start != n.source_start
                                        || n.end != n.source_end
                                })
                        })
                    {
                        match opentrack_audio::DecodedWav::open(&source_path) {
                            Ok(decoded) => {
                                source = AudioSource::Decoded(Arc::new(decoded));
                                self.data
                                    .audio_sources
                                    .insert(source_path.clone(), source.clone());
                            }
                            Err(error) => source_errors.push(format!("Live pitch source: {error}")),
                        }
                    }
                    let playback_source = match &source {
                        AudioSource::Decoded(_) => PlaybackSource::from_asset(&source, None),
                        AudioSource::Streamed(streamed) => {
                            active_stream_clip_ids.insert(clip_id);
                            let existing_reader = self
                                .data
                                .streaming_readers
                                .get(&clip_id)
                                .filter(|(cached_path, _)| cached_path == &source_path)
                                .map(|(_, reader)| Arc::clone(reader));
                            let reader = match existing_reader {
                                Some(reader) => Ok(reader),
                                None => streamed.open_playback().map(|reader| {
                                    self.data.streaming_readers.insert(
                                        clip_id,
                                        (source_path.clone(), Arc::clone(&reader)),
                                    );
                                    reader
                                }),
                            };
                            reader.and_then(|reader| {
                                PlaybackSource::from_asset(&source, Some(reader))
                            })
                        }
                    };
                    let playback_source = playback_source.and_then(|source| {
                        if let Some(edit) = &pitch_edit {
                            match source.clone().with_pitch_time(edit, tempo_bpm) {
                                Ok(processed) => Ok(processed),
                                Err(error) => {
                                    source_errors
                                        .push(format!("{error} Original audio remains active."));
                                    Ok(source)
                                }
                            }
                        } else {
                            Ok(source)
                        }
                    });
                    let playback_source = match playback_source {
                        Ok(source) => source,
                        Err(error) => {
                            source_errors.push(format!("{}: {error}", source_path.display()));
                            continue;
                        }
                    };
                    playback_clips.push(PlaybackClip {
                        source_fades,
                        source: playback_source,
                        start_beats,
                        gain: clip_gain,
                        source_offset_seconds,
                        duration_seconds,
                        fade_in_seconds,
                        fade_out_seconds,
                        muted: false,
                        solo: false,
                    });
                }
            }
            playback_tracks.push(PlaybackTrackSettings {
                is_group,
                audio_channels,
                clips: playback_clips,
                midi_notes,
                midi_controllers,
                instrument_factory: oscillator_instrument_factory(instrument_waveform),
                meter_peak,
                gain,
                pan,
                track_delay_ms,
                muted,
                solo,
                solo_defeat,
                equalizer,
                inserts,
                inserts_bypassed,
                hosted_inserts,
                monitor_input: if monitor_enabled {
                    match input_routing.as_deref().unwrap_or("Stereo In") {
                        "Left - Stereo In" => Some(PlaybackInputChannel::Left),
                        "Right - Stereo In" => Some(PlaybackInputChannel::Right),
                        "No Bus" => None,
                        _ => Some(PlaybackInputChannel::Stereo),
                    }
                } else {
                    None
                },
                output_route: if let Some(group_id) = output_routing
                    .as_deref()
                    .and_then(|route| route.strip_prefix("Group:"))
                    .and_then(|id| id.parse::<u64>().ok())
                {
                    project_tracks
                        .iter()
                        .position(|candidate| candidate.id == group_id)
                        .map_or(
                            PlaybackOutputRoute::Disconnected,
                            PlaybackOutputRoute::Group,
                        )
                } else {
                    match output_routing.as_deref().unwrap_or("Stereo Out") {
                        "Left" => PlaybackOutputRoute::Left,
                        "Right" => PlaybackOutputRoute::Right,
                        "No Bus" => PlaybackOutputRoute::Disconnected,
                        _ => PlaybackOutputRoute::Stereo,
                    }
                },
            });
        }
        self.data
            .streaming_readers
            .retain(|clip_id, _| active_stream_clip_ids.contains(clip_id));
        self.data
            .audio_sources
            .retain(|path, _| active_audio_paths.contains(path));
        let output_hosted_inserts =
            vst3_runtime.hosted_inserts(&self.data.project.output_channel.strip);
        if let Some(engine) = self.audio_engine.as_mut() {
            engine.set_playback_plan_with_hosted_output(
                tempo_bpm,
                playback_tracks,
                loop_range_beats,
                self.data.project.cycle_mode,
                &self.data.project.output_channel.strip,
                opentrack_audio::MetronomeSettings {
                    enabled: self.metronome_enabled,
                    level: self.metronome_level,
                    beats_per_bar: self.data.project.beats_per_bar,
                    beat_unit: self.data.project.beat_unit,
                    downbeat_note: self.metronome_pitch_midi,
                },
                output_hosted_inserts,
            );
        }
        self.update_input_capture_state();
        if !source_errors.is_empty() {
            self.status_message = format!(
                "Some audio sources could not be loaded: {}",
                source_errors.join("; ")
            );
        }
    }

    fn report_streaming_issues(&mut self) {
        let mut issues = Vec::new();
        for (clip_id, (_, reader)) in &self.data.streaming_readers {
            if reader.has_read_error() && self.data.reported_stream_errors.insert(*clip_id) {
                issues.push(format!("Streaming read failed for audio clip {clip_id}"));
            } else if reader.underrun_count() > 0
                && self.data.reported_stream_underruns.insert(*clip_id)
            {
                issues.push(format!(
                    "Audio stream underrun on clip {clip_id}; missing frames were rendered as silence"
                ));
            }
        }
        for (path, source) in &self.data.audio_sources {
            if source.waveform_failed() && self.data.reported_waveform_errors.insert(path.clone()) {
                issues.push(format!(
                    "Could not generate waveform for {}",
                    path.display()
                ));
            }
        }
        if !issues.is_empty() {
            self.status_message = issues.join(" · ");
        }
    }

    fn update_track_meter_levels(&mut self, delta_seconds: f32) {
        let settings = &self.metering_preferences;
        for (track_id, meter_peak) in &self.data.track_meter_peaks {
            self.data.peak_holds.entry(*track_id).or_default().update(
                self.data.track_meter_levels.entry(*track_id).or_default(),
                meter_peak.take(),
                delta_seconds,
                settings,
            );
        }
        let output_peaks = self
            .audio_engine
            .as_ref()
            .map(|e| e.take_output_peaks())
            .unwrap_or([0.0; 2]);
        self.data
            .peak_holds
            .entry(opentrack_core::project::OUTPUT_BUS_ID)
            .or_default()
            .update(
                &mut self.data.output_channel_meter_levels,
                output_peaks,
                delta_seconds,
                settings,
            );
        let input_peaks = if let Some(session) = self.recording_session.as_ref() {
            session.recorder.monitor_buffer().take_bus_peaks()
        } else if let Some(probe) = self.audio_input_probe.as_ref() {
            probe.monitor_buffer().take_bus_peaks()
        } else {
            [0.0; 2]
        };
        self.data
            .peak_holds
            .entry(opentrack_core::project::INPUT_BUS_ID)
            .or_default()
            .update(
                &mut self.data.input_channel_meter_levels,
                input_peaks,
                delta_seconds,
                settings,
            );
    }

    fn set_transport_playing(&mut self, playing: bool) {
        if playing && self.audio_engine.is_none() {
            self.reopen_audio();
            if self.audio_engine.is_none() {
                self.data.transport.playing = false;
                return;
            }
        }
        if !playing && self.is_recording() {
            self.finish_recording();
        }
        self.data.transport.playing = playing;
        if let Some(engine) = &self.audio_engine {
            engine.set_playing(playing);
        }
    }

    fn toggle_metronome(&mut self) {
        self.metronome_enabled = !self.metronome_enabled;
        self.rebuild_audio_plan();
    }

    fn project_timing_changed(&mut self) {
        let beat = self.data.transport.position_beats.max(0.0);
        let tempo = self.data.project.tempo_bpm.max(1.0);
        self.rebuild_audio_plan();
        if let Some(engine) = &self.audio_engine {
            engine.seek_seconds(beat * 60.0 / tempo);
        }
        self.mark_modified();
    }

    fn toggle_cycle(&mut self) {
        self.data.transport.cycle = !self.data.transport.cycle;
        self.rebuild_audio_plan();
        self.mark_modified();
    }

    fn request_project_action(&mut self, action: PendingProjectAction) {
        if self.audio_import_task.is_some() {
            self.status_message =
                "Wait for audio import to finish before changing projects".to_owned();
            return;
        }
        if self.is_recording() {
            self.set_transport_playing(false);
        }
        if self.has_unsaved_changes() {
            self.pending_project_action = Some(action);
            self.status_message = "Save changes before continuing".to_owned();
        } else {
            self.apply_project_action(action);
        }
    }

    fn apply_project_action(&mut self, action: PendingProjectAction) {
        match action {
            PendingProjectAction::New(parent) => self.reset_project(parent),
            PendingProjectAction::Open(path) => self.begin_project_load(path),
            PendingProjectAction::Close => {}
        }
    }

    fn reset_project(&mut self, parent: PathBuf) {
        self.set_transport_playing(false);
        self.project_history.clear();
        self.project_history.skip_next_commit = true;
        self.data = AppData::default();
        self.audio_event_clipboard = None;
        self.dock_state = default_dock_state();
        self.workspace_state = initial_workspace_state(&self.dock_state)
            .expect("the built-in dock layout must be serializable");
        self.arrange_view_state = ArrangeViewState::default();
        self.sample_editor.reset_target();
        self.project_rate_change = None;
        self.sample_editor_active = false;
        self.project_window = restore_project_window_state(
            self.workspace_state
                .active_index
                .and_then(|index| self.workspace_state.workspaces.get(index)),
            self.project_window.epoch,
        );
        self.track_add_dialog = None;
        self.main_window_restore_bounds = None;
        self.midi_editor_open = false;
        self.vst3_editor_bounds = None;
        self.channel_settings_window =
            reset_native_window_state(self.channel_settings_window.epoch);
        self.mixer_window = reset_native_window_state(self.mixer_window.epoch);
        self.media_window = reset_native_window_state(self.media_window.epoch);
        let instantiated = self
            .workspace_state
            .instantiate_global_presets(&self.workspace_preferences);
        self.show_transport = true;
        self.edit_tool = EditTool::Select;
        if let Some(engine) = &self.audio_engine {
            engine.seek_seconds(0.0);
        }
        self.project_path = None;
        self.project_open = false;
        self.project_folder_managed = false;
        self.project_folder_setup = Some(ProjectFolderSetup::New {
            project_name: self.data.project.name.clone(),
            parent,
            error: None,
        });
        self.status_message = if instantiated == 0 {
            "New project".to_owned()
        } else {
            format!("New project with {instantiated} global workspace preset(s)")
        };
        self.rebuild_audio_plan();
        self.mark_saved();
    }

    fn begin_project_folder_setup(&mut self) {
        if self.project_folder_setup.is_some() {
            return;
        }
        if let Some(source_path) = self.project_path.clone() {
            self.project_folder_setup = Some(ProjectFolderSetup::Legacy {
                source_path,
                error: None,
            });
            return;
        }
        let parent = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.project_folder_setup = Some(ProjectFolderSetup::New {
            project_name: self.data.project.name.clone(),
            parent,
            error: None,
        });
    }

    fn create_new_project_folder(
        &mut self,
        project_name: &str,
        parent: &Path,
    ) -> Result<(), String> {
        let project_name = project_name.trim();
        let project_name = if project_name.is_empty() {
            "Untitled Project"
        } else {
            project_name
        };
        let file_name = safe_project_filename(project_name);
        let root = parent.join(project_folder_name(project_name));
        let path = root.join(file_name);
        if project_folder_is_occupied(&root, &path) {
            return Err(format!(
                "The project folder {} already exists; choose another project name or parent folder",
                root.display()
            ));
        }
        create_project_directory_layout(&root)
            .map_err(|error| format!("could not create {}: {error}", root.display()))?;
        let previous_name = self.data.project.name.clone();
        let previous_path = self.project_path.clone();
        self.data.project.name = project_name.to_owned();
        self.project_path = Some(path.clone());
        self.project_folder_managed = true;
        if self.write_project(path) {
            self.project_open = true;
            self.update_input_capture_state();
            Ok(())
        } else {
            let error = self.status_message.clone();
            self.data.project.name = previous_name;
            self.project_path = previous_path;
            self.project_folder_managed = false;
            Err(error)
        }
    }

    fn assign_legacy_project_folder(
        &mut self,
        source_path: &Path,
        root: &Path,
        copy_project: bool,
    ) -> Result<(), String> {
        create_project_directory_layout(root)
            .map_err(|error| format!("could not prepare {}: {error}", root.display()))?;
        let source_parent = source_path.parent().unwrap_or_else(|| Path::new("."));
        let same_folder = paths_refer_to_same_directory(source_parent, root);
        let source_is_opr = source_path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("opr"));
        let destination = if !copy_project && same_folder && source_is_opr {
            Some(source_path.to_path_buf())
        } else {
            unique_project_file_path(
                root,
                source_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or(&self.data.project.name),
            )
        };
        let Some(destination) = destination else {
            return Err("no unused project file name is available in that folder".to_owned());
        };
        self.project_folder_managed = true;
        if self.write_project(destination) {
            self.project_open = true;
            self.update_input_capture_state();
            Ok(())
        } else {
            self.project_folder_managed = false;
            Err(self.status_message.clone())
        }
    }

    fn show_project_open_error(&mut self, ctx: &egui::Context) {
        let Some(message) = self.project_open_error.clone() else {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("project-open-error"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        };
        let mut open = true;
        let mut dismissed = false;
        let response = egui::Window::new("Could not open project")
            .id(Id::new("project-open-error"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_max_width(500.0);
                ui.label(message);
                if ui.button("OK").clicked() {
                    dismissed = true;
                }
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("project-open-error"),
            &mut open,
            response.as_ref(),
        );
        if !open || dismissed {
            self.project_open_error = None;
        }
    }

    fn begin_project_load(&mut self, path: PathBuf) {
        if self.project_load_task.is_some() {
            return;
        }
        let worker_path = path.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let progress = Arc::new(Mutex::new(ProjectLoadStatus::default()));
        let worker_progress = Arc::clone(&progress);
        let configured_host_rate = self
            .audio_engine
            .as_ref()
            .map(|engine| engine.sample_rate)
            .unwrap_or(0);
        let configured_host_block = self
            .audio_engine
            .as_ref()
            .map(|engine| engine.output_buffer_frames.max(64))
            .unwrap_or(512);
        let spawned = std::thread::Builder::new()
            .name("opentrack-project-load".to_owned())
            .spawn(move || {
                update_project_load_status(
                    &worker_progress,
                    0.04,
                    "Reading and validating project".to_owned(),
                );
                let result = (|| -> Result<LoadedProject, String> {
                    let document =
                        ProjectFile::read(&worker_path).map_err(|error| error.to_string())?;
                    let mut vst3_plugins =
                        document
                            .project
                            .tracks
                            .iter()
                            .chain(std::iter::once(&document.project.output_channel.strip))
                            .flat_map(|track| {
                                track.inserts.iter().enumerate().filter_map(
                                    move |(slot, insert)| {
                                        let opentrack_core::project::InsertEffectKind::Vst3 {
                                            name,
                                            ..
                                        } = &insert.kind
                                        else {
                                            return None;
                                        };
                                        Some((track.id, slot, name.clone()))
                                    },
                                )
                            })
                            .collect::<Vec<_>>();
                    vst3_plugins.sort_by_key(|(track_id, slot, _)| (*track_id, *slot));
                    let mut vst3_runtime = vst3_host::Runtime::default();
                    vst3_runtime.set_format(
                        if configured_host_rate == 0 {
                            document.project.sample_rate_hz
                        } else {
                            configured_host_rate
                        },
                        configured_host_block,
                    );
                    update_project_load_status(
                        &worker_progress,
                        0.2,
                        "Collecting referenced audio sources".to_owned(),
                    );
                    let mut paths = document
                        .project
                        .tracks
                        .iter()
                        .flat_map(|track| &track.clips)
                        .flat_map(|clip| {
                            [
                                clip.source_path.clone(),
                                sample_editor::playback_path(clip).to_path_buf(),
                            ]
                        })
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>();
                    paths.sort();
                    let total_sources = paths.len();
                    let mut audio_sources = HashMap::with_capacity(total_sources);
                    for (index, path) in paths.into_iter().enumerate() {
                        let fraction = if total_sources == 0 {
                            0.7
                        } else {
                            0.2 + 0.5 * index as f32 / total_sources as f32
                        };
                        update_project_load_status(
                            &worker_progress,
                            fraction,
                            format!(
                                "Opening audio source {} of {total_sources}: {}",
                                index + 1,
                                project_file_name(&path)
                            ),
                        );
                        if let Ok(source) = AudioSource::open(&path) {
                            audio_sources.insert(path, source);
                        }
                    }
                    update_project_load_status(
                        &worker_progress,
                        0.72,
                        "Audio sources ready; preparing plug-in load queue".to_owned(),
                    );
                    Ok(LoadedProject {
                        document,
                        audio_sources,
                        vst3_runtime,
                        vst3_plugins,
                    })
                })();
                let _ = sender.send(result);
            });
        if let Err(error) = spawned {
            self.status_message = format!("Could not start project load: {error}");
            self.project_open_error = Some(error.to_string());
            return;
        }
        self.project_open_error = None;
        self.project_load_task = Some(ProjectLoadTask {
            path: path.clone(),
            result: receiver,
            progress,
            pending_project: None,
            next_plugin_to_load: 0,
        });
        self.status_message = format!(
            "Reading project and opening audio sources: {}",
            project_file_name(&path)
        );
    }

    fn poll_project_load(&mut self, ctx: &egui::Context) {
        let plugin_to_load = self.project_load_task.as_ref().and_then(|task| {
            let loaded = task.pending_project.as_ref()?;
            let (track_id, slot, name) = loaded.vst3_plugins.get(task.next_plugin_to_load)?;
            Some((
                task.next_plugin_to_load,
                loaded.vst3_plugins.len(),
                *track_id,
                *slot,
                name.clone(),
            ))
        });
        if let Some((_plugin_index, plugin_count, track_id, slot, _name)) = plugin_to_load {
            let task = self
                .project_load_task
                .as_mut()
                .expect("project load task was checked");
            let loaded = task
                .pending_project
                .as_mut()
                .expect("pending project was checked");
            let track = loaded
                .document
                .project
                .tracks
                .iter()
                .find(|track| track.id == track_id)
                .or_else(|| {
                    (loaded.document.project.output_channel.strip.id == track_id)
                        .then_some(&loaded.document.project.output_channel.strip)
                });
            if let Some(track) = track {
                loaded.vst3_runtime.load_insert(track, slot);
            }
            task.next_plugin_to_load += 1;
            let next_plugin = loaded.vst3_plugins.get(task.next_plugin_to_load).cloned();
            if let Some((_, _, name)) = next_plugin {
                let fraction = 0.72 + 0.2 * task.next_plugin_to_load as f32 / plugin_count as f32;
                update_project_load_status(
                    &task.progress,
                    fraction,
                    format!(
                        "Loading VST3 plug-in {} of {plugin_count}: {}",
                        task.next_plugin_to_load + 1,
                        if name.is_empty() {
                            "unnamed plug-in"
                        } else {
                            &name
                        }
                    ),
                );
            } else {
                update_project_load_status(
                    &task.progress,
                    0.94,
                    "Restoring workspace and preparing audio plan".to_owned(),
                );
            }
            ctx.request_repaint();
            return;
        }
        if self.project_load_task.as_ref().is_some_and(|task| {
            task.pending_project
                .as_ref()
                .is_some_and(|loaded| task.next_plugin_to_load >= loaded.vst3_plugins.len())
        }) {
            let task = self.project_load_task.take().expect("load task is present");
            let loaded = task.pending_project.expect("loaded project was checked");
            self.finish_project_load(task.path, loaded);
            return;
        }
        let result = self
            .project_load_task
            .as_ref()
            .map(|task| match task.result.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("project load worker stopped unexpectedly".to_owned()))
                }
            })
            .flatten();
        if let Some(result) = result {
            match result {
                Ok(loaded) => {
                    if let Some(task) = &mut self.project_load_task {
                        let plugin_count = loaded.vst3_plugins.len();
                        let first_plugin =
                            loaded.vst3_plugins.first().map(|(_, _, name)| name.clone());
                        task.pending_project = Some(loaded);
                        if let Some(name) = first_plugin {
                            update_project_load_status(
                                &task.progress,
                                0.72,
                                format!(
                                    "Loading VST3 plug-in 1 of {plugin_count}: {}",
                                    if name.is_empty() {
                                        "unnamed plug-in"
                                    } else {
                                        &name
                                    }
                                ),
                            );
                        } else {
                            update_project_load_status(
                                &task.progress,
                                0.96,
                                "Restoring workspace and starting audio".to_owned(),
                            );
                        }
                    }
                    ctx.request_repaint();
                }
                Err(error) => {
                    let task = self.project_load_task.take().expect("load task is present");
                    self.status_message = format!("Could not open project: {error}");
                    self.project_open_error = Some(format!("{}\n\n{error}", task.path.display()));
                }
            }
        } else if let Some(task) = &self.project_load_task {
            let phase = task
                .progress
                .lock()
                .map(|status| status.phase.clone())
                .unwrap_or_else(|_| "Loading project".to_owned());
            self.status_message = format!("{phase}: {}", project_file_name(&task.path));
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }

    fn show_project_loading(&self, ctx: &egui::Context) {
        let Some(task) = &self.project_load_task else {
            return;
        };
        let status = task
            .progress
            .lock()
            .map(|status| status.clone())
            .unwrap_or_default();
        egui::Window::new("Loading Project")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(360.0);
                ui.label(project_file_name(&task.path));
                ui.add_space(8.0);
                ui.add(
                    egui::ProgressBar::new(status.fraction.clamp(0.0, 1.0))
                        .animate(true)
                        .desired_width(340.0),
                );
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new(status.phase)
                        .small()
                        .color(theme::palette(ui).muted),
                );
            });
    }

    #[cfg(test)]
    fn load_project(&mut self, path: std::path::PathBuf) {
        self.project_open_error = None;
        match ProjectFile::read(&path) {
            Ok(document) => self.finish_project_load(
                path,
                LoadedProject {
                    document,
                    audio_sources: HashMap::new(),
                    vst3_runtime: vst3_host::Runtime::default(),
                    vst3_plugins: Vec::new(),
                },
            ),
            Err(error) => {
                self.status_message = format!("Could not open project: {error}");
                self.project_open_error = Some(format!("{}\n\n{error}", path.display()));
            }
        }
    }

    fn finish_project_load(&mut self, path: std::path::PathBuf, loaded: LoadedProject) {
        let LoadedProject {
            document,
            audio_sources,
            vst3_runtime,
            vst3_plugins: _,
        } = loaded;
        let project_folder_managed =
            document.project_folder_layout > 0 && project_directory_layout_exists(&path);
        let mut workspace_state = document.workspace_state;
        let previous_workspace_state = workspace_state.clone();
        let instantiated = workspace_state.instantiate_global_presets(&self.workspace_preferences);
        let workspace_state_changed = workspace_state != previous_workspace_state;
        let midi_editor_snapshot = workspace_state
            .active_index
            .and_then(|index| workspace_state.workspaces.get(index))
            .and_then(midi_editor_window_snapshot);
        let active_workspace = workspace_state
            .active_index
            .and_then(|index| workspace_state.workspaces.get(index))
            .cloned();
        let vst3_editor_bounds = active_workspace
            .as_ref()
            .and_then(|workspace| project_window_snapshot(workspace, "vst3-editor"))
            .and_then(|window| window.bounds.clone());
        let project_window =
            restore_project_window_state(active_workspace.as_ref(), self.project_window.epoch);
        let arrange_view_state = active_workspace
            .as_ref()
            .map(arrange_view_state_from_workspace)
            .unwrap_or_default();
        let midi_editor_open = midi_editor_snapshot.is_some_and(|window| window.open);
        let main_window_bounds = active_workspace
            .as_ref()
            .and_then(main_window_window_snapshot)
            .and_then(|window| window.bounds.clone());
        self.set_transport_playing(false);
        self.close_vst3_editor();
        self.vst3_runtime = vst3_runtime;
        self.project_history.clear();
        self.project_history.skip_next_commit = true;
        self.data = AppData {
            project: document.project,
            transport: TransportState {
                cycle: document.cycle,
                ..Default::default()
            },
            selected_media: document.selected_media,
            audio_sources,
            streaming_readers: HashMap::new(),
            track_meter_peaks: HashMap::new(),
            track_meter_levels: HashMap::new(),
            peak_holds: HashMap::new(),
            input_channel_meter_levels: [0.0; 2],
            output_channel_meter_levels: [0.0; 2],
            reported_stream_errors: HashSet::new(),
            reported_stream_underruns: HashSet::new(),
            reported_waveform_errors: HashSet::new(),
        };
        self.audio_event_clipboard = None;
        self.workspace_state = workspace_state;
        self.dock_state = document.live_dock_state;
        self.arrange_view_state = arrange_view_state;
        self.sample_editor.reset_target();
        self.project_rate_change = None;
        self.sample_editor_active = false;
        self.project_window = project_window;
        self.track_add_dialog = None;
        if let Some(bounds) = main_window_bounds {
            self.main_window_bounds = Some(bounds.clone());
            self.main_window_restore_bounds = Some(bounds);
        } else {
            self.main_window_restore_bounds = None;
        }
        self.midi_editor_open = midi_editor_open;
        self.vst3_editor_bounds = vst3_editor_bounds;
        self.channel_settings_window = restore_native_window_state(
            active_workspace.as_ref(),
            CHANNEL_SETTINGS_WINDOW_ID,
            self.channel_settings_window.epoch,
        );
        self.mixer_window = restore_native_window_state(
            active_workspace.as_ref(),
            MIXER_WINDOW_ID,
            self.mixer_window.epoch,
        );
        self.media_window = restore_native_window_state(
            active_workspace.as_ref(),
            MEDIA_WINDOW_ID,
            self.media_window.epoch,
        );
        self.show_transport = document.show_transport;
        self.edit_tool = EditTool::Select;
        self.project_path = Some(path.clone());
        self.project_open = project_folder_managed;
        self.project_folder_managed = project_folder_managed;
        self.project_folder_setup = (!project_folder_managed).then(|| ProjectFolderSetup::Legacy {
            source_path: path.clone(),
            error: None,
        });
        self.status_message = if instantiated > 0 {
            format!(
                "Opened {}; added {instantiated} global workspace preset(s)",
                project_file_name(&path)
            )
        } else if workspace_state_changed {
            format!(
                "Opened {}; updated workspace preset tracking",
                project_file_name(&path)
            )
        } else {
            format!("Opened {}", project_file_name(&path))
        };
        if !project_folder_managed {
            self.status_message = format!(
                "Opened {}; choose or create its project folder before recording",
                project_file_name(&path)
            );
        }
        if let Some(engine) = &self.audio_engine {
            engine.seek_seconds(0.0);
        }
        self.rebuild_audio_plan();
        self.mark_saved();
    }

    fn save_project(&mut self) -> bool {
        if !self.project_open {
            self.status_message = "No project is open".to_owned();
            return false;
        }
        if self.is_recording() {
            self.set_transport_playing(false);
        }
        if !self.project_folder_managed {
            if self.project_folder_setup.is_none() {
                self.begin_project_folder_setup();
            }
            self.status_message =
                "Choose or create a project folder before saving this project".to_owned();
            return false;
        }
        if let Some(path) = self.project_path.clone().filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("opr"))
        }) {
            self.write_project(path)
        } else {
            // Offer a new OPR path when saving a legacy project; preserve its source file.
            self.save_project_as()
        }
    }

    fn save_project_as(&mut self) -> bool {
        if !self.project_open {
            self.status_message = "No project is open".to_owned();
            return false;
        }
        if self.is_recording() {
            self.set_transport_playing(false);
        }
        let mut dialog = FileDialog::new()
            .add_filter("OpenTrack Project", &["opr"])
            .set_title("Save OpenTrack Project As")
            .set_file_name(safe_project_filename(&self.data.project.name));
        if let Some(directory) = self.project_path.as_deref().and_then(|path| path.parent()) {
            dialog = dialog.set_directory(directory);
        }
        let Some(mut path) = dialog.save_file() else {
            return false;
        };
        path.set_extension("opr");
        self.write_project(path)
    }

    fn write_project(&mut self, mut path: std::path::PathBuf) -> bool {
        self.vst3_runtime.capture_states(&mut self.data.project);
        if let Err(message) = self.data.project.validate() {
            self.status_message = format!("Could not save project: {message}");
            return false;
        }
        if !self.project_folder_managed {
            let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            else {
                self.status_message = "Choose a folder for this project before saving".to_owned();
                return false;
            };
            let folder_name = path
                .file_stem()
                .map(|stem| project_folder_name(&stem.to_string_lossy()))
                .unwrap_or_else(|| "Untitled Project".to_owned());
            let root = parent.join(folder_name);
            let Some(file_name) = path.file_name().map(|name| name.to_os_string()) else {
                self.status_message = "The selected project path has no file name".to_owned();
                return false;
            };
            let destination = root.join(&file_name);
            if destination.exists() {
                self.status_message = format!(
                    "{} already exists; choose another project name or location",
                    destination.display()
                );
                return false;
            }
            if let Err(error) = create_project_directory_layout(&root) {
                self.status_message = format!("Could not create project folder: {error}");
                return false;
            }
            path = destination;
        }
        let previous_folder_state = self.project_folder_managed;
        self.project_folder_managed = true;
        if let Some(root) = path.parent()
            && let Err(error) = create_project_directory_layout(root)
        {
            self.project_folder_managed = previous_folder_state;
            self.status_message = format!("Could not prepare project folders: {error}");
            return false;
        }
        let write_result = self
            .project_file()
            .and_then(|document| document.write(&path));
        match write_result {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.project_folder_setup = None;
                self.status_message = format!("Saved {}", project_file_name(&path));
                self.mark_saved();
                true
            }
            Err(error) => {
                self.project_folder_managed = previous_folder_state;
                self.status_message = format!("Could not save project: {error}");
                false
            }
        }
    }

    fn show_unsaved_prompt(&mut self, ctx: &egui::Context) {
        if self.pending_project_action.is_none() {
            return;
        }

        let mut choice = None;
        egui::Window::new("Unsaved project changes")
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Save your changes before continuing?");
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        choice = Some(UnsavedChoice::Save);
                    }
                    if ui.button("Discard").clicked() {
                        choice = Some(UnsavedChoice::Discard);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(UnsavedChoice::Cancel);
                    }
                });
            });

        match choice {
            Some(UnsavedChoice::Save) => {
                if self.save_project() {
                    self.apply_pending_project_action(ctx);
                }
            }
            Some(UnsavedChoice::Discard) => match self.project_history.discard_recordings() {
                Ok(()) => {
                    // Discard stays discarded even if opening the next project fails.
                    if let Ok(saved) =
                        serde_json::from_slice::<Project>(&self.saved_project_fingerprint)
                    {
                        self.data.project = saved;
                        self.rebuild_audio_plan();
                        self.refresh_modified_from_saved();
                    }
                    self.project_history.clear();
                    self.apply_pending_project_action(ctx);
                }
                Err(error) => {
                    self.status_message = format!("Could not discard recording files: {error}")
                }
            },
            Some(UnsavedChoice::Cancel) => {
                self.pending_project_action = None;
                self.status_message = "Ready".to_owned();
            }
            None => {}
        }
    }

    fn show_project_folder_setup(&mut self, ctx: &egui::Context) {
        let Some(mut setup) = self.project_folder_setup.take() else {
            return;
        };
        let mut open = true;
        let mut cancel = false;
        let mut action = None;
        egui::Window::new(match &setup {
            ProjectFolderSetup::New { .. } => "Create Project Folder",
            ProjectFolderSetup::Legacy { .. } => "Set Up Project Folder",
        })
        .id(Id::new("project-folder-setup"))
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| match &mut setup {
            ProjectFolderSetup::New {
                project_name,
                parent,
                error,
            } => {
                ui.set_min_width(440.0);
                ui.label("Create the project's folder before saving or recording.");
                ui.label("Project name");
                ui.text_edit_singleline(project_name);
                ui.separator();
                ui.label("Create inside");
                ui.label(parent.display().to_string());
                if ui.button("Choose Parent Folder…").clicked()
                    && let Some(directory) = FileDialog::new()
                        .set_directory(&*parent)
                        .set_title("Choose Parent Folder")
                        .pick_folder()
                {
                    *parent = directory;
                }
                ui.label(
                    "OpenTrack creates Audio/Recordings, Audio/Imports, and Metadata folders.",
                );
                if let Some(error) = error.as_ref() {
                    ui.colored_label(theme::RECORD, error);
                }
                ui.horizontal(|ui| {
                    if ui.button("Create Project").clicked() {
                        action = Some(ProjectFolderAction::CreateNew {
                            name: project_name.clone(),
                            parent: parent.clone(),
                        });
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            }
            ProjectFolderSetup::Legacy { source_path, error } => {
                ui.set_min_width(440.0);
                ui.label("This project predates managed project folders.");
                ui.label("Choose where its new recordings and project file should live.");
                ui.label(source_path.display().to_string());
                if let Some(error) = error.as_ref() {
                    ui.colored_label(theme::RECORD, error);
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Use Current Folder").clicked() {
                        action = Some(ProjectFolderAction::AssignLegacy {
                            root: source_path
                                .parent()
                                .unwrap_or_else(|| Path::new("."))
                                .to_path_buf(),
                            copy_project: false,
                        });
                    }
                    if ui.button("Create Beside Project").clicked() {
                        let parent = source_path.parent().unwrap_or_else(|| Path::new("."));
                        let name = source_path
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or("Project");
                        action = Some(ProjectFolderAction::AssignLegacy {
                            root: unique_project_folder_path(parent, name),
                            copy_project: true,
                        });
                    }
                    if ui.button("Create New Folder…").clicked()
                        && let Some(parent) = FileDialog::new()
                            .set_directory(source_path.parent().unwrap_or_else(|| Path::new(".")))
                            .set_title("Choose Where to Create the Project Folder")
                            .pick_folder()
                    {
                        action = Some(ProjectFolderAction::CreateLegacyFolder { parent });
                    }
                    if ui.button("Choose Existing Folder…").clicked()
                        && let Some(directory) = FileDialog::new()
                            .set_directory(source_path.parent().unwrap_or_else(|| Path::new(".")))
                            .set_title("Choose Project Folder")
                            .pick_folder()
                    {
                        action = Some(ProjectFolderAction::AssignLegacy {
                            root: directory,
                            copy_project: true,
                        });
                    }
                    if ui.button("Cancel Open").clicked() {
                        cancel = true;
                    }
                });
            }
        });

        let result = match action {
            Some(ProjectFolderAction::CreateNew { name, parent }) => {
                Some(self.create_new_project_folder(&name, &parent))
            }
            Some(ProjectFolderAction::AssignLegacy { root, copy_project }) => {
                let source_path = match &setup {
                    ProjectFolderSetup::Legacy { source_path, .. } => source_path.clone(),
                    ProjectFolderSetup::New { .. } => unreachable!(),
                };
                Some(self.assign_legacy_project_folder(&source_path, &root, copy_project))
            }
            Some(ProjectFolderAction::CreateLegacyFolder { parent }) => {
                let source_path = match &setup {
                    ProjectFolderSetup::Legacy { source_path, .. } => source_path.clone(),
                    ProjectFolderSetup::New { .. } => unreachable!(),
                };
                let name = source_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("Project");
                let root = unique_project_folder_path(&parent, name);
                Some(self.assign_legacy_project_folder(&source_path, &root, true))
            }
            None => None,
        };

        match result {
            Some(Ok(())) => {
                self.project_folder_setup = None;
                if self.pending_project_action.is_some() {
                    self.apply_pending_project_action(ctx);
                }
            }
            Some(Err(error)) => {
                match &mut setup {
                    ProjectFolderSetup::New { error: target, .. }
                    | ProjectFolderSetup::Legacy { error: target, .. } => *target = Some(error),
                }
                if open && !cancel {
                    self.project_folder_setup = Some(setup);
                }
            }
            None if open && !cancel => self.project_folder_setup = Some(setup),
            None => self.cancel_project_folder_setup(),
        }
    }

    fn cancel_project_folder_setup(&mut self) {
        self.project_folder_setup = None;
        self.project_path = None;
        self.project_open = false;
        self.project_folder_managed = false;
        self.data = AppData::default();
        self.audio_event_clipboard = None;
        self.project_history.clear();
        self.mark_saved();
        self.status_message = "Project folder setup canceled".to_owned();
    }

    fn open_event_colors_dialog(&mut self) {
        let slot_index = self
            .arrange_view_state
            .selected_event_color
            .map(usize::from)
            .unwrap_or(0)
            .min(self.arrange_view_state.event_colors.len() - 1);
        let rgb = self.arrange_view_state.event_colors[slot_index];
        self.event_colors_dialog = Some(EventColorsDialog {
            slot_index,
            draft_rgb: rgb,
        });
    }

    fn close_vst3_editor(&mut self) {
        if let Some(host) = self.vst3_editor_host.take() {
            if let Some(bounds) = host.bounds() {
                self.vst3_editor_bounds = Some(bounds);
            }
            host.close();
        }
        if let Some((track_id, slot)) = self.vst3_editor_window.take() {
            self.vst3_runtime.close_editor(track_id, slot);
            self.vst3_runtime.capture_states(&mut self.data.project);
        }
    }

    fn open_vst3_editor(&mut self, track_id: u64, slot: usize) -> Result<(), String> {
        if self.vst3_editor_window == Some((track_id, slot))
            && let Some(window) = &self.vst3_editor_host
            && window.is_alive()
        {
            window.focus();
            return Ok(());
        }
        self.close_vst3_editor();
        let title = self
            .data
            .project
            .channels()
            .find(|track| track.id == track_id)
            .and_then(|track| track.inserts.get(slot))
            .map(|insert| insert_effect_name(&insert.kind))
            .unwrap_or_else(|| "VST3 Plug-in".to_owned());
        let effect = self
            .vst3_runtime
            .instance(track_id, slot)
            .ok_or_else(|| "the VST3 instance is unavailable".to_owned())?;
        // Create the native editor without an owner first: the UI thread waits
        // for its STA editor thread to initialize. Attach ownership afterward
        // so the plug-in stays over OpenTrack's Project and lower-zone panels.
        let window = vst3_host::EditorHost::open(&title, self.vst3_editor_bounds.clone(), effect)?;
        if let Some(owner) = self.root_window_handle {
            window.set_owner(owner);
        }
        self.vst3_editor_host = Some(window);
        self.vst3_editor_window = Some((track_id, slot));
        Ok(())
    }

    fn show_vst3_windows(&mut self, ctx: &egui::Context) {
        if let Some(bounds) = self
            .vst3_editor_host
            .as_ref()
            .and_then(vst3_host::EditorHost::bounds)
        {
            self.vst3_editor_bounds = Some(bounds);
        }
        if self.vst3_editor_window.is_some() {
            // iPlug2 routes processor-to-editor meter messages through its
            // component thread's idle timer. Components are instantiated on
            // this UI thread, separate from the native editor window thread.
            vst3_host::pump_ui_thread_timers();
        }
        if self.vst3_runtime.take_dirty() {
            self.mark_modified();
        }

        if let Some((track_id, slot)) = self.vst3_editor_window {
            self.vst3_runtime.sync_editor_parameters(track_id, slot);
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        if let (Some((track_id, slot)), Some(engine)) =
            (self.vst3_editor_window, self.audio_engine.as_ref())
        {
            let track_index = self
                .data
                .project
                .tracks
                .iter()
                .position(|track| track.id == track_id);
            let diagnostic = engine.hosted_processing_diagnostic();
            if let Some(track_index) = track_index
                && diagnostic.error_count > 0
                && diagnostic.last_track == Some(track_index)
                && diagnostic.last_insert == Some(slot)
            {
                let message = format!(
                    "VST3 audio processing failed on {} (insert {}); dry audio is passing through",
                    self.data.project.tracks[track_index].name,
                    slot + 1
                );
                if self.status_message != message {
                    self.status_message = message;
                }
            }
        }
        if self
            .vst3_editor_host
            .as_ref()
            .is_some_and(|window| !window.is_alive())
        {
            if self.vst3_editor_window.take().is_some() {
                self.vst3_runtime.capture_states(&mut self.data.project);
                self.mark_modified();
            }
            self.vst3_editor_host = None;
        }
        if let Some(target) = self.channel_settings_ui.vst3_parameter_slot.take() {
            self.vst3_parameter_window = None;
            if let Err(error) = self.open_vst3_editor(target.0, target.1) {
                self.status_message = format!("Native VST3 editor unavailable: {error}");
                self.vst3_parameter_window = Some(target);
            }
        }
        let Some((track_id, slot)) = self.vst3_parameter_window else {
            return;
        };
        let title = self
            .data
            .project
            .channels()
            .find(|track| track.id == track_id)
            .and_then(|track| track.inserts.get(slot))
            .map(|insert| insert_effect_name(&insert.kind))
            .unwrap_or_else(|| "VST3 Plug-in".to_owned());
        let parameters = self.vst3_runtime.parameters(track_id, slot);
        let failure = self.vst3_runtime.failure(track_id, slot).map(str::to_owned);
        let has_editor = self.vst3_runtime.has_editor(track_id, slot);
        let mut enabled = self
            .data
            .project
            .channel_by_id_mut(track_id)
            .and_then(|track| track.inserts.get(slot))
            .is_some_and(|insert| insert.enabled);
        let mut set_parameter = None;
        let mut toggle_enabled = false;
        let mut remove = false;
        let mut retry = false;
        let mut open_editor = false;
        let mut close_editor = false;
        let mut open = true;
        let window_id = Id::new(("vst3-parameters", track_id, slot));
        let response = egui::Window::new(format!("VST3 \u{2014} {title}"))
            .id(window_id)
            .open(&mut open)
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                if let Some(failure) = &failure {
                    ui.colored_label(theme::RECORD, failure);
                    if ui.button("Retry Load").clicked() {
                        retry = true;
                    }
                }
                if let Some(engine) = &self.audio_engine {
                    let diagnostic = engine.hosted_processing_diagnostic();
                    if diagnostic.error_count > 0 {
                        ui.colored_label(
                            theme::WARNING,
                            format!(
                                "{} VST3 processing error(s); the channel fell back to dry audio.",
                                diagnostic.error_count
                            ),
                        );
                    }
                }
                if ui.checkbox(&mut enabled, "Enabled / bypass").changed() {
                    toggle_enabled = true;
                }
                if let Some(parameters) = &parameters {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for parameter in parameters {
                            let mut value = parameter.normalized as f32;
                            let label = if parameter.units.is_empty() {
                                parameter.title.clone()
                            } else {
                                format!("{} ({})", parameter.title, parameter.units)
                            };
                            if ui
                                .add(egui::Slider::new(&mut value, 0.0..=1.0).text(label))
                                .changed()
                            {
                                set_parameter = Some((parameter.id, f64::from(value)));
                            }
                        }
                    });
                } else {
                    ui.label("Parameters are unavailable; the instance may have failed to load.");
                }
                ui.horizontal(|ui| {
                    if has_editor
                        && ui
                            .add_enabled(
                                self.root_window_handle.is_some(),
                                egui::Button::new("Edit"),
                            )
                            .clicked()
                    {
                        open_editor = true;
                    }
                    if self.vst3_editor_window == Some((track_id, slot))
                        && ui.button("Close Editor").clicked()
                    {
                        close_editor = true;
                    }
                    if ui.button("Remove Plug-in").clicked() {
                        remove = true;
                    }
                });
            });
        dismiss_window_on_outside_click(ctx, window_id, &mut open, response.as_ref());
        if let Some((id, value)) = set_parameter {
            self.vst3_runtime.set_parameter(track_id, slot, id, value);
            self.vst3_runtime.capture_states(&mut self.data.project);
            self.mark_modified();
        }
        if retry {
            self.vst3_runtime.retry(track_id, slot);
            self.rebuild_audio_plan();
        }
        if toggle_enabled
            && let Some(track) = self.data.project.channel_by_id_mut(track_id)
            && let Some(insert) = track.inserts.get_mut(slot)
        {
            insert.enabled = enabled;
            self.rebuild_audio_plan();
            self.mark_modified();
        }
        if open_editor {
            if let Err(error) = self.open_vst3_editor(track_id, slot) {
                self.status_message = format!("VST3 editor: {error}");
            } else {
                self.vst3_parameter_window = None;
            }
        }
        if close_editor {
            self.vst3_editor_host = None;
            self.vst3_runtime.capture_states(&mut self.data.project);
            self.vst3_editor_window = None;
            self.mark_modified();
        }
        if remove {
            if let Some(track) = self.data.project.channel_by_id_mut(track_id)
                && slot < track.inserts.len()
            {
                track.inserts.remove(slot);
            }
            self.vst3_runtime.close_editor(track_id, slot);
            self.vst3_editor_host = None;
            self.vst3_editor_window = None;
            self.vst3_parameter_window = None;
            self.rebuild_audio_plan();
            self.mark_modified();
        } else if !open {
            self.vst3_parameter_window = None;
        }
    }

    fn show_event_colors_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.event_colors_dialog.take() else {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("project-event-colors-dialog"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut accept = false;
        let mut reset = false;
        let mut cancel = false;
        let mut color = Color32::from_rgb(
            dialog.draft_rgb[0],
            dialog.draft_rgb[1],
            dialog.draft_rgb[2],
        );
        let response = egui::Window::new("Event Colors")
            .id(Id::new("project-event-colors-dialog"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.label(format!("Event color slot {}", dialog.slot_index + 1));
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgba(&mut color);
                    ui.label(format!("R {}  G {}  B {}", color.r(), color.g(), color.b()));
                });
                ui.label("Select a slot to edit");
                ui.horizontal_wrapped(|ui| {
                    for (index, rgb) in self.arrange_view_state.event_colors.iter().enumerate() {
                        let swatch = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        let selected = index == dialog.slot_index;
                        let response = ui.add_sized(
                            Vec2::splat(20.0),
                            egui::Button::new(" ").fill(swatch).stroke(if selected {
                                Stroke::new(2.0, theme::palette(ui).text)
                            } else {
                                Stroke::NONE
                            }),
                        );
                        if response.clicked() {
                            dialog.slot_index = index;
                            let draft = self.arrange_view_state.event_colors[index];
                            color = Color32::from_rgb(draft[0], draft[1], draft[2]);
                        }
                    }
                });
                ui.separator();
                ui.label("Apply the chosen color to the selected event color slot.");
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        apply = true;
                    }
                    if ui.button("Reset").clicked() {
                        reset = true;
                    }
                    if ui.button("OK").clicked() {
                        accept = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("project-event-colors-dialog"),
            &mut open,
            response.as_ref(),
        );
        dialog.draft_rgb = [color.r(), color.g(), color.b()];
        if apply || accept || reset {
            if reset {
                self.arrange_view_state.event_colors[dialog.slot_index] =
                    project_controls::COLORS[dialog.slot_index];
                let rgb = self.arrange_view_state.event_colors[dialog.slot_index];
                dialog.draft_rgb = rgb;
            } else {
                self.arrange_view_state.event_colors[dialog.slot_index] = dialog.draft_rgb;
            }
            self.arrange_view_state.selected_event_color = Some(dialog.slot_index as u8);
            self.mark_modified();
        }
        if !accept && !cancel && open {
            self.event_colors_dialog = Some(dialog);
        }
    }

    fn show_track_rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.track_rename_dialog.clone() else {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("track-rename-dialog"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        };
        let track_exists = self
            .data
            .project
            .tracks
            .iter()
            .any(|track| track.id == dialog.track_id);
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        let response = egui::Window::new("Rename Track")
            .id(Id::new("track-rename-dialog"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                if !track_exists {
                    ui.label("This track no longer exists.");
                } else {
                    ui.label("Track name");
                    let name = ui.add(
                        egui::TextEdit::singleline(&mut dialog.draft)
                            .desired_width(260.0)
                            .hint_text("Track name"),
                    );
                    let enter_pressed =
                        name.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !dialog.draft.trim().is_empty(),
                                egui::Button::new("Rename"),
                            )
                            .clicked()
                            || enter_pressed
                        {
                            save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                }
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("track-rename-dialog"),
            &mut open,
            response.as_ref(),
        );

        if save {
            self.rename_track_by_id(dialog.track_id, &dialog.draft);
            self.track_rename_dialog = None;
        } else if cancel || !open || !track_exists {
            self.track_rename_dialog = None;
        } else {
            self.track_rename_dialog = Some(dialog);
        }
    }

    fn show_track_delete_prompt(&mut self, ctx: &egui::Context) {
        let Some(track_id) = self.pending_track_delete_id else {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("track-delete-prompt"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        };
        let Some(track_name) = self
            .data
            .project
            .tracks
            .iter()
            .find(|track| track.id == track_id)
            .map(|track| track.name.clone())
        else {
            self.pending_track_delete_id = None;
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("track-delete-prompt"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        };
        let mut open = true;
        let mut remove = false;
        let mut cancel = false;
        let response = egui::Window::new("Remove Track")
            .id(Id::new("track-delete-prompt"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Remove {track_name} and all of its clips and MIDI parts?"
                ));
                ui.label(
                    "The track can be restored with Undo. Audio files remain in the Media Pool.",
                );
                if self.is_recording() {
                    ui.colored_label(theme::WARNING, "Stop recording before removing a track.");
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.is_recording(), egui::Button::new("Remove Track"))
                        .clicked()
                    {
                        remove = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("track-delete-prompt"),
            &mut open,
            response.as_ref(),
        );

        if remove {
            self.remove_track_by_id(track_id);
            self.pending_track_delete_id = None;
        } else if cancel || !open {
            self.pending_track_delete_id = None;
        }
    }

    fn apply_pending_project_action(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.pending_project_action.take() {
            if matches!(action, PendingProjectAction::Close) {
                self.confirmed_close_request = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                self.apply_project_action(action);
            }
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if close_requested && self.confirmed_close_request {
            self.confirmed_close_request = false;
            return;
        }
        if close_requested && self.audio_import_task.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.status_message =
                "Wait for the project audio import to finish before closing".to_owned();
            return;
        }
        if close_requested && self.has_unsaved_changes() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_project_action = Some(PendingProjectAction::Close);
            self.status_message = "Save changes before closing".to_owned();
        }
    }

    fn capture_active_workspace(&mut self) -> Result<(), ProjectFileError> {
        let active_index = self.workspace_state.active_index.ok_or_else(|| {
            ProjectFileError::InvalidData("project has no active workspace".to_owned())
        })?;
        let active = self
            .workspace_state
            .workspaces
            .get(active_index)
            .ok_or_else(|| {
                ProjectFileError::InvalidData("active workspace index is invalid".to_owned())
            })?;
        if active.locked {
            return Ok(());
        }
        Self::store_active_workspace_windows(
            &mut self.workspace_state,
            &self.dock_state,
            self.main_window_bounds.clone(),
            &self.arrange_view_state,
            &self.project_window,
            self.midi_editor_open,
            self.vst3_editor_bounds.clone(),
            &self.channel_settings_window,
            &self.mixer_window,
            &self.media_window,
        )
    }

    fn activate_workspace(&mut self, index: usize) {
        let Some(workspace) = self.workspace_state.workspaces.get(index).cloned() else {
            return;
        };
        let is_current_workspace = self.workspace_state.active_index == Some(index);
        if is_current_workspace && !workspace.locked {
            return;
        }
        let dock_state = match dock_state_from_workspace(&workspace) {
            Ok(dock_state) => dock_state,
            Err(error) => {
                self.status_message = format!("Could not activate workspace: {error}");
                return;
            }
        };
        if !is_current_workspace && let Err(error) = self.capture_active_workspace() {
            self.status_message = format!("Could not save workspace layout: {error}");
            return;
        }
        let _ = self.workspace_state.activate_index(index);
        self.dock_state = dock_state;
        self.arrange_view_state = arrange_view_state_from_workspace(&workspace);
        self.project_window =
            restore_project_window_state(Some(&workspace), self.project_window.epoch);
        let active_main_bounds =
            main_window_window_snapshot(&workspace).and_then(|window| window.bounds.clone());
        if let Some(bounds) = active_main_bounds.clone() {
            self.main_window_bounds = Some(bounds);
        }
        self.main_window_restore_bounds = active_main_bounds;
        let midi_editor_snapshot = midi_editor_window_snapshot(&workspace);
        self.midi_editor_open = midi_editor_snapshot.is_some_and(|window| window.open);
        self.vst3_editor_bounds = project_window_snapshot(&workspace, "vst3-editor")
            .and_then(|window| window.bounds.clone());
        self.channel_settings_window = restore_native_window_state(
            Some(&workspace),
            CHANNEL_SETTINGS_WINDOW_ID,
            self.channel_settings_window.epoch,
        );
        self.mixer_window =
            restore_native_window_state(Some(&workspace), MIXER_WINDOW_ID, self.mixer_window.epoch);
        self.media_window =
            restore_native_window_state(Some(&workspace), MEDIA_WINDOW_ID, self.media_window.epoch);
        self.status_message = format!("Activated workspace: {}", workspace.name);
    }

    fn create_workspace(&mut self) {
        if self.workspace_state.workspaces.len() >= 9 {
            self.status_message = "A project can have at most 9 shortcut workspaces".to_owned();
            return;
        }
        if let Err(error) = self.capture_active_workspace() {
            self.status_message = format!("Could not save workspace layout: {error}");
            return;
        }
        let workspace_number = self.workspace_state.workspaces.len() + 1;
        let next_shortcut_id = self
            .workspace_state
            .use_explicit_ids
            .then(|| {
                (1..=9).filter_map(WorkspaceId::new).find(|candidate| {
                    self.workspace_state
                        .workspaces
                        .iter()
                        .all(|workspace| workspace.shortcut_id != Some(*candidate))
                })
            })
            .flatten();
        let new_index = self
            .workspace_state
            .create_workspace(format!("Workspace {workspace_number}"));
        if let Some(shortcut_id) = next_shortcut_id {
            self.workspace_state.workspaces[new_index].shortcut_id = Some(shortcut_id);
        }
        if let Err(error) = Self::store_active_workspace_windows(
            &mut self.workspace_state,
            &self.dock_state,
            self.main_window_bounds.clone(),
            &self.arrange_view_state,
            &self.project_window,
            self.midi_editor_open,
            self.vst3_editor_bounds.clone(),
            &self.channel_settings_window,
            &self.mixer_window,
            &self.media_window,
        ) {
            self.status_message = format!("Could not initialize workspace: {error}");
            return;
        }
        self.status_message = format!("Created Workspace {workspace_number}");
    }

    fn save_active_workspace_as_global_preset(&mut self) {
        if let Err(error) = self.capture_active_workspace() {
            self.status_message = format!("Could not save workspace preset: {error}");
            return;
        }
        let Some(workspace) = self
            .workspace_state
            .active_index
            .and_then(|index| self.workspace_state.workspaces.get(index))
            .cloned()
        else {
            self.status_message = "Project has no active workspace".to_owned();
            return;
        };
        let name = unique_workspace_preset_name(
            &workspace.name,
            &self.workspace_preferences.global_presets,
        );
        self.workspace_preferences
            .global_presets
            .push(WorkspacePreset::from_workspace(name.clone(), &workspace));
        self.status_message = format!("Saved global workspace preset: {name}");
    }

    fn save_active_workspace_as_project_preset(&mut self) {
        if let Err(error) = self.capture_active_workspace() {
            self.status_message = format!("Could not save workspace preset: {error}");
            return;
        }
        let Some(workspace) = self
            .workspace_state
            .active_index
            .and_then(|index| self.workspace_state.workspaces.get(index))
            .cloned()
        else {
            self.status_message = "Project has no active workspace".to_owned();
            return;
        };
        let name =
            unique_workspace_preset_name(&workspace.name, &self.workspace_state.project_presets);
        self.workspace_state
            .project_presets
            .push(WorkspacePreset::from_workspace(name.clone(), &workspace));
        self.mark_modified();
        self.status_message = format!("Saved project workspace preset: {name}");
    }

    fn copy_global_workspace_presets_to_project(&mut self) {
        let previous = self.workspace_state.project_presets.clone();
        self.workspace_state
            .include_global_presets(&self.workspace_preferences);
        if self.workspace_state.project_presets != previous {
            self.mark_modified();
            self.status_message = format!(
                "Copied {} global workspace preset(s) into the project",
                self.workspace_preferences.global_presets.len()
            );
        } else {
            self.status_message =
                "Project workspace presets already match global presets".to_owned();
        }
    }

    fn create_workspace_from_preset(
        &mut self,
        preset: WorkspacePreset,
        global_preset_source: Option<String>,
    ) {
        if self.workspace_state.workspaces.len() >= 9 {
            self.status_message = "A project can have at most 9 shortcut workspaces".to_owned();
            return;
        }
        if let Err(error) = self.capture_active_workspace() {
            self.status_message = format!("Could not save current workspace: {error}");
            return;
        }

        let mut workspace = Workspace::new(preset.name.clone());
        preset.apply_to(&mut workspace);
        let dock_state = match dock_state_from_workspace(&workspace) {
            Ok(dock_state) => dock_state,
            Err(error) => {
                self.status_message = format!("Could not create workspace from preset: {error}");
                return;
            }
        };
        let shortcut_id = self
            .workspace_state
            .use_explicit_ids
            .then(|| {
                (1..=9).filter_map(WorkspaceId::new).find(|candidate| {
                    self.workspace_state
                        .workspaces
                        .iter()
                        .all(|existing| existing.shortcut_id != Some(*candidate))
                })
            })
            .flatten();
        let index = self.workspace_state.workspaces.len();
        workspace.shortcut_id = shortcut_id;
        workspace.instantiated_from_preset = global_preset_source;
        self.workspace_state.workspaces.push(workspace);
        self.workspace_state.active_index = Some(index);
        self.dock_state = dock_state;
        self.arrange_view_state = arrange_view_state_from_workspace(
            self.workspace_state
                .workspaces
                .get(index)
                .expect("workspace was just inserted"),
        );
        self.status_message = format!("Created workspace from preset: {}", preset.name);
    }

    fn toggle_workspace_lock(&mut self) {
        let Some(active_index) = self.workspace_state.active_index else {
            self.status_message = "Project has no active workspace".to_owned();
            return;
        };
        let Some(active) = self.workspace_state.workspaces.get(active_index) else {
            self.status_message = "Active workspace index is invalid".to_owned();
            return;
        };
        let was_locked = active.locked;
        if !was_locked {
            if let Err(error) = self.capture_active_workspace() {
                self.status_message = format!("Could not save workspace layout: {error}");
                return;
            }
        }
        self.workspace_state.set_active_locked(!was_locked);
        if was_locked {
            if let Err(error) = self.capture_active_workspace() {
                self.status_message = format!("Could not save workspace layout: {error}");
                return;
            }
        }
        self.status_message = if was_locked {
            "Workspace unlocked".to_owned()
        } else {
            "Workspace locked".to_owned()
        };
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // The Win32 subclass is installed on the root HWND only. Child
        // viewports must use egui's focused-window events instead of draining
        // the root queue (which loses keys such as Space in Channel Settings).
        let use_native_key_events = cfg!(windows)
            && ctx.viewport_id() == egui::ViewportId::ROOT
            && self.native_key_capture.is_some();
        let native_key_events = if use_native_key_events {
            self.native_key_capture
                .as_ref()
                .map(NativeKeyCapture::take_events)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let chords = if use_native_key_events {
            native_key_events
                .into_iter()
                .map(|event| event.chord)
                .collect()
        } else {
            egui_key_chords(ctx)
        };
        if !chords.is_empty() {
            if !self.project_history.begin(&self.data.project) {
                self.status_message =
                    "Undo history is unavailable for this project size".to_owned();
            }
            self.project_history.commit_after_frame = true;
        }
        for chord in chords {
            self.dispatch_key_chord(chord, ctx);
        }
    }

    fn dispatch_key_chord(&mut self, chord: KeyChord, ctx: &egui::Context) {
        if let Some(target) = self.key_capture_target.take() {
            self.receive_key_assignment(target, chord);
            return;
        }
        if self.pending_key_assignment.is_some() {
            return;
        }
        let modifiers = chord.modifiers();
        if self.media_operation_error.is_some() {
            if chord.key() == KeyCode::Escape {
                self.media_operation_error = None;
            }
            return;
        }
        if self.project_rate_change.is_some() {
            if chord.key() == KeyCode::Escape {
                self.project_rate_change = None;
            }
            return;
        }
        if self.project_open
            && chord.key() == KeyCode::Delete
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.windows
            && !self.project_setup_open
            && !self.preferences_ui.is_open
            && self.pending_track_delete_id.is_none()
        {
            if self.midi_editor_open && !self.selected_midi_note_ids.is_empty() {
                self.project_history.finish_frame(&self.data.project, true);
                if !self.project_history.begin(&self.data.project) {
                    self.status_message =
                        "Cannot delete notes: undo snapshot is unavailable".into();
                    return;
                }
                let selected = &self.selected_midi_note_ids;
                if let Some(part_id) = self.data.project.selected_midi_part_id {
                    for track in &mut self.data.project.tracks {
                        if let Some(part) =
                            track.midi_parts.iter_mut().find(|part| part.id == part_id)
                        {
                            part.notes.retain(|note| !selected.contains(&note.id));
                            break;
                        }
                    }
                }
                self.selected_midi_note_ids.clear();
                self.selected_midi_note_id = None;
                self.project_history.finish_frame(&self.data.project, true);
                self.rebuild_audio_plan();
                self.mark_modified();
                self.status_message = "Removed selected MIDI notes; Undo restores them".into();
                return;
            }
            self.delete_audio_selection(modifiers.shift);
            return;
        }
        if self.project_open
            && chord.key() == KeyCode::Letter('P')
            && modifiers.control
            && !modifiers.alt
            && !modifiers.windows
        {
            self.focus_tab(PanelTab::Media);
            return;
        }
        if self.project_open
            && chord.key() == KeyCode::Letter('S')
            && modifiers.shift
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.windows
        {
            self.project_setup_open = true;
            return;
        }
        if self.project_open
            && chord.key() == KeyCode::Function(4)
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.shift
            && !modifiers.windows
        {
            self.vst_connections_open = !self.vst_connections_open;
            return;
        }
        let outcome = {
            let Some(profile) = self.keymap_profile.as_ref() else {
                return;
            };
            let contexts: &[&str] = if self.midi_editor_open {
                &["editor", "arrange"]
            } else {
                &["arrange"]
            };
            match profile.keymap.resolve(chord, contexts) {
                BindingResolution::Unbound => None,
                BindingResolution::Bound(binding) => {
                    let command_id = binding.command.as_str().to_owned();
                    let label = profile
                        .catalog
                        .get(&binding.command)
                        .map(|definition| definition.label.clone())
                        .unwrap_or_else(|| command_id.clone());
                    Some(Ok((command_id, label)))
                }
                BindingResolution::Conflict(bindings) => {
                    let names = bindings
                        .iter()
                        .map(|binding| {
                            profile
                                .catalog
                                .get(&binding.command)
                                .map(|definition| definition.label.as_str())
                                .unwrap_or(binding.command.as_str())
                        })
                        .collect::<Vec<_>>()
                        .join(" / ");
                    Some(Err(names))
                }
            }
        };

        match outcome {
            Some(Ok((command_id, label))) => {
                self.dispatch_reference_command(&command_id, &label, chord, ctx);
            }
            Some(Err(names)) => {
                self.status_message = format!("Conflicting reference shortcuts: {names}");
            }
            None => {
                let modifiers = chord.modifiers();
                if modifiers.control
                    && !modifiers.alt
                    && !modifiers.windows
                    && !modifiers.shift
                    && chord.key() == KeyCode::Letter('Y')
                {
                    self.redo_project();
                }
            }
        }
    }

    fn command_shortcut_label(&self, command_id: &str) -> String {
        self.keymap_profile
            .as_ref()
            .map(|profile| {
                profile
                    .keymap
                    .bindings()
                    .iter()
                    .filter(|binding| binding.command.as_str() == command_id)
                    .map(|binding| format_key_chord(binding.chord))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    }

    fn receive_key_assignment(&mut self, target: KeyCaptureTarget, chord: KeyChord) {
        let modifiers = chord.modifiers();
        if chord.key() == KeyCode::Escape
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.shift
            && !modifiers.windows
        {
            self.status_message = "Shortcut capture canceled".to_owned();
            return;
        }
        if target
            .replacing
            .as_ref()
            .is_some_and(|existing| existing.chord == chord)
        {
            self.status_message = "Shortcut unchanged".to_owned();
            return;
        }
        let Some(profile) = self.keymap_profile.as_ref() else {
            return;
        };
        if profile
            .keymap
            .bindings()
            .iter()
            .any(|binding| binding.command == target.command && binding.chord == chord)
        {
            self.status_message = "That shortcut is already assigned to this command".to_owned();
            return;
        }
        let conflicts = profile
            .keymap
            .conflicts_for(&target.command, chord, &target.scope)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        if conflicts.is_empty() {
            self.apply_key_assignment(target, chord, &[]);
        } else {
            self.pending_key_assignment = Some(PendingKeyAssignment {
                target,
                chord,
                conflicts,
            });
        }
    }

    fn apply_key_assignment(
        &mut self,
        target: KeyCaptureTarget,
        chord: KeyChord,
        conflicts: &[KeyBinding],
    ) {
        let Some(profile) = self.keymap_profile.as_mut() else {
            return;
        };
        for conflict in conflicts {
            profile.keymap.remove_binding(conflict);
        }
        if let Some(existing) = target.replacing.as_ref() {
            profile.keymap.remove_binding(existing);
        }
        profile.keymap.bind(KeyBinding {
            command: target.command.clone(),
            chord,
            scope: target.scope,
        });
        let label = profile
            .catalog
            .get(&target.command)
            .map(|definition| definition.label.as_str())
            .unwrap_or(target.command.as_str());
        self.status_message = format!("Assigned {} to {label}", format_key_chord(chord));
    }

    fn dispatch_reference_command(
        &mut self,
        command_id: &str,
        label: &str,
        chord: KeyChord,
        ctx: &egui::Context,
    ) {
        if !self.project_open && !matches!(command_id, "file.new" | "file.open" | "file.quit") {
            return;
        }
        match command_id {
            "file.new" => self.request_new_project(),
            "file.open" => self.request_open_project(),
            "file.save" => {
                self.save_project();
            }
            "file.save_as" => {
                self.save_project_as();
            }
            "file.quit" => {
                if self.has_unsaved_changes() {
                    self.pending_project_action = Some(PendingProjectAction::Close);
                    self.status_message = "Save changes before closing".to_owned();
                } else {
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
                }
            }
            "edit.undo" => self.undo_project(),
            "edit.redo" => self.redo_project(),
            "edit.copy" => self.copy_selected_audio_events(),
            "edit.cut" => self.cut_selected_audio_events(),
            "edit.paste" => self.paste_audio_events(),
            "edit.open_default_editor" => self.open_selected_midi_editor(),
            "edit.open_close_editor" => self.midi_editor_open = false,
            "devices.mixer" => self.focus_tab(PanelTab::Mixer),
            "media.open_loop_browser" => self.focus_media_category(1),
            "media.open_mediabay" => self.focus_media_category(0),
            "media.open_sound_browser" => self.focus_media_category(2),
            "transport.panel_transport_panel" => self.show_transport = !self.show_transport,
            "transport.play" => {
                if self.is_recording() {
                    self.finish_recording();
                }
                self.set_transport_playing(true);
                self.data.transport.recording = false;
            }
            "transport.start_stop" => {
                self.set_transport_playing(!self.data.transport.playing);
                self.data.transport.recording = false;
            }
            "transport.stop" => {
                self.set_transport_playing(false);
                self.data.transport.recording = false;
            }
            "transport.return_to_zero" => {
                self.data.transport.position_beats = 0.0;
                if let Some(engine) = &self.audio_engine {
                    engine.seek_seconds(0.0);
                }
            }
            "transport.cycle" => {
                self.toggle_cycle();
            }
            "transport.record" => self.toggle_recording(),
            "transport.metronome_on" => self.toggle_metronome(),
            "edit.record_enable" => self.modify_selected_track(|track| {
                track.armed = !track.armed;
            }),
            "edit.mute" => self.modify_selected_track(|track| {
                track.muted = !track.muted;
            }),
            "edit.solo" => self.modify_selected_track(|track| {
                track.solo = !track.solo;
                if track.solo {
                    track.solo_defeat = false;
                }
            }),
            "zoom.zoom_in_tracks" => self.zoom_tracks(1.2),
            "zoom.zoom_out_tracks" => self.zoom_tracks(1.0 / 1.2),
            "zoom.zoom_tracks_exclusive" => self.toggle_zoom_tracks_exclusive(),
            "tool.select_tool" => self.edit_tool = EditTool::Select,
            "tool.range_tool" => self.edit_tool = EditTool::Range,
            "tool.split_tool" => self.edit_tool = EditTool::Split,
            "tool.glue_tool" => self.edit_tool = EditTool::Glue,
            "tool.delete_tool" => self.edit_tool = EditTool::Delete,
            "tool.zoom_tool" => self.edit_tool = EditTool::Zoom,
            "tool.mute_tool" => self.edit_tool = EditTool::Mute,
            "tool.draw_tool" => self.edit_tool = EditTool::Draw,
            "tool.play_tool" => self.edit_tool = EditTool::Play,
            "tool.drumstick_tool" => self.edit_tool = EditTool::Drumstick,
            "tool.next_tool" => self.cycle_edit_tool(true),
            "tool.previous_tool" => self.cycle_edit_tool(false),
            "workspace.lock_unlock_active_workspace" => self.toggle_workspace_lock(),
            "workspace.new" => self.create_workspace(),
            "workspace.organize" => self.workspace_organizer_open = true,
            "workspace.workspace_1_to_9" => {
                if let KeyCode::NumpadDigit(digit) = chord.key() {
                    self.activate_workspace_shortcut(digit);
                }
            }
            "project.open_browser" | "project.open_close_pool" => self.focus_media_category(0),
            _ => {
                self.status_message = format!("Not implemented yet: {label}");
            }
        }
    }

    fn zoom_tracks(&mut self, scale: f32) {
        self.arrange_view_state.leave_exclusive_mode();
        self.arrange_view_state.track_vertical_zoom =
            (self.arrange_view_state.track_vertical_zoom * scale).clamp(0.5, 4.0);
        self.status_message = if scale >= 1.0 {
            "Zoomed tracks in".to_owned()
        } else {
            "Zoomed tracks out".to_owned()
        };
    }

    fn toggle_zoom_tracks_exclusive(&mut self) {
        let Some(selected_track_id) = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize))
            .map(|track| track.id)
        else {
            self.status_message = "Select a track to zoom it exclusively".to_owned();
            return;
        };

        let selected_ids = vec![selected_track_id];
        if self.arrange_view_state.exclusive_track_ids == selected_ids {
            self.arrange_view_state.leave_exclusive_mode();
            self.status_message = "Restored track heights".to_owned();
            return;
        }

        let original_heights = self
            .arrange_view_state
            .pre_exclusive_track_heights
            .clone()
            .unwrap_or_else(|| self.arrange_view_state.track_heights.clone());
        let mut exclusive_heights = original_heights.clone();
        for track in &self.data.project.tracks {
            exclusive_heights.insert(track.id, 36.0);
        }
        exclusive_heights.insert(selected_track_id, 156.0);
        self.arrange_view_state.track_heights = exclusive_heights;
        self.arrange_view_state.exclusive_track_ids = selected_ids;
        self.arrange_view_state.pre_exclusive_track_heights = Some(original_heights);
        self.status_message = "Zoomed selected track exclusively".to_owned();
    }

    fn activate_workspace_shortcut(&mut self, digit: u8) {
        if !(1..=9).contains(&digit) {
            return;
        }
        let index = if self.workspace_state.use_explicit_ids {
            WorkspaceId::new(digit).and_then(|shortcut_id| {
                self.workspace_state
                    .workspaces
                    .iter()
                    .position(|workspace| workspace.shortcut_id == Some(shortcut_id))
            })
        } else {
            let index = usize::from(digit - 1);
            (index < self.workspace_state.workspaces.len()).then_some(index)
        };
        if let Some(index) = index {
            self.activate_workspace(index);
        }
    }

    fn focus_media_category(&mut self, category: usize) {
        self.data.selected_media = category.min(3);
        self.focus_tab(PanelTab::Media);
    }

    fn set_audio_input_test(&mut self, enabled: bool) {
        self.device_test_active = enabled;
        self.update_input_capture_state();
        if !enabled {
            self.audio_input_probe_status = Some("Input test stopped".to_owned());
        }
    }

    fn project_input_monitor_requested(&self) -> bool {
        self.data.project.tracks.iter().any(|track| {
            track.kind == TrackKind::Audio
                && (track.monitor || (self.input_monitoring && track.armed))
                && track.input_routing.as_deref() != Some("No Bus")
        })
    }

    fn update_input_capture_state(&mut self) {
        let monitor_requested = self.project_input_monitor_requested() || self.input_monitoring;
        if let Some(session) = self.recording_session.as_ref() {
            self.audio_input_probe = None;
            let monitor = session.recorder.monitor_buffer();
            monitor.set_input_channel(&self.data.project.input_channel.strip);
            let enabled = monitor_requested && self.audio_engine.is_some();
            monitor.set_enabled(enabled);
            if let Some(engine) = &self.audio_engine {
                engine.set_input_monitor(enabled.then_some(monitor));
            }
            return;
        }

        let capture_requested = (self.project_open && self.audio_engine.is_some())
            || self.device_test_active
            || (self.project_input_monitor_requested() && self.audio_engine.is_some());
        if !capture_requested {
            self.audio_input_probe = None;
            if let Some(engine) = &self.audio_engine {
                engine.set_input_monitor(None);
            }
            return;
        }

        if self.audio_input_probe.is_none() {
            match AudioInputProbe::open_with_mode(
                self.selected_input_device.as_deref(),
                self.audio_device_mode,
            ) {
                Ok(probe) => {
                    self.audio_input_probe_status = Some(format!(
                        "Listening to {} · {} Hz · {} channel(s)",
                        probe.device_name, probe.sample_rate, probe.channels
                    ));
                    self.audio_input_probe = Some(probe);
                }
                Err(error) => {
                    self.audio_input_probe_status = Some(error);
                    self.device_test_active = false;
                    if let Some(engine) = &self.audio_engine {
                        engine.set_input_monitor(None);
                    }
                    return;
                }
            }
        }
        if let Some(probe) = self.audio_input_probe.as_ref() {
            let monitor = probe.monitor_buffer();
            monitor.set_input_channel(&self.data.project.input_channel.strip);
            let enabled = self.project_input_monitor_requested() && self.audio_engine.is_some();
            monitor.set_enabled(enabled);
            if let Some(engine) = &self.audio_engine {
                engine.set_input_monitor(enabled.then_some(monitor));
            }
        }
    }

    fn refresh_audio_input_devices(&mut self) {
        let inventory = scan_audio_devices();
        self.audio_device_diagnostics = inventory.diagnostics;
        let devices = inventory.devices;
        if !devices.is_empty() || self.audio_device_diagnostics.is_empty() {
            let output_missing = self
                .selected_output_device
                .as_ref()
                .is_some_and(|selected_id| {
                    !devices
                        .iter()
                        .any(|device| device.supports_output && &device.id == selected_id)
                });
            self.audio_input_devices = devices
                .iter()
                .filter(|device| device.supports_input)
                .map(|device| AudioInputDevice {
                    id: device.id.clone(),
                    name: device.name.clone(),
                })
                .collect();
            self.audio_output_devices = devices
                .iter()
                .filter(|device| device.supports_output)
                .cloned()
                .collect();
            self.audio_devices = devices;
            self.status_message = format!(
                "Found {} capture endpoint(s) and {} playback endpoint(s)",
                self.audio_input_devices.len(),
                self.audio_output_devices.len()
            );
            if output_missing {
                self.release_audio();
                self.status_message =
                    "Selected output is unavailable; choose a device in Device Setup".into();
            }
        } else {
            self.audio_devices.clear();
            self.audio_input_devices.clear();
            self.audio_output_devices.clear();
            self.status_message = format!(
                "Could not enumerate audio endpoints: {}",
                self.audio_device_diagnostics.join("; ")
            );
        }
        self.refresh_audio_input_format();
        self.refresh_audio_output_configurations();
    }

    fn refresh_audio_input_format(&mut self) {
        self.resolved_input_device_name =
            AudioRecorder::input_device_name(self.selected_input_device.as_deref()).ok();
        match AudioRecorder::input_format(self.selected_input_device.as_deref()) {
            Ok(format) => {
                self.cached_input_format = Some(format);
                self.audio_input_format_error = None;
            }
            Err(error) => {
                self.cached_input_format = None;
                self.audio_input_format_error = Some(error);
            }
        }
    }

    fn refresh_audio_output_configurations(&mut self) {
        if let Some(info) = self
            .audio_engine
            .as_ref()
            .and_then(|engine| engine.asio_info.as_ref())
        {
            self.audio_output_configs.clear();
            self.audio_output_sample_rates = info.sample_rates.clone();
            self.audio_output_buffer_choices = info.buffer_choices.clone();
            return;
        }
        self.audio_output_configs =
            match output_configurations(self.selected_output_device.as_deref()) {
                Ok(configs) => configs,
                Err(error) => {
                    self.audio_output_configs.clear();
                    self.audio_output_sample_rates.clear();
                    self.audio_output_buffer_choices.clear();
                    self.status_message = format!("Could not read audio output formats: {error}");
                    return;
                }
            };
        self.audio_output_sample_rates = audio_sample_rate_choices(&self.audio_output_configs);
        if self.audio_device_mode == AudioDeviceMode::SharedLowLatency {
            self.selected_output_sample_rate = None;
            self.audio_output_buffer_choices.clear();
            return;
        }
        if self
            .selected_output_sample_rate
            .is_some_and(|rate| !self.audio_output_sample_rates.contains(&rate))
        {
            self.selected_output_sample_rate = None;
        }
        self.audio_output_buffer_choices =
            audio_buffer_choices(&self.audio_output_configs, self.selected_output_sample_rate);
        if self
            .selected_output_buffer_frames
            .is_some_and(|frames| !self.audio_output_buffer_choices.contains(&frames))
        {
            self.selected_output_buffer_frames = None;
        }
    }

    fn apply_audio_output_settings(
        &mut self,
        device_id: Option<String>,
        sample_rate: Option<u32>,
        buffer_frames: Option<u32>,
        output_mode: AudioDeviceMode,
    ) {
        if self.is_recording() {
            self.status_message = "Stop recording before changing the audio output".to_owned();
            return;
        }
        let sample_rate = if output_mode == AudioDeviceMode::SharedLowLatency
            && !device_id
                .as_deref()
                .is_some_and(|id| id.starts_with("asio:"))
        {
            None
        } else {
            sample_rate
        };
        let previous_device = self.selected_output_device.clone();
        let previous_sample_rate = self.selected_output_sample_rate;
        let previous_buffer_frames = self.selected_output_buffer_frames;
        let previous_output_mode = self.audio_device_mode;
        let was_playing = self.data.transport.playing;
        let position_seconds = (self.data.transport.position_beats.max(0.0) * 60.0
            / self.data.project.tempo_bpm.max(1.0)) as f64;
        let tone_enabled = self
            .audio_engine
            .as_ref()
            .is_some_and(AudioEngine::test_tone_enabled);
        self.audio_input_probe = None;
        if let Some(engine) = &self.audio_engine {
            engine.set_input_monitor(None);
        }
        drop(self.audio_engine.take());

        match AudioEngine::open_output_device_mode(
            device_id.as_deref(),
            sample_rate,
            buffer_frames,
            output_mode,
        ) {
            Ok(engine) => {
                engine.set_playing(was_playing);
                engine.seek_seconds(position_seconds);
                engine.set_test_tone_enabled(tone_enabled);
                if let Some((waveform, gain)) = self.midi_monitor_profile() {
                    engine.set_live_midi_waveform(waveform);
                    engine.set_live_midi_gain(gain);
                    engine.set_live_midi_enabled(
                        self.midi_thru_enabled && self.midi_input_stream.is_some(),
                    );
                }
                let device_name = engine.device_name.clone();
                let selected_period_frames = if output_mode == AudioDeviceMode::SharedLowLatency
                    && engine.output_mode == AudioDeviceMode::SharedLowLatency
                {
                    buffer_frames.map(|requested| {
                        engine
                            .output_period_choices
                            .iter()
                            .copied()
                            .min_by_key(|period| period.abs_diff(requested))
                            .unwrap_or(engine.output_period_frames)
                    })
                } else {
                    buffer_frames
                };
                self.audio_engine = Some(engine);
                if device_id
                    .as_deref()
                    .is_some_and(|id| id.starts_with("asio:"))
                {
                    self.selected_input_device = device_id.clone();
                } else if self
                    .selected_input_device
                    .as_deref()
                    .is_some_and(|id| id.starts_with("asio:"))
                {
                    self.selected_input_device = None;
                }
                self.selected_output_device = device_id;
                self.refresh_audio_input_format();
                self.selected_output_sample_rate = sample_rate;
                self.selected_output_buffer_frames = selected_period_frames;
                self.audio_device_mode = output_mode;
                self.refresh_audio_output_configurations();
                self.rebuild_audio_plan();
                self.update_input_capture_state();
                self.status_message = format!("Audio output: {device_name}");
            }
            Err(error) => {
                let recovery = AudioEngine::open_output_device_mode(
                    previous_device.as_deref(),
                    previous_sample_rate,
                    previous_buffer_frames,
                    previous_output_mode,
                );
                match recovery {
                    Ok(engine) => {
                        engine.set_playing(was_playing);
                        engine.seek_seconds(position_seconds);
                        engine.set_test_tone_enabled(tone_enabled);
                        self.audio_engine = Some(engine);
                        self.audio_device_mode = previous_output_mode;
                        self.rebuild_audio_plan();
                        self.update_input_capture_state();
                        self.status_message = format!(
                            "Could not apply audio output settings ({error}); restored the previous device"
                        );
                    }
                    Err(recovery_error) => {
                        self.audio_engine = None;
                        self.data.transport.playing = false;
                        self.update_input_capture_state();
                        self.status_message = format!(
                            "Could not apply audio output settings ({error}); previous output could not be restored ({recovery_error})"
                        );
                    }
                }
            }
        }
    }

    fn release_audio(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before releasing the audio devices".to_owned();
            return;
        }
        self.device_test_active = false;
        self.audio_input_probe = None;
        if let Some(engine) = &self.audio_engine {
            engine.set_playing(false);
            engine.set_input_monitor(None);
            engine.set_live_midi_enabled(false);
        }
        self.audio_engine = None;
        self.data.transport.playing = false;
        self.status_message = "Audio devices released".to_owned();
    }

    fn release_exclusive_audio_if_idle(&mut self) {
        if !self.release_exclusive_audio_when_idle
            || self
                .audio_engine
                .as_ref()
                .is_none_or(|engine| engine.output_mode != AudioDeviceMode::Exclusive)
        {
            return;
        }
        let active = self.device_setup_open
            || self.project_open
            || self.data.transport.playing
            || self.is_recording()
            || self.device_test_active
            || self.project_input_monitor_requested()
            || self
                .audio_engine
                .as_ref()
                .is_some_and(AudioEngine::test_tone_enabled)
            || (self.midi_thru_enabled && self.midi_input_stream.is_some());
        if !active {
            self.release_audio();
        }
    }

    fn reopen_audio(&mut self) {
        if self.audio_engine.is_some() {
            return;
        }
        match AudioEngine::open_output_device_mode(
            self.selected_output_device.as_deref(),
            self.selected_output_sample_rate,
            self.selected_output_buffer_frames,
            self.audio_device_mode,
        ) {
            Ok(engine) => {
                let position_seconds = self.data.transport.position_beats.max(0.0) * 60.0
                    / self.data.project.tempo_bpm.max(1.0);
                engine.seek_seconds(position_seconds);
                let device_name = engine.device_name.clone();
                if let Some((waveform, gain)) = self.midi_monitor_profile() {
                    engine.set_live_midi_waveform(waveform);
                    engine.set_live_midi_gain(gain);
                    engine.set_live_midi_enabled(
                        self.midi_thru_enabled && self.midi_input_stream.is_some(),
                    );
                }
                self.audio_engine = Some(engine);
                self.rebuild_audio_plan();
                self.update_input_capture_state();
                self.status_message = format!("Audio output: {device_name}");
            }
            Err(error) => {
                self.status_message = format!("Could not reopen the audio output: {error}");
            }
        }
    }

    fn set_input_monitoring(&mut self, enabled: bool) {
        self.input_monitoring = enabled;
        self.update_input_capture_state();
    }

    fn refresh_midi_input_devices(&mut self) {
        match MidiInputStream::available_devices() {
            Ok(devices) => {
                let selected_available = self
                    .selected_midi_input_device
                    .as_ref()
                    .is_none_or(|selected| devices.iter().any(|device| &device.id == selected));
                self.midi_input_devices = devices;
                if !selected_available {
                    self.midi_input_stream = None;
                    self.selected_midi_input_device = None;
                    if let Some(engine) = &self.audio_engine {
                        engine.set_live_midi_enabled(false);
                    }
                }
                self.status_message = format!(
                    "Found {} MIDI input device(s)",
                    self.midi_input_devices.len()
                );
            }
            Err(error) => {
                self.midi_input_devices.clear();
                self.midi_input_stream = None;
                self.selected_midi_input_device = None;
                if let Some(engine) = &self.audio_engine {
                    engine.set_live_midi_enabled(false);
                }
                self.status_message = format!("Could not refresh MIDI inputs: {error}");
            }
        }
    }

    fn select_midi_input_device(&mut self, device_id: Option<String>) {
        if device_id == self.selected_midi_input_device {
            return;
        }
        self.midi_input_stream = None;
        if let Some(engine) = &self.audio_engine {
            engine.set_live_midi_enabled(false);
        }
        match device_id {
            Some(device_id) => match MidiInputStream::open(&device_id) {
                Ok(stream) => {
                    let device_name = stream.device_name.clone();
                    self.midi_input_stream = Some(stream);
                    self.selected_midi_input_device = Some(device_id);
                    self.status_message = format!("MIDI input set to {device_name}");
                }
                Err(error) => {
                    self.selected_midi_input_device = None;
                    self.status_message = format!("Could not open MIDI input: {error}");
                }
            },
            None => {
                self.selected_midi_input_device = None;
                self.status_message = "MIDI input disconnected".to_owned();
            }
        }
    }

    fn midi_monitor_profile(&self) -> Option<(InstrumentWaveform, f32)> {
        let project = &self.data.project;
        let any_solo = project.tracks.iter().any(|track| track.solo);
        let track = project
            .selected_track
            .and_then(|index| project.tracks.get(index as usize))
            .filter(|track| matches!(track.kind, TrackKind::Midi | TrackKind::Instrument))
            .filter(|track| track.armed)
            .or_else(|| {
                project.tracks.iter().find(|track| {
                    matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) && track.armed
                })
            })?;
        let gain = if track.muted || (any_solo && !track.solo) {
            0.0
        } else {
            track.gain
        };
        let waveform = if track.kind == TrackKind::Instrument {
            track.instrument_waveform
        } else {
            InstrumentWaveform::Sine
        };
        Some((waveform, gain))
    }

    fn handle_midi_input_events(&mut self, ctx: &egui::Context) {
        let Some(stream) = self.midi_input_stream.as_ref() else {
            if let Some(engine) = &self.audio_engine {
                engine.set_live_midi_enabled(false);
            }
            return;
        };

        ctx.request_repaint_after(std::time::Duration::from_millis(8));
        let device_name = stream.device_name.clone();
        let dropped_messages = stream.dropped_messages();
        let mut events = std::mem::take(&mut self.midi_event_scratch);
        events.clear();
        stream.drain_into(&mut events);

        let monitor_profile = self.midi_monitor_profile();
        let monitor_enabled =
            self.midi_thru_enabled && self.audio_engine.is_some() && monitor_profile.is_some();
        if let Some(engine) = &self.audio_engine {
            engine.set_live_midi_enabled(monitor_enabled);
            if let Some((waveform, gain)) = monitor_profile {
                engine.set_live_midi_waveform(waveform);
                engine.set_live_midi_gain(gain);
            }
            if monitor_enabled {
                for event in &events {
                    if !engine.send_live_midi_message(event.message) {
                        self.status_message =
                            "MIDI monitor queue is full; an event was dropped".to_owned();
                    }
                }
            }
        }

        let tempo = self.data.project.tempo_bpm;
        let transport_beat = self.data.transport.position_beats;
        let now = Instant::now();
        let mut recorded = false;
        if self.midi_recording_session.is_some() {
            for event in &events {
                let age = now
                    .saturating_duration_since(event.received_at)
                    .as_secs_f64();
                let event_beat = (transport_beat - age * tempo / 60.0).max(0.0);
                if let Some(session) = self.midi_recording_session.as_mut() {
                    recorded |= record_midi_message(
                        &mut self.data.project,
                        session,
                        event.message,
                        event_beat,
                    );
                }
            }
        }
        if recorded {
            self.modified = true;
        }

        if dropped_messages > self.midi_reported_dropped_messages {
            let newly_dropped = dropped_messages - self.midi_reported_dropped_messages;
            self.midi_reported_dropped_messages = dropped_messages;
            self.status_message =
                format!("MIDI input queue dropped {newly_dropped} message(s) from {device_name}");
        }
        if !events.is_empty() {
            ctx.request_repaint();
        }
        self.midi_event_scratch = events;
    }

    fn toggle_recording(&mut self) {
        if !self.project_open {
            self.status_message = "Create or open a project before recording".to_owned();
            return;
        }
        if self.is_recording() {
            self.finish_recording();
            self.set_transport_playing(true);
            return;
        }

        let armed_audio_tracks: Vec<_> = self
            .data
            .project
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio && track.armed)
            .map(|track| {
                let default_input = if track.audio_channels == 1 {
                    "Left - Stereo In"
                } else {
                    "Stereo In"
                };
                let route = match track.input_routing.as_deref().unwrap_or(default_input) {
                    "Left - Stereo In" => Some(AudioRecordRoute::Left),
                    "Right - Stereo In" => Some(AudioRecordRoute::Right),
                    "No Bus" => None,
                    _ => Some(AudioRecordRoute::Stereo),
                };
                (track.id, track.name.clone(), route)
            })
            .collect();
        let armed_midi_tracks: Vec<_> = self
            .data
            .project
            .tracks
            .iter()
            .filter(|track| {
                matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) && track.armed
            })
            .map(|track| (track.id, track.name.clone()))
            .collect();
        if armed_audio_tracks.is_empty() && armed_midi_tracks.is_empty() {
            self.status_message =
                "Arm an audio, MIDI, or Instrument track before recording".to_owned();
            return;
        }
        if let Some((_, track_name, None)) = armed_audio_tracks
            .iter()
            .find(|(_, _, route)| route.is_none())
        {
            self.status_message =
                format!("Track {track_name} is armed but has no input bus assigned");
            return;
        }
        if armed_audio_tracks.is_empty()
            && !armed_midi_tracks.is_empty()
            && self.midi_input_stream.is_none()
        {
            self.status_message = "Select a MIDI input device before recording MIDI".to_owned();
            return;
        }

        let start_beats = self
            .audio_engine
            .as_ref()
            .map(|engine| engine.position_seconds() * self.data.project.tempo_bpm / 60.0)
            .unwrap_or(self.data.transport.position_beats);
        let mut audio_recorder = None;
        if !armed_audio_tracks.is_empty() {
            if !self.project_folder_managed {
                self.begin_project_folder_setup();
                self.status_message =
                    "Choose or create this project's folder before recording audio".to_owned();
                return;
            }
            let Some(project_root) = self
                .project_path
                .as_deref()
                .and_then(Path::parent)
                .filter(|path| !path.as_os_str().is_empty())
            else {
                self.status_message =
                    "Save this project into its folder before recording audio".to_owned();
                return;
            };
            let recording_directory = project_root.join("Audio").join("Recordings");
            if let Err(error) = fs::create_dir_all(&recording_directory) {
                self.status_message =
                    format!("Could not prepare the project's recording folder: {error}");
                return;
            }
            let Some(paths) = next_recording_paths(
                &recording_directory,
                &armed_audio_tracks,
                &self.project_history.reserved_recording_paths(),
            ) else {
                self.status_message =
                    "Could not find an unused take name in Audio/Recordings".to_owned();
                return;
            };
            let routes: Vec<_> = armed_audio_tracks
                .iter()
                .filter_map(|(_, _, route)| *route)
                .collect();
            self.device_test_active = false;
            self.audio_input_probe = None;
            if let Some(engine) = &self.audio_engine {
                engine.set_input_monitor(None);
            }
            audio_recorder = match AudioRecorder::start_input_paths_routed_mode_with_channel(
                &paths,
                &routes,
                self.selected_input_device.as_deref(),
                self.audio_device_mode,
                Some(self.data.project.sample_rate_hz),
                self.data.project.recording_format,
                &self.data.project.input_channel.strip,
            ) {
                Ok(recorder) => Some(recorder),
                Err(error) => {
                    self.status_message = format!("Could not start audio recording: {error}");
                    self.update_input_capture_state();
                    return;
                }
            };
        }

        self.project_history.begin(&self.data.project);
        let mut midi_tracks = Vec::new();
        if self.midi_input_stream.is_some() {
            let part_duration = f64::from(self.data.project.beats_per_bar.max(1)) * 4.0;
            for (track_id, _) in &armed_midi_tracks {
                let Some(track_index) = self
                    .data
                    .project
                    .tracks
                    .iter()
                    .position(|track| track.id == *track_id)
                else {
                    continue;
                };
                if let Some(part_id) =
                    self.data
                        .project
                        .add_midi_part(track_index, start_beats, part_duration)
                {
                    midi_tracks.push(MidiRecordingTrack {
                        track_id: *track_id,
                        part_id,
                        open_notes: HashMap::new(),
                    });
                }
            }
        }
        if audio_recorder.is_none() && midi_tracks.is_empty() {
            self.status_message = if armed_midi_tracks.is_empty() {
                "No armed track is ready to record".to_owned()
            } else {
                "Could not create a MIDI part on the armed track(s)".to_owned()
            };
            return;
        }

        if let Some(recorder) = audio_recorder {
            self.recording_session = Some(RecordingSession {
                routes: armed_audio_tracks
                    .iter()
                    .filter_map(|(_, _, route)| *route)
                    .collect(),
                recorder,
                track_ids: armed_audio_tracks
                    .iter()
                    .map(|(track_id, _, _)| *track_id)
                    .collect(),
                start_beats,
            });
            self.update_input_capture_state();
        }
        if !midi_tracks.is_empty() {
            self.midi_recording_session = Some(MidiRecordingSession {
                tracks: midi_tracks,
                start_beats,
                captured_events: 0,
            });
            self.modified = true;
        }
        self.data.transport.position_beats = start_beats;
        self.data.transport.recording = true;
        self.set_transport_playing(true);

        let mut status_parts = Vec::new();
        if let Some(session) = &self.recording_session {
            let audio_description = if armed_audio_tracks.len() == 1 {
                format!("audio {}", armed_audio_tracks[0].1)
            } else {
                format!("audio on {} tracks", armed_audio_tracks.len())
            };
            status_parts.push(format!(
                "{audio_description} from {} (capture {} Hz → record {} Hz, {} ch, {}; {} · {} frame period)",
                session.recorder.device_name,
                session.recorder.capture_sample_rate,
                session.recorder.sample_rate,
                session.recorder.channels,
                session.recorder.recording_format.label(),
                session.recorder.input_mode_detail,
                session.recorder.input_period_frames
            ));
        }
        if let Some(session) = &self.midi_recording_session {
            let input_name = self
                .midi_input_stream
                .as_ref()
                .map(|stream| stream.device_name.as_str())
                .unwrap_or("MIDI input");
            status_parts.push(format!(
                "MIDI on {} track(s) from {input_name}",
                session.tracks.len()
            ));
        } else if !armed_midi_tracks.is_empty() {
            status_parts.push("armed MIDI tracks have no connected MIDI input".to_owned());
        }
        self.status_message = format!("Recording {}", status_parts.join(" + "));
        if self.midi_recording_session.is_some() {
            self.rebuild_audio_plan();
        }
    }

    fn finish_recording(&mut self) {
        if let Some(session) = self.midi_recording_session.take() {
            let event_count = session.captured_events;
            let track_count = session.tracks.len();
            let end_beats = self.data.transport.position_beats;
            let beats_per_bar = f64::from(self.data.project.beats_per_bar.max(1));
            finish_midi_recording(&mut self.data.project, session, end_beats, beats_per_bar);
            self.project_history.commit_after_frame = true;
            self.modified = true;
            self.rebuild_audio_plan();
            self.status_message =
                format!("Recorded {event_count} MIDI event(s) on {track_count} track(s)");
        }
        let Some(session) = self.recording_session.take() else {
            self.data.transport.recording = false;
            return;
        };
        self.finish_audio_recording(session);
    }

    fn finish_audio_recording(&mut self, session: RecordingSession) {
        session.recorder.monitor_buffer().set_enabled(false);
        if let Some(engine) = &self.audio_engine {
            engine.set_input_monitor(None);
        }
        self.data.transport.recording = false;
        let recording_path = session.recorder.path.clone();
        let track_ids = session.track_ids;
        let start_beats = session.start_beats;
        let summaries = match session.recorder.finish_all() {
            Ok(summaries) => summaries,
            Err(error) => {
                self.update_input_capture_state();
                self.status_message = format!(
                    "Recording stopped with an error at {}: {error}",
                    recording_path.display()
                );
                return;
            }
        };
        self.update_input_capture_state();
        let mut recorded_names = Vec::new();
        let mut recorded_paths = Vec::new();
        let mut issues = Vec::new();
        let mut dropped_frames = 0_u64;
        let mut monitor_dropped_frames = 0_u64;
        let mut input_stream_failed = false;
        let mut input_discontinuities = 0_u64;
        let mut input_timestamp_errors = 0_u64;
        for (track_id, summary) in track_ids.into_iter().zip(summaries) {
            dropped_frames = dropped_frames.max(summary.dropped_frames);
            monitor_dropped_frames = monitor_dropped_frames.max(summary.monitor_dropped_frames);
            input_stream_failed |= summary.input_stream_failed;
            input_discontinuities = input_discontinuities.max(summary.input_discontinuities);
            input_timestamp_errors = input_timestamp_errors.max(summary.input_timestamp_errors);
            if summary.frames_written == 0 {
                let _ = std::fs::remove_file(&summary.path);
                continue;
            }

            let source = match AudioSource::open(&summary.path) {
                Ok(source) => source,
                Err(error) => {
                    issues.push(format!(
                        "{} could not be loaded: {error}",
                        summary.path.display()
                    ));
                    continue;
                }
            };
            let Some(track_index) = self
                .data
                .project
                .tracks
                .iter()
                .position(|track| track.id == track_id && track.kind == TrackKind::Audio)
            else {
                issues.push(format!(
                    "{} was saved but its track no longer exists",
                    summary.path.display()
                ));
                continue;
            };
            if self
                .data
                .project
                .add_audio_clip(
                    track_index,
                    summary.path.clone(),
                    start_beats,
                    source.duration_seconds(),
                )
                .is_none()
            {
                issues.push(format!(
                    "{} was saved but could not be placed on its track",
                    summary.path.display()
                ));
                continue;
            }
            recorded_paths.push(summary.path.clone());
            recorded_names.push(
                summary
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "take.wav".to_owned()),
            );
            self.data.audio_sources.insert(summary.path.clone(), source);
        }

        if recorded_names.is_empty() {
            self.status_message = if issues.is_empty() {
                "No input audio was captured; no takes were added".to_owned()
            } else {
                issues.join(" · ")
            };
            return;
        }

        let undo_root = recording_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.join("Metadata").join("Undo"))
            .unwrap_or_else(|| std::env::temp_dir().join("OpenTrack-Undo"));
        self.project_history
            .unsaved_recordings
            .extend(recorded_paths.iter().cloned());
        self.project_history.pending_recording = Some(recording_undo::RecordingUndo::new(
            recorded_paths,
            undo_root,
        ));
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        let mut status = format!(
            "Recorded {} take(s): {}",
            recorded_names.len(),
            recorded_names.join(", ")
        );
        if dropped_frames > 0 {
            status.push_str(&format!(
                "; {dropped_frames} input frames dropped because the writer could not keep up"
            ));
        }
        if monitor_dropped_frames > 0 {
            status.push_str(&format!(
                "; live monitor dropped {} input frames",
                monitor_dropped_frames
            ));
        }
        if input_stream_failed {
            status.push_str("; the input device reported a stream error");
        }
        if input_discontinuities > 0 {
            status.push_str(&format!(
                "; the capture engine reported {input_discontinuities} discontinuity packet(s)"
            ));
        }
        if input_timestamp_errors > 0 {
            status.push_str(&format!(
                "; the capture engine reported {input_timestamp_errors} invalid timestamp packet(s)"
            ));
        }
        if !issues.is_empty() {
            status.push_str("; ");
            status.push_str(&issues.join(" · "));
        }
        self.status_message = status;
    }

    fn modify_selected_track(&mut self, modify: impl FnOnce(&mut Track)) {
        let Some(index) = self.data.project.selected_track.map(|index| index as usize) else {
            self.status_message = "Select a track first".to_owned();
            return;
        };
        let Some(track) = self.data.project.tracks.get_mut(index) else {
            self.status_message = "Select a track first".to_owned();
            return;
        };
        modify(track);
        self.rebuild_audio_plan();
        self.mark_modified();
    }

    fn open_selected_track_rename(&mut self) {
        let Some(track) = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize))
        else {
            self.status_message = "Select a track first".to_owned();
            return;
        };
        self.track_rename_dialog = Some(TrackRenameDialog {
            track_id: track.id,
            draft: track.name.clone(),
        });
    }

    fn move_selected_track(&mut self, offset: isize) {
        let Some(index) = self
            .data
            .project
            .selected_track
            .map(|index| index as usize)
            .filter(|index| *index < self.data.project.tracks.len())
        else {
            return;
        };
        let destination = index as isize + offset;
        if destination < 0 || destination >= self.data.project.tracks.len() as isize {
            return;
        }
        if self.data.project.move_track(index, destination as usize) {
            self.rebuild_audio_plan();
            self.mark_modified();
            self.status_message = "Track order changed".to_owned();
        }
    }

    fn request_selected_track_delete(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before removing a track".to_owned();
            return;
        }
        self.pending_track_delete_id = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize))
            .map(|track| track.id);
    }

    fn duplicate_tracks(&mut self, clicked_track_id: Option<u64>) {
        if self.is_recording() {
            self.status_message = "Stop recording before duplicating tracks".to_owned();
            return;
        }
        let ids = if let Some(clicked_id) = clicked_track_id {
            if self
                .arrange_view_state
                .selected_track_ids
                .contains(&clicked_id)
            {
                self.arrange_view_state
                    .selected_track_ids
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
            } else {
                vec![clicked_id]
            }
        } else if !self.arrange_view_state.selected_track_ids.is_empty() {
            self.arrange_view_state
                .selected_track_ids
                .iter()
                .copied()
                .collect::<Vec<_>>()
        } else {
            self.data
                .project
                .selected_track
                .and_then(|index| self.data.project.tracks.get(index as usize))
                .map(|track| vec![track.id])
                .unwrap_or_default()
        };
        let Some(duplicated) = self.data.project.duplicate_tracks(&ids) else {
            self.status_message = "Could not duplicate the selected tracks".to_owned();
            return;
        };
        self.arrange_view_state.selected_track_ids = duplicated.iter().copied().collect();
        self.arrange_view_state.selection_anchor_track_id = duplicated.first().copied();
        self.selected_midi_note_id = None;
        self.midi_note_drag = None;
        self.midi_note_draw = None;
        self.midi_controller_drag = None;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Duplicated {} track{}; undo is available",
            duplicated.len(),
            if duplicated.len() == 1 { "" } else { "s" }
        );
    }

    fn remove_track_by_id(&mut self, track_id: u64) {
        if self.is_recording() {
            self.status_message = "Stop recording before removing a track".to_owned();
            return;
        }
        let Some(index) = self
            .data
            .project
            .tracks
            .iter()
            .position(|track| track.id == track_id)
        else {
            self.status_message = "The track no longer exists".to_owned();
            return;
        };
        let track_name = self.data.project.tracks[index].name.clone();
        if self.data.project.remove_track(index).is_some() {
            self.selected_midi_note_id = None;
            self.midi_note_drag = None;
            self.midi_note_draw = None;
            self.midi_controller_drag = None;
            self.rebuild_audio_plan();
            self.mark_modified();
            self.status_message = format!("Removed track {track_name}; undo is available");
        }
    }

    fn rename_track_by_id(&mut self, track_id: u64, name: &str) {
        let Some(index) = self
            .data
            .project
            .tracks
            .iter()
            .position(|track| track.id == track_id)
        else {
            self.status_message = "The track no longer exists".to_owned();
            return;
        };
        if self.data.project.rename_track(index, name) {
            self.mark_modified();
            self.status_message = "Track renamed".to_owned();
        } else if name.trim().is_empty() {
            self.status_message = "A track name cannot be empty".to_owned();
        }
    }

    fn cycle_edit_tool(&mut self, forward: bool) {
        const TOOLS: [EditTool; 10] = [
            EditTool::Select,
            EditTool::Range,
            EditTool::Split,
            EditTool::Glue,
            EditTool::Delete,
            EditTool::Zoom,
            EditTool::Mute,
            EditTool::Draw,
            EditTool::Play,
            EditTool::Drumstick,
        ];
        let index = TOOLS
            .iter()
            .position(|tool| *tool == self.edit_tool)
            .unwrap_or_default();
        let next = if forward {
            (index + 1) % TOOLS.len()
        } else {
            (index + TOOLS.len() - 1) % TOOLS.len()
        };
        self.edit_tool = TOOLS[next];
    }

    fn move_workspace(&mut self, from: usize, to: usize) {
        if from == to
            || from >= self.workspace_state.workspaces.len()
            || to >= self.workspace_state.workspaces.len()
        {
            return;
        }
        self.workspace_state.workspaces.swap(from, to);
        self.workspace_state.active_index = self.workspace_state.active_index.map(|active| {
            if active == from {
                to
            } else if active == to {
                from
            } else {
                active
            }
        });
        self.status_message = "Workspace order updated".to_owned();
    }

    fn toggle_workspace_lock_at(&mut self, index: usize) {
        if self.workspace_state.active_index == Some(index) {
            self.toggle_workspace_lock();
        } else if let Some(workspace) = self.workspace_state.workspaces.get_mut(index) {
            workspace.locked = !workspace.locked;
            self.status_message = if workspace.locked {
                format!("Locked workspace: {}", workspace.name)
            } else {
                format!("Unlocked workspace: {}", workspace.name)
            };
        }
    }

    fn show_key_commands_window(&mut self, ctx: &egui::Context) {
        if !self.key_commands_open
            && self.key_capture_target.is_none()
            && self.pending_key_assignment.is_none()
        {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("key-commands-window"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        }

        let (categories, rows) = self
            .keymap_profile
            .as_ref()
            .map(|profile| {
                let categories = profile
                    .catalog
                    .iter()
                    .map(|definition| definition.category.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let rows = profile
                    .catalog
                    .iter()
                    .map(|definition| {
                        let bindings = profile
                            .keymap
                            .bindings()
                            .iter()
                            .filter(|binding| binding.command == definition.id)
                            .cloned()
                            .collect::<Vec<_>>();
                        (
                            definition.id.clone(),
                            definition.label.clone(),
                            definition.category.clone(),
                            bindings,
                        )
                    })
                    .collect::<Vec<_>>();
                (categories, rows)
            })
            .unwrap_or_default();

        let mut open = self.key_commands_open;
        let mut selected_category = self.key_command_category.clone();
        let mut action = None;
        let response = egui::Window::new("Key Commands")
            .id(Id::new("key-commands-window"))
            .open(&mut open)
            .default_size(Vec2::new(760.0, 620.0))
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("key-command-category")
                        .selected_text(
                            selected_category
                                .as_deref()
                                .unwrap_or("All categories"),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut selected_category,
                                None,
                                "All categories",
                            );
                            for category in &categories {
                                ui.selectable_value(
                                    &mut selected_category,
                                    Some(category.clone()),
                                    category,
                                );
                            }
                        });
                    if ui.button("Restore Cubase 5 Windows defaults").clicked() {
                        action = Some(KeyCommandUiAction::RestoreDefaults);
                    }
                });
                ui.label(
                    egui::RichText::new(
                        "Assign adds a shortcut. Change replaces one assignment. Conflicts need confirmation.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                if self.key_capture_target.is_some() {
                    ui.label(
                        egui::RichText::new("Press the desired key combination…")
                            .strong()
                            .color(theme::palette(ui).accent),
                    );
                }
                ui.separator();

                if self.keymap_profile.is_none() {
                    ui.label("The reference key command list could not be loaded.");
                    return;
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (command, label, category, bindings) in &rows {
                            if selected_category
                                .as_deref()
                                .is_some_and(|selected| selected != category)
                            {
                                continue;
                            }
                            ui.group(|ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(label).strong());
                                        ui.label(
                                            egui::RichText::new(category)
                                                .small()
                                                .color(theme::palette(ui).muted),
                                        );
                                    });
                                    if bindings.is_empty() {
                                        ui.weak("Unassigned");
                                    }
                                    for binding in bindings {
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.monospace(format_key_chord(binding.chord));
                                                if ui.small_button("Change").clicked() {
                                                    action = Some(
                                                        KeyCommandUiAction::BeginCapture(
                                                            KeyCaptureTarget {
                                                                command: command.clone(),
                                                                scope: binding.scope.clone(),
                                                                replacing: Some(binding.clone()),
                                                            },
                                                        ),
                                                    );
                                                }
                                                if ui.small_button("Remove").clicked() {
                                                    action = Some(KeyCommandUiAction::Remove(
                                                        binding.clone(),
                                                    ));
                                                }
                                            });
                                        });
                                    }
                                    if ui.small_button("Add shortcut").clicked() {
                                        let scope = bindings
                                            .first()
                                            .map(|binding| binding.scope.clone())
                                            .unwrap_or_else(|| default_key_scope(command));
                                        action = Some(KeyCommandUiAction::BeginCapture(
                                            KeyCaptureTarget {
                                                command: command.clone(),
                                                scope,
                                                replacing: None,
                                            },
                                        ));
                                    }
                                });
                            });
                        }
                    });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("key-commands-window"),
            &mut open,
            response.as_ref(),
        );
        self.key_commands_open = open;
        self.key_command_category = selected_category;
        if !open && self.key_capture_target.is_some() {
            self.key_capture_target = None;
            self.status_message = "Shortcut capture canceled".to_owned();
        }

        match action {
            Some(KeyCommandUiAction::BeginCapture(target)) => {
                self.pending_key_assignment = None;
                self.key_capture_target = Some(target);
            }
            Some(KeyCommandUiAction::Remove(binding)) => {
                let label = self
                    .keymap_profile
                    .as_ref()
                    .and_then(|profile| profile.catalog.get(&binding.command))
                    .map(|definition| definition.label.clone())
                    .unwrap_or_else(|| binding.command.as_str().to_owned());
                if let Some(profile) = self.keymap_profile.as_mut() {
                    profile.keymap.remove_binding(&binding);
                }
                self.status_message =
                    format!("Removed {} from {}", format_key_chord(binding.chord), label);
            }
            Some(KeyCommandUiAction::RestoreDefaults) => {
                match KeymapProfile::windows_reference_default() {
                    Ok(profile) => {
                        self.keymap_profile = Some(profile);
                        self.status_message = "Restored Cubase 5 Windows key commands".to_owned();
                    }
                    Err(error) => {
                        self.status_message = format!("Could not restore key commands: {error}");
                    }
                }
            }
            None => {}
        }

        self.show_key_capture_prompt(ctx);
        self.show_key_conflict_prompt(ctx);
    }

    fn show_key_capture_prompt(&mut self, ctx: &egui::Context) {
        let Some(target) = self.key_capture_target.as_ref() else {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("key-command-capture"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        };
        let label = self
            .keymap_profile
            .as_ref()
            .and_then(|profile| profile.catalog.get(&target.command))
            .map(|definition| definition.label.clone())
            .unwrap_or_else(|| target.command.as_str().to_owned());
        let mut open = true;
        let mut cancel = false;
        let response = egui::Window::new("Assign Shortcut")
            .id(Id::new("key-command-capture"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Press a shortcut for {label}. Escape cancels."));
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("key-command-capture"),
            &mut open,
            response.as_ref(),
        );
        if cancel || !open {
            self.key_capture_target = None;
            self.status_message = "Shortcut capture canceled".to_owned();
        }
    }

    fn show_key_conflict_prompt(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_key_assignment.clone() else {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("key-command-conflict"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        };
        let (command_label, conflict_labels) = self
            .keymap_profile
            .as_ref()
            .map(|profile| {
                let command_label = profile
                    .catalog
                    .get(&pending.target.command)
                    .map(|definition| definition.label.clone())
                    .unwrap_or_else(|| pending.target.command.as_str().to_owned());
                let conflicts = pending
                    .conflicts
                    .iter()
                    .map(|binding| {
                        profile
                            .catalog
                            .get(&binding.command)
                            .map(|definition| definition.label.clone())
                            .unwrap_or_else(|| binding.command.as_str().to_owned())
                    })
                    .collect::<Vec<_>>();
                (command_label, conflicts)
            })
            .unwrap_or_else(|| (pending.target.command.as_str().to_owned(), Vec::new()));
        let mut open = true;
        let mut decision = None;
        let response = egui::Window::new("Shortcut conflict")
            .id(Id::new("key-command-conflict"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "{} is already assigned to {}.",
                    format_key_chord(pending.chord),
                    conflict_labels.join(", ")
                ));
                ui.label(format!("Assign it to {command_label} instead?"));
                ui.horizontal(|ui| {
                    if ui.button("Replace conflicting assignment(s)").clicked() {
                        decision = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        decision = Some(false);
                    }
                });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("key-command-conflict"),
            &mut open,
            response.as_ref(),
        );
        if !open {
            decision = Some(false);
        }
        if let Some(replace) = decision {
            self.pending_key_assignment = None;
            if replace {
                self.apply_key_assignment(pending.target, pending.chord, &pending.conflicts);
            } else {
                self.status_message = "Shortcut change canceled".to_owned();
            }
        }
    }

    fn show_workspace_organizer(&mut self, ctx: &egui::Context) {
        if !self.workspace_organizer_open {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("workspace-organizer"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        }

        let mut open = true;
        let mut activate_index = None;
        let mut move_request = None;
        let mut lock_index = None;
        let mut create_workspace = false;
        let mut preset_action = None;
        let mut project_preset_renamed = false;
        let response = egui::Window::new("Workspace Organizer")
            .id(Id::new("workspace-organizer"))
            .open(&mut open)
            .collapsible(false)
            .default_size(Vec2::new(760.0, 690.0))
            .show(ctx, |ui| {
                ui.checkbox(
                    &mut self.workspace_state.use_explicit_ids,
                    "Use IDs for workspace shortcuts",
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Order");
                    ui.label("Workspace name");
                    ui.label("Shortcut ID");
                    ui.label("Recall lock");
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let workspace_count = self.workspace_state.workspaces.len();
                    for index in 0..workspace_count {
                        let active = self.workspace_state.active_index == Some(index);
                        let use_explicit_ids = self.workspace_state.use_explicit_ids;
                        let current_id = self.workspace_state.workspaces[index].shortcut_id;
                        let id_options: Vec<_> = (1..=9)
                            .filter_map(WorkspaceId::new)
                            .filter(|id| {
                                current_id == Some(*id)
                                    || !self.workspace_state.workspaces.iter().enumerate().any(
                                        |(other_index, workspace)| {
                                            other_index != index
                                                && workspace.shortcut_id == Some(*id)
                                        },
                                    )
                            })
                            .collect();

                        let workspace = &mut self.workspace_state.workspaces[index];
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(active, format!("{}", index + 1))
                                .clicked()
                            {
                                activate_index = Some(index);
                            }
                            let name = ui.add(
                                egui::TextEdit::singleline(&mut workspace.name)
                                    .desired_width(190.0),
                            );
                            if name.lost_focus() && workspace.name.trim().is_empty() {
                                workspace.name = format!("Workspace {}", index + 1);
                            }
                            ui.add_enabled_ui(use_explicit_ids, |ui| {
                                egui::ComboBox::from_id_salt(("workspace-shortcut-id", index))
                                    .selected_text(workspace.shortcut_id.map_or_else(
                                        || "None".to_owned(),
                                        |id| id.get().to_string(),
                                    ))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut workspace.shortcut_id,
                                            None,
                                            "None",
                                        );
                                        for id in &id_options {
                                            ui.selectable_value(
                                                &mut workspace.shortcut_id,
                                                Some(*id),
                                                id.get().to_string(),
                                            );
                                        }
                                    });
                            });
                            if ui
                                .small_button(if workspace.locked { "Unlock" } else { "Lock" })
                                .clicked()
                            {
                                lock_index = Some(index);
                            }
                            if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
                                move_request = Some((index, index - 1));
                            }
                            if ui
                                .add_enabled(index + 1 < workspace_count, egui::Button::new("↓"))
                                .clicked()
                            {
                                move_request = Some((index, index + 1));
                            }
                        });
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.workspace_state.workspaces.len() < 9,
                            egui::Button::new("New Workspace"),
                        )
                        .clicked()
                    {
                        create_workspace = true;
                    }
                    ui.label("Alt+Numpad1..9 selects; Alt+Numpad0 locks.");
                });

                ui.separator();
                ui.collapsing("Workspace Presets", |ui| {
                    if ui
                        .checkbox(
                            &mut self.workspace_preferences.auto_instantiate_presets,
                            "Auto instantiate global presets when creating or opening projects",
                        )
                        .changed()
                    {
                        self.status_message = if self.workspace_preferences.auto_instantiate_presets
                        {
                            "Global workspace presets will be added to new and opened projects"
                                .to_owned()
                        } else {
                            "Automatic workspace preset instantiation disabled".to_owned()
                        };
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Save Active as Global Preset").clicked() {
                            preset_action = Some(WorkspacePresetUiAction::SaveActiveGlobal);
                        }
                        if ui.button("Save Active as Project Preset").clicked() {
                            preset_action = Some(WorkspacePresetUiAction::SaveActiveProject);
                        }
                        if ui
                            .add_enabled(
                                !self.workspace_preferences.global_presets.is_empty(),
                                egui::Button::new("Copy Global Presets to Project"),
                            )
                            .clicked()
                        {
                            preset_action = Some(WorkspacePresetUiAction::CopyGlobalToProject);
                        }
                    });

                    ui.separator();
                    ui.label(egui::RichText::new("Global presets").strong());
                    let global_count = self.workspace_preferences.global_presets.len();
                    if global_count == 0 {
                        ui.weak("No global presets saved");
                    }
                    egui::ScrollArea::vertical()
                        .max_height(145.0)
                        .show(ui, |ui| {
                            for index in 0..global_count {
                                let preset = &mut self.workspace_preferences.global_presets[index];
                                ui.horizontal(|ui| {
                                    let name = ui.add(
                                        egui::TextEdit::singleline(&mut preset.name)
                                            .desired_width(230.0),
                                    );
                                    if name.lost_focus() && preset.name.trim().is_empty() {
                                        preset.name = format!("Global Preset {}", index + 1);
                                    }
                                    if ui.small_button("Create Workspace").clicked() {
                                        preset_action =
                                            Some(WorkspacePresetUiAction::CreateGlobal(index));
                                    }
                                    if ui.small_button("Remove").clicked() {
                                        preset_action =
                                            Some(WorkspacePresetUiAction::RemoveGlobal(index));
                                    }
                                });
                            }
                        });

                    ui.separator();
                    ui.label(egui::RichText::new("Project presets").strong());
                    let project_count = self.workspace_state.project_presets.len();
                    if project_count == 0 {
                        ui.weak(
                            "No project presets; copy global presets or save the active workspace",
                        );
                    }
                    egui::ScrollArea::vertical()
                        .max_height(145.0)
                        .show(ui, |ui| {
                            for index in 0..project_count {
                                let preset = &mut self.workspace_state.project_presets[index];
                                ui.horizontal(|ui| {
                                    let name = ui.add(
                                        egui::TextEdit::singleline(&mut preset.name)
                                            .desired_width(230.0),
                                    );
                                    if name.changed() {
                                        project_preset_renamed = true;
                                    }
                                    if name.lost_focus() && preset.name.trim().is_empty() {
                                        preset.name = format!("Project Preset {}", index + 1);
                                        project_preset_renamed = true;
                                    }
                                    if ui.small_button("Create Workspace").clicked() {
                                        preset_action =
                                            Some(WorkspacePresetUiAction::CreateProject(index));
                                    }
                                    if ui.small_button("Remove").clicked() {
                                        preset_action =
                                            Some(WorkspacePresetUiAction::RemoveProject(index));
                                    }
                                });
                            }
                        });
                });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("workspace-organizer"),
            &mut open,
            response.as_ref(),
        );
        self.workspace_organizer_open = open;

        if let Some(index) = activate_index {
            self.activate_workspace(index);
        }
        if let Some((from, to)) = move_request {
            self.move_workspace(from, to);
        }
        if let Some(index) = lock_index {
            self.toggle_workspace_lock_at(index);
        }
        if create_workspace {
            self.create_workspace();
        }
        if project_preset_renamed {
            self.mark_modified();
        }
        match preset_action {
            Some(WorkspacePresetUiAction::SaveActiveGlobal) => {
                self.save_active_workspace_as_global_preset();
            }
            Some(WorkspacePresetUiAction::SaveActiveProject) => {
                self.save_active_workspace_as_project_preset();
            }
            Some(WorkspacePresetUiAction::CopyGlobalToProject) => {
                self.copy_global_workspace_presets_to_project();
            }
            Some(WorkspacePresetUiAction::CreateGlobal(index)) => {
                if let Some(preset) = self
                    .workspace_preferences
                    .global_presets
                    .get(index)
                    .cloned()
                {
                    self.create_workspace_from_preset(preset.clone(), Some(preset.name));
                }
            }
            Some(WorkspacePresetUiAction::RemoveGlobal(index)) => {
                if index < self.workspace_preferences.global_presets.len() {
                    let preset = self.workspace_preferences.global_presets.remove(index);
                    self.status_message =
                        format!("Removed global workspace preset: {}", preset.name);
                }
            }
            Some(WorkspacePresetUiAction::CreateProject(index)) => {
                if let Some(preset) = self.workspace_state.project_presets.get(index).cloned() {
                    let global_preset_source = self
                        .workspace_preferences
                        .global_presets
                        .iter()
                        .find(|global| {
                            global.name == preset.name && global.windows == preset.windows
                        })
                        .map(|global| global.name.clone());
                    self.create_workspace_from_preset(preset, global_preset_source);
                }
            }
            Some(WorkspacePresetUiAction::RemoveProject(index)) => {
                if index < self.workspace_state.project_presets.len() {
                    let preset = self.workspace_state.project_presets.remove(index);
                    self.mark_modified();
                    self.status_message =
                        format!("Removed project workspace preset: {}", preset.name);
                }
            }
            None => {}
        }
    }

    fn focus_tab(&mut self, tab: PanelTab) {
        match tab {
            PanelTab::Project => {
                self.project_window.open = true;
                self.project_window.minimized = false;
                self.project_window.focus_requested = true;
            }
            PanelTab::Mixer => {
                self.mixer_window.open = true;
                self.mixer_window.focus_requested = true;
            }
            PanelTab::Media => {
                self.media_window.open = true;
                self.media_window.focus_requested = true;
            }
        }
    }

    fn open_selected_midi_editor(&mut self) {
        let available = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize))
            .is_some_and(|track| {
                matches!(track.kind, TrackKind::Midi | TrackKind::Instrument)
                    && self
                        .data
                        .project
                        .selected_midi_part_id
                        .is_some_and(|part_id| {
                            track.midi_parts.iter().any(|part| part.id == part_id)
                        })
            });
        if available {
            self.midi_editor_open = true;
            self.sample_editor_active = false;
            self.mixer_window.open = true;
        } else {
            self.status_message = "Select a MIDI part before opening its editor".to_owned();
        }
    }

    fn advance_transport(&mut self, ctx: &egui::Context) {
        if self
            .audio_engine
            .as_ref()
            .is_some_and(AudioEngine::take_output_stream_error)
        {
            self.set_transport_playing(false);
            self.audio_input_probe = None;
            self.audio_engine = None;
            self.status_message =
                "Audio device changed or stopped. Open Audio in Device Setup to reconnect."
                    .to_owned();
            return;
        }
        if let Some(engine) = self.audio_engine.as_mut() {
            engine.reclaim_retired_plans();
            if engine.take_output_stream_error() {
                engine.set_playing(false);
                self.data.transport.playing = false;
                self.status_message = format!(
                    "Audio output stream failed on {}; check the device or select another output",
                    engine.device_name
                );
            }
            self.data.transport.playing = engine.is_playing();
            self.data.transport.position_beats =
                engine.position_seconds() * self.data.project.tempo_bpm / 60.0;
            if self.data.transport.playing {
                ctx.request_repaint();
            }
            return;
        }

        let now = ctx.input(|input| input.time);
        let delta = self
            .data
            .transport
            .last_clock
            .replace(now)
            .map(|last| (now - last).clamp(0.0, 0.1))
            .unwrap_or(0.0);

        if self.data.transport.playing {
            let project = &self.data.project;
            self.data.transport.position_beats += delta * project.tempo_bpm / 60.0;
            if self.data.transport.cycle {
                self.data.transport.position_beats =
                    project.cycle_position_beats(self.data.transport.position_beats);
            }
            ctx.request_repaint();
        }
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        self.menu_open = false;
        if !self.project_open {
            self.show_empty_project_menu(ui);
            return;
        }
        let workspace_items: Vec<_> = self
            .workspace_state
            .workspaces
            .iter()
            .enumerate()
            .map(|(index, workspace)| {
                (
                    index,
                    workspace.name.clone(),
                    self.workspace_state.active_index == Some(index),
                )
            })
            .collect();
        let active_workspace_locked = self
            .workspace_state
            .active_index
            .and_then(|index| self.workspace_state.workspaces.get(index))
            .is_some_and(|workspace| workspace.locked);
        let can_create_workspace = workspace_items.len() < 9;
        let midi_editor_available = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize))
            .is_some_and(|track| {
                matches!(track.kind, TrackKind::Midi | TrackKind::Instrument)
                    && self
                        .data
                        .project
                        .selected_midi_part_id
                        .is_some_and(|part_id| {
                            track.midi_parts.iter().any(|part| part.id == part_id)
                        })
            });
        let selected_track_index = self
            .data
            .project
            .selected_track
            .map(|index| index as usize)
            .filter(|index| *index < self.data.project.tracks.len());
        let selected_track_available = selected_track_index.is_some();
        let unavailable_audio_paths =
            unresolved_audio_source_paths(&self.data.project, &self.data.audio_sources);
        let undo_shortcut = self.command_shortcut_label("edit.undo");
        let redo_shortcut = self.command_shortcut_label("edit.redo");

        ui.set_min_height(theme::spacing::MENU_BAR_HEIGHT);
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme::menu_bar_fill());
        egui::MenuBar::new()
            .style(theme::menu_bar_style)
            .config(egui::containers::menu::MenuConfig::new().style(theme::menu_popup_style))
            .ui(ui, |ui| {
                self.menu_open = egui::containers::menu::MenuState::from_ui(ui, |state, _| {
                    state.open_item.is_some()
                });
                ui.menu_button("File", |ui| {
                    if menu_command(ui, "New Project...", "Ctrl+N", true).clicked() {
                        self.request_new_project();
                        ui.close();
                    }
                    menu_command(ui, "New Library...", "", false);
                    if menu_command(ui, "Open...", "Ctrl+O", true).clicked() {
                        self.request_open_project();
                        ui.close();
                    }
                    menu_command(ui, "Open Library...", "", false);
                    menu_command(ui, "Close", "Ctrl+W", false);
                    ui.separator();
                    if menu_command(ui, "Save", "Ctrl+S", true).clicked() {
                        self.save_project();
                        ui.close();
                    }
                    if menu_command(ui, "Save As...", "Ctrl+Shift+S", true).clicked() {
                        self.save_project_as();
                        ui.close();
                    }
                    menu_command(ui, "Back up Project...", "", false);
                    menu_command(ui, "Save as Template...", "", false);
                    menu_command(ui, "Save Library...", "", false);
                    menu_command(ui, "Revert", "", false);
                    ui.separator();
                    menu_command(ui, "Page Setup...", "", false);
                    menu_command(ui, "Print...", "", false);
                    ui.separator();
                    ui.menu_button("Import", |ui| {
                        if menu_command(ui, "Audio...", "", true).clicked() {
                            self.import_audio_file();
                            ui.close();
                        }
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Audio CD...", ""),
                                ("Video File...", ""),
                                ("MIDI File...", ""),
                                ("Track Archive...", ""),
                                ("Tempo Track...", ""),
                                ("MusicXML...", ""),
                                ("OMF...", ""),
                            ],
                        );
                    });
                    ui.menu_button("Export", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Audio Mixdown...", ""),
                                ("Selected Tracks...", ""),
                                ("Tempo Track...", ""),
                            ],
                        );
                    });
                    menu_command(ui, "Replace Audio in Video File...", "", false);
                    menu_command(ui, "Cleanup...", "", false);
                    ui.separator();
                    if menu_command(ui, "Preferences...", "", true).clicked() {
                        self.preferences_ui.open(&self.metering_preferences);
                        ui.close();
                    }
                    if menu_command(ui, "Key Commands...", "", true).clicked() {
                        self.key_commands_open = true;
                        ui.close();
                    }
                    ui.menu_button("Recent Projects", |ui| {
                        menu_command(ui, "No Recent Projects", "", false);
                    });
                    ui.separator();
                    if menu_command(ui, "Quit", "Ctrl+Q", true).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if menu_command(ui, "Undo", &undo_shortcut, self.project_history.can_undo())
                        .clicked()
                    {
                        self.undo_project();
                        ui.close();
                    }
                    if menu_command(ui, "Redo", &redo_shortcut, self.project_history.can_redo())
                        .clicked()
                    {
                        self.redo_project();
                        ui.close();
                    }
                    menu_command(ui, "History...", "", false);
                    ui.separator();
                    let can_cut = self.project_open
                        && !self.is_recording()
                        && !self
                            .arrange_view_state
                            .audio_events
                            .selected_ids()
                            .is_empty();
                    if menu_command(ui, "Cut", "Ctrl+X", can_cut).clicked() {
                        self.cut_selected_audio_events();
                        ui.close();
                    }
                    let can_copy = self.project_open
                        && !self.is_recording()
                        && !self
                            .arrange_view_state
                            .audio_events
                            .selected_ids()
                            .is_empty();
                    if menu_command(ui, "Copy", "Ctrl+C", can_copy).clicked() {
                        self.copy_selected_audio_events();
                        ui.close();
                    }
                    if menu_command(
                        ui,
                        "Paste",
                        "Ctrl+V",
                        self.project_open
                            && !self.is_recording()
                            && self.audio_event_clipboard.is_some(),
                    )
                    .clicked()
                    {
                        self.paste_audio_events();
                        ui.close();
                    }
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Paste at Origin", "Alt+V"),
                            ("Split at Cursor", "Alt+X"),
                            ("Split Loop", ""),
                        ],
                    );
                    let can_delete_audio = self.project_open
                        && !self.is_recording()
                        && media_pool::selection_target(
                            &self.data.project,
                            &self.arrange_view_state.audio_events,
                        )
                        .is_some();
                    let can_delete = can_delete_audio
                        || (self.project_open
                            && !self.is_recording()
                            && self.data.project.selected_midi_part_id.is_some());
                    if menu_command(ui, "Delete", "Delete", can_delete).clicked() {
                        self.delete_audio_selection(false);
                        ui.close();
                    }
                    if menu_command(
                        ui,
                        "Delete with Source File",
                        "Shift+Delete",
                        can_delete_audio,
                    )
                    .clicked()
                    {
                        self.delete_audio_selection(true);
                        ui.close();
                    }
                    ui.menu_button("Range", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Select in Loop", ""),
                                ("Set Left Locator", ""),
                                ("Set Right Locator", ""),
                                ("Crop", ""),
                            ],
                        );
                    });
                    ui.menu_button("Select", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("All", "Ctrl+A"),
                                ("None", "Ctrl+Shift+A"),
                                ("Invert", ""),
                                ("Events under Cursor", ""),
                            ],
                        );
                    });
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Duplicate", "Ctrl+D"),
                            ("Repeat...", "Ctrl+K"),
                            ("Fill Loop", ""),
                        ],
                    );
                    ui.menu_button("Move to", |ui| {
                        disabled_menu_commands(ui, &[("Left Locator", ""), ("Right Locator", "")]);
                    });
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Convert to Real Copy", ""),
                            ("Group", "Ctrl+G"),
                            ("Ungroup", "Ctrl+U"),
                            ("Lock...", "Ctrl+Shift+L"),
                            ("Unlock", "Ctrl+Shift+U"),
                            ("Mute", "Shift+M"),
                            ("Unmute", "Shift+U"),
                            ("Project Logical Editor...", ""),
                            ("Process Project Logical Editor", ""),
                            ("Automation follows Events", ""),
                            ("Auto Select Events under Cursor", ""),
                            ("Enlarge Selected Track", ""),
                        ],
                    );
                    ui.menu_button("Zoom", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Zoom In", "H"),
                                ("Zoom Out", "G"),
                                ("Zoom In Vertically", "Shift+F"),
                                ("Zoom Out Vertically", "Alt+S"),
                                ("Zoom to Selection", "Shift+E"),
                                ("Zoom to Project", "Alt+Down"),
                            ],
                        );
                    });
                    menu_command(ui, "Macros", "", false);
                    ui.separator();
                    if menu_command(ui, "Key Commands...", "", true).clicked() {
                        self.key_commands_open = true;
                        ui.close();
                    }
                });
                ui.menu_button("Project", |ui| {
                    ui.menu_button("Add Track", |ui| {
                        for (label, kind) in [
                            ("Audio", TrackKind::Audio),
                            ("Group Channel", TrackKind::Group),
                            ("Instrument", TrackKind::Instrument),
                            ("MIDI", TrackKind::Midi),
                        ] {
                            if menu_command(ui, label, "", true).clicked() {
                                self.track_add_dialog = Some(TrackAddDialog {
                                    kind,
                                    count: 1,
                                    audio_channels: 1,
                                    browse_presets: true,
                                });
                                ui.close();
                            }
                        }
                        ui.separator();
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Arranger", ""),
                                ("FX Channel", ""),
                                ("Folder", ""),
                                ("Marker", ""),
                                ("Ruler", ""),
                                ("Signature", ""),
                                ("Tempo", ""),
                                ("Transpose", ""),
                                ("Video", ""),
                                ("Browse Sounds...", ""),
                            ],
                        );
                    });
                    if ui
                        .add_enabled(
                            selected_track_available && !self.is_recording(),
                            egui::Button::new("Duplicate Tracks"),
                        )
                        .clicked()
                    {
                        self.duplicate_tracks(None);
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            selected_track_available && !self.is_recording(),
                            egui::Button::new("Remove Selected Tracks"),
                        )
                        .clicked()
                    {
                        self.request_selected_track_delete();
                        ui.close();
                    }
                    menu_command(ui, "Remove Empty Tracks", "", false);
                    ui.menu_button("Track Folding", |ui| {
                        disabled_menu_commands(ui, &[("Expand All", ""), ("Collapse All", "")]);
                    });
                    ui.separator();
                    if menu_command(ui, "Pool", "Ctrl+P", true).clicked() {
                        self.focus_tab(PanelTab::Media);
                        ui.close();
                    }
                    if menu_command(ui, "Project Setup...", "Shift+S", true).clicked() {
                        self.project_setup_open = true;
                        ui.close();
                    }
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Markers", "Ctrl+M"),
                            ("Tempo Track", "Ctrl+T"),
                            ("Browser", "Ctrl+B"),
                            ("Automation Panel", ""),
                            ("Beat Calculator...", ""),
                            ("Set Timecode at Cursor", ""),
                            ("Notepad", ""),
                            ("Auto Fades Settings...", ""),
                        ],
                    );
                    ui.separator();
                    if ui
                        .add_enabled(
                            selected_track_available,
                            egui::Button::new("Rename Selected Track..."),
                        )
                        .clicked()
                    {
                        self.open_selected_track_rename();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            selected_track_index.is_some_and(|index| index > 0),
                            egui::Button::new("Move Selected Track Up"),
                        )
                        .clicked()
                    {
                        self.move_selected_track(-1);
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            selected_track_index
                                .is_some_and(|index| index + 1 < self.data.project.tracks.len()),
                            egui::Button::new("Move Selected Track Down"),
                        )
                        .clicked()
                    {
                        self.move_selected_track(1);
                        ui.close();
                    }
                });
                ui.menu_button("Audio", |ui| {
                    ui.menu_button("Process", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Fade In", ""),
                                ("Fade Out", ""),
                                ("Envelope...", ""),
                                ("Gain...", ""),
                                ("Merge Clipboard", ""),
                                ("Normalize...", ""),
                                ("Remove DC Offset", ""),
                                ("Reverse", ""),
                                ("Silence", ""),
                                ("Stereo Flip", ""),
                                ("Time Stretch...", ""),
                                ("Resample...", ""),
                            ],
                        );
                    });
                    ui.menu_button("Plug-ins", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Reverb", ""),
                                ("Dynamics", ""),
                                ("EQ", ""),
                                ("Filter", ""),
                                ("Distortion", ""),
                                ("Modulation", ""),
                                ("Other", ""),
                            ],
                        );
                    });
                    menu_command(ui, "Spectrum Analyzer", "", false);
                    menu_command(ui, "Statistics", "", false);
                    ui.menu_button("Hitpoints", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Calculate Hitpoints", ""),
                                ("Create Markers", ""),
                                ("Create Regions", ""),
                                ("Quantize", ""),
                            ],
                        );
                    });
                    ui.menu_button("Realtime Processing", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Direct Offline Processing", ""),
                                ("Add Plug-in", ""),
                                ("Remove Plug-in", ""),
                            ],
                        );
                    });
                    ui.menu_button("Advanced", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Detect Silence...", ""),
                                ("Spectrum Analyzer...", ""),
                                ("Statistics...", ""),
                            ],
                        );
                    });
                });
                ui.menu_button("MIDI", |ui| {
                    if menu_command(ui, "Open Key Editor", "", midi_editor_available).clicked() {
                        self.open_selected_midi_editor();
                        ui.close();
                    }
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Open Score Editor", ""),
                            ("Open Drum Editor", ""),
                            ("Open List Editor", ""),
                            ("Open In-Place Editor", ""),
                        ],
                    );
                    ui.separator();
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Quantize Setup...", ""),
                            ("Logical Presets", ""),
                            ("Drum Map Setup...", ""),
                            ("Insert Velocities", ""),
                            ("VST Expression", ""),
                            ("CC Automation Setup...", ""),
                            ("Reset", ""),
                        ],
                    );
                    ui.separator();
                    ui.label("Input device");
                    let mut selected_midi_device = self.selected_midi_input_device.clone();
                    let selected_midi_name = selected_midi_device
                        .as_ref()
                        .and_then(|selected_id| {
                            self.midi_input_devices
                                .iter()
                                .find(|device| &device.id == selected_id)
                        })
                        .map(|device| device.name.clone())
                        .unwrap_or_else(|| "No input".to_owned());
                    egui::ComboBox::from_id_salt("midi-input-device")
                        .selected_text(selected_midi_name)
                        .width(260.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut selected_midi_device, None, "No input");
                            for device in &self.midi_input_devices {
                                let label = if self
                                    .midi_input_devices
                                    .iter()
                                    .filter(|candidate| candidate.name == device.name)
                                    .count()
                                    > 1
                                {
                                    format!("{} ({})", device.name, device.id)
                                } else {
                                    device.name.clone()
                                };
                                ui.selectable_value(
                                    &mut selected_midi_device,
                                    Some(device.id.clone()),
                                    label,
                                );
                            }
                        });
                    if selected_midi_device != self.selected_midi_input_device {
                        self.select_midi_input_device(selected_midi_device);
                    }
                    if self.midi_input_devices.is_empty() {
                        ui.label(
                            egui::RichText::new("No MIDI input devices found")
                                .color(theme::palette(ui).muted),
                        );
                    }
                    if ui.button("Refresh MIDI input devices").clicked() {
                        self.refresh_midi_input_devices();
                    }
                    ui.separator();
                    let mut midi_thru_enabled = self.midi_thru_enabled;
                    ui.add_enabled_ui(self.audio_engine.is_some(), |ui| {
                        if ui
                            .checkbox(&mut midi_thru_enabled, "MIDI Thru (starter instrument)")
                            .changed()
                        {
                            self.midi_thru_enabled = midi_thru_enabled;
                        }
                    });
                    if self.audio_engine.is_none() {
                        ui.label(
                            egui::RichText::new("MIDI Thru needs an audio output device")
                                .small()
                                .color(theme::palette(ui).muted),
                        );
                    }
                    if self.midi_thru_enabled {
                        ui.label(
                            egui::RichText::new(
                                "Arm a MIDI or Instrument track to monitor its input",
                            )
                            .small()
                            .color(theme::palette(ui).muted),
                        );
                    }
                });
                ui.menu_button("Scores", |ui| {
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Open Selection", "Ctrl+R"),
                            ("Open Layout...", ""),
                            ("Page Mode", ""),
                            ("Settings...", ""),
                            ("Group/Ungroup Notes", ""),
                            ("Convert to Grace Note", ""),
                            ("Build N-Tuplet...", ""),
                            ("Insert Slur", ""),
                            ("Hide/Show", ""),
                            ("Flip", ""),
                        ],
                    );
                    ui.menu_button("Align Elements", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Left", ""),
                                ("Right", ""),
                                ("Top", ""),
                                ("Bottom", ""),
                                ("Distribute Horizontally", ""),
                                ("Distribute Vertically", ""),
                            ],
                        );
                    });
                    disabled_menu_commands(
                        ui,
                        &[("Make Chord Symbols", ""), ("Make Guitar Symbols", "")],
                    );
                    ui.menu_button("Functions", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Explode", ""),
                                ("Merge", ""),
                                ("Explode Pitches", ""),
                                ("Make Chord Symbols", ""),
                            ],
                        );
                    });
                    disabled_menu_commands(ui, &[("Auto Layout...", ""), ("Reset Layout...", "")]);
                    ui.menu_button("Advanced Layout", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Optimize All", ""),
                                ("Move Bars", ""),
                                ("Insert Page Break", ""),
                                ("Remove Page Break", ""),
                            ],
                        );
                    });
                });
                ui.menu_button("Media", |ui| {
                    menu_command(ui, "Open Pool Window", "Ctrl+P", false);
                    if menu_command(ui, "Open MediaBay", "F5", true).clicked() {
                        self.data.selected_media = 0;
                        self.focus_tab(PanelTab::Media);
                        ui.close();
                    }
                    if menu_command(ui, "Open Loop Browser", "F6", true).clicked() {
                        self.data.selected_media = 1;
                        self.focus_tab(PanelTab::Media);
                        ui.close();
                    }
                    if menu_command(ui, "Open Sound Browser", "F7", true).clicked() {
                        self.data.selected_media = 2;
                        self.focus_tab(PanelTab::Media);
                        ui.close();
                    }
                    ui.separator();
                    if menu_command(ui, "Import Medium...", "", true).clicked() {
                        self.import_audio_file();
                        ui.close();
                    }
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Import Audio CD...", ""),
                            ("Import Pool...", ""),
                            ("Export Pool...", ""),
                            ("Find Missing Files...", ""),
                            ("Remove Missing Files", ""),
                            ("Reconstruct", ""),
                            ("Convert Files...", ""),
                            ("Conform Files...", ""),
                            ("Extract Audio from Video File", ""),
                            ("Generate Thumbnail Cache", ""),
                            ("Create Folder", ""),
                            ("Empty Trash", ""),
                            ("Remove Unused Media", ""),
                            ("Prepare Archive...", ""),
                            ("Set Pool Record Folder", ""),
                            ("Minimize File", ""),
                            ("New Version", ""),
                        ],
                    );
                    ui.menu_button("Insert into Project", |ui| {
                        disabled_menu_commands(
                            ui,
                            &[
                                ("Audio Track", ""),
                                ("Instrument Track", ""),
                                ("MIDI Track", ""),
                            ],
                        );
                    });
                    disabled_menu_commands(
                        ui,
                        &[("Select in Project", ""), ("Search Media...", "")],
                    );
                    ui.separator();
                    if menu_command(
                        ui,
                        "Relink Unavailable Audio...",
                        "",
                        !unavailable_audio_paths.is_empty(),
                    )
                    .clicked()
                    {
                        self.relink_unavailable_audio();
                        ui.close();
                    }
                });
                ui.menu_button("Transport", |ui| {
                    if checked_menu_command(ui, "Transport Panel", "F2", self.show_transport, true)
                        .clicked()
                    {
                        self.show_transport = !self.show_transport;
                        ui.close();
                    }
                    ui.separator();
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Locators to Selection", "P"),
                            ("Locate Selection", "L"),
                            ("Locate Selection End", ""),
                            ("Locate Next Marker", "Shift+N"),
                            ("Locate Previous Marker", "Shift+B"),
                            ("Locate Next Event", "N"),
                            ("Locate Previous Event", "B"),
                            ("Post-roll from Selection Start", ""),
                            ("Post-roll from Selection End", ""),
                            ("Pre-roll to Selection Start", ""),
                            ("Pre-roll to Selection End", ""),
                            ("Play from Selection Start", ""),
                            ("Play from Selection End", ""),
                            ("Play until Selection Start", ""),
                            ("Play until Selection End", ""),
                            ("Play until Next Marker", ""),
                            ("Play Selection Range", "Alt+Space"),
                            ("Loop Selection", "Shift+G"),
                        ],
                    );
                    menu_command(ui, "Use Pre-roll", "", false);
                    menu_command(ui, "Use Post-roll", "", false);
                    menu_command(ui, "Start Record at Left Locator", "", false);
                    ui.separator();
                    if menu_command(ui, "Metronome Setup...", "", true).clicked() {
                        self.metronome_settings_open = true;
                        ui.close();
                    }
                    if checked_menu_command(ui, "Metronome On", "C", self.metronome_enabled, true)
                        .clicked()
                    {
                        self.toggle_metronome();
                        ui.close();
                    }
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Precount On", ""),
                            ("Project Synchronization Setup...", ""),
                            ("Use External Sync", "T"),
                            ("Retrospective Record", "Shift+Num *"),
                        ],
                    );
                });
                ui.menu_button("Devices", |ui| {
                    if menu_command(ui, "Device Setup...", "", true).clicked() {
                        self.device_setup_open = true;
                        ui.close();
                    }
                    if menu_command(ui, "VST Connections...", "F4", self.project_open).clicked() {
                        self.vst_connections_open = true;
                        ui.close();
                    }
                    ui.separator();
                    if menu_command(ui, "Plug-in Information", "", true).clicked() {
                        self.plugin_information_open = true;
                        ui.close();
                    }
                    ui.separator();
                    if menu_command(ui, "Realtime Performance...", "", true).clicked() {
                        self.realtime_diagnostics_open = true;
                        ui.close();
                    }
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Control Room Mixer", ""),
                            ("Control Room Overview", ""),
                            ("MIDI Device Manager", ""),
                            ("MMC Master", ""),
                        ],
                    );
                    if menu_command(ui, "Mixer", "F3", true).clicked() {
                        self.mixer_window.open = true;
                        self.mixer_window.focus_requested = true;
                        ui.close();
                    }
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Mixer 2", ""),
                            ("Mixer 3", ""),
                            ("Remaining Record Time Display", ""),
                            ("Time Display", ""),
                            ("VST Instruments", "F11"),
                            ("VST Performance", "F12"),
                            ("Video Window", "F8"),
                            ("Virtual Keyboard", "Alt+K"),
                            ("Show Panel", ""),
                        ],
                    );
                });
                ui.menu_button("Window", |ui| {
                    ui.menu_button("Workspaces", |ui| {
                        for (index, name, active) in &workspace_items {
                            if ui.selectable_label(*active, name).clicked() {
                                self.activate_workspace(*index);
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button("Organize Workspaces...").clicked() {
                            self.workspace_organizer_open = true;
                            ui.close();
                        }
                        if ui
                            .add_enabled(can_create_workspace, egui::Button::new("New Workspace"))
                            .clicked()
                        {
                            self.create_workspace();
                            ui.close();
                        }
                        let lock_label = if active_workspace_locked {
                            "Unlock Active Workspace"
                        } else {
                            "Lock Active Workspace"
                        };
                        if ui.button(lock_label).clicked() {
                            self.toggle_workspace_lock();
                            ui.close();
                        }
                    });
                    menu_command(ui, "Windows...", "", false);
                    ui.separator();
                    if ui
                        .selectable_label(true, format!("{} - Project", self.data.project.name))
                        .clicked()
                    {
                        self.project_window.open = true;
                        self.project_window.minimized = false;
                        self.project_window.focus_requested = true;
                        ui.close();
                    }
                    if ui
                        .selectable_label(self.show_transport, "Transport")
                        .clicked()
                    {
                        self.show_transport = true;
                        ui.close();
                    }
                    if ui
                        .selectable_label(self.mixer_window.open, "Mixer")
                        .clicked()
                    {
                        self.mixer_window.open = true;
                        self.mixer_window.focus_requested = true;
                        ui.close();
                    }
                    if let Some(track_name) = self
                        .data
                        .project
                        .selected_track
                        .and_then(|index| self.data.project.tracks.get(index as usize))
                        .map(|track| track.name.clone())
                    {
                        if ui
                            .selectable_label(
                                self.channel_settings_window.open,
                                format!("VST Audio Channel Settings - {track_name}"),
                            )
                            .clicked()
                        {
                            self.channel_settings_window.request_channel_settings(None);
                            ui.close();
                        }
                    }
                    if ui
                        .selectable_label(self.media_window.open, "MediaBay")
                        .clicked()
                    {
                        self.data.selected_media = 0;
                        self.media_window.open = true;
                        self.media_window.focus_requested = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(midi_editor_available, egui::Button::new("MIDI Editor"))
                        .clicked()
                    {
                        self.midi_editor_open = true;
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    menu_command(ui, "OpenTrack on the Web", "", false);
                    menu_command(ui, "Registration", "", false);
                    if menu_command(ui, "Credits and Copyrights", "", true).clicked() {
                        self.about_open = true;
                        ui.close();
                    }
                    if menu_command(ui, "About OpenTrack...", "", true).clicked() {
                        self.about_open = true;
                        ui.close();
                    }
                });
            });
    }

    fn show_empty_project_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_height(theme::spacing::MENU_BAR_HEIGHT);
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme::menu_bar_fill());
        egui::MenuBar::new()
            .style(theme::menu_bar_style)
            .config(egui::containers::menu::MenuConfig::new().style(theme::menu_popup_style))
            .ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if menu_command(ui, "New Project...", "Ctrl+N", true).clicked() {
                        self.request_new_project();
                        ui.close();
                    }
                    if menu_command(ui, "Open...", "Ctrl+O", true).clicked() {
                        self.request_open_project();
                        ui.close();
                    }
                    ui.separator();
                    menu_command(ui, "Save", "Ctrl+S", false);
                    menu_command(ui, "Save As...", "Ctrl+Shift+S", false);
                    ui.separator();
                    if menu_command(ui, "Quit", "Alt+F4", true).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    menu_command(ui, "Undo", "", false);
                    menu_command(ui, "Redo", "", false);
                    ui.separator();
                    if menu_command(ui, "Key Commands...", "", true).clicked() {
                        self.key_commands_open = true;
                        ui.close();
                    }
                });
                ui.menu_button("Project", |ui| {
                    menu_command(ui, "No project is open", "", false);
                });
                ui.menu_button("Audio", |ui| {
                    menu_command(ui, "No project is open", "", false);
                });
                ui.menu_button("Devices", |ui| {
                    if menu_command(ui, "Device Setup...", "", true).clicked() {
                        self.device_setup_open = true;
                        ui.close();
                    }
                    menu_command(ui, "VST Connections...", "F4", false);
                });
                for menu in ["MIDI", "Scores", "Media", "Transport", "Window"] {
                    ui.menu_button(menu, |ui| {
                        menu_command(ui, "No project is open", "", false);
                    });
                }
                ui.menu_button("Help", |ui| {
                    if menu_command(ui, "About OpenTrack...", "", true).clicked() {
                        self.about_open = true;
                        ui.close();
                    }
                });
            });
    }

    fn handle_native_window_input(&mut self, ctx: &egui::Context) {
        let (pointer_event, pointer_released, key_pressed, pointer_down) =
            viewport_interaction(ctx);
        if pointer_event || key_pressed {
            if !self.project_history.begin(&self.data.project) {
                self.status_message =
                    "Undo history is unavailable for this project size".to_owned();
            }
        }
        if pointer_released || (key_pressed && !pointer_down) {
            self.project_history.commit_after_frame = true;
        }
        self.handle_shortcuts(ctx);
    }

    fn show_about_window(&mut self, ctx: &egui::Context) {
        let response = egui::Window::new("About OpenTrack-")
            .id(Id::new("about-opentrack-window"))
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("OpenTrack-");
                ui.label(format!(
                    "Rust OpenTrack DAW prototype — version {}",
                    env!("CARGO_PKG_VERSION")
                ));
                ui.separator();
                ui.label("Built for x86-64 Windows.");
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("about-opentrack-window"),
            &mut self.about_open,
            response.as_ref(),
        );
    }

    fn show_mixer_window(&mut self, ui: &mut egui::Ui) {
        self.mixer_window.viewport_live = false;
        self.mixer_window.focus_requested = false;
        let panel_rect = ui.max_rect();
        self.mixer_window.bounds = Some(docked_bounds(panel_rect));
        let palette = theme::palette(ui);
        ui.painter().rect_filled(panel_rect, 0.0, palette.surface);
        if docked_panel_header(ui, "", false) {
            self.mixer_window.open = false;
            self.sample_editor_active = false;
            self.midi_editor_open = false;
            return;
        }
        for (index, label) in [(0, "Mixer"), (1, "Audio Editor"), (2, "MIDI Editor")] {
            let rect = Rect::from_min_size(
                panel_rect.min + Vec2::new(5.0 + index as f32 * 105.0, 1.0),
                Vec2::new(102.0, PANEL_HEADER_HEIGHT - 2.0),
            );
            let active = match index {
                0 => !self.sample_editor_active && !self.midi_editor_open,
                1 => self.sample_editor_active,
                _ => self.midi_editor_open,
            };
            if ui
                .put(rect, egui::Button::new(label).selected(active))
                .clicked()
            {
                self.sample_editor_active = index == 1;
                self.midi_editor_open = index == 2;
            }
        }
        let body_rect = Rect::from_min_max(
            Pos2::new(
                panel_rect.left() + 5.0,
                panel_rect.top() + PANEL_HEADER_HEIGHT,
            ),
            Pos2::new(panel_rect.right() - 5.0, panel_rect.bottom() - 3.0),
        );
        let position_before = self.data.transport.position_beats;
        let selection_before = self.data.project.selected_track;
        let midi_editor_visible = self.midi_editor_open;
        let mut mixer_track_controls_before = std::mem::take(&mut self.mixer_track_control_cache);
        mixer_track_controls_before.clear();
        if !self.sample_editor_active && !midi_editor_visible {
            mixer_track_controls_before.extend(
                self.data
                    .project
                    .tracks
                    .iter()
                    .map(|track| (track.gain, track.pan)),
            );
        }
        let mixer_output_controls_before = (
            self.data.project.output_channel.strip.gain,
            self.data.project.output_channel.strip.pan,
        );
        let mut mixer_selection_requested = false;
        let mut midi_live_velocity_update = None;
        let sample_grid = self
            .arrange_view_state
            .snap_grid_beats(self.data.project.beats_per_bar);
        self.sample_editor.set_render_directory(
            self.project_path
                .as_deref()
                .and_then(|p| p.parent())
                .map(|p| p.join("Metadata").join("PitchTime")),
        );
        let (modified, requires_plan_rebuild) = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt("mixer-panel-body")
                    .max_rect(body_rect),
                |ui| {
                    ui.set_clip_rect(body_rect);
                    ui.set_width(body_rect.width());
                    if self.midi_editor_open {
                        let changed =
                            self.show_midi_editor_panel(ui, &mut midi_live_velocity_update);
                        (changed, changed && midi_live_velocity_update.is_none())
                    } else if self.sample_editor_active {
                        self.sample_editor.cycle_enabled = self.data.transport.cycle;
                        let changed = sample_editor::show(
                            ui,
                            &mut self.data.project,
                            &self.data.audio_sources,
                            &mut self.sample_editor,
                            &mut self.edit_tool,
                            sample_grid,
                            self.recording_session.is_none(),
                            &mut self.data.transport.position_beats,
                            self.data.transport.playing,
                        );
                        (changed, changed)
                    } else {
                        let (changed, selection_requested, requires_rebuild) = show_mixer(
                            ui,
                            &mut self.data.project,
                            &self.data.track_meter_levels,
                            &mut self.data.peak_holds,
                            self.data.input_channel_meter_levels,
                            self.data.output_channel_meter_levels,
                            true,
                            false,
                            &mut self.channel_settings_window,
                            &mut self.channel_settings_ui,
                            &self.vst3_plugin_catalog,
                        );
                        mixer_selection_requested |= selection_requested;
                        (changed, requires_rebuild)
                    }
                },
            )
            .inner;
        if mixer_selection_requested || self.data.project.selected_track != selection_before {
            self.sync_arrange_track_selection();
        }
        self.update_note_audition(ui.ctx());
        ui.painter().rect_stroke(
            panel_rect,
            0.0,
            Stroke::new(1.0, palette.grid),
            egui::StrokeKind::Inside,
        );
        if self.sample_editor.event_focus {
            self.sample_editor.event_focus = false;
            if let Some((_, id)) = self.sample_editor.target {
                self.arrange_view_state.audio_events.select_one(id);
            }
        }
        if let Some((track, id)) = self.sample_editor.analysis_attached.take()
            && let Some(clip) = self
                .data
                .project
                .tracks
                .iter()
                .find(|t| t.id == track)
                .and_then(|t| t.clips.iter().find(|c| c.id == id))
        {
            self.project_history.retain_analysis(track, clip);
        }
        if self.data.transport.position_beats != position_before {
            if let Some(engine) = &self.audio_engine {
                engine.seek_seconds(
                    self.data.transport.position_beats * 60.0
                        / self.data.project.tempo_bpm.max(1.0),
                );
            }
        }
        let midi_edit_drag_active = self.midi_editor_open
            && ui
                .ctx()
                .input(|input| input.pointer.button_down(egui::PointerButton::Primary))
            && (self.midi_note_drag.is_some()
                || self.midi_note_draw.is_some()
                || self.midi_controller_drag.is_some());
        let finish_pending_midi_rebuild = self.midi_pending_audio_rebuild && !midi_edit_drag_active;
        if modified || finish_pending_midi_rebuild {
            let output = &self.data.project.output_channel.strip;
            let output_changed = output.gain != mixer_output_controls_before.0
                || output.pan != mixer_output_controls_before.1;
            let mut has_live_mix_changes = output_changed;
            let live_updated = self.audio_engine.as_ref().is_some_and(|engine| {
                let midi_updated =
                    midi_live_velocity_update.map_or(true, |(track, note, velocity)| {
                        engine.update_midi_note_velocity(track, note, velocity)
                    });
                if output_changed {
                    engine.update_output_channel_parameters(
                        output.gain,
                        output.pan,
                        &output.equalizer,
                    );
                }
                let mut mix_updated = true;
                for (index, (track, (gain, pan))) in self
                    .data
                    .project
                    .tracks
                    .iter()
                    .zip(mixer_track_controls_before.iter().copied())
                    .enumerate()
                {
                    if track.gain != gain || track.pan != pan {
                        has_live_mix_changes = true;
                        mix_updated &= engine.update_track_mix(index, track.gain, track.pan);
                    }
                }
                midi_updated && mix_updated
            });
            if self.audio_engine.is_none() {
                has_live_mix_changes |= self
                    .data
                    .project
                    .tracks
                    .iter()
                    .zip(mixer_track_controls_before.iter().copied())
                    .any(|(track, (gain, pan))| track.gain != gain || track.pan != pan);
            }
            if midi_edit_drag_active && modified {
                self.midi_pending_audio_rebuild = true;
            } else if self.midi_pending_audio_rebuild {
                self.rebuild_audio_plan();
            } else if requires_plan_rebuild || (has_live_mix_changes && !live_updated) {
                self.rebuild_audio_plan();
            }
            if modified {
                self.mark_modified();
            }
        }
        self.mixer_track_control_cache = mixer_track_controls_before;
    }

    fn update_note_audition(&mut self, ctx: &egui::Context) {
        let request = if self.sample_editor_active {
            self.sample_editor.note_audition.clone()
        } else {
            None
        };
        let Some(request) = request else {
            if self.active_note_audition.is_some() {
                if let Some(engine) = &self.audio_engine {
                    engine.stop_note_audition();
                }
                self.active_note_audition = None;
                self.note_audition_stop_at = Some(Instant::now() + Duration::from_millis(40));
            }
            if self
                .note_audition_stop_at
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                if let Some(engine) = &self.audio_engine {
                    engine.clear_note_audition();
                }
                self.note_audition_stop_at = None;
            } else if let Some(deadline) = self.note_audition_stop_at {
                ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
            }
            return;
        };
        if self.active_note_audition.as_ref() == Some(&request) {
            return;
        }
        let Some(track) = self
            .data
            .project
            .tracks
            .iter()
            .find(|track| track.id == request.track_id)
        else {
            return;
        };
        let Some(clip) = track.clips.iter().find(|clip| clip.id == request.clip_id) else {
            return;
        };
        let Some(asset) = self
            .data
            .audio_sources
            .get(sample_editor::playback_path(clip))
        else {
            return;
        };
        let Ok(playback) = opentrack_audio::PlaybackSource::from_asset(asset, None) else {
            return;
        };
        let playback = match &clip.pitch_time {
            Some(edit) => playback
                .clone()
                .with_pitch_time(edit, self.data.project.tempo_bpm)
                .unwrap_or(playback),
            None => playback,
        };
        let source_offset = clip
            .pitch_time
            .as_ref()
            .map_or(clip.source_offset_seconds, |edit| {
                edit.analysis.source_start_seconds
            });
        let duration = (request.note.end - request.note.start).max(0.01);
        let preview_length = duration.min(0.35);
        let preview_start = request.note.start + (duration - preview_length) * 0.5;
        let level = (clip.gain * track.gain).clamp(0.0, 1.0).max(0.2);
        let Ok(audition) = opentrack_audio::NoteAudition::new(
            playback,
            source_offset,
            preview_start,
            preview_start + preview_length,
            level,
        ) else {
            return;
        };
        if let Some(engine) = &self.audio_engine {
            engine.set_note_audition(audition);
        }
        self.active_note_audition = Some(request);
        self.note_audition_stop_at = None;
    }

    fn update_channel_analyzer(&mut self) {
        if !self.channel_settings_window.open {
            self.channel_settings_ui.analyzer.close();
            return;
        }
        let project = &self.data.project;
        let owner = match self.channel_settings_window.channel_bus {
            Some(true) => Some(opentrack_core::project::INPUT_BUS_ID),
            Some(false) => Some(opentrack_core::project::OUTPUT_BUS_ID),
            None => project
                .selected_track
                .and_then(|i| project.tracks.get(i as usize))
                .map(|t| t.id),
        };
        let choices = project
            .channels()
            .map(|t| analyzer_view::Choice {
                id: t.id,
                name: t.name.clone(),
                equalizer: t.equalizer.clone(),
            })
            .collect();
        let mut taps = self.data.track_meter_peaks.clone();
        if let Some(engine) = &self.audio_engine {
            taps.insert(
                opentrack_core::project::OUTPUT_BUS_ID,
                engine.output_meter_tap(),
            );
        }
        let input = self
            .recording_session
            .as_ref()
            .map(|s| s.recorder.monitor_buffer())
            .or_else(|| self.audio_input_probe.as_ref().map(|p| p.monitor_buffer()));
        if let Some(input) = input {
            taps.insert(opentrack_core::project::INPUT_BUS_ID, input.meter_tap());
        }
        let rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate)
            .unwrap_or(project.sample_rate_hz);
        self.channel_settings_ui
            .analyzer
            .update(owner, choices, taps, rate);
    }

    /// True while an application dialog owns input. Windows are separate
    /// toplevels on Linux, where nothing disables the owner the way Win32 does,
    /// so every other window is rendered disabled instead of accepting clicks.
    fn dialog_owns_input(&self) -> bool {
        self.device_setup_open
            || self.vst_connections_open
            || self.project_setup_open
            || self.preferences_ui.is_open
            || self.key_commands_open
            || self.workspace_organizer_open
            || self.about_open
            || self.metronome_settings_open
            || self.tempo_signature_settings_open
            || self.realtime_diagnostics_open
            || self.project_folder_setup.is_some()
            || self.pending_project_action.is_some()
            || self.track_rename_dialog.is_some()
            || self.track_add_dialog.is_some()
            || self.event_colors_dialog.is_some()
            || self.pending_track_delete_id.is_some()
    }

    fn show_transport_window(&mut self, ctx: &egui::Context) {
        if !self.project_open || !self.show_transport {
            self.transport_window.viewport_live = false;
            return;
        }

        const SIZE: Vec2 = Vec2::new(837.0, 69.0);
        let restore_bounds = if self.transport_window.viewport_live {
            None
        } else {
            self.transport_window.bounds.as_ref()
        };
        let reopening = !self.transport_window.viewport_live;
        let id = egui::ViewportId::from_hash_of("floating-transport-panel-m1-position-v5");
        let mut builder = viewport_builder("Transport Panel", SIZE, SIZE, restore_bounds)
            .with_visible(!reopening)
            .with_decorations(false)
            .with_resizable(false)
            .with_window_level(if self.transport_window.always_on_top {
                egui::viewport::WindowLevel::AlwaysOnTop
            } else {
                egui::viewport::WindowLevel::Normal
            });
        if restore_bounds.is_none() {
            builder = builder.with_position(Pos2::new(552.0, 663.0));
        }

        let mut open = true;
        let mut rendered = false;
        let mut bounds = None;
        ctx.show_viewport_immediate(id, builder, |window_ctx, _class| {
            rendered = true;
            if reopening {
                window_ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            open &= reopening || !window_ctx.input(|input| input.viewport().close_requested());
            self.handle_native_window_input(window_ctx);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::central_panel(&window_ctx.style())
                        .inner_margin(egui::Margin::ZERO),
                )
                .show(window_ctx, |ui| {
                    let pointer_blocked = self.dialog_owns_input() || self.menu_open;
                    let overlay_open = pointer_blocked || egui::Popup::is_any_open(ui.ctx());
                    blocked_input_scope(ui, pointer_blocked, overlay_open, |ui| {
                        self.show_transport(ui)
                    });
                });
            bounds = native_window_bounds(window_ctx);
        });

        if !rendered {
            open = false;
            self.status_message =
                "Transport window could not be created; reopen it from the Window menu".to_owned();
        }
        self.show_transport &= open;
        self.transport_window.viewport_live = open && self.show_transport;
        if let Some(bounds) = bounds {
            self.transport_window.bounds = Some(bounds);
        }
        if self.transport_window.viewport_live {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }

    fn show_channel_settings_window(&mut self, ctx: &egui::Context) {
        self.update_channel_analyzer();
        if !self.channel_settings_window.open {
            self.channel_settings_window.viewport_live = false;
            return;
        }
        let live_channel_id = match self.channel_settings_window.channel_bus {
            Some(true) => Some(opentrack_core::project::INPUT_BUS_ID),
            Some(false) => Some(opentrack_core::project::OUTPUT_BUS_ID),
            None => self
                .data
                .project
                .selected_track
                .and_then(|index| self.data.project.tracks.get(index as usize))
                .map(|track| track.id),
        };
        let live_controls_before = live_channel_id.and_then(|id| {
            self.data
                .project
                .channels()
                .find(|track| track.id == id)
                .map(|track| (id, track.gain, track.pan, track.equalizer.clone()))
        });
        let track_name = match self.channel_settings_window.channel_bus {
            Some(true) => self.data.project.input_channel.strip.name.clone(),
            Some(false) => self.data.project.output_channel.strip.name.clone(),
            None => self
                .data
                .project
                .selected_track
                .and_then(|index| self.data.project.tracks.get(index as usize))
                .map(|track| track.name.clone())
                .unwrap_or_else(|| "No channel selected".into()),
        };
        let restore_bounds = if self.channel_settings_window.viewport_live {
            None
        } else {
            self.channel_settings_window
                .restore_bounds
                .clone()
                .or_else(|| self.channel_settings_window.bounds.clone())
        };
        let id = egui::ViewportId::from_hash_of((
            CHANNEL_SETTINGS_WINDOW_ID,
            self.channel_settings_window.epoch,
        ));
        let focus_requested = self.channel_settings_window.focus_requested;
        let viewport_live = self.channel_settings_window.viewport_live;
        if focus_requested {
            self.channel_settings_window.focus_requested = false;
        }
        let mut builder = viewport_builder(
            format!("VST Audio Channel Settings - {track_name}"),
            Vec2::new(902.0, 430.0),
            Vec2::new(820.0, 420.0),
            restore_bounds.as_ref(),
        );
        builder = builder
            .with_visible(viewport_live || focus_requested)
            .with_window_level(egui::viewport::WindowLevel::Normal);
        if focus_requested {
            builder = builder.with_active(true);
        }
        if restore_bounds.is_none() {
            builder = builder.with_position(Pos2::new(382.0, 145.0));
        }
        let mut open = true;
        let mut modified = false;
        let mut bounds = None;
        let mut rendered = false;
        let selection_before = self.data.project.selected_track;
        let mut mixer_selection_requested = false;
        ctx.show_viewport_immediate(id, builder, |window_ctx, _class| {
            rendered = true;
            if focus_requested {
                window_ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                window_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                window_ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                window_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            self.handle_native_window_input(window_ctx);
            open &=
                focus_requested || !window_ctx.input(|input| input.viewport().close_requested());
            egui::CentralPanel::default()
                .frame(egui::Frame::central_panel(&window_ctx.style()).inner_margin(4.0))
                .show(window_ctx, |ui| {
                    let pointer_blocked = self.dialog_owns_input() || self.menu_open;
                    let overlay_open = pointer_blocked || egui::Popup::is_any_open(ui.ctx());
                    blocked_input_scope(ui, pointer_blocked, overlay_open, |ui| {
                        let (changed, selection_requested, _) = show_mixer(
                            ui,
                            &mut self.data.project,
                            &self.data.track_meter_levels,
                            &mut self.data.peak_holds,
                            self.data.input_channel_meter_levels,
                            self.data.output_channel_meter_levels,
                            false,
                            true,
                            &mut self.channel_settings_window,
                            &mut self.channel_settings_ui,
                            &self.vst3_plugin_catalog,
                        );
                        modified |= changed;
                        mixer_selection_requested |= selection_requested;
                    });
                });
            channel_settings::setup(window_ctx, &mut self.channel_settings_ui);
            bounds = native_window_bounds(window_ctx);
        });
        if let Some(action) = self.channel_settings_ui.pending_action.take() {
            modified |= self.apply_channel_settings_action(action);
        }
        if mixer_selection_requested || self.data.project.selected_track != selection_before {
            self.sync_arrange_track_selection();
        }
        if !rendered {
            open = false;
            self.status_message =
                "Channel Settings window could not be created; click Edit to retry".into();
        }
        let modal = self.pending_project_action.is_some()
            || self.pending_track_delete_id.is_some()
            || self.project_rate_change.is_some()
            || self.project_folder_setup.is_some()
            || self.project_open_error.is_some()
            || self.media_operation_error.is_some()
            || self.track_add_dialog.is_some()
            || self.track_rename_dialog.is_some()
            || self.event_colors_dialog.is_some();
        let dialog_open = modal || self.dialog_owns_input();
        if self.channel_window_owner.apply(
            self.channel_settings_ui.preferences.always_on_top,
            dialog_open,
            modal,
        ) {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        }
        // Windows keeps the editor above its owner through Win32 window
        // ownership. Other desktops have no owner concept, so ask the compositor
        // for a top-level window level while the preference is set.
        #[cfg(not(windows))]
        {
            let level = if self.channel_settings_ui.preferences.always_on_top && !dialog_open {
                egui::viewport::WindowLevel::AlwaysOnTop
            } else {
                egui::viewport::WindowLevel::Normal
            };
            if self.channel_settings_window.window_level != Some(level) {
                self.channel_settings_window.window_level = Some(level);
                ctx.send_viewport_cmd_to(id, egui::ViewportCommand::WindowLevel(level));
            }
        }
        self.channel_settings_window.open = open;
        self.channel_settings_window.viewport_live = open;
        if restore_bounds.is_some() {
            self.channel_settings_window.restore_bounds = None;
        }
        if let Some(bounds) = bounds {
            self.channel_settings_window.bounds = Some(bounds);
        }
        if modified {
            let live_controls_changed = live_controls_before
                .and_then(|(id, gain, pan, equalizer)| {
                    let track = self.data.project.channels().find(|track| track.id == id)?;
                    ((track.gain != gain) || (track.pan != pan) || (track.equalizer != equalizer))
                        .then_some((id, track.gain, track.pan, track.equalizer.clone()))
                })
                .is_some_and(|(id, gain, pan, equalizer)| {
                    self.audio_engine.as_ref().is_some_and(|engine| {
                        if id == opentrack_core::project::OUTPUT_BUS_ID {
                            engine.update_output_channel_parameters(gain, pan, &equalizer);
                            true
                        } else if id < opentrack_core::project::INPUT_BUS_ID {
                            self.data
                                .project
                                .tracks
                                .iter()
                                .position(|track| track.id == id)
                                .is_some_and(|index| {
                                    engine.update_track_channel_parameters(
                                        index, gain, pan, &equalizer,
                                    )
                                })
                        } else {
                            false
                        }
                    })
                });
            if !live_controls_changed {
                self.rebuild_audio_plan();
            }
            self.mark_modified();
        }
        self.update_channel_analyzer();
        if self.channel_settings_window.open {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }

    fn apply_channel_settings_action(&mut self, action: ChannelSettingsAction) -> bool {
        let selected_index = match self.channel_settings_window.channel_bus {
            Some(input) => Some(self.data.project.tracks.len() + if input { 0 } else { 1 }),
            None => self
                .data
                .project
                .selected_track
                .map(|i| i as usize)
                .filter(|i| *i < self.data.project.tracks.len()),
        };
        let (extension, dialog_title, default_name, saving) = match action {
            ChannelSettingsAction::SaveSelected => (
                "otchannel",
                "Save Selected Channel Settings",
                "Channel Settings.otchannel",
                true,
            ),
            ChannelSettingsAction::LoadSelected => {
                ("otchannel", "Load Selected Channel Settings", "", false)
            }
            ChannelSettingsAction::SaveAll => (
                "otmixer",
                "Save All Mixer Settings",
                "Mixer Settings.otmixer",
                true,
            ),
            ChannelSettingsAction::LoadAll => ("otmixer", "Load All Mixer Settings", "", false),
        };

        if saving {
            let channels = match action {
                ChannelSettingsAction::SaveSelected => {
                    let Some(index) = selected_index else {
                        self.status_message =
                            "Select a channel before saving its settings".to_owned();
                        return false;
                    };
                    vec![ChannelSettingsSnapshot::from(
                        self.data.project.channel_at(index).unwrap(),
                    )]
                }
                ChannelSettingsAction::SaveAll => self
                    .data
                    .project
                    .channels()
                    .map(ChannelSettingsSnapshot::from)
                    .collect(),
                _ => return false,
            };
            let mut dialog = FileDialog::new()
                .add_filter("OpenTrack Channel Settings", &[extension])
                .set_title(dialog_title)
                .set_file_name(default_name);
            if let Some(directory) = self.project_path.as_deref().and_then(|path| path.parent()) {
                dialog = dialog.set_directory(directory);
            }
            let Some(mut path) = dialog.save_file() else {
                return false;
            };
            path.set_extension(extension);
            let file = ChannelSettingsPresetFile {
                version: 1,
                channels,
            };
            match serde_json::to_vec_pretty(&file)
                .map_err(|error| error.to_string())
                .and_then(|bytes| std::fs::write(&path, bytes).map_err(|error| error.to_string()))
            {
                Ok(()) => {
                    self.status_message = format!("Saved mixer settings to {}", path.display());
                    false
                }
                Err(error) => {
                    self.status_message = format!("Could not save mixer settings: {error}");
                    false
                }
            }
        } else {
            let Some(path) = FileDialog::new()
                .add_filter("OpenTrack Channel Settings", &[extension])
                .set_title(dialog_title)
                .pick_file()
            else {
                return false;
            };
            let file = match std::fs::read(&path)
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    serde_json::from_slice::<ChannelSettingsPresetFile>(&bytes)
                        .map_err(|error| error.to_string())
                }) {
                Ok(file) if file.version == 1 => file,
                Ok(_) => {
                    self.status_message = "Unsupported mixer settings version".to_owned();
                    return false;
                }
                Err(error) => {
                    self.status_message = format!("Could not read mixer settings: {error}");
                    return false;
                }
            };

            let mut candidate = self.data.project.clone();
            match action {
                ChannelSettingsAction::LoadSelected => {
                    let Some(index) = selected_index else {
                        self.status_message = "Select a channel before loading settings".to_owned();
                        return false;
                    };
                    if file.channels.len() != 1 {
                        self.status_message =
                            "This file does not contain one channel preset".to_owned();
                        return false;
                    }
                    if !channel_snapshot_compatible(
                        &file.channels[0],
                        candidate.channel_at(index).unwrap(),
                    ) {
                        self.status_message =
                            "The selected channel does not match this preset".to_owned();
                        return false;
                    }
                    file.channels[0].apply_to(candidate.channel_at_mut(index).unwrap());
                }
                ChannelSettingsAction::LoadAll => {
                    if file.channels.len() != candidate.tracks.len()
                        && file.channels.len() != candidate.tracks.len() + 2
                    {
                        self.status_message =
                            "The mixer preset track count does not match this project".to_owned();
                        return false;
                    }
                    let mut used_tracks = vec![false; candidate.tracks.len() + 2];
                    let mut targets = Vec::with_capacity(file.channels.len());
                    for snapshot in &file.channels {
                        let Some(index) =
                            candidate.channels().enumerate().find_map(|(index, track)| {
                                (!used_tracks[index]
                                    && channel_snapshot_matches_track(snapshot, track))
                                .then_some(index)
                            })
                        else {
                            self.status_message =
                                "Mixer preset channels do not match this project".to_owned();
                            return false;
                        };
                        used_tracks[index] = true;
                        targets.push(index);
                    }
                    for (snapshot, index) in file.channels.iter().zip(targets) {
                        snapshot.apply_to(candidate.channel_at_mut(index).unwrap());
                    }
                }
                _ => return false,
            }
            if let Err(error) = candidate.validate() {
                self.status_message = format!("Mixer preset is invalid: {error}");
                return false;
            }
            self.data.project = candidate;
            self.status_message = format!("Loaded mixer settings from {}", path.display());
            true
        }
    }

    fn show_media_window(&mut self, ctx: &egui::Context) {
        if !self.media_window.open {
            self.media_window.viewport_live = false;
            return;
        }
        let title = "Media Pool";
        let restore_bounds = if self.media_window.viewport_live {
            None
        } else {
            self.media_window
                .restore_bounds
                .clone()
                .or_else(|| self.media_window.bounds.clone())
        };
        let id = egui::ViewportId::from_hash_of((MEDIA_WINDOW_ID, self.media_window.epoch));
        if self.media_window.focus_requested && self.media_window.viewport_live {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
            self.media_window.focus_requested = false;
        }
        let builder = viewport_builder(
            title,
            Vec2::new(760.0, 560.0),
            Vec2::new(500.0, 360.0),
            restore_bounds.as_ref(),
        )
        .with_visible(self.media_window.viewport_live);
        let mut open = true;
        let mut insert = None;
        let mut bounds = None;
        ctx.show_viewport_immediate(id, builder, |window_ctx, _class| {
            self.handle_native_window_input(window_ctx);
            open &= !window_ctx.input(|input| input.viewport().close_requested());
            egui::CentralPanel::default().show(window_ctx, |ui| {
                let pointer_blocked = self.dialog_owns_input() || self.menu_open;
                let overlay_open = pointer_blocked || egui::Popup::is_any_open(ui.ctx());
                blocked_input_scope(ui, pointer_blocked, overlay_open, |ui| {
                    insert = media_pool::show(ui, &self.data.project);
                });
            });
            bounds = native_window_bounds(window_ctx);
        });
        self.media_window.open = open;
        self.media_window.viewport_live = open;
        if restore_bounds.is_some() {
            self.media_window.restore_bounds = None;
        }
        if let Some(bounds) = bounds {
            self.media_window.bounds = Some(bounds);
        }
        if let Some(path) = insert {
            self.add_pool_media(path);
        }
    }

    fn show_midi_editor_panel(
        &mut self,
        ui: &mut egui::Ui,
        live_velocity_update: &mut Option<(usize, u64, u8)>,
    ) -> bool {
        let selected_track = self
            .data
            .project
            .selected_track
            .map(|index| index as usize)
            .filter(|index| {
                self.data.project.tracks.get(*index).is_some_and(|track| {
                    matches!(track.kind, TrackKind::Midi | TrackKind::Instrument)
                })
            });
        let selected_part_id = selected_track.and_then(|track_index| {
            let part_id = self.data.project.selected_midi_part_id?;
            self.data.project.tracks[track_index]
                .midi_parts
                .iter()
                .any(|part| part.id == part_id)
                .then_some(part_id)
        });
        let mut modified = false;
        if let (Some(track_index), Some(part_id)) = (selected_track, selected_part_id) {
            modified |= show_midi_editor(
                ui,
                &mut self.data.project,
                track_index,
                part_id,
                &mut self.midi_position_quantize_beats,
                &mut self.midi_length_quantize_beats,
                &mut self.midi_insert_velocity,
                &mut self.midi_controller_number,
                live_velocity_update,
                &mut self.selected_midi_note_id,
                &mut self.selected_midi_note_ids,
                &mut self.midi_note_drag,
                &mut self.midi_note_draw,
                &mut self.midi_controller_drag,
                &mut self.edit_tool,
                &mut self.data.transport.position_beats,
                &mut self.midi_pitch_view,
            );
        } else {
            self.midi_note_drag = None;
            self.midi_note_draw = None;
            self.midi_controller_drag = None;
            ui.label("Draw a MIDI part in a MIDI or Instrument track, or select an existing part.");
        }
        modified
    }

    fn show_project_window(&mut self, ui: &mut egui::Ui) {
        self.project_window.focus_requested = false;
        self.project_window.minimized = false;
        self.project_window.maximized = false;
        let panel_rect = ui.max_rect();
        self.project_window.bounds = Some(docked_bounds(panel_rect));
        let palette = theme::palette(ui);
        ui.painter().rect_filled(panel_rect, 0.0, palette.surface);
        let project_name =
            project_display_name(self.project_path.as_deref(), &self.data.project.name);
        if docked_panel_header(ui, &format!("Project - {project_name}"), true) {
            self.project_window.open = false;
            return;
        }
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        self.show_toolbar(ui);
        if self.arrange_view_state.show_info_line {
            show_project_info_line(ui);
        }
        let body_rect = Rect::from_min_max(
            Pos2::new(panel_rect.left(), ui.cursor().top()),
            panel_rect.right_bottom(),
        );
        let bottom_lane_height = 17.0;
        let zoom_width = 130.0;
        let header_readout_width: f32 = (if self.arrange_view_state.show_inspector {
            159.0
        } else {
            0.0
        }) + 260.0;
        let readout_width =
            header_readout_width.min((panel_rect.width() - zoom_width - 8.0).max(0.0));
        let readout = Rect::from_min_size(
            Pos2::new(panel_rect.left(), panel_rect.bottom() - bottom_lane_height),
            Vec2::new(readout_width, bottom_lane_height),
        );
        let zoom_rect = Rect::from_min_max(
            Pos2::new(panel_rect.right() - zoom_width, readout.top()),
            panel_rect.right_bottom(),
        );
        let input_format = self.cached_input_format;
        let rate = format!("{} Hz", self.data.project.sample_rate_hz);
        let recording_directory = self
            .recording_session
            .as_ref()
            .and_then(|session| session.recorder.path.parent())
            .map(Path::to_path_buf)
            .or_else(|| {
                self.project_path
                    .as_deref()
                    .and_then(Path::parent)
                    .filter(|path| !path.as_os_str().is_empty())
                    .map(Path::to_path_buf)
            })
            .or_else(|| std::env::current_dir().ok());
        let active_track_ids = self
            .recording_session
            .as_ref()
            .map(|session| session.track_ids.iter().copied().collect::<HashSet<_>>());
        let mut output_channels = 0_u64;
        for track in self.data.project.tracks.iter().filter(|track| {
            track.kind == TrackKind::Audio
                && active_track_ids
                    .as_ref()
                    .map(|ids| ids.contains(&track.id))
                    .unwrap_or(track.armed)
        }) {
            let route = track
                .input_routing
                .as_deref()
                .unwrap_or(if track.audio_channels == 1 {
                    "Left - Stereo In"
                } else {
                    "Stereo In"
                });
            if route == "No Bus" {
                continue;
            }
            output_channels += if route.contains("Left") || route.contains("Right") {
                1
            } else {
                u64::from(
                    input_format
                        .map(|format| format.channels)
                        .unwrap_or(u16::from(track.audio_channels))
                        .clamp(1, 2),
                )
            };
        }
        if output_channels == 0 {
            output_channels = u64::from(
                input_format
                    .map(|format| format.channels)
                    .unwrap_or(2)
                    .clamp(1, 2),
            );
        }
        let maximum = recording_directory
            .as_deref()
            .and_then(|directory| available_disk_space_bytes(directory))
            .and_then(|available| {
                let bytes_per_second = u64::from(self.data.project.sample_rate_hz)
                    .checked_mul(u64::from(
                        self.data.project.recording_format.bits_per_sample() / 8,
                    ))?
                    .checked_mul(output_channels)?;
                (bytes_per_second > 0).then(|| {
                    let seconds = available / bytes_per_second;
                    format!("{:03}h {:02}m", seconds / 3600, (seconds % 3600) / 60)
                })
            })
            .unwrap_or_else(|| "unavailable".to_owned());
        let recording_format_label = match self.data.project.recording_format {
            RecordingFormat::Pcm16 => "16 Bit PCM",
            RecordingFormat::Pcm24 => "24 Bit PCM",
            RecordingFormat::Float32 => "32 Bit Float",
        };
        let beats_per_bar = self.data.project.beats_per_bar;
        let position_before = self.data.transport.position_beats;
        let sample_rate = self.data.project.sample_rate_hz;
        let mut context_action = None;
        let can_remove = !self.is_recording();
        let recording_preview = self.recording_session.as_ref().map(|s| RecordingPreview {
            start_beats: s.start_beats,
            tracks: s
                .track_ids
                .iter()
                .copied()
                .zip(s.routes.iter().copied())
                .collect(),
            waveform: s.recorder.live_waveform(),
        });
        let project_changes = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt("project-panel-body")
                    .max_rect(body_rect),
                |ui| {
                    ui.set_clip_rect(body_rect);
                    ui.set_width(body_rect.width());
                    show_project(
                        ui,
                        &mut self.edit_tool,
                        &mut self.data.project,
                        &mut self.data.transport.position_beats,
                        self.data.transport.cycle,
                        beats_per_bar,
                        sample_rate,
                        &self.data.track_meter_levels,
                        &self.data.peak_holds,
                        &self.data.audio_sources,
                        recording_preview.as_ref(),
                        &mut self.arrange_view_state,
                        &mut self.midi_editor_open,
                        &mut self.midi_part_draw,
                        &mut self.channel_settings_window,
                        &mut self.track_rename_dialog,
                        can_remove,
                        &mut context_action,
                    )
                },
            )
            .inner;
        let mut modified = project_changes.modified;
        let mut live_mix_update = project_changes.live_mix;
        let defer_audio_plan = project_changes.defer_audio_plan;
        let settings_response = ui.interact(
            readout,
            ui.id().with("project-recording-setup"),
            Sense::click(),
        );
        if settings_response.clicked() {
            self.project_setup_open = true;
        }
        ui.painter().rect_filled(
            readout,
            0.0,
            if settings_response.hovered() {
                theme::hover_button_fill(palette.canvas, ui.visuals())
            } else {
                palette.canvas
            },
        );
        ui.painter().with_clip_rect(readout).text(
            readout.left_center() + Vec2::new(3.0, 0.0),
            Align2::LEFT_CENTER,
            format!("Rec: {rate} - {recording_format_label} - Max: {maximum}"),
            FontId::proportional(9.0),
            palette.text,
        );
        project_controls::zoom_control(ui, zoom_rect, &mut self.arrange_view_state.timeline_zoom);
        if self.midi_editor_open {
            self.mixer_window.open = true;
            self.sample_editor_active = false;
        }
        if (self.data.transport.position_beats - position_before).abs() > f64::EPSILON
            && let Some(engine) = &self.audio_engine
        {
            engine.seek_seconds(
                self.data.transport.position_beats.max(0.0) * 60.0
                    / self.data.project.tempo_bpm.max(1.0),
            );
        }
        if let Some(target) = self.arrange_view_state.audio_events.open_editor.take() {
            self.sample_editor.target = Some(target);
            self.sample_editor_active = true;
            self.mixer_window.open = true;
        }
        if let Some((track, id, name)) = self.arrange_view_state.audio_events.rename_request.take()
        {
            match self.rename_event_source(track, id, &name) {
                Ok(changed) => {
                    modified |= changed;
                    if changed {
                        live_mix_update = None;
                    }
                }
                Err(error) => {
                    self.status_message = error.clone();
                    self.media_operation_error = Some(error);
                }
            }
        }
        if let Some(message) = self.arrange_view_state.audio_events.message.take() {
            self.status_message = message;
        }
        ui.painter().rect_stroke(
            panel_rect,
            0.0,
            Stroke::new(1.0, palette.grid),
            egui::StrokeKind::Inside,
        );
        match context_action {
            Some(project_controls::Action::Add(kind)) => {
                self.track_add_dialog = Some(TrackAddDialog {
                    kind,
                    count: 1,
                    audio_channels: 1,
                    browse_presets: false,
                });
            }
            Some(project_controls::Action::Remove(id)) => {
                if can_remove {
                    self.pending_track_delete_id = Some(id);
                }
            }
            Some(project_controls::Action::Duplicate(id)) => {
                self.duplicate_tracks(Some(id));
            }
            Some(project_controls::Action::ShowAutomation(id)) => {
                let changed = self.arrange_view_state.automation_visible.insert(id);
                modified |= changed;
                if changed {
                    live_mix_update = None;
                }
            }
            Some(project_controls::Action::HideAllAutomation) => {
                if !self.arrange_view_state.automation_visible.is_empty() {
                    self.arrange_view_state.automation_visible.clear();
                    modified = true;
                    live_mix_update = None;
                }
            }
            None => {}
        }
        if modified {
            self.mark_modified();
            let live_mix_updated = live_mix_update.is_some_and(|(track_id, gain, pan)| {
                self.data
                    .project
                    .tracks
                    .iter()
                    .position(|track| track.id == track_id)
                    .is_some_and(|index| {
                        self.audio_engine
                            .as_ref()
                            .is_some_and(|engine| engine.update_track_mix(index, gain, pan))
                    })
            });
            if !live_mix_updated && !defer_audio_plan {
                self.rebuild_audio_plan();
            }
        }
    }

    fn show_docked_workspace(&mut self, ui: &mut egui::Ui) {
        if self.midi_editor_open {
            self.mixer_window.open = true;
        }
        let area = ui.max_rect();
        let project_open = self.project_window.open;
        let mixer_open = self.mixer_window.open || self.midi_editor_open;
        if !project_open && !mixer_open {
            return;
        }
        let docked_right = (area.right() - PANEL_RIGHT_GUTTER).max(area.left());
        if project_open && mixer_open {
            let minimum_height = 140.0_f32.min(area.height() * 0.4);
            let target_fraction = if self.arrange_view_state.project_dock_fraction.is_finite() {
                self.arrange_view_state
                    .project_dock_fraction
                    .clamp(0.25, 0.75)
            } else {
                PROJECT_PANEL_HEIGHT / area.height().max(1.0)
            };
            let split_y = (area.top() + area.height() * target_fraction)
                .clamp(area.top() + minimum_height, area.bottom() - minimum_height);
            let project_rect = Rect::from_min_max(area.min, Pos2::new(docked_right, split_y));
            let mixer_rect = Rect::from_min_max(
                Pos2::new(area.left(), split_y),
                Pos2::new(docked_right, area.bottom()),
            );
            self.show_docked_panel(ui, project_rect, true);
            self.show_docked_panel(ui, mixer_rect, false);
            let divider = Rect::from_min_max(
                Pos2::new(area.left(), split_y - 2.0),
                Pos2::new(docked_right, split_y + 2.0),
            );
            ui.painter()
                .rect_filled(divider, 0.0, theme::palette(ui).grid);
            let response = ui
                .interact(
                    divider,
                    Id::new("project-mixer-dock-divider"),
                    Sense::drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeVertical);
            if response.dragged()
                && let Some(pointer) = response.interact_pointer_pos()
            {
                self.arrange_view_state.project_dock_fraction =
                    ((pointer.y - area.top()) / area.height()).clamp(0.25, 0.75);
            }
        } else if project_open {
            self.show_docked_panel(
                ui,
                Rect::from_min_max(area.min, Pos2::new(docked_right, area.bottom())),
                true,
            );
        } else {
            let height = MIXER_PANEL_HEIGHT.min(area.height());
            let rect = Rect::from_min_max(
                Pos2::new(area.left(), area.bottom() - height),
                Pos2::new(docked_right, area.bottom()),
            );
            self.show_docked_panel(ui, rect, false);
        }
    }

    fn show_docked_panel(&mut self, ui: &mut egui::Ui, rect: Rect, project: bool) {
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt(if project {
                    "project-docked-panel"
                } else {
                    "mixer-docked-panel"
                })
                .max_rect(rect),
            |ui| {
                ui.set_clip_rect(rect);
                ui.set_width(rect.width());
                ui.set_height(rect.height());
                if project {
                    self.show_project_window(ui);
                } else {
                    self.show_mixer_window(ui);
                }
            },
        );
    }

    fn show_track_add_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.track_add_dialog.take() else {
            let mut open = false;
            dismiss_window_on_outside_click(
                ctx,
                Id::new("add-track-configuration-dialog"),
                &mut open,
                None::<&egui::InnerResponse<()>>,
            );
            return;
        };
        let title = match dialog.kind {
            TrackKind::Audio => "Add Audio Track",
            TrackKind::Group => "Add Group Channel",
            TrackKind::Instrument => "Add Instrument Track",
            TrackKind::Midi => "Add MIDI Track",
        };
        let configuration = match dialog.kind {
            TrackKind::Audio => "Audio Track Configuration",
            TrackKind::Group => "Group Channel Configuration",
            TrackKind::Instrument => "Instrument Track Configuration",
            TrackKind::Midi => "MIDI Track Configuration",
        };
        let viewport = ctx.content_rect();
        // Window::fixed_size applies to the content area; the captured outer window is 338×162.
        let dialog_size = Vec2::new(338.0, 128.0);
        let dialog_pos = Pos2::new(
            (viewport.width() - dialog_size.x).max(0.0) * 0.5,
            (viewport.height() - dialog_size.y).max(0.0) * 0.5,
        );
        let mut open = true;
        let mut accepted = false;
        let mut cancelled = false;
        let response = egui::Window::new(title)
            .id(Id::new("add-track-configuration-dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos(dialog_pos)
            .fixed_size(dialog_size)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 1.0);
                ui.spacing_mut().interact_size.y = 16.0;
                ui.spacing_mut().button_padding.y = 1.0;
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 1.0);
                        ui.spacing_mut().interact_size.y = 16.0;
                        ui.spacing_mut().button_padding.y = 1.0;
                        ui.vertical(|ui| {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(configuration).size(12.0))
                            });
                            ui.separator();
                            ui.horizontal(|ui| {
                                let controls_width = match dialog.kind {
                                    TrackKind::Audio => 40.0 + 6.0 + 78.0 + 6.0 + 78.0,
                                    TrackKind::Group => 40.0 + 6.0 + 78.0,
                                    TrackKind::Instrument => 40.0 + 6.0 + 160.0,
                                    TrackKind::Midi => 40.0,
                                };
                                ui.add_space(
                                    ((ui.available_width() - controls_width) * 0.5 - 18.0).max(0.0),
                                );
                                ui.vertical(|ui| {
                                    ui.add_sized(
                                        Vec2::new(40.0, 16.0),
                                        egui::DragValue::new(&mut dialog.count)
                                            .range(1..=64)
                                            .speed(1),
                                    );
                                    ui.label(egui::RichText::new("count").size(10.0));
                                });
                                match dialog.kind {
                                    TrackKind::Audio => {
                                        ui.vertical(|ui| {
                                            egui::ComboBox::from_id_salt(
                                                "add-audio-track-configuration",
                                            )
                                            .width(78.0)
                                            .selected_text(if dialog.audio_channels == 1 {
                                                "Mono"
                                            } else {
                                                "Stereo"
                                            })
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    ui.selectable_value(
                                                        &mut dialog.audio_channels,
                                                        1,
                                                        "Mono",
                                                    );
                                                    ui.selectable_value(
                                                        &mut dialog.audio_channels,
                                                        2,
                                                        "Stereo",
                                                    );
                                                },
                                            );
                                            ui.label(
                                                egui::RichText::new("configuration").size(10.0),
                                            );
                                        });
                                        ui.vertical(|ui| {
                                            // Only the displayed default is recorded; speaker choices are unverified.
                                            ui.add_enabled_ui(false, |ui| {
                                                egui::ComboBox::from_id_salt("add-audio-speakers")
                                                    .width(78.0)
                                                    .selected_text("M")
                                                    .show_ui(ui, |_| {});
                                            });
                                            ui.label(egui::RichText::new("speakers").size(10.0));
                                        });
                                    }
                                    TrackKind::Group => {
                                        ui.vertical(|ui| {
                                            egui::ComboBox::from_id_salt(
                                                "add-group-channel-configuration",
                                            )
                                            .width(78.0)
                                            .selected_text(if dialog.audio_channels == 1 {
                                                "Mono"
                                            } else {
                                                "Stereo"
                                            })
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    ui.selectable_value(
                                                        &mut dialog.audio_channels,
                                                        1,
                                                        "Mono",
                                                    );
                                                    ui.selectable_value(
                                                        &mut dialog.audio_channels,
                                                        2,
                                                        "Stereo",
                                                    );
                                                },
                                            );
                                            ui.label(
                                                egui::RichText::new("configuration").size(10.0),
                                            );
                                        });
                                    }
                                    TrackKind::Instrument => {
                                        ui.add_enabled_ui(false, |ui| {
                                            egui::ComboBox::from_id_salt("add-instrument-choice")
                                                .selected_text("No VST Instrument")
                                                .show_ui(ui, |_| {});
                                        });
                                    }
                                    TrackKind::Midi => {}
                                }
                            });
                            ui.radio_value(&mut dialog.browse_presets, true, "Browse Presets");
                        });
                    });
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    if ui
                        .add_sized(
                            Vec2::new(70.0, 22.0),
                            egui::Button::new(egui::RichText::new("Cancel").size(12.0)),
                        )
                        .clicked()
                    {
                        cancelled = true;
                    }
                    if ui
                        .add_sized(
                            Vec2::new(78.0, 22.0),
                            egui::Button::new(egui::RichText::new("OK").size(12.0)),
                        )
                        .clicked()
                    {
                        accepted = true;
                    }
                });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("add-track-configuration-dialog"),
            &mut open,
            response.as_ref(),
        );
        if accepted && open {
            let count = dialog.count.clamp(1, 64);
            let mut last_track_id = None;
            for _ in 0..count {
                let index = self
                    .data
                    .project
                    .insert_track_below_selection(dialog.kind, dialog.audio_channels);
                last_track_id = self.data.project.tracks.get(index).map(|track| track.id);
            }
            self.arrange_view_state.selected_track_ids.clear();
            if let Some(track_id) = last_track_id {
                self.arrange_view_state.selected_track_ids.insert(track_id);
                self.arrange_view_state.selection_anchor_track_id = Some(track_id);
            }
            self.mark_modified();
            self.rebuild_audio_plan();
        } else if !cancelled && open {
            self.track_add_dialog = Some(dialog);
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(ui);
        let (band, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        ui.painter()
            .rect_filled(band, 0.0, palette.canvas_alternate);
        ui.painter().line_segment(
            [band.left_bottom(), band.right_bottom()],
            Stroke::new(1.0, palette.grid),
        );
        let inner = Rect::from_min_max(
            Pos2::new(band.left(), band.top() + 2.0),
            Pos2::new(band.right(), band.bottom() - 2.0),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("project-toolbar-band")
                .max_rect(inner),
            |ui| {
                ui.set_clip_rect(inner);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                    ui.add_space(10.0);
                    let _ = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Power,
                        "Activate Project",
                        false,
                        false,
                    );
                    let _ = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Constraint,
                        "Constrain Delay Compensation",
                        false,
                        false,
                    );
                    ui.separator();
                    let inspector = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Inspector,
                        if self.arrange_view_state.show_inspector {
                            "Hide Inspector"
                        } else {
                            "Show Inspector"
                        },
                        self.arrange_view_state.show_inspector,
                        true,
                    );
                    if inspector.clicked() {
                        self.arrange_view_state.show_inspector =
                            !self.arrange_view_state.show_inspector;
                    }
                    let info_line = toolbar_icon_button(
                        ui,
                        ToolbarIcon::InfoLine,
                        if self.arrange_view_state.show_info_line {
                            "Hide Event Info Line"
                        } else {
                            "Show Event Info Line"
                        },
                        self.arrange_view_state.show_info_line,
                        true,
                    );
                    if info_line.clicked() {
                        self.arrange_view_state.show_info_line =
                            !self.arrange_view_state.show_info_line;
                    }
                    let _ = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Overview,
                        "Show Overview",
                        false,
                        false,
                    );
                    let pool = toolbar_window_toggle(
                        ui,
                        ToolbarIcon::Pool,
                        &mut self.media_window,
                        "Open Pool",
                        "Close Pool",
                    );
                    if pool.clicked() && self.media_window.open {
                        self.data.selected_media = 0;
                    }
                    toolbar_window_toggle(
                        ui,
                        ToolbarIcon::Mixer,
                        &mut self.mixer_window,
                        "Open Mixer",
                        "Close Mixer",
                    );
                    let _ = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Automation,
                        "Show Automation Panel",
                        false,
                        false,
                    );
                    let mode = toolbar_choice(ui, "Touch", 84.0);
                    mode.on_hover_text("Global Automation Mode");

                    ui.separator();
                    let previous = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Previous,
                        "Goto Previous Marker / Zero",
                        false,
                        true,
                    );
                    if previous.clicked() {
                        self.data.transport.position_beats = 0.0;
                        self.set_transport_playing(false);
                        if let Some(engine) = &self.audio_engine {
                            engine.seek_seconds(0.0);
                        }
                    }
                    let _ = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Next,
                        "Goto Next Marker",
                        false,
                        false,
                    );
                    let cycle = toolbar_icon_button_tinted(
                        ui,
                        ToolbarIcon::Cycle,
                        "Transport Cycle On/Off",
                        self.data.transport.cycle,
                        true,
                        Some(crate::timeline::cycle_color(self.data.project.cycle_mode)),
                    );
                    if cycle.clicked() {
                        self.toggle_cycle();
                    }
                    let stop =
                        toolbar_icon_button(ui, ToolbarIcon::Stop, "Transport Stop", false, true);
                    if stop.clicked() {
                        self.set_transport_playing(false);
                        self.data.transport.recording = false;
                    }
                    let play = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Play,
                        "Transport Play",
                        self.data.transport.playing,
                        true,
                    );
                    if play.clicked() {
                        if self.is_recording() {
                            self.finish_recording();
                        }
                        self.set_transport_playing(true);
                        self.data.transport.recording = false;
                    }
                    let record = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Record,
                        "Transport Record",
                        self.data.transport.recording,
                        true,
                    );
                    if record.clicked() {
                        self.toggle_recording();
                    }

                    ui.separator();
                    for (icon, tooltip, tool, enabled) in [
                        (
                            ToolbarIcon::Select,
                            "Object Selection",
                            EditTool::Select,
                            true,
                        ),
                        (ToolbarIcon::Split, "Scissors", EditTool::Split, true),
                        (ToolbarIcon::Draw, "Draw MIDI Parts", EditTool::Draw, true),
                        (ToolbarIcon::Range, "Range Selection", EditTool::Range, true),
                        (ToolbarIcon::Glue, "Glue", EditTool::Glue, true),
                        (ToolbarIcon::Erase, "Erase", EditTool::Delete, true),
                        (ToolbarIcon::Zoom, "Zoom", EditTool::Zoom, true),
                        (ToolbarIcon::Mute, "Mute", EditTool::Mute, true),
                    ] {
                        let response =
                            toolbar_icon_button(ui, icon, tooltip, self.edit_tool == tool, enabled);
                        if response.clicked() {
                            self.edit_tool = tool;
                        }
                    }
                    let _ =
                        toolbar_icon_button(ui, ToolbarIcon::TimeWarp, "Time Warp", false, false);
                    let _ = toolbar_icon_button(ui, ToolbarIcon::Line, "Line", false, false);
                    if toolbar_icon_button(
                        ui,
                        ToolbarIcon::PlayTool,
                        "Play",
                        self.edit_tool == EditTool::Play,
                        true,
                    )
                    .clicked()
                    {
                        self.edit_tool = EditTool::Play;
                    }
                    let _ = toolbar_icon_button(ui, ToolbarIcon::Color, "Color Tool", false, false);

                    ui.separator();
                    let root_key = toolbar_icon_button(
                        ui,
                        ToolbarIcon::RootKey,
                        "Project Root Key",
                        false,
                        false,
                    );
                    let _ = root_key;
                    let autoscroll = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Autoscroll,
                        "Autoscroll",
                        self.arrange_view_state.autoscroll_enabled,
                        true,
                    );
                    if autoscroll.clicked() {
                        self.arrange_view_state.autoscroll_enabled =
                            !self.arrange_view_state.autoscroll_enabled;
                    }
                    let snap = toolbar_icon_button(
                        ui,
                        ToolbarIcon::Snap,
                        "Snap On/Off",
                        self.arrange_view_state.snap_enabled,
                        true,
                    );
                    if snap.clicked() {
                        self.arrange_view_state.snap_enabled =
                            !self.arrange_view_state.snap_enabled;
                    }
                    let _ =
                        toolbar_icon_button(ui, ToolbarIcon::SnapType, "Snap Type", false, false);
                    ui.add_space(5.0);
                    project_grid_choices(ui, &mut self.arrange_view_state);
                    ui.add_space(14.0);
                    let _ = toolbar_icon_button(
                        ui,
                        ToolbarIcon::ZeroCrossing,
                        "Snap to Zero Crossing",
                        false,
                        false,
                    );
                    ui.add_space(13.0);
                    let selector_color = self
                        .arrange_view_state
                        .selected_event_color
                        .and_then(|index| {
                            self.arrange_view_state.event_colors.get(usize::from(index))
                        })
                        .map(|rgb| Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
                        .unwrap_or_else(|| Color32::from_gray(160));
                    let (selector_rect, selector_response) =
                        ui.allocate_exact_size(Vec2::new(42.0, 22.0), Sense::click());
                    ui.painter()
                        .rect_filled(selector_rect, 1.0, theme::palette(ui).surface_raised);
                    let swatch = Rect::from_min_size(
                        selector_rect.min + Vec2::new(4.0, 3.0),
                        Vec2::new(25.0, 16.0),
                    );
                    ui.painter().rect_filled(swatch, 1.0, selector_color);
                    ui.painter().rect_stroke(
                        swatch,
                        1.0,
                        Stroke::new(1.0, theme::palette(ui).grid),
                        egui::StrokeKind::Inside,
                    );
                    crate::icon_system::paint_app_icon(
                        ui,
                        Rect::from_center_size(
                            Pos2::new(selector_rect.right() - 9.0, selector_rect.center().y),
                            Vec2::splat(18.0),
                        ),
                        crate::icon_system::IconId::ChevronDown,
                        theme::palette(ui).text,
                    );
                    let mut color_choice = None;
                    let mut open_color_editor = false;
                    egui::Popup::menu(&selector_response)
                        .align(egui::RectAlign::BOTTOM_START)
                        .width(152.0)
                        .show(|ui| {
                            theme::menu_popup_style(ui.style_mut());
                            let (default_row, default_response) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 20.0),
                                Sense::click(),
                            );
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    default_row.min + Vec2::new(5.0, 3.0),
                                    Vec2::new(14.0, 14.0),
                                ),
                                0.0,
                                Color32::from_gray(160),
                            );
                            ui.painter().text(
                                default_row.left_center() + Vec2::new(25.0, 0.0),
                                Align2::LEFT_CENTER,
                                "Default Color",
                                FontId::proportional(11.0),
                                theme::palette(ui).text,
                            );
                            if default_response.clicked() {
                                color_choice = Some(None);
                                ui.close();
                            }
                            for (index, rgb) in
                                self.arrange_view_state.event_colors.iter().enumerate()
                            {
                                let (row, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 20.0),
                                    Sense::click(),
                                );
                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        row.min + Vec2::new(5.0, 3.0),
                                        Vec2::new(14.0, 14.0),
                                    ),
                                    0.0,
                                    Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                                );
                                ui.painter().text(
                                    row.left_center() + Vec2::new(25.0, 0.0),
                                    Align2::LEFT_CENTER,
                                    format!("Color {}", index + 1),
                                    FontId::proportional(11.0),
                                    theme::palette(ui).text,
                                );
                                if response.clicked() {
                                    color_choice = Some(Some(index as u8));
                                    ui.close();
                                }
                            }
                            ui.separator();
                            if ui.button("Select Colors...").clicked() {
                                open_color_editor = true;
                                ui.close();
                            }
                        });
                    selector_response.on_hover_text("Color Selector");
                    if let Some(color_index) = color_choice {
                        self.arrange_view_state.selected_event_color = color_index;
                        self.mark_modified();
                    }
                    if open_color_editor {
                        self.open_event_colors_dialog();
                    }
                });
            },
        );
        ui.advance_cursor_after_rect(band);
    }

    fn show_transport(&mut self, ui: &mut egui::Ui) {
        transport_panel::show(self, ui);
    }

    fn realtime_diagnostic_text(&self) -> Option<String> {
        let engine = self.audio_engine.as_ref()?;
        let asio_overloads = engine.asio_overload_window_count();
        let asio_collisions = engine.asio_callback_collision_count().unwrap_or(0);
        let mut diagnostic = format!(
            "Device {} | {} Hz | period {} frames | buffer {} frames | DSP {:.0}% | {} render misses | {} ASIO overload windows | {} ASIO callback collisions",
            engine.device_name,
            engine.sample_rate,
            engine.output_period_frames,
            engine.output_buffer_frames,
            engine.callback_load_percent(),
            engine.callback_overrun_count(),
            asio_overloads.map_or_else(|| "n/a".to_owned(), |count| count.to_string()),
            asio_collisions,
        );
        let mut has_callback_timing = false;
        if let Some(timing) = engine.callback_timing_summary() {
            has_callback_timing = true;
            diagnostic.push_str(&format!(
                " | render interval p95/max {} / {} us | render duration p95/max {} / {} us | {} samples / {} overwritten",
                timing.interval_p95_us,
                timing.interval_max_us,
                timing.duration_p95_us,
                timing.duration_max_us,
                timing.samples,
                timing.overwritten_samples,
            ));
        }
        if let Some(timing) = engine.asio_callback_timing_summary() {
            has_callback_timing = true;
            let (configured_block, effective_block) = engine.hosted_internal_block_frames();
            let period_us = u64::from(engine.output_period_frames) * 1_000_000
                / u64::from(engine.sample_rate.max(1));
            diagnostic.push_str(&format!(
                " | ASIO interval p95/max {} / {} us | duration p95/max {} / {} us | period {} us | host block {} configured / {} effective",
                timing.interval_p95_us,
                timing.interval_max_us,
                timing.duration_p95_us,
                timing.duration_max_us,
                period_us,
                configured_block,
                effective_block,
            ));
        }
        if let Some(timing) = engine.wasapi_callback_timing_summary() {
            has_callback_timing = true;
            diagnostic.push_str(&format!(
                " | WASAPI fill interval p95/max {} / {} us | duration p95/max {} / {} us | period {} frames | {} samples / {} overwritten",
                timing.interval_p95_us,
                timing.interval_max_us,
                timing.duration_p95_us,
                timing.duration_max_us,
                engine.output_period_frames,
                timing.samples,
                timing.overwritten_samples,
            ));
        }
        for (track_index, timing) in engine.hosted_track_timing_summaries() {
            let track_label = self
                .data
                .project
                .tracks
                .get(track_index)
                .map(|track| track.name.as_str())
                .unwrap_or("unknown");
            diagnostic.push_str(&format!(
                " | track {} ({track_label}) hosted block p95 {} us / max {} us | {} samples / {} overwritten",
                track_index,
                timing.duration_p95_us,
                timing.duration_max_us,
                timing.samples,
                timing.overwritten_samples,
            ));
        }
        if let Some(timing) = engine.output_stage_timing_summary() {
            diagnostic.push_str(&format!(
                " | Stereo Out block p95 {} us / max {} us | {} samples / {} overwritten",
                timing.duration_p95_us,
                timing.duration_max_us,
                timing.samples,
                timing.overwritten_samples,
            ));
        }
        let input_timing = self
            .recording_session
            .as_ref()
            .and_then(|session| session.recorder.wasapi_input_timing_summary())
            .or_else(|| {
                self.audio_input_probe
                    .as_ref()
                    .and_then(AudioInputProbe::wasapi_input_timing_summary)
            });
        if let Some(timing) = input_timing {
            diagnostic.push_str(&format!(
                " | WASAPI input packet p95 {} us / max {} us | {} samples / {} overwritten",
                timing.duration_p95_us,
                timing.duration_max_us,
                timing.samples,
                timing.overwritten_samples,
            ));
        }
        if let Some(mmcss) = engine.asio_mmcss_summary() {
            diagnostic.push_str(&format!(" | {mmcss}"));
        }
        if !has_callback_timing {
            diagnostic.push_str(
                " | timing rings unavailable; restart with OPENTRACK_REALTIME_DIAGNOSTICS=1",
            );
        }
        Some(diagnostic)
    }

    fn show_status(&mut self, ui: &mut egui::Ui) {
        let modified = self.has_unsaved_changes();
        let (state, color) = if modified {
            ("MODIFIED", theme::WARNING)
        } else if self.project_path.is_some() {
            ("SAVED", theme::palette(ui).accent)
        } else {
            ("NEW", theme::palette(ui).accent)
        };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(state).small().color(color));
            ui.separator();
            ui.label(
                egui::RichText::new(format!("{} tracks", self.data.project.tracks.len()))
                    .small()
                    .color(theme::palette(ui).muted),
            );
            if !self.status_message.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new(&self.status_message).small().color(
                    if self.status_message.starts_with("Could not") {
                        theme::RECORD
                    } else {
                        theme::palette(ui).muted
                    },
                ));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(engine) = &self.audio_engine {
                    ui.ctx().request_repaint_after(Duration::from_millis(250));
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {} Hz · {} ch",
                            engine.device_name, engine.sample_rate, engine.channels
                        ))
                        .small()
                        .color(theme::palette(ui).muted),
                    );
                    ui.separator();
                    let overruns = engine.callback_overrun_count();
                    let asio_overloads = engine.asio_overload_window_count();
                    let asio_collisions = engine.asio_callback_collision_count();
                    let refresh_details = self
                        .realtime_diagnostic_refreshed_at
                        .is_none_or(|last| last.elapsed() >= Duration::from_millis(250));
                    if refresh_details {
                    let diagnostic = if let Some(windows) = asio_overloads {
                        format!(
                            "DSP {:.0}% · {overruns} render overruns · {windows} ASIO overload windows",
                            engine.callback_load_percent(),
                        )
                    } else {
                        format!(
                            "DSP {:.0}% · {overruns} render overruns",
                            engine.callback_load_percent(),
                        )
                    };
                    let diagnostic = if let Some(timing) = engine.callback_timing_summary() {
                        format!(
                            "{diagnostic} | render interval p95/max {} / {} us | render duration p95/max {} / {} us | {} samples | {} lost",
                            timing.interval_p95_us,
                            timing.interval_max_us,
                            timing.duration_p95_us,
                            timing.duration_max_us,
                            timing.samples,
                            timing.overwritten_samples,
                        )
                    } else {
                        diagnostic
                    };
                    let diagnostic = if let Some(timing) = engine.asio_callback_timing_summary() {
                        let (configured_block, effective_block) =
                            engine.hosted_internal_block_frames();
                        let period_frames = engine.output_period_frames;
                        let period_us = u64::from(period_frames) * 1_000_000
                            / u64::from(engine.sample_rate.max(1));
                        format!(
                            "{diagnostic} | ASIO interval p95/max {} / {} us | duration p95/max {} / {} us | period {period_frames} frames / {period_us} us | host block {configured_block} configured / {effective_block} effective",
                            timing.interval_p95_us,
                            timing.interval_max_us,
                            timing.duration_p95_us,
                            timing.duration_max_us,
                        )
                    } else {
                        diagnostic
                    };
                    let diagnostic = if let Some(timing) = engine.wasapi_callback_timing_summary() {
                        format!(
                            "{diagnostic} | WASAPI fill interval p95/max {} / {} us | duration p95/max {} / {} us | period {} frames | {} samples | {} lost",
                            timing.interval_p95_us,
                            timing.interval_max_us,
                            timing.duration_p95_us,
                            timing.duration_max_us,
                            engine.output_period_frames,
                            timing.samples,
                            timing.overwritten_samples,
                        )
                    } else {
                        diagnostic
                    };
                    let diagnostic = if let Some((track_index, timing)) = engine
                        .hosted_track_timing_summaries()
                        .into_iter()
                        .max_by_key(|(_, timing)| timing.duration_p95_us)
                    {
                        format!(
                            "{diagnostic} | hosted track {track_index} block p95 {} us / max {} us | {} samples / {} overwritten",
                            timing.duration_p95_us,
                            timing.duration_max_us,
                            timing.samples,
                            timing.overwritten_samples,
                        )
                    } else {
                        diagnostic
                    };
                    let diagnostic = if let Some(timing) = engine.output_stage_timing_summary() {
                        format!(
                            "{diagnostic} | Stereo Out block p95 {} us / max {} us | {} samples / {} overwritten",
                            timing.duration_p95_us,
                            timing.duration_max_us,
                            timing.samples,
                            timing.overwritten_samples,
                        )
                    } else {
                        diagnostic
                    };
                    let input_timing = self
                        .recording_session
                        .as_ref()
                        .and_then(|session| session.recorder.wasapi_input_timing_summary())
                        .or_else(|| {
                            self.audio_input_probe
                                .as_ref()
                                .and_then(AudioInputProbe::wasapi_input_timing_summary)
                        });
                    let diagnostic = if let Some(timing) = input_timing {
                        format!(
                            "{diagnostic} | WASAPI input packet p95 {} us / max {} us | {} samples / {} overwritten",
                            timing.duration_p95_us,
                            timing.duration_max_us,
                            timing.samples,
                            timing.overwritten_samples,
                        )
                    } else {
                        diagnostic
                    };
                    let diagnostic = format!(
                        "Device {} | {} Hz | period {} frames | buffer {} frames | ASIO callback collisions {} | {diagnostic}",
                        engine.device_name,
                        engine.sample_rate,
                        engine.output_period_frames,
                        engine.output_buffer_frames,
                        asio_collisions.unwrap_or(0),
                    );
                    let diagnostic = if let Some(mmcss) = engine.asio_mmcss_summary() {
                        format!("{diagnostic} | {mmcss}")
                    } else {
                        diagnostic
                    };
                    self.realtime_diagnostic_details = diagnostic;
                    self.realtime_diagnostic_refreshed_at = Some(Instant::now());
                    }
                    let has_overruns = overruns > 0
                        || asio_overloads.is_some_and(|count| count > 0)
                        || asio_collisions.is_some_and(|count| count > 0);
                    let brief = if let Some(windows) = asio_overloads {
                        let collisions = asio_collisions.unwrap_or(0);
                        format!(
                            "DSP {:.0}% | {overruns} misses | {windows} ASIO overload windows | {collisions} callback collisions",
                            engine.callback_load_percent(),
                        )
                    } else {
                        format!(
                            "DSP {:.0}% | {overruns} render misses",
                            engine.callback_load_percent(),
                        )
                    };
                    ui.label(
                        egui::RichText::new(brief)
                        .small()
                        .color(if has_overruns {
                            theme::WARNING
                        } else {
                            theme::palette(ui).muted
                        }),
                    )
                    .on_hover_text(self.realtime_diagnostic_details.as_str())
                    .context_menu(|ui| {
                        if ui.button("Copy realtime diagnostics").clicked() {
                            ui.ctx()
                                .copy_text(self.realtime_diagnostic_details.clone());
                            ui.close();
                        }
                    });
                } else {
                    ui.label(
                        egui::RichText::new("Audio output unavailable")
                            .small()
                            .color(theme::WARNING),
                    );
                }
            });
        });
    }

    fn show_realtime_diagnostics(&mut self, ctx: &egui::Context) {
        if !self.realtime_diagnostics_open {
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(250));
        if self
            .realtime_diagnostic_refreshed_at
            .is_none_or(|last| last.elapsed() >= Duration::from_millis(250))
        {
            self.realtime_diagnostic_details = self
                .realtime_diagnostic_text()
                .unwrap_or_else(|| "Audio output unavailable".to_owned());
            self.realtime_diagnostic_refreshed_at = Some(Instant::now());
        }

        let mut open = self.realtime_diagnostics_open;
        let diagnostics = self.realtime_diagnostic_details.clone();
        egui::Window::new("Realtime Performance")
            .open(&mut open)
            .resizable(true)
            .default_width(560.0)
            .show(ctx, |ui| {
                ui.label("Live device and audio processing measurements");
                egui::ScrollArea::vertical()
                    .max_height(230.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&diagnostics).monospace());
                    });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.audio_engine.is_some(),
                            egui::Button::new("Copy diagnostics"),
                        )
                        .clicked()
                    {
                        ui.ctx().copy_text(diagnostics.clone());
                        self.status_message = "Realtime diagnostics copied".to_owned();
                    }
                    if self.audio_engine.is_some()
                        && !diagnostics.contains("timing rings unavailable")
                    {
                        ui.label("Timing capture active");
                    }
                });
            });
        self.realtime_diagnostics_open = open;
    }
}

impl eframe::App for OpenTrackApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from(visuals.panel_fill).to_array()
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_project_load(&ctx);
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme::palette(ui).surface);
        let title = self.project_window_title();
        if self.native_window_title.as_deref() != Some(title.as_str()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.native_window_title = Some(title);
        }
        if let Some(bounds) = native_window_bounds(&ctx) {
            self.main_window_bounds = Some(bounds);
        }
        if let Some(bounds) = self.main_window_restore_bounds.take() {
            apply_viewport_bounds(&ctx, &bounds);
            self.main_window_bounds = Some(bounds);
        }
        let now = Instant::now();
        let meter_delta = self
            .last_meter_update
            .replace(now)
            .map(|previous| now.saturating_duration_since(previous).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.update_track_meter_levels(meter_delta);
        let meter_source_active = self.recording_session.is_some()
            || self.audio_input_probe.is_some()
            || self.data.transport.playing
            || (self.audio_engine.is_some() && self.project_input_monitor_requested());
        let meter_is_decaying = self
            .data
            .track_meter_levels
            .values()
            .any(|levels| levels.iter().any(|level| *level > 0.0001))
            || self
                .data
                .input_channel_meter_levels
                .iter()
                .any(|level| *level > 0.0001);
        if meter_source_active
            || meter_is_decaying
            || self
                .data
                .output_channel_meter_levels
                .iter()
                .any(|v| *v > 0.001)
            || self
                .data
                .peak_holds
                .values()
                .any(|h| h.markers.iter().any(|v| *v > 0.001))
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(20));
        }
        self.poll_audio_import_task();
        if self.audio_import_task.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        self.poll_vst3_plugin_scan(&ctx);
        let (pointer_event, pointer_released, key_pressed, pointer_down) =
            viewport_interaction(&ctx);
        if pointer_event || key_pressed {
            if !self.project_history.begin(&self.data.project) {
                self.status_message =
                    "Undo history is unavailable for this project size".to_owned();
            }
        }
        if key_pressed && !pointer_down {
            self.project_history.commit_after_frame = true;
        }
        self.handle_shortcuts(&ctx);
        self.handle_close_request(&ctx);
        self.advance_transport(&ctx);
        self.handle_midi_input_events(&ctx);
        self.release_exclusive_audio_if_idle();
        self.report_streaming_issues();
        if self
            .data
            .audio_sources
            .values()
            .any(|source| !source.waveform_ready())
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        egui::Panel::top("menu-bar")
            .frame(
                egui::Frame::new()
                    .fill(theme::palette(ui).surface_raised)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| self.show_menu(ui));
        let hide_status_bar = self.mixer_window.open;
        if self.project_open && !hide_status_bar {
            egui::Panel::bottom("status-bar")
                .exact_size(25.0)
                .show(ui, |ui| self.show_status(ui));
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::palette(ui).canvas_alternate)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| {
                let pointer_blocked = self.dialog_owns_input() || self.menu_open;
                let overlay_open = pointer_blocked || egui::Popup::is_any_open(ui.ctx());
                blocked_input_scope(ui, pointer_blocked, overlay_open, |ui| {
                    if self.project_open {
                        self.show_docked_workspace(ui);
                    }
                });
            });
        self.show_realtime_diagnostics(&ctx);
        if self.project_open {
            self.show_track_add_dialog(&ctx);
            self.show_event_colors_dialog(&ctx);
            self.show_channel_settings_window(&ctx);
            self.show_media_window(&ctx);
            self.show_workspace_organizer(&ctx);
            self.show_track_rename_dialog(&ctx);
            self.show_track_delete_prompt(&ctx);
        }
        self.show_transport_window(&ctx);
        self.show_key_commands_window(&ctx);
        self.show_unsaved_prompt(&ctx);
        self.show_project_open_error(&ctx);
        self.show_project_loading(&ctx);
        self.show_project_rate_change(&ctx);
        self.show_media_operation_error(&ctx);
        self.show_project_folder_setup(&ctx);
        self.show_about_window(&ctx);
        transport_panel::settings(self, &ctx);
        transport_panel::tempo_signature_settings(self, &ctx);
        self.preferences_ui
            .show(&ctx, &mut self.metering_preferences);
        self.channel_settings_ui.analyzer.release_ms =
            self.metering_preferences.spectrum_release_ms;
        self.show_audio_device_setup(&ctx);
        self.show_vst_connections(&ctx);
        self.show_plugin_information(&ctx);
        if self.project_open {
            self.show_vst3_windows(&ctx);
        }
        if self.project_open {
            self.show_project_setup(&ctx);
        }
        if !self.is_recording() {
            self.project_history.finish_frame(
                &self.data.project,
                pointer_released || (key_pressed && !pointer_down),
            );
        }
        if self.startup_visibility_pending {
            self.startup_visibility_pending = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            "metering-preferences-v1",
            &self.metering_preferences,
        );
        eframe::set_value(storage, "sample-editor-snap-v1", &self.sample_editor.snap);
        eframe::set_value(
            storage,
            "sample-editor-pitch-view-v1",
            &self.sample_editor.pitch_view,
        );
        eframe::set_value(storage, "midi-editor-pitch-view-v1", &self.midi_pitch_view);
        eframe::set_value(storage, "dock-state", &self.dock_state);
        eframe::set_value(
            storage,
            "transport-window-bounds-v1",
            &self.transport_window.bounds,
        );
        eframe::set_value(
            storage,
            "transport-window-always-on-top-v1",
            &self.transport_window.always_on_top,
        );
        eframe::set_value(
            storage,
            "channel-settings-preferences-v1",
            &self.channel_settings_ui.preferences,
        );
        eframe::set_value(
            storage,
            "workspace-preferences-v1",
            &self.workspace_preferences,
        );
        eframe::set_value(storage, "audio-input-device", &self.selected_input_device);
        eframe::set_value(storage, "audio-input-monitoring", &self.input_monitoring);
        eframe::set_value(storage, "audio-output-device", &self.selected_output_device);
        eframe::set_value(
            storage,
            "audio-output-sample-rate",
            &self.selected_output_sample_rate,
        );
        eframe::set_value(
            storage,
            "audio-output-buffer-frames",
            &self.selected_output_buffer_frames,
        );
        eframe::set_value(
            storage,
            "audio-device-mode-v1",
            &self.audio_device_mode.preference_value(),
        );
        eframe::set_value(
            storage,
            "release-exclusive-audio-when-idle",
            &self.release_exclusive_audio_when_idle,
        );
        eframe::set_value(
            storage,
            "midi-input-device",
            &self.selected_midi_input_device,
        );
        eframe::set_value(storage, "midi-thru-enabled", &self.midi_thru_enabled);
        eframe::set_value(storage, "metronome-enabled", &self.metronome_enabled);
        eframe::set_value(storage, "metronome-level", &self.metronome_level);
        eframe::set_value(storage, "metronome-pitch-midi", &self.metronome_pitch_midi);
        if let Some(profile) = &self.keymap_profile {
            eframe::set_value(
                storage,
                "reference-keymap-bindings-v1",
                &profile.keymap.bindings(),
            );
        }
        eframe::set_value(
            storage,
            "midi-position-quantize",
            &self.midi_position_quantize_beats,
        );
        eframe::set_value(
            storage,
            "midi-length-quantize",
            &self.midi_length_quantize_beats,
        );
        eframe::set_value(storage, "midi-insert-velocity", &self.midi_insert_velocity);
        eframe::set_value(storage, "vst3-plugin-folders-v1", &self.vst3_plugin_folders);
        eframe::set_value(storage, "vst3-plugin-catalog-v1", &self.vst3_plugin_catalog);
    }
}

fn egui_key_chords(ctx: &egui::Context) -> Vec<KeyChord> {
    ctx.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| {
                let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = event
                else {
                    return None;
                };
                let key = egui_key_code(*key)?;
                Some(KeyChord::new(
                    key,
                    KeyModifiers {
                        control: modifiers.ctrl,
                        alt: modifiers.alt,
                        shift: modifiers.shift,
                        windows: modifiers.mac_cmd,
                    },
                ))
            })
            .collect()
    })
}

fn egui_key_code(key: egui::Key) -> Option<KeyCode> {
    let name = key.name();
    if name.len() == 1 {
        let character = name.as_bytes()[0];
        if character.is_ascii_alphabetic() {
            return Some(KeyCode::Letter(char::from(character)));
        }
        if character.is_ascii_digit() {
            return Some(KeyCode::Digit(character - b'0'));
        }
    }
    if let Some(number) = name.strip_prefix('F').and_then(|value| value.parse().ok()) {
        if (1..=35).contains(&number) {
            return Some(KeyCode::Function(number));
        }
    }
    match name {
        "Left" => Some(KeyCode::Arrow(ArrowKey::Left)),
        "Right" => Some(KeyCode::Arrow(ArrowKey::Right)),
        "Up" => Some(KeyCode::Arrow(ArrowKey::Up)),
        "Down" => Some(KeyCode::Arrow(ArrowKey::Down)),
        "Enter" => Some(KeyCode::Enter),
        "Escape" => Some(KeyCode::Escape),
        "Space" => Some(KeyCode::Space),
        "Insert" => Some(KeyCode::Insert),
        "Delete" => Some(KeyCode::Delete),
        "Backspace" => Some(KeyCode::Backspace),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        "Comma" => Some(KeyCode::Comma),
        "Minus" => Some(KeyCode::Minus),
        "Period" => Some(KeyCode::Period),
        "Plus" => Some(KeyCode::Plus),
        "Slash" => Some(KeyCode::Slash),
        _ => None,
    }
}

fn default_dock_state() -> DockState<PanelTab> {
    DockState::new(vec![PanelTab::Project])
}

fn track_kind_label(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Audio => "AUDIO",
        TrackKind::Group => "GROUP",
        TrackKind::Instrument => "INSTRUMENT",
        TrackKind::Midi => "MIDI",
    }
}

fn filter_type_name(filter: FilterType) -> &'static str {
    match filter {
        FilterType::Bell | FilterType::ParametricI => "Parametric I",
        FilterType::ParametricII => "Parametric II",
        FilterType::LowShelf | FilterType::LowShelfIV => "Low Shelf IV",
        FilterType::LowShelfI => "Low Shelf I",
        FilterType::LowShelfII => "Low Shelf II",
        FilterType::LowShelfIII => "Low Shelf III",
        FilterType::HighShelf => "High Shelf",
        FilterType::LowPass => "Low Pass",
        FilterType::HighPass | FilterType::HighPassI => "High Pass I",
        FilterType::HighPassII => "High Pass II",
    }
}

/// Compact Inspector fields with shared Tabler level/balance/time icons.
fn inspector_parameter_row(
    ui: &mut egui::Ui,
    symbol: u8,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    unit: &str,
    tooltip: Option<&str>,
) -> bool {
    let palette = theme::palette(ui);
    let mut changed = false;
    let previous_interact_height = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = 18.0;
    let row = ui.horizontal(|ui| {
        ui.spacing_mut().interact_size.y = 18.0;
        ui.spacing_mut().button_padding.y = 0.0;
        ui.style_mut().explanation_tooltips = false;
        {
            let visuals = &mut ui.visuals_mut().widgets.inactive;
            visuals.weak_bg_fill = palette.canvas;
            visuals.bg_fill = palette.canvas;
            visuals.bg_stroke = Stroke::NONE;
            visuals.corner_radius = egui::CornerRadius::ZERO;
        }
        let (icon, _) = ui.allocate_exact_size(Vec2::new(45.0, 20.0), Sense::hover());
        let icon_id = match symbol {
            0 => crate::icon_system::IconId::Monitor,
            1 => crate::icon_system::IconId::Pan,
            _ => crate::icon_system::IconId::TimeAlignment,
        };
        crate::icon_system::paint_app_icon(ui, icon, icon_id, palette.text);
        let mut field = egui::DragValue::new(value)
            .range(range)
            .clamp_existing_to_range(false)
            .speed(if symbol == 1 { 1.0 } else { 0.1 })
            .fixed_decimals(2)
            .suffix(if unit.is_empty() {
                String::new()
            } else {
                String::new()
            });
        if symbol == 1 {
            field = field
                .custom_formatter(|number, _| {
                    if number.abs() < 0.5 {
                        "C".to_owned()
                    } else if number < 0.0 {
                        format!("L {:.0}", -number)
                    } else {
                        format!("R {number:.0}")
                    }
                })
                .custom_parser(|text| {
                    let text = text.trim();
                    if text.eq_ignore_ascii_case("c") {
                        Some(0.0)
                    } else if let Some(number) = text.strip_prefix(['L', 'l']) {
                        number.trim().parse::<f64>().ok().map(|v| -v.abs())
                    } else if let Some(number) = text.strip_prefix(['R', 'r']) {
                        number.trim().parse::<f64>().ok().map(f64::abs)
                    } else {
                        text.parse().ok()
                    }
                });
        }
        let response = ui.add_sized(Vec2::new(ui.available_width(), 20.0), field);
        if !response.has_focus() {
            ui.painter().rect_filled(response.rect, 0.0, palette.canvas);
            let text = if symbol == 0
                && *value <= channel_gain::MUTE_DB + 0.1
                && *value >= channel_gain::MUTE_DB - 0.1
            {
                "\u{2212}\u{221e}".to_owned()
            } else if symbol == 1 {
                if value.abs() < 0.5 {
                    "C".to_owned()
                } else if *value < 0.0 {
                    format!("L {:.0}", -*value)
                } else {
                    format!("R {:.0}", *value)
                }
            } else {
                format!("{:.2}", *value)
            };
            ui.painter().text(
                if symbol == 1 {
                    response.rect.center()
                } else {
                    response.rect.right_center() - Vec2::new(4.0, 0.0)
                },
                if symbol == 1 {
                    Align2::CENTER_CENTER
                } else {
                    Align2::RIGHT_CENTER
                },
                text,
                FontId::proportional(11.0),
                palette.text,
            );
        }
        let bar = response.rect.shrink2(Vec2::new(3.0, 1.0));
        let y = bar.bottom() - 1.0;
        ui.painter().line_segment(
            [Pos2::new(bar.left(), y), Pos2::new(bar.right(), y)],
            Stroke::new(1.0, palette.grid),
        );
        let fraction = match symbol {
            0 => ((*value - channel_gain::MUTE_DB)
                / (channel_gain::MAX_DB - channel_gain::MUTE_DB))
                .clamp(0.0, 1.0),
            1 => ((*value + 100.0) / 200.0).clamp(0.0, 1.0),
            _ => ((*value + 2000.0) / 4000.0).clamp(0.0, 1.0),
        };
        let x = egui::lerp(bar.x_range(), fraction);
        if symbol == 0 {
            ui.painter().line_segment(
                [Pos2::new(bar.left(), y), Pos2::new(x, y)],
                Stroke::new(1.0, palette.accent),
            );
        }
        ui.painter().line_segment(
            [Pos2::new(x, y - 2.0), Pos2::new(x, y)],
            Stroke::new(2.0, palette.accent),
        );
        changed = response.changed();
    });
    ui.spacing_mut().interact_size.y = previous_interact_height;
    if let Some(text) = tooltip {
        row.response.on_hover_text(text);
    }
    changed
}

fn blend_color(from: Color32, to: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| {
        (f32::from(a) + (f32::from(b) - f32::from(a)) * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
    )
}

fn project_track_header_fill(palette: theme::Palette, selected: bool, dark_mode: bool) -> Color32 {
    if selected {
        Color32::from_gray(if dark_mode { 102 } else { 224 })
    } else {
        palette.surface_raised
    }
}

fn show_project_info_line(ui: &mut egui::Ui) {
    let palette = theme::palette(ui);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 29.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.canvas);
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, palette.grid),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        "No Object Selected",
        FontId::proportional(10.5),
        palette.text,
    );
}

fn show_project_inspector(
    ui: &mut egui::Ui,
    project: &mut Project,
    settings: &mut NativeWindowState,
) -> project_inspector::InspectorChanges {
    project_inspector::show(ui, project, settings)
}

struct ProjectViewChanges {
    modified: bool,
    live_mix: Option<(u64, f32, f32)>,
    defer_audio_plan: bool,
}

const PANEL_HEADER_HEIGHT: f32 = 29.0;
const PANEL_RIGHT_GUTTER: f32 = 0.0;
const PROJECT_PANEL_HEIGHT: f32 = 535.0;
const MIXER_PANEL_HEIGHT: f32 = 449.0;

fn docked_bounds(rect: Rect) -> WindowBounds {
    WindowBounds {
        left: f64::from(rect.left()),
        top: f64::from(rect.top()),
        width: f64::from(rect.width()),
        height: f64::from(rect.height()),
        inner_width: None,
        inner_height: None,
        monitor_id: None,
    }
}

#[cfg(windows)]
fn available_disk_space_bytes(directory: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let path = directory
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut available = 0_u64;
    // SAFETY: `path` is NUL-terminated UTF-16 and all output pointers remain valid for the call.
    let result = unsafe {
        GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (result != 0).then_some(available)
}

#[cfg(not(windows))]
fn available_disk_space_bytes(_directory: &Path) -> Option<u64> {
    None
}

fn docked_panel_header(ui: &mut egui::Ui, title: &str, project: bool) -> bool {
    let palette = theme::palette(ui);
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), PANEL_HEADER_HEIGHT),
        Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, palette.surface_raised);
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0, palette.text.gamma_multiply(0.10)),
    );
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, palette.grid),
    );
    let title_left = if project {
        rect.left() + 25.0
    } else {
        rect.left() + 9.0
    };
    if project {
        let center = Pos2::new(rect.left() + 14.0, rect.center().y);
        painter.add(egui::Shape::convex_polygon(
            vec![
                center + Vec2::new(-4.0, -4.5),
                center + Vec2::new(4.0, 0.0),
                center + Vec2::new(-4.0, 4.5),
            ],
            palette.accent,
            Stroke::NONE,
        ));
    }
    painter
        .with_clip_rect(Rect::from_min_max(
            Pos2::new(title_left, rect.top()),
            Pos2::new(rect.right() - 34.0, rect.bottom()),
        ))
        .text(
            Pos2::new(title_left, rect.center().y),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional(11.5),
            palette.text,
        );

    let button_size = Vec2::new(20.0, 13.0);
    let button_y = rect.center().y - button_size.y * 0.5;
    let close_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 5.0 - button_size.x, button_y),
        button_size,
    );
    panel_close_button(ui, close_rect, palette)
        .on_hover_text("Close")
        .clicked()
}

fn panel_close_button(ui: &mut egui::Ui, rect: Rect, palette: theme::Palette) -> egui::Response {
    let response = ui.interact(
        rect,
        Id::new(("panel-close-button", rect.min.x as i32, rect.min.y as i32)),
        Sense::click(),
    );
    let close_hover = response.hovered();
    let fill = if close_hover {
        theme::RECORD
    } else {
        Color32::TRANSPARENT
    };
    theme::paint_control(ui, rect, &response, fill, ui.is_enabled(), false);
    let ink = if close_hover {
        Color32::WHITE
    } else {
        palette.text
    };
    crate::icon_system::paint_app_icon(ui, rect, crate::icon_system::IconId::Close, ink);
    response
}

fn track_hidden_by_collapsed_group(project: &Project, track_id: u64) -> bool {
    let mut parent_id = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .and_then(|track| track.parent_group_id);
    for _ in 0..2 {
        let Some(id) = parent_id else { return false };
        let Some(parent) = project.tracks.iter().find(|track| track.id == id) else {
            return false;
        };
        if parent.group_collapsed {
            return true;
        }
        parent_id = parent.parent_group_id;
    }
    false
}

fn project_track_group_depth(project: &Project, track_id: u64) -> usize {
    let mut depth = 0;
    let mut parent_id = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .and_then(|track| track.parent_group_id);
    for _ in 0..2 {
        let Some(id) = parent_id else { break };
        let Some(parent) = project.tracks.iter().find(|track| track.id == id) else {
            break;
        };
        depth += 1;
        parent_id = parent.parent_group_id;
    }
    depth
}

fn project_track_inherited_group_state(project: &Project, track_id: u64) -> (bool, bool, bool) {
    let mut inherited = (false, false, false);
    let Some(track) = project.tracks.iter().find(|track| track.id == track_id) else {
        return inherited;
    };
    let mut parent_id = track.parent_group_id;
    for _ in 0..2 {
        let Some(id) = parent_id else { break };
        let Some(parent) = project.tracks.iter().find(|track| track.id == id) else {
            break;
        };
        inherited.0 |= parent.muted;
        inherited.1 |= parent.solo;
        inherited.2 |= parent.solo_defeat;
        parent_id = parent.parent_group_id;
    }
    if track.kind == TrackKind::Group {
        for child in project
            .tracks
            .iter()
            .filter(|candidate| project_track_is_in_group(project, candidate.id, track_id))
        {
            inherited.1 |= child.solo;
            inherited.2 |= child.solo_defeat;
        }
    }
    inherited
}

fn project_track_is_in_group(project: &Project, track_id: u64, group_id: u64) -> bool {
    let mut parent_id = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .and_then(|track| track.parent_group_id);
    for _ in 0..2 {
        let Some(id) = parent_id else { return false };
        if id == group_id {
            return true;
        }
        parent_id = project
            .tracks
            .iter()
            .find(|track| track.id == id)
            .and_then(|track| track.parent_group_id);
    }
    false
}

/// Builds normalized group-only display peaks by combining the child clip
/// envelopes on the project timeline. This remains a visual summary; inserts
/// and group-bus processing are not part of the current audio routing model.
fn group_waveform_columns(
    project: &Project,
    group_id: u64,
    sources: &HashMap<PathBuf, AudioSource>,
    start_seconds: f64,
    end_seconds: f64,
    count: usize,
    output_channels: usize,
) -> Vec<[[f32; 2]; 2]> {
    let mut columns = vec![[[0.0; 2]; 2]; count];
    if count == 0 || end_seconds <= start_seconds || !project.tempo_bpm.is_finite() {
        return columns;
    }

    let duration = end_seconds - start_seconds;
    let column_seconds = duration / count as f64;
    let seconds_per_beat = 60.0 / project.tempo_bpm.max(1.0);
    let output_channels = output_channels.clamp(1, 2);
    for track in project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio)
        .filter(|track| project_track_is_in_group(project, track.id, group_id))
    {
        // Editing previews remain visible regardless of playback mute/solo,
        // just like the individual child event waveforms.
        for clip in &track.clips {
            if !clip.start_beats.is_finite()
                || !clip.duration_seconds.is_finite()
                || clip.duration_seconds <= 0.0
            {
                continue;
            }
            let Some(source) = sources
                .get(sample_editor::playback_path(clip))
                .or_else(|| sources.get(&clip.source_path))
            else {
                continue;
            };
            let clip_start = clip.start_beats * seconds_per_beat;
            let clip_end = clip_start + clip.duration_seconds;
            let visible_start = start_seconds.max(clip_start);
            let visible_end = end_seconds.min(clip_end);
            if visible_end <= visible_start {
                continue;
            }
            let first = (((visible_start - start_seconds) / duration * count as f64).floor()
                as usize)
                .min(count);
            let last = (((visible_end - start_seconds) / duration * count as f64).ceil() as usize)
                .clamp(first, count);
            if first == last {
                continue;
            }
            let first_time = start_seconds + first as f64 * column_seconds;
            let last_time = start_seconds + last as f64 * column_seconds;
            let source_start = (clip.source_offset_seconds + first_time - clip_start)
                .clamp(0.0, source.duration_seconds());
            let source_end = (clip.source_offset_seconds + last_time - clip_start)
                .clamp(source_start, source.duration_seconds());
            let source_channels = source.waveform().channels.max(1) as usize;
            let count = last - first;
            let left = source.waveform_columns(source_start, source_end, count, 0);
            let right = if source_channels > 1 {
                source.waveform_columns(source_start, source_end, count, 1)
            } else {
                left.clone()
            };
            let mono_pan = (track.pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
            let mono_pan_gain = [mono_pan.cos(), mono_pan.sin()];

            for (offset, (left_range, right_range)) in left.iter().zip(&right).enumerate() {
                let index = first + offset;
                let local_seconds = ((start_seconds + (index as f64 + 0.5) * column_seconds)
                    - clip_start)
                    .clamp(0.0, clip.duration_seconds);
                let envelope = clip.gain
                    * track.gain
                    * opentrack_audio::event_envelope(
                        local_seconds,
                        clip.duration_seconds,
                        clip.fade_in_seconds,
                        clip.fade_out_seconds,
                    )
                    * clip
                        .source_fades
                        .iter()
                        .map(|fade| fade.gain_at(local_seconds + clip.source_offset_seconds))
                        .product::<f32>();
                let mut input = [*left_range, *right_range];
                if output_channels == 1 && source_channels > 1 {
                    input[0] = [
                        (input[0][0] + input[1][0]) * 0.5,
                        (input[0][1] + input[1][1]) * 0.5,
                    ];
                }
                for output_channel in 0..output_channels {
                    let channel_gain = if source_channels == 1 && output_channels == 2 {
                        mono_pan_gain[output_channel]
                    } else if source_channels > 1 && output_channels == 2 {
                        if output_channel == 0 && track.pan > 0.0 {
                            1.0 - track.pan
                        } else if output_channel == 1 && track.pan < 0.0 {
                            1.0 + track.pan
                        } else {
                            1.0
                        }
                    } else {
                        1.0
                    };
                    let gain = envelope * channel_gain;
                    let range = if output_channels == 1 {
                        input[0]
                    } else {
                        input[output_channel.min(source_channels - 1)]
                    };
                    let scaled = if gain >= 0.0 {
                        [range[0] * gain, range[1] * gain]
                    } else {
                        [range[1] * gain, range[0] * gain]
                    };
                    columns[index][output_channel][0] += scaled[0];
                    columns[index][output_channel][1] += scaled[1];
                }
            }
        }
    }

    let peak = columns
        .iter()
        .flat_map(|column| column.iter())
        .flat_map(|range| range.iter())
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max);
    if peak > 1e-5 {
        let gain = 0.88 / peak;
        for column in &mut columns {
            for range in column {
                range[0] = (range[0] * gain).clamp(-1.0, 1.0);
                range[1] = (range[1] * gain).clamp(-1.0, 1.0);
            }
        }
    }
    columns
}

fn show_project(
    ui: &mut egui::Ui,
    edit_tool: &mut EditTool,
    project: &mut Project,
    position_beats: &mut f64,
    cycle_enabled: bool,
    beats_per_bar: u8,
    sample_rate: u32,
    meter_levels: &HashMap<u64, [f32; 2]>,
    peak_holds: &HashMap<u64, metering::Hold>,
    audio_sources: &HashMap<PathBuf, AudioSource>,
    recording_preview: Option<&RecordingPreview>,
    arrange_view_state: &mut ArrangeViewState,
    midi_editor_open: &mut bool,
    midi_part_draw: &mut Option<MidiPartDraw>,
    channel_settings_window: &mut NativeWindowState,
    _track_rename_dialog: &mut Option<TrackRenameDialog>,
    can_remove: bool,
    context_action: &mut Option<project_controls::Action>,
) -> ProjectViewChanges {
    let mut modified = false;
    let mut inspector_changes = project_inspector::InspectorChanges::default();
    let mut pending_midi_part_edit = None;
    let mut pending_midi_part_remove = None;
    let mut pending_midi_part_split = None;
    if *edit_tool != EditTool::Draw {
        *midi_part_draw = None;
    }
    if arrange_view_state.selected_track_ids.is_empty()
        && let Some(track_id) = project
            .selected_track
            .and_then(|index| project.tracks.get(index as usize))
            .map(|track| track.id)
    {
        arrange_view_state.selected_track_ids.insert(track_id);
        arrange_view_state.selection_anchor_track_id = Some(track_id);
    }
    if arrange_view_state.show_inspector {
        egui::Panel::left("project-inspector")
            .exact_size(159.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::palette(ui).surface_raised)
                    .stroke(Stroke::new(1.0, theme::palette(ui).grid))
                    .inner_margin(egui::Margin::same(3)),
            )
            .show(ui, |ui| {
                inspector_changes = show_project_inspector(ui, project, channel_settings_window);
            });
    }

    let lane_header_width = 264.0;
    let ruler_height = 22.0;
    crate::view_controls::apply_solid_scrollbars(ui);
    let scroll_parent = ui.available_rect_before_wrap();
    let timeline_view_width = (scroll_parent.width() - lane_header_width - 34.0).max(1.0);
    let bar_width = (timeline_view_width / 49.0
        * arrange_view_state
            .timeline_zoom
            .clamp(crate::timeline::MIN_ZOOM, crate::timeline::MAX_ZOOM))
    .max(4.0);
    let track_ids = project
        .tracks
        .iter()
        .map(|track| track.id)
        .collect::<Vec<_>>();
    let group_track_ids = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Group)
        .map(|track| track.id)
        .collect::<Vec<_>>();
    let mut custom_scroll_offset = None;
    let wheel_hover = ui.input(|i| i.pointer.hover_pos());
    if let Some(mut action) = crate::view_controls::wheel(ui, scroll_parent) {
        use crate::view_controls::WheelAction;
        if matches!(action, WheelAction::TimeZoom(_))
            && wheel_hover.is_some_and(|pos| pos.x < scroll_parent.left() + lane_header_width)
        {
            action = match action {
                WheelAction::TimeZoom(amount) => WheelAction::VerticalPan(amount),
                other => other,
            };
        }
        match action {
            WheelAction::VerticalZoom(amount) => {
                let previous = arrange_view_state.track_vertical_zoom;
                arrange_view_state.track_vertical_zoom =
                    (previous * (amount * 0.003).exp()).clamp(0.5, 4.0);
                let anchor = ui
                    .input(|i| i.pointer.hover_pos())
                    .map_or(0.0, |p| (p.y - scroll_parent.top() - ruler_height).max(0.0));
                arrange_view_state.track_scroll_y = ((arrange_view_state.track_scroll_y + anchor)
                    * arrange_view_state.track_vertical_zoom
                    / previous
                    - anchor)
                    .max(0.0);
            }
            WheelAction::HorizontalPan(amount) => {
                arrange_view_state.timeline_scroll_x =
                    (arrange_view_state.timeline_scroll_x - amount).max(0.0);
            }
            WheelAction::VerticalPan(amount) => {
                arrange_view_state.track_scroll_y =
                    (arrange_view_state.track_scroll_y - amount).max(0.0);
            }
            WheelAction::TimeZoom(amount) => {
                let previous = bar_width;
                arrange_view_state.timeline_zoom = (arrange_view_state.timeline_zoom
                    * (amount * 0.004).exp())
                .clamp(crate::timeline::MIN_ZOOM, crate::timeline::MAX_ZOOM);
                let next = (timeline_view_width / 49.0 * arrange_view_state.timeline_zoom).max(4.0);
                let anchor = ui.input(|i| i.pointer.hover_pos()).map_or(0.0, |p| {
                    (p.x - scroll_parent.left() - lane_header_width).max(0.0)
                });
                arrange_view_state.timeline_scroll_x =
                    ((arrange_view_state.timeline_scroll_x + anchor) * next / previous - anchor)
                        .max(0.0);
            }
        }
        custom_scroll_offset = Some(Vec2::new(
            arrange_view_state.timeline_scroll_x,
            arrange_view_state.track_scroll_y,
        ));
    }
    let bar_width = (timeline_view_width / 49.0 * arrange_view_state.timeline_zoom).max(4.0);
    arrange_view_state
        .selected_track_ids
        .retain(|track_id| track_ids.contains(track_id));
    if let Some(track_id) = project
        .selected_track
        .and_then(|index| track_ids.get(index as usize))
        .copied()
        && !arrange_view_state.selected_track_ids.contains(&track_id)
    {
        arrange_view_state.selected_track_ids.clear();
        arrange_view_state.selected_track_ids.insert(track_id);
        arrange_view_state.selection_anchor_track_id = Some(track_id);
    }
    if arrange_view_state
        .selection_anchor_track_id
        .is_some_and(|anchor| !track_ids.contains(&anchor))
    {
        arrange_view_state.selection_anchor_track_id = project
            .selected_track
            .and_then(|index| track_ids.get(index as usize))
            .copied();
    }
    let beats_per_bar_f64 = f64::from(beats_per_bar);
    let snap_grid_beats = arrange_view_state.snap_grid_beats(beats_per_bar);
    let furthest_clip_beat = project
        .tracks
        .iter()
        .flat_map(|track| {
            track
                .clips
                .iter()
                .map(|clip| clip.start_beats + clip.duration_seconds * project.tempo_bpm / 60.0)
                .chain(
                    track
                        .midi_parts
                        .iter()
                        .map(|part| part.start_beats + part.duration_beats),
                )
        })
        .fold(32.0 * beats_per_bar_f64, f64::max);
    const MINIMUM_TIMELINE_BARS: f64 = 256.0;
    const TIMELINE_RUNWAY_BARS: f64 = 256.0;
    const MAXIMUM_TIMELINE_BARS: f64 = 1_000_000.0;
    let furthest_clip_bar = (furthest_clip_beat / beats_per_bar_f64).ceil() + 4.0;
    let playhead_bar = (*position_beats / beats_per_bar_f64).max(0.0);
    let content_bars = furthest_clip_bar
        .max(MINIMUM_TIMELINE_BARS)
        .max(playhead_bar.ceil() + TIMELINE_RUNWAY_BARS)
        .max(project.loop_end_beats / beats_per_bar_f64 + TIMELINE_RUNWAY_BARS)
        .max(
            (f64::from(arrange_view_state.timeline_scroll_x + timeline_view_width)
                / f64::from(bar_width))
            .ceil()
                + TIMELINE_RUNWAY_BARS,
        )
        .min(MAXIMUM_TIMELINE_BARS) as usize;
    let visible_rows = project.tracks.len();
    let row_heights = (0..visible_rows)
        .map(|row| {
            if track_hidden_by_collapsed_group(project, track_ids[row]) {
                return 0.0;
            }
            let take_lane_count = project
                .tracks
                .get(row)
                .map(|track| audio_events::take_lane_layout(track, project.tempo_bpm).0)
                .unwrap_or(1);
            track_ids
                .get(row)
                .map(|track_id| {
                    arrange_view_state.track_height(*track_id)
                        * if arrange_view_state.take_lanes_visible.contains(track_id) {
                            take_lane_count as f32
                        } else {
                            1.0
                        }
                        + if arrange_view_state.automation_visible.contains(track_id) {
                            42.0
                        } else {
                            0.0
                        }
                })
                .unwrap_or(42.0)
        })
        .collect::<Vec<_>>();
    let mut row_offsets = Vec::with_capacity(visible_rows);
    let mut tracks_height = 0.0;
    for height in &row_heights {
        row_offsets.push(tracks_height);
        tracks_height += height;
    }
    let total_size = Vec2::new(
        lane_header_width + bar_width * content_bars as f32,
        ui.available_height().max(ruler_height + tracks_height),
    );
    let mut exclusive_solo_track_id = None;
    let mut reorder_after_draw: Option<(Vec<u64>, usize)> = None;
    let mut reorder_indicator = None;
    let mut group_drop_after_draw: Option<(u64, Vec<u64>)> = None;
    let mut group_drop_indicator = None;
    let mut clear_all_muted = false;
    let mut clear_all_solo = false;
    // Stack the vertical track scrollbar and track-height zoom in one lane.
    let track_scrollbar_width = 20.0;
    let scroll_content_right = scroll_parent.right() - track_scrollbar_width;
    let track_zoom_height = 94.0;
    let horizontal_scroll_right = scroll_parent.right() - 134.0;
    let horizontal_scroll_left =
        (scroll_parent.left() + lane_header_width).min(horizontal_scroll_right - 1.0);
    let scroll_bar_rect = Rect::from_min_max(
        Pos2::new(horizontal_scroll_left, scroll_parent.top() + ruler_height),
        Pos2::new(
            horizontal_scroll_right,
            scroll_parent.bottom() - track_zoom_height - 16.0,
        ),
    );
    let scroll_output = egui::ScrollArea::both()
        .id_salt("project-arrange-scroll")
        .scroll_source(egui::scroll_area::ScrollSource {
            mouse_wheel: false,
            ..Default::default()
        })
        .scroll_offset(Vec2::new(
            arrange_view_state.timeline_scroll_x.max(0.0),
            arrange_view_state.track_scroll_y.max(0.0),
        ))
        .scroll_bar_rect(scroll_bar_rect)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (canvas, _) = ui.allocate_exact_size(total_size, Sense::hover());
            let painter = ui.painter_at(canvas);
            let palette = theme::palette(ui);
            let viewport = ui.clip_rect();
            let header_left = viewport.left();
            let grid_left = canvas.min.x + lane_header_width;
            let ruler =
                Rect::from_min_size(viewport.min, Vec2::new(viewport.width(), ruler_height));
            let track_clip = Rect::from_min_max(
                Pos2::new(viewport.left(), ruler.bottom()),
                viewport.right_bottom(),
            );
            let grid_clip = Rect::from_min_max(
                Pos2::new(viewport.left() + lane_header_width, ruler.bottom()),
                viewport.right_bottom(),
            );
            painter.rect_filled(canvas, egui::CornerRadius::ZERO, palette.canvas);

            painter.rect_filled(ruler, egui::CornerRadius::ZERO, palette.surface_raised);
            painter.line_segment(
                [ruler.left_bottom(), ruler.right_bottom()],
                Stroke::new(1.0, palette.grid),
            );
            painter.line_segment(
                [ruler.left_top(), ruler.right_top()],
                Stroke::new(1.0, palette.text.gamma_multiply(0.08)),
            );
            let ruler_response_rect = Rect::from_min_max(
                Pos2::new(grid_clip.left(), ruler.top()),
                Pos2::new(grid_clip.right(), ruler.bottom()),
            );
            // Keep the range strip separate from normal seek/zoom gestures,
            // while retaining one ruler response for its right-click menu.
            let range_strip = Rect::from_min_max(
                ruler_response_rect.min,
                Pos2::new(ruler_response_rect.right(), ruler.top() + 7.0),
            );
            let seek_strip = Rect::from_min_max(range_strip.left_bottom(), ruler_response_rect.max);
            let ruler_response = ui.interact(
                ruler_response_rect,
                Id::new("project-time-ruler"),
                Sense::click_and_drag(),
            );
            let ruler_gesture_id = Id::new("project-ruler-gesture");
            let (primary_down, primary_released, press_origin, pointer) = ui.input(|input| {
                (
                    input.pointer.button_down(egui::PointerButton::Primary),
                    input.pointer.button_released(egui::PointerButton::Primary),
                    input.pointer.press_origin(),
                    input.pointer.interact_pos(),
                )
            });
            let range_gesture_id = Id::new("project-ruler-range-gesture");
            let existing_left_x =
                grid_left + (project.loop_start_beats / beats_per_bar_f64) as f32 * bar_width;
            let existing_right_x =
                grid_left + (project.loop_end_beats / beats_per_bar_f64) as f32 * bar_width;
            let hovered_edge = pointer.and_then(|pointer| {
                crate::timeline::range_edge_at_x(pointer.x, existing_left_x, existing_right_x)
            });
            let over_existing_range = pointer.is_some_and(|pointer| {
                let beat = crate::timeline::ruler_position_beats(
                    pointer.x,
                    grid_left,
                    bar_width,
                    beats_per_bar,
                );
                beat >= project.loop_start_beats && beat <= project.loop_end_beats
            });
            if primary_down
                && press_origin.is_some_and(|origin| range_strip.contains(origin))
                && ruler_response.is_pointer_button_down_on()
                && ui
                    .data(|data| data.get_temp::<RulerRangeGestureState>(range_gesture_id))
                    .is_none()
                && let Some(origin) = press_origin
            {
                let pointer_start = crate::timeline::ruler_position_beats(
                    origin.x,
                    grid_left,
                    bar_width,
                    beats_per_bar,
                );
                let edge =
                    crate::timeline::range_edge_at_x(origin.x, existing_left_x, existing_right_x);
                let moving = edge.is_none()
                    && pointer_start >= project.loop_start_beats
                    && pointer_start <= project.loop_end_beats;
                let (mut anchor_beats, endpoint_beats) = if project.cycle_mode == CycleMode::Skip {
                    (project.loop_end_beats, project.loop_start_beats)
                } else {
                    (project.loop_start_beats, project.loop_end_beats)
                };
                let kind = if let Some(left) = edge {
                    RulerRangeDragKind::Resize {
                        left,
                        anchor: anchor_beats,
                        endpoint: endpoint_beats,
                        pointer_start,
                    }
                } else if moving {
                    RulerRangeDragKind::Move {
                        anchor: anchor_beats,
                        endpoint: endpoint_beats,
                        pointer_start,
                    }
                } else {
                    anchor_beats = pointer_start;
                    RulerRangeDragKind::Draw
                };
                if matches!(kind, RulerRangeDragKind::Draw)
                    && let Some(grid) = snap_grid_beats
                {
                    anchor_beats = crate::timeline::snap_beats_to_grid(anchor_beats, grid);
                }
                ui.data_mut(|data| {
                    data.insert_temp(
                        range_gesture_id,
                        RulerRangeGestureState {
                            origin,
                            anchor_beats,
                            endpoint_beats: if matches!(kind, RulerRangeDragKind::Draw) {
                                anchor_beats
                            } else {
                                endpoint_beats
                            },
                            cancelled: false,
                            kind,
                        },
                    )
                });
            }
            let mut range_preview = None;
            let mut moving_range = false;
            let mut resizing_range = false;
            if let Some(mut gesture) =
                ui.data(|data| data.get_temp::<RulerRangeGestureState>(range_gesture_id))
            {
                moving_range = matches!(gesture.kind, RulerRangeDragKind::Move { .. });
                resizing_range = matches!(gesture.kind, RulerRangeDragKind::Resize { .. });
                gesture.cancelled |= ui.input(|input| input.key_pressed(egui::Key::Escape));
                if !gesture.cancelled
                    && let Some(pointer) = pointer
                {
                    if primary_down {
                        let current = arrange_view_state.timeline_scroll_x;
                        let next = crate::timeline::edge_scroll_offset(
                            pointer.x,
                            grid_clip.left(),
                            grid_clip.right().min(scroll_content_right),
                            current,
                            (total_size.x - viewport.width()).max(0.0),
                            ui.input(|input| input.stable_dt),
                        );
                        if (next - current).abs() > f32::EPSILON {
                            arrange_view_state.timeline_scroll_x = next;
                            custom_scroll_offset =
                                Some(Vec2::new(next, arrange_view_state.track_scroll_y));
                            ui.ctx()
                                .request_repaint_after(std::time::Duration::from_millis(16));
                        }
                    }
                    let grid_origin =
                        viewport.left() + lane_header_width - arrange_view_state.timeline_scroll_x;
                    let pointer_beats = crate::timeline::ruler_position_beats(
                        pointer.x,
                        grid_origin,
                        bar_width,
                        beats_per_bar,
                    )
                    .min(1_000_000.0);
                    match gesture.kind {
                        RulerRangeDragKind::Resize {
                            left,
                            anchor,
                            endpoint,
                            pointer_start,
                        } => {
                            let delta = pointer_beats - pointer_start;
                            (gesture.anchor_beats, gesture.endpoint_beats) =
                                if delta.abs() <= f64::EPSILON {
                                    (anchor, endpoint)
                                } else {
                                    crate::timeline::resize_range(
                                        anchor,
                                        endpoint,
                                        left,
                                        if left {
                                            anchor.min(endpoint)
                                        } else {
                                            anchor.max(endpoint)
                                        } + delta,
                                        snap_grid_beats,
                                    )
                                };
                        }
                        RulerRangeDragKind::Move {
                            anchor,
                            endpoint,
                            pointer_start,
                        } => {
                            (gesture.anchor_beats, gesture.endpoint_beats) =
                                crate::timeline::translate_range(
                                    anchor,
                                    endpoint,
                                    pointer_beats - pointer_start,
                                    snap_grid_beats,
                                );
                        }
                        RulerRangeDragKind::Draw => {
                            gesture.endpoint_beats = if let Some(grid) = snap_grid_beats {
                                crate::timeline::snap_beats_to_grid(pointer_beats, grid)
                            } else {
                                pointer_beats
                            };
                        }
                    }
                    range_preview = Some((gesture.anchor_beats, gesture.endpoint_beats));
                    if primary_released && (pointer.x - gesture.origin.x).abs() >= 3.0 {
                        let before = (
                            project.loop_start_beats,
                            project.loop_end_beats,
                            project.cycle_mode,
                        );
                        if project.set_cycle_range(gesture.anchor_beats, gesture.endpoint_beats) {
                            modified |= before
                                != (
                                    project.loop_start_beats,
                                    project.loop_end_beats,
                                    project.cycle_mode,
                                );
                        }
                    }
                }
                if primary_released || !primary_down {
                    ui.data_mut(|data| data.remove::<RulerRangeGestureState>(range_gesture_id));
                } else {
                    ui.data_mut(|data| data.insert_temp(range_gesture_id, gesture));
                }
            }
            if (pointer.is_some_and(|pointer| range_strip.contains(pointer))
                && ruler_response.hovered()
                || range_preview.is_some())
                && !ui.input(|input| input.pointer.button_down(egui::PointerButton::Middle))
            {
                if resizing_range || (range_preview.is_none() && hovered_edge.is_some()) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                } else if moving_range || (range_preview.is_none() && over_existing_range) {
                    crate::timeline::show_move_cursor(ui, moving_range && primary_down);
                } else {
                    crate::system_tool_icons::tool_cursor(ui, ToolbarIcon::Draw);
                }
            }

            let (range_anchor, range_endpoint) = range_preview.unwrap_or_else(|| {
                if project.cycle_mode == CycleMode::Skip {
                    (project.loop_end_beats, project.loop_start_beats)
                } else {
                    (project.loop_start_beats, project.loop_end_beats)
                }
            });
            let range_color = crate::timeline::cycle_color(if range_endpoint < range_anchor {
                CycleMode::Skip
            } else {
                CycleMode::Loop
            });
            let range_grid_origin =
                viewport.left() + lane_header_width - arrange_view_state.timeline_scroll_x;
            let range_start_x = range_grid_origin
                + (range_anchor.min(range_endpoint) / beats_per_bar_f64) as f32 * bar_width;
            let range_end_x = range_grid_origin
                + (range_anchor.max(range_endpoint) / beats_per_bar_f64) as f32 * bar_width;
            let range_painter = painter.with_clip_rect(ruler_response_rect);
            let range_rect = Rect::from_min_max(
                Pos2::new(range_start_x, ruler.top()),
                Pos2::new(range_end_x, ruler.bottom()),
            );
            range_painter.rect_filled(
                range_rect,
                0.0,
                range_color.gamma_multiply(if cycle_enabled || range_preview.is_some() {
                    0.48
                } else {
                    0.22
                }),
            );
            range_painter.rect_filled(
                Rect::from_min_max(range_rect.min, Pos2::new(range_end_x, range_strip.bottom())),
                0.0,
                range_color.gamma_multiply(if cycle_enabled || range_preview.is_some() {
                    1.0
                } else {
                    0.55
                }),
            );
            for x in [range_start_x, range_end_x] {
                range_painter.line_segment(
                    [Pos2::new(x, ruler.top()), Pos2::new(x, ruler.bottom())],
                    Stroke::new(1.0, range_color),
                );
            }
            if primary_down
                && press_origin.is_some_and(|origin| seek_strip.contains(origin))
                && ruler_response.is_pointer_button_down_on()
                && ui
                    .data(|data| data.get_temp::<RulerGestureState>(ruler_gesture_id))
                    .is_none()
                && let Some(origin) = press_origin
            {
                ui.data_mut(|data| {
                    data.insert_temp(
                        ruler_gesture_id,
                        RulerGestureState {
                            origin,
                            start_zoom: arrange_view_state.timeline_zoom,
                            start_bar_width: bar_width,
                            start_scroll_x: arrange_view_state.timeline_scroll_x,
                            edge_scroll_x: 0.0,
                            active: false,
                        },
                    )
                });
            }
            if (primary_down || primary_released)
                && let Some(mut gesture) =
                    ui.data(|data| data.get_temp::<RulerGestureState>(ruler_gesture_id))
                && let Some(pointer) = pointer
            {
                let delta = pointer - gesture.origin;
                gesture.active |= delta.length_sq() >= 9.0;
                if gesture.active {
                    // Both dimensions operate throughout the same capture. Zoom
                    // holds the original musical anchor; x always seeks through
                    // the resulting transform, including accumulated edge scroll.
                    let next_zoom = (gesture.start_zoom * 3.5_f32.powf(delta.y / 50.0))
                        .clamp(crate::timeline::MIN_ZOOM, crate::timeline::MAX_ZOOM);
                    let effective_bar_width =
                        gesture.start_bar_width * next_zoom / gesture.start_zoom.max(0.001);
                    let viewport_grid_left = viewport.left() + lane_header_width;
                    let anchor_bar = (gesture.origin.x - viewport_grid_left
                        + gesture.start_scroll_x)
                        / gesture.start_bar_width.max(1.0);
                    let zoom_scroll =
                        anchor_bar * effective_bar_width - (gesture.origin.x - viewport_grid_left);
                    let max_scroll = (lane_header_width
                        + effective_bar_width * content_bars as f32
                        - viewport.width())
                    .max(0.0);
                    let current_scroll =
                        (zoom_scroll + gesture.edge_scroll_x).clamp(0.0, max_scroll);
                    let next_scroll = if primary_down {
                        crate::timeline::edge_scroll_offset(
                            pointer.x,
                            grid_clip.left(),
                            grid_clip.right().min(scroll_content_right),
                            current_scroll,
                            max_scroll,
                            ui.input(|input| input.stable_dt),
                        )
                    } else {
                        current_scroll
                    };
                    // Only actual edge motion accumulates. A zoom clamped at
                    // project start must not turn into a persistent pan offset.
                    gesture.edge_scroll_x += next_scroll - current_scroll;
                    if (next_scroll - arrange_view_state.timeline_scroll_x).abs() > f32::EPSILON
                        || (next_zoom - arrange_view_state.timeline_zoom).abs() > f32::EPSILON
                    {
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(16));
                    }
                    arrange_view_state.timeline_zoom = next_zoom;
                    arrange_view_state.timeline_scroll_x = next_scroll;
                    custom_scroll_offset =
                        Some(Vec2::new(next_scroll, arrange_view_state.track_scroll_y));
                    *position_beats = crate::timeline::ruler_position_beats(
                        pointer.x,
                        viewport_grid_left - next_scroll,
                        effective_bar_width,
                        beats_per_bar,
                    );
                }
                ui.data_mut(|data| data.insert_temp(ruler_gesture_id, gesture));
            }
            let seek_drag_released = primary_released
                && ui
                    .data(|data| data.get_temp::<RulerGestureState>(ruler_gesture_id))
                    .is_some_and(|gesture| gesture.active);
            if primary_released || !primary_down {
                ui.data_mut(|data| data.remove::<RulerGestureState>(ruler_gesture_id));
            }
            let mut sought = seek_drag_released;
            if ruler_response.clicked_by(egui::PointerButton::Primary)
                && !seek_drag_released
                && let Some(pointer) = ruler_response.interact_pointer_pos()
                && seek_strip.contains(pointer)
            {
                *position_beats = crate::timeline::ruler_position_beats(
                    pointer.x,
                    grid_left,
                    bar_width,
                    beats_per_bar,
                );
                sought = true;
            }
            if sought && let Some(grid) = snap_grid_beats {
                *position_beats = crate::timeline::snap_beats_to_grid(*position_beats, grid);
            }
            let old_ruler_format = arrange_view_state.ruler_format;
            let old_ruler_time_linear = arrange_view_state.ruler_time_linear;
            let old_ruler_bars_beats_linear = arrange_view_state.ruler_bars_beats_linear;
            ruler_response.context_menu(|ui| {
                theme::menu_popup_style(ui.style_mut());
                for (index, label) in [
                    "Bars+Beats",
                    "Seconds",
                    "Timecode",
                    "Samples",
                    "60 fps (User)",
                ]
                .into_iter()
                .enumerate()
                {
                    if ui
                        .selectable_value(&mut arrange_view_state.ruler_format, index as u8, label)
                        .clicked()
                    {
                        ui.close();
                    }
                }
                ui.separator();
                ui.checkbox(&mut arrange_view_state.ruler_time_linear, "Time Linear");
                ui.checkbox(
                    &mut arrange_view_state.ruler_bars_beats_linear,
                    "Bars+Beats Linear",
                );
            });
            modified |= old_ruler_format != arrange_view_state.ruler_format
                || old_ruler_time_linear != arrange_view_state.ruler_time_linear
                || old_ruler_bars_beats_linear != arrange_view_state.ruler_bars_beats_linear;
            let ruler_grid_painter = painter.with_clip_rect(Rect::from_min_max(
                Pos2::new(header_left + lane_header_width, ruler.top()),
                ruler.right_bottom(),
            ));

            let track_toolbar = [
                ("=", "Track Controls Settings", false),
                ("?", "Toggle In-place Editor for Selected Tracks", false),
                ("M", "Unmute All", true),
                ("S", "Deactivate All Solo", true),
                ("R", "All Automation to Read Status", true),
                ("W", "All Automation to Write Status", true),
                ("C", "Show/Hide Track Colors", true),
            ];
            for (index, (label, tooltip, enabled)) in track_toolbar.into_iter().enumerate() {
                let rect = Rect::from_min_size(
                    Pos2::new(header_left + 4.0 + index as f32 * 25.0, ruler.top() + 3.0),
                    Vec2::new(21.0, 15.0),
                );
                use crate::icon_system::IconId;
                let icon = match label {
                    "M" => Some(IconId::GlyphMute),
                    "S" => Some(IconId::GlyphSolo),
                    "R" => Some(IconId::GlyphRecord),
                    "W" => Some(IconId::GlyphWrite),
                    _ => None,
                };
                let response = ui
                    .put(
                        rect,
                        egui::Button::new(if icon.is_some() { "" } else { label }).small(),
                    )
                    .on_hover_text(tooltip);
                if let Some(icon) = icon {
                    crate::icon_system::paint_app_icon(
                        ui,
                        rect.shrink(1.0),
                        icon,
                        ui.visuals().text_color(),
                    );
                }
                if enabled && response.clicked() {
                    match index {
                        2 => clear_all_muted = true,
                        3 => clear_all_solo = true,
                        4 => {
                            for track in &mut project.tracks {
                                if !track.automation_read {
                                    track.automation_read = true;
                                    modified = true;
                                }
                            }
                        }
                        5 => {
                            for track in &mut project.tracks {
                                if !track.automation_write {
                                    track.automation_write = true;
                                    modified = true;
                                }
                            }
                        }
                        6 => {
                            arrange_view_state.show_track_colors =
                                !arrange_view_state.show_track_colors;
                            modified = true;
                        }
                        _ => {}
                    }
                }
            }

            let pan_id = Id::new("project-middle-button-pan");
            let pan_rect =
                Rect::from_min_max(Pos2::new(grid_clip.left(), ruler.top()), grid_clip.max);
            let pan_was_active = ui.data(|data| data.get_temp::<bool>(pan_id).unwrap_or(false));
            let pan_pressed = ui.input(|input| {
                input.pointer.button_pressed(egui::PointerButton::Middle)
                    && input
                        .pointer
                        .hover_pos()
                        .is_some_and(|pos| pan_rect.contains(pos))
            });
            if pan_pressed {
                ui.data_mut(|data| data.insert_temp(pan_id, true));
            }
            let pan_active = pan_pressed || pan_was_active;
            if pan_active
                && ui.input(|input| input.pointer.button_down(egui::PointerButton::Middle))
            {
                let delta = if pan_pressed {
                    Vec2::ZERO
                } else {
                    ui.input(|input| input.pointer.delta())
                };
                arrange_view_state.timeline_scroll_x =
                    (arrange_view_state.timeline_scroll_x - delta.x).max(0.0);
                arrange_view_state.track_scroll_y =
                    (arrange_view_state.track_scroll_y - delta.y).max(0.0);
                custom_scroll_offset = Some(Vec2::new(
                    arrange_view_state.timeline_scroll_x,
                    arrange_view_state.track_scroll_y,
                ));
            }
            if pan_active
                && ui.input(|input| input.pointer.button_released(egui::PointerButton::Middle))
            {
                ui.data_mut(|data| data.remove::<bool>(pan_id));
            }

            ui.set_clip_rect(track_clip);
            let painter = painter.with_clip_rect(track_clip);

            let background_rect = Rect::from_min_max(
                Pos2::new(header_left, canvas.top() + ruler_height + tracks_height),
                Pos2::new(header_left + lane_header_width, canvas.bottom()),
            );
            if background_rect.is_positive() {
                let response = ui.interact(
                    background_rect,
                    Id::new("project-empty-track-list"),
                    Sense::click(),
                );
                project_controls::background_menu(&response, context_action);
            }

            // Paint header selection before the grid so lane fills never erase bar lines.
            let mut pending_midi_part: Option<(u64, f64, f64)> = None;
            for row in 0..visible_rows {
                let top = canvas.top() + ruler_height + row_offsets[row];
                let track_id = track_ids[row];
                let header_indent = project_track_group_depth(project, track_id) as f32 * 24.0;
                let header = Rect::from_min_size(
                    Pos2::new(header_left + header_indent, top),
                    Vec2::new(lane_header_width - header_indent, row_heights[row]),
                );
                let selected = arrange_view_state.selected_track_ids.contains(&track_id);
                painter.rect_filled(
                    header,
                    0.0,
                    project_track_header_fill(palette, selected, ui.visuals().dark_mode),
                );
                if header_indent > 0.0 {
                    let center_y = top + row_heights[row] * 0.5;
                    let connector_x = header_left + header_indent - 9.0;
                    painter.line_segment(
                        [
                            Pos2::new(header_left + 8.0, center_y),
                            Pos2::new(connector_x, center_y),
                        ],
                        Stroke::new(1.5, palette.muted.gamma_multiply(0.8)),
                    );
                }
                painter.line_segment(
                    [
                        Pos2::new(header_left, top + row_heights[row]),
                        Pos2::new(viewport.right(), top + row_heights[row]),
                    ],
                    Stroke::new(1.0, palette.grid),
                );
            }
            let grid_painter = painter.with_clip_rect(grid_clip);
            grid_painter.line_segment(
                [
                    Pos2::new(grid_left, canvas.min.y),
                    Pos2::new(grid_left, viewport.bottom()),
                ],
                Stroke::new(1.0, palette.grid),
            );
            let first_visible_bar = ((viewport.left() + lane_header_width - grid_left) / bar_width)
                .floor()
                .max(0.0) as usize;
            let last_visible_bar = ((viewport.right() - grid_left) / bar_width)
                .ceil()
                .max(0.0)
                .min(content_bars as f32) as usize;
            let ruler_font = FontId::monospace(theme::type_size::RULER);
            let label_width = [first_visible_bar, last_visible_bar]
                .into_iter()
                .map(|bar| {
                    ruler_grid_painter
                        .layout_no_wrap(
                            ruler_label(
                                bar,
                                arrange_view_state.ruler_format,
                                beats_per_bar,
                                project.tempo_bpm,
                                sample_rate,
                            ),
                            ruler_font.clone(),
                            palette.text,
                        )
                        .size()
                        .x
                })
                .fold(0.0_f32, f32::max);
            let label_stride =
                crate::timeline::ruler_stride(bar_width, (label_width + 12.0).max(44.0));
            let tick_divisions =
                if arrange_view_state.grid_type == crate::timeline::GridType::UseQuantize {
                    0
                } else {
                    crate::timeline::ruler_subdivisions(bar_width, beats_per_bar, 12.0)
                };
            let subdivision_label_width = ruler_grid_painter
                .layout_no_wrap(
                    format!("{}.{}.4", last_visible_bar + 1, beats_per_bar),
                    ruler_font.clone(),
                    palette.text,
                )
                .size()
                .x;
            let label_divisions = if arrange_view_state.ruler_format == 0 {
                crate::timeline::ruler_subdivisions(
                    bar_width,
                    beats_per_bar,
                    (subdivision_label_width + 12.0).max(44.0),
                )
            } else {
                0
            };
            let tick_stride = crate::timeline::ruler_stride(bar_width, 8.0);
            let first_tick = first_visible_bar / tick_stride * tick_stride;
            for bar in (first_tick..=last_visible_bar).step_by(tick_stride) {
                let x = grid_left + bar as f32 * bar_width;
                grid_painter.line_segment(
                    [
                        Pos2::new(x, canvas.min.y + ruler_height),
                        Pos2::new(x, viewport.bottom()),
                    ],
                    Stroke::new(1.0, palette.grid),
                );
                ruler_grid_painter.line_segment(
                    [
                        Pos2::new(x, ruler.top()),
                        Pos2::new(
                            x,
                            ruler.top() + if bar % label_stride == 0 { 8.0 } else { 4.0 },
                        ),
                    ],
                    Stroke::new(1.0, palette.muted),
                );
                if bar < content_bars && bar % label_stride == 0 {
                    ruler_grid_painter.text(
                        Pos2::new(x + 2.0, ruler.center().y),
                        Align2::LEFT_CENTER,
                        ruler_label(
                            bar,
                            arrange_view_state.ruler_format,
                            beats_per_bar,
                            project.tempo_bpm,
                            sample_rate,
                        ),
                        ruler_font.clone(),
                        palette.text,
                    );
                }
            }

            // Refine the same globally anchored bar grid as space opens up.
            // Musical subdivisions never change the selected Snap grid.
            for bar in first_visible_bar..=last_visible_bar {
                if tick_divisions > 0 {
                    for slot in 1..usize::from(beats_per_bar) * tick_divisions {
                        let x = grid_left
                            + (bar as f32
                                + slot as f32 / (f32::from(beats_per_bar) * tick_divisions as f32))
                                * bar_width;
                        let sixteenth = slot * 4 / tick_divisions;
                        let (height, strength) = if sixteenth % 4 == 0 {
                            (7.0, 0.65)
                        } else if sixteenth % 2 == 0 {
                            (5.0, 0.40)
                        } else {
                            (3.0, 0.25)
                        };
                        grid_painter.line_segment(
                            [
                                Pos2::new(x, grid_clip.top()),
                                Pos2::new(x, grid_clip.bottom()),
                            ],
                            Stroke::new(1.0, palette.grid.gamma_multiply(strength)),
                        );
                        ruler_grid_painter.line_segment(
                            [
                                Pos2::new(x, ruler.top()),
                                Pos2::new(x, ruler.top() + height),
                            ],
                            Stroke::new(1.0, palette.muted.gamma_multiply(strength.max(0.5))),
                        );
                    }
                }
                if label_divisions > 0 && bar < content_bars {
                    for slot in 1..usize::from(beats_per_bar) * label_divisions {
                        let x = grid_left
                            + (bar as f32
                                + slot as f32
                                    / (f32::from(beats_per_bar) * label_divisions as f32))
                                * bar_width;
                        ruler_grid_painter.text(
                            Pos2::new(x + 2.0, ruler.center().y),
                            Align2::LEFT_CENTER,
                            crate::timeline::subdivision_label(bar, slot, label_divisions),
                            ruler_font.clone(),
                            palette.text,
                        );
                    }
                }
            }

            // Quantize grid positions include triplets/dotted values and stay
            // anchored at project zero; label density remains independent.
            if arrange_view_state.grid_type == crate::timeline::GridType::UseQuantize {
                let mut step = arrange_view_state.quantize.step_beats();
                // Draw a power-of-two subset when the selected grid is too
                // fine to see. Snapping always keeps its original resolution.
                while (step / beats_per_bar_f64) as f32 * bar_width < 8.0 {
                    step *= 2.0;
                }
                if (step / beats_per_bar_f64) as f32 * bar_width >= 8.0 {
                    let first = (((grid_clip.left() - grid_left) / bar_width) as f64
                        * beats_per_bar_f64
                        / step)
                        .floor()
                        .max(0.0) as u64;
                    let last = (((grid_clip.right() - grid_left) / bar_width) as f64
                        * beats_per_bar_f64
                        / step)
                        .ceil()
                        .max(0.0) as u64;
                    for index in first..=last {
                        let beat = index as f64 * step;
                        if (beat / beats_per_bar_f64 - (beat / beats_per_bar_f64).round()).abs()
                            < 1e-8
                        {
                            continue;
                        }
                        let x = grid_left + (beat / beats_per_bar_f64) as f32 * bar_width;
                        grid_painter.line_segment(
                            [
                                Pos2::new(x, grid_clip.top()),
                                Pos2::new(x, grid_clip.bottom()),
                            ],
                            Stroke::new(1.0, palette.grid.gamma_multiply(0.55)),
                        );
                        ruler_grid_painter.line_segment(
                            [Pos2::new(x, ruler.top()), Pos2::new(x, ruler.top() + 5.0)],
                            Stroke::new(1.0, palette.muted.gamma_multiply(0.7)),
                        );
                    }
                }
            }

            // Right-angle corners sit on the locator bounds, with each flag
            // pointing inward over the selected interval. Draw above ticks.
            let flag_size = 7.0_f32.min((range_end_x - range_start_x).max(0.0) * 0.25);
            for (x, inward) in [(range_start_x, 1.0), (range_end_x, -1.0)] {
                range_painter.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(x, ruler.top()),
                        Pos2::new(x + inward * flag_size, ruler.top()),
                        Pos2::new(x, ruler.top() + flag_size),
                    ],
                    Color32::WHITE,
                    Stroke::new(1.0, Color32::BLACK),
                ));
            }

            for row in 0..visible_rows {
                let lane_height = row_heights[row];
                if lane_height <= 0.0 {
                    continue;
                }
                let base_track_height = arrange_view_state.track_height(track_ids[row]);
                let take_lane_count = project
                    .tracks
                    .get(row)
                    .map(|track| audio_events::take_lane_layout(track, project.tempo_bpm).0)
                    .unwrap_or(1);
                let take_lanes_expanded = arrange_view_state
                    .take_lanes_visible
                    .contains(&track_ids[row]);
                let media_lane_height = base_track_height
                    * if take_lanes_expanded {
                        take_lane_count as f32
                    } else {
                        1.0
                    };
                let top = canvas.min.y + ruler_height + row_offsets[row];
                let row_track_id = track_ids[row];
                let header_indent = project_track_group_depth(project, row_track_id) as f32 * 24.0;
                // Child header controls stay inside the inset panel while the
                // arrangement grid remains aligned with the rest of the tracks.
                let row_header_width = lane_header_width - header_indent;
                let row_rect = Rect::from_min_size(
                    Pos2::new(header_left + header_indent, top),
                    Vec2::new(total_size.x - header_indent, lane_height),
                );
                if primary_down
                    && press_origin.is_some_and(|origin| {
                        origin.x >= header_left
                            && origin.x < header_left + lane_header_width
                            && row_rect.top() <= origin.y
                            && origin.y < row_rect.bottom()
                    })
                {
                    arrange_view_state.pending_track_drag_ids = Some(
                        if arrange_view_state
                            .selected_track_ids
                            .contains(&row_track_id)
                        {
                            track_ids
                                .iter()
                                .copied()
                                .filter(|id| arrange_view_state.selected_track_ids.contains(id))
                                .collect()
                        } else {
                            vec![row_track_id]
                        },
                    );
                    arrange_view_state.pending_group_drop = None;
                }
                let inherited_group_state =
                    project_track_inherited_group_state(project, row_track_id);
                let group_child_solo = project
                    .tracks
                    .get(row)
                    .is_some_and(|group| group.kind == TrackKind::Group)
                    && project.tracks.iter().any(|child| {
                        project_track_is_in_group(project, child.id, row_track_id)
                            && (child.solo || child.solo_defeat)
                    });
                if let Some(track) = project.tracks.get_mut(row) {
                    let resize_rect = Rect::from_min_max(
                        Pos2::new(row_rect.left(), top + lane_height - 5.0),
                        Pos2::new(row_rect.left() + row_header_width, top + lane_height + 2.0),
                    );
                    let resize_response = ui.interact(
                        resize_rect,
                        Id::new(("track-height-resize", track.id)),
                        Sense::drag(),
                    );
                    if resize_response.hovered() || resize_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                    if resize_response.drag_started() {
                        arrange_view_state.leave_exclusive_mode();
                    }
                    if resize_response.dragged() {
                        let delta = ui.input(|input| input.pointer.delta().y);
                        if ui.input(|input| input.modifiers.ctrl) {
                            for track_id in &track_ids {
                                let height = arrange_view_state.track_height(*track_id) + delta;
                                arrange_view_state.set_track_height(*track_id, height);
                            }
                        } else {
                            let height = arrange_view_state.track_height(track.id) + delta;
                            arrange_view_state.set_track_height(track.id, height);
                        }
                    }

                    let arrange_rect = Rect::from_min_size(
                        Pos2::new(grid_left, row_rect.min.y),
                        Vec2::new(total_size.x - lane_header_width, lane_height),
                    );
                    let midi_track = matches!(track.kind, TrackKind::Midi | TrackKind::Instrument);
                    let event_track = track.kind == TrackKind::Audio
                        || (track.kind == TrackKind::Group && track.group_collapsed);
                    let arrange_response = ui.interact(
                        arrange_rect.intersect(grid_clip),
                        Id::new(("track-arrange-row", track.id)),
                        if event_track {
                            // Audio clip handles register their own click-and-drag responses
                            // later in the frame. A full-row drag response here captures the
                            // pointer first and prevents those handles from receiving drags.
                            Sense::hover()
                        } else {
                            Sense::click_and_drag()
                        },
                    );
                    if *edit_tool == EditTool::Draw && arrange_response.hovered() {
                        crate::system_tool_icons::tool_cursor_only_at(
                            ui,
                            ToolbarIcon::Draw,
                            arrange_response.hover_pos(),
                        );
                    }
                    if midi_track && *edit_tool == EditTool::Draw {
                        let minimum_part_grid = snap_grid_beats.unwrap_or(1.0);
                        let beat_at_x = |x: f32| {
                            let beat =
                                (((x - grid_left) / bar_width).max(0.0) as f64) * beats_per_bar_f64;
                            snap_grid_beats.map_or(beat, |grid| {
                                crate::timeline::snap_beats_to_grid(beat, grid)
                            })
                        };
                        let pointer = ui.input(|input| input.pointer.interact_pos());
                        let origin = ui.input(|input| input.pointer.press_origin());
                        let over_part = |point: Pos2| {
                            let beat = (((point.x - grid_left) / bar_width).max(0.0) as f64)
                                * beats_per_bar_f64;
                            point.y >= top + 6.0
                                && point.y <= top + media_lane_height - 6.0
                                && track.midi_parts.iter().any(|part| {
                                    beat >= part.start_beats
                                        && beat <= part.start_beats + part.duration_beats
                                })
                        };
                        let draw_origin = origin.filter(|point| {
                            arrange_rect.contains(*point)
                                && point.y >= top + 6.0
                                && point.y <= top + media_lane_height - 6.0
                                && !over_part(*point)
                        });
                        if arrange_response.drag_started()
                            && let Some(origin) = draw_origin
                        {
                            let beat = beat_at_x(origin.x);
                            *midi_part_draw = Some(MidiPartDraw {
                                track_id: track.id,
                                start_beats: beat,
                                end_beats: beat,
                            });
                        }
                        if let Some(draw) = midi_part_draw
                            && draw.track_id == track.id
                        {
                            if let Some(pointer) = pointer {
                                draw.end_beats = beat_at_x(pointer.x);
                            }
                            if arrange_response.drag_stopped() {
                                pending_midi_part =
                                    Some((track.id, draw.start_beats, draw.end_beats));
                                *midi_part_draw = None;
                            }
                        } else if arrange_response.clicked()
                            && let Some(point) = pointer
                            && arrange_rect.contains(point)
                            && !over_part(point)
                        {
                            let start = beat_at_x(point.x);
                            pending_midi_part = Some((track.id, start, start + minimum_part_grid));
                        }
                    }
                    let header_response = ui.interact(
                        Rect::from_min_size(
                            row_rect.min,
                            Vec2::new(row_header_width - 20.0, lane_height),
                        ),
                        Id::new(("project-track-header", track.id)),
                        Sense::click_and_drag(),
                    );
                    let name_rect = Rect::from_min_size(
                        Pos2::new(row_rect.min.x + 82.0, top + 2.0),
                        Vec2::new(row_header_width - 102.0, 20.0),
                    );
                    let name_response = ui.interact(
                        name_rect,
                        Id::new(("track-header-name", track.id)),
                        Sense::click_and_drag(),
                    );
                    if header_response.clicked()
                        || name_response.clicked()
                        || header_response.secondary_clicked()
                        || name_response.secondary_clicked()
                    {
                        arrange_view_state.audio_events.focus_row();
                    }
                    if header_response.secondary_clicked() || name_response.secondary_clicked() {
                        if !arrange_view_state.selected_track_ids.contains(&track.id) {
                            select_arrange_track(
                                &mut project.selected_track,
                                &mut project.selected_midi_part_id,
                                &track_ids,
                                arrange_view_state,
                                row,
                                egui::Modifiers::NONE,
                            );
                        }
                    }
                    project_controls::track_menu(
                        &header_response,
                        track.id,
                        can_remove,
                        arrange_view_state.automation_visible.contains(&track.id),
                        !arrange_view_state.automation_visible.is_empty(),
                        context_action,
                    );
                    project_controls::track_menu(
                        &name_response,
                        track.id,
                        can_remove,
                        arrange_view_state.automation_visible.contains(&track.id),
                        !arrange_view_state.automation_visible.is_empty(),
                        context_action,
                    );
                    if arrange_response.clicked()
                        || name_response.clicked()
                        || header_response.clicked()
                    {
                        let modifiers = ui.input(|input| input.modifiers);
                        select_arrange_track(
                            &mut project.selected_track,
                            &mut project.selected_midi_part_id,
                            &track_ids,
                            arrange_view_state,
                            row,
                            modifiers,
                        );
                    }
                    let rename_id = Id::new(("inline-track-rename", track.id));
                    let rename_started = name_response.double_clicked();
                    if rename_started {
                        ui.data_mut(|data| data.insert_temp(rename_id, track.name.clone()));
                    }
                    let mut name_editing = false;
                    if let Some(mut draft) = ui.data_mut(|data| data.get_temp::<String>(rename_id))
                    {
                        name_editing = true;
                        let response = ui.put(
                            name_rect,
                            egui::TextEdit::singleline(&mut draft)
                                .font(FontId::proportional(11.0))
                                .margin(Vec2::new(2.0, 0.0)),
                        );
                        if rename_started {
                            response.request_focus();
                        }
                        let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
                        let commit = ui.input(|i| i.key_pressed(egui::Key::Enter))
                            || (response.lost_focus() && !rename_started);
                        if cancel || commit {
                            if commit && !draft.trim().is_empty() {
                                track.name = draft.trim().to_owned();
                                modified = true;
                            }
                            ui.data_mut(|data| data.remove::<String>(rename_id));
                        } else {
                            ui.data_mut(|data| data.insert_temp(rename_id, draft));
                        }
                    }
                    if header_response.dragged()
                        || name_response.dragged()
                        || header_response.drag_stopped()
                        || name_response.drag_stopped()
                    {
                        let drag_active = header_response.dragged() || name_response.dragged();
                        let drag_stopped =
                            header_response.drag_stopped() || name_response.drag_stopped();
                        if let Some(pointer) = ui.input(|input| {
                            input
                                .pointer
                                .hover_pos()
                                .or_else(|| input.pointer.interact_pos())
                        }) {
                            let dragged_ids = if arrange_view_state
                                .selected_track_ids
                                .contains(&track.id)
                            {
                                track_ids
                                    .iter()
                                    .copied()
                                    .filter(|id| arrange_view_state.selected_track_ids.contains(id))
                                    .collect::<Vec<_>>()
                            } else {
                                vec![track.id]
                            };
                            let hovered_row = (0..visible_rows).find(|candidate| {
                                let height = row_heights[*candidate];
                                height > 0.0
                                    && pointer.y
                                        >= canvas.min.y + ruler_height + row_offsets[*candidate]
                                    && pointer.y
                                        <= canvas.min.y
                                            + ruler_height
                                            + row_offsets[*candidate]
                                            + height
                            });
                            let group_target = hovered_row.and_then(|candidate| {
                                let group_id = *track_ids.get(candidate)?;
                                (group_track_ids.contains(&group_id)
                                    && dragged_ids.iter().any(|id| *id != group_id))
                                .then_some((candidate, group_id))
                            });

                            // Keep the last valid group target through the release
                            // frame so a transient pointer update cannot turn this
                            // gesture into an ordinary row reorder.
                            if drag_active && !drag_stopped {
                                arrange_view_state.pending_group_drop =
                                    group_target.and_then(|(_, group_id)| {
                                        let ids = dragged_ids
                                            .iter()
                                            .copied()
                                            .filter(|id| *id != group_id)
                                            .collect::<Vec<_>>();
                                        (!ids.is_empty()).then_some((group_id, ids))
                                    });
                            }

                            let resolved_group_drop = group_target
                                .map(|(_, group_id)| {
                                    let ids = dragged_ids
                                        .iter()
                                        .copied()
                                        .filter(|id| *id != group_id)
                                        .collect::<Vec<_>>();
                                    (group_id, ids)
                                })
                                .filter(|(_, ids)| !ids.is_empty())
                                .or_else(|| arrange_view_state.pending_group_drop.clone());

                            if let Some((candidate, _)) = group_target {
                                group_drop_indicator = Some(candidate);
                            }
                            if drag_stopped {
                                if let Some((group_id, dragged_ids)) = resolved_group_drop {
                                    group_drop_after_draw = Some((group_id, dragged_ids));
                                    // Grouping wins if the release also falls in the
                                    // normal reorder zone on this frame.
                                    reorder_after_draw = None;
                                } else if group_target.is_none() {
                                    let insertion = (0..visible_rows)
                                        .filter(|candidate| row_heights[*candidate] > 0.0)
                                        .find(|candidate| {
                                            pointer.y
                                                < canvas.min.y
                                                    + ruler_height
                                                    + row_offsets[*candidate]
                                                    + row_heights[*candidate] * 0.5
                                        })
                                        .unwrap_or(visible_rows);
                                    reorder_after_draw = Some((dragged_ids, insertion));
                                }
                                arrange_view_state.pending_group_drop = None;
                            } else if group_target.is_none() {
                                let insertion = (0..visible_rows)
                                    .filter(|candidate| row_heights[*candidate] > 0.0)
                                    .find(|candidate| {
                                        pointer.y
                                            < canvas.min.y
                                                + ruler_height
                                                + row_offsets[*candidate]
                                                + row_heights[*candidate] * 0.5
                                    })
                                    .unwrap_or(visible_rows);
                                reorder_indicator = Some(insertion);
                            }
                        } else if drag_stopped {
                            group_drop_after_draw = arrange_view_state.pending_group_drop.take();
                        }
                    }
                    let track_color = if arrange_view_state.show_track_colors {
                        project_controls::track_color(track)
                    } else {
                        theme::palette(ui).muted
                    };
                    let color_rect = Rect::from_min_size(
                        Pos2::new(row_rect.left() + row_header_width - 20.0, top + 1.0),
                        Vec2::new(8.0, lane_height - 2.0),
                    );
                    modified |= project_controls::color_strip(
                        ui,
                        color_rect,
                        track,
                        arrange_view_state.show_track_colors,
                    );
                    let meter_rect = Rect::from_min_size(
                        Pos2::new(color_rect.right(), top + 1.0),
                        Vec2::new(11.0, lane_height - 2.0),
                    );
                    paint_track_meter_channels(
                        ui,
                        meter_rect,
                        meter_levels.get(&track.id).copied().unwrap_or([0.0; 2]),
                        track_color,
                        track.audio_channels,
                    );
                    if let Some(hold) = peak_holds.get(&track.id) {
                        paint_peak_markers_channels(
                            ui,
                            meter_rect,
                            hold.markers,
                            track.audio_channels,
                        );
                    }
                    painter.text(
                        Pos2::new(row_rect.left() + 24.0, top + 10.0),
                        Align2::CENTER_CENTER,
                        (row + 1).to_string(),
                        FontId::proportional(10.0),
                        theme::palette(ui).muted,
                    );
                    project_controls::type_icon(
                        ui,
                        Pos2::new(row_rect.left() + 9.0, top + 11.0),
                        track,
                    );
                    let reveal_rect = Rect::from_center_size(
                        Pos2::new(row_rect.left() + 9.0, top + 31.0),
                        Vec2::splat(16.0),
                    );
                    let reveal = ui
                        .interact(
                            reveal_rect,
                            Id::new(("automation-reveal", track.id)),
                            Sense::click(),
                        )
                        .on_hover_text("Show/Hide Automation");
                    let expanded = arrange_view_state.automation_visible.contains(&track.id);
                    if reveal.hovered() || header_response.hovered() || expanded {
                        painter.circle_filled(
                            reveal_rect.center(),
                            7.0,
                            theme::palette(ui).surface_raised,
                        );
                        let c = reveal_rect.center();
                        let direction = if expanded { -1.0 } else { 1.0 };
                        painter.add(egui::Shape::line(
                            vec![
                                c + Vec2::new(-3.0, -direction * 1.5),
                                c + Vec2::new(0.0, direction * 1.5),
                                c + Vec2::new(3.0, -direction * 1.5),
                            ],
                            Stroke::new(1.0, theme::palette(ui).text),
                        ));
                    }
                    if reveal.clicked() {
                        if expanded {
                            arrange_view_state.automation_visible.remove(&track.id);
                        } else {
                            arrange_view_state.automation_visible.insert(track.id);
                        }
                        modified = true;
                    }
                    if expanded {
                        let automation_rect = Rect::from_min_max(
                            Pos2::new(grid_clip.left(), top + media_lane_height),
                            Pos2::new(grid_clip.right(), top + lane_height),
                        );
                        grid_painter.rect_filled(
                            automation_rect,
                            0.0,
                            theme::palette(ui).canvas_alternate,
                        );
                        grid_painter.line_segment(
                            [automation_rect.left_top(), automation_rect.right_top()],
                            Stroke::new(1.0, theme::palette(ui).grid),
                        );
                        let y = top + lane_height - 42.0;
                        let ar = Rect::from_min_size(
                            Pos2::new(row_rect.left() + 32.0, y),
                            Vec2::new(row_header_width - 52.0, 42.0),
                        );
                        painter.rect_filled(ar, 0.0, theme::palette(ui).surface);
                        painter.line_segment(
                            [
                                Pos2::new(grid_left, y + 11.0),
                                Pos2::new(viewport.right(), y + 11.0),
                            ],
                            Stroke::new(1.0, theme::palette(ui).muted),
                        );
                        modified |= project_controls::automation_lane(ui, ar, track);
                    }
                    if take_lanes_expanded {
                        for take in 0..take_lane_count {
                            let lane_top = top + take as f32 * base_track_height;
                            if take > 0 {
                                let line_y = lane_top;
                                painter.line_segment(
                                    [
                                        Pos2::new(grid_clip.left(), line_y),
                                        Pos2::new(grid_clip.right(), line_y),
                                    ],
                                    Stroke::new(1.0, theme::palette(ui).grid),
                                );
                                painter.line_segment(
                                    [
                                        Pos2::new(row_rect.left(), line_y),
                                        Pos2::new(color_rect.left(), line_y),
                                    ],
                                    Stroke::new(1.0, theme::palette(ui).grid),
                                );
                            }
                            let label = if take + 1 == take_lane_count {
                                format!("Take {} · current", take + 1)
                            } else {
                                format!("Take {}", take + 1)
                            };
                            painter.text(
                                Pos2::new(
                                    row_rect.left() + 35.0,
                                    lane_top + base_track_height * 0.5,
                                ),
                                Align2::LEFT_CENTER,
                                label,
                                FontId::proportional(9.0),
                                if take + 1 == take_lane_count {
                                    theme::palette(ui).text
                                } else {
                                    theme::palette(ui).muted
                                },
                            );
                        }
                    }
                    let compact_controls =
                        (media_lane_height - if expanded { 42.0 } else { 0.0 }) < 54.0;
                    let button_height = 14.0;
                    let text_painter = painter.with_clip_rect(Rect::from_min_size(
                        Pos2::new(row_rect.min.x + 32.0, top),
                        Vec2::new(row_header_width - 32.0, lane_height),
                    ));
                    if !name_editing {
                        text_painter.text(
                            Pos2::new(row_rect.min.x + 85.0, top + 10.0),
                            Align2::LEFT_CENTER,
                            &track.name,
                            FontId::proportional(if compact_controls {
                                theme::type_size::TRACK_COMPACT
                            } else {
                                theme::type_size::TRACK_NAME
                            }),
                            project_controls::label_ink(project_track_header_fill(
                                palette,
                                arrange_view_state.selected_track_ids.contains(&track.id),
                                ui.visuals().dark_mode,
                            )),
                        );
                    }
                    let channel_label = match track.kind {
                        TrackKind::Audio if track.audio_channels == 1 => "mono",
                        TrackKind::Audio => "stereo",
                        TrackKind::Group if track.audio_channels == 1 => "mono group",
                        TrackKind::Group => "stereo group",
                        _ => track_kind_label(track.kind),
                    };
                    if !take_lanes_expanded {
                        text_painter.text(
                            Pos2::new(row_rect.min.x + 35.0, top + 26.0),
                            Align2::LEFT_CENTER,
                            channel_label,
                            FontId::proportional(theme::type_size::SMALL),
                            track_color,
                        );
                    }
                    let take_lanes_supported = track.kind == TrackKind::Audio;
                    let group_collapse_control = track.kind == TrackKind::Group;
                    let lane_control_label = if group_collapse_control {
                        "▤G"
                    } else {
                        "▤"
                    };
                    let lane_control_active = if group_collapse_control {
                        !track.group_collapsed
                    } else {
                        arrange_view_state.take_lanes_visible.contains(&track.id)
                    };
                    for (button_index, (label, active, _fill, tooltip)) in [
                        (
                            "M",
                            track.muted || inherited_group_state.0,
                            theme::WARNING,
                            if inherited_group_state.0 {
                                "Mute (inherited from group)"
                            } else {
                                "Mute"
                            },
                        ),
                        (
                            "S",
                            track.solo
                                || track.solo_defeat
                                || inherited_group_state.1
                                || inherited_group_state.2,
                            theme::palette(ui).accent,
                            if group_child_solo {
                                "Solo is active on a child track"
                            } else if inherited_group_state.1 || inherited_group_state.2 {
                                "Solo State (inherited from group; ALT click for Solo Defeat)"
                            } else {
                                "Solo State (Solo Defeat with ALT click)"
                            },
                        ),
                        ("R", track.armed, theme::RECORD, "Record Enable"),
                        (
                            "MON",
                            track.monitor,
                            theme::palette(ui).surface_raised,
                            "Monitor",
                        ),
                        (
                            "E",
                            false,
                            theme::palette(ui).surface_raised,
                            "Open Track Effects",
                        ),
                        (
                            lane_control_label,
                            lane_control_active,
                            theme::palette(ui).surface_raised,
                            if group_collapse_control {
                                if track.group_collapsed {
                                    "Expand Group Tracks"
                                } else {
                                    "Collapse Group Tracks"
                                }
                            } else if take_lanes_supported {
                                "Show/Hide Track Take Lanes"
                            } else {
                                "Take Lanes are available on audio tracks"
                            },
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let (offset, control_top, width) = match button_index {
                            0 => (37.0, 3.0, 22.0),
                            1 => (61.0, 3.0, 22.0),
                            2 => (88.0, 22.0, 16.0),
                            3 => (107.0, 22.0, 14.0),
                            4 => (130.0, 22.0, 22.0),
                            5 => (154.0, 22.0, 22.0),
                            _ => (177.0, 22.0, 16.0),
                        };
                        let control_scale = (row_header_width / lane_header_width).clamp(0.75, 1.0);
                        let button_rect = Rect::from_min_size(
                            Pos2::new(row_rect.left() + offset * control_scale, top + control_top),
                            Vec2::new(width * control_scale, button_height),
                        );
                        let response = project_inspector::button(
                            ui,
                            button_rect,
                            ("track-control", track.id, button_index),
                            label,
                            tooltip,
                            active,
                            button_index != 5 || take_lanes_supported || group_collapse_control,
                            button_index == 2,
                        );
                        if response.clicked() {
                            match button_index {
                                0 => {
                                    track.muted = !track.muted;
                                    modified = true;
                                }
                                1 => {
                                    let modifiers = ui.input(|input| input.modifiers);
                                    if modifiers.alt {
                                        track.solo_defeat = !track.solo_defeat;
                                        if track.solo_defeat {
                                            track.solo = false;
                                        }
                                    } else {
                                        track.solo = !track.solo;
                                        if track.solo {
                                            track.solo_defeat = false;
                                            if modifiers.ctrl {
                                                exclusive_solo_track_id = Some(track.id);
                                            }
                                        }
                                    }
                                    modified = true;
                                }
                                2 => {
                                    track.armed = !track.armed;
                                    modified = true;
                                }
                                3 => {
                                    track.monitor = !track.monitor;
                                    modified = true;
                                }
                                4 => {
                                    project.selected_track = Some(row as u32);
                                    project.selected_midi_part_id = None;
                                    arrange_view_state.selected_track_ids.clear();
                                    arrange_view_state.selected_track_ids.insert(track.id);
                                    arrange_view_state.selection_anchor_track_id = Some(track.id);
                                    channel_settings_window.request_channel_settings(None);
                                    modified = true;
                                }
                                5 => {
                                    if group_collapse_control {
                                        track.group_collapsed = !track.group_collapsed;
                                    } else if !arrange_view_state
                                        .take_lanes_visible
                                        .remove(&track.id)
                                    {
                                        arrange_view_state.take_lanes_visible.insert(track.id);
                                    }
                                    modified = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    for part in &track.midi_parts {
                        let start_bar = part.start_beats / beats_per_bar_f64;
                        let duration_bars = part.duration_beats / beats_per_bar_f64;
                        if !start_bar.is_finite() || start_bar > content_bars as f64 {
                            continue;
                        }
                        let part_left =
                            canvas.min.x + lane_header_width + start_bar as f32 * bar_width;
                        let part_right =
                            (part_left + duration_bars as f32 * bar_width).min(canvas.max.x);
                        if part_right <= part_left {
                            continue;
                        }
                        // Match audio event geometry exactly: MIDI parts occupy the full media lane.
                        let part_rect = Rect::from_min_size(
                            Pos2::new(part_left, top),
                            Vec2::new(part_right - part_left, media_lane_height),
                        );
                        let part_color = track_color;
                        let selected = project.selected_track == Some(row as u32)
                            && project.selected_midi_part_id == Some(part.id);
                        let part_response = ui.interact(
                            part_rect.intersect(grid_clip),
                            Id::new(("midi-part", part.id)),
                            Sense::click_and_drag(),
                        );
                        if part_response.clicked() || part_response.double_clicked() {
                            let selected_track = Some(row as u32);
                            let selection_changed = project.selected_track != selected_track
                                || project.selected_midi_part_id != Some(part.id);
                            project.selected_track = selected_track;
                            project.selected_midi_part_id = Some(part.id);
                            arrange_view_state.audio_events.clear_selection();
                            arrange_view_state.selected_track_ids.clear();
                            arrange_view_state.selected_track_ids.insert(track.id);
                            arrange_view_state.selection_anchor_track_id = Some(track.id);
                            modified |= selection_changed;
                        }
                        if part_response.double_clicked() {
                            *midi_editor_open = true;
                        }
                        if part_response.clicked_by(egui::PointerButton::Primary)
                            && *edit_tool == EditTool::Split
                            && let Some(pointer) = part_response.interact_pointer_pos()
                        {
                            let beat =
                                f64::from((pointer.x - grid_left) / bar_width) * beats_per_bar_f64;
                            let beat = snap_grid_beats.map_or(beat, |grid| {
                                crate::timeline::snap_beats_to_grid(beat, grid)
                            });
                            pending_midi_part_split = Some((track.id, part.id, beat));
                        }
                        if part_response.clicked_by(egui::PointerButton::Primary)
                            && *edit_tool == EditTool::Delete
                        {
                            pending_midi_part_remove = Some((track.id, part.id));
                        }
                        if part_response.hovered() && *edit_tool == EditTool::Draw {
                            crate::system_tool_icons::tool_cursor_only_at(
                                ui,
                                ToolbarIcon::Draw,
                                part_response.hover_pos(),
                            );
                        } else if part_response.hovered() && selected {
                            let pointer = part_response.hover_pos();
                            if pointer.is_some_and(|pos| pos.x >= part_right - 9.0) {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            } else {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                            }
                        }
                        if part_response.drag_started()
                            && *edit_tool == EditTool::Select
                            && let Some(pointer) =
                                ui.ctx().input(|input| input.pointer.press_origin())
                        {
                            project.selected_track = Some(row as u32);
                            project.selected_midi_part_id = Some(part.id);
                            arrange_view_state.audio_events.clear_selection();
                            arrange_view_state.selected_track_ids.clear();
                            arrange_view_state.selected_track_ids.insert(track.id);
                            arrange_view_state.selection_anchor_track_id = Some(track.id);
                            let pointer_beats =
                                f64::from((pointer.x - grid_left) / bar_width) * beats_per_bar_f64;
                            let resize = pointer.x >= part_right - 9.0;
                            arrange_view_state.midi_part_drag = Some((
                                track.id,
                                part.id,
                                resize,
                                part.start_beats,
                                part.duration_beats,
                                if resize {
                                    0.0
                                } else {
                                    pointer_beats - part.start_beats
                                },
                            ));
                        }
                        if part_response.dragged()
                            && let Some((
                                drag_track,
                                drag_part,
                                resize,
                                original_start,
                                original_duration,
                                grab,
                            )) = arrange_view_state.midi_part_drag
                            && drag_track == track.id
                            && drag_part == part.id
                            && let Some(pointer) = part_response.interact_pointer_pos()
                        {
                            let pointer_beats =
                                f64::from((pointer.x - grid_left) / bar_width) * beats_per_bar_f64;
                            let snap = snap_grid_beats.unwrap_or(0.0625).max(0.0625);
                            if resize {
                                let end = crate::timeline::snap_beats_to_grid(pointer_beats, snap);
                                let duration = (end - original_start).max(0.0625);
                                if (part.duration_beats - duration).abs() > f64::EPSILON {
                                    pending_midi_part_edit =
                                        Some((track.id, part.id, original_start, duration));
                                }
                            } else {
                                let start = crate::timeline::snap_beats_to_grid(
                                    (pointer_beats - grab).max(0.0),
                                    snap,
                                );
                                if (part.start_beats - start).abs() > f64::EPSILON {
                                    pending_midi_part_edit =
                                        Some((track.id, part.id, start, original_duration));
                                }
                            }
                        }
                        if part_response.drag_stopped() {
                            arrange_view_state.midi_part_drag = None;
                        }
                        grid_painter.rect_filled(part_rect, egui::CornerRadius::ZERO, part_color);
                        let part_name_band = Rect::from_min_max(
                            Pos2::new(
                                part_rect.left(),
                                (part_rect.bottom() - 13.0).max(part_rect.top()),
                            ),
                            part_rect.max,
                        );
                        grid_painter.rect_filled(
                            part_name_band,
                            egui::CornerRadius::ZERO,
                            if selected { Color32::BLACK } else { part_color },
                        );
                        grid_painter.rect_stroke(
                            part_rect,
                            egui::CornerRadius::ZERO,
                            Stroke::new(
                                1.0,
                                if selected {
                                    Color32::WHITE
                                } else {
                                    Color32::BLACK
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        if selected {
                            for point in [
                                Pos2::new(part_rect.left() + 3.0, part_rect.bottom() - 3.0),
                                Pos2::new(part_rect.right() - 3.0, part_rect.bottom() - 3.0),
                            ] {
                                grid_painter.rect_filled(
                                    Rect::from_center_size(point, Vec2::splat(6.0)),
                                    0.0,
                                    Color32::WHITE,
                                );
                            }
                        }
                        let note_area = Rect::from_min_max(
                            part_rect.min + Vec2::splat(2.0),
                            Pos2::new(part_rect.right() - 2.0, part_name_band.top()),
                        );
                        let note_painter = grid_painter.with_clip_rect(note_area);
                        for note in &part.notes {
                            let note_start_beats = part.start_beats + note.start_beats;
                            let note_end_beats = note_start_beats + note.duration_beats;
                            if note_end_beats <= part.start_beats
                                || note_start_beats >= part.start_beats + part.duration_beats
                            {
                                continue;
                            }
                            let note_start_beats = note_start_beats
                                .max(part.start_beats)
                                .min(part.start_beats + part.duration_beats);
                            let note_end_beats =
                                note_end_beats.min(part.start_beats + part.duration_beats);
                            let note_x = canvas.min.x
                                + lane_header_width
                                + (note_start_beats / beats_per_bar_f64) as f32 * bar_width;
                            let note_width =
                                (((note_end_beats - note_start_beats) / beats_per_bar_f64) as f32
                                    * bar_width)
                                    .max(1.0);
                            let (lowest_pitch, highest_pitch) =
                                part.notes.iter().fold((127_u8, 0_u8), |(low, high), note| {
                                    (low.min(note.pitch), high.max(note.pitch))
                                });
                            let pitch_span =
                                f32::from(highest_pitch.saturating_sub(lowest_pitch)).max(1.0);
                            let pitch_fraction = f32::from(highest_pitch - note.pitch) / pitch_span;
                            let preview_height = 6.0_f32.min((note_area.height() - 4.0).max(1.0));
                            let note_y = top
                                + 4.0
                                + pitch_fraction
                                    * (note_area.height() - 4.0 - preview_height).max(0.0);
                            let note_rect = Rect::from_min_size(
                                Pos2::new(note_x, note_y),
                                Vec2::new(note_width, preview_height),
                            );
                            note_painter.rect_filled(
                                note_rect,
                                egui::CornerRadius::same(1),
                                Color32::from_rgb(242, 245, 250),
                            );
                            note_painter.rect_stroke(
                                note_rect,
                                egui::CornerRadius::same(1),
                                Stroke::new(0.75, part_color),
                                egui::StrokeKind::Inside,
                            );
                        }
                        grid_painter
                            .with_clip_rect(part_name_band.shrink(2.0))
                            .text(
                                Pos2::new(part_name_band.left() + 4.0, part_name_band.center().y),
                                Align2::LEFT_CENTER,
                                &part.name,
                                FontId::proportional(theme::type_size::LABEL),
                                if selected {
                                    Color32::WHITE
                                } else {
                                    project_controls::label_ink(part_color)
                                },
                            );
                    }
                    if let Some(draw) = *midi_part_draw
                        && draw.track_id == track.id
                    {
                        let start = draw.start_beats.min(draw.end_beats);
                        let end = draw
                            .start_beats
                            .max(draw.end_beats)
                            .max(start + snap_grid_beats.unwrap_or(1.0));
                        let preview_rect = Rect::from_min_max(
                            Pos2::new(
                                grid_left + (start / beats_per_bar_f64) as f32 * bar_width,
                                top + 6.0,
                            ),
                            Pos2::new(
                                grid_left + (end / beats_per_bar_f64) as f32 * bar_width,
                                top + media_lane_height - 6.0,
                            ),
                        );
                        grid_painter.rect_filled(
                            preview_rect.intersect(grid_clip),
                            egui::CornerRadius::same(4),
                            project_controls::track_color(track).gamma_multiply(0.35),
                        );
                        grid_painter.rect_stroke(
                            preview_rect.intersect(grid_clip),
                            egui::CornerRadius::same(4),
                            Stroke::new(1.5, theme::palette(ui).accent),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }

            if let Some((track_id, part_id, start, duration)) = pending_midi_part_edit.take()
                && let Some(part) = project
                    .tracks
                    .iter_mut()
                    .find(|track| track.id == track_id)
                    .and_then(|track| track.midi_parts.iter_mut().find(|part| part.id == part_id))
            {
                modified |= (part.start_beats - start).abs() > f64::EPSILON
                    || (part.duration_beats - duration).abs() > f64::EPSILON;
                part.start_beats = start;
                part.duration_beats = duration;
            }

            if let Some((track_id, part_id)) = pending_midi_part_remove.take()
                && let Some(track) = project.tracks.iter_mut().find(|track| track.id == track_id)
            {
                let before = track.midi_parts.len();
                track.midi_parts.retain(|part| part.id != part_id);
                modified |= track.midi_parts.len() != before;
                if project.selected_midi_part_id == Some(part_id) {
                    project.selected_midi_part_id = None;
                    *midi_editor_open = false;
                }
                arrange_view_state.midi_part_drag = None;
            }

            if let Some((track_id, part_id, beat)) = pending_midi_part_split.take()
                && let Some(track_index) =
                    project.tracks.iter().position(|track| track.id == track_id)
                && let Some(new_part_id) = split_midi_part(project, track_index, part_id, beat)
            {
                project.selected_track = Some(track_index as u32);
                project.selected_midi_part_id = Some(new_part_id);
                arrange_view_state.selected_track_ids.clear();
                arrange_view_state.selected_track_ids.insert(track_id);
                arrange_view_state.selection_anchor_track_id = Some(track_id);
                modified = true;
            }

            if let Some((track_id, first, last)) = pending_midi_part.take()
                && let Some(track_index) =
                    project.tracks.iter().position(|track| track.id == track_id)
            {
                let start = first.min(last).max(0.0);
                let duration = (first.max(last) - start).max(snap_grid_beats.unwrap_or(1.0));
                let previous_track = project.selected_track;
                let previous_part = project.selected_midi_part_id;
                if project
                    .add_midi_part(track_index, start, duration)
                    .is_some()
                {
                    project.selected_track = previous_track;
                    project.selected_midi_part_id = previous_part;
                    modified = true;
                }
            }

            let event_lanes = project
                .tracks
                .iter()
                .enumerate()
                .map(|(row, t)| {
                    let (take_lane_count, take_lane_indices) =
                        audio_events::take_lane_layout(t, project.tempo_bpm);
                    let take_lanes_visible = arrange_view_state.take_lanes_visible.contains(&t.id);
                    let base_height = arrange_view_state.track_height(t.id);
                    let hidden_by_group = track_hidden_by_collapsed_group(project, t.id);
                    let media_height = base_height
                        * if take_lanes_visible {
                            take_lane_count as f32
                        } else {
                            1.0
                        };
                    audio_events::Lane {
                        id: t.id,
                        channels: t.audio_channels,
                        audio: (t.kind == TrackKind::Audio && !hidden_by_group)
                            || (t.kind == TrackKind::Group
                                && t.group_collapsed
                                && !hidden_by_group),
                        rect: Rect::from_min_size(
                            Pos2::new(grid_left, canvas.top() + ruler_height + row_offsets[row]),
                            Vec2::new(
                                grid_clip.width(),
                                if hidden_by_group { 0.0 } else { media_height },
                            ),
                        ),
                        take_lanes_visible,
                        take_lane_count,
                        take_lane_indices,
                        automation: arrange_view_state.automation_visible.contains(&t.id).then(
                            || {
                                let top =
                                    canvas.top() + ruler_height + row_offsets[row] + media_height;
                                Rect::from_min_max(
                                    Pos2::new(grid_left, top),
                                    Pos2::new(
                                        grid_clip.right(),
                                        canvas.top()
                                            + ruler_height
                                            + row_offsets[row]
                                            + row_heights[row],
                                    ),
                                )
                            },
                        ),
                        color: project_controls::track_color(t),
                    }
                })
                .collect::<Vec<_>>();
            let outcome = audio_events::show_with_tool(
                ui,
                project,
                audio_sources,
                &event_lanes,
                grid_clip,
                canvas.min.x + lane_header_width,
                bar_width / beats_per_bar as f32,
                snap_grid_beats,
                arrange_view_state.track_scroll_y,
                &mut arrange_view_state.audio_events,
                can_remove,
                edit_tool,
            );
            modified |= outcome.changed;
            if let Some((group_id, _)) = arrange_view_state.audio_events.selected_group_segment()
                && let Some(group_index) =
                    project.tracks.iter().position(|track| track.id == group_id)
            {
                project.selected_track = Some(group_index as u32);
                project.selected_midi_part_id = None;
                arrange_view_state.selected_track_ids.clear();
                arrange_view_state.selected_track_ids.insert(group_id);
                arrange_view_state.selection_anchor_track_id = Some(group_id);
            }
            let group_duration_seconds =
                content_bars as f64 * beats_per_bar_f64 * 60.0 / project.tempo_bpm.max(1.0);
            if group_duration_seconds.is_finite() && group_duration_seconds > 0.0 {
                for (row, group) in
                    project.tracks.iter().enumerate().filter(|(_, track)| {
                        track.kind == TrackKind::Group && track.group_collapsed
                    })
                {
                    if row_heights[row] <= 0.0 {
                        continue;
                    }
                    let children_have_audio = project.tracks.iter().any(|track| {
                        track.kind == TrackKind::Audio
                            && project_track_is_in_group(project, track.id, group.id)
                            && !track.clips.is_empty()
                    });
                    if !children_have_audio {
                        continue;
                    }
                    let group_segments = audio_events::group_audio_segments(project, group.id);
                    let group_preview = arrange_view_state
                        .audio_events
                        .group_segment_preview(project.tempo_bpm)
                        .filter(|preview| preview.group_id == group.id);
                    let Some((group_start_beats, _)) = group_segments.first().copied() else {
                        continue;
                    };
                    let Some((_, group_end_beats)) = group_segments.last().copied() else {
                        continue;
                    };
                    let top = canvas.top() + ruler_height + row_offsets[row];
                    let lane_height = arrange_view_state.track_height(group.id);
                    let waveform_rect = Rect::from_min_max(
                        Pos2::new(grid_left, top + 3.0),
                        Pos2::new(
                            grid_left + total_size.x - lane_header_width,
                            top + (lane_height - 3.0).max(3.0),
                        ),
                    );
                    let visible = waveform_rect.intersect(grid_clip);
                    if visible.width() <= 0.0 || waveform_rect.width() <= 0.0 {
                        continue;
                    }
                    let start_seconds = ((visible.left() - waveform_rect.left())
                        / waveform_rect.width()) as f64
                        * group_duration_seconds;
                    let end_seconds = ((visible.right() - waveform_rect.left())
                        / waveform_rect.width()) as f64
                        * group_duration_seconds;
                    let count = (visible.width() * ui.ctx().pixels_per_point())
                        .ceil()
                        .clamp(2.0, 4096.0) as usize;
                    let channels = usize::from(group.audio_channels.clamp(1, 2));
                    let mut columns = group_waveform_columns(
                        project,
                        group.id,
                        audio_sources,
                        start_seconds,
                        end_seconds,
                        count,
                        channels,
                    );
                    if let Some(preview) = &group_preview {
                        let seconds_per_beat = 60.0 / project.tempo_bpm.max(1.0);
                        for (index, column) in columns.iter_mut().enumerate() {
                            let beat = (start_seconds
                                + (index as f64 + 0.5) / count as f64
                                    * (end_seconds - start_seconds))
                                / seconds_per_beat;
                            if beat >= preview.original_bounds.0 - 1e-6
                                && beat < preview.original_bounds.1 - 1e-6
                            {
                                *column = [[0.0; 2]; 2];
                            }
                        }
                    }
                    let track_color = if arrange_view_state.show_track_colors {
                        project_controls::track_color(group)
                    } else {
                        theme::palette(ui).muted
                    };
                    let event_rect = Rect::from_min_max(
                        Pos2::new(
                            waveform_rect.left()
                                + (group_start_beats * 60.0
                                    / project.tempo_bpm.max(1.0)
                                    / group_duration_seconds)
                                    as f32
                                    * waveform_rect.width(),
                            waveform_rect.top(),
                        ),
                        Pos2::new(
                            waveform_rect.left()
                                + (group_end_beats * 60.0
                                    / project.tempo_bpm.max(1.0)
                                    / group_duration_seconds)
                                    as f32
                                    * waveform_rect.width(),
                            waveform_rect.bottom(),
                        ),
                    );
                    grid_painter.rect_filled(event_rect.intersect(grid_clip), 0.0, track_color);
                    grid_painter.rect_filled(
                        Rect::from_min_max(
                            Pos2::new(
                                event_rect.left(),
                                (event_rect.bottom() - 13.0).max(event_rect.top()),
                            ),
                            event_rect.max,
                        )
                        .intersect(grid_clip),
                        0.0,
                        track_color.gamma_multiply(0.8),
                    );
                    waveform_view::draw_colored(
                        &grid_painter,
                        waveform_rect,
                        grid_clip,
                        channels,
                        group_duration_seconds,
                        if ui.visuals().dark_mode {
                            Color32::from_gray(230)
                        } else {
                            Color32::from_gray(28)
                        },
                        |_start, _end, _count, channel| {
                            columns
                                .iter()
                                .map(|column| column[channel.min(1)])
                                .collect()
                        },
                    );
                    if let Some(preview) = &group_preview {
                        let seconds_per_beat = 60.0 / project.tempo_bpm.max(1.0);
                        let beat_to_x = |beat: f64| {
                            waveform_rect.left()
                                + (beat * seconds_per_beat / group_duration_seconds) as f32
                                    * waveform_rect.width()
                        };
                        let preview_columns =
                            |clips: &[(u64, opentrack_core::project::AudioClip)],
                             bounds: (f64, f64)| {
                                let mut preview_project = project.clone();
                                for track in &mut preview_project.tracks {
                                    let replacements = clips
                                        .iter()
                                        .filter(|(track_id, _)| *track_id == track.id)
                                        .map(|(_, clip)| (clip.id, clip.clone()))
                                        .collect::<std::collections::HashMap<_, _>>();
                                    for clip in &mut track.clips {
                                        if let Some(replacement) = replacements.get(&clip.id) {
                                            *clip = replacement.clone();
                                        }
                                    }
                                }
                                let start = (bounds.0 * seconds_per_beat).max(0.0);
                                let end = (bounds.1 * seconds_per_beat).max(start);
                                let width = (beat_to_x(bounds.1) - beat_to_x(bounds.0)).max(1.0);
                                let preview_count = (width * ui.ctx().pixels_per_point())
                                    .ceil()
                                    .clamp(2.0, 4096.0)
                                    as usize;
                                group_waveform_columns(
                                    &preview_project,
                                    group.id,
                                    audio_sources,
                                    start,
                                    end,
                                    preview_count,
                                    channels,
                                )
                            };
                        let original_rect = Rect::from_min_max(
                            Pos2::new(beat_to_x(preview.original_bounds.0), waveform_rect.top()),
                            Pos2::new(beat_to_x(preview.original_bounds.1), waveform_rect.bottom()),
                        );
                        let original_visible = original_rect.intersect(visible);
                        if original_visible.is_positive() {
                            let ghost_columns =
                                preview_columns(&preview.original_clips, preview.original_bounds);
                            let ghost_color = track_color.gamma_multiply(0.32);
                            grid_painter.rect_filled(original_visible, 0.0, ghost_color);
                            grid_painter.rect_filled(
                                Rect::from_min_max(
                                    Pos2::new(
                                        original_rect.left(),
                                        (original_rect.bottom() - 13.0).max(original_rect.top()),
                                    ),
                                    original_rect.max,
                                )
                                .intersect(visible),
                                0.0,
                                track_color.gamma_multiply(0.24),
                            );
                            waveform_view::draw_colored(
                                &grid_painter,
                                original_rect,
                                visible,
                                channels,
                                (preview.original_bounds.1 - preview.original_bounds.0)
                                    * seconds_per_beat,
                                if ui.visuals().dark_mode {
                                    Color32::from_gray(230).gamma_multiply(0.36)
                                } else {
                                    Color32::from_gray(28).gamma_multiply(0.36)
                                },
                                |_start, _end, _count, channel| {
                                    ghost_columns
                                        .iter()
                                        .map(|column| column[channel.min(1)])
                                        .collect()
                                },
                            );
                            grid_painter.rect_stroke(
                                original_visible,
                                0.0,
                                Stroke::new(1.0, track_color.gamma_multiply(0.55)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let candidate_rect = Rect::from_min_max(
                            Pos2::new(beat_to_x(preview.candidate_bounds.0), waveform_rect.top()),
                            Pos2::new(
                                beat_to_x(preview.candidate_bounds.1),
                                waveform_rect.bottom(),
                            ),
                        );
                        let candidate_visible = candidate_rect.intersect(visible);
                        if candidate_visible.is_positive() {
                            let live_columns =
                                preview_columns(&preview.candidate_clips, preview.candidate_bounds);
                            grid_painter.rect_filled(candidate_rect, 0.0, track_color);
                            grid_painter.rect_filled(
                                Rect::from_min_max(
                                    Pos2::new(
                                        candidate_rect.left(),
                                        (candidate_rect.bottom() - 13.0).max(candidate_rect.top()),
                                    ),
                                    candidate_rect.max,
                                )
                                .intersect(visible),
                                0.0,
                                track_color.gamma_multiply(0.8),
                            );
                            waveform_view::draw_colored(
                                &grid_painter,
                                candidate_rect,
                                visible,
                                channels,
                                (preview.candidate_bounds.1 - preview.candidate_bounds.0)
                                    * seconds_per_beat,
                                if ui.visuals().dark_mode {
                                    Color32::from_gray(230)
                                } else {
                                    Color32::from_gray(28)
                                },
                                |_start, _end, _count, channel| {
                                    live_columns
                                        .iter()
                                        .map(|column| column[channel.min(1)])
                                        .collect()
                                },
                            );
                            if let Some(proxy) = audio_events::group_segment_proxy_from_clips(
                                project,
                                group.id,
                                preview.candidate_bounds.0,
                                preview.candidate_bounds.1,
                                &preview.candidate_clips,
                            ) {
                                audio_events::paint_group_segment_controls(
                                    ui,
                                    &proxy,
                                    candidate_rect,
                                    visible,
                                    track_color,
                                );
                            }
                        }
                    }
                    // Only shared edit boundaries divide the group's virtual
                    // event. Independent child clip edits stay visually joined.
                    let seam = Color32::from_gray(32);
                    for (beat, _) in group_segments.iter().skip(1).copied() {
                        let x = waveform_rect.left()
                            + ((beat * 60.0 / project.tempo_bpm.max(1.0) / group_duration_seconds)
                                as f32)
                                * waveform_rect.width();
                        grid_painter.with_clip_rect(visible).line_segment(
                            [
                                Pos2::new(x, waveform_rect.top()),
                                Pos2::new(x, waveform_rect.bottom()),
                            ],
                            Stroke::new(1.5, seam),
                        );
                    }
                    if group_preview.is_none()
                        && let Some((selected_group_id, segment_index)) =
                            arrange_view_state.audio_events.selected_group_segment()
                        && selected_group_id == group.id
                        && let Some((start, end)) = group_segments.get(segment_index).copied()
                    {
                        let selected_rect = Rect::from_min_max(
                            Pos2::new(
                                waveform_rect.left()
                                    + (start * 60.0
                                        / project.tempo_bpm.max(1.0)
                                        / group_duration_seconds)
                                        as f32
                                        * waveform_rect.width(),
                                waveform_rect.top(),
                            ),
                            Pos2::new(
                                waveform_rect.left()
                                    + (end * 60.0
                                        / project.tempo_bpm.max(1.0)
                                        / group_duration_seconds)
                                        as f32
                                        * waveform_rect.width(),
                                waveform_rect.bottom(),
                            ),
                        );
                        if let Some(proxy) =
                            audio_events::group_segment_proxy(project, group.id, start, end)
                        {
                            audio_events::paint_group_segment_controls(
                                ui,
                                &proxy,
                                selected_rect,
                                visible,
                                track_color,
                            );
                        }
                    }
                }
            }
            // Paint the live take after arranged clips so it remains the
            // visible top layer while recording over existing material.
            if let Some(preview) = recording_preview {
                for (track_id, route) in &preview.tracks {
                    let Some(lane) = event_lanes.iter().find(|lane| lane.id == *track_id) else {
                        continue;
                    };
                    let duration = preview.waveform.duration_seconds();
                    let left = canvas.min.x
                        + lane_header_width
                        + (preview.start_beats / beats_per_bar_f64) as f32 * bar_width;
                    let width = (duration * project.tempo_bpm / 60.0 / beats_per_bar_f64) as f32
                        * bar_width;
                    let rect = Rect::from_min_size(
                        Pos2::new(left, lane.rect.top()),
                        Vec2::new(width.max(1.0), lane.rect.height()),
                    );
                    grid_painter.rect_filled(rect, 2.0, theme::RECORD.gamma_multiply(0.24));
                    grid_painter.rect_stroke(
                        rect,
                        2.0,
                        Stroke::new(1.0, theme::RECORD),
                        egui::StrokeKind::Inside,
                    );
                    let channels = if *route == AudioRecordRoute::Stereo {
                        preview.waveform.channels as usize
                    } else {
                        1
                    };
                    waveform_view::draw(
                        &grid_painter,
                        rect,
                        grid_clip,
                        channels,
                        duration,
                        |a, b, n, channel| {
                            preview.waveform.columns(
                                a,
                                b,
                                n,
                                if *route == AudioRecordRoute::Right {
                                    1
                                } else {
                                    channel
                                },
                            )
                        },
                    );
                    grid_painter.with_clip_rect(rect.intersect(grid_clip)).text(
                        rect.left_top() + Vec2::new(4.0, 2.0),
                        Align2::LEFT_TOP,
                        "Recording...",
                        FontId::proportional(10.0),
                        theme::palette(ui).text,
                    );
                }
            }
            if outcome.scroll != 0.0 {
                arrange_view_state.timeline_scroll_x =
                    (arrange_view_state.timeline_scroll_x + outcome.scroll).max(0.0);
                custom_scroll_offset = Some(Vec2::new(
                    arrange_view_state.timeline_scroll_x,
                    arrange_view_state.track_scroll_y,
                ));
            }

            if let Some(insertion) = reorder_indicator {
                let y = if insertion >= visible_rows {
                    canvas.min.y + ruler_height + tracks_height
                } else {
                    canvas.min.y + ruler_height + row_offsets[insertion]
                };
                painter.line_segment(
                    [Pos2::new(header_left, y), Pos2::new(viewport.right(), y)],
                    Stroke::new(2.0, palette.accent),
                );
            }
            if let Some(group_row) = group_drop_indicator {
                let y =
                    canvas.min.y + ruler_height + row_offsets[group_row] + row_heights[group_row]
                        - 2.0;
                painter.line_segment(
                    [
                        Pos2::new(header_left + lane_header_width * 0.5, y),
                        Pos2::new(header_left + lane_header_width, y),
                    ],
                    Stroke::new(3.0, palette.accent),
                );
            }

            if project.tracks.is_empty() {
                grid_painter.text(
                    Pos2::new(
                        canvas.min.x + lane_header_width + 18.0,
                        canvas.min.y + ruler_height + 22.0,
                    ),
                    Align2::LEFT_CENTER,
                    "Add a track to begin arranging",
                    FontId::proportional(theme::type_size::BODY),
                    theme::palette(ui).muted,
                );
            }

            // A quiet overlay covers the selected time interval through all
            // track lanes and empty canvas, but never the pinned headers.
            grid_painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(range_start_x, grid_clip.top()),
                    Pos2::new(range_end_x, grid_clip.bottom()),
                ),
                0.0,
                range_color.gamma_multiply(if cycle_enabled || range_preview.is_some() {
                    0.10
                } else {
                    0.05
                }),
            );
            let bar_position = (*position_beats / f64::from(beats_per_bar)) as f32;
            let playhead_x = canvas.min.x + lane_header_width + bar_position * bar_width;
            grid_painter.line_segment(
                [
                    Pos2::new(playhead_x, canvas.min.y),
                    Pos2::new(playhead_x, viewport.bottom()),
                ],
                Stroke::new(1.0, theme::PLAYHEAD),
            );
            if pan_active
                && ui.input(|input| input.pointer.button_down(egui::PointerButton::Middle))
            {
                crate::timeline::show_move_cursor(ui, true);
            }
            if primary_released {
                let release_group_id = pointer
                    .and_then(|pointer| {
                        (0..visible_rows).find(|row| {
                            row_heights[*row] > 0.0
                                && pointer.y >= canvas.min.y + ruler_height + row_offsets[*row]
                                && pointer.y
                                    < canvas.min.y
                                        + ruler_height
                                        + row_offsets[*row]
                                        + row_heights[*row]
                        })
                    })
                    .and_then(|row| track_ids.get(row).copied())
                    .filter(|id| group_track_ids.contains(id));
                if let (Some(group_id), Some(dragged_ids)) = (
                    release_group_id,
                    arrange_view_state.pending_track_drag_ids.take(),
                ) {
                    let dragged_ids = dragged_ids
                        .into_iter()
                        .filter(|id| *id != group_id)
                        .collect::<Vec<_>>();
                    if !dragged_ids.is_empty() {
                        group_drop_after_draw = Some((group_id, dragged_ids));
                        reorder_after_draw = None;
                    }
                } else if group_drop_after_draw.is_none() {
                    group_drop_after_draw = arrange_view_state.pending_group_drop.take();
                    if group_drop_after_draw.is_some() {
                        reorder_after_draw = None;
                    }
                } else {
                    arrange_view_state.pending_track_drag_ids = None;
                }
                arrange_view_state.pending_track_drag_ids = None;
                arrange_view_state.pending_group_drop = None;
            }
        });
    if let Some(offset) = custom_scroll_offset {
        arrange_view_state.timeline_scroll_x = offset.x.max(0.0);
        arrange_view_state.track_scroll_y = offset.y.max(0.0);
    } else {
        arrange_view_state.timeline_scroll_x = scroll_output.state.offset.x.max(0.0);
        arrange_view_state.track_scroll_y = scroll_output.state.offset.y.max(0.0);
    }
    let track_scroll = Rect::from_min_max(
        Pos2::new(scroll_content_right, scroll_parent.top() + ruler_height),
        Pos2::new(scroll_parent.right(), scroll_parent.bottom() - 16.0),
    );
    let track_zoom_rect = Rect::from_min_max(
        Pos2::new(
            scroll_parent.right() - 19.0,
            scroll_parent.bottom() - track_zoom_height - 16.0,
        ),
        Pos2::new(scroll_parent.right() - 1.0, scroll_parent.bottom() - 16.0),
    );
    let previous_track_zoom = arrange_view_state.track_vertical_zoom;
    project_controls::track_zoom_control(
        ui,
        track_zoom_rect,
        &mut arrange_view_state.track_vertical_zoom,
    );
    let _ = track_scroll;
    if (arrange_view_state.track_vertical_zoom - previous_track_zoom).abs() > f32::EPSILON {
        arrange_view_state.track_scroll_y *=
            arrange_view_state.track_vertical_zoom / previous_track_zoom.max(0.001);
    }
    if clear_all_muted {
        for track in &mut project.tracks {
            if track.muted {
                track.muted = false;
                modified = true;
            }
        }
    }
    if clear_all_solo {
        for track in &mut project.tracks {
            if track.solo {
                track.solo = false;
                modified = true;
            }
        }
    }
    if let Some((track_ids, insertion_index)) = reorder_after_draw {
        modified |= project.move_tracks_to(&track_ids, insertion_index);
    }
    if let Some((group_id, track_ids)) = group_drop_after_draw {
        modified |= project.assign_tracks_to_group(group_id, &track_ids);
    }
    if let Some(solo_track_id) = exclusive_solo_track_id {
        for track in &mut project.tracks {
            if track.id != solo_track_id && track.solo {
                track.solo = false;
            }
        }
    }
    ProjectViewChanges {
        modified: modified || inspector_changes.modified,
        live_mix: (!modified && !inspector_changes.requires_plan_rebuild)
            .then_some(inspector_changes.live_mix)
            .flatten(),
        defer_audio_plan: arrange_view_state.audio_events.edit_drag_active(),
    }
}

fn ruler_label(
    bar_index: usize,
    format: u8,
    beats_per_bar: u8,
    tempo_bpm: f64,
    sample_rate: u32,
) -> String {
    let beats = bar_index as f64 * f64::from(beats_per_bar.max(1));
    let seconds = beats * 60.0 / tempo_bpm.max(1.0);
    match format {
        1 => {
            let whole = seconds.floor().max(0.0) as u64;
            format!("{:02}:{:02}", whole / 60, whole % 60)
        }
        2 => format_timecode(seconds, 30),
        3 => format!(
            "{}",
            (seconds.max(0.0) * f64::from(sample_rate)).round() as u64
        ),
        4 => format_timecode(seconds, 60),
        _ => (bar_index + 1).to_string(),
    }
}

fn format_timecode(seconds: f64, frames_per_second: u32) -> String {
    let fps = u64::from(frames_per_second.max(1));
    let frames = (seconds.max(0.0) * fps as f64).round() as u64;
    let whole_seconds = frames / fps;
    let frame = frames % fps;
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        whole_seconds / 3600,
        (whole_seconds / 60) % 60,
        whole_seconds % 60,
        frame
    )
}

fn select_arrange_track(
    selected_track: &mut Option<u32>,
    selected_midi_part_id: &mut Option<u64>,
    track_ids: &[u64],
    arrange_view_state: &mut ArrangeViewState,
    row: usize,
    modifiers: egui::Modifiers,
) {
    let Some(track_id) = track_ids.get(row).copied() else {
        return;
    };

    if modifiers.shift {
        let anchor = arrange_view_state
            .selection_anchor_track_id
            .and_then(|anchor| track_ids.iter().position(|id| *id == anchor))
            .unwrap_or(row);
        if !modifiers.ctrl {
            arrange_view_state.selected_track_ids.clear();
        }
        for index in anchor.min(row)..=anchor.max(row) {
            if let Some(id) = track_ids.get(index) {
                arrange_view_state.selected_track_ids.insert(*id);
            }
        }
    } else if modifiers.ctrl {
        if !arrange_view_state.selected_track_ids.remove(&track_id) {
            arrange_view_state.selected_track_ids.insert(track_id);
        }
        if arrange_view_state.selected_track_ids.is_empty() {
            arrange_view_state.selected_track_ids.insert(track_id);
        }
        arrange_view_state.selection_anchor_track_id = Some(track_id);
    } else {
        arrange_view_state.selected_track_ids.clear();
        arrange_view_state.selected_track_ids.insert(track_id);
        arrange_view_state.selection_anchor_track_id = Some(track_id);
    }

    *selected_track = if arrange_view_state.selected_track_ids.contains(&track_id) {
        u32::try_from(row).ok()
    } else {
        arrange_view_state
            .selected_track_ids
            .iter()
            .find_map(|selected| {
                track_ids
                    .iter()
                    .position(|id| id == selected)
                    .and_then(|index| u32::try_from(index).ok())
            })
    };
    *selected_midi_part_id = None;
}

fn split_midi_part(
    project: &mut Project,
    track_index: usize,
    part_id: u64,
    beat: f64,
) -> Option<u64> {
    let part_index = project
        .tracks
        .get(track_index)?
        .midi_parts
        .iter()
        .position(|part| part.id == part_id)?;
    let original = project.tracks[track_index].midi_parts[part_index].clone();
    let split_offset = beat - original.start_beats;
    if !split_offset.is_finite()
        || split_offset < 0.0625
        || split_offset > original.duration_beats - 0.0625
    {
        return None;
    }

    {
        let left = &mut project.tracks[track_index].midi_parts[part_index];
        left.duration_beats = split_offset;
        left.notes.retain_mut(|note| {
            if note.start_beats >= split_offset {
                return false;
            }
            note.duration_beats = note.duration_beats.min(split_offset - note.start_beats);
            note.duration_beats > 0.0
        });
        left.controller_events
            .retain(|event| event.position_beats < split_offset);
    }

    let new_part_id = project.add_midi_part(
        track_index,
        original.start_beats + split_offset,
        original.duration_beats - split_offset,
    )?;
    {
        let new_part = project.tracks[track_index]
            .midi_parts
            .iter_mut()
            .find(|part| part.id == new_part_id)?;
        new_part.name = format!("{} (split)", original.name);
    }

    for note in original.notes {
        let end = note.start_beats + note.duration_beats;
        if end <= split_offset {
            continue;
        }
        let start = (note.start_beats - split_offset).max(0.0);
        let duration = end.min(original.duration_beats) - split_offset.max(note.start_beats);
        if duration > 0.0 {
            let _ = project.add_midi_note_on_channel(
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
    for event in original.controller_events {
        if event.position_beats >= split_offset {
            let _ = project.add_midi_controller_event_on_channel(
                track_index,
                new_part_id,
                event.position_beats - split_offset,
                event.controller,
                event.channel,
                event.value,
            );
        }
    }
    Some(new_part_id)
}

fn show_midi_editor(
    ui: &mut egui::Ui,
    project: &mut Project,
    track_index: usize,
    part_id: u64,
    position_quantize_beats: &mut f64,
    length_quantize_beats: &mut f64,
    insert_velocity: &mut u8,
    controller_number: &mut u8,
    live_velocity_update: &mut Option<(usize, u64, u8)>,
    selected_note_id: &mut Option<u64>,
    selected_note_ids: &mut BTreeSet<u64>,
    note_drag: &mut Option<MidiNoteDrag>,
    note_draw: &mut Option<MidiNoteDraw>,
    controller_drag: &mut Option<MidiControllerDrag>,
    tool: &mut EditTool,
    transport_position_beats: &mut f64,
    pitch_view: &mut sample_editor::PitchViewport,
) -> bool {
    crate::view_controls::apply_solid_scrollbars(ui);
    let Some(track) = project.tracks.get(track_index) else {
        return false;
    };
    if !matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) {
        return false;
    }
    let Some(part_index) = track.midi_parts.iter().position(|part| part.id == part_id) else {
        return false;
    };
    let track_id = track.id;
    let note_selection_drag_id =
        ui.make_persistent_id(("midi-note-selection-drag", track_id, part_id));
    let mut selection_drag =
        ui.data(|data| data.get_temp::<MidiNoteSelectionDrag>(note_selection_drag_id));
    let track_name = track.name.clone();
    let track_color = project_controls::track_color(track);
    let part_name = track.midi_parts[part_index].name.clone();
    let part_start_beats = track.midi_parts[part_index].start_beats;
    let part_duration_beats = track.midi_parts[part_index].duration_beats;
    let note_count = track.midi_parts[part_index].notes.len();
    let visible_beats = track.midi_parts[part_index]
        .notes
        .iter()
        .map(|note| note.start_beats + note.duration_beats)
        .fold(part_duration_beats, f64::max)
        .ceil()
        .clamp(1.0, 4_096.0) as usize;
    let selected_note_index = track.midi_parts[part_index]
        .notes
        .iter()
        .position(|note| Some(note.id) == *selected_note_id);
    selected_note_ids.retain(|id| {
        track.midi_parts[part_index]
            .notes
            .iter()
            .any(|note| note.id == *id)
    });
    if selected_note_index.is_none() {
        *selected_note_id = None;
    }
    if note_drag
        .as_ref()
        .is_some_and(|drag| drag.track_id != track_id || drag.part_id != part_id)
    {
        *note_drag = None;
    }
    if note_draw.is_some_and(|draw| draw.track_id != track_id || draw.part_id != part_id) {
        *note_draw = None;
    }
    if selection_drag.is_some_and(|drag| drag.track_id != track_id || drag.part_id != part_id) {
        selection_drag = None;
    }
    if controller_drag.is_some_and(|drag| drag.track_id != track_id || drag.part_id != part_id) {
        *controller_drag = None;
    }
    let mut modified = false;
    let position_quantize = if position_quantize_beats.is_finite() && *position_quantize_beats > 0.0
    {
        *position_quantize_beats
    } else {
        1.0
    };
    let insert_length = if length_quantize_beats.is_finite() && *length_quantize_beats > 0.0 {
        *length_quantize_beats
    } else {
        1.0
    };

    let bounds = ui.available_rect_before_wrap();
    let toolbar = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
    let info = Rect::from_min_size(
        Pos2::new(bounds.left(), toolbar.bottom() + 2.0),
        Vec2::new(bounds.width(), 38.0),
    );
    const INSPECTOR_WIDTH: f32 = 232.0;
    const KEYBOARD_WIDTH: f32 = 64.0;
    const RULER_HEIGHT: f32 = 22.0;
    let zoom_id = ui.id().with(("midi-time-zoom", track_id, part_id));
    let beat_width = ui.data(|d| d.get_temp::<f32>(zoom_id)).unwrap_or(44.0);
    const CONTROLLER_DEFAULT_HEIGHT: f32 = 126.0;
    const CONTROLLER_MIN_HEIGHT: f32 = 96.0;
    const CONTROLLER_PLOT_TOP: f32 = 24.0;
    const CONTROLLER_BOTTOM_MARGIN: f32 = 14.0;
    let overview = Rect::from_min_size(
        Pos2::new(bounds.left(), info.bottom() + 3.0),
        Vec2::new(bounds.width(), 28.0),
    );
    let piano_view = Rect::from_min_max(
        Pos2::new(bounds.left() + 232.0 + 5.0 + 64.0, overview.bottom() + 2.0),
        Pos2::new(bounds.right() - 20.0, bounds.bottom()),
    );
    let controller_height_id = ui.id().with(("midi-controller-height", track_id, part_id));
    let controller_resize_active_id =
        ui.id()
            .with(("midi-controller-resize-active", track_id, part_id));
    let max_controller_height = (piano_view.height() - RULER_HEIGHT - 72.0)
        .max(CONTROLLER_MIN_HEIGHT)
        .min(480.0);
    let mut controller_lane_height = ui
        .data(|d| d.get_temp::<f32>(controller_height_id))
        .unwrap_or(CONTROLLER_DEFAULT_HEIGHT)
        .clamp(CONTROLLER_MIN_HEIGHT, max_controller_height);
    let controller_plot_height =
        (controller_lane_height - CONTROLLER_PLOT_TOP - CONTROLLER_BOTTOM_MARGIN).max(52.0);
    let palette = theme::palette(ui);
    let mut fit_part = false;
    ui.painter()
        .rect_filled(toolbar, 0.0, palette.surface_raised);
    ui.painter().rect_filled(info, 0.0, palette.surface);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(toolbar.shrink2(Vec2::new(4.0, 2.0))),
        |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
            ui.horizontal(|ui| {
                sample_editor::toolbar_tool(
                    ui,
                    tool,
                    EditTool::Select,
                    ToolbarIcon::Select,
                    "Object Selection",
                );
                sample_editor::toolbar_tool(
                    ui,
                    tool,
                    EditTool::Split,
                    ToolbarIcon::Split,
                    "Scissors",
                );
                sample_editor::toolbar_tool(
                    ui,
                    tool,
                    EditTool::Draw,
                    ToolbarIcon::Draw,
                    "Draw Notes",
                );
                fit_part =
                    toolbar_icon_button(ui, ToolbarIcon::Zoom, "Fit Part", false, true).clicked();
                ui.add_space(9.0);
                ui.label("Bars + Beats");
                ui.label(format!(
                    "Tempo {:.2}   {}/{}",
                    project.tempo_bpm, project.beats_per_bar, project.beat_unit
                ));
                ui.add_space(8.0);
                let mut snap_enabled = true;
                ui.checkbox(&mut snap_enabled, "Snap");
                egui::ComboBox::from_id_salt(("midi-grid", track_id, part_id))
                    .width(88.0)
                    .selected_text(midi_quantize_label(*position_quantize_beats))
                    .show_ui(ui, |ui| {
                        for (beats, label) in MIDI_QUANTIZE_VALUES {
                            ui.selectable_value(position_quantize_beats, beats, label);
                        }
                    });
                egui::ComboBox::from_id_salt(("midi-length", track_id, part_id))
                    .width(70.0)
                    .selected_text(midi_quantize_label(*length_quantize_beats))
                    .show_ui(ui, |ui| {
                        for (beats, label) in MIDI_QUANTIZE_VALUES {
                            ui.selectable_value(length_quantize_beats, beats, label);
                        }
                    });
                ui.add_enabled(false, egui::Button::new("Split at Selection"));
                ui.label(format!(
                    "Cursor {}",
                    format_transport_position(
                        *transport_position_beats,
                        project.beats_per_bar,
                        project.beat_unit
                    )
                ));
            });
        },
    );
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(info.shrink2(Vec2::new(4.0, 0.0))),
        |ui| {
            let width = 106.0_f32.min((info.width() - 45.0) / 6.0);
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 0.0);
            ui.horizontal_top(|ui| {
                sample_editor::field(
                    ui,
                    width,
                    "Position (beats)",
                    &mut project.tracks[track_index].midi_parts[part_index].start_beats,
                    0.0..=1e8,
                    0.01,
                    true,
                );
                sample_editor::field(
                    ui,
                    width,
                    "Length (beats)",
                    &mut project.tracks[track_index].midi_parts[part_index].duration_beats,
                    0.0625..=1e8,
                    0.01,
                    true,
                );
                let mut pitch = selected_note_index
                    .map(|i| {
                        f64::from(project.tracks[track_index].midi_parts[part_index].notes[i].pitch)
                    })
                    .unwrap_or(0.0);
                sample_editor::field(
                    ui,
                    width,
                    "Pitch",
                    &mut pitch,
                    0.0..=127.0,
                    1.0,
                    selected_note_index.is_some(),
                );
                let mut velocity = selected_note_index
                    .map(|i| {
                        f64::from(
                            project.tracks[track_index].midi_parts[part_index].notes[i].velocity,
                        )
                    })
                    .unwrap_or(f64::from(*insert_velocity));
                if sample_editor::field(
                    ui,
                    width,
                    "Velocity",
                    &mut velocity,
                    1.0..=127.0,
                    1.0,
                    true,
                ) {
                    *insert_velocity = velocity.round() as u8;
                }
                sample_editor::field(
                    ui,
                    width,
                    "Note Length",
                    &mut *length_quantize_beats,
                    0.0625..=64.0,
                    0.01,
                    true,
                );
                let mut notes = note_count as f64;
                sample_editor::field(ui, width, "Notes", &mut notes, 0.0..=1e8, 1.0, false);
            });
        },
    );
    let overview_track = overview;
    let overview_response = ui.interact(
        overview_track,
        ui.id().with(("midi-overview", track_id, part_id)),
        Sense::click_and_drag(),
    );
    ui.painter()
        .rect_filled(overview, 0.0, track_color.gamma_multiply(0.2));
    let overview_part = Rect::from_min_max(
        Pos2::new(overview_track.left(), overview.top() + 3.0),
        Pos2::new(overview_track.right(), overview.bottom() - 3.0),
    );
    ui.painter()
        .rect_filled(overview_part, 0.0, palette.canvas_alternate);
    let beats_per_bar = usize::from(project.beats_per_bar.max(1));
    let overview_beats = part_duration_beats.max(0.0625) as f32;
    let part_left = overview_part.left() + (0.0 / overview_beats) * overview_part.width();
    let part_right = overview_part.left()
        + (part_duration_beats as f32 / overview_beats) * overview_part.width();
    let part_context_rect = Rect::from_min_max(
        Pos2::new(part_left, overview_part.top()),
        Pos2::new(
            part_right.min(overview_part.right()),
            overview_part.bottom(),
        ),
    );
    ui.painter()
        .rect_filled(part_context_rect, 0.0, track_color.gamma_multiply(0.16));
    ui.painter().rect_stroke(
        part_context_rect,
        0.0,
        Stroke::new(0.75, track_color.gamma_multiply(0.7)),
        egui::StrokeKind::Inside,
    );
    for beat in 0..=overview_beats.ceil() as usize {
        if beat % beats_per_bar == 0 {
            let x = overview_part.left() + beat as f32 / overview_beats * overview_part.width();
            ui.painter().line_segment(
                [
                    Pos2::new(x, overview_part.top()),
                    Pos2::new(x, overview_part.bottom()),
                ],
                Stroke::new(0.5, palette.grid.gamma_multiply(0.7)),
            );
        }
    }
    let notes = &project.tracks[track_index].midi_parts[part_index].notes;
    let (lowest_pitch, highest_pitch) = notes.iter().fold((127_u8, 0_u8), |(low, high), note| {
        (low.min(note.pitch), high.max(note.pitch))
    });
    let pitch_span = f32::from(highest_pitch.saturating_sub(lowest_pitch)).max(1.0);
    let note_height = 5.0_f32.min(overview_part.height());
    let pitch_space = (overview_part.height() - note_height).max(0.0);
    let note_painter = ui.painter().with_clip_rect(part_context_rect);
    for note in notes {
        let left = overview_part.left()
            + (note.start_beats / overview_beats as f64) as f32 * overview_part.width();
        let right = overview_part.left()
            + ((note.start_beats + note.duration_beats) / overview_beats as f64) as f32
                * overview_part.width();
        let pitch_fraction = f32::from(highest_pitch - note.pitch) / pitch_span;
        let top = overview_part.top() + pitch_fraction * pitch_space;
        let note_rect = Rect::from_min_size(
            Pos2::new(left, top),
            Vec2::new((right - left).max(5.0), note_height),
        );
        note_painter.rect_filled(note_rect, 0.5, Color32::from_rgb(242, 245, 250));
        note_painter.rect_stroke(
            note_rect,
            0.5,
            Stroke::new(0.7, track_color),
            egui::StrokeKind::Inside,
        );
    }
    let bpb = beats_per_bar;
    pitch_view.sanitize();
    let wheel_action = crate::view_controls::wheel(ui, piano_view);
    match wheel_action {
        Some(crate::view_controls::WheelAction::VerticalPan(amount)) => pitch_view.scroll(amount),
        Some(crate::view_controls::WheelAction::VerticalZoom(amount)) => {
            pitch_view.zoom_scroll(amount)
        }
        _ => {}
    }
    let scrollbar_height = ui.spacing().scroll.bar_width
        + ui.spacing().scroll.bar_inner_margin
        + ui.spacing().scroll.bar_outer_margin;
    let note_area_height =
        (piano_view.height() - scrollbar_height - RULER_HEIGHT - controller_lane_height).max(72.0);
    let size = Vec2::new(
        KEYBOARD_WIDTH + (visible_beats as f32 * beat_width).max(piano_view.width()),
        RULER_HEIGHT + note_area_height,
    );
    let mut edit_action = None;
    let mut pitch_controls_rects = None;
    let mut overview_viewport_rect = None;
    let content_rect = Rect::from_min_max(Pos2::new(bounds.left(), piano_view.top()), bounds.max);
    ui.scope_builder(
        egui::UiBuilder::new()
            .layout(egui::Layout::top_down(egui::Align::Min))
            .max_rect(content_rect),
        |ui| {
            ui.set_clip_rect(content_rect);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(232.0);
                    egui::Frame::new()
                        .fill(palette.surface_raised)
                        .inner_margin(egui::Margin::same(7))
                        .show(ui, |ui| {
                            ui.set_min_height((content_rect.height() - 14.0).max(0.0));
                            let track_header = ui
                                .allocate_exact_size(Vec2::new(218.0, 22.0), Sense::hover())
                                .0;
                            ui.painter().rect_filled(track_header, 0.0, track_color);
                            ui.painter().with_clip_rect(track_header).text(
                                track_header.left_center() + Vec2::new(4.0, 0.0),
                                Align2::LEFT_CENTER,
                                &track_name,
                                FontId::proportional(11.0),
                                project_controls::label_ink(track_color),
                            );
                            ui.separator();
                            egui::CollapsingHeader::new("Part")
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.label(&part_name);
                                    ui.label(format!("{:.3} beats", part_duration_beats));
                                });
                            ui.separator();
                            if selected_note_index.is_none() {
                                ui.horizontal(|ui| {
                                    ui.scope(|ui| {
                                        ui.visuals_mut().widgets.inactive.bg_fill = palette.canvas;
                                        ui.add_sized(
                                            [100.0, 18.0],
                                            egui::Slider::new(insert_velocity, 1..=127)
                                                .show_value(false),
                                        );
                                    });
                                    ui.label(format!("{}  Velocity", *insert_velocity));
                                });
                            }
                            ui.separator();
                            if let Some(note_index) = selected_note_index {
                                let note = &mut project.tracks[track_index].midi_parts[part_index]
                                    .notes[note_index];
                                ui.label(format!("Selected {}", midi_pitch_label(note.pitch)));
                                let max_length = part_duration_beats - note.start_beats;
                                if max_length >= 0.0625 {
                                    modified |= ui
                                        .add(
                                            egui::Slider::new(
                                                &mut note.duration_beats,
                                                0.0625..=max_length,
                                            )
                                            .logarithmic(true)
                                            .text("Length"),
                                        )
                                        .changed();
                                }
                                let velocity_changed = ui
                                    .add(
                                        egui::Slider::new(&mut note.velocity, 1..=127)
                                            .text("Velocity"),
                                    )
                                    .changed();
                                if velocity_changed {
                                    *live_velocity_update =
                                        Some((track_index, note.id, note.velocity));
                                    modified = true;
                                }
                            } else {
                                ui.label("Select a note to edit its length and velocity.");
                            }
                            ui.separator();
                            ui.label(
                                egui::RichText::new("CONTROLLER")
                                    .small()
                                    .color(palette.muted),
                            );
                            egui::ComboBox::from_id_salt((
                                "midi-controller-lane",
                                track_id,
                                part_id,
                            ))
                            .selected_text(midi_controller_label(*controller_number))
                            .show_ui(ui, |ui| {
                                for (controller, label) in MIDI_CONTROLLER_VALUES {
                                    ui.selectable_value(controller_number, controller, label);
                                }
                            });
                            ui.label("Click to add a point; drag to edit; right-click to remove.");
                        });
                });
                let overview_scroll_id =
                    ui.make_persistent_id(("midi-piano-roll", track_id, part_id));
                let scroll_output = egui::ScrollArea::horizontal()
                    .id_salt(("midi-piano-roll", track_id, part_id))
                    .scroll_source(egui::scroll_area::ScrollSource {
                        mouse_wheel: false,
                        ..Default::default()
                    })
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .max_width((content_rect.width() - INSPECTOR_WIDTH - 5.0 - 20.0).max(0.0))
                    .max_height(piano_view.height())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let stacked_size = Vec2::new(size.x, size.y + controller_lane_height);
                        let (stack, _) = ui.allocate_exact_size(stacked_size, Sense::hover());
                        let canvas = Rect::from_min_size(stack.min, size);
                        let response = ui.interact(
                            canvas,
                            ui.id().with("midi-note-grid"),
                            Sense::click_and_drag(),
                        );
                        let painter = ui.painter_at(canvas);
                        painter.rect_filled(
                            canvas,
                            egui::CornerRadius::ZERO,
                            theme::palette(ui).canvas,
                        );
                        let ruler_rect =
                            Rect::from_min_size(canvas.min, Vec2::new(size.x, RULER_HEIGHT));
                        painter.rect_filled(
                            ruler_rect,
                            egui::CornerRadius::ZERO,
                            palette.surface_raised,
                        );
                        let ruler_response = ui.interact(
                            ruler_rect,
                            ui.id().with(("midi-time-ruler", track_id, part_id)),
                            Sense::click_and_drag(),
                        );

                        let pitch_area = Rect::from_min_max(
                            Pos2::new(canvas.left() + KEYBOARD_WIDTH, canvas.top() + RULER_HEIGHT),
                            canvas.max,
                        );
                        let keyboard_area = Rect::from_min_max(
                            Pos2::new(canvas.left(), pitch_area.top()),
                            Pos2::new(canvas.left() + KEYBOARD_WIDTH, pitch_area.bottom()),
                        );
                        pitch_controls_rects = Some((keyboard_area, pitch_area));
                        let row_height = pitch_area.height() / pitch_view.span;
                        for pitch in 0..=127_i32 {
                            let y = pitch_view.y(pitch as f32, pitch_area);
                            let row_rect = Rect::from_min_max(
                                Pos2::new(pitch_area.left(), y - row_height * 0.5),
                                Pos2::new(pitch_area.right(), y + row_height * 0.5),
                            )
                            .intersect(pitch_area);
                            if row_rect.is_positive() {
                                let key_rect = Rect::from_min_max(
                                    Pos2::new(keyboard_area.left(), row_rect.top()),
                                    Pos2::new(keyboard_area.right(), row_rect.bottom()),
                                );
                                sample_editor::paint_pitch_key_row(
                                    ui, key_rect, row_rect, pitch, row_height,
                                );
                            }
                        }

                        for beat in 0..=visible_beats {
                            let x = canvas.min.x + KEYBOARD_WIDTH + beat as f32 * beat_width;
                            let bar_line = beat % bpb == 0;
                            painter.line_segment(
                                [
                                    Pos2::new(x, pitch_area.top()),
                                    Pos2::new(x, pitch_area.bottom()),
                                ],
                                Stroke::new(
                                    if bar_line { 1.0 } else { 0.5 },
                                    if bar_line {
                                        theme::palette(ui).grid
                                    } else {
                                        theme::palette(ui).surface_raised
                                    },
                                ),
                            );
                            if bar_line && beat < visible_beats {
                                let ruler_position = format_transport_position(
                                    beat as f64,
                                    project.beats_per_bar,
                                    project.beat_unit,
                                );
                                painter.text(
                                    Pos2::new(x + 4.0, canvas.min.y + RULER_HEIGHT * 0.5),
                                    Align2::LEFT_CENTER,
                                    ruler_position,
                                    FontId::monospace(theme::type_size::LABEL),
                                    theme::palette(ui).muted,
                                );
                            }
                        }
                        let ruler_gesture_id =
                            ui.id().with(("midi-ruler-gesture", track_id, part_id));
                        if ruler_response.drag_started_by(egui::PointerButton::Primary)
                            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                        {
                            let anchor = (origin.x - canvas.left() - KEYBOARD_WIDTH) / beat_width;
                            ui.data_mut(|d| {
                                d.insert_temp(ruler_gesture_id, (origin, anchor, beat_width))
                            });
                        }
                        if ruler_response.dragged_by(egui::PointerButton::Primary)
                            && let Some(pointer) = ruler_response.interact_pointer_pos()
                            && let Some((origin, anchor, initial_width)) =
                                ui.data(|d| d.get_temp::<(Pos2, f32, f32)>(ruler_gesture_id))
                        {
                            let new_width = (initial_width
                                * ((pointer.y - origin.y) * 0.012).exp())
                            .clamp(4.0, 512.0);
                            let mut scroll =
                                egui::scroll_area::State::load(ui.ctx(), overview_scroll_id)
                                    .unwrap_or_default();
                            let screen_x = origin.x - piano_view.left();
                            let edge = if pointer.x < piano_view.left() + 32.0 {
                                -((piano_view.left() + 32.0 - pointer.x) / 32.0).clamp(0.0, 1.0)
                            } else if pointer.x > piano_view.right() - 32.0 {
                                ((pointer.x - piano_view.right() + 32.0) / 32.0).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            let delta = edge
                                * piano_view.width()
                                * 0.65
                                * ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
                            scroll.offset.x = (anchor * new_width - screen_x + delta).max(0.0);
                            scroll.store(ui.ctx(), overview_scroll_id);
                            ui.data_mut(|d| d.insert_temp(zoom_id, new_width));
                            *transport_position_beats = (part_start_beats
                                + f64::from(
                                    (pointer.x - canvas.left() - KEYBOARD_WIDTH) / beat_width,
                                ))
                            .max(0.0);
                            ui.ctx().request_repaint();
                        }
                        if ruler_response.drag_stopped_by(egui::PointerButton::Primary) {
                            ui.data_mut(|d| d.remove::<(Pos2, f32, f32)>(ruler_gesture_id));
                        }
                        if let Some(pointer) = ruler_response.interact_pointer_pos()
                            && ruler_response.clicked_by(egui::PointerButton::Primary)
                        {
                            let local_x = pointer.x - canvas.min.x - KEYBOARD_WIDTH;
                            let beat =
                                (local_x / beat_width).clamp(0.0, part_duration_beats as f32);
                            *transport_position_beats = part_start_beats + f64::from(beat);
                        }
                        if ruler_response.clicked_by(egui::PointerButton::Primary)
                            || ruler_response.drag_stopped_by(egui::PointerButton::Primary)
                        {
                            *transport_position_beats = crate::timeline::snap_beats_to_grid(
                                *transport_position_beats,
                                position_quantize,
                            );
                        }
                        let playhead_x = canvas.min.x
                            + KEYBOARD_WIDTH
                            + (*transport_position_beats - part_start_beats) as f32 * beat_width;
                        if playhead_x >= canvas.min.x + KEYBOARD_WIDTH && playhead_x <= canvas.max.x
                        {
                            painter.line_segment(
                                [
                                    Pos2::new(playhead_x, canvas.min.y),
                                    Pos2::new(playhead_x, canvas.bottom()),
                                ],
                                Stroke::new(1.0, theme::PLAYHEAD),
                            );
                        }

                        let part_grid = Rect::from_min_max(
                            Pos2::new(canvas.min.x + KEYBOARD_WIDTH, canvas.min.y + RULER_HEIGHT),
                            Pos2::new(
                                (canvas.min.x
                                    + KEYBOARD_WIDTH
                                    + part_duration_beats as f32 * beat_width)
                                    .min(canvas.max.x),
                                canvas.max.y,
                            ),
                        )
                        .intersect(Rect::from_min_max(
                            Pos2::new(piano_view.left(), pitch_area.top()),
                            Pos2::new(piano_view.right(), pitch_area.bottom()),
                        ));
                        if response.context_menu_opened()
                            || response
                                .hover_pos()
                                .is_some_and(|pointer| part_grid.contains(pointer))
                        {
                            response.context_menu(|ui| {
                                theme::menu_popup_style(ui.style_mut());
                                ui.horizontal(|ui| {
                                    for (value, icon, label) in [
                                        (EditTool::Select, ToolbarIcon::Select, "Mouse"),
                                        (EditTool::Split, ToolbarIcon::Split, "Scissors"),
                                        (EditTool::Draw, ToolbarIcon::Draw, "Draw Notes"),
                                    ] {
                                        if toolbar_icon_button(
                                            ui,
                                            icon,
                                            label,
                                            *tool == value,
                                            true,
                                        )
                                        .clicked()
                                        {
                                            *tool = value;
                                            ui.close();
                                        }
                                    }
                                });
                            });
                        }
                        let note_painter = painter.with_clip_rect(part_grid);
                        for note in &project.tracks[track_index].midi_parts[part_index].notes {
                            let is_selected = Some(note.id) == *selected_note_id
                                || selected_note_ids.contains(&note.id);
                            let note_rect = Rect::from_min_size(
                                Pos2::new(
                                    canvas.min.x
                                        + KEYBOARD_WIDTH
                                        + note.start_beats as f32 * beat_width,
                                    pitch_view.y(f32::from(note.pitch), part_grid)
                                        - row_height * 0.5,
                                ),
                                Vec2::new(
                                    (note.duration_beats as f32 * beat_width).max(3.0),
                                    (row_height - 1.0).max(1.0),
                                ),
                            );
                            let note_color = track_color
                                .gamma_multiply(0.35 + 0.65 * f32::from(note.velocity) / 127.0);
                            note_painter.rect_filled(
                                note_rect,
                                egui::CornerRadius::ZERO,
                                note_color,
                            );
                            note_painter.rect_stroke(
                                note_rect,
                                egui::CornerRadius::ZERO,
                                Stroke::new(
                                    if is_selected { 2.0 } else { 1.0 },
                                    if is_selected {
                                        Color32::WHITE
                                    } else {
                                        track_color
                                    },
                                ),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if let Some(drag) = selection_drag
                            .filter(|drag| drag.track_id == track_id && drag.part_id == part_id)
                        {
                            let selection_rect =
                                Rect::from_two_pos(drag.start, drag.current).intersect(part_grid);
                            note_painter.rect_filled(
                                selection_rect,
                                egui::CornerRadius::ZERO,
                                palette.accent.gamma_multiply(0.18),
                            );
                            note_painter.rect_stroke(
                                selection_rect,
                                egui::CornerRadius::ZERO,
                                Stroke::new(1.0, palette.accent),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if *tool == EditTool::Draw
                            && let Some(draw) = note_draw
                                .as_ref()
                                .filter(|draw| draw.track_id == track_id && draw.part_id == part_id)
                        {
                            let preview = Rect::from_min_size(
                                Pos2::new(
                                    canvas.min.x
                                        + KEYBOARD_WIDTH
                                        + draw.start_beats as f32 * beat_width,
                                    pitch_view.y(f32::from(draw.pitch), part_grid)
                                        - row_height * 0.5,
                                ),
                                Vec2::new(
                                    ((draw.end_beats - draw.start_beats) as f32 * beat_width)
                                        .max(3.0),
                                    (row_height - 1.0).max(1.0),
                                ),
                            );
                            note_painter.rect_filled(
                                preview,
                                egui::CornerRadius::same(2),
                                track_color.gamma_multiply(0.35),
                            );
                            note_painter.rect_stroke(
                                preview,
                                egui::CornerRadius::same(2),
                                Stroke::new(1.5, Color32::WHITE),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if *tool == EditTool::Draw
                            && note_draw.is_none()
                            && let Some(pointer) =
                                response.hover_pos().filter(|pos| part_grid.contains(*pos))
                        {
                            let beat =
                                f64::from((pointer.x - canvas.min.x - KEYBOARD_WIDTH) / beat_width);
                            let pitch =
                                pitch_view.center + (part_grid.center().y - pointer.y) / row_height;
                            if beat >= 0.0
                                && beat < part_duration_beats
                                && (0.0..=127.0).contains(&pitch)
                                && !project.tracks[track_index].midi_parts[part_index]
                                    .notes
                                    .iter()
                                    .any(|note| {
                                        f32::from(note.pitch).round() == pitch.round()
                                            && beat >= note.start_beats
                                            && beat < note.start_beats + note.duration_beats
                                    })
                            {
                                let start = ((beat / position_quantize).floor()
                                    * position_quantize)
                                    .clamp(0.0, part_duration_beats);
                                let duration = insert_length.min(part_duration_beats - start);
                                if duration >= 0.0625 {
                                    let ghost = Rect::from_min_size(
                                        Pos2::new(
                                            canvas.min.x
                                                + KEYBOARD_WIDTH
                                                + start as f32 * beat_width,
                                            pitch_view.y(pitch.round(), part_grid)
                                                - row_height * 0.5,
                                        ),
                                        Vec2::new(
                                            (duration as f32 * beat_width).max(3.0),
                                            (row_height - 1.0).max(1.0),
                                        ),
                                    );
                                    note_painter.rect_filled(
                                        ghost,
                                        egui::CornerRadius::same(2),
                                        track_color.gamma_multiply(0.38),
                                    );
                                    note_painter.rect_stroke(
                                        ghost,
                                        egui::CornerRadius::same(2),
                                        Stroke::new(1.0, track_color),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                            }
                        }

                        if *tool == EditTool::Select
                            && let Some(pointer) =
                                response.hover_pos().filter(|pos| part_grid.contains(*pos))
                        {
                            let pitch =
                                pitch_view.center + (part_grid.center().y - pointer.y) / row_height;
                            if (0.0..=127.0).contains(&pitch)
                                && project.tracks[track_index].midi_parts[part_index]
                                    .notes
                                    .iter()
                                    .any(|note| {
                                        if f32::from(note.pitch).round() != pitch.round() {
                                            return false;
                                        }
                                        let left = canvas.min.x
                                            + KEYBOARD_WIDTH
                                            + note.start_beats as f32 * beat_width;
                                        let right = left
                                            + (note.duration_beats as f32 * beat_width).max(3.0);
                                        let near_left = (pointer.x - left).abs() <= 6.0;
                                        let near_right = (right - pointer.x).abs() <= 6.0;
                                        let in_note = pointer.x >= left && pointer.x < right;
                                        in_note && (near_left || near_right)
                                    })
                            {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            }
                        }

                        // Note creation and editing are MIDI-editor-local gestures.
                        // Empty-space drags create notes only in Draw mode; middle-button
                        // drag belongs only to pan.
                        if response.drag_started_by(egui::PointerButton::Primary) {
                            let press_position = ui
                                .ctx()
                                .input(|input| input.pointer.press_origin())
                                .or_else(|| response.interact_pointer_pos());
                            if let Some(pointer) = press_position {
                                let pointer_pitch = pitch_view.center
                                    + (part_grid.center().y - pointer.y) / row_height;
                                let pointer_beat = f64::from(
                                    (pointer.x - canvas.min.x - KEYBOARD_WIDTH) / beat_width,
                                );
                                let note_under_pointer = part_grid.contains(pointer)
                                    && project.tracks[track_index].midi_parts[part_index]
                                        .notes
                                        .iter()
                                        .any(|note| {
                                            note.pitch == pointer_pitch.round() as u8
                                                && pointer_beat >= note.start_beats
                                                && pointer_beat
                                                    < note.start_beats + note.duration_beats
                                        });
                                if (*tool == EditTool::Range
                                    || (*tool == EditTool::Select && !note_under_pointer))
                                    && part_grid.contains(pointer)
                                {
                                    let additive = ui.input(|input| input.modifiers.shift);
                                    if !additive {
                                        selected_note_ids.clear();
                                    }
                                    *selected_note_id = None;
                                    selection_drag = Some(MidiNoteSelectionDrag {
                                        track_id,
                                        part_id,
                                        start: pointer,
                                        current: pointer,
                                    });
                                } else if *tool != EditTool::Split {
                                    let beat_x = pointer.x - canvas.min.x - KEYBOARD_WIDTH;
                                    let pitch_position = pitch_view.center
                                        + (part_grid.center().y - pointer.y) / row_height;
                                    let beat_position = f64::from(beat_x / beat_width);
                                    if beat_x >= 0.0
                                        && part_grid.contains(pointer)
                                        && (0.0..=127.0).contains(&pitch_position)
                                        && beat_position < part_duration_beats
                                    {
                                        let pitch = pitch_position.round() as u8;
                                        let note_hit = project.tracks[track_index].midi_parts
                                            [part_index]
                                            .notes
                                            .iter()
                                            .find(|note| {
                                                let note_left = canvas.min.x
                                                    + KEYBOARD_WIDTH
                                                    + note.start_beats as f32 * beat_width;
                                                let note_right = note_left
                                                    + (note.duration_beats as f32 * beat_width)
                                                        .max(3.0);
                                                note.pitch == pitch
                                                    && pointer.x >= note_left
                                                    && pointer.x < note_right
                                            })
                                            .map(|note| {
                                                (
                                                    note.id,
                                                    note.start_beats,
                                                    note.duration_beats,
                                                    note.pitch,
                                                    canvas.min.x
                                                        + KEYBOARD_WIDTH
                                                        + note.start_beats as f32 * beat_width,
                                                    canvas.min.x
                                                        + KEYBOARD_WIDTH
                                                        + (note.start_beats + note.duration_beats)
                                                            as f32
                                                            * beat_width,
                                                )
                                            });
                                        if let Some((
                                            note_id,
                                            start_beats,
                                            duration_beats,
                                            note_pitch,
                                            note_left,
                                            note_right,
                                        )) = note_hit
                                        {
                                            let left_edge_distance = (pointer.x - note_left).abs();
                                            let right_edge_distance =
                                                (note_right - pointer.x).abs();
                                            let mode = if *tool == EditTool::Select
                                                && left_edge_distance <= 6.0
                                                && left_edge_distance <= right_edge_distance
                                            {
                                                MidiNoteDragMode::ResizeStart
                                            } else if *tool == EditTool::Select
                                                && right_edge_distance <= 6.0
                                            {
                                                MidiNoteDragMode::ResizeEnd
                                            } else {
                                                MidiNoteDragMode::Move
                                            };
                                            if !selected_note_ids.contains(&note_id) {
                                                selected_note_ids.clear();
                                                selected_note_ids.insert(note_id);
                                            }
                                            *selected_note_id = Some(note_id);
                                            let members = if matches!(mode, MidiNoteDragMode::Move)
                                                && selected_note_ids.len() > 1
                                            {
                                                project.tracks[track_index].midi_parts[part_index]
                                                    .notes
                                                    .iter()
                                                    .filter(|note| {
                                                        selected_note_ids.contains(&note.id)
                                                    })
                                                    .map(|note| MidiNoteDragMember {
                                                        note_id: note.id,
                                                        start_beats: note.start_beats,
                                                        duration_beats: note.duration_beats,
                                                        pitch: note.pitch,
                                                    })
                                                    .collect()
                                            } else {
                                                vec![MidiNoteDragMember {
                                                    note_id,
                                                    start_beats,
                                                    duration_beats,
                                                    pitch: note_pitch,
                                                }]
                                            };
                                            *note_drag = Some(MidiNoteDrag {
                                                track_id,
                                                part_id,
                                                mode,
                                                members,
                                                delta_beats: 0.0,
                                                delta_rows: 0.0,
                                            });
                                        } else if *tool == EditTool::Draw {
                                            let start = ((beat_position / position_quantize)
                                                .floor()
                                                * position_quantize)
                                                .clamp(0.0, part_duration_beats);
                                            let remaining = part_duration_beats - start;
                                            if remaining >= 0.0625 {
                                                *selected_note_id = None;
                                                *note_draw = Some(MidiNoteDraw {
                                                    track_id,
                                                    part_id,
                                                    start_beats: start,
                                                    end_beats: start
                                                        + insert_length.min(remaining).max(0.0625),
                                                    pitch,
                                                    velocity: (*insert_velocity).clamp(1, 127),
                                                });
                                            }
                                        } else {
                                            *selected_note_id = None;
                                        }
                                    }
                                }
                            }
                        }

                        if response.dragged_by(egui::PointerButton::Primary) {
                            let delta = response.drag_delta();
                            if let Some(drag) = selection_drag.as_mut()
                                && drag.track_id == track_id
                                && drag.part_id == part_id
                                && let Some(pointer) = response.interact_pointer_pos()
                            {
                                drag.current = pointer;
                            }
                            if let Some(draw) = note_draw.as_mut()
                                && draw.track_id == track_id
                                && draw.part_id == part_id
                                && let Some(pointer) = response.interact_pointer_pos()
                            {
                                let minimum =
                                    position_quantize.min(part_duration_beats - draw.start_beats);
                                let beat = f64::from(
                                    (pointer.x - canvas.min.x - KEYBOARD_WIDTH) / beat_width,
                                );
                                draw.end_beats = ((beat / position_quantize).round()
                                    * position_quantize)
                                    .clamp(draw.start_beats + minimum, part_duration_beats);
                            }
                            if let Some(drag) = note_drag.as_mut()
                                && drag.track_id == track_id
                                && drag.part_id == part_id
                            {
                                drag.delta_beats += f64::from(delta.x / beat_width);
                                drag.delta_rows += f64::from(delta.y / row_height.max(1.0));
                            }
                            if let Some(drag) = note_drag
                                .as_ref()
                                .filter(|drag| drag.track_id == track_id && drag.part_id == part_id)
                                .cloned()
                            {
                                let quantize = if position_quantize_beats.is_finite()
                                    && *position_quantize_beats > 0.0
                                {
                                    *position_quantize_beats
                                } else {
                                    1.0
                                };
                                let notes =
                                    &mut project.tracks[track_index].midi_parts[part_index].notes;
                                for member in drag.members {
                                    if let Some(note) =
                                        notes.iter_mut().find(|note| note.id == member.note_id)
                                    {
                                        match drag.mode {
                                            MidiNoteDragMode::Move => {
                                                let max_start = (part_duration_beats
                                                    - member.duration_beats)
                                                    .max(0.0);
                                                let snapped_start = ((member.start_beats
                                                    + drag.delta_beats)
                                                    / quantize)
                                                    .round()
                                                    * quantize;
                                                let next_start =
                                                    snapped_start.clamp(0.0, max_start);
                                                let next_pitch = (i16::from(member.pitch)
                                                    - drag.delta_rows.round() as i16)
                                                    .clamp(0, 127)
                                                    as u8;
                                                modified |= note.start_beats != next_start
                                                    || note.pitch != next_pitch;
                                                note.start_beats = next_start;
                                                note.pitch = next_pitch;
                                            }
                                            MidiNoteDragMode::ResizeEnd => {
                                                let max_length =
                                                    part_duration_beats - member.start_beats;
                                                if max_length >= 0.0625 {
                                                    let snapped_length = ((member.duration_beats
                                                        + drag.delta_beats)
                                                        / quantize)
                                                        .round()
                                                        * quantize;
                                                    let next_length =
                                                        snapped_length.clamp(0.0625, max_length);
                                                    modified |= note.duration_beats != next_length;
                                                    note.duration_beats = next_length;
                                                }
                                            }
                                            MidiNoteDragMode::ResizeStart => {
                                                let fixed_end =
                                                    member.start_beats + member.duration_beats;
                                                let latest_start = fixed_end - 0.0625;
                                                if latest_start >= 0.0 {
                                                    let snapped_start = ((member.start_beats
                                                        + drag.delta_beats)
                                                        / quantize)
                                                        .round()
                                                        * quantize;
                                                    let next_start =
                                                        snapped_start.clamp(0.0, latest_start);
                                                    let next_duration = fixed_end - next_start;
                                                    modified |= note.start_beats != next_start
                                                        || note.duration_beats != next_duration;
                                                    note.start_beats = next_start;
                                                    note.duration_beats = next_duration;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if response.drag_stopped() {
                            *note_drag = None;
                            if let Some(draw) = note_draw.take()
                                && draw.track_id == track_id
                                && draw.part_id == part_id
                                && draw.end_beats > draw.start_beats
                            {
                                edit_action = Some(MidiEditAction::Add(
                                    draw.start_beats,
                                    draw.end_beats - draw.start_beats,
                                    draw.pitch,
                                    draw.velocity,
                                ));
                            }
                            if let Some(drag) = selection_drag.take()
                                && drag.track_id == track_id
                                && drag.part_id == part_id
                            {
                                let selection_rect = Rect::from_two_pos(drag.start, drag.current)
                                    .intersect(part_grid);
                                let notes =
                                    &project.tracks[track_index].midi_parts[part_index].notes;
                                for note in notes {
                                    let note_rect = Rect::from_min_size(
                                        Pos2::new(
                                            canvas.min.x
                                                + KEYBOARD_WIDTH
                                                + note.start_beats as f32 * beat_width,
                                            pitch_view.y(f32::from(note.pitch), part_grid)
                                                - row_height * 0.5,
                                        ),
                                        Vec2::new(
                                            (note.duration_beats as f32 * beat_width).max(3.0),
                                            (row_height - 1.0).max(1.0),
                                        ),
                                    );
                                    if selection_rect.intersects(note_rect) {
                                        selected_note_ids.insert(note.id);
                                    }
                                }
                                *selected_note_id = if selected_note_ids.len() == 1 {
                                    selected_note_ids.iter().next().copied()
                                } else {
                                    None
                                };
                            }
                        }

                        if let Some(pointer) = response.interact_pointer_pos()
                            && response.clicked_by(egui::PointerButton::Primary)
                        {
                            let beat_x = pointer.x - canvas.min.x - KEYBOARD_WIDTH;
                            let pitch_position =
                                pitch_view.center + (part_grid.center().y - pointer.y) / row_height;
                            if beat_x >= 0.0
                                && part_grid.contains(pointer)
                                && (0.0..=127.0).contains(&pitch_position)
                            {
                                let beat_position = f64::from(beat_x / beat_width);
                                let beat = beat_position.floor() as usize;
                                if beat < visible_beats && beat_position < part_duration_beats {
                                    let pitch = pitch_position.round() as u8;
                                    let note_hit = project.tracks[track_index].midi_parts
                                        [part_index]
                                        .notes
                                        .iter()
                                        .find(|note| {
                                            note.pitch == pitch
                                                && pointer.x
                                                    >= canvas.min.x
                                                        + KEYBOARD_WIDTH
                                                        + note.start_beats as f32 * beat_width
                                                && pointer.x
                                                    < canvas.min.x
                                                        + KEYBOARD_WIDTH
                                                        + (note.start_beats + note.duration_beats)
                                                            as f32
                                                            * beat_width
                                        })
                                        .map(|note| note.id);
                                    if let Some(note_id) = note_hit {
                                        if *tool == EditTool::Split {
                                            let quantize = position_quantize;
                                            let split_at =
                                                (beat_position / quantize).round() * quantize;
                                            edit_action =
                                                Some(MidiEditAction::Split(note_id, split_at));
                                        } else {
                                            selection_drag = None;
                                            selected_note_ids.clear();
                                            selected_note_ids.insert(note_id);
                                            edit_action = Some(MidiEditAction::Select(note_id));
                                        }
                                    } else if matches!(*tool, EditTool::Range | EditTool::Select) {
                                        selection_drag = None;
                                        *selected_note_id = None;
                                    } else if *tool == EditTool::Draw {
                                        selected_note_ids.clear();
                                        let quantize = if position_quantize.is_finite()
                                            && position_quantize > 0.0
                                        {
                                            position_quantize
                                        } else {
                                            1.0
                                        };
                                        let start = ((beat_position / quantize).floor() * quantize)
                                            .clamp(0.0, part_duration_beats);
                                        let duration = insert_length
                                            .min(part_duration_beats - start)
                                            .max(0.0625);
                                        edit_action = Some(MidiEditAction::Add(
                                            start,
                                            duration,
                                            pitch,
                                            (*insert_velocity).clamp(1, 127),
                                        ));
                                    } else {
                                        selected_note_ids.clear();
                                        *selected_note_id = None;
                                    }
                                }
                            }
                        }

                        let controller_size = Vec2::new(size.x, controller_lane_height);
                        let controller_canvas =
                            Rect::from_min_size(canvas.left_bottom(), controller_size);
                        let controller_response = ui.interact(
                            controller_canvas,
                            ui.id().with("midi-controller-lane"),
                            Sense::click_and_drag(),
                        );
                        let controller_resize_rect = Rect::from_min_max(
                            controller_canvas.min,
                            Pos2::new(controller_canvas.right(), controller_canvas.top() + 14.0),
                        );
                        let (
                            press_origin,
                            pointer_pos,
                            primary_pressed,
                            primary_down,
                            primary_released,
                            pointer_delta_y,
                        ) = ui.input(|input| {
                            (
                                input.pointer.press_origin(),
                                input.pointer.interact_pos(),
                                input.pointer.button_pressed(egui::PointerButton::Primary),
                                input.pointer.button_down(egui::PointerButton::Primary),
                                input.pointer.button_released(egui::PointerButton::Primary),
                                input.pointer.delta().y,
                            )
                        });
                        let mut controller_resizing = ui.data(|d| {
                            d.get_temp::<bool>(controller_resize_active_id)
                                .unwrap_or(false)
                        });
                        if primary_pressed
                            && press_origin
                                .is_some_and(|origin| controller_resize_rect.contains(origin))
                        {
                            controller_resizing = true;
                            ui.data_mut(|d| d.insert_temp(controller_resize_active_id, true));
                            *controller_drag = None;
                        }
                        if controller_resizing && primary_down {
                            controller_lane_height = (controller_lane_height - pointer_delta_y)
                                .clamp(CONTROLLER_MIN_HEIGHT, max_controller_height);
                            ui.data_mut(|d| {
                                d.insert_temp(controller_height_id, controller_lane_height)
                            });
                            ui.ctx().request_repaint();
                        }
                        if controller_resizing && (primary_released || !primary_down) {
                            ui.data_mut(|d| d.remove::<bool>(controller_resize_active_id));
                        }
                        if controller_resizing
                            || pointer_pos.is_some_and(|pos| controller_resize_rect.contains(pos))
                        {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                        }
                        let controller_painter = ui.painter_at(controller_canvas);
                        controller_painter.rect_filled(
                            controller_canvas,
                            egui::CornerRadius::ZERO,
                            theme::palette(ui).canvas,
                        );
                        controller_painter.rect_filled(
                            Rect::from_min_max(
                                controller_canvas.min,
                                Pos2::new(
                                    controller_canvas.min.x + KEYBOARD_WIDTH,
                                    controller_canvas.max.y,
                                ),
                            ),
                            egui::CornerRadius::ZERO,
                            theme::palette(ui).surface,
                        );
                        let plot = Rect::from_min_size(
                            Pos2::new(
                                controller_canvas.min.x + KEYBOARD_WIDTH,
                                controller_canvas.min.y + CONTROLLER_PLOT_TOP,
                            ),
                            Vec2::new(size.x - KEYBOARD_WIDTH, controller_plot_height),
                        );
                        let controller_header = Rect::from_min_max(
                            Pos2::new(
                                controller_canvas.min.x + KEYBOARD_WIDTH,
                                controller_canvas.min.y,
                            ),
                            Pos2::new(controller_canvas.max.x, plot.top()),
                        );
                        controller_painter.rect_filled(
                            controller_header,
                            egui::CornerRadius::ZERO,
                            theme::palette(ui).surface_raised,
                        );
                        controller_painter.rect_filled(
                            plot,
                            egui::CornerRadius::ZERO,
                            theme::palette(ui).canvas,
                        );
                        controller_painter.line_segment(
                            [controller_canvas.left_top(), controller_canvas.right_top()],
                            Stroke::new(2.0, theme::palette(ui).grid),
                        );
                        controller_painter.rect_filled(
                            Rect::from_center_size(
                                Pos2::new(
                                    controller_canvas.center().x,
                                    controller_canvas.top() + 3.0,
                                ),
                                Vec2::new(42.0, 2.0),
                            ),
                            1.0,
                            theme::palette(ui).muted,
                        );
                        controller_painter.text(
                            Pos2::new(
                                controller_header.left() + 7.0,
                                controller_canvas.min.y + 12.0,
                            ),
                            Align2::LEFT_CENTER,
                            midi_controller_label(*controller_number),
                            FontId::monospace(theme::type_size::LABEL),
                            theme::palette(ui).text,
                        );
                        controller_painter.line_segment(
                            [plot.left_top(), plot.right_top()],
                            Stroke::new(1.0, theme::palette(ui).grid),
                        );
                        let mut lane_events: Vec<_> = project.tracks[track_index].midi_parts
                            [part_index]
                            .controller_events
                            .iter()
                            .filter(|event| event.controller == *controller_number)
                            .map(|event| (event.id, event.position_beats, event.value))
                            .collect();
                        lane_events.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
                        for beat in 0..=visible_beats {
                            let x = plot.left() + beat as f32 * beat_width;
                            let bar_line = beat % bpb == 0;
                            controller_painter.line_segment(
                                [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
                                Stroke::new(
                                    if bar_line { 1.0 } else { 0.5 },
                                    if bar_line {
                                        theme::palette(ui).grid
                                    } else {
                                        theme::palette(ui).surface_raised
                                    },
                                ),
                            );
                        }
                        for value in [0_u8, 32, 64, 96] {
                            let y =
                                plot.bottom() - f32::from(value) / 127.0 * controller_plot_height;
                            controller_painter.line_segment(
                                [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                                Stroke::new(0.5, theme::palette(ui).surface_raised),
                            );
                            controller_painter.with_clip_rect(plot).text(
                                Pos2::new(plot.left() + 3.0, (y - 2.0).max(plot.top() + 10.0)),
                                Align2::LEFT_BOTTOM,
                                value.to_string(),
                                FontId::monospace(theme::type_size::TINY),
                                theme::palette(ui).muted,
                            );
                        }
                        let default_value = midi_controller_default_value(*controller_number);
                        let default_y = plot.bottom()
                            - f32::from(default_value) / 127.0 * controller_plot_height;
                        let mut previous_point = Some((plot.left(), default_y));
                        for (_, position_beats, value) in &lane_events {
                            let x = plot.left() + *position_beats as f32 * beat_width;
                            let y =
                                plot.bottom() - f32::from(*value) / 127.0 * controller_plot_height;
                            if let Some((previous_x, previous_y)) = previous_point {
                                controller_painter.line_segment(
                                    [Pos2::new(previous_x, previous_y), Pos2::new(x, previous_y)],
                                    Stroke::new(1.5, theme::palette(ui).accent),
                                );
                                controller_painter.line_segment(
                                    [Pos2::new(x, previous_y), Pos2::new(x, y)],
                                    Stroke::new(1.5, theme::palette(ui).accent),
                                );
                            }
                            controller_painter.circle_filled(
                                Pos2::new(x, y),
                                4.0,
                                theme::palette(ui).accent,
                            );
                            previous_point = Some((x, y));
                        }
                        if let Some((previous_x, previous_y)) = previous_point {
                            controller_painter.line_segment(
                                [
                                    Pos2::new(previous_x, previous_y),
                                    Pos2::new(plot.right(), previous_y),
                                ],
                                Stroke::new(1.0, theme::palette(ui).accent),
                            );
                        }

                        let controller_event_hit = |pointer: Pos2| {
                            lane_events
                                .iter()
                                .filter_map(|(event_id, position_beats, value)| {
                                    let x = plot.left() + *position_beats as f32 * beat_width;
                                    let y = plot.bottom()
                                        - f32::from(*value) / 127.0 * controller_plot_height;
                                    let distance_sq =
                                        (pointer.x - x).powi(2) + (pointer.y - y).powi(2);
                                    (distance_sq <= 100.0).then_some((
                                        *event_id,
                                        *position_beats,
                                        *value,
                                        distance_sq,
                                    ))
                                })
                                .min_by(|a, b| a.3.total_cmp(&b.3))
                                .map(|(id, position, value, _)| (id, position, value))
                        };
                        if !controller_resizing
                            && controller_response.drag_started_by(egui::PointerButton::Primary)
                            && let Some(pointer) = ui
                                .ctx()
                                .input(|input| input.pointer.press_origin())
                                .or_else(|| controller_response.interact_pointer_pos())
                            && plot.contains(pointer)
                            && let Some((event_id, position, value)) = controller_event_hit(pointer)
                        {
                            *controller_drag = Some(MidiControllerDrag {
                                track_id,
                                part_id,
                                event_id,
                                start_position_beats: position,
                                start_value: value,
                                delta_beats: 0.0,
                                delta_value: 0.0,
                            });
                        }
                        if !controller_resizing
                            && controller_response.dragged_by(egui::PointerButton::Primary)
                        {
                            let delta = controller_response.drag_delta();
                            if let Some(drag) = controller_drag.as_mut()
                                && drag.track_id == track_id
                                && drag.part_id == part_id
                            {
                                drag.delta_beats += f64::from(delta.x / beat_width);
                                drag.delta_value +=
                                    f64::from(-delta.y * 127.0 / controller_plot_height);
                            }
                            if let Some(drag) = *controller_drag
                                && drag.track_id == track_id
                                && drag.part_id == part_id
                                && let Some(event) = project.tracks[track_index].midi_parts
                                    [part_index]
                                    .controller_events
                                    .iter_mut()
                                    .find(|event| event.id == drag.event_id)
                            {
                                let quantize = if position_quantize_beats.is_finite()
                                    && *position_quantize_beats > 0.0
                                {
                                    *position_quantize_beats
                                } else {
                                    1.0
                                };
                                let position = ((drag.start_position_beats + drag.delta_beats)
                                    / quantize)
                                    .round()
                                    * quantize;
                                let next_position = position.clamp(0.0, part_duration_beats);
                                let next_value = (f64::from(drag.start_value) + drag.delta_value)
                                    .round()
                                    .clamp(0.0, 127.0)
                                    as u8;
                                modified |= event.position_beats != next_position
                                    || event.value != next_value;
                                event.position_beats = next_position;
                                event.value = next_value;
                            }
                        }
                        if !controller_resizing && controller_response.drag_stopped() {
                            *controller_drag = None;
                        }
                        if !controller_resizing
                            && let Some(pointer) = controller_response.interact_pointer_pos()
                            && plot.contains(pointer)
                            && (controller_response.clicked_by(egui::PointerButton::Primary)
                                || controller_response.secondary_clicked())
                        {
                            let hit = controller_event_hit(pointer);
                            if controller_response.secondary_clicked() {
                                if let Some((event_id, _, _)) = hit {
                                    let events = &mut project.tracks[track_index].midi_parts
                                        [part_index]
                                        .controller_events;
                                    let previous_len = events.len();
                                    events.retain(|event| event.id != event_id);
                                    modified |= events.len() != previous_len;
                                }
                            } else if controller_response.clicked_by(egui::PointerButton::Primary)
                                && hit.is_none()
                            {
                                let beat = f64::from((pointer.x - plot.left()) / beat_width);
                                let quantize = if position_quantize_beats.is_finite()
                                    && *position_quantize_beats > 0.0
                                {
                                    *position_quantize_beats
                                } else {
                                    1.0
                                };
                                let position = ((beat / quantize).floor() * quantize)
                                    .clamp(0.0, part_duration_beats);
                                let value = ((plot.bottom() - pointer.y) / controller_plot_height
                                    * 127.0)
                                    .round()
                                    .clamp(0.0, 127.0)
                                    as u8;
                                modified |= project
                                    .add_midi_controller_event(
                                        track_index,
                                        part_id,
                                        position,
                                        *controller_number,
                                        value,
                                    )
                                    .is_some();
                            }
                        }
                    });
                let (press_origin, pointer_delta, middle_down) = ui.input(|input| {
                    (
                        input.pointer.press_origin(),
                        input.pointer.delta(),
                        input.pointer.button_down(egui::PointerButton::Middle),
                    )
                });
                let mut state = scroll_output.state;
                let mut scroll_changed = false;
                match wheel_action {
                    Some(crate::view_controls::WheelAction::HorizontalPan(amount)) => {
                        state.offset.x = (state.offset.x - amount).max(0.0);
                        scroll_changed = true;
                    }
                    Some(crate::view_controls::WheelAction::TimeZoom(amount)) => {
                        let pointer_x = ui
                            .input(|i| i.pointer.hover_pos())
                            .map_or(piano_view.center().x, |p| p.x);
                        let screen_x =
                            (pointer_x - piano_view.left()).clamp(0.0, piano_view.width());
                        let anchor = (state.offset.x + screen_x) / beat_width;
                        let new_width = (beat_width * (amount * 0.004).exp()).clamp(4.0, 512.0);
                        state.offset.x = (anchor * new_width - screen_x).max(0.0);
                        ui.data_mut(|d| d.insert_temp(zoom_id, new_width));
                        scroll_changed = true;
                    }
                    _ => {}
                }
                if fit_part {
                    state.offset = Vec2::ZERO;
                    scroll_changed = true;
                }
                if middle_down
                    && press_origin.is_some_and(|pos| scroll_output.inner_rect.contains(pos))
                {
                    pitch_view.pan_pixels(pointer_delta.y, note_area_height);
                    state.offset.x = (state.offset.x - pointer_delta.x).max(0.0);
                    state.offset.y = 0.0;
                    scroll_changed = true;
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                }
                let content_width = scroll_output.content_size.x;
                let max_scroll_x = (content_width - scroll_output.inner_rect.width()).max(0.0);
                let view_beats = ((scroll_output.inner_rect.width() - KEYBOARD_WIDTH).max(0.0)
                    / beat_width)
                    .min(visible_beats as f32);
                if (overview_response.clicked() || overview_response.dragged())
                    && let Some(pointer) = overview_response.interact_pointer_pos()
                    && !overview_response.dragged()
                {
                    let fraction = ((pointer.x - overview_part.left())
                        / overview_part.width().max(1.0))
                    .clamp(0.0, 1.0);
                    let target_center = fraction * overview_beats;
                    state.offset.x =
                        ((target_center - view_beats * 0.5) * beat_width).clamp(0.0, max_scroll_x);
                    scroll_changed = true;
                }
                let view_left_beats = state.offset.x / beat_width;
                let visible_fraction = (view_beats / overview_beats).clamp(0.0, 1.0);
                let viewport_width = overview_part.width() * visible_fraction;
                let viewport_left =
                    overview_part.left() + view_left_beats / overview_beats * overview_part.width();
                let viewport_rect = Rect::from_min_size(
                    Pos2::new(viewport_left, overview_part.top()),
                    Vec2::new(viewport_width.max(1.0), overview_part.height()),
                );
                overview_viewport_rect = Some(viewport_rect);
                if overview_response.dragged()
                    && let Some(pointer) = overview_response.interact_pointer_pos()
                {
                    let fraction = ((pointer.x - overview_part.left())
                        / overview_part.width().max(1.0))
                    .clamp(0.0, 1.0);
                    state.offset.x = ((fraction * overview_beats - view_beats * 0.5) * beat_width)
                        .clamp(0.0, max_scroll_x);
                    scroll_changed = true;
                }
                if scroll_changed {
                    state.store(ui.ctx(), scroll_output.id);
                    ui.ctx().request_repaint();
                }
            });
        },
    );
    if let Some(viewport_rect) = overview_viewport_rect {
        ui.painter().with_clip_rect(overview_part).rect_stroke(
            viewport_rect,
            0.0,
            Stroke::new(1.0, Color32::from_gray(210)),
            egui::StrokeKind::Inside,
        );
    }
    if let Some((keyboard, wave)) = pitch_controls_rects {
        let fixed_keyboard = Rect::from_min_max(
            Pos2::new(piano_view.left() - KEYBOARD_WIDTH, keyboard.top()),
            Pos2::new(piano_view.left(), keyboard.bottom()),
        );
        ui.painter()
            .rect_filled(fixed_keyboard, 0.0, palette.surface);
        let row_height = wave.height() / pitch_view.span;
        for pitch in 0..=127_i32 {
            let y = pitch_view.y(pitch as f32, wave);
            let row = Rect::from_min_max(
                Pos2::new(fixed_keyboard.left(), y - row_height * 0.5),
                Pos2::new(fixed_keyboard.right(), y + row_height * 0.5),
            )
            .intersect(fixed_keyboard);
            if row.is_positive() {
                sample_editor::paint_pitch_key_row(ui, row, row, pitch, row_height);
            }
        }
        ui.painter().rect_filled(
            Rect::from_min_max(Pos2::new(bounds.right() - 20.0, wave.top()), bounds.max),
            0.0,
            palette.surface_raised,
        );
        let rail = Rect::from_min_max(
            Pos2::new(bounds.right() - 20.0, wave.top()),
            Pos2::new(bounds.right(), bounds.bottom()),
        );
        sample_editor::pitch_view_controls(ui, rail, keyboard, wave, pitch_view);
    }

    match edit_action {
        Some(MidiEditAction::Add(start_beats, duration_beats, pitch, velocity)) => {
            *selected_note_id = project.add_midi_note(
                track_index,
                part_id,
                start_beats,
                duration_beats,
                pitch,
                velocity,
            );
            modified |= selected_note_id.is_some();
        }
        Some(MidiEditAction::Select(note_id)) => *selected_note_id = Some(note_id),
        Some(MidiEditAction::Split(note_id, beat)) => {
            let split_note = {
                let part = &mut project.tracks[track_index].midi_parts[part_index];
                if let Some(note) = part.notes.iter_mut().find(|note| note.id == note_id) {
                    let end = note.start_beats + note.duration_beats;
                    if beat > note.start_beats + 0.0625 && beat < end - 0.0625 {
                        note.duration_beats = beat - note.start_beats;
                        Some((note.pitch, note.channel, note.velocity, end - beat))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some((pitch, channel, velocity, duration)) = split_note {
                modified |= project
                    .add_midi_note_on_channel(
                        track_index,
                        part_id,
                        beat,
                        duration,
                        pitch,
                        channel,
                        velocity,
                    )
                    .is_some();
            }
        }
        None => {}
    }
    if let Some(drag) = selection_drag {
        ui.data_mut(|data| data.insert_temp(note_selection_drag_id, drag));
    } else {
        ui.data_mut(|data| data.remove::<MidiNoteSelectionDrag>(note_selection_drag_id));
    }
    modified
}

#[cfg(test)]
mod editor_recovery_tests {
    use super::*;

    #[test]
    fn group_edit_waveform_survives_unrelated_solo_and_group_mute() {
        let path = std::env::temp_dir().join(format!(
            "opentrack-group-preview-{}.wav",
            std::process::id()
        ));
        let samples: Vec<i16> = (0..800)
            .map(|i| if i % 16 < 8 { 12000 } else { -12000 })
            .collect();
        let data_len = (samples.len() * 2) as u32;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&8000_u32.to_le_bytes());
        wav.extend_from_slice(&16000_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(&path, wav).unwrap();
        let source = AudioSource::open(&path).unwrap();
        let mut project = Project::default();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Midi);
        let group = project.tracks[0].id;
        project.tracks[1].parent_group_id = Some(group);
        project.add_audio_clip(1, path.clone(), 0.0, 0.1).unwrap();
        let sources = HashMap::from([(path.clone(), source)]);
        let before = group_waveform_columns(&project, group, &sources, 0.0, 0.1, 20, 1);
        assert!(before.iter().any(|column| column[0][1] > 0.1));
        project.tracks[2].solo = true;
        project.tracks[0].muted = true;
        assert_eq!(
            before,
            group_waveform_columns(&project, group, &sources, 0.0, 0.1, 20, 1)
        );
        drop(sources);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn expression_lane_is_visible_and_keeps_its_size_after_drag_release() {
        let ctx = egui::Context::default();
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let part = project.add_midi_part(0, 0.0, 16.0).unwrap();
        let mut pitch = sample_editor::PitchViewport::default();
        let (mut position, mut length, mut velocity, mut controller) = (0.25, 0.25, 100, 11);
        let mut live_velocity_update = None;
        let (mut selected, mut drag, mut draw, mut cc_drag) = (None, None, None, None);
        let mut selected_notes = BTreeSet::new();
        let (mut tool, mut transport) = (EditTool::Select, 0.0);
        let mut divider = Pos2::ZERO;
        let mut initial_header_y = 0.0;
        for frame in 0..6 {
            let events = match frame {
                2 => vec![
                    egui::Event::PointerMoved(divider),
                    egui::Event::PointerButton {
                        pos: divider,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                3 => vec![egui::Event::PointerMoved(divider + Vec2::new(0.0, 28.0))],
                4 => vec![egui::Event::PointerButton {
                    pos: divider + Vec2::new(0.0, 28.0),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                _ => vec![],
            };
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 700.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    show_midi_editor(
                        ui,
                        &mut project,
                        0,
                        part,
                        &mut position,
                        &mut length,
                        &mut velocity,
                        &mut controller,
                        &mut live_velocity_update,
                        &mut selected,
                        &mut selected_notes,
                        &mut drag,
                        &mut draw,
                        &mut cc_drag,
                        &mut tool,
                        &mut transport,
                        &mut pitch,
                    );
                },
            );
            assert!(
                output.shapes.iter().any(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if text.galley.text() == "Expression (CC 11)" && text.pos.x > 300.0 =>
                    {
                        shape
                            .clip_rect
                            .intersects(text.galley.rect.translate(text.pos.to_vec2()))
                            && text.pos.y < 680.0
                    }
                    _ => false,
                }),
                "expression plot header must remain inside the editor viewport"
            );
            let header = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if text.galley.text() == "Expression (CC 11)" && text.pos.x > 300.0 =>
                    {
                        Some(text.galley.rect.translate(text.pos.to_vec2()))
                    }
                    _ => None,
                })
                .expect("expression header");
            if frame == 1 {
                divider = Pos2::new(header.center().x, header.center().y - 8.0);
                initial_header_y = header.top();
            }
            if frame == 5 {
                assert!(
                    header.top() > initial_header_y + 20.0,
                    "dragging the divider down must shrink the lane and retain that size after release"
                );
            }
            output.textures_delta.clear();
        }
    }
}

enum MidiEditAction {
    Add(f64, f64, u8, u8),
    Select(u64),
    Split(u64, f64),
}

const MIDI_QUANTIZE_VALUES: [(f64, &str); 7] = [
    (0.0625, "1/64 note"),
    (0.125, "1/32 note"),
    (0.25, "1/16 note"),
    (0.5, "1/8 note"),
    (1.0, "1/4 note"),
    (2.0, "1/2 note"),
    (4.0, "Whole note"),
];

const MIDI_CONTROLLER_VALUES: [(u8, &str); 4] = [
    (1, "Modulation (CC 1)"),
    (7, "Volume (CC 7)"),
    (10, "Pan (CC 10)"),
    (11, "Expression (CC 11)"),
];

fn is_midi_quantize_value(value: f64) -> bool {
    MIDI_QUANTIZE_VALUES
        .iter()
        .any(|(candidate, _)| value == *candidate)
}

fn midi_quantize_label(value: f64) -> &'static str {
    MIDI_QUANTIZE_VALUES
        .iter()
        .find(|(candidate, _)| value == *candidate)
        .map(|(_, label)| *label)
        .unwrap_or("1/4 note")
}

fn midi_controller_label(controller: u8) -> &'static str {
    MIDI_CONTROLLER_VALUES
        .iter()
        .find(|(candidate, _)| *candidate == controller)
        .map(|(_, label)| *label)
        .unwrap_or("Expression (CC 11)")
}

fn midi_controller_default_value(controller: u8) -> u8 {
    match controller {
        7 | 11 => 127,
        10 => 64,
        _ => 0,
    }
}

fn midi_pitch_label(pitch: u8) -> String {
    sample_editor::pitch_note_name(i32::from(pitch))
}

fn show_mixer(
    ui: &mut egui::Ui,
    project: &mut Project,
    meter_levels: &HashMap<u64, [f32; 2]>,
    peak_holds: &mut HashMap<u64, metering::Hold>,
    input_meter_peaks: [f32; 2],
    output_meter_peaks: [f32; 2],
    show_strips: bool,
    show_settings: bool,
    channel_settings_window: &mut NativeWindowState,
    channel_settings_ui: &mut ChannelSettingsUiState,
    vst3_plugin_catalog: &[PluginDescriptor],
) -> (bool, bool, bool) {
    if show_strips {
        show_mixer_channels(
            ui,
            project,
            meter_levels,
            peak_holds,
            input_meter_peaks,
            output_meter_peaks,
            channel_settings_window,
        )
    } else if show_settings {
        {
            let changed = channel_settings::show(
                ui,
                project,
                meter_levels,
                peak_holds,
                channel_settings_ui,
                &mut channel_settings_window.channel_bus,
                input_meter_peaks,
                output_meter_peaks,
                vst3_plugin_catalog,
            );
            (changed, false, false)
        }
    } else {
        (false, false, false)
    }
}

fn show_mixer_channels(
    ui: &mut egui::Ui,
    project: &mut Project,
    meter_levels: &HashMap<u64, [f32; 2]>,
    peak_holds: &mut HashMap<u64, metering::Hold>,
    input_meter_peaks: [f32; 2],
    output_meter_peaks: [f32; 2],
    channel_settings_window: &mut NativeWindowState,
) -> (bool, bool, bool) {
    mixer::show_channels(
        ui,
        project,
        meter_levels,
        peak_holds,
        input_meter_peaks,
        output_meter_peaks,
        channel_settings_window,
    )
}

fn set_channel_view_item(state: &mut ChannelSettingsUiState, item: ChannelSettingsViewItem) {
    if state.bus_context && !item.available_for_bus() {
        return;
    }
    if matches!(
        item,
        ChannelSettingsViewItem::Equalizers
            | ChannelSettingsViewItem::EqualizersCurve
            | ChannelSettingsViewItem::EqualizersCurveDials
    ) {
        let was_visible = state.preferences.visible_items.contains(&item);
        state.preferences.visible_items.retain(|visible| {
            !matches!(
                visible,
                ChannelSettingsViewItem::Equalizers
                    | ChannelSettingsViewItem::EqualizersCurve
                    | ChannelSettingsViewItem::EqualizersCurveDials
            )
        });
        if !was_visible {
            state.preferences.visible_items.push(item);
        }
    } else if let Some(index) = state
        .preferences
        .visible_items
        .iter()
        .position(|visible| *visible == item)
    {
        state.preferences.visible_items.remove(index);
    } else {
        state.preferences.visible_items.push(item);
    }
}

fn channel_menu_item(ui: &mut egui::Ui, checked: bool, label: &str) -> egui::Response {
    ui.button(format!("{}  {label}", if checked { "✓" } else { "   " }))
}

fn show_channel_context_menu(response: &egui::Response, state: &mut ChannelSettingsUiState) {
    let menu_config =
        egui::containers::menu::MenuConfig::new().style(theme::channel_settings_menu_popup_style);
    egui::Popup::context_menu(response)
        .id(response.id.with("channel-settings-context-menu"))
        .width(220.0)
        .style(theme::channel_settings_menu_popup_style)
        .info(egui::UiStackInfo::new(egui::UiKind::Menu).with_tag_value(
            egui::containers::menu::MenuConfig::MENU_CONFIG_TAG,
            menu_config,
        ))
        .show(|ui| {
            ui.menu_button("Customize View", |ui| {
                for item in ChannelSettingsViewItem::ALL {
                    if state.bus_context && !item.available_for_bus() {
                        continue;
                    }
                    let supported = item.implemented();
                    let selected = state.preferences.visible_items.contains(&item);
                    let label = if supported {
                        item.label().to_owned()
                    } else {
                        format!("{} (not implemented)", item.label())
                    };
                    if ui
                        .add_enabled(
                            supported,
                            egui::Button::new(format!(
                                "{}  {label}",
                                if selected { "✓" } else { "   " }
                            )),
                        )
                        .clicked()
                    {
                        set_channel_view_item(state, item);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Show All").clicked() {
                    state.preferences.visible_items = vec![
                        ChannelSettingsViewItem::Inserts,
                        ChannelSettingsViewItem::EqualizersCurve,
                        ChannelSettingsViewItem::Sends,
                        ChannelSettingsViewItem::Fader,
                    ];
                    ui.close();
                }
                if ui.button("Default").clicked() {
                    state.preferences.visible_items = default_channel_settings_items();
                    ui.close();
                }
                if ui.button("Fader Only").clicked() {
                    state.preferences.visible_items = vec![ChannelSettingsViewItem::Fader];
                    ui.close();
                }
                ui.separator();
                if ui.button("Setup...").clicked() {
                    state.setup_open = true;
                    ui.close();
                }
            });
            ui.separator();
            if channel_menu_item(ui, state.preferences.always_on_top, "Always on Top").clicked() {
                state.preferences.always_on_top = !state.preferences.always_on_top;
                ui.close();
            }
            ui.separator();
            for (label, action) in [
                (
                    "Save Selected Channels",
                    ChannelSettingsAction::SaveSelected,
                ),
                (
                    "Load Selected Channels",
                    ChannelSettingsAction::LoadSelected,
                ),
                ("Save All Mixer Settings", ChannelSettingsAction::SaveAll),
                ("Load All Mixer Settings", ChannelSettingsAction::LoadAll),
            ] {
                if ui.button(label).clicked() {
                    state.pending_action = Some(action);
                    ui.close();
                }
            }
        });
}

#[derive(Clone)]
pub(super) enum InsertSelection {
    Add(InsertEffectKind),
    Remove,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InsertMenuCategory {
    Dynamics,
    Eq,
    Delay,
    Distortion,
    Filter,
    Mastering,
    Modulation,
    PitchShift,
    Restoration,
    Reverb,
    Spatial,
    Tools,
    Vocoder,
    Analyzer,
    Other,
}

impl InsertMenuCategory {
    const ALL: [Self; 15] = [
        Self::Dynamics,
        Self::Eq,
        Self::Delay,
        Self::Distortion,
        Self::Filter,
        Self::Mastering,
        Self::Modulation,
        Self::PitchShift,
        Self::Restoration,
        Self::Reverb,
        Self::Spatial,
        Self::Tools,
        Self::Vocoder,
        Self::Analyzer,
        Self::Other,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Dynamics => "Dynamics",
            Self::Eq => "EQ",
            Self::Delay => "Delay",
            Self::Distortion => "Distortion",
            Self::Filter => "Filter",
            Self::Mastering => "Mastering",
            Self::Modulation => "Modulation",
            Self::PitchShift => "Pitch Shift",
            Self::Restoration => "Restoration",
            Self::Reverb => "Reverb",
            Self::Spatial => "Spatial",
            Self::Tools => "Tools",
            Self::Vocoder => "Vocoder",
            Self::Analyzer => "Analyzer",
            Self::Other => "Other",
        }
    }

    const fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::Dynamics => &[
                "dynamics",
                "compressor",
                "limiter",
                "gate",
                "expander",
                "maximizer",
                "deesser",
                "multibandcompressor",
                "vintagedynamics",
                "vstdynamics",
            ],
            Self::Eq => &["eq", "equalizer", "equaliser", "parametric"],
            Self::Delay => &["delay", "echo", "pingpong", "stereodelay"],
            Self::Distortion => &[
                "distortion",
                "saturation",
                "overdrive",
                "fuzz",
                "bitcrusher",
                "drive",
            ],
            Self::Filter => &["filter", "lowpass", "highpass", "bandpass", "notch", "comb"],
            Self::Mastering => &["mastering", "master"],
            Self::Modulation => &[
                "modulation",
                "chorus",
                "flanger",
                "phaser",
                "tremolo",
                "vibrato",
                "rotary",
                "ringmod",
                "autopan",
            ],
            Self::PitchShift => &[
                "pitch",
                "pitchshift",
                "harmony",
                "harmonizer",
                "octaver",
                "transpose",
            ],
            Self::Restoration => &[
                "restoration",
                "noisereduction",
                "denoise",
                "declick",
                "decrackle",
                "dehum",
                "declipper",
            ],
            Self::Reverb => &["reverb", "convolution", "room", "hall", "plate", "spring"],
            Self::Spatial => &[
                "spatial",
                "surround",
                "3d",
                "ambisonic",
                "immersive",
                "binaural",
            ],
            Self::Tools => &["tools", "tool", "utility", "helper", "diagnostic", "test"],
            Self::Vocoder => &["vocoder"],
            Self::Analyzer => &[
                "analyzer", "analyser", "meter", "analysis", "spectrum", "scope",
            ],
            Self::Other => &[],
        }
    }
}

fn normalized_insert_subcategory(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn insert_category_from_subcategories<'a>(
    subcategories: impl IntoIterator<Item = &'a str>,
) -> InsertMenuCategory {
    let normalized = subcategories
        .into_iter()
        .map(normalized_insert_subcategory)
        .filter(|subcategory| !subcategory.is_empty())
        .collect::<Vec<_>>();
    for category in InsertMenuCategory::ALL {
        if normalized
            .iter()
            .any(|subcategory| category.aliases().contains(&subcategory.as_str()))
        {
            return category;
        }
    }
    InsertMenuCategory::Other
}

fn vst3_insert_category(plugin: &PluginDescriptor) -> InsertMenuCategory {
    let mapped =
        insert_category_from_subcategories(plugin.subcategories.iter().map(String::as_str));
    if mapped == InsertMenuCategory::Other && plugin.category == PluginCategory::Analyzer {
        InsertMenuCategory::Analyzer
    } else {
        mapped
    }
}

fn is_insert_catalog_plugin(plugin: &PluginDescriptor) -> bool {
    plugin.is_usable() && !matches!(plugin.category, PluginCategory::Instrument)
}

fn default_vst3_plugin_folders() -> Vec<PathBuf> {
    opentrack_plugin_scan::default_vst3_folders()
        .into_iter()
        .filter(|folder| folder.is_dir())
        .collect()
}

fn sort_vst3_plugin_catalog(catalog: &mut [PluginDescriptor]) {
    catalog.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.vendor.to_lowercase().cmp(&right.vendor.to_lowercase()))
            .then_with(|| left.class_id.cmp(&right.class_id))
    });
}

fn catalog_plugins_for_category<'a>(
    catalog: &'a [PluginDescriptor],
    category: InsertMenuCategory,
) -> Vec<&'a PluginDescriptor> {
    let mut plugins = catalog
        .iter()
        .filter(|plugin| {
            is_insert_catalog_plugin(plugin) && vst3_insert_category(plugin) == category
        })
        .collect::<Vec<_>>();
    plugins.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.vendor.to_lowercase().cmp(&right.vendor.to_lowercase()))
            .then_with(|| left.class_id.cmp(&right.class_id))
    });
    plugins
}

fn vst3_insert_effect(plugin: &PluginDescriptor) -> InsertEffectKind {
    InsertEffectKind::Vst3 {
        class_id: plugin.class_id.to_hex(),
        module_path: plugin.module_path.clone(),
        name: plugin.name.clone(),
        vendor: plugin.vendor.clone(),
        component_state: Vec::new(),
        controller_state: Vec::new(),
    }
}

fn show_catalog_insert_items(
    ui: &mut egui::Ui,
    catalog: &[PluginDescriptor],
    category: InsertMenuCategory,
    added: &mut Option<InsertSelection>,
) {
    let plugins = catalog_plugins_for_category(catalog, category);
    if plugins.is_empty() {
        return;
    }
    ui.separator();
    for plugin in plugins {
        let hover = if plugin.vendor.is_empty() {
            plugin.module_path.display().to_string()
        } else {
            format!("{}\n{}", plugin.vendor, plugin.module_path.display())
        };
        if ui.button(&plugin.name).on_hover_text(hover).clicked() {
            *added = Some(InsertSelection::Add(vst3_insert_effect(plugin)));
            ui.close();
        }
    }
}

fn show_insert_effect_picker(
    ui: &mut egui::Ui,
    added: &mut Option<InsertSelection>,
    catalog: &[PluginDescriptor],
) {
    if ui.button("No Effect").clicked() {
        *added = Some(InsertSelection::Remove);
        ui.close();
    }
    ui.separator();
    ui.menu_button("Delay", |ui| {
        for name in [
            "DoubleDelay",
            "ModDelay",
            "ModMachine",
            "MonoDelay",
            "PingPongDelay",
        ] {
            ui.add_enabled(false, egui::Button::new(name));
        }
        if ui.button("StereoDelay").clicked() {
            *added = Some(InsertSelection::Add(InsertEffectKind::default()));
            ui.close();
        }
        show_catalog_insert_items(ui, catalog, InsertMenuCategory::Delay, added);
    });
    ui.menu_button("Distortion", |ui| {
        if catalog_plugins_for_category(catalog, InsertMenuCategory::Distortion).is_empty() {
            show_unimplemented_effect_group(ui);
        }
        show_catalog_insert_items(ui, catalog, InsertMenuCategory::Distortion, added);
    });
    ui.menu_button("Dynamics", |ui| {
        for name in [
            "DeEsser",
            "EnvelopeShaper",
            "Expander",
            "Gate",
            "Limiter",
            "Maximizer",
            "MIDI Gate",
            "MultibandCompressor",
            "VintageCompressor",
            "VSTDynamics",
        ] {
            ui.add_enabled(false, egui::Button::new(name));
        }
        show_catalog_insert_items(ui, catalog, InsertMenuCategory::Dynamics, added);
    });
    ui.menu_button("EQ", |ui| {
        for name in ["GEQ-10", "GEQ-30", "StudioEQ"] {
            ui.add_enabled(false, egui::Button::new(name));
        }
        show_catalog_insert_items(ui, catalog, InsertMenuCategory::Eq, added);
    });
    for category in [
        InsertMenuCategory::Filter,
        InsertMenuCategory::Mastering,
        InsertMenuCategory::Modulation,
        InsertMenuCategory::Other,
        InsertMenuCategory::PitchShift,
        InsertMenuCategory::Restoration,
        InsertMenuCategory::Reverb,
        InsertMenuCategory::Spatial,
    ] {
        ui.menu_button(category.label(), |ui| {
            if catalog_plugins_for_category(catalog, category).is_empty() {
                show_unimplemented_effect_group(ui);
            }
            show_catalog_insert_items(ui, catalog, category, added);
        });
    }
    ui.menu_button("Surround", |ui| show_unimplemented_effect_group(ui));
    ui.menu_button("SurroundDither", |ui| show_unimplemented_effect_group(ui));
    for category in [
        InsertMenuCategory::Tools,
        InsertMenuCategory::Vocoder,
        InsertMenuCategory::Analyzer,
    ] {
        ui.menu_button(category.label(), |ui| {
            if catalog_plugins_for_category(catalog, category).is_empty() {
                show_unimplemented_effect_group(ui);
            }
            show_catalog_insert_items(ui, catalog, category, added);
        });
    }
    ui.separator();
    ui.menu_button("OpenTrack Effects", |ui| {
        if ui.button("Test Generator").clicked() {
            *added = Some(InsertSelection::Add(InsertEffectKind::TestGenerator {
                waveform: GeneratorWaveform::Sine,
                frequency_hz: 440.0,
                gain_db: -18.0,
            }));
            ui.close();
        }
    });
}

fn show_unimplemented_effect_group(ui: &mut egui::Ui) {
    ui.add_enabled(false, egui::Button::new("No implemented effects"));
}

fn insert_effect_name(effect: &InsertEffectKind) -> String {
    match effect {
        InsertEffectKind::TestGenerator { .. } => "Test Generator".to_owned(),
        InsertEffectKind::StereoDelay { .. } => "Stereo Delay".to_owned(),
        InsertEffectKind::LegacyUnavailableCompressor { .. } => "Unavailable Compressor".to_owned(),
        InsertEffectKind::Vst3 { name, .. } if !name.is_empty() => name.clone(),
        InsertEffectKind::Vst3 { .. } => "VST3 Plug-in".to_owned(),
    }
}

fn show_eq_dial(
    ui: &mut egui::Ui,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    logarithmic: bool,
    modified: &mut bool,
) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::click_and_drag());
    if response.dragged() {
        let delta = -ui.input(|input| input.pointer.delta().y);
        let old = *value;
        if logarithmic {
            let range = maximum.ln() - minimum.ln();
            *value = (value.ln() + delta * range / 180.0)
                .exp()
                .clamp(minimum, maximum);
        } else {
            *value = (*value + delta * (maximum - minimum) / 180.0).clamp(minimum, maximum);
        }
        *modified |= old != *value;
    }
    let normalized = if logarithmic {
        ((*value).ln() - minimum.ln()) / (maximum.ln() - minimum.ln())
    } else {
        (*value - minimum) / (maximum - minimum)
    };
    mixer::knob_graphic(ui, rect, normalized.clamp(0.0, 1.0) * 48.0 - 24.0);
}

fn linear_gain_db(gain: f32) -> f32 {
    if gain <= 0.0 {
        -60.0
    } else {
        20.0 * gain.log10()
    }
}

#[cfg(test)]
fn retain_channel_peak(holds: &mut HashMap<u64, metering::Hold>, id: u64, peaks: [f32; 2]) {
    let held = holds.entry(id).or_default();
    for peak in peaks {
        if peak.is_finite() {
            held.maximum = held.maximum.max(peak);
        }
    }
}
fn meter_fill_fraction(peak: f32) -> f32 {
    if !peak.is_finite() || peak <= 0.0 {
        return 0.0;
    }
    ((20.0 * peak.log10() + 60.0) / (60.0 + channel_gain::MAX_DB)).clamp(0.0, 1.0)
}
fn paint_peak_markers(ui: &egui::Ui, rect: Rect, peaks: [f32; 2]) {
    paint_peak_markers_channels(ui, rect, peaks, 2);
}

fn paint_peak_markers_channels(ui: &egui::Ui, rect: Rect, peaks: [f32; 2], channels: u8) {
    let count = usize::from(channels.clamp(1, 2));
    let width = ((rect.width() - (count - 1) as f32) / count as f32).max(1.0);
    for i in 0..count {
        let peak = if count == 1 {
            peaks[0].max(peaks[1])
        } else {
            peaks[i]
        };
        if peak <= 0.001 {
            continue;
        }
        let y = (rect.bottom() - rect.height() * meter_fill_fraction(peak))
            .clamp(rect.top() + 0.5, rect.bottom() - 0.5);
        let x = rect.left() + i as f32 * (width + 1.0);
        ui.painter().with_clip_rect(rect).line_segment(
            [Pos2::new(x, y), Pos2::new(x + width, y)],
            Stroke::new(
                1.0,
                if peak > 1.0 {
                    theme::RECORD
                } else {
                    theme::palette(ui).text
                },
            ),
        );
    }
}

fn paint_track_meter(ui: &egui::Ui, rect: Rect, peaks: [f32; 2], color: Color32) {
    paint_track_meter_channels(ui, rect, peaks, color, 2);
}

fn paint_track_meter_channels(
    ui: &egui::Ui,
    rect: Rect,
    peaks: [f32; 2],
    color: Color32,
    channels: u8,
) {
    let palette = theme::palette(ui);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 0.0, palette.canvas);
    let count = usize::from(channels.clamp(1, 2));
    let width = ((rect.width() - (count - 1) as f32) / count as f32).max(1.0);
    let zero_y = rect.bottom() - rect.height() * meter_fill_fraction(1.0);
    for index in 0..count {
        let peak = if count == 1 {
            peaks[0].max(peaks[1])
        } else {
            peaks[index]
        };
        let channel = Rect::from_min_size(
            rect.min + Vec2::new(index as f32 * (width + 1.0), 0.0),
            Vec2::new(width, rect.height()),
        );
        painter.rect_filled(
            Rect::from_min_max(channel.min, Pos2::new(channel.right(), zero_y)),
            0.0,
            theme::RECORD.gamma_multiply(0.16),
        );
        let top = channel.bottom() - channel.height() * meter_fill_fraction(peak);
        if top < channel.bottom() {
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(channel.left(), top.max(zero_y)), channel.max),
                0.0,
                color,
            );
            if peak > 1.0 {
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(channel.left(), top),
                        Pos2::new(channel.right(), zero_y),
                    ),
                    0.0,
                    theme::RECORD,
                );
            }
        }
        painter.line_segment(
            [
                Pos2::new(channel.left(), zero_y),
                Pos2::new(channel.right(), zero_y),
            ],
            Stroke::new(0.6, palette.muted),
        );
    }
}

#[cfg(test)]
mod meter_tests {
    use super::*;
    #[test]
    fn mono_meter_is_one_full_lane_and_stereo_keeps_two_separated_lanes() {
        for channels in [1, 2] {
            let ctx = egui::Context::default();
            let rect = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(16.0, 100.0));
            let color = Color32::from_rgb(41, 179, 102);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                paint_track_meter_channels(ui, rect, [0.5, 0.25], color, channels);
            });
            let lanes: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(shape) if shape.fill == color => Some(shape.rect),
                    _ => None,
                })
                .collect();
            assert_eq!(lanes.len(), usize::from(channels));
            if channels == 1 {
                assert_eq!(lanes[0].width(), rect.width());
                assert!(
                    (lanes[0].height() - rect.height() * meter_fill_fraction(0.5)).abs() < 0.001
                );
            } else {
                assert_eq!(lanes[1].left() - lanes[0].right(), 1.0);
                assert!(lanes[0].height() > lanes[1].height());
            }
            output.textures_delta.clear();
        }
    }
    #[test]
    fn stereo_meter_only_lights_red_above_unity() {
        let ctx = egui::Context::default();
        let color = Color32::from_rgb(76, 142, 210);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint_track_meter(
                ui,
                Rect::from_min_size(Pos2::ZERO, Vec2::new(21.0, 100.0)),
                [1.0, 2.0],
                color,
            );
        });
        assert!(
            out.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.fill == color
            )),
            "the meter fill must use the channel color"
        );
        let red: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.fill == theme::RECORD => Some(r.rect),
                _ => None,
            })
            .collect();
        assert_eq!(red.len(), 1);
        assert!(red[0].left() > 10.0);
        assert!((red[0].bottom() - 100.0 * (1.0 - meter_fill_fraction(1.0))).abs() < 0.01);
        out.textures_delta.clear();
    }
    #[test]
    fn peaks_hold_across_silence_and_isolate_channels_until_reset() {
        let mut holds = HashMap::new();
        retain_channel_peak(&mut holds, 1, [0.5, 1.25]);
        retain_channel_peak(&mut holds, 1, [0.0, 0.0]);
        retain_channel_peak(&mut holds, 2, [0.25, 0.1]);
        assert_eq!(holds[&1].maximum, 1.25);
        assert_eq!(holds[&2].maximum, 0.25);
        retain_channel_peak(&mut holds, 1, [f32::NAN, f32::INFINITY]);
        assert_eq!(holds[&1].maximum, 1.25);
        *holds.get_mut(&1).unwrap() = metering::Hold::default();
        retain_channel_peak(&mut holds, 1, [0.5, 0.0]);
        assert_eq!(holds[&1].maximum, 0.5);
        assert!(meter_fill_fraction(1.0) < meter_fill_fraction(1.25));
    }

    #[test]
    fn mixer_meter_maps_minus_sixty_through_positive_headroom() {
        assert_eq!(meter_fill_fraction(0.0), 0.0);
        assert_eq!(meter_fill_fraction(0.001), 0.0);
        assert!(
            (meter_fill_fraction(0.01) - 20.0 / (60.0 + super::channel_gain::MAX_DB)).abs()
                < 1.0e-6
        );
        assert!(
            (meter_fill_fraction(1.0) - 60.0 / (60.0 + super::channel_gain::MAX_DB)).abs()
                < 0.000001
        );
        assert_eq!(meter_fill_fraction(4.0), 1.0);
    }
}

const MIDI_RECORD_MIN_NOTE_BEATS: f64 = 1.0 / 960.0;
const MIDI_RECORD_MAX_PART_BEATS: f64 = 1_000_000.0;

fn record_midi_message(
    project: &mut Project,
    session: &mut MidiRecordingSession,
    message: MidiInputMessage,
    event_beats: f64,
) -> bool {
    if !event_beats.is_finite() || !session.start_beats.is_finite() {
        return false;
    }
    let event_relative_beats = (event_beats - session.start_beats)
        .max(0.0)
        .min(MIDI_RECORD_MAX_PART_BEATS);
    let mut captured = false;
    for recording_track in &mut session.tracks {
        let Some(track_index) = project
            .tracks
            .iter()
            .position(|track| track.id == recording_track.track_id)
        else {
            continue;
        };
        if !matches!(
            project.tracks[track_index].kind,
            TrackKind::Midi | TrackKind::Instrument
        ) {
            continue;
        }
        match message {
            MidiInputMessage::NoteOn {
                channel,
                key,
                velocity,
            } if velocity > 0 => {
                let note_start = event_relative_beats
                    .min(MIDI_RECORD_MAX_PART_BEATS - MIDI_RECORD_MIN_NOTE_BEATS);
                let note_end = note_start + MIDI_RECORD_MIN_NOTE_BEATS;
                if ensure_midi_record_part_end(
                    project,
                    recording_track.track_id,
                    recording_track.part_id,
                    note_end,
                )
                .is_none()
                {
                    continue;
                }
                if let Some(note_id) = project.add_midi_note_on_channel(
                    track_index,
                    recording_track.part_id,
                    note_start,
                    MIDI_RECORD_MIN_NOTE_BEATS,
                    key.min(127),
                    channel.min(15),
                    velocity.clamp(1, 127),
                ) {
                    recording_track
                        .open_notes
                        .entry((channel.min(15), key.min(127)))
                        .or_default()
                        .push(OpenMidiNote {
                            note_id,
                            start_beats: note_start,
                        });
                    captured = true;
                }
            }
            MidiInputMessage::NoteOn { .. } => {}
            MidiInputMessage::NoteOff { channel, key, .. } => {
                let note_key = (channel.min(15), key.min(127));
                let (open_note, remove_key) = match recording_track.open_notes.get_mut(&note_key) {
                    Some(notes) => {
                        let open_note = notes.pop();
                        (open_note, notes.is_empty())
                    }
                    None => (None, false),
                };
                if remove_key {
                    recording_track.open_notes.remove(&note_key);
                }
                let Some(open_note) = open_note else {
                    continue;
                };
                let note_end = event_relative_beats
                    .max(open_note.start_beats + MIDI_RECORD_MIN_NOTE_BEATS)
                    .min(MIDI_RECORD_MAX_PART_BEATS);
                let Some(track_index) = ensure_midi_record_part_end(
                    project,
                    recording_track.track_id,
                    recording_track.part_id,
                    note_end,
                ) else {
                    continue;
                };
                if let Some(note) = project.tracks[track_index]
                    .midi_parts
                    .iter_mut()
                    .find(|part| part.id == recording_track.part_id)
                    .and_then(|part| {
                        part.notes
                            .iter_mut()
                            .find(|note| note.id == open_note.note_id)
                    })
                {
                    note.duration_beats = note_end - open_note.start_beats;
                    captured = true;
                }
            }
            MidiInputMessage::ControlChange {
                channel,
                controller,
                value,
            } => {
                if ensure_midi_record_part_end(
                    project,
                    recording_track.track_id,
                    recording_track.part_id,
                    event_relative_beats,
                )
                .is_none()
                {
                    continue;
                }
                if project
                    .add_midi_controller_event_on_channel(
                        track_index,
                        recording_track.part_id,
                        event_relative_beats,
                        controller.min(127),
                        channel.min(15),
                        value.min(127),
                    )
                    .is_some()
                {
                    captured = true;
                }
            }
        }
    }
    if captured {
        session.captured_events = session.captured_events.saturating_add(1);
    }
    captured
}

fn ensure_midi_record_part_end(
    project: &mut Project,
    track_id: u64,
    part_id: u64,
    needed_end_beats: f64,
) -> Option<usize> {
    if !needed_end_beats.is_finite() || needed_end_beats < 0.0 {
        return None;
    }
    let track_index = project
        .tracks
        .iter()
        .position(|track| track.id == track_id)?;
    let part = project.tracks[track_index]
        .midi_parts
        .iter_mut()
        .find(|part| part.id == part_id)?;
    part.duration_beats = part
        .duration_beats
        .max(needed_end_beats.min(MIDI_RECORD_MAX_PART_BEATS));
    Some(track_index)
}

fn finish_midi_recording(
    project: &mut Project,
    mut session: MidiRecordingSession,
    end_beats: f64,
    beats_per_bar: f64,
) {
    let minimum_part_beats = beats_per_bar.max(1.0);
    let requested_end = if end_beats.is_finite() {
        (end_beats - session.start_beats).max(0.0)
    } else {
        0.0
    };
    for recording_track in &mut session.tracks {
        let open_notes = std::mem::take(&mut recording_track.open_notes);
        for ((channel, key), notes) in open_notes {
            for open_note in notes {
                let note_end = requested_end
                    .max(open_note.start_beats + MIDI_RECORD_MIN_NOTE_BEATS)
                    .min(MIDI_RECORD_MAX_PART_BEATS);
                if let Some(track_index) = ensure_midi_record_part_end(
                    project,
                    recording_track.track_id,
                    recording_track.part_id,
                    note_end,
                ) {
                    if let Some(note) = project.tracks[track_index]
                        .midi_parts
                        .iter_mut()
                        .find(|part| part.id == recording_track.part_id)
                        .and_then(|part| {
                            part.notes.iter_mut().find(|note| {
                                note.id == open_note.note_id
                                    && note.channel == channel
                                    && note.pitch == key
                            })
                        })
                    {
                        note.duration_beats = note_end - open_note.start_beats;
                    }
                }
            }
        }

        let Some(track) = project
            .tracks
            .iter_mut()
            .find(|track| track.id == recording_track.track_id)
        else {
            continue;
        };
        let Some(part) = track
            .midi_parts
            .iter_mut()
            .find(|part| part.id == recording_track.part_id)
        else {
            continue;
        };
        let note_end = part
            .notes
            .iter()
            .map(|note| note.start_beats + note.duration_beats)
            .fold(0.0_f64, f64::max);
        let controller_end = part
            .controller_events
            .iter()
            .map(|event| event.position_beats)
            .fold(0.0_f64, f64::max);
        let content_end = requested_end.max(note_end).max(controller_end);
        let rounded_end = if content_end <= minimum_part_beats {
            minimum_part_beats
        } else {
            (content_end / minimum_part_beats).ceil() * minimum_part_beats
        };
        part.duration_beats = rounded_end
            .max(minimum_part_beats)
            .min(MIDI_RECORD_MAX_PART_BEATS);
    }
}

fn default_key_scope(command: &CommandId) -> BindingScope {
    if command.as_str() == "edit.open_close_editor" {
        BindingScope::Contexts(vec!["editor".to_owned()])
    } else {
        BindingScope::Global
    }
}

fn unique_workspace_preset_name(base: &str, existing: &[WorkspacePreset]) -> String {
    let base = if base.trim().is_empty() {
        "Workspace Preset"
    } else {
        base.trim()
    };
    if existing.iter().all(|preset| preset.name != base) {
        return base.to_owned();
    }
    for suffix in 2..u32::MAX {
        let candidate = format!("{base} {suffix}");
        if existing.iter().all(|preset| preset.name != candidate) {
            return candidate;
        }
    }
    format!("{base} Copy")
}

fn format_key_chord(chord: KeyChord) -> String {
    let modifiers = chord.modifiers();
    let mut parts = Vec::with_capacity(5);
    if modifiers.control {
        parts.push("Ctrl".to_owned());
    }
    if modifiers.alt {
        parts.push("Alt".to_owned());
    }
    if modifiers.shift {
        parts.push("Shift".to_owned());
    }
    if modifiers.windows {
        parts.push("Win".to_owned());
    }
    parts.push(match chord.key() {
        KeyCode::Letter(letter) => letter.to_ascii_uppercase().to_string(),
        KeyCode::Digit(digit) => digit.to_string(),
        KeyCode::Function(number) => format!("F{number}"),
        KeyCode::Arrow(ArrowKey::Up) => "Up".to_owned(),
        KeyCode::Arrow(ArrowKey::Down) => "Down".to_owned(),
        KeyCode::Arrow(ArrowKey::Left) => "Left".to_owned(),
        KeyCode::Arrow(ArrowKey::Right) => "Right".to_owned(),
        KeyCode::Enter => "Return".to_owned(),
        KeyCode::Escape => "Escape".to_owned(),
        KeyCode::Space => "Space".to_owned(),
        KeyCode::Insert => "Insert".to_owned(),
        KeyCode::Delete => "Delete".to_owned(),
        KeyCode::Backspace => "Backspace".to_owned(),
        KeyCode::Home => "Home".to_owned(),
        KeyCode::End => "End".to_owned(),
        KeyCode::Comma => "Comma".to_owned(),
        KeyCode::Minus => "Minus".to_owned(),
        KeyCode::Period => "Period".to_owned(),
        KeyCode::Plus => "Plus".to_owned(),
        KeyCode::Slash => "Slash".to_owned(),
        KeyCode::NumpadDigit(digit) => format!("Numpad{digit}"),
        KeyCode::NumpadAdd => "NumpadAdd".to_owned(),
        KeyCode::NumpadSubtract => "NumpadSubtract".to_owned(),
        KeyCode::NumpadMultiply => "NumpadMultiply".to_owned(),
        KeyCode::NumpadDivide => "NumpadSlash".to_owned(),
        KeyCode::NumpadDecimal => "NumpadDecimal".to_owned(),
        KeyCode::NumpadComma => "NumpadComma".to_owned(),
    });
    parts.join("+")
}

fn format_transport_position(beats: f64, beats_per_bar: u8, beat_unit: u8) -> String {
    let meter_beats = beats.max(0.0) * f64::from(beat_unit.max(1)) / 4.0;
    let total_ticks = (meter_beats * 4.0 * 100.0).round() as u64;
    let ticks_per_bar = u64::from(beats_per_bar.max(1)) * 4 * 100;
    let bar = total_ticks / ticks_per_bar + 1;
    let within_bar = total_ticks % ticks_per_bar;
    let beat = within_bar / 400 + 1;
    let within_beat = within_bar % 400;
    let sixteenth = within_beat / 100 + 1;
    let tick = within_beat % 100;
    format!("{bar}.{beat}.{sixteenth}.{tick:02}")
}

fn safe_project_filename(project_name: &str) -> String {
    let sanitized: String = project_name
        .chars()
        .map(|character| {
            if character.is_control() || "<>:\"/\\|?*".contains(character) {
                '_'
            } else {
                character
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches(|character| character == ' ' || character == '.');
    let name = if sanitized.is_empty() {
        "Untitled Project"
    } else {
        sanitized
    };
    format!("{name}.opr")
}

fn recording_paths_for_tracks(
    base_path: &Path,
    tracks: &[(u64, String, Option<AudioRecordRoute>)],
) -> Vec<PathBuf> {
    if tracks.len() <= 1 {
        return vec![base_path.to_path_buf()];
    }

    let parent = base_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let base_name = base_path
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Take".to_owned());
    tracks
        .iter()
        .map(|(track_id, track_name, _)| {
            let track_name = track_name
                .chars()
                .map(|character| {
                    if matches!(
                        character,
                        '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                    ) {
                        '_'
                    } else {
                        character
                    }
                })
                .collect::<String>();
            let track_name = track_name
                .trim()
                .trim_end_matches(|character| character == ' ' || character == '.');
            let track_name = if track_name.is_empty() {
                "Track"
            } else {
                track_name
            };
            parent.join(format!("{base_name} - {track_name} ({track_id}).wav"))
        })
        .collect()
}

fn next_recording_paths(
    directory: &Path,
    tracks: &[(u64, String, Option<AudioRecordRoute>)],
    reserved: &HashSet<PathBuf>,
) -> Option<Vec<PathBuf>> {
    (1..=99_999).find_map(|number| {
        let base = directory.join(format!("Take {number:03}.wav"));
        let paths = recording_paths_for_tracks(&base, tracks);
        paths
            .iter()
            .all(|path| !path.exists() && !reserved.contains(path))
            .then_some(paths)
    })
}

fn audio_import_destination(source: &Path, project_root: &Path) -> Result<(PathBuf, bool), String> {
    let is_inside_project = match (fs::canonicalize(source), fs::canonicalize(project_root)) {
        (Ok(source), Ok(root)) => source.starts_with(root),
        _ => source.starts_with(project_root),
    };
    if is_inside_project {
        return Ok((source.to_path_buf(), false));
    }
    let directory = project_root.join("Audio").join("Imports");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let file_name = source
        .file_name()
        .ok_or_else(|| "the selected audio file has no name".to_owned())?;
    let file_name = file_name.to_string_lossy();
    let (stem, extension) = Path::new(file_name.as_ref())
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            (
                stem.to_owned(),
                Path::new(file_name.as_ref())
                    .extension()
                    .map(|ext| ext.to_os_string()),
            )
        })
        .unwrap_or_else(|| ("Audio".to_owned(), None));
    let candidate = directory.join(&*file_name);
    if !candidate.exists() {
        return Ok((candidate, true));
    }
    for index in 2..=99_999 {
        let name = match &extension {
            Some(extension) => format!("{stem} ({index}).{}", extension.to_string_lossy()),
            None => format!("{stem} ({index})"),
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return Ok((candidate, true));
        }
    }
    Err("no unused name is available in Audio/Imports".to_owned())
}

fn audio_import_temporary_path(destination: &Path) -> PathBuf {
    let sequence = NEXT_AUDIO_IMPORT_ID.fetch_add(1, Ordering::Relaxed);
    let name = destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    destination.with_file_name(format!(
        ".{name}.importing-{}-{sequence}",
        std::process::id()
    ))
}

fn project_folder_name(project_name: &str) -> String {
    Path::new(&safe_project_filename(project_name))
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("Untitled Project")
        .to_owned()
}

fn create_project_directory_layout(root: &Path) -> std::io::Result<()> {
    fs::create_dir_all(root.join("Audio").join("Recordings"))?;
    fs::create_dir_all(root.join("Audio").join("Imports"))?;
    fs::create_dir_all(root.join("Metadata"))
}

/// Files OpenTrack reads as a project.
const PROJECT_FILE_EXTENSIONS: [&str; 3] = ["opr", "otp", "rwp"];

/// True when `root` already holds a project file at its top level.
fn folder_holds_project(root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    PROJECT_FILE_EXTENSIONS
                        .iter()
                        .any(|known| extension.eq_ignore_ascii_case(known))
                })
    })
}

/// True when a new project may not take over its folder.
///
/// A folder that exists without a project file is adopted rather than rejected:
/// an earlier attempt, or a cancelled save, can leave the `Audio`/`Metadata`
/// layout behind before any project file is written.
fn project_folder_is_occupied(root: &Path, project_file: &Path) -> bool {
    project_file.exists() || (root.exists() && folder_holds_project(root))
}

fn project_directory_layout_exists(project_file: &Path) -> bool {
    let Some(root) = project_file.parent() else {
        return false;
    };
    root.join("Audio").join("Recordings").is_dir()
        && root.join("Audio").join("Imports").is_dir()
        && root.join("Metadata").is_dir()
}

fn unique_project_file_path(root: &Path, project_name: &str) -> Option<PathBuf> {
    let file_name = safe_project_filename(project_name);
    let candidate = root.join(&file_name);
    if !candidate.exists() {
        return Some(candidate);
    }
    let stem = Path::new(&file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Project");
    (2..=99_999)
        .map(|number| root.join(format!("{stem} ({number}).opr")))
        .find(|path| !path.exists())
}

fn unique_project_folder_path(parent: &Path, project_name: &str) -> PathBuf {
    let base = project_folder_name(project_name);
    let candidate = parent.join(&base);
    if !candidate.exists() {
        return candidate;
    }
    for number in 2..=99_999 {
        let candidate = parent.join(format!("{base} ({number})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{base} Copy"))
}

fn paths_refer_to_same_directory(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn project_file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn update_project_load_status(progress: &Mutex<ProjectLoadStatus>, fraction: f32, phase: String) {
    if let Ok(mut status) = progress.lock() {
        status.fraction = fraction.clamp(0.0, 1.0);
        status.phase = phase;
    }
}

/// Runs `add` with pointer input hidden while another layer owns the mouse.
///
/// `Ui::disable` stops widgets, but the timeline and rulers read the pointer
/// straight from the input state, so a click aimed at a dialog above them would
/// still select or drag underneath it. The input is restored afterwards so the
/// dialog that is drawn later in the frame still receives it.
fn blocked_input_scope(
    ui: &mut egui::Ui,
    pointer_blocked: bool,
    overlay_open: bool,
    add: impl FnOnce(&mut egui::Ui),
) {
    if pointer_blocked {
        let ctx = ui.ctx().clone();
        let saved = ctx.input(|input| input.clone());
        ctx.input_mut(|input| {
            input.pointer = egui::PointerState::default();
            input.events.clear();
            input.smooth_scroll_delta = Vec2::ZERO;
        });
        ui.disable();
        add(ui);
        ctx.input_mut(|input| *input = saved);
    } else {
        if overlay_open {
            ui.disable();
        }
        add(ui);
    }
}

fn dismiss_window_on_outside_click<R>(
    ctx: &egui::Context,
    id: Id,
    open: &mut bool,
    response: Option<&egui::InnerResponse<R>>,
) {
    let frame = ctx.cumulative_frame_nr();
    let was_visible_last_frame = ctx
        .data(|data| data.get_temp::<u64>(id))
        .is_some_and(|last_frame| last_frame.saturating_add(1) == frame);
    // Picking an item in a combo box closes the popup in the same frame the
    // click happens, so `is_any_open` is already false by the time this runs and
    // the window would close as if the click had landed outside it. Remember
    // that a popup was open last frame and ignore that click.
    let popup_open = egui::Popup::is_any_open(ctx);
    let popup_was_open = ctx
        .data(|data| data.get_temp::<bool>(id.with("popup-open")))
        .unwrap_or(false);
    if *open
        && was_visible_last_frame
        && !popup_open
        && !popup_was_open
        && ctx.input(|input| input.pointer.any_click())
        && response.is_some_and(|window| window.response.clicked_elsewhere())
    {
        *open = false;
    }
    ctx.data_mut(|data| {
        data.insert_temp(id.with("popup-open"), popup_open);
        if *open {
            data.insert_temp(id, frame);
        } else {
            data.remove_temp::<u64>(id);
        }
    });
}

fn project_display_name(path: Option<&Path>, fallback: &str) -> String {
    path.and_then(Path::file_stem)
        .filter(|stem| !stem.is_empty())
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| fallback.to_owned())
}

fn unresolved_audio_source_paths(
    project: &Project,
    audio_sources: &HashMap<PathBuf, AudioSource>,
) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    project
        .tracks
        .iter()
        .flat_map(|track| track.clips.iter())
        .filter(|clip| {
            audio_sources
                .get(&clip.source_path)
                .is_none_or(AudioSource::waveform_failed)
        })
        .map(|clip| clip.source_path.clone())
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn relink_audio_source_path(project: &mut Project, old_path: &Path, new_path: &Path) -> usize {
    let mut relinked = 0;
    if project.media_pool.remove(old_path) {
        project.media_pool.insert(new_path.to_path_buf());
    }
    for clip in project
        .tracks
        .iter_mut()
        .flat_map(|track| track.clips.iter_mut())
    {
        if clip.source_path == old_path {
            clip.source_path = new_path.to_path_buf();
            relinked += 1;
        }
    }
    relinked
}

#[cfg(test)]
mod project_folder_tests {
    use super::*;

    fn temp_root(label: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "opentrack-folder-adopt-{}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn a_folder_without_a_project_is_adopted() {
        let root = temp_root("adopt");
        create_project_directory_layout(&root).expect("layout should be created");
        assert!(!project_folder_is_occupied(
            &root,
            &root.join("EXAMPLE.opr")
        ));
        // Adopting must reuse the folders a previous attempt already created.
        create_project_directory_layout(&root).expect("layout should be reusable");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_holding_a_project_file_is_not_adopted() {
        let root = temp_root("occupied");
        create_project_directory_layout(&root).expect("layout should be created");
        fs::write(root.join("Example.OPR"), b"{}").expect("project file should be written");
        assert!(project_folder_is_occupied(&root, &root.join("EXAMPLE.opr")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_existing_target_project_file_is_not_overwritten() {
        let root = temp_root("target");
        fs::create_dir_all(&root).expect("folder should be created");
        let project_file = root.join("EXAMPLE.opr");
        fs::write(&project_file, b"{}").expect("project file should be written");
        assert!(project_folder_is_occupied(&root, &project_file));
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod toolbar_presentation_tests {
    use super::*;

    #[test]
    fn mixer_toolbar_button_tracks_and_toggles_the_real_window_state() {
        let ctx = egui::Context::default();
        let mut window = NativeWindowState::default();
        let mut frame = |events| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    toolbar_window_toggle(
                        ui,
                        ToolbarIcon::Mixer,
                        &mut window,
                        "Open Mixer",
                        "Close Mixer",
                    );
                },
            );
            out.textures_delta.clear();
            (window.open, window.focus_requested)
        };
        assert_eq!(frame(vec![]), (false, false));
        let button = |pressed| egui::Event::PointerButton {
            pos: Pos2::new(10.0, 10.0),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![
            egui::Event::PointerMoved(Pos2::new(10.0, 10.0)),
            button(true),
        ]);
        assert_eq!(frame(vec![button(false)]), (true, true));
        frame(vec![button(true)]);
        assert_eq!(frame(vec![button(false)]), (false, false));
    }

    #[test]
    fn enabled_dropdowns_have_strong_ink_in_both_themes() {
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let ctx = egui::Context::default();
            theme::install(&ctx);
            ctx.set_theme(if theme == egui::Theme::Dark {
                egui::ThemePreference::Dark
            } else {
                egui::ThemePreference::Light
            });
            let mut expected = Color32::TRANSPARENT;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 160.0))),
                    ..Default::default()
                },
                |ui| {
                    expected = if ui.visuals().dark_mode {
                        Color32::from_gray(230)
                    } else {
                        Color32::from_gray(24)
                    };
                    ui.horizontal(|ui| {
                        project_grid_choices(ui, &mut ArrangeViewState::default());
                        toolbar_choice(ui, "Touch", 84.0);
                    });
                    ui.horizontal(|ui| {
                        for icon in [
                            ToolbarIcon::Power,
                            ToolbarIcon::Constraint,
                            ToolbarIcon::Inspector,
                            ToolbarIcon::InfoLine,
                            ToolbarIcon::Overview,
                            ToolbarIcon::Pool,
                            ToolbarIcon::Mixer,
                            ToolbarIcon::Automation,
                            ToolbarIcon::Previous,
                            ToolbarIcon::Next,
                            ToolbarIcon::Cycle,
                            ToolbarIcon::Stop,
                            ToolbarIcon::Play,
                            ToolbarIcon::Record,
                            ToolbarIcon::Select,
                            ToolbarIcon::Range,
                            ToolbarIcon::Split,
                            ToolbarIcon::Glue,
                            ToolbarIcon::Erase,
                            ToolbarIcon::Zoom,
                            ToolbarIcon::Mute,
                            ToolbarIcon::TimeWarp,
                            ToolbarIcon::Draw,
                            ToolbarIcon::Line,
                            ToolbarIcon::PlayTool,
                            ToolbarIcon::Color,
                            ToolbarIcon::RootKey,
                            ToolbarIcon::Autoscroll,
                            ToolbarIcon::Snap,
                            ToolbarIcon::SnapType,
                            ToolbarIcon::ZeroCrossing,
                        ] {
                            toolbar_icon_button(
                                ui,
                                icon,
                                "",
                                matches!(icon, ToolbarIcon::Select | ToolbarIcon::Snap),
                                true,
                            );
                        }
                    });
                    ui.horizontal(|ui| {
                        toolbar_icon_button(ui, ToolbarIcon::Mixer, "", false, true);
                        toolbar_icon_button(ui, ToolbarIcon::Mixer, "", true, true);
                        toolbar_icon_button(ui, ToolbarIcon::Power, "", false, false);
                        toolbar_icon_button(ui, ToolbarIcon::Power, "", true, true);
                        for size in [12.0, 16.0, 20.0] {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
                            crate::icon_system::paint_app_icon(
                                ui,
                                r,
                                crate::icon_system::IconId::Power,
                                expected,
                            );
                        }
                    });
                },
            );
            // Headless rendering deliberately discards atlas updates, including
            // if a later assertion or optional artifact write fails.
            out.textures_delta.clear();
            let mut found = 0;
            fn visit(shape: &egui::Shape, expected: Color32, found: &mut usize) {
                match shape {
                    egui::Shape::Text(t) if ["Bar", "1/16"].contains(&t.galley.text()) => {
                        let color = t
                            .override_text_color
                            .unwrap_or(t.galley.job.sections[0].format.color);
                        assert_eq!(color, expected);
                        *found += 1;
                    }
                    egui::Shape::Vec(v) => {
                        for s in v {
                            visit(s, expected, found);
                        }
                    }
                    _ => {}
                }
            }
            for s in &out.shapes {
                visit(&s.shape, expected, &mut found);
            }
            assert_eq!(found, 2);
            if let Ok(prefix) = std::env::var("OPENTRACK_ICON_REVIEW_JSON") {
                fn color(c: Color32) -> serde_json::Value {
                    serde_json::json!(c.to_srgba_unmultiplied())
                }
                fn pos(p: Pos2) -> [f32; 2] {
                    [p.x, p.y]
                }
                fn export(s: &egui::Shape, items: &mut Vec<serde_json::Value>) {
                    use serde_json::json;
                    match s {
                        egui::Shape::Vec(v)=>for s in v { export(s,items); },
                        egui::Shape::Rect(r)=>items.push(json!({"kind":"rect","min":pos(r.rect.min),"max":pos(r.rect.max),"fill":color(r.fill),"stroke":color(r.stroke.color),"width":r.stroke.width,"radius":r.corner_radius.nw})),
                        egui::Shape::Circle(c)=>items.push(json!({"kind":"circle","center":pos(c.center),"radius":c.radius,"fill":color(c.fill),"stroke":color(c.stroke.color),"width":c.stroke.width})),
                        egui::Shape::LineSegment{points,stroke}=>items.push(json!({"kind":"path","points":points.map(pos),"closed":false,"stroke":color(stroke.color),"width":stroke.width})),
                        egui::Shape::Path(p)=>items.push(json!({"kind":"path","points":p.points.iter().copied().map(pos).collect::<Vec<_>>(),"closed":p.closed,"fill":color(p.fill),"stroke":color(match p.stroke.color { egui::epaint::ColorMode::Solid(c)=>c,_=>Color32::TRANSPARENT }),"width":p.stroke.width})),
                        egui::Shape::Text(t)=>items.push(json!({"kind":"text","pos":pos(t.pos),"text":t.galley.text(),"size":t.galley.job.sections[0].format.font_id.size,"fill":color(t.override_text_color.unwrap_or(t.galley.job.sections[0].format.color))})),
                        _=>{}
                    }
                }
                let mut items = Vec::new();
                for s in &out.shapes {
                    export(&s.shape, &mut items);
                }
                std::fs::write(
                    format!(
                        "{prefix}-{}.json",
                        if theme == egui::Theme::Dark {
                            "dark"
                        } else {
                            "light"
                        }
                    ),
                    serde_json::to_vec(&items).unwrap(),
                )
                .unwrap();
            }
            out.textures_delta.clear();
        }
    }
}

#[cfg(test)]
mod ruler_interaction_tests {
    use super::*;

    struct Fixture {
        ctx: egui::Context,
        project: Project,
        view: ArrangeViewState,
        position: f64,
        cursor: egui::CursorIcon,
        ruler_labels: Vec<String>,
        track_label_x: HashMap<String, f32>,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                ctx: egui::Context::default(),
                project: Project::default(),
                view: ArrangeViewState {
                    show_inspector: false,
                    ..Default::default()
                },
                position: 0.0,
                cursor: egui::CursorIcon::Default,
                ruler_labels: Vec::new(),
                track_label_x: HashMap::new(),
            }
        }

        fn frame(&mut self, events: Vec<egui::Event>) -> bool {
            let ctx = self.ctx.clone();
            let mut modified = false;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    modified = show_project(
                        ui,
                        &mut EditTool::Select,
                        &mut self.project,
                        &mut self.position,
                        true,
                        4,
                        44_100,
                        &HashMap::new(),
                        &HashMap::new(),
                        &HashMap::new(),
                        None,
                        &mut self.view,
                        &mut false,
                        &mut None,
                        &mut NativeWindowState::default(),
                        &mut None,
                        true,
                        &mut None,
                    )
                    .modified;
                },
            );
            self.cursor = output.platform_output.cursor_icon;
            self.ruler_labels.clear();
            self.track_label_x.clear();
            fn collect_labels(shape: &egui::Shape, labels: &mut Vec<String>) {
                match shape {
                    egui::Shape::Text(text)
                        if text.pos.x >= 198.0
                            && text.pos.x < 1200.0
                            && text.pos.y >= 0.0
                            && text.pos.y < 22.0 =>
                    {
                        labels.push(text.galley.text().to_owned())
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect_labels(shape, labels);
                        }
                    }
                    _ => {}
                }
            }
            let track_names = self
                .project
                .tracks
                .iter()
                .map(|track| track.name.as_str())
                .collect::<Vec<_>>();
            fn collect_track_labels(
                shape: &egui::Shape,
                names: &[&str],
                positions: &mut HashMap<String, f32>,
            ) {
                match shape {
                    egui::Shape::Text(text)
                        if names.iter().any(|name| *name == text.galley.text()) =>
                    {
                        positions.insert(text.galley.text().to_owned(), text.pos.x);
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect_track_labels(shape, names, positions);
                        }
                    }
                    _ => {}
                }
            }
            for shape in &output.shapes {
                collect_labels(&shape.shape, &mut self.ruler_labels);
                collect_track_labels(&shape.shape, &track_names, &mut self.track_label_x);
            }
            output.textures_delta.clear();
            modified
        }

        fn button(&mut self, x: f32, y: f32, pressed: bool) -> bool {
            let pos = Pos2::new(x, y);
            self.frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ])
        }
    }

    #[test]
    fn project_track_drag_into_group_sets_parent_and_collapses_with_group() {
        for source_above in [false, true] {
            let mut f = Fixture::new();
            for kind in if source_above {
                [TrackKind::Audio, TrackKind::Group]
            } else {
                [TrackKind::Group, TrackKind::Audio]
            } {
                f.project.add_track(kind);
            }
            let group_id = f
                .project
                .tracks
                .iter()
                .find(|t| t.kind == TrackKind::Group)
                .unwrap()
                .id;
            let child_id = f
                .project
                .tracks
                .iter()
                .find(|t| t.kind == TrackKind::Audio)
                .unwrap()
                .id;
            for id in [group_id, child_id] {
                f.view.set_track_height(id, 80.0);
            }
            f.frame(vec![]);
            f.frame(vec![]);
            let source_y = if source_above { 32.0 } else { 112.0 };
            let target_y = if source_above { 145.0 } else { 65.0 };
            f.button(120.0, source_y, true);
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(120.0, target_y))]);
            f.frame(vec![]);
            assert_eq!(f.view.pending_group_drop, Some((group_id, vec![child_id])));
            f.button(120.0, target_y, false);
            assert_eq!(
                f.project
                    .tracks
                    .iter()
                    .find(|t| t.id == child_id)
                    .unwrap()
                    .parent_group_id,
                Some(group_id)
            );
            f.frame(vec![]);
            let group_name = f
                .project
                .tracks
                .iter()
                .find(|track| track.id == group_id)
                .unwrap()
                .name
                .clone();
            let child_name = f
                .project
                .tracks
                .iter()
                .find(|track| track.id == child_id)
                .unwrap()
                .name
                .clone();
            assert_eq!(project_track_group_depth(&f.project, child_id), 1);
            assert!(
                f.track_label_x[&child_name] - f.track_label_x[&group_name] >= 23.0,
                "child label must visibly indent beneath its group"
            );
            f.button(9.0, 33.0, true);
            f.button(9.0, 33.0, false);
            assert!(
                f.project
                    .tracks
                    .iter()
                    .find(|t| t.id == group_id)
                    .unwrap()
                    .group_collapsed
            );
            assert!(track_hidden_by_collapsed_group(&f.project, child_id));
            let restored: Project =
                serde_json::from_str(&serde_json::to_string(&f.project).unwrap()).unwrap();
            assert_eq!(
                restored
                    .tracks
                    .iter()
                    .find(|t| t.id == child_id)
                    .unwrap()
                    .parent_group_id,
                Some(group_id)
            );
        }
    }

    #[test]
    fn project_group_drop_commits_from_mouse_release_over_group_with_inspector_open() {
        let mut f = Fixture::new();
        f.view.show_inspector = true;
        f.project.add_track(TrackKind::Audio);
        f.project.add_track(TrackKind::Group);
        let child_id = f.project.tracks[0].id;
        let group_id = f.project.tracks[1].id;
        for id in [child_id, group_id] {
            f.view.set_track_height(id, 80.0);
        }
        f.frame(vec![]);

        // Press in the audio header, then deliver the move and release in one
        // frame, as happens when a native drag skips a source-widget stop event.
        f.button(300.0, 32.0, true);
        let target = Pos2::new(300.0, 145.0);
        f.frame(vec![
            egui::Event::PointerMoved(target),
            egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        assert_eq!(
            f.project
                .tracks
                .iter()
                .find(|track| track.id == child_id)
                .unwrap()
                .parent_group_id,
            Some(group_id)
        );
    }

    #[test]
    #[ignore = "requires OPENTRACK_GROUP_PROJECT pointing to an explicit read-only project"]
    fn project_track_group_drop_in_explicit_project() {
        let path = std::env::var_os("OPENTRACK_GROUP_PROJECT").expect("explicit project path");
        let mut f = Fixture::new();
        f.project = crate::project_file::ProjectFile::read(Path::new(&path))
            .unwrap()
            .project;
        let child_id = f.project.tracks[0].id;
        f.project.add_track(TrackKind::Group);
        let group_id = f.project.tracks.last().unwrap().id;
        let target_y = 22.0 + (f.project.tracks.len() - 1) as f32 * 80.0 + 43.0;
        for track in &f.project.tracks {
            f.view.set_track_height(track.id, 80.0);
        }
        f.frame(vec![]);
        f.frame(vec![]);
        f.button(120.0, 32.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(120.0, target_y))]);
        f.frame(vec![]);
        assert_eq!(f.view.pending_group_drop, Some((group_id, vec![child_id])));
        f.button(120.0, target_y, false);
        assert_eq!(
            f.project
                .tracks
                .iter()
                .find(|t| t.id == child_id)
                .unwrap()
                .parent_group_id,
            Some(group_id)
        );
    }

    #[test]
    fn project_raw_wheel_gestures_zoom_time_scroll_tracks_and_zoom_heights_without_content_edits() {
        let mut f = Fixture::new();
        for _ in 0..20 {
            f.project.add_track(TrackKind::Audio);
        }
        let before = f.project.clone();
        f.frame(vec![]);
        let event = |modifiers, amount| {
            vec![
                egui::Event::PointerMoved(Pos2::new(600.0, 200.0)),
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: Vec2::new(0.0, amount),
                    phase: egui::TouchPhase::Move,
                    modifiers,
                },
            ]
        };
        assert!(!f.frame(event(egui::Modifiers::SHIFT, -1.0)));
        assert!(f.view.track_scroll_y > 0.0);
        assert_eq!(f.view.timeline_scroll_x, 0.0);
        let previous = f.view.track_vertical_zoom;
        assert!(!f.frame(event(
            egui::Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
            1.0
        )));
        assert!(f.view.track_vertical_zoom > previous);
        let previous = f.view.timeline_zoom;
        assert!(!f.frame(event(egui::Modifiers::NONE, 1.0)));
        assert!(f.view.timeline_zoom > previous);
        assert_eq!(f.project, before);
    }

    #[test]
    fn automation_expansion_reserves_a_separate_area_and_rejects_audio_event_drop_there() {
        let mut f = Fixture::new();
        f.project.add_track(TrackKind::Audio);
        f.project.add_track(TrackKind::Audio);
        for id in f.project.tracks.iter().map(|t| t.id).collect::<Vec<_>>() {
            f.view.set_track_height(id, 80.0);
        }
        f.view.automation_visible.insert(f.project.tracks[0].id);
        f.view.timeline_zoom = 16.0;
        f.view.grid_type = crate::timeline::GridType::UseQuantize;
        f.project
            .add_audio_clip(0, PathBuf::from("event.wav"), 1.0, 2.0)
            .unwrap();
        f.frame(vec![]);
        f.frame(vec![]);
        f.button(400.0, 60.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(580.0, 120.0))]);
        f.button(580.0, 120.0, false);
        assert_eq!(f.project.tracks[0].clips.len(), 1);
        assert!(f.project.tracks[1].clips.is_empty());
        f.button(400.0, 60.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(580.0, 180.0))]);
        f.button(580.0, 180.0, false);
        assert!(f.project.tracks[0].clips.is_empty());
        assert_eq!(f.project.tracks[1].clips[0].start_beats, 3.25);
    }

    #[test]
    fn full_project_grid_captures_audio_event_drag_without_seeking_or_scrolling() {
        let mut f = Fixture::new();
        f.project.add_track(TrackKind::Audio);
        f.project.add_track(TrackKind::Audio);
        for id in f.project.tracks.iter().map(|t| t.id).collect::<Vec<_>>() {
            f.view.set_track_height(id, 80.0);
        }
        f.view.timeline_zoom = 16.0;
        f.view.grid_type = crate::timeline::GridType::UseQuantize;
        f.project
            .add_audio_clip(0, PathBuf::from("event.wav"), 1.0, 2.0)
            .unwrap();
        f.frame(vec![]);
        f.frame(vec![]);
        f.button(400.0, 60.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(580.0, 140.0))]);
        f.frame(vec![]);
        f.button(580.0, 140.0, false);
        assert!(f.project.tracks[0].clips.is_empty());
        assert_eq!(f.project.tracks[1].clips.len(), 1);
        assert_eq!(f.project.tracks[1].clips[0].start_beats, 3.25);
        assert_eq!(f.position, 0.0);
        assert_eq!(f.view.timeline_scroll_x, 0.0);
    }

    #[test]
    fn actual_ruler_drag_snaps_to_bar_on_release_and_snap_off_leaves_it_continuous() {
        let mut f = Fixture::new();
        f.frame(vec![]);
        f.button(320.0, 16.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(387.0, 16.0))]);
        assert!(f.position > 0.0);
        assert!(
            f.position.rem_euclid(4.0) > 0.01,
            "drag must remain continuous before release"
        );
        assert!(!f.button(387.0, 16.0, false));
        assert_eq!(f.position.rem_euclid(4.0), 0.0);
        f.view.snap_enabled = false;
        f.button(320.0, 16.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(401.0, 16.0))]);
        f.button(401.0, 16.0, false);
        assert!(f.position.rem_euclid(4.0) > 0.01);
    }

    #[test]
    fn actual_ruler_paints_progressively_finer_note_labels_and_coarsens_again() {
        let mut f = Fixture::new();
        for (zoom, expected, absent) in [
            (1.0, "1", Some("1.2")),
            (16.0, "1.2", Some("1.1.3")),
            (32.0, "1.1.3", Some("1.1.2")),
            (64.0, "1.1.2", None),
            (1.0, "1", Some("1.2")),
        ] {
            f.view.timeline_zoom = zoom;
            f.frame(vec![]);
            assert!(
                f.ruler_labels.iter().any(|label| label == expected),
                "zoom {zoom}: {:?}",
                f.ruler_labels
            );
            if let Some(absent) = absent {
                assert!(!f.ruler_labels.iter().any(|label| label == absent));
            }
        }
    }

    #[test]
    fn actual_top_strip_drags_commit_directional_ranges_without_seeking() {
        let mut f = Fixture::new();
        f.frame(vec![]);
        let original = serde_json::to_value(&f.project).unwrap();
        f.button(320.0, 3.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(500.0, 3.0))]);
        assert_eq!(
            serde_json::to_value(&f.project).unwrap(),
            original,
            "preview must not edit the project"
        );
        assert!(f.button(500.0, 3.0, false));
        assert_eq!(f.project.cycle_mode, CycleMode::Loop);
        assert!(f.project.loop_end_beats > f.project.loop_start_beats);
        assert_eq!(f.project.loop_start_beats.rem_euclid(4.0), 0.0);
        assert_eq!(f.project.loop_end_beats.rem_euclid(4.0), 0.0);
        assert_eq!(f.position, 0.0);
        // Start beyond the existing range: its interior now moves the range.
        f.button(540.0, 3.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(320.0, 80.0))]);
        assert!(
            f.button(320.0, 80.0, false),
            "captured range must commit outside the ruler"
        );
        assert_eq!(f.project.cycle_mode, CycleMode::Skip);
        assert_eq!(f.position, 0.0);
    }

    #[test]
    fn selected_grid_controls_seek_draw_move_and_resize() {
        use crate::timeline::{GridType, QuantizeNote};
        for (grid_type, note, expected) in [
            (GridType::Bar, QuantizeNote::Eighth, 8.0),
            (GridType::Beat, QuantizeNote::Eighth, 9.0),
            (GridType::UseQuantize, QuantizeNote::Eighth, 9.5),
            (GridType::UseQuantize, QuantizeNote::Sixteenth, 9.25),
        ] {
            let mut f = Fixture::new();
            f.view.timeline_zoom = 4.0;
            f.view.grid_type = grid_type;
            f.view.quantize.note = note;
            f.frame(vec![]);
            f.button(382.0, 16.0, true);
            f.button(382.0, 16.0, false);
            assert_eq!(f.position, expected);
            f.button(320.0, 16.0, true);
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(382.0, 16.0))]);
            f.button(382.0, 16.0, false);
            assert_eq!(f.position, expected);
        }
        let mut f = Fixture::new();
        f.view.timeline_zoom = 4.0;
        f.view.grid_type = GridType::UseQuantize;
        f.view.quantize.note = QuantizeNote::Eighth;
        f.project.set_loop_range(100.0, 110.0);
        f.frame(vec![]);
        f.button(320.0, 3.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(382.0, 3.0))]);
        f.button(382.0, 3.0, false);
        assert_eq!(
            (f.project.loop_start_beats, f.project.loop_end_beats),
            (6.0, 9.5)
        );
        f.button(340.0, 3.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(382.0, 3.0))]);
        f.button(382.0, 3.0, false);
        assert_eq!(
            (f.project.loop_start_beats, f.project.loop_end_beats),
            (8.0, 11.5)
        );
        let right_x = 198.0 + 11.5 / 4.0 * ((1200.0 - 198.0 - 34.0) / 49.0 * 4.0);
        f.button(right_x + 2.0, 3.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(460.0, 3.0))]);
        f.button(460.0, 3.0, false);
        assert_eq!(
            (f.project.loop_start_beats, f.project.loop_end_beats),
            (8.0, 13.0)
        );
    }

    #[test]
    fn grid_and_quantize_popups_change_the_actual_settings() {
        fn frame(
            ctx: &egui::Context,
            view: &mut ArrangeViewState,
            events: Vec<egui::Event>,
        ) -> Vec<(String, Pos2)> {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    ui.horizontal(|ui| project_grid_choices(ui, view));
                },
            );
            fn collect(shape: &egui::Shape, labels: &mut Vec<(String, Pos2)>) {
                match shape {
                    egui::Shape::Text(text) => labels.push((
                        text.galley.text().to_owned(),
                        text.pos + Vec2::new(2.0, 4.0),
                    )),
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect(shape, labels);
                        }
                    }
                    _ => {}
                }
            }
            let mut labels = Vec::new();
            for shape in &out.shapes {
                collect(&shape.shape, &mut labels);
            }
            out.textures_delta.clear();
            labels
        }
        fn position(labels: &[(String, Pos2)], wanted: &str) -> Pos2 {
            labels
                .iter()
                .find(|(text, _)| text == wanted)
                .unwrap_or_else(|| panic!("missing {wanted}: {labels:?}"))
                .1
        }
        fn click(
            ctx: &egui::Context,
            view: &mut ArrangeViewState,
            pos: Pos2,
        ) -> Vec<(String, Pos2)> {
            for pressed in [true, false] {
                frame(
                    ctx,
                    view,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            frame(ctx, view, vec![])
        }
        let ctx = egui::Context::default();
        let mut view = ArrangeViewState::default();
        let labels = frame(&ctx, &mut view, vec![]);
        let labels = click(&ctx, &mut view, position(&labels, "Bar"));
        let labels = click(&ctx, &mut view, position(&labels, "Beat"));
        assert_eq!(view.grid_type, crate::timeline::GridType::Beat);
        let labels = click(&ctx, &mut view, position(&labels, "1/16"));
        let labels = click(&ctx, &mut view, position(&labels, "1/8"));
        assert_eq!(view.quantize.note, crate::timeline::QuantizeNote::Eighth);
        let labels = click(&ctx, &mut view, position(&labels, "Beat"));
        let labels = click(&ctx, &mut view, position(&labels, "Use Quantize"));
        assert_eq!(view.snap_grid_beats(4), Some(0.5));
        let labels = click(&ctx, &mut view, position(&labels, "1/8"));
        let labels = click(&ctx, &mut view, position(&labels, "Triplet"));
        click(&ctx, &mut view, position(&labels, "1/8 T"));
        assert!((view.snap_grid_beats(4).unwrap() - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn range_edit_is_undoable_and_escape_or_stationary_grab_does_not_edit() {
        let mut f = Fixture::new();
        f.frame(vec![]);
        let mut history = ProjectHistory::default();
        history.begin(&f.project);
        f.button(320.0, 3.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(500.0, 3.0))]);
        f.button(500.0, 3.0, false);
        assert!(history.finish_frame(&f.project, true));
        f.button(320.0, 3.0, true);
        assert!(!f.button(320.0, 3.0, false));
        let drawn = serde_json::to_value(&f.project).unwrap();
        f.button(400.0, 3.0, true);
        f.frame(vec![
            egui::Event::PointerMoved(Pos2::new(600.0, 3.0)),
            egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        assert!(!f.button(600.0, 3.0, false));
        assert_eq!(serde_json::to_value(&f.project).unwrap(), drawn);
        let restored = history.undo(&f.project).unwrap();
        assert_eq!(
            (restored.loop_start_beats, restored.loop_end_beats),
            (0.0, 16.0)
        );
        assert_eq!(
            serde_json::to_value(history.redo(&restored).unwrap()).unwrap(),
            drawn
        );
    }

    #[test]
    fn hand_drag_moves_loop_and_skip_as_a_rigid_range() {
        for skip in [false, true] {
            let mut f = Fixture::new();
            f.project.set_cycle_range(
                if skip { 37.25 } else { 20.5 },
                if skip { 20.5 } else { 37.25 },
            );
            f.frame(vec![]);
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(320.0, 3.0))]);
            assert_eq!(f.cursor, crate::timeline::move_cursor_icon(false));
            let before = serde_json::to_value(&f.project).unwrap();
            let mut history = ProjectHistory::default();
            history.begin(&f.project);
            f.button(320.0, 3.0, true);
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(400.0, 80.0))]);
            assert_eq!(f.cursor, crate::timeline::move_cursor_icon(true));
            assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
            assert!(f.button(400.0, 80.0, false));
            assert_eq!(
                (f.project.loop_start_beats, f.project.loop_end_beats),
                (36.5, 53.25)
            );
            assert_eq!(
                f.project.cycle_mode,
                if skip {
                    CycleMode::Skip
                } else {
                    CycleMode::Loop
                }
            );
            assert_eq!(f.position, 0.0);
            assert!(history.finish_frame(&f.project, true));
            assert_eq!(
                serde_json::to_value(history.undo(&f.project).unwrap()).unwrap(),
                before
            );
        }
    }

    #[test]
    fn ruler_seek_and_zoom_remain_available_in_either_initial_direction() {
        for first in [Pos2::new(390.0, 16.0), Pos2::new(320.0, 46.0)] {
            let mut f = Fixture::new();
            f.view.snap_enabled = false;
            f.frame(vec![]);
            f.button(320.0, 16.0, true);
            f.frame(vec![egui::Event::PointerMoved(first)]);
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(420.0, 46.0))]);
            let zoomed_in = f.view.timeline_zoom;
            let sought = f.position;
            assert!(zoomed_in > 1.0);
            assert!(sought > 0.0);
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(480.0, 46.0))]);
            assert_eq!(f.view.timeline_zoom, zoomed_in);
            assert!(
                f.position > sought,
                "horizontal seek must work after vertical zoom"
            );
            let next_position = f.position;
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(480.0, 26.0))]);
            assert!(
                f.view.timeline_zoom < zoomed_in,
                "zoom must work after horizontal seeking"
            );
            assert_ne!(
                f.position, next_position,
                "seek must use the updated zoom transform"
            );
            let anchor = crate::timeline::ruler_position_beats(
                320.0,
                198.0 - f.view.timeline_scroll_x,
                (1200.0 - 198.0 - 34.0) / 49.0 * f.view.timeline_zoom,
                4,
            );
            assert!(
                (anchor - (320.0 - 198.0) / ((1200.0 - 198.0 - 34.0) / 49.0) * 4.0).abs() < 0.001
            );
            f.view.snap_enabled = true;
            f.button(480.0, 26.0, false);
            assert_eq!(f.position.rem_euclid(4.0), 0.0);
        }
    }

    #[test]
    fn zooming_past_project_start_and_back_restores_the_original_anchor() {
        let mut f = Fixture::new();
        f.view.snap_enabled = false;
        f.frame(vec![]);
        f.button(320.0, 16.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(320.0, -40.0))]);
        assert_eq!(f.view.timeline_scroll_x, 0.0);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(320.0, 46.0))]);
        let expected = (320.0 - 198.0) / ((1200.0 - 198.0 - 34.0) / 49.0) * 4.0;
        assert!((f.position - expected).abs() < 0.001);
    }

    #[test]
    fn corner_handles_resize_only_their_boundary_for_loop_and_skip() {
        let bar_width = (1200.0 - 198.0 - 34.0) / 49.0;
        for skip in [false, true] {
            for left in [false, true] {
                let mut f = Fixture::new();
                f.project.set_cycle_range(
                    if skip { 40.0 } else { 20.0 },
                    if skip { 20.0 } else { 40.0 },
                );
                f.frame(vec![]);
                let bound = if left { 20.0 } else { 40.0 };
                let target = if left { 12.0 } else { 52.0 };
                // Grab within the triangle, not exactly on its boundary, to
                // verify that taking hold does not jump the locator.
                let start_x = 198.0 + bound / 4.0 * bar_width + if left { 2.0 } else { -2.0 };
                let end_x = start_x + (target - bound) / 4.0 * bar_width;
                f.frame(vec![egui::Event::PointerMoved(Pos2::new(start_x, 3.0))]);
                assert_eq!(f.cursor, egui::CursorIcon::ResizeHorizontal);
                let before = serde_json::to_value(&f.project).unwrap();
                let mut history = ProjectHistory::default();
                history.begin(&f.project);
                f.button(start_x, 3.0, true);
                f.frame(vec![egui::Event::PointerMoved(Pos2::new(end_x, 80.0))]);
                assert_eq!(f.cursor, egui::CursorIcon::ResizeHorizontal);
                assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
                assert!(f.button(end_x, 80.0, false));
                assert_eq!(
                    (f.project.loop_start_beats, f.project.loop_end_beats),
                    if left { (12.0, 40.0) } else { (20.0, 52.0) }
                );
                assert_eq!(
                    f.project.cycle_mode,
                    if skip {
                        CycleMode::Skip
                    } else {
                        CycleMode::Loop
                    }
                );
                assert_eq!(f.position, 0.0);
                assert!(history.finish_frame(&f.project, true));
                assert_eq!(
                    serde_json::to_value(history.undo(&f.project).unwrap()).unwrap(),
                    before
                );
            }
        }
    }

    #[test]
    fn middle_drag_in_ruler_or_edit_canvas_uses_hand_and_pans_without_editing() {
        for y in [3.0, 100.0] {
            let mut f = Fixture::new();
            for _ in 0..20 {
                f.project.add_track(TrackKind::Audio);
            }
            f.view.timeline_scroll_x = 200.0;
            f.view.track_scroll_y = 100.0;
            f.frame(vec![]);
            let before = serde_json::to_value(&f.project).unwrap();
            let button = |x, y, pressed| egui::Event::PointerButton {
                pos: Pos2::new(x, y),
                button: egui::PointerButton::Middle,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            f.frame(vec![
                egui::Event::PointerMoved(Pos2::new(600.0, y)),
                button(600.0, y, true),
            ]);
            assert_eq!(f.cursor, crate::timeline::move_cursor_icon(true));
            assert_eq!(
                (f.view.timeline_scroll_x, f.view.track_scroll_y),
                (200.0, 100.0),
                "taking hold must not jump"
            );
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(560.0, y - 30.0))]);
            assert_eq!(f.cursor, crate::timeline::move_cursor_icon(true));
            assert_eq!(
                (f.view.timeline_scroll_x, f.view.track_scroll_y),
                (240.0, 130.0)
            );
            f.frame(vec![button(560.0, y - 30.0, false)]);
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(500.0, y - 50.0))]);
            assert_eq!(
                (f.view.timeline_scroll_x, f.view.track_scroll_y),
                (240.0, 130.0)
            );
            assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
            assert_eq!(f.position, 0.0);
        }
    }
}

#[cfg(test)]
mod project_history_tests {
    use super::*;

    #[test]
    fn project_history_undoes_and_redoes_a_track_edit() {
        let mut history = ProjectHistory::default();
        let mut project = Project::default();
        assert!(history.begin(&project));
        project.add_track(TrackKind::Audio);
        assert!(history.finish_frame(&project, true));

        let undone = history.undo(&project).expect("undo snapshot should exist");
        assert!(undone.tracks.is_empty());
        let redone = history.redo(&undone).expect("redo snapshot should exist");
        assert_eq!(redone, project);
    }

    #[test]
    fn a_new_project_edit_clears_the_redo_stack() {
        let mut history = ProjectHistory::default();
        let mut project = Project::default();
        history.begin(&project);
        project.add_track(TrackKind::Audio);
        history.finish_frame(&project, true);
        project = history.undo(&project).expect("undo snapshot should exist");
        assert!(history.can_redo());

        history.begin(&project);
        project.add_track(TrackKind::Midi);
        history.finish_frame(&project, true);
        assert!(!history.can_redo());
        assert_eq!(project.tracks[0].kind, TrackKind::Midi);
    }

    #[test]
    fn unavailable_audio_paths_are_unique_and_relink_all_matching_clips() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_audio_clip(0, PathBuf::from("missing/take.wav"), 0.0, 1.0);
        project.add_audio_clip(0, PathBuf::from("missing/take.wav"), 4.0, 1.0);
        project.add_audio_clip(0, PathBuf::from("other.wav"), 8.0, 1.0);

        let unavailable = unresolved_audio_source_paths(&project, &HashMap::new());
        assert_eq!(
            unavailable,
            [
                PathBuf::from("missing/take.wav"),
                PathBuf::from("other.wav")
            ]
        );

        let replaced = relink_audio_source_path(
            &mut project,
            Path::new("missing/take.wav"),
            Path::new("restored/take.wav"),
        );
        assert_eq!(replaced, 2);
        assert_eq!(
            project.tracks[0].clips[0].source_path,
            PathBuf::from("restored/take.wav")
        );
        assert_eq!(
            project.tracks[0].clips[1].source_path,
            PathBuf::from("restored/take.wav")
        );
        assert_eq!(
            project.tracks[0].clips[2].source_path,
            PathBuf::from("other.wav")
        );
    }

    #[test]
    fn relinking_an_audio_source_can_be_undone_and_redone() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_audio_clip(0, PathBuf::from("missing/take.wav"), 0.0, 1.0);
        let mut history = ProjectHistory::default();

        assert!(history.begin(&project));
        assert_eq!(
            relink_audio_source_path(
                &mut project,
                Path::new("missing/take.wav"),
                Path::new("restored/take.wav"),
            ),
            1
        );
        history.commit_after_frame = true;
        assert!(history.finish_frame(&project, false));

        let undone = history.undo(&project).expect("relink should be undoable");
        assert_eq!(
            undone.tracks[0].clips[0].source_path,
            PathBuf::from("missing/take.wav")
        );
        let redone = history.redo(&undone).expect("relink should be redoable");
        assert_eq!(
            redone.tracks[0].clips[0].source_path,
            PathBuf::from("restored/take.wav")
        );
    }
}

#[cfg(test)]
mod bus_persistence_tests {
    use super::*;
    #[test]
    fn mixer_presets_distinguish_buses_from_same_named_tracks_and_bus_edits_undo() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0].name = "Stereo In".into();
        let snapshot = ChannelSettingsSnapshot::from(&project.input_channel.strip);
        assert!(channel_snapshot_matches_track(
            &snapshot,
            &project.input_channel.strip
        ));
        assert!(!channel_snapshot_matches_track(
            &snapshot,
            &project.tracks[0]
        ));
        let mut history = ProjectHistory::default();
        history.begin(&project);
        project.input_channel.strip.gain = 0.25;
        assert!(history.finish_frame(&project, true));
        project = history.undo(&project).expect("bus edit undo");
        assert_eq!(project.input_channel.strip.gain, 1.0);
        project = history.redo(&project).expect("bus edit redo");
        assert_eq!(project.input_channel.strip.gain, 0.25);
    }
}

#[cfg(test)]
mod plugin_catalog_tests {
    use super::*;
    use opentrack_plugins::{PluginArchitecture, PluginClassId, PluginFormat, PluginScanStatus};

    fn vst3_descriptor(
        seed: u8,
        name: &str,
        category: PluginCategory,
        subcategories: &[&str],
    ) -> PluginDescriptor {
        PluginDescriptor {
            format: PluginFormat::Vst3,
            class_id: PluginClassId::from_bytes([seed; 16]),
            name: name.to_owned(),
            vendor: "Example Audio".to_owned(),
            version: "1.0".to_owned(),
            category,
            subcategories: subcategories
                .iter()
                .map(|subcategory| (*subcategory).to_owned())
                .collect(),
            module_path: PathBuf::from(format!(r"C:\VST3\{name}.vst3")),
            architecture: PluginArchitecture::X64,
            buses: Vec::new(),
            parameter_count: 0,
            parameters: Vec::new(),
            has_editor: false,
            latency_samples: 0,
            tail_samples: 0,
            scan_status: PluginScanStatus::Enumerated,
            scanned_at_unix_ms: 1_700_000_000_000,
            diagnostics: None,
        }
    }

    #[test]
    fn vst3_subcategories_map_to_normal_insert_categories() {
        assert_eq!(
            insert_category_from_subcategories(["Fx", "Dynamics", "Compressor"]),
            InsertMenuCategory::Dynamics
        );
        assert_eq!(
            insert_category_from_subcategories(["Fx", "Delay", "Modulation"]),
            InsertMenuCategory::Delay
        );
        assert_eq!(
            insert_category_from_subcategories(["Fx", "Pitch Shift"]),
            InsertMenuCategory::PitchShift
        );
        assert_eq!(
            insert_category_from_subcategories(["Fx", "Analyzer"]),
            InsertMenuCategory::Analyzer
        );
        assert_eq!(
            insert_category_from_subcategories(["Fx", "NoSuchCategory"]),
            InsertMenuCategory::Other
        );

        let analyzer = vst3_descriptor(1, "Scope", PluginCategory::Analyzer, &["Fx"]);
        assert_eq!(
            vst3_insert_category(&analyzer),
            InsertMenuCategory::Analyzer
        );
        let effect = vst3_descriptor(2, "Mystery", PluginCategory::Effect, &["Fx"]);
        assert_eq!(vst3_insert_category(&effect), InsertMenuCategory::Other);
    }

    #[test]
    fn insert_picker_uses_normal_categories_without_a_vst3_root() {
        let ctx = egui::Context::default();
        let catalog = vec![vst3_descriptor(
            1,
            "Studio Compressor",
            PluginCategory::Effect,
            &["Fx", "Dynamics"],
        )];
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let mut selected = None;
            show_insert_effect_picker(ui, &mut selected, &catalog);
            assert!(selected.is_none());
        });
        let labels = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(labels.iter().any(|label| label == "Dynamics"));
        assert!(labels.iter().any(|label| label == "Analyzer"));
        assert!(!labels.iter().any(|label| label.contains("VST3 Plug-in")));
        output.textures_delta.clear();
    }

    #[test]
    fn catalog_groups_plugins_and_creates_a_vst3_insert() {
        let catalog = vec![
            vst3_descriptor(
                1,
                "Studio Compressor",
                PluginCategory::Effect,
                &["Fx", "Dynamics", "Compressor"],
            ),
            vst3_descriptor(2, "Graphic EQ", PluginCategory::Effect, &["Fx", "EQ"]),
            vst3_descriptor(3, "Mystery", PluginCategory::Effect, &["Fx", "Weird"]),
            vst3_descriptor(4, "Synth", PluginCategory::Instrument, &["Instrument"]),
        ];

        let dynamics = catalog_plugins_for_category(&catalog, InsertMenuCategory::Dynamics);
        assert_eq!(
            dynamics
                .iter()
                .map(|plugin| plugin.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Studio Compressor"]
        );
        let eq = catalog_plugins_for_category(&catalog, InsertMenuCategory::Eq);
        assert_eq!(eq.len(), 1);
        assert_eq!(eq[0].name, "Graphic EQ");
        let other = catalog_plugins_for_category(&catalog, InsertMenuCategory::Other);
        assert_eq!(
            other
                .iter()
                .map(|plugin| plugin.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Mystery"]
        );
        assert!(
            catalog_plugins_for_category(&catalog, InsertMenuCategory::Tools).is_empty(),
            "instruments and unassigned categories must not appear under Tools"
        );

        let kind = vst3_insert_effect(dynamics[0]);
        match kind {
            InsertEffectKind::Vst3 {
                class_id,
                module_path,
                name,
                vendor,
                component_state,
                controller_state,
            } => {
                assert_eq!(class_id, "01010101010101010101010101010101");
                assert_eq!(
                    module_path,
                    PathBuf::from(r"C:\VST3\Studio Compressor.vst3")
                );
                assert_eq!(name, "Studio Compressor");
                assert_eq!(vendor, "Example Audio");
                assert!(component_state.is_empty());
                assert!(controller_state.is_empty());
            }
            other => panic!("expected a VST3 insert, got {other:?}"),
        }
    }

    #[derive(Default)]
    struct TestStorage {
        values: HashMap<String, String>,
    }

    impl eframe::Storage for TestStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.values.get(key).cloned()
        }

        fn set_string(&mut self, key: &str, value: String) {
            self.values.insert(key.to_owned(), value);
        }

        fn remove_string(&mut self, key: &str) {
            self.values.remove(key);
        }

        fn flush(&mut self) {}
    }

    #[test]
    fn plugin_folders_and_catalog_round_trip_through_eframe_storage() {
        let folders = vec![
            PathBuf::from(r"C:\Program Files\Common Files\VST3"),
            PathBuf::from(r"C:\Users\Example\AppData\Local\Programs\Common\VST3"),
        ];
        let catalog = vec![vst3_descriptor(
            7,
            "Cached Compressor",
            PluginCategory::Effect,
            &["Fx", "Dynamics"],
        )];
        let mut storage = TestStorage::default();
        eframe::set_value(&mut storage, "vst3-plugin-folders-v1", &folders);
        eframe::set_value(&mut storage, "vst3-plugin-catalog-v1", &catalog);

        let restored_folders =
            eframe::get_value::<Vec<PathBuf>>(&storage, "vst3-plugin-folders-v1")
                .expect("folders round-trip");
        let restored_catalog =
            eframe::get_value::<Vec<PluginDescriptor>>(&storage, "vst3-plugin-catalog-v1")
                .expect("catalog round-trip");
        assert_eq!(restored_folders, folders);
        assert_eq!(restored_catalog, catalog);
    }
}

#[cfg(test)]
mod recording_history_tests {
    use super::*;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct Fixture {
        root: PathBuf,
        paths: Vec<PathBuf>,
        imported: PathBuf,
        project: Project,
        history: ProjectHistory,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "opentrack-record-undo-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            let imported = root.join("imported.wav");
            std::fs::write(&imported, b"existing audio").unwrap();
            let mut project = Project::default();
            project.add_track(TrackKind::Audio);
            project
                .add_audio_clip(0, imported.clone(), 0.0, 1.0)
                .unwrap();
            let mut history = ProjectHistory::default();
            history.saved(&project);
            history.begin(&project);
            let paths = vec![root.join("recorded1.wav"), root.join("recorded2.wav")];
            for path in &paths {
                std::fs::write(path, b"recorded audio").unwrap();
                project.add_audio_clip(0, path.clone(), 4.0, 1.0).unwrap();
            }
            history.unsaved_recordings.extend(paths.iter().cloned());
            history.pending_recording = Some(recording_undo::RecordingUndo::new(
                paths.clone(),
                root.join("Undo"),
            ));
            assert!(history.finish_frame(&project, true));
            Self {
                root,
                paths,
                imported,
                project,
                history,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.history.clear();
            for p in &self.paths {
                let _ = std::fs::remove_file(p);
            }
            let _ = std::fs::remove_file(&self.imported);
            let _ = std::fs::remove_dir(self.root.join("Undo"));
            let _ = std::fs::remove_dir(&self.root);
        }
    }
    #[test]
    fn recording_undo_removes_files_redo_restores_exact_bytes_and_branch_discards_recovery() {
        let mut f = Fixture::new();
        f.project = f.history.undo(&f.project).unwrap();
        assert!(f.paths.iter().all(|p| !p.exists()));
        assert!(f.imported.exists());
        assert_eq!(f.project.tracks[0].clips.len(), 1);
        f.project = f.history.redo(&f.project).unwrap();
        for p in &f.paths {
            assert_eq!(std::fs::read(p).unwrap(), b"recorded audio");
        }
        f.project = f.history.undo(&f.project).unwrap();
        f.history.begin(&f.project);
        f.project.tracks[0].name = "new edit".into();
        assert!(f.history.finish_frame(&f.project, true));
        assert!(!f.history.can_redo());
        assert!(!f.root.join("Undo").exists());
        assert!(f.imported.exists());
    }
    #[test]
    fn failed_undo_or_redo_keeps_history_and_never_overwrites_another_file() {
        let mut f = Fixture::new();
        std::fs::remove_file(&f.paths[1]).unwrap();
        assert!(f.history.undo(&f.project).is_none());
        assert!(f.paths[0].exists());
        assert!(f.history.can_undo());
        std::fs::write(&f.paths[1], b"recorded audio").unwrap();
        f.project = f.history.undo(&f.project).unwrap();
        std::fs::write(&f.paths[0], b"new unrelated file").unwrap();
        assert!(f.history.redo(&f.project).is_none());
        assert_eq!(std::fs::read(&f.paths[0]).unwrap(), b"new unrelated file");
        assert!(f.history.can_redo());
        assert!(!f.paths[1].exists());
        std::fs::remove_file(&f.paths[0]).unwrap();
        f.project = f.history.redo(&f.project).unwrap();
        assert!(f.paths.iter().all(|p| p.exists()));
    }
    #[test]
    fn discarding_changes_restores_recordings_referenced_by_saved_project_even_after_redo_is_cleared()
     {
        let mut f = Fixture::new();
        f.history.saved(&f.project);
        f.project = f.history.undo(&f.project).unwrap();
        f.history.begin(&f.project);
        f.project.tracks[0].name = "branch".into();
        assert!(f.history.finish_frame(&f.project, true));
        assert!(!f.paths[0].exists());
        f.history.restore_saved_recordings().unwrap();
        assert!(f.paths.iter().all(|p| p.exists()));
    }
    #[test]
    fn discard_deletes_unsaved_recordings_but_preserves_imported_and_saved_recordings() {
        let mut f = Fixture::new();
        f.history.discard_recordings().unwrap();
        assert!(f.paths.iter().all(|p| !p.exists()));
        assert!(f.imported.exists());
        let mut saved = Fixture::new();
        saved.history.saved(&saved.project);
        saved.history.discard_recordings().unwrap();
        assert!(saved.paths.iter().all(|p| p.exists()));
        assert!(saved.imported.exists());
    }
    #[test]
    fn discard_deletes_takes_even_after_their_recording_undo_step_is_trimmed() {
        let mut f = Fixture::new();
        for i in 0..40 {
            f.history.begin(&f.project);
            f.project.tracks[0].name = format!("edit {i}");
            assert!(f.history.finish_frame(&f.project, true));
        }
        f.history.discard_recordings().unwrap();
        assert!(f.paths.iter().all(|p| !p.exists()));
        assert!(f.imported.exists());
    }
    #[test]
    fn next_take_name_does_not_reuse_an_undone_or_saved_recording_path() {
        let f = Fixture::new();
        let tracks = vec![(1, "Audio".into(), Some(AudioRecordRoute::Stereo))];
        let reserved = HashSet::from([f.root.join("Take 001.wav")]);
        assert_eq!(
            next_recording_paths(&f.root, &tracks, &reserved).unwrap(),
            vec![f.root.join("Take 002.wav")]
        );
    }
    #[test]
    fn ordinary_event_removal_undo_does_not_delete_imported_audio() {
        let mut f = Fixture::new();
        f.history.clear();
        f.history.begin(&f.project);
        f.project.tracks[0].clips.clear();
        assert!(f.history.finish_frame(&f.project, true));
        f.project = f.history.undo(&f.project).unwrap();
        assert!(f.imported.exists());
        assert!(f.paths.iter().all(|p| p.exists()));
    }
}

#[cfg(test)]
mod project_open_error_tests {
    use super::*;
    #[test]
    fn rejected_project_open_is_visible_from_empty_shell_and_preserves_current_state() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        let before = app.data.project.clone();
        let path =
            std::env::temp_dir().join(format!("opentrack-invalid-open-{}.opr", std::process::id()));
        std::fs::write(&path, br#"{"format":"opentrack-project","version":999}"#).unwrap();
        app.load_project(path.clone());
        std::fs::remove_file(path).unwrap();
        assert_eq!(app.data.project, before);
        assert!(!app.project_open);
        assert!(app.project_open_error.as_ref().unwrap().contains("999"));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 600.0))),
                ..Default::default()
            },
            |_| app.show_project_open_error(&ctx),
        );
        assert!(!output.shapes.is_empty());
        let _ = ctx.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
        output.textures_delta.clear();
    }
}

#[cfg(test)]
mod channel_window_lifecycle_tests {
    use super::*;

    #[test]
    fn mixer_track_selection_updates_the_shared_project_selection() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.selected_track = Some(1);
        app.arrange_view_state.selected_track_ids.clear();
        app.sync_arrange_track_selection();
        let expected = app.data.project.tracks[1].id;
        assert_eq!(app.data.project.selected_track, Some(1));
        assert_eq!(
            app.arrange_view_state.selected_track_ids,
            std::collections::BTreeSet::from([expected])
        );
        assert_eq!(
            app.arrange_view_state.selection_anchor_track_id,
            Some(expected)
        );
    }

    #[test]
    fn closing_bus_editor_then_reopening_other_channels_recreates_and_renders_editor() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.selected_track = Some(0);
        let render = |app: &mut OpenTrackApp, close: bool| {
            let mut input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 600.0))),
                ..Default::default()
            };
            if close {
                input
                    .viewports
                    .get_mut(&egui::ViewportId::ROOT)
                    .unwrap()
                    .events
                    .push(egui::ViewportEvent::Close);
            }
            let mut out = ctx.run_ui(input, |ui| app.show_channel_settings_window(ui.ctx()));
            out.textures_delta.clear();
            out
        };
        for bus in [Some(false), Some(true), None, Some(false), None] {
            let epoch = app.channel_settings_window.epoch;
            app.channel_settings_window.request_channel_settings(bus);
            assert_ne!(app.channel_settings_window.epoch, epoch);
            // An old close request must not cancel an explicit reopen.
            render(&mut app, true);
            assert!(app.channel_settings_window.open);
            assert!(app.channel_settings_window.viewport_live);
            render(&mut app, false);
            let output = render(&mut app, false);
            fn has_controls(shape: &egui::Shape) -> bool {
                match shape {
                    egui::Shape::Text(t) => t.galley.text() == "Post Inserts/EQ",
                    egui::Shape::Vec(shapes) => shapes.iter().any(has_controls),
                    _ => false,
                }
            }
            assert!(
                output.shapes.iter().any(|s| has_controls(&s.shape)),
                "editor controls must render after window layout settles"
            );
            render(&mut app, true);
            assert!(!app.channel_settings_window.open);
            assert!(!app.channel_settings_window.viewport_live);
        }
        assert_eq!(app.data.project.selected_track, Some(0));
    }
}

#[cfg(test)]
mod outside_click_window_tests {
    use super::*;

    #[test]
    fn transient_window_closes_on_outside_click_but_stays_open_on_inside_click() {
        let ctx = egui::Context::default();
        let mut open = true;
        let window_id = Id::new("outside-dismiss-test-window");
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        for pointer_event in [
            None,
            Some(Pos2::new(350.0, 250.0)),
            Some(Pos2::new(40.0, 40.0)),
        ] {
            let events = pointer_event.map_or_else(Vec::new, |position| {
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]
            });
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let response = egui::Window::new("Transient test")
                        .id(window_id)
                        .default_pos(Pos2::new(300.0, 200.0))
                        .fixed_size(Vec2::new(200.0, 100.0))
                        .open(&mut open)
                        .show(ui.ctx(), |ui| ui.label("contents"));
                    dismiss_window_on_outside_click(
                        ui.ctx(),
                        window_id,
                        &mut open,
                        response.as_ref(),
                    );
                },
            );
            output.textures_delta.clear();
            if let Some(position) = pointer_event {
                assert_eq!(
                    open,
                    screen.contains(position)
                        && Rect::from_min_size(Pos2::new(300.0, 200.0), Vec2::new(200.0, 100.0))
                            .contains(position),
                    "outside clicks close while clicks inside the window leave it open"
                );
            }
        }
        assert!(!open, "outside click should dismiss the transient window");
    }
}
