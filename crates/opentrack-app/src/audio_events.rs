//! Audio-event interaction, preview and painting. Edits use one domain mutation path.
use super::*;
use opentrack_core::project::AudioClip;
#[derive(Clone, Copy, Debug, PartialEq)]
enum Tool {
    Move,
    Left,
    Right,
    FadeIn,
    FadeOut,
    Gain,
}
#[derive(Clone, Debug)]
struct Gesture {
    track: u64,
    original: AudioClip,
    press: Pos2,
    grab_beats: f64,
    tool: Tool,
    source_duration: f64,
    candidate: AudioClip,
    target: Option<u64>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    selected: Option<u64>,
    gesture: Option<Gesture>,
}
pub(super) struct Lane {
    pub id: u64,
    pub channels: u8,
    pub audio: bool,
    pub rect: Rect,
    pub color: Color32,
}
pub(super) struct Outcome {
    pub changed: bool,
    pub scroll: f32,
}
fn event_rect(c: &AudioClip, lane: &Lane, origin: f32, px: f32, tempo: f64) -> Rect {
    Rect::from_min_size(
        Pos2::new(origin + c.start_beats as f32 * px, lane.rect.top()),
        Vec2::new(
            (c.duration_seconds * tempo / 60.0) as f32 * px,
            lane.rect.height(),
        ),
    )
}
fn wave_rect(r: Rect) -> Rect {
    Rect::from_min_max(
        r.min,
        Pos2::new(r.right(), (r.bottom() - 13.0).max(r.top() + 1.0)),
    )
}
fn gain_y(c: &AudioClip, r: Rect) -> f32 {
    let db = 20.0 * c.gain.max(0.000001).log10();
    r.top() + 3.0 + (-db / 24.0).clamp(0.0, 1.0) * (r.height() - 6.0).max(0.0)
}
fn handles(c: &AudioClip, r: Rect) -> [(Tool, Pos2); 5] {
    let wave = wave_rect(r);
    let width = r.width();
    [
        (Tool::Left, Pos2::new(r.left() + 3.0, r.bottom() - 3.0)),
        (Tool::Right, Pos2::new(r.right() - 3.0, r.bottom() - 3.0)),
        (
            Tool::FadeIn,
            Pos2::new(
                r.left() + width * (c.fade_in_seconds / c.duration_seconds) as f32 + 3.0,
                wave.top() + 3.0,
            ),
        ),
        (
            Tool::FadeOut,
            Pos2::new(
                r.right() - width * (c.fade_out_seconds / c.duration_seconds) as f32 - 3.0,
                wave.top() + 3.0,
            ),
        ),
        (Tool::Gain, Pos2::new(r.center().x, gain_y(c, wave))),
    ]
}
fn hit(c: &AudioClip, r: Rect, pos: Pos2, selected: bool) -> Tool {
    if selected {
        for (tool, p) in handles(c, r) {
            if Rect::from_center_size(p, Vec2::splat(12.0)).contains(pos) {
                return tool;
            }
        }
    }
    Tool::Move
}
fn cursor(tool: Tool) -> egui::CursorIcon {
    match tool {
        Tool::Move => egui::CursorIcon::Move,
        Tool::Gain => egui::CursorIcon::ResizeVertical,
        _ => egui::CursorIcon::ResizeHorizontal,
    }
}
fn snap(value: f64, grid: Option<f64>) -> f64 {
    grid.map_or(value.max(0.0), |step| {
        crate::timeline::snap_beats_to_grid(value, step)
    })
}
fn update(
    g: &mut Gesture,
    pos: Pos2,
    origin: f32,
    px: f32,
    tempo: f64,
    grid: Option<f64>,
    lanes: &[Lane],
    min_seconds: f64,
) {
    let beats = ((pos.x - origin) / px) as f64;
    let original = &g.original;
    let mut c = original.clone();
    g.target = Some(g.track);
    match g.tool {
        Tool::Move => {
            c.start_beats = snap(beats - g.grab_beats, grid);
            let source = lanes.iter().find(|l| l.id == g.track);
            g.target = lanes
                .iter()
                .find(|l| {
                    l.rect.y_range().contains(pos.y)
                        && l.audio
                        && source.is_some_and(|s| s.channels == l.channels)
                })
                .map(|l| l.id);
        }
        Tool::Left => {
            let end = original.start_beats + original.duration_seconds * tempo / 60.0;
            let earliest =
                (original.start_beats - original.source_offset_seconds * tempo / 60.0).max(0.0);
            c.start_beats =
                snap(beats, grid).clamp(earliest, (end - min_seconds * tempo / 60.0).max(earliest));
            let delta = (c.start_beats - original.start_beats) * 60.0 / tempo;
            c.source_offset_seconds = (original.source_offset_seconds + delta).max(0.0);
            c.duration_seconds = (original.duration_seconds - delta).max(min_seconds);
        }
        Tool::Right => {
            let end = snap(beats, grid);
            c.duration_seconds = ((end - c.start_beats) * 60.0 / tempo).clamp(
                min_seconds,
                (g.source_duration - c.source_offset_seconds).max(min_seconds),
            );
        }
        Tool::FadeIn => {
            c.fade_in_seconds = ((beats - c.start_beats) * 60.0 / tempo)
                .clamp(0.0, (c.duration_seconds - c.fade_out_seconds).max(0.0))
        }
        Tool::FadeOut => {
            c.fade_out_seconds = (c.duration_seconds - (beats - c.start_beats) * 60.0 / tempo)
                .clamp(0.0, (c.duration_seconds - c.fade_in_seconds).max(0.0))
        }
        Tool::Gain => {
            let mut db = (20.0 * original.gain.max(0.000001).log10() - (pos.y - g.press.y) * 0.25)
                .clamp(-60.0, 12.0412);
            if db.abs() < 0.75 {
                db = 0.0;
            }
            c.gain = 10.0_f32.powf(db / 20.0).min(4.0);
        }
    }
    c.fade_in_seconds = c.fade_in_seconds.min(c.duration_seconds);
    c.fade_out_seconds = c
        .fade_out_seconds
        .min((c.duration_seconds - c.fade_in_seconds).max(0.0));
    g.candidate = c;
}
fn paint(
    ui: &egui::Ui,
    c: &AudioClip,
    lane: &Lane,
    origin: f32,
    px: f32,
    tempo: f64,
    clip: Rect,
    sources: &HashMap<PathBuf, AudioSource>,
    selected: bool,
    ghost: bool,
) {
    let r = event_rect(c, lane, origin, px, tempo);
    if !r.intersects(clip) || r.width() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(clip);
    let alpha = if ghost { 0.28 } else { 1.0 };
    painter.rect_filled(r, 0.0, lane.color.gamma_multiply(alpha));
    let band = Rect::from_min_max(Pos2::new(r.left(), (r.bottom() - 13.0).max(r.top())), r.max);
    painter.rect_filled(
        band,
        0.0,
        if selected {
            Color32::BLACK.gamma_multiply(alpha)
        } else {
            lane.color.gamma_multiply(0.8 * alpha)
        },
    );
    let wave = wave_rect(r);
    if let Some(source) = sources.get(&c.source_path) {
        super::waveform_view::draw(
            &painter,
            wave,
            clip,
            source.waveform().channels as usize,
            c.duration_seconds,
            |a, b, n, ch| {
                let mut values = source.waveform_columns(
                    a + c.source_offset_seconds,
                    b + c.source_offset_seconds,
                    n,
                    ch,
                );
                for (i, v) in values.iter_mut().enumerate() {
                    let t = a + (b - a) * (i as f64 + 0.5) / n as f64;
                    let gain = c.gain
                        * opentrack_audio::event_envelope(
                            t,
                            c.duration_seconds,
                            c.fade_in_seconds,
                            c.fade_out_seconds,
                        )
                        * alpha;
                    v[0] *= gain;
                    v[1] *= gain;
                }
                values
            },
        );
    }
    let ink = if selected {
        Color32::WHITE
    } else {
        project_controls::label_ink(lane.color)
    };
    painter.with_clip_rect(band.intersect(clip)).text(
        band.left_top() + Vec2::new(4.0, 0.0),
        Align2::LEFT_TOP,
        &c.name,
        FontId::proportional(11.0),
        ink.gamma_multiply(alpha),
    );
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(
            1.0,
            if selected {
                theme::RECORD
            } else {
                Color32::BLACK
            }
            .gamma_multiply(alpha),
        ),
        egui::StrokeKind::Inside,
    );
    if selected && !ghost {
        let blue = Color32::from_rgb(70, 170, 245);
        let gain = gain_y(c, wave);
        painter.line_segment(
            [Pos2::new(r.left(), gain), Pos2::new(r.right(), gain)],
            Stroke::new(1.0, blue),
        );
        painter.line_segment(
            [
                Pos2::new(r.left(), wave.bottom()),
                Pos2::new(
                    r.left() + r.width() * (c.fade_in_seconds / c.duration_seconds) as f32,
                    wave.top(),
                ),
            ],
            Stroke::new(1.0, blue),
        );
        painter.line_segment(
            [
                Pos2::new(
                    r.right() - r.width() * (c.fade_out_seconds / c.duration_seconds) as f32,
                    wave.top(),
                ),
                Pos2::new(r.right(), wave.bottom()),
            ],
            Stroke::new(1.0, blue),
        );
        for (_, pos) in handles(c, r) {
            painter.rect_filled(
                Rect::from_center_size(pos, Vec2::splat(6.0)),
                0.0,
                Color32::WHITE,
            );
        }
    }
}
pub(super) fn show(
    ui: &mut egui::Ui,
    project: &mut Project,
    sources: &HashMap<PathBuf, AudioSource>,
    lanes: &[Lane],
    visible: Rect,
    origin: f32,
    px: f32,
    grid: Option<f64>,
    state: &mut State,
    enabled: bool,
) -> Outcome {
    let mut out = Outcome {
        changed: false,
        scroll: 0.0,
    };
    let tempo = project.tempo_bpm;
    let mut reset_gain = None;
    let input = ui.input(|i| {
        (
            i.pointer.hover_pos(),
            i.pointer.primary_down(),
            i.pointer.button_released(egui::PointerButton::Primary),
            i.key_pressed(egui::Key::Escape),
        )
    });
    if state.gesture.is_none()
        && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
    {
        if let Some(pos) = input.0.filter(|p| visible.contains(*p)) {
            let over_event = lanes.iter().any(|lane| {
                project
                    .tracks
                    .iter()
                    .find(|t| t.id == lane.id)
                    .is_some_and(|t| {
                        t.clips
                            .iter()
                            .any(|c| event_rect(c, lane, origin, px, tempo).contains(pos))
                    })
            });
            if !over_event {
                state.selected = None;
            }
        }
    }
    if !enabled {
        state.gesture = None;
    }
    if let Some(mut g) = state.gesture.take() {
        if input.3 {
            if g.tool != Tool::Move {
                out.changed |= project.edit_audio_clip(g.track, g.track, g.original.clone());
            }
        } else if let Some(pos) = input.0 {
            update(
                &mut g,
                pos,
                origin,
                px,
                tempo,
                grid,
                lanes,
                1.0 / project.sample_rate_hz.max(1) as f64,
            );
            ui.ctx().set_cursor_icon(if g.target.is_none() {
                egui::CursorIcon::NotAllowed
            } else {
                cursor(g.tool)
            });
            if g.tool != Tool::Move || input.2 {
                if let Some(target) = g.target {
                    let changed = project.edit_audio_clip(g.track, target, g.candidate.clone());
                    if changed && g.tool == Tool::Move {
                        project.selected_track = project
                            .tracks
                            .iter()
                            .position(|t| t.id == target)
                            .map(|i| i as u32);
                    }
                    out.changed |= changed;
                }
            }
            if input.1 {
                if matches!(g.tool, Tool::Move | Tool::Left | Tool::Right) {
                    let edge = 32.0;
                    let strength = if pos.x < visible.left() + edge {
                        -((visible.left() + edge - pos.x) / edge).clamp(0.0, 1.0)
                    } else if pos.x > visible.right() - edge {
                        ((pos.x - visible.right() + edge) / edge).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    out.scroll = strength * 600.0 * ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
                }
                if g.tool == Tool::Gain {
                    egui::Tooltip::always_open(
                        ui.ctx().clone(),
                        ui.layer_id(),
                        Id::new("event-gain-tooltip"),
                        egui::PopupAnchor::Position(pos + Vec2::new(14.0, 14.0)),
                    )
                    .show(|ui| {
                        ui.label(format!(
                            "{:.2} dB",
                            20.0 * g.candidate.gain.max(0.000001).log10()
                        ));
                    });
                }
                state.gesture = Some(g);
                ui.ctx().request_repaint();
            }
        }
    }
    for lane in lanes.iter().filter(|l| l.audio) {
        let Some(track) = project.tracks.iter().find(|t| t.id == lane.id) else {
            continue;
        };
        for c in &track.clips {
            let selected = state.selected == Some(c.id);
            let ghost = state.gesture.as_ref().is_some_and(|g| {
                g.tool == Tool::Move && g.original.id == c.id && g.target.is_some()
            });
            paint(
                ui, c, lane, origin, px, tempo, visible, sources, selected, ghost,
            );
            let r = event_rect(c, lane, origin, px, tempo).intersect(visible);
            if !r.is_positive() || !enabled {
                continue;
            }
            let response = ui.interact(r, Id::new(("audio-event", c.id)), Sense::click_and_drag());
            if response.clicked() {
                state.selected = Some(c.id);
                ui.ctx().request_repaint();
            }
            if response.hovered() {
                if let Some(pos) = response.hover_pos() {
                    ui.ctx().set_cursor_icon(cursor(hit(
                        c,
                        event_rect(c, lane, origin, px, tempo),
                        pos,
                        selected,
                    )));
                }
            }
            if response.double_clicked()
                && input.0.is_some_and(|pos| {
                    hit(c, event_rect(c, lane, origin, px, tempo), pos, selected) == Tool::Gain
                })
            {
                let mut edited = c.clone();
                edited.gain = 1.0;
                reset_gain = Some((lane.id, edited));
            }
            if response.drag_started_by(egui::PointerButton::Primary) && state.gesture.is_none() {
                let press = ui.input(|i| i.pointer.press_origin()).unwrap_or(r.center());
                let tool = hit(c, event_rect(c, lane, origin, px, tempo), press, selected);
                state.selected = Some(c.id);
                let mut g = Gesture {
                    track: lane.id,
                    original: c.clone(),
                    candidate: c.clone(),
                    press,
                    grab_beats: ((press.x - origin) / px) as f64 - c.start_beats,
                    tool,
                    source_duration: sources
                        .get(&c.source_path)
                        .map(|s| s.duration_seconds())
                        .unwrap_or(c.duration_seconds + c.source_offset_seconds),
                    target: Some(lane.id),
                };
                if let Some(pos) = input.0 {
                    update(
                        &mut g,
                        pos,
                        origin,
                        px,
                        tempo,
                        grid,
                        lanes,
                        1.0 / project.sample_rate_hz.max(1) as f64,
                    );
                }
                state.gesture = Some(g);
                ui.ctx().request_repaint();
            }
        }
    }
    if let Some((track, edited)) = reset_gain {
        out.changed |= project.edit_audio_clip(track, track, edited);
    }
    if let Some(g) = state.gesture.as_ref().filter(|g| g.tool == Tool::Move) {
        if let Some(lane) = lanes.iter().find(|l| Some(l.id) == g.target) {
            paint(
                ui,
                &g.candidate,
                lane,
                origin,
                px,
                tempo,
                visible,
                sources,
                true,
                false,
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        ctx: egui::Context,
        project: Project,
        state: State,
        lanes: Vec<Lane>,
        grid: Option<f64>,
    }
    impl Fixture {
        fn new() -> Self {
            let mut project = Project::default();
            project.tempo_bpm = 120.0;
            project.add_track(TrackKind::Audio);
            project.add_track(TrackKind::Audio);
            project.add_track_with_audio_channels(TrackKind::Audio, 1);
            project
                .add_audio_clip(0, PathBuf::from("test.wav"), 1.0, 4.0)
                .unwrap();
            let lanes = project
                .tracks
                .iter()
                .enumerate()
                .map(|(i, t)| Lane {
                    id: t.id,
                    channels: t.audio_channels,
                    audio: true,
                    rect: Rect::from_min_size(
                        Pos2::new(100.0, i as f32 * 70.0),
                        Vec2::new(1100.0, 70.0),
                    ),
                    color: Color32::from_rgb(110, 80, 150),
                })
                .collect();
            let mut f = Self {
                ctx: egui::Context::default(),
                project,
                state: State::default(),
                lanes,
                grid: Some(0.25),
            };
            f.frame(vec![]);
            f.frame(vec![]);
            f
        }
        fn frame(&mut self, events: Vec<egui::Event>) -> bool {
            let mut changed = false;
            let mut out = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 230.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    changed = show(
                        ui,
                        &mut self.project,
                        &HashMap::new(),
                        &self.lanes,
                        Rect::from_min_max(Pos2::new(100.0, 0.0), Pos2::new(1200.0, 210.0)),
                        100.0,
                        100.0,
                        self.grid,
                        &mut self.state,
                        true,
                    )
                    .changed;
                },
            );
            let _ = self
                .ctx
                .tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
            out.textures_delta.clear();
            changed
        }
        fn button(&mut self, pos: Pos2, pressed: bool) -> bool {
            self.frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ])
        }
        fn select(&mut self) {
            self.button(Pos2::new(450.0, 30.0), true);
            self.button(Pos2::new(450.0, 30.0), false);
            self.frame(vec![]);
        }
    }
    #[test]
    fn actual_left_drag_previews_without_mutating_then_moves_to_compatible_lane_using_snap() {
        let mut f = Fixture::new();
        f.button(Pos2::new(450.0, 30.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(590.0, 100.0))]);
        assert_eq!(f.project.tracks[0].clips.len(), 1);
        assert!(f.project.tracks[1].clips.is_empty());
        assert_eq!(f.state.gesture.as_ref().unwrap().candidate.start_beats, 2.5);
        assert!(f.button(Pos2::new(590.0, 100.0), false));
        assert!(f.project.tracks[0].clips.is_empty());
        assert_eq!(f.project.tracks[1].clips[0].start_beats, 2.5);
        assert_eq!(f.project.tracks[1].clips[0].duration_seconds, 4.0);
    }
    #[test]
    fn incompatible_lane_drop_is_rejected_and_snap_off_preserves_free_position() {
        let mut f = Fixture::new();
        let before = f.project.tracks[0].clips[0].clone();
        f.button(Pos2::new(450.0, 30.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(590.0, 170.0))]);
        f.button(Pos2::new(590.0, 170.0), false);
        assert_eq!(f.project.tracks[0].clips[0], before);
        assert!(f.project.tracks[2].clips.is_empty());
        f.grid = None;
        f.button(Pos2::new(450.0, 30.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(590.0, 100.0))]);
        f.button(Pos2::new(590.0, 100.0), false);
        assert!((f.project.tracks[1].clips[0].start_beats - 2.4).abs() < 0.0001);
    }
    #[test]
    fn selected_white_trim_handle_changes_source_offset_and_escape_restores_original() {
        let mut f = Fixture::new();
        f.select();
        let before = f.project.tracks[0].clips[0].clone();
        f.button(Pos2::new(203.0, 67.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(300.0, 67.0))]);
        f.frame(vec![]);
        let c = &f.project.tracks[0].clips[0];
        assert_eq!(c.start_beats, 2.0);
        assert_eq!(c.source_offset_seconds, 0.5);
        assert_eq!(c.duration_seconds, 3.5);
        f.frame(vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(f.project.tracks[0].clips[0], before);
        f.button(Pos2::new(300.0, 67.0), false);
    }
    #[test]
    fn fade_and_gain_handles_change_real_event_parameters_and_gain_snaps_back_to_unity() {
        let mut f = Fixture::new();
        f.select();
        f.button(Pos2::new(203.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(400.0, 3.0))]);
        f.button(Pos2::new(400.0, 3.0), false);
        assert!((f.project.tracks[0].clips[0].fade_in_seconds - 1.0).abs() < 0.001);
        f.button(Pos2::new(600.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(600.0, 27.0))]);
        f.frame(vec![]);
        assert!((20.0 * f.project.tracks[0].clips[0].gain.log10() + 6.0).abs() < 0.01);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(600.0, 4.0))]);
        f.button(Pos2::new(600.0, 4.0), false);
        assert_eq!(f.project.tracks[0].clips[0].gain, 1.0);
    }
    #[test]
    fn bounds_cover_whole_lane_and_all_edit_handles_are_white() {
        let mut f = Fixture::new();
        f.select();
        let c = &f.project.tracks[0].clips[0];
        let r = event_rect(c, &f.lanes[0], 100.0, 100.0, 120.0);
        assert_eq!(r.top(), 0.0);
        assert_eq!(r.bottom(), 70.0);
        let mut out = f.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 230.0))),
                ..Default::default()
            },
            |ui| {
                paint(
                    ui,
                    c,
                    &f.lanes[0],
                    100.0,
                    100.0,
                    120.0,
                    ui.max_rect(),
                    &HashMap::new(),
                    true,
                    false,
                );
            },
        );
        let handles=out.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.fill==Color32::WHITE && r.rect.size()==Vec2::splat(6.0))).count();
        assert_eq!(handles, 5);
        out.textures_delta.clear();
    }
    #[test]
    fn right_trim_fade_out_and_gain_boost_are_undoable_and_preserve_media() {
        let mut f = Fixture::new();
        f.select();
        let before = f.project.clone();
        let mut history = ProjectHistory::default();
        history.begin(&before);
        f.button(Pos2::new(997.0, 67.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(850.0, 67.0))]);
        f.button(Pos2::new(850.0, 67.0), false);
        assert!((f.project.tracks[0].clips[0].duration_seconds - 3.25).abs() < 0.0001);
        f.button(Pos2::new(847.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(750.0, 3.0))]);
        f.button(Pos2::new(750.0, 3.0), false);
        assert!((f.project.tracks[0].clips[0].fade_out_seconds - 0.5).abs() < 0.001);
        f.button(Pos2::new(525.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(525.0, -21.0))]);
        f.button(Pos2::new(525.0, -21.0), false);
        assert!((20.0 * f.project.tracks[0].clips[0].gain.log10() - 6.0).abs() < 0.01);
        assert!(history.finish_frame(&f.project, true));
        let undo = history.undo(&f.project).unwrap();
        assert_eq!(undo, before);
        assert_eq!(history.redo(&undo).unwrap(), f.project);
    }
}
