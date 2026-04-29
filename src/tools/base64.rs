use base64::{Engine as _, engine::general_purpose};
use eframe::egui::{RichText, TextEdit, TextStyle, Ui};

use crate::ui::panel;

pub(crate) struct Base64Tool {
    plain_text: String,
    base64_text: String,
    decode_error: Option<String>,
}

impl Default for Base64Tool {
    fn default() -> Self {
        let plain_text = "Hello, RustKnife!".to_owned();
        let base64_text = encode_text(&plain_text);
        Self {
            plain_text,
            base64_text,
            decode_error: None,
        }
    }
}

impl Base64Tool {
    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - 14.0) * 0.5);
                panel(ui, "Plain Text", |ui| {
                    let response = ui.add(
                        TextEdit::multiline(&mut self.plain_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(18)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type plain text"),
                    );

                    if response.changed() {
                        self.base64_text = encode_text(&self.plain_text);
                        self.decode_error = None;
                    }
                });
            });

            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                panel(ui, "Base64", |ui| {
                    let response = ui.add(
                        TextEdit::multiline(&mut self.base64_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(18)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type Base64 text"),
                    );

                    if response.changed() {
                        match decode_text(&self.base64_text) {
                            Ok(plain_text) => {
                                self.plain_text = plain_text;
                                self.decode_error = None;
                            }
                            Err(error) => {
                                self.decode_error = Some(error);
                            }
                        }
                    }

                    if let Some(error) = &self.decode_error {
                        ui.add_space(8.0);
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    } else {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!("{} byte(s) encoded.", self.plain_text.len()))
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                });
            });
        });
    }
}

fn encode_text(text: &str) -> String {
    general_purpose::STANDARD.encode(text.as_bytes())
}

fn decode_text(base64_text: &str) -> Result<String, String> {
    let bytes = general_purpose::STANDARD
        .decode(base64_text.trim())
        .map_err(|error| format!("Invalid Base64: {error}"))?;
    String::from_utf8(bytes).map_err(|error| format!("Decoded bytes are not UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_plain_text() {
        assert_eq!(encode_text("Hello, RustKnife!"), "SGVsbG8sIFJ1c3RLbmlmZSE=");
    }

    #[test]
    fn decodes_base64_text() {
        assert_eq!(
            decode_text("SGVsbG8sIFJ1c3RLbmlmZSE=").unwrap(),
            "Hello, RustKnife!"
        );
    }

    #[test]
    fn rejects_invalid_base64() {
        assert!(decode_text("not base64!!").is_err());
    }
}
