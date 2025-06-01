use std::sync::Arc;

use crate::font::{FontMap, FontSpec, FontSpecTrait};
use crate::schemas::base::BaseSchema;
use crate::schemas::{Error, FontSnafu, JsonPosition};
use crate::utils::OpBuffer;
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;

use super::BasePdf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonDynamicTextSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
    font_name: String,
    character_spacing: Option<f32>,
    line_height: Option<f32>,
    font_size: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct DynamicText {
    base: BaseSchema,
    content: String,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: Pt,
    font_id: FontId,
    font_spec: Arc<dyn FontSpecTrait>,
}

impl DynamicText {
    pub fn new(
        x: Mm,
        y: Mm,
        width: Mm,
        height: Mm,
        font_name: String,
        font_size: Pt,
        content: String,
        font_map: &FontMap,
    ) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .find(&font_name)
            .whatever_context("Font specified in the schema is not loaded")?;
        let font_spec = FontSpec::new(font.clone());
        let base = BaseSchema::new(String::from("cell"), x, y, width, height);

        Ok(Self {
            base,
            content,
            character_spacing: Pt(0.0),
            line_height: None,
            font_size,
            font_id: font_id.clone(),
            font_spec: Arc::new(font_spec.clone()),
        })
    }

    pub fn from_json(json: JsonDynamicTextSchema, font_map: &FontMap) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .find(&json.font_name)
            .whatever_context("Font specified in the schema is not loaded")?;
        let font_spec = FontSpec::new(font.clone());

        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );

        let character_spacing = json.character_spacing.map(|f| Pt(f)).unwrap_or(Pt(0.0));
        let line_height = json.line_height;
        let font_size = match json.font_size {
            Some(f) => Pt(f),
            None => {
                unimplemented!()
            }
        };
        let text = Self {
            base,
            content: json.content,
            character_spacing,
            line_height,
            font_size,
            font_id: font_id.clone(),
            font_spec: Arc::new(font_spec.clone()),
        };

        Ok(text)
    }

    pub fn get_base(self) -> BaseSchema {
        self.base
    }

    pub fn render(
        &mut self,
        base_pdf: &BasePdf,
        current_page: usize,
        current_top_mm: Option<Mm>,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error> {
        println!("current_top_mm {:?}", current_top_mm);

        let top_margin_in_mm = base_pdf.padding.top;
        let bottom_margin_in_mm = base_pdf.padding.bottom;
        let page_height_in_mm = base_pdf.height;

        let mut page_counter = 0;
        let y_top_mm: Mm = current_top_mm.unwrap_or(self.base.y);
        let mut y_line_mm: Mm = y_top_mm;
        let y_bottom_mm = base_pdf.height - bottom_margin_in_mm;
        let line_height_in_mm: Mm = Pt(self.line_height.unwrap_or(1.0) * self.font_size.0).into();
        let character_spacing = self.character_spacing;

        let lines: Vec<String> = self
            .font_spec
            .split_text_to_size(
                &self.content,
                self.font_size.clone(),
                self.base.width.into(),
                character_spacing,
            )
            .context(FontSnafu)?;

        let mut pages: Vec<Vec<String>> = Vec::new();

        for line in lines.into_iter() {
            y_line_mm = y_line_mm + line_height_in_mm;

            if y_line_mm > y_bottom_mm {
                page_counter += 1;
                y_line_mm = top_margin_in_mm;
            }

            if let Some(page) = pages.get_mut(page_counter) {
                page.push(line.clone());
            } else {
                pages.insert(page_counter, vec![line.clone()]);
            }
        }

        let next_y_line_mm = y_line_mm;

        let mut y_start = y_top_mm;

        let pages_increased = pages.len() - 1;

        for (page_index, page) in pages.into_iter().enumerate() {
            let mut ops: Vec<Op> = Vec::new();

            for (index, line) in page.into_iter().enumerate() {
                let y_position =
                    page_height_in_mm - y_start - line_height_in_mm * (index + 1) as f32;

                let matrix = super::pdf_utils::calculate_transform_matrix_with_center_pivot(
                    self.base.x,
                    self.base.y,
                    self.base.width,
                    self.base.height,
                    None,
                );

                let line_ops = super::pdf_utils::create_text_ops(
                    matrix,
                    &self.font_id,
                    self.font_size,
                    self.base.x,
                    y_position,
                    None,
                    None,
                    character_spacing,
                    &line,
                    self.line_height,
                    &csscolorparser::parse("#000000").unwrap(), // デフォルトの黒色
                );

                ops.extend_from_slice(&line_ops);
            }
            buffer.insert(current_page + page_index, ops);
            y_start = top_margin_in_mm;
        }
        Ok((current_page + pages_increased, Some(next_y_line_mm)))
    }

    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }

    pub fn set_height(&mut self, height: Mm) {
        self.base.height = height;
    }
}
