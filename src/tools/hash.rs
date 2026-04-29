use eframe::egui::{
    self, Align, Frame, Layout, Margin, RichText, ScrollArea, TextEdit, TextStyle, Ui,
};
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::ui::panel;

pub(crate) struct HashTool {
    algorithm: HashAlgorithm,
    input_text: String,
    output_text: String,
    compare_text: String,
    selected_file_path: Option<String>,
    selected_file_bytes: Vec<u8>,
    selected_file_revision: u64,
    file_error: Option<String>,
    #[cfg(target_arch = "wasm32")]
    file_receiver: Option<std::sync::mpsc::Receiver<WebFileSelection>>,
    #[cfg(target_arch = "wasm32")]
    file_picker_pending: bool,
    cached_algorithm: HashAlgorithm,
    cached_input: CachedInput,
}

impl Default for HashTool {
    fn default() -> Self {
        let algorithm = HashAlgorithm::Sha256;
        let input_text = "Hello, RustKnife!".to_owned();
        let output_text = hash_bytes(algorithm, input_text.as_bytes());

        Self {
            algorithm,
            input_text: input_text.clone(),
            output_text,
            compare_text: String::new(),
            selected_file_path: None,
            selected_file_bytes: Vec::new(),
            selected_file_revision: 0,
            file_error: None,
            #[cfg(target_arch = "wasm32")]
            file_receiver: None,
            #[cfg(target_arch = "wasm32")]
            file_picker_pending: false,
            cached_algorithm: algorithm,
            cached_input: CachedInput::Text(input_text),
        }
    }
}

impl HashTool {
    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        self.poll_file_selection();
        self.refresh_output();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::group(ui.style())
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label("Algorithm");
                            algorithm_picker(ui, &mut self.algorithm);
                        });
                    });

                ui.add_space(14.0);
                panel(ui, "Input", |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() * 0.68).max(420.0));
                            ui.add(
                                TextEdit::multiline(&mut self.input_text)
                                    .font(TextStyle::Monospace)
                                    .desired_rows(12)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Type text to hash"),
                            );
                        });

                        ui.add_space(14.0);
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width());
                            file_picker(ui, self);
                        });
                    });
                });

                self.refresh_output();

                ui.add_space(14.0);
                panel(ui, "Output Text", |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.output_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} byte(s) hashed from {} with {}.",
                                self.input_byte_count(),
                                self.input_source_label(),
                                self.algorithm.label()
                            ))
                            .color(ui.visuals().weak_text_color()),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Copy").clicked() {
                                ui.copy_text(self.output_text.clone());
                            }
                        });
                    });
                });

                ui.add_space(14.0);
                panel(ui, "Compare Text", |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.compare_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY)
                            .hint_text("Paste a hash to compare"),
                    );
                    ui.add_space(8.0);
                    match compare_hashes(&self.output_text, &self.compare_text) {
                        CompareResult::Empty => {
                            ui.label(
                                RichText::new("Paste a hash to compare.")
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }
                        CompareResult::Match => {
                            ui.colored_label(ui.visuals().selection.bg_fill, "Hash matches.");
                        }
                        CompareResult::Mismatch => {
                            ui.colored_label(ui.visuals().error_fg_color, "Hash does not match.");
                        }
                    }
                });
            });
    }

    fn refresh_output(&mut self) {
        let input = self.current_input();
        if self.algorithm != self.cached_algorithm || input != self.cached_input {
            self.output_text = match input {
                CachedInput::Text(_) => hash_bytes(self.algorithm, self.input_text.as_bytes()),
                CachedInput::File(..) => hash_bytes(self.algorithm, &self.selected_file_bytes),
            };
            self.cached_algorithm = self.algorithm;
            self.cached_input = input;
        }
    }

    fn current_input(&self) -> CachedInput {
        if self.selected_file_path.is_some() && self.file_error.is_none() {
            CachedInput::File(self.selected_file_bytes.len(), self.selected_file_revision)
        } else {
            CachedInput::Text(self.input_text.clone())
        }
    }

    fn input_byte_count(&self) -> usize {
        match self.current_input() {
            CachedInput::Text(_) => self.input_text.len(),
            CachedInput::File(..) => self.selected_file_bytes.len(),
        }
    }

    fn input_source_label(&self) -> &'static str {
        match self.current_input() {
            CachedInput::Text(_) => "text",
            CachedInput::File(..) => "file",
        }
    }

    fn clear_file(&mut self) {
        self.selected_file_path = None;
        self.selected_file_bytes.clear();
        self.selected_file_revision = self.selected_file_revision.wrapping_add(1);
        self.file_error = None;
        #[cfg(target_arch = "wasm32")]
        {
            self.file_receiver = None;
            self.file_picker_pending = false;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn select_file(&mut self) {
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            return;
        };

        let display_path = path.display().to_string();
        match std::fs::read(&path) {
            Ok(bytes) => {
                self.selected_file_path = Some(display_path);
                self.selected_file_bytes = bytes;
                self.selected_file_revision = self.selected_file_revision.wrapping_add(1);
                self.file_error = None;
            }
            Err(error) => {
                self.selected_file_path = Some(display_path);
                self.selected_file_bytes.clear();
                self.selected_file_revision = self.selected_file_revision.wrapping_add(1);
                self.file_error = Some(format!("Could not read file: {error}"));
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn select_file(&mut self, ctx: egui::Context) {
        if self.file_picker_pending {
            return;
        }

        let (sender, receiver) = std::sync::mpsc::channel();
        self.file_receiver = Some(receiver);
        self.file_picker_pending = true;
        self.file_error = None;

        wasm_bindgen_futures::spawn_local(async move {
            let selection = match rfd::AsyncFileDialog::new().pick_file().await {
                Some(file) => {
                    let file_name = file.file_name();
                    let bytes = file.read().await;
                    WebFileSelection::Selected { file_name, bytes }
                }
                None => WebFileSelection::Canceled,
            };

            let _ = sender.send(selection);
            ctx.request_repaint();
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn poll_file_selection(&mut self) {}

    #[cfg(target_arch = "wasm32")]
    fn poll_file_selection(&mut self) {
        let Some(receiver) = self.file_receiver.take() else {
            return;
        };

        match receiver.try_recv() {
            Ok(WebFileSelection::Selected { file_name, bytes }) => {
                self.selected_file_path = Some(file_name);
                self.selected_file_bytes = bytes;
                self.selected_file_revision = self.selected_file_revision.wrapping_add(1);
                self.file_error = None;
                self.file_picker_pending = false;
            }
            Ok(WebFileSelection::Canceled) => {
                self.file_picker_pending = false;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                self.file_receiver = Some(receiver);
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.file_picker_pending = false;
                self.file_error = Some("File selection did not complete.".to_owned());
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
enum WebFileSelection {
    Selected { file_name: String, bytes: Vec<u8> },
    Canceled,
}

#[derive(Clone, PartialEq, Eq)]
enum CachedInput {
    Text(String),
    File(usize, u64),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HashAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Blake3,
    Crc32,
}

impl HashAlgorithm {
    const ALL: [Self; 7] = [
        Self::Md5,
        Self::Sha1,
        Self::Sha256,
        Self::Sha384,
        Self::Sha512,
        Self::Blake3,
        Self::Crc32,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
            Self::Blake3 => "BLAKE3",
            Self::Crc32 => "CRC32",
        }
    }
}

fn algorithm_picker(ui: &mut Ui, selected_algorithm: &mut HashAlgorithm) {
    egui::ComboBox::from_id_salt("hash-algorithm")
        .selected_text(selected_algorithm.label())
        .show_ui(ui, |ui| {
            for algorithm in HashAlgorithm::ALL {
                ui.selectable_value(selected_algorithm, algorithm, algorithm.label());
            }
        });
}

fn file_picker(ui: &mut Ui, tool: &mut HashTool) {
    #[cfg(not(target_arch = "wasm32"))]
    if ui.button("Select File").clicked() {
        tool.select_file();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let label = if tool.file_picker_pending {
            "Selecting..."
        } else {
            "Select File"
        };
        if ui
            .add_enabled(!tool.file_picker_pending, egui::Button::new(label))
            .clicked()
        {
            tool.select_file(ui.ctx().clone());
        }
    }

    ui.add_space(8.0);
    if let Some(path) = &tool.selected_file_path {
        ui.label(RichText::new("Selected file").strong());
        ui.label(RichText::new(path).monospace());
        if tool.file_error.is_none() {
            ui.label(
                RichText::new(format!("{} byte(s)", tool.selected_file_bytes.len()))
                    .color(ui.visuals().weak_text_color()),
            );
        }
        if ui.button("Clear File").clicked() {
            tool.clear_file();
        }
    } else {
        ui.label(RichText::new("No file selected.").color(ui.visuals().weak_text_color()));
    }

    if let Some(error) = &tool.file_error {
        ui.add_space(8.0);
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
}

fn hash_bytes(algorithm: HashAlgorithm, bytes: &[u8]) -> String {
    match algorithm {
        HashAlgorithm::Md5 => digest_to_hex(Md5::digest(bytes)),
        HashAlgorithm::Sha1 => digest_to_hex(Sha1::digest(bytes)),
        HashAlgorithm::Sha256 => digest_to_hex(Sha256::digest(bytes)),
        HashAlgorithm::Sha384 => digest_to_hex(Sha384::digest(bytes)),
        HashAlgorithm::Sha512 => digest_to_hex(Sha512::digest(bytes)),
        HashAlgorithm::Blake3 => blake3::hash(bytes).to_hex().to_string(),
        HashAlgorithm::Crc32 => format!("{:08x}", crc32fast::hash(bytes)),
    }
}

fn digest_to_hex(digest: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = digest.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompareResult {
    Empty,
    Match,
    Mismatch,
}

fn compare_hashes(output_text: &str, compare_text: &str) -> CompareResult {
    let normalized_compare = normalize_hash(compare_text);
    if normalized_compare.is_empty() {
        return CompareResult::Empty;
    }

    if normalize_hash(output_text) == normalized_compare {
        CompareResult::Match
    } else {
        CompareResult::Mismatch
    }
}

fn normalize_hash(hash: &str) -> String {
    hash.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_with_md5() {
        assert_eq!(
            hash_bytes(HashAlgorithm::Md5, b"abc"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    #[test]
    fn hashes_with_sha1() {
        assert_eq!(
            hash_bytes(HashAlgorithm::Sha1, b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    #[test]
    fn hashes_with_sha256() {
        assert_eq!(
            hash_bytes(HashAlgorithm::Sha256, b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hashes_with_crc32() {
        assert_eq!(hash_bytes(HashAlgorithm::Crc32, b"abc"), "352441c2");
    }

    #[test]
    fn compares_hashes_without_case_or_whitespace_sensitivity() {
        assert_eq!(
            compare_hashes(
                "900150983cd24fb0d6963f7d28e17f72",
                "90015098 3CD24FB0D6963F7D28E17F72"
            ),
            CompareResult::Match
        );
    }

    #[test]
    fn reports_empty_compare_text() {
        assert_eq!(
            compare_hashes("900150983cd24fb0d6963f7d28e17f72", " \n\t"),
            CompareResult::Empty
        );
    }
}
