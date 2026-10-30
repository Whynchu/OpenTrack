#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod crash_log;
mod icon_system;
mod icons;
mod native_keys;
mod native_windows;
mod project_file;
mod startup_splash;
mod system_tool_icons;
mod theme;
mod timeline;
mod view_controls;

fn main() -> eframe::Result {
    crash_log::install();
    startup_splash::show();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            // Wayland and X11 match a window to its application icon through
            // this id. Without it a compositor has nothing to look up and falls
            // back to a generic placeholder, even though an icon is supplied.
            .with_app_id("opentrack")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 640.0])
            .with_maximized(true)
            .with_visible(false)
            .with_icon(icons::opentrack_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "OpenTrack-",
        options,
        Box::new(move |creation| {
            native_windows::install_resource_icons(native_windows::creation_root(creation));
            let app = app::OpenTrackApp::new(creation);
            Ok(Box::new(app))
        }),
    )
}
