//! Pitch display coordinates and navigation. Display zoom never changes audio.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Viewport {
    pub center: f32,
    pub span: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            center: 60.0,
            span: 24.0,
        }
    }
}

impl Viewport {
    pub fn pan_pixels(&mut self, delta_y: f32, view_height: f32) {
        self.center += delta_y * self.span / view_height.max(1.0);
        self.sanitize();
    }

    pub fn scroll(&mut self, amount: f32) {
        self.center += amount * self.span * 0.0025;
        self.sanitize();
    }

    pub fn zoom_scroll(&mut self, amount: f32) {
        self.span *= (-amount * 0.003).exp();
        self.sanitize();
    }

    pub fn sanitize(&mut self) {
        if !self.span.is_finite() {
            self.span = 24.0;
        }
        if !self.center.is_finite() {
            self.center = 60.0;
        }
        self.span = self.span.clamp(6.0, 72.0);
        self.center = self.center.clamp(self.span * 0.5, 127.0 - self.span * 0.5);
    }

    pub fn y(&self, midi: f32, rect: Rect) -> f32 {
        rect.center().y - (midi - self.center) * rect.height() / self.span
    }

    pub fn center_on(&mut self, analysis: &opentrack_audio::PitchAnalysis) {
        let mut pitches: Vec<_> = analysis
            .points
            .iter()
            .filter_map(|point| {
                point
                    .frequency_hz
                    .filter(|hz| hz.is_finite() && *hz > 0.0)
                    .map(|hz| 69.0 + 12.0 * (hz / 440.0).log2())
            })
            .collect();
        if !pitches.is_empty() {
            pitches.sort_by(f32::total_cmp);
            self.center = pitches[pitches.len() / 2];
        }
        self.sanitize();
    }
}

const BLACK_KEY_FRACTION: f32 = 0.58;
const MIN_LABEL_ROW_HEIGHT: f32 = 12.0;
const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub(super) fn black_key(midi: i32) -> bool {
    matches!(midi.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

pub(super) fn note_name(midi: i32) -> String {
    format!(
        "{}{}",
        NOTE_NAMES[midi.rem_euclid(12) as usize],
        midi.div_euclid(12) - 1
    )
}

/// Paint one piano key and its matching pitch row. Shared with the MIDI Editor
/// so both editors use the same key geometry, colors, and octave labels.
pub(super) fn key_row(ui: &egui::Ui, keyboard: Rect, row: Rect, midi: i32, row_height: f32) {
    let p = theme::palette(ui);
    let painter = ui.painter().with_clip_rect(keyboard.union(row));
    let black = black_key(midi);
    painter.rect_filled(
        row,
        0.0,
        if black {
            p.midi_row_black
        } else {
            p.midi_row_white
        },
    );
    let key = Rect::from_min_max(
        Pos2::new(keyboard.left(), row.top()),
        Pos2::new(keyboard.right(), row.bottom()),
    )
    .intersect(keyboard);
    painter.rect_filled(key, 0.0, p.midi_key_white);
    if black {
        painter.rect_filled(
            Rect::from_min_max(
                key.min,
                Pos2::new(
                    keyboard.left() + keyboard.width() * BLACK_KEY_FRACTION,
                    key.bottom(),
                ),
            ),
            0.0,
            p.midi_key_black,
        );
    }
    let pixels_per_point = ui.ctx().pixels_per_point().max(1.0);
    let separator_y = ((row.bottom() * pixels_per_point).round() + 0.5) / pixels_per_point;
    painter.line_segment(
        [
            Pos2::new(row.left(), separator_y),
            Pos2::new(row.right(), separator_y),
        ],
        Stroke::new(1.0 / pixels_per_point, p.grid.gamma_multiply(0.85)),
    );
    // Draw the key boundary after filling the keyboard so adjacent keys stay
    // distinct at compact semitone heights. Snap the stroke to a physical
    // pixel to avoid the half-pixel antialiasing that made these disappear.
    let separator_y = ((key.bottom() * pixels_per_point).round() + 0.5) / pixels_per_point;
    let separator = if ui.visuals().dark_mode {
        if black {
            Color32::from_gray(70)
        } else {
            Color32::from_gray(45)
        }
    } else if black {
        Color32::from_gray(130)
    } else {
        Color32::from_gray(195)
    };
    painter.line_segment(
        [
            Pos2::new(key.left(), separator_y),
            Pos2::new(key.right(), separator_y),
        ],
        Stroke::new(1.0 / pixels_per_point, separator),
    );
    if row_height >= MIN_LABEL_ROW_HEIGHT {
        let (label_right, color) = if black {
            (
                keyboard.left() + keyboard.width() * BLACK_KEY_FRACTION - 3.0,
                Color32::WHITE,
            )
        } else {
            (
                keyboard.right() - 3.0,
                if ui.visuals().dark_mode {
                    Color32::from_gray(225)
                } else {
                    Color32::from_gray(35)
                },
            )
        };
        painter.text(
            Pos2::new(label_right, row.center().y),
            Align2::RIGHT_CENTER,
            note_name(midi),
            FontId::proportional(theme::type_size::SMALL),
            color,
        );
    }
}

pub(super) fn grid(ui: &egui::Ui, keyboard: Rect, wave: Rect, view: &Viewport) {
    let p = theme::palette(ui);
    let painter = ui.painter().with_clip_rect(keyboard.union(wave));
    let row_height = wave.height() / view.span;
    for midi in 0..=127 {
        let y = view.y(midi as f32, wave);
        let row = Rect::from_min_max(
            Pos2::new(wave.left(), y - row_height * 0.5),
            Pos2::new(wave.right(), y + row_height * 0.5),
        )
        .intersect(wave);
        if !row.is_positive() {
            continue;
        }
        key_row(ui, keyboard, row, midi, row_height);
    }
    painter.line_segment(
        [keyboard.right_top(), keyboard.right_bottom()],
        Stroke::new(1.0, p.grid),
    );
}

pub(super) fn controls(
    ui: &mut egui::Ui,
    rail: Rect,
    keyboard: Rect,
    wave: Rect,
    view: &mut Viewport,
) {
    view.sanitize();
    let p = theme::palette(ui);
    ui.painter().rect_filled(rail, 0.0, p.surface_raised);
    let keys = ui.interact(
        keyboard,
        ui.id().with("pitch-keyboard-pan"),
        Sense::click_and_drag(),
    );
    if keys.dragged_by(egui::PointerButton::Primary) {
        view.pan_pixels(ui.input(|i| i.pointer.delta().y), wave.height());
    }
    if keys.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    let navigation = Rect::from_min_max(
        rail.min + Vec2::splat(2.0),
        Pos2::new(
            rail.right() - 2.0,
            (rail.bottom() - 100.0).max(rail.top() + 12.0),
        ),
    );
    let nav = ui.interact(
        navigation,
        ui.id().with("pitch-scroll"),
        Sense::click_and_drag(),
    );
    if nav.clicked() || nav.dragged() {
        if let Some(pos) = nav.interact_pointer_pos() {
            view.center =
                (1.0 - ((pos.y - navigation.top()) / navigation.height()).clamp(0.0, 1.0)) * 127.0;
        }
    }
    view.sanitize();
    let thumb_height = (navigation.height() * view.span / 127.0)
        .max(12.0)
        .min(navigation.height());
    let thumb_y = navigation.bottom() - view.center / 127.0 * navigation.height();
    let thumb = Rect::from_center_size(
        Pos2::new(
            navigation.center().x,
            thumb_y.clamp(
                navigation.top() + thumb_height * 0.5,
                navigation.bottom() - thumb_height * 0.5,
            ),
        ),
        Vec2::new(navigation.width(), thumb_height),
    );
    ui.painter().rect_filled(navigation, 1.0, p.canvas);
    ui.painter().rect_filled(
        thumb,
        2.0,
        if nav.hovered() || nav.dragged() {
            crate::theme::slider_highlight(ui.visuals())
        } else {
            p.muted.gamma_multiply(0.65)
        },
    );

    let zoom_rect = Rect::from_min_max(
        Pos2::new(rail.left() + 1.0, rail.bottom() - 94.0),
        Pos2::new(rail.right() - 1.0, rail.bottom() - 1.0),
    );
    let mut zoom = 72.0 / view.span;
    crate::view_controls::zoom(
        ui,
        zoom_rect,
        ui.id().with("pitch-zoom"),
        &mut zoom,
        [1.0, 12.0],
        3.0,
        true,
    );
    view.span = 72.0 / zoom;
    view.sanitize();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn right_edge_zoom_and_pitch_scroll_accept_real_pointer_gestures() {
        let ctx = egui::Context::default();
        let mut view = Viewport::default();
        let wave = Rect::from_min_max(Pos2::new(60.0, 20.0), Pos2::new(780.0, 300.0));
        let keyboard = Rect::from_min_max(Pos2::new(20.0, 20.0), wave.left_bottom());
        let rail = Rect::from_min_max(wave.right_top(), Pos2::new(800.0, 300.0));
        let frame = |events: Vec<egui::Event>, view: &mut Viewport| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(820.0, 320.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    controls(ui, rail, keyboard, wave, view);
                    grid(ui, keyboard, wave, view);
                },
            );
            out.textures_delta.clear();
        };
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![], &mut view);
        let press = Pos2::new(790.0, 256.0);
        frame(
            vec![egui::Event::PointerMoved(press), button(press, true)],
            &mut view,
        );
        let moved = Pos2::new(790.0, 230.0);
        frame(vec![egui::Event::PointerMoved(moved)], &mut view);
        frame(vec![button(moved, false)], &mut view);
        assert!(view.span < 24.0, "dragging zoom up enlarges semitone rows");
        assert_eq!(view.center, 60.0, "zoom retains its pitch anchor");
        let scroll = Pos2::new(790.0, 42.0);
        frame(
            vec![egui::Event::PointerMoved(scroll), button(scroll, true)],
            &mut view,
        );
        frame(vec![button(scroll, false)], &mut view);
        assert!(view.center > 90.0, "pitch scrollbar exposes higher notes");
        let stored = serde_json::to_string(&view).unwrap();
        let restored: Viewport = serde_json::from_str(&stored).unwrap();
        assert_eq!(restored.span, view.span);
        assert_eq!(restored.center, view.center);
    }

    #[test]
    fn vertical_zoom_changes_row_height_preserving_center_and_sanitizes_saved_state() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 240.0));
        let mut view = Viewport::default();
        let center_y = view.y(60.0, rect);
        let initial_row = view.y(60.0, rect) - view.y(61.0, rect);
        view.span /= 2.0;
        assert_eq!(view.y(60.0, rect), center_y);
        assert_eq!(view.y(60.0, rect) - view.y(61.0, rect), initial_row * 2.0);
        view.center = f32::NAN;
        view.span = f32::INFINITY;
        view.sanitize();
        assert_eq!(view.center, 60.0);
        assert_eq!(view.span, 24.0);
    }

    #[test]
    fn every_key_is_named_inside_its_own_key_and_labels_hide_when_rows_are_too_small() {
        let keyboard = Rect::from_min_max(Pos2::new(20.0, 20.0), Pos2::new(84.0, 320.0));
        let wave = Rect::from_min_max(keyboard.right_top(), Pos2::new(784.0, 320.0));
        let mut view = Viewport::default();
        view.center = 66.0;
        view.span = 13.0;
        let visible = |view: &Viewport| {
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(820.0, 340.0))),
                    ..Default::default()
                },
                |ui| grid(ui, keyboard, wave, view),
            );
            output.textures_delta.clear();
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => {
                        let rect = Rect::from_min_size(text.pos, text.galley.size());
                        rect.intersects(keyboard)
                            .then(|| (text.galley.text().to_owned(), rect))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let labels = visible(&view);
        for midi in 60..=71 {
            let name = note_name(midi);
            assert!(
                labels.iter().any(|(text, _)| text == &name),
                "{name} must name its key"
            );
        }
        let black_edge = keyboard.left() + keyboard.width() * BLACK_KEY_FRACTION;
        let c_sharp = labels
            .iter()
            .find(|(text, _)| text == "C#4")
            .map(|(_, rect)| rect)
            .unwrap();
        assert!(
            c_sharp.left() >= keyboard.left() && c_sharp.right() <= black_edge,
            "a black key's name belongs inside the black key"
        );
        let c = labels
            .iter()
            .find(|(text, _)| text == "C4")
            .map(|(_, rect)| rect)
            .unwrap();
        assert!(
            c.left() > black_edge && c.right() <= keyboard.right(),
            "a white key's name stays in the white part of the key"
        );
        view.center = 2.0;
        view.span = 6.0;
        let lowest_sharp = visible(&view);
        let c_sharp_negative = lowest_sharp
            .iter()
            .find(|(text, _)| text == "C#-1")
            .map(|(_, rect)| rect)
            .expect("the lowest octave is named too");
        assert!(
            c_sharp_negative.left() >= keyboard.left() && c_sharp_negative.right() <= black_edge,
            "the widest black-key name still fits inside the black key"
        );
        view.center = 66.0;
        view.span = 72.0;
        assert!(
            visible(&view).is_empty(),
            "labels are hidden rather than shown partially when rows are too small"
        );
    }
}
