//! Dense signed waveform silhouettes shared by completed and growing events.
use super::*;
pub(super) fn draw(
    painter: &egui::Painter,
    rect: Rect,
    visible: Rect,
    channels: usize,
    duration: f64,
    mut columns: impl FnMut(f64, f64, usize, usize) -> Vec<[f32; 2]>,
) {
    let visible = rect.intersect(visible);
    if visible.width() <= 0.0 || rect.width() <= 0.0 || duration <= 0.0 {
        return;
    }
    let start = ((visible.left() - rect.left()) / rect.width()) as f64 * duration;
    let end = ((visible.right() - rect.left()) / rect.width()) as f64 * duration;
    let count = (visible.width() * painter.ctx().pixels_per_point())
        .ceil()
        .clamp(2.0, 4096.0) as usize;
    let lanes = channels.clamp(1, 2);
    let lane_height = rect.height().max(1.0) / lanes as f32;
    let color = if painter.ctx().global_style().visuals.dark_mode {
        Color32::from_gray(215)
    } else {
        Color32::from_gray(40)
    };
    let painter = painter.with_clip_rect(visible);
    for channel in 0..lanes {
        let samples = columns(start, end, count, channel);
        let center = rect.top() + lane_height * (channel as f32 + 0.5);
        let amplitude = (lane_height * 0.5 - 1.0).max(0.0);
        let mut mesh = egui::Mesh::default();
        let mut upper = Vec::new();
        let mut lower = Vec::new();
        for (i, range) in samples.iter().enumerate() {
            let x = egui::lerp(
                visible.left()..=visible.right(),
                i as f32 / (count - 1) as f32,
            );
            let a = Pos2::new(x, center - range[1].clamp(-1.0, 1.0) * amplitude);
            let b = Pos2::new(x, center - range[0].clamp(-1.0, 1.0) * amplitude);
            upper.push(a);
            lower.push(b);
            mesh.colored_vertex(a, color.gamma_multiply(0.65));
            mesh.colored_vertex(b, color.gamma_multiply(0.65));
            if i > 0 {
                let n = (i * 2) as u32;
                mesh.add_triangle(n - 2, n - 1, n);
                mesh.add_triangle(n - 1, n + 1, n);
            }
        }
        painter.add(egui::Shape::mesh(mesh));
        if upper.len() > 1 {
            painter.add(egui::Shape::line(
                upper,
                Stroke::new(0.7, color.gamma_multiply(0.8)),
            ));
            painter.add(egui::Shape::line(
                lower,
                Stroke::new(0.7, color.gamma_multiply(0.8)),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipped_waveform_draws_dense_connected_stereo_mesh_and_queries_correct_time_interval() {
        let ctx = egui::Context::default();
        let mut queried = Vec::new();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 120.0))),
                ..Default::default()
            },
            |ui| {
                draw(
                    ui.painter(),
                    Rect::from_min_size(Pos2::new(-100.0, 0.0), Vec2::new(1000.0, 100.0)),
                    Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 100.0)),
                    2,
                    10.0,
                    |a, b, n, c| {
                        queried.push((a, b, n, c));
                        vec![if c == 0 { [0.25, 0.5] } else { [-0.75, -0.25] }; n]
                    },
                );
            },
        );
        assert_eq!(queried.len(), 2);
        assert!((queried[0].0 - 1.0).abs() < 0.0001);
        assert!((queried[0].1 - 5.0).abs() < 0.0001);
        assert!(queried[0].2 >= 400);
        let meshes = out
            .shapes
            .iter()
            .filter_map(|s| {
                if let egui::Shape::Mesh(m) = &s.shape {
                    Some(m)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(meshes.len(), 2);
        assert!(
            meshes
                .iter()
                .all(|m| m.vertices.len() >= 800 && m.indices.len() >= 2394)
        );
        assert!(meshes[0].vertices.iter().all(|v| v.pos.y < 34.5));
        assert!(meshes[1].vertices.iter().all(|v| v.pos.y > 77.5));
        let _ = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        out.textures_delta.clear();
    }
}
