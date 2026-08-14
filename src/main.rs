#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::CrabKnife;
use gpui::{AppContext, Application, Bounds, WindowBounds, WindowOptions, point, px, size};
use gpui_component::Root;

fn main() {
    Application::new().run(|cx| {
        gpui_component::init(cx);
        cx.spawn(async move |cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(120.0), px(80.0)),
                    size(px(1180.0), px(760.0)),
                ))),
                window_min_size: Some(size(px(920.0), px(620.0))),
                app_id: Some("moe.keo.crabknife".into()),
                ..Default::default()
            };
            cx.open_window(options, |window, cx| {
                let app = cx.new(|cx| CrabKnife::new(window, cx));
                cx.new(|cx| Root::new(app, window, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
