use super::InvalidColorSnafu;
use crate::schemas::pdf_utils::{calculate_transform_matrix_with_center_pivot};
use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use printpdf::{Color, CurTransMat, Mm, Op, PdfDocument, Point, Pt, Rgb, LinePoint, Polygon, PolygonRing, PaintMode, WindingOrder};
use serde::Deserialize;
use snafu::prelude::*;

/*
{
    "name": "field3",
    "type": "line",
    "position": {
        "x": 10,
        "y": 32.65
    },
    "width": 62.5,
    "height": 0,
    "rotate": 0,
    "opacity": 1,
    "borderWidth": 1,
    "color": "#000000",
    "readOnly": true,
    "required": false
}
*/
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonLineSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: Option<f32>,
    rotate: Option<f32>,
    opacity: Option<f32>,
    border_width: Option<f32>,
    color: String,
}

#[derive(Debug, Clone)]
pub struct Line {
    base: BaseSchema,
    rotate: Option<f32>,
    opacity: Option<f32>,
    border_width: Pt,
    color: csscolorparser::Color,
}

impl TryFrom<JsonLineSchema> for Schema {
    type Error = Error;

    fn try_from(json: JsonLineSchema) -> Result<Self, Error> {
        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height.unwrap_or(0.0)), // 線の場合heightは0でOK
        );

        let color = csscolorparser::parse(&json.color).context(InvalidColorSnafu)?;
        Ok(Schema::Line(Line {
            base,
            rotate: json.rotate,
            opacity: json.opacity,
            border_width: Pt(json.border_width.unwrap_or(1.0)),
            color,
        }))
    }
}

impl Line {
    pub fn get_base(self) -> BaseSchema {
        self.base
    }

    pub fn render(
        &self,
        parent_height: Mm,
        _doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let ops = draw_line(DrawLine {
            x: self.base.x,
            y: self.base.y,
            width: self.base.width,
            rotate: self.rotate,
            page_height: parent_height,
            color: Color::Rgb(Rgb {
                r: self.color.r,
                g: self.color.g,
                b: self.color.b,
                icc_profile: None,
            }),
            border_width: Mm(self.border_width.0),
        });

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

#[derive(Debug, Clone)]
pub struct DrawLine {
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub rotate: Option<f32>,
    pub page_height: Mm,
    pub color: Color,
    pub border_width: Mm,
}

pub fn draw_line(props: DrawLine) -> Vec<Op> {
    let matrix_values = calculate_transform_matrix_with_center_pivot(
        props.x,
        props.page_height - props.y,
        props.width,
        Mm(0.0), // 線の高さは0
        props.rotate,
    );

    let matrix_op: Op = Op::SetTransformationMatrix {
        matrix: CurTransMat::Raw(matrix_values),
    };

    let x1: Pt = Pt(0.0);
    let y1: Pt = Pt(0.0);
    let x2: Pt = props.width.into();
    let y2: Pt = Pt(0.0);

    // 線を多角形として描画
    let polygon = Polygon {
        rings: vec![PolygonRing {
            points: vec![
                LinePoint {
                    p: Point { x: x1, y: y1 },
                    bezier: false,
                },
                LinePoint {
                    p: Point { x: x2, y: y2 },
                    bezier: false,
                },
            ],
        }],
        mode: PaintMode::Stroke,
        winding_order: WindingOrder::NonZero,
    };

    vec![
        Op::SaveGraphicsState,
        matrix_op,
        Op::SetOutlineColor { col: props.color },
        Op::SetOutlineThickness {
            pt: props.border_width.into(),
        },
        Op::DrawPolygon {
            polygon: polygon.clone(),
        },
        Op::RestoreGraphicsState,
    ]
}