use super::BasePdf;
use crate::font::{FontMap, FontSize, FontSpec};
use crate::schemas::base::BaseSchema;
use crate::schemas::{Error, JsonPosition, TextUtil};
use crate::utils::OpBuffer;
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTextSchema {
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
pub struct Text {
    base: BaseSchema,
    content: String,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: FontSize,
    font_id: FontId,
    font_spec: FontSpec,
}

impl Text {
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
            font_size: FontSize::Fixed(font_size),
            font_id: font_id.clone(),
            font_spec: font_spec.clone(),
        })
    }

    pub fn get_base(&self) -> &BaseSchema {
        &self.base
    }

    pub fn from_json(json: JsonTextSchema, font_map: &FontMap) -> Result<Text, Error> {
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
            Some(f) => FontSize::Fixed(Pt(f)),
            None => {
                unimplemented!()
            }
        };
        let text = Text {
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
        base_pdf: &BasePdf,
        current_page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let font_size = self.get_font_size()?;

        let splitted_paragraphs: Vec<String> = TextUtil::split_text_to_size(
            &self.font_spec,
            &self.content,
            font_size,
            self.base.width.into(),
            self.character_spacing,
        )?;

        let line_height = Pt(self.line_height.unwrap_or(1.0) * font_size.0);

        let mut ops: Vec<Op> = vec![
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: self.base.x.into(),
                    y: (base_pdf.height - self.base.y).into(),
                },
            },
            Op::SetLineHeight { lh: line_height },
            Op::SetCharacterSpacing {
                // multiplier: character_spacing.clone(),
                multiplier: 0.0,
            },
            Op::SetFillColor {
                col: Color::Rgb(Rgb {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    icc_profile: None,
                }),
            },
            Op::SetFontSize {
                size: font_size.clone(),
                font: self.font_id.clone(),
            },
        ];

        for line in splitted_paragraphs {
            let line_ops = vec![
                Op::WriteText {
                    items: vec![TextItem::Text(line)],
                    font: self.font_id.clone(),
                },
                Op::AddLineBreak,
            ];
            ops.extend_from_slice(&line_ops);
        }

        ops.extend_from_slice(&[Op::EndTextSection]);
        buffer.insert(current_page, ops);

        Ok(())
    }

    pub fn set_x(&mut self, x: Mm) {
        self.base.x = x;
    }
    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }
    pub fn set_width(&mut self, width: Mm) {
        self.base.width = width;
    }
    pub fn set_height(&mut self, height: Mm) {
        self.base.height = height;
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }
    pub fn get_height(&self) -> Result<Mm, Error> {
        let font_size = self.get_font_size()?;

        let lines: Vec<String> = TextUtil::split_text_to_size(
            &self.font_spec,
            &self.content,
            font_size,
            self.base.width.into(),
            self.character_spacing,
        )?;

        Ok(Pt(lines.len() as f32 * font_size.0).into())
    }

    fn get_font_size(&self) -> Result<Pt, Error> {
        match self.font_size.clone() {
            FontSize::Fixed(font_size) => Ok(font_size),
            FontSize::Dynamic(dynamic) => TextUtil::calculate_dynamic_font_size(
                &self.font_spec,
                dynamic,
                self.line_height,
                self.character_spacing,
                self.base.width,
                self.base.height,
                &self.content,
            ),
        }
    }
}
