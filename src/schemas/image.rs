use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use printpdf::{Mm, Op, PdfDocument, Px, RawImage};
use serde::Deserialize;
use snafu::{whatever, ResultExt};
use std::io::Cursor;

use super::BasePdf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonImageSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
}

#[derive(Debug, Clone)]
pub struct Image {
    base: BaseSchema,
    content: image::DynamicImage,
}

impl TryFrom<JsonImageSchema> for Schema {
    type Error = Error;
    fn try_from(json: JsonImageSchema) -> Result<Self, Self::Error> {
        let content: DynamicImage = Image::decode_base64_to_image_buffer(&json.content)?;

        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );
        Ok(Schema::Image(Image { base, content }))
    }
}
impl Image {
    fn decode_base64_to_image_buffer(content: &str) -> Result<DynamicImage, Error> {
        let parts: Vec<&str> = content.split(',').collect();
        if parts.len() != 2 {
            whatever!("Invalid image content");
        }
        let decoded = general_purpose::STANDARD
            .decode(parts[1])
            .whatever_context("Unable to decode data url image")?;
        Ok(image::load_from_memory(&decoded).unwrap())
    }

    pub fn new(
        name: String,
        x: Mm,
        y: Mm,
        width: Mm,
        height: Mm,
        content: String,
    ) -> Result<Self, Error> {
        let content: DynamicImage = Self::decode_base64_to_image_buffer(&content)?;
        let base = BaseSchema::new(name, x, y, width, height);

        Ok(Self { base, content })
    }

    pub fn get_base(self) -> BaseSchema {
        self.base
    }

    pub fn render(
        &self,
        parent_height: Mm,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let rgb_image = self.content.to_rgb8();

        let mut buf = Cursor::new(Vec::new());
        rgb_image.write_to(&mut buf, ImageFormat::Jpeg).unwrap();

        let mut warnings = Vec::new();
        let image = RawImage::decode_from_bytes(&buf.get_ref(), &mut warnings).unwrap();

        let image_x_object_id = doc.add_image(&image);

        let transform = self.base.get_matrix(parent_height, Some(Px(image.width)));

        let ops = vec![
            Op::SaveGraphicsState,
            Op::UseXobject {
                id: image_x_object_id,
                transform,
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
    pub fn get_width(&self) -> Mm {
        self.base.width
    }
    pub fn get_height(&self) -> Mm {
        self.base.height
    }
}
