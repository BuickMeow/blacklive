mod actions;
mod app;
mod arrangement;
mod browser;
mod chrome;
mod detail;
mod files;
mod icons;
mod menu;
mod project;
mod session;
mod star_grid;
mod theme;

use app::App;
use eframe::egui;

fn main() {
    let snapshot = std::env::var_os("BLACKLIVE_SNAPSHOT").is_some();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1710.0, 1073.0])
            .with_min_inner_size([1280.0, 780.0])
            .with_position([0.0, 38.0])
            .with_decorations(true)
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_titlebar_buttons_shown(true)
            .with_resizable(true)
            .with_visible(!snapshot)
            .with_title(" "),
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
