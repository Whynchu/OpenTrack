//! Application-wide track library: named track templates with their kind,
//! insert rack and portable routing.
//!
//! OpenTrack keeps one library in application settings rather than a file per
//! library, so File > New Library and File > Open Library both show the same
//! window. Routing that names a group or FX return is project-specific and is
//! dropped when saving, so a template never points at an id that does not exist
//! in the target project.

use super::*;
use serde::{Deserialize, Serialize};

/// Most tracks one library keeps.
pub(super) const MAX_LIBRARY_TRACKS: usize = 100;
/// Most inserts a library track keeps.
pub(super) const MAX_LIBRARY_INSERTS: usize = 16;
/// Longest library track name.
pub(super) const MAX_LIBRARY_NAME: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct LibraryTrack {
    pub name: String,
    pub kind: TrackKind,
    #[serde(default = "default_audio_channels")]
    pub audio_channels: u8,
    #[serde(default)]
    pub inserts: Vec<InsertEffect>,
    #[serde(default)]
    pub input_routing: Option<String>,
    #[serde(default)]
    pub output_routing: Option<String>,
}

fn default_audio_channels() -> u8 {
    2
}

impl LibraryTrack {
    pub(super) fn kind_label(&self) -> &'static str {
        match self.kind {
            TrackKind::Audio => "Audio",
            TrackKind::Midi => "MIDI",
            TrackKind::Instrument => "Instrument",
            TrackKind::Group => "Group",
            TrackKind::Fx => "FX Return",
            TrackKind::Folder => "Folder",
        }
    }
}

/// Input routes that do not name a project-specific channel.
pub(super) fn portable_input_route(route: &str) -> Option<String> {
    match route {
        "Stereo In" | "Left - Stereo In" | "Right - Stereo In" | "No Bus" => Some(route.to_owned()),
        _ => None,
    }
}

/// Output routes that do not name a project-specific group or FX return.
pub(super) fn portable_output_route(route: &str) -> Option<String> {
    match route {
        "Stereo Out" | "Left" | "Right" | "No Bus" => Some(route.to_owned()),
        _ => None,
    }
}

/// Trims names, clamps widths, drops non-portable routes, caps and de-duplicates.
pub(super) fn sanitize_library(mut entries: Vec<LibraryTrack>) -> Vec<LibraryTrack> {
    entries.retain(|entry| !entry.name.trim().is_empty());
    for entry in &mut entries {
        entry.name = entry
            .name
            .trim()
            .chars()
            .take(MAX_LIBRARY_NAME)
            .collect::<String>();
        entry.audio_channels = entry.audio_channels.clamp(1, 2);
        entry.inserts.truncate(MAX_LIBRARY_INSERTS);
        entry.input_routing = entry
            .input_routing
            .as_deref()
            .and_then(portable_input_route);
        entry.output_routing = entry
            .output_routing
            .as_deref()
            .and_then(portable_output_route);
    }
    let mut seen = std::collections::BTreeSet::new();
    entries.retain(|entry| seen.insert(entry.name.to_lowercase()));
    entries.truncate(MAX_LIBRARY_TRACKS);
    entries
}

/// `base`, then `base 2`, and so on, avoiding names already in the library.
fn unique_library_name(entries: &[LibraryTrack], base: &str) -> String {
    let trimmed = base.trim();
    let base = if trimmed.is_empty() { "Track" } else { trimmed };
    if !entries
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case(base))
    {
        return base.to_owned();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base} {suffix}");
        if !entries
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
        suffix += 1;
    }
}

impl OpenTrackApp {
    fn open_track_library(&mut self) {
        self.track_library_open = true;
    }

    /// Saves the selected track's kind, inserts and portable routing.
    pub(super) fn save_selected_track_to_library(&mut self) -> Option<String> {
        let Some(index) = self.data.project.selected_track.map(|index| index as usize) else {
            self.status_message = "Select a track before saving it to the library".to_owned();
            return None;
        };
        let Some(track) = self.data.project.tracks.get(index) else {
            self.status_message = "The selected track is no longer in the project".to_owned();
            return None;
        };
        if self.track_library.len() >= MAX_LIBRARY_TRACKS {
            self.status_message =
                format!("The track library holds at most {MAX_LIBRARY_TRACKS} tracks");
            return None;
        }
        let name = unique_library_name(&self.track_library, &track.name);
        let entry = LibraryTrack {
            name: name.clone(),
            kind: track.kind,
            audio_channels: track.audio_channels.clamp(1, 2),
            inserts: track.inserts.clone(),
            input_routing: track
                .input_routing
                .as_deref()
                .and_then(portable_input_route),
            output_routing: track
                .output_routing
                .as_deref()
                .and_then(portable_output_route),
        };
        self.track_library.push(entry);
        self.status_message = format!(
            "Saved '{}' to the track library ({} insert(s)); group/FX routing is project-specific and not saved",
            name,
            track.inserts.len()
        );
        Some(name)
    }

    fn delete_library_track(&mut self, index: usize) {
        if index < self.track_library.len() {
            let removed = self.track_library.remove(index);
            self.status_message = format!("Removed '{}' from the track library", removed.name);
        }
    }

    /// Adds a library track to the project as one undo step.
    fn add_library_track(&mut self, index: usize) -> bool {
        let Some(entry) = self.track_library.get(index).cloned() else {
            return false;
        };
        let mut candidate = self.data.project.clone();
        candidate.add_track_with_audio_channels(entry.kind, entry.audio_channels);
        let name = unique_track_name(&candidate, &entry.name);
        let Some(track) = candidate.tracks.last_mut() else {
            self.status_message = "Could not add the library track".to_owned();
            return false;
        };
        track.name = name;
        track.inserts = entry.inserts.clone();
        track.input_routing = entry.input_routing.clone();
        track.output_routing = entry.output_routing.clone();
        if candidate.validate().is_err() {
            self.status_message = "That library track would leave an invalid project".to_owned();
            return false;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message =
                "Cannot add the library track: undo snapshot is unavailable".to_owned();
            return false;
        }
        self.project_history.label_next("Add Track from Library");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Added '{}' from the library ({} insert(s))",
            entry.name,
            entry.inserts.len()
        );
        true
    }

    pub(super) fn show_track_library(&mut self, ctx: &egui::Context) {
        if !self.track_library_open {
            return;
        }
        let entries = self.track_library.clone();
        let has_selection = self.data.project.selected_track.is_some();
        let mut open = true;
        let mut save = false;
        let mut add = None;
        let mut delete = None;
        let response = egui::Window::new("Track Library")
            .id(Id::new("track-library"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(560.0, 380.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(520.0);
                ui.label(
                    egui::RichText::new(
                        "One application-wide library of track templates: kind, insert rack and portable routing. New and Open both show it.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(has_selection, egui::Button::new("Save Selected Track"))
                        .on_hover_text(
                            "Saves the selected track's kind, inserts and portable routing",
                        )
                        .clicked()
                    {
                        save = true;
                    }
                    ui.label(format!("{} template(s)", entries.len()));
                });
                ui.separator();
                if entries.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No tracks saved yet. Select a track and use Save Selected Track or File > Save Library.",
                        )
                        .small()
                        .color(theme::palette(ui).muted),
                    );
                }
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for (index, entry) in entries.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.button("Add").clicked() {
                                    add = Some(index);
                                }
                                ui.label(egui::RichText::new(&entry.name).strong());
                                ui.label(entry.kind_label());
                                ui.label(format!("{} insert(s)", entry.inserts.len()));
                                let mut routing = Vec::new();
                                if let Some(route) = &entry.input_routing {
                                    routing.push(format!("in {route}"));
                                }
                                if let Some(route) = &entry.output_routing {
                                    routing.push(format!("out {route}"));
                                }
                                if !routing.is_empty() {
                                    ui.label(
                                        egui::RichText::new(routing.join(", "))
                                            .small()
                                            .color(theme::palette(ui).muted),
                                    );
                                }
                                if ui.small_button("\u{2715}").clicked() {
                                    delete = Some(index);
                                }
                            });
                        }
                    });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("track-library"),
            &mut open,
            response.as_ref(),
        );
        if save {
            self.save_selected_track_to_library();
        }
        if let Some(index) = add {
            self.add_library_track(index);
        }
        if let Some(index) = delete {
            self.delete_library_track(index);
        }
        self.track_library_open = open;
    }
}

/// The entry name, suffixed when the project already uses it.
fn unique_track_name(project: &Project, base: &str) -> String {
    let base = if base.trim().is_empty() {
        "Track"
    } else {
        base.trim()
    };
    if !project
        .tracks
        .iter()
        .any(|track| track.name.eq_ignore_ascii_case(base))
    {
        return base.to_owned();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base} {suffix}");
        if !project
            .tracks
            .iter()
            .any(|track| track.name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.selected_track = Some(0);
        app.project_history.clear();
        app
    }

    fn insert() -> InsertEffect {
        InsertEffect {
            id: 0,
            enabled: true,
            bypassed: false,
            kind: InsertEffectKind::StereoDelay {
                time_ms: 120.0,
                feedback: 0.2,
                mix: 0.25,
            },
        }
    }

    #[test]
    fn saving_captures_kind_inserts_and_only_portable_routing() {
        let mut app = app();
        app.data.project.tracks[0].name = "Lead Guitar".to_owned();
        app.data.project.tracks[0].inserts.push(insert());
        app.data.project.tracks[0].input_routing = Some("Stereo In".to_owned());
        app.data.project.tracks[0].output_routing = Some("Group:7".to_owned());

        let name = app.save_selected_track_to_library().expect("saved");
        assert_eq!(name, "Lead Guitar");
        let entry = &app.track_library[0];
        assert_eq!(entry.kind, TrackKind::Audio);
        assert_eq!(entry.inserts.len(), 1);
        assert_eq!(entry.input_routing.as_deref(), Some("Stereo In"));
        assert!(
            entry.output_routing.is_none(),
            "a group route is project-specific and must not be saved"
        );

        // Saving the same track again takes a new name.
        app.data.project.tracks[0].name = "Lead Guitar".to_owned();
        assert_eq!(
            app.save_selected_track_to_library().as_deref(),
            Some("Lead Guitar 2")
        );
        assert_eq!(app.track_library.len(), 2);
    }

    #[test]
    fn adding_a_library_track_restores_inserts_and_routing_as_one_step() {
        let mut app = app();
        app.data.project.tracks[0].name = "Lead Guitar".to_owned();
        app.data.project.tracks[0].inserts.push(insert());
        app.data.project.tracks[0].input_routing = Some("Stereo In".to_owned());
        app.data.project.tracks[0].output_routing = Some("Stereo Out".to_owned());
        app.save_selected_track_to_library().expect("saved");

        assert!(app.add_library_track(0));
        assert_eq!(app.data.project.tracks.len(), 2);
        let added = &app.data.project.tracks[1];
        assert_eq!(added.name, "Lead Guitar 2", "the project name is unique");
        assert_eq!(added.inserts.len(), 1);
        assert_eq!(added.input_routing.as_deref(), Some("Stereo In"));
        assert_eq!(added.output_routing.as_deref(), Some("Stereo Out"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(
            app.project_history.steps()[0].label,
            "Add Track from Library"
        );

        // Deleting removes it from the library.
        app.delete_library_track(0);
        assert!(app.track_library.is_empty());
        assert!(!app.add_library_track(0));
    }

    #[test]
    fn the_library_is_sanitised_and_bounded() {
        let mut entries = vec![
            LibraryTrack {
                name: "  Vocals  ".to_owned(),
                kind: TrackKind::Audio,
                audio_channels: 9,
                inserts: vec![insert(); MAX_LIBRARY_INSERTS + 3],
                input_routing: Some("Group:3".to_owned()),
                output_routing: Some("Left".to_owned()),
            },
            LibraryTrack {
                name: "vocals".to_owned(),
                kind: TrackKind::Audio,
                audio_channels: 2,
                inserts: Vec::new(),
                input_routing: None,
                output_routing: None,
            },
            LibraryTrack {
                name: "   ".to_owned(),
                kind: TrackKind::Audio,
                audio_channels: 2,
                inserts: Vec::new(),
                input_routing: None,
                output_routing: None,
            },
        ];
        entries.extend((0..120).map(|index| LibraryTrack {
            name: format!("Template {index}"),
            kind: TrackKind::Midi,
            audio_channels: 2,
            inserts: Vec::new(),
            input_routing: None,
            output_routing: None,
        }));
        let sanitized = sanitize_library(entries);
        assert_eq!(sanitized.len(), MAX_LIBRARY_TRACKS);
        assert_eq!(sanitized[0].name, "Vocals");
        assert_eq!(sanitized[0].audio_channels, 2);
        assert_eq!(sanitized[0].inserts.len(), MAX_LIBRARY_INSERTS);
        assert_eq!(sanitized[0].input_routing, None);
        assert_eq!(sanitized[0].output_routing.as_deref(), Some("Left"));
        assert_eq!(
            sanitized
                .iter()
                .filter(|entry| entry.name.eq_ignore_ascii_case("vocals"))
                .count(),
            1
        );

        assert_eq!(portable_output_route("FX:4"), None);
        assert_eq!(
            portable_output_route("Stereo Out").as_deref(),
            Some("Stereo Out")
        );
        assert_eq!(portable_input_route("No Bus").as_deref(), Some("No Bus"));
    }

    #[test]
    fn the_window_lists_saved_tracks() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.data.project.tracks[0].name = "Lead Guitar".to_owned();
        app.data.project.tracks[0].inserts.push(insert());
        app.data.project.tracks[0].output_routing = Some("Stereo Out".to_owned());
        app.save_selected_track_to_library().expect("saved");
        app.track_library_open = true;
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 760.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_track_library(&ctx),
            );
            output.textures_delta.clear();
            if frame == 0 {
                continue;
            }
            fn walk(shape: &egui::Shape, texts: &mut Vec<String>) {
                match shape {
                    egui::Shape::Text(text) => texts.push(text.galley.text().to_owned()),
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            walk(shape, texts);
                        }
                    }
                    _ => {}
                }
            }
            for shape in &output.shapes {
                walk(&shape.shape, &mut texts);
            }
        }
        let joined = texts.join("\n");
        assert!(joined.contains("Track Library"), "{joined}");
        assert!(joined.contains("Lead Guitar"), "{joined}");
        assert!(joined.contains("1 insert(s)"), "{joined}");
        assert!(joined.contains("out Stereo Out"), "{joined}");
    }
}
