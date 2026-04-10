use derive_new::new;
use icu_segmenter::{GraphemeClusterSegmenter, WordSegmenter};
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;
use std::cell::RefCell;
use std::cmp::max;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;

// Thread-local WordSegmenter instance cached for performance
thread_local! {
    static WORD_SEGMENTER: RefCell<WordSegmenter> = RefCell::new(WordSegmenter::new_auto());
    static GRAPHEME_SEGMENTER: RefCell<GraphemeClusterSegmenter> =
        RefCell::new(GraphemeClusterSegmenter::new());
}

#[derive(Debug, Snafu)]
pub enum Error {
    CharacterNotInFont { char: char },
    InvalidFontSize,
    FontNotFound,
    DynamicFontMissingHeight,
}

pub trait FontSpecTrait {
    fn calculate_dynamic_font_size(
        &self,
        dynamic: DynamicFontSize,
        line_height: Option<f32>,
        character_spacing: Pt,
        width: Mm,
        height: Mm,
        content: &str,
    ) -> Result<Pt, Error>;

    fn split_text_to_size(
        &self,
        content: &str,
        font_size: Pt,
        box_width: Pt,
        character_spacing: Pt,
    ) -> Result<Vec<String>, Error>;

    fn width_of_text_at_size(
        &self,
        text: &str,
        font_size: Pt,
        character_spacing: Pt,
    ) -> Result<Pt, Error>;
    // fn height_of_font_at_size(&self, font_size: Pt) -> Pt;
}

impl Debug for dyn FontSpecTrait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontSpecTrait").finish()
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LineBreakMode {
    #[default]
    Word,
    // `char` is user-facing naming; the implementation uses grapheme clusters.
    Char,
}

#[derive(Debug, Clone)]
pub struct FontSpec {
    font: Arc<ParsedFont>,
    line_break_mode: LineBreakMode,
}

impl FontSpec {
    pub fn new(font: Arc<ParsedFont>) -> Self {
        Self {
            font,
            line_break_mode: LineBreakMode::Word,
        }
    }

    pub fn with_line_break_mode(mut self, line_break_mode: LineBreakMode) -> Self {
        self.line_break_mode = line_break_mode;
        self
    }

    fn apply_japanese_kinsoku(lines: Vec<String>) -> Vec<String> {
        filter_end_jp(filter_start_jp(lines))
    }

    pub fn calculate_character_spacing(text: &str, residual: Mm) -> Mm {
        // Early return for empty text
        if text.is_empty() {
            return Mm(0.0);
        }

        let char_count = Self::split_text_by_grapheme_cluster(text).len();

        // For justify alignment, distribute space between characters
        // If we have N characters, we have N-1 gaps between them
        if char_count <= 1 {
            return Mm(0.0);
        }

        residual / (char_count - 1) as f32
    }

    fn get_splitted_lines(
        &self,
        paragraph: &str,
        font_size: Pt,
        box_width: Pt,
        character_spacing: Pt,
    ) -> Result<Vec<String>, Error> {
        let segments = match self.line_break_mode {
            LineBreakMode::Word => Self::split_text_by_word_segmenter(paragraph),
            LineBreakMode::Char => Self::split_text_by_grapheme_cluster(paragraph),
        };

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for segment in segments {
            if segment.is_empty() {
                continue;
            }

            if current_line.is_empty() {
                let text_width =
                    self.width_of_text_at_size(&segment, font_size, character_spacing)?;

                if text_width <= box_width {
                    current_line = segment;
                } else {
                    current_line = self.push_grapheme_wrapped_lines(
                        &mut lines,
                        segment,
                        font_size,
                        box_width,
                        character_spacing,
                    )?;
                }
                continue;
            }

            let combined = format!("{}{}", current_line, segment);
            let combined_width =
                self.width_of_text_at_size(&combined, font_size, character_spacing)?;

            if combined_width <= box_width {
                current_line = combined;
                continue;
            }

            let segment_width =
                self.width_of_text_at_size(&segment, font_size, character_spacing)?;

            if segment_width <= box_width {
                lines.push(std::mem::take(&mut current_line));
                current_line = segment;
            } else {
                lines.push(std::mem::take(&mut current_line));
                current_line = self.push_grapheme_wrapped_lines(
                    &mut lines,
                    segment,
                    font_size,
                    box_width,
                    character_spacing,
                )?;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        Ok(lines)
    }

    fn push_grapheme_wrapped_lines(
        &self,
        lines: &mut Vec<String>,
        segment: String,
        font_size: Pt,
        box_width: Pt,
        character_spacing: Pt,
    ) -> Result<String, Error> {
        let mut current_line = String::new();

        for cluster in Self::split_text_by_grapheme_cluster(&segment) {
            if current_line.is_empty() {
                current_line = cluster;
                continue;
            }

            let combined = format!("{}{}", current_line, cluster);
            let combined_width =
                self.width_of_text_at_size(&combined, font_size, character_spacing)?;

            if combined_width <= box_width {
                current_line = combined;
            } else {
                lines.push(std::mem::take(&mut current_line));
                current_line = cluster;
            }
        }

        Ok(current_line)
    }

    fn split_text_by_word_segmenter(paragraph: &str) -> Vec<String> {
        let mut segments: Vec<String> = Vec::new();
        let mut last_breakpoint = 0;

        WORD_SEGMENTER.with(|segmenter| {
            for breakpoint in segmenter.borrow().segment_str(paragraph) {
                if last_breakpoint > 0 || breakpoint > 0 {
                    segments.push(paragraph[last_breakpoint..breakpoint].to_string());
                }
                last_breakpoint = breakpoint;
            }
        });

        segments
    }

    fn split_text_by_grapheme_cluster(paragraph: &str) -> Vec<String> {
        let mut segments: Vec<String> = Vec::new();
        let mut last_breakpoint = 0;

        GRAPHEME_SEGMENTER.with(|segmenter| {
            for breakpoint in segmenter.borrow().segment_str(paragraph) {
                if last_breakpoint > 0 || breakpoint > 0 {
                    segments.push(paragraph[last_breakpoint..breakpoint].to_string());
                }
                last_breakpoint = breakpoint;
            }
        });

        segments
    }

    fn glyph_width_for_char(&self, ch: char, percentage_font_scaling: f32, tofu_width: f32) -> f32 {
        if is_non_rendering_cluster_char(ch) {
            return 0.0;
        }

        if ch == ' ' {
            return self.font.space_width.unwrap_or(0) as f32 * percentage_font_scaling;
        }

        self.font
            .lookup_glyph_index(ch as u32)
            .map(|glyph_index| self.font.get_horizontal_advance(glyph_index))
            .map(|width| width as f32 * percentage_font_scaling)
            .unwrap_or(tofu_width * percentage_font_scaling)
    }

    fn calculate_constraints(
        &self,
        font_size: Pt,
        line_height: Option<f32>,
        character_spacing: Pt,
        width: Mm,
        dynamic: DynamicFontSize,
        content: &str,
    ) -> Result<(Mm, Mm), Error> {
        let mut total_width_in_mm: Mm = Mm(0.0);
        let mut total_height_in_mm: Mm = Mm(0.0);
        let line_height: f32 = line_height.unwrap_or(1.0);
        let first_line_text_height: Pt = self.height_of_font_at_size(font_size);
        let first_line_height_in_mm: Mm = (first_line_text_height * line_height).into();
        let other_row_height_in_mm: Mm = (font_size * line_height).into();
        let splitted_paragraphs =
            self.split_text_to_size(content, font_size, width.into(), character_spacing)?;

        let needs_width = dynamic.is_fit_vertical() || dynamic.is_fit_horizontal();

        for (line_index, line) in splitted_paragraphs.into_iter().enumerate() {
            // Calculate width only once if needed
            if needs_width {
                // Remove trailing newline if present
                let line_to_measure = if line.ends_with('\n') {
                    &line[..line.len() - 1]
                } else {
                    &line
                };

                let text_width =
                    self.width_of_text_at_size(line_to_measure, font_size, character_spacing)?;

                total_width_in_mm = max(total_width_in_mm, text_width.into());
            }

            // Calculate height
            if line_index == 0 {
                total_height_in_mm += first_line_height_in_mm;
            } else {
                total_height_in_mm += other_row_height_in_mm;
            }
        }

        Ok((total_width_in_mm, total_height_in_mm))
    }

    fn height_of_font_at_size(&self, font_size: Pt) -> Pt {
        font_size
    }
}

impl FontSpecTrait for FontSpec {
    fn split_text_to_size(
        &self,
        content: &str,
        font_size: Pt,
        box_width: Pt,
        character_spacing: Pt,
    ) -> Result<Vec<String>, Error> {
        let mut splitted_paragraphs: Vec<Vec<String>> = Vec::new();

        for paragraph in content.lines() {
            let lines =
                self.get_splitted_lines(paragraph, font_size, box_width, character_spacing)?;
            splitted_paragraphs.push(Self::apply_japanese_kinsoku(lines));
        }

        Ok(splitted_paragraphs.concat())
    }

    fn calculate_dynamic_font_size(
        &self,
        dynamic: DynamicFontSize,
        line_height: Option<f32>,
        character_spacing: Pt,
        width: Mm,
        height: Mm,
        content: &str,
    ) -> Result<Pt, Error> {
        let mut font_size: Pt = dynamic.get_min();
        let (mut total_width_in_mm, mut total_height_in_mm) = self.calculate_constraints(
            font_size,
            line_height,
            character_spacing,
            width,
            dynamic.clone(),
            content,
        )?;

        while dynamic.should_font_grow_to_fit(
            font_size,
            width,
            height,
            total_width_in_mm,
            total_height_in_mm,
        ) {
            font_size += Pt(0.25);
            let (new_total_width_in_mm, new_total_height_in_mm) = self.calculate_constraints(
                font_size,
                line_height,
                character_spacing,
                width,
                dynamic.clone(),
                content,
            )?;
            if new_total_height_in_mm < width {
                total_width_in_mm = new_total_width_in_mm;
                total_height_in_mm = new_total_height_in_mm
            } else {
                font_size -= Pt(0.25);
                break;
            }
        }

        while dynamic.should_font_shrink_to_fit(
            font_size,
            width,
            height,
            total_width_in_mm,
            total_height_in_mm,
        ) {
            font_size -= Pt(0.25);
            (total_width_in_mm, total_height_in_mm) = self.calculate_constraints(
                font_size,
                line_height,
                character_spacing,
                width,
                dynamic.clone(),
                content,
            )?;
        }

        Ok(font_size)
    }

    fn width_of_text_at_size(
        &self,
        text: &str,
        font_size: Pt,
        character_spacing: Pt,
    ) -> Result<Pt, Error> {
        // Early return for empty text
        if text.is_empty() {
            return Ok(Pt(0.0));
        }

        let percentage_font_scaling = 1000.0 / (self.font.font_metrics.units_per_em as f32);

        // TOFU (replacement character) width - use a reasonable default based on font size
        let tofu_width = 500.0; // Units in font metrics

        let mut total_width = 0.0_f32;
        let mut cluster_count = 0_usize;

        for cluster in Self::split_text_by_grapheme_cluster(text) {
            let sanitized_cluster = sanitize_cluster_for_font(&cluster, &self.font);
            let cluster_width = sanitized_cluster.chars().fold(0.0_f32, |acc, ch| {
                acc + self.glyph_width_for_char(ch, percentage_font_scaling, tofu_width)
            });

            total_width += if cluster_width > 0.0 {
                cluster_width
            } else {
                tofu_width * percentage_font_scaling
            };
            cluster_count += 1;
        }

        let scaled = total_width * (font_size.0) / 1000.0;
        let additional_spacing = if cluster_count > 1 {
            ((cluster_count - 1) as f32) * character_spacing.0
        } else {
            0.0
        };

        Ok(Pt(scaled + additional_spacing))
    }
}

#[derive(Debug, Clone, Deserialize)]
// #[serde(tag = "type")]
#[serde(untagged)]
pub enum JsonFontSize {
    Fixed(f32),
    Dynamic {
        min: f32,
        max: f32,
        fit: DynamicFontSizeFit,
    },
}

#[derive(Debug, Clone)]
pub enum FontSize {
    Fixed(Pt),
    Dynamic(DynamicFontSize),
}

impl FontSize {
    pub fn from_json(json: JsonFontSize) -> FontSize {
        match json {
            JsonFontSize::Fixed(f) => FontSize::Fixed(Pt(f)),
            JsonFontSize::Dynamic { min, max, fit } => FontSize::Dynamic(DynamicFontSize {
                min: Pt(min),
                max: Pt(max),
                fit,
            }),
        }
    }
}

#[derive(Debug, Clone, new)]
pub struct DynamicFontSize {
    min: Pt,
    max: Pt,
    fit: DynamicFontSizeFit,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DynamicFontSizeFit {
    Horizontal,
    Vertical,
}

impl DynamicFontSize {
    pub fn should_font_grow_to_fit(
        &self,
        font_size: Pt,
        width: Mm,
        height: Mm,
        total_width_in_mm: Mm,
        total_height_in_mm: Mm,
    ) -> bool {
        if font_size >= self.max {
            false
        } else if self.fit == DynamicFontSizeFit::Horizontal {
            total_width_in_mm < width
        } else {
            total_height_in_mm < height
        }
    }

    pub fn should_font_shrink_to_fit(
        &self,
        font_size: Pt,
        width: Mm,
        height: Mm,
        total_width_in_mm: Mm,
        total_height_in_mm: Mm,
    ) -> bool {
        if font_size <= self.min || font_size <= Pt(0.0) {
            false
        } else {
            total_width_in_mm > width || total_height_in_mm > height
        }
    }

    pub fn get_min(&self) -> Pt {
        self.min
    }

    pub fn is_fit_vertical(&self) -> bool {
        self.fit == DynamicFontSizeFit::Vertical
    }

    pub fn is_fit_horizontal(&self) -> bool {
        self.fit == DynamicFontSizeFit::Horizontal
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct FontMap {
    map: BTreeMap<String, (FontId, Arc<ParsedFont>)>,
}

impl FontMap {
    pub fn add_font(
        &mut self,
        font_name: String,
        font_id: FontId,
        font: &ParsedFont,
    ) -> Option<(FontId, Arc<ParsedFont>)> {
        self.map
            .insert(font_name.clone(), (font_id.clone(), Arc::new(font.clone())))
    }

    pub fn find(&self, font_name: &str) -> Option<&(FontId, Arc<ParsedFont>)> {
        self.map.get(font_name)
    }
}

// Line breaking constants for Japanese typography rules
const LINE_START_FORBIDDEN_CHARS_JA: &[char] = &[
    // Closing brackets / quotes
    ')', '）', ']', '］', '}', '｝', '>', '＞', '≫', '｣', '」', '』', '】', '〕', '〗', '〙', '〛',
    '〉', '》', '〞', '〟', '’', '”', '»', '｠', // Hyphens / prolonged sound marks
    '-', '‐', '‑', '‒', '–', '—', '―', '〜', '～', 'ー', 'ｰ',
    // Dividing punctuation / middle dots / separators
    '!', '！', '?', '？', '‼', '⁇', '⁈', '⁉', '・', '･', ':', '：', ';', '；', '/', '／',
    // Full stops / commas
    '.', '．', '。', ',', '，', '、', '､', // Iteration marks
    'ヽ', 'ヾ', 'ゝ', 'ゞ', '々', '〻', '〃', // Small kana
    'ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ', 'っ', 'ゃ', 'ゅ', 'ょ', 'ゎ', 'ゕ', 'ゖ', 'ァ', 'ィ', 'ゥ', 'ェ',
    'ォ', 'ッ', 'ャ', 'ュ', 'ョ', 'ヮ', 'ヵ', 'ヶ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｯ', 'ｬ', 'ｭ', 'ｮ',
];

const LINE_END_FORBIDDEN_CHARS_JA: &[char] = &[
    // Opening brackets / quotes
    '(', '（', '[', '［', '{', '｛', '<', '＜', '≪', '｢', '「', '『', '【', '〔', '〖', '〘', '〚',
    '〈', '《', '〝', '‘', '“', '\'', '"', '｟', '«',
];

// Line breaking constants for German typography rules
const LINE_END_FORBIDDEN_CHARS_DE: &[char] = &['-', '–', '—', '(', '[', '{', '„', '"', '\''];

const LINE_START_FORBIDDEN_CHARS_DE: &[char] = &[
    '.', ',', '!', '?', ':', ';', ')', ']', '}', '"', '"', '"', '\'',
];

const FALLBACK_CHARS: &[char] = &[
    '□',        // U+25A1 White Square (classic TOFU character)
    '\u{FFFD}', // U+FFFD Replacement Character (Unicode standard replacement)
    '?',        // Question mark (widely supported)
    '.',        // Period (basic punctuation)
    'X',        // Letter X (ASCII fallback)
    ' ',        // Space (should exist in any font)
];

fn is_non_rendering_cluster_char(ch: char) -> bool {
    matches!(ch, '\u{200C}' | '\u{200D}')
        || ('\u{FE00}'..='\u{FE0F}').contains(&ch)
        || ('\u{E0100}'..='\u{E01EF}').contains(&ch)
}

fn fallback_char_for_font(font: &ParsedFont) -> Option<char> {
    FALLBACK_CHARS
        .iter()
        .copied()
        .find(|fallback| font.lookup_glyph_index(*fallback as u32).is_some())
}

fn cluster_is_supported_by_font(cluster: &str, font: &ParsedFont) -> bool {
    if cluster == " " {
        return true;
    }

    let mut has_visible_glyph = false;

    for ch in cluster.chars() {
        if font.lookup_glyph_index(ch as u32).is_some() {
            has_visible_glyph = true;
            continue;
        }

        if is_non_rendering_cluster_char(ch) {
            continue;
        }

        return false;
    }

    has_visible_glyph
}

fn sanitize_cluster_for_font(cluster: &str, font: &ParsedFont) -> String {
    if cluster_is_supported_by_font(cluster, font) {
        cluster.to_string()
    } else {
        fallback_char_for_font(font)
            .map(|fallback| fallback.to_string())
            .unwrap_or_else(|| cluster.to_string())
    }
}

pub(crate) fn sanitize_text_for_font(text: &str, font: &ParsedFont) -> String {
    FontSpec::split_text_by_grapheme_cluster(text)
        .into_iter()
        .map(|cluster| sanitize_cluster_for_font(&cluster, font))
        .collect()
}

/// Generic function to filter lines to prevent forbidden characters at line start
/// Processes lines in reverse order to handle character movement properly
pub fn filter_start_forbidden_chars(lines: Vec<String>, forbidden_chars: &[char]) -> Vec<String> {
    let mut filtered: Vec<String> = Vec::new();
    let mut char_to_append: Option<char> = None;

    // Process lines in reverse order
    for line in lines.iter().rev() {
        if line.trim().is_empty() {
            filtered.push(String::new());
        } else {
            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                continue;
            }

            let char_at_start = chars[0];

            if forbidden_chars.contains(&char_at_start) {
                if line.trim().len() == 1 {
                    // Single character line - keep it as is
                    filtered.push(line.clone());
                    char_to_append = None;
                } else {
                    // Multi-character line - remove first character
                    let remaining_line: String = chars[1..].iter().collect();
                    if let Some(append_char) = char_to_append {
                        filtered.push(format!("{}{}", remaining_line, append_char));
                    } else {
                        filtered.push(remaining_line);
                    }
                    char_to_append = Some(char_at_start);
                }
            } else {
                // Not a forbidden character
                if let Some(append_char) = char_to_append {
                    filtered.push(format!("{}{}", line, append_char));
                    char_to_append = None;
                } else {
                    filtered.push(line.clone());
                }
            }
        }
    }

    // Handle leftover character
    if let Some(append_char) = char_to_append {
        if !filtered.is_empty() {
            let first_item = &filtered[0];
            let combined_item = format!("{}{}", append_char, first_item);
            filtered[0] = combined_item;
        } else {
            filtered.push(append_char.to_string());
        }
    }

    // Reverse back to original order
    filtered.reverse();
    filtered
}

/// Generic function to filter lines to prevent forbidden characters at line end
/// Processes lines in forward order
pub fn filter_end_forbidden_chars(lines: Vec<String>, forbidden_chars: &[char]) -> Vec<String> {
    let mut filtered: Vec<String> = Vec::new();
    let mut char_to_prepend: Option<char> = None;

    // Process lines in forward order
    for line in lines.iter() {
        if line.trim().is_empty() {
            filtered.push(String::new());
        } else {
            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                continue;
            }

            let char_at_end = chars[chars.len() - 1];

            if forbidden_chars.contains(&char_at_end) {
                if line.trim().len() == 1 {
                    // Single character line - keep it as is
                    filtered.push(line.clone());
                    char_to_prepend = None;
                } else {
                    // Multi-character line - remove last character
                    let remaining_line: String = chars[..chars.len() - 1].iter().collect();
                    if let Some(prepend_char) = char_to_prepend {
                        filtered.push(format!("{}{}", prepend_char, remaining_line));
                    } else {
                        filtered.push(remaining_line);
                    }
                    char_to_prepend = Some(char_at_end);
                }
            } else {
                // Not a forbidden character
                if let Some(prepend_char) = char_to_prepend {
                    filtered.push(format!("{}{}", prepend_char, line));
                    char_to_prepend = None;
                } else {
                    filtered.push(line.clone());
                }
            }
        }
    }

    // Handle leftover character
    if let Some(prepend_char) = char_to_prepend {
        if !filtered.is_empty() {
            let last_index = filtered.len() - 1;
            let last_item = &filtered[last_index];
            let combined_item = format!("{}{}", last_item, prepend_char);
            filtered[last_index] = combined_item;
        } else {
            filtered.push(prepend_char.to_string());
        }
    }

    filtered
}

/// Japanese line start forbidden characters processing (行頭禁則)
/// Prevents certain characters from appearing at the beginning of lines
pub fn filter_start_jp(lines: Vec<String>) -> Vec<String> {
    filter_start_forbidden_chars(lines, LINE_START_FORBIDDEN_CHARS_JA)
}

/// Japanese line end forbidden characters processing (行末禁則)
/// Prevents certain characters from appearing at the end of lines
pub fn filter_end_jp(lines: Vec<String>) -> Vec<String> {
    filter_end_forbidden_chars(lines, LINE_END_FORBIDDEN_CHARS_JA)
}

/// German line start forbidden characters processing
/// Prevents certain characters from appearing at the beginning of lines
pub fn filter_start_de(lines: Vec<String>) -> Vec<String> {
    filter_start_forbidden_chars(lines, LINE_START_FORBIDDEN_CHARS_DE)
}

/// German line end forbidden characters processing
/// Prevents certain characters from appearing at the end of lines
pub fn filter_end_de(lines: Vec<String>) -> Vec<String> {
    filter_end_forbidden_chars(lines, LINE_END_FORBIDDEN_CHARS_DE)
}

#[cfg(test)]
mod tests {
    use super::{
        filter_end_jp, filter_start_jp, sanitize_text_for_font, FontSpec, FontSpecTrait,
        LineBreakMode,
    };
    use printpdf::{ParsedFont, Pt};
    use std::path::PathBuf;
    use std::sync::{Arc, OnceLock};

    fn test_font() -> Arc<ParsedFont> {
        static FONT: OnceLock<Arc<ParsedFont>> = OnceLock::new();

        FONT.get_or_init(|| {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("fonts")
                .join("NotoSansJP-Regular.ttf");
            let font_bytes = std::fs::read(path).expect("test font should be readable");
            let parsed_font = ParsedFont::from_bytes(&font_bytes, 0, &mut Vec::new())
                .expect("test font should parse");
            Arc::new(parsed_font)
        })
        .clone()
    }

    fn test_font_spec(line_break_mode: LineBreakMode) -> FontSpec {
        FontSpec::new(test_font()).with_line_break_mode(line_break_mode)
    }

    #[test]
    fn filter_start_jp_moves_halfwidth_closing_paren() {
        let lines = vec!["abc".to_string(), ")def".to_string()];

        assert_eq!(
            filter_start_jp(lines),
            vec!["abc)".to_string(), "def".to_string()]
        );
    }

    #[test]
    fn filter_start_jp_moves_fullwidth_closing_paren() {
        let lines = vec!["abc".to_string(), "）def".to_string()];

        assert_eq!(
            filter_start_jp(lines),
            vec!["abc）".to_string(), "def".to_string()]
        );
    }

    #[test]
    fn filter_end_jp_moves_halfwidth_opening_paren() {
        let lines = vec!["abc(".to_string(), "def".to_string()];

        assert_eq!(
            filter_end_jp(lines),
            vec!["abc".to_string(), "(def".to_string()]
        );
    }

    #[test]
    fn filter_end_jp_moves_fullwidth_opening_paren() {
        let lines = vec!["abc（".to_string(), "def".to_string()];

        assert_eq!(
            filter_end_jp(lines),
            vec!["abc".to_string(), "（def".to_string()]
        );
    }

    #[test]
    fn apply_japanese_kinsoku_handles_mixed_width_characters_in_order() {
        let lines = vec!["abc(".to_string(), "）def".to_string()];

        assert_eq!(
            FontSpec::apply_japanese_kinsoku(lines),
            vec!["abc(）".to_string(), "def".to_string()]
        );
    }

    #[test]
    fn apply_japanese_kinsoku_keeps_non_forbidden_lines_unchanged() {
        let lines = vec!["abc".to_string(), "def".to_string()];

        assert_eq!(FontSpec::apply_japanese_kinsoku(lines.clone()), lines);
    }

    #[test]
    fn line_break_mode_char_can_split_inside_word() {
        let text = "abc def";
        let font_size = Pt(12.0);
        let word_spec = test_font_spec(LineBreakMode::Word);
        let char_spec = test_font_spec(LineBreakMode::Char);
        let box_width = char_spec
            .width_of_text_at_size("abc d", font_size, Pt(0.0))
            .unwrap();

        assert_eq!(
            word_spec
                .split_text_to_size(text, font_size, box_width, Pt(0.0))
                .unwrap(),
            vec!["abc ".to_string(), "def".to_string()]
        );
        assert_eq!(
            char_spec
                .split_text_to_size(text, font_size, box_width, Pt(0.0))
                .unwrap(),
            vec!["abc d".to_string(), "ef".to_string()]
        );
    }

    #[test]
    fn line_break_mode_char_keeps_family_emoji_as_single_cluster() {
        let text = "x👨‍👩‍👧‍👦y";
        let font_size = Pt(12.0);
        let spec = test_font_spec(LineBreakMode::Char);
        let box_width = spec
            .width_of_text_at_size("x👨‍👩‍👧‍👦", font_size, Pt(0.0))
            .unwrap();

        assert_eq!(
            spec.split_text_to_size(text, font_size, box_width, Pt(0.0))
                .unwrap(),
            vec!["x👨‍👩‍👧‍👦".to_string(), "y".to_string()]
        );
    }

    #[test]
    fn word_mode_fallback_keeps_family_emoji_as_single_cluster() {
        let text = "x👨‍👩‍👧‍👦y";
        let font_size = Pt(12.0);
        let spec = test_font_spec(LineBreakMode::Word);
        let box_width = spec
            .width_of_text_at_size("x👨‍👩‍👧‍👦", font_size, Pt(0.0))
            .unwrap();

        assert_eq!(
            spec.split_text_to_size(text, font_size, box_width, Pt(0.0))
                .unwrap(),
            vec!["x👨‍👩‍👧‍👦".to_string(), "y".to_string()]
        );
    }

    #[test]
    fn sanitize_text_for_font_replaces_unsupported_emoji_cluster_once() {
        let sanitized = sanitize_text_for_font("a👍🏽b", &test_font());

        assert_eq!(sanitized.chars().count(), 3);
        assert!(sanitized.starts_with('a'));
        assert!(sanitized.ends_with('b'));
        assert!(!sanitized.contains('👍'));
        assert!(!sanitized.contains('🏽'));
    }
}
