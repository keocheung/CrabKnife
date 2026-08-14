use base64::{Engine as _, engine::general_purpose};
use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, StyledExt as _, button::Button, button::ButtonVariants as _, input::Input,
    input::InputState,
};
use md5::Md5;
use regex::Regex;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Regex,
    Hex,
    Base64,
    Hash,
    Radix,
    Float,
    Password,
    Qr,
}

impl Tool {
    const ALL: [Self; 8] = [
        Self::Regex,
        Self::Hex,
        Self::Base64,
        Self::Hash,
        Self::Radix,
        Self::Float,
        Self::Password,
        Self::Qr,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Regex => "Regex Tester",
            Self::Hex => "Hex to String",
            Self::Base64 => "Base64",
            Self::Hash => "Hash / Checksum",
            Self::Radix => "Radix Converter",
            Self::Float => "Float Converter",
            Self::Password => "Password Generator",
            Self::Qr => "QR Code",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::Regex => "Enter pattern on the first line and test text below it",
            Self::Hex => "Paste hexadecimal bytes (spaces and 0x prefixes are allowed)",
            Self::Base64 => "Enter plain text to encode, or prefix encoded data with decode:",
            Self::Hash => "Enter text to calculate SHA-256 and MD5 digests",
            Self::Radix => "Enter a decimal integer",
            Self::Float => "Enter a decimal floating-point number",
            Self::Password => "Enter the desired password length (8–128)",
            Self::Qr => "Enter the text to place in the QR code",
        }
    }
}

pub(crate) struct CrabKnife {
    active: Tool,
    input: Entity<InputState>,
    output: Entity<InputState>,
}

impl CrabKnife {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder(Tool::Regex.hint())
                .default_value("\\b(\\w+)@\\w+\\.\\w+\\b\nSend logs to dev@example.com")
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("Results appear here")
        });
        Self {
            active: Tool::Regex,
            input,
            output,
        }
    }

    fn run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.input.read(cx).value().to_string();
        let result = transform(self.active, &value);
        self.output
            .update(cx, |state, cx| state.set_value(result, window, cx));
        cx.notify();
    }
}

impl Render for CrabKnife {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = cx.entity();
        let sidebar = Tool::ALL.into_iter().enumerate().fold(
            div()
                .v_flex()
                .gap_1()
                .p_4()
                .w(px(224.))
                .h_full()
                .border_r_1()
                .border_color(theme.border),
            |column, (index, tool)| {
                let selected = self.active == tool;
                let entity = entity.clone();
                column.child(
                    Button::new(("nav-tool", index))
                        .label(tool.label())
                        .w_full()
                        .map(|button| {
                            if selected {
                                button.primary()
                            } else {
                                button.ghost()
                            }
                        })
                        .on_click(move |_, _, cx: &mut App| {
                            entity.update(cx, |app, cx| {
                                app.active = tool;
                                cx.notify();
                            });
                        }),
                )
            },
        );

        let run_entity = entity.clone();
        div()
            .h_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                sidebar
                    .child(div().mt_4().text_xl().font_semibold().child("CrabKnife"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Developer tools · GPUI"),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(68.))
                            .px_6()
                            .items_center()
                            .border_b_1()
                            .border_color(theme.border)
                            .child(div().text_2xl().font_semibold().child(self.active.label())),
                    )
                    .child(
                        div()
                            .v_flex()
                            .gap_4()
                            .p_6()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(self.active.hint()),
                            )
                            .child(card("Input", Input::new(&self.input).h(px(230.))))
                            .child(
                                Button::new("run-tool")
                                    .primary()
                                    .label("Run conversion")
                                    .on_click(move |_, window, cx| {
                                        run_entity.update(cx, |app, cx| app.run(window, cx));
                                    }),
                            )
                            .child(card("Output", Input::new(&self.output).h(px(230.)))),
                    ),
            )
    }
}

fn card(title: &'static str, child: impl IntoElement) -> impl IntoElement {
    div()
        .v_flex()
        .gap_3()
        .p_4()
        .border_1()
        .rounded_lg()
        .child(div().font_semibold().child(title))
        .child(child)
}

fn transform(tool: Tool, text: &str) -> String {
    match tool {
        Tool::Regex => {
            let (pattern, haystack) = text.split_once('\n').unwrap_or((text, ""));
            match Regex::new(pattern) {
                Ok(regex) => {
                    let matches: Vec<_> = regex
                        .find_iter(haystack)
                        .enumerate()
                        .map(|(i, m)| {
                            format!("#{}  {}..{}  {}", i + 1, m.start(), m.end(), m.as_str())
                        })
                        .collect();
                    if matches.is_empty() {
                        "No matches.".into()
                    } else {
                        matches.join("\n")
                    }
                }
                Err(error) => format!("Invalid pattern: {error}"),
            }
        }
        Tool::Hex => {
            let clean: String = text
                .replace("0x", "")
                .chars()
                .filter(|c| c.is_ascii_hexdigit())
                .collect();
            let bytes: Result<Vec<_>, _> = (0..clean.len() / 2)
                .map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16))
                .collect();
            bytes
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .unwrap_or_else(|e| e.to_string())
        }
        Tool::Base64 => text.strip_prefix("decode:").map_or_else(
            || general_purpose::STANDARD.encode(text),
            |encoded| {
                general_purpose::STANDARD
                    .decode(encoded.trim())
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
                    .unwrap_or_else(|e| e.to_string())
            },
        ),
        Tool::Hash => format!(
            "SHA-256  {:x}\nMD5      {:x}",
            Sha256::digest(text.as_bytes()),
            Md5::digest(text.as_bytes())
        ),
        Tool::Radix => text
            .trim()
            .parse::<i128>()
            .map(|n| format!("Hex     {n:#x}\nOctal   {n:#o}\nBinary  {n:#b}"))
            .unwrap_or_else(|e| e.to_string()),
        Tool::Float => text
            .trim()
            .parse::<f64>()
            .map(|n| {
                format!(
                    "f64 bits  0x{:016X}\nBinary    {:064b}",
                    n.to_bits(),
                    n.to_bits()
                )
            })
            .unwrap_or_else(|e| e.to_string()),
        Tool::Password => {
            let len = text.trim().parse::<usize>().unwrap_or(24).clamp(8, 128);
            const ALPHABET: &[u8] =
                b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*";
            let mut bytes = vec![0_u8; len];
            if getrandom::fill(&mut bytes).is_err() {
                return "Unable to access secure randomness".into();
            }
            bytes
                .into_iter()
                .map(|b| ALPHABET[b as usize % ALPHABET.len()] as char)
                .collect()
        }
        Tool::Qr => match qrcode::QrCode::new(text.as_bytes()) {
            Ok(code) => format!(
                "QR code ready\n{} × {} modules\n{} bytes encoded",
                code.width(),
                code.width(),
                text.len()
            ),
            Err(error) => error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn base64_conversion() {
        assert_eq!(transform(Tool::Base64, "CrabKnife"), "Q3JhYktuaWZl");
    }
    #[test]
    fn hex_conversion() {
        assert_eq!(transform(Tool::Hex, "43 72 61 62"), "Crab");
    }
    #[test]
    fn regex_conversion() {
        assert!(transform(Tool::Regex, r"\d+\nabc 42").contains("42"));
    }
}
