//! Original vector glyphs shared by Project and channel/effect controls.
use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Copy)]
pub(crate) enum ToolbarIcon {
    Power,
    Constraint,
    Inspector,
    InfoLine,
    Overview,
    Pool,
    Mixer,
    Automation,
    Previous,
    Next,
    Cycle,
    Stop,
    Play,
    Record,
    Select,
    Range,
    Split,
    Glue,
    Erase,
    Zoom,
    Mute,
    TimeWarp,
    Draw,
    Line,
    PlayTool,
    Color,
    RootKey,
    Autoscroll,
    Snap,
    SnapType,
    ZeroCrossing,
}

/// Standard broken ring with the gap centered at twelve o'clock. Geometry
/// scales to small channel switches instead of relying on a font glyph.
pub(crate) fn power(painter: &egui::Painter, rect: Rect, ink: Color32) {
    let c = rect.center() + Vec2::new(0.0, 0.4);
    let radius = (rect.width().min(rect.height()) * 0.34).min(7.0);
    let width = (radius * 0.23).clamp(1.0, 1.6);
    let stroke = Stroke::new(width, ink);
    let arc: Vec<_> = (0..=32)
        .map(|i| {
            let angle = (45.0 + i as f32 * 270.0 / 32.0).to_radians();
            c + Vec2::new(angle.sin(), -angle.cos()) * radius
        })
        .collect();
    for endpoint in [arc[0], arc[32]] {
        painter.circle_filled(endpoint, width * 0.5, ink);
    }
    painter.add(egui::Shape::line(arc, stroke));
    let stem = [
        c + Vec2::new(0.0, -radius * 1.15),
        c + Vec2::new(0.0, -radius * 0.15),
    ];
    painter.line_segment(stem, stroke);
    for endpoint in stem {
        painter.circle_filled(endpoint, width * 0.5, ink);
    }
}

fn pointer(painter: &egui::Painter, center: Pos2) {
    let at = |x, y| center + Vec2::new(x, y);
    // Two convex pieces form the familiar arrow, with a narrow diagonal tail.
    for points in [
        vec![at(-4.5, -7.0), at(-4.5, 5.0), at(-1.0, 1.5), at(5.5, 1.5)],
        vec![at(-1.5, 0.0), at(0.5, -1.0), at(4.0, 6.0), at(2.0, 7.0)],
    ] {
        painter.add(egui::Shape::convex_polygon(
            points,
            Color32::BLACK,
            Stroke::NONE,
        ));
    }
}

pub(crate) fn paint_toolbar(ui: &egui::Ui, rect: Rect, icon: ToolbarIcon, color: Color32) {
    let painter = ui.painter();
    let p = theme::palette(ui);
    let stroke = Stroke::new(1.25, color);
    let center = rect.center();
    let point = |x: f32, y: f32| Pos2::new(center.x + x * 0.86, center.y + y * 0.86);
    let outline = |min: Pos2, max: Pos2| Rect::from_two_pos(min, max);
    let triangle = |points: [Pos2; 3], fill: Color32| {
        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            fill,
            Stroke::NONE,
        ));
    };

    match icon {
        ToolbarIcon::Power => {
            power(painter, rect.shrink2(Vec2::new(3.0, 2.0)), color);
        }
        ToolbarIcon::Constraint => {
            for y in [-6.0, 0.0, 6.0] {
                painter.line_segment([point(-8.0, y), point(8.0, y)], stroke);
            }
            painter.circle_filled(point(-3.0, -6.0), 2.4, color);
            painter.circle_filled(point(4.0, 0.0), 2.4, color);
            painter.circle_filled(point(0.0, 6.0), 2.4, color);
        }
        ToolbarIcon::Inspector => {
            let bounds = outline(point(-8.0, -8.0), point(8.0, 8.0));
            painter.rect_stroke(
                bounds,
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([point(-3.0, -7.0), point(-3.0, 7.0)], stroke);
            painter.line_segment([point(0.0, -3.0), point(6.0, -3.0)], stroke);
            painter.line_segment([point(0.0, 1.0), point(6.0, 1.0)], stroke);
        }
        ToolbarIcon::InfoLine => {
            let bounds = outline(point(-8.0, -7.0), point(8.0, 7.0));
            painter.rect_stroke(
                bounds,
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.circle_filled(point(-4.0, -2.0), 1.2, color);
            painter.line_segment([point(-1.0, -2.0), point(5.0, -2.0)], stroke);
            painter.line_segment([point(-4.0, 2.0), point(5.0, 2.0)], stroke);
        }
        ToolbarIcon::Overview => {
            painter.line_segment([point(-8.0, -6.0), point(8.0, -6.0)], stroke);
            painter.rect_stroke(
                outline(point(-7.0, -2.0), point(-2.0, 5.0)),
                egui::CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                outline(point(0.0, -2.0), point(7.0, 5.0)),
                egui::CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        ToolbarIcon::Pool => {
            painter.line_segment([point(-8.0, -5.0), point(-1.0, -5.0)], stroke);
            painter.line_segment([point(-1.0, -5.0), point(1.0, -2.0)], stroke);
            painter.line_segment([point(1.0, -2.0), point(8.0, -2.0)], stroke);
            painter.line_segment([point(-8.0, -5.0), point(-8.0, 7.0)], stroke);
            painter.line_segment([point(-8.0, 7.0), point(8.0, 7.0)], stroke);
            painter.line_segment([point(8.0, 7.0), point(8.0, -2.0)], stroke);
        }
        ToolbarIcon::Mixer => {
            for (x, y) in [(-6.0, -2.0), (0.0, 3.0), (6.0, -4.0)] {
                painter.line_segment([point(x, -7.0), point(x, y - 1.5)], stroke);
                painter.line_segment([point(x, y + 1.5), point(x, 7.0)], stroke);
                painter.rect_filled(
                    outline(point(x - 2.0, y - 1.2), point(x + 2.0, y + 1.2)),
                    0.5,
                    color,
                );
            }
        }
        ToolbarIcon::Automation => {
            let nodes = [
                point(-7.0, 5.0),
                point(-2.0, -2.0),
                point(3.0, 2.0),
                point(7.0, -6.0),
            ];
            for pair in nodes.windows(2) {
                painter.line_segment([pair[0], pair[1]], stroke);
            }
            for node in nodes {
                painter.circle_filled(node, 1.7, color);
            }
        }
        ToolbarIcon::Previous => {
            painter.line_segment([point(-7.0, -7.0), point(-7.0, 7.0)], stroke);
            triangle([point(-2.0, 0.0), point(6.0, -7.0), point(6.0, 7.0)], color);
        }
        ToolbarIcon::Next => {
            painter.line_segment([point(7.0, -7.0), point(7.0, 7.0)], stroke);
            triangle(
                [point(2.0, 0.0), point(-6.0, -7.0), point(-6.0, 7.0)],
                color,
            );
        }
        ToolbarIcon::Cycle => {
            painter.line_segment([point(-7.0, -4.0), point(4.0, -4.0)], stroke);
            painter.line_segment([point(4.0, -4.0), point(2.0, -6.0)], stroke);
            painter.line_segment([point(4.0, -4.0), point(2.0, -2.0)], stroke);
            painter.line_segment([point(7.0, 4.0), point(-4.0, 4.0)], stroke);
            painter.line_segment([point(-4.0, 4.0), point(-2.0, 2.0)], stroke);
            painter.line_segment([point(-4.0, 4.0), point(-2.0, 6.0)], stroke);
        }
        ToolbarIcon::Stop => {
            painter.rect_filled(
                outline(point(-5.0, -5.0), point(5.0, 5.0)),
                egui::CornerRadius::ZERO,
                color,
            );
        }
        ToolbarIcon::Play | ToolbarIcon::PlayTool => {
            triangle(
                [point(-4.0, -7.0), point(7.0, 0.0), point(-4.0, 7.0)],
                color,
            );
            if matches!(icon, ToolbarIcon::PlayTool) {
                painter.line_segment([point(-8.0, -8.0), point(-4.0, -3.0)], stroke);
            }
        }
        ToolbarIcon::Record => {
            painter.circle_filled(point(0.0, 0.0), 6.0, theme::RECORD);
            painter.circle_stroke(point(0.0, 0.0), 6.0, Stroke::new(1.0, color));
        }
        ToolbarIcon::Select => {
            pointer(painter, center);
        }
        ToolbarIcon::Range => {
            painter.rect_stroke(
                outline(point(-7.0, -6.0), point(7.0, 6.0)),
                egui::CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([point(-3.0, -6.0), point(-3.0, 6.0)], stroke);
        }
        ToolbarIcon::Split => {
            for y in [-4.0, 4.0] {
                painter.circle_stroke(point(-4.5, y), 2.3, stroke);
            }
            painter.line_segment([point(-2.5, 3.0), point(6.5, -6.5)], stroke);
            painter.line_segment([point(-2.5, -3.0), point(6.5, 6.5)], stroke);
            painter.circle_filled(point(0.0, 0.0), 1.2, color);
        }
        ToolbarIcon::Glue => {
            // A simple capped bottle, readable without internal decoration.
            painter.rect_filled(outline(point(-2.0, -7.0), point(2.0, -4.0)), 0.5, color);
            painter.add(egui::Shape::closed_line(
                vec![
                    point(-2.0, -4.0),
                    point(2.0, -4.0),
                    point(4.0, -1.0),
                    point(4.0, 6.0),
                    point(-4.0, 6.0),
                    point(-4.0, -1.0),
                ],
                stroke,
            ));
        }
        ToolbarIcon::Erase => {
            painter.add(egui::Shape::closed_line(
                vec![
                    point(1.0, -6.0),
                    point(7.0, -1.0),
                    point(1.0, 6.0),
                    point(-3.0, 6.0),
                    point(-7.0, 2.0),
                ],
                stroke,
            ));
            painter.line_segment([point(-3.0, -2.0), point(3.0, 3.0)], stroke);
        }
        ToolbarIcon::Zoom => {
            painter.circle_stroke(point(-2.0, -2.0), 4.5, stroke);
            painter.line_segment([point(2.0, 2.0), point(8.0, 8.0)], stroke);
        }
        ToolbarIcon::Mute => {
            painter.line_segment([point(-8.0, -3.0), point(-4.0, -3.0)], stroke);
            painter.line_segment([point(-4.0, -3.0), point(1.0, -7.0)], stroke);
            painter.line_segment([point(1.0, -7.0), point(1.0, 7.0)], stroke);
            painter.line_segment([point(1.0, 7.0), point(-4.0, 3.0)], stroke);
            painter.line_segment([point(-4.0, 3.0), point(-8.0, 3.0)], stroke);
            painter.line_segment([point(4.0, -4.0), point(8.0, 4.0)], stroke);
            painter.line_segment([point(8.0, -4.0), point(4.0, 4.0)], stroke);
        }
        ToolbarIcon::TimeWarp => {
            let points = [
                point(-7.0, -6.0),
                point(-2.0, -2.0),
                point(2.0, 3.0),
                point(7.0, 7.0),
            ];
            for pair in points.windows(2) {
                painter.line_segment([pair[0], pair[1]], stroke);
            }
            painter.line_segment([point(5.0, -7.0), point(5.0, 7.0)], Stroke::new(1.0, color));
        }
        ToolbarIcon::Draw => {
            painter.add(egui::Shape::closed_line(
                vec![
                    point(-7.0, 7.0),
                    point(-5.0, 1.0),
                    point(4.0, -8.0),
                    point(8.0, -4.0),
                    point(-1.0, 5.0),
                ],
                stroke,
            ));
            painter.line_segment([point(2.0, -6.0), point(6.0, -2.0)], stroke);
            triangle(
                [point(-7.0, 7.0), point(-6.0, 3.0), point(-3.0, 6.0)],
                color,
            );
        }
        ToolbarIcon::Line => {
            painter.line_segment([point(-7.0, 6.0), point(7.0, -6.0)], stroke);
            painter.circle_filled(point(-7.0, 6.0), 1.8, color);
            painter.circle_filled(point(7.0, -6.0), 1.8, color);
        }
        ToolbarIcon::Color => {
            // Monochrome swatches keep this tool consistent with its neighbors.
            for (x, y) in [(-3.0, -3.0), (3.0, -3.0), (-3.0, 3.0), (3.0, 3.0)] {
                painter.rect_stroke(
                    outline(point(x - 2.0, y - 2.0), point(x + 2.0, y + 2.0)),
                    0.5,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
        }
        ToolbarIcon::RootKey => {
            let field = rect.shrink2(Vec2::new(7.0, 3.0));
            painter.rect_filled(field, 1.0, p.canvas);
            painter.text(
                field.left_center() + Vec2::new(4.0, 0.0),
                Align2::LEFT_CENTER,
                "-",
                FontId::proportional(11.0),
                p.text,
            );
            let c = field.right_center() - Vec2::new(5.0, 0.0);
            triangle(
                [
                    c + Vec2::new(-2.0, -1.0),
                    c + Vec2::new(2.0, -1.0),
                    c + Vec2::new(0.0, 2.0),
                ],
                p.text,
            );
        }
        ToolbarIcon::Autoscroll => {
            painter.line_segment([point(-6.0, -5.0), point(-2.0, -1.0)], stroke);
            painter.line_segment([point(-2.0, -1.0), point(-6.0, 3.0)], stroke);
            painter.line_segment([point(6.0, -5.0), point(2.0, -1.0)], stroke);
            painter.line_segment([point(2.0, -1.0), point(6.0, 3.0)], stroke);
            painter.line_segment([point(-7.0, 7.0), point(7.0, 7.0)], stroke);
        }
        ToolbarIcon::Snap => {
            painter.line_segment([point(-5.0, -6.0), point(-5.0, 1.0)], stroke);
            painter.line_segment([point(5.0, -6.0), point(5.0, 1.0)], stroke);
            painter.add(egui::Shape::line(
                (0..=20)
                    .map(|i| {
                        let a = i as f32 / 20.0 * std::f32::consts::PI;
                        point(-5.0 * a.cos(), 1.0 + 5.0 * a.sin())
                    })
                    .collect(),
                stroke,
            ));
            for x in [-5.0, 5.0] {
                painter.line_segment([point(x - 1.5, -6.0), point(x + 1.5, -6.0)], stroke);
            }
        }
        ToolbarIcon::SnapType => {
            let field = rect.shrink2(Vec2::new(3.0, 3.0));
            painter.rect_filled(field, 1.0, p.canvas);
            let c = field.center() - Vec2::new(4.0, 0.0);
            for delta in [-3.0, 3.0] {
                painter.line_segment(
                    [c + Vec2::new(delta, -6.0), c + Vec2::new(delta, 6.0)],
                    Stroke::new(1.1, p.muted),
                );
                painter.line_segment(
                    [c + Vec2::new(-6.0, delta), c + Vec2::new(6.0, delta)],
                    Stroke::new(1.1, p.muted),
                );
            }
            let a = field.right_center() - Vec2::new(5.0, 0.0);
            triangle(
                [
                    a + Vec2::new(-2.5, -1.5),
                    a + Vec2::new(2.5, -1.5),
                    a + Vec2::new(0.0, 2.0),
                ],
                p.muted,
            );
        }
        ToolbarIcon::ZeroCrossing => {
            painter.line_segment(
                [point(-7.0, 0.0), point(7.0, 0.0)],
                Stroke::new(0.8, color.gamma_multiply(0.6)),
            );
            painter.add(egui::Shape::line(
                (0..=28)
                    .map(|i| {
                        let x = -7.0 + i as f32 * 0.5;
                        point(x, -(x * std::f32::consts::PI / 7.0).sin() * 4.5)
                    })
                    .collect(),
                stroke,
            ));
        }
    }
}
