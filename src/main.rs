use eframe::egui::{
    self, Align, CentralPanel, Color32, Context, FontData, FontDefinitions, FontFamily, FontId,
    Frame, Layout, Margin, Panel, RichText, ScrollArea, Stroke, TextBuffer, TextEdit, TextStyle,
    Ui, Vec2, ViewportBuilder,
    text::{LayoutJob, TextFormat},
};
use fontdb::{Database, Style, Weight};
use regex::{Regex, RegexBuilder};
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use toml_edit::{DocumentMut, value};

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    RegexTester,
    Settings,
}

struct RustKnifeApp {
    active_tool: Tool,
    regex: RegexTool,
    settings: Settings,
    font_needs_update: bool,
}

impl RustKnifeApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
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

        let ui_size = self.settings.ui_font_size;
        let editor_size = self.settings.editor_font_size;
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

fn nav_button(ui: &mut Ui, active_tool: &mut Tool, tool: Tool, icon: &str, label: &str) {
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

struct RegexTool {
    pattern: String,
    test_text: String,
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_new_line: bool,
}

impl Default for RegexTool {
    fn default() -> Self {
        Self {
            pattern: r"\b\w+@\w+\.\w+\b".to_owned(),
            test_text: "Send logs to dev@example.com and security@example.org.\nInvalid: dev@local"
                .to_owned(),
            case_insensitive: false,
            multi_line: true,
            dot_matches_new_line: false,
        }
    }
}

impl RegexTool {
    fn build_regex(&self) -> Result<Regex, regex::Error> {
        RegexBuilder::new(&self.pattern)
            .case_insensitive(self.case_insensitive)
            .multi_line(self.multi_line)
            .dot_matches_new_line(self.dot_matches_new_line)
            .build()
    }

    fn ui(&mut self, ui: &mut Ui) {
        let result = self.build_regex();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.55).max(420.0));
                panel(ui, "Pattern", |ui| {
                    ui.add(
                        TextEdit::singleline(&mut self.pattern)
                            .font(TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .hint_text("Enter a Rust regex pattern"),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.case_insensitive, "Case insensitive");
                        ui.checkbox(&mut self.multi_line, "Multi-line");
                        ui.checkbox(&mut self.dot_matches_new_line, "Dot matches newline");
                    });

                    if let Err(error) = &result {
                        ui.add_space(8.0);
                        ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                    }
                });

                ui.add_space(14.0);
                panel(ui, "Test Text", |ui| {
                    let highlight_regex = result.as_ref().ok().cloned();
                    let mut layouter = move |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
                        let font_id = TextStyle::Monospace.resolve(ui.style());
                        let visuals = ui.visuals();
                        let job = highlighted_text_job(
                            text.as_str(),
                            highlight_regex.as_ref(),
                            font_id,
                            visuals.text_color(),
                            visuals.dark_mode,
                            wrap_width,
                        );
                        ui.fonts_mut(|fonts| fonts.layout_job(job))
                    };

                    ui.add(
                        TextEdit::multiline(&mut self.test_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(22)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter)
                            .hint_text("Paste text to test against the expression"),
                    );
                });
            });

            ui.add_space(14.0);
            ui.vertical(|ui| {
                panel(ui, "Matches", |ui| {
                    self.match_list(ui, result.as_ref().ok())
                });
            });
        });
    }

    fn match_list(&self, ui: &mut Ui, regex: Option<&Regex>) {
        let Some(regex) = regex else {
            ui.label(
                RichText::new("Fix the pattern to see matches.")
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };

        let captures: Vec<_> = regex.captures_iter(&self.test_text).collect();
        ui.horizontal(|ui| {
            ui.label(RichText::new(captures.len().to_string()).heading());
            ui.label(if captures.len() == 1 {
                "match found"
            } else {
                "matches found"
            });
        });
        ui.separator();

        if captures.is_empty() {
            ui.label(RichText::new("No matches.").color(ui.visuals().weak_text_color()));
            return;
        }

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, captures) in captures.iter().enumerate() {
                    let Some(mat) = captures.get(0) else {
                        continue;
                    };

                    Frame::group(ui.style())
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("#{}", index + 1)).strong());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{}..{}", mat.start(), mat.end()))
                                            .monospace()
                                            .color(ui.visuals().weak_text_color()),
                                    );
                                });
                            });
                            ui.add_space(4.0);
                            ui.label(RichText::new(mat.as_str()).monospace());

                            let groups: Vec<_> = captures
                                .iter()
                                .enumerate()
                                .skip(1)
                                .filter_map(|(group_index, group)| {
                                    group.map(|group| (group_index, group))
                                })
                                .collect();

                            if !groups.is_empty() {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        groups.len(),
                                        if groups.len() == 1 { "group" } else { "groups" }
                                    ))
                                    .color(ui.visuals().weak_text_color()),
                                );

                                for (group_index, group) in groups {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("${group_index}"))
                                                .monospace()
                                                .color(group_color(group_index)),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{}..{}",
                                                group.start(),
                                                group.end()
                                            ))
                                            .monospace()
                                            .color(ui.visuals().weak_text_color()),
                                        );
                                        ui.label(RichText::new(group.as_str()).monospace());
                                    });
                                }
                            }
                        });
                    ui.add_space(8.0);
                }
            });
    }
}

struct HighlightRange {
    range: Range<usize>,
    background: Color32,
    priority: usize,
}

fn highlighted_text_job(
    text: &str,
    regex: Option<&Regex>,
    font_id: FontId,
    text_color: Color32,
    dark_mode: bool,
    wrap_width: f32,
) -> LayoutJob {
    let mut ranges = Vec::new();

    if let Some(regex) = regex {
        for captures in regex.captures_iter(text) {
            if let Some(mat) = captures.get(0) {
                push_highlight(
                    &mut ranges,
                    mat.start()..mat.end(),
                    match_background(dark_mode),
                    1,
                );
            }

            for (group_index, group) in captures.iter().enumerate().skip(1) {
                if let Some(group) = group {
                    push_highlight(
                        &mut ranges,
                        group.start()..group.end(),
                        group_background(group_index, dark_mode),
                        10 + group_index,
                    );
                }
            }
        }
    }

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    if ranges.is_empty() {
        job.append(
            text,
            0.0,
            TextFormat {
                font_id,
                color: text_color,
                ..Default::default()
            },
        );
        return job;
    }

    let mut boundaries = Vec::with_capacity(ranges.len() * 2 + 2);
    boundaries.push(0);
    boundaries.push(text.len());
    for range in &ranges {
        boundaries.push(range.range.start);
        boundaries.push(range.range.end);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    for window in boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        if start == end {
            continue;
        }

        let background = ranges
            .iter()
            .filter(|range| range.range.start <= start && end <= range.range.end)
            .max_by_key(|range| range.priority)
            .map_or(Color32::TRANSPARENT, |range| range.background);

        job.append(
            &text[start..end],
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: text_color,
                background,
                ..Default::default()
            },
        );
    }

    job
}

fn push_highlight(
    ranges: &mut Vec<HighlightRange>,
    range: Range<usize>,
    background: Color32,
    priority: usize,
) {
    if range.start < range.end {
        ranges.push(HighlightRange {
            range,
            background,
            priority,
        });
    }
}

fn group_color(group_index: usize) -> Color32 {
    const COLORS: [Color32; 5] = [
        Color32::from_rgb(22, 112, 196),
        Color32::from_rgb(142, 78, 198),
        Color32::from_rgb(31, 132, 90),
        Color32::from_rgb(192, 86, 33),
        Color32::from_rgb(188, 57, 83),
    ];
    COLORS[(group_index - 1) % COLORS.len()]
}

fn match_background(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(54, 42, 12)
    } else {
        Color32::from_rgb(255, 242, 178)
    }
}

fn group_background(group_index: usize, dark_mode: bool) -> Color32 {
    const LIGHT_COLORS: [Color32; 5] = [
        Color32::from_rgb(205, 231, 255),
        Color32::from_rgb(231, 218, 255),
        Color32::from_rgb(207, 239, 222),
        Color32::from_rgb(255, 225, 201),
        Color32::from_rgb(255, 216, 225),
    ];
    const DARK_COLORS: [Color32; 5] = [
        Color32::from_rgb(14, 40, 62),
        Color32::from_rgb(42, 28, 66),
        Color32::from_rgb(16, 48, 30),
        Color32::from_rgb(56, 30, 12),
        Color32::from_rgb(56, 22, 34),
    ];
    let colors = if dark_mode { DARK_COLORS } else { LIGHT_COLORS };
    colors[(group_index - 1) % colors.len()]
}

#[derive(Clone, PartialEq, Eq)]
enum FontChoice {
    Proportional,
    Monospace,
    System(usize),
    Custom,
}

impl FontChoice {
    fn label(&self, system_fonts: &[SystemFont]) -> String {
        match self {
            Self::Proportional => "Proportional".to_owned(),
            Self::Monospace => "Monospace".to_owned(),
            Self::System(index) => system_fonts
                .get(*index)
                .map(|font| font.name.clone())
                .unwrap_or_else(|| "System font".to_owned()),
            Self::Custom => "Custom font file".to_owned(),
        }
    }

    fn built_in_family(&self) -> FontFamily {
        match self {
            Self::Proportional => FontFamily::Proportional,
            Self::Monospace => FontFamily::Monospace,
            Self::System(_) | Self::Custom => FontFamily::Proportional,
        }
    }
}

struct SystemFont {
    name: String,
    id: fontdb::ID,
    monospaced: bool,
}

struct Settings {
    ui_font: FontChoice,
    editor_font: FontChoice,
    system_fonts: Vec<SystemFont>,
    font_database: Database,
    custom_font_path: String,
    custom_font_data: Option<Vec<u8>>,
    custom_font_error: Option<String>,
    ui_font_size: f32,
    editor_font_size: f32,
}

impl Default for Settings {
    fn default() -> Self {
        let (font_database, system_fonts) = load_system_fonts();
        let editor_font = system_fonts
            .iter()
            .position(|font| font.monospaced)
            .map(FontChoice::System)
            .unwrap_or(FontChoice::Monospace);

        Self {
            ui_font: FontChoice::Proportional,
            editor_font,
            system_fonts,
            font_database,
            custom_font_path: String::new(),
            custom_font_data: None,
            custom_font_error: None,
            ui_font_size: 16.0,
            editor_font_size: 16.0,
        }
    }
}

impl Settings {
    fn load() -> Self {
        let mut settings = Self::default();
        let Some(path) = config_path() else {
            return settings;
        };
        let Ok(contents) = std::fs::read_to_string(path) else {
            return settings;
        };
        let Ok(config) = contents.parse::<DocumentMut>() else {
            return settings;
        };

        if let Some(font) =
            load_font_choice(&config, "ui_font", "ui_system_font", &settings.system_fonts)
        {
            settings.ui_font = font;
        }
        if let Some(font) = load_font_choice(
            &config,
            "editor_font",
            "editor_system_font",
            &settings.system_fonts,
        ) {
            settings.editor_font = font;
        }
        if let Some(size) = load_font_size(&config, "ui_font_size") {
            settings.ui_font_size = size;
        }
        if let Some(size) = load_font_size(&config, "editor_font_size") {
            settings.editor_font_size = size;
        }
        if let Some(path) = config
            .get("custom_font_path")
            .and_then(|value| value.as_str())
        {
            settings.custom_font_path = path.to_owned();
            settings.load_custom_font();
        }

        settings
    }

    fn save(&self) {
        let Some(path) = config_path() else {
            return;
        };
        let mut config = DocumentMut::new();
        save_font_choice(
            &mut config,
            "ui_font",
            "ui_system_font",
            &self.ui_font,
            &self.system_fonts,
        );
        save_font_choice(
            &mut config,
            "editor_font",
            "editor_system_font",
            &self.editor_font,
            &self.system_fonts,
        );
        config["custom_font_path"] = value(self.custom_font_path.clone());
        config["ui_font_size"] = value(self.ui_font_size as f64);
        config["editor_font_size"] = value(self.editor_font_size as f64);

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, config.to_string());
    }

    fn ui_font_family(&self) -> FontFamily {
        match self.ui_font {
            FontChoice::System(_) | FontChoice::Custom => FontFamily::Proportional,
            _ => self.ui_font.built_in_family(),
        }
    }

    fn editor_font_family(&self) -> FontFamily {
        match self.editor_font {
            FontChoice::System(_) | FontChoice::Custom => FontFamily::Monospace,
            _ => self.editor_font.built_in_family(),
        }
    }

    fn font_data_for(&self, choice: &FontChoice) -> Option<FontData> {
        match choice {
            FontChoice::System(index) => {
                let font = self.system_fonts.get(*index)?;
                let (bytes, face_index) = self
                    .font_database
                    .with_face_data(font.id, |data, face_index| (data.to_vec(), face_index))?;
                let mut font_data = FontData::from_owned(bytes);
                font_data.index = face_index;
                Some(font_data)
            }
            FontChoice::Custom => self
                .custom_font_data
                .as_ref()
                .map(|bytes| FontData::from_owned(bytes.clone())),
            FontChoice::Proportional | FontChoice::Monospace => None,
        }
    }

    fn ui_font_data(&self) -> Option<FontData> {
        self.font_data_for(&self.ui_font)
    }

    fn editor_font_data(&self) -> Option<FontData> {
        self.font_data_for(&self.editor_font)
    }

    fn load_custom_font(&mut self) {
        if self.custom_font_path.trim().is_empty() {
            self.custom_font_data = None;
            self.custom_font_error = None;
            return;
        }

        match std::fs::read(self.custom_font_path.trim()) {
            Ok(bytes) => {
                self.custom_font_data = Some(bytes);
                self.custom_font_error = None;
            }
            Err(error) => {
                self.custom_font_data = None;
                self.custom_font_error = Some(error.to_string());
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        panel(ui, "Appearance", |ui| {
            changed |= font_picker(
                ui,
                "UI font",
                "ui-font-family",
                &mut self.ui_font,
                &self.system_fonts,
            );

            ui.add_space(14.0);
            ui.label("UI font size");
            changed |= ui
                .add(egui::Slider::new(&mut self.ui_font_size, 12.0..=24.0).suffix(" px"))
                .changed();

            ui.add_space(18.0);
            changed |= font_picker(
                ui,
                "Editor font",
                "editor-font-family",
                &mut self.editor_font,
                &self.system_fonts,
            );

            ui.add_space(14.0);
            ui.label("Editor font size");
            changed |= ui
                .add(egui::Slider::new(&mut self.editor_font_size, 12.0..=24.0).suffix(" px"))
                .changed();

            if self.ui_font == FontChoice::Custom || self.editor_font == FontChoice::Custom {
                ui.add_space(18.0);
                changed |= custom_font_loader(
                    ui,
                    &mut self.custom_font_path,
                    &mut self.custom_font_data,
                    &mut self.custom_font_error,
                );
            }

            ui.add_space(12.0);
            ui.label(
                RichText::new(
                    "UI font is applied to navigation and labels. Editor font is applied to regex and text editors.",
                )
                .color(ui.visuals().weak_text_color()),
            );
        });

        changed
    }
}

fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("RustKnife").join("config.toml"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".config").join("RustKnife").join("config.toml"))
    }
}

fn load_font_choice(
    config: &DocumentMut,
    choice_key: &str,
    system_key: &str,
    system_fonts: &[SystemFont],
) -> Option<FontChoice> {
    match config.get(choice_key)?.as_str()? {
        "proportional" => Some(FontChoice::Proportional),
        "monospace" => Some(FontChoice::Monospace),
        "custom" => Some(FontChoice::Custom),
        "system" => {
            let name = config.get(system_key)?.as_str()?;
            system_fonts
                .iter()
                .position(|font| font.name == name)
                .map(FontChoice::System)
        }
        _ => None,
    }
}

fn save_font_choice(
    config: &mut DocumentMut,
    choice_key: &str,
    system_key: &str,
    choice: &FontChoice,
    system_fonts: &[SystemFont],
) {
    match choice {
        FontChoice::Proportional => config[choice_key] = value("proportional"),
        FontChoice::Monospace => config[choice_key] = value("monospace"),
        FontChoice::Custom => config[choice_key] = value("custom"),
        FontChoice::System(index) => {
            config[choice_key] = value("system");
            if let Some(font) = system_fonts.get(*index) {
                config[system_key] = value(font.name.clone());
            }
        }
    }
}

fn load_font_size(config: &DocumentMut, key: &str) -> Option<f32> {
    let value = config.get(key)?;
    let size = value
        .as_float()
        .or_else(|| value.as_integer().map(|value| value as f64))? as f32;
    (12.0..=24.0).contains(&size).then_some(size)
}

fn load_system_fonts() -> (Database, Vec<SystemFont>) {
    let mut database = Database::new();
    database.load_system_fonts();

    let mut best_by_name = BTreeMap::new();
    for face in database.faces() {
        let Some((name, _language)) = face.families.first() else {
            continue;
        };
        let score = font_match_score(face);
        best_by_name
            .entry(name.clone())
            .and_modify(|(best_score, best_id, monospaced)| {
                if score < *best_score {
                    *best_score = score;
                    *best_id = face.id;
                    *monospaced = face.monospaced;
                }
            })
            .or_insert((score, face.id, face.monospaced));
    }

    let fonts = best_by_name
        .into_iter()
        .map(|(name, (_score, id, monospaced))| SystemFont {
            name,
            id,
            monospaced,
        })
        .collect();

    (database, fonts)
}

fn font_match_score(face: &fontdb::FaceInfo) -> u16 {
    let style_penalty = if face.style == Style::Normal { 0 } else { 1000 };
    let weight_penalty = face.weight.0.abs_diff(Weight::NORMAL.0);
    style_penalty + weight_penalty
}

fn font_picker(
    ui: &mut Ui,
    label: &str,
    id_salt: &str,
    selected_font: &mut FontChoice,
    system_fonts: &[SystemFont],
) -> bool {
    let mut changed = false;

    ui.label(label);
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected_font.label(system_fonts))
        .width(ui.available_width().min(360.0))
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(selected_font, FontChoice::Proportional, "Proportional")
                .changed();
            changed |= ui
                .selectable_value(selected_font, FontChoice::Monospace, "Monospace")
                .changed();
            changed |= ui
                .selectable_value(selected_font, FontChoice::Custom, "Custom font file")
                .changed();

            ui.separator();
            ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                for (index, font) in system_fonts.iter().enumerate() {
                    let label = if font.monospaced {
                        format!("{}  [mono]", font.name)
                    } else {
                        font.name.clone()
                    };
                    changed |= ui
                        .selectable_value(selected_font, FontChoice::System(index), label)
                        .changed();
                }
            });
        });

    changed
}

fn custom_font_loader(
    ui: &mut Ui,
    custom_font_path: &mut String,
    custom_font_data: &mut Option<Vec<u8>>,
    custom_font_error: &mut Option<String>,
) -> bool {
    let mut changed = false;

    ui.label("Custom font file path");
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(custom_font_path).hint_text("/Library/Fonts/Arial.ttf"));
        if ui.button("Load").clicked() {
            match std::fs::read(custom_font_path.trim()) {
                Ok(bytes) => {
                    *custom_font_data = Some(bytes);
                    *custom_font_error = None;
                    changed = true;
                }
                Err(error) => {
                    *custom_font_data = None;
                    *custom_font_error = Some(error.to_string());
                    changed = true;
                }
            }
        }
    });

    if let Some(error) = custom_font_error {
        ui.colored_label(Color32::from_rgb(176, 42, 55), error);
    } else if custom_font_data.is_some() {
        ui.colored_label(Color32::from_rgb(30, 120, 74), "Custom font loaded.");
    }

    changed
}

fn panel(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    Frame::group(ui.style())
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(title).strong());
            ui.add_space(10.0);
            add_contents(ui);
        });
}
