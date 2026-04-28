use std::ops::Range;

use eframe::egui::{
    Align, Color32, FontId, Frame, Layout, Margin, RichText, ScrollArea, TextBuffer, TextEdit,
    TextStyle, Ui,
    text::{LayoutJob, TextFormat},
};
use regex::{Regex, RegexBuilder};

use crate::ui::panel;

pub(crate) struct RegexTool {
    pattern: String,
    test_text: String,
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_new_line: bool,
}

impl Default for RegexTool {
    fn default() -> Self {
        Self {
            pattern: r"\b\w+@\w+\.\w+\b".to_owned(),
            test_text: "Send logs to dev@example.com and security@example.org.\nInvalid: dev@local"
                .to_owned(),
            case_insensitive: false,
            multi_line: true,
            dot_matches_new_line: false,
        }
    }
}

impl RegexTool {
    fn build_regex(&self) -> Result<Regex, regex::Error> {
        RegexBuilder::new(&self.pattern)
            .case_insensitive(self.case_insensitive)
            .multi_line(self.multi_line)
            .dot_matches_new_line(self.dot_matches_new_line)
            .build()
    }

    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        let result = self.build_regex();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.55).max(420.0));
                panel(ui, "Pattern", |ui| {
                    ui.add(
                        TextEdit::singleline(&mut self.pattern)
                            .font(TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .hint_text("Enter a Rust regex pattern"),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.case_insensitive, "Case insensitive");
                        ui.checkbox(&mut self.multi_line, "Multi-line");
                        ui.checkbox(&mut self.dot_matches_new_line, "Dot matches newline");
                    });

                    if let Err(error) = &result {
                        ui.add_space(8.0);
                        ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                    }
                });

                ui.add_space(14.0);
                panel(ui, "Test Text", |ui| {
                    let highlight_regex = result.as_ref().ok().cloned();
                    let mut layouter = move |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
                        let font_id = TextStyle::Monospace.resolve(ui.style());
                        let visuals = ui.visuals();
                        let job = highlighted_text_job(
                            text.as_str(),
                            highlight_regex.as_ref(),
                            font_id,
                            visuals.text_color(),
                            visuals.dark_mode,
                            wrap_width,
                        );
                        ui.fonts_mut(|fonts| fonts.layout_job(job))
                    };

                    ui.add(
                        TextEdit::multiline(&mut self.test_text)
                            .font(TextStyle::Monospace)
                            .desired_rows(22)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter)
                            .hint_text("Paste text to test against the expression"),
                    );
                });
            });

            ui.add_space(14.0);
            ui.vertical(|ui| {
                panel(ui, "Matches", |ui| {
                    self.match_list(ui, result.as_ref().ok())
                });
            });
        });
    }

    fn match_list(&self, ui: &mut Ui, regex: Option<&Regex>) {
        let Some(regex) = regex else {
            ui.label(
                RichText::new("Fix the pattern to see matches.")
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        };

        let captures: Vec<_> = regex.captures_iter(&self.test_text).collect();
        ui.horizontal(|ui| {
            ui.label(RichText::new(captures.len().to_string()).heading());
            ui.label(if captures.len() == 1 {
                "match found"
            } else {
                "matches found"
            });
        });
        ui.separator();

        if captures.is_empty() {
            ui.label(RichText::new("No matches.").color(ui.visuals().weak_text_color()));
            return;
        }

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, captures) in captures.iter().enumerate() {
                    let Some(mat) = captures.get(0) else {
                        continue;
                    };

                    Frame::group(ui.style())
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("#{}", index + 1)).strong());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{}..{}", mat.start(), mat.end()))
                                            .monospace()
                                            .color(ui.visuals().weak_text_color()),
                                    );
                                });
                            });
                            ui.add_space(4.0);
                            ui.label(RichText::new(mat.as_str()).monospace());

                            let groups: Vec<_> = captures
                                .iter()
                                .enumerate()
                                .skip(1)
                                .filter_map(|(group_index, group)| {
                                    group.map(|group| (group_index, group))
                                })
                                .collect();

                            if !groups.is_empty() {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        groups.len(),
                                        if groups.len() == 1 { "group" } else { "groups" }
                                    ))
                                    .color(ui.visuals().weak_text_color()),
                                );

                                for (group_index, group) in groups {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("${group_index}"))
                                                .monospace()
                                                .color(group_color(group_index)),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{}..{}",
                                                group.start(),
                                                group.end()
                                            ))
                                            .monospace()
                                            .color(ui.visuals().weak_text_color()),
                                        );
                                        ui.label(RichText::new(group.as_str()).monospace());
                                    });
                                }
                            }
                        });
                    ui.add_space(8.0);
                }
            });
    }
}

struct HighlightRange {
    range: Range<usize>,
    background: Color32,
    priority: usize,
}

fn highlighted_text_job(
    text: &str,
    regex: Option<&Regex>,
    font_id: FontId,
    text_color: Color32,
    dark_mode: bool,
    wrap_width: f32,
) -> LayoutJob {
    let mut ranges = Vec::new();

    if let Some(regex) = regex {
        for captures in regex.captures_iter(text) {
            if let Some(mat) = captures.get(0) {
                push_highlight(
                    &mut ranges,
                    mat.start()..mat.end(),
                    match_background(dark_mode),
                    1,
                );
            }

            for (group_index, group) in captures.iter().enumerate().skip(1) {
                if let Some(group) = group {
                    push_highlight(
                        &mut ranges,
                        group.start()..group.end(),
                        group_background(group_index, dark_mode),
                        10 + group_index,
                    );
                }
            }
        }
    }

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    if ranges.is_empty() {
        job.append(
            text,
            0.0,
            TextFormat {
                font_id,
                color: text_color,
                ..Default::default()
            },
        );
        return job;
    }

    let mut boundaries = Vec::with_capacity(ranges.len() * 2 + 2);
    boundaries.push(0);
    boundaries.push(text.len());
    for range in &ranges {
        boundaries.push(range.range.start);
        boundaries.push(range.range.end);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    for window in boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        if start == end {
            continue;
        }

        let background = ranges
            .iter()
            .filter(|range| range.range.start <= start && end <= range.range.end)
            .max_by_key(|range| range.priority)
            .map_or(Color32::TRANSPARENT, |range| range.background);

        job.append(
            &text[start..end],
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: text_color,
                background,
                ..Default::default()
            },
        );
    }

    job
}

fn push_highlight(
    ranges: &mut Vec<HighlightRange>,
    range: Range<usize>,
    background: Color32,
    priority: usize,
) {
    if range.start < range.end {
        ranges.push(HighlightRange {
            range,
            background,
            priority,
        });
    }
}

fn group_color(group_index: usize) -> Color32 {
    const COLORS: [Color32; 5] = [
        Color32::from_rgb(22, 112, 196),
        Color32::from_rgb(142, 78, 198),
        Color32::from_rgb(31, 132, 90),
        Color32::from_rgb(192, 86, 33),
        Color32::from_rgb(188, 57, 83),
    ];
    COLORS[(group_index - 1) % COLORS.len()]
}

fn match_background(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(54, 42, 12)
    } else {
        Color32::from_rgb(255, 242, 178)
    }
}

fn group_background(group_index: usize, dark_mode: bool) -> Color32 {
    const LIGHT_COLORS: [Color32; 5] = [
        Color32::from_rgb(205, 231, 255),
        Color32::from_rgb(231, 218, 255),
        Color32::from_rgb(207, 239, 222),
        Color32::from_rgb(255, 225, 201),
        Color32::from_rgb(255, 216, 225),
    ];
    const DARK_COLORS: [Color32; 5] = [
        Color32::from_rgb(14, 40, 62),
        Color32::from_rgb(42, 28, 66),
        Color32::from_rgb(16, 48, 30),
        Color32::from_rgb(56, 30, 12),
        Color32::from_rgb(56, 22, 34),
    ];
    let colors = if dark_mode { DARK_COLORS } else { LIGHT_COLORS };
    colors[(group_index - 1) % colors.len()]
}
