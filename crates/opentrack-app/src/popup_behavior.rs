//! Shared hover dismissal for menus and popups.
use eframe::egui::{self, Id, Order, Pos2, Rect, Vec2};

const ACTIVATION_ZONE_ID: &str = "opentrack-popup-activation-zone";

/// Close egui menus/popups after the pointer leaves both their foreground
/// surface and the control that activated them. Foreground areas include nested
/// submenus, so moving between a menu and its child keeps the chain open.
///
/// The activation zone is the whole widget that opened the popup, not only the
/// pivot point: a popup opened from the middle of a button stays open while the
/// pointer is anywhere on that button.
pub(crate) fn close_when_pointer_leaves(ctx: &egui::Context) -> bool {
    // All viewports share one data store, so the zone has to be scoped to the
    // viewport that opened the popup. Otherwise the main window's check, which
    // has no popup open, deletes the zone the Channel Settings window stored,
    // and that window re-seeds it from the moving pointer every frame.
    let activation_id = Id::new((ACTIVATION_ZONE_ID, ctx.viewport_id()));
    if !egui::Popup::is_any_open(ctx) {
        ctx.data_mut(|data| data.remove::<Rect>(activation_id));
        return false;
    }

    let pointer = ctx.input(|input| input.pointer.hover_pos());
    let activation_zone = ctx
        .data(|data| data.get_temp::<Rect>(activation_id))
        .or_else(|| {
            // Seeded once per popup lifecycle. A click inside a nested popup
            // must not re-anchor the chain to that popup's own widget.
            let clicked = ctx.interaction_snapshot(|interaction| interaction.clicked);
            clicked
                .and_then(|id| ctx.read_response(id))
                .map(|response| response.rect)
                .or_else(|| pointer.map(|point| Rect::from_center_size(point, Vec2::splat(24.0))))
        });
    if let Some(zone) = activation_zone {
        ctx.data_mut(|data| data.insert_temp(activation_id, zone));
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
    if pointer_in_popup_zones(pointer, activation_zone, &foreground_rects) {
        true
    } else {
        egui::Popup::close_all(ctx);
        ctx.data_mut(|data| data.remove::<Rect>(activation_id));
        false
    }
}

fn pointer_in_popup_zones(
    pointer: Option<Pos2>,
    activation_zone: Option<Rect>,
    foreground_rects: &[Rect],
) -> bool {
    let Some(pointer) = pointer else {
        return false;
    };
    activation_zone.is_some_and(|zone| zone.expand(8.0).contains(pointer))
        || foreground_rects
            .iter()
            .any(|rect| rect.expand(8.0).contains(pointer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_stays_open_anywhere_on_the_activating_control_and_inside_the_popup() {
        let button = Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(70.0, 40.0));
        let popup = Rect::from_min_max(Pos2::new(40.0, 50.0), Pos2::new(100.0, 120.0));
        // An edge of the button is far from its centre, which the old 24 px
        // pivot square would have dismissed.
        assert!(pointer_in_popup_zones(
            Some(Pos2::new(66.0, 37.0)),
            Some(button),
            &[popup]
        ));
        assert!(pointer_in_popup_zones(
            Some(Pos2::new(50.0, 60.0)),
            Some(button),
            &[popup]
        ));
    }

    #[test]
    fn popup_closes_after_pointer_leaves_the_control_and_the_popup_surface() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(Pos2::new(30.0, 30.0))],
                ..Default::default()
            },
            |ui| {
                let ctx = ui.ctx();
                ctx.data_mut(|data| {
                    data.insert_temp(
                        Id::new(ACTIVATION_ZONE_ID),
                        Rect::from_min_max(Pos2::new(20.0, 20.0), Pos2::new(60.0, 40.0)),
                    );
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
    fn clicks_inside_popup_do_not_reanchor_the_activation_zone() {
        let button = Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(70.0, 40.0));
        let menu = Rect::from_min_max(Pos2::new(40.0, 40.0), Pos2::new(100.0, 100.0));
        assert!(pointer_in_popup_zones(
            Some(Pos2::new(80.0, 80.0)),
            Some(button),
            &[menu]
        ));
        assert!(!pointer_in_popup_zones(
            Some(Pos2::new(200.0, 200.0)),
            Some(button),
            &[menu]
        ));
    }
}
