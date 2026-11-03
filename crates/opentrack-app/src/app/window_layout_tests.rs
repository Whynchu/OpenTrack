//! Layout tests for the windows added in the feature work.
//!
//! Each window is rendered headlessly at a desktop size and at a small
//! (640x480) size. Besides checking that the title and a representative
//! control are present, the tests read the window's area rectangle from egui
//! memory and assert that it stays inside the screen, and that every painted
//! text and image rect stays inside the window. The loading screen and the
//! video image are additionally checked for centering. Visual judgement (DPI,
//! theme, exact spacing) stays with the desktop review.

use super::*;

fn new_app(ctx: &egui::Context) -> OpenTrackApp {
    let creation = eframe::CreationContext::_new_kittest(ctx.clone());
    let mut app = OpenTrackApp::new_internal(&creation, false);
    app.project_open = true;
    app
}

struct Rendered {
    texts: Vec<String>,
    text_rects: Vec<Rect>,
    image_rects: std::collections::BTreeMap<String, Rect>,
}

fn render_for<F>(app: &mut OpenTrackApp, ctx: &egui::Context, screen: Vec2, mut show: F) -> Rendered
where
    F: FnMut(&mut OpenTrackApp, &egui::Context),
{
    let mut rendered = Rendered {
        texts: Vec::new(),
        text_rects: Vec::new(),
        image_rects: std::collections::BTreeMap::new(),
    };
    let frames = 4;
    for frame in 0..frames {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, screen)),
                time: Some(frame as f64 * 0.05),
                ..Default::default()
            },
            |_| show(app, ctx),
        );
        output.textures_delta.clear();
        if frame + 1 < frames {
            continue;
        }
        // Only the painted (clipped) part of a shape counts: a scroll area lays
        // out rows beyond its viewport and clips them out of view.
        fn walk(shape: &egui::Shape, clip: Rect, rendered: &mut Rendered) {
            match shape {
                egui::Shape::Text(text) => {
                    rendered.texts.push(text.galley.text().to_owned());
                    rendered.text_rects.push(
                        text.galley
                            .rect
                            .translate(text.pos.to_vec2())
                            .intersect(clip),
                    );
                }
                egui::Shape::Mesh(mesh) => {
                    rendered
                        .image_rects
                        .entry(format!("{:?}", mesh.texture_id))
                        .or_insert(shape.visual_bounding_rect().intersect(clip));
                }
                egui::Shape::Rect(rect_shape) => {
                    if let Some(brush) = &rect_shape.brush {
                        rendered
                            .image_rects
                            .entry(format!("{:?}", brush.fill_texture_id))
                            .or_insert(rect_shape.rect.intersect(clip));
                    }
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        walk(shape, clip, rendered);
                    }
                }
                _ => {}
            }
        }
        for clipped in &output.shapes {
            walk(&clipped.shape, clipped.clip_rect, &mut rendered);
        }
    }
    rendered
}

fn assert_inside(name: &str, what: &str, rect: Rect, screen: Vec2) {
    const TOLERANCE: f32 = 1.0;
    assert!(
        rect.min.x >= -TOLERANCE
            && rect.min.y >= -TOLERANCE
            && rect.max.x <= screen.x + TOLERANCE
            && rect.max.y <= screen.y + TOLERANCE,
        "{name} at {screen:?}: {what} {rect:?} leaves the screen"
    );
}

fn assert_at_both_sizes(
    name: &str,
    window_id: &str,
    setup: impl Fn(&mut OpenTrackApp, &egui::Context),
    show: impl Fn(&mut OpenTrackApp, &egui::Context),
    required: &[&str],
) {
    for screen in [Vec2::new(1100.0, 780.0), Vec2::new(640.0, 480.0)] {
        let ctx = egui::Context::default();
        let mut app = new_app(&ctx);
        setup(&mut app, &ctx);
        let rendered = render_for(&mut app, &ctx, screen, &show);
        let joined = rendered.texts.join("\n");
        for needle in required {
            assert!(
                joined.contains(needle),
                "{name} at {screen:?}: missing {needle:?}\n{joined}"
            );
        }
        let window = ctx
            .memory(|memory| memory.area_rect(Id::new(window_id)))
            .unwrap_or_else(|| panic!("{name} at {screen:?}: no window area {window_id}"));
        assert_inside(name, "window", window, screen);
        for (index, rect) in rendered.text_rects.iter().enumerate() {
            assert!(
                window.expand(1.0).contains_rect(*rect),
                "{name} at {screen:?}: text {:?} at {rect:?} escapes the window {window:?}",
                rendered.texts.get(index).map(String::as_str).unwrap_or("")
            );
        }
    }
}

fn setup_macros(app: &mut OpenTrackApp, _: &egui::Context) {
    app.macros_open = true;
    app.macros = vec![macros::Macro {
        name: "Session".to_owned(),
        commands: Vec::new(),
    }];
    app.macro_selected = Some(0);
}

#[test]
fn the_new_dialogs_open_with_their_controls_at_desktop_and_small_sizes() {
    assert_at_both_sizes(
        "Tempo Track",
        "tempo-track-editor",
        |app, _| {
            app.tempo_editor = Some(tempo_editor::Draft::from_project(&app.data.project));
        },
        |app, ctx| app.show_tempo_editor(ctx),
        &["Tempo Track", "Beats per bar", "Beat unit"],
    );

    assert_at_both_sizes(
        "Event Colors",
        "project-event-colors-dialog",
        |app, _| {
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
        "offline-processing",
        |app, _| {
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
        "window-list",
        |app, _| app.windows_dialog_open = true,
        |app, ctx| app.show_windows_dialog(ctx),
        &["Windows", "Project", "Media"],
    );

    assert_at_both_sizes(
        "Revert",
        "revert-prompt",
        |app, _| {
            app.revert_prompt = true;
            app.project_path = Some(PathBuf::from("/tmp/Song.opr"));
        },
        |app, ctx| app.show_revert_prompt(ctx),
        &["Revert", "Cancel"],
    );

    assert_at_both_sizes(
        "Registration",
        "registration-info",
        |app, _| app.registration_open = true,
        |app, ctx| app.show_registration(ctx),
        &["Registration", "needs no registration"],
    );

    assert_at_both_sizes(
        "Plugin Browser",
        "plugin-browser",
        |app, _| {
            app.plugin_browser_open = true;
            app.plugin_browser_category = InsertMenuCategory::Dynamics;
        },
        |app, ctx| app.show_plugin_browser(ctx),
        &["Plug-in Browser", "Search", "No scanned VST3 effects"],
    );

    assert_at_both_sizes(
        "Cleanup",
        "cleanup-prompt",
        |app, _| {
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
        "vst-connections",
        |app, _| {
            app.vst_connections_open = true;
            app.data.project.add_track(TrackKind::Group);
        },
        |app, ctx| app.show_device_connections(ctx),
        &["VST Connections", "Device Buses", "Project Buses"],
    );

    assert_at_both_sizes(
        "Macros",
        "command-macros",
        setup_macros,
        |app, ctx| app.show_macros(ctx),
        &["Macros", "Session", "Steps"],
    );

    assert_at_both_sizes(
        "Track Library",
        "track-library",
        |app, _| {
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
        "sound-browser",
        |app, _| app.sound_browser_open = true,
        |app, ctx| app.show_sound_browser(ctx),
        &["Browse Sounds", "Choose Folder", "Search"],
    );

    assert_at_both_sizes(
        "Control Room",
        "control-room",
        |app, _| app.control_room_open = true,
        |app, ctx| app.show_control_room(ctx),
        &["Control Room", "Listen", "Talkback"],
    );

    assert_at_both_sizes(
        "Project Synchronization Setup",
        "project-sync-setup",
        |app, _| app.sync_setup_open = true,
        |app, ctx| app.show_sync_setup(ctx),
        &[
            "Project Synchronization Setup",
            "MIDI Output",
            "MMC device ID",
        ],
    );

    assert_at_both_sizes(
        "Score",
        "score-view",
        |app, _| {
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
        "score-layout",
        |app, _| app.score_layout_open = true,
        |app, ctx| app.show_score_layout(ctx),
        &["Score Layout", "Bars per system", "Zoom"],
    );

    assert_at_both_sizes(
        "Score Settings",
        "score-settings",
        |app, _| app.score_settings_open = true,
        |app, ctx| app.show_score_settings(ctx),
        &["Score Settings", "Zoom"],
    );

    assert_at_both_sizes(
        "Video Window",
        "video-window",
        |app, _| app.video_window_open = true,
        |app, ctx| app.show_video_window(ctx),
        &["Video Window", "Open Video"],
    );
}

#[test]
fn the_project_loading_screen_is_centered_and_reports_progress() {
    for screen in [
        Vec2::new(1100.0, 780.0),
        Vec2::new(640.0, 480.0),
        Vec2::new(1920.0, 1080.0),
    ] {
        let ctx = egui::Context::default();
        let mut app = new_app(&ctx);
        let (_sender, receiver) = std::sync::mpsc::channel();
        app.project_load_task = Some(ProjectLoadTask {
            path: PathBuf::from("Song.opr"),
            result: receiver,
            progress: std::sync::Arc::new(std::sync::Mutex::new(ProjectLoadStatus {
                fraction: 0.65,
                phase: "Reading and validating project".to_owned(),
            })),
            pending_project: None,
            next_plugin_to_load: 0,
        });
        let rendered = render_for(&mut app, &ctx, screen, |app, ctx| {
            app.show_project_loading(ctx)
        });
        let joined = rendered.texts.join("\n");
        assert!(joined.contains("Loading Project"), "{joined}");
        assert!(joined.contains("Song.opr"), "{joined}");
        assert!(joined.contains("65%"), "progress text: {joined}");
        assert!(
            joined.contains("Reading and validating project"),
            "{joined}"
        );

        let window = ctx
            .memory(|memory| memory.area_rect(Id::new(Some("Loading Project"))))
            .expect("loading window");
        assert_inside("Loading Project", "window", window, screen);
        let offset = window.center() - Pos2::new(screen.x * 0.5, screen.y * 0.5);
        assert!(
            offset.length() <= 1.0,
            "Loading Project at {screen:?}: center {:?} is {offset:?} from the screen center",
            window.center()
        );
        // The title, bar and phase are laid out in one centered column.
        let body_rects = rendered
            .texts
            .iter()
            .zip(&rendered.text_rects)
            .filter(|(text, _)| text.as_str() != "Loading Project")
            .map(|(_, rect)| *rect)
            .collect::<Vec<_>>();
        let column_left = body_rects
            .iter()
            .map(|rect| rect.min.x)
            .fold(f32::INFINITY, f32::min);
        let column_right = body_rects
            .iter()
            .map(|rect| rect.max.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let column_center = (column_left + column_right) * 0.5;
        assert!(
            (column_center - window.center().x).abs() <= 2.0,
            "Loading Project at {screen:?}: content center {column_center} vs window center {}",
            window.center().x
        );
    }
}

#[test]
fn the_video_image_is_centered_and_fitted_in_its_window() {
    let screen = Vec2::new(640.0, 480.0);
    let ctx = egui::Context::default();
    let mut app = new_app(&ctx);
    app.video_window_open = true;
    let texture = ctx.load_texture(
        "video-frame",
        egui::ColorImage::filled([1280, 720], egui::Color32::BLACK),
        egui::TextureOptions::LINEAR,
    );
    let texture_id = texture.id();
    app.video_texture = Some(texture);
    let rendered = render_for(&mut app, &ctx, screen, |app, ctx| {
        app.show_video_window(ctx)
    });
    let window = ctx
        .memory(|memory| memory.area_rect(Id::new("video-window")))
        .expect("video window");
    assert_inside("Video Window", "window", window, screen);
    let image = rendered
        .image_rects
        .get(&format!("{texture_id:?}"))
        .copied()
        .unwrap_or_else(|| {
            panic!(
                "video frame image was painted; meshes: {:?}",
                rendered.image_rects
            )
        });
    assert!(
        (image.center().x - window.center().x).abs() <= 2.0,
        "video image center {} vs window center {}",
        image.center().x,
        window.center().x
    );
    assert!(
        window.expand(1.0).contains_rect(image),
        "video image {image:?} escapes the window {window:?}"
    );
    assert!(
        image.width() <= window.width() && image.height() <= window.height(),
        "video image {image:?} is larger than its window {window:?}"
    );
}
