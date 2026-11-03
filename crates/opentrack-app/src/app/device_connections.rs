//! Read-only VST Connections view: the device buses and the project's bus
//! channels, with the routing the project already uses.
//!
//! This is a readout, not a router: changing a track's output or a send happens
//! in the channel strip. The window answers "which buses exist, how wide are
//! they, where do they go, and who feeds them" for the device inputs/outputs and
//! for every group and FX return.

use super::*;
use opentrack_core::routing::OutputRoute;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DeviceBus {
    pub name: String,
    pub channels: u8,
    pub device: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ProjectBus {
    pub id: u64,
    pub name: String,
    pub kind: &'static str,
    pub channels: u8,
    pub output: String,
    pub sends: usize,
}

/// True when an audio track records from the input bus.
fn records_from_input(track: &Track) -> bool {
    if track.kind != TrackKind::Audio {
        return false;
    }
    match track.input_routing.as_deref() {
        None => true,
        Some(route) => !route.trim().eq_ignore_ascii_case("No Bus"),
    }
}

/// True when a track reaches Stereo Out directly, not through a group.
fn routes_to_stereo_out(track: &Track) -> bool {
    if track.kind.is_folder() || track.parent_group_id.is_some() {
        return false;
    }
    match track.output_routing.as_deref() {
        None => true,
        Some(route) => matches!(
            OutputRoute::parse(route),
            Ok(OutputRoute::StereoOut | OutputRoute::Left | OutputRoute::Right)
        ),
    }
}

/// The two device buses with their channel width, device and user counts.
pub(super) fn device_buses(
    project: &Project,
    input_device: &str,
    output_device: &str,
) -> [DeviceBus; 2] {
    let input = &project.input_channel.strip;
    let output = &project.output_channel.strip;
    let input_users = project
        .tracks
        .iter()
        .filter(|track| records_from_input(track))
        .count();
    let output_users = project
        .tracks
        .iter()
        .filter(|track| routes_to_stereo_out(track))
        .count();
    [
        DeviceBus {
            name: input.name.clone(),
            channels: input.audio_channels.max(1),
            device: input_device.to_owned(),
            detail: format!("{input_users} audio track(s) record from this bus"),
        },
        DeviceBus {
            name: output.name.clone(),
            channels: output.audio_channels.max(1),
            device: output_device.to_owned(),
            detail: format!("{output_users} channel(s) route here directly"),
        },
    ]
}

/// A group's stored output, resolved to a readable name.
fn output_label(project: &Project, track: &Track) -> String {
    match track.output_routing.as_deref() {
        None => {
            if let Some(group_id) = track.parent_group_id {
                project
                    .channels()
                    .find(|candidate| candidate.id == group_id)
                    .map(|group| group.name.clone())
                    .unwrap_or_else(|| "Group".to_owned())
            } else {
                "Stereo Out".to_owned()
            }
        }
        Some(route) => match OutputRoute::parse(route) {
            Ok(OutputRoute::Group(id)) => project
                .channels()
                .find(|candidate| candidate.id == id)
                .map(|group| format!("Group: {}", group.name))
                .unwrap_or_else(|| route.to_owned()),
            Ok(OutputRoute::NoBus) => "No Bus".to_owned(),
            _ => route.to_owned(),
        },
    }
}

/// Every group and FX return with its width, output and incoming send count.
pub(super) fn project_buses(project: &Project) -> Vec<ProjectBus> {
    project
        .tracks
        .iter()
        .filter(|track| track.kind.is_bus())
        .map(|track| {
            let prefix = if track.kind == TrackKind::Fx {
                "FX:"
            } else {
                "Group:"
            };
            let target = format!("{prefix}{}", track.id);
            let sends = project
                .tracks
                .iter()
                .flat_map(|source| source.send_routing.iter())
                .filter(|route| route.as_deref() == Some(target.as_str()))
                .count();
            ProjectBus {
                id: track.id,
                name: track.name.clone(),
                kind: if track.kind == TrackKind::Fx {
                    "FX Return"
                } else {
                    "Group"
                },
                channels: track.audio_channels.max(1),
                output: output_label(project, track),
                sends,
            }
        })
        .collect()
}

impl OpenTrackApp {
    /// Draws the read-only VST Connections window. `vst_connections_open` is
    /// toggled by the Devices row, F4 and the keymap command, so this is the one
    /// surface that has to exist for those controls to do anything.
    pub(super) fn show_device_connections(&mut self, ctx: &egui::Context) {
        if !self.vst_connections_open {
            return;
        }
        let input_device = self
            .resolved_input_device_name
            .clone()
            .unwrap_or_else(|| "System default".to_owned());
        let output_device = self
            .audio_engine
            .as_ref()
            .map(|engine| engine.device_name.clone())
            .or_else(|| self.selected_output_device.clone())
            .unwrap_or_else(|| "System default".to_owned());
        let devices = device_buses(&self.data.project, &input_device, &output_device);
        let buses = project_buses(&self.data.project);
        let mut open = true;
        let response = egui::Window::new("VST Connections")
            .id(Id::new("vst-connections"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(560.0, 320.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(560.0);
                ui.label(
                    egui::RichText::new(
                        "Read-only view of the device buses and project buses. Change a track's output or a send in its channel strip.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.separator();
                ui.label(egui::RichText::new("Device Buses").strong());
                for bus in &devices {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&bus.name).strong());
                        ui.label(format!("{} ch", bus.channels));
                        ui.label(
                            egui::RichText::new(format!("Device: {}", bus.device))
                                .small()
                                .color(theme::palette(ui).muted),
                        );
                    });
                    ui.label(
                        egui::RichText::new(&bus.detail)
                            .small()
                            .color(theme::palette(ui).muted),
                    );
                }
                ui.separator();
                ui.label(egui::RichText::new("Project Buses").strong());
                if buses.is_empty() {
                    ui.label(
                        egui::RichText::new("No group or FX return channels yet.")
                            .small()
                            .color(theme::palette(ui).muted),
                    );
                }
                for bus in &buses {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&bus.name).strong());
                        ui.label(bus.kind);
                        ui.label(format!("{} ch", bus.channels));
                        ui.label(format!("Output: {}", bus.output));
                        ui.label(format!(
                            "{} send(s)",
                            bus.sends
                        ));
                    });
                }
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("vst-connections"),
            &mut open,
            response.as_ref(),
        );
        self.vst_connections_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_buses() -> Project {
        let mut project = Project::default();
        project.add_track(TrackKind::Group); // 0: bus
        let group_id = project.tracks[0].id;
        project.add_track(TrackKind::Audio); // 1: child of the group
        project.tracks[1].parent_group_id = Some(group_id);
        project.tracks[1].output_routing = Some(format!("Group:{group_id}"));
        project.add_track(TrackKind::Audio); // 2: straight to Stereo Out
        project.add_track(TrackKind::Fx); // 3: return with one send
        let fx_id = project.tracks[3].id;
        project.tracks[2]
            .send_routing
            .push(Some(format!("FX:{fx_id}")));
        project
    }

    #[test]
    fn device_buses_report_width_devices_and_direct_users() {
        let project = project_with_buses();
        let buses = device_buses(&project, "Microphone (USB)", "Speakers (USB)");
        assert_eq!(buses[0].name, "Stereo In");
        assert_eq!(buses[0].channels, 2);
        assert_eq!(buses[0].device, "Microphone (USB)");
        assert_eq!(buses[0].detail, "2 audio track(s) record from this bus");
        assert_eq!(buses[1].name, "Stereo Out");
        assert_eq!(buses[1].device, "Speakers (USB)");
        // The group and the FX return also reach Stereo Out; only the audio
        // child is routed through its group instead.
        assert_eq!(buses[1].detail, "3 channel(s) route here directly");
    }

    #[test]
    fn project_buses_report_kind_output_and_sends() {
        let project = project_with_buses();
        let buses = project_buses(&project);
        assert_eq!(buses.len(), 2);
        assert_eq!(buses[0].kind, "Group");
        assert_eq!(buses[0].output, "Stereo Out");
        assert_eq!(buses[0].sends, 0);
        assert_eq!(buses[1].kind, "FX Return");
        assert_eq!(buses[1].output, "Stereo Out");
        assert_eq!(buses[1].sends, 1);

        // A group feeding another group shows the resolved name.
        let mut nested = project_with_buses();
        nested.add_track(TrackKind::Group);
        let outer = nested.tracks[4].id;
        nested.tracks[0].output_routing = Some(format!("Group:{outer}"));
        let buses = project_buses(&nested);
        assert!(
            buses[0].output.starts_with("Group: "),
            "{}",
            buses[0].output
        );
    }

    #[test]
    fn the_window_draws_the_buses() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project = project_with_buses();
        app.vst_connections_open = true;
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_device_connections(&ctx),
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
        assert!(joined.contains("VST Connections"), "{joined}");
        assert!(joined.contains("Stereo In"), "{joined}");
        assert!(joined.contains("Stereo Out"), "{joined}");
        assert!(joined.contains("FX Return"), "{joined}");
        assert!(joined.contains("1 send(s)"), "{joined}");
    }
}
