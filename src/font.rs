use derive_new::new;
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Snafu)]
pub enum Error {
    CharacterNotInFont { char: char },
    InvalidFontSize,
    FontNotFound,
    DynamicFontMissingHeight,
}

#[derive(Debug, Clone)]
pub struct FontSpec {
    font: Box<ParsedFont>,
}

impl FontSpec {
    pub fn new(font: Box<ParsedFont>) -> Self {
        Self { font: font.clone() }
    }
    pub fn width_of_text_at_size(
        &self,
        text: String,
        font_size: Pt,
        character_spacing: Pt,
    ) -> Result<Pt, Error> {
        let percentage_font_scaling = 1000.0 / (self.font.font_metrics.units_per_em as f32);

        let possible_standard_sizes: Result<Vec<f32>, char> = text
            .chars()
            .map(|char| {
                if char == ' ' {
                    Ok(self.font.space_width.unwrap_or(0) as f32 * percentage_font_scaling)
                } else {
                    self.font
                        .lookup_glyph_index(char as u32)
                        .map(|glyph_index| self.font.get_horizontal_advance(glyph_index))
                        .map(|width| (width as f32) * percentage_font_scaling)
                        .ok_or(char)
                }
            })
            .collect();

        match possible_standard_sizes {
            Ok(standard_sizes) => {
                let scaled =
                    standard_sizes.into_iter().sum::<f32>() * (font_size.0 as f32) / 1000.0;
                let additional_spacing = ((text.chars().count() - 1) as f32) * character_spacing.0;
                Ok(Pt(scaled + additional_spacing))
            }
            Err(char) => Err(Error::CharacterNotInFont { char }),
        }
    }

    pub fn height_of_font_at_size(&self, font_size: Pt) -> Pt {
        font_size
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
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

    pub fn is_fit_verfical(&self) -> bool {
        self.fit == DynamicFontSizeFit::Vertical
    }

    pub fn is_fit_horizontal(&self) -> bool {
        self.fit == DynamicFontSizeFit::Horizontal
    }
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DynamicFontSizeFit {
    Horizontal,
    Vertical,
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

    pub fn find(&self, font_name: String) -> Option<&(FontId, Box<ParsedFont>)> {
        self.map.get(&font_name)
    }
}
