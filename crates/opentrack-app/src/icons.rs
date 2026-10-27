//! Shared toolbar artwork and the native application window icon.
use eframe::egui::{self, Color32, Rect};

/// Compact OpenTrack application mark for native application windows.
pub(crate) fn opentrack_icon() -> egui::IconData {
    // Keep a 256px source so Windows can downsample cleanly for its 16/32px
    // title-bar and taskbar slots instead of enlarging a coarse 64px bitmap.
    const SIZE: u32 = 256;
    let svg = include_bytes!("../assets/opentrack-icon.svg");
    let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default())
        .expect("bundled OpenTrack logo SVG should parse");
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(SIZE, SIZE).expect("application icon dimensions are bounded");
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            SIZE as f32 / tree.size().width(),
            SIZE as f32 / tree.size().height(),
        ),
        &mut pixmap.as_mut(),
    );
    let mut rgba = pixmap.data().to_vec();
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha > 0 {
            for component in &mut pixel[..3] {
                *component = (u16::from(*component) * 255 / alpha).min(255) as u8;
            }
        }
    }
    egui::IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}

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

pub(crate) fn paint_toolbar(ui: &egui::Ui, rect: Rect, icon: ToolbarIcon, color: Color32) {
    crate::icon_system::paint_toolbar(ui, rect, icon, color);
}

#[cfg(test)]
mod tests {
    #[test]
    fn supplied_mark_and_splash_svg_parse() {
        for svg in [
            include_bytes!("../assets/opentrack-icon.svg").as_slice(),
            include_bytes!("../assets/opentrack-splash.svg").as_slice(),
        ] {
            resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default())
                .expect("supplied OpenTrack branding SVG should parse");
        }
        assert!(include_str!("../assets/opentrack-splash.svg").contains(">OpenTrack</text>"));
        let icon = super::opentrack_icon();
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        assert_eq!((icon.width, icon.height), (256, 256));
        let occupied_x = icon
            .rgba
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, pixel)| pixel[3] > 0)
            .map(|(index, _)| index % 256)
            .collect::<Vec<_>>();
        assert!(occupied_x.iter().max().unwrap() - occupied_x.iter().min().unwrap() > 160);
    }
}
