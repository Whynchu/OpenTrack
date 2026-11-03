//! Devices > Video Window: the decoded video frame at the transport position.
//!
//! The FFmpeg decoder is an optional build feature (`video-ffmpeg`). When it is
//! absent the window explains that instead of showing a blank surface; when it is
//! present, opening a file decodes the frame nearest the playhead and refreshes
//! it as the transport moves.

use super::*;
use opentrack_video::{VideoDecoder, backend_available};

impl OpenTrackApp {
    pub(super) fn open_video_window(&mut self) {
        if !backend_available() {
            self.status_message =
                "This build has no FFmpeg video decoder; build with the video-ffmpeg feature"
                    .to_owned();
            return;
        }
        self.video_window_open = true;
    }

    fn choose_video_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "m4v", "mov", "mkv", "avi", "webm", "y4m"])
            .set_title("Open Video")
            .pick_file()
        else {
            return;
        };
        self.open_video_file(path);
    }

    pub(super) fn open_video_file(&mut self, path: PathBuf) {
        match VideoDecoder::open(&path) {
            Ok(decoder) => {
                self.video_path = Some(path.clone());
                self.video_decoder = Some(decoder);
                self.video_texture = None;
                self.video_last_frame = None;
                self.video_error = None;
                self.status_message = format!(
                    "Video: {}",
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string())
                );
            }
            Err(error) => {
                self.video_error = Some(error.clone());
                self.status_message = format!("Could not open the video: {error}");
            }
        }
    }

    fn update_video_texture(&mut self, ctx: &egui::Context) {
        if self.video_decoder.is_none() {
            return;
        }
        let seconds =
            self.data.transport.position_beats * 60.0 / self.data.project.tempo_bpm.max(1.0);
        if self.video_texture.is_some()
            && self
                .video_last_frame
                .is_some_and(|last| (last - seconds).abs() < 1.0 / 30.0)
        {
            return;
        }
        let decoded = match self.video_decoder.as_mut() {
            Some(decoder) => decoder.frame_at(seconds),
            None => return,
        };
        match decoded {
            Ok(frame) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width as usize, frame.height as usize],
                    &frame.rgba,
                );
                self.video_texture =
                    Some(ctx.load_texture("video-frame", image, egui::TextureOptions::LINEAR));
                self.video_last_frame = Some(frame.timestamp_seconds);
                self.video_error = None;
            }
            Err(error) => {
                self.video_error = Some(error);
            }
        }
    }

    pub(super) fn show_video_window(&mut self, ctx: &egui::Context) {
        if !self.video_window_open {
            return;
        }
        self.update_video_texture(ctx);
        let mut open = true;
        let mut choose = false;
        let name = self
            .video_path
            .as_ref()
            .map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string())
            })
            .unwrap_or_else(|| "No video open".to_owned());
        let response = egui::Window::new("Video Window")
            .id(Id::new("video-window"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(660.0, 420.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(480.0);
                ui.horizontal(|ui| {
                    if ui.button("Open Video...").clicked() {
                        choose = true;
                    }
                    ui.label(name.clone());
                    ui.label(
                        egui::RichText::new(format!(
                            "follows the playhead at {:.2} s",
                            self.data.transport.position_beats * 60.0
                                / self.data.project.tempo_bpm.max(1.0)
                        ))
                        .small()
                        .color(theme::palette(ui).muted),
                    );
                });
                ui.separator();
                if !backend_available() {
                    ui.label("This build has no FFmpeg video decoder. Rebuild with the video-ffmpeg feature and an FFmpeg development install.");
                }
                if let Some(error) = &self.video_error {
                    ui.colored_label(theme::RECORD, error);
                }
                match &self.video_texture {
                    Some(texture) => {
                        let size = texture.size_vec2();
                        let max = Vec2::new(
                            ui.available_width().max(64.0),
                            (ui.available_height() - 12.0).max(64.0),
                        );
                        let scale = (max.x / size.x).min(max.y / size.y).min(1.0);
                        ui.vertical_centered(|ui| {
                            ui.image(egui::load::SizedTexture::new(
                                texture.id(),
                                size * scale,
                            ));
                        });
                    }
                    None => {
                        ui.label(
                            egui::RichText::new("Open a video to show its frame at the playhead.")
                                .small()
                                .color(theme::palette(ui).muted),
                        );
                    }
                }
            });
        dismiss_window_on_outside_click(ctx, Id::new("video-window"), &mut open, response.as_ref());
        if choose {
            self.choose_video_file();
        }
        self.video_window_open = open;
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
        app
    }

    #[test]
    fn the_window_reports_a_build_without_the_decoder() {
        let mut app = app();
        app.open_video_window();
        if backend_available() {
            assert!(app.video_window_open);
        } else {
            assert!(!app.video_window_open);
            assert!(app.status_message.contains("no FFmpeg video decoder"));
        }

        // The window itself always explains the backend state.
        app.video_window_open = true;
        let ctx = egui::Context::default();
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_video_window(&ctx),
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
        assert!(joined.contains("Video Window"), "{joined}");
        assert!(joined.contains("Open Video"), "{joined}");
        if !backend_available() {
            assert!(joined.contains("no FFmpeg video decoder"), "{joined}");
        }
    }

    #[cfg(feature = "video-ffmpeg")]
    #[test]
    fn a_video_opens_and_decodes_at_the_playhead() {
        let root =
            std::env::temp_dir().join(format!("opentrack-video-window-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let path = root.join("clip.y4m");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\n");
        for _ in 0..2 {
            bytes.extend_from_slice(b"FRAME\n");
            bytes.extend_from_slice(&[76, 76, 76, 76, 85, 255]);
        }
        std::fs::write(&path, bytes).expect("y4m");

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.video_window_open = true;
        app.open_video_file(path.clone());
        assert!(app.video_error.is_none(), "{:?}", app.video_error);
        assert!(app.video_decoder.is_some());
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| app.show_video_window(&ctx));
        output.textures_delta.clear();
        assert!(app.video_texture.is_some(), "{:?}", app.video_error);
        let _ = std::fs::remove_dir_all(root);
    }
}
