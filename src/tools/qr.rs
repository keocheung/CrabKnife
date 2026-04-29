use std::io::Cursor;

use eframe::egui::{self, Color32, RichText, ScrollArea, Sense, TextEdit, TextStyle, Ui, Vec2};
use image::{DynamicImage, ImageBuffer, ImageFormat, Luma};
use qrcode::{Color, EcLevel, QrCode};

use crate::ui::panel;

pub(crate) struct QrTool {
    input_text: String,
    error_correction: ErrorCorrection,
    cached_text: String,
    cached_error_correction: ErrorCorrection,
    cached_code: Option<QrCode>,
    cached_error: Option<String>,
    export_error: Option<String>,
}

impl Default for QrTool {
    fn default() -> Self {
        let input_text = "https://example.com".to_owned();
        let error_correction = ErrorCorrection::Medium;
        let (cached_code, cached_error) = build_qr_code(&input_text, error_correction);

        Self {
            cached_text: input_text.clone(),
            input_text,
            cached_error_correction: error_correction,
            error_correction,
            cached_code,
            cached_error,
            export_error: None,
        }
    }
}

impl QrTool {
    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        self.refresh_cache();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() * 0.55).max(420.0));
                        panel(ui, "Text", |ui| {
                            ui.add(
                                TextEdit::multiline(&mut self.input_text)
                                    .font(TextStyle::Monospace)
                                    .desired_rows(14)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Type text to encode as a QR code"),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("{} byte(s)", self.input_text.len()))
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });

                        ui.add_space(14.0);
                        panel(ui, "Redundancy", |ui| {
                            error_correction_picker(ui, &mut self.error_correction);
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(self.error_correction.description())
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });

                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        panel(ui, "QR Code", |ui| self.preview(ui));
                    });
                });
            });
    }

    fn refresh_cache(&mut self) {
        if self.input_text == self.cached_text
            && self.error_correction == self.cached_error_correction
        {
            return;
        }

        self.cached_text = self.input_text.clone();
        self.cached_error_correction = self.error_correction;
        let (code, error) = build_qr_code(&self.input_text, self.error_correction);
        self.cached_code = code;
        self.cached_error = error;
        self.export_error = None;
    }

    fn preview(&mut self, ui: &mut Ui) {
        if let Some(error) = &self.cached_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
            return;
        }

        let Some(code) = &self.cached_code else {
            ui.label(
                RichText::new("Enter text to create a QR code.")
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };

        draw_qr_code(ui, code);
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!(
                "{} x {} modules, {} correction.",
                code.width(),
                code.width(),
                self.error_correction.label()
            ))
            .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(8.0);
        export_button(ui, self);

        if let Some(error) = &self.export_error {
            ui.add_space(8.0);
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_png(&mut self) {
        let Some(code) = &self.cached_code else {
            return;
        };

        let Some(mut path) = rfd::FileDialog::new()
            .set_file_name("qr-code.png")
            .add_filter("PNG image", &["png"])
            .save_file()
        else {
            return;
        };

        if path.extension().is_none() {
            path.set_extension("png");
        }

        match qr_png_bytes(code, 12, 4).and_then(|bytes| {
            std::fs::write(&path, bytes).map_err(|error| format!("Could not save PNG: {error}"))
        }) {
            Ok(()) => self.export_error = None,
            Err(error) => self.export_error = Some(error),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ErrorCorrection {
    Low,
    Medium,
    Quartile,
    High,
}

impl ErrorCorrection {
    const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::Quartile, Self::High];

    fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::Quartile => "Quartile",
            Self::High => "High",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Low => "Recovers about 7% of damaged data.",
            Self::Medium => "Recovers about 15% of damaged data.",
            Self::Quartile => "Recovers about 25% of damaged data.",
            Self::High => "Recovers about 30% of damaged data.",
        }
    }

    fn ec_level(self) -> EcLevel {
        match self {
            Self::Low => EcLevel::L,
            Self::Medium => EcLevel::M,
            Self::Quartile => EcLevel::Q,
            Self::High => EcLevel::H,
        }
    }
}

fn error_correction_picker(ui: &mut Ui, selected_level: &mut ErrorCorrection) {
    egui::ComboBox::from_id_salt("qr-error-correction")
        .selected_text(selected_level.label())
        .show_ui(ui, |ui| {
            for level in ErrorCorrection::ALL {
                ui.selectable_value(selected_level, level, level.label());
            }
        });
}

fn build_qr_code(
    text: &str,
    error_correction: ErrorCorrection,
) -> (Option<QrCode>, Option<String>) {
    if text.is_empty() {
        return (None, None);
    }

    match QrCode::with_error_correction_level(text.as_bytes(), error_correction.ec_level()) {
        Ok(code) => (Some(code), None),
        Err(error) => (
            None,
            Some(format!(
                "Could not create QR code at this redundancy level: {error}"
            )),
        ),
    }
}

fn draw_qr_code(ui: &mut Ui, code: &QrCode) {
    const QUIET_ZONE_MODULES: usize = 4;

    let module_count = code.width() + QUIET_ZONE_MODULES * 2;
    let side = ui.available_width().min(420.0);
    let module_side = (side / module_count as f32).floor().max(1.0);
    let image_side = module_side * module_count as f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(image_side), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 0.0, Color32::WHITE);

    for y in 0..code.width() {
        for x in 0..code.width() {
            if code[(x, y)] != Color::Dark {
                continue;
            }

            let min = rect.min
                + Vec2::new(
                    (x + QUIET_ZONE_MODULES) as f32 * module_side,
                    (y + QUIET_ZONE_MODULES) as f32 * module_side,
                );
            let max = min + Vec2::splat(module_side);
            painter.rect_filled(egui::Rect::from_min_max(min, max), 0.0, Color32::BLACK);
        }
    }

    painter.rect_stroke(
        rect,
        0.0,
        ui.visuals().widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Outside,
    );
}

fn export_button(ui: &mut Ui, tool: &mut QrTool) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if ui.button("Export PNG").clicked() {
            tool.export_png();
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = tool;
        ui.add_enabled(false, egui::Button::new("Export PNG"));
        ui.label(
            RichText::new("PNG export is available in the desktop app.")
                .color(ui.visuals().weak_text_color()),
        );
    }
}

fn qr_png_bytes(
    code: &QrCode,
    module_pixels: u32,
    quiet_zone_modules: u32,
) -> Result<Vec<u8>, String> {
    let width = code.width() as u32;
    let pixel_width = (width + quiet_zone_modules * 2) * module_pixels;
    let mut image = ImageBuffer::from_pixel(pixel_width, pixel_width, Luma([255u8]));

    for y in 0..width {
        for x in 0..width {
            if code[(x as usize, y as usize)] != Color::Dark {
                continue;
            }

            let start_x = (x + quiet_zone_modules) * module_pixels;
            let start_y = (y + quiet_zone_modules) * module_pixels;
            for pixel_y in start_y..start_y + module_pixels {
                for pixel_x in start_x..start_x + module_pixels {
                    image.put_pixel(pixel_x, pixel_y, Luma([0u8]));
                }
            }
        }
    }

    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|error| format!("Could not encode PNG: {error}"))?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_qr_code_for_text() {
        let (code, error) = build_qr_code("Hello, RustKnife!", ErrorCorrection::Medium);

        assert!(error.is_none());
        assert!(code.is_some());
    }

    #[test]
    fn empty_text_has_no_error_or_code() {
        let (code, error) = build_qr_code("", ErrorCorrection::Medium);

        assert!(code.is_none());
        assert!(error.is_none());
    }

    #[test]
    fn encodes_png_bytes() {
        let (code, error) = build_qr_code("Hello, RustKnife!", ErrorCorrection::High);
        let bytes = qr_png_bytes(&code.unwrap(), 4, 4).unwrap();

        assert!(error.is_none());
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
