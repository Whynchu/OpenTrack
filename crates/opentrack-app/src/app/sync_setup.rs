//! Project Synchronization Setup and the bounded MIDI Machine Control master.
//!
//! OpenTrack can send MMC transport commands over a Windows MIDI output. It has
//! no clock input, so external sync (following another device's clock) is not
//! offered: the dialog says so instead of pretending.

use super::*;
use opentrack_audio::{MMC_PLAY, MMC_STOP, MidiOutputStream, mmc_locate, mmc_message};

impl OpenTrackApp {
    pub(super) fn open_sync_setup(&mut self) {
        self.sync_setup_open = true;
        self.refresh_midi_outputs();
    }

    fn refresh_midi_outputs(&mut self) {
        match MidiOutputStream::available_devices() {
            Ok(devices) => {
                let count = devices.len();
                self.midi_output_devices = devices;
                if let Some(selected) = self.selected_midi_output.clone()
                    && !self
                        .midi_output_devices
                        .iter()
                        .any(|device| device.id == selected)
                {
                    self.selected_midi_output = None;
                    self.midi_output = None;
                }
                self.status_message = format!("{count} MIDI output port(s) available");
            }
            Err(error) => {
                self.status_message = format!("Could not list MIDI outputs: {error}");
            }
        }
    }

    fn select_midi_output(&mut self, id: Option<String>) {
        self.selected_midi_output = id;
        self.midi_output = None;
    }

    /// Sends one MMC message when MMC is enabled, opening the port on demand.
    fn send_mmc(&mut self, message: Vec<u8>) {
        if !self.mmc_enabled {
            return;
        }
        let Some(selected) = self.selected_midi_output.clone() else {
            self.status_message =
                "Choose a MIDI output in Project Synchronization Setup".to_owned();
            return;
        };
        if self.midi_output.is_none() {
            match MidiOutputStream::open(&selected) {
                Ok(stream) => {
                    self.status_message = format!("MMC output: {}", stream.device_name);
                    self.midi_output = Some(stream);
                }
                Err(error) => {
                    self.status_message = format!("MIDI output: {error}");
                    return;
                }
            }
        }
        if let Some(stream) = &self.midi_output
            && let Err(error) = stream.send(&message)
        {
            self.status_message = format!("MIDI output: {error}");
            self.midi_output = None;
        }
    }

    /// Sends the MMC play/stop command for a transport state change.
    pub(super) fn send_mmc_transport(&mut self, playing: bool) {
        if !self.mmc_enabled {
            return;
        }
        let message = if playing {
            mmc_message(self.mmc_device_id, MMC_PLAY)
        } else {
            mmc_message(self.mmc_device_id, MMC_STOP)
        };
        self.send_mmc(message);
    }

    /// Sends an MMC locate for a stopped transport position.
    pub(super) fn send_mmc_locate(&mut self, beat: f64) {
        if !self.mmc_enabled || self.data.transport.playing {
            return;
        }
        let seconds = beat.max(0.0) * 60.0 / self.data.project.tempo_bpm.max(1.0);
        if let Some(message) = mmc_locate(self.mmc_device_id, seconds, 25) {
            self.send_mmc(message);
        }
    }

    pub(super) fn show_sync_setup(&mut self, ctx: &egui::Context) {
        if !self.sync_setup_open {
            return;
        }
        let devices = self.midi_output_devices.clone();
        let mut open = true;
        let mut chosen: Option<Option<String>> = None;
        let mut refresh = false;
        let mut enabled = self.mmc_enabled;
        let mut device_id = self.mmc_device_id;
        let selected_label = self
            .selected_midi_output
            .as_ref()
            .and_then(|id| devices.iter().find(|device| &device.id == id))
            .map(|device| device.name.clone())
            .unwrap_or_else(|| "No MIDI output".to_owned());
        let response = egui::Window::new("Project Synchronization Setup")
            .id(Id::new("project-sync-setup"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(440.0);
                ui.label(
                    egui::RichText::new(
                        "MIDI Machine Control master: OpenTrack sends MMC play, stop and locate over the chosen output. External clock sync needs a clock input and stays unavailable.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.separator();
                egui::Grid::new("sync-setup-grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("MIDI Output");
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("midi-output-device")
                                .width(240.0)
                                .selected_text(selected_label.clone())
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            self.selected_midi_output.is_none(),
                                            "No MIDI output",
                                        )
                                        .clicked()
                                    {
                                        chosen = Some(None);
                                    }
                                    for device in &devices {
                                        if ui
                                            .selectable_label(
                                                self.selected_midi_output.as_deref()
                                                    == Some(device.id.as_str()),
                                                &device.name,
                                            )
                                            .clicked()
                                        {
                                            chosen = Some(Some(device.id.clone()));
                                        }
                                    }
                                });
                            if ui.button("Refresh").clicked() {
                                refresh = true;
                            }
                        });
                        ui.end_row();
                        ui.label("MMC device ID");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut device_id).range(0..=127));
                            ui.label(
                                egui::RichText::new("127 addresses every device")
                                    .small()
                                    .color(theme::palette(ui).muted),
                            );
                        });
                        ui.end_row();
                    });
                if devices.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No MIDI output ports are available on this platform or none are installed.",
                        )
                        .small()
                        .color(theme::palette(ui).muted),
                    );
                }
                if ui
                    .checkbox(&mut enabled, "Send MMC on play, stop and locate")
                    .changed()
                {
                    // The checkbox drives the persisted preference directly.
                }
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "MMC master is one-way: OpenTrack does not follow an external transport or clock. Use External Sync therefore stays unavailable.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("project-sync-setup"),
            &mut open,
            response.as_ref(),
        );
        if let Some(choice) = chosen {
            self.select_midi_output(choice);
        }
        if refresh {
            self.refresh_midi_outputs();
        }
        self.mmc_enabled = enabled;
        self.mmc_device_id = device_id.min(127);
        self.sync_setup_open = open;
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
    fn mmc_is_sent_only_when_enabled_and_a_port_is_chosen() {
        let mut app = app();
        // Disabled: nothing is sent and no message is written.
        let before = app.status_message.clone();
        app.send_mmc_transport(true);
        assert_eq!(app.status_message, before);

        // Enabled without a port: reports instead of sending.
        app.mmc_enabled = true;
        app.send_mmc_transport(true);
        assert!(app.status_message.contains("Choose a MIDI output"));

        // A stale port id fails to open and reports; no crash, no silent send.
        app.selected_midi_output = Some("missing-port".to_owned());
        app.send_mmc_transport(false);
        assert!(
            app.status_message.contains("MIDI output")
                || app.status_message.contains("no longer available"),
            "{}",
            app.status_message
        );

        // Locate is suppressed while the transport plays.
        app.data.transport.playing = true;
        let before = app.status_message.clone();
        app.send_mmc_locate(4.0);
        assert_eq!(app.status_message, before);
    }

    #[test]
    fn the_setup_dialog_draws_its_controls() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.sync_setup_open = true;
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_sync_setup(&ctx),
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
        assert!(joined.contains("Project Synchronization Setup"), "{joined}");
        assert!(joined.contains("MIDI Output"), "{joined}");
        assert!(joined.contains("MMC device ID"), "{joined}");
        assert!(joined.contains("Send MMC"), "{joined}");
        assert!(joined.contains("External"), "{joined}");
    }
}
