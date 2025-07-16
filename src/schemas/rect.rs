use super::InvalidColorSnafu;
use crate::schemas::pdf_utils::{draw_rectangle, DrawRectangle};
use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use printpdf::{Color, Mm, PdfDocument, Pt, Rgb};
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
