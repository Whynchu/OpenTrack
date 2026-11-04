//! Shared hover dismissal for menus and popups.
use eframe::egui::{self, Id, Order, Pos2, Rect, Vec2};

const ACTIVATION_ZONE_ID: &str = "opentrack-popup-activation-zone";
const TOP_MENU_STATE_ID: &str = "opentrack-top-menu-state";

/// The top-level menu bar buttons seen on the previous frame.
///
/// Keeping the rectangles lets the next frame move an open menu to the entry
/// under the pointer before any popup is painted, so two top-level menus are
/// never visible together.
#[derive(Clone, Default)]
struct TopMenuState {
    buttons: Vec<(Id, Rect)>,
    open: Option<Id>,
    mode: u8,
}

fn top_menu_state_id(ctx: &egui::Context) -> Id {
    Id::new((TOP_MENU_STATE_ID, ctx.viewport_id()))
}

/// Starts a menu-bar frame and applies hover switching to the previous frame's
/// open top-level menu.
///
/// A menu that is already open follows the pointer to another menu-bar entry.
/// A click on a different entry closes the old menu first and lets the ordinary
/// menu-button click open the new one; a click on the same entry toggles it
/// closed as usual.
pub(crate) fn begin_menu_bar(ui: &egui::Ui, mode: u8) {
    let ctx = ui.ctx().clone();
    let state_id = top_menu_state_id(&ctx);
    let state = ctx
        .data(|data| data.get_temp::<TopMenuState>(state_id))
        .unwrap_or_default();
    if state.mode != mode {
        // Opening or closing a project swaps the whole menu bar. Do not leave
        // a popup from the other bar floating over the new one.
        egui::Popup::close_all(&ctx);
        ctx.data_mut(|data| {
            data.insert_temp(
                state_id,
                TopMenuState {
                    mode,
                    ..TopMenuState::default()
                },
            )
        });
        return;
    }
    let open = state.open.filter(|id| egui::Popup::is_id_open(&ctx, *id));
    if let Some(open) = open {
        let pointer = ctx.input(|input| input.pointer.hover_pos());
        let clicked = ctx.input(|input| input.pointer.any_click());
        if let Some(pointer) = pointer {
            let target = state
                .buttons
                .iter()
                .find(|(id, rect)| *id != open && rect.contains(pointer))
                .map(|(id, _)| *id);
            if let Some(target) = target {
                if clicked {
                    // The clicked button will open itself. Close the old menu
                    // now so both are never painted in the same frame.
                    egui::Popup::close_all(&ctx);
                } else {
                    egui::Popup::open_id(&ctx, target);
                }
            }
        }
        // The whole bar is an activation area, so moving between menu entries
        // keeps the menu chain open until the pointer leaves it entirely.
        if let Some(bar) = state
            .buttons
            .iter()
            .map(|(_, rect)| *rect)
            .reduce(Rect::union)
        {
            keep_activation_zone(&ctx, bar);
        }
    }
    ctx.data_mut(|data| {
        data.insert_temp(
            state_id,
            TopMenuState {
                mode,
                ..TopMenuState::default()
            },
        )
    });
}

/// Records one top-level menu button for the next frame's hover switching.
pub(crate) fn menu_bar_button(
    ui: &mut egui::Ui,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let response = ui.menu_button(label, add_contents).response;
    let popup_id = egui::Popup::default_response_id(&response);
    let state_id = top_menu_state_id(ui.ctx());
    ui.ctx().data_mut(|data| {
        let mut state = data.get_temp::<TopMenuState>(state_id).unwrap_or_default();
        state.buttons.push((popup_id, response.rect));
        data.insert_temp(state_id, state);
    });
}

/// Stores which top-level menu is open after the bar has been drawn.
pub(crate) fn end_menu_bar(ui: &egui::Ui) {
    let ctx = ui.ctx().clone();
    let state_id = top_menu_state_id(&ctx);
    let state = ctx
        .data(|data| data.get_temp::<TopMenuState>(state_id))
        .unwrap_or_default();
    let open = state
        .buttons
        .iter()
        .find(|(id, _)| egui::Popup::is_id_open(&ctx, *id))
        .map(|(id, _)| *id);
    ctx.data_mut(|data| {
        data.insert_temp(
            state_id,
            TopMenuState {
                buttons: state.buttons,
                open,
                mode: state.mode,
            },
        )
    });
}

/// Extends the shared activation zone so the pointer may travel over `rect`
/// without dismissing the open popup chain.
fn keep_activation_zone(ctx: &egui::Context, rect: Rect) {
    let activation_id = Id::new((ACTIVATION_ZONE_ID, ctx.viewport_id()));
    ctx.data_mut(|data| {
        let zone = data
            .get_temp::<Rect>(activation_id)
            .map_or(rect, |zone| zone.union(rect));
        data.insert_temp(activation_id, zone);
    });
}

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

    fn menu_frame(ctx: &egui::Context, events: Vec<egui::Event>) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420.0, 240.0))),
                events,
                ..Default::default()
            },
            |ui| {
                begin_menu_bar(ui, 0);
                egui::MenuBar::new().ui(ui, |ui| {
                    menu_bar_button(ui, "One", |ui| {
                        let _ = ui.button("First item");
                    });
                    menu_bar_button(ui, "Two", |ui| {
                        let _ = ui.button("Second item");
                    });
                });
                end_menu_bar(ui);
            },
        );
        output.textures_delta.clear();
    }

    fn menu_state(ctx: &egui::Context) -> TopMenuState {
        ctx.data(|data| data.get_temp::<TopMenuState>(top_menu_state_id(ctx)))
            .expect("menu bar state")
    }

    #[test]
    fn an_open_menu_follows_hover_between_bar_entries() {
        let ctx = egui::Context::default();
        menu_frame(&ctx, Vec::new());
        let buttons = menu_state(&ctx).buttons;
        assert_eq!(buttons.len(), 2);
        let (one_id, one_rect) = buttons[0];
        let (two_id, two_rect) = buttons[1];

        let button = |pos: Pos2, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };

        // Press and release on the first entry opens it.
        menu_frame(
            &ctx,
            vec![
                egui::Event::PointerMoved(one_rect.center()),
                button(one_rect.center(), true),
            ],
        );
        menu_frame(&ctx, vec![button(one_rect.center(), false)]);
        assert!(egui::Popup::is_id_open(&ctx, one_id));
        assert_eq!(menu_state(&ctx).open, Some(one_id));

        // Merely moving to the second entry switches the open menu.
        menu_frame(&ctx, vec![egui::Event::PointerMoved(two_rect.center())]);
        assert!(!egui::Popup::is_id_open(&ctx, one_id));
        assert!(egui::Popup::is_id_open(&ctx, two_id));
        assert_eq!(menu_state(&ctx).open, Some(two_id));

        // A click on the open entry closes it.
        menu_frame(
            &ctx,
            vec![
                egui::Event::PointerMoved(two_rect.center()),
                button(two_rect.center(), true),
            ],
        );
        menu_frame(&ctx, vec![button(two_rect.center(), false)]);
        assert!(!egui::Popup::is_id_open(&ctx, two_id));
        assert_eq!(menu_state(&ctx).open, None);
    }
}
