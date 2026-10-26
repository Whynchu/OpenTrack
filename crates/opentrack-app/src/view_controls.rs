//! Shared arrange/editor navigation, before egui remaps Shift to X and Ctrl to zoom.
use eframe::egui::{self, Id, Pos2, Rect, Sense, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum WheelAction {
    TimeZoom(f32),
    HorizontalPan(f32),
    VerticalPan(f32),
    VerticalZoom(f32),
}

pub(crate) fn wheel(ui: &egui::Ui, bounds: Rect) -> Option<WheelAction> {
    ui.input(|input| {
        if !input
            .pointer
            .hover_pos()
            .is_some_and(|pos| bounds.contains(pos))
        {
            return None;
        }
        let mut amount = 0.0;
        let mut modifiers = input.modifiers;
        for event in &input.events {
            if let egui::Event::MouseWheel {
                unit,
                delta,
                modifiers: event_modifiers,
                ..
            } = event
            {
                // Read the original event: egui has already moved Shift-wheel
                // into X and removed Ctrl-wheel from smooth_scroll_delta.
                amount += (delta.y + delta.x)
                    * match unit {
                        egui::MouseWheelUnit::Point => 1.0,
                        egui::MouseWheelUnit::Line => 40.0,
                        egui::MouseWheelUnit::Page => bounds.height(),
                    };
                modifiers = *event_modifiers;
            }
        }
        if amount == 0.0 {
            None
        } else if modifiers.ctrl && modifiers.shift {
            Some(WheelAction::VerticalZoom(amount))
        } else if modifiers.shift {
            Some(WheelAction::VerticalPan(amount))
        } else if modifiers.ctrl {
            Some(WheelAction::HorizontalPan(amount))
        } else {
            Some(WheelAction::TimeZoom(amount))
        }
    })
}

/// Logarithmic zoom slider. Both orientations use the Sample Editor rail,
/// rectangular thumb, hover/drag accent and +/- buttons.
pub(crate) fn zoom(
    ui: &mut egui::Ui,
    rect: Rect,
    id: Id,
    value: &mut f32,
    limits: [f32; 2],
    reset: f32,
    vertical: bool,
) {
    let p = crate::theme::palette(ui);
    ui.painter().rect_filled(rect, 0.0, p.surface_raised);
    let size = if vertical {
        Vec2::new(rect.width(), 16.0)
    } else {
        Vec2::new(16.0, rect.height())
    };
    let plus = Rect::from_min_size(
        if vertical {
            rect.min
        } else {
            Pos2::new(rect.right() - 16.0, rect.top())
        },
        size,
    );
    let minus = Rect::from_min_size(
        if vertical {
            Pos2::new(rect.left(), rect.bottom() - 16.0)
        } else {
            rect.min
        },
        size,
    );
    for (r, label, scale) in [(plus, "+", 1.25), (minus, "-", 0.8)] {
        if ui.put(r, egui::Button::new(label).frame(false)).clicked() {
            *value *= scale;
        }
    }
    let rail = if vertical {
        Rect::from_min_max(
            plus.left_bottom() + Vec2::new(0.0, 3.0),
            minus.right_top() - Vec2::new(0.0, 3.0),
        )
    } else {
        Rect::from_min_max(
            minus.right_top() + Vec2::new(3.0, 0.0),
            plus.left_bottom() - Vec2::new(3.0, 0.0),
        )
    };
    let response = ui.interact(rail, id, Sense::click_and_drag());
    if (response.clicked() || response.dragged())
        && let Some(pos) = response.interact_pointer_pos()
    {
        let f = if vertical {
            (rail.bottom() - pos.y) / rail.height().max(1.0)
        } else {
            (pos.x - rail.left()) / rail.width().max(1.0)
        };
        *value = limits[0] * (limits[1] / limits[0]).powf(f.clamp(0.0, 1.0));
    }
    if response.double_clicked() {
        *value = reset;
    }
    *value = value.clamp(limits[0], limits[1]);
    let f = ((*value / limits[0]).ln() / (limits[1] / limits[0]).ln()).clamp(0.0, 1.0);
    let (a, b, center, size) = if vertical {
        (
            rail.center_top(),
            rail.center_bottom(),
            Pos2::new(rail.center().x, rail.bottom() - f * rail.height()),
            Vec2::new(rect.width() - 5.0, 7.0),
        )
    } else {
        (
            rail.left_center(),
            rail.right_center(),
            Pos2::new(rail.left() + f * rail.width(), rail.center().y),
            Vec2::new(7.0, rect.height() - 5.0),
        )
    };
    ui.painter()
        .line_segment([a, b], Stroke::new(2.0, p.canvas));
    ui.painter().rect_filled(
        Rect::from_center_size(center, size),
        1.0,
        if response.hovered() || response.dragged() {
            p.accent
        } else {
            p.muted
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_wheel_events_survive_egui_shift_axis_and_ctrl_zoom_routing() {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0));
        for (modifiers, expected) in [
            (egui::Modifiers::NONE, WheelAction::TimeZoom(40.0)),
            (
                egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                WheelAction::HorizontalPan(40.0),
            ),
            (egui::Modifiers::SHIFT, WheelAction::VerticalPan(40.0)),
            (
                egui::Modifiers {
                    shift: true,
                    ctrl: true,
                    ..Default::default()
                },
                WheelAction::VerticalZoom(40.0),
            ),
        ] {
            let mut actual = None;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(rect),
                    events: vec![
                        egui::Event::PointerMoved(rect.center()),
                        egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Line,
                            delta: Vec2::new(0.0, 1.0),
                            phase: egui::TouchPhase::Move,
                            modifiers,
                        },
                    ],
                    ..Default::default()
                },
                |ui| {
                    actual = wheel(ui, rect);
                },
            );
            out.textures_delta.clear();
            assert_eq!(actual, Some(expected));
        }
    }
}
