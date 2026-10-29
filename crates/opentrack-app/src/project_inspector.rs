//! Original Project Inspector; layout evidence: approved L1 and t2-039,060..080.
use super::*;

pub(super) fn button(
    ui: &mut egui::Ui,
    r: Rect,
    key: impl std::hash::Hash + std::fmt::Debug,
    label: &str,
    tooltip: &str,
    active: bool,
    enabled: bool,
    round: bool,
) -> egui::Response {
    let enabled = enabled && ui.is_enabled();
    let response = ui.interact(
        r,
        ui.make_persistent_id(key),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let p = theme::palette(ui);
    let normal_fill = if active {
        if round {
            theme::RECORD
        } else if label == "M" {
            theme::WARNING
        } else {
            p.accent
        }
    } else {
        blend_color(
            p.surface_raised,
            Color32::WHITE,
            if ui.visuals().dark_mode { 0.52 } else { 0.76 },
        )
    };
    let fill = theme::paint_control(ui, r, &response, normal_fill, enabled, round);
    let ink = theme::control_ink(fill, enabled);
    if round {
        ui.painter()
            .circle_filled(r.center(), r.height() * 0.19, p.canvas);
    } else {
        symbol(ui, r, label, ink);
    }
    response.on_hover_text(tooltip)
}

fn symbol(ui: &mut egui::Ui, r: Rect, name: &str, ink: Color32) {
    let id = match name {
        "M" => Some(crate::icon_system::IconId::GlyphMute),
        "S" => Some(crate::icon_system::IconId::GlyphSolo),
        "L" => Some(crate::icon_system::IconId::GlyphListen),
        "R" => Some(crate::icon_system::IconId::GlyphRecord),
        "W" => Some(crate::icon_system::IconId::GlyphWrite),
        "E" | "e" => Some(crate::icon_system::IconId::GlyphFx),
        "▦" => Some(crate::icon_system::IconId::SnapType),
        "×" => Some(crate::icon_system::IconId::Close),
        "◀" | "◉" => Some(crate::icon_system::IconId::Monitor),
        "♪" => Some(crate::icon_system::IconId::TrackMusic),
        "▱" => Some(crate::icon_system::IconId::Document),
        "▤" => Some(crate::icon_system::IconId::TakeLanes),
        "▤G" => Some(crate::icon_system::IconId::GroupLanes),
        "✳" | "▯" => Some(crate::icon_system::IconId::Channel),
        "↔" => Some(crate::icon_system::IconId::Insert),
        "◇" => Some(crate::icon_system::IconId::Eq),
        "⊥" => Some(crate::icon_system::IconId::Send),
        _ => None,
    };
    if let Some(id) = id {
        crate::icon_system::paint_app_icon(ui, r, id, ink);
        return;
    }
    let p = ui.painter();
    let c = r.center();
    let stroke = Stroke::new(1.1, ink);
    let pt = |x, y| c + Vec2::new(x, y);
    let line = |a, b| p.line_segment([a, b], stroke);
    match name {
        "▦" => {
            for x in [-4.0, 0.0, 4.0] {
                for y in [-3.0, 2.0] {
                    p.rect_stroke(
                        Rect::from_min_size(pt(x, y), Vec2::splat(3.0)),
                        0.0,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        "×" => {
            line(pt(-5.0, -4.0), pt(5.0, 4.0));
            line(pt(-5.0, 4.0), pt(5.0, -4.0));
        }
        "◀" => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    pt(-4.0, -2.0),
                    pt(-1.0, -2.0),
                    pt(3.0, -5.0),
                    pt(3.0, 5.0),
                    pt(-1.0, 2.0),
                    pt(-4.0, 2.0),
                ],
                ink,
                Stroke::NONE,
            ));
        }
        "♪" => {
            line(pt(2.0, -5.0), pt(2.0, 3.0));
            line(pt(2.0, -5.0), pt(6.0, -4.0));
            p.circle_filled(pt(0.0, 3.0), 2.5, ink);
        }
        "▱" => {
            p.rect_stroke(
                Rect::from_min_size(pt(-5.0, -1.0), Vec2::new(10.0, 6.0)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            p.add(egui::Shape::line(
                vec![pt(-3.0, -1.0), pt(-3.0, -5.0), pt(3.0, -5.0), pt(3.0, -1.0)],
                stroke,
            ));
        }
        "▤" => {
            p.rect_stroke(
                r.shrink2(Vec2::new(5.0, 3.0)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            for y in [-3.0, 0.0, 3.0] {
                line(pt(-3.0, y), pt(3.0, y));
            }
        }
        "✳" => {
            for angle in [0.0_f32, 1.0472, 2.0944] {
                let v = Vec2::new(angle.cos(), angle.sin()) * 5.0;
                line(c - v, c + v);
            }
        }
        "↔" => {
            line(pt(-6.0, 0.0), pt(6.0, 0.0));
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(5.0, 5.0)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        "◇" => {
            p.add(egui::Shape::closed_line(
                vec![pt(0.0, -6.0), pt(5.0, 0.0), pt(0.0, 6.0), pt(-5.0, 0.0)],
                stroke,
            ));
            line(pt(0.0, -7.0), pt(0.0, 7.0));
        }
        "⊥" => {
            line(pt(-6.0, 3.0), pt(6.0, 3.0));
            line(pt(0.0, -5.0), pt(0.0, 3.0));
            p.rect_stroke(
                Rect::from_center_size(pt(0.0, -3.0), Vec2::new(5.0, 4.0)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        "▯" => {
            line(pt(0.0, -6.0), pt(0.0, 6.0));
            p.rect_stroke(
                Rect::from_center_size(pt(0.0, -1.0), Vec2::new(5.0, 4.0)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        "◉" => {
            p.circle_stroke(c, 6.0, stroke);
            line(pt(-4.0, -4.0), pt(4.0, 4.0));
        }
        _ => {
            p.text(
                c,
                Align2::CENTER_CENTER,
                name,
                theme::control_label_font(ui, r),
                ink,
            );
        }
    }
}

fn field(ui: &mut egui::Ui, label: &str, tooltip: &str, height: f32) {
    field_impl(ui, label, tooltip, height, Sense::hover());
}

fn field_impl(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: &str,
    height: f32,
    sense: Sense,
) -> egui::Response {
    let p = theme::palette(ui);
    let (r, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), sense);
    ui.painter().rect_filled(
        r,
        1.0,
        if sense.senses_click() && response.hovered() {
            theme::hover_button_fill(p.canvas, ui.visuals())
        } else {
            p.canvas
        },
    );
    ui.painter()
        .rect_stroke(r, 1.0, Stroke::new(1.0, p.grid), egui::StrokeKind::Inside);
    ui.painter().text(
        r.left_center() + Vec2::new(18.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(10.5),
        if sense.senses_click() {
            p.text
        } else {
            p.muted
        },
    );
    let c = r.left_center() + Vec2::new(8.0, 0.0);
    ui.painter().line_segment(
        [c + Vec2::new(-4.0, -4.0), c + Vec2::new(-4.0, 3.0)],
        Stroke::new(1.0, p.muted),
    );
    ui.painter().line_segment(
        [c + Vec2::new(-4.0, 3.0), c + Vec2::new(4.0, 3.0)],
        Stroke::new(1.0, p.muted),
    );
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            c + Vec2::new(1.0, 0.0),
            c + Vec2::new(5.0, 3.0),
            c + Vec2::new(1.0, 6.0),
        ],
        p.muted,
        Stroke::NONE,
    ));
    response.on_hover_text(tooltip)
}

fn routing_field(
    ui: &mut egui::Ui,
    track: &mut Track,
    input: bool,
    group_names: &HashMap<u64, String>,
) -> bool {
    let default = if input {
        if track.audio_channels == 1 {
            "Left - Stereo In"
        } else {
            "Stereo In"
        }
    } else {
        "Stereo Out"
    };
    let label = if input {
        track.input_routing.as_deref()
    } else {
        track.output_routing.as_deref()
    }
    .unwrap_or(default)
    .to_owned();
    let label = label
        .strip_prefix("Group:")
        .and_then(|value| value.parse::<u64>().ok())
        .map(|group_id| {
            group_names
                .get(&group_id)
                .map_or_else(|| "Group Bus".to_owned(), |name| format!("Group: {name}"))
        })
        .unwrap_or(label);
    let response = field_impl(
        ui,
        &label,
        if input {
            "Input Routing (all selected with Shift + ALT + click)"
        } else {
            "Output Routing (all selected with Shift + ALT + click)"
        },
        18.0,
        Sense::click(),
    );
    let mut changed = false;
    egui::Popup::menu(&response).show(|ui| {
        theme::menu_popup_style(ui.style_mut());
        let choices: &[&str] = if input {
            &[
                "No Bus",
                "Stereo In",
                "Left - Stereo In",
                "Right - Stereo In",
            ]
        } else {
            &["No Bus", "Stereo Out", "Left", "Right"]
        };
        for value in choices {
            if ui
                .add_enabled(
                    !input || *value != "Stereo In" || track.audio_channels != 1,
                    egui::Button::new(*value),
                )
                .clicked()
            {
                if input {
                    track.input_routing = Some((*value).into());
                } else {
                    track.output_routing = Some((*value).into());
                    track.group_output_auto = false;
                }
                changed = true;
                ui.close();
            }
        }
    });
    changed
}

pub(super) fn show(
    ui: &mut egui::Ui,
    project: &mut Project,
    channel_settings_window: &mut NativeWindowState,
) -> InspectorChanges {
    let Some(track_index) = project
        .selected_track
        .map(|i| i as usize)
        .filter(|i| *i < project.tracks.len())
    else {
        ui.label("No track selected");
        return InspectorChanges::default();
    };
    let mut modified = false;
    let mut requires_plan_rebuild = false;
    let mut live_mix_changed = false;
    let mut solo_exclusively = None;
    let group_names = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Group)
        .map(|track| (track.id, track.name.clone()))
        .collect::<HashMap<_, _>>();
    {
        let track = &mut project.tracks[track_index];
        let track_id = track.id;
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let p = theme::palette(ui);
        let (header, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), Sense::hover());
        ui.painter().rect_filled(
            header,
            0.0,
            if project_controls::has_track_color(track) {
                project_controls::track_color(track)
            } else {
                blend_color(p.surface, p.text, 0.10)
            },
        );
        ui.painter().text(
            header.left_center() + Vec2::new(3.0, 0.0),
            Align2::LEFT_CENTER,
            &track.name,
            FontId::proportional(11.0),
            if project_controls::has_track_color(track) {
                project_controls::label_ink(project_controls::track_color(track))
            } else {
                p.text
            },
        );
        let e = Rect::from_center_size(
            header.right_center() - Vec2::new(9.0, 0.0),
            Vec2::splat(16.0),
        );
        if button(
            ui,
            e,
            (track_id, "edit"),
            "e",
            "Edit Channel Settings",
            false,
            true,
            false,
        )
        .clicked()
        {
            channel_settings_window.request_channel_settings(None);
        }
        let (controls, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 50.0), Sense::hover());
        ui.painter().rect_stroke(
            controls.shrink(1.0),
            3.0,
            Stroke::new(1.0, p.grid),
            egui::StrokeKind::Inside,
        );
        for (index, (label, tooltip, active, enabled)) in [
            ("M", "Mute", track.muted, true),
            (
                "S",
                "Solo State (Solo Defeat with ALT click)",
                track.solo || track.solo_defeat,
                true,
            ),
            ("R", "Read Enable", track.automation_read, true),
            ("W", "Write Enable", track.automation_write, true),
            ("▦", "Open Device Panels", false, false),
            ("×", "Auto Fades Settings...", false, false),
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::from_min_size(
                controls.min + Vec2::new(2.0 + index as f32 * 25.0, 7.0),
                Vec2::new(21.0, 14.0),
            );
            let response = button(
                ui,
                r,
                (track_id, "primary", index),
                label,
                tooltip,
                active,
                enabled,
                false,
            );
            if response.clicked() {
                match index {
                    0 => {
                        track.muted = !track.muted;
                        modified = true;
                        requires_plan_rebuild = true;
                    }
                    1 => {
                        let modifiers = ui.input(|i| i.modifiers);
                        if modifiers.alt {
                            track.solo_defeat = !track.solo_defeat;
                            if track.solo_defeat {
                                track.solo = false;
                            }
                        } else {
                            track.solo = !track.solo;
                            if track.solo {
                                track.solo_defeat = false;
                                if modifiers.ctrl {
                                    solo_exclusively = Some(track_id);
                                }
                            }
                        }
                        modified = true;
                        requires_plan_rebuild = true;
                    }
                    2 => {
                        track.automation_read = !track.automation_read;
                        modified = true;
                        requires_plan_rebuild = true;
                    }
                    3 => {
                        track.automation_write = !track.automation_write;
                        modified = true;
                        requires_plan_rebuild = true;
                    }
                    _ => {}
                }
            }
        }
        let record = Rect::from_min_size(controls.min + Vec2::new(2.0, 28.0), Vec2::splat(24.0));
        if button(
            ui,
            record,
            (track_id, "record"),
            "",
            "Enable Record",
            track.armed,
            true,
            true,
        )
        .clicked()
        {
            track.armed = !track.armed;
            modified = true;
            requires_plan_rebuild = true;
        }
        for (index, (label, tooltip, left, width)) in [
            ("◀", "Monitor", 29.0, 16.0),
            (
                "♪",
                "Toggle Timebase between Musical and Linear",
                53.0,
                22.0,
            ),
            ("▱", "Lock", 77.0, 22.0),
            ("▤", "Lane Display Type", 101.0, 22.0),
            ("✳", "Freeze Audio Channel", 125.0, 22.0),
        ]
        .into_iter()
        .enumerate()
        {
            button(
                ui,
                Rect::from_min_size(controls.min + Vec2::new(left, 28.0), Vec2::new(width, 14.0)),
                (track_id, "secondary", index),
                label,
                tooltip,
                false,
                false,
                false,
            );
        }
        ui.add_space(5.0);
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);
        let parameter_well = Rect::from_min_size(
            ui.cursor().min + Vec2::new(47.0, 0.0),
            Vec2::new((ui.available_width() - 47.0).max(30.0), 64.0),
        );
        ui.painter().rect_filled(parameter_well, 1.0, p.canvas);
        ui.painter().rect_stroke(
            parameter_well,
            1.0,
            Stroke::new(1.0, p.grid),
            egui::StrokeKind::Inside,
        );
        let mut gain_db = channel_gain::inspector_db(track.gain);
        if inspector_parameter_row(
            ui,
            0,
            &mut gain_db,
            channel_gain::MUTE_DB..=channel_gain::MAX_DB,
            "dB",
            Some("Volume"),
        ) {
            track.gain = channel_gain::from_inspector_db(gain_db);
            modified = true;
            live_mix_changed = true;
        }
        let mut pan_percent = track.pan * 100.0;
        if inspector_parameter_row(ui, 1, &mut pan_percent, -100.0..=100.0, "", Some("Pan")) {
            track.pan = pan_percent / 100.0;
            modified = true;
            live_mix_changed = true;
        }
        // No delay tooltip was observed in the reference sweep.
        let delay_changed = inspector_parameter_row(
            ui,
            2,
            &mut track.track_delay_ms,
            -2000.0..=2000.0,
            "ms",
            None,
        );
        modified |= delay_changed;
        requires_plan_rebuild |= delay_changed;

        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.add_space(12.0);
        let routing_changed = routing_field(ui, track, true, &group_names)
            | routing_field(ui, track, false, &group_names);
        modified |= routing_changed;
        requires_plan_rebuild |= routing_changed;
        ui.add_space(8.0);
        field(ui, "No Track Preset", "Apply Track Preset", 22.0);
        ui.add_space(6.0);
        let sections = [
            ("Inserts", "inserts", Some("Shows Active Inserts")),
            ("Equalizers", "equalizers", Some("Shows Active Equalizers")),
            ("Sends", "sends", Some("Shows Active Sends")),
            ("Channel", "channel", None),
            ("Notepad", "notepad", Some("Notepad is empty")),
            (
                "Quick Controls",
                "quick-controls",
                Some("Shows Active Quick Controls"),
            ),
        ];
        for (title, key, tooltip) in sections {
            let id = ui.make_persistent_id(("project-inspector", track_id, key));
            let mut open = ui
                .data_mut(|data| data.get_temp::<bool>(id))
                .unwrap_or(false);
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
            let palette = theme::palette(ui);
            ui.painter()
                .rect_filled(rect, 0.0, blend_color(palette.surface, palette.text, 0.10));
            ui.painter().rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, palette.grid),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.left_center() + Vec2::new(7.0, 0.0),
                Align2::LEFT_CENTER,
                title,
                FontId::proportional(11.0),
                palette.text,
            );
            ui.painter().line_segment(
                [
                    Pos2::new(rect.right() - 21.0, rect.top()),
                    Pos2::new(rect.right() - 21.0, rect.bottom()),
                ],
                Stroke::new(1.0, palette.grid),
            );
            let section_symbol = match key {
                "inserts" => "↔",
                "equalizers" => "◇",
                "sends" => "⊥",
                "channel" => "▯",
                "notepad" => "▤",
                _ => "◉",
            };
            symbol(
                ui,
                Rect::from_center_size(
                    rect.right_center() - Vec2::new(10.0, 0.0),
                    Vec2::splat(18.0),
                ),
                section_symbol,
                palette.muted,
            );
            if response.clicked() {
                open = !open;
                ui.data_mut(|data| data.insert_temp(id, open));
            }
            if let Some(tooltip) = tooltip {
                response.on_hover_text(tooltip);
            }
            if open {
                ui.indent(id, |ui| match key {
                    "inserts" => {
                        if track.inserts.is_empty() {
                            ui.label("No active inserts");
                        } else {
                            for insert in &track.inserts {
                                let name = match &insert.kind {
                                    InsertEffectKind::TestGenerator { .. } => "Test Generator",
                                    InsertEffectKind::StereoDelay { .. } => "Stereo Delay",
                                    InsertEffectKind::LegacyUnavailableCompressor { .. } => {
                                        "Unavailable Compressor"
                                    }
                                    InsertEffectKind::Vst3 { .. } => "VST3 Plug-in",
                                };
                                ui.label(name);
                            }
                        }
                    }
                    "equalizers" => {
                        for (index, band) in track.equalizer.bands.iter().enumerate() {
                            ui.label(format!(
                                "EQ Band {}  {:.0} Hz",
                                index + 1,
                                band.frequency_hz
                            ));
                        }
                    }
                    "sends" => {
                        ui.label("No active sends");
                    }
                    "channel" => {
                        ui.label("Channel controls");
                    }
                    "notepad" => {
                        ui.label("Notepad is empty");
                    }
                    "quick-controls" => {
                        ui.label("No quick controls");
                    }
                    _ => {}
                });
            }
        }
    }
    if let Some(track_id) = solo_exclusively {
        for track in &mut project.tracks {
            if track.id != track_id {
                track.solo = false;
            }
        }
    }
    InspectorChanges {
        modified,
        live_mix: (live_mix_changed && !requires_plan_rebuild).then_some((
            project.tracks[track_index].id,
            project.tracks[track_index].gain,
            project.tracks[track_index].pan,
        )),
        requires_plan_rebuild,
    }
}

#[derive(Default)]
pub(super) struct InspectorChanges {
    pub modified: bool,
    pub live_mix: Option<(u64, f32, f32)>,
    pub requires_plan_rebuild: bool,
}
