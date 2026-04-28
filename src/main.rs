mod app;
mod settings;
mod tools;
mod ui;

use app::RustKnifeApp;
use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("RustKnife")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([920.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustKnife",
        options,
        Box::new(|cc| Ok(Box::new(RustKnifeApp::new(cc)))),
    )
}
