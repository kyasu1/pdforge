use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, Luma};
use printpdf::{ExternalXObject, Mm, Op, PdfDocument, Px, RawImage};
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

    fn parse(content: &str) -> Result<ExternalXObject, Error> {
        printpdf::svg::Svg::parse(content).whatever_context("Invalid SVG File")
    }

    pub fn render(
        &self,
        page_height_in_mm: Mm,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let svg_x_object_id = doc.add_xobject(&self.content);

        let transform = self.base.get_matrix(page_height_in_mm, self.content.width);

        let ops = vec![Op::UseXobject {
            id: svg_x_object_id,
            transform,
        }];

        println!("SVG OPS {:?}", ops);
        buffer.insert(page, ops);

        Ok(())
    }

    pub fn set_x(&mut self, x: Mm) {
        self.base.x = x;
    }

    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }
    pub fn get_width(&self) -> Mm {
        self.base.width
    }

    pub fn get_height(&self) -> Mm {
        self.base.height
    }
}
