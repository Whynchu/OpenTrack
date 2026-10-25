use eframe::egui::{self, Color32, FontId, Stroke, Vec2};

// Signal colors retain their meaning in either Windows app theme.
pub const RECORD: Color32 = Color32::from_rgb(220, 65, 65);
pub const WARNING: Color32 = Color32::from_rgb(205, 150, 35);
pub const TRACK_AUDIO: Color32 = Color32::from_rgb(45, 140, 175);
pub const TRACK_INSTRUMENT: Color32 = Color32::from_rgb(139, 112, 190);
pub const TRACK_MIDI: Color32 = Color32::from_rgb(65, 150, 85);

#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Color32,
    pub canvas_alternate: Color32,
    pub surface: Color32,
    pub surface_raised: Color32,
    pub grid: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub solo_defeat: Color32,
    pub master_surface: Color32,
    pub midi_key_black: Color32,
    pub midi_key_white: Color32,
    pub midi_row_black: Color32,
    pub midi_row_white: Color32,
}

pub fn palette(ui: &egui::Ui) -> Palette {
    palette_for(ui.visuals())
}

pub fn toolbar_button_fill(visuals: &egui::Visuals) -> Color32 {
    let from = visuals.widgets.inactive.bg_fill;
    let amount = if visuals.dark_mode { 0.48 } else { 0.70 };
    let mix = |a: u8| {
        (f32::from(a) + (255.0 - f32::from(a)) * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(mix(from.r()), mix(from.g()), mix(from.b()))
}

/// Small, consistent hover response for buttons drawn by custom Project views.
pub fn hover_button_fill(base: Color32, visuals: &egui::Visuals) -> Color32 {
    let amount = if visuals.dark_mode { 0.14 } else { 0.10 };
    let target = if visuals.dark_mode { 255.0 } else { 0.0 };
    let mix = |channel: u8| {
        (f32::from(channel) + (target - f32::from(channel)) * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(mix(base.r()), mix(base.g()), mix(base.b()), base.a())
}

fn set_hover_style(style: &mut egui::Style) {
    let base = style.visuals.widgets.inactive.bg_fill;
    style.visuals.widgets.hovered.bg_fill = hover_button_fill(base, &style.visuals);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(
        0.8,
        style
            .visuals
            .widgets
            .inactive
            .fg_stroke
            .color
            .gamma_multiply(0.35),
    );
}

fn palette_for(visuals: &egui::Visuals) -> Palette {
    Palette {
        canvas: visuals.extreme_bg_color,
        canvas_alternate: visuals.faint_bg_color,
        surface: visuals.panel_fill,
        surface_raised: visuals.widgets.inactive.bg_fill,
        grid: visuals.widgets.noninteractive.bg_stroke.color,
        text: visuals.text_color(),
        muted: visuals.weak_text_color(),
        accent: visuals.hyperlink_color,
        solo_defeat: visuals.widgets.hovered.bg_fill,
        master_surface: visuals.window_fill,
        midi_key_black: if visuals.dark_mode {
            Color32::from_gray(35)
        } else {
            Color32::from_gray(100)
        },
        midi_key_white: if visuals.dark_mode {
            Color32::from_gray(85)
        } else {
            Color32::from_gray(245)
        },
        midi_row_black: visuals.extreme_bg_color,
        midi_row_white: visuals.faint_bg_color,
    }
}

pub mod type_size {
    pub const MENU: f32 = 13.0;
    pub const TINY: f32 = 8.0;
    pub const SMALL: f32 = 9.0;
    pub const LABEL: f32 = 10.0;
    pub const RULER: f32 = 11.0;
    pub const BODY: f32 = 12.0;
    pub const TRACK_COMPACT: f32 = 11.0;
    pub const TRACK_NAME: f32 = 13.0;
}

pub mod spacing {
    pub const ITEM_GAP_X: f32 = 6.0;
    pub const ITEM_GAP_Y: f32 = 4.0;
    pub const BUTTON_PADDING_X: f32 = 7.0;
    pub const BUTTON_PADDING_Y: f32 = 3.0;
    pub const MENU_BAR_HEIGHT: f32 = 24.0;
    pub const MENU_ITEM_HEIGHT: f32 = 24.0;
    pub const MENU_ROW_HEIGHT: f32 = 21.0;
    pub const MENU_WIDTH: f32 = 246.0;
    pub const MENU_MARGIN: i8 = 3;
    pub const MENU_BAR_ITEM_PADDING_X: f32 = 8.0;
    pub const MENU_POPUP_ITEM_PADDING_X: f32 = 7.0;
}

pub fn install(ctx: &egui::Context) {
    // eframe/winit reads the Windows app theme at startup and handles ThemeChanged.
    // Explicitly override any forced theme left in persisted egui settings.
    ctx.set_theme(egui::ThemePreference::System);
    ctx.options_mut(|options| options.sync_window_theme = true);
    for (theme, mut visuals) in [
        (egui::Theme::Light, egui::Visuals::light()),
        (egui::Theme::Dark, egui::Visuals::dark()),
    ] {
        visuals.hyperlink_color = if visuals.dark_mode {
            Color32::from_rgb(96, 176, 245)
        } else {
            Color32::from_rgb(0, 103, 192)
        };
        visuals.selection.bg_fill = if visuals.dark_mode {
            Color32::from_rgb(35, 78, 115)
        } else {
            Color32::from_rgb(190, 220, 246)
        };
        visuals.selection.stroke = Stroke::new(1.0, visuals.text_color());
        let mut hover_style = egui::Style {
            visuals: visuals.clone(),
            ..Default::default()
        };
        set_hover_style(&mut hover_style);
        visuals.widgets.hovered = hover_style.visuals.widgets.hovered.clone();
        visuals.window_corner_radius = egui::CornerRadius::same(2);
        visuals.menu_corner_radius = egui::CornerRadius::same(2);
        ctx.set_visuals_of(theme, visuals);
    }
    install_windows_font(ctx);
    ctx.all_styles_mut(|style| {
        for (role, size) in [
            (egui::TextStyle::Body, 13.0),
            (egui::TextStyle::Button, 13.0),
            (egui::TextStyle::Small, 11.0),
            (egui::TextStyle::Heading, 17.0),
        ] {
            style.text_styles.insert(role, FontId::proportional(size));
        }
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, FontId::monospace(12.0));
        style.spacing.interact_size.y = 22.0;
        style.spacing.item_spacing = Vec2::new(spacing::ITEM_GAP_X, spacing::ITEM_GAP_Y);
        style.spacing.button_padding =
            Vec2::new(spacing::BUTTON_PADDING_X, spacing::BUTTON_PADDING_Y);
    });
}

fn install_windows_font(ctx: &egui::Context) {
    #[cfg(windows)]
    if let Some(windows_dir) = std::env::var_os("WINDIR") {
        let path = std::path::PathBuf::from(windows_dir)
            .join("Fonts")
            .join("segoeui.ttf");
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "windows-ui".into(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "windows-ui".into());
            ctx.set_fonts(fonts);
        }
    }
}

pub fn menu_bar_fill() -> Color32 {
    egui::Visuals::dark().panel_fill
}
pub fn menu_bar_style(style: &mut egui::Style) {
    let background = menu_bar_fill();
    style.visuals.panel_fill = background;
    style.visuals.override_text_color = Some(Color32::WHITE);
    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.bg_fill = background;
        widget.weak_bg_fill = background;
        widget.fg_stroke.color = Color32::WHITE;
    }
    set_hover_style(style);
    style.spacing.item_spacing = Vec2::ZERO;
    style.spacing.button_padding = Vec2::new(spacing::MENU_BAR_ITEM_PADDING_X, 1.0);
    style.spacing.interact_size = Vec2::new(18.0, spacing::MENU_ITEM_HEIGHT);
    style.override_font_id = Some(FontId::proportional(type_size::MENU));
}

pub fn menu_popup_style(style: &mut egui::Style) {
    style.visuals.override_text_color = None;
    let base = if style.visuals.dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    style.visuals.widgets = base.widgets;

    set_hover_style(style);
    style.spacing.item_spacing = Vec2::ZERO;
    style.spacing.button_padding = Vec2::new(spacing::MENU_POPUP_ITEM_PADDING_X, 1.0);
    style.spacing.interact_size = Vec2::new(18.0, spacing::MENU_ROW_HEIGHT);
    style.spacing.menu_margin = egui::Margin::same(spacing::MENU_MARGIN);
    style.spacing.menu_width = spacing::MENU_WIDTH;
    style.override_font_id = Some(FontId::proportional(type_size::MENU));
}

pub fn channel_settings_menu_popup_style(style: &mut egui::Style) {
    set_hover_style(style);
    style.spacing.item_spacing = Vec2::ZERO;
    style.spacing.button_padding = Vec2::new(8.0, 1.0);
    style.spacing.interact_size = Vec2::new(18.0, 22.0);
    style.spacing.menu_margin = egui::Margin::same(2);
    style.spacing.menu_width = 220.0;
    style.override_font_id = Some(FontId::proportional(type_size::MENU));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawn_surfaces_and_text_follow_both_themes() {
        for visuals in [egui::Visuals::light(), egui::Visuals::dark()] {
            let palette = palette_for(&visuals);
            assert_eq!(palette.canvas, visuals.extreme_bg_color);
            assert_eq!(palette.text, visuals.text_color());
            assert_eq!(palette.surface, visuals.panel_fill);
            let mut bar_style = egui::Style {
                visuals: visuals.clone(),
                ..Default::default()
            };
            menu_bar_style(&mut bar_style);
            assert_eq!(bar_style.visuals.dark_mode, visuals.dark_mode);
            assert_eq!(bar_style.visuals.text_color(), Color32::WHITE);

            // Popups use the normal surface and keep the theme's regular ink;
            // the lighter menu-bar surface has its own dark ink in dark mode.
            let mut popup_style = egui::Style {
                visuals: visuals.clone(),
                ..Default::default()
            };
            menu_popup_style(&mut popup_style);
            assert_eq!(popup_style.visuals.text_color(), palette.text);
        }
        assert_ne!(
            palette_for(&egui::Visuals::light()).text,
            palette_for(&egui::Visuals::dark()).text
        );
    }

    #[test]
    fn install_restores_system_preference_over_a_saved_override() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::ThemePreference::Dark);
        install(&ctx);
        assert_eq!(
            ctx.options(|o| o.theme_preference),
            egui::ThemePreference::System
        );
        assert!(ctx.options(|o| o.sync_window_theme));
    }

    #[test]
    fn system_theme_changes_update_drawn_views_and_menus() {
        let ctx = egui::Context::default();
        install(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark, egui::Theme::Light] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    system_theme: Some(theme),
                    ..Default::default()
                },
                |ui| {
                    assert_eq!(ui.visuals().dark_mode, theme == egui::Theme::Dark);
                    assert_eq!(palette(ui).text, ui.visuals().text_color());
                    let mut menu = ui.style().as_ref().clone();
                    menu_popup_style(&mut menu);
                    assert_eq!(menu.visuals.dark_mode, ui.visuals().dark_mode);
                },
            );
            // This headless test has no renderer to consume font atlas updates.
            output.textures_delta.clear();
        }
    }
}
