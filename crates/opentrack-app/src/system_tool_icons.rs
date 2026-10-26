//! Windows-owned tool artwork. Read installed fonts/cursor resources; do not bundle assets.
use crate::icons::ToolbarIcon;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Vec2};
const FAMILY: &str = "windows-tool-icons";
pub(crate) fn install(ctx: &egui::Context, fonts: &mut egui::FontDefinitions) {
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("WINDIR") {
        for name in ["SegoeIcons.ttf", "segmdl2.ttf"] {
            if let Ok(bytes) =
                std::fs::read(std::path::PathBuf::from(&root).join("Fonts").join(name))
            {
                fonts
                    .font_data
                    .insert(FAMILY.into(), egui::FontData::from_owned(bytes).into());
                fonts
                    .families
                    .insert(egui::FontFamily::Name(FAMILY.into()), vec![FAMILY.into()]);
                ctx.data_mut(|d| d.insert_temp(egui::Id::new(FAMILY), true));
                break;
            }
        }
    }
    #[cfg(not(windows))]
    let _ = (ctx, fonts);
}
fn glyph(icon: ToolbarIcon) -> Option<char> {
    match icon {
        ToolbarIcon::Split => Some('\u{e8c6}'),
        ToolbarIcon::Draw => Some('\u{e70f}'),
        ToolbarIcon::Zoom => Some('\u{e71e}'),
        _ => None,
    }
}
fn available(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new(FAMILY)).unwrap_or(false))
}
pub(crate) fn paint(ui: &egui::Ui, rect: Rect, icon: ToolbarIcon, color: Color32) -> bool {
    if matches!(icon, ToolbarIcon::Select) {
        if let Some(texture) = arrow_texture(ui.ctx()) {
            let size = texture.size_vec2();
            let scale = (rect.width() / size.x).min(rect.height() / size.y).min(1.0);
            ui.painter().image(
                texture.id(),
                Rect::from_center_size(rect.center(), size * scale),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            return true;
        }
    }
    if let Some(glyph) = glyph(icon).filter(|_| available(ui.ctx())) {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            glyph,
            FontId::new(
                rect.height().min(rect.width()).clamp(12.0, 20.0),
                egui::FontFamily::Name(FAMILY.into()),
            ),
            color,
        );
        return true;
    }
    false
}
/// Windows supplies no stock scissors/pencil cursor: use its installed glyph beside native crosshair.
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
    paint_tool_glyph(ui, icon, hotspot);
}

/// Draws only the tool glyph and hides the native pointer. Used when Snap is
/// off, where a crosshair-plus-glyph would ambiguously suggest a snapped guide.
pub(crate) fn tool_cursor_only_at(ui: &egui::Ui, icon: ToolbarIcon, hotspot: Option<Pos2>) {
    ui.ctx().set_cursor_icon(egui::CursorIcon::None);
    paint_tool_glyph(ui, icon, hotspot);
}

fn paint_tool_glyph(ui: &egui::Ui, icon: ToolbarIcon, hotspot: Option<Pos2>) {
    if let (Some(glyph), Some(pointer)) = (glyph(icon).filter(|_| available(ui.ctx())), hotspot) {
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("windows-tool-cursor"),
        ));
        let font = FontId::new(18.0, egui::FontFamily::Name(FAMILY.into()));
        let pos = pointer;
        let align = if matches!(icon, ToolbarIcon::Split) {
            Align2::CENTER_CENTER
        } else {
            Align2::LEFT_TOP
        };
        for offset in [
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
        ] {
            painter.text(pos + offset, align, glyph, font.clone(), Color32::WHITE);
        }
        painter.text(pos, align, glyph, font, Color32::BLACK);
    }
}
fn arrow_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("windows-arrow-artwork");
    if let Some(value) = ctx.data(|d| d.get_temp::<Option<egui::TextureHandle>>(id)) {
        return value;
    }
    let value = system_arrow()
        .map(|image| ctx.load_texture("Windows system arrow", image, egui::TextureOptions::LINEAR));
    ctx.data_mut(|d| d.insert_temp(id, value.clone()));
    value
}
#[cfg(not(windows))]
fn system_arrow() -> Option<egui::ColorImage> {
    None
}
#[cfg(windows)]
fn system_arrow() -> Option<egui::ColorImage> {
    use windows_sys::Win32::{Graphics::Gdi::*, UI::WindowsAndMessaging::*};
    // SAFETY: all GDI objects are private to this call. Shared cursor isn't destroyed;
    // GetIconInfo bitmap copies and our selected DIB/DC are released on every path.
    unsafe {
        let cursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        if cursor.is_null() {
            return None;
        }
        let mut info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(cursor, &mut info) == 0 {
            return None;
        }
        let mut bitmap: BITMAP = std::mem::zeroed();
        let handle = if info.hbmColor.is_null() {
            info.hbmMask
        } else {
            info.hbmColor
        };
        let read = GetObjectW(
            handle,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bitmap as *mut _ as _,
        );
        if !info.hbmColor.is_null() {
            DeleteObject(info.hbmColor);
        }
        if !info.hbmMask.is_null() {
            DeleteObject(info.hbmMask);
        }
        if read == 0 {
            return None;
        }
        let width = bitmap.bmWidth;
        let height = if info.hbmColor.is_null() {
            bitmap.bmHeight / 2
        } else {
            bitmap.bmHeight
        };
        if !(1..=256).contains(&width) || !(1..=256).contains(&height) {
            return None;
        }
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        if dc.is_null() {
            return None;
        }
        let mut desc: BITMAPINFO = std::mem::zeroed();
        desc.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        desc.bmiHeader.biWidth = width;
        desc.bmiHeader.biHeight = -height;
        desc.bmiHeader.biPlanes = 1;
        desc.bmiHeader.biBitCount = 32;
        desc.bmiHeader.biCompression = BI_RGB;
        let mut bits = std::ptr::null_mut();
        let dib = CreateDIBSection(
            dc,
            &desc,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if dib.is_null() || bits.is_null() {
            if !dib.is_null() {
                DeleteObject(dib);
            }
            DeleteDC(dc);
            return None;
        }
        let previous = SelectObject(dc, dib);
        let len = (width * height * 4) as usize;
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, len);
        pixels.fill(0);
        let ok_black = DrawIconEx(
            dc,
            0,
            0,
            cursor,
            width,
            height,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        ) != 0;
        GdiFlush();
        let black = pixels.to_vec();
        pixels.fill(255);
        let ok_white = DrawIconEx(
            dc,
            0,
            0,
            cursor,
            width,
            height,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        ) != 0;
        GdiFlush();
        let white = pixels.to_vec();
        SelectObject(dc, previous);
        DeleteObject(dib);
        DeleteDC(dc);
        if !ok_black || !ok_white {
            return None;
        }
        let mut rgba = vec![0; len];
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for i in 0..(width * height) as usize {
            let a = 255
                - white[i * 4..i * 4 + 3]
                    .iter()
                    .zip(&black[i * 4..i * 4 + 3])
                    .map(|(w, b)| w.saturating_sub(*b))
                    .max()
                    .unwrap();
            if a > 0 {
                let x = i as i32 % width;
                let y = i as i32 / width;
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
            }
            for ch in 0..3 {
                rgba[i * 4 + ch] = if a == 0 {
                    0
                } else {
                    (black[i * 4 + 2 - ch] as u32 * 255 / a as u32).min(255) as u8
                };
            }
            rgba[i * 4 + 3] = a;
        }
        if right <= left || bottom <= top {
            return None;
        }
        let mut crop = Vec::new();
        for y in top..bottom {
            let row = (y * width + left) as usize * 4;
            crop.extend_from_slice(&rgba[row..row + (right - left) as usize * 4]);
        }
        Some(egui::ColorImage::from_rgba_unmultiplied(
            [(right - left) as usize, (bottom - top) as usize],
            &crop,
        ))
    }
}
#[cfg(all(test, windows))]
mod tests {
    use eframe::egui::{self, Pos2, Rect, Vec2};
    #[test]
    fn native_arrow_has_visible_artwork() {
        let image = super::system_arrow().unwrap();
        assert!(image.pixels.iter().any(|p| p.a() > 0));
        assert!(image.size[0] <= 256 && image.size[1] <= 256);
    }

    #[test]
    fn scissors_keeps_native_crosshair_visible_at_the_raw_pointer() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0))),
                ..Default::default()
            },
            |ui| {
                super::tool_cursor_at(
                    ui,
                    crate::icons::ToolbarIcon::Split,
                    Some(Pos2::new(45.0, 50.0)),
                )
            },
        );
        out.textures_delta.clear();
        assert_eq!(out.platform_output.cursor_icon, egui::CursorIcon::Crosshair);
    }

    #[test]
    fn snap_off_tool_cursor_hides_the_native_pointer() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0))),
                ..Default::default()
            },
            |ui| {
                super::tool_cursor_only_at(
                    ui,
                    crate::icons::ToolbarIcon::Split,
                    Some(Pos2::new(45.0, 50.0)),
                )
            },
        );
        out.textures_delta.clear();
        assert_eq!(out.platform_output.cursor_icon, egui::CursorIcon::None);
    }
}
