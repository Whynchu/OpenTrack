#[path = "audio_configuration.rs"]
mod audio_configuration;
#[path = "audio_events.rs"]
pub(crate) mod audio_events;
#[path = "channel_settings.rs"]
mod channel_settings;
#[path = "mixer.rs"]
mod mixer;
#[path = "project_controls.rs"]
mod project_controls;
#[path = "project_inspector.rs"]
mod project_inspector;
#[path = "recording_undo.rs"]
mod recording_undo;
#[path = "waveform_view.rs"]
mod waveform_view;

use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    time::Instant,
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

#[derive(Clone, Copy, Default, Eq, PartialEq)]
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
}

#[derive(Default)]
struct CompressorEditorState {
    window: NativeWindowState,
    track_id: Option<u64>,
    insert_index: usize,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChannelSettingsPreferences {
    #[serde(default = "default_channel_settings_items")]
    visible_items: Vec<ChannelSettingsViewItem>,
    #[serde(default)]
    presets: Vec<ChannelSettingsLayoutPreset>,
    #[serde(default)]
    always_on_top: bool,
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
            always_on_top: false,
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
    preferences: ChannelSettingsPreferences,
    setup_open: bool,
    selected_preset: Option<usize>,
    preset_name_draft: String,
    pending_action: Option<ChannelSettingsAction>,
    applied_window_level: Option<bool>,
    editing_track_id: Option<u64>,
    rename_draft: String,
    rename_focus_requested: bool,
    setup_selected_visible: Option<usize>,
    setup_selected_hidden: Option<ChannelSettingsViewItem>,
    // Per-slot Send activation is currently UI state only; the track/audio model has no Send DSP.
    enabled_sends: HashSet<(u64, usize)>,
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
        .with_inner_size(default_size)
        .with_min_inner_size(minimum_size)
        .with_resizable(true);
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
    project_window: ProjectWindowState,
    workspace_preferences: WorkspacePreferences,
    data: AppData,
    last_meter_update: Option<Instant>,
    main_window_bounds: Option<WindowBounds>,
    main_window_restore_bounds: Option<WindowBounds>,
    show_transport: bool,
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
    device_setup_page: u8,
    vst_connections_open: bool,
    vst_connections_tab: usize,
    project_setup_open: bool,
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
    midi_editor_bounds: Option<WindowBounds>,
    midi_editor_restore_bounds: Option<WindowBounds>,
    midi_editor_window_epoch: u64,
    midi_editor_viewport_live: bool,
    midi_editor_focus_requested: bool,
    channel_settings_window: NativeWindowState,
    channel_settings_ui: ChannelSettingsUiState,
    mixer_window: NativeWindowState,
    compressor_editor: CompressorEditorState,
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
    midi_note_drag: Option<MidiNoteDrag>,
    midi_controller_drag: Option<MidiControllerDrag>,
    project_history: ProjectHistory,
    recording_session: Option<RecordingSession>,
    audio_import_task: Option<AudioImportTask>,
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

#[derive(Clone, Copy)]
struct MidiNoteDrag {
    track_id: u64,
    part_id: u64,
    note_id: u64,
    mode: MidiNoteDragMode,
    start_beats: f64,
    duration_beats: f64,
    pitch: u8,
    delta_beats: f64,
    delta_rows: f64,
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
    Resize,
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
    let c = rect.right_center() - Vec2::new(7.0, 0.0);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            c + Vec2::new(-3.0, -2.0),
            c + Vec2::new(3.0, -2.0),
            c + Vec2::new(0.0, 2.0),
        ],
        ink,
        Stroke::NONE,
    ));
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
    let painter = ui.painter();
    let normal_fill = if selected {
        selected_color.unwrap_or(p.accent)
    } else {
        theme::toolbar_button_fill(ui.visuals())
    };
    let fill = if enabled && response.hovered() {
        theme::hover_button_fill(normal_fill, ui.visuals())
    } else {
        normal_fill
    };
    painter.rect_filled(rect, 1.0, fill);
    let outline = if enabled && response.hovered() {
        ui.visuals().widgets.hovered.bg_stroke
    } else {
        Stroke::new(1.0, p.grid)
    };
    painter.rect_stroke(rect, 1.0, outline, egui::StrokeKind::Inside);
    painter.line_segment(
        [
            rect.left_top() + Vec2::splat(1.0),
            rect.right_top() + Vec2::new(-1.0, 1.0),
        ],
        Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.25)),
    );
    let color = if enabled {
        project_controls::label_ink(fill)
    } else if project_controls::label_ink(fill) == Color32::WHITE {
        Color32::from_gray(180)
    } else {
        Color32::from_gray(88)
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

fn transport_panel_button(
    ui: &mut egui::Ui,
    origin: Pos2,
    id: impl std::hash::Hash + std::fmt::Debug,
    bounds: [f32; 4],
    label: &str,
    tooltip: &str,
    enabled: bool,
    selected: bool,
) -> egui::Response {
    transport_panel_button_tinted(
        ui, origin, id, bounds, label, tooltip, enabled, selected, None,
    )
}

fn transport_panel_button_tinted(
    ui: &mut egui::Ui,
    origin: Pos2,
    id: impl std::hash::Hash + std::fmt::Debug,
    bounds: [f32; 4],
    label: &str,
    tooltip: &str,
    enabled: bool,
    selected: bool,
    selected_color: Option<Color32>,
) -> egui::Response {
    let rect = Rect::from_min_size(
        origin + Vec2::new(bounds[0], bounds[1]),
        Vec2::new(bounds[2], bounds[3]),
    );
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let response = ui.interact(rect, ui.make_persistent_id(id), sense);
    let visuals = ui.style().interact(&response);
    if selected || response.hovered() {
        let fill = if selected && label == "●" {
            theme::RECORD
        } else if selected {
            selected_color.unwrap_or(theme::palette(ui).accent)
        } else {
            visuals.bg_fill
        };
        let stroke = if selected {
            selected_color
                .map(|color| Stroke::new(1.0, color))
                .unwrap_or(ui.visuals().selection.stroke)
        } else {
            visuals.bg_stroke
        };
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(1), fill);
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(1),
            stroke,
            egui::StrokeKind::Inside,
        );
    }
    let text_color = if selected {
        Color32::WHITE
    } else if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().text_color().gamma_multiply(0.7)
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(9.0),
        text_color,
    );
    if tooltip.is_empty() {
        response
    } else {
        response.on_hover_text(tooltip)
    }
}

fn transport_panel_text(
    ui: &mut egui::Ui,
    origin: Pos2,
    bounds: [f32; 4],
    text: &str,
    color: Color32,
    size: f32,
    monospace: bool,
) {
    let rect = Rect::from_min_size(
        origin + Vec2::new(bounds[0], bounds[1]),
        Vec2::new(bounds[2], bounds[3]),
    );
    let font = if monospace {
        FontId::monospace(size)
    } else {
        FontId::proportional(size)
    };
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, text, font, color);
}

fn disabled_menu_commands(ui: &mut egui::Ui, commands: &[(&str, &str)]) {
    for (label, shortcut) in commands {
        menu_command(ui, label, shortcut, false);
    }
}

#[derive(Default)]
struct ProjectHistory {
    undo: VecDeque<HistoryEntry>,
    redo: VecDeque<HistoryEntry>,
    pending_recording: Option<recording_undo::RecordingUndo>,
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
}
impl HistoryEntry {
    fn len(&self) -> usize {
        self.snapshot.len()
    }
}

impl ProjectHistory {
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
        if let Some(recording) = previous.recording.as_mut() {
            if let Err(error) = recording.remove(&restored) {
                self.last_error = Some(format!("Could not undo recording: {error}"));
                self.undo.push_back(previous);
                return None;
            }
        }
        self.redo.push_back(HistoryEntry {
            snapshot: current,
            recording: previous.recording.take(),
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
        if let Some(recording) = next.recording.as_mut() {
            if let Err(error) = recording.restore() {
                self.last_error = Some(format!("Could not redo recording: {error}"));
                self.redo.push_back(next);
                return None;
            }
        }
        self.undo.push_back(HistoryEntry {
            snapshot: current,
            recording: next.recording.take(),
        });
        self.trim();
        Some(restored)
    }

    fn dispose_entry(&mut self, mut entry: HistoryEntry) {
        if entry
            .recording
            .as_ref()
            .is_some_and(|r| r.protects_saved(&self.saved_paths))
        {
            self.saved_recording_recovery
                .push(entry.recording.take().unwrap());
        }
    }
    fn saved(&mut self, project: &Project) {
        self.unsaved_recordings.clear();
        self.saved_paths = project
            .tracks
            .iter()
            .flat_map(|t| t.clips.iter().map(|c| c.source_path.clone()))
            .collect();
        self.saved_recording_recovery
            .retain(|r| r.protects_saved(&self.saved_paths));
    }
    fn restore_saved_recordings(&mut self) -> Result<(), String> {
        for entry in self.undo.iter_mut().chain(self.redo.iter_mut()) {
            if let Some(r) = entry
                .recording
                .as_mut()
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
            .filter_map(|e| e.recording.as_ref())
            .chain(self.saved_recording_recovery.iter())
        {
            paths.extend(r.paths().iter().cloned());
        }
        paths
    }
    fn discard_recordings(&mut self) -> Result<(), String> {
        self.restore_saved_recordings()?;
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
        self.unsaved_recordings.clear();
        self.saved_recording_recovery.clear();
        self.saved_paths.clear();
        self.undo.clear();
        self.redo.clear();
        self.pending_recording = None;
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
            .unwrap_or(1.0);
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

        let mut app = Self {
            dock_state,
            workspace_state,
            arrange_view_state: ArrangeViewState::default(),
            project_window,
            workspace_preferences,
            data: AppData::default(),
            last_meter_update: None,
            main_window_bounds: None,
            main_window_restore_bounds: None,
            show_transport: true,
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
            device_setup_page: 0,
            vst_connections_open: false,
            vst_connections_tab: 0,
            project_setup_open: false,
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
            midi_editor_bounds: None,
            midi_editor_restore_bounds: None,
            midi_editor_window_epoch: 0,
            midi_editor_viewport_live: false,
            midi_editor_focus_requested: false,
            channel_settings_window: NativeWindowState::default(),
            channel_settings_ui: ChannelSettingsUiState {
                applied_window_level: Some(false),
                preferences: channel_settings_preferences,
                preset_name_draft: "Default".to_owned(),
                ..Default::default()
            },
            mixer_window: NativeWindowState::default(),
            compressor_editor: CompressorEditorState::default(),
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
            midi_note_drag: None,
            midi_controller_drag: None,
            project_history: ProjectHistory::default(),
            recording_session: None,
            audio_import_task: None,
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
                self.midi_editor_bounds.clone(),
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
        midi_editor_bounds: Option<WindowBounds>,
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
        store_active_project_window_state(
            workspace_state,
            "midi-editor",
            midi_editor_open,
            midi_editor_bounds,
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
            self.rebuild_audio_plan();
            self.refresh_modified_from_saved();
            self.status_message = "Undid project edit".to_owned();
        } else if let Some(error) = self.project_history.last_error.take() {
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
            self.rebuild_audio_plan();
            self.refresh_modified_from_saved();
            self.status_message = "Redid project edit".to_owned();
        } else if let Some(error) = self.project_history.last_error.take() {
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
        let tempo_bpm = self.data.project.tempo_bpm;
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
        let track_descriptors: Vec<_> = project_tracks
            .iter()
            .map(|track| {
                let meter_peak = Arc::clone(
                    track_meter_peaks
                        .entry(track.id)
                        .or_insert_with(|| Arc::new(opentrack_audio::StereoMeter::default())),
                );
                (
                    meter_peak,
                    track.gain,
                    track.pan,
                    track.track_delay_ms,
                    track.muted,
                    track.solo,
                    track.solo_defeat,
                    track.equalizer.clone(),
                    track.inserts.clone(),
                    track.inserts_bypassed,
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
                                clip.source_path.clone(),
                                clip.start_beats,
                                clip.gain,
                                clip.source_offset_seconds,
                                clip.duration_seconds,
                                clip.fade_in_seconds,
                                clip.fade_out_seconds,
                            )
                        })
                        .collect::<Vec<_>>(),
                    track
                        .midi_parts
                        .iter()
                        .flat_map(|part| {
                            part.notes.iter().map(|note| {
                                (
                                    part.start_beats + note.start_beats,
                                    note.duration_beats,
                                    440.0_f64 * 2.0_f64.powf((f64::from(note.pitch) - 69.0) / 12.0),
                                    note.channel,
                                    f32::from(note.velocity) / 127.0,
                                )
                            })
                        })
                        .map(
                            |(start_beats, duration_beats, frequency_hz, channel, velocity)| {
                                PlaybackMidiNote {
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
        let mut active_audio_paths = HashSet::new();
        let mut active_stream_clip_ids = HashSet::new();
        for (
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
                if let Some(source) = source {
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
                    let playback_source = match playback_source {
                        Ok(source) => source,
                        Err(error) => {
                            source_errors.push(format!("{}: {error}", source_path.display()));
                            continue;
                        }
                    };
                    playback_clips.push(PlaybackClip {
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
                output_route: match output_routing.as_deref().unwrap_or("Stereo Out") {
                    "Left" => PlaybackOutputRoute::Left,
                    "Right" => PlaybackOutputRoute::Right,
                    "No Bus" => PlaybackOutputRoute::Disconnected,
                    _ => PlaybackOutputRoute::Stereo,
                },
            });
        }
        self.data
            .streaming_readers
            .retain(|clip_id, _| active_stream_clip_ids.contains(clip_id));
        self.data
            .audio_sources
            .retain(|path, _| active_audio_paths.contains(path));
        if let Some(engine) = self.audio_engine.as_mut() {
            engine.set_playback_plan_with_output(
                tempo_bpm,
                playback_tracks,
                loop_range_beats,
                self.data.project.cycle_mode,
                &self.data.project.output_channel.strip,
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
        let decay = 10.0_f32.powf(-2.0 * delta_seconds.clamp(0.0, 5.0));
        let meter_levels = &mut self.data.track_meter_levels;
        for (track_id, meter_peak) in &self.data.track_meter_peaks {
            let peaks = meter_peak.take();
            let levels = meter_levels.entry(*track_id).or_insert([0.0; 2]);
            for (level, peak) in levels.iter_mut().zip(peaks) {
                *level = peak.max(*level * decay);
            }
        }
        let output_peaks = self
            .audio_engine
            .as_ref()
            .map(|engine| engine.take_output_peaks())
            .unwrap_or([0.0; 2]);
        for (level, peak) in self
            .data
            .output_channel_meter_levels
            .iter_mut()
            .zip(output_peaks)
        {
            *level = peak.max(*level * decay);
        }
        let input_peaks = if let Some(session) = self.recording_session.as_ref() {
            session.recorder.monitor_buffer().take_bus_peaks()
        } else if let Some(probe) = self.audio_input_probe.as_ref() {
            probe.monitor_buffer().take_bus_peaks()
        } else {
            [0.0; 2]
        };
        for (level, peak) in self
            .data
            .input_channel_meter_levels
            .iter_mut()
            .zip(input_peaks)
        {
            *level = peak.max(*level * decay);
        }
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
            PendingProjectAction::Open(path) => self.load_project(path),
            PendingProjectAction::Close => {}
        }
    }

    fn reset_project(&mut self, parent: PathBuf) {
        self.set_transport_playing(false);
        self.project_history.clear();
        self.project_history.skip_next_commit = true;
        self.data = AppData::default();
        self.dock_state = default_dock_state();
        self.workspace_state = initial_workspace_state(&self.dock_state)
            .expect("the built-in dock layout must be serializable");
        self.arrange_view_state = ArrangeViewState::default();
        self.project_window = restore_project_window_state(
            self.workspace_state
                .active_index
                .and_then(|index| self.workspace_state.workspaces.get(index)),
            self.project_window.epoch,
        );
        self.track_add_dialog = None;
        self.main_window_restore_bounds = None;
        self.midi_editor_open = false;
        self.midi_editor_bounds = None;
        self.midi_editor_restore_bounds = None;
        self.midi_editor_window_epoch = self.midi_editor_window_epoch.wrapping_add(1);
        self.midi_editor_viewport_live = false;
        self.midi_editor_focus_requested = false;
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
        if root.exists() || path.exists() {
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
            return;
        };
        let mut open = true;
        let mut dismissed = false;
        egui::Window::new("Could not open project")
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
        if !open || dismissed {
            self.project_open_error = None;
        }
    }

    fn load_project(&mut self, path: std::path::PathBuf) {
        self.project_open_error = None;
        match ProjectFile::read(&path) {
            Ok(document) => {
                let project_folder_managed =
                    document.project_folder_layout > 0 && project_directory_layout_exists(&path);
                let mut workspace_state = document.workspace_state;
                let previous_workspace_state = workspace_state.clone();
                let instantiated =
                    workspace_state.instantiate_global_presets(&self.workspace_preferences);
                let workspace_state_changed = workspace_state != previous_workspace_state;
                let midi_editor_snapshot = workspace_state
                    .active_index
                    .and_then(|index| workspace_state.workspaces.get(index))
                    .and_then(midi_editor_window_snapshot);
                let active_workspace = workspace_state
                    .active_index
                    .and_then(|index| workspace_state.workspaces.get(index))
                    .cloned();
                let project_window = restore_project_window_state(
                    active_workspace.as_ref(),
                    self.project_window.epoch,
                );
                let arrange_view_state = active_workspace
                    .as_ref()
                    .map(arrange_view_state_from_workspace)
                    .unwrap_or_default();
                let midi_editor_open = midi_editor_snapshot.is_some_and(|window| window.open);
                let midi_editor_bounds =
                    midi_editor_snapshot.and_then(|window| window.bounds.clone());
                let main_window_bounds = active_workspace
                    .as_ref()
                    .and_then(main_window_window_snapshot)
                    .and_then(|window| window.bounds.clone());
                self.set_transport_playing(false);
                self.project_history.clear();
                self.project_history.skip_next_commit = true;
                self.data = AppData {
                    project: document.project,
                    transport: TransportState {
                        cycle: document.cycle,
                        ..Default::default()
                    },
                    selected_media: document.selected_media,
                    audio_sources: HashMap::new(),
                    streaming_readers: HashMap::new(),
                    track_meter_peaks: HashMap::new(),
                    track_meter_levels: HashMap::new(),
                    input_channel_meter_levels: [0.0; 2],
                    output_channel_meter_levels: [0.0; 2],
                    reported_stream_errors: HashSet::new(),
                    reported_stream_underruns: HashSet::new(),
                    reported_waveform_errors: HashSet::new(),
                };
                self.workspace_state = workspace_state;
                self.dock_state = document.live_dock_state;
                self.arrange_view_state = arrange_view_state;
                self.project_window = project_window;
                self.track_add_dialog = None;
                if let Some(bounds) = main_window_bounds {
                    self.main_window_bounds = Some(bounds.clone());
                    self.main_window_restore_bounds = Some(bounds);
                } else {
                    self.main_window_restore_bounds = None;
                }
                self.midi_editor_open = midi_editor_open;
                self.midi_editor_bounds = midi_editor_bounds.clone();
                self.midi_editor_restore_bounds = midi_editor_bounds;
                self.midi_editor_window_epoch = self.midi_editor_window_epoch.wrapping_add(1);
                self.midi_editor_viewport_live = false;
                self.midi_editor_focus_requested = false;
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
                self.project_folder_setup =
                    (!project_folder_managed).then(|| ProjectFolderSetup::Legacy {
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
            Err(error) => {
                self.status_message = format!("Could not open project: {error}");
                self.project_open_error = Some(format!("{}\n\n{error}", path.display()));
            }
        }
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

    fn show_event_colors_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.event_colors_dialog.take() else {
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
        egui::Window::new("Event Colors")
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
        egui::Window::new("Rename Track")
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
            return;
        };
        let mut open = true;
        let mut remove = false;
        let mut cancel = false;
        egui::Window::new("Remove Track")
            .id(Id::new("track-delete-prompt"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Remove {track_name} and all of its clips and MIDI parts?"
                ));
                ui.label("The track can be restored with Undo.");
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
            self.midi_editor_bounds.clone(),
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
        self.midi_editor_bounds = midi_editor_snapshot.and_then(|window| window.bounds.clone());
        self.midi_editor_restore_bounds = self.midi_editor_bounds.clone();
        self.midi_editor_window_epoch = self.midi_editor_window_epoch.wrapping_add(1);
        self.midi_editor_viewport_live = false;
        self.midi_editor_focus_requested = false;
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
            self.midi_editor_bounds.clone(),
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
        let native_key_events = self
            .native_key_capture
            .as_ref()
            .map(NativeKeyCapture::take_events)
            .unwrap_or_default();
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let chords = if cfg!(windows) && self.native_key_capture.is_some() {
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
        for track in &self.data.project.tracks {
            let height = self.arrange_view_state.track_height(track.id) * scale;
            self.arrange_view_state.set_track_height(track.id, height);
        }
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
        egui::Window::new("Key Commands")
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
        egui::Window::new("Assign Shortcut")
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
        if cancel || !open {
            self.key_capture_target = None;
            self.status_message = "Shortcut capture canceled".to_owned();
        }
    }

    fn show_key_conflict_prompt(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_key_assignment.clone() else {
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
        let mut decision = None;
        egui::Window::new("Shortcut conflict")
            .id(Id::new("key-command-conflict"))
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
            return;
        }

        let mut open = true;
        let mut activate_index = None;
        let mut move_request = None;
        let mut lock_index = None;
        let mut create_workspace = false;
        let mut preset_action = None;
        let mut project_preset_renamed = false;
        egui::Window::new("Workspace Organizer")
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
            self.midi_editor_focus_requested = true;
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
                    menu_command(ui, "Preferences...", "", false);
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
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Cut", "Ctrl+X"),
                            ("Copy", "Ctrl+C"),
                            ("Paste", "Ctrl+V"),
                            ("Paste at Origin", "Alt+V"),
                            ("Delete", "Backspace"),
                            ("Split at Cursor", "Alt+X"),
                            ("Split Loop", ""),
                        ],
                    );
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
                                ("Group Channel", ""),
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
                    menu_command(ui, "Duplicate Tracks", "", false);
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
                    if menu_command(ui, "Project Setup...", "Shift+S", true).clicked() {
                        self.project_setup_open = true;
                        ui.close();
                    }
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Pool", "Ctrl+P"),
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
                    disabled_menu_commands(
                        ui,
                        &[
                            ("Metronome Setup...", ""),
                            ("Metronome On", "C"),
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
                            ("Plug-in Information", ""),
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
                            self.channel_settings_window.channel_bus = None;
                            self.channel_settings_window.open = true;
                            self.channel_settings_window.focus_requested = true;
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
                        self.midi_editor_focus_requested = true;
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
        egui::Window::new("About OpenTrack-")
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
    }

    fn show_mixer_window(&mut self, ui: &mut egui::Ui) {
        self.mixer_window.viewport_live = false;
        self.mixer_window.focus_requested = false;
        let panel_rect = ui.max_rect();
        self.mixer_window.bounds = Some(docked_bounds(panel_rect));
        let palette = theme::palette(ui);
        ui.painter().rect_filled(panel_rect, 0.0, palette.surface);
        if docked_panel_header(ui, "Mixer", false) {
            self.mixer_window.open = false;
            return;
        }
        let body_rect = Rect::from_min_max(
            Pos2::new(
                panel_rect.left() + 5.0,
                panel_rect.top() + PANEL_HEADER_HEIGHT,
            ),
            Pos2::new(panel_rect.right() - 5.0, panel_rect.bottom() - 3.0),
        );
        let modified = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt("mixer-panel-body")
                    .max_rect(body_rect),
                |ui| {
                    ui.set_clip_rect(body_rect);
                    ui.set_width(body_rect.width());
                    show_mixer(
                        ui,
                        &mut self.data.project,
                        &self.data.track_meter_levels,
                        self.data.input_channel_meter_levels,
                        self.data.output_channel_meter_levels,
                        true,
                        false,
                        &mut self.channel_settings_window,
                        &mut self.compressor_editor,
                        &mut self.channel_settings_ui,
                    )
                },
            )
            .inner;
        ui.painter().rect_stroke(
            panel_rect,
            0.0,
            Stroke::new(1.0, palette.grid),
            egui::StrokeKind::Inside,
        );
        if modified {
            self.rebuild_audio_plan();
            self.mark_modified();
        }
    }

    fn show_channel_settings_window(&mut self, ctx: &egui::Context) {
        if !self.channel_settings_window.open {
            self.channel_settings_window.viewport_live = false;
            self.channel_settings_ui.applied_window_level = None;
            return;
        }
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
        if self.channel_settings_window.focus_requested
            && self.channel_settings_window.viewport_live
        {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
        }
        self.channel_settings_window.focus_requested = false;
        let mut builder = viewport_builder(
            format!("VST Audio Channel Settings - {track_name}"),
            Vec2::new(902.0, 430.0),
            Vec2::new(820.0, 420.0),
            restore_bounds.as_ref(),
        );
        if restore_bounds.is_none() {
            builder = builder.with_position(Pos2::new(382.0, 145.0));
        }
        let mut open = true;
        let mut modified = false;
        let mut bounds = None;
        ctx.show_viewport_immediate(id, builder, |window_ctx, _class| {
            self.handle_native_window_input(window_ctx);
            open &= !window_ctx.input(|input| input.viewport().close_requested());
            egui::CentralPanel::default()
                .frame(egui::Frame::central_panel(&window_ctx.style()).inner_margin(4.0))
                .show(window_ctx, |ui| {
                    modified |= show_mixer(
                        ui,
                        &mut self.data.project,
                        &self.data.track_meter_levels,
                        self.data.input_channel_meter_levels,
                        self.data.output_channel_meter_levels,
                        false,
                        true,
                        &mut self.channel_settings_window,
                        &mut self.compressor_editor,
                        &mut self.channel_settings_ui,
                    );
                });
            channel_settings::setup(window_ctx, &mut self.channel_settings_ui);
            bounds = native_window_bounds(window_ctx);
        });
        if let Some(action) = self.channel_settings_ui.pending_action.take() {
            modified |= self.apply_channel_settings_action(action);
        }
        if self.channel_settings_ui.applied_window_level
            != Some(self.channel_settings_ui.preferences.always_on_top)
        {
            let level = if self.channel_settings_ui.preferences.always_on_top {
                egui::viewport::WindowLevel::AlwaysOnTop
            } else {
                egui::viewport::WindowLevel::Normal
            };
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::WindowLevel(level));
            self.channel_settings_ui.applied_window_level =
                Some(self.channel_settings_ui.preferences.always_on_top);
        }
        self.channel_settings_window.open = open;
        self.channel_settings_window.viewport_live = open;
        if !open {
            self.channel_settings_ui.applied_window_level = None;
        }
        if restore_bounds.is_some() {
            self.channel_settings_window.restore_bounds = None;
        }
        if let Some(bounds) = bounds {
            self.channel_settings_window.bounds = Some(bounds);
        }
        if modified {
            self.rebuild_audio_plan();
            self.mark_modified();
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

    fn show_compressor_editor_window(&mut self, ctx: &egui::Context) {
        if !self.compressor_editor.window.open {
            self.compressor_editor.window.viewport_live = false;
            return;
        }
        let Some(track_id) = self.compressor_editor.track_id else {
            self.compressor_editor.window.open = false;
            self.compressor_editor.window.viewport_live = false;
            return;
        };
        let insert_index = self.compressor_editor.insert_index;
        let track_name = self
            .data
            .project
            .channels()
            .find(|track| track.id == track_id)
            .map(|track| track.name.clone());
        let Some(track_name) = track_name else {
            self.compressor_editor.window.open = false;
            self.compressor_editor.window.viewport_live = false;
            return;
        };
        let restore_bounds = if self.compressor_editor.window.viewport_live {
            None
        } else {
            self.compressor_editor
                .window
                .restore_bounds
                .clone()
                .or_else(|| self.compressor_editor.window.bounds.clone())
        };
        let id = egui::ViewportId::from_hash_of((
            "compressor-editor",
            self.compressor_editor.window.epoch,
        ));
        if self.compressor_editor.window.focus_requested
            && self.compressor_editor.window.viewport_live
        {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
        }
        self.compressor_editor.window.focus_requested = false;
        let title = format!("{track_name}: Ins. {} - Compressor", insert_index + 1);
        let mut builder = viewport_builder(
            title,
            Vec2::new(464.0, 227.0),
            Vec2::new(430.0, 210.0),
            restore_bounds.as_ref(),
        );
        if restore_bounds.is_none() {
            builder = builder.with_position(Pos2::new(1326.0, 330.0));
        }
        let mut open = true;
        let mut modified = false;
        let mut bounds = None;
        ctx.show_viewport_immediate(id, builder, |window_ctx, _class| {
            self.handle_native_window_input(window_ctx);
            open &= !window_ctx.input(|input| input.viewport().close_requested());
            egui::CentralPanel::default().show(window_ctx, |ui| {
                if let Some(track) = self.data.project.channel_by_id_mut(track_id) {
                    if let Some(insert) = track.inserts.get_mut(insert_index) {
                        if let InsertEffectKind::Compressor {
                            threshold_db,
                            ratio,
                            attack_ms,
                            hold_ms,
                            release_ms,
                            analysis,
                            makeup_db,
                            soft_knee,
                            auto_makeup,
                            auto_release,
                            rms_mode,
                            live,
                        } = &mut insert.kind
                        {
                            modified |= show_compressor_editor(
                                ui,
                                threshold_db,
                                ratio,
                                attack_ms,
                                hold_ms,
                                release_ms,
                                analysis,
                                makeup_db,
                                soft_knee,
                                auto_makeup,
                                auto_release,
                                rms_mode,
                                live,
                                &mut insert.enabled,
                            );
                        } else {
                            ui.label("The selected insert is not a Compressor.");
                        }
                    } else {
                        ui.label("The selected insert is no longer available.");
                    }
                }
            });
            bounds = native_window_bounds(window_ctx);
        });
        self.compressor_editor.window.open = open;
        self.compressor_editor.window.viewport_live = open;
        if restore_bounds.is_some() {
            self.compressor_editor.window.restore_bounds = None;
        }
        if let Some(bounds) = bounds {
            self.compressor_editor.window.bounds = Some(bounds);
        }
        if modified {
            self.rebuild_audio_plan();
            self.mark_modified();
        }
    }

    fn show_media_window(&mut self, ctx: &egui::Context) {
        if !self.media_window.open {
            self.media_window.viewport_live = false;
            return;
        }
        let title = match self.data.selected_media {
            1 => "Loop Browser",
            2 => "Sound Browser",
            _ => "MediaBay",
        };
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
        }
        self.media_window.focus_requested = false;
        let builder = viewport_builder(
            title,
            Vec2::new(760.0, 560.0),
            Vec2::new(500.0, 360.0),
            restore_bounds.as_ref(),
        );
        let mut open = true;
        let mut modified = false;
        let mut bounds = None;
        ctx.show_viewport_immediate(id, builder, |window_ctx, _class| {
            self.handle_native_window_input(window_ctx);
            open &= !window_ctx.input(|input| input.viewport().close_requested());
            egui::CentralPanel::default().show(window_ctx, |ui| {
                modified |= show_media(ui, &mut self.data.selected_media);
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
        if modified {
            self.mark_modified();
        }
    }

    fn show_midi_editor_window(&mut self, ctx: &egui::Context) {
        if !self.midi_editor_open {
            self.midi_editor_viewport_live = false;
            self.midi_editor_focus_requested = false;
            self.midi_note_drag = None;
            self.midi_controller_drag = None;
            return;
        }
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
        let title = selected_track
            .zip(selected_part_id)
            .and_then(|(index, part_id)| {
                let track = self.data.project.tracks.get(index)?;
                let part = track.midi_parts.iter().find(|part| part.id == part_id)?;
                Some(format!("MIDI Editor — {} — {}", track.name, part.name))
            })
            .unwrap_or_else(|| "MIDI Editor".to_owned());
        let restore_bounds = if self.midi_editor_viewport_live {
            None
        } else {
            self.midi_editor_restore_bounds
                .clone()
                .or_else(|| self.midi_editor_bounds.clone())
        };
        let id = egui::ViewportId::from_hash_of(("midi-editor", self.midi_editor_window_epoch));
        if self.midi_editor_focus_requested && self.midi_editor_viewport_live {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
        }
        self.midi_editor_focus_requested = false;
        let builder = viewport_builder(
            title,
            Vec2::new(900.0, 600.0),
            Vec2::new(680.0, 420.0),
            restore_bounds.as_ref(),
        );
        let mut open = true;
        let mut modified = false;
        let mut bounds = None;
        ctx.show_viewport_immediate(id, builder, |window_ctx, _class| {
            self.handle_native_window_input(window_ctx);
            open &= !window_ctx.input(|input| input.viewport().close_requested());
            open &= self.midi_editor_open;
            egui::CentralPanel::default().show(window_ctx, |ui| {
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
                        &mut self.selected_midi_note_id,
                        &mut self.midi_note_drag,
                        &mut self.midi_controller_drag,
                    );
                } else {
                    self.midi_note_drag = None;
                    self.midi_controller_drag = None;
                    ui.label("Select a MIDI part in the Project view, or create one there.");
                }
            });
            bounds = native_window_bounds(window_ctx);
        });
        self.midi_editor_viewport_live = open;
        if restore_bounds.is_some() {
            self.midi_editor_restore_bounds = None;
        }
        if let Some(bounds) = bounds {
            self.midi_editor_bounds = Some(bounds);
        }
        self.midi_editor_open = open;
        if modified {
            self.rebuild_audio_plan();
            self.mark_modified();
        }
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
            Pos2::new(panel_rect.right(), panel_rect.bottom() - 17.0),
        );
        let readout = Rect::from_min_size(
            Pos2::new(panel_rect.left(), panel_rect.bottom() - 17.0),
            Vec2::new(370.0, 16.0),
        );
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
        ui.painter().with_clip_rect(readout).text(
            readout.left_center() + Vec2::new(3.0, 0.0),
            Align2::LEFT_CENTER,
            format!("Rec: {rate} - {recording_format_label} - Max: {maximum}"),
            FontId::proportional(10.0),
            palette.text,
        );
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
        let mut modified = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt("project-panel-body")
                    .max_rect(body_rect),
                |ui| {
                    ui.set_clip_rect(body_rect);
                    ui.set_width(body_rect.width());
                    show_project(
                        ui,
                        &mut self.data.project,
                        &mut self.data.transport.position_beats,
                        self.data.transport.cycle,
                        beats_per_bar,
                        sample_rate,
                        &self.data.track_meter_levels,
                        &self.data.audio_sources,
                        recording_preview.as_ref(),
                        &mut self.arrange_view_state,
                        &mut self.midi_editor_open,
                        &mut self.midi_editor_focus_requested,
                        &mut self.channel_settings_window,
                        &mut self.track_rename_dialog,
                        can_remove,
                        &mut context_action,
                    )
                },
            )
            .inner;
        if (self.data.transport.position_beats - position_before).abs() > f64::EPSILON
            && let Some(engine) = &self.audio_engine
        {
            engine.seek_seconds(
                self.data.transport.position_beats.max(0.0) * 60.0
                    / self.data.project.tempo_bpm.max(1.0),
            );
        }
        let zoom_rect = Rect::from_min_size(
            Pos2::new(panel_rect.right() - 113.0, panel_rect.bottom() - 17.0),
            Vec2::new(113.0, 16.0),
        );
        project_controls::zoom_control(ui, zoom_rect, &mut self.arrange_view_state.timeline_zoom);
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
            Some(project_controls::Action::ShowAutomation(id)) => {
                modified |= self.arrange_view_state.automation_visible.insert(id);
            }
            Some(project_controls::Action::HideAllAutomation) => {
                if !self.arrange_view_state.automation_visible.is_empty() {
                    self.arrange_view_state.automation_visible.clear();
                    modified = true;
                }
            }
            None => {}
        }
        if modified {
            self.mark_modified();
            self.rebuild_audio_plan();
        }
    }

    fn show_docked_workspace(&mut self, ui: &mut egui::Ui) {
        let area = ui.max_rect();
        let project_open = self.project_window.open;
        let mixer_open = self.mixer_window.open;
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
            return;
        };
        let title = match dialog.kind {
            TrackKind::Audio => "Add Audio Track",
            TrackKind::Instrument => "Add Instrument Track",
            TrackKind::Midi => "Add MIDI Track",
        };
        let configuration = match dialog.kind {
            TrackKind::Audio => "Audio Track Configuration",
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
        egui::Window::new(title)
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
                        (ToolbarIcon::Range, "Range Selection", EditTool::Range, true),
                        (ToolbarIcon::Split, "Split", EditTool::Split, true),
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
                    for (icon, tooltip, tool) in [(ToolbarIcon::Draw, "Draw", EditTool::Draw)] {
                        let response =
                            toolbar_icon_button(ui, icon, tooltip, self.edit_tool == tool, true);
                        if response.clicked() {
                            self.edit_tool = tool;
                        }
                    }
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
                    let arrow_center =
                        Pos2::new(selector_rect.right() - 7.0, selector_rect.center().y);
                    ui.painter().add(egui::Shape::convex_polygon(
                        vec![
                            arrow_center + Vec2::new(-3.0, -1.5),
                            arrow_center + Vec2::new(3.0, -1.5),
                            arrow_center + Vec2::new(0.0, 2.0),
                        ],
                        theme::palette(ui).text,
                        Stroke::NONE,
                    ));
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
        let palette = theme::palette(ui);
        let content = ui.available_rect_before_wrap();
        let origin = content.min;
        let panel = Rect::from_min_size(origin, content.size());
        let painter = ui.painter().clone();
        painter.rect_filled(panel, egui::CornerRadius::ZERO, palette.surface_raised);
        painter.rect_stroke(
            panel,
            egui::CornerRadius::ZERO,
            Stroke::new(1.0, palette.grid),
            egui::StrokeKind::Inside,
        );
        for x in [24.0, 96.0, 248.0, 486.0, 612.0, 746.0] {
            painter.line_segment(
                [origin + Vec2::new(x, 4.0), origin + Vec2::new(x, 59.0)],
                Stroke::new(1.0, palette.grid),
            );
        }

        // Compact original controls follow the captured Transport Panel's one-row layout.
        for (index, height) in [25.0, 38.0, 31.0, 44.0, 36.0].into_iter().enumerate() {
            let x = 5.0 + index as f32 * 3.5;
            let bar =
                Rect::from_min_size(origin + Vec2::new(x, 55.0 - height), Vec2::new(2.0, height));
            painter.rect_filled(bar, egui::CornerRadius::ZERO, palette.text);
        }

        let normal = transport_panel_button(
            ui,
            origin,
            "record-mode",
            [28.0, 6.0, 63.0, 15.0],
            "Normal ▾",
            "Linear Record Mode",
            false,
            false,
        );
        let mix = transport_panel_button(
            ui,
            origin,
            "midi-record-mode",
            [28.0, 23.0, 63.0, 15.0],
            "Mix (MIDI) ▾",
            "Cycle Record Mode",
            false,
            false,
        );
        let auto = transport_panel_button(
            ui,
            origin,
            "auto-quantize",
            [28.0, 41.0, 31.0, 14.0],
            "AUTO",
            "Automatic MIDI Record Quantize",
            false,
            false,
        );
        let _ = (normal, mix, auto);
        transport_panel_text(
            ui,
            origin,
            [61.0, 41.0, 30.0, 14.0],
            "OFF",
            palette.muted,
            9.0,
            true,
        );

        let left = transport_panel_button(
            ui,
            origin,
            "left-locator",
            [98.0, 7.0, 145.0, 21.0],
            &format!(
                "L  {}",
                format_transport_position(
                    if self.data.project.cycle_mode == CycleMode::Skip {
                        self.data.project.loop_end_beats
                    } else {
                        self.data.project.loop_start_beats
                    },
                    self.data.project.beats_per_bar
                )
            ),
            "Left Locator Position",
            false,
            false,
        );
        let right = transport_panel_button(
            ui,
            origin,
            "right-locator",
            [98.0, 34.0, 145.0, 21.0],
            &format!(
                "R  {}",
                format_transport_position(
                    if self.data.project.cycle_mode == CycleMode::Skip {
                        self.data.project.loop_start_beats
                    } else {
                        self.data.project.loop_end_beats
                    },
                    self.data.project.beats_per_bar
                )
            ),
            "Right Locator Position",
            false,
            false,
        );
        let _ = (left, right);

        let position = transport_panel_button(
            ui,
            origin,
            "primary-time",
            [253.0, 6.0, 130.0, 23.0],
            &format_transport_position(
                self.data.transport.position_beats,
                self.data.project.beats_per_bar,
            ),
            "Primary Time Display",
            false,
            false,
        );
        let seconds = self.data.transport.position_beats.max(0.0) * 60.0
            / self.data.project.tempo_bpm.max(1.0);
        let time_display = format!(
            "{}:{:02}:{:02}.{:03}",
            (seconds / 3600.0).floor() as u64,
            ((seconds / 60.0).floor() as u64) % 60,
            seconds.floor() as u64 % 60,
            (seconds.fract() * 1000.0).round() as u16
        );
        transport_panel_text(
            ui,
            origin,
            [384.0, 6.0, 97.0, 23.0],
            &time_display,
            palette.text,
            11.0,
            true,
        );
        let _ = position;

        let previous = transport_panel_button(
            ui,
            origin,
            "left-locator-jump",
            [252.0, 34.0, 25.0, 23.0],
            "|◀",
            "Goto Previous Marker / Zero",
            true,
            false,
        );
        let rewind = transport_panel_button(
            ui,
            origin,
            "rewind",
            [278.0, 34.0, 25.0, 23.0],
            "◀◀",
            "Rewind",
            true,
            false,
        );
        let fast_forward = transport_panel_button(
            ui,
            origin,
            "fast-forward",
            [304.0, 34.0, 25.0, 23.0],
            "▶▶",
            "Fast Forward",
            true,
            false,
        );
        let next = transport_panel_button(
            ui,
            origin,
            "right-locator-jump",
            [330.0, 34.0, 25.0, 23.0],
            "▶|",
            "Goto Next Marker / Project End",
            false,
            false,
        );
        let cycle = transport_panel_button_tinted(
            ui,
            origin,
            "cycle",
            [356.0, 34.0, 25.0, 23.0],
            "↻",
            "Cycle",
            true,
            self.data.transport.cycle,
            Some(crate::timeline::cycle_color(self.data.project.cycle_mode)),
        );
        let stop = transport_panel_button(
            ui,
            origin,
            "stop",
            [382.0, 34.0, 25.0, 23.0],
            "■",
            "Stop",
            true,
            false,
        );
        let play = transport_panel_button(
            ui,
            origin,
            "play",
            [408.0, 34.0, 25.0, 23.0],
            "▶",
            "Play",
            true,
            self.data.transport.playing,
        );
        let record = transport_panel_button(
            ui,
            origin,
            "record",
            [434.0, 34.0, 25.0, 23.0],
            "●",
            "Record",
            true,
            self.data.transport.recording,
        );
        let _ = next;

        let click = transport_panel_button(
            ui,
            origin,
            "click",
            [491.0, 7.0, 37.0, 15.0],
            "CLICK",
            "Metronome/Click (use CTRL to open metronome settings)",
            false,
            false,
        );
        let tempo_mode = transport_panel_button(
            ui,
            origin,
            "tempo-mode",
            [491.0, 25.0, 54.0, 15.0],
            "TEMPO TRACK",
            "Activate Tempo Track (use CTRL to open project tempo track editor)",
            false,
            false,
        );
        let sync_mode = transport_panel_button(
            ui,
            origin,
            "sync-mode",
            [491.0, 43.0, 42.0, 15.0],
            "SYNC INT.",
            "Activate External Sync (use CTRL to open synchronisation settings)",
            false,
            false,
        );
        let _ = (click, tempo_mode, sync_mode);
        transport_panel_text(
            ui,
            origin,
            [530.0, 7.0, 22.0, 15.0],
            "OFF",
            palette.muted,
            9.0,
            true,
        );
        transport_panel_text(
            ui,
            origin,
            [550.0, 25.0, 23.0, 15.0],
            &format!(
                "{}/{}",
                self.data.project.beats_per_bar, self.data.project.beat_unit
            ),
            palette.text,
            8.0,
            true,
        );
        transport_panel_text(
            ui,
            origin,
            [574.0, 25.0, 34.0, 15.0],
            &format!("{:.3}", self.data.project.tempo_bpm),
            palette.text,
            8.0,
            true,
        );
        transport_panel_text(
            ui,
            origin,
            [535.0, 43.0, 73.0, 15.0],
            "Offline",
            palette.muted,
            9.0,
            true,
        );

        let show_markers = transport_panel_button(
            ui,
            origin,
            "show-markers",
            [616.0, 6.0, 36.0, 15.0],
            "SHOW",
            "Open Markers",
            false,
            false,
        );
        let marker_selector = transport_panel_button(
            ui,
            origin,
            "marker-selector",
            [655.0, 6.0, 74.0, 15.0],
            "MARKER",
            "",
            false,
            false,
        );
        let _ = (show_markers, marker_selector);
        for row in 0..3 {
            for column in 0..5 {
                let marker = row * 5 + column + 1;
                let response = transport_panel_button(
                    ui,
                    origin,
                    format!("marker-{marker}"),
                    [
                        616.0 + column as f32 * 26.0,
                        24.0 + row as f32 * 12.0,
                        25.0,
                        11.0,
                    ],
                    &marker.to_string(),
                    if marker == 1 {
                        "Jump to Marker 1 (use ALT to set)"
                    } else {
                        ""
                    },
                    false,
                    false,
                );
                let _ = response;
            }
        }

        for index in 0..5 {
            let x = 753.0 + index as f32 * 14.0;
            let meter = Rect::from_min_size(origin + Vec2::new(x, 8.0), Vec2::new(8.0, 47.0));
            painter.rect_filled(meter, egui::CornerRadius::ZERO, palette.surface);
            painter.rect_stroke(
                meter,
                egui::CornerRadius::ZERO,
                Stroke::new(1.0, palette.grid),
                egui::StrokeKind::Inside,
            );
            for y in (12..54).step_by(6) {
                painter.line_segment(
                    [
                        origin + Vec2::new(x + 2.0, y as f32),
                        origin + Vec2::new(x + 6.0, y as f32),
                    ],
                    Stroke::new(1.0, palette.muted),
                );
            }
        }

        if previous.clicked() {
            self.data.transport.position_beats = self.data.project.loop_start_beats;
            self.set_transport_playing(false);
            if let Some(engine) = &self.audio_engine {
                engine.seek_seconds(
                    self.data.transport.position_beats * 60.0
                        / self.data.project.tempo_bpm.max(1.0),
                );
            }
        }
        if rewind.clicked() || fast_forward.clicked() {
            let delta = if rewind.clicked() { -1.0 } else { 1.0 };
            self.data.transport.position_beats =
                (self.data.transport.position_beats + delta).max(0.0);
            if let Some(engine) = &self.audio_engine {
                engine.seek_seconds(
                    self.data.transport.position_beats * 60.0
                        / self.data.project.tempo_bpm.max(1.0),
                );
            }
        }
        if cycle.clicked() {
            self.toggle_cycle();
        }
        if stop.clicked() {
            self.set_transport_playing(false);
            self.data.transport.recording = false;
        }
        if play.clicked() {
            if self.is_recording() {
                self.finish_recording();
            }
            self.set_transport_playing(true);
            self.data.transport.recording = false;
        }
        if record.clicked() {
            self.toggle_recording();
        }
    }

    fn show_status(&self, ui: &mut egui::Ui) {
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
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {} Hz · {} ch",
                            engine.device_name, engine.sample_rate, engine.channels
                        ))
                        .small()
                        .color(theme::palette(ui).muted),
                    );
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
}

impl eframe::App for OpenTrackApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from(visuals.panel_fill).to_array()
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
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
        if meter_source_active || meter_is_decaying {
            ctx.request_repaint_after(std::time::Duration::from_millis(20));
        }
        self.poll_audio_import_task();
        if self.audio_import_task.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
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
        if self.project_open && self.show_transport {
            egui::Window::new("Transport Panel")
                .id(egui::Id::new("floating-transport-panel-m1-position-v5"))
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .movable(true)
                .constrain(false)
                .default_pos(Pos2::new(552.0, 663.0))
                .fixed_size(Vec2::new(837.0, 69.0))
                .frame(
                    egui::Frame::new()
                        .fill(theme::palette(ui).surface_raised)
                        .stroke(Stroke::new(1.0, theme::palette(ui).grid))
                        .inner_margin(egui::Margin::same(2)),
                )
                .show(ui.ctx(), |ui| self.show_transport(ui));
        }
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
                if self.project_open {
                    self.show_docked_workspace(ui);
                }
            });
        if self.project_open {
            self.show_track_add_dialog(&ctx);
            self.show_event_colors_dialog(&ctx);
            self.show_channel_settings_window(&ctx);
            self.show_compressor_editor_window(&ctx);
            self.show_media_window(&ctx);
            self.show_midi_editor_window(&ctx);
            self.show_workspace_organizer(&ctx);
            self.show_track_rename_dialog(&ctx);
            self.show_track_delete_prompt(&ctx);
        }
        self.show_key_commands_window(&ctx);
        self.show_unsaved_prompt(&ctx);
        self.show_project_open_error(&ctx);
        self.show_project_folder_setup(&ctx);
        self.show_about_window(&ctx);
        self.show_audio_device_setup(&ctx);
        self.show_vst_connections(&ctx);
        if self.project_open {
            self.show_project_setup(&ctx);
        }
        if !self.is_recording() {
            self.project_history.finish_frame(
                &self.data.project,
                pointer_released || (key_pressed && !pointer_down),
            );
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "dock-state", &self.dock_state);
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
        TrackKind::Instrument => "INSTRUMENT",
        TrackKind::Midi => "MIDI",
    }
}

fn track_kind_color(kind: TrackKind) -> Color32 {
    match kind {
        TrackKind::Audio => theme::TRACK_AUDIO,
        TrackKind::Instrument => theme::TRACK_INSTRUMENT,
        TrackKind::Midi => theme::TRACK_MIDI,
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

fn generator_waveform_name(waveform: GeneratorWaveform) -> &'static str {
    match waveform {
        GeneratorWaveform::Sine => "Sine",
        GeneratorWaveform::Square => "Square",
        GeneratorWaveform::Sawtooth => "Sawtooth",
        GeneratorWaveform::Triangle => "Triangle",
        GeneratorWaveform::WhiteNoise => "White noise",
        GeneratorWaveform::BrownNoise => "Brown noise",
        GeneratorWaveform::PinkNoise => "Pink noise",
    }
}

fn instrument_waveform_name(waveform: InstrumentWaveform) -> &'static str {
    match waveform {
        InstrumentWaveform::Sine => "Sine oscillator",
        InstrumentWaveform::Square => "Square oscillator",
        InstrumentWaveform::Sawtooth => "Sawtooth oscillator",
        InstrumentWaveform::Triangle => "Triangle oscillator",
    }
}

/// Compact Inspector fields with original, code-drawn level/balance/time symbols.
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
        let center = Pos2::new(icon.right() - 12.0, icon.center().y);
        let painter = ui.painter();
        let stroke = Stroke::new(1.2, palette.text);
        let point = |x, y| center + Vec2::new(x, y);
        match symbol {
            0 => {
                painter.add(egui::Shape::convex_polygon(
                    vec![point(-8.0, 4.0), point(7.0, -3.0), point(7.0, 4.0)],
                    palette.text,
                    Stroke::NONE,
                ));
            }
            1 => {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        point(-7.0, -3.0),
                        point(0.0, 3.0),
                        point(7.0, -3.0),
                        point(7.0, 4.0),
                        point(-7.0, 4.0),
                    ],
                    palette.text,
                    Stroke::NONE,
                ));
            }
            _ => {
                painter.circle_stroke(center, 6.0, stroke);
                painter.line_segment([center, point(0.0, -4.0)], stroke);
                painter.line_segment([center, point(3.0, 1.0)], stroke);
                painter.line_segment([point(-3.0, -8.0), point(3.0, -8.0)], stroke);
                painter.line_segment([point(-12.0, 0.0), point(-7.0, 0.0)], stroke);
                painter.line_segment([point(7.0, 0.0), point(12.0, 0.0)], stroke);
                painter.add(egui::Shape::convex_polygon(
                    vec![point(-12.0, 0.0), point(-9.0, -2.5), point(-9.0, 2.5)],
                    palette.text,
                    Stroke::NONE,
                ));
                painter.add(egui::Shape::convex_polygon(
                    vec![point(12.0, 0.0), point(9.0, -2.5), point(9.0, 2.5)],
                    palette.text,
                    Stroke::NONE,
                ));
            }
        }
        let mut field = egui::DragValue::new(value)
            .range(range)
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
            let text = if symbol == 1 {
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
            0 => ((*value + 60.0) / 60.0).clamp(0.0, 1.0),
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

fn project_track_header_fill(palette: theme::Palette, selected: bool) -> Color32 {
    if selected {
        blend_color(palette.surface_raised, palette.accent, 0.16)
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
) -> bool {
    project_inspector::show(ui, project, settings)
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
    } else if response.hovered() {
        palette.surface
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    if fill != Color32::TRANSPARENT {
        painter.rect_filled(rect, 0.0, fill);
    }
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, palette.grid.gamma_multiply(0.7)),
        egui::StrokeKind::Inside,
    );
    let ink = if close_hover {
        Color32::WHITE
    } else {
        palette.text
    };
    let center = rect.center();
    let radius = 3.5;
    painter.line_segment(
        [
            center + Vec2::new(-radius, -radius),
            center + Vec2::new(radius, radius),
        ],
        Stroke::new(1.4, ink),
    );
    painter.line_segment(
        [
            center + Vec2::new(radius, -radius),
            center + Vec2::new(-radius, radius),
        ],
        Stroke::new(1.4, ink),
    );
    response
}

fn show_project(
    ui: &mut egui::Ui,
    project: &mut Project,
    position_beats: &mut f64,
    cycle_enabled: bool,
    beats_per_bar: u8,
    sample_rate: u32,
    meter_levels: &HashMap<u64, [f32; 2]>,
    audio_sources: &HashMap<PathBuf, AudioSource>,
    recording_preview: Option<&RecordingPreview>,
    arrange_view_state: &mut ArrangeViewState,
    midi_editor_open: &mut bool,
    midi_editor_focus_requested: &mut bool,
    channel_settings_window: &mut NativeWindowState,
    _track_rename_dialog: &mut Option<TrackRenameDialog>,
    can_remove: bool,
    context_action: &mut Option<project_controls::Action>,
) -> bool {
    let mut modified = false;
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
                modified |= show_project_inspector(ui, project, channel_settings_window);
            });
    }

    let lane_header_width = 198.0;
    let ruler_height = 22.0;
    ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
    ui.style_mut().spacing.scroll.bar_width = 14.0;
    ui.style_mut().spacing.scroll.bar_inner_margin = 1.0;
    ui.style_mut().spacing.scroll.bar_outer_margin = 1.0;
    let scroll_parent = ui.available_rect_before_wrap();
    let timeline_view_width = (scroll_parent.width() - lane_header_width - 14.0).max(1.0);
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
            track_ids
                .get(row)
                .map(|track_id| {
                    arrange_view_state.track_height(*track_id)
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
    let mut reorder_after_draw = None;
    let mut reorder_indicator = None;
    let mut clear_all_muted = false;
    let mut clear_all_solo = false;
    let mut custom_scroll_offset = None;

    let scroll_bar_rect = Rect::from_min_max(
        scroll_parent.min + Vec2::new(lane_header_width, ruler_height),
        scroll_parent.max - Vec2::new(14.0, 16.0),
    );
    let scroll_output = egui::ScrollArea::both()
        .id_salt("project-arrange-scroll")
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
                            grid_clip.right().min(scroll_bar_rect.right()),
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
                    // egui has no native pencil cursor. Draw an original small pencil
                    // at the pointer, including throughout a captured range drag.
                    ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                    if let Some(pointer) = pointer {
                        let cursor_painter = ui.ctx().layer_painter(egui::LayerId::new(
                            egui::Order::Tooltip,
                            Id::new("ruler-range-pencil"),
                        ));
                        cursor_painter.add(egui::Shape::convex_polygon(
                            vec![
                                pointer,
                                pointer + Vec2::new(3.0, -7.0),
                                pointer + Vec2::new(10.0, -14.0),
                                pointer + Vec2::new(14.0, -10.0),
                                pointer + Vec2::new(7.0, -3.0),
                            ],
                            Color32::WHITE,
                            Stroke::new(1.0, Color32::BLACK),
                        ));
                        cursor_painter.line_segment(
                            [pointer, pointer + Vec2::new(3.0, -3.0)],
                            Stroke::new(2.0, Color32::BLACK),
                        );
                    }
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
                            grid_clip.right().min(scroll_bar_rect.right()),
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
                ("≡", "Track Controls Settings", false),
                ("↔", "Toggle In-place Editor for Selected Tracks", false),
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
                let response = ui
                    .put(rect, egui::Button::new(label).small())
                    .on_hover_text(tooltip);
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
            for row in 0..visible_rows {
                let top = canvas.top() + ruler_height + row_offsets[row];
                let header = Rect::from_min_size(
                    Pos2::new(header_left, top),
                    Vec2::new(lane_header_width, row_heights[row]),
                );
                let selected = project
                    .tracks
                    .get(row)
                    .is_some_and(|track| arrange_view_state.selected_track_ids.contains(&track.id));
                painter.rect_filled(header, 0.0, project_track_header_fill(palette, selected));
                if selected {
                    painter.rect_stroke(
                        header,
                        0.0,
                        Stroke::new(1.0, palette.accent.gamma_multiply(0.75)),
                        egui::StrokeKind::Inside,
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
                let top = canvas.min.y + ruler_height + row_offsets[row];
                let row_rect = Rect::from_min_size(
                    Pos2::new(header_left, top),
                    Vec2::new(total_size.x, lane_height),
                );
                if let Some(track) = project.tracks.get_mut(row) {
                    let resize_rect = Rect::from_min_max(
                        Pos2::new(header_left, top + lane_height - 5.0),
                        Pos2::new(header_left + lane_header_width, top + lane_height + 2.0),
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
                    let arrange_response = ui.interact(
                        arrange_rect.intersect(grid_clip),
                        Id::new(("track-arrange-row", track.id)),
                        Sense::click(),
                    );
                    let header_response = ui.interact(
                        Rect::from_min_size(
                            row_rect.min,
                            Vec2::new(lane_header_width - 20.0, lane_height),
                        ),
                        Id::new(("project-track-header", track.id)),
                        Sense::click_and_drag(),
                    );
                    let name_rect = Rect::from_min_size(
                        Pos2::new(row_rect.min.x + 82.0, top + 2.0),
                        Vec2::new(lane_header_width - 102.0, 20.0),
                    );
                    let name_response = ui.interact(
                        name_rect,
                        Id::new(("track-header-name", track.id)),
                        Sense::click_and_drag(),
                    );
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
                    if header_response.dragged() || name_response.dragged() {
                        if let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                            let target = row_offsets
                                .iter()
                                .zip(&row_heights)
                                .position(|(offset, height)| {
                                    pointer.y < canvas.min.y + ruler_height + offset + height * 0.5
                                })
                                .unwrap_or(visible_rows);
                            reorder_indicator = Some(target);
                        }
                    }
                    if header_response.drag_stopped() || name_response.drag_stopped() {
                        if let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                            let insertion = row_offsets
                                .iter()
                                .zip(&row_heights)
                                .position(|(offset, height)| {
                                    pointer.y < canvas.min.y + ruler_height + offset + height * 0.5
                                })
                                .unwrap_or(visible_rows);
                            let target = if row < insertion {
                                insertion.saturating_sub(1)
                            } else {
                                insertion
                            };
                            if target != row && target < visible_rows {
                                reorder_after_draw = Some((row, target));
                            }
                        }
                    }
                    if arrange_view_state.selected_track_ids.contains(&track.id) {
                        painter.rect_filled(
                            Rect::from_min_size(row_rect.min, Vec2::new(3.0, lane_height)),
                            egui::CornerRadius::ZERO,
                            theme::palette(ui).accent,
                        );
                    }
                    let track_color = if arrange_view_state.show_track_colors {
                        project_controls::track_color(track)
                    } else {
                        theme::palette(ui).muted
                    };
                    let color_rect = Rect::from_min_size(
                        Pos2::new(row_rect.left() + lane_header_width - 20.0, top + 1.0),
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
                    paint_track_meter(
                        ui,
                        meter_rect,
                        meter_levels.get(&track.id).copied().unwrap_or([0.0; 2]),
                    );
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
                        let y = top + lane_height - 42.0;
                        let ar = Rect::from_min_size(
                            Pos2::new(row_rect.left() + 32.0, y),
                            Vec2::new(lane_header_width - 52.0, 42.0),
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
                    let compact_controls = (lane_height - if expanded { 42.0 } else { 0.0 }) < 54.0;
                    let text_painter = painter.with_clip_rect(Rect::from_min_size(
                        Pos2::new(row_rect.min.x + 32.0, top),
                        Vec2::new(lane_header_width - 32.0, lane_height),
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
                            )),
                        );
                    }
                    let channel_label = match track.kind {
                        TrackKind::Audio if track.audio_channels == 1 => "mono",
                        TrackKind::Audio => "stereo",
                        _ => track_kind_label(track.kind),
                    };
                    text_painter.text(
                        Pos2::new(row_rect.min.x + 35.0, top + 26.0),
                        Align2::LEFT_CENTER,
                        channel_label,
                        FontId::proportional(theme::type_size::SMALL),
                        track_color,
                    );
                    let button_height = 14.0;
                    for (button_index, (label, active, _fill, tooltip)) in [
                        ("M", track.muted, theme::WARNING, "Mute"),
                        (
                            "S",
                            track.solo || track.solo_defeat,
                            theme::palette(ui).accent,
                            "Solo State (Solo Defeat with ALT click)",
                        ),
                        ("R", track.armed, theme::RECORD, "Record Enable"),
                        (
                            "◉",
                            track.monitor,
                            theme::palette(ui).surface_raised,
                            "Monitor",
                        ),
                        (
                            "R",
                            track.automation_read,
                            theme::palette(ui).surface_raised,
                            "Read Enable",
                        ),
                        (
                            "W",
                            track.automation_write,
                            theme::palette(ui).surface_raised,
                            "Write Enable",
                        ),
                        (
                            "e",
                            false,
                            theme::palette(ui).surface_raised,
                            "Edit Channel Settings",
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if compact_controls && button_index == 6 {
                            continue;
                        }
                        let (offset, control_top, width) = match button_index {
                            0 => (37.0, 3.0, 22.0),
                            1 => (61.0, 3.0, 22.0),
                            2 => (88.0, 22.0, 16.0),
                            3 => (107.0, 22.0, 14.0),
                            4 => (130.0, 22.0, 22.0),
                            5 => (154.0, 22.0, 22.0),
                            _ => (177.0, 22.0, 16.0),
                        };
                        let button_rect = Rect::from_min_size(
                            Pos2::new(row_rect.left() + offset, top + control_top),
                            Vec2::new(width, button_height),
                        );
                        let response = project_inspector::button(
                            ui,
                            button_rect,
                            ("track-control", track.id, button_index),
                            if button_index == 3 { "◀" } else { label },
                            tooltip,
                            active,
                            true,
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
                                    track.automation_read = !track.automation_read;
                                    modified = true;
                                }
                                5 => {
                                    track.automation_write = !track.automation_write;
                                    modified = true;
                                }
                                6 => {
                                    project.selected_track = Some(row as u32);
                                    project.selected_midi_part_id = None;
                                    arrange_view_state.selected_track_ids.clear();
                                    arrange_view_state.selected_track_ids.insert(track.id);
                                    arrange_view_state.selection_anchor_track_id = Some(track.id);
                                    channel_settings_window.channel_bus = None;
                                    channel_settings_window.open = true;
                                    channel_settings_window.focus_requested = true;
                                    modified = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    if let Some(preview) = recording_preview {
                        if let Some((_, route)) =
                            preview.tracks.iter().find(|(id, _)| *id == track.id)
                        {
                            let duration = preview.waveform.duration_seconds();
                            let left = canvas.min.x
                                + lane_header_width
                                + (preview.start_beats / beats_per_bar_f64) as f32 * bar_width;
                            let width = (duration * project.tempo_bpm / 60.0 / beats_per_bar_f64)
                                as f32
                                * bar_width;
                            let rect = Rect::from_min_size(
                                Pos2::new(left, top),
                                Vec2::new(width.max(1.0), lane_height),
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
                                |a, b, n, c| {
                                    preview.waveform.columns(
                                        a,
                                        b,
                                        n,
                                        if *route == AudioRecordRoute::Right {
                                            1
                                        } else {
                                            c
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
                    for part in &track.midi_parts {
                        let start_bar = part.start_beats / beats_per_bar_f64;
                        let duration_bars = part.duration_beats / beats_per_bar_f64;
                        if !start_bar.is_finite() || start_bar > content_bars as f64 {
                            continue;
                        }
                        let part_left =
                            canvas.min.x + lane_header_width + start_bar as f32 * bar_width;
                        let part_right = (part_left + (duration_bars as f32 * bar_width).max(8.0))
                            .min(canvas.max.x);
                        if part_right <= part_left {
                            continue;
                        }
                        let part_rect = Rect::from_min_max(
                            Pos2::new(part_left, top + 6.0),
                            Pos2::new(part_right, top + lane_height - 6.0),
                        );
                        let part_color = track_color;
                        let part_response = ui.interact(
                            part_rect.intersect(grid_clip),
                            Id::new(("midi-part", part.id)),
                            Sense::click(),
                        );
                        if part_response.clicked() || part_response.double_clicked() {
                            let selected_track = Some(row as u32);
                            let selection_changed = project.selected_track != selected_track
                                || project.selected_midi_part_id != Some(part.id);
                            project.selected_track = selected_track;
                            project.selected_midi_part_id = Some(part.id);
                            modified |= selection_changed;
                        }
                        if part_response.double_clicked() {
                            *midi_editor_open = true;
                            *midi_editor_focus_requested = true;
                        }
                        grid_painter.rect_filled(
                            part_rect,
                            egui::CornerRadius::same(4),
                            part_color.gamma_multiply(0.20),
                        );
                        grid_painter.rect_stroke(
                            part_rect,
                            egui::CornerRadius::same(4),
                            Stroke::new(
                                if project.selected_midi_part_id == Some(part.id) {
                                    2.0
                                } else {
                                    1.0
                                },
                                if project.selected_midi_part_id == Some(part.id) {
                                    theme::palette(ui).accent
                                } else {
                                    part_color
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        let note_painter = grid_painter.with_clip_rect(part_rect.shrink(2.0));
                        for note in &part.notes {
                            let absolute_start = part.start_beats + note.start_beats;
                            let note_x = canvas.min.x
                                + lane_header_width
                                + (absolute_start / beats_per_bar_f64) as f32 * bar_width;
                            let note_width = ((note.duration_beats / beats_per_bar_f64) as f32
                                * bar_width)
                                .max(2.0);
                            let pitch_fraction = (127 - note.pitch) as f32 / 127.0;
                            let note_y = top + 4.0 + pitch_fraction * (lane_height - 12.0);
                            let note_rect = Rect::from_min_size(
                                Pos2::new(note_x, note_y),
                                Vec2::new(note_width, 3.0),
                            );
                            note_painter.rect_filled(
                                note_rect,
                                egui::CornerRadius::same(1),
                                part_color,
                            );
                        }
                        grid_painter.with_clip_rect(part_rect.shrink(5.0)).text(
                            Pos2::new(part_rect.left() + 5.0, part_rect.top() + 3.0),
                            Align2::LEFT_TOP,
                            &part.name,
                            FontId::proportional(theme::type_size::LABEL),
                            theme::palette(ui).text,
                        );
                    }
                }
            }

            let event_lanes = project
                .tracks
                .iter()
                .enumerate()
                .map(|(row, t)| audio_events::Lane {
                    id: t.id,
                    channels: t.audio_channels,
                    audio: t.kind == TrackKind::Audio,
                    rect: Rect::from_min_size(
                        Pos2::new(grid_left, canvas.top() + ruler_height + row_offsets[row]),
                        Vec2::new(grid_clip.width(), row_heights[row]),
                    ),
                    color: project_controls::track_color(t),
                })
                .collect::<Vec<_>>();
            let outcome = audio_events::show(
                ui,
                project,
                audio_sources,
                &event_lanes,
                grid_clip,
                canvas.min.x + lane_header_width,
                bar_width / beats_per_bar as f32,
                snap_grid_beats,
                &mut arrange_view_state.audio_events,
                can_remove,
            );
            modified |= outcome.changed;
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
                Stroke::new(2.0, theme::RECORD),
            );
            if pan_active
                && ui.input(|input| input.pointer.button_down(egui::PointerButton::Middle))
            {
                crate::timeline::show_move_cursor(ui, true);
            }
        });
    if let Some(offset) = custom_scroll_offset {
        arrange_view_state.timeline_scroll_x = offset.x.max(0.0);
        arrange_view_state.track_scroll_y = offset.y.max(0.0);
    } else {
        arrange_view_state.timeline_scroll_x = scroll_output.state.offset.x.max(0.0);
        arrange_view_state.track_scroll_y = scroll_output.state.offset.y.max(0.0);
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
    if let Some((from, to)) = reorder_after_draw {
        modified |= project.move_track(from, to);
    }
    if let Some(solo_track_id) = exclusive_solo_track_id {
        for track in &mut project.tracks {
            if track.id != solo_track_id && track.solo {
                track.solo = false;
            }
        }
    }
    modified
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

fn show_midi_editor(
    ui: &mut egui::Ui,
    project: &mut Project,
    track_index: usize,
    part_id: u64,
    position_quantize_beats: &mut f64,
    length_quantize_beats: &mut f64,
    insert_velocity: &mut u8,
    controller_number: &mut u8,
    selected_note_id: &mut Option<u64>,
    note_drag: &mut Option<MidiNoteDrag>,
    controller_drag: &mut Option<MidiControllerDrag>,
) -> bool {
    let Some(track) = project.tracks.get(track_index) else {
        return false;
    };
    if !matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) {
        return false;
    }
    let Some(part_index) = track.midi_parts.iter().position(|part| part.id == part_id) else {
        return false;
    };
    let track_kind = track.kind;
    let track_id = track.id;
    let track_name = track.name.clone();
    let part = &track.midi_parts[part_index];
    let part_name = part.name.clone();
    let part_duration_beats = part.duration_beats;
    let note_count = part.notes.len();
    let visible_beats = part
        .notes
        .iter()
        .map(|note| note.start_beats + note.duration_beats)
        .fold(part_duration_beats, f64::max)
        .ceil()
        .clamp(1.0, 4_096.0) as usize;
    let selected_note_index = part
        .notes
        .iter()
        .position(|note| Some(note.id) == *selected_note_id);
    if selected_note_index.is_none() {
        *selected_note_id = None;
    }
    if note_drag.is_some_and(|drag| drag.track_id != track_id || drag.part_id != part_id) {
        *note_drag = None;
    }
    if controller_drag.is_some_and(|drag| drag.track_id != track_id || drag.part_id != part_id) {
        *controller_drag = None;
    }
    let mut modified = false;

    ui.heading(format!("MIDI Editor — {track_name} — {part_name}"));
    ui.horizontal_wrapped(|ui| {
        ui.label("Click an empty cell to add a note; drag a note to move it.");
        ui.label("Drag a note's right edge to resize it.");
        ui.label("Right-click a note to remove it.");
        ui.separator();
        ui.label(format!(
            "{note_count} notes · {:.2} beats",
            part_duration_beats
        ));
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Position snap");
        egui::ComboBox::from_id_salt(("midi-position-snap", track_id, part_id))
            .selected_text(midi_quantize_label(*position_quantize_beats))
            .show_ui(ui, |ui| {
                for (beats, label) in MIDI_QUANTIZE_VALUES {
                    ui.selectable_value(position_quantize_beats, beats, label);
                }
            });
        ui.label("Insert length");
        egui::ComboBox::from_id_salt(("midi-insert-length", track_id, part_id))
            .selected_text(midi_quantize_label(*length_quantize_beats))
            .show_ui(ui, |ui| {
                for (beats, label) in MIDI_QUANTIZE_VALUES {
                    ui.selectable_value(length_quantize_beats, beats, label);
                }
            });
        if selected_note_index.is_none() {
            ui.add(egui::Slider::new(insert_velocity, 1..=127).text("Insert velocity"));
        }
    });
    if let Some(note_index) = selected_note_index {
        let part = &mut project.tracks[track_index].midi_parts[part_index];
        let note = &mut part.notes[note_index];
        ui.horizontal(|ui| {
            ui.label(format!("Selected note: {}", midi_pitch_label(note.pitch)));
            let max_length = part_duration_beats - note.start_beats;
            if max_length >= 0.0625 {
                modified |= ui
                    .add(
                        egui::Slider::new(&mut note.duration_beats, 0.0625..=max_length)
                            .logarithmic(true)
                            .text("Length"),
                    )
                    .changed();
            } else {
                ui.label("Length fixed by part boundary");
            }
            modified |= ui
                .add(egui::Slider::new(&mut note.velocity, 1..=127).text("Velocity"))
                .changed();
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Controller lane");
        egui::ComboBox::from_id_salt(("midi-controller-lane", track_id, part_id))
            .selected_text(midi_controller_label(*controller_number))
            .show_ui(ui, |ui| {
                for (controller, label) in MIDI_CONTROLLER_VALUES {
                    ui.selectable_value(controller_number, controller, label);
                }
            });
        ui.label("Click to add a point, drag to edit, right-click to remove.");
    });
    ui.separator();

    const KEYBOARD_WIDTH: f32 = 88.0;
    const RULER_HEIGHT: f32 = 26.0;
    const ROW_HEIGHT: f32 = 14.0;
    const BEAT_WIDTH: f32 = 44.0;
    let bpb = usize::from(project.beats_per_bar.max(1));
    let size = Vec2::new(
        KEYBOARD_WIDTH + visible_beats as f32 * BEAT_WIDTH,
        RULER_HEIGHT + 128.0 * ROW_HEIGHT,
    );
    let mut edit_action = None;
    egui::ScrollArea::both()
        .id_salt(("midi-piano-roll", track_id, part_id))
        .max_height(460.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (canvas, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            let painter = ui.painter_at(canvas);
            painter.rect_filled(canvas, egui::CornerRadius::ZERO, theme::palette(ui).canvas);
            painter.rect_filled(
                Rect::from_min_size(canvas.min, Vec2::new(size.x, RULER_HEIGHT)),
                egui::CornerRadius::ZERO,
                theme::palette(ui).surface,
            );

            for row in 0..128_usize {
                let pitch = (127 - row) as u8;
                let top = canvas.min.y + RULER_HEIGHT + row as f32 * ROW_HEIGHT;
                let key_rect = Rect::from_min_size(
                    Pos2::new(canvas.min.x, top),
                    Vec2::new(KEYBOARD_WIDTH, ROW_HEIGHT),
                );
                let pitch_class = pitch % 12;
                let black_key = matches!(pitch_class, 1 | 3 | 6 | 8 | 10);
                painter.rect_filled(
                    key_rect,
                    egui::CornerRadius::ZERO,
                    if black_key {
                        theme::palette(ui).midi_key_black
                    } else {
                        theme::palette(ui).midi_key_white
                    },
                );
                let row_rect = Rect::from_min_max(
                    Pos2::new(canvas.min.x + KEYBOARD_WIDTH, top),
                    Pos2::new(canvas.max.x, top + ROW_HEIGHT),
                );
                painter.rect_filled(
                    row_rect,
                    egui::CornerRadius::ZERO,
                    if black_key {
                        theme::palette(ui).midi_row_black
                    } else {
                        theme::palette(ui).midi_row_white
                    },
                );
                painter.line_segment(
                    [row_rect.left_bottom(), row_rect.right_bottom()],
                    Stroke::new(0.5, theme::palette(ui).surface_raised),
                );
                painter.text(
                    Pos2::new(key_rect.left() + 7.0, key_rect.center().y),
                    Align2::LEFT_CENTER,
                    midi_pitch_label(pitch),
                    FontId::monospace(theme::type_size::SMALL),
                    if black_key {
                        Color32::WHITE
                    } else {
                        theme::palette(ui).text
                    },
                );
            }

            for beat in 0..=visible_beats {
                let x = canvas.min.x + KEYBOARD_WIDTH + beat as f32 * BEAT_WIDTH;
                let bar_line = beat % bpb == 0;
                painter.line_segment(
                    [
                        Pos2::new(x, canvas.min.y + RULER_HEIGHT),
                        Pos2::new(x, canvas.max.y),
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
                    painter.text(
                        Pos2::new(x + 4.0, canvas.min.y + RULER_HEIGHT * 0.5),
                        Align2::LEFT_CENTER,
                        format!("Bar {}", beat / bpb + 1),
                        FontId::monospace(theme::type_size::LABEL),
                        theme::palette(ui).muted,
                    );
                }
            }

            let part_grid = Rect::from_min_max(
                Pos2::new(canvas.min.x + KEYBOARD_WIDTH, canvas.min.y + RULER_HEIGHT),
                Pos2::new(
                    (canvas.min.x + KEYBOARD_WIDTH + part_duration_beats as f32 * BEAT_WIDTH)
                        .min(canvas.max.x),
                    canvas.max.y,
                ),
            );
            let note_painter = painter.with_clip_rect(part_grid);
            for note in &project.tracks[track_index].midi_parts[part_index].notes {
                let note_rect = Rect::from_min_size(
                    Pos2::new(
                        canvas.min.x + KEYBOARD_WIDTH + note.start_beats as f32 * BEAT_WIDTH,
                        canvas.min.y + RULER_HEIGHT + (127 - note.pitch) as f32 * ROW_HEIGHT,
                    ),
                    Vec2::new(
                        (note.duration_beats as f32 * BEAT_WIDTH).max(3.0),
                        ROW_HEIGHT - 1.0,
                    ),
                );
                let note_color = track_kind_color(track_kind)
                    .gamma_multiply(0.35 + 0.65 * f32::from(note.velocity) / 127.0);
                note_painter.rect_filled(note_rect, egui::CornerRadius::same(2), note_color);
                note_painter.rect_stroke(
                    note_rect,
                    egui::CornerRadius::same(2),
                    Stroke::new(
                        if Some(note.id) == *selected_note_id {
                            2.0
                        } else {
                            1.0
                        },
                        if Some(note.id) == *selected_note_id {
                            theme::palette(ui).accent
                        } else {
                            track_kind_color(track_kind)
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
            }

            if response.drag_started() {
                let press_position = ui
                    .ctx()
                    .input(|input| input.pointer.press_origin())
                    .or_else(|| response.interact_pointer_pos());
                if let Some(pointer) = press_position {
                    let beat_x = pointer.x - canvas.min.x - KEYBOARD_WIDTH;
                    let note_y = pointer.y - canvas.min.y - RULER_HEIGHT;
                    let row = (note_y / ROW_HEIGHT).floor() as usize;
                    let beat_position = f64::from(beat_x / BEAT_WIDTH);
                    if beat_x >= 0.0
                        && note_y >= 0.0
                        && row < 128
                        && beat_position < part_duration_beats
                    {
                        let pitch = (127 - row) as u8;
                        let note_hit = project.tracks[track_index].midi_parts[part_index]
                            .notes
                            .iter()
                            .find(|note| {
                                let note_left = canvas.min.x
                                    + KEYBOARD_WIDTH
                                    + note.start_beats as f32 * BEAT_WIDTH;
                                let note_right =
                                    note_left + (note.duration_beats as f32 * BEAT_WIDTH).max(3.0);
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
                                        + (note.start_beats + note.duration_beats) as f32
                                            * BEAT_WIDTH,
                                )
                            });
                        if let Some((
                            note_id,
                            start_beats,
                            duration_beats,
                            note_pitch,
                            note_right,
                        )) = note_hit
                        {
                            let mode = if pointer.x >= note_right - 6.0 {
                                MidiNoteDragMode::Resize
                            } else {
                                MidiNoteDragMode::Move
                            };
                            *selected_note_id = Some(note_id);
                            *note_drag = Some(MidiNoteDrag {
                                track_id,
                                part_id,
                                note_id,
                                mode,
                                start_beats,
                                duration_beats,
                                pitch: note_pitch,
                                delta_beats: 0.0,
                                delta_rows: 0.0,
                            });
                        }
                    }
                }
            }

            if response.dragged() {
                let delta = response.drag_delta();
                if let Some(drag) = note_drag.as_mut()
                    && drag.track_id == track_id
                    && drag.part_id == part_id
                {
                    drag.delta_beats += f64::from(delta.x / BEAT_WIDTH);
                    drag.delta_rows += f64::from(delta.y / ROW_HEIGHT);
                }
                if let Some(drag) = *note_drag
                    && drag.track_id == track_id
                    && drag.part_id == part_id
                    && let Some(note) = project.tracks[track_index].midi_parts[part_index]
                        .notes
                        .iter_mut()
                        .find(|note| note.id == drag.note_id)
                {
                    let quantize =
                        if position_quantize_beats.is_finite() && *position_quantize_beats > 0.0 {
                            *position_quantize_beats
                        } else {
                            1.0
                        };
                    match drag.mode {
                        MidiNoteDragMode::Move => {
                            let max_start = (part_duration_beats - drag.duration_beats).max(0.0);
                            let snapped_start = ((drag.start_beats + drag.delta_beats) / quantize)
                                .round()
                                * quantize;
                            let next_start = snapped_start.clamp(0.0, max_start);
                            let next_pitch = (i16::from(drag.pitch)
                                - drag.delta_rows.round() as i16)
                                .clamp(0, 127) as u8;
                            modified |= note.start_beats != next_start || note.pitch != next_pitch;
                            note.start_beats = next_start;
                            note.pitch = next_pitch;
                        }
                        MidiNoteDragMode::Resize => {
                            let max_length = part_duration_beats - drag.start_beats;
                            if max_length >= 0.0625 {
                                let snapped_length =
                                    ((drag.duration_beats + drag.delta_beats) / quantize).round()
                                        * quantize;
                                let next_length = snapped_length.clamp(0.0625, max_length);
                                modified |= note.duration_beats != next_length;
                                note.duration_beats = next_length;
                            }
                        }
                    }
                }
            }
            if response.drag_stopped() {
                *note_drag = None;
            }

            if let Some(pointer) = response.interact_pointer_pos()
                && (response.clicked() || response.secondary_clicked())
            {
                let beat_x = pointer.x - canvas.min.x - KEYBOARD_WIDTH;
                let note_y = pointer.y - canvas.min.y - RULER_HEIGHT;
                if beat_x >= 0.0 && note_y >= 0.0 {
                    let beat_position = f64::from(beat_x / BEAT_WIDTH);
                    let beat = beat_position.floor() as usize;
                    let row = (note_y / ROW_HEIGHT).floor() as usize;
                    if beat < visible_beats && beat_position < part_duration_beats && row < 128 {
                        let pitch = (127 - row) as u8;
                        let note_hit = project.tracks[track_index].midi_parts[part_index]
                            .notes
                            .iter()
                            .find(|note| {
                                note.pitch == pitch
                                    && pointer.x
                                        >= canvas.min.x
                                            + KEYBOARD_WIDTH
                                            + note.start_beats as f32 * BEAT_WIDTH
                                    && pointer.x
                                        < canvas.min.x
                                            + KEYBOARD_WIDTH
                                            + (note.start_beats + note.duration_beats) as f32
                                                * BEAT_WIDTH
                            })
                            .map(|note| note.id);
                        if response.secondary_clicked() {
                            edit_action = note_hit.map(MidiEditAction::Remove);
                        } else if let Some(note_id) = note_hit {
                            edit_action = Some(MidiEditAction::Select(note_id));
                        } else {
                            let quantize = if position_quantize_beats.is_finite()
                                && *position_quantize_beats > 0.0
                            {
                                *position_quantize_beats
                            } else {
                                1.0
                            };
                            let snapped_position = (beat_position / quantize).floor() * quantize;
                            let remaining = part_duration_beats - snapped_position;
                            let requested_length = if length_quantize_beats.is_finite()
                                && *length_quantize_beats > 0.0
                            {
                                *length_quantize_beats
                            } else {
                                1.0
                            };
                            let duration = requested_length.min(remaining);
                            if duration >= 0.0625 {
                                edit_action = Some(MidiEditAction::Add(
                                    snapped_position,
                                    duration,
                                    pitch,
                                    (*insert_velocity).clamp(1, 127),
                                ));
                            }
                        }
                    }
                }
            }

            const CONTROLLER_LANE_HEIGHT: f32 = 126.0;
            const CONTROLLER_PLOT_HEIGHT: f32 = 88.0;
            const CONTROLLER_PLOT_TOP: f32 = 24.0;
            let controller_size = Vec2::new(
                KEYBOARD_WIDTH + visible_beats as f32 * BEAT_WIDTH,
                CONTROLLER_LANE_HEIGHT,
            );
            let (controller_canvas, controller_response) =
                ui.allocate_exact_size(controller_size, Sense::click_and_drag());
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
                Vec2::new(visible_beats as f32 * BEAT_WIDTH, CONTROLLER_PLOT_HEIGHT),
            );
            controller_painter.rect_filled(
                plot,
                egui::CornerRadius::ZERO,
                theme::palette(ui).canvas,
            );
            controller_painter.text(
                Pos2::new(
                    controller_canvas.min.x + 7.0,
                    controller_canvas.min.y + 12.0,
                ),
                Align2::LEFT_CENTER,
                midi_controller_label(*controller_number),
                FontId::monospace(theme::type_size::LABEL),
                theme::palette(ui).text,
            );
            let mut lane_events: Vec<_> = project.tracks[track_index].midi_parts[part_index]
                .controller_events
                .iter()
                .filter(|event| event.controller == *controller_number)
                .map(|event| (event.id, event.position_beats, event.value))
                .collect();
            lane_events.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for beat in 0..=visible_beats {
                let x = plot.left() + beat as f32 * BEAT_WIDTH;
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
            for value in [0_u8, 32, 64, 96, 127] {
                let y = plot.bottom() - f32::from(value) / 127.0 * CONTROLLER_PLOT_HEIGHT;
                controller_painter.line_segment(
                    [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                    Stroke::new(0.5, theme::palette(ui).surface_raised),
                );
                controller_painter.text(
                    Pos2::new(plot.left() + 3.0, y - 2.0),
                    Align2::LEFT_BOTTOM,
                    value.to_string(),
                    FontId::monospace(theme::type_size::TINY),
                    theme::palette(ui).muted,
                );
            }
            let default_value = midi_controller_default_value(*controller_number);
            let default_y =
                plot.bottom() - f32::from(default_value) / 127.0 * CONTROLLER_PLOT_HEIGHT;
            let mut previous_point = Some((plot.left(), default_y));
            for (_, position_beats, value) in &lane_events {
                let x = plot.left() + *position_beats as f32 * BEAT_WIDTH;
                let y = plot.bottom() - f32::from(*value) / 127.0 * CONTROLLER_PLOT_HEIGHT;
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
                controller_painter.circle_filled(Pos2::new(x, y), 4.0, theme::palette(ui).accent);
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
                        let x = plot.left() + *position_beats as f32 * BEAT_WIDTH;
                        let y = plot.bottom() - f32::from(*value) / 127.0 * CONTROLLER_PLOT_HEIGHT;
                        let distance_sq = (pointer.x - x).powi(2) + (pointer.y - y).powi(2);
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
            if controller_response.drag_started()
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
            if controller_response.dragged() {
                let delta = controller_response.drag_delta();
                if let Some(drag) = controller_drag.as_mut()
                    && drag.track_id == track_id
                    && drag.part_id == part_id
                {
                    drag.delta_beats += f64::from(delta.x / BEAT_WIDTH);
                    drag.delta_value += f64::from(-delta.y * 127.0 / CONTROLLER_PLOT_HEIGHT);
                }
                if let Some(drag) = *controller_drag
                    && drag.track_id == track_id
                    && drag.part_id == part_id
                    && let Some(event) = project.tracks[track_index].midi_parts[part_index]
                        .controller_events
                        .iter_mut()
                        .find(|event| event.id == drag.event_id)
                {
                    let quantize =
                        if position_quantize_beats.is_finite() && *position_quantize_beats > 0.0 {
                            *position_quantize_beats
                        } else {
                            1.0
                        };
                    let position = ((drag.start_position_beats + drag.delta_beats) / quantize)
                        .round()
                        * quantize;
                    let next_position = position.clamp(0.0, part_duration_beats);
                    let next_value = (f64::from(drag.start_value) + drag.delta_value)
                        .round()
                        .clamp(0.0, 127.0) as u8;
                    modified |= event.position_beats != next_position || event.value != next_value;
                    event.position_beats = next_position;
                    event.value = next_value;
                }
            }
            if controller_response.drag_stopped() {
                *controller_drag = None;
            }
            if let Some(pointer) = controller_response.interact_pointer_pos()
                && plot.contains(pointer)
                && (controller_response.clicked() || controller_response.secondary_clicked())
            {
                let hit = controller_event_hit(pointer);
                if controller_response.secondary_clicked() {
                    if let Some((event_id, _, _)) = hit {
                        let events = &mut project.tracks[track_index].midi_parts[part_index]
                            .controller_events;
                        let previous_len = events.len();
                        events.retain(|event| event.id != event_id);
                        modified |= events.len() != previous_len;
                    }
                } else if hit.is_none() {
                    let beat = f64::from((pointer.x - plot.left()) / BEAT_WIDTH);
                    let quantize =
                        if position_quantize_beats.is_finite() && *position_quantize_beats > 0.0 {
                            *position_quantize_beats
                        } else {
                            1.0
                        };
                    let position =
                        ((beat / quantize).floor() * quantize).clamp(0.0, part_duration_beats);
                    let value = ((plot.bottom() - pointer.y) / CONTROLLER_PLOT_HEIGHT * 127.0)
                        .round()
                        .clamp(0.0, 127.0) as u8;
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
        Some(MidiEditAction::Remove(note_id)) => {
            let notes = &mut project.tracks[track_index].midi_parts[part_index].notes;
            let before = notes.len();
            notes.retain(|note| note.id != note_id);
            if *selected_note_id == Some(note_id) {
                *selected_note_id = None;
            }
            modified |= notes.len() != before;
        }
        None => {}
    }
    modified
}

enum MidiEditAction {
    Add(f64, f64, u8, u8),
    Select(u64),
    Remove(u64),
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
    const NOTE_NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{} · {pitch}", NOTE_NAMES[usize::from(pitch % 12)])
}

fn show_mixer(
    ui: &mut egui::Ui,
    project: &mut Project,
    meter_levels: &HashMap<u64, [f32; 2]>,
    input_meter_peaks: [f32; 2],
    output_meter_peaks: [f32; 2],
    show_strips: bool,
    show_settings: bool,
    channel_settings_window: &mut NativeWindowState,
    compressor_editor: &mut CompressorEditorState,
    channel_settings_ui: &mut ChannelSettingsUiState,
) -> bool {
    if show_strips {
        show_mixer_channels(
            ui,
            project,
            meter_levels,
            input_meter_peaks,
            output_meter_peaks,
            channel_settings_window,
        )
    } else if show_settings {
        channel_settings::show(
            ui,
            project,
            meter_levels,
            compressor_editor,
            channel_settings_ui,
            &mut channel_settings_window.channel_bus,
            input_meter_peaks,
            output_meter_peaks,
        )
    } else {
        false
    }
}

fn show_mixer_channels(
    ui: &mut egui::Ui,
    project: &mut Project,
    meter_levels: &HashMap<u64, [f32; 2]>,
    input_meter_peaks: [f32; 2],
    output_meter_peaks: [f32; 2],
    channel_settings_window: &mut NativeWindowState,
) -> bool {
    mixer::show_channels(
        ui,
        project,
        meter_levels,
        input_meter_peaks,
        output_meter_peaks,
        channel_settings_window,
    )
}

fn set_channel_view_item(state: &mut ChannelSettingsUiState, item: ChannelSettingsViewItem) {
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

fn show_insert_effect_picker(ui: &mut egui::Ui, added: &mut Option<Option<InsertEffectKind>>) {
    if ui.button("No Effect").clicked() {
        *added = Some(None);
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
            *added = Some(Some(InsertEffectKind::default()));
            ui.close();
        }
    });
    ui.menu_button("Distortion", |ui| show_unimplemented_effect_group(ui));
    ui.menu_button("Dynamics", |ui| {
        if ui.button("Compressor").clicked() {
            *added = Some(Some(InsertEffectKind::Compressor {
                threshold_db: -20.0,
                ratio: 2.0,
                attack_ms: 1.0,
                hold_ms: 1.0,
                release_ms: 500.0,
                analysis: 80.0,
                makeup_db: 0.0,
                soft_knee: true,
                auto_makeup: true,
                auto_release: false,
                rms_mode: false,
                live: true,
            }));
            ui.close();
        }
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
    });
    ui.menu_button("EQ", |ui| {
        for name in ["GEQ-10", "GEQ-30", "StudioEQ"] {
            ui.add_enabled(false, egui::Button::new(name));
        }
    });
    for category in [
        "Filter",
        "Mastering",
        "Modulation",
        "Other",
        "Pitch Shift - Pitch Correct",
        "Restoration",
        "Reverb",
        "Spatial",
        "Surround",
        "SurroundDither",
        "Tools",
        "Vocoder",
    ] {
        ui.menu_button(category, |ui| show_unimplemented_effect_group(ui));
    }
    ui.separator();
    ui.menu_button("OpenTrack Effects", |ui| {
        if ui.button("Test Generator").clicked() {
            *added = Some(Some(InsertEffectKind::TestGenerator {
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

fn insert_effect_name(effect: &InsertEffectKind) -> &'static str {
    match effect {
        InsertEffectKind::TestGenerator { .. } => "Test Generator",
        InsertEffectKind::StereoDelay { .. } => "Stereo Delay",
        InsertEffectKind::Compressor { .. } => "Compressor",
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

fn show_compressor_editor(
    ui: &mut egui::Ui,
    threshold_db: &mut f32,
    ratio: &mut f32,
    attack_ms: &mut f32,
    hold_ms: &mut f32,
    release_ms: &mut f32,
    analysis: &mut f32,
    makeup_db: &mut f32,
    soft_knee: &mut bool,
    auto_makeup: &mut bool,
    auto_release: &mut bool,
    rms_mode: &mut bool,
    live: &mut bool,
    enabled: &mut bool,
) -> bool {
    let mut modified = false;
    let (canvas, _) = ui.allocate_exact_size(Vec2::new(450.0, 212.0), Sense::hover());
    let origin = canvas.min;
    let graph_threshold = *threshold_db;
    let graph_ratio = *ratio;
    let graph_makeup = *makeup_db;
    let power = Rect::from_min_size(origin, Vec2::new(22.0, 20.0));
    let response = ui.put(power, egui::Button::new("").selected(*enabled));
    crate::icons::power(
        ui.painter(),
        power.shrink2(Vec2::new(2.0, 2.0)),
        ui.visuals().text_color(),
    );
    if response.clicked() {
        *enabled = !*enabled;
        modified = true;
    }
    for (label, x) in [("▤", 26.0), ("R", 48.0), ("W", 70.0), ("↔", 92.0)] {
        ui.put(
            Rect::from_min_size(origin + Vec2::new(x, 0.0), Vec2::new(18.0, 20.0)),
            egui::Button::new(label),
        );
    }
    ui.put(
        Rect::from_min_size(origin + Vec2::new(128.0, 0.0), Vec2::new(132.0, 20.0)),
        egui::Button::new(" "),
    );

    for (label, value, range, decimals, center_x) in [
        ("Threshold", threshold_db, -60.0..=0.0, 1, 32.0),
        ("Ratio", ratio, 1.0..=20.0, 2, 92.0),
        ("Make-Up", makeup_db, 0.0..=24.0, 1, 152.0),
    ] {
        modified |= show_compressor_knob(
            ui, origin, label, value, range, decimals, center_x, 19.0, 72.0, 31.0,
        );
    }
    modified |= show_compressor_toggle(ui, origin, soft_knee, "soft knee", 72.0, 101.0);
    modified |= show_compressor_toggle(ui, origin, auto_makeup, "auto", 132.0, 101.0);

    for (label, value, range, decimals, center_x) in [
        ("Attack", attack_ms, 0.1..=200.0, 1, 32.0),
        ("Hold", hold_ms, 0.0..=500.0, 0, 92.0),
        ("Release", release_ms, 5.0..=2_000.0, 0, 152.0),
        ("Analysis", analysis, 0.0..=100.0, 0, 212.0),
    ] {
        modified |= show_compressor_knob(
            ui, origin, label, value, range, decimals, center_x, 119.0, 174.0, 133.0,
        );
    }
    modified |= show_compressor_toggle(ui, origin, auto_release, "auto", 132.0, 199.0);
    modified |= show_compressor_toggle(ui, origin, rms_mode, "peak - rms", 195.0, 199.0);
    modified |= show_compressor_toggle(ui, origin, live, "live", 370.0, 199.0);

    let graph = Rect::from_min_size(origin + Vec2::new(250.0, 20.0), Vec2::new(198.0, 174.0));
    draw_compressor_curve(ui, graph, graph_threshold, graph_ratio, graph_makeup);
    modified
}

fn show_compressor_toggle(
    ui: &mut egui::Ui,
    origin: Pos2,
    value: &mut bool,
    label: &str,
    x: f32,
    y: f32,
) -> bool {
    let button = Rect::from_min_size(origin + Vec2::new(x, y), Vec2::new(12.0, 12.0));
    let response = ui.put(button, egui::Button::new(if *value { "●" } else { "○" }));
    let label_rect =
        Rect::from_min_size(origin + Vec2::new(x + 14.0, y - 1.0), Vec2::new(64.0, 14.0));
    ui.put(
        label_rect,
        egui::Label::new(
            egui::RichText::new(label)
                .italics()
                .size(8.0)
                .color(theme::palette(ui).muted),
        ),
    );
    if response.clicked() {
        *value = !*value;
        true
    } else {
        false
    }
}

fn show_compressor_knob(
    ui: &mut egui::Ui,
    origin: Pos2,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    decimals: usize,
    center_x: f32,
    label_y: f32,
    center_y: f32,
    value_y: f32,
) -> bool {
    let mut modified = false;
    let min = *range.start();
    let max = *range.end();
    let label_rect = Rect::from_min_size(
        origin + Vec2::new(center_x - 30.0, label_y),
        Vec2::new(60.0, 12.0),
    );
    ui.painter().text(
        label_rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(9.0),
        theme::palette(ui).text,
    );
    let knob_rect =
        Rect::from_center_size(origin + Vec2::new(center_x, center_y), Vec2::splat(42.0));
    let knob = ui.interact(
        knob_rect,
        egui::Id::new(("compressor-knob", label)),
        Sense::click_and_drag(),
    );
    let drag_start = knob.id.with("drag-start");
    if knob.drag_started() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(drag_start, *value));
    }
    if knob.dragged() {
        let start = ui
            .ctx()
            .data(|data| data.get_temp::<f32>(drag_start))
            .unwrap_or(*value);
        let next = start - knob.drag_delta().y * (max - min) / 120.0;
        let next = next.clamp(min, max);
        modified |= (next - *value).abs() > f32::EPSILON;
        *value = next;
    }
    if knob.drag_stopped() {
        ui.ctx().data_mut(|data| data.remove::<f32>(drag_start));
    }
    let palette = theme::palette(ui);
    let painter = ui.painter().with_clip_rect(ui.clip_rect());
    let center = knob_rect.center();
    let radius = 18.0;
    painter.circle_filled(center, radius, palette.surface_raised);
    painter.circle_stroke(center, radius, Stroke::new(1.0, palette.grid));
    let start_angle = 3.0 * std::f32::consts::PI / 4.0;
    let end_angle = 9.0 * std::f32::consts::PI / 4.0;
    let normalized = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let pointer_angle = start_angle + (end_angle - start_angle) * normalized;
    let mut previous = None;
    for step in 0..=24 {
        let t = step as f32 / 24.0;
        let angle = start_angle + (end_angle - start_angle) * t;
        let point = center + Vec2::new(angle.cos(), angle.sin()) * 18.0;
        if let Some(previous) = previous {
            painter.line_segment(
                [previous, point],
                Stroke::new(
                    2.0,
                    if t <= normalized {
                        palette.accent
                    } else {
                        palette.grid
                    },
                ),
            );
        }
        previous = Some(point);
    }
    let pointer = center + Vec2::new(pointer_angle.cos(), pointer_angle.sin()) * 10.0;
    painter.line_segment([center, pointer], Stroke::new(2.0, palette.text));
    let value_rect = Rect::from_min_size(
        origin + Vec2::new(center_x - 30.0, value_y),
        Vec2::new(60.0, 14.0),
    );
    let value_response = ui
        .scope(|ui| {
            let palette = theme::palette(ui);
            let fill = egui::Color32::from_rgb(4, 12, 20);
            let visuals = ui.visuals_mut();
            for state in [
                &mut visuals.widgets.inactive,
                &mut visuals.widgets.hovered,
                &mut visuals.widgets.active,
            ] {
                state.bg_fill = fill;
                state.weak_bg_fill = fill;
                state.bg_stroke = Stroke::NONE;
                state.fg_stroke = Stroke::new(1.0, palette.text);
                state.corner_radius = egui::CornerRadius::ZERO;
            }
            ui.put(
                value_rect,
                egui::DragValue::new(value)
                    .range(min..=max)
                    .speed((max - min) / 500.0)
                    .fixed_decimals(decimals),
            )
        })
        .inner;
    modified |= value_response.changed();
    modified
}

fn draw_compressor_curve(ui: &egui::Ui, rect: Rect, threshold: f32, ratio: f32, makeup: f32) {
    let palette = theme::palette(ui);
    let painter = ui.painter();
    let plot_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 29.0, rect.top() + 22.0),
        Vec2::new(126.0, 126.0),
    );
    for (label, x) in [
        ("IN", rect.left() + 3.0),
        ("GR", rect.left() + 167.0),
        ("OUT", rect.left() + 186.0),
    ] {
        painter.text(
            Pos2::new(x + 5.0, rect.top() + 9.0),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(7.5),
            palette.muted,
        );
    }
    for x in [rect.left() + 3.0, rect.left() + 164.0, rect.left() + 184.0] {
        let bar = Rect::from_min_size(
            Pos2::new(x, plot_rect.top()),
            Vec2::new(14.0, plot_rect.height()),
        );
        painter.rect_filled(bar, egui::CornerRadius::same(1), palette.surface_raised);
        painter.rect_stroke(
            bar,
            egui::CornerRadius::same(1),
            Stroke::new(1.0, palette.grid),
            egui::StrokeKind::Inside,
        );
    }
    painter.rect_filled(plot_rect, egui::CornerRadius::same(1), palette.canvas);
    painter.rect_stroke(
        plot_rect,
        egui::CornerRadius::same(1),
        Stroke::new(1.0, palette.grid),
        egui::StrokeKind::Inside,
    );
    let mut points = Vec::with_capacity(65);
    for step in 0..=64 {
        let input = -60.0 + 60.0 * step as f32 / 64.0;
        let output = if input <= threshold {
            input
        } else {
            threshold + (input - threshold) / ratio.max(1.0)
        } + makeup;
        let x = plot_rect.left() + plot_rect.width() * (input + 60.0) / 60.0;
        let y = plot_rect.bottom() - plot_rect.height() * (output + 60.0).clamp(0.0, 60.0) / 60.0;
        points.push(Pos2::new(x, y));
    }
    let mut fill_points = points.clone();
    fill_points.push(Pos2::new(plot_rect.right(), plot_rect.bottom()));
    fill_points.push(Pos2::new(plot_rect.left(), plot_rect.bottom()));
    painter.add(egui::Shape::convex_polygon(
        fill_points,
        Color32::from_rgba_unmultiplied(
            palette.accent.r(),
            palette.accent.g(),
            palette.accent.b(),
            42,
        ),
        Stroke::NONE,
    ));
    for step in 0..=4 {
        let t = step as f32 / 4.0;
        let x = plot_rect.left() + plot_rect.width() * t;
        let y = plot_rect.top() + plot_rect.height() * t;
        painter.line_segment(
            [
                Pos2::new(x, plot_rect.top()),
                Pos2::new(x, plot_rect.bottom()),
            ],
            Stroke::new(1.0, palette.surface_raised),
        );
        painter.line_segment(
            [
                Pos2::new(plot_rect.left(), y),
                Pos2::new(plot_rect.right(), y),
            ],
            Stroke::new(1.0, palette.surface_raised),
        );
    }
    painter.add(egui::Shape::line(points, Stroke::new(2.0, palette.accent)));
}

#[allow(dead_code)]
fn show_mixer_legacy(
    ui: &mut egui::Ui,
    project: &mut Project,
    meter_levels: &HashMap<u64, [f32; 2]>,
    show_strips: bool,
    show_settings: bool,
) -> bool {
    let mut modified = false;
    let mut exclusive_solo_track_id = None;
    let mut selected_track = project.selected_track;
    if show_strips && project.tracks.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Add tracks to create mixer channels")
                    .color(theme::palette(ui).muted),
            );
        });
        return false;
    }
    if show_strips {
        ui.horizontal(|ui| {
            ui.heading("Mixer");
            ui.separator();
            ui.label(
                egui::RichText::new("CHANNELS")
                    .small()
                    .color(theme::palette(ui).muted),
            );
        });
        ui.separator();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for (track_index, track) in project.tracks.iter_mut().enumerate() {
                    egui::Frame::group(ui.style())
                        .fill(theme::palette(ui).surface)
                        .corner_radius(egui::CornerRadius::same(6))
                        .show(ui, |ui| {
                            ui.set_min_width(86.0);
                            ui.horizontal(|ui| {
                                ui.colored_label(track_kind_color(track.kind), "CH");
                                if ui
                                    .selectable_label(
                                        selected_track == Some(track_index as u32),
                                        track.name.as_str(),
                                    )
                                    .clicked()
                                {
                                    selected_track = Some(track_index as u32);
                                    modified = true;
                                }
                            });
                            ui.add_space(8.0);
                            let meter_height = 156.0;
                            let (meter, meter_response) = ui
                                .allocate_exact_size(Vec2::new(12.0, meter_height), Sense::hover());
                            let peak = meter_levels.get(&track.id).copied().unwrap_or([0.0; 2]).into_iter().fold(0.0_f32, f32::max);
                            let meter_fraction = meter_fill_fraction(peak);
                            ui.painter().rect_filled(
                                meter,
                                egui::CornerRadius::same(3),
                                theme::palette(ui).canvas,
                            );
                            if meter_fraction > 0.0 {
                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        Pos2::new(
                                            meter.min.x,
                                            meter.max.y - meter_height * meter_fraction,
                                        ),
                                        Vec2::new(meter.width(), meter_height * meter_fraction),
                                    ),
                                    egui::CornerRadius::same(3),
                                    theme::palette(ui).accent,
                                );
                            }
                            if peak >= 1.0 {
                                ui.painter().rect_filled(
                                    Rect::from_min_size(meter.min, Vec2::new(meter.width(), 4.0)),
                                    egui::CornerRadius::same(1),
                                    theme::RECORD,
                                );
                            }
                            let meter_text = if peak > 0.0 {
                                format!("{:.1} dBFS", 20.0 * peak.log10())
                            } else {
                                "-∞ dBFS".to_owned()
                            };
                            meter_response.on_hover_text(meter_text);
                            if ui
                                .add(
                                    egui::Slider::new(&mut track.gain, 0.0..=1.0)
                                        .vertical()
                                        .show_value(false),
                                )
                                .changed()
                            {
                                modified = true;
                            }
                            ui.horizontal(|ui| {
                                let mute = egui::Button::new("M").fill(if track.muted {
                                    theme::WARNING
                                } else {
                                    theme::palette(ui).surface_raised
                                });
                                if ui.add(mute).clicked() {
                                    track.muted = !track.muted;
                                    modified = true;
                                }
                                let solo_fill = if track.solo {
                                    theme::palette(ui).accent
                                } else if track.solo_defeat {
                                    theme::palette(ui).solo_defeat
                                } else {
                                    theme::palette(ui).surface_raised
                                };
                                let solo_tooltip = if track.solo_defeat {
                                    "Solo Defeat; Alt-click to disable"
                                } else {
                                    "Solo; Ctrl-click to solo exclusively; Alt-click for Solo Defeat"
                                };
                                let solo = egui::Button::new("S").fill(solo_fill);
                                if ui.add(solo).on_hover_text(solo_tooltip).clicked() {
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
                                let arm = egui::Button::new("R").fill(if track.armed {
                                    theme::RECORD
                                } else {
                                    theme::palette(ui).surface_raised
                                });
                                if ui.add(arm).clicked() {
                                    track.armed = !track.armed;
                                    modified = true;
                                }
                            });
                        });
                }
                egui::Frame::group(ui.style())
                    .fill(theme::palette(ui).master_surface)
                    .corner_radius(egui::CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.set_min_width(92.0);
                        ui.label(egui::RichText::new("MASTER").strong().color(theme::palette(ui).accent));
                        ui.add_space(180.0);
                        ui.label(egui::RichText::new("0.0 dB").monospace());
                    });
            });
        });
    }
    if let Some(solo_track_id) = exclusive_solo_track_id {
        for track in &mut project.tracks {
            if track.id != solo_track_id && track.solo {
                track.solo = false;
            }
        }
    }
    if show_settings {
        egui::ScrollArea::vertical()
            .id_salt("mixer-channel-settings")
            .show(ui, |ui| {
                if project.tracks.is_empty() {
                    ui.label("Add a track to open its channel settings.");
                    ui.horizontal(|ui| {
                        for (kind, label) in [
                            (TrackKind::Audio, "+ Audio Track"),
                            (TrackKind::Instrument, "+ Instrument Track"),
                            (TrackKind::Midi, "+ MIDI Track"),
                        ] {
                            if ui.button(label).clicked() {
                                project.add_track(kind);
                                selected_track = project.selected_track;
                                modified = true;
                            }
                        }
                    });
                } else {
                    let selected_name = selected_track
                        .and_then(|index| project.tracks.get(index as usize))
                        .map(|track| track.name.clone())
                        .unwrap_or_else(|| "Select a track".to_owned());
                    ui.horizontal(|ui| {
                        ui.label("Track");
                        egui::ComboBox::from_id_salt("channel-settings-track")
                            .selected_text(selected_name)
                            .show_ui(ui, |ui| {
                                for (index, track) in project.tracks.iter().enumerate() {
                                    modified |= ui
                                        .selectable_value(
                                            &mut selected_track,
                                            Some(index as u32),
                                            &track.name,
                                        )
                                        .changed();
                                }
                            });
                    });
                    if selected_track.is_none() {
                        ui.label("Select a track to edit its instrument, EQ, and inserts.");
                    }
                }
                if project.selected_track != selected_track {
                    project.selected_track = selected_track;
                    project.selected_midi_part_id = None;
                }
                if let Some(selected_index) = selected_track
                    .map(|index| index as usize)
                    .filter(|index| *index < project.tracks.len())
                {
                    let track = &mut project.tracks[selected_index];
                    ui.separator();
                    if track.kind == TrackKind::Instrument {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("BUILT-IN INSTRUMENT")
                                    .small()
                                    .color(theme::palette(ui).muted),
                            );
                            egui::ComboBox::from_id_salt(("instrument-waveform", track.id))
                                .selected_text(instrument_waveform_name(track.instrument_waveform))
                                .show_ui(ui, |ui| {
                                    for waveform in [
                                        InstrumentWaveform::Sine,
                                        InstrumentWaveform::Square,
                                        InstrumentWaveform::Sawtooth,
                                        InstrumentWaveform::Triangle,
                                    ] {
                                        modified |= ui
                                            .selectable_value(
                                                &mut track.instrument_waveform,
                                                waveform,
                                                instrument_waveform_name(waveform),
                                            )
                                            .changed();
                                    }
                                });
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("BUILT-IN EQ")
                                .small()
                                .color(theme::palette(ui).muted),
                        );
                        modified |= ui
                            .checkbox(&mut track.equalizer.bypassed, "Bypass")
                            .changed();
                    });
                    for (band_index, band) in track.equalizer.bands.iter_mut().enumerate() {
                        egui::CollapsingHeader::new(format!("EQ Band {}", band_index + 1))
                            .default_open(band.enabled)
                            .show(ui, |ui| {
                                modified |= ui.checkbox(&mut band.enabled, "Enabled").changed();
                                egui::ComboBox::from_id_salt(("eq-filter", track.id, band_index))
                                    .selected_text(filter_type_name(band.filter))
                                    .show_ui(ui, |ui| {
                                        let available_filters: &[FilterType] =
                                            if matches!(band_index, 1 | 2) {
                                                &[FilterType::Bell]
                                            } else {
                                                &[
                                                    FilterType::Bell,
                                                    FilterType::LowShelf,
                                                    FilterType::HighShelf,
                                                    FilterType::LowPass,
                                                    FilterType::HighPass,
                                                ]
                                            };
                                        for filter in available_filters.iter().copied() {
                                            modified |= ui
                                                .selectable_value(
                                                    &mut band.filter,
                                                    filter,
                                                    filter_type_name(filter),
                                                )
                                                .changed();
                                        }
                                    });
                                modified |= ui
                                    .add(
                                        egui::Slider::new(&mut band.frequency_hz, 20.0..=20_000.0)
                                            .logarithmic(true)
                                            .text("Frequency (Hz)"),
                                    )
                                    .changed();
                                modified |= ui
                                    .add(
                                        egui::Slider::new(&mut band.gain_db, -24.0..=24.0)
                                            .text("Gain (dB)"),
                                    )
                                    .changed();
                                modified |= ui
                                    .add(egui::Slider::new(&mut band.q, 0.1..=10.0).text("Q"))
                                    .changed();
                            });
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("INSERT EFFECTS")
                                .small()
                                .color(theme::palette(ui).muted),
                        );
                        if ui.button("+ Test Generator").clicked() {
                            track.inserts.push(InsertEffect {
                                enabled: true,
                                kind: InsertEffectKind::TestGenerator {
                                    waveform: GeneratorWaveform::Sine,
                                    frequency_hz: 440.0,
                                    gain_db: -18.0,
                                },
                            });
                            modified = true;
                        }
                        if ui.button("+ Stereo Delay").clicked() {
                            track.inserts.push(InsertEffect {
                                enabled: true,
                                kind: InsertEffectKind::default(),
                            });
                            modified = true;
                        }
                        if ui.button("+ Compressor").clicked() {
                            track.inserts.push(InsertEffect {
                                enabled: true,
                                kind: InsertEffectKind::Compressor {
                                    threshold_db: -18.0,
                                    ratio: 3.0,
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
                                },
                            });
                            modified = true;
                        }
                    });
                    let mut remove_insert = None;
                    let mut move_insert = None;
                    let insert_count = track.inserts.len();
                    for (insert_index, insert) in track.inserts.iter_mut().enumerate() {
                        egui::Frame::group(ui.style())
                            .fill(theme::palette(ui).surface)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let effect_name = match &insert.kind {
                                        InsertEffectKind::TestGenerator { .. } => "Test Generator",
                                        InsertEffectKind::StereoDelay { .. } => "Stereo Delay",
                                        InsertEffectKind::Compressor { .. } => "Compressor",
                                    };
                                    modified |=
                                        ui.checkbox(&mut insert.enabled, effect_name).changed();
                                    if ui
                                        .add_enabled(
                                            insert_index > 0,
                                            egui::Button::new("↑").small(),
                                        )
                                        .on_hover_text("Move insert up")
                                        .clicked()
                                    {
                                        move_insert = Some((insert_index, insert_index - 1));
                                    }
                                    if ui
                                        .add_enabled(
                                            insert_index + 1 < insert_count,
                                            egui::Button::new("↓").small(),
                                        )
                                        .on_hover_text("Move insert down")
                                        .clicked()
                                    {
                                        move_insert = Some((insert_index, insert_index + 1));
                                    }
                                    if ui.small_button("Remove").clicked() {
                                        remove_insert = Some(insert_index);
                                    }
                                });
                                match &mut insert.kind {
                                    InsertEffectKind::TestGenerator {
                                        waveform,
                                        frequency_hz,
                                        gain_db,
                                    } => {
                                        egui::ComboBox::from_id_salt((
                                            "generator-waveform",
                                            track.id,
                                            insert_index,
                                        ))
                                        .selected_text(generator_waveform_name(*waveform))
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                for option in [
                                                    GeneratorWaveform::Sine,
                                                    GeneratorWaveform::Square,
                                                    GeneratorWaveform::Sawtooth,
                                                    GeneratorWaveform::Triangle,
                                                    GeneratorWaveform::WhiteNoise,
                                                    GeneratorWaveform::BrownNoise,
                                                    GeneratorWaveform::PinkNoise,
                                                ] {
                                                    modified |= ui
                                                        .selectable_value(
                                                            waveform,
                                                            option,
                                                            generator_waveform_name(option),
                                                        )
                                                        .changed();
                                                }
                                            },
                                        );
                                        modified |= ui
                                            .add(
                                                egui::Slider::new(frequency_hz, 1.0..=20_000.0)
                                                    .logarithmic(true)
                                                    .text("Frequency (Hz)"),
                                            )
                                            .changed();
                                        modified |= ui
                                            .add(
                                                egui::Slider::new(gain_db, -60.0..=0.0)
                                                    .text("Gain (dB)"),
                                            )
                                            .changed();
                                    }
                                    InsertEffectKind::StereoDelay {
                                        time_ms,
                                        feedback,
                                        mix,
                                    } => {
                                        modified |= ui
                                            .add(
                                                egui::Slider::new(time_ms, 1.0..=2_000.0)
                                                    .text("Time (ms)"),
                                            )
                                            .changed();
                                        modified |= ui
                                            .add(
                                                egui::Slider::new(feedback, 0.0..=0.95)
                                                    .text("Feedback"),
                                            )
                                            .changed();
                                        modified |= ui
                                            .add(egui::Slider::new(mix, 0.0..=1.0).text("Wet"))
                                            .changed();
                                    }
                                    InsertEffectKind::Compressor {
                                        threshold_db,
                                        ratio,
                                        attack_ms,
                                        release_ms,
                                        makeup_db,
                                        ..
                                    } => {
                                        modified |= ui
                                            .add(
                                                egui::Slider::new(threshold_db, -60.0..=0.0)
                                                    .text("Threshold (dB)"),
                                            )
                                            .changed();
                                        modified |= ui
                                            .add(egui::Slider::new(ratio, 1.0..=20.0).text("Ratio"))
                                            .changed();
                                        modified |= ui
                                            .add(
                                                egui::Slider::new(attack_ms, 0.1..=200.0)
                                                    .logarithmic(true)
                                                    .text("Attack (ms)"),
                                            )
                                            .changed();
                                        modified |= ui
                                            .add(
                                                egui::Slider::new(release_ms, 5.0..=2_000.0)
                                                    .logarithmic(true)
                                                    .text("Release (ms)"),
                                            )
                                            .changed();
                                        modified |= ui
                                            .add(
                                                egui::Slider::new(makeup_db, 0.0..=24.0)
                                                    .text("Makeup (dB)"),
                                            )
                                            .changed();
                                    }
                                }
                            });
                    }
                    if let Some(insert_index) = remove_insert {
                        track.inserts.remove(insert_index);
                        modified = true;
                    } else if let Some((from, to)) = move_insert {
                        track.inserts.swap(from, to);
                        modified = true;
                    }
                }
            });
    }
    modified
}

fn meter_fill_fraction(peak: f32) -> f32 {
    if !peak.is_finite() || peak <= 0.0 {
        return 0.0;
    }
    ((20.0 * peak.log10() + 60.0) / 60.0).clamp(0.0, 1.0)
}

fn paint_track_meter(ui: &egui::Ui, rect: Rect, peaks: [f32; 2]) {
    let palette = theme::palette(ui);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 0.0, palette.canvas);
    let width = (rect.width() - 1.0) * 0.5;
    for (index, peak) in peaks.into_iter().enumerate() {
        let channel = Rect::from_min_size(
            rect.min + Vec2::new(index as f32 * (width + 1.0), 0.0),
            Vec2::new(width, rect.height()),
        );
        let fill = meter_fill_fraction(peak);
        if fill > 0.0 {
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(channel.left(), channel.bottom() - channel.height() * fill),
                    channel.max,
                ),
                0.0,
                palette.accent,
            );
        }
        if peak >= 1.0 {
            painter.rect_filled(
                Rect::from_min_size(channel.min, Vec2::new(channel.width(), 3.0)),
                0.0,
                theme::RECORD,
            );
        }
    }
}

#[cfg(test)]
mod meter_tests {
    use super::meter_fill_fraction;

    #[test]
    fn mixer_meter_maps_a_sixty_dbfs_range() {
        assert_eq!(meter_fill_fraction(0.0), 0.0);
        assert_eq!(meter_fill_fraction(0.001), 0.0);
        assert!((meter_fill_fraction(0.01) - 1.0 / 3.0).abs() < 1.0e-6);
        assert_eq!(meter_fill_fraction(1.0), 1.0);
        assert_eq!(meter_fill_fraction(4.0), 1.0);
    }
}

fn show_media(ui: &mut egui::Ui, selected_media: &mut usize) -> bool {
    let mut modified = false;
    ui.horizontal(|ui| {
        ui.heading("MediaBay");
        ui.separator();
        ui.label(
            egui::RichText::new("LIBRARY")
                .small()
                .color(theme::palette(ui).muted),
        );
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            for (index, item) in ["Files", "Loops", "Instruments", "Presets"]
                .into_iter()
                .enumerate()
            {
                if ui
                    .selectable_label(*selected_media == index, item)
                    .clicked()
                {
                    if *selected_media != index {
                        *selected_media = index;
                        modified = true;
                    }
                }
            }
        });
        ui.separator();
        ui.centered_and_justified(|ui| {
            let location = ["Files", "Loops", "Instruments", "Presets"]
                .get(*selected_media)
                .copied()
                .unwrap_or("Files");
            ui.label(
                egui::RichText::new(format!("{location}: choose a media location to browse"))
                    .color(theme::palette(ui).muted),
            );
        });
    });
    modified
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

fn format_transport_position(beats: f64, beats_per_bar: u8) -> String {
    let beats = beats.max(0.0);
    let whole = beats.floor() as u64;
    let beats_per_bar = u64::from(beats_per_bar.max(1));
    let bar = whole / beats_per_bar + 1;
    let beat = whole % beats_per_bar + 1;
    let within_beat = beats.fract() * 4.0;
    let sixteenth = within_beat.floor() as u64 + 1;
    let tick = ((within_beat.fract()) * 120.0).round() as u16;
    format!("{bar}.{beat}.{sixteenth}.{tick}")
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
                            crate::icons::power(ui.painter(), r, expected);
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
                        &mut self.project,
                        &mut self.position,
                        true,
                        4,
                        44_100,
                        &HashMap::new(),
                        &HashMap::new(),
                        None,
                        &mut self.view,
                        &mut false,
                        &mut false,
                        &mut NativeWindowState::default(),
                        &mut None,
                        true,
                        &mut None,
                    );
                },
            );
            self.cursor = output.platform_output.cursor_icon;
            self.ruler_labels.clear();
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
            for shape in &output.shapes {
                collect_labels(&shape.shape, &mut self.ruler_labels);
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
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(590.0, 140.0))]);
        f.frame(vec![]);
        f.button(590.0, 140.0, false);
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
            f.button(385.0, 16.0, true);
            f.button(385.0, 16.0, false);
            assert_eq!(f.position, expected);
            f.button(320.0, 16.0, true);
            f.frame(vec![egui::Event::PointerMoved(Pos2::new(385.0, 16.0))]);
            f.button(385.0, 16.0, false);
            assert_eq!(f.position, expected);
        }
        let mut f = Fixture::new();
        f.view.timeline_zoom = 4.0;
        f.view.grid_type = GridType::UseQuantize;
        f.view.quantize.note = QuantizeNote::Eighth;
        f.project.set_loop_range(100.0, 110.0);
        f.frame(vec![]);
        f.button(320.0, 3.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(385.0, 3.0))]);
        f.button(385.0, 3.0, false);
        assert_eq!(
            (f.project.loop_start_beats, f.project.loop_end_beats),
            (6.0, 9.5)
        );
        f.button(340.0, 3.0, true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(385.0, 3.0))]);
        f.button(385.0, 3.0, false);
        assert_eq!(
            (f.project.loop_start_beats, f.project.loop_end_beats),
            (8.0, 11.5)
        );
        let right_x = 198.0 + 11.5 / 4.0 * ((1200.0 - 198.0 - 14.0) / 49.0 * 4.0);
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
                (1200.0 - 198.0 - 14.0) / 49.0 * f.view.timeline_zoom,
                4,
            );
            assert!(
                (anchor - (320.0 - 198.0) / ((1200.0 - 198.0 - 14.0) / 49.0) * 4.0).abs() < 0.001
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
        let expected = (320.0 - 198.0) / ((1200.0 - 198.0 - 14.0) / 49.0) * 4.0;
        assert!((f.position - expected).abs() < 0.001);
    }

    #[test]
    fn corner_handles_resize_only_their_boundary_for_loop_and_skip() {
        let bar_width = (1200.0 - 198.0 - 14.0) / 49.0;
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
