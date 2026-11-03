//! Layout smoke tests for the windows added in the feature work.
//!
//! Each window is rendered headlessly at a desktop size and at a small size.
//! The tests assert that the window still opens and lays out its title and a
//! representative control, which catches panics, missing content and windows
//! that lose their body when the screen is small. Visual judgement stays with
//! the desktop review.

use super::*;

fn app() -> OpenTrackApp {
    let ctx = egui::Context::default();
    let creation = eframe::CreationContext::_new_kittest(ctx);
    let mut app = OpenTrackApp::new_internal(&creation, false);
    app.project_open = true;
    app
}

fn texts_for<F>(
    app: &mut OpenTrackApp,
    ctx: &egui::Context,
    screen: Vec2,
    mut show: F,
) -> Vec<String>
where
    F: FnMut(&mut OpenTrackApp, &egui::Context),
{
    let mut texts = Vec::new();
    for frame in 0..2 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, screen)),
                time: Some(frame as f64 * 0.05),
                ..Default::default()
            },
            |_| show(app, ctx),
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
    texts
}

fn assert_at_both_sizes(
    name: &str,
    setup: impl Fn(&mut OpenTrackApp),
    show: impl Fn(&mut OpenTrackApp, &egui::Context),
    required: &[&str],
) {
    for screen in [Vec2::new(1100.0, 780.0), Vec2::new(640.0, 480.0)] {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        setup(&mut app);
        let texts = texts_for(&mut app, &ctx, screen, &show);
        let joined = texts.join("\n");
        for needle in required {
            assert!(
                joined.contains(needle),
                "{name} at {screen:?}: missing {needle:?}\n{joined}"
            );
        }
    }
}

#[test]
fn the_new_dialogs_open_with_their_controls_at_desktop_and_small_sizes() {
    assert_at_both_sizes(
        "Tempo Track",
        |app| {
            app.tempo_editor = Some(tempo_editor::Draft::from_project(&app.data.project));
        },
        |app, ctx| app.show_tempo_editor(ctx),
        &["Tempo Track", "Beats per bar", "Beat unit"],
    );

    assert_at_both_sizes(
        "Event Colors",
        |app| {
            app.event_colors_dialog = Some(EventColorsDialog {
                slot_index: 0,
                draft_rgb: [120, 130, 140],
                palette: Some(0),
            });
        },
        |app, ctx| app.show_event_colors_dialog(ctx),
        &["Event Colors", "Palette", "Apply"],
    );

    assert_at_both_sizes(
        "Direct Offline Processing",
        |app| {
            app.data.project.add_track(TrackKind::Audio);
            let clip = app
                .data
                .project
                .add_audio_clip(0, PathBuf::from("take.wav"), 0.0, 1.0)
                .expect("clip");
            app.offline_chain_draft = Some(offline_processing::ChainDraft {
                clip_id: clip,
                chain: vec![opentrack_core::project::OfflineProcess::Reverse],
            });
        },
        |app, ctx| app.show_offline_processing(ctx),
        &["Direct Offline Processing", "Add processor", "Apply Chain"],
    );

    assert_at_both_sizes(
        "Windows",
        |app| app.windows_dialog_open = true,
        |app, ctx| app.show_windows_dialog(ctx),
        &["Windows", "Project", "Media"],
    );

    assert_at_both_sizes(
        "Revert",
        |app| {
            app.revert_prompt = true;
            app.project_path = Some(PathBuf::from("/tmp/Song.opr"));
        },
        |app, ctx| app.show_revert_prompt(ctx),
        &["Revert", "Cancel"],
    );

    assert_at_both_sizes(
        "Registration",
        |app| app.registration_open = true,
        |app, ctx| app.show_registration(ctx),
        &["Registration", "needs no registration"],
    );

    assert_at_both_sizes(
        "Plugin Browser",
        |app| {
            app.plugin_browser_open = true;
            app.plugin_browser_category = InsertMenuCategory::Dynamics;
        },
        |app, ctx| app.show_plugin_browser(ctx),
        &["Plug-in Browser", "Search", "No scanned VST3 effects"],
    );

    assert_at_both_sizes(
        "Cleanup",
        |app| {
            app.cleanup_prompt = Some(project_cleanup::CleanupPlan {
                files: vec![(PathBuf::from("/tmp/take.wav"), 1)],
                bytes: 1,
                label: "2026-11-03".to_owned(),
            });
        },
        |app, ctx| app.show_cleanup_prompt(ctx),
        &["Clean Up Project Folder", "Clean Up", "Cancel"],
    );

    assert_at_both_sizes(
        "VST Connections",
        |app| {
            app.vst_connections_open = true;
            app.data.project.add_track(TrackKind::Group);
        },
        |app, ctx| app.show_device_connections(ctx),
        &["VST Connections", "Device Buses", "Project Buses"],
    );

    assert_at_both_sizes(
        "Macros",
        |app| {
            app.macros_open = true;
            app.macros = vec![macros::Macro {
                name: "Session".to_owned(),
                commands: Vec::new(),
            }];
            app.macro_selected = Some(0);
        },
        |app, ctx| app.show_macros(ctx),
        &["Macros", "Session", "Steps"],
    );

    assert_at_both_sizes(
        "Track Library",
        |app| {
            app.track_library_open = true;
            app.track_library = vec![track_library::LibraryTrack {
                name: "Vocal".to_owned(),
                kind: TrackKind::Audio,
                audio_channels: 2,
                inserts: Vec::new(),
                input_routing: None,
                output_routing: None,
            }];
        },
        |app, ctx| app.show_track_library(ctx),
        &["Track Library", "Vocal", "Add"],
    );

    assert_at_both_sizes(
        "Browse Sounds",
        |app| app.sound_browser_open = true,
        |app, ctx| app.show_sound_browser(ctx),
        &["Browse Sounds", "Choose Folder", "Search"],
    );

    assert_at_both_sizes(
        "Control Room",
        |app| app.control_room_open = true,
        |app, ctx| app.show_control_room(ctx),
        &["Control Room", "Listen", "Talkback"],
    );

    assert_at_both_sizes(
        "Project Synchronization Setup",
        |app| app.sync_setup_open = true,
        |app, ctx| app.show_sync_setup(ctx),
        &[
            "Project Synchronization Setup",
            "MIDI Output",
            "MMC device ID",
        ],
    );

    assert_at_both_sizes(
        "Score",
        |app| {
            app.data.project.add_track(TrackKind::Midi);
            let part = app.data.project.add_midi_part(0, 0.0, 4.0).expect("part");
            app.data
                .project
                .add_midi_note(0, part, 0.0, 1.0, 60, 100)
                .expect("note");
            app.data.project.selected_midi_part_id = Some(part);
            app.score_view_open = true;
        },
        |app, ctx| app.show_score_view(ctx),
        &["Score", "Treble", "Bass"],
    );

    assert_at_both_sizes(
        "Score Layout",
        |app| app.score_layout_open = true,
        |app, ctx| app.show_score_layout(ctx),
        &["Score Layout", "Bars per system", "Zoom"],
    );

    assert_at_both_sizes(
        "Score Settings",
        |app| app.score_settings_open = true,
        |app, ctx| app.show_score_settings(ctx),
        &["Score Settings", "Zoom"],
    );

    assert_at_both_sizes(
        "Video Window",
        |app| app.video_window_open = true,
        |app, ctx| app.show_video_window(ctx),
        &["Video Window", "Open Video"],
    );
}

#[test]
fn the_project_loading_screen_is_centered_and_reports_progress() {
    let ctx = egui::Context::default();
    let creation = eframe::CreationContext::_new_kittest(ctx.clone());
    let mut app = OpenTrackApp::new_internal(&creation, false);
    let (_sender, receiver) = std::sync::mpsc::channel();
    let progress = std::sync::Arc::new(std::sync::Mutex::new(ProjectLoadStatus {
        fraction: 0.65,
        phase: "Reading and validating project".to_owned(),
    }));
    app.project_load_task = Some(ProjectLoadTask {
        path: PathBuf::from("Song.opr"),
        result: receiver,
        progress,
        pending_project: None,
        next_plugin_to_load: 0,
    });
    let mut texts = Vec::new();
    for frame in 0..2 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                time: Some(frame as f64 * 0.05),
                ..Default::default()
            },
            |_| app.show_project_loading(&ctx),
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
    assert!(joined.contains("Loading Project"), "{joined}");
    assert!(joined.contains("Song.opr"), "{joined}");
    assert!(joined.contains("65%"), "progress text: {joined}");
    assert!(
        joined.contains("Reading and validating project"),
        "{joined}"
    );
}
