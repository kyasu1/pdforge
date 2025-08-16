use derive_new::new;
use icu_segmenter::WordSegmenter;
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;
use std::cmp::max;
use std::collections::BTreeMap;
use std::fmt::Debug;

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
        text: String,
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

#[derive(Debug, Clone)]
pub struct FontSpec {
    font: Box<ParsedFont>,
}

impl FontSpec {
    pub fn new(font: Box<ParsedFont>) -> Self {
        Self { font: font.clone() }
    }

    pub fn calculate_character_spacing(text: String, residual: Mm) -> Mm {
        use icu_segmenter::GraphemeClusterSegmenter;
        let segmenter = GraphemeClusterSegmenter::new();
        let len = segmenter.segment_str(&text).count() as f32;
        residual / len
    }

    fn get_splitted_lines_by_segmenter(
        &self,
        paragraph: String,
        font_size: Pt,
        box_width: Pt,
        character_spacing: Pt,
    ) -> Result<Vec<String>, Error> {
        let segments = Self::split_text_by_segmenter(paragraph);

        let mut lines: Vec<String> = vec![];
        let mut line_count: usize = 0;
        let mut current_text_size = Pt(0.0);

        for segment in segments.iter() {
            let text_width: Pt =
                self.width_of_text_at_size(segment.clone(), font_size, character_spacing)?;

            if current_text_size + text_width <= box_width {
                match lines.get(line_count) {
                    Some(current) => {
                        lines[line_count] = format!("{}{}", current, segment);
                        current_text_size = current_text_size + text_width + character_spacing
                    }
                    None => {
                        lines.push(String::from(segment));
                        current_text_size = text_width + character_spacing
                    }
                }
            } else if segment.is_empty() {
                line_count += 1;
                lines.push(String::from(""));
                current_text_size = Pt(0.0);
            } else if text_width <= box_width {
                line_count += 1;
                lines.push(String::from(segment));
                current_text_size = text_width + character_spacing;
            } else {
                for char in segment.chars() {
                    let size = self.width_of_text_at_size(
                        char.clone().to_string(),
                        font_size,
                        character_spacing,
                    )?;

                    if current_text_size + size <= box_width {
                        match lines.get(line_count) {
                            Some(current) => {
                                lines[line_count] = format!("{}{}", current, char);
                                current_text_size = current_text_size + size + character_spacing;
                            }
                            None => {
                                lines.push(char.to_string());
                                current_text_size = size + character_spacing;
                            }
                        }
                    } else {
                        line_count += 1;
                        lines.push(char.to_string());
                        current_text_size = size + character_spacing;
                    }
                }
            }
        }

        Ok(lines)
    }

    fn split_text_by_segmenter(paragraph: String) -> Vec<String> {
        let segmenter = WordSegmenter::new_auto();
        let breakpoints: Vec<usize> = segmenter.segment_str(&paragraph).collect();

        let mut segments: Vec<String> = vec![];
        let mut last_breakpoint = 0;
        for breakpoint in breakpoints {
            segments.push(paragraph[last_breakpoint..breakpoint].to_string());
            last_breakpoint = breakpoint;
        }

        if segments.is_empty() {
            vec![]
        } else {
            segments.remove(0);
            segments
        }
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
        let character_spacing = character_spacing;
        let splitted_paragraphs =
            self.split_text_to_size(content, font_size, width.clone().into(), character_spacing)?;

        for (line_index, line) in splitted_paragraphs.into_iter().enumerate() {
            if dynamic.is_fit_vertical() {
                let text_width = self.width_of_text_at_size(
                    line.replace("\n", ""),
                    font_size,
                    character_spacing,
                )?;

                total_width_in_mm = max(total_width_in_mm, text_width.into());
            }
            if line_index == 0 {
                total_height_in_mm += first_line_height_in_mm;
            } else {
                total_height_in_mm += other_row_height_in_mm;
            }

            if dynamic.is_fit_horizontal() {
                let text_width =
                    self.width_of_text_at_size(line.to_string(), font_size, character_spacing)?;

                total_width_in_mm = max(total_width_in_mm, text_width.into());
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

        for paragraph in content.lines().collect::<Vec<&str>>() {
            let lines = self.get_splitted_lines_by_segmenter(
                String::from(paragraph),
                font_size.clone(),
                box_width.into(),
                character_spacing.clone(),
            )?;
            splitted_paragraphs.push(lines);
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
            width.clone(),
            dynamic.clone(),
            content,
        )?;

        while dynamic.should_font_grow_to_fit(
            font_size.clone(),
            width,
            height,
            total_width_in_mm,
            total_height_in_mm,
        ) {
            font_size += Pt(0.25);
            let (new_total_width_in_mm, new_total_height_in_mm) = self.calculate_constraints(
                font_size.clone(),
                line_height,
                character_spacing,
                width.clone(),
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
            font_size.clone(),
            width.clone(),
            height.clone(),
            total_width_in_mm,
            total_height_in_mm,
        ) {
            font_size -= Pt(0.25);
            (total_width_in_mm, total_height_in_mm) = self.calculate_constraints(
                font_size,
                line_height,
                character_spacing,
                width.clone(),
                dynamic.clone(),
                content,
            )?;
        }

        Ok(font_size)
    }

    fn width_of_text_at_size(
        &self,
        text: String,
        font_size: Pt,
        character_spacing: Pt,
    ) -> Result<Pt, Error> {
        let percentage_font_scaling = 1000.0 / (self.font.font_metrics.units_per_em as f32);
        
        // TOFU (replacement character) width - use a reasonable default based on font size
        let tofu_width = 500.0; // Units in font metrics

        let standard_sizes: Vec<f32> = text
            .chars()
            .map(|char| {
                if char == ' ' {
                    self.font.space_width.unwrap_or(0) as f32 * percentage_font_scaling
                } else {
                    self.font
                        .lookup_glyph_index(char as u32)
                        .map(|glyph_index| self.font.get_horizontal_advance(glyph_index))
                        .map(|width| (width as f32) * percentage_font_scaling)
                        .unwrap_or(tofu_width * percentage_font_scaling) // Use TOFU width if glyph not found
                }
            })
            .collect();

        let scaled = standard_sizes.into_iter().sum::<f32>() * (font_size.0 as f32) / 1000.0;
        let additional_spacing = ((text.chars().count() - 1) as f32) * character_spacing.0;
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
    map: BTreeMap<String, (FontId, Box<ParsedFont>)>,
}

impl FontMap {
    pub fn add_font(
        &mut self,
        font_name: String,
        font_id: FontId,
        font: &ParsedFont,
    ) -> Option<(FontId, Box<ParsedFont>)> {
        self.map
            .insert(font_name.clone(), (font_id.clone(), Box::new(font.clone())))
    }

    pub fn find(&self, font_name: &str) -> Option<&(FontId, Box<ParsedFont>)> {
        self.map.get(font_name)
    }
}
