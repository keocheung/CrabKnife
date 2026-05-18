use eframe::egui::{self, RichText, TextEdit, TextStyle, Ui};

use crate::ui::panel;

pub(crate) struct RadixTool {
    signed_text: String,
    hex_text: String,
    octal_text: String,
    binary_text: String,
    width: IntegerWidth,
    endian: Endian,
    error: Option<String>,
}

impl Default for RadixTool {
    fn default() -> Self {
        let signed_text = "-20".to_owned();
        let width = IntegerWidth::Bits16;
        let endian = Endian::Big;
        let encoded = encode_signed_text(&signed_text, width, endian)
            .expect("default signed integer should encode");

        Self {
            signed_text,
            hex_text: encoded.hex,
            octal_text: encoded.octal,
            binary_text: encoded.binary,
            width,
            endian,
            error: None,
        }
    }
}

impl RadixTool {
    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        let mut settings_changed = false;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.68).max(520.0));

                panel(ui, "Signed Decimal", |ui| {
                    let response = ui.add(
                        TextEdit::singleline(&mut self.signed_text)
                            .font(TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type a signed decimal integer"),
                    );

                    if response.changed() {
                        self.update_from_signed();
                    }
                });

                ui.add_space(14.0);
                panel(ui, "Hex Bytes", |ui| {
                    let response = ui.add(
                        TextEdit::multiline(&mut self.hex_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type bytes, for example FF EC"),
                    );

                    if response.changed() {
                        self.update_from_hex();
                    }
                });

                ui.add_space(14.0);
                panel(ui, "Octal Bytes", |ui| {
                    let response = ui.add(
                        TextEdit::multiline(&mut self.octal_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type octal bytes, for example 377 354"),
                    );

                    if response.changed() {
                        self.update_from_octal();
                    }
                });

                ui.add_space(14.0);
                panel(ui, "Binary", |ui| {
                    let response = ui.add(
                        TextEdit::multiline(&mut self.binary_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type bits, for example 1111 1111 1110 1100"),
                    );

                    if response.changed() {
                        self.update_from_binary();
                    }
                });

                ui.add_space(8.0);
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                } else {
                    ui.label(
                        RichText::new(format!(
                            "{} byte(s), signed two's-complement, {}.",
                            self.width.byte_len(),
                            self.endian.label()
                        ))
                        .color(ui.visuals().weak_text_color()),
                    );
                }
            });

            ui.add_space(14.0);
            ui.vertical(|ui| {
                panel(ui, "Format", |ui| {
                    settings_changed |= width_picker(ui, &mut self.width);
                    ui.add_space(10.0);
                    settings_changed |= endian_picker(ui, &mut self.endian);
                });
            });
        });

        if settings_changed {
            self.update_from_signed();
        }
    }

    fn update_from_signed(&mut self) {
        match encode_signed_text(&self.signed_text, self.width, self.endian) {
            Ok(encoded) => {
                self.hex_text = encoded.hex;
                self.octal_text = encoded.octal;
                self.binary_text = encoded.binary;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn update_from_hex(&mut self) {
        match decode_hex_text(&self.hex_text, self.width, self.endian) {
            Ok(decoded) => {
                self.signed_text = decoded.signed;
                self.octal_text = decoded.octal;
                self.binary_text = decoded.binary;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn update_from_octal(&mut self) {
        match decode_octal_text(&self.octal_text, self.width, self.endian) {
            Ok(decoded) => {
                self.signed_text = decoded.signed;
                self.hex_text = decoded.hex;
                self.binary_text = decoded.binary;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn update_from_binary(&mut self) {
        match decode_binary_text(&self.binary_text, self.width, self.endian) {
            Ok(decoded) => {
                self.signed_text = decoded.signed;
                self.hex_text = decoded.hex;
                self.octal_text = decoded.octal;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IntegerWidth {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
    Bits128,
}

impl IntegerWidth {
    const ALL: [Self; 5] = [
        Self::Bits8,
        Self::Bits16,
        Self::Bits32,
        Self::Bits64,
        Self::Bits128,
    ];

    fn bits(self) -> u32 {
        match self {
            Self::Bits8 => 8,
            Self::Bits16 => 16,
            Self::Bits32 => 32,
            Self::Bits64 => 64,
            Self::Bits128 => 128,
        }
    }

    fn byte_len(self) -> usize {
        (self.bits() / 8) as usize
    }

    fn label(self) -> &'static str {
        match self {
            Self::Bits8 => "8-bit",
            Self::Bits16 => "16-bit",
            Self::Bits32 => "32-bit",
            Self::Bits64 => "64-bit",
            Self::Bits128 => "128-bit",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Endian {
    Big,
    Little,
}

impl Endian {
    const ALL: [Self; 2] = [Self::Big, Self::Little];

    fn label(self) -> &'static str {
        match self {
            Self::Big => "Big-endian",
            Self::Little => "Little-endian",
        }
    }
}

fn width_picker(ui: &mut Ui, selected_width: &mut IntegerWidth) -> bool {
    let mut changed = false;

    ui.label("Width");
    egui::ComboBox::from_id_salt("radix-width")
        .selected_text(selected_width.label())
        .show_ui(ui, |ui| {
            for width in IntegerWidth::ALL {
                changed |= ui
                    .selectable_value(selected_width, width, width.label())
                    .changed();
            }
        });

    changed
}

fn endian_picker(ui: &mut Ui, selected_endian: &mut Endian) -> bool {
    let mut changed = false;

    ui.label("Byte order");
    egui::ComboBox::from_id_salt("radix-endian")
        .selected_text(selected_endian.label())
        .show_ui(ui, |ui| {
            for endian in Endian::ALL {
                changed |= ui
                    .selectable_value(selected_endian, endian, endian.label())
                    .changed();
            }
        });

    changed
}

fn encode_signed_text(
    signed_text: &str,
    width: IntegerWidth,
    endian: Endian,
) -> Result<EncodedTexts, String> {
    let value = signed_text
        .trim()
        .parse::<i128>()
        .map_err(|error| format!("Invalid signed decimal integer: {error}"))?;
    let bytes = signed_to_bytes(value, width, endian)?;
    Ok(EncodedTexts::from_bytes(&bytes))
}

fn decode_hex_text(
    hex_text: &str,
    width: IntegerWidth,
    endian: Endian,
) -> Result<DecodedTexts, String> {
    let bytes = parse_hex_bytes(hex_text)?;
    decode_bytes(bytes, width, endian)
}

fn decode_octal_text(
    octal_text: &str,
    width: IntegerWidth,
    endian: Endian,
) -> Result<DecodedTexts, String> {
    let bytes = parse_octal_bytes(octal_text)?;
    decode_bytes(bytes, width, endian)
}

fn decode_binary_text(
    binary_text: &str,
    width: IntegerWidth,
    endian: Endian,
) -> Result<DecodedTexts, String> {
    let bytes = parse_binary_bytes(binary_text, width)?;
    decode_bytes(bytes, width, endian)
}

fn decode_bytes(
    bytes: Vec<u8>,
    width: IntegerWidth,
    endian: Endian,
) -> Result<DecodedTexts, String> {
    validate_byte_len(bytes.len(), width)?;
    let value = bytes_to_signed(&bytes, width, endian);
    Ok(DecodedTexts {
        signed: value.to_string(),
        hex: format_hex_bytes(&bytes),
        octal: format_octal_bytes(&bytes),
        binary: format_binary_bytes(&bytes),
    })
}

struct EncodedTexts {
    hex: String,
    octal: String,
    binary: String,
}

impl EncodedTexts {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            hex: format_hex_bytes(bytes),
            octal: format_octal_bytes(bytes),
            binary: format_binary_bytes(bytes),
        }
    }
}

struct DecodedTexts {
    signed: String,
    hex: String,
    octal: String,
    binary: String,
}

fn signed_to_bytes(value: i128, width: IntegerWidth, endian: Endian) -> Result<Vec<u8>, String> {
    validate_signed_range(value, width)?;

    let bits = width.bits();
    let raw = if bits == 128 {
        value as u128
    } else if value < 0 {
        (1_u128 << bits) - value.unsigned_abs()
    } else {
        value as u128
    };

    let mut bytes = raw.to_be_bytes()[16 - width.byte_len()..].to_vec();
    if endian == Endian::Little {
        bytes.reverse();
    }
    Ok(bytes)
}

fn bytes_to_signed(bytes: &[u8], width: IntegerWidth, endian: Endian) -> i128 {
    let mut ordered = bytes.to_vec();
    if endian == Endian::Little {
        ordered.reverse();
    }

    let mut raw = 0_u128;
    for byte in ordered {
        raw = (raw << 8) | u128::from(byte);
    }

    let bits = width.bits();
    let sign_bit = 1_u128 << (bits - 1);
    if raw & sign_bit == 0 {
        raw as i128
    } else if bits == 128 {
        raw as i128
    } else {
        (raw as i128) - (1_i128 << bits)
    }
}

fn validate_signed_range(value: i128, width: IntegerWidth) -> Result<(), String> {
    let bits = width.bits();
    let min = if bits == 128 {
        i128::MIN
    } else {
        -(1_i128 << (bits - 1))
    };
    let max = if bits == 128 {
        i128::MAX
    } else {
        (1_i128 << (bits - 1)) - 1
    };

    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "Value is outside the {} range ({} to {}).",
            width.label(),
            min,
            max
        ))
    }
}

fn validate_byte_len(actual: usize, width: IntegerWidth) -> Result<(), String> {
    let expected = width.byte_len();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "Expected {expected} byte(s) for {}, got {actual}.",
            width.label()
        ))
    }
}

fn parse_hex_bytes(hex_text: &str) -> Result<Vec<u8>, String> {
    let mut nibbles = Vec::new();
    let mut chars = hex_text.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '0' && matches!(chars.peek(), Some('x' | 'X')) {
            chars.next();
            continue;
        }

        if character.is_ascii_hexdigit() {
            nibbles.push(
                character
                    .to_digit(16)
                    .expect("ASCII hex digit should parse") as u8,
            );
        } else if character.is_ascii_whitespace()
            || matches!(character, '_' | '-' | ':' | ',' | ';')
        {
            continue;
        } else {
            return Err(format!("Invalid hex character: {character}"));
        }
    }

    if nibbles.len() % 2 != 0 {
        return Err("Hex input must contain complete bytes.".to_owned());
    }

    Ok(nibbles
        .chunks_exact(2)
        .map(|pair| (pair[0] << 4) | pair[1])
        .collect())
}

fn parse_binary_bytes(binary_text: &str, width: IntegerWidth) -> Result<Vec<u8>, String> {
    let bits = binary_text
        .chars()
        .filter_map(|character| match character {
            '0' => Some(Ok(0_u8)),
            '1' => Some(Ok(1_u8)),
            character if character.is_ascii_whitespace() || matches!(character, '_' | '-') => None,
            character => Some(Err(format!("Invalid binary character: {character}"))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if bits.len() != width.bits() as usize {
        return Err(format!(
            "Expected {} bit(s) for {}, got {}.",
            width.bits(),
            width.label(),
            bits.len()
        ));
    }

    Ok(bits
        .chunks_exact(8)
        .map(|chunk| chunk.iter().fold(0_u8, |byte, bit| (byte << 1) | bit))
        .collect())
}

fn parse_octal_bytes(octal_text: &str) -> Result<Vec<u8>, String> {
    octal_text
        .split(|character: char| {
            character.is_ascii_whitespace() || matches!(character, '_' | '-' | ':' | ',' | ';')
        })
        .filter(|part| !part.is_empty())
        .map(|part| {
            let digits = part
                .strip_prefix("0o")
                .or_else(|| part.strip_prefix("0O"))
                .unwrap_or(part);

            if digits.is_empty() {
                return Err("Octal input contains an empty byte.".to_owned());
            }
            if !digits
                .chars()
                .all(|character| matches!(character, '0'..='7'))
            {
                return Err(format!("Invalid octal byte: {part}"));
            }

            u8::from_str_radix(digits, 8)
                .map_err(|_| format!("Octal byte is outside 000-377: {part}"))
        })
        .collect()
}

fn format_hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_octal_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:03o}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_binary_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:08b}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_negative_value_as_twos_complement() {
        let encoded = encode_signed_text("-20", IntegerWidth::Bits16, Endian::Big).unwrap();
        assert_eq!(encoded.hex, "FF EC");
        assert_eq!(encoded.octal, "377 354");
        assert_eq!(encoded.binary, "11111111 11101100");
    }

    #[test]
    fn encodes_little_endian_bytes() {
        let encoded = encode_signed_text("-20", IntegerWidth::Bits16, Endian::Little).unwrap();
        assert_eq!(encoded.hex, "EC FF");
        assert_eq!(encoded.octal, "354 377");
        assert_eq!(encoded.binary, "11101100 11111111");
    }

    #[test]
    fn decodes_big_endian_hex_bytes() {
        let decoded = decode_hex_text("FF EC", IntegerWidth::Bits16, Endian::Big).unwrap();
        assert_eq!(decoded.signed, "-20");
        assert_eq!(decoded.octal, "377 354");
        assert_eq!(decoded.binary, "11111111 11101100");
    }

    #[test]
    fn decodes_little_endian_hex_bytes() {
        let decoded = decode_hex_text("EC FF", IntegerWidth::Bits16, Endian::Little).unwrap();
        assert_eq!(decoded.signed, "-20");
        assert_eq!(decoded.octal, "354 377");
        assert_eq!(decoded.binary, "11101100 11111111");
    }

    #[test]
    fn decodes_octal_bytes() {
        let decoded = decode_octal_text("377 354", IntegerWidth::Bits16, Endian::Big).unwrap();
        assert_eq!(decoded.signed, "-20");
        assert_eq!(decoded.hex, "FF EC");
        assert_eq!(decoded.binary, "11111111 11101100");
    }

    #[test]
    fn decodes_binary_text() {
        let decoded =
            decode_binary_text("1111 1111 1110 1100", IntegerWidth::Bits16, Endian::Big).unwrap();
        assert_eq!(decoded.signed, "-20");
        assert_eq!(decoded.hex, "FF EC");
        assert_eq!(decoded.octal, "377 354");
    }

    #[test]
    fn rejects_out_of_range_signed_values() {
        assert!(encode_signed_text("128", IntegerWidth::Bits8, Endian::Big).is_err());
        assert!(encode_signed_text("-129", IntegerWidth::Bits8, Endian::Big).is_err());
    }

    #[test]
    fn handles_i128_min() {
        let bytes = signed_to_bytes(i128::MIN, IntegerWidth::Bits128, Endian::Big).unwrap();
        assert_eq!(
            bytes_to_signed(&bytes, IntegerWidth::Bits128, Endian::Big),
            i128::MIN
        );
    }
}
