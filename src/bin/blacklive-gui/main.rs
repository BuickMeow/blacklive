mod actions;
mod app;
mod files;
mod options;
mod stats;
mod status;

use app::App;
use eframe::egui;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1020.0, 780.0])
            .with_min_inner_size([820.0, 600.0])
            .with_title("Blacklive — MIDI → Ableton .als"),
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
