mod app;
mod native_keys;
mod project_file;
mod theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "OpenTrack-",
        options,
        Box::new(|creation| Ok(Box::new(app::WorkstationApp::new(creation)))),
    )
}
