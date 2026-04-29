#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod settings;
mod tools;
mod ui;

use app::CrabKnifeApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::egui::ViewportBuilder;

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

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;

    let document = web_sys::window()
        .expect("no window")
        .document()
        .expect("no document");
    let canvas = document
        .get_element_by_id("crab_knife_canvas")
        .expect("no canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("not a canvas element");

    let web_options = eframe::WebOptions::default();
    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(CrabKnifeApp::new(cc)))),
        )
        .await?;
    Ok(())
}
