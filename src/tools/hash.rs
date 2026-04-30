use eframe::egui::{
    self, Align, Frame, Layout, Margin, RichText, ScrollArea, TextEdit, TextStyle, Ui,
};
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::time::{Duration, Instant};

#[cfg(target_arch = "wasm32")]
use std::{cell::Cell, rc::Rc, sync::mpsc};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

use crate::ui::panel;

const HASHING_CANCELED: &str = "Hashing canceled.";
#[cfg(not(target_arch = "wasm32"))]
const NATIVE_HASH_CHUNK_SIZE: usize = 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const NATIVE_BLAKE3_HASH_CHUNK_SIZE: usize = 1024 * 1024;
#[cfg(target_arch = "wasm32")]
const WASM_HASH_CHUNK_SIZE: u64 = 1024 * 1024;

pub(crate) struct HashTool {
    algorithm: HashAlgorithm,
    input_text: String,
    output_text: String,
    compare_text: String,
    selected_file_path: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    selected_file_fs_path: Option<PathBuf>,
    #[cfg(target_arch = "wasm32")]
    selected_web_file: Option<rfd::FileHandle>,
    selected_file_size: Option<u64>,
    hash_receiver: Option<mpsc::Receiver<HashWorkerMessage>>,
    hash_progress: Option<HashProgress>,
    hash_in_progress: bool,
    hash_cancel_flag: Option<HashCancelFlag>,
    hash_started_at: Option<Instant>,
    output_duration: Option<Duration>,
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
        let input_text = "Hello, CrabKnife!".to_owned();
        let output_text = hash_bytes(algorithm, input_text.as_bytes());

        Self {
            algorithm,
            input_text: input_text.clone(),
            output_text,
            compare_text: String::new(),
            selected_file_path: None,
            #[cfg(not(target_arch = "wasm32"))]
            selected_file_fs_path: None,
            #[cfg(target_arch = "wasm32")]
            selected_web_file: None,
            selected_file_size: None,
            hash_receiver: None,
            hash_progress: None,
            hash_in_progress: false,
            hash_cancel_flag: None,
            hash_started_at: None,
            output_duration: None,
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
        self.poll_hash_progress();
        self.poll_file_selection();
        self.refresh_output();

        if self.hash_in_progress {
            ui.ctx().request_repaint();
        }

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::group(ui.style())
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label("Algorithm");
                            ui.add_enabled_ui(!self.hash_in_progress, |ui| {
                                algorithm_picker(ui, &mut self.algorithm);
                            });
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
                        if let Some(progress) = self.hash_progress {
                            let bar = if progress.total_bytes > 0 {
                                egui::ProgressBar::new(
                                    progress.processed_bytes as f32 / progress.total_bytes as f32,
                                )
                            } else {
                                egui::ProgressBar::new(0.0).animate(true)
                            };
                            let progress_width = (ui.available_width() * 0.5).max(220.0);
                            ui.add_sized(
                                [progress_width, ui.spacing().interact_size.y],
                                bar.show_percentage().text(format!(
                                    "Hashing file: {} / {}",
                                    format_byte_count(progress.processed_bytes),
                                    format_byte_count(progress.total_bytes)
                                )),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "{} hashed from {} with {}{}.",
                                    format_byte_count(self.input_byte_count() as u64),
                                    self.input_source_label(),
                                    self.algorithm.label(),
                                    self.output_duration
                                        .map(|duration| format!(
                                            " in {}",
                                            format_duration(duration)
                                        ))
                                        .unwrap_or_default()
                                ))
                                .color(ui.visuals().weak_text_color()),
                            );
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add_enabled(!self.hash_in_progress, egui::Button::new("Copy"))
                                .clicked()
                            {
                                ui.copy_text(self.output_text.clone());
                            }
                            if self.hash_in_progress && ui.button("Cancel").clicked() {
                                self.cancel_file_hash();
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
            match input {
                CachedInput::Text(_) => {
                    self.cancel_file_hash();
                    let started_at = Instant::now();
                    self.output_text = hash_bytes(self.algorithm, self.input_text.as_bytes());
                    self.output_duration = Some(started_at.elapsed());
                }
                CachedInput::File(_) => {
                    self.start_file_hash();
                }
            }
            self.cached_algorithm = self.algorithm;
            self.cached_input = input;
        }
    }

    fn current_input(&self) -> CachedInput {
        if self.selected_file_path.is_some() {
            CachedInput::File(self.selected_file_revision)
        } else {
            CachedInput::Text(self.input_text.clone())
        }
    }

    fn input_byte_count(&self) -> usize {
        match self.current_input() {
            CachedInput::Text(_) => self.input_text.len(),
            CachedInput::File(_) => self.selected_file_byte_count(),
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.selected_file_fs_path = None;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.selected_web_file = None;
        }
        self.selected_file_size = None;
        self.cancel_file_hash();
        self.selected_file_revision = self.selected_file_revision.wrapping_add(1);
        self.file_error = None;
        #[cfg(target_arch = "wasm32")]
        {
            self.file_receiver = None;
            self.file_picker_pending = false;
        }
    }

    fn cancel_file_hash(&mut self) {
        if let Some(cancel_flag) = &self.hash_cancel_flag {
            cancel_flag.cancel();
        }
        self.hash_receiver = None;
        self.hash_progress = None;
        self.hash_in_progress = false;
        self.hash_cancel_flag = None;
        self.hash_started_at = None;
    }

    fn poll_hash_progress(&mut self) {
        let Some(receiver) = self.hash_receiver.take() else {
            return;
        };

        let mut keep_receiver = true;
        while let Ok(message) = receiver.try_recv() {
            match message {
                HashWorkerMessage::Progress(progress) => {
                    self.hash_progress = Some(progress);
                }
                HashWorkerMessage::Finished(result) => {
                    self.hash_in_progress = false;
                    self.hash_progress = None;
                    self.hash_cancel_flag = None;
                    self.output_duration = self
                        .hash_started_at
                        .take()
                        .map(|started_at| started_at.elapsed());
                    match result {
                        Ok(output) => {
                            self.output_text = output;
                            self.file_error = None;
                        }
                        Err(error) => {
                            self.output_text.clear();
                            self.output_duration = None;
                            self.file_error = if error == HASHING_CANCELED {
                                None
                            } else {
                                Some(error)
                            };
                        }
                    }
                    keep_receiver = false;
                    break;
                }
            }
        }

        if keep_receiver {
            self.hash_receiver = Some(receiver);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn select_file(&mut self) {
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            return;
        };

        self.cancel_file_hash();
        self.selected_file_path = Some(path.display().to_string());
        self.selected_file_size = std::fs::metadata(&path).ok().map(|metadata| metadata.len());
        self.selected_file_fs_path = Some(path);
        self.selected_file_revision = self.selected_file_revision.wrapping_add(1);
        self.file_error = None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_file_hash(&mut self) {
        let Some(path) = self.selected_file_fs_path.clone() else {
            self.output_text.clear();
            self.file_error = Some("Selected file is not available.".to_owned());
            return;
        };

        self.cancel_file_hash();
        let (sender, receiver) = mpsc::channel();
        let algorithm = self.algorithm;
        let cancel_flag = HashCancelFlag::new();
        self.output_text.clear();
        self.file_error = None;
        self.hash_progress = None;
        self.hash_in_progress = true;
        self.hash_receiver = Some(receiver);
        self.hash_cancel_flag = Some(cancel_flag.clone());
        self.hash_started_at = Some(Instant::now());
        self.output_duration = None;

        std::thread::spawn(move || {
            let result = hash_file(&path, algorithm, &sender, &cancel_flag);
            let _ = sender.send(HashWorkerMessage::Finished(result));
        });
    }

    fn selected_file_byte_count(&self) -> usize {
        self.selected_file_size
            .unwrap_or(
                self.hash_progress
                    .map_or(0, |progress| progress.total_bytes),
            )
            .try_into()
            .unwrap_or(usize::MAX)
    }

    #[cfg(target_arch = "wasm32")]
    fn select_file(&mut self, ctx: egui::Context) {
        if self.file_picker_pending {
            return;
        }

        self.cancel_file_hash();
        let (sender, receiver) = std::sync::mpsc::channel();
        self.file_receiver = Some(receiver);
        self.file_picker_pending = true;
        self.file_error = None;

        wasm_bindgen_futures::spawn_local(async move {
            let selection = match rfd::AsyncFileDialog::new().pick_file().await {
                Some(file) => WebFileSelection::Selected {
                    file_name: file.file_name(),
                    file_size: file.inner().size() as u64,
                    file,
                },
                None => WebFileSelection::Canceled,
            };

            let _ = sender.send(selection);
            ctx.request_repaint();
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn start_file_hash(&mut self) {
        let Some(file_handle) = self.selected_web_file.clone() else {
            self.output_text.clear();
            self.file_error = Some("Selected file is not available.".to_owned());
            return;
        };

        self.cancel_file_hash();
        let (sender, receiver) = mpsc::channel();
        let algorithm = self.algorithm;
        let cancel_flag = HashCancelFlag::new();
        self.output_text.clear();
        self.file_error = None;
        self.hash_progress = None;
        self.hash_in_progress = true;
        self.hash_receiver = Some(receiver);
        self.hash_cancel_flag = Some(cancel_flag.clone());
        self.hash_started_at = Some(Instant::now());
        self.output_duration = None;

        wasm_bindgen_futures::spawn_local(async move {
            let result = hash_file(&file_handle, algorithm, &sender, &cancel_flag).await;
            let _ = sender.send(HashWorkerMessage::Finished(result));
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
            Ok(WebFileSelection::Selected {
                file_name,
                file_size,
                file,
            }) => {
                self.selected_file_path = Some(file_name);
                self.selected_web_file = Some(file);
                self.selected_file_size = Some(file_size);
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
    Selected {
        file_name: String,
        file_size: u64,
        file: rfd::FileHandle,
    },
    Canceled,
}

#[derive(Clone, PartialEq, Eq)]
enum CachedInput {
    Text(String),
    File(u64),
}

#[derive(Clone, Copy)]
struct HashProgress {
    processed_bytes: u64,
    total_bytes: u64,
}

enum HashWorkerMessage {
    Progress(HashProgress),
    Finished(Result<String, String>),
}

#[derive(Clone)]
struct HashCancelFlag {
    #[cfg(not(target_arch = "wasm32"))]
    inner: Arc<AtomicBool>,
    #[cfg(target_arch = "wasm32")]
    inner: Rc<Cell<bool>>,
}

impl HashCancelFlag {
    fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            inner: Arc::new(AtomicBool::new(false)),
            #[cfg(target_arch = "wasm32")]
            inner: Rc::new(Cell::new(false)),
        }
    }

    fn cancel(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.inner.store(true, Ordering::Relaxed);
        #[cfg(target_arch = "wasm32")]
        self.inner.set(true);
    }

    fn is_canceled(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.load(Ordering::Relaxed)
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.get()
        }
    }
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
                RichText::new(format!("{} byte(s)", tool.selected_file_byte_count()))
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

#[cfg(not(target_arch = "wasm32"))]
fn native_hash_chunk_size(algorithm: HashAlgorithm) -> usize {
    match algorithm {
        HashAlgorithm::Blake3 => NATIVE_BLAKE3_HASH_CHUNK_SIZE,
        _ => NATIVE_HASH_CHUNK_SIZE,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn hash_file(
    path: &std::path::Path,
    algorithm: HashAlgorithm,
    sender: &mpsc::Sender<HashWorkerMessage>,
    cancel_flag: &HashCancelFlag,
) -> Result<String, String> {
    let file = File::open(path).map_err(|error| format!("Could not read file: {error}"))?;
    let total_bytes = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    let mut reader = BufReader::new(file);
    let mut hasher = StreamingHasher::new(algorithm);
    let mut processed_bytes = 0_u64;
    let mut buffer = vec![0_u8; native_hash_chunk_size(algorithm)];

    loop {
        if cancel_flag.is_canceled() {
            return Err(HASHING_CANCELED.to_owned());
        }

        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Could not read file: {error}"))?;
        if read == 0 {
            break;
        }

        hasher.update_native_file_chunk(&buffer[..read]);
        processed_bytes += read as u64;
        let _ = sender.send(HashWorkerMessage::Progress(HashProgress {
            processed_bytes,
            total_bytes,
        }));
    }

    Ok(hasher.finalize())
}

#[cfg(target_arch = "wasm32")]
async fn hash_file(
    file_handle: &rfd::FileHandle,
    algorithm: HashAlgorithm,
    sender: &mpsc::Sender<HashWorkerMessage>,
    cancel_flag: &HashCancelFlag,
) -> Result<String, String> {
    let file = file_handle.inner().clone();
    let total_bytes = file.size() as u64;
    let mut hasher = StreamingHasher::new(algorithm);
    let mut processed_bytes = 0_u64;
    let chunk_size = WASM_HASH_CHUNK_SIZE;

    while processed_bytes < total_bytes {
        if cancel_flag.is_canceled() {
            return Err(HASHING_CANCELED.to_owned());
        }

        let end = (processed_bytes + chunk_size).min(total_bytes);
        let chunk = file
            .slice_with_f64_and_f64(processed_bytes as f64, end as f64)
            .map_err(|error| format!("Could not read file: {error:?}"))?;
        let bytes = JsFuture::from(chunk.array_buffer())
            .await
            .map_err(|error| format!("Could not read file: {error:?}"))?;
        let buffer = js_sys::Uint8Array::new(&bytes).to_vec();

        hasher.update(&buffer);
        processed_bytes = end;
        let _ = sender.send(HashWorkerMessage::Progress(HashProgress {
            processed_bytes,
            total_bytes,
        }));
    }

    Ok(hasher.finalize())
}

enum StreamingHasher {
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
    Blake3(blake3::Hasher),
    Crc32(crc32fast::Hasher),
}

impl StreamingHasher {
    fn new(algorithm: HashAlgorithm) -> Self {
        match algorithm {
            HashAlgorithm::Md5 => Self::Md5(Md5::new()),
            HashAlgorithm::Sha1 => Self::Sha1(Sha1::new()),
            HashAlgorithm::Sha256 => Self::Sha256(Sha256::new()),
            HashAlgorithm::Sha384 => Self::Sha384(Sha384::new()),
            HashAlgorithm::Sha512 => Self::Sha512(Sha512::new()),
            HashAlgorithm::Blake3 => Self::Blake3(blake3::Hasher::new()),
            HashAlgorithm::Crc32 => Self::Crc32(crc32fast::Hasher::new()),
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Md5(hasher) => hasher.update(bytes),
            Self::Sha1(hasher) => hasher.update(bytes),
            Self::Sha256(hasher) => hasher.update(bytes),
            Self::Sha384(hasher) => hasher.update(bytes),
            Self::Sha512(hasher) => hasher.update(bytes),
            Self::Blake3(hasher) => {
                hasher.update(bytes);
            }
            Self::Crc32(hasher) => hasher.update(bytes),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn update_native_file_chunk(&mut self, bytes: &[u8]) {
        match self {
            Self::Blake3(hasher) => {
                hasher.update_rayon(bytes);
            }
            _ => self.update(bytes),
        }
    }

    fn finalize(self) -> String {
        match self {
            Self::Md5(hasher) => digest_to_hex(hasher.finalize()),
            Self::Sha1(hasher) => digest_to_hex(hasher.finalize()),
            Self::Sha256(hasher) => digest_to_hex(hasher.finalize()),
            Self::Sha384(hasher) => digest_to_hex(hasher.finalize()),
            Self::Sha512(hasher) => digest_to_hex(hasher.finalize()),
            Self::Blake3(hasher) => hasher.finalize().to_hex().to_string(),
            Self::Crc32(hasher) => format!("{:08x}", hasher.finalize()),
        }
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

fn format_byte_count(byte_count: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut value = byte_count as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{byte_count} B")
    } else if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit_index])
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit_index])
    } else {
        format!("{value:.2} {}", UNITS[unit_index])
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();

    if seconds < 0.001 {
        format!("{:.0} us", seconds * 1_000_000.0)
    } else if seconds < 1.0 {
        format!("{:.1} ms", seconds * 1_000.0)
    } else if seconds < 10.0 {
        format!("{seconds:.2} s")
    } else {
        format!("{seconds:.1} s")
    }
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
    fn formats_byte_counts_for_display() {
        assert_eq!(format_byte_count(512), "512 B");
        assert_eq!(format_byte_count(1024), "1.00 KiB");
        assert_eq!(format_byte_count(10 * 1024 * 1024), "10.0 MiB");
        assert_eq!(format_byte_count(1024 * 1024 * 1024), "1.00 GiB");
    }

    #[test]
    fn formats_durations_for_display() {
        assert_eq!(format_duration(Duration::from_micros(500)), "500 us");
        assert_eq!(format_duration(Duration::from_millis(25)), "25.0 ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50 s");
        assert_eq!(format_duration(Duration::from_secs(12)), "12.0 s");
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
