use super::{Alignment, BasePdf, BoundingBox, Frame, VerticalAlignment};
use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, Luma};
use printpdf::{Mm, Op, PdfDocument, Px, RawImage};
use serde::Deserialize;
use snafu::ResultExt;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonQrCodeSchema {
    name: String,
    content: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    rotate: Option<f32>,
    alignment: Option<Alignment>,
    vertical_alignment: Option<VerticalAlignment>,
    padding: Option<Frame>,
}

#[derive(Debug, Clone)]
pub struct QrCode {
    base: BaseSchema,
    content: String,
    rotate: Option<f32>,
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    bounding_box: Option<BoundingBox>,
    padding: Option<Frame>,
}

impl From<JsonQrCodeSchema> for Schema {
    fn from(json: JsonQrCodeSchema) -> Self {
        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );

        let alignment = json.alignment.unwrap_or(Alignment::Left);
        let vertical_alignment = json.vertical_alignment.unwrap_or(VerticalAlignment::Top);

        Schema::QrCode(QrCode {
            base,
            content: json.content,
            rotate: json.rotate,
            alignment,
            vertical_alignment,
            bounding_box: None,
            padding: json.padding,
        })
    }
}
impl QrCode {
    pub fn new(name: String, x: Mm, y: Mm, width: Mm, height: Mm, content: String) -> Self {
        let base = BaseSchema::new(name, x, y, width, height);
        Self {
            base,
            content,
            rotate: None,
            alignment: Alignment::Center,
            vertical_alignment: VerticalAlignment::Middle,
            bounding_box: None,
            padding: None,
        }
    }

    pub fn get_base(self) -> BaseSchema {
        match self.bounding_box {
            Some(ref bounding_box) => BaseSchema {
                name: self.base.name,
                x: bounding_box.x,
                y: bounding_box.y,
                width: bounding_box.width,
                height: bounding_box.height,
            },
            None => self.base,
        }
    }

    // パディングを考慮したボックスの有効サイズを計算
    fn get_effective_box_size(&self) -> (Mm, Mm) {
        match self.bounding_box.clone() {
            Some(bounding_box) => {
                let box_width = bounding_box.width
                    - self.padding.as_ref().map_or(Mm(0.0), |p| p.left + p.right);

                let box_height = bounding_box.height
                    - self.padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom);

                (box_width, box_height)
            }

            None => {
                let box_width =
                    self.base.width - self.padding.as_ref().map_or(Mm(0.0), |p| p.left + p.right);

                let box_height =
                    self.base.height - self.padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom);

                (box_width, box_height)
            }
        }
    }

    // 垂直方向の配置オフセットを計算
    fn calculate_vertical_offset(&self, box_height: Mm, qr_height: Mm) -> Mm {
        match self.vertical_alignment {
            VerticalAlignment::Top => Mm(0.0),
            VerticalAlignment::Middle => (box_height - qr_height) / 2.0,
            VerticalAlignment::Bottom => box_height - qr_height,
        }
    }

    // 水平方向の配置オフセットを計算
    fn calculate_horizontal_offset(&self, box_width: Mm, qr_width: Mm) -> Mm {
        match self.alignment {
            Alignment::Left => Mm(0.0),
            Alignment::Center => (box_width - qr_width) / 2.0,
            Alignment::Right => box_width - qr_width,
            Alignment::Justify => Mm(0.0), // 通常、QRコードには適用されない
        }
    }

    pub fn render(
        &self,
        parent_height: Mm,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let qrcode_width = Px(256);

        let code =
            qrcode::QrCode::new(&self.content).whatever_context("Failed to create a QR code")?;
        let luma: image::ImageBuffer<Luma<u8>, Vec<u8>> = code.render::<Luma<u8>>().build();
        let w = luma.width();
        let h = luma.height();

        let mut buf: Vec<u8> = Vec::new();
        let encoder: PngEncoder<&mut Vec<u8>> = PngEncoder::new(&mut buf);
        encoder
            .write_image(&luma.into_raw(), w, h, ExtendedColorType::L8)
            .map_err(|_| Error::ImageEncoding {
                message: "Failed to encode QR code to PNG".to_string(),
            })?;

        let mut warnings = Vec::new();
        let image = RawImage::decode_from_bytes(&buf, &mut warnings)
            .whatever_context("Failed to decode QR code")?;

        let image_x_object_id = doc.add_image(&image);

        // QRコードの実際のサイズを計算
        let (box_width, box_height) = self.get_effective_box_size();
        let qr_width = self.base.width;
        let qr_height = self.base.height;

        // 配置オフセットを計算
        let x_offset = self.calculate_horizontal_offset(box_width, qr_width);
        let y_offset = self.calculate_vertical_offset(box_height, qr_height);

        // パディングとオフセットを考慮した位置を計算
        let x = self
            .bounding_box
            .clone()
            .map(|bounding_box| bounding_box.x)
            .unwrap_or(self.base.x)
            + self.padding.as_ref().map_or(Mm(0.0), |p| p.left)
            + x_offset;
        let y = self
            .bounding_box
            .clone()
            .map(|bounding_box| bounding_box.y)
            .unwrap_or(self.base.y)
            + self.padding.as_ref().map_or(Mm(0.0), |p| p.top)
            + y_offset;

        // 新しい基本スキーマを一時的に作成して、正しい位置に変換行列を取得
        let temp_base = BaseSchema::new(self.base.name.clone(), x, y, qr_width, qr_height);

        let transform = temp_base.get_matrix(parent_height, Some(qrcode_width));

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

    pub fn set_bounding_box(&mut self, bounding_box: BoundingBox) {
        self.bounding_box = Some(bounding_box);
    }

    pub fn set_x(&mut self, x: Mm) {
        match self.bounding_box {
            Some(ref mut bounding_box) => {
                bounding_box.x = x;
            }
            None => {
                self.base.x = x;
            }
        }
    }

    pub fn set_y(&mut self, y: Mm) {
        match self.bounding_box {
            Some(ref mut bounding_box) => {
                bounding_box.y = y;
            }
            None => {
                self.base.y = y;
            }
        }
    }

    // pub fn set_width(&mut self, width: Mm) {
    //     self.base.width = width;
    // }

    pub fn set_height(&mut self, height: Mm) {
        match self.bounding_box {
            Some(ref mut bounding_box) => {
                bounding_box.height = height;
            }
            None => {
                self.base.y = height;
            }
        }
    }

    pub fn get_width(&self) -> Mm {
        self.base.width
    }

    pub fn get_height(&self) -> Mm {
        self.base.height
    }
}
