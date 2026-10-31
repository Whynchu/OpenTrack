//! Transport geometry follows workflow-L1/02-mixer-transport-1.png (837 x 69).
use super::*;

const SIZE: Vec2 = Vec2::new(837.0, 69.0);

fn rect(origin: Pos2, b: [f32; 4]) -> Rect {
    Rect::from_min_size(origin + Vec2::new(b[0], b[1]), Vec2::new(b[2], b[3]))
}
fn label(ui: &egui::Ui, r: Rect, value: &str, size: f32, color: Color32) {
    ui.painter().text(
        r.center(),
        Align2::CENTER_CENTER,
        value,
        FontId::proportional(size),
        color,
    );
}
fn pitch_arrow(ui: &mut egui::Ui, left: bool, tip: &str) -> egui::Response {
    let (response, painter) = ui.allocate_painter(Vec2::new(22.0, 20.0), Sense::click());
    let fill = theme::paint_control(
        ui,
        response.rect,
        &response,
        theme::toolbar_button_fill(ui.visuals()),
        ui.is_enabled(),
        false,
    );
    let direction = if left { -1.0 } else { 1.0 };
    let center = response.rect.center();
    let arrow_tip = center + Vec2::new(direction * 4.0, 0.0);
    let tail = center - Vec2::new(direction * 4.0, 0.0);
    let ink = project_controls::label_ink(fill);
    painter.line_segment([tail, arrow_tip], Stroke::new(1.5, ink));
    painter.line_segment(
        [arrow_tip, arrow_tip + Vec2::new(-direction * 3.0, -3.0)],
        Stroke::new(1.5, ink),
    );
    painter.line_segment(
        [arrow_tip, arrow_tip + Vec2::new(-direction * 3.0, 3.0)],
        Stroke::new(1.5, ink),
    );
    response.on_hover_text(tip)
}
fn field(ui: &egui::Ui, origin: Pos2, b: [f32; 4], value: &str, size: f32) {
    let r = rect(origin, b);
    let p = theme::palette(ui);
    ui.painter().rect_filled(r, 1.0, p.canvas);
    ui.painter().text(
        r.right_center() - Vec2::new(4.0, 0.0),
        Align2::RIGHT_CENTER,
        value,
        FontId::monospace(size),
        p.text,
    );
}
fn button(
    ui: &mut egui::Ui,
    origin: Pos2,
    id: &str,
    b: [f32; 4],
    title: &str,
    enabled: bool,
    selected: bool,
    tint: Color32,
    icon: Option<ToolbarIcon>,
    tip: &str,
) -> egui::Response {
    let r = rect(origin, b);
    let enabled = enabled && ui.is_enabled();
    let response = ui.interact(
        r,
        ui.id().with(id),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let p = theme::palette(ui);
    let raised = icon.is_some() || b[1] == 39.0;
    let fill = if selected {
        tint
    } else if raised {
        theme::toolbar_button_fill(ui.visuals())
    } else {
        p.surface
    };
    let fill = theme::paint_control(ui, r, &response, fill, enabled, false);
    let ink = if !enabled {
        p.muted
    } else if raised || selected {
        project_controls::label_ink(fill)
    } else if enabled {
        p.text
    } else {
        p.muted
    };
    if let Some(icon) = icon {
        crate::icons::paint_toolbar(ui, r, icon, ink);
    } else {
        label(ui, r, title, 10.0, ink);
    }
    if tip.is_empty() {
        response
    } else {
        response.on_hover_text(tip)
    }
}
fn meter(ui: &egui::Ui, origin: Pos2, x: f32, peak: f32, width: f32) {
    let p = theme::palette(ui);
    let r = rect(origin, [x, 5.0, width, 58.0]);
    ui.painter().rect_filled(r, 0.0, p.canvas);
    ui.painter()
        .rect_stroke(r, 0.0, Stroke::new(1.0, p.grid), egui::StrokeKind::Inside);
    let level = if peak > 0.0 {
        ((20.0 * peak.log10() + 60.0) / 66.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    for row in 0..14 {
        let cell = Rect::from_min_size(
            Pos2::new(r.left() + 1.0, r.bottom() - 3.0 - row as f32 * 4.0),
            Vec2::new(width - 2.0, 2.0),
        );
        let lit = (row as f32 + 0.5) / 14.0 <= level;
        ui.painter().rect_filled(
            cell,
            0.0,
            if lit {
                if row >= 13 { theme::RECORD } else { p.accent }
            } else {
                p.surface_raised
            },
        );
    }
}
fn seek(app: &mut OpenTrackApp, beat: f64) {
    app.data.transport.position_beats = beat.max(0.0);
    if let Some(engine) = &app.audio_engine {
        engine.seek_seconds(
            app.data.transport.position_beats * 60.0 / app.data.project.tempo_bpm.max(1.0),
        );
    }
}

fn move_embedded_panel(bounds: &mut Option<WindowBounds>, panel: Rect, delta: Vec2) {
    let bounds = bounds.get_or_insert_with(|| docked_bounds(panel));
    bounds.left += f64::from(delta.x);
    bounds.top += f64::from(delta.y);
}

pub(super) fn show(app: &mut OpenTrackApp, ui: &mut egui::Ui, embedded: bool) -> bool {
    let (panel, panel_response) = ui.allocate_exact_size(SIZE, Sense::click_and_drag());
    let drag_handle_rect = Rect::from_min_size(
        Pos2::new(panel.center().x - 15.0, panel.top()),
        Vec2::new(30.0, 5.0),
    );
    let drag_handle = ui.interact(
        drag_handle_rect,
        ui.id().with("transport-panel-drag-handle"),
        Sense::click_and_drag(),
    );
    if embedded && (panel_response.dragged() || drag_handle.dragged()) {
        // The Wayland fallback is an egui window inside Project, not a native
        // viewport. Move its persisted position directly because StartDrag
        // only applies to native viewports and the panel owns this drag.
        let delta = if drag_handle.dragged() {
            drag_handle.drag_delta()
        } else {
            panel_response.drag_delta()
        };
        move_embedded_panel(&mut app.transport_window.bounds, panel, delta);
    } else if !embedded && (panel_response.drag_started() || drag_handle.drag_started()) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    let panel_dragged = panel_response.dragged() || drag_handle.dragged();
    panel_response.clone().context_menu(|ui| {
        theme::menu_popup_style(ui.style_mut());
        ui.set_min_width(190.0);
        if ui
            .checkbox(&mut app.transport_window.always_on_top, "Always on Top")
            .changed()
        {
            ui.close();
        }
    });
    let o = panel.min;
    let p = theme::palette(ui);
    ui.painter().rect_filled(panel, 1.0, p.surface_raised);
    ui.painter().rect_stroke(
        panel,
        1.0,
        Stroke::new(1.0, p.grid),
        egui::StrokeKind::Inside,
    );
    let grip = Rect::from_min_size(
        Pos2::new(panel.center().x - 7.0, panel.top() + 1.0),
        Vec2::new(14.0, 1.5),
    );
    ui.painter().rect_filled(grip, 1.0, p.muted);
    drag_handle
        .clone()
        .on_hover_text("Drag here to move the Transport panel");
    for x in [17.0, 127.0, 247.0, 524.0, 644.0, 741.0, 821.0] {
        ui.painter().line_segment(
            [o + Vec2::new(x, 3.0), o + Vec2::new(x, 65.0)],
            Stroke::new(1.0, p.grid),
        );
    }
    for (x, peak) in [
        (22.0, app.data.input_channel_meter_levels[0]),
        (28.0, app.data.input_channel_meter_levels[1]),
    ] {
        meter(ui, o, x, peak, 5.0);
    }
    button(
        ui,
        o,
        "record-mode",
        [41.0, 5.0, 82.0, 20.0],
        "Normal  ▾",
        false,
        false,
        p.accent,
        None,
        "Linear Record Mode",
    );
    button(
        ui,
        o,
        "midi-record-mode",
        [41.0, 27.0, 82.0, 20.0],
        "Mix (MIDI) ▾",
        false,
        false,
        p.accent,
        None,
        "Cycle Record Mode",
    );
    button(
        ui,
        o,
        "auto-quantize",
        [41.0, 50.0, 34.0, 14.0],
        "AUTO",
        false,
        false,
        p.accent,
        None,
        "Automatic MIDI Record Quantize",
    );
    label(ui, rect(o, [76.0, 50.0, 47.0, 14.0]), "OFF", 10.0, p.muted);
    // Locator labels and the two independent position rows retain their reference grouping.
    let (left, right) = if app.data.project.cycle_mode == CycleMode::Skip {
        (
            app.data.project.loop_end_beats,
            app.data.project.loop_start_beats,
        )
    } else {
        (
            app.data.project.loop_start_beats,
            app.data.project.loop_end_beats,
        )
    };
    for (title, y, beat) in [("L", 5.0, left), ("R", 36.0, right)] {
        label(ui, rect(o, [130.0, y, 18.0, 15.0]), title, 11.0, p.text);
        field(
            ui,
            o,
            [150.0, y, 94.0, 17.0],
            &format_transport_position(
                beat,
                app.data.project.beats_per_bar,
                app.data.project.beat_unit,
            ),
            12.0,
        );
        field(ui, o, [150.0, y + 18.0, 94.0, 12.0], "0. 0. 0", 9.0);
        ui.painter().line_segment(
            [
                o + Vec2::new(133.0, y + 20.0),
                o + Vec2::new(144.0, y + 25.0),
            ],
            Stroke::new(1.0, p.muted),
        );
    }
    let beat = app.data.transport.position_beats;
    field(
        ui,
        o,
        [251.0, 5.0, 159.0, 23.0],
        &format_transport_position(
            beat,
            app.data.project.beats_per_bar,
            app.data.project.beat_unit,
        ),
        16.0,
    );
    let ms = (beat.max(0.0) * 60.0 / app.data.project.tempo_bpm.max(1.0) * 1000.0).round() as u64;
    field(
        ui,
        o,
        [412.0, 5.0, 108.0, 23.0],
        &format!(
            "{}:{:02}:{:02}.{:03}",
            ms / 3_600_000,
            ms / 60_000 % 60,
            ms / 1000 % 60,
            ms % 1000
        ),
        11.0,
    );
    ui.painter()
        .rect_filled(rect(o, [251.0, 30.0, 269.0, 5.0]), 0.0, p.canvas);
    let cycle_color = crate::timeline::cycle_color(app.data.project.cycle_mode);
    let previous = button(
        ui,
        o,
        "previous",
        [251.0, 39.0, 21.0, 25.0],
        "",
        true,
        false,
        p.accent,
        Some(ToolbarIcon::Previous),
        "Goto Previous Marker / Zero",
    );
    let rewind = button(
        ui,
        o,
        "rewind",
        [273.0, 39.0, 36.0, 25.0],
        "",
        true,
        false,
        p.accent,
        None,
        "Rewind",
    );
    let forward = button(
        ui,
        o,
        "forward",
        [310.0, 39.0, 36.0, 25.0],
        "",
        true,
        false,
        p.accent,
        None,
        "Fast Forward",
    );
    // Draw doubled simple triangles rather than relying on font-specific transport symbols.
    for (r, left) in [(rewind.rect, true), (forward.rect, false)] {
        for dx in [-4.5, 4.5] {
            let c = r.center() + Vec2::new(dx, 0.0);
            let d = if left { -1.0 } else { 1.0 };
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(4.0 * d, 0.0),
                    c + Vec2::new(-3.0 * d, -5.0),
                    c + Vec2::new(-3.0 * d, 5.0),
                ],
                project_controls::label_ink(theme::toolbar_button_fill(ui.visuals())),
                Stroke::NONE,
            ));
        }
    }
    button(
        ui,
        o,
        "next",
        [347.0, 39.0, 21.0, 25.0],
        "",
        false,
        false,
        p.accent,
        Some(ToolbarIcon::Next),
        "Goto Next Marker / Project End",
    );
    let cycle = button(
        ui,
        o,
        "cycle",
        [369.0, 39.0, 37.0, 25.0],
        "",
        true,
        app.data.transport.cycle,
        cycle_color,
        Some(ToolbarIcon::Cycle),
        "Cycle",
    );
    let stop = button(
        ui,
        o,
        "stop",
        [407.0, 39.0, 37.0, 25.0],
        "",
        true,
        !app.data.transport.playing,
        theme::WARNING,
        Some(ToolbarIcon::Stop),
        "Stop",
    );
    let play = button(
        ui,
        o,
        "play",
        [445.0, 39.0, 37.0, 25.0],
        "",
        true,
        app.data.transport.playing,
        p.accent,
        Some(ToolbarIcon::Play),
        "Play",
    );
    let record = button(
        ui,
        o,
        "record",
        [483.0, 39.0, 37.0, 25.0],
        "",
        true,
        app.data.transport.recording,
        theme::RECORD,
        Some(ToolbarIcon::Record),
        "Record",
    );
    let click = button(
        ui,
        o,
        "click",
        [528.0, 5.0, 35.0, 14.0],
        "CLICK",
        true,
        app.metronome_enabled,
        p.accent,
        None,
        "Metronome/Click (use CTRL to open metronome settings)",
    );
    label(
        ui,
        rect(o, [565.0, 5.0, 34.0, 14.0]),
        if app.metronome_enabled { "ON" } else { "OFF" },
        10.0,
        if app.metronome_enabled {
            p.text
        } else {
            p.muted
        },
    );
    button(
        ui,
        o,
        "precount",
        [620.0, 5.0, 19.0, 14.0],
        "II◀",
        false,
        false,
        p.accent,
        None,
        "",
    );
    for (bounds, title) in [
        ([528.0, 23.0, 38.0, 12.0], "TEMPO"),
        ([567.0, 23.0, 43.0, 12.0], "TRACK"),
        ([528.0, 51.0, 35.0, 12.0], "SYNC"),
        ([565.0, 51.0, 27.0, 12.0], "INT."),
    ] {
        label(ui, rect(o, bounds), title, 9.0, p.muted);
    }
    let signature_rect = rect(o, [611.0, 23.0, 29.0, 12.0]);
    let signature = ui.interact(
        signature_rect,
        ui.id().with("tempo-signature"),
        Sense::click(),
    );
    ui.painter()
        .rect_filled(signature_rect, 1.0, theme::palette(ui).canvas);
    label(
        ui,
        signature_rect,
        &format!(
            "{}/{}",
            app.data.project.beats_per_bar, app.data.project.beat_unit
        ),
        10.0,
        theme::palette(ui).text,
    );
    let signature = signature.on_hover_text("Edit time signature");
    if signature.clicked() {
        app.tempo_signature_settings_open = !app.tempo_signature_settings_open;
    }
    let tempo_rect = rect(o, [567.0, 35.0, 73.0, 15.0]);
    let tempo_pointer_down = ui.input(|input| {
        input.pointer.primary_down()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|position| tempo_rect.contains(position))
    });
    if tempo_pointer_down {
        app.project_history.begin(&app.data.project);
    }
    let old_tempo = app.data.project.tempo_bpm;
    let tempo_response = ui.put(
        tempo_rect,
        egui::DragValue::new(&mut app.data.project.tempo_bpm)
            .range(1.0..=1000.0)
            .speed(0.1)
            .fixed_decimals(2),
    );
    if tempo_response.changed() {
        app.project_history.begin(&app.data.project);
        app.project_history.commit_after_frame = true;
        if app.data.project.tempo_bpm != old_tempo {
            app.project_timing_changed();
        }
    }
    label(
        ui,
        rect(o, [595.0, 51.0, 45.0, 12.0]),
        "Offline",
        9.0,
        p.muted,
    );
    button(
        ui,
        o,
        "show-markers",
        [648.0, 5.0, 30.0, 14.0],
        "SHOW",
        false,
        false,
        p.accent,
        None,
        "Open Markers",
    );
    label(
        ui,
        rect(o, [681.0, 5.0, 56.0, 14.0]),
        "MARKER",
        10.0,
        p.muted,
    );
    for row in 0..3 {
        for col in 0..5 {
            let marker = row * 5 + col + 1;
            label(
                ui,
                rect(
                    o,
                    [
                        648.0 + col as f32 * 18.0,
                        24.0 + row as f32 * 12.0,
                        16.0,
                        11.0,
                    ],
                ),
                &marker.to_string(),
                10.0,
                p.muted,
            );
        }
    }
    for (x, peak) in [
        (746.0, app.data.output_channel_meter_levels[0]),
        (755.0, app.data.output_channel_meter_levels[1]),
    ] {
        meter(ui, o, x, peak, 6.0);
    }
    for col in 0..2 {
        for row in 0..4 {
            let r = rect(
                o,
                [
                    766.0 + col as f32 * 18.0,
                    5.0 + row as f32 * 15.0,
                    16.0,
                    12.0,
                ],
            );
            ui.painter().rect_filled(r, 0.0, p.canvas);
            ui.painter()
                .rect_stroke(r, 0.0, Stroke::new(1.0, p.grid), egui::StrokeKind::Inside);
        }
    }
    let gain_rect = rect(o, [806.0, 5.0, 12.0, 58.0]);
    let gain = ui.interact(
        gain_rect,
        ui.id().with("output-gain"),
        Sense::click_and_drag(),
    );
    ui.painter().rect_filled(gain_rect, 1.0, p.canvas);
    let position = channel_gain::gain_to_position(app.data.project.output_channel.strip.gain);
    let knob = Rect::from_center_size(
        Pos2::new(
            gain_rect.center().x,
            gain_rect.top() + 4.0 + position * (gain_rect.height() - 8.0),
        ),
        Vec2::new(8.0, 7.0),
    );
    ui.painter()
        .rect_filled(knob, 2.0, if gain.hovered() { p.text } else { p.muted });
    if (gain.clicked() || gain.dragged())
        && let Some(pos) = gain.interact_pointer_pos()
    {
        app.data.project.output_channel.strip.gain = channel_gain::position_to_gain(
            ((pos.y - gain_rect.top() - 4.0) / (gain_rect.height() - 8.0)).clamp(0.0, 1.0),
        );
        app.rebuild_audio_plan();
        app.mark_modified();
    }
    if click.clicked() {
        if ui.input(|i| i.modifiers.ctrl) {
            app.metronome_settings_open = true;
        } else {
            app.toggle_metronome();
        }
    }
    if previous.clicked() {
        app.set_transport_playing(false);
        seek(app, app.data.project.loop_start_beats);
    }
    if rewind.clicked() || forward.clicked() {
        seek(app, beat + if rewind.clicked() { -1.0 } else { 1.0 });
    }
    if cycle.clicked() {
        app.toggle_cycle();
    }
    if stop.clicked() {
        app.set_transport_playing(false);
        app.data.transport.recording = false;
    }
    if play.clicked() {
        if app.is_recording() {
            app.finish_recording();
        }
        app.set_transport_playing(true);
        app.data.transport.recording = false;
    }
    if record.clicked() {
        app.toggle_recording();
    }
    panel_dragged
}

pub(super) fn settings(app: &mut OpenTrackApp, ctx: &egui::Context) {
    if !app.metronome_settings_open {
        return;
    }
    let mut open = true;
    let mut changed = false;
    egui::Window::new("Metronome Setup")
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            changed |= ui
                .checkbox(
                    &mut app.metronome_enabled,
                    "Enable click during playback and recording",
                )
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut app.metronome_level, 0.0..=1.0).text("Click level"))
                .changed();
            ui.horizontal(|ui| {
                ui.label("Downbeat pitch");
                if pitch_arrow(ui, true, "Lower click pitch").clicked() {
                    app.metronome_pitch_midi = app.metronome_pitch_midi.saturating_sub(1).max(24);
                    changed = true;
                }
                ui.label(format!(
                    "{} / {}",
                    midi_pitch_name(app.metronome_pitch_midi),
                    midi_pitch_name(app.metronome_pitch_midi.saturating_sub(12))
                ));
                if pitch_arrow(ui, false, "Raise click pitch").clicked() {
                    app.metronome_pitch_midi = app.metronome_pitch_midi.saturating_add(1).min(108);
                    changed = true;
                }
            });
            ui.label("Tempo and time signature follow the project.");
        });
    app.metronome_settings_open = open;
    if changed {
        app.rebuild_audio_plan();
    }
}

fn midi_pitch_name(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
    ];
    format!("{}{}", NAMES[usize::from(note % 12)], note / 12 - 1)
}

pub(super) fn tempo_signature_settings(app: &mut OpenTrackApp, ctx: &egui::Context) {
    if !app.tempo_signature_settings_open {
        return;
    }
    if ctx.input(|input| input.pointer.primary_down()) {
        app.project_history.begin(&app.data.project);
    }
    let mut open = true;
    let mut changed = false;
    egui::Window::new("Tempo & Time Signature")
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Tempo");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut app.data.project.tempo_bpm)
                            .range(1.0..=1000.0)
                            .speed(0.1)
                            .fixed_decimals(2)
                            .suffix(" BPM"),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Time signature");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut app.data.project.beats_per_bar)
                            .range(1..=32)
                            .speed(1),
                    )
                    .changed();
                ui.label("/");
                egui::ComboBox::from_id_salt("time-signature-unit")
                    .selected_text(app.data.project.beat_unit.to_string())
                    .show_ui(ui, |ui| {
                        for unit in [1, 2, 4, 8, 16, 32] {
                            changed |= ui
                                .selectable_value(
                                    &mut app.data.project.beat_unit,
                                    unit,
                                    unit.to_string(),
                                )
                                .changed();
                        }
                    });
            });
        });
    app.tempo_signature_settings_open = open;
    if changed {
        app.project_history.commit_after_frame = true;
        app.project_timing_changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_transport_drag_applies_delta_to_saved_position() {
        let mut bounds = None;
        let panel = Rect::from_min_size(Pos2::new(100.0, 80.0), SIZE);
        move_embedded_panel(&mut bounds, panel, Vec2::new(36.0, 24.0));
        let moved = bounds
            .as_ref()
            .expect("first drag establishes the saved position");
        assert!((moved.left - 136.0).abs() < 0.1);
        assert!((moved.top - 104.0).abs() < 0.1);

        move_embedded_panel(&mut bounds, panel, Vec2::new(-6.0, 12.0));
        let moved = bounds.as_ref().unwrap();
        assert!((moved.left - 130.0).abs() < 0.1);
        assert!((moved.top - 116.0).abs() < 0.1);
    }

    #[test]
    fn click_control_toggles_real_state_and_ctrl_opens_setup_without_project_edit() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        let original = app.data.project.clone();
        let mut frame = |mut events: Vec<egui::Event>, modifiers| {
            events.insert(0, egui::Event::ModifiersChanged(modifiers));
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 100.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    show(&mut app, ui, false);
                },
            );
            output.textures_delta.clear();
            output
        };
        let output = frame(vec![], egui::Modifiers::NONE);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "CLICK" => {
                    Some(t.pos + t.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap();
        for modifiers in [egui::Modifiers::NONE, egui::Modifiers::CTRL] {
            for pressed in [true, false] {
                frame(
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers,
                        },
                    ],
                    modifiers,
                );
            }
        }
        drop(frame);
        assert!(app.metronome_enabled);
        assert!(app.metronome_settings_open);
        assert_eq!(app.data.project, original);
        assert!(app.audio_engine.is_none() && app.audio_input_probe.is_none());
    }

    #[test]
    fn transport_tick_display_carries_at_one_hundred_without_widening() {
        assert_eq!(
            super::super::format_transport_position(0.2475, 4, 4),
            "1.1.1.99"
        );
        assert_eq!(
            super::super::format_transport_position(0.249, 4, 4),
            "1.1.2.00"
        );
        assert_eq!(
            super::super::format_transport_position(0.999, 4, 4),
            "1.2.1.00"
        );
        assert_eq!(
            super::super::format_transport_position(3.999, 4, 4),
            "2.1.1.00"
        );
        assert_eq!(
            super::super::format_transport_position(5.0, 3, 4),
            "2.3.1.00"
        );
        assert_eq!(
            super::super::format_transport_position(3.0, 6, 8),
            "2.1.1.00"
        );
        assert_eq!(
            super::super::format_transport_position(1.0, 6, 8),
            "1.3.1.00"
        );
    }

    #[test]
    fn click_pitch_labels_use_plain_ascii_pitch_names() {
        assert_eq!(super::midi_pitch_name(87), "Eb6");
        assert_eq!(super::midi_pitch_name(75), "Eb5");
    }

    #[test]
    fn project_timing_change_marks_project_and_keeps_cursor_in_musical_time() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.transport.position_beats = 7.25;
        app.data.project.tempo_bpm = 96.0;
        app.data.project.beats_per_bar = 6;
        app.data.project.beat_unit = 8;
        app.project_timing_changed();
        assert_eq!(app.data.transport.position_beats, 7.25);
        assert!(app.modified);
    }
}
