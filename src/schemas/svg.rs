use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use printpdf::{ExternalXObject, Mm, Op, PdfDocument};
use serde::Deserialize;
use snafu::ResultExt;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSvgSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
}

#[derive(Debug, Clone)]
pub struct Svg {
    base: BaseSchema,
    content: ExternalXObject,
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
        Ok(Schema::Svg(Svg { base, content }))
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
        Ok(Self { base, content })
    }

    pub fn get_base(self) -> BaseSchema {
        self.base
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

        let transform = self.base.get_matrix(parent_height, self.content.width);

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
