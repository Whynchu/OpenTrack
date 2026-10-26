//! Compact configuration views based on REF-A14-01/02/03.
use super::*;

fn compact(ui: &mut egui::Ui) {
    ui.style_mut().override_font_id = Some(FontId::proportional(11.0));
    ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
    ui.spacing_mut().interact_size.y = 19.0;
    ui.spacing_mut().button_padding = Vec2::new(5.0, 2.0);
}

impl OpenTrackApp {
    pub(super) fn show_audio_device_setup(&mut self, ctx: &egui::Context) -> Option<Rect> {
        if !self.device_setup_open {
            return None;
        }
        let mut open = true;
        let response = egui::Window::new("Device Setup")
            .open(&mut open)
            .default_size(Vec2::new(638.0, 295.0))
            .min_size(Vec2::new(638.0, 270.0))
            .max_height(330.0)
            .show(ctx, |ui| {
                compact(ui);
                let driver_name = self
                    .audio_engine
                    .as_ref()
                    .and_then(|e| e.asio_info.as_ref())
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| "WASAPI".into());
                let height = (ui.available_height() - 35.0).clamp(195.0, 230.0);
                ui.horizontal_top(|ui| {
                    egui::Frame::group(ui.style())
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            // Frame inherits the surrounding horizontal layout; the
                            // navigation tree must explicitly be vertical for indent().
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                ui.set_width(254.0);
                                ui.set_min_height(height);
                                ui.label("Devices");
                                ui.separator();
                                ui.label("▾  MIDI");
                                ui.indent("audio-setup-midi-tree", |ui| {
                                    ui.selectable_value(
                                        &mut self.device_setup_page,
                                        2,
                                        "MIDI Port Setup",
                                    );
                                });
                                ui.add_space(10.0);
                                ui.selectable_value(
                                    &mut self.device_setup_page,
                                    0,
                                    "▾  VST Audio System",
                                );
                                ui.indent("audio-setup-driver-tree", |ui| {
                                    ui.selectable_value(
                                        &mut self.device_setup_page,
                                        1,
                                        driver_name,
                                    );
                                });
                            });
                        });
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 3.0).max(320.0));
                        egui::ScrollArea::vertical()
                            .id_salt("audio-setup-page-scroll")
                            .max_height(height)
                            .show(ui, |ui| {
                                ui.set_min_height(height - 5.0);
                                match self.device_setup_page {
                                    0 => self.audio_system_page(ui),
                                    1 => self.audio_driver_page(ui),
                                    _ => self.audio_midi_page(ui),
                                }
                            });
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Refresh").clicked() {
                        self.refresh_audio_input_devices();
                    }
                    if self.project_open && ui.button("Project Setup...").clicked() {
                        self.project_setup_open = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized([76.0, 21.0], egui::Button::new("OK"))
                            .clicked()
                        {
                            self.device_setup_open = false;
                        }
                    });
                });
            });
        self.device_setup_open &= open;
        if !self.device_setup_open {
            self.set_audio_input_test(false);
        }
        response.map(|window| window.response.rect)
    }

    fn audio_system_page(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("VST Audio System").size(16.0));
        });
        ui.add_space(5.0);
        let active_asio = self
            .selected_output_device
            .as_deref()
            .is_some_and(|id| id.starts_with("asio:"));
        let mut driver = if active_asio {
            self.selected_output_device.clone()
        } else {
            None
        };
        let label = self
            .audio_engine
            .as_ref()
            .and_then(|e| e.asio_info.as_ref())
            .map(|i| i.name.as_str())
            .or_else(|| {
                self.audio_output_devices
                    .iter()
                    .find(|d| Some(&d.id) == driver.as_ref())
                    .map(|d| d.name.as_str())
            })
            .unwrap_or("WASAPI");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("audio-system-driver")
                .width(206.0)
                .selected_text(label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut driver, None, "WASAPI");
                    for device in self
                        .audio_output_devices
                        .iter()
                        .filter(|d| d.host_name == "ASIO")
                    {
                        ui.selectable_value(&mut driver, Some(device.id.clone()), &device.name);
                    }
                });
            ui.label("Audio Driver");
        });
        let old_driver = if active_asio {
            self.selected_output_device.clone()
        } else {
            None
        };
        if driver != old_driver {
            self.apply_audio_output_settings(driver, None, None, self.audio_device_mode);
        }
        if !active_asio {
            let mut mode = self.audio_device_mode;
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("audio-system-wasapi-mode")
                    .width(206.0)
                    .selected_text(mode_label(mode))
                    .show_ui(ui, |ui| {
                        for value in [
                            AudioDeviceMode::SharedLowLatency,
                            AudioDeviceMode::SharedCompatible,
                            AudioDeviceMode::Exclusive,
                        ] {
                            ui.selectable_value(&mut mode, value, mode_label(value));
                        }
                    });
                ui.label("Mode");
            });
            if mode != self.audio_device_mode {
                self.apply_audio_output_settings(
                    self.selected_output_device.clone(),
                    None,
                    None,
                    mode,
                );
            }
        }
        ui.add_space(8.0);
        if let Some(engine) = self.audio_engine.as_ref() {
            egui::Grid::new("audio-system-latency")
                .spacing([24.0, 7.0])
                .show(ui, |ui| {
                    if let Some(info) = engine.asio_info.as_ref() {
                        ui.label("Input Latency:");
                        ui.label(format!(
                            "{:.3} ms",
                            f64::from(info.input_latency_frames) * 1000.0
                                / f64::from(engine.sample_rate)
                        ));
                        ui.end_row();
                        ui.label("Output Latency:");
                        ui.label(format!(
                            "{:.3} ms",
                            f64::from(info.output_latency_frames) * 1000.0
                                / f64::from(engine.sample_rate)
                        ));
                        ui.end_row();
                    } else {
                        ui.label("Input Period:");
                        ui.label(
                            self.audio_input_probe
                                .as_ref()
                                .map(|p| format!("{} frames", p.input_period_frames))
                                .unwrap_or_else(|| "Inactive".into()),
                        );
                        ui.end_row();
                        ui.label("Output Period:");
                        ui.label(format!(
                            "{} frames ({:.2} ms)",
                            engine.output_period_frames,
                            f64::from(engine.output_period_frames) * 1000.0
                                / f64::from(engine.sample_rate)
                        ));
                        ui.end_row();
                    }
                    ui.label("Sample Rate:");
                    ui.label(format!("{:.1} Hz", engine.sample_rate));
                    ui.end_row();
                });
        } else {
            ui.colored_label(theme::WARNING, "Audio is released");
        }
        ui.add_space(30.0);
        ui.strong("Advanced Options");
        ui.separator();
        ui.checkbox(
            &mut self.release_exclusive_audio_when_idle,
            "Release exclusive audio when no project is open",
        );
        let mut monitor = self.input_monitoring;
        if ui
            .checkbox(&mut monitor, "Monitor armed tracks while recording")
            .changed()
        {
            self.set_input_monitoring(monitor);
        }
        ui.add_space(12.0);
        if self.audio_engine.is_some() {
            if ui.button("Release Audio").clicked() {
                self.release_audio();
            }
        } else if ui.button("Open Audio").clicked() {
            self.reopen_audio();
        }
        if !self.audio_device_diagnostics.is_empty() {
            ui.add_space(6.0);
            for line in &self.audio_device_diagnostics {
                ui.colored_label(theme::WARNING, line);
            }
        }
    }

    fn audio_driver_page(&mut self, ui: &mut egui::Ui) {
        let asio = self.audio_engine.as_ref().and_then(|e| e.asio_info.clone());
        if asio.is_none()
            && self
                .selected_output_device
                .as_deref()
                .is_some_and(|id| id.starts_with("asio:"))
        {
            ui.label("The selected ASIO driver is not open.");
            ui.label(&self.status_message);
            if ui.button("Open Audio").clicked() {
                self.reopen_audio();
            }
            return;
        }
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(asio.as_ref().map(|i| i.name.as_str()).unwrap_or("WASAPI"))
                    .size(16.0),
            );
        });
        ui.add_space(8.0);
        if let Some(info) = asio {
            if ui
                .add_enabled(!self.is_recording(), egui::Button::new("Control Panel"))
                .clicked()
            {
                if let Some(id) = self.selected_output_device.clone() {
                    if !self.is_recording() {
                        self.release_audio();
                        let result = opentrack_audio::open_asio_control_panel(id.clone());
                        self.apply_audio_output_settings(
                            Some(id),
                            None,
                            None,
                            self.audio_device_mode,
                        );
                        if let Err(error) = result {
                            self.status_message = error;
                        }
                    }
                }
            }
            self.audio_rate_buffer_fields(ui, true);
            ui.add_space(12.0);
            egui::Grid::new("asio-driver-ports")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Port System Name");
                    ui.strong("I/O");
                    ui.strong("Visible");
                    ui.end_row();
                    for (ports, direction) in
                        [(&info.input_ports, "Input"), (&info.output_ports, "Output")]
                    {
                        for name in ports {
                            ui.label(name);
                            ui.label(direction);
                            ui.label("Yes");
                            ui.end_row();
                        }
                    }
                });
        } else {
            ui.label("Capture device:");
            let mut input = self.selected_input_device.clone();
            let input_name = self
                .resolved_input_device_name
                .clone()
                .unwrap_or_else(|| "Unavailable".into());
            ui.add_enabled_ui(!self.is_recording(), |ui| {
                egui::ComboBox::from_id_salt("driver-page-input")
                    .width(300.0)
                    .selected_text(input_name)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut input, None, "System default input");
                        for device in self
                            .audio_input_devices
                            .iter()
                            .filter(|d| !d.id.starts_with("asio:"))
                        {
                            ui.selectable_value(&mut input, Some(device.id.clone()), &device.name);
                        }
                    });
            });
            if input != self.selected_input_device && !self.is_recording() {
                self.audio_input_probe = None;
                self.selected_input_device = input;
                self.refresh_audio_input_format();
                self.update_input_capture_state();
            }
            ui.add_space(6.0);
            ui.label("Playback device:");
            let mut output = self.selected_output_device.clone();
            let name = self
                .audio_engine
                .as_ref()
                .map(|e| e.device_name.clone())
                .unwrap_or_else(|| "Unavailable".into());
            egui::ComboBox::from_id_salt("driver-page-output")
                .width(300.0)
                .selected_text(name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut output, None, "System default output");
                    for device in self
                        .audio_output_devices
                        .iter()
                        .filter(|d| d.host_name != "ASIO")
                    {
                        ui.selectable_value(&mut output, Some(device.id.clone()), &device.name);
                    }
                });
            if output != self.selected_output_device {
                self.apply_audio_output_settings(output, None, None, self.audio_device_mode);
            }
            self.audio_rate_buffer_fields(ui, false);
        }
        ui.add_space(12.0);
        ui.strong("Input Levels");
        if ui
            .button(if self.device_test_active {
                "Stop Input Test"
            } else {
                "Start Input Test"
            })
            .clicked()
        {
            self.set_audio_input_test(!self.device_test_active);
        }
        for (index, peak) in self
            .data
            .input_channel_meter_levels
            .iter()
            .copied()
            .enumerate()
        {
            ui.horizontal(|ui| {
                ui.label(format!("{}", index + 1));
                ui.add(egui::ProgressBar::new(meter_fill_fraction(peak)).desired_width(210.0));
                ui.label(if peak > 0.0 {
                    format!("{:.1} dB", 20.0 * peak.log10())
                } else {
                    "−∞ dB".into()
                });
            });
        }
        if let Some(error) = &self.audio_input_format_error {
            ui.colored_label(theme::WARNING, error);
        }
        if let Some(status) = &self.audio_input_probe_status {
            ui.label(status);
        }
    }

    fn audio_rate_buffer_fields(&mut self, ui: &mut egui::Ui, asio: bool) {
        let recording = self.is_recording();
        let Some(engine) = self.audio_engine.as_ref() else {
            return;
        };
        let actual_rate = engine.sample_rate;
        let actual_frames = engine.output_period_frames;
        let choices = if asio {
            self.audio_output_buffer_choices.clone()
        } else if self.audio_device_mode == AudioDeviceMode::SharedLowLatency {
            engine.output_period_choices.clone()
        } else {
            self.audio_output_buffer_choices.clone()
        };
        let mut rate = self.selected_output_sample_rate;
        let mut frames = self.selected_output_buffer_frames;
        ui.add_space(8.0);
        ui.add_enabled_ui(!recording, |ui| {
            egui::Grid::new("driver-rate-buffer-grid").show(ui, |ui| {
                ui.label("Sample Rate:");
                ui.add_enabled_ui(
                    asio || self.audio_device_mode == AudioDeviceMode::Exclusive,
                    |ui| {
                        egui::ComboBox::from_id_salt("driver-sample-rate")
                            .width(180.0)
                            .selected_text(format!("{actual_rate} Hz"))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut rate, None, "Driver default");
                                for r in &self.audio_output_sample_rates {
                                    ui.selectable_value(&mut rate, Some(*r), format!("{r} Hz"));
                                }
                            });
                    },
                );
                ui.end_row();
                ui.label("Buffer Size:");
                ui.add_enabled_ui(!choices.is_empty(), |ui| {
                    egui::ComboBox::from_id_salt("driver-buffer-size")
                        .width(180.0)
                        .selected_text(format!("{actual_frames} frames"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut frames, None, "Driver default");
                            for value in &choices {
                                ui.selectable_value(
                                    &mut frames,
                                    Some(*value),
                                    format!(
                                        "{value} frames ({:.2} ms)",
                                        f64::from(*value) * 1000.0 / f64::from(actual_rate)
                                    ),
                                );
                            }
                        });
                });
                ui.end_row();
            });
        });
        if rate != self.selected_output_sample_rate || frames != self.selected_output_buffer_frames
        {
            self.apply_audio_output_settings(
                self.selected_output_device.clone(),
                rate,
                frames,
                self.audio_device_mode,
            );
        }
    }

    fn audio_midi_page(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("MIDI Port Setup").size(16.0));
        });
        let mut selected = self.selected_midi_input_device.clone();
        let name = self
            .midi_input_devices
            .iter()
            .find(|d| Some(&d.id) == selected.as_ref())
            .map(|d| d.name.as_str())
            .unwrap_or("No MIDI input");
        egui::ComboBox::from_id_salt("setup-midi-input")
            .width(300.0)
            .selected_text(name)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, None, "No MIDI input");
                for device in &self.midi_input_devices {
                    ui.selectable_value(&mut selected, Some(device.id.clone()), &device.name);
                }
            });
        if selected != self.selected_midi_input_device {
            self.select_midi_input_device(selected);
        }
        if ui.button("Refresh MIDI Ports").clicked() {
            self.refresh_midi_input_devices();
        }
    }

    pub(super) fn show_project_setup(&mut self, ctx: &egui::Context) {
        if !self.project_setup_open {
            return;
        }
        let mut open = true;
        let mut requested = None;
        let recording = self.is_recording();
        egui::Window::new("Project Setup")
            .open(&mut open)
            .resizable(false)
            .default_width(385.0)
            .show(ctx, |ui| {
                compact(ui);
                ui.add_enabled_ui(!recording && self.project_rate_change.is_none(), |ui| {
                    egui::Grid::new("project-audio-setup-grid")
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Sample Rate");
                            let mut rate = self.data.project.sample_rate_hz;
                            egui::ComboBox::from_id_salt("project-recording-sample-rate")
                                .width(210.0)
                                .selected_text(format!("{rate} Hz"))
                                .show_ui(ui, |ui| {
                                    for value in
                                        [22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000]
                                    {
                                        ui.selectable_value(
                                            &mut rate,
                                            value,
                                            format!("{value} Hz"),
                                        );
                                    }
                                });
                            ui.end_row();
                            ui.label("Record Format");
                            let mut format = self.data.project.recording_format;
                            egui::ComboBox::from_id_salt("project-recording-encoding")
                                .width(210.0)
                                .selected_text(format.label())
                                .show_ui(ui, |ui| {
                                    for value in [
                                        RecordingFormat::Pcm16,
                                        RecordingFormat::Pcm24,
                                        RecordingFormat::Float32,
                                    ] {
                                        ui.selectable_value(&mut format, value, value.label());
                                    }
                                });
                            ui.end_row();
                            if rate!=self.data.project.sample_rate_hz || format!=self.data.project.recording_format {requested=Some((rate,format));}

                        });
                });
                ui.add_space(8.0);
                if recording {
                    ui.label("Stop recording to change these settings.");
                } else {
                    ui.label(
                        "Recording format applies to new recordings. Rate changes review existing media and conform playback.",
                    );
                }
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized([75.0, 22.0], egui::Button::new("OK"))
                        .clicked()
                    {
                        self.project_setup_open = false;
                    }
                });
            });
        self.project_setup_open &= open;
        if let Some((rate, format)) = requested {
            self.request_project_rate(rate, format);
        }
    }

    pub(super) fn show_vst_connections(&mut self, ctx: &egui::Context) {
        if !self.vst_connections_open || !self.project_open {
            return;
        }
        let mut open = true;
        let title = if self.vst_connections_tab == 0 {
            "VST Connections - Inputs"
        } else if self.vst_connections_tab == 1 {
            "VST Connections - Outputs"
        } else {
            "VST Connections"
        };
        egui::Window::new(title)
            .id(egui::Id::new("vst-connections-window"))
            .open(&mut open)
            .default_size(Vec2::new(820.0, 397.0))
            .min_size(Vec2::new(760.0, 340.0))
            .show(ctx, |ui| {
                compact(ui);
                ui.spacing_mut().item_spacing = Vec2::new(1.0, 4.0);
                ui.horizontal(|ui| {
                    for (n, name) in [
                        "Inputs",
                        "Outputs",
                        "Group/FX",
                        "External FX",
                        "External Instruments",
                        "Studio",
                    ]
                    .iter()
                    .enumerate()
                    {
                        if ui
                            .add_sized(
                                [
                                    (ui.available_width() / (6 - n) as f32 - 1.0).max(85.0),
                                    22.0,
                                ],
                                egui::Button::new(*name).selected(self.vst_connections_tab == n),
                            )
                            .clicked()
                        {
                            self.vst_connections_tab = n;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_enabled(false, egui::Button::new("⊞ ⊟ All"));
                    ui.add_space(15.0);
                    ui.add_enabled(
                        false,
                        egui::Button::new("Add Bus").min_size(Vec2::new(120.0, 18.0)),
                    );
                    ui.add_space(30.0);
                    ui.label("Presets");
                    ui.add_enabled(false, egui::Button::new("-                  ▾"));
                    ui.add_enabled(false, egui::Button::new("+"));
                    ui.add_enabled(false, egui::Button::new("−"));
                });
                let input = self.vst_connections_tab == 0;
                if self.vst_connections_tab > 1 {
                    ui.add_space(15.0);
                    ui.label("This connection type is unavailable.");
                    return;
                }
                let device = if input {
                    self.resolved_input_device_name
                        .clone()
                        .unwrap_or_else(|| "Unavailable".into())
                } else {
                    self.audio_engine
                        .as_ref()
                        .map(|e| e.device_name.clone())
                        .unwrap_or_else(|| "Unavailable".into())
                };
                let ports = self
                    .audio_engine
                    .as_ref()
                    .and_then(|e| e.asio_info.as_ref())
                    .map(|i| {
                        if input {
                            i.input_ports.clone()
                        } else {
                            i.output_ports.clone()
                        }
                    })
                    .unwrap_or_else(|| {
                        let count = if input {
                            self.cached_input_format.map(|f| f.channels).unwrap_or(0)
                        } else {
                            self.audio_engine.as_ref().map(|e| e.channels).unwrap_or(0)
                        };
                        (0..count.min(2))
                            .map(|n| {
                                format!("{} {}", if input { "Input" } else { "Output" }, n + 1)
                            })
                            .collect()
                    });
                let height = (ui.available_height() - 18.0).max(240.0);
                egui::ScrollArea::both()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width().max(790.0), height - 15.0),
                            Sense::hover(),
                        );
                        let p = theme::palette(ui);
                        let painter = ui.painter();
                        painter.rect_filled(rect, 0.0, p.canvas_alternate);
                        let widths = [100.0, 80.0, 130.0, 130.0, if input { 0.0 } else { 40.0 }];
                        let mut x = rect.left();
                        for (n, label) in [
                            "Bus Name",
                            "Speakers",
                            "Audio Device",
                            "Device Port",
                            "Click",
                        ]
                        .iter()
                        .enumerate()
                        {
                            if widths[n] == 0.0 {
                                continue;
                            }
                            let cell = Rect::from_min_size(
                                Pos2::new(x, rect.top()),
                                Vec2::new(widths[n], 18.0),
                            );
                            painter.rect_filled(cell, 0.0, p.surface_raised);
                            painter.text(
                                cell.left_center() + Vec2::new(4.0, 0.0),
                                Align2::LEFT_CENTER,
                                label,
                                FontId::proportional(11.0),
                                p.text,
                            );
                            x += widths[n];
                            painter.line_segment(
                                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                                Stroke::new(0.6, p.grid),
                            );
                        }
                        let rows = 1 + ports.len();
                        for row in 0..rows {
                            let y = rect.top() + 18.0 + row as f32 * 17.0;
                            let values = if row == 0 {
                                vec![
                                    format!(
                                        "⊟  {}",
                                        if input { "Stereo In" } else { "Stereo Out" }
                                    ),
                                    if ports.len() == 1 {
                                        "Mono".into()
                                    } else {
                                        "Stereo".into()
                                    },
                                    device.clone(),
                                    String::new(),
                                    if input { String::new() } else { "Click".into() },
                                ]
                            } else {
                                vec![
                                    format!("       · {}", if row == 1 { "left" } else { "right" }),
                                    String::new(),
                                    String::new(),
                                    ports[row - 1].clone(),
                                    String::new(),
                                ]
                            };
                            let mut x = rect.left();
                            for (col, value) in values.iter().enumerate() {
                                if widths[col] == 0.0 {
                                    continue;
                                }
                                let cell = Rect::from_min_size(
                                    Pos2::new(x, y),
                                    Vec2::new(widths[col], 17.0),
                                );
                                if col == 3 && row > 0 {
                                    painter.rect_filled(
                                        cell,
                                        0.0,
                                        p.accent.gamma_multiply(if input { 0.18 } else { 0.25 }),
                                    );
                                }
                                painter.with_clip_rect(cell).text(
                                    cell.left_center() + Vec2::new(4.0, 0.0),
                                    Align2::LEFT_CENTER,
                                    value,
                                    FontId::proportional(11.0),
                                    p.text,
                                );
                                x += widths[col];
                            }
                            painter.line_segment(
                                [
                                    Pos2::new(rect.left(), y + 17.0),
                                    Pos2::new(rect.right(), y + 17.0),
                                ],
                                Stroke::new(0.6, p.grid),
                            );
                        }
                        painter.rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(1.0, p.grid),
                            egui::StrokeKind::Inside,
                        );
                    });
                ui.label(
                    egui::RichText::new(
                        "Current device buses · Additional bus editing unavailable",
                    )
                    .size(10.0)
                    .color(theme::palette(ui).muted),
                );
            });
        self.vst_connections_open = open;
    }

    pub(super) fn poll_vst3_plugin_scan(&mut self, ctx: &egui::Context) {
        let result = match self.plugin_scan_task.as_ref() {
            Some(task) => match task.result.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(150));
                    None
                }
                Err(TryRecvError::Disconnected) => Some(Err(
                    "the VST3 scan worker stopped before returning a result".to_owned(),
                )),
            },
            None => return,
        };
        let Some(result) = result else {
            return;
        };
        self.plugin_scan_task = None;
        match result {
            Ok(report) => {
                let new_plugins = report.plugins.len();
                let available_plugins = report
                    .plugins
                    .iter()
                    .filter(|plugin| plugin.is_usable())
                    .count();
                let failed_modules = report.totals.rejected;
                let unavailable_folders = report
                    .folders
                    .iter()
                    .filter(|folder| folder.status != opentrack_plugin_scan::FolderStatus::Scanned)
                    .count();
                self.merge_vst3_plugin_catalog(report.plugins.clone());
                self.apply_vst3_scan_failures(&report);
                self.plugin_scan_status = format!(
                    "Scanned {} folder(s): {new_plugins} plug-in class(es) found, {available_plugins} available for inserts. {failed_modules} module(s) failed; {unavailable_folders} folder(s) unavailable.",
                    report.folders.len()
                );
                self.plugin_scan_error = None;
                self.plugin_scan_report = Some(report);
            }
            Err(error) => {
                self.plugin_scan_status = "VST3 plug-in scan failed".to_owned();
                self.plugin_scan_error = Some(error);
            }
        }
    }

    fn merge_vst3_plugin_catalog(&mut self, plugins: Vec<PluginDescriptor>) {
        for plugin in plugins {
            if let Some(existing) = self.vst3_plugin_catalog.iter_mut().find(|existing| {
                existing.format == plugin.format && existing.class_id == plugin.class_id
            }) {
                *existing = plugin;
            } else {
                self.vst3_plugin_catalog.push(plugin);
            }
        }
        sort_vst3_plugin_catalog(&mut self.vst3_plugin_catalog);
    }

    fn apply_vst3_scan_failures(&mut self, report: &opentrack_plugin_scan::ScanReport) {
        for module in report
            .modules
            .iter()
            .filter(|module| module.status.is_rejected())
        {
            let path = PathBuf::from(&module.path);
            for plugin in &mut self.vst3_plugin_catalog {
                if plugin.module_path == path {
                    plugin.scan_status = module.status;
                    if let Some(diagnostics) = &module.diagnostics {
                        plugin.diagnostics = Some(diagnostics.clone());
                    }
                }
            }
        }
        sort_vst3_plugin_catalog(&mut self.vst3_plugin_catalog);
    }

    fn start_vst3_plugin_scan(&mut self) {
        if self.plugin_scan_task.is_some() {
            return;
        }
        if self.vst3_plugin_folders.is_empty() {
            self.plugin_scan_status =
                "Add at least one VST3 plug-in folder before scanning.".to_owned();
            self.plugin_scan_error = Some("No VST3 plug-in folders are configured.".to_owned());
            return;
        }
        let folders = self.vst3_plugin_folders.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        self.plugin_scan_task = Some(PluginScanTask { result: receiver });
        self.plugin_scan_report = None;
        self.plugin_scan_error = None;
        self.plugin_scan_status = format!(
            "Scanning {} VST3 folder(s) in the background\u{2026}",
            folders.len()
        );
        match std::thread::Builder::new()
            .name("opentrack-vst3-scan".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    opentrack_plugin_scan::scan_folders_enumerating(
                        &folders,
                        opentrack_plugin_scan::DiscoveryLimits::default(),
                    )
                }))
                .map_err(|_| "the VST3 scan worker panicked".to_owned());
                let _ = sender.send(result);
            }) {
            Ok(_) => {}
            Err(error) => {
                self.plugin_scan_task = None;
                self.plugin_scan_status = "VST3 plug-in scan could not start".to_owned();
                self.plugin_scan_error = Some(error.to_string());
            }
        }
    }

    fn add_vst3_plugin_folder(&mut self, folder: PathBuf) {
        if self.vst3_plugin_folders.iter().any(|existing| {
            existing
                .to_string_lossy()
                .eq_ignore_ascii_case(&folder.to_string_lossy())
        }) {
            return;
        }
        self.vst3_plugin_folders.push(folder);
        self.plugin_scan_status =
            "Folder added. Click Scan or Rescan to update the catalog.".to_owned();
        self.plugin_scan_error = None;
    }

    fn remove_vst3_plugin_folder(&mut self, index: usize) {
        if index < self.vst3_plugin_folders.len() {
            self.vst3_plugin_folders.remove(index);
            self.plugin_scan_status =
                "Folder removed. Existing catalog entries are kept until a later scan replaces them."
                    .to_owned();
            self.plugin_scan_error = None;
        }
    }

    pub(super) fn show_plugin_information(&mut self, ctx: &egui::Context) {
        if !self.plugin_information_open {
            return;
        }
        let mut open = true;
        let mut add_folder = false;
        let mut add_module_file = false;
        let mut scan_requested = false;
        let mut remove_folder = None;
        egui::Window::new("Plug-in Information")
            .id(egui::Id::new("plugin-information-window"))
            .open(&mut open)
            .resizable(true)
            .default_size(Vec2::new(940.0, 560.0))
            .min_size(Vec2::new(720.0, 420.0))
            .show(ctx, |ui| {
                compact(ui);
                ui.label(egui::RichText::new("VST3 Plug-in Folders").strong());
                ui.label(
                    "OpenTrack scans these folders for VST3 effects and keeps the result in the reusable insert catalog.",
                );
                egui::Frame::group(ui.style())
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::both()
                            .id_salt("vst3-plugin-folders")
                            .max_height(108.0)
                            .show(ui, |ui| {
                                if self.vst3_plugin_folders.is_empty() {
                                    ui.add_enabled(
                                        false,
                                        egui::Button::new("No plug-in folders configured"),
                                    );
                                }
                                for (index, folder) in
                                    self.vst3_plugin_folders.iter().enumerate()
                                {
                                    ui.horizontal(|ui| {
                                        ui.label(folder.display().to_string());
                                        let folder_report =
                                            self.plugin_scan_report.as_ref().and_then(|report| {
                                                report.folders.iter().find(|entry| {
                                                    Path::new(&entry.path) == folder.as_path()
                                                })
                                            });
                                        if let Some(report) = folder_report {
                                            let (label, color) = match report.status {
                                                opentrack_plugin_scan::FolderStatus::Scanned => (
                                                    format!("{} module(s)", report.modules_found),
                                                    theme::palette(ui).muted,
                                                ),
                                                opentrack_plugin_scan::FolderStatus::Missing => {
                                                    ("missing".to_owned(), theme::WARNING)
                                                }
                                                opentrack_plugin_scan::FolderStatus::NotDirectory => {
                                                    ("not a directory".to_owned(), theme::WARNING)
                                                }
                                                opentrack_plugin_scan::FolderStatus::Error => {
                                                    ("error".to_owned(), theme::RECORD)
                                                }
                                            };
                                            let response = ui.colored_label(color, label);
                                            if let Some(diagnostics) = &report.diagnostics {
                                                response.on_hover_text(diagnostics);
                                            }
                                        } else if folder.is_dir() {
                                            ui.label("not scanned");
                                        } else {
                                            ui.colored_label(theme::WARNING, "missing");
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("Remove")
                                                    .on_hover_text(
                                                        "Remove this folder from scanning. Existing catalog entries are kept.",
                                                    )
                                                    .clicked()
                                                {
                                                    remove_folder = Some(index);
                                                }
                                            },
                                        );
                                    });
                                }
                            });
                    });
                ui.horizontal(|ui| {
                    if ui.button("Add Folder\u{2026}").clicked() {
                        add_folder = true;
                    }
                    if ui
                        .button("Add Plug-in File\u{2026}")
                        .on_hover_text(
                            "Adds the folder containing a .vst3 module. Scanning remains folder-based.",
                        )
                        .clicked()
                    {
                        add_module_file = true;
                    }
                    let scan_label = if self.vst3_plugin_catalog.is_empty() {
                        "Scan"
                    } else {
                        "Rescan"
                    };
                    if ui
                        .add_enabled(
                            self.plugin_scan_task.is_none() && !self.vst3_plugin_folders.is_empty(),
                            egui::Button::new(scan_label),
                        )
                        .clicked()
                    {
                        scan_requested = true;
                    }
                    if self.plugin_scan_task.is_some() {
                        ui.spinner();
                    }
                });
                if let Some(error) = &self.plugin_scan_error {
                    ui.colored_label(theme::RECORD, error);
                }
                ui.label(&self.plugin_scan_status);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Plug-in Catalog").strong());
                    ui.label(format!(
                        "{} class(es) reusable by the insert picker",
                        self.vst3_plugin_catalog.len()
                    ));
                });
                if self.vst3_plugin_catalog.is_empty() {
                    ui.label("No plug-ins are in the catalog yet. Add a folder and scan.");
                } else {
                    egui::ScrollArea::both()
                        .id_salt("vst3-plugin-catalog-table")
                        .max_height(270.0)
                        .show(ui, |ui| {
                            egui::Grid::new("vst3-plugin-catalog-grid")
                                .striped(true)
                                .num_columns(5)
                                .spacing(Vec2::new(12.0, 3.0))
                                .show(ui, |ui| {
                                    for heading in
                                        ["Name", "Vendor", "Category / Subcategory", "Module", "Status"]
                                    {
                                        ui.label(egui::RichText::new(heading).strong());
                                    }
                                    ui.end_row();
                                    for plugin in &self.vst3_plugin_catalog {
                                        ui.label(plugin.name.clone());
                                        ui.label(if plugin.vendor.is_empty() {
                                            "\u{2014}".to_owned()
                                        } else {
                                            plugin.vendor.clone()
                                        });
                                        let category = vst3_insert_category(plugin).label();
                                        ui.label(if plugin.subcategories.is_empty() {
                                            category.to_owned()
                                        } else {
                                            format!("{category} / {}", plugin.subcategories.join(", "))
                                        });
                                        let module = plugin.module_path.display().to_string();
                                        ui.add(egui::Label::new(module.clone()).truncate())
                                            .on_hover_text(module);
                                        let (status, color) = if plugin.is_usable() {
                                            (
                                                format!(
                                                    "{} \u{00b7} available",
                                                    plugin_scan_status_label(plugin.scan_status)
                                                ),
                                                theme::palette(ui).text,
                                            )
                                        } else {
                                            (
                                                format!(
                                                    "{} \u{00b7} unavailable",
                                                    plugin_scan_status_label(plugin.scan_status)
                                                ),
                                                theme::WARNING,
                                            )
                                        };
                                        ui.colored_label(color, status);
                                        ui.end_row();
                                    }
                                });
                        });
                }
                if let Some(report) = &self.plugin_scan_report {
                    let failed = report
                        .modules
                        .iter()
                        .filter(|module| module.status.is_rejected())
                        .collect::<Vec<_>>();
                    if !failed.is_empty() {
                        ui.separator();
                        egui::CollapsingHeader::new(format!(
                            "Failed modules from the last scan ({})",
                            failed.len()
                        ))
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("vst3-plugin-scan-failures")
                                .max_height(120.0)
                                .show(ui, |ui| {
                                    for module in failed {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.colored_label(
                                                theme::WARNING,
                                                plugin_scan_status_label(module.status),
                                            );
                                            ui.label(&module.path);
                                            if let Some(diagnostics) = &module.diagnostics {
                                                ui.label(diagnostics);
                                            }
                                        });
                                    }
                                });
                        });
                    }
                }
            });
        self.plugin_information_open = open;
        if let Some(index) = remove_folder {
            self.remove_vst3_plugin_folder(index);
        }
        if add_folder && let Some(folder) = FileDialog::new().pick_folder() {
            self.add_vst3_plugin_folder(folder);
        }
        if add_module_file
            && let Some(path) = FileDialog::new()
                .add_filter("VST3 plug-in", &["vst3"])
                .pick_file()
        {
            match vst3_host::resolve_module_selection(&path) {
                Ok(module) => {
                    if let Some(folder) = module.parent() {
                        self.add_vst3_plugin_folder(folder.to_path_buf());
                    }
                }
                Err(error) => {
                    self.plugin_scan_status = "Could not add the plug-in file".to_owned();
                    self.plugin_scan_error = Some(error);
                }
            }
        }
        if scan_requested {
            self.start_vst3_plugin_scan();
        }
    }
}

fn mode_label(mode: AudioDeviceMode) -> &'static str {
    match mode {
        AudioDeviceMode::SharedLowLatency => "Shared - Low Latency",
        AudioDeviceMode::SharedCompatible => "Shared - Compatible",
        AudioDeviceMode::Exclusive => "Exclusive",
    }
}

fn plugin_scan_status_label(status: opentrack_plugins::PluginScanStatus) -> &'static str {
    use opentrack_plugins::PluginScanStatus;
    match status {
        PluginScanStatus::NotScanned => "not scanned",
        PluginScanStatus::ReadyForEnumeration => "ready to enumerate",
        PluginScanStatus::Enumerated => "enumerated",
        PluginScanStatus::UnsupportedArchitecture => "unsupported architecture",
        PluginScanStatus::InvalidModule => "invalid module",
        PluginScanStatus::Unreadable => "unreadable",
        PluginScanStatus::LoadFailed => "load failed",
        PluginScanStatus::TimedOut => "timed out",
        PluginScanStatus::Crashed => "crashed",
        PluginScanStatus::NeedsAuthorization => "needs authorization",
        PluginScanStatus::Quarantined => "quarantined",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_setup_opens_and_renders_all_pages_without_a_device_stream() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.device_setup_open = true;
        for page in [0, 0, 0, 1, 1, 2, 0] {
            app.device_setup_page = page;
            let mut window_rect = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0))),
                    ..Default::default()
                },
                |_| {
                    window_rect = app.show_audio_device_setup(&ctx);
                },
            );
            let _ = ctx.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
            output.textures_delta.clear();
            assert!(app.device_setup_open);
            if let Some(rect) = window_rect {
                assert!(rect.height() < 370.0, "Device Setup is too tall: {rect:?}");
            }
        }
    }
}
