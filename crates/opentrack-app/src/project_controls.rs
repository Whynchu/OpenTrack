//! Original Project color selector and region-specific menus.
//! Evidence: Project-live-study/Project-study-11,13,14,15.
use super::*;

// Shared starting palette for the Event Colors selector and track strips.
pub(super) const COLORS: [[u8; 3]; 16] = crate::project_file::DEFAULT_EVENT_COLORS;
/// Stereo In/Out use a blue identity until the user picks another color.
pub(super) const BUS_DEFAULT_COLOR: Color32 = Color32::from_rgb(76, 142, 210);

pub(super) fn track_color(track: &Track) -> Color32 {
    track
        .color_index
        .and_then(|i| COLORS.get(usize::from(i)))
        .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
        .unwrap_or_else(|| {
            if track.id >= opentrack_core::project::INPUT_BUS_ID {
                BUS_DEFAULT_COLOR
            } else {
                Color32::from_gray(160)
            }
        })
}

pub(super) fn has_track_color(track: &Track) -> bool {
    track.color_index.is_some()
}

pub(super) fn color_strip(
    ui: &mut egui::Ui,
    rect: Rect,
    track: &mut Track,
    show_color: bool,
) -> bool {
    let palette = theme::palette(ui);
    let color = if show_color {
        track_color(track)
    } else {
        palette.muted
    };
    ui.painter().rect_filled(rect, 0.0, color);
    let response = ui
        .interact(
            rect,
            Id::new(("track-color-selector", track.id)),
            Sense::click(),
        )
        .on_hover_text("Track Color Selector");
    if response.hovered() {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            0.0,
            ui.visuals().widgets.hovered.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    let mut changed = false;
    egui::Popup::menu(&response)
        .align(egui::RectAlign::RIGHT_START)
        .frame(egui::Frame::popup(ui.style()).inner_margin(egui::Margin::same(2)))
        .width(374.0)
        .show(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let (bar, _) = ui.allocate_exact_size(Vec2::new(374.0, 40.0), Sense::hover());
            let mut caption = track
                .color_index
                .map(|i| format!("Color {}", i + 1))
                .unwrap_or_else(|| String::from("Default Color"));
            // Live reference: the first grey swatch is Default Color.
            let first = Rect::from_min_size(bar.min, Vec2::new(22.0, 40.0));
            ui.painter()
                .rect_filled(first, 0.0, Color32::from_gray(160));
            let default = ui.interact(
                first,
                Id::new(("track-default-color", track.id)),
                Sense::click(),
            );
            if default.hovered() {
                caption = String::from("Default Color");
                ui.painter().rect_stroke(
                    first.shrink(1.0),
                    0.0,
                    ui.visuals().widgets.hovered.bg_stroke,
                    egui::StrokeKind::Inside,
                );
            }
            if default.clicked() {
                let had_palette_color = track.color_index.take().is_some();
                changed |= had_palette_color;
                ui.close();
            }
            for (index, rgb) in COLORS.iter().enumerate() {
                let r = Rect::from_min_size(
                    bar.min + Vec2::new((index + 1) as f32 * 22.0, 0.0),
                    Vec2::new(22.0, 40.0),
                );
                ui.painter()
                    .rect_filled(r, 0.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                if track.color_index == Some(index as u8) {
                    ui.painter().rect_stroke(
                        r.shrink(1.0),
                        0.0,
                        Stroke::new(1.5, palette.text),
                        egui::StrokeKind::Inside,
                    );
                }
                let swatch = ui.interact(
                    r,
                    Id::new(("track-color-swatch", track.id, index)),
                    Sense::click(),
                );
                if swatch.hovered() {
                    caption = format!("Color {}", index + 1);
                    ui.painter().rect_stroke(
                        r.shrink(1.0),
                        0.0,
                        ui.visuals().widgets.hovered.bg_stroke,
                        egui::StrokeKind::Inside,
                    );
                }
                if swatch.clicked() {
                    track.color_index = Some(index as u8);
                    changed = true;
                    ui.close();
                }
            }
            let (label, _) = ui.allocate_exact_size(Vec2::new(374.0, 22.0), Sense::hover());
            ui.painter().rect_filled(label, 0.0, palette.surface);
            ui.painter().text(
                label.left_center() + Vec2::new(5.0, 0.0),
                Align2::LEFT_CENTER,
                caption,
                FontId::proportional(11.0),
                palette.text,
            );
        });
    changed
}

#[derive(Clone, Copy)]
pub(super) enum Action {
    Add(TrackKind),
    Duplicate(u64),
    Remove(u64),
    ShowAutomation(u64),
    ShowUsedAutomation(Option<u64>),
    HideAllAutomation,
}

pub(super) fn used_automation_track_ids(project: &Project, only_track: Option<u64>) -> Vec<u64> {
    project
        .tracks
        .iter()
        .filter(|track| only_track.is_none_or(|id| id == track.id))
        .filter(|track| track_has_used_automation(track))
        .map(|track| track.id)
        .collect()
}

pub(super) fn track_has_used_automation(track: &Track) -> bool {
    matches!(track.kind, TrackKind::Audio | TrackKind::Instrument)
        && track.automation_lanes.iter().any(|lane| {
            lane.target == opentrack_core::project::AutomationTarget::TrackVolume
                && !lane.points.is_empty()
                && lane.is_valid()
        })
}

fn add_items(ui: &mut egui::Ui, flat: bool, action: &mut Option<Action>) {
    for (name, kind) in [
        ("Audio", TrackKind::Audio),
        ("Group Channel", TrackKind::Group),
        ("Instrument", TrackKind::Instrument),
        ("MIDI", TrackKind::Midi),
    ] {
        let label = if flat {
            format!("Add {name} Track")
        } else {
            name.to_owned()
        };
        if ui.button(label).clicked() {
            *action = Some(Action::Add(kind));
            ui.close();
        }
    }
    ui.separator();
    for name in [
        "Arranger",
        "FX Channel",
        "Folder",
        "Marker",
        "Ruler",
        "Signature",
        "Tempo",
        "Transpose",
        "Video",
    ] {
        let label = if flat {
            format!("Add {name} Track")
        } else {
            name.to_owned()
        };
        ui.add_enabled(false, egui::Button::new(label));
    }
    ui.separator();
    ui.add_enabled(false, egui::Button::new("Browse Sounds..."));
}

pub(super) fn background_menu(
    response: &egui::Response,
    has_used_automation: bool,
    action: &mut Option<Action>,
) {
    response.context_menu(|ui| {
        theme::menu_popup_style(ui.style_mut());
        ui.set_min_width(205.0);
        add_items(ui, true, action);
        ui.separator();
        if ui
            .add_enabled(
                has_used_automation,
                egui::Button::new("Show All Used Automation"),
            )
            .clicked()
        {
            *action = Some(Action::ShowUsedAutomation(None));
            ui.close();
        }
        ui.add_enabled(false, egui::Button::new("Hide All Automation"));
    });
}

pub(super) fn track_menu(
    response: &egui::Response,
    track_id: u64,
    can_remove: bool,
    has_used_automation: bool,
    automation_visible: bool,
    any_automation_visible: bool,
    action: &mut Option<Action>,
) {
    response.context_menu(|ui| {
        theme::menu_popup_style(ui.style_mut());
        ui.set_min_width(305.0);
        ui.menu_button("Add Track", |ui| add_items(ui, false, action));
        ui.separator();
        ui.add_enabled(false, egui::Button::new("Select All Events"));
        if ui
            .add_enabled(can_remove, egui::Button::new("Duplicate Tracks"))
            .clicked()
        {
            *action = Some(Action::Duplicate(track_id));
            ui.close();
        }
        if ui
            .add_enabled(can_remove, egui::Button::new("Remove Selected Tracks"))
            .clicked()
        {
            *action = Some(Action::Remove(track_id));
            ui.close();
        }
        ui.add_enabled(false, egui::Button::new("Track Controls Settings..."));
        ui.separator();
        if ui
            .add_enabled(!automation_visible, egui::Button::new("Show Automation"))
            .clicked()
        {
            *action = Some(Action::ShowAutomation(track_id));
            ui.close();
        }
        if ui
            .add_enabled(
                has_used_automation,
                egui::Button::new("Show Used Automation"),
            )
            .clicked()
        {
            *action = Some(Action::ShowUsedAutomation(Some(track_id)));
            ui.close();
        }
        if ui
            .add_enabled(
                any_automation_visible,
                egui::Button::new("Hide All Automation"),
            )
            .clicked()
        {
            *action = Some(Action::HideAllAutomation);
            ui.close();
        }
        ui.separator();
        for label in [
            "Disable Track",
            "Set Record Folder",
            "Auto Fades Settings...",
        ] {
            ui.add_enabled(false, egui::Button::new(label));
        }
        ui.separator();
        for label in ["Create Track Preset...", "Apply Track Preset..."] {
            ui.add_enabled(false, egui::Button::new(label));
        }
    });
}

pub(super) fn label_ink(color: Color32) -> Color32 {
    let luminance = 0.2126 * f32::from(color.r())
        + 0.7152 * f32::from(color.g())
        + 0.0722 * f32::from(color.b());
    if luminance > 145.0 {
        Color32::from_gray(18)
    } else {
        Color32::WHITE
    }
}

pub(super) fn type_icon(ui: &mut egui::Ui, center: Pos2, track: &Track) {
    let rect = Rect::from_center_size(center, Vec2::new(18.0, 16.0));
    let ink = theme::palette(ui).text;
    if track.kind == TrackKind::Audio {
        // Compact waveform glyph retained from the earlier Project track header.
        let waveform = [
            (-7.0, 0.0),
            (-6.0, -2.0),
            (-5.0, 3.0),
            (-4.0, -4.0),
            (-3.0, 5.0),
            (-2.0, -6.0),
            (-1.0, 3.0),
            (0.0, 0.0),
            (1.0, -4.0),
            (2.0, 6.0),
            (3.0, -5.0),
            (4.0, 4.0),
            (5.0, -3.0),
            (6.0, 2.0),
            (7.0, 0.0),
        ];
        ui.painter().add(egui::Shape::line(
            waveform
                .into_iter()
                .map(|(x, y)| center + Vec2::new(x, y))
                .collect(),
            Stroke::new(1.4, ink),
        ));
    } else {
        let icon = match track.kind {
            TrackKind::Group => crate::icon_system::IconId::Channel,
            TrackKind::Instrument => crate::icon_system::IconId::TrackMidi,
            TrackKind::Midi => crate::icon_system::IconId::TrackInstrument,
            TrackKind::Audio => unreachable!("audio icon is drawn above"),
        };
        crate::icon_system::paint_app_icon(ui, rect, icon, ink);
    }
    ui.interact(rect, Id::new(("track-type-icon", track.id)), Sense::hover())
        .on_hover_text(match track.kind {
            TrackKind::Audio => "Audio Track",
            TrackKind::Group => "Group Channel",
            TrackKind::Instrument => "Instrument Track",
            TrackKind::Midi => "MIDI Track",
        });
}
pub(super) fn automation_lane(ui: &mut egui::Ui, rect: Rect, track: &mut Track) -> bool {
    let mut changed = false;
    let p = theme::palette(ui);
    for (index, label) in ["R", "W"].into_iter().enumerate() {
        let r = Rect::from_min_size(
            rect.min + Vec2::new(2.0 + index as f32 * 24.0, 3.0),
            Vec2::new(22.0, 14.0),
        );
        let read_control =
            index == 0 && matches!(track.kind, TrackKind::Audio | TrackKind::Instrument);
        let active = read_control && track.automation_read;
        if project_inspector::button(
            ui,
            r,
            ("automation-lane-control", track.id, index),
            label,
            if index == 0 {
                "Read Enable (track volume only)"
            } else {
                "Automation writing is not implemented"
            },
            active,
            read_control,
            false,
        )
        .clicked()
        {
            track.automation_read = !track.automation_read;
            changed = true;
        }
    }
    let choice = Rect::from_min_size(
        rect.min + Vec2::new(50.0, 3.0),
        Vec2::new((rect.width() - 52.0).max(60.0), 14.0),
    );
    ui.painter().rect_filled(
        choice,
        1.0,
        blend_color(p.surface_raised, Color32::WHITE, 0.55),
    );
    ui.painter().rect_stroke(
        choice,
        1.0,
        Stroke::new(1.0, p.grid),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        choice.left_center() + Vec2::new(3.0, 0.0),
        Align2::LEFT_CENTER,
        "Volume",
        FontId::proportional(10.0),
        Color32::from_gray(18),
    );
    let c = choice.right_center() - Vec2::new(6.0, 0.0);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            c + Vec2::new(-2.0, -1.0),
            c + Vec2::new(2.0, -1.0),
            c + Vec2::new(0.0, 2.0),
        ],
        Color32::from_gray(30),
        Stroke::NONE,
    ));
    let response = ui.interact(
        choice,
        Id::new(("automation-parameter", track.id)),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter().rect_filled(
            choice,
            1.0,
            theme::hover_button_fill(
                blend_color(p.surface_raised, Color32::WHITE, 0.55),
                ui.visuals(),
            ),
        );
        ui.painter().rect_stroke(
            choice,
            1.0,
            ui.visuals().widgets.hovered.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let arrow = choice.right_center() - Vec2::new(6.0, 0.0);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                arrow + Vec2::new(-2.0, -1.0),
                arrow + Vec2::new(2.0, -1.0),
                arrow + Vec2::new(0.0, 2.0),
            ],
            Color32::from_gray(30),
            Stroke::NONE,
        ));
        ui.painter().text(
            choice.left_center() + Vec2::new(3.0, 0.0),
            Align2::LEFT_CENTER,
            "Volume",
            FontId::proportional(10.0),
            Color32::from_gray(18),
        );
    }
    egui::Popup::menu(&response).show(|ui| {
        ui.label("Volume");
    });
    let value = Rect::from_min_size(rect.min + Vec2::new(50.0, 23.0), Vec2::new(51.0, 14.0));
    ui.painter().rect_filled(
        value,
        1.0,
        blend_color(p.surface_raised, Color32::WHITE, 0.55),
    );
    ui.painter().text(
        value.center(),
        Align2::CENTER_CENTER,
        if track.gain <= 0.0 {
            "??".to_owned()
        } else {
            format!("{:.2}", 20.0 * track.gain.log10())
        },
        FontId::proportional(10.0),
        Color32::from_gray(18),
    );
    changed
}

pub(super) fn zoom_control(ui: &mut egui::Ui, rect: Rect, value: &mut f32) {
    crate::view_controls::zoom(
        ui,
        rect,
        Id::new("project-time-zoom"),
        value,
        [crate::timeline::MIN_ZOOM, crate::timeline::MAX_ZOOM],
        1.0,
        false,
    );
}

pub(super) fn track_zoom_control(ui: &mut egui::Ui, rect: Rect, value: &mut f32) {
    crate::view_controls::zoom(
        ui,
        rect,
        Id::new("project-track-zoom"),
        value,
        [0.5, 4.0],
        1.0,
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_write_control_is_disabled_and_preserves_saved_flag() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(220.0, 42.0));
        let mut track = opentrack_core::project::Project::default();
        track.add_track(TrackKind::Audio);
        let track = &mut track.tracks[0];
        track.automation_write = true;
        let point = rect.min + Vec2::new(2.0 + 24.0 + 11.0, 3.0 + 7.0);
        let run = |time, pressed, track: &mut Track| {
            let events = if pressed {
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]
            } else {
                vec![egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }]
            };
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(rect),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    automation_lane(ui, rect, track);
                },
            );
            output.textures_delta.clear();
            output
        };
        run(0.0, false, track);
        run(0.1, true, track);
        run(0.2, false, track);
        assert!(track.automation_write);
    }

    #[test]
    fn used_automation_tracks_include_only_supported_nonempty_volume_lanes() {
        use opentrack_core::project::{AutomationLane, AutomationPoint, AutomationTarget};

        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Instrument);
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Midi);
        project.tracks[0].automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![AutomationPoint {
                beat: 0.0,
                value: 1.0,
            }],
        });
        project.tracks[1].automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![],
        });
        project.tracks[2].automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![AutomationPoint {
                beat: 0.0,
                value: 0.5,
            }],
        });
        project.tracks[3].automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![AutomationPoint {
                beat: 0.0,
                value: 0.75,
            }],
        });

        assert_eq!(
            used_automation_track_ids(&project, None),
            vec![project.tracks[0].id]
        );
        assert_eq!(
            used_automation_track_ids(&project, Some(project.tracks[0].id)),
            vec![project.tracks[0].id]
        );
        assert!(used_automation_track_ids(&project, Some(project.tracks[1].id)).is_empty());
    }

    #[test]
    fn stereo_buses_default_to_blue_and_accept_palette_colors() {
        let project = opentrack_core::project::Project::default();
        let input = &project.input_channel.strip;
        assert!(input.color_index.is_none());
        assert_eq!(track_color(input), BUS_DEFAULT_COLOR);
        assert_eq!(
            track_color(&project.output_channel.strip),
            BUS_DEFAULT_COLOR
        );

        let mut chosen = input.clone();
        chosen.color_index = Some(3);
        assert_eq!(
            track_color(&chosen),
            Color32::from_rgb(COLORS[3][0], COLORS[3][1], COLORS[3][2])
        );
    }
}
