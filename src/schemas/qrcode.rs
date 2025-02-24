use crate::schemas::{base::BaseSchema, Error, JsonPosition};
use crate::utils::OpBuffer;
use printpdf::*;
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
        let qrcode_width = 1024;
        let luma =
            qrcode_generator::to_png_to_vec(self.content.to_string(), QrCodeEcc::Low, qrcode_width)
                .unwrap();
        let image = RawImage::decode_from_bytes(&luma).unwrap();
        let image_xobject_id = doc.add_image(&image);
        let ratio = 300.0 / (qrcode_width as f32) / 25.4;
        let transform = XObjectTransform {
            translate_x: Some(self.base.x().into()),
            translate_y: Some((page_height_in_mm - self.base.y()).into()),
            rotate: None,
            scale_x: Some(self.base.width().0 * ratio),
            scale_y: Some(self.base.height().0 * ratio),
            dpi: None,
        };

        let ops = vec![Op::UseXObject {
            id: image_xobject_id,
            transform,
        }];

        buffer.insert(page, ops);

        Ok(())
    }
}
