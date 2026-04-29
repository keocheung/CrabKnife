#[cfg(not(target_arch = "wasm32"))]
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use eframe::egui::{
    self, Color32, FontData, FontFamily, FontTweak, RichText, ScrollArea, TextEdit, Ui, Vec2,
    epaint::text::VariationCoords,
};
#[cfg(not(target_arch = "wasm32"))]
use fontdb::{Database, Family, Query, Stretch, Style, Weight};
use toml_edit::{Array, DocumentMut, value};

use crate::ui::panel;

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
    #[cfg(not(target_arch = "wasm32"))]
    id: fontdb::ID,
    monospaced: bool,
}

#[derive(Clone, PartialEq)]
struct FontVariationCoord {
    tag: String,
    value: f32,
}

impl FontVariationCoord {
    fn new(tag: &str, value: f32) -> Self {
        Self {
            tag: tag.to_owned(),
            value,
        }
    }

    fn is_valid_tag(&self) -> bool {
        self.tag.len() == 4 && self.tag.bytes().all(|byte| byte.is_ascii_alphanumeric())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FontWeightChoice {
    Thin,
    ExtraLight,
    Light,
    Regular,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeightChoice {
    const ALL: [Self; 9] = [
        Self::Thin,
        Self::ExtraLight,
        Self::Light,
        Self::Regular,
        Self::Medium,
        Self::Semibold,
        Self::Bold,
        Self::ExtraBold,
        Self::Black,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Thin => "Thin",
            Self::ExtraLight => "Extra light",
            Self::Light => "Light",
            Self::Regular => "Regular",
            Self::Medium => "Medium",
            Self::Semibold => "Semibold",
            Self::Bold => "Bold",
            Self::ExtraBold => "Extra bold",
            Self::Black => "Black",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Thin => "thin",
            Self::ExtraLight => "extra_light",
            Self::Light => "light",
            Self::Regular => "regular",
            Self::Medium => "medium",
            Self::Semibold => "semibold",
            Self::Bold => "bold",
            Self::ExtraBold => "extra_bold",
            Self::Black => "black",
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn weight(self) -> Weight {
        match self {
            Self::Thin => Weight::THIN,
            Self::ExtraLight => Weight::EXTRA_LIGHT,
            Self::Light => Weight::LIGHT,
            Self::Regular => Weight::NORMAL,
            Self::Medium => Weight::MEDIUM,
            Self::Semibold => Weight::SEMIBOLD,
            Self::Bold => Weight::BOLD,
            Self::ExtraBold => Weight::EXTRA_BOLD,
            Self::Black => Weight::BLACK,
        }
    }
}

pub(crate) struct Settings {
    ui_font: FontChoice,
    editor_font: FontChoice,
    system_fonts: Vec<SystemFont>,
    #[cfg(not(target_arch = "wasm32"))]
    font_database: Database,
    custom_font_path: String,
    custom_font_data: Option<Vec<u8>>,
    custom_font_error: Option<String>,
    ui_font_size: f32,
    editor_font_size: f32,
    ui_font_weight: FontWeightChoice,
    editor_font_weight: FontWeightChoice,
    ui_font_variations: Vec<FontVariationCoord>,
    editor_font_variations: Vec<FontVariationCoord>,
}

impl Default for Settings {
    fn default() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (font_database, system_fonts) = load_system_fonts();
        #[cfg(target_arch = "wasm32")]
        let system_fonts: Vec<SystemFont> = Vec::new();

        let editor_font = system_fonts
            .iter()
            .position(|font| font.monospaced)
            .map(FontChoice::System)
            .unwrap_or(FontChoice::Monospace);

        Self {
            ui_font: FontChoice::Proportional,
            editor_font,
            system_fonts,
            #[cfg(not(target_arch = "wasm32"))]
            font_database,
            custom_font_path: String::new(),
            custom_font_data: None,
            custom_font_error: None,
            ui_font_size: 16.0,
            editor_font_size: 16.0,
            ui_font_weight: FontWeightChoice::Regular,
            editor_font_weight: FontWeightChoice::Regular,
            ui_font_variations: Vec::new(),
            editor_font_variations: Vec::new(),
        }
    }
}

impl Settings {
    pub(crate) fn load() -> Self {
        let mut settings = Self::default();
        let Some(contents) = Self::load_config_string() else {
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
        if let Some(weight) = load_font_weight(&config, "ui_font_weight") {
            settings.ui_font_weight = weight;
        }
        if let Some(weight) = load_font_weight(&config, "editor_font_weight") {
            settings.editor_font_weight = weight;
        }
        settings.ui_font_variations = load_font_variations(&config, "ui_font_variations");
        settings.editor_font_variations = load_font_variations(&config, "editor_font_variations");
        if let Some(path) = config
            .get("custom_font_path")
            .and_then(|value| value.as_str())
        {
            settings.custom_font_path = path.to_owned();
            settings.load_custom_font();
        }

        settings
    }

    pub(crate) fn save(&self) {
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
        config["ui_font_weight"] = value(self.ui_font_weight.key());
        config["editor_font_weight"] = value(self.editor_font_weight.key());
        config["ui_font_variations"] = value(save_font_variations(&self.ui_font_variations));
        config["editor_font_variations"] =
            value(save_font_variations(&self.editor_font_variations));

        Self::save_config_string(&config.to_string());
    }

    pub(crate) fn ui_font_size(&self) -> f32 {
        self.ui_font_size
    }

    pub(crate) fn editor_font_size(&self) -> f32 {
        self.editor_font_size
    }

    pub(crate) fn ui_font_family(&self) -> FontFamily {
        match self.ui_font {
            FontChoice::System(_) | FontChoice::Custom => FontFamily::Proportional,
            _ => self.ui_font.built_in_family(),
        }
    }

    pub(crate) fn editor_font_family(&self) -> FontFamily {
        match self.editor_font {
            FontChoice::System(_) | FontChoice::Custom => FontFamily::Monospace,
            _ => self.editor_font.built_in_family(),
        }
    }

    fn font_data_for(
        &self,
        choice: &FontChoice,
        variations: &[FontVariationCoord],
    ) -> Option<FontData> {
        match choice {
            #[cfg(not(target_arch = "wasm32"))]
            FontChoice::System(index) => {
                let font = self.system_fonts.get(*index)?;
                self.system_font_data(&font.name, Weight::NORMAL, variations)
            }
            #[cfg(target_arch = "wasm32")]
            FontChoice::System(_) => None,
            FontChoice::Custom => self
                .custom_font_data
                .as_ref()
                .map(|bytes| with_font_variations(FontData::from_owned(bytes.clone()), variations)),
            FontChoice::Proportional | FontChoice::Monospace => None,
        }
    }

    fn weighted_font_data_for(
        &self,
        choice: &FontChoice,
        weight: FontWeightChoice,
        variations: &[FontVariationCoord],
    ) -> Option<FontData> {
        let _ = weight;
        match choice {
            #[cfg(not(target_arch = "wasm32"))]
            FontChoice::System(index) => {
                let font = self.system_fonts.get(*index)?;
                self.system_font_data(&font.name, weight.weight(), variations)
            }
            #[cfg(target_arch = "wasm32")]
            FontChoice::System(_) => None,
            _ => self.font_data_for(choice, variations),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn system_font_data(
        &self,
        family_name: &str,
        weight: Weight,
        variations: &[FontVariationCoord],
    ) -> Option<FontData> {
        let families = [Family::Name(family_name)];
        let query = Query {
            families: &families,
            weight,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };
        let id = self.font_database.query(&query).or_else(|| {
            self.system_fonts
                .iter()
                .find(|font| font.name == family_name)
                .map(|font| font.id)
        })?;
        let (bytes, face_index) = self
            .font_database
            .with_face_data(id, |data, face_index| (data.to_vec(), face_index))?;
        let mut font_data = FontData::from_owned(bytes);
        font_data.index = face_index;
        Some(with_font_variations(font_data, variations))
    }

    pub(crate) fn ui_font_data(&self) -> Option<FontData> {
        self.weighted_font_data_for(&self.ui_font, self.ui_font_weight, &self.ui_font_variations)
    }

    pub(crate) fn editor_font_data(&self) -> Option<FontData> {
        self.weighted_font_data_for(
            &self.editor_font,
            self.editor_font_weight,
            &self.editor_font_variations,
        )
    }

    fn load_custom_font(&mut self) {
        if self.custom_font_path.trim().is_empty() {
            self.custom_font_data = None;
            self.custom_font_error = None;
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
    fn load_config_string() -> Option<String> {
        let path = config_path().or_else(legacy_config_path)?;
        std::fs::read_to_string(path).ok()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_config_string(contents: &str) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, contents);
    }

    #[cfg(target_arch = "wasm32")]
    fn load_config_string() -> Option<String> {
        let storage = web_sys::window()?.local_storage().ok()??;
        storage.get_item("crabknife_config").ok()?
    }

    #[cfg(target_arch = "wasm32")]
    fn save_config_string(contents: &str) {
        let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
        else {
            return;
        };
        let _ = storage.set_item("crabknife_config", contents);
    }

    pub(crate) fn ui(&mut self, ui: &mut Ui) -> bool {
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

            ui.add_space(14.0);
            changed |= font_weight_picker(
                ui,
                "UI font weight",
                "ui-font-weight",
                &mut self.ui_font_weight,
            );

            ui.add_space(14.0);
            changed |= font_variation_editor(
                ui,
                "UI font variations",
                "ui-font-variations",
                &mut self.ui_font_variations,
            );

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

            ui.add_space(14.0);
            changed |= font_weight_picker(
                ui,
                "Editor font weight",
                "editor-font-weight",
                &mut self.editor_font_weight,
            );

            ui.add_space(14.0);
            changed |= font_variation_editor(
                ui,
                "Editor font variations",
                "editor-font-variations",
                &mut self.editor_font_variations,
            );

            #[cfg(not(target_arch = "wasm32"))]
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
                    "UI font is applied to navigation and labels. Editor font is applied to regex and text editors. Weight uses the closest available system font face. Variations apply to system and custom font files that support the selected axes.",
                )
                .color(ui.visuals().weak_text_color()),
            );
        });

        changed
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("CrabKnife").join("config.toml"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".config").join("CrabKnife").join("config.toml"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn legacy_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("RustKnife").join("config.toml"))
            .filter(|path| path.exists())
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".config").join("RustKnife").join("config.toml"))
            .filter(|path| path.exists())
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

fn load_font_weight(config: &DocumentMut, key: &str) -> Option<FontWeightChoice> {
    match config.get(key)?.as_str()? {
        "thin" => Some(FontWeightChoice::Thin),
        "extra_light" => Some(FontWeightChoice::ExtraLight),
        "light" => Some(FontWeightChoice::Light),
        "regular" => Some(FontWeightChoice::Regular),
        "medium" => Some(FontWeightChoice::Medium),
        "semibold" => Some(FontWeightChoice::Semibold),
        "bold" => Some(FontWeightChoice::Bold),
        "extra_bold" => Some(FontWeightChoice::ExtraBold),
        "black" => Some(FontWeightChoice::Black),
        _ => None,
    }
}

fn load_font_variations(config: &DocumentMut, key: &str) -> Vec<FontVariationCoord> {
    let Some(value) = config.get(key) else {
        return Vec::new();
    };
    let Some(array) = value.as_array() else {
        return Vec::new();
    };

    array
        .iter()
        .filter_map(|value| {
            let entry = value.as_str()?;
            let (tag, raw_value) = entry.split_once('=')?;
            let coord = FontVariationCoord::new(tag.trim(), raw_value.trim().parse().ok()?);
            coord.is_valid_tag().then_some(coord)
        })
        .collect()
}

fn save_font_variations(variations: &[FontVariationCoord]) -> Array {
    let mut array = Array::new();
    variations
        .iter()
        .filter(|coord| coord.is_valid_tag() && coord.value.is_finite())
        .for_each(|coord| array.push(format!("{}={}", coord.tag, coord.value)));
    array
}

fn with_font_variations(font_data: FontData, variations: &[FontVariationCoord]) -> FontData {
    let variation_values = variations
        .iter()
        .filter(|coord| coord.is_valid_tag() && coord.value.is_finite())
        .map(|coord| (coord.tag.as_str(), coord.value));

    font_data.tweak(FontTweak {
        coords: VariationCoords::new(variation_values),
        ..Default::default()
    })
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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

fn font_weight_picker(
    ui: &mut Ui,
    label: &str,
    id_salt: &str,
    selected_weight: &mut FontWeightChoice,
) -> bool {
    let mut changed = false;

    ui.label(label);
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected_weight.label())
        .width(ui.available_width().min(220.0))
        .show_ui(ui, |ui| {
            for weight in FontWeightChoice::ALL {
                changed |= ui
                    .selectable_value(selected_weight, weight, weight.label())
                    .changed();
            }
        });

    changed
}

fn font_variation_editor(
    ui: &mut Ui,
    label: &str,
    id_salt: &str,
    variations: &mut Vec<FontVariationCoord>,
) -> bool {
    let original = variations.clone();
    let mut remove_index = None;

    ui.label(label);
    egui::Grid::new(id_salt)
        .num_columns(3)
        .spacing(Vec2::new(8.0, 6.0))
        .show(ui, |ui| {
            for (index, coord) in variations.iter_mut().enumerate() {
                ui.add(
                    TextEdit::singleline(&mut coord.tag)
                        .desired_width(48.0)
                        .hint_text("wght"),
                );
                ui.add(
                    egui::DragValue::new(&mut coord.value)
                        .speed(1.0)
                        .range(-1000.0..=2000.0),
                );
                if ui.small_button("Remove").clicked() {
                    remove_index = Some(index);
                }
                ui.end_row();
            }
        });

    if let Some(index) = remove_index {
        variations.remove(index);
    }

    ui.horizontal(|ui| {
        if ui.button("Add variation").clicked() {
            variations.push(FontVariationCoord::new("wght", 400.0));
        }
        if !variations.is_empty() && ui.button("Clear").clicked() {
            variations.clear();
        }
    });

    let has_invalid_tag = variations.iter().any(|coord| !coord.is_valid_tag());
    if has_invalid_tag {
        ui.colored_label(
            Color32::from_rgb(176, 42, 55),
            "Variation tags must be four ASCII letters or digits.",
        );
    }

    *variations != original
}

#[cfg(not(target_arch = "wasm32"))]
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
