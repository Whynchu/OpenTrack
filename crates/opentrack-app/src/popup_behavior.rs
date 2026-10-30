//! Shared hover dismissal for menus and popups.
use eframe::egui::{self, Id, Order, Pos2, Rect, Vec2};

const ACTIVATION_POINT_ID: &str = "opentrack-popup-activation-point";

/// Close egui menus/popups after the pointer leaves both their foreground
/// surface and the point that activated them. Foreground areas include nested
/// submenus, so moving between a menu and its child keeps the chain open.
pub(crate) fn close_when_pointer_leaves(ctx: &egui::Context) -> bool {
    // All viewports share one data store, so the point has to be scoped to the
    // viewport that opened the popup. Otherwise the main window's check, which
    // has no popup open, deletes the point the Channel Settings window stored,
    // and that window re-seeds it from the moving pointer every frame.
    let activation_id = Id::new((ACTIVATION_POINT_ID, ctx.viewport_id()));
    if !egui::Popup::is_any_open(ctx) {
        ctx.data_mut(|data| data.remove_temp::<Pos2>(activation_id));
        return false;
    }

    let pointer = ctx.input(|input| input.pointer.hover_pos());
    let activation_point = ctx.data(|data| data.get_temp::<Pos2>(activation_id));
    let activation_point = if ctx.input(|input| input.pointer.any_click()) {
        // A newly opened popup starts a new activation cycle. Preserve the
        // original click position; later clicks inside a nested popup must
        // not move the point and keep the chain alive after leaving it.
        activation_point.or(pointer)
    } else {
        activation_point.or(pointer)
    };
    if let Some(point) = activation_point {
        ctx.data_mut(|data| data.insert_temp(activation_id, point));
    }

    let foreground_rects = ctx.memory(|memory| {
        memory
            .areas()
            .visible_layer_ids()
            .into_iter()
            .filter(|layer| layer.order == Order::Foreground)
            .filter_map(|layer| memory.area_rect(layer.id))
            .collect::<Vec<_>>()
    });
    if pointer_in_popup_zones(pointer, activation_point, &foreground_rects) {
        true
    } else {
        egui::Popup::close_all(ctx);
        ctx.data_mut(|data| data.remove_temp::<Pos2>(activation_id));
        false
    }
}

fn pointer_in_popup_zones(
    pointer: Option<Pos2>,
    activation_point: Option<Pos2>,
    foreground_rects: &[Rect],
) -> bool {
    let Some(pointer) = pointer else {
        return false;
    };
    let pointer_near_activation = activation_point.is_some_and(|activation| {
        Rect::from_center_size(activation, Vec2::splat(24.0)).contains(pointer)
    });
    pointer_near_activation
        || foreground_rects
            .iter()
            .any(|rect| rect.expand(8.0).contains(pointer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_stays_open_at_activation_point_and_inside_foreground_area() {
        let activation = Pos2::new(30.0, 30.0);
        let popup = Rect::from_min_max(Pos2::new(40.0, 50.0), Pos2::new(100.0, 120.0));
        assert!(pointer_in_popup_zones(
            Some(activation),
            Some(activation),
            &[popup]
        ));
        assert!(pointer_in_popup_zones(
            Some(Pos2::new(39.0, 48.0)),
            Some(activation),
            &[popup]
        ));
    }

    #[test]
    fn popup_closes_after_pointer_leaves_activation_and_popup_surface() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(Pos2::new(30.0, 30.0))],
                ..Default::default()
            },
            |ui| {
                let ctx = ui.ctx();
                ctx.data_mut(|data| {
                    data.insert_temp(Id::new(ACTIVATION_POINT_ID), Pos2::new(30.0, 30.0));
                });
                egui::Popup::open_id(ctx, Id::new("test-popup"));
                assert!(close_when_pointer_leaves(ctx));
            },
        );
        output.textures_delta.clear();
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(Pos2::new(200.0, 200.0))],
                ..Default::default()
            },
            |ui| {
                let ctx = ui.ctx();
                assert!(!close_when_pointer_leaves(ctx));
                assert!(!egui::Popup::is_any_open(ctx));
            },
        );
        output.textures_delta.clear();
    }

    #[test]
    fn clicks_inside_popup_do_not_reanchor_activation_point() {
        let activation = Pos2::new(30.0, 30.0);
        let menu = Rect::from_min_max(Pos2::new(40.0, 40.0), Pos2::new(100.0, 100.0));
        assert!(pointer_in_popup_zones(
            Some(Pos2::new(80.0, 80.0)),
            Some(activation),
            &[menu]
        ));
        assert!(!pointer_in_popup_zones(
            Some(Pos2::new(200.0, 200.0)),
            Some(activation),
            &[menu]
        ));
    }
}
