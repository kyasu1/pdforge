use crate::font::*;
use crate::schemas::base::BaseSchema;
use crate::utils::OpBuffer;
use derive_new::new;
use icu_segmenter::WordSegmenter;
use printpdf::*;
use std::cmp::max;

#[derive(Debug, Clone, new)]
pub struct FlowingTextSchema {
    base: BaseSchema,
    content: String,
    font_name: String,
    character_spacing: Option<Pt>,
    line_height: Option<Pt>,
    font_size: Pt
}

pub struct FlowingText {
    schema: FlowingTextSchema,
    font: FontSpec,
    font_id: FontId,
}

impl FlowingText {
    pub fn new(schema: TextSchema, font_map: &FontMap) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .map
            .get(&schema.font_name.clone())
            .ok_or(Error::FontNotFound)?;

        Ok(Self {
            schema,
            font: FontSpec::new(font),
            font_id: font_id.clone(),
        })
    }
}


