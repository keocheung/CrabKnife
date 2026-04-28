mod app;
mod settings;
mod tools;
mod ui;

use app::CrabKnifeApp;
use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("CrabKnife")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([920.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "CrabKnife",
        options,
        Box::new(|cc| Ok(Box::new(CrabKnifeApp::new(cc)))),
    )
}
