//! OpenTrack Track Archive v1: a directory package with a signed manifest and
//! copied media. This is OpenTrack's own format; it is not Cubase Track Archive,
//! OMR or OMF.
//!
//! The package is written into a staging directory in the destination's parent
//! and renamed into place only after every file and the manifest are complete.

use super::*;
use opentrack_core::project::{Project, Track};
use opentrack_core::routing::OutputRoute;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) const ARCHIVE_FORMAT: &str = "opentrack-track-archive";
pub(super) const ARCHIVE_VERSION: u32 = 1;
const MANIFEST_NAME: &str = "manifest.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ArchiveManifest {
    pub format: String,
    pub version: u32,
    pub app_version: String,
    pub tracks: Vec<Track>,
    pub media: Vec<ArchiveMedia>,
    pub external_routes: Vec<String>,
    pub external_plug_ins: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ArchiveMedia {
    pub original_path: String,
    pub archive_path: String,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ArchiveReport {
    pub track_count: usize,
    pub media_count: usize,
    pub external_routes: Vec<String>,
    pub external_plug_ins: Vec<String>,
    pub destination: PathBuf,
}

fn included_tracks(project: &Project, selected: &[u64]) -> Vec<Track> {
    if selected.is_empty() {
        return Vec::new();
    }
    let mut ids = selected.iter().copied().collect::<BTreeSet<_>>();
    loop {
        let previous = ids.len();
        // Keep descendants of selected groups/folders.
        for track in &project.tracks {
            if track
                .parent_group_id
                .is_some_and(|parent| ids.contains(&parent))
                || track
                    .parent_folder_id
                    .is_some_and(|parent| ids.contains(&parent))
            {
                ids.insert(track.id);
            }
        }
        // Keep the owning bus/folder so references and hierarchy survive.
        for id in ids.clone() {
            if let Some(track) = project.tracks.iter().find(|track| track.id == id) {
                if let Some(parent) = track.parent_group_id {
                    ids.insert(parent);
                }
                if let Some(parent) = track.parent_folder_id {
                    ids.insert(parent);
                }
            }
        }
        if ids.len() == previous {
            break;
        }
    }
    project
        .tracks
        .iter()
        .filter(|track| ids.contains(&track.id))
        .cloned()
        .collect()
}

fn safe_file_name(name: &str) -> String {
    let filtered = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let trimmed = filtered.trim_matches(['.', ' ']).to_owned();
    if trimmed.is_empty() {
        "media".to_owned()
    } else {
        trimmed.chars().take(96).collect()
    }
}

fn copy_stage(destination: &Path) -> PathBuf {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "track-archive".to_owned());
    parent.join(format!(
        ".{name}.staging-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ))
}

pub(super) fn export(
    project: &Project,
    selected_tracks: &[u64],
    destination: &Path,
) -> Result<ArchiveReport, String> {
    if destination.exists() {
        return Err(format!(
            "The destination already exists; choose a new folder: {}",
            destination.display()
        ));
    }
    let tracks = included_tracks(project, selected_tracks);
    if tracks.is_empty() {
        return Err("Select at least one track to export".to_owned());
    }
    let included_ids = tracks.iter().map(|track| track.id).collect::<BTreeSet<_>>();
    let mut external_routes = Vec::new();
    let mut external_plug_ins = BTreeSet::new();
    for track in &tracks {
        let routes = track
            .output_routing
            .as_deref()
            .into_iter()
            .chain(track.send_routing.iter().flatten().map(String::as_str));
        for route in routes {
            if let Ok(route) = OutputRoute::parse(route)
                && let Some(bus) = route.bus()
                && !included_ids.contains(&bus)
            {
                external_routes.push(format!("{} -> {route:?}", track.name));
            }
        }
        for insert in &track.inserts {
            if let opentrack_core::project::InsertEffectKind::Vst3 {
                class_id,
                module_path,
                name,
                ..
            } = &insert.kind
            {
                external_plug_ins.insert(format!("{name} [{class_id}] {}", module_path.display()));
            }
        }
    }
    external_routes.sort();
    external_routes.dedup();

    let staging = copy_stage(destination);
    if staging.exists() {
        return Err("The archive staging folder already exists".to_owned());
    }
    let result = (|| -> Result<ArchiveReport, String> {
        std::fs::create_dir_all(staging.join("Media"))
            .map_err(|error| format!("Could not create the archive folder: {error}"))?;
        let mut media = Vec::new();
        let mut by_source: BTreeMap<PathBuf, String> = BTreeMap::new();
        let mut used_names = BTreeSet::new();
        for track in &tracks {
            for clip in &track.clips {
                if by_source.contains_key(&clip.source_path) {
                    continue;
                }
                let source = clip.source_path.as_path();
                if !source.is_file() {
                    return Err(format!(
                        "Missing media: {}; the archive was not published",
                        source.display()
                    ));
                }
                let base = source
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "media".to_owned());
                let mut name = safe_file_name(&base);
                let mut suffix = 1;
                while !used_names.insert(name.clone()) {
                    name = format!("{}_{suffix}_{}", suffix, safe_file_name(&base));
                    suffix += 1;
                }
                let archive_path = format!("Media/{name}");
                std::fs::copy(source, staging.join(&archive_path))
                    .map_err(|error| format!("Could not copy {}: {error}", source.display()))?;
                by_source.insert(clip.source_path.clone(), archive_path.clone());
                media.push(ArchiveMedia {
                    original_path: clip.source_path.to_string_lossy().into_owned(),
                    archive_path,
                });
            }
        }
        let manifest = ArchiveManifest {
            format: ARCHIVE_FORMAT.to_owned(),
            version: ARCHIVE_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            tracks,
            media: media.clone(),
            external_routes: external_routes.clone(),
            external_plug_ins: external_plug_ins.iter().cloned().collect(),
        };
        let bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| format!("Could not write the archive manifest: {error}"))?;
        std::fs::write(staging.join(MANIFEST_NAME), bytes)
            .map_err(|error| format!("Could not write the archive manifest: {error}"))?;
        std::fs::rename(&staging, destination)
            .map_err(|error| format!("Could not publish the archive: {error}"))?;
        Ok(ArchiveReport {
            track_count: manifest.tracks.len(),
            media_count: media.len(),
            external_routes,
            external_plug_ins: manifest.external_plug_ins,
            destination: destination.to_path_buf(),
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}

impl OpenTrackApp {
    pub(super) fn export_track_archive(&mut self) {
        if self.track_archive_task.is_some() {
            self.status_message = "A track archive export is already running".to_owned();
            return;
        }
        let mut selected = self
            .arrange_view_state
            .selected_track_ids
            .iter()
            .copied()
            .collect::<Vec<_>>();
        if selected.is_empty()
            && let Some(track) = self
                .data
                .project
                .selected_track
                .and_then(|index| self.data.project.tracks.get(index as usize))
        {
            selected.push(track.id);
        }
        if selected.is_empty() {
            self.status_message = "Select at least one track to export".to_owned();
            return;
        }
        let Some(destination) = FileDialog::new()
            .set_title("Export OpenTrack Track Archive")
            .pick_folder()
        else {
            return;
        };
        let destination = destination.join("OpenTrack Track Archive");
        let project = self.data.project.clone();
        let selected_count = selected.len();
        let destination_for_status = destination.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = export(&project, &selected, &destination)
                .map_err(|error| format!("Track archive export failed: {error}"));
            let _ = sender.send(result);
        });
        self.track_archive_task = Some(receiver);
        self.status_message = format!(
            "Exporting {selected_count} track(s) to {}...",
            destination_for_status.display()
        );
    }

    pub(super) fn import_track_archive(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before importing a track archive".to_owned();
            return;
        }
        let Some(project_dir) = self
            .project_path
            .as_deref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
        else {
            self.status_message = "Save the project before importing a track archive".to_owned();
            return;
        };
        let Some(package) = FileDialog::new()
            .set_title("Import OpenTrack Track Archive")
            .pick_folder()
        else {
            return;
        };
        let (tracks, report) = match prepare_import(&self.data.project, &project_dir, &package) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = error;
                return;
            }
        };
        let mut candidate = self.data.project.clone();
        candidate.tracks.extend(tracks);
        refresh_counters(&mut candidate);
        if candidate.validate().is_err() {
            self.status_message = "The imported tracks are invalid; nothing changed".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot import: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Import Track Archive");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Imported {} track(s) and {} media file(s) from OpenTrack Track Archive v1. Disconnected routes: {}; external plug-ins: {}",
            report.track_count,
            report.media_count,
            if report.disconnected_routes.is_empty() {
                "none".to_owned()
            } else {
                report.disconnected_routes.join(", ")
            },
            if report.external_plug_ins.is_empty() {
                "none".to_owned()
            } else {
                report.external_plug_ins.join(", ")
            }
        );
    }

    pub(super) fn poll_track_archive_task(&mut self) {
        let Some(receiver) = &self.track_archive_task else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("Track archive export stopped unexpectedly".to_owned())
            }
        };
        self.track_archive_task = None;
        match result {
            Ok(report) => {
                self.status_message = format!(
                    "Exported {} track(s) and {} media file(s) to {}. External routes: {}; external plug-ins: {}",
                    report.track_count,
                    report.media_count,
                    report.destination.display(),
                    if report.external_routes.is_empty() {
                        "none".to_owned()
                    } else {
                        report.external_routes.join(", ")
                    },
                    if report.external_plug_ins.is_empty() {
                        "none".to_owned()
                    } else {
                        report.external_plug_ins.join(", ")
                    }
                );
            }
            Err(error) => self.status_message = error,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct ImportReport {
    pub track_count: usize,
    pub media_count: usize,
    pub disconnected_routes: Vec<String>,
    pub external_plug_ins: Vec<String>,
}

fn remap_imported_route(
    route: &mut Option<String>,
    id_map: &BTreeMap<u64, u64>,
    track_name: &str,
    disconnected: &mut Vec<String>,
) {
    let Some(value) = route.as_deref() else {
        return;
    };
    match OutputRoute::parse(value) {
        Ok(parsed @ (OutputRoute::Group(id) | OutputRoute::Fx(id))) => {
            if let Some(new_id) = id_map.get(&id).copied() {
                *route = Some(
                    match parsed {
                        OutputRoute::Fx(_) => OutputRoute::Fx(new_id),
                        _ => OutputRoute::Group(new_id),
                    }
                    .persisted(),
                );
            } else {
                disconnected.push(format!("{track_name} -> {parsed:?}"));
                *route = None;
            }
        }
        Ok(parsed) if parsed.bus().is_some() => {
            disconnected.push(format!("{track_name} -> {parsed:?}"));
            *route = None;
        }
        _ => {}
    }
}

pub(super) fn refresh_counters(project: &mut Project) {
    project.next_track_id = project
        .tracks
        .iter()
        .map(|track| track.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
    project.next_track_number = project.next_track_number.max(project.tracks.len() as u32);
    project.next_clip_id = project
        .tracks
        .iter()
        .flat_map(|track| track.clips.iter().map(|clip| clip.id))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
    project.next_midi_part_id = project
        .tracks
        .iter()
        .flat_map(|track| track.midi_parts.iter().map(|part| part.id))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
    project.next_midi_note_id = project
        .tracks
        .iter()
        .flat_map(|track| track.midi_parts.iter())
        .flat_map(|part| part.notes.iter().map(|note| note.id))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
    project.next_midi_controller_id = project
        .tracks
        .iter()
        .flat_map(|track| track.midi_parts.iter())
        .flat_map(|part| part.controller_events.iter().map(|event| event.id))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
}

pub(super) fn prepare_import(
    project: &Project,
    project_dir: &Path,
    package: &Path,
) -> Result<(Vec<Track>, ImportReport), String> {
    let manifest_path = package.join(MANIFEST_NAME);
    let metadata = std::fs::metadata(&manifest_path)
        .map_err(|error| format!("Could not read the archive manifest: {error}"))?;
    if metadata.len() > 16 * 1024 * 1024 {
        return Err("The archive manifest is too large".to_owned());
    }
    let manifest: ArchiveManifest = serde_json::from_slice(
        &std::fs::read(&manifest_path)
            .map_err(|error| format!("Could not read the archive manifest: {error}"))?,
    )
    .map_err(|error| format!("The archive manifest is malformed: {error}"))?;
    if manifest.format != ARCHIVE_FORMAT {
        return Err(format!(
            "Unsupported archive format '{}'; only OpenTrack Track Archive v1 is supported",
            manifest.format
        ));
    }
    if manifest.version != ARCHIVE_VERSION {
        return Err(format!(
            "Unsupported OpenTrack archive version {}; this build reads version 1",
            manifest.version
        ));
    }
    if manifest.tracks.is_empty() || manifest.tracks.len() > 1024 {
        return Err("The archive has an invalid track count".to_owned());
    }
    if manifest.media.len() > 4096 {
        return Err("The archive has too many media entries".to_owned());
    }

    let package_root = package
        .canonicalize()
        .map_err(|error| format!("Could not resolve the archive folder: {error}"))?;
    let import_folder = project_dir.join("Audio").join("Imports").join(format!(
        "Archive-{}-{}",
        package
            .file_name()
            .map(|name| safe_file_name(&name.to_string_lossy()))
            .unwrap_or_else(|| "Import".to_owned()),
        std::process::id()
    ));
    let mut copied = Vec::new();
    let copy_result = (|| -> Result<BTreeMap<String, PathBuf>, String> {
        std::fs::create_dir_all(&import_folder)
            .map_err(|error| format!("Could not create the import folder: {error}"))?;
        let mut remapped = BTreeMap::new();
        for media in &manifest.media {
            let relative = Path::new(&media.archive_path);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
                || media.archive_path.contains(':')
                || media.archive_path.contains('\\')
            {
                return Err(format!(
                    "The archive contains an unsafe media path: {}",
                    media.archive_path
                ));
            }
            let source = package_root.join(relative);
            let resolved = source
                .canonicalize()
                .map_err(|error| format!("Missing archive media {}: {error}", source.display()))?;
            if !resolved.starts_with(&package_root) {
                return Err(format!(
                    "Archive media escapes the package: {}",
                    media.archive_path
                ));
            }
            let name = source
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "media".to_owned());
            let mut target = import_folder.join(safe_file_name(&name));
            let mut suffix = 1;
            while target.exists() {
                target =
                    import_folder.join(format!("{}_{suffix}_{}", suffix, safe_file_name(&name)));
                suffix += 1;
            }
            std::fs::copy(&resolved, &target)
                .map_err(|error| format!("Could not copy archive media: {error}"))?;
            copied.push(target.clone());
            remapped.insert(media.original_path.clone(), target);
        }
        Ok(remapped)
    })();
    let remapped_media = match copy_result {
        Ok(remapped) => remapped,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&import_folder);
            return Err(error);
        }
    };

    let mut next_track_id = project.next_track_id.max(1);
    let mut next_track_number = project.next_track_number;
    let mut next_clip_id = project.next_clip_id.max(1);
    let mut next_part_id = project.next_midi_part_id.max(1);
    let mut next_note_id = project.next_midi_note_id.max(1);
    let mut next_controller_id = project.next_midi_controller_id.max(1);
    let mut id_map = BTreeMap::new();
    for track in &manifest.tracks {
        if track.id == 0
            || next_track_id >= opentrack_core::project::INPUT_BUS_ID
            || id_map.contains_key(&track.id)
        {
            let _ = std::fs::remove_dir_all(&import_folder);
            return Err("The archive contains duplicate or invalid track IDs".to_owned());
        }
        id_map.insert(track.id, next_track_id);
        next_track_id += 1;
        next_track_number = next_track_number.saturating_add(1);
    }
    let mut used_names = project
        .tracks
        .iter()
        .map(|track| track.name.clone())
        .collect::<BTreeSet<_>>();
    let mut disconnected_routes = Vec::new();
    let mut prepared = Vec::new();
    for source in &manifest.tracks {
        let mut track = source.clone();
        track.id = *id_map.get(&source.id).expect("mapped track");
        track.parent_group_id = source
            .parent_group_id
            .and_then(|id| id_map.get(&id).copied());
        track.parent_folder_id = source
            .parent_folder_id
            .and_then(|id| id_map.get(&id).copied());
        let mut name = source.name.clone();
        let mut suffix = 2;
        while !used_names.insert(name.clone()) {
            name = format!("{} ({suffix})", source.name);
            suffix += 1;
        }
        track.name = name;
        for clip in &mut track.clips {
            clip.id = next_clip_id;
            next_clip_id += 1;
            if let Some(path) = remapped_media.get(&clip.source_path.to_string_lossy().into_owned())
            {
                clip.source_path = path.clone();
            }
        }
        for part in &mut track.midi_parts {
            part.id = next_part_id;
            next_part_id += 1;
            for note in &mut part.notes {
                note.id = next_note_id;
                next_note_id += 1;
            }
            for controller in &mut part.controller_events {
                controller.id = next_controller_id;
                next_controller_id += 1;
            }
        }
        remap_imported_route(
            &mut track.output_routing,
            &id_map,
            &track.name,
            &mut disconnected_routes,
        );
        for route in &mut track.send_routing {
            remap_imported_route(route, &id_map, &track.name, &mut disconnected_routes);
        }
        prepared.push(track);
    }
    let report = ImportReport {
        track_count: prepared.len(),
        media_count: copied.len(),
        disconnected_routes,
        external_plug_ins: manifest.external_plug_ins,
    };
    Ok((prepared, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_wav(path: &Path) {
        let data = vec![0_u8; 44 + 8];
        std::fs::write(path, data).expect("wav fixture");
    }

    #[test]
    fn archive_exports_manifest_media_and_reports_external_routes() {
        let root = std::env::temp_dir().join(format!("opentrack-archive-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let media = root.join("shared.wav");
        write_test_wav(&media);
        let mut project = Project::default();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        project
            .add_audio_clip(1, media.clone(), 0.0, 1.0)
            .expect("first");
        project
            .add_audio_clip(2, media.clone(), 2.0, 1.0)
            .expect("second");
        let group = project.tracks[0].id;
        project.tracks[1].parent_group_id = Some(group);
        project.tracks[2].send_routing = vec![Some(format!("Group:{group}"))];
        let selected = vec![project.tracks[1].id];
        let destination = root.join("Archive");
        let report = export(&project, &selected, &destination).expect("export");
        assert_eq!(report.track_count, 2, "child and its group");
        assert_eq!(report.media_count, 1, "shared media copied once");
        assert!(report.external_routes.is_empty());
        let manifest: ArchiveManifest =
            serde_json::from_slice(&std::fs::read(destination.join(MANIFEST_NAME)).unwrap())
                .expect("manifest");
        assert_eq!(manifest.format, ARCHIVE_FORMAT);
        assert_eq!(manifest.version, ARCHIVE_VERSION);
        assert!(destination.join(&manifest.media[0].archive_path).is_file());
        assert!(media.exists(), "the source is retained");

        // An existing destination is refused without touching the source.
        assert!(export(&project, &selected, &destination).is_err());
        // A missing source refuses publication and leaves no staging package.
        let missing = root.join("missing.wav");
        project.tracks[1].clips[0].source_path = missing;
        assert!(export(&project, &selected, &root.join("Second")).is_err());
        assert!(!root.join("Second").exists());
        assert!(
            !std::fs::read_dir(&root)
                .unwrap()
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().contains("staging"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn archive_import_remaps_ids_hierarchy_and_media_and_disconnects_external_routes() {
        let root = std::env::temp_dir().join(format!("opentrack-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let project_dir = root.join("Project");
        std::fs::create_dir_all(project_dir.join("Audio").join("Imports")).unwrap();
        let media = root.join("clip.wav");
        write_test_wav(&media);

        let mut source = Project::default();
        source.add_track(TrackKind::Group);
        source.add_track(TrackKind::Audio);
        source
            .add_audio_clip(1, media.clone(), 0.0, 1.0)
            .expect("clip");
        let group = source.tracks[0].id;
        source.tracks[1].parent_group_id = Some(group);
        let package = root.join("Package");
        export(&source, &[source.tracks[1].id], &package).expect("export");

        let mut target = Project::default();
        target.add_track(TrackKind::Audio);
        target.tracks[0].name = source.tracks[0].name.clone();
        let (tracks, report) = prepare_import(&target, &project_dir, &package).expect("import");
        assert_eq!(report.track_count, 2);
        assert_eq!(report.media_count, 1);
        assert!(tracks.iter().all(|track| track.id >= target.next_track_id));
        assert_ne!(tracks[0].name, source.tracks[0].name);
        let imported_group = tracks[0].id;
        assert_eq!(tracks[1].parent_group_id, Some(imported_group));
        assert!(tracks[1].clips[0].source_path.exists());
        assert!(tracks[1].clips[0].source_path.starts_with(&project_dir));
        target.tracks.extend(tracks);
        refresh_counters(&mut target);
        assert_eq!(target.validate(), Ok(()));

        // A manifest with a traversal path is rejected before tracks are prepared.
        let escaped = root.join("Escaped");
        std::fs::create_dir_all(&escaped).unwrap();
        let mut manifest: ArchiveManifest =
            serde_json::from_slice(&std::fs::read(package.join(MANIFEST_NAME)).unwrap()).unwrap();
        manifest.media[0].archive_path = "../clip.wav".to_owned();
        std::fs::write(
            escaped.join(MANIFEST_NAME),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(prepare_import(&target, &project_dir, &escaped).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn external_bus_links_are_reported() {
        let root =
            std::env::temp_dir().join(format!("opentrack-archive-ext-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut project = Project::default();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        let group = project.tracks[0].id;
        project.tracks[1].send_routing = vec![Some(format!("Group:{group}"))];
        let report = export(&project, &[project.tracks[1].id], &root.join("Out")).expect("export");
        assert!(
            report
                .external_routes
                .iter()
                .any(|route| route.contains("Group"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
