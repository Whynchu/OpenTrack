//! Versioned native project serialization. This format is separate from `.cpr`.

#[cfg(unix)]
use std::fs::File;
use std::{
    error::Error,
    ffi::OsString,
    fmt, fs,
    fs::OpenOptions,
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use egui_dock::DockState;
use serde::{Deserialize, Serialize};
use workstation_core::project::Project;
use workstation_core::workspaces::{
    ProjectWorkspaceState, WindowBounds, WindowClass, WindowId, WindowSnapshot, Workspace,
};

use crate::app::PanelTab;

const FILE_FORMAT: &str = "rust-audio-workstation";
const FORMAT_VERSION: u32 = 14;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAIN_WINDOW_ID: &str = "main";
const MIDI_EDITOR_WINDOW_ID: &str = "midi-editor";
const ARRANGE_VIEW_STATE_KEY: &str = "arrange_view_state_v1";
pub(crate) const CHANNEL_SETTINGS_WINDOW_ID: &str = "channel-settings";
pub(crate) const MIXER_WINDOW_ID: &str = "mixer";
pub(crate) const MEDIA_WINDOW_ID: &str = "media";
const DOCK_STATE_KEY: &str = "egui_dock_state_v1";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
pub(crate) struct ProjectFile {
    format: String,
    version: u32,
    pub(crate) project: Project,
    pub(crate) workspace_state: ProjectWorkspaceState,
    pub(crate) live_dock_state: DockState<PanelTab>,
    pub(crate) show_transport: bool,
    pub(crate) selected_media: usize,
    pub(crate) cycle: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ArrangeViewState {
    #[serde(default)]
    pub(crate) track_heights: std::collections::BTreeMap<u64, f32>,
    #[serde(default)]
    pub(crate) exclusive_track_ids: Vec<u64>,
    #[serde(default)]
    pub(crate) pre_exclusive_track_heights: Option<std::collections::BTreeMap<u64, f32>>,
}

impl ArrangeViewState {
    pub(crate) fn track_height(&self, track_id: u64) -> f32 {
        self.track_heights
            .get(&track_id)
            .copied()
            .filter(|height| height.is_finite())
            .unwrap_or(62.0)
            .clamp(36.0, 240.0)
    }

    pub(crate) fn set_track_height(&mut self, track_id: u64, height: f32) {
        if height.is_finite() {
            self.track_heights
                .insert(track_id, height.clamp(36.0, 240.0));
        }
    }

    pub(crate) fn leave_exclusive_mode(&mut self) {
        if let Some(previous_heights) = self.pre_exclusive_track_heights.take() {
            self.track_heights = previous_heights;
        }
        self.exclusive_track_ids.clear();
    }

    fn normalize(&mut self) {
        self.track_heights.retain(|_, height| height.is_finite());
        for height in self.track_heights.values_mut() {
            *height = height.clamp(36.0, 240.0);
        }
        if let Some(previous_heights) = &mut self.pre_exclusive_track_heights {
            previous_heights.retain(|_, height| height.is_finite());
            for height in previous_heights.values_mut() {
                *height = height.clamp(36.0, 240.0);
            }
        } else {
            self.exclusive_track_ids.clear();
        }
    }
}

impl ProjectFile {
    pub(crate) fn new(
        project: Project,
        workspace_state: ProjectWorkspaceState,
        live_dock_state: DockState<PanelTab>,
        show_transport: bool,
        selected_media: usize,
        cycle: bool,
    ) -> Self {
        Self {
            format: FILE_FORMAT.to_owned(),
            version: FORMAT_VERSION,
            project,
            workspace_state,
            live_dock_state,
            show_transport,
            selected_media,
            cycle,
        }
    }

    pub(crate) fn read(path: &Path) -> Result<Self, ProjectFileError> {
        let metadata = fs::metadata(path)?;
        if metadata.len() > MAX_FILE_BYTES {
            return Err(ProjectFileError::TooLarge);
        }

        let bytes = fs::read(path)?;
        let header: FileHeader = serde_json::from_slice(&bytes)?;
        if header.format != FILE_FORMAT {
            return Err(ProjectFileError::WrongFormat);
        }
        let document = match header.version {
            1 => {
                let legacy: ProjectFileV1 = serde_json::from_slice(&bytes)?;
                Self {
                    format: FILE_FORMAT.to_owned(),
                    version: FORMAT_VERSION,
                    project: legacy.project,
                    workspace_state: workspace_state_from_dock("Arrangement", &legacy.dock_state)?,
                    live_dock_state: legacy.dock_state,
                    show_transport: legacy.show_transport,
                    selected_media: legacy.selected_media,
                    cycle: legacy.cycle,
                }
            }
            2 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            3 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            4 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            5 => migrate_version_five(&bytes)?,
            6 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            7 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            8 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            9 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            10 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            11 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            12 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            13 => {
                let mut legacy: Self = serde_json::from_slice(&bytes)?;
                legacy.version = FORMAT_VERSION;
                legacy
            }
            FORMAT_VERSION => serde_json::from_slice(&bytes)?,
            version => return Err(ProjectFileError::UnsupportedVersion(version)),
        };
        document.validate()?;
        Ok(document)
    }

    pub(crate) fn write(&self, path: &Path) -> Result<(), ProjectFileError> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        atomic_write(path, &bytes)?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ProjectFileError> {
        if self.format != FILE_FORMAT {
            return Err(ProjectFileError::WrongFormat);
        }
        if self.version != FORMAT_VERSION {
            return Err(ProjectFileError::UnsupportedVersion(self.version));
        }
        if self.selected_media > 3 {
            return Err(ProjectFileError::InvalidData(
                "selected media category is out of range".to_owned(),
            ));
        }
        if self.live_dock_state.find_tab(&PanelTab::Project).is_none() {
            return Err(ProjectFileError::InvalidData(
                "project pane is missing from the live dock state".to_owned(),
            ));
        }
        self.workspace_state
            .validate()
            .map_err(|message| ProjectFileError::InvalidData(message.to_owned()))?;
        if self.workspace_state.active_index.is_none() {
            return Err(ProjectFileError::InvalidData(
                "project has no active workspace".to_owned(),
            ));
        }
        for workspace in &self.workspace_state.workspaces {
            let dock_state = dock_state_from_workspace(workspace)?;
            if dock_state.find_tab(&PanelTab::Project).is_none() {
                return Err(ProjectFileError::InvalidData(format!(
                    "workspace '{}' is missing its project pane",
                    workspace.name
                )));
            }
        }
        self.project
            .validate()
            .map_err(|message| ProjectFileError::InvalidData(message.to_owned()))
    }
}

fn migrate_version_five(bytes: &[u8]) -> Result<ProjectFile, ProjectFileError> {
    let mut document: serde_json::Value = serde_json::from_slice(bytes)?;
    let Some(project) = document.get_mut("project") else {
        return Err(ProjectFileError::InvalidData(
            "version 5 project data is missing".to_owned(),
        ));
    };
    let Some(project_object) = project.as_object_mut() else {
        return Err(ProjectFileError::InvalidData(
            "version 5 project data is not an object".to_owned(),
        ));
    };
    let selected_track = project_object
        .get("selected_track")
        .and_then(serde_json::Value::as_u64);
    let Some(tracks) = project_object
        .get_mut("tracks")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Err(ProjectFileError::InvalidData(
            "version 5 project tracks are missing".to_owned(),
        ));
    };

    let mut next_part_id = 1_u64;
    let mut selected_part_id = None;
    for (track_index, track) in tracks.iter_mut().enumerate() {
        let Some(track_object) = track.as_object_mut() else {
            return Err(ProjectFileError::InvalidData(
                "version 5 track is not an object".to_owned(),
            ));
        };
        let notes = track_object
            .remove("midi_notes")
            .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
        let Some(note_values) = notes.as_array() else {
            return Err(ProjectFileError::InvalidData(
                "version 5 MIDI notes are not an array".to_owned(),
            ));
        };
        if track_object
            .get("midi_parts")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|parts| !parts.is_empty())
        {
            return Err(ProjectFileError::InvalidData(
                "version 5 track contains both MIDI notes and parts".to_owned(),
            ));
        }

        if note_values.is_empty() {
            track_object.insert("midi_parts".to_owned(), serde_json::json!([]));
            continue;
        }

        let duration_beats = note_values
            .iter()
            .filter_map(|note| {
                Some(note.get("start_beats")?.as_f64()? + note.get("duration_beats")?.as_f64()?)
            })
            .filter(|end| end.is_finite())
            .fold(1.0_f64, f64::max);
        let part_id = next_part_id;
        next_part_id = next_part_id.checked_add(1).ok_or_else(|| {
            ProjectFileError::InvalidData("too many MIDI parts in version 5 project".to_owned())
        })?;
        let part = serde_json::json!({
            "id": part_id,
            "name": format!("MIDI Part {part_id:02}"),
            "start_beats": 0.0,
            "duration_beats": duration_beats,
            "notes": notes,
        });
        track_object.insert("midi_parts".to_owned(), serde_json::json!([part]));
        if selected_track == u64::try_from(track_index).ok() {
            selected_part_id = Some(part_id);
        }
    }
    project_object.insert(
        "next_midi_part_id".to_owned(),
        serde_json::Value::from(next_part_id),
    );
    project_object.insert(
        "selected_midi_part_id".to_owned(),
        selected_part_id
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
    );
    document["version"] = serde_json::Value::from(FORMAT_VERSION);
    Ok(serde_json::from_value(document)?)
}

#[derive(Deserialize)]
struct FileHeader {
    format: String,
    version: u32,
}

#[derive(Deserialize)]
struct ProjectFileV1 {
    project: Project,
    dock_state: DockState<PanelTab>,
    show_transport: bool,
    selected_media: usize,
    cycle: bool,
}

pub(crate) fn initial_workspace_state(
    dock_state: &DockState<PanelTab>,
) -> Result<ProjectWorkspaceState, ProjectFileError> {
    workspace_state_from_dock("Arrangement", dock_state)
}

pub(crate) fn store_active_dock_state(
    workspace_state: &mut ProjectWorkspaceState,
    dock_state: &DockState<PanelTab>,
    bounds: Option<WindowBounds>,
    arrange_view_state: &ArrangeViewState,
) -> Result<(), ProjectFileError> {
    let window = main_window_snapshot(dock_state, bounds, arrange_view_state)?;
    if store_active_window_snapshot(workspace_state, window) {
        Ok(())
    } else {
        Err(ProjectFileError::InvalidData(
            "active workspace is missing or locked".to_owned(),
        ))
    }
}

pub(crate) fn midi_editor_window_snapshot(workspace: &Workspace) -> Option<&WindowSnapshot> {
    project_window_snapshot(workspace, MIDI_EDITOR_WINDOW_ID)
}

pub(crate) fn project_window_snapshot<'a>(
    workspace: &'a Workspace,
    id: &str,
) -> Option<&'a WindowSnapshot> {
    workspace
        .windows
        .iter()
        .find(|window| window.id.as_str() == id)
}

pub(crate) fn main_window_window_snapshot(workspace: &Workspace) -> Option<&WindowSnapshot> {
    project_window_snapshot(workspace, MAIN_WINDOW_ID)
}

pub(crate) fn arrange_view_state_from_workspace(workspace: &Workspace) -> ArrangeViewState {
    let mut state = main_window_window_snapshot(workspace)
        .and_then(|window| window.view_state.get(ARRANGE_VIEW_STATE_KEY))
        .and_then(|serialized| serde_json::from_str::<ArrangeViewState>(serialized).ok())
        .unwrap_or_default();
    state.normalize();
    state
}

pub(crate) fn store_active_project_window_state(
    workspace_state: &mut ProjectWorkspaceState,
    id: &str,
    open: bool,
    bounds: Option<WindowBounds>,
) -> Result<(), ProjectFileError> {
    let window = WindowSnapshot {
        id: WindowId::new(id),
        class: WindowClass::ProjectSpecific,
        open,
        bounds,
        view_state: Default::default(),
    };
    if store_active_window_snapshot(workspace_state, window) {
        Ok(())
    } else {
        Err(ProjectFileError::InvalidData(
            "active workspace is missing or locked".to_owned(),
        ))
    }
}

fn store_active_window_snapshot(
    workspace_state: &mut ProjectWorkspaceState,
    window: WindowSnapshot,
) -> bool {
    let Some(index) = workspace_state.active_index else {
        return false;
    };
    let Some(workspace) = workspace_state.workspaces.get_mut(index) else {
        return false;
    };
    if workspace.locked {
        return false;
    }
    if let Some(existing) = workspace
        .windows
        .iter_mut()
        .find(|existing| existing.id == window.id)
    {
        *existing = window;
    } else {
        workspace.windows.push(window);
    }
    true
}

pub(crate) fn dock_state_from_workspace(
    workspace: &Workspace,
) -> Result<DockState<PanelTab>, ProjectFileError> {
    let window = workspace
        .windows
        .iter()
        .find(|window| window.id.as_str() == MAIN_WINDOW_ID && window.open)
        .ok_or_else(|| {
            ProjectFileError::InvalidData(format!(
                "workspace '{}' has no open main window",
                workspace.name
            ))
        })?;
    let serialized_state = window.view_state.get(DOCK_STATE_KEY).ok_or_else(|| {
        ProjectFileError::InvalidData(format!("workspace '{}' has no dock layout", workspace.name))
    })?;
    Ok(serde_json::from_str(serialized_state)?)
}

fn workspace_state_from_dock(
    name: &str,
    dock_state: &DockState<PanelTab>,
) -> Result<ProjectWorkspaceState, ProjectFileError> {
    let window = main_window_snapshot(dock_state, None, &ArrangeViewState::default())?;
    let mut workspace = Workspace::new(name);
    workspace.windows.push(window);
    Ok(ProjectWorkspaceState {
        workspaces: vec![workspace],
        project_presets: Vec::new(),
        active_index: Some(0),
        use_explicit_ids: false,
    })
}

fn main_window_snapshot(
    dock_state: &DockState<PanelTab>,
    bounds: Option<WindowBounds>,
    arrange_view_state: &ArrangeViewState,
) -> Result<WindowSnapshot, ProjectFileError> {
    let mut view_state = std::collections::BTreeMap::new();
    view_state.insert(
        DOCK_STATE_KEY.to_owned(),
        serde_json::to_string(dock_state)?,
    );
    view_state.insert(
        ARRANGE_VIEW_STATE_KEY.to_owned(),
        serde_json::to_string(arrange_view_state)?,
    );
    Ok(WindowSnapshot {
        id: WindowId::new(MAIN_WINDOW_ID),
        class: WindowClass::Main,
        open: true,
        bounds,
        view_state,
    })
}

#[derive(Debug)]
pub(crate) enum ProjectFileError {
    Io(std::io::Error),
    Json(serde_json::Error),
    TooLarge,
    WrongFormat,
    UnsupportedVersion(u32),
    InvalidData(String),
}

impl From<std::io::Error> for ProjectFileError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ProjectFileError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl fmt::Display for ProjectFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "file access failed: {error}"),
            Self::Json(error) => write!(formatter, "project data could not be decoded: {error}"),
            Self::TooLarge => write!(formatter, "project file exceeds the 64 MiB safety limit"),
            Self::WrongFormat => write!(formatter, "file is not an OpenTrack- project"),
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "project format version {version} is not supported"
                )
            }
            Self::InvalidData(message) => write!(formatter, "invalid project data: {message}"),
        }
    }
}

impl Error for ProjectFileError {}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "project path has no filename")
    })?;

    let (temporary_path, mut temporary_file) = loop {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".tmp-{}-{sequence}", std::process::id()));
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => break (temporary_path, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };

    if let Err(error) = temporary_file
        .write_all(bytes)
        .and_then(|_| temporary_file.sync_all())
    {
        drop(temporary_file);
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    drop(temporary_file);

    #[cfg(windows)]
    let replace_result = replace_file_windows(&temporary_path, path);
    #[cfg(not(windows))]
    let replace_result = fs::rename(&temporary_path, path);

    if let Err(error) = replace_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    #[cfg(unix)]
    File::open(parent)?.sync_all()?;

    Ok(())
}

#[cfg(windows)]
fn replace_file_windows(source: &Path, destination: &Path) -> io::Result<()> {
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;

    use egui_dock::DockState;
    use workstation_core::project::{FilterType, Project, Track, TrackKind};

    use crate::app::PanelTab;

    use super::{
        DOCK_STATE_KEY, FORMAT_VERSION, ProjectFile, TEMP_FILE_SEQUENCE, initial_workspace_state,
    };

    #[test]
    fn earlier_tracks_without_channel_state_receive_defaults() {
        let serialized = r#"{
            "id": 1,
            "name": "Audio 01",
            "kind": "audio",
            "muted": false,
            "solo": false,
            "armed": false,
            "gain": 0.78
        }"#;
        let track: Track = serde_json::from_str(serialized).expect("legacy track should load");
        assert_eq!(track.equalizer.bands.len(), 4);
        assert_eq!(track.equalizer.bands[0].filter, FilterType::LowShelf);
        assert!(track.inserts.is_empty());
    }

    #[test]
    fn version_three_project_migrates_new_channel_state_defaults() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let dock_state = DockState::new(vec![PanelTab::Project]);
        let workspace_state = initial_workspace_state(&dock_state).expect("workspace should build");
        let document = ProjectFile::new(project, workspace_state, dock_state, true, 0, false);
        let mut serialized = serde_json::to_value(document).expect("project should serialize");
        serialized["version"] = serde_json::Value::from(3);
        let track = serialized["project"]["tracks"][0]
            .as_object_mut()
            .expect("track should be an object");
        track.remove("equalizer");
        track.remove("inserts");
        track.remove("midi_parts");
        serialized["project"]
            .as_object_mut()
            .expect("project should be an object")
            .remove("next_midi_part_id");
        serialized["project"]
            .as_object_mut()
            .expect("project should be an object")
            .remove("selected_midi_part_id");
        serialized["project"]
            .as_object_mut()
            .expect("project should be an object")
            .remove("next_midi_note_id");

        let mut live_dock_state = serialized["live_dock_state"].clone();
        set_test_dock_geometry(&mut live_dock_state);
        serialized["live_dock_state"] = live_dock_state;
        let dock_state_text =
            serialized["workspace_state"]["workspaces"][0]["windows"][0]["view_state"]
                [DOCK_STATE_KEY]
                .as_str()
                .expect("workspace dock state should be serialized");
        let mut workspace_dock_state: serde_json::Value =
            serde_json::from_str(dock_state_text).expect("workspace dock state should decode");
        set_test_dock_geometry(&mut workspace_dock_state);
        serialized["workspace_state"]["workspaces"][0]["windows"][0]["view_state"]
            [DOCK_STATE_KEY] = serde_json::Value::String(
            serde_json::to_string(&workspace_dock_state).expect("dock state should encode"),
        );

        let path = std::env::temp_dir().join(format!(
            "opentrack-v3-project-{}-{}.rwp",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(
            &path,
            serde_json::to_vec(&serialized).expect("json should encode"),
        )
        .expect("legacy project should be written");
        let read_result = ProjectFile::read(&path);
        let _ = fs::remove_file(path);
        let migrated = read_result.expect("version 3 project should migrate");

        assert_eq!(migrated.version, FORMAT_VERSION);
        assert_eq!(migrated.project.tracks[0].equalizer.bands.len(), 4);
        assert!(migrated.project.tracks[0].inserts.is_empty());
        assert!(migrated.project.tracks[0].midi_parts.is_empty());
        assert_eq!(migrated.project.next_midi_part_id, 1);
        assert_eq!(migrated.project.selected_midi_part_id, None);
        assert_eq!(migrated.project.next_midi_note_id, 1);
    }

    #[test]
    fn version_four_project_migrates_default_midi_note_state() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let dock_state = finite_test_dock_state();
        let workspace_state = initial_workspace_state(&dock_state).expect("workspace should build");
        let document = ProjectFile::new(project, workspace_state, dock_state, true, 0, false);
        let mut serialized = serde_json::to_value(document).expect("project should serialize");
        serialized["version"] = serde_json::Value::from(4);
        serialized["project"]
            .as_object_mut()
            .expect("project should be an object")
            .remove("next_midi_note_id");
        serialized["project"]
            .as_object_mut()
            .expect("project should be an object")
            .remove("next_midi_part_id");
        serialized["project"]
            .as_object_mut()
            .expect("project should be an object")
            .remove("selected_midi_part_id");
        serialized["project"]["tracks"][0]
            .as_object_mut()
            .expect("track should be an object")
            .remove("midi_parts");

        let path = std::env::temp_dir().join(format!(
            "opentrack-v4-project-{}-{}.rwp",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(
            &path,
            serde_json::to_vec(&serialized).expect("json should encode"),
        )
        .expect("legacy project should be written");
        let read_result = ProjectFile::read(&path);
        let _ = fs::remove_file(path);
        let migrated = read_result.expect("version 4 project should migrate");
        assert_eq!(migrated.version, FORMAT_VERSION);
        assert_eq!(migrated.project.next_midi_note_id, 1);
        assert_eq!(migrated.project.next_midi_part_id, 1);
        assert_eq!(migrated.project.selected_midi_part_id, None);
        assert!(migrated.project.tracks[0].midi_parts.is_empty());
    }

    #[test]
    fn version_five_migrates_direct_notes_into_a_selected_midi_part() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        project.next_midi_note_id = 8;
        let dock_state = finite_test_dock_state();
        let workspace_state = initial_workspace_state(&dock_state).expect("workspace should build");
        let document = ProjectFile::new(project, workspace_state, dock_state, true, 0, false);
        let mut serialized = serde_json::to_value(document).expect("project should serialize");
        serialized["version"] = serde_json::Value::from(5);
        let project_object = serialized["project"]
            .as_object_mut()
            .expect("project should be an object");
        project_object.remove("next_midi_part_id");
        project_object.remove("selected_midi_part_id");
        let track = project_object["tracks"][0]
            .as_object_mut()
            .expect("track should be an object");
        track.remove("midi_parts");
        track.insert(
            "midi_notes".to_owned(),
            serde_json::json!([{
                "id": 7,
                "start_beats": 2.0,
                "duration_beats": 0.5,
                "pitch": 72,
                "velocity": 108
            }]),
        );

        let path = std::env::temp_dir().join(format!(
            "opentrack-v5-project-{}-{}.rwp",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(
            &path,
            serde_json::to_vec(&serialized).expect("json should encode"),
        )
        .expect("legacy project should be written");
        let read_result = ProjectFile::read(&path);
        let _ = fs::remove_file(path);
        let migrated = read_result.expect("version 5 project should migrate");

        assert_eq!(migrated.version, FORMAT_VERSION);
        assert_eq!(migrated.project.next_midi_part_id, 2);
        assert_eq!(migrated.project.selected_midi_part_id, Some(1));
        let part = &migrated.project.tracks[0].midi_parts[0];
        assert_eq!(part.id, 1);
        assert_eq!(part.start_beats, 0.0);
        assert_eq!(part.duration_beats, 2.5);
        assert_eq!(part.notes.len(), 1);
        assert_eq!(part.notes[0].id, 7);
        assert_eq!(part.notes[0].start_beats, 2.0);
        assert_eq!(part.notes[0].duration_beats, 0.5);
        assert_eq!(part.notes[0].pitch, 72);
        assert_eq!(part.notes[0].velocity, 108);
    }

    #[test]
    fn version_thirteen_round_trips_eq_inserts_midi_parts_cycle_locators_and_instrument() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(0, PathBuf::from("Take 01.wav"), 0.0, 1.5)
            .expect("audio track should accept a WAV clip");
        let track = &mut project.tracks[0];
        track.equalizer.bands[0].enabled = true;
        track.equalizer.bands[0].frequency_hz = 70.0;
        track.equalizer.bands[0].gain_db = 3.0;
        track.inserts.push(workstation_core::project::InsertEffect {
            enabled: true,
            kind: workstation_core::project::InsertEffectKind::StereoDelay {
                time_ms: 250.0,
                feedback: 0.4,
                mix: 0.2,
            },
        });
        track.inserts.push(workstation_core::project::InsertEffect {
            enabled: true,
            kind: workstation_core::project::InsertEffectKind::TestGenerator {
                waveform: workstation_core::project::GeneratorWaveform::PinkNoise,
                frequency_hz: 1_000.0,
                gain_db: -12.0,
            },
        });
        track.inserts.push(workstation_core::project::InsertEffect {
            enabled: true,
            kind: workstation_core::project::InsertEffectKind::Compressor {
                threshold_db: -18.0,
                ratio: 3.0,
                attack_ms: 10.0,
                release_ms: 120.0,
                makeup_db: 2.0,
            },
        });
        project.add_track(TrackKind::Midi);
        project.add_track(TrackKind::Instrument);
        project.tracks[2].instrument_waveform =
            workstation_core::project::InstrumentWaveform::Triangle;
        assert!(project.set_loop_range(4.0, 20.0));
        let part_id = project
            .add_midi_part(1, 0.0, 16.0)
            .expect("MIDI part should be valid");
        project
            .add_midi_note_on_channel(1, part_id, 2.0, 0.5, 72, 9, 108)
            .expect("MIDI note should be valid");
        project
            .add_midi_controller_event(1, part_id, 3.0, 11, 96)
            .expect("MIDI controller event should be valid");
        let dock_state = finite_test_dock_state();
        let workspace_state = initial_workspace_state(&dock_state).expect("workspace should build");
        let document =
            ProjectFile::new(project.clone(), workspace_state, dock_state, true, 0, false);
        let path = std::env::temp_dir().join(format!(
            "opentrack-v14-project-{}-{}.rwp",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        document
            .write(&path)
            .expect("version 14 project should write");
        let read_result = ProjectFile::read(&path);
        let _ = fs::remove_file(path);
        let loaded = read_result.expect("version 14 project should reopen");
        assert_eq!(loaded.version, FORMAT_VERSION);
        assert_eq!(loaded.project, project);

        let mut legacy = serde_json::to_value(document).expect("project should serialize");
        legacy["version"] = serde_json::Value::from(7);
        legacy["project"]["tracks"][0]["inserts"]
            .as_array_mut()
            .expect("insert list should be an array")
            .retain(|insert| insert["type"] != "compressor");
        for track in legacy["project"]["tracks"]
            .as_array_mut()
            .expect("track list should be an array")
        {
            track
                .as_object_mut()
                .expect("track should be an object")
                .remove("instrument_waveform");
            track.as_object_mut().expect("track should be an object")["midi_parts"]
                .as_array_mut()
                .into_iter()
                .flatten()
                .for_each(|part| {
                    part.as_object_mut()
                        .expect("MIDI part should be an object")
                        .remove("controller_events");
                    part["notes"]
                        .as_array_mut()
                        .into_iter()
                        .flatten()
                        .for_each(|note| {
                            note.as_object_mut()
                                .expect("MIDI note should be an object")
                                .remove("channel");
                        });
                });
        }
        legacy["project"]
            .as_object_mut()
            .expect("project should be an object")
            .remove("next_midi_controller_id");
        let legacy_path = std::env::temp_dir().join(format!(
            "opentrack-v7-project-{}-{}.rwp",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(
            &legacy_path,
            serde_json::to_vec(&legacy).expect("legacy project should encode"),
        )
        .expect("legacy version 7 project should write");
        let legacy_read_result = ProjectFile::read(&legacy_path);
        let _ = fs::remove_file(legacy_path);
        let migrated = legacy_read_result.expect("version 7 project should migrate");
        assert_eq!(migrated.version, FORMAT_VERSION);
        assert_eq!(
            migrated.project.tracks[2].instrument_waveform,
            workstation_core::project::InstrumentWaveform::Sine
        );
    }

    #[test]
    fn version_nine_migrates_to_version_thirteen_and_preserves_inserts() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.tracks[0]
            .inserts
            .push(workstation_core::project::InsertEffect {
                enabled: true,
                kind: workstation_core::project::InsertEffectKind::StereoDelay {
                    time_ms: 240.0,
                    feedback: 0.3,
                    mix: 0.2,
                },
            });
        let dock_state = finite_test_dock_state();
        let workspace_state = initial_workspace_state(&dock_state).expect("workspace should build");
        let document =
            ProjectFile::new(project.clone(), workspace_state, dock_state, true, 0, false);
        let mut legacy = serde_json::to_value(document).expect("project should serialize");
        legacy["version"] = serde_json::Value::from(9);

        let path = std::env::temp_dir().join(format!(
            "opentrack-v9-project-{}-{}.rwp",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(
            &path,
            serde_json::to_vec(&legacy).expect("legacy project should encode"),
        )
        .expect("version 9 project should write");
        let read_result = ProjectFile::read(&path);
        let _ = fs::remove_file(path);
        let migrated = read_result.expect("version 9 project should migrate");
        assert_eq!(migrated.version, FORMAT_VERSION);
        assert_eq!(migrated.project, project);
    }

    #[test]
    fn version_ten_migrates_midi_notes_to_default_channel_zero() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let part_id = project.add_midi_part(0, 0.0, 8.0).unwrap();
        project
            .add_midi_note_on_channel(0, part_id, 1.0, 0.5, 64, 12, 100)
            .unwrap();
        let dock_state = finite_test_dock_state();
        let workspace_state = initial_workspace_state(&dock_state).expect("workspace should build");
        let document = ProjectFile::new(project, workspace_state, dock_state, true, 0, false);
        let mut legacy = serde_json::to_value(document).expect("project should serialize");
        legacy["version"] = serde_json::Value::from(10);
        legacy["project"]["tracks"][0]["midi_parts"][0]["notes"][0]
            .as_object_mut()
            .expect("MIDI note should be an object")
            .remove("channel");

        let path = std::env::temp_dir().join(format!(
            "opentrack-v10-project-{}-{}.rwp",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(
            &path,
            serde_json::to_vec(&legacy).expect("legacy project should encode"),
        )
        .expect("version 10 project should write");
        let read_result = ProjectFile::read(&path);
        let _ = fs::remove_file(path);
        let migrated = read_result.expect("version 10 project should migrate");
        assert_eq!(migrated.version, FORMAT_VERSION);
        assert_eq!(migrated.project.tracks[0].midi_parts[0].notes[0].channel, 0);
    }

    #[test]
    fn version_eight_migrates_to_empty_midi_controller_lanes() {
        let mut project = Project::default();
        project.add_track(TrackKind::Midi);
        let part_id = project.add_midi_part(0, 0.0, 8.0).unwrap();
        project.add_midi_note(0, part_id, 0.0, 1.0, 60, 96).unwrap();
        let dock_state = finite_test_dock_state();
        let workspace_state = initial_workspace_state(&dock_state).expect("workspace should build");
        let document = ProjectFile::new(project, workspace_state, dock_state, true, 0, false);
        let mut legacy = serde_json::to_value(document).expect("project should serialize");
        legacy["version"] = serde_json::Value::from(8);
        legacy["project"]
            .as_object_mut()
            .expect("project should be an object")
            .remove("next_midi_controller_id");
        legacy["project"]["tracks"][0]["midi_parts"][0]
            .as_object_mut()
            .expect("part should be an object")
            .remove("controller_events");

        let path = std::env::temp_dir().join(format!(
            "opentrack-v8-project-{}-{}.rwp",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(
            &path,
            serde_json::to_vec(&legacy).expect("legacy project should encode"),
        )
        .expect("legacy project should write");
        let read_result = ProjectFile::read(&path);
        let _ = fs::remove_file(path);
        let migrated = read_result.expect("version 8 project should migrate");
        assert_eq!(migrated.version, FORMAT_VERSION);
        assert_eq!(migrated.project.next_midi_controller_id, 1);
        assert!(
            migrated.project.tracks[0].midi_parts[0]
                .controller_events
                .is_empty()
        );
    }

    fn set_test_dock_geometry(dock_state: &mut serde_json::Value) {
        let leaf = &mut dock_state["surfaces"][0]["Main"]["nodes"][0]["Leaf"];
        for rectangle in ["rect", "viewport"] {
            for edge in ["min", "max"] {
                for axis in ["x", "y"] {
                    leaf[rectangle][edge][axis] = serde_json::Value::from(0.0);
                }
            }
        }
    }

    fn finite_test_dock_state() -> DockState<PanelTab> {
        let mut dock_state = serde_json::to_value(DockState::new(vec![PanelTab::Project]))
            .expect("dock state should serialize");
        set_test_dock_geometry(&mut dock_state);
        serde_json::from_value(dock_state).expect("finite dock state should deserialize")
    }
}
