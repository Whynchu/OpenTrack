//! Saved window layouts and workspace recall behavior.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Stable identifier for an application window.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowId(String);

impl WindowId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Workspace number used by the reference application's shortcut slots.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceId(u8);

impl WorkspaceId {
    /// Creates a workspace ID in the supported shortcut range 1-9.
    pub fn new(value: u8) -> Option<Self> {
        (1..=9).contains(&value).then_some(Self(value))
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Window class used when copying layouts into global presets.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowClass {
    Main,
    ProjectSpecific,
}

/// Window bounds in logical desktop coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowBounds {
    /// Position of the decorated OS window.
    pub left: f64,
    pub top: f64,
    /// Size of the decorated OS window.
    pub width: f64,
    pub height: f64,
    /// Client-area size used when restoring a native window. Older project
    /// files omit these values and fall back to `width` and `height`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_height: Option<f64>,
    pub monitor_id: Option<String>,
}

/// Window state that is restored with a workspace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub id: WindowId,
    pub class: WindowClass,
    pub open: bool,
    pub bounds: Option<WindowBounds>,
    /// View settings such as zoom, track height, or editor mode. Keys are
    /// stable application identifiers; values are versioned text payloads.
    pub view_state: BTreeMap<String, String>,
}

/// A named layout saved with an individual project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub shortcut_id: Option<WorkspaceId>,
    pub locked: bool,
    /// Identifies a workspace created automatically from a global preset.
    #[serde(default)]
    pub instantiated_from_preset: Option<String>,
    pub windows: Vec<WindowSnapshot>,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            shortcut_id: None,
            locked: false,
            instantiated_from_preset: None,
            windows: Vec::new(),
        }
    }
}

/// Reusable layout preset. Presets include main windows only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePreset {
    pub name: String,
    pub windows: Vec<WindowSnapshot>,
}

impl WorkspacePreset {
    pub fn from_workspace(name: impl Into<String>, workspace: &Workspace) -> Self {
        Self {
            name: name.into(),
            windows: workspace
                .windows
                .iter()
                .filter(|window| window.class == WindowClass::Main)
                .cloned()
                .collect(),
        }
    }

    /// Applies main-window state while preserving project-specific windows.
    pub fn apply_to(&self, workspace: &mut Workspace) {
        let project_windows: Vec<_> = workspace
            .windows
            .iter()
            .filter(|window| window.class == WindowClass::ProjectSpecific)
            .cloned()
            .collect();

        workspace.windows = self
            .windows
            .iter()
            .filter(|window| window.class == WindowClass::Main)
            .cloned()
            .collect();
        workspace.windows.extend(project_windows);
    }
}

/// Global workspace preferences and presets.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePreferences {
    pub global_presets: Vec<WorkspacePreset>,
    pub auto_instantiate_presets: bool,
}

/// Workspace state persisted with one project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectWorkspaceState {
    pub workspaces: Vec<Workspace>,
    pub project_presets: Vec<WorkspacePreset>,
    pub active_index: Option<usize>,
    pub use_explicit_ids: bool,
}

impl ProjectWorkspaceState {
    /// Rejects invalid active indexes, shortcut IDs, and duplicate window roles.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.workspaces.len() > 9 {
            return Err("a project can contain at most 9 shortcut workspaces");
        }
        if self
            .active_index
            .is_some_and(|index| index >= self.workspaces.len())
        {
            return Err("active workspace index is outside the workspace list");
        }

        let mut shortcut_ids = BTreeSet::new();
        for workspace in &self.workspaces {
            if workspace.name.trim().is_empty() {
                return Err("workspace name is empty");
            }
            if workspace
                .instantiated_from_preset
                .as_ref()
                .is_some_and(|name| name.trim().is_empty())
            {
                return Err("workspace preset source name is empty");
            }
            if let Some(id) = workspace.shortcut_id {
                if WorkspaceId::new(id.get()).is_none() || !shortcut_ids.insert(id) {
                    return Err("workspace shortcut IDs must be unique values from 1 to 9");
                }
            }

            let mut window_ids = BTreeSet::new();
            for window in &workspace.windows {
                if !window_ids.insert(window.id.clone()) {
                    return Err("window IDs must be unique within a workspace");
                }
                if let Some(bounds) = &window.bounds {
                    if !bounds.left.is_finite()
                        || !bounds.top.is_finite()
                        || !bounds.width.is_finite()
                        || !bounds.height.is_finite()
                        || bounds.width <= 0.0
                        || bounds.height <= 0.0
                        || bounds
                            .inner_width
                            .is_some_and(|value| !value.is_finite() || value <= 0.0)
                        || bounds
                            .inner_height
                            .is_some_and(|value| !value.is_finite() || value <= 0.0)
                    {
                        return Err("window bounds must be finite and have positive dimensions");
                    }
                }
            }
        }
        for preset in &self.project_presets {
            if preset.name.trim().is_empty() {
                return Err("workspace preset name is empty");
            }
            let mut window_ids = BTreeSet::new();
            for window in &preset.windows {
                if !window_ids.insert(window.id.clone()) {
                    return Err("window IDs must be unique within a workspace preset");
                }
                if let Some(bounds) = &window.bounds
                    && (!bounds.left.is_finite()
                        || !bounds.top.is_finite()
                        || !bounds.width.is_finite()
                        || !bounds.height.is_finite()
                        || bounds.width <= 0.0
                        || bounds.height <= 0.0
                        || bounds
                            .inner_width
                            .is_some_and(|value| !value.is_finite() || value <= 0.0)
                        || bounds
                            .inner_height
                            .is_some_and(|value| !value.is_finite() || value <= 0.0))
                {
                    return Err(
                        "workspace preset bounds must be finite and have positive dimensions",
                    );
                }
            }
        }
        Ok(())
    }

    /// Adds a workspace and makes it active, matching the documented behavior.
    pub fn create_workspace(&mut self, name: impl Into<String>) -> usize {
        self.workspaces.push(Workspace::new(name));
        let index = self.workspaces.len() - 1;
        self.active_index = Some(index);
        index
    }

    /// Activates a workspace by list position.
    pub fn activate_index(&mut self, index: usize) -> Option<&Workspace> {
        self.workspaces.get(index)?;
        self.active_index = Some(index);
        self.workspaces.get(index)
    }

    /// Activates a workspace using the documented shortcut mapping.
    pub fn activate_shortcut_slot(&mut self, slot: u8) -> Option<&Workspace> {
        let index = if self.use_explicit_ids {
            let id = WorkspaceId::new(slot)?;
            self.workspaces
                .iter()
                .position(|workspace| workspace.shortcut_id == Some(id))?
        } else {
            if !(1..=9).contains(&slot) {
                return None;
            }
            usize::from(slot - 1)
        };
        self.activate_index(index)
    }

    /// Stores layout changes automatically unless the active workspace is locked.
    pub fn store_active_windows(&mut self, windows: Vec<WindowSnapshot>) -> bool {
        let Some(index) = self.active_index else {
            return false;
        };
        let Some(workspace) = self.workspaces.get_mut(index) else {
            return false;
        };
        if workspace.locked {
            return false;
        }
        workspace.windows = windows;
        true
    }

    /// Changes the active workspace's lock state and returns the new state.
    pub fn set_active_locked(&mut self, locked: bool) -> Option<bool> {
        let index = self.active_index?;
        let workspace = self.workspaces.get_mut(index)?;
        workspace.locked = locked;
        Some(locked)
    }

    /// Copies global presets into the project list, updating same-name entries.
    pub fn include_global_presets(&mut self, preferences: &WorkspacePreferences) {
        for preset in &preferences.global_presets {
            if preset.name.trim().is_empty() {
                continue;
            }
            if let Some(existing) = self
                .project_presets
                .iter_mut()
                .find(|existing| existing.name == preset.name)
            {
                *existing = preset.clone();
            } else {
                self.project_presets.push(preset.clone());
            }
        }
    }

    /// Converts global presets into project workspaces when the preference is enabled.
    pub fn instantiate_global_presets(&mut self, preferences: &WorkspacePreferences) -> usize {
        if !preferences.auto_instantiate_presets {
            return 0;
        }

        let mut instantiated = 0;
        for preset in &preferences.global_presets {
            if preset.name.trim().is_empty() {
                continue;
            }
            if self.workspaces.iter().any(|workspace| {
                workspace.instantiated_from_preset.as_deref() == Some(preset.name.as_str())
            }) {
                continue;
            }

            let main_windows: Vec<_> = preset
                .windows
                .iter()
                .filter(|window| window.class == WindowClass::Main)
                .cloned()
                .collect();
            if let Some(existing) = self.workspaces.iter_mut().find(|workspace| {
                workspace.name == preset.name
                    && workspace
                        .windows
                        .iter()
                        .filter(|window| window.class == WindowClass::Main)
                        .cloned()
                        .collect::<Vec<_>>()
                        == main_windows
            }) {
                // Version 12 workspaces have no origin field. Recognize an
                // exact name/layout match once so automatic instantiation is
                // idempotent after migration.
                existing.instantiated_from_preset = Some(preset.name.clone());
                continue;
            }
            if self.workspaces.len() >= 9 {
                break;
            }

            let mut workspace = Workspace::new(preset.name.clone());
            workspace.instantiated_from_preset = Some(preset.name.clone());
            workspace.windows = main_windows;
            self.workspaces.push(workspace);
            instantiated += 1;
        }
        instantiated
    }
}
