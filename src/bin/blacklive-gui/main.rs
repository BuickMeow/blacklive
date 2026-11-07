mod actions;
mod app;
mod arrangement;
mod browser;
mod chrome;
mod detail;
mod files;
mod icons;
mod project;
mod session;
mod theme;

use app::App;
use eframe::egui;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1710.0, 1073.0])
            .with_min_inner_size([1280.0, 780.0])
            .with_position([0.0, 38.0])
            .with_decorations(false)
            .with_resizable(true)
            .with_title("Blacklive"),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "Blacklive",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    ) {
        eprintln!("无法启动 GUI: {err}");
    }
}
