use std::iter::Peekable;
use std::ops::Range;
use std::str::CharIndices;

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
    cached_pattern: String,
    cached_case_insensitive: bool,
    cached_multi_line: bool,
    cached_dot_matches_new_line: bool,
    cached_test_text: String,
    cached_regex: Option<Regex>,
    cached_error: Option<String>,
    cached_matches: Vec<MatchInfo>,
}

struct MatchInfo {
    text: String,
    start: usize,
    end: usize,
    groups: Vec<GroupInfo>,
}

struct GroupInfo {
    index: usize,
    text: String,
    start: usize,
    end: usize,
}

impl Default for RegexTool {
    fn default() -> Self {
        let pattern = r"\b(\w+)@\w+\.\w+\b".to_owned();
        let test_text =
            "Send logs to dev@example.com and security@example.org.\nInvalid: dev@local".to_owned();
        let case_insensitive = false;
        let multi_line = true;
        let dot_matches_new_line = false;

        let regex = RegexBuilder::new(&pattern)
            .case_insensitive(case_insensitive)
            .multi_line(multi_line)
            .dot_matches_new_line(dot_matches_new_line)
            .build()
            .ok();
        let matches = collect_matches(regex.as_ref(), &test_text);

        Self {
            cached_pattern: pattern.clone(),
            pattern,
            cached_case_insensitive: case_insensitive,
            case_insensitive,
            cached_multi_line: multi_line,
            multi_line,
            cached_dot_matches_new_line: dot_matches_new_line,
            dot_matches_new_line,
            cached_test_text: test_text.clone(),
            test_text,
            cached_error: None,
            cached_regex: regex,
            cached_matches: matches,
        }
    }
}

impl RegexTool {
    fn refresh_cache(&mut self) {
        let pattern_changed = self.pattern != self.cached_pattern
            || self.case_insensitive != self.cached_case_insensitive
            || self.multi_line != self.cached_multi_line
            || self.dot_matches_new_line != self.cached_dot_matches_new_line;

        if pattern_changed {
            self.cached_pattern = self.pattern.clone();
            self.cached_case_insensitive = self.case_insensitive;
            self.cached_multi_line = self.multi_line;
            self.cached_dot_matches_new_line = self.dot_matches_new_line;

            match RegexBuilder::new(&self.pattern)
                .case_insensitive(self.case_insensitive)
                .multi_line(self.multi_line)
                .dot_matches_new_line(self.dot_matches_new_line)
                .build()
            {
                Ok(regex) => {
                    self.cached_regex = Some(regex);
                    self.cached_error = None;
                }
                Err(error) => {
                    self.cached_regex = None;
                    self.cached_error = Some(error.to_string());
                }
            }
        }

        if pattern_changed || self.test_text != self.cached_test_text {
            self.cached_test_text = self.test_text.clone();
            self.cached_matches = collect_matches(self.cached_regex.as_ref(), &self.test_text);
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        self.refresh_cache();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.55).max(420.0));
                panel(ui, "Pattern", |ui| {
                    let mut pattern_layouter = |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
                        let font_id = TextStyle::Monospace.resolve(ui.style());
                        let dark_mode = ui.visuals().dark_mode;
                        let job = pattern_highlight_job(
                            text.as_str(),
                            font_id,
                            ui.visuals().text_color(),
                            dark_mode,
                            wrap_width,
                        );
                        ui.fonts_mut(|fonts| fonts.layout_job(job))
                    };
                    ui.add(
                        TextEdit::singleline(&mut self.pattern)
                            .font(TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut pattern_layouter)
                            .hint_text("Enter a Rust regex pattern"),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.case_insensitive, "Case insensitive");
                        ui.checkbox(&mut self.multi_line, "Multi-line");
                        ui.checkbox(&mut self.dot_matches_new_line, "Dot matches newline");
                    });

                    if let Some(error) = &self.cached_error {
                        ui.add_space(8.0);
                        ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                    }
                });

                ui.add_space(14.0);
                panel(ui, "Test Text", |ui| {
                    let matches = &self.cached_matches;
                    let mut layouter = |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
                        let font_id = TextStyle::Monospace.resolve(ui.style());
                        let visuals = ui.visuals();
                        let job = highlighted_text_job(
                            text.as_str(),
                            matches,
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
                panel(ui, "Matches", |ui| self.match_list(ui));
            });
        });
    }

    fn match_list(&self, ui: &mut Ui) {
        if self.cached_regex.is_none() {
            ui.label(
                RichText::new("Fix the pattern to see matches.")
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        }

        let matches = &self.cached_matches;
        ui.horizontal(|ui| {
            ui.label(RichText::new(matches.len().to_string()).heading());
            ui.label(if matches.len() == 1 {
                "match found"
            } else {
                "matches found"
            });
        });
        ui.separator();

        if matches.is_empty() {
            ui.label(RichText::new("No matches.").color(ui.visuals().weak_text_color()));
            return;
        }

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, mat) in matches.iter().enumerate() {
                    Frame::group(ui.style())
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("#{}", index + 1)).strong());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{}..{}", mat.start, mat.end))
                                            .monospace()
                                            .color(ui.visuals().weak_text_color()),
                                    );
                                });
                            });
                            ui.add_space(4.0);
                            ui.label(RichText::new(&mat.text).monospace());

                            if !mat.groups.is_empty() {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        mat.groups.len(),
                                        if mat.groups.len() == 1 {
                                            "group"
                                        } else {
                                            "groups"
                                        }
                                    ))
                                    .color(ui.visuals().weak_text_color()),
                                );

                                for group in &mat.groups {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("${}", group.index))
                                                .monospace()
                                                .color(group_color(group.index)),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{}..{}",
                                                group.start, group.end
                                            ))
                                            .monospace()
                                            .color(ui.visuals().weak_text_color()),
                                        );
                                        ui.label(RichText::new(&group.text).monospace());
                                    });
                                }
                            }
                        });
                    ui.add_space(8.0);
                }
            });
    }
}

fn collect_matches(regex: Option<&Regex>, text: &str) -> Vec<MatchInfo> {
    let Some(regex) = regex else {
        return Vec::new();
    };
    regex
        .captures_iter(text)
        .filter_map(|captures| {
            let mat = captures.get(0)?;
            let groups = captures
                .iter()
                .enumerate()
                .skip(1)
                .filter_map(|(i, g)| {
                    let g = g?;
                    Some(GroupInfo {
                        index: i,
                        text: g.as_str().to_owned(),
                        start: g.start(),
                        end: g.end(),
                    })
                })
                .collect();
            Some(MatchInfo {
                text: mat.as_str().to_owned(),
                start: mat.start(),
                end: mat.end(),
                groups,
            })
        })
        .collect()
}

struct HighlightRange {
    range: Range<usize>,
    background: Color32,
    priority: usize,
}

fn highlighted_text_job(
    text: &str,
    matches: &[MatchInfo],
    font_id: FontId,
    text_color: Color32,
    dark_mode: bool,
    wrap_width: f32,
) -> LayoutJob {
    let mut ranges = Vec::new();

    for mat in matches {
        push_highlight(
            &mut ranges,
            mat.start..mat.end,
            match_background(dark_mode),
            1,
        );
        for group in &mat.groups {
            push_highlight(
                &mut ranges,
                group.start..group.end,
                group_background(group.index, dark_mode),
                10 + group.index,
            );
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

// --- Pattern syntax highlighting ---

#[derive(Clone, Copy)]
enum PatternTokenKind {
    Literal,
    Escape,
    CharClass,
    Quantifier,
    Anchor,
    Group,
    Alternation,
    Dot,
}

fn pattern_highlight_job(
    pattern: &str,
    font_id: FontId,
    text_color: Color32,
    dark_mode: bool,
    wrap_width: f32,
) -> LayoutJob {
    let tokens = tokenize_pattern(pattern);
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    if tokens.is_empty() {
        job.append(
            pattern,
            0.0,
            TextFormat {
                font_id,
                color: text_color,
                ..Default::default()
            },
        );
        return job;
    }

    for (range, kind) in &tokens {
        let color = match kind {
            PatternTokenKind::Literal => text_color,
            _ => pattern_token_color(*kind, dark_mode),
        };
        job.append(
            &pattern[range.clone()],
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color,
                ..Default::default()
            },
        );
    }

    job
}

fn pattern_token_color(kind: PatternTokenKind, dark_mode: bool) -> Color32 {
    if dark_mode {
        match kind {
            PatternTokenKind::Escape | PatternTokenKind::Dot => Color32::from_rgb(86, 182, 194),
            PatternTokenKind::CharClass => Color32::from_rgb(229, 192, 123),
            PatternTokenKind::Group => Color32::from_rgb(198, 120, 221),
            PatternTokenKind::Quantifier => Color32::from_rgb(209, 154, 102),
            PatternTokenKind::Anchor | PatternTokenKind::Alternation => {
                Color32::from_rgb(224, 108, 117)
            }
            PatternTokenKind::Literal => unreachable!(),
        }
    } else {
        match kind {
            PatternTokenKind::Escape | PatternTokenKind::Dot => Color32::from_rgb(1, 132, 188),
            PatternTokenKind::CharClass => Color32::from_rgb(193, 132, 1),
            PatternTokenKind::Group => Color32::from_rgb(166, 38, 164),
            PatternTokenKind::Quantifier => Color32::from_rgb(152, 104, 1),
            PatternTokenKind::Anchor | PatternTokenKind::Alternation => {
                Color32::from_rgb(228, 86, 73)
            }
            PatternTokenKind::Literal => unreachable!(),
        }
    }
}

fn tokenize_pattern(pattern: &str) -> Vec<(Range<usize>, PatternTokenKind)> {
    let mut tokens = Vec::new();
    let mut chars = pattern.char_indices().peekable();

    while let Some(&(i, ch)) = chars.peek() {
        match ch {
            '\\' => tokenize_escape(&mut chars, &mut tokens),
            '[' => tokenize_char_class(&mut chars, &mut tokens),
            '(' => tokenize_group_open(&mut chars, &mut tokens),
            ')' => {
                chars.next();
                tokens.push((i..i + 1, PatternTokenKind::Group));
            }
            '*' | '+' => {
                chars.next();
                let mut end = i + 1;
                if matches!(chars.peek(), Some(&(_, '?'))) {
                    chars.next();
                    end += 1;
                }
                tokens.push((i..end, PatternTokenKind::Quantifier));
            }
            '?' => {
                chars.next();
                tokens.push((i..i + 1, PatternTokenKind::Quantifier));
            }
            '{' => tokenize_repetition(&mut chars, &mut tokens),
            '^' | '$' => {
                chars.next();
                tokens.push((i..i + 1, PatternTokenKind::Anchor));
            }
            '|' => {
                chars.next();
                tokens.push((i..i + 1, PatternTokenKind::Alternation));
            }
            '.' => {
                chars.next();
                tokens.push((i..i + 1, PatternTokenKind::Dot));
            }
            _ => {
                chars.next();
                tokens.push((i..i + ch.len_utf8(), PatternTokenKind::Literal));
            }
        }
    }

    tokens
}

fn tokenize_escape(
    chars: &mut Peekable<CharIndices>,
    tokens: &mut Vec<(Range<usize>, PatternTokenKind)>,
) {
    let (start, _) = chars.next().unwrap();
    if let Some(&(_, esc)) = chars.peek() {
        chars.next();
        let end = start + 1 + esc.len_utf8();
        let kind = match esc {
            'b' | 'B' | 'A' | 'z' | 'Z' => PatternTokenKind::Anchor,
            _ => PatternTokenKind::Escape,
        };
        tokens.push((start..end, kind));
    } else {
        tokens.push((start..start + 1, PatternTokenKind::Escape));
    }
}

fn tokenize_char_class(
    chars: &mut Peekable<CharIndices>,
    tokens: &mut Vec<(Range<usize>, PatternTokenKind)>,
) {
    let (start, _) = chars.next().unwrap();
    let mut end = start + 1;

    if matches!(chars.peek(), Some(&(_, '^'))) {
        let (j, c) = chars.next().unwrap();
        end = j + c.len_utf8();
    }
    if matches!(chars.peek(), Some(&(_, ']'))) {
        let (j, c) = chars.next().unwrap();
        end = j + c.len_utf8();
    }

    while let Some(&(j, c)) = chars.peek() {
        chars.next();
        end = j + c.len_utf8();
        if c == '\\' {
            if let Some(&(j2, c2)) = chars.peek() {
                chars.next();
                end = j2 + c2.len_utf8();
            }
        } else if c == ']' {
            break;
        }
    }

    tokens.push((start..end, PatternTokenKind::CharClass));
}

fn tokenize_group_open(
    chars: &mut Peekable<CharIndices>,
    tokens: &mut Vec<(Range<usize>, PatternTokenKind)>,
) {
    let (start, _) = chars.next().unwrap();
    let mut end = start + 1;

    if !matches!(chars.peek(), Some(&(_, '?'))) {
        tokens.push((start..end, PatternTokenKind::Group));
        return;
    }
    chars.next();
    end += 1;

    match chars.peek() {
        Some(&(_, ':')) | Some(&(_, '=')) | Some(&(_, '!')) => {
            chars.next();
            end += 1;
        }
        Some(&(_, '<')) => {
            chars.next();
            end += 1;
            match chars.peek() {
                Some(&(_, '=')) | Some(&(_, '!')) => {
                    chars.next();
                    end += 1;
                }
                _ => {
                    while let Some(&(j, c)) = chars.peek() {
                        chars.next();
                        end = j + c.len_utf8();
                        if c == '>' {
                            break;
                        }
                    }
                }
            }
        }
        Some(&(_, 'P')) => {
            chars.next();
            end += 1;
            if matches!(chars.peek(), Some(&(_, '<'))) {
                chars.next();
                end += 1;
                while let Some(&(j, c)) = chars.peek() {
                    chars.next();
                    end = j + c.len_utf8();
                    if c == '>' {
                        break;
                    }
                }
            }
        }
        Some(&(_, c)) if "imsUux-".contains(c) => {
            while let Some(&(j, c)) = chars.peek() {
                if "imsUux-".contains(c) {
                    chars.next();
                    end = j + c.len_utf8();
                } else {
                    if c == ':' {
                        chars.next();
                        end = j + 1;
                    }
                    break;
                }
            }
        }
        _ => {}
    }

    tokens.push((start..end, PatternTokenKind::Group));
}

fn tokenize_repetition(
    chars: &mut Peekable<CharIndices>,
    tokens: &mut Vec<(Range<usize>, PatternTokenKind)>,
) {
    let (start, _) = chars.next().unwrap();
    let mut end = start + 1;
    let saved = chars.clone();

    let mut has_digit = false;
    while let Some(&(j, c)) = chars.peek() {
        if c.is_ascii_digit() {
            has_digit = true;
            chars.next();
            end = j + 1;
        } else {
            break;
        }
    }

    let mut valid = false;
    if has_digit {
        match chars.peek() {
            Some(&(j, '}')) => {
                chars.next();
                end = j + 1;
                valid = true;
            }
            Some(&(_, ',')) => {
                chars.next();
                end += 1;
                while let Some(&(j, c)) = chars.peek() {
                    if c.is_ascii_digit() {
                        chars.next();
                        end = j + 1;
                    } else {
                        break;
                    }
                }
                if let Some(&(j, '}')) = chars.peek() {
                    chars.next();
                    end = j + 1;
                    valid = true;
                }
            }
            _ => {}
        }
    }

    if valid {
        if matches!(chars.peek(), Some(&(_, '?'))) {
            chars.next();
            end += 1;
        }
        tokens.push((start..end, PatternTokenKind::Quantifier));
    } else {
        *chars = saved;
        tokens.push((start..start + 1, PatternTokenKind::Literal));
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
