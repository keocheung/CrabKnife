use eframe::egui::{self, Color32, Frame, Margin, RichText, Stroke, Ui, Vec2};

use crate::app::Tool;

pub(crate) fn nav_button(ui: &mut Ui, active_tool: &mut Tool, tool: Tool, icon: &str, label: &str) {
    let selected = *active_tool == tool;
    let fill = if selected {
        ui.visuals().selection.bg_fill
    } else {
        Color32::TRANSPARENT
    };
    let text_color = if selected {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };

    let button = egui::Button::new(
        RichText::new(format!("{icon}  {label}"))
            .strong()
            .color(text_color),
    )
    .fill(fill)
    .stroke(Stroke::NONE)
    .min_size(Vec2::new(ui.available_width(), 38.0));

    if ui.add(button).clicked() {
        *active_tool = tool;
    }
}

pub(crate) fn panel(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    Frame::group(ui.style())
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(title).strong());
            ui.add_space(10.0);
            add_contents(ui);
        });
}
