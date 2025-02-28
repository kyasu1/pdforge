use crate::font::{FontMap, FontSpec};
use crate::schemas::base::BaseSchema;
use crate::schemas::{Error, JsonPosition, TextUtil};
use crate::utils::OpBuffer;
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;

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
    font_spec: FontSpec,
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
            .find(font_name.clone())
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
            font_spec: font_spec.clone(),
        })
    }

    pub fn from_json(json: JsonDynamicTextSchema, font_map: &FontMap) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .find(json.font_name.clone())
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
            font_spec: font_spec.clone(),
        };

        Ok(text)
    }
    pub fn render(
        &mut self,
        page_height_in_mm: Mm,
        current_page: usize,
        current_top_mm: Option<Mm>,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error> {
        let top_margin_in_mm = Mm(20.0);
        let bottom_margin_in_mm = Mm(20.0);
        let page_height: Pt = page_height_in_mm.into();

        let mut index_offset = 0;
        let mut page_counter = 0;
        let mut y_top_mm: Mm = current_top_mm.unwrap_or(self.base.y);
        // let mut y_bottom_mm = self.schema.base.get_y() + self.schema.base.get_height();
        let y_bottom_mm = page_height_in_mm - bottom_margin_in_mm;

        let line_height = Pt(self.line_height.unwrap_or(1.0) * self.font_size.0);

        let character_spacing = self.character_spacing;

        let lines: Vec<String> = TextUtil::split_text_to_size(
            &self.font_spec,
            &self.content,
            self.font_size.clone(),
            self.base.width.into(),
            character_spacing,
        )?;

        let y_offset: Pt = Pt(0.0);
        let mut y_line: Pt = Pt(0.0);
        let lines_splitted_to_pages: Vec<Vec<String>> = lines.into_iter().enumerate().fold(
            Vec::new(),
            |mut acc: Vec<Vec<String>>, (index, current)| {
                let row_y_offset = self.font_size * (index as f32 - index_offset as f32);
                y_line = page_height - y_top_mm.into() - y_offset - row_y_offset;
                if y_line <= page_height - y_bottom_mm.into() {
                    page_counter += 1;
                    acc.push(vec![current.clone()]);
                    index_offset = index;
                    y_top_mm = top_margin_in_mm;
                } else {
                    if let Some(page) = acc.get_mut(page_counter) {
                        page.push(current.clone());
                    } else {
                        acc.push(vec![current.clone()]);
                    }
                }
                acc
            },
        );

        let next_y_line_mm = page_height_in_mm - y_line.into() + self.font_size.into();

        let mut pages_increased = 0;
        let mut y_start = page_height - (current_top_mm.unwrap_or(self.base.y).into());

        for (page_index, page) in lines_splitted_to_pages.into_iter().enumerate() {
            let mut ops: Vec<Op> = vec![
                Op::StartTextSection,
                Op::SetTextCursor {
                    pos: Point {
                        x: self.base.x.into(),
                        y: y_start,
                    },
                },
                Op::SetLineHeight { lh: line_height },
                Op::SetCharacterSpacing {
                    // multiplier: character_spacing.clone(),
                    multiplier: 0.0,
                },
            ];

            for line in page {
                let line_ops = vec![
                    Op::WriteText {
                        text: line.to_string(),
                        font: self.font_id.clone(),
                        size: self.font_size.clone(),
                    },
                    Op::AddLineBreak,
                ];
                ops.extend_from_slice(&line_ops);
            }

            ops.extend_from_slice(&[Op::EndTextSection]);
            buffer.insert(current_page + page_index, ops);
            pages_increased += 1;
            y_start = page_height - top_margin_in_mm.into();
        }
        Ok((
            current_page + pages_increased - 1,
            // Some((page_height - y_line + top_margin_in_mm.into()).into()),
            // Some(page_height_in_mm - y_line.into()),
            Some(next_y_line_mm),
        ))
    }

    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }
}
