//! Project templates: a saved structure to start new projects from.
//!
//! A template is the project with its content removed (see
//! [`opentrack_core::project::Project::as_template`]): tracks, groups, routing,
//! gains, EQ, inserts and sends set up once, reused for the next song. Templates
//! live outside any project so they are reachable from every project and survive
//! the folder they were made in.
//!
//! The file format is deliberately small and versioned rather than a full project
//! document, so a template can also be written by hand and a future format change
//! is detectable instead of silently misread.

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

use opentrack_core::project::Project;

const TEMPLATE_FORMAT: &str = "opentrack-template";
const TEMPLATE_VERSION: u32 = 1;
pub(super) const MAX_TEMPLATE_NAME_CHARS: usize = 64;
pub(super) const MAX_TEMPLATES: usize = 200;

/// One saved template on disk.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TemplateEntry {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
    pub tracks: usize,
}

#[derive(Deserialize, Serialize)]
struct TemplateFile {
    format: String,
    version: u32,
    name: String,
    project: Project,
}

/// Root of the application's own data directory.
///
/// `OPENTRACK_DATA_DIR` overrides it, which is what the tests use so they never
/// touch the real user directory.
pub(super) fn data_root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("OPENTRACK_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("OpenTrack"))
    }
    #[cfg(not(windows))]
    {
        if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
            return Some(PathBuf::from(dir).join("opentrack"));
        }
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share/opentrack"))
    }
}

pub(super) fn templates_directory() -> Option<PathBuf> {
    data_root().map(|root| root.join("templates"))
}

/// Turns a user-typed name into a safe file name, or `None` when it is empty.
pub(super) fn sanitize_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let cleaned = trimmed
        .chars()
        .take(MAX_TEMPLATE_NAME_CHARS)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    (!cleaned.is_empty()).then_some(cleaned)
}

/// Writes `project`'s structure as a template and returns its path.
pub(super) fn save(
    directory: &Path,
    name: &str,
    project: &Project,
) -> Result<(String, PathBuf), String> {
    let safe = sanitize_name(name).ok_or_else(|| "Give the template a name".to_owned())?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let path = directory.join(format!("{safe}.otemplate.json"));
    let document = TemplateFile {
        format: TEMPLATE_FORMAT.to_owned(),
        version: TEMPLATE_VERSION,
        name: safe.clone(),
        project: project.as_template(),
    };
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("could not encode the template: {error}"))?;
    // Stage then rename, so a failure cannot leave a half-written template.
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, bytes)
        .map_err(|error| format!("could not write {}: {error}", temp.display()))?;
    std::fs::rename(&temp, &path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        format!("could not replace {}: {error}", path.display())
    })?;
    Ok((safe, path))
}

/// Every readable template in `directory`, newest first.
pub(super) fn list(directory: &Path) -> Vec<TemplateEntry> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut templates = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .ends_with(".otemplate.json")
        })
        .filter_map(|entry| {
            let path = entry.path();
            let document = read_file(&path).ok()?;
            let metadata = entry.metadata().ok()?;
            Some(TemplateEntry {
                name: document.name,
                path,
                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                tracks: document.project.tracks.len(),
            })
        })
        .take(MAX_TEMPLATES)
        .collect::<Vec<_>>();
    templates.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.name.cmp(&right.name))
    });
    templates
}

fn read_file(path: &Path) -> Result<TemplateFile, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let document: TemplateFile = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not a readable template: {error}", path.display()))?;
    if document.format != TEMPLATE_FORMAT {
        return Err(format!("{} is not an OpenTrack template", path.display()));
    }
    if document.version != TEMPLATE_VERSION {
        return Err(format!(
            "{} was written by a newer OpenTrack (version {})",
            path.display(),
            document.version
        ));
    }
    Ok(document)
}

/// Reads a template's project, ready to become a new project.
pub(super) fn load(path: &Path) -> Result<(String, Project), String> {
    let document = read_file(path)?;
    Ok((document.name, document.project))
}

/// Removes a template and reports whether it existed.
pub(super) fn remove(path: &Path) -> bool {
    std::fs::remove_file(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::OpenTrackApp;
    use eframe::egui;
    use opentrack_core::project::TrackKind;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("opentrack-templates-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn project_with_content() -> Project {
        let mut project = Project::default();
        project.name = "Song".to_owned();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        project.tracks[0].name = "Drums Bus".to_owned();
        project.tracks[1].name = "Kick".to_owned();
        let group = project.tracks[0].id;
        project.tracks[1].parent_group_id = Some(group);
        project.tracks[1].gain = 0.5;
        project
            .add_audio_clip(1, PathBuf::from("/tmp/kick.wav"), 0.0, 2.0)
            .expect("clip");
        project.add_marker(4.0, "Chorus").expect("marker");
        project.loop_start_beats = 4.0;
        project.loop_end_beats = 12.0;
        project
    }

    #[test]
    fn a_template_keeps_the_structure_and_drops_the_song() {
        let project = project_with_content();
        let template = project.as_template();

        assert_eq!(
            template.tracks.len(),
            2,
            "tracks are the point of a template"
        );
        assert_eq!(template.tracks[0].name, "Drums Bus");
        assert_eq!(template.tracks[1].gain, 0.5, "mix settings are kept");
        assert_eq!(
            template.tracks[1].parent_group_id,
            Some(project.tracks[0].id)
        );

        assert!(template.tracks.iter().all(|track| track.clips.is_empty()));
        assert!(
            template
                .tracks
                .iter()
                .all(|track| track.midi_parts.is_empty())
        );
        assert!(template.media_pool.is_empty());
        assert!(template.markers.is_empty());
        assert_eq!(template.next_marker_id, 1);
        assert_eq!(template.next_clip_id, 1);
        assert_eq!(template.name, "Untitled Project");
        let default_project = Project::default();
        assert_eq!(template.loop_start_beats, default_project.loop_start_beats);
        assert_eq!(template.loop_end_beats, default_project.loop_end_beats);
        assert_eq!(template.selected_track, None);
        assert!(template.validate().is_ok());
    }

    #[test]
    fn saving_and_loading_a_template_round_trips_the_structure() {
        let dir = temp_dir("roundtrip");
        let project = project_with_content();
        let (name, path) = save(&dir, "Drum Bus Start", &project).expect("save");
        assert_eq!(name, "Drum Bus Start");
        assert!(path.exists());

        let listed = list(&dir);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Drum Bus Start");
        assert_eq!(listed[0].tracks, 2);

        let (loaded_name, loaded) = load(&path).expect("load");
        assert_eq!(loaded_name, "Drum Bus Start");
        assert_eq!(loaded.tracks.len(), 2);
        assert_eq!(loaded.tracks[1].gain, 0.5);
        assert!(loaded.tracks.iter().all(|track| track.clips.is_empty()));

        // Saving the same name replaces the template instead of duplicating it.
        let _ = save(&dir, "Drum Bus Start", &project).expect("save again");
        assert_eq!(list(&dir).len(), 1);

        assert!(remove(&path));
        assert!(!remove(&path));
        assert!(list(&dir).is_empty());
    }

    #[test]
    fn names_are_sanitized_and_empty_names_are_refused() {
        assert_eq!(
            sanitize_name("  Drums / Bus  ").as_deref(),
            Some("Drums _ Bus")
        );
        assert_eq!(sanitize_name("   "), None);
        assert_eq!(sanitize_name("..."), None);
        let long = "x".repeat(MAX_TEMPLATE_NAME_CHARS * 2);
        assert_eq!(
            sanitize_name(&long).map(|name| name.chars().count()),
            Some(MAX_TEMPLATE_NAME_CHARS)
        );

        let dir = temp_dir("naming");
        assert!(save(&dir, "", &project_with_content()).is_err());
    }

    #[test]
    fn a_corrupt_or_foreign_template_is_reported_not_listed() {
        let dir = temp_dir("corrupt");
        std::fs::write(dir.join("Broken.otemplate.json"), b"{ not json").expect("write");
        std::fs::write(
            dir.join("Other.otemplate.json"),
            br#"{"format":"something-else","version":1,"name":"Other","project":{}}"#,
        )
        .expect("write");
        assert!(list(&dir).is_empty(), "unreadable templates are skipped");
        assert!(load(&dir.join("Broken.otemplate.json")).is_err());
        assert!(load(&dir.join("Other.otemplate.json")).is_err());
    }

    #[test]
    fn the_app_saves_and_reopens_a_template_without_the_song() {
        // The app reads its template folder from this variable, so the test never
        // touches the real user directory.
        let root = temp_dir("app-data");
        // SAFETY: this test binary does not read the variable concurrently.
        unsafe { std::env::set_var("OPENTRACK_DATA_DIR", &root) };

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Group);
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.tracks[1].name = "Kick".to_owned();
        app.data.project.tracks[1].gain = 0.25;
        let group = app.data.project.tracks[0].id;
        app.data.project.tracks[1].parent_group_id = Some(group);
        app.data
            .project
            .add_audio_clip(1, PathBuf::from("/tmp/kick.wav"), 0.0, 2.0)
            .expect("clip");
        app.data.project.add_marker(8.0, "Chorus").expect("marker");

        app.save_project_as_template("Drum Starter");
        assert!(
            app.status_message
                .contains("Saved the template 'Drum Starter'"),
            "status was {}",
            app.status_message
        );
        assert!(app.template_name_draft.is_none(), "the dialog closed");

        let directory = templates_directory().expect("directory");
        let listed = list(&directory);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].tracks, 2);

        let (name, project) = load(&listed[0].path).expect("load");
        assert_eq!(name, "Drum Starter");
        assert_eq!(project.tracks[1].name, "Kick");
        assert_eq!(project.tracks[1].gain, 0.25);
        assert_eq!(project.tracks[1].parent_group_id, Some(group));
        assert!(project.tracks.iter().all(|track| track.clips.is_empty()));
        assert!(project.markers.is_empty());

        // An empty name is refused and says so without writing anything.
        app.save_project_as_template("   ");
        assert!(app.status_message.contains("Could not save the template"));

        let _ = std::fs::remove_dir_all(root);
        unsafe { std::env::remove_var("OPENTRACK_DATA_DIR") };
    }
}
