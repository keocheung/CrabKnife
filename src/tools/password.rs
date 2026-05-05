use eframe::egui::{self, RichText, TextEdit, TextStyle, Ui};

use crate::ui::panel;

const UPPERCASE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGITS: &[u8] = b"0123456789";
const SPECIAL: &[u8] = b"!@#$%^&*()-_=+[]{};:,.<>/?";

pub(crate) struct PasswordTool {
    include_uppercase: bool,
    include_lowercase: bool,
    include_digits: bool,
    include_special: bool,
    length: usize,
    count: usize,
    output: String,
    error: Option<String>,
}

impl Default for PasswordTool {
    fn default() -> Self {
        let mut tool = Self {
            include_uppercase: true,
            include_lowercase: true,
            include_digits: true,
            include_special: false,
            length: 32,
            count: 1,
            output: String::new(),
            error: None,
        };
        tool.regenerate();
        tool
    }
}

impl PasswordTool {
    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.38).max(320.0));
                panel(ui, "Options", |ui| {
                    ui.checkbox(&mut self.include_uppercase, "Uppercase letters");
                    ui.checkbox(&mut self.include_lowercase, "Lowercase letters");
                    ui.checkbox(&mut self.include_digits, "Numbers");
                    ui.checkbox(&mut self.include_special, "Special characters");

                    ui.add_space(12.0);
                    ui.add(egui::Slider::new(&mut self.length, 1..=256).text("Length"));
                    ui.add(egui::Slider::new(&mut self.count, 1..=100).text("Count"));

                    ui.add_space(12.0);
                    if ui
                        .add_sized([ui.available_width(), 38.0], egui::Button::new("Generate"))
                        .clicked()
                    {
                        self.regenerate();
                    }

                    if let Some(error) = &self.error {
                        ui.add_space(8.0);
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    } else {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Random bytes come from the system random API.")
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                });
            });

            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                panel(ui, "Generated Passwords", |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.output)
                            .font(TextStyle::Monospace)
                            .desired_rows(18)
                            .desired_width(f32::INFINITY),
                    );
                });
            });
        });
    }

    fn regenerate(&mut self) {
        match generate_passwords(&self.options(), self.length, self.count) {
            Ok(passwords) => {
                self.output = passwords.join("\n");
                self.error = None;
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
    }

    fn options(&self) -> PasswordOptions {
        PasswordOptions {
            include_uppercase: self.include_uppercase,
            include_lowercase: self.include_lowercase,
            include_digits: self.include_digits,
            include_special: self.include_special,
        }
    }
}

#[derive(Clone, Copy)]
struct PasswordOptions {
    include_uppercase: bool,
    include_lowercase: bool,
    include_digits: bool,
    include_special: bool,
}

fn generate_passwords(
    options: &PasswordOptions,
    length: usize,
    count: usize,
) -> Result<Vec<String>, String> {
    let alphabet = alphabet(options)?;
    let mut passwords = Vec::with_capacity(count);
    for _ in 0..count {
        passwords.push(generate_password(&alphabet, length)?);
    }
    Ok(passwords)
}

fn generate_password(alphabet: &[u8], length: usize) -> Result<String, String> {
    let mut password = String::with_capacity(length);
    while password.len() < length {
        let index = random_index(alphabet.len())?;
        password.push(alphabet[index] as char);
    }
    Ok(password)
}

fn alphabet(options: &PasswordOptions) -> Result<Vec<u8>, String> {
    let mut alphabet = Vec::new();
    if options.include_uppercase {
        alphabet.extend_from_slice(UPPERCASE);
    }
    if options.include_lowercase {
        alphabet.extend_from_slice(LOWERCASE);
    }
    if options.include_digits {
        alphabet.extend_from_slice(DIGITS);
    }
    if options.include_special {
        alphabet.extend_from_slice(SPECIAL);
    }

    if alphabet.is_empty() {
        Err("Select at least one character set.".to_owned())
    } else {
        Ok(alphabet)
    }
}

fn random_index(alphabet_len: usize) -> Result<usize, String> {
    let acceptance_zone = 256 - (256 % alphabet_len);
    loop {
        let mut byte = [0_u8; 1];
        getrandom::fill(&mut byte).map_err(|error| format!("System random API failed: {error}"))?;
        let value = usize::from(byte[0]);
        if value < acceptance_zone {
            return Ok(value % alphabet_len);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_use_letters_and_digits() {
        let alphabet = alphabet(&PasswordOptions {
            include_uppercase: true,
            include_lowercase: true,
            include_digits: true,
            include_special: false,
        })
        .unwrap();

        assert!(alphabet.contains(&b'A'));
        assert!(alphabet.contains(&b'a'));
        assert!(alphabet.contains(&b'0'));
        assert!(!alphabet.contains(&b'!'));
    }

    #[test]
    fn rejects_empty_alphabet() {
        assert!(
            alphabet(&PasswordOptions {
                include_uppercase: false,
                include_lowercase: false,
                include_digits: false,
                include_special: false,
            })
            .is_err()
        );
    }

    #[test]
    fn generates_requested_count_and_length() {
        let passwords = generate_passwords(
            &PasswordOptions {
                include_uppercase: true,
                include_lowercase: false,
                include_digits: false,
                include_special: false,
            },
            12,
            3,
        )
        .unwrap();

        assert_eq!(passwords.len(), 3);
        assert!(passwords.iter().all(|password| password.len() == 12));
        assert!(
            passwords
                .iter()
                .all(|password| password.bytes().all(|byte| UPPERCASE.contains(&byte)))
        );
    }
}
