use std::sync::LazyLock;

use eframe::egui::{self, RichText, TextEdit, TextStyle, Ui};
use encoding_rs::GBK;
use regex::Regex;

use crate::ui::panel;

pub(crate) struct HexTool {
    hex_text: String,
    encoding: TextEncoding,
}

impl Default for HexTool {
    fn default() -> Self {
        Self {
            hex_text: "48656c6c6f2c20527573744b6e69666521".to_owned(),
            encoding: TextEncoding::Utf8,
        }
    }
}

impl HexTool {
    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        let bytes = hex_to_bytes(&self.hex_text);
        let mut decoded_text = decode_bytes(&bytes, self.encoding);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.62).max(460.0));

                panel(ui, "Hex", |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.hex_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(12)
                            .desired_width(f32::INFINITY)
                            .hint_text("Paste hexadecimal text"),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "{} byte{} decoded. Non-hex characters are ignored.",
                            bytes.len(),
                            if bytes.len() == 1 { "" } else { "s" }
                        ))
                        .color(ui.visuals().weak_text_color()),
                    );
                });

                ui.add_space(14.0);
                panel(ui, "String", |ui| {
                    ui.add(
                        TextEdit::multiline(&mut decoded_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(12)
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                });
            });

            ui.add_space(14.0);
            ui.vertical(|ui| {
                panel(ui, "Encoding", |ui| {
                    encoding_picker(ui, &mut self.encoding);
                });
            });
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Gbk,
}

impl TextEncoding {
    const ALL: [Self; 4] = [Self::Utf8, Self::Utf16Le, Self::Utf16Be, Self::Gbk];

    fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Gbk => "GBK",
        }
    }
}

fn encoding_picker(ui: &mut Ui, selected_encoding: &mut TextEncoding) {
    egui::ComboBox::from_id_salt("hex-text-encoding")
        .selected_text(selected_encoding.label())
        .show_ui(ui, |ui| {
            for encoding in TextEncoding::ALL {
                ui.selectable_value(selected_encoding, encoding, encoding.label());
            }
        });
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut high_nibble = None;
    let normalized_hex = normalize_prefixed_hex(hex);

    for value in normalized_hex
        .chars()
        .filter_map(|character| character.to_digit(16))
    {
        let value = value as u8;
        if let Some(high) = high_nibble.take() {
            bytes.push((high << 4) | value);
        } else {
            high_nibble = Some(value);
        }
    }

    bytes
}

fn normalize_prefixed_hex(hex: &str) -> String {
    static PREFIXED_HEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"0x([0-9A-Fa-f]{2})").expect("prefixed hex regex should compile")
    });

    PREFIXED_HEX.replace_all(hex, "$1").into_owned()
}

fn decode_bytes(bytes: &[u8], encoding: TextEncoding) -> String {
    match encoding {
        TextEncoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        TextEncoding::Utf16Le => decode_utf16(bytes, u16::from_le_bytes),
        TextEncoding::Utf16Be => decode_utf16(bytes, u16::from_be_bytes),
        TextEncoding::Gbk => GBK.decode(bytes).0.into_owned(),
    }
}

fn decode_utf16(bytes: &[u8], make_unit: fn([u8; 2]) -> u16) -> String {
    let units = bytes
        .chunks_exact(2)
        .map(|chunk| make_unit([chunk[0], chunk[1]]));
    char::decode_utf16(units)
        .map(|result| result.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_non_hex_characters() {
        assert_eq!(hex_to_bytes("48 zz 65-6c/6c:6f!"), b"Hello");
    }

    #[test]
    fn strips_hex_byte_prefixes_before_decoding() {
        assert_eq!(hex_to_bytes("0x48 0x65 0x6c 0x6c 0x6f"), b"Hello");
    }

    #[test]
    fn ignores_trailing_single_nibble() {
        assert_eq!(hex_to_bytes("486"), vec![0x48]);
    }

    #[test]
    fn decodes_utf16_endianness() {
        assert_eq!(
            decode_bytes(&[0x60, 0x4f, 0x7d, 0x59], TextEncoding::Utf16Le),
            "你好"
        );
        assert_eq!(
            decode_bytes(&[0x4f, 0x60, 0x59, 0x7d], TextEncoding::Utf16Be),
            "你好"
        );
    }

    #[test]
    fn decodes_gbk() {
        assert_eq!(
            decode_bytes(&[0xc4, 0xe3, 0xba, 0xc3], TextEncoding::Gbk),
            "你好"
        );
    }
}
