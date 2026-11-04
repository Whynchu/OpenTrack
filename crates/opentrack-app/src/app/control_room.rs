//! Control Room: a bounded, persisted monitor section.
//!
//! OpenTrack has no separate monitor bus or talkback input in its engine, so
//! this window does not pretend to route audio. It stores the monitor
//! preferences (listen source and talkback) in application settings and shows
//! the routing those preferences would monitor, so the state is visible and
//! survives a restart while the DSP work is still outstanding.

use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ControlRoomReadout {
    /// Resolved listen source, when it exists in this project.
    pub listen: Option<String>,
    pub listen_kind: Option<&'static str>,
    pub listen_route: Option<String>,
    pub listen_sends: usize,
    pub groups: usize,
    pub fx_returns: usize,
    pub sends_into_returns: usize,
    pub master: String,
}

fn kind_label(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Audio => "Audio",
        TrackKind::Midi => "MIDI",
        TrackKind::Instrument => "Instrument",
        TrackKind::Group => "Group",
        TrackKind::Fx => "FX Return",
        TrackKind::InstrumentOutput => "Instrument Output",
        TrackKind::Folder => "Folder",
    }
}

/// Resolves the stored listen id and counts the routing a monitor would see.
pub(super) fn readout(project: &Project, listen: Option<u64>) -> ControlRoomReadout {
    let mut readout = ControlRoomReadout {
        master: project.output_channel.strip.name.clone(),
        ..ControlRoomReadout::default()
    };
    if let Some(id) = listen
        && let Some(track) = project.channels().find(|track| track.id == id)
    {
        readout.listen = Some(track.name.clone());
        readout.listen_kind = Some(kind_label(track.kind));
        readout.listen_route = Some(
            track
                .output_routing
                .clone()
                .unwrap_or_else(|| "Stereo Out".to_owned()),
        );
        readout.listen_sends = project
            .tracks
            .iter()
            .flat_map(|source| source.send_routing.iter())
            .filter(|route| route.as_deref() == Some(format!("FX:{id}").as_str()))
            .count();
    }
    readout.groups = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Group)
        .count();
    readout.fx_returns = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Fx)
        .count();
    readout.sends_into_returns = project
        .tracks
        .iter()
        .flat_map(|source| source.send_routing.iter())
        .filter(|route| {
            route
                .as_deref()
                .is_some_and(|route| route.starts_with("FX:"))
        })
        .count();
    readout
}

impl OpenTrackApp {
    pub(super) fn open_control_room(&mut self, tab: u8) {
        self.control_room_open = true;
        self.control_room_tab = tab;
    }

    fn set_monitor_listen(&mut self, id: Option<u64>) {
        if let Some(id) = id {
            if self.data.project.channels().all(|track| track.id != id) {
                self.status_message =
                    "That channel is not in this project; the listen source was not changed"
                        .to_owned();
                return;
            }
            let name = self
                .data
                .project
                .channels()
                .find(|track| track.id == id)
                .map(|track| track.name.clone())
                .unwrap_or_else(|| "Channel".to_owned());
            self.monitor_listen = Some(id);
            self.status_message = format!("Listen source: {name}");
        } else {
            self.monitor_listen = None;
            self.status_message = "Listen source cleared".to_owned();
        }
    }

    fn toggle_monitor_talkback(&mut self) {
        self.monitor_talkback = !self.monitor_talkback;
        self.status_message = if self.monitor_talkback {
            "Talkback on (monitor preference; no talkback input DSP is implemented)".to_owned()
        } else {
            "Talkback off".to_owned()
        };
    }

    pub(super) fn show_control_room(&mut self, ctx: &egui::Context) {
        if !self.control_room_open {
            return;
        }
        let readout = readout(&self.data.project, self.monitor_listen);
        let channels = self
            .data
            .project
            .channels()
            .filter(|track| track.kind != TrackKind::Folder)
            .map(|track| {
                (
                    track.id,
                    format!("{} ({})", track.name, kind_label(track.kind)),
                )
            })
            .collect::<Vec<_>>();
        let mut open = true;
        let mut tab = self.control_room_tab;
        let mut listen = self.monitor_listen;
        let mut chosen = None;
        let mut talkback = false;
        let response = egui::Window::new("Control Room")
            .id(Id::new("control-room"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(520.0, 360.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(480.0);
                ui.label(
                    egui::RichText::new(
                        "Monitor preferences only: OpenTrack has no separate monitor bus or talkback input yet, so this window stores the listen source and talkback state and shows the routing they would monitor.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(tab == 0, "Mixer")
                        .clicked()
                    {
                        tab = 0;
                    }
                    if ui
                        .selectable_label(tab == 1, "Overview")
                        .clicked()
                    {
                        tab = 1;
                    }
                });
                ui.separator();
                if tab == 0 {
                    ui.horizontal(|ui| {
                        ui.label("Listen");
                        let selected = listen
                            .and_then(|id| {
                                channels
                                    .iter()
                                    .find(|(candidate, _)| *candidate == id)
                                    .map(|(_, label)| label.clone())
                            })
                            .unwrap_or_else(|| "Off".to_owned());
                        egui::ComboBox::from_id_salt("control-room-listen")
                            .width(240.0)
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(listen.is_none(), "Off").clicked() {
                                    chosen = Some(None);
                                }
                                for (id, label) in &channels {
                                    if ui
                                        .selectable_label(listen == Some(*id), label)
                                        .clicked()
                                    {
                                        chosen = Some(Some(*id));
                                    }
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        let mut on = self.monitor_talkback;
                        if ui.checkbox(&mut on, "Talkback").changed() {
                            talkback = true;
                        }
                        ui.label(
                            egui::RichText::new("preference only; no input DSP")
                                .small()
                                .color(theme::palette(ui).muted),
                        );
                    });
                    ui.separator();
                    match &readout.listen {
                        Some(name) => {
                            ui.label(format!(
                                "Listening to {name}{}",
                                readout
                                    .listen_kind
                                    .map(|kind| format!(" ({kind})"))
                                    .unwrap_or_default()
                            ));
                            ui.label(format!(
                                "Output: {} · {} send(s) from other tracks",
                                readout.listen_route.as_deref().unwrap_or("Stereo Out"),
                                readout.listen_sends
                            ));
                        }
                        None => {
                            ui.label("No listen source selected.");
                        }
                    }
                } else {
                    ui.label(format!("Master output: {}", readout.master));
                    ui.label(format!(
                        "{} group channel(s) · {} FX return(s) · {} send(s) into returns",
                        readout.groups, readout.fx_returns, readout.sends_into_returns
                    ));
                    ui.label(format!(
                        "Listen: {}",
                        if self.monitor_listen.is_some() {
                            readout.listen.as_deref().unwrap_or("selected channel not in this project")
                        } else {
                            "Off"
                        }
                    ));
                    ui.label(format!(
                        "Talkback: {}",
                        if self.monitor_talkback { "on" } else { "off" }
                    ));
                }
            });
        dismiss_window_on_outside_click(ctx, Id::new("control-room"), &mut open, response.as_ref());
        self.control_room_tab = tab;
        if let Some(choice) = chosen {
            self.set_monitor_listen(choice);
        }
        if talkback {
            self.toggle_monitor_talkback();
        }
        self.control_room_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (Project, u64) {
        let mut project = Project::default();
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Fx);
        let fx = project.tracks[2].id;
        project.tracks[1]
            .send_routing
            .push(Some(format!("FX:{fx}")));
        project.tracks[1].output_routing = Some("Stereo Out".to_owned());
        (project, fx)
    }

    fn app() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        (app.data.project, _) = project();
        app
    }

    #[test]
    fn the_readout_resolves_the_listen_source_and_counts_routing() {
        let (project, fx) = project();
        let off = readout(&project, None);
        assert!(off.listen.is_none());
        assert_eq!(off.groups, 1);
        assert_eq!(off.fx_returns, 1);
        assert_eq!(off.sends_into_returns, 1);
        assert_eq!(off.master, "Stereo Out");

        let selected = readout(&project, Some(fx));
        assert_eq!(selected.listen.as_deref(), Some("FX 03"));
        assert_eq!(selected.listen_kind, Some("FX Return"));
        assert_eq!(selected.listen_sends, 1);
        assert_eq!(selected.listen_route.as_deref(), Some("Stereo Out"));

        // A stale id resolves to nothing instead of inventing a channel.
        assert!(readout(&project, Some(9_999)).listen.is_none());
    }

    #[test]
    fn listen_and_talkback_are_stored_and_validated() {
        let mut app = app();
        let fx = app.data.project.tracks[2].id;
        app.set_monitor_listen(Some(fx));
        assert_eq!(app.monitor_listen, Some(fx));
        assert!(app.status_message.contains("Listen source: FX 03"));

        // A channel that is not in the project is refused.
        let before = app.monitor_listen;
        app.set_monitor_listen(Some(9_999));
        assert_eq!(app.monitor_listen, before);
        assert!(app.status_message.contains("not in this project"));

        app.set_monitor_listen(None);
        assert!(app.monitor_listen.is_none());

        assert!(!app.monitor_talkback);
        app.toggle_monitor_talkback();
        assert!(app.monitor_talkback);
        assert!(app.status_message.contains("Talkback on"));
        app.toggle_monitor_talkback();
        assert!(!app.monitor_talkback);

        app.open_control_room(1);
        assert!(app.control_room_open);
        assert_eq!(app.control_room_tab, 1);
    }

    #[test]
    fn the_window_draws_both_tabs() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        (app.data.project, _) = project();
        let fx = app.data.project.tracks[2].id;
        app.set_monitor_listen(Some(fx));
        app.monitor_talkback = true;
        app.control_room_open = true;
        app.control_room_tab = 0;
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 760.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_control_room(&ctx),
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
        assert!(joined.contains("Control Room"), "{joined}");
        assert!(joined.contains("Listen"), "{joined}");
        assert!(joined.contains("Talkback"), "{joined}");
        assert!(joined.contains("FX 03"), "{joined}");
    }
}
