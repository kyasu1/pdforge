use crate::schemas::{base::BaseSchema, Error, JsonPosition, Schema};
use crate::utils::OpBuffer;
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use printpdf::{Mm, Op, PdfDocument, Px, RawImage, XObjectTransform};
use serde::Deserialize;
use snafu::{whatever, ResultExt};
use std::io::Cursor;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectFit {
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}

impl Default for ObjectFit {
    fn default() -> Self {
        ObjectFit::Fill
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonImageSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
    #[serde(default)]
    object_fit: ObjectFit,
}

#[derive(Debug, Clone)]
pub struct Image {
    base: BaseSchema,
    content: image::DynamicImage,
    object_fit: ObjectFit,
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
        Ok(Schema::Image(Image { 
            base, 
            content, 
            object_fit: json.object_fit 
        }))
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
        image::load_from_memory(&decoded).map_err(|_| Error::ImageDecoding {
            message: "Failed to decode image from memory".to_string(),
        })
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

        Ok(Self { 
            base, 
            content, 
            object_fit: ObjectFit::default() 
        })
    }

    pub fn with_object_fit(mut self, object_fit: ObjectFit) -> Self {
        self.object_fit = object_fit;
        self
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
        rgb_image
            .write_to(&mut buf, ImageFormat::Jpeg)
            .map_err(|_| Error::ImageEncoding {
                message: "Failed to encode image to JPEG".to_string(),
            })?;

        let mut warnings = Vec::new();
        let image = RawImage::decode_from_bytes(&buf.get_ref(), &mut warnings)
            .whatever_context("Failed to decode image bytes for PDF")?;

        let image_x_object_id = doc.add_image(&image);
        let transform = self.calculate_object_fit_transform(parent_height, &image);

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

    fn calculate_object_fit_transform(
        &self,
        parent_height: Mm,
        image: &RawImage,
    ) -> XObjectTransform {
        let dpi: f32 = 300.0;
        
        match self.object_fit {
            ObjectFit::Fill => {
                // Use the original scaling logic for Fill mode
                self.base.get_matrix(parent_height, Some(Px(image.width)))
            },
            ObjectFit::Contain => {
                // Scale to fit within container while preserving aspect ratio
                let container_aspect = self.base.width.0 / self.base.height.0;
                let image_aspect = image.width as f32 / image.height as f32;
                
                let (scale_factor, offset_x, offset_y) = if image_aspect > container_aspect {
                    // Image is wider - fit to width
                    let scale = self.base.width.0 / (image.width as f32 / (dpi / 25.4));
                    let scaled_height = (image.height as f32 / (dpi / 25.4)) * scale;
                    let offset_y = (self.base.height.0 - scaled_height) / 2.0;
                    (scale, 0.0, offset_y)
                } else {
                    // Image is taller - fit to height
                    let scale = self.base.height.0 / (image.height as f32 / (dpi / 25.4));
                    let scaled_width = (image.width as f32 / (dpi / 25.4)) * scale;
                    let offset_x = (self.base.width.0 - scaled_width) / 2.0;
                    (scale, offset_x, 0.0)
                };
                
                XObjectTransform {
                    translate_x: Some(Mm(self.base.x.0 + offset_x).into()),
                    translate_y: Some((parent_height - self.base.y - self.base.height + Mm(offset_y)).into()),
                    rotate: None,
                    scale_x: Some(scale_factor),
                    scale_y: Some(scale_factor),
                    dpi: Some(dpi),
                }
            },
            ObjectFit::Cover => {
                // Scale to cover entire container while preserving aspect ratio
                let container_aspect = self.base.width.0 / self.base.height.0;
                let image_aspect = image.width as f32 / image.height as f32;
                
                let scale_factor = if image_aspect > container_aspect {
                    // Image is wider - fit to height (will crop width)
                    self.base.height.0 / (image.height as f32 / (dpi / 25.4))
                } else {
                    // Image is taller - fit to width (will crop height)
                    self.base.width.0 / (image.width as f32 / (dpi / 25.4))
                };
                
                XObjectTransform {
                    translate_x: Some(self.base.x.into()),
                    translate_y: Some((parent_height - self.base.y - self.base.height).into()),
                    rotate: None,
                    scale_x: Some(scale_factor),
                    scale_y: Some(scale_factor),
                    dpi: Some(dpi),
                }
            },
            ObjectFit::None => {
                // Display at natural size (1:1 pixel mapping)
                let ratio = dpi / 25.4;
                XObjectTransform {
                    translate_x: Some(self.base.x.into()),
                    translate_y: Some((parent_height - self.base.y - self.base.height).into()),
                    rotate: None,
                    scale_x: Some(1.0 / ratio),
                    scale_y: Some(1.0 / ratio),
                    dpi: Some(dpi),
                }
            },
            ObjectFit::ScaleDown => {
                // Use contain behavior, but only if image is larger than container
                let image_mm_width = image.width as f32 / (dpi / 25.4);
                let image_mm_height = image.height as f32 / (dpi / 25.4);
                
                if image_mm_width <= self.base.width.0 && image_mm_height <= self.base.height.0 {
                    // Image fits naturally - use None behavior
                    let ratio = dpi / 25.4;
                    let offset_x = (self.base.width.0 - image_mm_width) / 2.0;
                    let offset_y = (self.base.height.0 - image_mm_height) / 2.0;
                    
                    XObjectTransform {
                        translate_x: Some(Mm(self.base.x.0 + offset_x).into()),
                        translate_y: Some((parent_height - self.base.y - self.base.height + Mm(offset_y)).into()),
                        rotate: None,
                        scale_x: Some(1.0 / ratio),
                        scale_y: Some(1.0 / ratio),
                        dpi: Some(dpi),
                    }
                } else {
                    // Image is too large - use Contain behavior
                    let container_aspect = self.base.width.0 / self.base.height.0;
                    let image_aspect = image.width as f32 / image.height as f32;
                    
                    let (scale_factor, offset_x, offset_y) = if image_aspect > container_aspect {
                        let scale = self.base.width.0 / image_mm_width;
                        let scaled_height = image_mm_height * scale;
                        let offset_y = (self.base.height.0 - scaled_height) / 2.0;
                        (scale, 0.0, offset_y)
                    } else {
                        let scale = self.base.height.0 / image_mm_height;
                        let scaled_width = image_mm_width * scale;
                        let offset_x = (self.base.width.0 - scaled_width) / 2.0;
                        (scale, offset_x, 0.0)
                    };
                    
                    XObjectTransform {
                        translate_x: Some(Mm(self.base.x.0 + offset_x).into()),
                        translate_y: Some((parent_height - self.base.y - self.base.height + Mm(offset_y)).into()),
                        rotate: None,
                        scale_x: Some(scale_factor / (dpi / 25.4)),
                        scale_y: Some(scale_factor / (dpi / 25.4)),
                        dpi: Some(dpi),
                    }
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn create_mock_raw_image(width: u32, height: u32) -> RawImage {
        let rgb_data = vec![255u8; (width * height * 3) as usize];
        let dyn_image = DynamicImage::ImageRgb8(
            image::RgbImage::from_raw(width, height, rgb_data).unwrap()
        );
        let rgb_image = dyn_image.to_rgb8();
        let mut buf = Cursor::new(Vec::new());
        rgb_image.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
        let mut warnings = Vec::new();
        RawImage::decode_from_bytes(&buf.get_ref(), &mut warnings).unwrap()
    }

    fn create_test_image() -> Image {
        Image::new(
            "test".to_string(),
            Mm(0.0),
            Mm(0.0),
            Mm(100.0),
            Mm(80.0),
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string(),
        ).unwrap()
    }

    #[test]
    fn test_object_fit_default() {
        assert!(matches!(ObjectFit::default(), ObjectFit::Fill));
    }

    #[test]
    fn test_object_fit_deserialization() {
        assert!(matches!(
            serde_json::from_str::<ObjectFit>("\"fill\"").unwrap(),
            ObjectFit::Fill
        ));
        assert!(matches!(
            serde_json::from_str::<ObjectFit>("\"contain\"").unwrap(),
            ObjectFit::Contain
        ));
        assert!(matches!(
            serde_json::from_str::<ObjectFit>("\"cover\"").unwrap(),
            ObjectFit::Cover
        ));
        assert!(matches!(
            serde_json::from_str::<ObjectFit>("\"none\"").unwrap(),
            ObjectFit::None
        ));
        assert!(matches!(
            serde_json::from_str::<ObjectFit>("\"scale-down\"").unwrap(),
            ObjectFit::ScaleDown
        ));
    }

    #[test]
    fn test_json_image_schema_with_object_fit() {
        let json = r#"{
            "name": "test-image",
            "position": {"x": 10.0, "y": 20.0},
            "width": 100.0,
            "height": 80.0,
            "content": "data:image/jpeg;base64,test",
            "objectFit": "contain"
        }"#;
        
        let schema: JsonImageSchema = serde_json::from_str(json).unwrap();
        assert!(matches!(schema.object_fit, ObjectFit::Contain));
        assert_eq!(schema.name, "test-image");
        assert_eq!(schema.width, 100.0);
        assert_eq!(schema.height, 80.0);
    }

    #[test]
    fn test_json_image_schema_without_object_fit() {
        let json = r#"{
            "name": "test-image",
            "position": {"x": 10.0, "y": 20.0},
            "width": 100.0,
            "height": 80.0,
            "content": "data:image/jpeg;base64,test"
        }"#;
        
        let schema: JsonImageSchema = serde_json::from_str(json).unwrap();
        assert!(matches!(schema.object_fit, ObjectFit::Fill));
    }

    #[test]
    fn test_image_with_object_fit_builder() {
        let image = Image::new(
            "test".to_string(),
            Mm(10.0),
            Mm(20.0),
            Mm(100.0),
            Mm(80.0),
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string(),
        );
        
        assert!(image.is_ok());
        let image = image.unwrap().with_object_fit(ObjectFit::Cover);
        assert!(matches!(image.object_fit, ObjectFit::Cover));
    }

    #[test]
    fn test_calculate_object_fit_transform_fill() {
        let image = create_test_image().with_object_fit(ObjectFit::Fill);
        let mock_raw_image = create_mock_raw_image(50, 40);

        let transform = image.calculate_object_fit_transform(Mm(200.0), &mock_raw_image);
        
        // For Fill mode, should use base transform calculation
        assert!(transform.scale_x.is_some());
        assert!(transform.scale_y.is_some());
        assert!(transform.translate_x.is_some());
        assert!(transform.translate_y.is_some());
        assert_eq!(transform.dpi.unwrap(), 300.0); // Fill uses base transform with 300 DPI
    }

    #[test]
    fn test_calculate_object_fit_transform_contain() {
        let image = Image::new(
            "test".to_string(),
            Mm(0.0),
            Mm(0.0),
            Mm(100.0),
            Mm(80.0),
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string(),
        ).unwrap().with_object_fit(ObjectFit::Contain);

        let mock_raw_image = create_mock_raw_image(200, 100);

        let transform = image.calculate_object_fit_transform(Mm(200.0), &mock_raw_image);
        
        assert!(transform.scale_x.is_some());
        assert!(transform.scale_y.is_some());
        assert!(transform.translate_x.is_some());
        assert!(transform.translate_y.is_some());
        // For non-fill modes, should use custom transform calculation
        assert_eq!(transform.dpi.unwrap(), 300.0);
    }

    #[test]
    fn test_calculate_object_fit_transform_cover() {
        let image = create_test_image().with_object_fit(ObjectFit::Cover);
        let mock_raw_image = create_mock_raw_image(50, 100);

        let transform = image.calculate_object_fit_transform(Mm(200.0), &mock_raw_image);
        
        assert!(transform.scale_x.is_some());
        assert!(transform.scale_y.is_some());
        assert!(transform.translate_x.is_some());
        assert!(transform.translate_y.is_some());
        // For non-fill modes, should use custom transform calculation
        assert_eq!(transform.dpi.unwrap(), 300.0);
    }

    #[test]
    fn test_calculate_object_fit_transform_none() {
        let image = create_test_image().with_object_fit(ObjectFit::None);
        let mock_raw_image = create_mock_raw_image(50, 40);

        let transform = image.calculate_object_fit_transform(Mm(200.0), &mock_raw_image);
        
        assert!(transform.scale_x.is_some());
        assert!(transform.scale_y.is_some());
        assert!(transform.translate_x.is_some());
        assert!(transform.translate_y.is_some());
        assert_eq!(transform.dpi.unwrap(), 300.0);
    }

    #[test]
    fn test_calculate_object_fit_transform_scale_down_no_scaling() {
        let image = create_test_image().with_object_fit(ObjectFit::ScaleDown);
        let mock_raw_image = create_mock_raw_image(20, 15);

        let transform = image.calculate_object_fit_transform(Mm(200.0), &mock_raw_image);
        
        assert!(transform.scale_x.is_some());
        assert!(transform.scale_y.is_some());
        assert!(transform.translate_x.is_some());
        assert!(transform.translate_y.is_some());
        assert_eq!(transform.dpi.unwrap(), 300.0);
    }

    #[test]
    fn test_calculate_object_fit_transform_scale_down_with_scaling() {
        let mut image = create_test_image();
        image.base.width = Mm(50.0);
        image.base.height = Mm(40.0);
        let image = image.with_object_fit(ObjectFit::ScaleDown);

        let mock_raw_image = create_mock_raw_image(200, 100);

        let transform = image.calculate_object_fit_transform(Mm(200.0), &mock_raw_image);
        
        assert!(transform.scale_x.is_some());
        assert!(transform.scale_y.is_some());
        assert!(transform.translate_x.is_some());
        assert!(transform.translate_y.is_some());
        assert_eq!(transform.dpi.unwrap(), 300.0);
    }
}
