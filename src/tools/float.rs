use eframe::egui::{self, RichText, TextEdit, TextStyle, Ui};

use crate::ui::panel;

pub(crate) struct FloatTool {
    decimal_text: String,
    hex_text: String,
    binary_text: String,
    precision: FloatPrecision,
    endian: Endian,
    error: Option<String>,
}

impl Default for FloatTool {
    fn default() -> Self {
        let decimal_text = "3.1415926".to_owned();
        let precision = FloatPrecision::Single;
        let endian = Endian::Big;
        let encoded = encode_decimal_text(&decimal_text, precision, endian)
            .expect("default floating-point value should encode");

        Self {
            decimal_text,
            hex_text: encoded.hex,
            binary_text: encoded.binary,
            precision,
            endian,
            error: None,
        }
    }
}

impl FloatTool {
    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        let mut settings_changed = false;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.68).max(520.0));

                panel(ui, "Decimal", |ui| {
                    let response = ui.add(
                        TextEdit::singleline(&mut self.decimal_text)
                            .font(TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type a decimal floating-point value"),
                    );

                    if response.changed() {
                        self.update_from_decimal();
                    }
                });

                ui.add_space(14.0);
                panel(ui, "Hex Encoding", |ui| {
                    let response = ui.add(
                        TextEdit::singleline(&mut self.hex_text)
                            .font(TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type IEEE 754 bits as hex"),
                    );

                    if response.changed() {
                        self.update_from_hex();
                    }
                });

                ui.add_space(14.0);
                panel(ui, "Binary Encoding", |ui| {
                    let response = ui.add(
                        TextEdit::multiline(&mut self.binary_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type IEEE 754 bits as binary"),
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
                            "{} IEEE 754 encoding, {} bit(s), {}.",
                            self.precision.label(),
                            self.precision.bits(),
                            self.endian.label()
                        ))
                        .color(ui.visuals().weak_text_color()),
                    );
                }
            });

            ui.add_space(14.0);
            ui.vertical(|ui| {
                panel(ui, "Format", |ui| {
                    settings_changed |= precision_picker(ui, &mut self.precision);
                    ui.add_space(10.0);
                    settings_changed |= endian_picker(ui, &mut self.endian);
                });
            });
        });

        if settings_changed {
            self.update_from_decimal();
        }
    }

    fn update_from_decimal(&mut self) {
        match encode_decimal_text(&self.decimal_text, self.precision, self.endian) {
            Ok(encoded) => {
                self.hex_text = encoded.hex;
                self.binary_text = encoded.binary;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn update_from_hex(&mut self) {
        match decode_hex_text(&self.hex_text, self.precision, self.endian) {
            Ok(decoded) => {
                self.decimal_text = decoded.decimal;
                self.binary_text = decoded.binary;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn update_from_binary(&mut self) {
        match decode_binary_text(&self.binary_text, self.precision, self.endian) {
            Ok(decoded) => {
                self.decimal_text = decoded.decimal;
                self.hex_text = decoded.hex;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum FloatPrecision {
    Half,
    Single,
    Double,
}

impl FloatPrecision {
    const ALL: [Self; 3] = [Self::Half, Self::Single, Self::Double];

    fn bits(self) -> usize {
        match self {
            Self::Half => 16,
            Self::Single => 32,
            Self::Double => 64,
        }
    }

    fn hex_digits(self) -> usize {
        self.bits() / 4
    }

    fn label(self) -> &'static str {
        match self {
            Self::Half => "Half precision (binary16)",
            Self::Single => "Single precision (binary32)",
            Self::Double => "Double precision (binary64)",
        }
    }
}

fn precision_picker(ui: &mut Ui, selected_precision: &mut FloatPrecision) -> bool {
    let mut changed = false;

    ui.label("Precision");
    egui::ComboBox::from_id_salt("float-precision")
        .selected_text(selected_precision.label())
        .show_ui(ui, |ui| {
            for precision in FloatPrecision::ALL {
                changed |= ui
                    .selectable_value(selected_precision, precision, precision.label())
                    .changed();
            }
        });

    changed
}

fn endian_picker(ui: &mut Ui, selected_endian: &mut Endian) -> bool {
    let mut changed = false;

    ui.label("Byte order");
    egui::ComboBox::from_id_salt("float-endian")
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

fn encode_decimal_text(
    decimal_text: &str,
    precision: FloatPrecision,
    endian: Endian,
) -> Result<EncodedTexts, String> {
    let value = parse_decimal(decimal_text)?;
    Ok(EncodedTexts::from_bits(
        decimal_to_bits(value, precision),
        precision,
        endian,
    ))
}

fn decode_hex_text(
    hex_text: &str,
    precision: FloatPrecision,
    endian: Endian,
) -> Result<DecodedTexts, String> {
    let bits = parse_hex_bits(hex_text, precision, endian)?;
    Ok(DecodedTexts::from_bits(bits, precision, endian))
}

fn decode_binary_text(
    binary_text: &str,
    precision: FloatPrecision,
    endian: Endian,
) -> Result<DecodedTexts, String> {
    let bits = parse_binary_bits(binary_text, precision, endian)?;
    Ok(DecodedTexts::from_bits(bits, precision, endian))
}

struct EncodedTexts {
    hex: String,
    binary: String,
}

impl EncodedTexts {
    fn from_bits(bits: u64, precision: FloatPrecision, endian: Endian) -> Self {
        Self {
            hex: format_hex_bits(bits, precision, endian),
            binary: format_binary_bits(bits, precision, endian),
        }
    }
}

struct DecodedTexts {
    decimal: String,
    hex: String,
    binary: String,
}

impl DecodedTexts {
    fn from_bits(bits: u64, precision: FloatPrecision, endian: Endian) -> Self {
        Self {
            decimal: format_decimal(bits_to_decimal(bits, precision), precision),
            hex: format_hex_bits(bits, precision, endian),
            binary: format_binary_bits(bits, precision, endian),
        }
    }
}

fn parse_decimal(decimal_text: &str) -> Result<f64, String> {
    let trimmed = decimal_text.trim();
    if trimmed.is_empty() {
        return Err("Decimal input is empty.".to_owned());
    }

    trimmed
        .parse::<f64>()
        .map_err(|error| format!("Invalid decimal floating-point value: {error}"))
}

fn decimal_to_bits(value: f64, precision: FloatPrecision) -> u64 {
    match precision {
        FloatPrecision::Half => u64::from(f64_to_f16_bits(value)),
        FloatPrecision::Single => u64::from((value as f32).to_bits()),
        FloatPrecision::Double => value.to_bits(),
    }
}

fn bits_to_decimal(bits: u64, precision: FloatPrecision) -> f64 {
    match precision {
        FloatPrecision::Half => f64::from(f16_bits_to_f32(bits as u16)),
        FloatPrecision::Single => f64::from(f32::from_bits(bits as u32)),
        FloatPrecision::Double => f64::from_bits(bits),
    }
}

fn parse_hex_bits(
    hex_text: &str,
    precision: FloatPrecision,
    endian: Endian,
) -> Result<u64, String> {
    let mut digits = Vec::new();
    let mut chars = hex_text.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '0' && matches!(chars.peek(), Some('x' | 'X')) {
            chars.next();
            continue;
        }

        if character.is_ascii_hexdigit() {
            digits.push(character);
        } else if character.is_ascii_whitespace()
            || matches!(character, '_' | '-' | ':' | ',' | ';')
        {
            continue;
        } else {
            return Err(format!("Invalid hex character: {character}"));
        }
    }

    let expected = precision.hex_digits();
    if digits.len() != expected {
        return Err(format!(
            "Expected {expected} hex digit(s) for {}, got {}.",
            precision.label(),
            digits.len()
        ));
    }

    let bytes = digits
        .chunks_exact(2)
        .map(|pair| {
            let high = pair[0].to_digit(16).expect("ASCII hex digit should parse") as u8;
            let low = pair[1].to_digit(16).expect("ASCII hex digit should parse") as u8;
            (high << 4) | low
        })
        .collect::<Vec<_>>();
    Ok(bytes_to_bits(&bytes, endian))
}

fn parse_binary_bits(
    binary_text: &str,
    precision: FloatPrecision,
    endian: Endian,
) -> Result<u64, String> {
    let bits = binary_text
        .chars()
        .filter_map(|character| match character {
            '0' => Some(Ok(0_u8)),
            '1' => Some(Ok(1_u8)),
            character if character.is_ascii_whitespace() || matches!(character, '_' | '-') => None,
            character => Some(Err(format!("Invalid binary character: {character}"))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if bits.len() != precision.bits() {
        return Err(format!(
            "Expected {} bit(s) for {}, got {}.",
            precision.bits(),
            precision.label(),
            bits.len()
        ));
    }

    let bytes = bits
        .chunks_exact(8)
        .map(|chunk| chunk.iter().fold(0_u8, |byte, bit| (byte << 1) | bit))
        .collect::<Vec<_>>();
    Ok(bytes_to_bits(&bytes, endian))
}

fn format_hex_bits(bits: u64, precision: FloatPrecision, endian: Endian) -> String {
    let bytes = bits_to_bytes(bits, precision, endian);
    format!(
        "0x{}",
        bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join("")
    )
}

fn format_binary_bits(bits: u64, precision: FloatPrecision, endian: Endian) -> String {
    let raw = bits_to_bytes(bits, precision, endian)
        .iter()
        .map(|byte| format!("{byte:08b}"))
        .collect::<Vec<_>>()
        .join("");
    format_grouped_binary(&raw)
}

fn bits_to_bytes(bits: u64, precision: FloatPrecision, endian: Endian) -> Vec<u8> {
    let mut bytes = bits.to_be_bytes()[8 - precision.bits() / 8..].to_vec();
    if endian == Endian::Little {
        bytes.reverse();
    }
    bytes
}

fn bytes_to_bits(bytes: &[u8], endian: Endian) -> u64 {
    let mut ordered = bytes.to_vec();
    if endian == Endian::Little {
        ordered.reverse();
    }

    ordered
        .iter()
        .fold(0_u64, |bits, byte| (bits << 8) | u64::from(*byte))
}

fn format_grouped_binary(binary: &str) -> String {
    binary
        .as_bytes()
        .chunks(4)
        .map(|chunk| {
            std::str::from_utf8(chunk)
                .expect("binary digits should be UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_decimal(value: f64, precision: FloatPrecision) -> String {
    if value.is_nan() {
        "NaN".to_owned()
    } else if value.is_infinite() {
        if value.is_sign_negative() {
            "-inf".to_owned()
        } else {
            "inf".to_owned()
        }
    } else {
        match precision {
            FloatPrecision::Half => format!("{}", f16_bits_to_f32(f64_to_f16_bits(value))),
            FloatPrecision::Single => format!("{}", value as f32),
            FloatPrecision::Double => format!("{value}"),
        }
    }
}

fn f64_to_f16_bits(value: f64) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 48) & 0x8000) as u16;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    if exponent == 0x7ff {
        if mantissa == 0 {
            return sign | 0x7c00;
        }

        let payload = (mantissa >> 42) as u16;
        return sign | 0x7c00 | payload | 0x0200;
    }

    let half_exponent = exponent - 1023 + 15;
    if half_exponent >= 0x1f {
        return sign | 0x7c00;
    }

    if half_exponent <= 0 {
        if half_exponent < -10 {
            return sign;
        }

        let rounded = round_shift_u64(
            mantissa | 0x0010_0000_0000_0000,
            (43 - half_exponent) as u32,
        );
        return sign | rounded as u16;
    }

    let rounded_mantissa = round_shift_u64(mantissa, 42);
    if rounded_mantissa == 0x0400 {
        let rounded_exponent = half_exponent + 1;
        if rounded_exponent >= 0x1f {
            sign | 0x7c00
        } else {
            sign | ((rounded_exponent as u16) << 10)
        }
    } else {
        sign | ((half_exponent as u16) << 10) | rounded_mantissa as u16
    }
}

fn round_shift_u64(value: u64, shift: u32) -> u64 {
    let shifted = value >> shift;
    let remainder_mask = (1_u64 << shift) - 1;
    let remainder = value & remainder_mask;
    let halfway = 1_u64 << (shift - 1);

    if remainder > halfway || (remainder == halfway && shifted % 2 == 1) {
        shifted + 1
    } else {
        shifted
    }
}

fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = (u32::from(bits & 0x8000)) << 16;
    let exponent = (bits >> 10) & 0x1f;
    let mantissa = u32::from(bits & 0x03ff);

    let f32_bits = if exponent == 0 {
        if mantissa == 0 {
            sign
        } else {
            let mut normalized_mantissa = mantissa;
            let mut normalized_exponent = -14_i32;
            while normalized_mantissa & 0x0400 == 0 {
                normalized_mantissa <<= 1;
                normalized_exponent -= 1;
            }
            normalized_mantissa &= 0x03ff;
            sign | (((normalized_exponent + 127) as u32) << 23) | (normalized_mantissa << 13)
        }
    } else if exponent == 0x1f {
        sign | 0x7f80_0000 | (mantissa << 13)
    } else {
        sign | ((u32::from(exponent) + 112) << 23) | (mantissa << 13)
    };

    f32::from_bits(f32_bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_decimal_as_single_precision() {
        let encoded = encode_decimal_text("1.5", FloatPrecision::Single, Endian::Big).unwrap();
        assert_eq!(encoded.hex, "0x3FC00000");
        assert_eq!(encoded.binary, "0011 1111 1100 0000 0000 0000 0000 0000");
    }

    #[test]
    fn decodes_double_precision_hex() {
        let decoded =
            decode_hex_text("0x4009 21fb 5444 2d18", FloatPrecision::Double, Endian::Big).unwrap();
        assert_eq!(decoded.decimal, std::f64::consts::PI.to_string());
        assert_eq!(decoded.binary.len(), 79);
    }

    #[test]
    fn encodes_half_precision_values() {
        let encoded = encode_decimal_text("1.5", FloatPrecision::Half, Endian::Big).unwrap();
        assert_eq!(encoded.hex, "0x3E00");
        assert_eq!(encoded.binary, "0011 1110 0000 0000");

        let decoded =
            decode_binary_text("0011111000000000", FloatPrecision::Half, Endian::Big).unwrap();
        assert_eq!(decoded.decimal, "1.5");
        assert_eq!(decoded.hex, "0x3E00");
    }

    #[test]
    fn encodes_little_endian_bytes() {
        let encoded = encode_decimal_text("1.5", FloatPrecision::Single, Endian::Little).unwrap();
        assert_eq!(encoded.hex, "0x0000C03F");
        assert_eq!(encoded.binary, "0000 0000 0000 0000 1100 0000 0011 1111");

        let decoded = decode_hex_text("0000 C03F", FloatPrecision::Single, Endian::Little).unwrap();
        assert_eq!(decoded.decimal, "1.5");
        assert_eq!(decoded.binary, encoded.binary);
    }

    #[test]
    fn decodes_half_precision_subnormal() {
        let decoded = decode_hex_text("0001", FloatPrecision::Half, Endian::Big).unwrap();
        assert_eq!(decoded.decimal, "0.000000059604645");
        assert_eq!(decoded.binary, "0000 0000 0000 0001");
    }

    #[test]
    fn handles_infinity_and_nan() {
        assert_eq!(
            decode_hex_text("7C00", FloatPrecision::Half, Endian::Big)
                .unwrap()
                .decimal,
            "inf"
        );
        assert_eq!(
            decode_hex_text("7E00", FloatPrecision::Half, Endian::Big)
                .unwrap()
                .decimal,
            "NaN"
        );
    }

    #[test]
    fn rejects_wrong_width_input() {
        assert!(decode_hex_text("3F80", FloatPrecision::Single, Endian::Big).is_err());
        assert!(decode_binary_text("1010", FloatPrecision::Half, Endian::Big).is_err());
    }
}
