use crate::schemas::{
    base::{rotation_for_xobject, BaseSchema},
    Error, HasBaseSchema, JsonPosition, Schema,
};
use crate::utils::OpBuffer;
use printpdf::{ExternalXObject, Mm, Op, PdfDocument};
use serde::Deserialize;
use snafu::{OptionExt, ResultExt};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSvgSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
    rotate: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct Svg {
    base: BaseSchema,
    content: ExternalXObject,
    rotate: Option<f32>,
}

impl TryFrom<JsonSvgSchema> for Schema {
    type Error = Error;
    fn try_from(json: JsonSvgSchema) -> Result<Self, Self::Error> {
        let content = Svg::parse(&json.content)?;

        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );
        Ok(Schema::Svg(Svg {
            base,
            content,
            rotate: json.rotate,
        }))
    }
}

impl Svg {
    pub fn new(
        name: String,
        x: Mm,
        y: Mm,
        width: Mm,
        height: Mm,
        content: String,
    ) -> Result<Self, Error> {
        let content = Svg::parse(&content)?;
        let base = BaseSchema::new(name, x, y, width, height);
        Ok(Self {
            base,
            content,
            rotate: None,
        })
    }

    pub fn with_rotate(mut self, rotate: f32) -> Self {
        self.rotate = Some(rotate);
        self
    }

    pub fn get_base(&self) -> BaseSchema {
        self.base.clone()
    }

    fn parse(content: &str) -> Result<ExternalXObject, Error> {
        let mut warnings = Vec::new();
        printpdf::svg::Svg::parse(content, &mut warnings)
            .with_whatever_context(|err| format!("Invalid SVG file {}", err))
    }

    pub fn render(
        &self,
        parent_height: Mm,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let svg_x_object_id = doc.add_xobject(&self.content);

        let mut transform = self.base.get_matrix(parent_height, self.content.width);
        if let Some(angle_deg) = self.rotate {
            let (width, height) = self
                .content
                .width
                .zip(self.content.height)
                .whatever_context("SVG XObject is missing width or height")?;
            transform.rotate = Some(rotation_for_xobject(angle_deg, width, height, &transform));
        }

        let ops = vec![Op::UseXobject {
            id: svg_x_object_id,
            transform,
        }];

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

impl HasBaseSchema for Svg {
    fn base(&self) -> &BaseSchema {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BaseSchema {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printpdf::Px;

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
        <rect width="200" height="100" fill="black"/>
    </svg>"#;

    #[test]
    fn json_svg_schema_accepts_rotation() {
        let json = format!(
            r#"{{
                "name": "test-svg",
                "position": {{"x": 10.0, "y": 20.0}},
                "width": 40.0,
                "height": 20.0,
                "content": {},
                "rotate": 90.0
            }}"#,
            serde_json::to_string(SVG).unwrap()
        );

        let schema: JsonSvgSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema.rotate, Some(90.0));
    }

    #[test]
    fn render_applies_rotation_around_transformed_svg_center() {
        let svg = Svg::new(
            "test-svg".to_string(),
            Mm(10.0),
            Mm(20.0),
            Mm(40.0),
            Mm(20.0),
            SVG.to_string(),
        )
        .unwrap()
        .with_rotate(90.0);
        let mut doc = PdfDocument::new("svg_rotation_test");
        let mut buffer = OpBuffer::default();

        svg.render(Mm(150.0), &mut doc, 0, &mut buffer).unwrap();

        let transform = buffer.buffer[0]
            .iter()
            .find_map(|op| match op {
                Op::UseXobject { transform, .. } => Some(transform),
                _ => None,
            })
            .expect("UseXobject must be emitted");
        let rotation = transform.rotate.expect("rotation must be set");
        let width = svg.content.width.unwrap();
        let height = svg.content.height.unwrap();

        assert_eq!(rotation.angle_ccw_degrees, 90.0);
        assert_eq!(
            rotation.rotation_center_x,
            Px(((width.0 as f32 * transform.scale_x.unwrap()) / 2.0).round() as usize)
        );
        assert_eq!(
            rotation.rotation_center_y,
            Px(((height.0 as f32 * transform.scale_y.unwrap()) / 2.0).round() as usize)
        );
    }
}
