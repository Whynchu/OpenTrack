//! Tabler-style cursor overlays for editing tools without native equivalents.
use crate::icons::ToolbarIcon;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};

pub(crate) fn tool_cursor(ui: &egui::Ui, icon: ToolbarIcon) {
    tool_cursor_at(ui, icon, ui.input(|i| i.pointer.hover_pos()));
}

pub(crate) fn tool_cursor_at(ui: &egui::Ui, icon: ToolbarIcon, hotspot: Option<Pos2>) {
    ui.ctx()
        .set_cursor_icon(if matches!(icon, ToolbarIcon::Draw) {
            egui::CursorIcon::None
        } else {
            egui::CursorIcon::Crosshair
        });
    paint_cursor(ui, icon, hotspot);
}

pub(crate) fn tool_cursor_only_at(ui: &egui::Ui, icon: ToolbarIcon, hotspot: Option<Pos2>) {
    ui.ctx().set_cursor_icon(egui::CursorIcon::None);
    paint_cursor(ui, icon, hotspot);
}

fn paint_cursor(ui: &egui::Ui, icon: ToolbarIcon, hotspot: Option<Pos2>) {
    let Some(pointer) = hotspot else { return };
    let Some(id) = (match icon {
        ToolbarIcon::Split => Some(crate::icon_system::IconId::Split),
        ToolbarIcon::Draw => Some(crate::icon_system::IconId::Draw),
        ToolbarIcon::Zoom => Some(crate::icon_system::IconId::Zoom),
        _ => None,
    }) else {
        return;
    };
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("tabler-tool-cursor"),
    ));
    let size = 20.0;
    let offset = if matches!(icon, ToolbarIcon::Split) {
        Vec2::ZERO
    } else {
        Vec2::new(6.0, 4.0)
    };
    let rect = Rect::from_min_size(pointer + offset, Vec2::splat(size));
    for halo in [
        Vec2::new(-1.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(0.0, 1.0),
    ] {
        crate::icon_system::paint_app_icon_on(
            ui.ctx(),
            &painter,
            rect.translate(halo),
            id,
            Color32::WHITE,
        );
    }
    crate::icon_system::paint_app_icon_on(ui.ctx(), &painter, rect, id, Color32::BLACK);
}
