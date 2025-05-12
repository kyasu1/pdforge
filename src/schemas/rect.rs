use super::{BasePdf, InvalidColorSnafu};
use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use printpdf::{Color, LinePoint, Mm, Op, PdfDocument, Point, Polygon, PolygonRing, Pt, Rgb};
use serde::Deserialize;
use snafu::prelude::*;

/*
{
    "name": "field3",
    "type": "rectangle",
    "position": {
        "x": 10,
        "y": 32.65
    },
    "width": 62.5,
    "height": 37.5,
    "rotate": 0,
    "opacity": 1,
    "borderWidth": 1,
    "borderColor": "#000000",
    "color": "#FFFFFF",
    "readOnly": true,
    "radius": 10,
    "required": false
}
*/
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRectSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    rotate: Option<f32>,
    opacity: Option<f32>,
    border_width: Option<f32>,
    border_color: String,
    color: String,
}

#[derive(Debug, Clone)]
pub struct Rect {
    base: BaseSchema,

    rotate: Option<f32>,
    opacity: Option<f32>,
    border_width: Pt,
    border_color: csscolorparser::Color,
    color: csscolorparser::Color,
}

impl TryFrom<JsonRectSchema> for Schema {
    type Error = Error;

    fn try_from(json: JsonRectSchema) -> Result<Self, Error> {
        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );

        let border_color = csscolorparser::parse(&json.border_color).context(InvalidColorSnafu)?;
        let color = csscolorparser::parse(&json.color).context(InvalidColorSnafu)?;
        Ok(Schema::Rect(Rect {
            base,
            rotate: json.rotate,
            opacity: json.opacity,
            border_width: Pt(json.border_width.unwrap_or(1.0)),
            border_color,
            color,
        }))
    }
}

impl Rect {
    pub fn get_base(&self) -> &BaseSchema {
        &self.base
    }

    pub fn render(
        &self,
        base_pdf: &BasePdf,
        _doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let mode = if self.color == self.border_color {
            printpdf::PaintMode::Fill
        } else {
            printpdf::PaintMode::FillStroke
        };
        let ops = vec![
            Op::SaveGraphicsState,
            Op::SetFillColor {
                col: Color::Rgb(Rgb {
                    r: self.color.r,
                    g: self.color.g,
                    b: self.color.b,
                    icc_profile: None,
                }),
            },
            Op::SetOutlineColor {
                col: Color::Rgb(Rgb {
                    r: self.border_color.r,
                    g: self.border_color.g,
                    b: self.border_color.b,
                    icc_profile: None,
                }),
            },
            Op::SetOutlineThickness {
                pt: self.border_width,
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
                                    y: (base_pdf.height - self.base.y - self.base.height).into(),
                                },
                                bezier: false,
                            },
                            LinePoint {
                                p: Point {
                                    x: (self.base.x + self.base.width).into(),
                                    y: (base_pdf.height - self.base.y - self.base.height).into(),
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
                    mode,
                    winding_order: printpdf::WindingOrder::NonZero,
                },
            },
            Op::RestoreGraphicsState,
        ];

        buffer.insert(page, ops);

        Ok(())
    }

    pub fn set_x(&mut self, x: Mm) {
        self.base.x = x;
    }

    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }

    pub fn set_height(&mut self, height: Mm) {
        self.base.height = height;
    }

    pub fn get_width(&self) -> Mm {
        self.base.width
    }

    pub fn get_height(&self) -> Mm {
        self.base.height
    }
}
