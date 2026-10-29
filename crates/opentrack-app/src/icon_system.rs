//! Pinned Tabler toolbar icon registry and egui texture cache.
//!
//! Assets are from Tabler Icons v3.49.0 (commit bbed884), MIT licensed. See
//! `assets/tabler/LICENSE` and `assets/tabler/README.md`.
use crate::icons::ToolbarIcon;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum IconId {
    ProjectPower,
    DelayCompensation,
    Inspector,
    EventInfo,
    Overview,
    MediaPool,
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
    DrawCursor,
    Line,
    PlayTool,
    Color,
    RootKey,
    Autoscroll,
    Snap,
    SnapType,
    ZeroCrossing,
    TrackAudio,
    TrackMidi,
    TrackInstrument,
    Phase,
    Insert,
    Eq,
    Send,
    Monitor,
    Copy,
    Paste,
    Routing,
    Target,
    Channel,
    ChevronRight,
    ChevronUp,
    ChevronDown,
    Undo,
    Reveal,
    TakeLanes,
    GroupLanes,
    Power,
    SavePreset,
    Close,
    TrackMusic,
    Document,
    GlyphFx,
    GlyphListen,
    GlyphMute,
    GlyphRecord,
    GlyphSolo,
    GlyphWrite,
}

impl IconId {
    fn svg(self) -> &'static [u8] {
        match self {
            Self::ProjectPower => include_bytes!("../assets/tabler/power.svg"),
            Self::DelayCompensation => include_bytes!("../assets/tabler/clock-exclamation.svg"),
            Self::Inspector => include_bytes!("../assets/tabler/layout-sidebar.svg"),
            Self::EventInfo => include_bytes!("../assets/tabler/list-details.svg"),
            Self::Overview => include_bytes!("../assets/tabler/chart-area-line.svg"),
            Self::MediaPool => include_bytes!("../assets/tabler/folder.svg"),
            Self::Mixer => include_bytes!("../assets/tabler/adjustments-horizontal.svg"),
            Self::Automation => include_bytes!("../assets/tabler/automation.svg"),
            Self::Previous => include_bytes!("../assets/tabler/player-skip-back.svg"),
            Self::Next => include_bytes!("../assets/tabler/player-skip-forward.svg"),
            Self::Cycle => include_bytes!("../assets/tabler/repeat.svg"),
            Self::Stop => include_bytes!("../assets/tabler/player-stop.svg"),
            Self::Play | Self::PlayTool => include_bytes!("../assets/tabler/player-play.svg"),
            Self::Record => include_bytes!("../assets/tabler/player-record.svg"),
            Self::Select => include_bytes!("../assets/tabler/pointer.svg"),
            Self::Range => include_bytes!("../assets/tabler/select.svg"),
            Self::Split => include_bytes!("../assets/tabler/scissors.svg"),
            Self::Glue => include_bytes!("../assets/tabler/link.svg"),
            Self::Erase => include_bytes!("../assets/tabler/eraser.svg"),
            Self::Zoom => include_bytes!("../assets/tabler/zoom-in.svg"),
            Self::Mute => include_bytes!("../assets/tabler/volume-off.svg"),
            Self::TimeWarp => include_bytes!("../assets/tabler/clock-hour-3.svg"),
            Self::DrawCursor => include_bytes!("../assets/tabler/pencil-cursor.svg"),
            Self::Line => include_bytes!("../assets/tabler/line.svg"),
            Self::Color => include_bytes!("../assets/tabler/palette.svg"),
            Self::RootKey => include_bytes!("../assets/tabler/music.svg"),
            Self::Autoscroll => include_bytes!("../assets/tabler/arrow-bar-right.svg"),
            Self::Snap => include_bytes!("../assets/tabler/magnet.svg"),
            Self::SnapType => include_bytes!("../assets/tabler/grid-dots.svg"),
            Self::ZeroCrossing => include_bytes!("../assets/tabler/wave-sine.svg"),
            Self::TrackAudio => include_bytes!("../assets/tabler/wave-sine.svg"),
            Self::TrackMidi => include_bytes!("../assets/tabler/piano.svg"),
            Self::TrackInstrument => include_bytes!("../assets/tabler/keyboard.svg"),
            Self::Phase => include_bytes!("../assets/tabler/circle-off.svg"),
            Self::Insert => include_bytes!("../assets/tabler/plug.svg"),
            Self::Eq => include_bytes!("../assets/tabler/chart-line.svg"),
            Self::Send => include_bytes!("../assets/tabler/send.svg"),
            Self::Monitor => include_bytes!("../assets/tabler/volume.svg"),
            Self::Copy => include_bytes!("../assets/tabler/copy.svg"),
            Self::Paste => include_bytes!("../assets/tabler/clipboard.svg"),
            Self::Routing => include_bytes!("../assets/tabler/route.svg"),
            Self::Target => include_bytes!("../assets/tabler/target.svg"),
            Self::Channel => include_bytes!("../assets/tabler/adjustments-horizontal.svg"),
            Self::ChevronRight => include_bytes!("../assets/tabler/chevron-right.svg"),
            Self::ChevronUp => include_bytes!("../assets/tabler/chevron-up.svg"),
            Self::ChevronDown => include_bytes!("../assets/tabler/chevron-down.svg"),
            Self::Undo => include_bytes!("../assets/tabler/arrow-back-up.svg"),
            Self::Reveal => include_bytes!("../assets/tabler/eye.svg"),
            Self::TakeLanes => include_bytes!("../assets/original-glyphs/take-lanes.svg"),
            Self::GroupLanes => include_bytes!("../assets/original-glyphs/group-lanes.svg"),
            Self::Power => include_bytes!("../assets/tabler/power.svg"),
            Self::SavePreset => include_bytes!("../assets/tabler/device-floppy.svg"),
            Self::Close => include_bytes!("../assets/tabler/x.svg"),
            Self::TrackMusic => include_bytes!("../assets/tabler/music.svg"),
            Self::Document => include_bytes!("../assets/tabler/file-description.svg"),
            Self::GlyphFx => include_bytes!("../assets/original-glyphs/fx.svg"),
            Self::GlyphListen => include_bytes!("../assets/original-glyphs/listen.svg"),
            Self::GlyphMute => include_bytes!("../assets/original-glyphs/mute.svg"),
            Self::GlyphRecord => include_bytes!("../assets/original-glyphs/record.svg"),
            Self::GlyphSolo => include_bytes!("../assets/original-glyphs/solo.svg"),
            Self::GlyphWrite => include_bytes!("../assets/original-glyphs/write.svg"),
        }
    }

    fn transport_size(self) -> f32 {
        if matches!(
            self,
            Self::Previous
                | Self::Next
                | Self::Cycle
                | Self::Stop
                | Self::Play
                | Self::Record
                | Self::PlayTool
        ) {
            19.0
        } else {
            17.0
        }
    }
}

impl From<ToolbarIcon> for IconId {
    fn from(icon: ToolbarIcon) -> Self {
        match icon {
            ToolbarIcon::Power => Self::ProjectPower,
            ToolbarIcon::Constraint => Self::DelayCompensation,
            ToolbarIcon::Inspector => Self::Inspector,
            ToolbarIcon::InfoLine => Self::EventInfo,
            ToolbarIcon::Overview => Self::Overview,
            ToolbarIcon::Pool => Self::MediaPool,
            ToolbarIcon::Mixer => Self::Mixer,
            ToolbarIcon::Automation => Self::Automation,
            ToolbarIcon::Previous => Self::Previous,
            ToolbarIcon::Next => Self::Next,
            ToolbarIcon::Cycle => Self::Cycle,
            ToolbarIcon::Stop => Self::Stop,
            ToolbarIcon::Play => Self::Play,
            ToolbarIcon::Record => Self::Record,
            ToolbarIcon::Select => Self::Select,
            ToolbarIcon::Range => Self::Range,
            ToolbarIcon::Split => Self::Split,
            ToolbarIcon::Glue => Self::Glue,
            ToolbarIcon::Erase => Self::Erase,
            ToolbarIcon::Zoom => Self::Zoom,
            ToolbarIcon::Mute => Self::Mute,
            ToolbarIcon::TimeWarp => Self::TimeWarp,
            ToolbarIcon::Draw => Self::DrawCursor,
            ToolbarIcon::Line => Self::Line,
            ToolbarIcon::PlayTool => Self::PlayTool,
            ToolbarIcon::Color => Self::Color,
            ToolbarIcon::RootKey => Self::RootKey,
            ToolbarIcon::Autoscroll => Self::Autoscroll,
            ToolbarIcon::Snap => Self::Snap,
            ToolbarIcon::SnapType => Self::SnapType,
            ToolbarIcon::ZeroCrossing => Self::ZeroCrossing,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TextureKey {
    icon: IconId,
    pixels: u16,
    rgba: [u8; 4],
}

type TextureCache = HashMap<TextureKey, egui::TextureHandle>;

fn splash_cache(ctx: &egui::Context) -> Arc<Mutex<HashMap<(u16, u16), egui::TextureHandle>>> {
    let id = egui::Id::new("opentrack-splash-texture-cache");
    if let Some(cache) =
        ctx.data(|data| data.get_temp::<Arc<Mutex<HashMap<(u16, u16), egui::TextureHandle>>>>(id))
    {
        return cache;
    }
    let cache = Arc::new(Mutex::new(HashMap::new()));
    ctx.data_mut(|data| data.insert_temp(id, cache.clone()));
    cache
}

pub(crate) fn paint_splash(ui: &egui::Ui, rect: Rect) {
    let scale = ui.ctx().pixels_per_point();
    let width = (rect.width() * scale).round().clamp(16.0, 2048.0) as u16;
    let height = (rect.height() * scale).round().clamp(16.0, 2048.0) as u16;
    let cache = splash_cache(ui.ctx());
    let texture = if let Some(texture) = cache
        .lock()
        .expect("splash texture cache lock is not poisoned")
        .get(&(width, height))
        .cloned()
    {
        texture
    } else {
        let svg = include_bytes!("../assets/opentrack-splash.svg");
        let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default())
            .expect("bundled OpenTrack splash SVG should parse");
        let mut pixmap = resvg::tiny_skia::Pixmap::new(u32::from(width), u32::from(height))
            .expect("splash texture dimensions are bounded");
        let factor =
            (f32::from(width) / tree.size().width()).min(f32::from(height) / tree.size().height());
        let transform = resvg::tiny_skia::Transform::from_scale(factor, factor).post_translate(
            (f32::from(width) - tree.size().width() * factor) * 0.5,
            (f32::from(height) - tree.size().height() * factor) * 0.5,
        );
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        let image = egui::ColorImage::from_rgba_premultiplied(
            [usize::from(width), usize::from(height)],
            pixmap.data(),
        );
        let texture = ui.ctx().load_texture(
            format!("opentrack-splash-{width}x{height}"),
            image,
            egui::TextureOptions::LINEAR,
        );
        cache
            .lock()
            .expect("splash texture cache lock is not poisoned")
            .insert((width, height), texture.clone());
        texture
    };
    ui.painter().image(
        texture.id(),
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    let logo_scale = rect.width() / 360.0;
    let mut format = egui::text::TextFormat::simple(
        egui::FontId::new(
            40.0 * logo_scale,
            egui::FontFamily::Name("opentrack-wordmark".into()),
        ),
        Color32::WHITE,
    );
    format.extra_letter_spacing = -1.6 * logo_scale;
    let galley = ui
        .painter()
        .layout_job(egui::text::LayoutJob::single_section(
            "OpenTrack".into(),
            format,
        ));
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.top() + 149.0 * logo_scale - galley.size().y * 0.5,
        ),
        galley,
        Color32::WHITE,
    );
}

fn cache(ctx: &egui::Context) -> Arc<Mutex<TextureCache>> {
    let id = egui::Id::new("opentrack-tabler-texture-cache");
    if let Some(cache) = ctx.data(|data| data.get_temp::<Arc<Mutex<TextureCache>>>(id)) {
        return cache;
    }
    let cache = Arc::new(Mutex::new(TextureCache::default()));
    ctx.data_mut(|data| data.insert_temp(id, cache.clone()));
    cache
}

fn texture(ctx: &egui::Context, id: IconId, point_size: f32, ink: Color32) -> egui::TextureHandle {
    let pixels = (point_size * ctx.pixels_per_point())
        .round()
        .clamp(8.0, 128.0) as u16;
    let key = TextureKey {
        icon: id,
        pixels,
        rgba: ink.to_array(),
    };
    let cache = cache(ctx);
    if let Some(texture) = cache
        .lock()
        .expect("icon texture cache lock is not poisoned")
        .get(&key)
        .cloned()
    {
        return texture;
    }

    let colorized = String::from_utf8_lossy(id.svg()).replace(
        "currentColor",
        &format!("#{:02x}{:02x}{:02x}", ink.r(), ink.g(), ink.b()),
    );
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(colorized.as_bytes(), &options)
        .expect("bundled Tabler SVG should parse");
    let scale = f32::from(pixels) / tree.size().width();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(u32::from(pixels), u32::from(pixels))
        .expect("icon texture dimensions are bounded");
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let image = egui::ColorImage::from_rgba_premultiplied(
        [usize::from(pixels), usize::from(pixels)],
        pixmap.data(),
    );
    let name = format!(
        "tabler-{:?}-{}-{:02x}{:02x}{:02x}{:02x}",
        id,
        pixels,
        ink.r(),
        ink.g(),
        ink.b(),
        ink.a()
    );
    let texture = ctx.load_texture(name, image, egui::TextureOptions::LINEAR);
    cache
        .lock()
        .expect("icon texture cache lock is not poisoned")
        .insert(key, texture.clone());
    texture
}

pub(crate) fn paint_toolbar(ui: &egui::Ui, rect: Rect, icon: ToolbarIcon, ink: Color32) {
    let id = IconId::from(icon);
    let size = id
        .transport_size()
        .min(rect.width() - 4.0)
        .min(rect.height() - 4.0);
    let texture = texture(ui.ctx(), id, size, ink);
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
    ui.painter().image(
        texture.id(),
        icon_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub(crate) fn paint_app_icon(ui: &egui::Ui, rect: Rect, id: IconId, ink: Color32) {
    let size = rect.width().min(rect.height()).min(18.0).max(8.0);
    let texture = texture(ui.ctx(), id, size, ink);
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
    ui.painter().image(
        texture.id(),
        icon_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub(crate) fn paint_app_icon_on(
    ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    id: IconId,
    ink: Color32,
) {
    let size = rect.width().min(rect.height()).min(18.0).max(8.0);
    let texture = texture(ctx, id, size, ink);
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
    painter.image(
        texture.id(),
        icon_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP_ICONS: [IconId; 32] = [
        IconId::TrackAudio,
        IconId::TrackMidi,
        IconId::TrackInstrument,
        IconId::Phase,
        IconId::Insert,
        IconId::Eq,
        IconId::Send,
        IconId::Monitor,
        IconId::Copy,
        IconId::Paste,
        IconId::Routing,
        IconId::Target,
        IconId::Channel,
        IconId::ChevronRight,
        IconId::ChevronUp,
        IconId::ChevronDown,
        IconId::Undo,
        IconId::Reveal,
        IconId::TakeLanes,
        IconId::GroupLanes,
        IconId::Power,
        IconId::SavePreset,
        IconId::Close,
        IconId::TrackMusic,
        IconId::Document,
        IconId::ZeroCrossing,
        IconId::GlyphFx,
        IconId::GlyphListen,
        IconId::GlyphMute,
        IconId::GlyphRecord,
        IconId::GlyphSolo,
        IconId::GlyphWrite,
    ];

    const TOOLBAR_ICONS: [ToolbarIcon; 31] = [
        ToolbarIcon::Power,
        ToolbarIcon::Constraint,
        ToolbarIcon::Inspector,
        ToolbarIcon::InfoLine,
        ToolbarIcon::Overview,
        ToolbarIcon::Pool,
        ToolbarIcon::Mixer,
        ToolbarIcon::Automation,
        ToolbarIcon::Previous,
        ToolbarIcon::Next,
        ToolbarIcon::Cycle,
        ToolbarIcon::Stop,
        ToolbarIcon::Play,
        ToolbarIcon::Record,
        ToolbarIcon::Select,
        ToolbarIcon::Range,
        ToolbarIcon::Split,
        ToolbarIcon::Glue,
        ToolbarIcon::Erase,
        ToolbarIcon::Zoom,
        ToolbarIcon::Mute,
        ToolbarIcon::TimeWarp,
        ToolbarIcon::Draw,
        ToolbarIcon::Line,
        ToolbarIcon::PlayTool,
        ToolbarIcon::Color,
        ToolbarIcon::RootKey,
        ToolbarIcon::Autoscroll,
        ToolbarIcon::Snap,
        ToolbarIcon::SnapType,
        ToolbarIcon::ZeroCrossing,
    ];

    #[test]
    fn every_registered_asset_renders_at_common_display_scales_in_both_themes() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            for theme in [egui::Theme::Dark, egui::Theme::Light] {
                let ctx = egui::Context::default();
                crate::theme::install(&ctx);
                ctx.set_theme(if theme == egui::Theme::Dark {
                    egui::ThemePreference::Dark
                } else {
                    egui::ThemePreference::Light
                });
                ctx.set_pixels_per_point(scale);
                let mut rendered = 0;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(1000.0, 360.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        assert!((ui.ctx().pixels_per_point() - scale).abs() < 0.01);
                        paint_splash(ui, Rect::from_min_size(Pos2::ZERO, Vec2::new(360.0, 210.0)));
                        rendered += 1;
                        for icon in TOOLBAR_ICONS {
                            let id = IconId::from(icon);
                            let tint = if theme == egui::Theme::Dark {
                                Color32::from_gray(194)
                            } else {
                                Color32::from_gray(65)
                            };
                            let texture = texture(ui.ctx(), id, id.transport_size(), tint);
                            let expected = (id.transport_size() * scale).round().clamp(8.0, 128.0);
                            assert_eq!(texture.size_vec2(), Vec2::splat(expected));
                            rendered += 1;
                        }
                        for id in APP_ICONS {
                            let tint = if theme == egui::Theme::Dark {
                                Color32::from_gray(204)
                            } else {
                                Color32::from_gray(65)
                            };
                            let texture = texture(ui.ctx(), id, 17.0, tint);
                            let expected = (17.0_f32 * scale).round().clamp(8.0, 128.0);
                            assert_eq!(texture.size_vec2(), Vec2::splat(expected));
                            rendered += 1;
                        }
                    },
                );
                let wordmark_rendered = output.shapes.iter().any(|clipped| {
                    matches!(
                        &clipped.shape,
                        egui::Shape::Text(text) if text.galley.text() == "OpenTrack"
                    )
                });
                assert!(
                    wordmark_rendered,
                    "startup splash paints the OpenTrack wordmark"
                );
                output.textures_delta.clear();
                assert_eq!(rendered, TOOLBAR_ICONS.len() + APP_ICONS.len() + 1);
            }
        }
    }
}
