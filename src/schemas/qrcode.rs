use crate::schemas::{base::BaseSchema, Error, JsonPosition};
use crate::utils::OpBuffer;
use image::codecs::png::PngEncoder;
use image::{ColorType, ExtendedColorType, ImageEncoder, Luma};
use printpdf::{Mm, Op, PdfDocument, Px, RawImage};
use qrcode_generator::QrCodeEcc;
use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonQrCodeSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
}

#[derive(Debug, Clone)]
pub struct QrCode {
    base: BaseSchema,
    content: String,
}

impl QrCode {
    pub fn new(name: String, x: Mm, y: Mm, width: Mm, height: Mm, content: String) -> Self {
        let base = BaseSchema::new(name, x, y, width, height);
        Self { base, content }
    }
    pub fn from_json(json: JsonQrCodeSchema) -> Result<Self, String> {
        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );
        Ok(Self {
            base,
            content: json.content,
        })
    }

    pub fn draw(
        &self,
        page_height_in_mm: Mm,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let qrcode_width = Px(256);
        let image = if false {
            let luma = qrcode_generator::to_png_to_vec(
                self.content.to_string(),
                QrCodeEcc::Low,
                qrcode_width.0,
            )
            .unwrap();
            RawImage::decode_from_bytes(&luma).unwrap()
        } else {
            let code = qrcode::QrCode::new(&self.content).unwrap();
            let luma: image::ImageBuffer<Luma<u8>, Vec<u8>> = code.render::<Luma<u8>>().build();
            let w = luma.width();
            let h = luma.height();

            let mut buf: Vec<u8> = Vec::new();
            let encoder: PngEncoder<&mut Vec<u8>> = PngEncoder::new(&mut buf);
            encoder
                .write_image(&luma.into_raw(), w, h, ExtendedColorType::L8)
                .unwrap();
            RawImage::decode_from_bytes(&buf).unwrap()
        };

        let image_xobject_id = doc.add_image(&image);

        let transform = self.base.get_matrix(page_height_in_mm, Some(qrcode_width));

        let ops = vec![Op::UseXObject {
            id: image_xobject_id,
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
