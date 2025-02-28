use std::any::Any;
use crate::schemas::{base::BaseSchema, Error, JsonPosition};
use crate::utils::OpBuffer;
use base64::{engine::general_purpose, Engine as _};
use image::codecs::png::{PngDecoder, PngEncoder};
use image::{DynamicImage, EncodableLayout, ExtendedColorType, ImageFormat, Rgb};
use printpdf::{Mm, Op, PdfDocument, Px, RawImage};
use serde::Deserialize;
use std::io::Cursor;

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

impl Image {
    fn decode_base64_to_image_buffer(content: &str) -> Result<DynamicImage, String> {
        let parts: Vec<&str> = content.split(',').collect();
        if parts.len() != 2 {
            return Err(String::from("invalid data url"));
        }
        let decoded = general_purpose::STANDARD.decode(parts[1]).unwrap();
        Ok(image::load_from_memory(&decoded).unwrap())
    }

    pub fn new(
        name: String,
        x: Mm,
        y: Mm,
        width: Mm,
        height: Mm,
        content: String,
    ) -> Result<Self, String> {
        let content: DynamicImage = Self::decode_base64_to_image_buffer(&content)?;
        let base = BaseSchema::new(name, x, y, width, height);

        Ok(Self { base, content })
    }
    pub fn from_json(json: JsonImageSchema) -> Result<Self, String> {
        let content: DynamicImage = Self::decode_base64_to_image_buffer(&json.content)?;

        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );
        Ok(Self { base, content })
    }

    pub fn render(
        &self,
        page_height_in_mm: Mm,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let rgb_image = self.content.to_rgb8();

        let mut buf = Cursor::new(Vec::new());
        rgb_image.write_to(&mut buf, ImageFormat::Jpeg).unwrap();

        let image = RawImage::decode_from_bytes(&buf.get_ref()).unwrap();

        let image_x_object_id = doc.add_image(&image);

        let transform = self
            .base
            .get_matrix(page_height_in_mm, Some(Px(image.width)));

        let ops = vec![Op::UseXObject {
            id: image_x_object_id,
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
    pub fn get_width(&self) -> Mm {
        self.base.width
    }

    pub fn get_height(&self) -> Mm {
        self.base.height
    }
}
