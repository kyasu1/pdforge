use super::{Alignment, BasePdf, Frame, InvalidColorSnafu, VerticalAlignment};
use crate::font::{DynamicFontSize, FontMap, FontSize, FontSpec, JsonFontSize};
use crate::schemas::base::BaseSchema;
use crate::schemas::{Error, FontSnafu, JsonPosition, TextUtil};
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
    alignment: Option<Alignment>,
    vertical_alignment: Option<VerticalAlignment>,
    character_spacing: Option<f32>,
    line_height: Option<f32>,
    font_size: JsonFontSize,
    font_color: Option<String>,
    padding: Option<Frame>,
}

#[derive(Debug, Clone)]
pub struct Text {
    base: BaseSchema,
    content: String,
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: FontSize,
    font_id: FontId,
    font_spec: FontSpec,
    font_color: csscolorparser::Color,
    padding: Option<Frame>,
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
            alignment: Alignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            character_spacing: Pt(0.0),
            line_height: None,
            font_size: FontSize::Fixed(font_size),
            font_id: font_id.clone(),
            font_spec: font_spec.clone(),
            font_color: "#000"
                .parse::<csscolorparser::Color>()
                .context(InvalidColorSnafu)?,
            padding: None,
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
            JsonFontSize::Dynamic { min, max, fit } => {
                FontSize::Dynamic(DynamicFontSize::new(Pt(min), Pt(max), fit))
            }
            JsonFontSize::Fixed(f) => FontSize::Fixed(Pt(f)),
        };

        let alignment = json.alignment.unwrap_or(Alignment::Left);
        let vertical_alignment = json.vertical_alignment.unwrap_or(VerticalAlignment::Top);

        let font_color = csscolorparser::parse(&json.font_color.unwrap_or("#000000".to_string()))
            .context(InvalidColorSnafu)?;

        let text = Text {
            base,
            content: json.content,
            character_spacing,
            alignment,
            vertical_alignment,
            line_height,
            font_size,
            font_id: font_id.clone(),
            font_spec: font_spec.clone(),
            font_color,
            padding: json.padding,
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

        // Calculates the effective width of the text box by subtracting horizontal padding from the base width.
        // If padding is defined, subtracts the sum of left and right padding values from the base width.
        // If padding is not defined, uses the full base width.
        let box_width =
            self.base.width - self.padding.as_ref().map_or(Mm(0.0), |p| p.left + p.right);

        let box_height =
            self.base.height - self.padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom);

        let splitted_paragraphs: Vec<String> = TextUtil::split_text_to_size(
            &self.font_spec,
            &self.content,
            font_size,
            box_width.into(),
            self.character_spacing,
        )?;

        let line_height: Pt = font_size * self.line_height.unwrap_or(1.0);
        let line_height_in_mm: Mm = line_height.into();
        let total_height: Mm = line_height_in_mm * (splitted_paragraphs.len() as f32)
            - self.padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom);

        let y_offset: Mm = match self.vertical_alignment {
            VerticalAlignment::Top => Mm(0.0),
            VerticalAlignment::Middle => (box_height - total_height - line_height_in_mm) / 2.0,
            VerticalAlignment::Bottom => self.base.height - total_height,
        };
        println!(
            "alignment {:?}, y_offset: {:?}, total_height: {:?}, box_height: {:?}",
            self.vertical_alignment, y_offset, total_height, box_height
        );

        let mut ops: Vec<Op> = vec![];

        for (index, line) in splitted_paragraphs.iter().enumerate() {
            let line_width: Mm = self
                .font_spec
                .width_of_text_at_size(line.clone(), font_size, self.character_spacing)
                .context(FontSnafu)?
                .into();

            let residual: Mm = box_width - line_width;

            // let x_line: Mm = match self.alignment {
            //     Alignment::Left => self.base.x,
            //     Alignment::Center => self.base.x + residual / 2.0,
            //     Alignment::Right => self.base.x + residual,
            //     Alignment::Justify => self.base.x,
            // };

            let x_line: Mm = match self.alignment {
                Alignment::Left => self.base.x + self.padding.as_ref().map_or(Mm(0.0), |p| p.left),
                Alignment::Center => {
                    self.base.x
                        + residual / 2.0
                        + self
                            .padding
                            .as_ref()
                            .map_or(Mm(0.0), |p| (p.left - p.right) / 2.0)
                }
                Alignment::Right => {
                    self.base.x + residual - self.padding.as_ref().map_or(Mm(0.0), |p| p.right)
                }
                Alignment::Justify => {
                    self.base.x + self.padding.as_ref().map_or(Mm(0.0), |p| p.left)
                }
            };

            let character_spacing: Pt = match self.alignment {
                Alignment::Justify => {
                    if line.ends_with('\n') {
                        self.character_spacing
                    } else {
                        self.character_spacing
                            + TextUtil::calculate_character_spacing(line.clone(), residual).into()
                    }
                }
                _ => self.character_spacing,
            };

            // let y = (base_pdf.height
            //     - (self.base.y + y_offset)
            //     - line_height_in_mm * (index as i32 + 1) as f32);

            let y = base_pdf.height
                - (self.base.y + y_offset)
                - line_height_in_mm * (index as i32 + 1) as f32
                - self.padding.as_ref().map_or(Mm(0.0), |p| p.top);

            let line_ops = vec![
                Op::StartTextSection,
                Op::SetLineHeight { lh: line_height },
                Op::SetFillColor {
                    col: Color::Rgb(Rgb {
                        r: self.font_color.r,
                        g: self.font_color.g,
                        b: self.font_color.b,
                        icc_profile: None,
                    }),
                },
                Op::SetFontSize {
                    size: font_size.clone(),
                    font: self.font_id.clone(),
                },
                Op::SetTextCursor {
                    pos: Point {
                        x: x_line.into(),
                        y: y.into(),
                    },
                },
                Op::SetCharacterSpacing {
                    // multiplier: character_spacing.clone(),
                    multiplier: character_spacing.0,
                },
                Op::WriteText {
                    items: vec![TextItem::Text(line.clone())],
                    font: self.font_id.clone(),
                },
                // Op::AddLineBreak,
                Op::EndTextSection,
            ];
            ops.extend_from_slice(&line_ops);
        }

        // ops.extend_from_slice(&[Op::EndTextSection]);
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
            FontSize::Dynamic(dynamic_font_size) => TextUtil::calculate_dynamic_font_size(
                &self.font_spec,
                dynamic_font_size,
                self.line_height,
                self.character_spacing,
                self.base.width,
                self.base.height,
                &self.content,
            ),
        }
    }
}
