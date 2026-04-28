use std::sync::Arc;

use eframe::egui::{
    self, Align, CentralPanel, Color32, Context, FontDefinitions, FontFamily, FontId, Frame,
    Layout, Margin, Panel, RichText, TextStyle, Ui, Vec2,
};

use crate::settings::Settings;
use crate::tools::regex::RegexTool;
use crate::ui::nav_button;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    RegexTester,
    Settings,
}

pub(crate) struct RustKnifeApp {
    active_tool: Tool,
    regex: RegexTool,
    settings: Settings,
    font_needs_update: bool,
}

impl RustKnifeApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let app = Self {
            active_tool: Tool::RegexTester,
            regex: RegexTool::default(),
            settings: Settings::load(),
            font_needs_update: true,
        };

        cc.egui_ctx.set_theme(egui::ThemePreference::System);
        app.apply_font_settings(&cc.egui_ctx);
        app
    }

    fn apply_font_settings(&self, ctx: &Context) {
        let mut fonts = FontDefinitions::default();
        if let Some(font_data) = self.settings.ui_font_data() {
            fonts
                .font_data
                .insert("rust-knife-ui-font".to_owned(), Arc::new(font_data));
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "rust-knife-ui-font".to_owned());
        }
        if let Some(font_data) = self.settings.editor_font_data() {
            fonts
                .font_data
                .insert("rust-knife-editor-font".to_owned(), Arc::new(font_data));
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "rust-knife-editor-font".to_owned());
        }
        ctx.set_fonts(fonts);

        let ui_size = self.settings.ui_font_size();
        let editor_size = self.settings.editor_font_size();
        let ui_family = self.settings.ui_font_family();
        let editor_family = self.settings.editor_font_family();
        ctx.all_styles_mut(|style| {
            style.text_styles = [
                (
                    TextStyle::Heading,
                    FontId::new(ui_size + 8.0, ui_family.clone()),
                ),
                (TextStyle::Body, FontId::new(ui_size, ui_family.clone())),
                (
                    TextStyle::Monospace,
                    FontId::new(editor_size, editor_family.clone()),
                ),
                (TextStyle::Button, FontId::new(ui_size, ui_family.clone())),
                (
                    TextStyle::Small,
                    FontId::new((ui_size - 2.0).max(10.0), ui_family.clone()),
                ),
            ]
            .into();
            style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        });
        ctx.style_mut_of(egui::Theme::Dark, |style| {
            style.visuals.weak_text_color = Some(Color32::from_gray(215));
            style.visuals.window_fill = Color32::from_gray(14);
            style.visuals.panel_fill = Color32::from_gray(14);
            style.visuals.extreme_bg_color = Color32::from_gray(2);
            style.visuals.text_edit_bg_color = Some(Color32::from_gray(4));
        });
    }

    fn show_sidebar(&mut self, ui: &mut Ui) {
        ui.add_space(8.0);
        ui.heading("RustKnife");
        ui.label(RichText::new("Developer tools").color(ui.visuals().weak_text_color()));
        ui.add_space(18.0);

        nav_button(
            ui,
            &mut self.active_tool,
            Tool::RegexTester,
            ".*",
            "Regex Tester",
        );
        nav_button(ui, &mut self.active_tool, Tool::Settings, "Aa", "Settings");

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.label(RichText::new("v0.1.0").color(ui.visuals().weak_text_color()));
        });
    }

    fn show_header(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading(match self.active_tool {
                Tool::RegexTester => "Regex Tester",
                Tool::Settings => "Settings",
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("English").color(ui.visuals().weak_text_color()));
            });
        });
    }
}

impl eframe::App for RustKnifeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.font_needs_update {
            self.apply_font_settings(&ctx);
            self.font_needs_update = false;
        }

        let style = ctx.global_style();

        Panel::left("tools")
            .resizable(false)
            .exact_size(220.0)
            .frame(Frame::side_top_panel(&style).inner_margin(Margin::same(18)))
            .show_inside(ui, |ui| self.show_sidebar(ui));

        Panel::top("header")
            .resizable(false)
            .exact_size(64.0)
            .frame(Frame::side_top_panel(&style).inner_margin(Margin::symmetric(22, 12)))
            .show_inside(ui, |ui| self.show_header(ui));

        CentralPanel::default()
            .frame(Frame::central_panel(&style).inner_margin(Margin::same(22)))
            .show_inside(ui, |ui| match self.active_tool {
                Tool::RegexTester => self.regex.ui(ui),
                Tool::Settings => {
                    if self.settings.ui(ui) {
                        self.font_needs_update = true;
                        self.settings.save();
                    }
                }
            });
    }
}
