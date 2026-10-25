//! Lower-zone Sample Editor. Pitch analysis/warp processing are explicitly deferred.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct SnapSettings {
    pub enabled: bool,
    pub grid: crate::timeline::GridType,
    pub quantize: crate::timeline::QuantizeSetting,
}
impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            grid: crate::timeline::GridType::UseQuantize,
            quantize: Default::default(),
        }
    }
}
impl SnapSettings {
    fn step(&self, beats_per_bar: u8) -> Option<f64> {
        self.enabled
            .then(|| self.grid.step_beats(beats_per_bar, self.quantize))
    }
}
pub(super) struct State {
    pub target: Option<(u64, u64)>,
    view: Option<(f64, f64)>,
    owner: Option<(u64, u64)>,
    selection: Option<(f64, f64)>,
    toolbox: Option<Pos2>,
    message: String,
    musical: bool,
    pub snap: SnapSettings,
    ruler_drag: Option<(Pos2, f64, f64, f64, f64)>,
    pub event_focus: bool,
    pub cycle_enabled: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            target: None,
            view: None,
            owner: None,
            selection: None,
            toolbox: None,
            message: String::new(),
            musical: true,
            snap: SnapSettings::default(),
            ruler_drag: None,
            event_focus: false,
            cycle_enabled: false,
        }
    }
}
fn local_time(project_beat: f64, clip: &opentrack_core::project::AudioClip, tempo: f64) -> f64 {
    (project_beat - clip.start_beats) * 60.0 / tempo
}
fn musical_position(beat: f64, beats_per_bar: u8) -> String {
    format_transport_position(beat, beats_per_bar)
}

fn waveform_channels(track: u8, source: Option<&AudioSource>) -> usize {
    source
        .map_or(track.max(1) as usize, |s| {
            (track.max(1) as usize).min(s.channels().max(1) as usize)
        })
        .clamp(1, 2)
}
fn toolbar_tool(
    ui: &mut egui::Ui,
    tool: &mut EditTool,
    value: EditTool,
    icon: ToolbarIcon,
    label: &str,
) {
    if toolbar_icon_button(ui, icon, label, *tool == value, true).clicked() {
        *tool = value;
    }
}
fn field<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    width: f32,
    label: &str,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    speed: f64,
    enabled: bool,
) -> bool {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.label(
            egui::RichText::new(label)
                .size(9.0)
                .color(theme::palette(ui).muted),
        );
        ui.add_enabled(
            enabled,
            egui::DragValue::new(value)
                .range(range)
                .clamp_existing_to_range(false)
                .speed(speed),
        )
        .changed()
    })
    .inner
}
pub(super) fn show(
    ui: &mut egui::Ui,
    project: &mut Project,
    sources: &HashMap<PathBuf, AudioSource>,
    state: &mut State,
    tool: &mut EditTool,
    _project_grid: Option<f64>,
    enabled: bool,
    transport: &mut f64,
    playing: bool,
) -> bool {
    let Some((track_id, clip_id)) = state.target else {
        ui.label("Double-click an audio event to open it in the Sample Editor.");
        return false;
    };
    let Some(track) = project.tracks.iter().find(|t| t.id == track_id) else {
        ui.label("The edited track is no longer in the project.");
        return false;
    };
    let Some(original) = track.clips.iter().find(|c| c.id == clip_id).cloned() else {
        ui.label(
            "The edited event is no longer in the project. Undo or double-click another event.",
        );
        return false;
    };
    let color = project_controls::track_color(track);
    let source = sources.get(&original.source_path);
    let channels = waveform_channels(track.audio_channels, source);
    if state.owner != state.target {
        state.owner = state.target;
        state.view = None;
        state.selection = None;
        state.message.clear();
    }
    let mut edited = original.clone();
    let mut split = false;
    let bounds = ui.available_rect_before_wrap();
    let p = theme::palette(ui);
    let toolbar = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
    let info = Rect::from_min_size(
        Pos2::new(bounds.left(), toolbar.bottom() + 2.0),
        Vec2::new(bounds.width(), 38.0),
    );
    let overview = Rect::from_min_size(
        Pos2::new(bounds.left(), info.bottom() + 3.0),
        Vec2::new(bounds.width(), 28.0),
    );
    let sidebar = Rect::from_min_max(
        Pos2::new(bounds.left(), overview.bottom() + 2.0),
        Pos2::new(bounds.left() + 148.0, bounds.bottom() - 22.0),
    );
    let ruler = Rect::from_min_size(
        Pos2::new(sidebar.right() + 5.0, sidebar.top()),
        Vec2::new((bounds.right() - sidebar.right() - 5.0).max(1.0), 22.0),
    );
    let wave = Rect::from_min_max(
        ruler.left_bottom(),
        Pos2::new(
            bounds.right(),
            (bounds.bottom() - 22.0).max(ruler.bottom() + 10.0),
        ),
    );
    ui.painter().rect_filled(toolbar, 0.0, p.surface_raised);
    ui.painter().rect_filled(info, 0.0, p.surface);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(toolbar.shrink2(Vec2::new(4.0, 2.0))),
        |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
            ui.horizontal(|ui| {
                toolbar_tool(
                    ui,
                    tool,
                    EditTool::Select,
                    ToolbarIcon::Select,
                    "Object Selection",
                );
                toolbar_tool(
                    ui,
                    tool,
                    EditTool::Range,
                    ToolbarIcon::Range,
                    "Range Selection",
                );
                toolbar_tool(ui, tool, EditTool::Split, ToolbarIcon::Split, "Scissors");
                if toolbar_icon_button(ui, ToolbarIcon::Zoom, "Fit Event", false, true).clicked() {
                    state.view = None;
                }
                ui.add_space(9.0);
                egui::ComboBox::from_id_salt("sample-timebase")
                    .width(100.0)
                    .selected_text(if state.musical {
                        "Bars + Beats"
                    } else {
                        "Seconds"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.musical, true, "Bars + Beats");
                        ui.selectable_value(&mut state.musical, false, "Seconds");
                    });
                ui.label(format!(
                    "Tempo {:.2}   {}/4",
                    project.tempo_bpm, project.beats_per_bar
                ));
                ui.add_space(8.0);
                ui.checkbox(&mut state.snap.enabled, "Snap");
                egui::ComboBox::from_id_salt("editor-grid-type")
                    .width(88.0)
                    .selected_text(state.snap.grid.label())
                    .show_ui(ui, |ui| {
                        for value in crate::timeline::GridType::ALL {
                            ui.selectable_value(&mut state.snap.grid, value, value.label());
                        }
                    });
                egui::ComboBox::from_id_salt("editor-quantize")
                    .width(70.0)
                    .selected_text(state.snap.quantize.label())
                    .show_ui(ui, |ui| {
                        for value in crate::timeline::QuantizeNote::ALL {
                            ui.selectable_value(
                                &mut state.snap.quantize.note,
                                value,
                                value.label(),
                            );
                        }
                        ui.separator();
                        for value in [
                            crate::timeline::QuantizeVariation::Straight,
                            crate::timeline::QuantizeVariation::Triplet,
                            crate::timeline::QuantizeVariation::Dotted,
                        ] {
                            ui.selectable_value(
                                &mut state.snap.quantize.variation,
                                value,
                                match value {
                                    crate::timeline::QuantizeVariation::Straight => "Straight",
                                    crate::timeline::QuantizeVariation::Triplet => "Triplet",
                                    crate::timeline::QuantizeVariation::Dotted => "Dotted",
                                },
                            );
                        }
                    });
                if ui
                    .add_enabled(
                        enabled && state.selection.is_some(),
                        egui::Button::new("Split at Selection"),
                    )
                    .clicked()
                {
                    split = true;
                }
                ui.label(format!(
                    "Cursor {}",
                    musical_position(*transport, project.beats_per_bar)
                ));
            });
        },
    );
    let grid = state.snap.step(project.beats_per_bar);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(info.shrink2(Vec2::new(4.0, 0.0))),
        |ui| {
            let width = 106.0_f32.min((info.width() - 45.0) / 6.0);
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 0.0);
            ui.horizontal_top(|ui| {
                field(
                    ui,
                    width,
                    "Position (beats)",
                    &mut edited.start_beats,
                    0.0..=1e8,
                    0.01,
                    enabled,
                );
                field(
                    ui,
                    width,
                    "Source Start (s)",
                    &mut edited.source_offset_seconds,
                    0.0..=1e8,
                    0.001,
                    enabled,
                );
                field(
                    ui,
                    width,
                    "Length (s)",
                    &mut edited.duration_seconds,
                    1.0 / project.sample_rate_hz.max(1) as f64..=1e8,
                    0.001,
                    enabled,
                );
                let mut db = if original.gain > 0.0 {
                    20.0 * original.gain.log10()
                } else {
                    -60.0
                };
                if field(
                    ui,
                    width,
                    "Volume (dB)",
                    &mut db,
                    -60.0..=12.0412,
                    0.1,
                    enabled,
                ) {
                    edited.gain = if db <= -60.0 {
                        0.0
                    } else {
                        10.0_f32.powf(db / 20.0)
                    };
                }
                field(
                    ui,
                    width,
                    "Fade In (s)",
                    &mut edited.fade_in_seconds,
                    0.0..=original.duration_seconds,
                    0.001,
                    enabled,
                );
                field(
                    ui,
                    width,
                    "Fade Out (s)",
                    &mut edited.fade_out_seconds,
                    0.0..=original.duration_seconds,
                    0.001,
                    enabled,
                );
            });
        },
    );
    ui.painter().rect_filled(sidebar, 0.0, p.surface_raised);
    ui.scope_builder(egui::UiBuilder::new().max_rect(sidebar.shrink(4.0)), |ui| {
        ui.set_clip_rect(sidebar);
        ui.spacing_mut().item_spacing = Vec2::new(3.0, 4.0);
        let title = ui.allocate_space(Vec2::new(sidebar.width() - 8.0, 22.0)).1;
        ui.painter().rect_filled(title, 0.0, color);
        ui.painter().with_clip_rect(title).text(
            title.left_center() + Vec2::new(4.0, 0.0),
            Align2::LEFT_CENTER,
            &original.name,
            FontId::proportional(11.0),
            project_controls::label_ink(color),
        );
        egui::CollapsingHeader::new("Definition")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(if channels == 1 { "Mono" } else { "Stereo" });
                if let Some(source) = source {
                    ui.label(format!("{} Hz", source.sample_rate()));
                }
                ui.label(format!("{:.3} s", original.duration_seconds));
            });
        ui.separator();
        ui.add_enabled(false, egui::Button::new("Pitch & Time"));
        ui.label(
            egui::RichText::new("Pitch analysis and correction: planned")
                .small()
                .color(p.muted),
        );
        egui::CollapsingHeader::new("Range")
            .default_open(true)
            .show(ui, |ui| {
                if let Some((a, b)) = state.selection {
                    ui.label(format!("{:.3} - {:.3} s", a.min(b), a.max(b)));
                    ui.label(format!("Length {:.3} s", (b - a).abs()));
                } else {
                    ui.label("No range selected");
                }
            });
    });
    ui.painter().rect_filled(wave, 0.0, p.canvas);
    ui.painter()
        .rect_filled(wave, 0.0, color.gamma_multiply(0.15));
    ui.painter().rect_filled(overview, 0.0, p.canvas);
    ui.painter()
        .rect_filled(overview, 0.0, color.gamma_multiply(0.2));
    ui.painter().rect_filled(ruler, 0.0, p.surface_raised);
    let minimum_start = -original.start_beats * 60.0 / project.tempo_bpm;
    let max_span = (1_000_000.0 - original.start_beats).max(16.0) * 60.0 / project.tempo_bpm;
    let (mut start, mut end) = state.view.unwrap_or((0.0, original.duration_seconds));
    let local_cursor = local_time(*transport, &original, project.tempo_bpm);
    if playing
        && !ui.input(|i| i.pointer.any_down())
        && local_cursor >= 0.0
        && local_cursor < original.duration_seconds
        && (local_cursor < start || local_cursor > end)
    {
        let span = end - start;
        start = (local_cursor - span * 0.15).max(minimum_start);
        end = start + span;
    }
    let response = ui.interact(wave, ui.id().with("sample-wave"), Sense::click_and_drag());
    if response.hovered() {
        let delta = ui.input(|i| i.smooth_scroll_delta.y);
        if delta != 0.0 {
            let anchor = response
                .hover_pos()
                .map(|p| ((p.x - wave.left()) / wave.width()) as f64)
                .unwrap_or(0.5)
                .clamp(0.0, 1.0);
            let span = ((end - start) * (-delta as f64 * 0.004).exp())
                .clamp(0.001_f64.min(original.duration_seconds), max_span);
            start = (start + anchor * (end - start) - anchor * span).max(minimum_start);
            end = start + span;
        }
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        let span = end - start;
        start = (start - ui.input(|i| i.pointer.delta().x) as f64 / wave.width() as f64 * span)
            .max(minimum_start);
        end = start + span;
        ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
    }
    let overview_response = ui.interact(
        overview,
        ui.id().with("sample-overview"),
        Sense::click_and_drag(),
    );
    if overview_response.clicked() || overview_response.dragged() {
        if let Some(pos) = overview_response.interact_pointer_pos() {
            let span = end - start;
            start = (((pos.x - overview.left()) / overview.width()) as f64
                * original.duration_seconds
                - span * 0.5)
                .clamp(0.0, (original.duration_seconds - span).max(0.0));
            end = start + span;
        }
    }
    state.view = Some((start, end));
    let event_duration = original.duration_seconds;
    let to_seconds = move |x: f32| {
        (start + ((x - wave.left()) / wave.width()) as f64 * (end - start))
            .clamp(0.0, event_duration)
    };
    let snap_local = |seconds: f64| {
        let beat = original.start_beats + seconds * project.tempo_bpm / 60.0;
        let beat = grid.map_or(beat, |step| crate::timeline::snap_beats_to_grid(beat, step));
        ((beat - original.start_beats) * 60.0 / project.tempo_bpm)
            .clamp(0.0, original.duration_seconds)
    };
    if response.clicked_by(egui::PointerButton::Primary)
        || response.drag_started_by(egui::PointerButton::Primary)
    {
        if let Some(pos) =
            if *tool != EditTool::Split && response.drag_started_by(egui::PointerButton::Primary) {
                ui.input(|i| i.pointer.press_origin())
            } else {
                response.interact_pointer_pos()
            }
        {
            let t = snap_local(to_seconds(pos.x));
            state.selection = Some((t, t));
            if *tool == EditTool::Split {
                split = true;
            }
        }
    }
    if matches!(*tool, EditTool::Select | EditTool::Range)
        && response.dragged_by(egui::PointerButton::Primary)
    {
        if let (Some((a, _)), Some(pos)) = (state.selection, response.interact_pointer_pos()) {
            state.selection = Some((a, snap_local(to_seconds(pos.x))));
        }
    }
    audio_events::toolbox(ui, wave, &mut state.toolbox, tool);
    if response.hovered() {
        if *tool == EditTool::Split {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        } else if !response.dragged_by(egui::PointerButton::Middle) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
        }
    }
    let ruler_response = ui.interact(ruler, ui.id().with("sample-ruler"), Sense::click_and_drag());
    if state.ruler_drag.is_none()
        && (ruler_response.drag_started_by(egui::PointerButton::Primary)
            || (ruler_response.is_pointer_button_down_on()
                && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))))
    {
        if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
            let fraction = ((pos.x - wave.left()) / wave.width()) as f64;
            state.ruler_drag = Some((pos, start, end, start + fraction * (end - start), 0.0));
        }
    }
    if let (Some((press, old_start, old_end, anchor, mut edge_scroll)), Some(pos)) =
        (state.ruler_drag, ruler_response.interact_pointer_pos())
    {
        if ui.input(|i| i.pointer.primary_down())
            || ruler_response.drag_stopped_by(egui::PointerButton::Primary)
        {
            let span = ((old_end - old_start) * (-((pos.y - press.y) as f64) * 0.012).exp())
                .clamp(0.001, max_span);
            let fraction = ((press.x - wave.left()) / wave.width()) as f64;
            if ui.input(|i| i.pointer.primary_down()) {
                let zone = 32.0;
                let strength = if pos.x < wave.left() + zone {
                    -((wave.left() + zone - pos.x) / zone).clamp(0.0, 1.0)
                } else if pos.x > wave.right() - zone {
                    ((pos.x - wave.right() + zone) / zone).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                edge_scroll += strength as f64
                    * span
                    * 0.65
                    * ui.input(|i| i.stable_dt).clamp(0.001, 0.05) as f64;
                if strength != 0.0 {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(16));
                }
                state.ruler_drag = Some((press, old_start, old_end, anchor, edge_scroll));
            }
            start = (anchor - fraction * span + edge_scroll).max(minimum_start);
            end = start + span;
            *transport = (original.start_beats
                + (start + ((pos.x - wave.left()) / wave.width()) as f64 * span)
                    * project.tempo_bpm
                    / 60.0)
                .max(0.0);
        }
    }
    if ruler_response.clicked_by(egui::PointerButton::Primary) && state.ruler_drag.is_none() {
        if let Some(pos) = ruler_response.interact_pointer_pos() {
            *transport = (original.start_beats
                + (start + ((pos.x - wave.left()) / wave.width()) as f64 * (end - start))
                    * project.tempo_bpm
                    / 60.0)
                .max(0.0);
        }
    }
    if ruler_response.clicked_by(egui::PointerButton::Primary)
        || ruler_response.drag_stopped_by(egui::PointerButton::Primary)
    {
        if let Some(step) = grid {
            *transport = crate::timeline::snap_beats_to_grid(*transport, step);
        }
    }
    if !ui.input(|i| i.pointer.primary_down()) {
        state.ruler_drag = None;
    }
    if ruler_response.dragged_by(egui::PointerButton::Middle) {
        let span = end - start;
        start = (start - ui.input(|i| i.pointer.delta().x) as f64 / wave.width() as f64 * span)
            .max(minimum_start);
        end = start + span;
        ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
    }
    if ruler_response.hovered() || response.hovered() {
        if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
            state.event_focus = true;
        }
    }
    state.view = Some((start, end));
    let beat_start = original.start_beats + start * project.tempo_bpm / 60.0;
    let beat_end = original.start_beats + end * project.tempo_bpm / 60.0;
    let (first, last, step) = if state.musical {
        let minimum = (beat_end - beat_start) * 55.0 / wave.width().max(1.0) as f64;
        let mut step = if minimum > 1.0 {
            project.beats_per_bar.max(1) as f64
        } else {
            0.25
        };
        while step < minimum {
            step *= 2.0;
        }
        (beat_start, beat_end, step)
    } else {
        let minimum = (end - start) * 65.0 / wave.width().max(1.0) as f64;
        let base = 10.0_f64.powf(minimum.max(1e-6).log10().floor());
        let step = [1.0, 2.0, 5.0, 10.0]
            .into_iter()
            .map(|m| m * base)
            .find(|v| *v >= minimum)
            .unwrap();
        (start, end, step)
    };
    let tick_count = ((last - first) / step).ceil().clamp(0.0, 2000.0) as usize + 2;
    for i in 0..tick_count {
        let value = (first / step).ceil() * step + i as f64 * step;
        if value > last {
            break;
        }
        let local = if state.musical {
            (value - original.start_beats) * 60.0 / project.tempo_bpm
        } else {
            value
        };
        let x = wave.left() + ((local - start) / (end - start)) as f32 * wave.width();
        ui.painter().with_clip_rect(wave).line_segment(
            [Pos2::new(x, wave.top()), Pos2::new(x, wave.bottom())],
            Stroke::new(0.5, p.grid),
        );
        let label = if state.musical {
            musical_position(value, project.beats_per_bar)
        } else {
            format!("{value:.3}")
        };
        ui.painter().with_clip_rect(ruler).text(
            Pos2::new(x + 3.0, ruler.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(10.0),
            p.text,
        );
    }
    if let Some(s) = source {
        waveform_view::draw_colored(
            ui.painter(),
            overview,
            overview,
            channels,
            original.duration_seconds,
            color,
            |a, b, n, ch| {
                s.waveform_columns(
                    a + original.source_offset_seconds,
                    b + original.source_offset_seconds,
                    n,
                    ch,
                )
            },
        );
        let virtual_rect = Rect::from_min_size(
            Pos2::new(
                wave.left() - start as f32 / (end - start) as f32 * wave.width(),
                wave.top(),
            ),
            Vec2::new(
                wave.width() * original.duration_seconds as f32 / (end - start) as f32,
                wave.height(),
            ),
        );
        waveform_view::draw_colored(
            ui.painter(),
            virtual_rect,
            wave,
            channels,
            original.duration_seconds,
            color,
            |a, b, n, ch| {
                let mut values = s.waveform_columns(
                    a + original.source_offset_seconds,
                    b + original.source_offset_seconds,
                    n,
                    ch,
                );
                for (i, v) in values.iter_mut().enumerate() {
                    let t = a + (b - a) * (i as f64 + 0.5) / n as f64;
                    let gain = original.gain
                        * opentrack_audio::event_envelope(
                            t,
                            original.duration_seconds,
                            original.fade_in_seconds,
                            original.fade_out_seconds,
                        )
                        * original
                            .source_fades
                            .iter()
                            .map(|f| f.gain_at(t + original.source_offset_seconds))
                            .product::<f32>();
                    v[0] *= gain;
                    v[1] *= gain;
                }
                values
            },
        );
    } else {
        ui.painter().text(
            wave.center(),
            Align2::CENTER_CENTER,
            "Audio source unavailable",
            FontId::proportional(14.0),
            p.text,
        );
    }
    // The editor shares the Project's global locators. Convert from project beats
    // to event-local time; the source trim offset is deliberately irrelevant.
    let cycle_color = crate::timeline::cycle_color(project.cycle_mode);
    let cycle_start = local_time(project.loop_start_beats, &original, project.tempo_bpm);
    let cycle_end = local_time(project.loop_end_beats, &original, project.tempo_bpm);
    let cycle_alpha = if state.cycle_enabled { 0.11 } else { 0.045 };
    for (rect, view_start, view_end) in [
        (wave, start, end),
        (overview, 0.0, original.duration_seconds),
    ] {
        let x = |time: f64| {
            rect.left() + ((time - view_start) / (view_end - view_start)) as f32 * rect.width()
        };
        let region = Rect::from_min_max(
            Pos2::new(x(cycle_start.min(cycle_end)), rect.top()),
            Pos2::new(x(cycle_start.max(cycle_end)), rect.bottom()),
        )
        .intersect(rect);
        if region.is_positive() {
            ui.painter().with_clip_rect(rect).rect_filled(
                region,
                0.0,
                cycle_color.gamma_multiply(cycle_alpha),
            );
        }
    }
    let ruler_x =
        |time: f64| ruler.left() + ((time - start) / (end - start)) as f32 * ruler.width();
    for time in [cycle_start, cycle_end] {
        let x = ruler_x(time);
        if x >= ruler.left() && x <= ruler.right() {
            ui.painter().with_clip_rect(ruler).line_segment(
                [Pos2::new(x, ruler.top()), Pos2::new(x, ruler.bottom())],
                Stroke::new(
                    1.0,
                    cycle_color.gamma_multiply(if state.cycle_enabled { 0.9 } else { 0.4 }),
                ),
            );
        }
    }
    let start_x = ruler_x(cycle_start);
    let end_x = ruler_x(cycle_end);
    let flag_size = 7.0_f32.min(((end_x - start_x).abs()).max(0.0) * 0.25);
    for (x, inward) in [(start_x, 1.0), (end_x, -1.0)] {
        if x >= ruler.left() && x <= ruler.right() {
            ui.painter()
                .with_clip_rect(ruler)
                .add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(x, ruler.top()),
                        Pos2::new(x + inward * flag_size, ruler.top()),
                        Pos2::new(x, ruler.top() + flag_size),
                    ],
                    cycle_color.gamma_multiply(if state.cycle_enabled { 0.95 } else { 0.4 }),
                    Stroke::new(1.0, p.surface),
                ));
        }
    }
    for (local, label, align) in [
        (0.0, "Event Start", Align2::LEFT_TOP),
        (original.duration_seconds, "Event End", Align2::RIGHT_TOP),
    ] {
        let x = wave.left() + ((local - start) / (end - start)) as f32 * wave.width();
        let painter = ui.painter().with_clip_rect(wave);
        painter.line_segment(
            [Pos2::new(x, wave.top()), Pos2::new(x, wave.bottom())],
            Stroke::new(1.0, color),
        );
        painter.text(
            Pos2::new(x + if local == 0.0 { 3.0 } else { -3.0 }, wave.top() + 2.0),
            align,
            label,
            FontId::proportional(9.0),
            p.text,
        );
    }
    // Actual shared transport is independent of source offset and current view zoom.
    let local = local_time(*transport, &original, project.tempo_bpm);
    for (rect, a, b) in [
        (wave, start, end),
        (overview, 0.0, original.duration_seconds),
    ] {
        let x = rect.left() + ((local - a) / (b - a)) as f32 * rect.width();
        ui.painter().with_clip_rect(rect).line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, theme::PLAYHEAD),
        );
    }
    let view_rect = Rect::from_min_max(
        Pos2::new(
            overview.left() + start as f32 / original.duration_seconds as f32 * overview.width(),
            overview.top(),
        ),
        Pos2::new(
            overview.left() + end as f32 / original.duration_seconds as f32 * overview.width(),
            overview.bottom(),
        ),
    );
    ui.painter().with_clip_rect(overview).rect_stroke(
        view_rect,
        0.0,
        Stroke::new(1.0, theme::PLAYHEAD),
        egui::StrokeKind::Inside,
    );
    if *tool == EditTool::Split && response.hovered() {
        if let Some(pos) = response.hover_pos() {
            let beat = original.start_beats + to_seconds(pos.x) * project.tempo_bpm / 60.0;
            let beat = grid.map_or(beat, |step| crate::timeline::snap_beats_to_grid(beat, step));
            if let Some(local) =
                original.cut_offset_seconds(beat, project.tempo_bpm, project.sample_rate_hz)
            {
                let x = wave.left() + ((local - start) / (end - start)) as f32 * wave.width();
                crate::system_tool_icons::tool_cursor_at(
                    ui,
                    ToolbarIcon::Split,
                    Some(Pos2::new(x, pos.y)),
                );
                ui.painter().with_clip_rect(wave.union(ruler)).line_segment(
                    [Pos2::new(x, ruler.top()), Pos2::new(x, wave.bottom())],
                    Stroke::new(1.0, theme::PLAYHEAD),
                );
            }
        }
    }
    if let Some((a, b)) = state.selection {
        let x = |t: f64| wave.left() + ((t - start) / (end - start)) as f32 * wave.width();
        ui.painter().with_clip_rect(wave).rect_filled(
            Rect::from_min_max(
                Pos2::new(x(a.min(b)), wave.top()),
                Pos2::new(x(a.max(b)), wave.bottom()),
            ),
            0.0,
            p.accent.gamma_multiply(0.15),
        );
        ui.painter().with_clip_rect(wave).line_segment(
            [Pos2::new(x(a), wave.top()), Pos2::new(x(a), wave.bottom())],
            Stroke::new(1.0, p.accent),
        );
    }
    ui.painter().text(
        Pos2::new(wave.left(), bounds.bottom() - 8.0),
        Align2::LEFT_BOTTOM,
        &state.message,
        FontId::proportional(11.0),
        p.text,
    );
    if !enabled {
        return false;
    }
    if split {
        if let Some((a, _)) = state.selection {
            let beat = original.start_beats + a * project.tempo_bpm / 60.0;
            let beat = grid.map_or(beat, |step| crate::timeline::snap_beats_to_grid(beat, step));
            if let Some(id) = project.split_audio_clip(track_id, clip_id, beat) {
                state.target = Some((track_id, id));
                return true;
            }
            state.message = "Cut must be inside the event after grid snapping.".into();
        }
    }
    if edited == original {
        return false;
    }
    if let Some(s) = source {
        edited.source_offset_seconds = edited
            .source_offset_seconds
            .min((s.duration_seconds() - 1.0 / project.sample_rate_hz as f64).max(0.0));
        edited.duration_seconds = edited
            .duration_seconds
            .min(s.duration_seconds() - edited.source_offset_seconds);
    }
    edited.fade_in_seconds = edited.fade_in_seconds.min(edited.duration_seconds);
    edited.fade_out_seconds = edited
        .fade_out_seconds
        .min(edited.duration_seconds - edited.fade_in_seconds);
    project.edit_audio_clip(track_id, track_id, edited)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(crate) fn fixture_source(channels: u16) -> (PathBuf, AudioSource) {
        let path = std::env::temp_dir().join(format!(
            "opentrack-sample-editor-{}-{}.wav",
            std::process::id(),
            NEXT_AUDIO_IMPORT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let rate = 8000_u32;
        let frames = 8000_u32;
        let data = frames * channels as u32 * 2;
        let mut bytes = Vec::new();
        bytes.extend(b"RIFF");
        bytes.extend((36 + data).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16_u32.to_le_bytes());
        bytes.extend(1_u16.to_le_bytes());
        bytes.extend(channels.to_le_bytes());
        bytes.extend(rate.to_le_bytes());
        bytes.extend((rate * channels as u32 * 2).to_le_bytes());
        bytes.extend((channels * 2).to_le_bytes());
        bytes.extend(16_u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(data.to_le_bytes());
        for i in 0..frames {
            for _ in 0..channels {
                bytes.extend((((i as f32 * 0.1).sin() * 12000.0) as i16).to_le_bytes());
            }
        }
        std::fs::write(&path, bytes).unwrap();
        let source = AudioSource::open(&path).unwrap();
        (path, source)
    }
    #[test]
    fn mono_source_draws_once_in_track_color_and_shared_transport_ignores_source_offset() {
        let (path, source) = fixture_source(1);
        assert_eq!(source.channels(), 1);
        assert_eq!(source.sample_rate(), 8000);
        assert_eq!(waveform_channels(2, Some(&source)), 1);
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let id = project.add_audio_clip(0, path.clone(), 8.0, 2.0).unwrap();
        project.tracks[0].clips[0].source_offset_seconds = 0.25;
        project.tracks[0].clips[0].gain = 0.00001; // must not clamp a legacy quiet event merely by opening.
        let saved = project.clone();
        let color = project_controls::track_color(&project.tracks[0]);
        let mut state = State::default();
        state.snap.quantize.note = crate::timeline::QuantizeNote::Quarter;
        state.target = Some((project.tracks[0].id, id));
        let mut tool = EditTool::Select;
        let mut transport = 10.0;
        let sources = HashMap::from([(path.clone(), source)]);
        let ctx = egui::Context::default();
        let mut last = None;
        for _ in 0..3 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                    ..Default::default()
                },
                |ui| {
                    assert!(!show(
                        ui,
                        &mut project,
                        &sources,
                        &mut state,
                        &mut tool,
                        Some(1.0),
                        true,
                        &mut transport,
                        false
                    ))
                },
            );
            let wave = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if r.fill == color.gamma_multiply(0.15) => Some(r.rect),
                    _ => None,
                })
                .unwrap();
            assert!(out.shapes.iter().any(|s| match &s.shape {
                egui::Shape::LineSegment { points, stroke } =>
                    stroke.color == theme::PLAYHEAD
                        && (points[0].x - wave.center().x).abs() < 0.01
                        && points[0].y == wave.top()
                        && points[1].y == wave.bottom(),
                _ => false,
            }));
            assert_eq!(out.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Mesh(m) if m.texture_id==egui::TextureId::default() && m.vertices.len()>10 && m.vertices.iter().any(|v|v.color==color.gamma_multiply(0.65)))).count(),2,"one overview waveform and one main waveform for mono source");
            assert!(!out.shapes.iter().any(
                |s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("AudioWarp"))
            ));
            out.textures_delta.clear();
            last = Some(wave);
        }
        let wave = last.unwrap();
        let pos = Pos2::new(wave.left() + wave.width() * 0.25, wave.top() - 11.0);
        for pressed in [true, false] {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| {
                    assert!(!show(
                        ui,
                        &mut project,
                        &sources,
                        &mut state,
                        &mut tool,
                        Some(1.0),
                        true,
                        &mut transport,
                        false
                    ))
                },
            );
            out.textures_delta.clear();
        }
        assert_eq!(transport, 9.0);
        assert_eq!(project, saved);
        let (stereo_path, stereo) = fixture_source(2);
        assert_eq!(waveform_channels(1, Some(&stereo)), 1);
        assert_eq!(waveform_channels(2, Some(&stereo)), 2);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(stereo_path).unwrap();
    }
    #[test]
    fn editor_cut_guide_is_on_the_committed_grid_sample() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, "missing.wav".into(), 8.0, 2.0)
            .unwrap();
        let mut state = State::default();
        state.snap.quantize.note = crate::timeline::QuantizeNote::Quarter;
        state.target = Some((project.tracks[0].id, id));
        let mut tool = EditTool::Split;
        let mut transport = 8.0;
        let ctx = egui::Context::default();
        let mut wave = None;
        for _ in 0..3 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                    ..Default::default()
                },
                |ui| {
                    show(
                        ui,
                        &mut project,
                        &HashMap::new(),
                        &mut state,
                        &mut tool,
                        Some(1.0),
                        true,
                        &mut transport,
                        false,
                    );
                },
            );
            let color = project_controls::track_color(&project.tracks[0]);
            wave = out.shapes.iter().find_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.fill == color.gamma_multiply(0.15) => Some(r.rect),
                _ => None,
            });
            out.textures_delta.clear();
        }
        let wave = wave.unwrap();
        let pointer = Pos2::new(wave.left() + wave.width() * 0.35, wave.center().y);
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                events: vec![egui::Event::PointerMoved(pointer)],
                ..Default::default()
            },
            |ui| {
                show(
                    ui,
                    &mut project,
                    &HashMap::new(),
                    &mut state,
                    &mut tool,
                    Some(1.0),
                    true,
                    &mut transport,
                    false,
                );
            },
        );
        let expected = wave.left() + wave.width() * 0.25;
        assert!(out.shapes.iter().any(|s| match &s.shape {
            egui::Shape::LineSegment { points, stroke } =>
                stroke.color == theme::PLAYHEAD
                    && (points[0].x - expected).abs() < 0.01
                    && points[0].y == wave.top() - 22.0
                    && points[1].y == wave.bottom(),
            _ => false,
        }));
        out.textures_delta.clear();
        for pressed in [true, false] {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                    events: vec![
                        egui::Event::PointerMoved(pointer),
                        egui::Event::PointerButton {
                            pos: pointer,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| {
                    show(
                        ui,
                        &mut project,
                        &HashMap::new(),
                        &mut state,
                        &mut tool,
                        Some(1.0),
                        true,
                        &mut transport,
                        false,
                    );
                },
            );
            out.textures_delta.clear();
        }
        assert_eq!(project.tracks[0].clips.len(), 2);
        assert_eq!(project.tracks[0].clips[1].start_beats, 9.0);
    }
    #[test]
    fn independent_snap_empty_space_pan_and_combined_ruler_seek_zoom_are_real_gestures() {
        fn frame(
            ctx: &egui::Context,
            p: &mut Project,
            s: &mut State,
            t: &mut f64,
            events: Vec<egui::Event>,
        ) -> Rect {
            let color = project_controls::track_color(&p.tracks[0]);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    assert!(!show(
                        ui,
                        p,
                        &HashMap::new(),
                        s,
                        &mut EditTool::Select,
                        Some(4.0),
                        true,
                        t,
                        false
                    ))
                },
            );
            let r = out
                .shapes
                .iter()
                .find_map(|sh| match &sh.shape {
                    egui::Shape::Rect(r) if r.fill == color.gamma_multiply(0.15) => Some(r.rect),
                    _ => None,
                })
                .unwrap();
            out.textures_delta.clear();
            r
        }
        let ctx = egui::Context::default();
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        let id = p.add_audio_clip(0, "missing.wav".into(), 0.0, 1.0).unwrap();
        let saved = p.clone();
        let mut s = State::default();
        s.target = Some((p.tracks[0].id, id));
        let mut t = 0.0;
        assert_eq!(s.snap.step(4), Some(0.25));
        let mut wave = Rect::NOTHING;
        for _ in 0..3 {
            wave = frame(&ctx, &mut p, &mut s, &mut t, vec![]);
        }
        let start_pos = Pos2::new(wave.left() + wave.width() * 0.31, wave.center().y);
        let end_pos = Pos2::new(wave.left() + wave.width() * 0.82, wave.center().y);
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![
                egui::Event::PointerMoved(start_pos),
                egui::Event::PointerButton {
                    pos: start_pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![egui::Event::PointerMoved(end_pos)],
        );
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![egui::Event::PointerButton {
                pos: end_pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(
            s.selection,
            Some((0.25, 0.875)),
            "range snaps to editor sixteenths, not supplied Project bar grid"
        );
        let center = wave.center();
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![
                egui::Event::PointerMoved(center),
                egui::Event::MouseWheel {
                    phase: egui::TouchPhase::Move,
                    unit: egui::MouseWheelUnit::Point,
                    delta: Vec2::new(0.0, -400.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        for _ in 0..3 {
            frame(&ctx, &mut p, &mut s, &mut t, vec![]);
        }
        let (a, b) = s.view.unwrap();
        assert!(b - a > 1.0, "can zoom out beyond source duration");
        let button = |pos, button, pressed| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![
                egui::Event::PointerMoved(center),
                button(center, egui::PointerButton::Middle, true),
            ],
        );
        let moved = center - Vec2::new(100.0, 0.0);
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![egui::Event::PointerMoved(moved)],
        );
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![button(moved, egui::PointerButton::Middle, false)],
        );
        assert!(
            s.view.unwrap().0 > a,
            "middle pan enters empty right-hand space"
        );
        let press = Pos2::new(wave.left() + wave.width() * 0.4, wave.top() - 11.0);
        let initial = s.view.unwrap();
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![
                egui::Event::PointerMoved(press),
                button(press, egui::PointerButton::Primary, true),
            ],
        );
        let down_right = press + Vec2::new(100.0, 35.0);
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![egui::Event::PointerMoved(down_right)],
        );
        let seek = t;
        let zoomed = s.view.unwrap();
        assert!(zoomed.1 - zoomed.0 < initial.1 - initial.0);
        let up_left = press + Vec2::new(-50.0, -30.0);
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![egui::Event::PointerMoved(up_left)],
        );
        let widened = s.view.unwrap();
        assert!(widened.1 - widened.0 > initial.1 - initial.0);
        assert_ne!(seek, t);
        let edge = Pos2::new(wave.right() - 2.0, press.y);
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![egui::Event::PointerMoved(edge)],
        );
        let first = s.view.unwrap().0;
        for _ in 0..5 {
            frame(&ctx, &mut p, &mut s, &mut t, vec![]);
        }
        assert!(
            s.view.unwrap().0 > first,
            "holding at ruler edge keeps scrolling"
        );
        frame(
            &ctx,
            &mut p,
            &mut s,
            &mut t,
            vec![button(up_left, egui::PointerButton::Primary, false)],
        );
        assert!((t / 0.25 - (t / 0.25).round()).abs() < 1e-6);
        assert_eq!(p, saved);
        s.snap.quantize.note = crate::timeline::QuantizeNote::Eighth;
        s.reset_target();
        assert_eq!(s.snap.step(4), Some(0.5));
        let bytes = serde_json::to_vec(&s.snap).unwrap();
        let restored: SnapSettings = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.step(4), Some(0.5));
    }
    #[test]
    fn editor_opens_shared_event_without_modification_and_handles_removed_event() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, "missing.wav".into(), 0.0, 2.0)
            .unwrap();
        let saved = project.clone();
        let mut state = State::default();
        state.snap.quantize.note = crate::timeline::QuantizeNote::Quarter;
        state.target = Some((project.tracks[0].id, id));
        let mut tool = EditTool::Select;
        let ctx = egui::Context::default();
        for _ in 0..3 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                    ..Default::default()
                },
                |ui| {
                    assert!(!show(
                        ui,
                        &mut project,
                        &HashMap::new(),
                        &mut state,
                        &mut tool,
                        Some(0.25),
                        true,
                        &mut 0.0,
                        false
                    ))
                },
            );
            out.textures_delta.clear();
        }
        assert_eq!(project, saved);
        project.tracks[0].clips.clear();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(!show(
                ui,
                &mut project,
                &HashMap::new(),
                &mut state,
                &mut tool,
                None,
                true,
                &mut 0.0,
                false
            ))
        });
        out.textures_delta.clear();
    }

    #[test]
    fn editor_cycle_overlay_tracks_shared_bounds_mode_and_enable_state_without_dirtying_project() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, "missing.wav".into(), 2.0, 4.0)
            .unwrap();
        assert!(project.set_cycle_range(4.0, 8.0));
        let mut saved = project.clone();
        let mut state = State::default();
        state.target = Some((project.tracks[0].id, id));
        state.cycle_enabled = true;
        let mut tool = EditTool::Select;
        let mut transport = 4.0;
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                ..Default::default()
            },
            |ui| {
                show(
                    ui,
                    &mut project,
                    &HashMap::new(),
                    &mut state,
                    &mut tool,
                    None,
                    true,
                    &mut transport,
                    false,
                );
            },
        );
        let loop_color = crate::timeline::cycle_color(project.cycle_mode).gamma_multiply(0.11);
        assert!(out.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Rect(rect) if rect.fill == loop_color && rect.rect.is_positive()
        )));
        out.textures_delta.clear();
        project.cycle_mode = opentrack_core::project::CycleMode::Skip;
        saved.cycle_mode = project.cycle_mode;
        state.cycle_enabled = false;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 500.0))),
                ..Default::default()
            },
            |ui| {
                show(
                    ui,
                    &mut project,
                    &HashMap::new(),
                    &mut state,
                    &mut tool,
                    None,
                    true,
                    &mut transport,
                    false,
                );
            },
        );
        let skip_color = crate::timeline::cycle_color(project.cycle_mode).gamma_multiply(0.045);
        assert!(out.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Rect(rect) if rect.fill == skip_color && rect.rect.is_positive()
        )));
        assert_eq!(
            project, saved,
            "rendering shared locators must not mutate the project"
        );
        out.textures_delta.clear();
    }
}

impl State {
    pub fn reset_target(&mut self) {
        let snap = self.snap.clone();
        let musical = self.musical;
        *self = Self::default();
        self.snap = snap;
        self.musical = musical;
    }
}
