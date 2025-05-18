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
    background_color: Option<String>,
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
    background_color: Option<csscolorparser::Color>,
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
        alignment: Alignment,
        vertical_alignment: VerticalAlignment,
        font_map: &FontMap,
        padding: Option<Frame>,
    ) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .find(font_name.clone())
            .whatever_context("Font specified in the schema is not loaded")?;
        let font_spec = FontSpec::new(font.clone());
        let base = BaseSchema::new(String::from("cell"), x, y, width, height);

        Ok(Self {
            base,
            content,
            alignment,
            vertical_alignment,
            character_spacing: Pt(0.0),
            line_height: None,
            font_size: FontSize::Fixed(font_size),
            font_id: font_id.clone(),
            font_spec: font_spec.clone(),
            font_color: "#000"
                .parse::<csscolorparser::Color>()
                .context(InvalidColorSnafu)?,
            background_color: None,
            padding,
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

        let background_color = json
            .background_color
            .as_ref()
            .map(|c| csscolorparser::parse(c).context(InvalidColorSnafu))
            .transpose()?;

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
            background_color,
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
        let total_height: Mm = line_height_in_mm * (splitted_paragraphs.len() as f32);
        // + self.padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom);

        let y_offset: Mm = match self.vertical_alignment {
            VerticalAlignment::Top => Mm(0.0),
            VerticalAlignment::Middle => (box_height - total_height) / 2.0,
            VerticalAlignment::Bottom => box_height - total_height,
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

            let x_line: Mm = match self.alignment {
                Alignment::Left => self.base.x + self.padding.as_ref().map_or(Mm(0.0), |p| p.left),
                Alignment::Center => {
                    self.base.x
                        + residual / 2.0
                        + self.padding.as_ref().map_or(Mm(0.0), |p| (p.left))
                }
                Alignment::Right => {
                    self.base.x + residual + self.padding.as_ref().map_or(Mm(0.0), |p| p.left)
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

            let y = base_pdf.height
                - (self.base.y + y_offset)
                - line_height_in_mm * (index as i32 + 1) as f32
                - self.padding.as_ref().map_or(Mm(0.0), |p| p.top);

            let bg_ops = if let Some(bg_color) = self.background_color.clone() {
                vec![
                    Op::SaveGraphicsState,
                    Op::SetFillColor {
                        col: Color::Rgb(Rgb {
                            r: bg_color.r,
                            g: bg_color.g,
                            b: bg_color.b,
                            icc_profile: None,
                        }),
                    },
                    Op::DrawPolygon {
                        polygon: Polygon {
                            rings: vec![PolygonRing {
                                points: vec![
                                    LinePoint {
                                        p: Point {
                                            x: self.base.x.into(),
                                            y: (base_pdf.height - self.base.y).into(),
                                        },
                                        bezier: false,
                                    },
                                    LinePoint {
                                        p: Point {
                                            x: self.base.x.into(),
                                            y: (base_pdf.height - self.base.y - self.base.height)
                                                .into(),
                                        },
                                        bezier: false,
                                    },
                                    LinePoint {
                                        p: Point {
                                            x: (self.base.x + self.base.width).into(),
                                            y: (base_pdf.height - self.base.y - self.base.height)
                                                .into(),
                                        },
                                        bezier: false,
                                    },
                                    LinePoint {
                                        p: Point {
                                            x: (self.base.x + self.base.width).into(),
                                            y: (base_pdf.height - self.base.y).into(),
                                        },
                                        bezier: false,
                                    },
                                ],
                            }],
                            mode: printpdf::PaintMode::Fill,
                            winding_order: printpdf::WindingOrder::NonZero,
                        },
                    },
                    Op::RestoreGraphicsState,
                ]
            } else {
                vec![]
            };
            ops.extend_from_slice(&bg_ops);

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

        let height_in_mm: Mm = Pt(lines.len() as f32 * font_size.0).into();

        Ok(height_in_mm + self.padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom))
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
