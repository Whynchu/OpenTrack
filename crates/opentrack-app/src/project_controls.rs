//! Original Project color selector and region-specific menus.
//! Evidence: Project-live-study/Project-study-11,13,14,15.
use super::*;

// Shared starting palette for the Event Colors selector and track strips.
pub(super) const COLORS: [[u8; 3]; 16] = crate::project_file::DEFAULT_EVENT_COLORS;

pub(super) fn track_color(track: &Track) -> Color32 {
    track
        .color_index
        .and_then(|i| COLORS.get(usize::from(i)))
        .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
        .unwrap_or_else(|| Color32::from_gray(160))
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
    Remove(u64),
    ShowAutomation(u64),
    HideAllAutomation,
}

fn add_items(ui: &mut egui::Ui, flat: bool, action: &mut Option<Action>) {
    for (name, kind) in [
        ("Audio", TrackKind::Audio),
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
        "Group Channel",
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

pub(super) fn background_menu(response: &egui::Response, action: &mut Option<Action>) {
    response.context_menu(|ui| {
        theme::menu_popup_style(ui.style_mut());
        ui.set_min_width(205.0);
        add_items(ui, true, action);
        ui.separator();
        ui.add_enabled(false, egui::Button::new("Show All Used Automation"));
        ui.add_enabled(false, egui::Button::new("Hide All Automation"));
    });
}

pub(super) fn track_menu(
    response: &egui::Response,
    id: u64,
    can_remove: bool,
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
        ui.add_enabled(false, egui::Button::new("Duplicate Tracks"));
        if ui
            .add_enabled(can_remove, egui::Button::new("Remove Selected Tracks"))
            .clicked()
        {
            *action = Some(Action::Remove(id));
            ui.close();
        }
        ui.add_enabled(false, egui::Button::new("Track Controls Settings..."));
        ui.separator();
        if ui
            .add_enabled(!automation_visible, egui::Button::new("Show Automation"))
            .clicked()
        {
            *action = Some(Action::ShowAutomation(id));
            ui.close();
        }
        ui.add_enabled(
            false,
            egui::Button::new("Show Used Automation (Selected Tracks)"),
        );
        ui.add_enabled(false, egui::Button::new("Show All Used Automation"));
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
    let p = ui.painter();
    if track.kind == TrackKind::Audio {
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
        p.add(egui::Shape::line(
            waveform
                .into_iter()
                .map(|(x, y)| center + Vec2::new(x, y))
                .collect(),
            Stroke::new(1.4, ink),
        ));
    } else {
        for x in [-6.0, -2.0, 2.0, 6.0] {
            p.rect_filled(
                Rect::from_min_size(center + Vec2::new(x, -5.0), Vec2::new(3.0, 11.0)),
                0.0,
                ink,
            );
        }
        for x in [-3.0, 1.0] {
            p.rect_filled(
                Rect::from_min_size(center + Vec2::new(x, -5.0), Vec2::new(2.0, 6.0)),
                0.0,
                theme::palette(ui).surface,
            );
        }
    }
    ui.interact(rect, Id::new(("track-type-icon", track.id)), Sense::hover())
        .on_hover_text(match track.kind {
            TrackKind::Audio => "Audio Track",
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
        let active = if index == 0 {
            track.automation_read
        } else {
            track.automation_write
        };
        if project_inspector::button(
            ui,
            r,
            ("automation-lane-control", track.id, index),
            label,
            if index == 0 {
                "Read Enable"
            } else {
                "Write Enable"
            },
            active,
            true,
            false,
        )
        .clicked()
        {
            if index == 0 {
                track.automation_read = !track.automation_read;
            } else {
                track.automation_write = !track.automation_write;
            }
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
    let p = theme::palette(ui);
    ui.painter().rect_filled(rect, 0.0, p.surface_raised);
    for (index, label) in ["−", "+"].into_iter().enumerate() {
        let left = if index == 0 {
            rect.left()
        } else {
            rect.right() - 16.0
        };
        let r = Rect::from_min_size(Pos2::new(left, rect.top()), Vec2::new(16.0, 16.0));
        let response = ui.interact(r, Id::new(("project-zoom-step", index)), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(
                r,
                1.0,
                theme::hover_button_fill(p.surface_raised, ui.visuals()),
            );
            ui.painter().rect_stroke(
                r,
                1.0,
                ui.visuals().widgets.hovered.bg_stroke,
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            p.text,
        );
        if response.clicked() {
            *value = (*value * if index == 0 { 0.8 } else { 1.25 })
                .clamp(crate::timeline::MIN_ZOOM, crate::timeline::MAX_ZOOM);
        }
    }
    let rail = Rect::from_min_max(
        rect.min + Vec2::new(19.0, 2.0),
        rect.max - Vec2::new(19.0, 2.0),
    );
    let response = ui.interact(rail, Id::new("project-zoom-rail"), Sense::click_and_drag());
    if (response.dragged() || response.clicked())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let f = ((pointer.x - rail.left()) / rail.width()).clamp(0.0, 1.0);
        *value = crate::timeline::MIN_ZOOM
            * (crate::timeline::MAX_ZOOM / crate::timeline::MIN_ZOOM).powf(f);
    }
    ui.painter().line_segment(
        [rail.left_center(), rail.right_center()],
        Stroke::new(
            if response.hovered() { 1.5 } else { 1.0 },
            if response.hovered() { p.accent } else { p.text },
        ),
    );
    let x = egui::lerp(
        rail.x_range(),
        ((*value / crate::timeline::MIN_ZOOM).ln()
            / (crate::timeline::MAX_ZOOM / crate::timeline::MIN_ZOOM).ln())
        .clamp(0.0, 1.0),
    );
    ui.painter().rect_filled(
        Rect::from_center_size(Pos2::new(x, rail.center().y), Vec2::new(6.0, 10.0)),
        1.0,
        p.accent,
    );
}
