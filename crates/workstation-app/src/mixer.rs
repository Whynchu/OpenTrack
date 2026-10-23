//! Original Mixer graphics and measured layout. Evidence: approved L1 frame and t3-001..097.
//! This module is deliberately separate from Channel Settings and effect editors.
use super::*;

#[derive(Clone, Default)]
struct MixerView {
    hide_routing: bool,
    hide_input: bool,
    hide_audio: bool,
    hide_output: bool,
    clipboard: Option<Track>,
}

#[derive(Clone, Copy)]
enum Icon {
    Text(&'static str),
    Phase,
    Edit,
    Insert,
    Eq,
    Send,
    Speaker,
    Record,
    Copy,
    Paste,
    Routing,
    Target,
    Channel,
    Arrow,
    Connections,
    Chevron(bool),
}

fn icon(p: &egui::Painter, rect: Rect, icon: Icon, ink: Color32) {
    let c = rect.center();
    let line = Stroke::new(1.2, ink);
    let point = |x, y| c + Vec2::new(x, y);
    match icon {
        Icon::Text(text) => {
            p.text(
                c,
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(12.0),
                ink,
            );
        }
        Icon::Phase => {
            p.circle_stroke(c, 5.0, line);
            p.line_segment([point(-6.0, 6.0), point(6.0, -6.0)], line);
        }
        Icon::Edit => {
            p.text(
                c + Vec2::new(0.0, -2.5),
                Align2::CENTER_CENTER,
                "e",
                FontId::proportional(17.0),
                ink,
            );
        }
        Icon::Insert => {
            p.line_segment([point(-8.0, 0.0), point(-3.0, 0.0)], line);
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(6.0, 5.0)),
                0.0,
                line,
                egui::StrokeKind::Inside,
            );
            p.line_segment([point(3.0, 0.0), point(8.0, 0.0)], line);
        }
        Icon::Eq => {
            p.add(egui::Shape::closed_line(
                vec![
                    point(-8.0, 0.0),
                    point(0.0, -6.0),
                    point(8.0, 0.0),
                    point(0.0, 6.0),
                ],
                line,
            ));
            p.line_segment([point(0.0, -7.0), point(0.0, 7.0)], line);
        }
        Icon::Send => {
            p.line_segment([point(-8.0, 5.0), point(8.0, 5.0)], line);
            p.line_segment([point(-6.0, 4.0), point(-6.0, -2.0)], line);
            p.line_segment([point(-6.0, -2.0), point(-2.0, -2.0)], line);
            p.rect_stroke(
                Rect::from_center_size(point(1.0, -2.0), Vec2::new(6.0, 5.0)),
                0.0,
                line,
                egui::StrokeKind::Inside,
            );
            p.line_segment([point(4.0, -2.0), point(8.0, -2.0)], line);
        }
        Icon::Speaker => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    point(-6.0, -3.0),
                    point(-3.0, -3.0),
                    point(2.0, -7.0),
                    point(2.0, 7.0),
                    point(-3.0, 3.0),
                    point(-6.0, 3.0),
                ],
                ink,
                Stroke::NONE,
            ));
        }
        Icon::Record => {
            p.circle_filled(c, rect.height() * 0.27, ink);
        }
        Icon::Copy | Icon::Paste => {
            p.rect_stroke(
                Rect::from_center_size(point(-2.0, -2.0), Vec2::new(9.0, 10.0)),
                0.0,
                line,
                egui::StrokeKind::Inside,
            );
            p.rect_stroke(
                Rect::from_center_size(point(2.0, 2.0), Vec2::new(9.0, 10.0)),
                0.0,
                line,
                egui::StrokeKind::Inside,
            );
            if matches!(icon, Icon::Paste) {
                p.line_segment([point(-2.0, -7.0), point(4.0, -7.0)], line);
            }
        }
        Icon::Routing => {
            for x in [-5.0, 0.0, 5.0] {
                p.line_segment([point(x, -6.0), point(x, 6.0)], Stroke::new(2.0, ink));
            }
        }
        Icon::Target => {
            p.line_segment([point(-8.0, 0.0), point(8.0, 0.0)], line);
            p.line_segment([point(0.0, -7.0), point(0.0, 7.0)], line);
            p.circle_stroke(c, 4.0, line);
        }
        Icon::Channel => {
            for x in [-5.0, 0.0, 5.0] {
                p.line_segment([point(x, -6.0), point(x, 6.0)], line);
                p.rect_filled(
                    Rect::from_center_size(point(x, x * 0.5), Vec2::new(3.0, 4.0)),
                    0.0,
                    ink,
                );
            }
        }
        Icon::Arrow => {
            p.line_segment([point(-7.0, 0.0), point(7.0, 0.0)], line);
            p.line_segment([point(2.0, -5.0), point(7.0, 0.0)], line);
            p.line_segment([point(2.0, 5.0), point(7.0, 0.0)], line);
        }
        Icon::Connections => {
            for offset in [Vec2::new(-5.0, -4.0), Vec2::new(5.0, 4.0)] {
                p.rect_stroke(
                    Rect::from_center_size(c + offset, Vec2::new(6.0, 5.0)),
                    0.0,
                    line,
                    egui::StrokeKind::Inside,
                );
            }
            p.line_segment([point(-2.0, -4.0), point(5.0, -4.0)], line);
            p.line_segment([point(5.0, -4.0), point(5.0, 1.0)], line);
        }
        Icon::Chevron(up) => {
            let direction = if up { -1.0 } else { 1.0 };
            p.line_segment(
                [point(-4.0, -2.0 * direction), point(0.0, 2.0 * direction)],
                line,
            );
            p.line_segment(
                [point(0.0, 2.0 * direction), point(4.0, -2.0 * direction)],
                line,
            );
        }
    }
}

fn control(
    ui: &mut egui::Ui,
    rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    symbol: Icon,
    active: bool,
    round: bool,
    enabled: bool,
    tooltip: Option<&str>,
) -> egui::Response {
    let palette = theme::palette(ui);
    let response = ui.interact(
        rect,
        ui.id().with(id),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let dark = ui.visuals().dark_mode;
    let base = if dark {
        Color32::from_gray(if enabled { 158 } else { 100 })
    } else {
        Color32::from_gray(if enabled { 213 } else { 185 })
    };
    let fill = if active {
        if matches!(symbol, Icon::Record) {
            theme::RECORD
        } else {
            palette.accent
        }
    } else if response.hovered() && enabled {
        base.gamma_multiply(1.13)
    } else {
        base
    };
    let ink = if active && !matches!(symbol, Icon::Record) {
        palette.canvas
    } else {
        Color32::from_gray(24)
    };
    if round {
        ui.painter()
            .circle_filled(rect.center(), rect.height() * 0.5, fill);
        ui.painter().circle_stroke(
            rect.center(),
            rect.height() * 0.5 - 0.5,
            Stroke::new(1.0, palette.grid),
        );
    } else {
        ui.painter().rect_filled(rect, 2.0, fill);
        ui.painter().rect_stroke(
            rect,
            2.0,
            Stroke::new(1.0, palette.grid),
            egui::StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [
                rect.left_top() + Vec2::new(2.0, 1.0),
                rect.right_top() + Vec2::new(-2.0, 1.0),
            ],
            Stroke::new(1.0, Color32::from_white_alpha(90)),
        );
    }
    icon(ui.painter(), rect.shrink(1.0), symbol, ink);
    if let Some(text) = tooltip {
        response.on_hover_text(text)
    } else {
        response
    }
}

fn text(ui: &egui::Ui, rect: Rect, value: &str, size: f32, ink: Color32, align: Align2) {
    ui.painter().text(
        align.pos_in_rect(&rect),
        align,
        value,
        FontId::proportional(size),
        ink,
    );
}

pub(super) fn show_channels(
    ui: &mut egui::Ui,
    project: &mut Project,
    meter_levels: &HashMap<u64, f32>,
    settings: &mut NativeWindowState,
) -> bool {
    let state_id = ui.id().with("measured-mixer-view");
    let mut view = ui
        .data_mut(|data| data.get_temp::<MixerView>(state_id))
        .unwrap_or_default();
    let area = ui.available_rect_before_wrap();
    let height = (area.height() - 22.0).max(300.0);
    let mut modified = false;
    let mut selected = project.selected_track;
    let mut exclusive = None;
    let palette = theme::palette(ui);
    ui.painter().rect_filled(area, 0.0, palette.surface);
    let rail = Rect::from_min_size(area.min, Vec2::new(68.0, height));
    sidebar(ui, rail, project, &mut view, &mut modified);
    let input_x = area.left() + 70.0;
    if !view.hide_input {
        bus(
            ui,
            Rect::from_min_size(Pos2::new(input_x, area.top()), Vec2::new(84.0, height)),
            true,
            view.hide_routing,
        );
    }
    let start = input_x + if view.hide_input { 0.0 } else { 94.0 };
    let right = area.right() - if view.hide_output { 0.0 } else { 86.0 };
    let tracks_rect = Rect::from_min_max(
        Pos2::new(start, area.top()),
        Pos2::new(right.max(start), area.bottom()),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(tracks_rect), |ui| {
        ui.style_mut().spacing.scroll.floating = false;
        ui.style_mut().spacing.scroll.bar_width = 12.0;
        egui::ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .max_height(height + 14.0)
            .id_salt("mixer-track-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.horizontal_top(|ui| {
                    for (index, track) in project.tracks.iter_mut().enumerate() {
                        if view.hide_audio && track.kind == TrackKind::Audio {
                            continue;
                        }
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(82.0, height), Sense::hover());
                        let (changed, select, solo) = strip(
                            ui,
                            rect,
                            track,
                            index,
                            selected == Some(index as u32),
                            meter_levels.get(&track.id).copied().unwrap_or(0.0),
                            settings,
                            view.hide_routing,
                        );
                        modified |= changed;
                        if select {
                            selected = Some(index as u32);
                            modified = true;
                        }
                        if solo {
                            exclusive = Some(track.id);
                        }
                    }
                });
            });
    });
    if !view.hide_output {
        bus(
            ui,
            Rect::from_min_size(
                Pos2::new(area.right() - 84.0, area.top()),
                Vec2::new(84.0, height),
            ),
            false,
            view.hide_routing,
        );
    }
    if project.selected_track != selected {
        project.selected_track = selected;
        project.selected_midi_part_id = None;
    }
    if let Some(id) = exclusive {
        for track in &mut project.tracks {
            if track.id != id {
                track.solo = false;
            }
        }
    }
    ui.data_mut(|data| data.insert_temp(state_id, view));
    modified
}

fn sidebar(
    ui: &mut egui::Ui,
    r: Rect,
    project: &mut Project,
    view: &mut MixerView,
    modified: &mut bool,
) {
    let at = |x, y, w, h| Rect::from_min_size(r.min + Vec2::new(x, y), Vec2::new(w, h));
    let palette = theme::palette(ui);
    ui.painter().line_segment(
        [r.right_top(), r.right_bottom()],
        Stroke::new(1.0, palette.grid),
    );
    if control(
        ui,
        at(1.0, 63.0, 12.0, 12.0),
        "routing",
        Icon::Chevron(false),
        false,
        false,
        true,
        Some("Hide Routing"),
    )
    .clicked()
    {
        view.hide_routing = !view.hide_routing;
    }
    control(
        ui,
        at(1.0, 77.0, 12.0, 12.0),
        "target",
        Icon::Chevron(true),
        false,
        false,
        false,
        Some("Command Target: Selected Only"),
    );
    ui.painter().rect_filled(
        at(34.0, 88.0, 29.0, (r.height() - 100.0).max(100.0)),
        1.0,
        palette.canvas,
    );
    let buttons = [
        (Icon::Text("M"), "Mute Active/Deactivate All Mute", 126.0),
        (Icon::Text("S"), "Solo Active/Deactivate All Solo", 142.0),
        (
            Icon::Text("L"),
            "Listen Active/Deactivate All Listen",
            160.0,
        ),
        (Icon::Text("R"), "Read All Automation", 183.0),
        (Icon::Text("W"), "Write All Automation", 200.0),
    ];
    for (i, (symbol, tip, y)) in buttons.into_iter().enumerate() {
        let active = project.tracks.iter().any(|t| match i {
            0 => t.muted,
            1 => t.solo,
            2 => t.listen,
            3 => t.automation_read,
            _ => t.automation_write,
        });
        if control(
            ui,
            at(7.0, y, 21.0, 15.0),
            i,
            symbol,
            active,
            false,
            true,
            Some(tip),
        )
        .clicked()
        {
            for t in &mut project.tracks {
                match i {
                    0 => t.muted = false,
                    1 => t.solo = false,
                    2 => t.listen = false,
                    3 => t.automation_read = !active,
                    _ => t.automation_write = !active,
                }
            }
            *modified = true;
        }
    }
    control(
        ui,
        at(7.0, 240.0, 21.0, 15.0),
        "reset",
        Icon::Text("↶"),
        false,
        false,
        false,
        Some("Reset Mixer/Reset Channels"),
    );
    if control(
        ui,
        at(7.0, 276.0, 21.0, 15.0),
        "copy",
        Icon::Copy,
        false,
        false,
        project.selected_track.is_some(),
        Some("Copy First Selected Channel's Settings"),
    )
    .clicked()
    {
        view.clipboard = project
            .selected_track
            .and_then(|i| project.tracks.get(i as usize))
            .cloned();
    }
    if control(
        ui,
        at(7.0, 294.0, 21.0, 15.0),
        "paste",
        Icon::Paste,
        false,
        false,
        view.clipboard.is_some() && project.selected_track.is_some(),
        Some("Paste Settings to Selected Channels"),
    )
    .clicked()
    {
        if let (Some(source), Some(target)) = (
            &view.clipboard,
            project
                .selected_track
                .and_then(|i| project.tracks.get_mut(i as usize)),
        ) {
            target.gain = source.gain;
            target.pan = source.pan;
            target.input_gain_db = source.input_gain_db;
            target.input_phase_inverted = source.input_phase_inverted;
            target.equalizer = source.equalizer.clone();
            target.inserts = source.inserts.clone();
            target.send_routing = source.send_routing.clone();
            *modified = true;
        }
    }
    control(
        ui,
        at(7.0, 348.0, 21.0, 15.0),
        "connections",
        Icon::Connections,
        false,
        false,
        false,
        Some("Show VST Connections"),
    );
    control(
        ui,
        at(35.0, 94.0, 26.0, 18.0),
        "routing-mode",
        Icon::Routing,
        false,
        false,
        false,
        Some("Hide Channels set to 'Can Hide'"),
    );
    control(
        ui,
        at(35.0, 120.0, 26.0, 18.0),
        "target-mode",
        Icon::Target,
        false,
        false,
        false,
        Some("Set Target Channels to 'Can Hide'"),
    );
    for (i, (symbol, tip, y)) in [
        (Icon::Arrow, "Hide Input Channels", 183.0),
        (Icon::Arrow, "Hide Audio Channels", 200.0),
        (Icon::Channel, "Hide Group Channels", 216.0),
        (Icon::Eq, "Hide ReWire Channels", 240.0),
        (Icon::Routing, "Hide MIDI Channels", 258.0),
        (Icon::Routing, "Hide MIDI Channels", 276.0),
        (Icon::Channel, "Hide VST Instrument Channels", 294.0),
        (Icon::Arrow, "Hide FX Channels", 312.0),
        (Icon::Arrow, "Hide Output Channels", 330.0),
        (Icon::Text("••"), "Reveal All Channels", 348.0),
    ]
    .into_iter()
    .enumerate()
    {
        let enabled = matches!(i, 0 | 1 | 8 | 9);
        let active = match i {
            0 => view.hide_input,
            1 => view.hide_audio,
            8 => view.hide_output,
            _ => false,
        };
        if control(
            ui,
            at(37.0, y, 23.0, 15.0),
            100 + i,
            symbol,
            active,
            false,
            enabled,
            Some(tip),
        )
        .clicked()
        {
            match i {
                0 => view.hide_input = !view.hide_input,
                1 => view.hide_audio = !view.hide_audio,
                8 => view.hide_output = !view.hide_output,
                9 => {
                    view.hide_input = false;
                    view.hide_audio = false;
                    view.hide_output = false;
                }
                _ => {}
            }
        }
    }
}

fn routing(ui: &mut egui::Ui, rect: Rect, track: &mut Track, modified: &mut bool) {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.set_min_width(rect.width());
            ui.set_max_width(rect.width());
            ui.painter()
                .rect_filled(rect, 0.0, theme::palette(ui).canvas);
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.spacing_mut().button_padding = Vec2::new(1.0, 0.0);
            ui.spacing_mut().interact_size.y = 15.0;
            for input in [true, false] {
                let default = if input {
                    if track.audio_channels == 1 {
                        "Left - Stereo In"
                    } else {
                        "Stereo In"
                    }
                } else {
                    "Stereo Out"
                };
                let value = if input {
                    track.input_routing.as_deref()
                } else {
                    track.output_routing.as_deref()
                }
                .unwrap_or(default)
                .to_owned();
                let response = ui.menu_button(egui::RichText::new(value).size(10.0), |ui| {
                    if ui.button("No Bus").clicked() {
                        if input {
                            track.input_routing = Some("No Bus".into());
                        } else {
                            track.output_routing = Some("No Bus".into());
                        }
                        *modified = true;
                        ui.close();
                    }
                    if input {
                        ui.menu_button("Stereo In", |ui| {
                            for label in ["Left - Stereo In", "Right - Stereo In"] {
                                if ui.button(label).clicked() {
                                    track.input_routing = Some(label.into());
                                    *modified = true;
                                    ui.close();
                                }
                            }
                        });
                    } else {
                        for label in ["Stereo Out", "Left", "Right"] {
                            if ui.button(label).clicked() {
                                track.output_routing = Some(label.into());
                                *modified = true;
                                ui.close();
                            }
                        }
                    }
                });
                response.response.on_hover_text(if input {
                    "Input Routing (all selected with Shift + ALT + click)"
                } else {
                    "Output Routing (all selected with Shift + ALT + click)"
                });
            }
        },
    );
}

fn input_row(ui: &mut egui::Ui, r: Rect, track: &mut Track) -> bool {
    let at = |x, y, w, h| Rect::from_min_size(r.min + Vec2::new(x, y), Vec2::new(w, h));
    let mut modified = false;
    if control(
        ui,
        at(5.0, 53.0, 16.0, 15.0),
        "phase",
        Icon::Phase,
        track.input_phase_inverted,
        false,
        true,
        Some("Input Phase"),
    )
    .clicked()
    {
        track.input_phase_inverted = !track.input_phase_inverted;
        modified = true;
    }
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(at(24.0, 53.0, 30.0, 16.0)),
        |ui| {
            ui.spacing_mut().interact_size.y = 15.0;
            ui.spacing_mut().button_padding = Vec2::ZERO;
            modified |= ui
                .add_sized(
                    [30.0, 15.0],
                    egui::DragValue::new(&mut track.input_gain_db)
                        .range(-24.0..=24.0)
                        .speed(0.1)
                        .fixed_decimals(1),
                )
                .on_hover_text("Input Gain")
                .changed();
        },
    );
    let knob = at(57.0, 48.0, 24.0, 24.0);
    let response = ui
        .interact(
            knob,
            ui.id().with("input-gain-knob"),
            Sense::click_and_drag(),
        )
        .on_hover_text("Input Gain (use shift)");
    if response.dragged() {
        let speed = if ui.input(|i| i.modifiers.shift) {
            0.01
        } else {
            0.1
        };
        track.input_gain_db =
            (track.input_gain_db - response.drag_delta().y * speed).clamp(-24.0, 24.0);
        modified = true;
    }
    if response.double_clicked() {
        track.input_gain_db = 0.0;
        modified = true;
    }
    knob_graphic(ui, knob, track.input_gain_db);
    modified
}

fn knob_graphic(ui: &egui::Ui, rect: Rect, gain: f32) {
    let p = theme::palette(ui);
    let center = rect.center();
    ui.painter().circle_filled(center, 11.0, p.surface_raised);
    ui.painter()
        .circle_stroke(center, 11.0, Stroke::new(1.3, p.muted));
    let angle = -std::f32::consts::FRAC_PI_2 + gain / 24.0 * 2.1;
    let tip = center + Vec2::angled(angle) * 8.0;
    ui.painter().circle_filled(tip, 2.0, p.text);
}

fn strip(
    ui: &mut egui::Ui,
    r: Rect,
    track: &mut Track,
    index: usize,
    selected: bool,
    peak: f32,
    settings: &mut NativeWindowState,
    hide_routing: bool,
) -> (bool, bool, bool) {
    let mut modified = false;
    let mut select = false;
    let mut exclusive = false;
    let p = theme::palette(ui);
    ui.painter().rect_filled(r, 0.0, p.surface);
    let at = |x, y, w, h| Rect::from_min_size(r.min + Vec2::new(x, y), Vec2::new(w, h));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(r)
            .id_salt(track.id)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    {
        let ui = &mut child;
        ui.set_clip_rect(r);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                r.min + Vec2::new(1.0, 33.0),
                r.min + Vec2::new(67.0, 33.0),
                r.min + Vec2::new(57.0, 48.0),
                r.min + Vec2::new(1.0, 48.0),
            ],
            p.surface_raised,
            Stroke::NONE,
        ));

        if !hide_routing {
            routing(ui, at(0.0, 0.0, 82.0, 32.0), track, &mut modified);
        }
        text(
            ui,
            at(2.0, 33.0, 78.0, 15.0),
            if track.audio_channels == 1 {
                "mono"
            } else {
                "stereo"
            },
            11.0,
            p.muted,
            Align2::LEFT_CENTER,
        );
        modified |= input_row(ui, r, track);
        for right in [false, true] {
            let x = if right {
                r.right() - 7.0
            } else {
                r.left() + 5.0
            };
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(x - 3.0, r.top() + 80.0),
                    Pos2::new(x + 3.0, r.top() + 80.0),
                    Pos2::new(x, r.top() + 84.0),
                ],
                p.muted,
                Stroke::NONE,
            ));
        }

        // Every strip uses the same measured body origin. The footer follows window height.
        let body_bottom = r.height() - 31.0;
        let plate = at(2.0, 78.0, 80.0, (body_bottom - 82.0).max(256.0));
        let plate_color = if ui.visuals().dark_mode {
            Color32::from_gray(if selected { 102 } else { 58 })
        } else {
            Color32::from_gray(if selected { 224 } else { 183 })
        };
        ui.painter().rect_filled(plate, 4.0, plate_color);
        ui.painter().rect_stroke(
            plate,
            4.0,
            Stroke::new(1.0, p.muted),
            egui::StrokeKind::Inside,
        );
        let pan = at(4.0, 86.0, 74.0, 17.0);
        ui.painter().rect_filled(pan, 1.0, p.canvas);
        let pan_response = ui.interact(pan, ui.id().with("pan"), Sense::click_and_drag());
        if pan_response.dragged() {
            if let Some(pos) = pan_response.interact_pointer_pos() {
                track.pan = ((pos.x - pan.left()) / pan.width() * 2.0 - 1.0).clamp(-1.0, 1.0);
                modified = true;
            }
        }
        if pan_response.double_clicked() {
            track.pan = 0.0;
            modified = true;
        }
        let x = pan.center().x + track.pan * pan.width() * 0.46;
        ui.painter().line_segment(
            [
                Pos2::new(x, pan.top() + 2.0),
                Pos2::new(x, pan.bottom() - 2.0),
            ],
            Stroke::new(2.0, p.accent),
        );
        let pan_label = if track.pan.abs() < 0.005 {
            "C".to_owned()
        } else if track.pan < 0.0 {
            format!("L {:.0}", track.pan.abs() * 100.0)
        } else {
            format!("R {:.0}", track.pan * 100.0)
        };
        ui.painter()
            .rect_filled(at(4.0, 104.0, 31.0, 18.0), 1.0, p.canvas);
        text(
            ui,
            at(4.0, 104.0, 31.0, 18.0),
            &pan_label,
            11.0,
            p.text,
            Align2::CENTER_CENTER,
        );
        for (i, (label, active, tip, y)) in [
            ("M", track.muted, "Mute", 126.0),
            (
                "S",
                track.solo || track.solo_defeat,
                "Solo State (Solo Defeat with ALT click)",
                142.0,
            ),
            ("L", track.listen, "Listen", 160.0),
            ("R", track.automation_read, "Read Automation", 183.0),
            ("W", track.automation_write, "Write Automation", 200.0),
        ]
        .into_iter()
        .enumerate()
        {
            if control(
                ui,
                at(8.0, y, 21.0, 15.0),
                i,
                Icon::Text(label),
                active,
                false,
                true,
                Some(tip),
            )
            .clicked()
            {
                match i {
                    0 => track.muted = !track.muted,
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
                                exclusive = modifiers.ctrl;
                            }
                        }
                    }
                    2 => track.listen = !track.listen,
                    3 => track.automation_read = !track.automation_read,
                    _ => track.automation_write = !track.automation_write,
                }
                modified = true;
            }
        }
        if control(
            ui,
            at(8.0, 216.0, 21.0, 21.0),
            "edit",
            Icon::Edit,
            selected,
            true,
            true,
            Some("Edit Audio Channel Settings"),
        )
        .clicked()
        {
            select = true;
            settings.open = true;
            settings.focus_requested = true;
        }
        let has_inserts = !track.inserts.is_empty();
        let inserts_active = track.inserts.iter().any(|i| i.enabled);
        let insert = control(
            ui,
            at(8.0, 238.0, 21.0, 15.0),
            "inserts",
            Icon::Insert,
            inserts_active,
            false,
            has_inserts,
            Some("Inserts State (Bypass Inserts with click / Context-menu shows usage)"),
        );
        if insert.clicked() {
            for i in &mut track.inserts {
                i.enabled = !inserts_active;
            }
            modified = true;
        }
        insert.context_menu(|ui| {
            if track.inserts.is_empty() {
                ui.label("No Effect");
            } else {
                for effect in &track.inserts {
                    ui.label(super::insert_effect_name(&effect.kind));
                }
            }
        });
        let eq_active =
            !track.equalizer.bypassed && track.equalizer.bands.iter().any(|b| b.enabled);
        if control(
            ui,
            at(8.0, 256.0, 21.0, 15.0),
            "eq",
            Icon::Eq,
            eq_active,
            false,
            true,
            Some("EQs State (Bypass EQs with click)"),
        )
        .clicked()
        {
            track.equalizer.bypassed = !track.equalizer.bypassed;
            modified = true;
        }
        // Send audio processing is not implemented; expose the observed state and usage without a fake bypass.
        let send = control(
            ui,
            at(8.0, 273.0, 21.0, 15.0),
            "sends",
            Icon::Send,
            false,
            false,
            false,
            Some("Sends State (Bypass Sends with click / Context-menu shows usage)"),
        );
        send.context_menu(|ui| {
            for route in track.send_routing.iter().flatten() {
                ui.label(route);
            }
            if !track.send_routing.iter().any(Option::is_some) {
                ui.label("No Bus");
            }
        });
        if control(
            ui,
            at(8.0, body_bottom - 58.0, 21.0, 21.0),
            "monitor",
            Icon::Speaker,
            track.monitor,
            true,
            true,
            Some("Monitor"),
        )
        .clicked()
        {
            track.monitor = !track.monitor;
            modified = true;
        }
        if control(
            ui,
            at(5.0, body_bottom - 36.0, 27.0, 27.0),
            "record",
            Icon::Record,
            track.armed,
            true,
            true,
            Some("Record Enable"),
        )
        .clicked()
        {
            track.armed = !track.armed;
            modified = true;
        }
        modified |= fader(ui, r, &mut track.gain, peak, true);
        let name_rect = Rect::from_min_size(
            Pos2::new(r.left(), r.bottom() - 29.0),
            Vec2::new(r.width(), 16.0),
        );
        select |= ui
            .interact(name_rect, ui.id().with("select-channel"), Sense::click())
            .clicked();
        footer(ui, r, &track.name, Some(index + 1));
    }
    (modified, select, exclusive)
}

fn bus(ui: &mut egui::Ui, r: Rect, input: bool, _hide_routing: bool) {
    let p = theme::palette(ui);
    let at = |x, y, w, h| Rect::from_min_size(r.min + Vec2::new(x, y), Vec2::new(w, h));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(r)
            .id_salt(if input { "input-bus" } else { "output-bus" })
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    {
        let ui = &mut child;
        ui.set_clip_rect(r);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                r.min + Vec2::new(1.0, 33.0),
                r.min + Vec2::new(67.0, 33.0),
                r.min + Vec2::new(57.0, 48.0),
                r.min + Vec2::new(1.0, 48.0),
            ],
            p.surface_raised,
            Stroke::NONE,
        ));

        text(
            ui,
            at(2.0, 33.0, 78.0, 15.0),
            "stereo",
            11.0,
            p.muted,
            Align2::LEFT_CENTER,
        );
        control(
            ui,
            at(5.0, 53.0, 16.0, 15.0),
            "phase",
            Icon::Phase,
            false,
            false,
            false,
            Some("Input Phase"),
        );
        ui.painter()
            .rect_filled(at(24.0, 53.0, 30.0, 16.0), 1.0, p.canvas);
        text(
            ui,
            at(24.0, 53.0, 30.0, 16.0),
            "0.0",
            11.0,
            p.text,
            Align2::CENTER_CENTER,
        );
        ui.interact(
            at(24.0, 53.0, 30.0, 16.0),
            ui.id().with("gain"),
            Sense::hover(),
        )
        .on_hover_text("Input Gain");
        knob_graphic(ui, at(57.0, 48.0, 24.0, 24.0), 0.0);
        ui.interact(
            at(57.0, 48.0, 24.0, 24.0),
            ui.id().with("gain-knob"),
            Sense::hover(),
        )
        .on_hover_text("Input Gain (use shift)");
        let body_bottom = r.height() - 31.0;
        ui.painter().rect_stroke(
            at(5.0, 122.0, 30.0, (body_bottom - 127.0).max(180.0)),
            12.0,
            Stroke::new(1.0, p.muted),
            egui::StrokeKind::Inside,
        );
        ui.painter()
            .rect_filled(at(4.0, 86.0, 74.0, 17.0), 1.0, p.canvas);
        text(
            ui,
            at(4.0, 86.0, 74.0, 17.0),
            "no panner",
            10.0,
            p.muted,
            Align2::CENTER_CENTER,
        );
        for (i, (symbol, tip, y)) in [
            (Icon::Text("M"), "Mute", 126.0),
            (
                Icon::Text("S"),
                "Solo State (Solo Defeat with ALT click)",
                142.0,
            ),
            (Icon::Text("L"), "Listen", 160.0),
            (Icon::Text("R"), "Read Automation", 183.0),
            (Icon::Text("W"), "Write Automation", 200.0),
            (
                Icon::Edit,
                if input {
                    "Edit Input Channel Settings"
                } else {
                    "Edit Output Channel Settings"
                },
                216.0,
            ),
            (
                Icon::Insert,
                "Inserts State (Bypass Inserts with click / Context-menu shows usage)",
                238.0,
            ),
            (Icon::Eq, "EQs State (Bypass EQs with click)", 256.0),
        ]
        .into_iter()
        .enumerate()
        {
            if input && i == 1 {
                continue;
            }
            control(
                ui,
                at(10.0, y, 21.0, if i == 5 { 21.0 } else { 15.0 }),
                i,
                symbol,
                false,
                i == 5,
                false,
                Some(tip),
            );
        }
        let mut gain = 1.0;
        fader(ui, r, &mut gain, 0.0, false);
        footer(ui, r, if input { "Stereo In" } else { "Stereo Out" }, None);
    }
}

fn footer(ui: &mut egui::Ui, r: Rect, name: &str, number: Option<usize>) {
    let p = theme::palette(ui);
    let name_rect = Rect::from_min_size(
        Pos2::new(r.left(), r.bottom() - 29.0),
        Vec2::new(r.width() - 1.0, 16.0),
    );
    ui.painter().rect_filled(name_rect, 0.0, p.canvas);
    text(
        ui,
        name_rect.shrink2(Vec2::new(3.0, 0.0)),
        name,
        11.0,
        p.text,
        Align2::LEFT_CENTER,
    );
    let lower = Rect::from_min_size(
        Pos2::new(r.left(), r.bottom() - 12.0),
        Vec2::new(r.width() - 1.0, 12.0),
    );
    ui.painter().rect_filled(lower, 0.0, p.surface_raised);
    if let Some(n) = number {
        text(
            ui,
            lower.shrink2(Vec2::new(3.0, 0.0)),
            &n.to_string(),
            11.0,
            p.text,
            Align2::LEFT_CENTER,
        );
        icon(
            ui.painter(),
            Rect::from_min_size(
                lower.right_top() - Vec2::new(16.0, 0.0),
                Vec2::new(14.0, 12.0),
            ),
            Icon::Routing,
            p.muted,
        );
    }
}

fn fader(ui: &mut egui::Ui, r: Rect, gain: &mut f32, peak: f32, enabled: bool) -> bool {
    let p = theme::palette(ui);
    let top = r.top() + 104.0;
    let bottom = r.bottom() - 69.0;
    let plate = Rect::from_min_max(
        Pos2::new(r.left() + 39.0, top - 3.0),
        Pos2::new(r.right() - 2.0, bottom + 3.0),
    );
    let plate_color = if ui.visuals().dark_mode {
        Color32::from_gray(145)
    } else {
        Color32::from_gray(205)
    };
    ui.painter().rect_filled(plate, 2.0, plate_color);
    let rail = Rect::from_min_max(
        Pos2::new(r.left() + 49.0, top + 5.0),
        Pos2::new(r.left() + 51.0, bottom - 5.0),
    );
    ui.painter().rect_filled(rail, 0.0, p.surface);
    let meter = Rect::from_min_max(
        Pos2::new(r.left() + 62.0, top + 1.0),
        Pos2::new(r.right() - 5.0, bottom - 1.0),
    );
    ui.painter().rect_filled(meter, 0.0, p.canvas);
    for step in 0..=72 {
        let y = egui::lerp(meter.y_range(), step as f32 / 72.0);
        ui.painter().line_segment(
            [Pos2::new(meter.left(), y), Pos2::new(meter.right(), y)],
            Stroke::new(0.6, p.surface_raised),
        );
    }
    let fill = super::meter_fill_fraction(peak);
    if fill > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_max(
                Pos2::new(meter.left(), meter.bottom() - meter.height() * fill),
                meter.max,
            ),
            0.0,
            p.accent,
        );
    }
    for step in 0..=8 {
        let y = egui::lerp(rail.y_range(), step as f32 / 8.0);
        ui.painter().line_segment(
            [Pos2::new(r.left() + 42.0, y), Pos2::new(r.left() + 59.0, y)],
            Stroke::new(0.7, p.surface),
        );
    }
    // Only the observed unity position is calibrated. The current core gain range remains 0..1.
    const UNITY: f32 = 0.245;
    let fraction = if *gain <= 0.0 {
        1.0
    } else {
        (UNITY + (1.0 - UNITY) * (-super::linear_gain_db(*gain) / 60.0)).clamp(UNITY, 1.0)
    };
    let center = Pos2::new(r.left() + 50.0, egui::lerp(rail.y_range(), fraction));
    let handle = Rect::from_center_size(center, Vec2::new(14.0, 31.0));
    ui.painter()
        .rect_filled(handle, 6.0, Color32::from_gray(229));
    ui.painter().rect_stroke(
        handle,
        6.0,
        Stroke::new(1.0, Color32::from_gray(110)),
        egui::StrokeKind::Inside,
    );
    ui.painter().rect_filled(
        handle.shrink2(Vec2::new(2.0, 5.0)),
        4.0,
        Color32::from_gray(197),
    );
    ui.painter().line_segment(
        [
            Pos2::new(handle.left() + 1.0, center.y),
            Pos2::new(handle.right() - 1.0, center.y),
        ],
        Stroke::new(1.2, Color32::from_gray(40)),
    );
    let mut changed = false;
    if enabled {
        let response = ui.interact(
            Rect::from_min_max(
                Pos2::new(r.left() + 39.0, top),
                Pos2::new(r.left() + 61.0, bottom),
            ),
            ui.id().with("fader"),
            Sense::click_and_drag(),
        );
        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let frac = ((pos.y - rail.top()) / rail.height()).clamp(UNITY, 1.0);
                let db = -(frac - UNITY) / (1.0 - UNITY) * 60.0;
                let value = if frac >= 0.999 {
                    0.0
                } else {
                    10.0_f32.powf(db / 20.0)
                };
                changed |= *gain != value;
                *gain = value;
            }
        }
        if response.double_clicked() {
            changed |= *gain != 1.0;
            *gain = 1.0;
        }
    }
    let level = Rect::from_min_size(
        Pos2::new(r.left() + 39.0, r.bottom() - 65.0),
        Vec2::new(r.width() - 42.0, 29.0),
    );
    ui.painter().rect_filled(level, 2.0, p.canvas);
    text(
        ui,
        Rect::from_min_size(level.min, Vec2::new(level.width() - 2.0, 11.0)),
        if peak > 0.0 { "" } else { "−∞" },
        8.0,
        p.muted,
        Align2::RIGHT_CENTER,
    );
    text(
        ui,
        Rect::from_min_size(
            level.min + Vec2::new(2.0, 11.0),
            Vec2::new(level.width() - 4.0, 17.0),
        ),
        &format!("{:.2}", super::linear_gain_db(*gain)),
        11.0,
        p.text,
        Align2::LEFT_CENTER,
    );
    ui.interact(level, ui.id().with("channel-level"), Sense::hover())
        .on_hover_text("Channel Level");
    changed
}
