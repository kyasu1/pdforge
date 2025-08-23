use super::InvalidColorSnafu;
use crate::schemas::pdf_utils::{draw_rectangle, DrawRectangle};
use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use printpdf::{Color, Mm, Op, PaintMode, PdfDocument, LinePoint, Point, Polygon, PolygonRing, Pt, Rgb, WindingOrder};
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
        let rect = DrawRectangle {
            x: self.base.x,
            y: self.base.y,
            width: self.base.width,
            height: self.base.height,
            rotate: self.rotate,
            page_height: parent_height,
            color: Some(Color::Rgb(Rgb {
                r: self.color.r,
                g: self.color.g,
                b: self.color.b,
                icc_profile: None,
            })),
            border_width: Some(Mm(self.border_width.0)),
            border_color: Some(Color::Rgb(Rgb {
                r: self.border_color.r,
                g: self.border_color.g,
                b: self.border_color.b,
                icc_profile: None,
            })),
        };

        // let ops = vec![Op::SaveGraphicsState]
        //     .into_iter()
        //     .chain(draw_rectangle(rect))
        //     .chain(vec![Op::RestoreGraphicsState])
        //     .collect();

        let ops = draw_rectangle(rect);
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

// Rounded rectangle support for shape drawing
#[derive(Debug, Clone)]
pub struct DrawRoundedRectangle {
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,
    pub page_height: Mm,
    pub color: Option<Color>,
    pub border_width: Option<Mm>,
    pub border_color: Option<Color>,
    pub radius: Mm,
}

pub fn draw_rounded_rectangle(props: DrawRoundedRectangle) -> Vec<Op> {
    if props.radius.0 > props.width.0 / 2.0 || props.radius.0 > props.height.0 / 2.0 {
        return vec![];
    }

    const C: f32 = 0.551915024494;
    let kappa: Mm = Mm(C * props.radius.0);

    let _mode = match props.color {
        Some(_) => PaintMode::FillStroke,
        None => PaintMode::Stroke,
    };

    let color = props.color.unwrap_or(Color::Rgb(Rgb {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        icc_profile: None,
    }));

    let border_color = props.border_color.unwrap_or(Color::Rgb(Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        icc_profile: None,
    }));

    let border_width = props.border_width.unwrap_or(Mm(0.1));

    let bottom_y = props.page_height - props.y;
    let top_y = bottom_y + props.height;
    let right_x = props.x + props.width;

    // start drawing from lower left corner of the box counter clockwise direction
    let p10 = LinePoint {
        p: Point { x: (props.x).into(), y: (bottom_y + props.radius).into() },
        bezier: false,
    };
    let p11 = LinePoint {
        p: Point { x: (props.x).into(), y: (bottom_y + props.radius - kappa).into() },
        bezier: false,
    };
    let p12 = LinePoint {
        p: Point { x: (props.x + props.radius - kappa).into(), y: (bottom_y).into() },
        bezier: false,
    };
    let p13 = LinePoint {
        p: Point { x: (props.x + props.radius).into(), y: (bottom_y).into() },
        bezier: false,
    };

    // bottom right corner
    let p20 = LinePoint {
        p: Point { x: (right_x - props.radius).into(), y: (bottom_y).into() },
        bezier: false,
    };
    let p21 = LinePoint {
        p: Point { x: (right_x - props.radius + kappa).into(), y: (bottom_y).into() },
        bezier: false,
    };
    let p22 = LinePoint {
        p: Point { x: (right_x).into(), y: (bottom_y + props.radius - kappa).into() },
        bezier: false,
    };
    let p23 = LinePoint {
        p: Point { x: (right_x).into(), y: (bottom_y + props.radius).into() },
        bezier: false,
    };

    // top right corner
    let p30 = LinePoint {
        p: Point { x: (right_x).into(), y: (top_y - props.radius).into() },
        bezier: false,
    };
    let p31 = LinePoint {
        p: Point { x: (right_x).into(), y: (top_y - props.radius + kappa).into() },
        bezier: false,
    };
    let p32 = LinePoint {
        p: Point { x: (right_x - props.radius + kappa).into(), y: (top_y).into() },
        bezier: false,
    };
    let p33 = LinePoint {
        p: Point { x: (right_x - props.radius).into(), y: (top_y).into() },
        bezier: false,
    };

    // top left corner
    let p40 = LinePoint {
        p: Point { x: (props.x + props.radius).into(), y: (top_y).into() },
        bezier: false,
    };
    let p41 = LinePoint {
        p: Point { x: (props.x + props.radius - kappa).into(), y: (top_y).into() },
        bezier: false,
    };
    let p42 = LinePoint {
        p: Point { x: (props.x).into(), y: (top_y - props.radius + kappa).into() },
        bezier: false,
    };
    let p43 = LinePoint {
        p: Point { x: (props.x).into(), y: (top_y - props.radius).into() },
        bezier: false,
    };

    let polygon = Polygon {
        rings: vec![PolygonRing {
            points: vec![
                p10, p11, p12, p13, // bottom left corner
                p20, p21, p22, p23, // bottom right corner
                p30, p31, p32, p33, // top right corner
                p40, p41, p42, p43, // top left corner
            ],
        }],
        mode: PaintMode::FillStroke,
        winding_order: WindingOrder::NonZero,
    };

    let ops = vec![
        Op::SetOutlineColor {
            col: border_color.clone(),
        },
        Op::SetFillColor { col: color },
        Op::SetOutlineThickness { pt: Pt(border_width.0) },
        Op::DrawPolygon {
            polygon: polygon.clone(),
        },
    ];
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_rounded_rectangle_valid_radius() {
        let props = DrawRoundedRectangle {
            x: Mm(10.0),
            y: Mm(20.0),
            width: Mm(100.0),
            height: Mm(50.0),
            page_height: Mm(297.0),
            color: Some(Color::Rgb(Rgb {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                icc_profile: None,
            })),
            border_width: Some(Mm(1.0)),
            border_color: Some(Color::Rgb(Rgb {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                icc_profile: None,
            })),
            radius: Mm(5.0),
        };

        let ops = draw_rounded_rectangle(props);
        assert!(!ops.is_empty());
        assert_eq!(ops.len(), 4); // SetOutlineColor, SetFillColor, SetOutlineThickness, DrawPolygon
    }

    #[test]
    fn test_draw_rounded_rectangle_invalid_radius() {
        let props = DrawRoundedRectangle {
            x: Mm(10.0),
            y: Mm(20.0),
            width: Mm(100.0),
            height: Mm(50.0),
            page_height: Mm(297.0),
            color: Some(Color::Rgb(Rgb {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                icc_profile: None,
            })),
            border_width: Some(Mm(1.0)),
            border_color: Some(Color::Rgb(Rgb {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                icc_profile: None,
            })),
            radius: Mm(60.0), // Too large radius
        };

        let ops = draw_rounded_rectangle(props);
        assert!(ops.is_empty()); // Should return empty when radius is invalid
    }
}
