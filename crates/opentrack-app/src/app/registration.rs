//! Help > Registration: an informational dialog.
//!
//! OpenTrack has no registration, account, licence key or online activation, so
//! this window says exactly that instead of showing a form or linking anywhere.

use super::*;

impl OpenTrackApp {
    pub(super) fn show_registration(&mut self, ctx: &egui::Context) {
        if !self.registration_open {
            return;
        }
        let mut open = true;
        let mut close = false;
        let response = egui::Window::new("Registration")
            .id(Id::new("registration-info"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label("OpenTrack needs no registration.");
                ui.label(
                    "There is no account, licence key, activation server or sign-in. Every \
                     feature in this build is available without one, and the application \
                     does not contact a server for registration.",
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "The application source is Apache-2.0. Third-party dependencies and bundled \
                         artwork keep their own licences; see Credits and Copyrights.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("registration-info"),
            &mut open,
            response.as_ref(),
        );
        self.registration_open = open && !close;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registration_dialog_states_that_none_is_needed() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.registration_open = true;
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_registration(&ctx),
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
        assert!(joined.contains("Registration"), "{joined}");
        assert!(joined.contains("needs no registration"), "{joined}");
        assert!(joined.contains("no account"), "{joined}");
        assert!(!joined.to_lowercase().contains("http"), "no fake link");
        assert!(app.registration_open);
    }
}
