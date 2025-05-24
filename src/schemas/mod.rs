pub mod base;
pub mod dynamic_text;
pub mod image;
pub mod pdf_utils;
pub mod qrcode;
pub mod rect;
pub mod svg;
pub mod table;
pub mod text;

use crate::font::{self, FontMap};
use crate::utils::OpBuffer;
use base::BaseSchema;
use printpdf::{Mm, PdfDocument, PdfPage, PdfSaveOptions};
use serde::{Deserialize, Serialize};
use snafu::prelude::*;
use std::collections::HashMap;

use table::Table;
use text::Text;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not read file {filename}"))]
    TemplateFile {
        source: std::io::Error,
        filename: String,
    },
    #[snafu(display("Could not parse json file {message}"))]
    TemplateDeserialize {
        source: serde_json::Error,
        message: String,
    },

    #[snafu(display("Font error"))]
    FontError { source: font::Error },

    #[snafu(display("Invalid BasePDF"))]
    InvalidBasePdf,

    InvalidColor {
        source: csscolorparser::ParseColorError,
    },

    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error>, Some)))]
        source: Option<Box<dyn std::error::Error>>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum JsonSchema {
    Text(text::JsonTextSchema),
    DynamicText(dynamic_text::JsonDynamicTextSchema),
    Table(table::JsonTableSchema),
    QrCode(qrcode::JsonQrCodeSchema),
    Image(image::JsonImageSchema),
    Svg(svg::JsonSvgSchema),
    Rectangle(rect::JsonRectSchema),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct JsonBasePdf {
    width: f32,
    height: f32,
    padding: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonTemplate {
    schemas: Vec<serde_json::Value>,
    base_pdf: JsonBasePdf,
    #[serde(rename = "pdfmeVersion")]
    version: String,
}

//
//
//
//
pub trait SchemaTrait {
    fn render(
        &self,
        base_pdf: &BasePdf,
        current_top_mm: Option<Mm>,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error>;
    // fn set_x(&mut self, x: Mm);
    fn set_y(&mut self, y: Mm);
    fn set_height(&mut self, height: Mm);

    // fn get_width(&self) -> Mm;
}

#[derive(Debug, Clone)]
pub enum Schema {
    Text(text::Text),
    DynamicText(dynamic_text::DynamicText),
    Table(table::Table),
    QrCode(qrcode::QrCode),
    Image(image::Image),
    Svg(svg::Svg),
    Rect(rect::Rect),
}

impl Schema {
    pub fn get_base(self) -> BaseSchema {
        match self {
            Schema::Text(text) => text.get_base(),
            Schema::DynamicText(text) => text.get_base(),
            Schema::Table(table) => table.get_base(),
            Schema::QrCode(qr_code) => qr_code.get_base(),
            Schema::Image(image) => image.get_base(),
            Schema::Svg(svg) => svg.get_base(),
            Schema::Rect(rect) => rect.get_base(),
        }
    }
}

impl SchemaTrait for Schema {
    fn render(
        &self,
        base_pdf: &BasePdf,
        current_top_mm: Option<Mm>,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error> {
        match self.clone() {
            Schema::Text(mut schema) => {
                schema.render(&base_pdf, page, buffer)?;
                Ok((page, current_top_mm))
            }
            Schema::DynamicText(mut schema) => {
                schema.render(&base_pdf, page, current_top_mm, buffer)?;
                Ok((page, current_top_mm))
            }
            Schema::Table(mut table) => table.render(&base_pdf, doc, page, current_top_mm, buffer),
            Schema::QrCode(qr_code) => {
                qr_code.render(&base_pdf, doc, page, buffer)?;
                Ok((page, current_top_mm))
            }
            Schema::Image(image) => {
                image.render(&base_pdf, doc, page, buffer)?;
                Ok((page, current_top_mm))
            }
            Schema::Svg(svg) => {
                svg.render(&base_pdf, doc, page, buffer)?;
                Ok((page, current_top_mm))
            }
            Schema::Rect(rect) => {
                rect.render(&base_pdf, doc, page, buffer)?;
                Ok((page, current_top_mm))
            }
        }
    }

    fn set_y(&mut self, y: Mm) {
        match self {
            Schema::Text(text) => text.set_y(y),
            Schema::DynamicText(text) => text.set_y(y),
            Schema::QrCode(qr_code) => {
                qr_code.set_y(y);
            }
            _ => unimplemented!(),
        }
    }

    fn set_height(&mut self, height: Mm) {
        match self {
            Schema::Text(text) => text.set_height(height),
            Schema::DynamicText(text) => text.set_height(height),
            Schema::QrCode(qr_code) => {
                qr_code.set_height(height);
            }
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasePdf {
    width: Mm,
    height: Mm,
    padding: Frame,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonFrame {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Frame {
    top: Mm,
    right: Mm,
    bottom: Mm,
    left: Mm,
}

impl Frame {
    pub fn from_json(json: JsonFrame) -> Result<Self, Error> {
        Ok(Frame {
            top: Mm(json.top),
            right: Mm(json.right),
            bottom: Mm(json.bottom),
            left: Mm(json.left),
        })
    }
}

impl TryFrom<Vec<f32>> for Frame {
    type Error = Error;
    fn try_from(value: Vec<f32>) -> Result<Self, Self::Error> {
        Ok(Frame {
            top: Mm(value.get(0).context(InvalidBasePdfSnafu)?.clone()),
            right: Mm(value.get(1).context(InvalidBasePdfSnafu)?.clone()),
            bottom: Mm(value.get(2).context(InvalidBasePdfSnafu)?.clone()),
            left: Mm(value.get(3).context(InvalidBasePdfSnafu)?.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Template {
    pub schemas: Vec<serde_json::Value>,
    base_pdf: BasePdf,
    version: String,
}

impl Template {
    pub fn new(filename: &str) -> Result<Template, Error> {
        let raw = std::fs::read_to_string(filename).context(TemplateFileSnafu { filename })?;

        let json: JsonTemplate = serde_json::from_str(&raw).context(TemplateDeserializeSnafu {
            message: "Failed to parse JSON",
        })?;

        let base_pdf = BasePdf {
            width: Mm(json.base_pdf.width),
            height: Mm(json.base_pdf.height),
            padding: json.base_pdf.padding.try_into()?,
        };

        let template = Template {
            schemas: json.schemas,
            base_pdf,
            version: json.version,
        };
        Ok(template)
    }

    /*
       PDFmeのテンプレートは、複数のページを持つことができる。各ページはスキーマのリストにより構成される。
       スキーマには、テキスト、QRコード、画像、SVGなどの要素がある。
       テンプレートに含まれる各ページに対して、複数の入力データの集合を渡すことにより、データ集合毎にページを生成することができる。

    */
    pub fn render(
        &self,
        mut doc: &mut PdfDocument,
        font_map: &FontMap,
        inputs: Vec<Vec<HashMap<&'static str, String>>>,
    ) -> Result<Vec<u8>, Error> {
        // if inputs.len() != self.schemas.len() {
        //     return Err(Error::Whatever {
        //         message: "Input length does not match page length".to_string(),
        //         source: None,
        //     });
        // }

        let mut schemas: Vec<Vec<Schema>> = Vec::new();

        for (index, group) in inputs.iter().enumerate() {
            let json_schema = serde_json::to_string(&self.schemas[index]).unwrap();

            // Teraを使ってテンプレートをレンダリングする
            let mut tera = tera::Tera::default();
            tera.add_raw_template("schema_template", &json_schema)
                .unwrap();

            let mut json_schemas: Vec<Vec<JsonSchema>> = Vec::new();
            for input in group {
                let mut context = tera::Context::new();
                for (key, value) in input.iter() {
                    context.insert(*key, value);
                }
                let rendered = tera.render("schema_template", &context).unwrap();
                let parsed: Vec<JsonSchema> = serde_json::from_str(&rendered).unwrap();
                json_schemas.push(parsed);
            }

            // JSONをSchemaに変換する
            let converted: Vec<Vec<Schema>> = json_schemas
                .into_iter()
                .map(|page| {
                    page.into_iter()
                        .map(|schema| match schema {
                            JsonSchema::Text(json) => {
                                Schema::Text(Text::from_json(json, font_map).unwrap())
                            }
                            JsonSchema::DynamicText(json) => Schema::DynamicText(
                                dynamic_text::DynamicText::from_json(json, font_map).unwrap(),
                            ),
                            JsonSchema::Table(json) => {
                                Schema::Table(Table::from_json(json, font_map).unwrap())
                            }
                            JsonSchema::QrCode(json) => json.into(),
                            JsonSchema::Image(json) => json.try_into().unwrap(),
                            JsonSchema::Svg(json) => json.try_into().unwrap(),
                            JsonSchema::Rectangle(json) => json.try_into().unwrap(),
                        })
                        .collect::<Vec<Schema>>()
                })
                .collect();

            schemas.extend(converted);
        }

        let mut buffer = OpBuffer::default();

        let mut p = 0;
        let mut y: Option<Mm> = None;

        for (page_index, page) in schemas.iter().enumerate() {
            for schema in page {
                match schema.clone() {
                    Schema::Text(mut obj) => {
                        obj.render(&self.base_pdf, page_index, &mut buffer)?;
                    }
                    Schema::DynamicText(mut obj) => {
                        (p, y) = obj.render(&self.base_pdf, p, y, &mut buffer)?;
                    }
                    Schema::Table(mut obj) => {
                        (p, y) = obj.render(&self.base_pdf, doc, p, y, &mut buffer)?;
                    }
                    Schema::QrCode(obj) => {
                        obj.render(&self.base_pdf, &mut doc, page_index, &mut buffer)?;
                    }
                    Schema::Image(obj) => {
                        obj.render(&self.base_pdf, doc, page_index, &mut buffer)?
                    }
                    Schema::Svg(obj) => {
                        obj.render(&self.base_pdf, &mut doc, page_index, &mut buffer)?
                    }
                    Schema::Rect(obj) => {
                        obj.render(&self.base_pdf, &mut doc, page_index, &mut buffer)?
                    }
                }
            }
        }

        let mut pages: Vec<PdfPage> = Vec::new();

        for ops in buffer.buffer {
            let page = PdfPage::new(self.base_pdf.width, self.base_pdf.height, ops.to_vec());
            pages.push(page)
        }

        let mut warn = Vec::new();
        let bytes = doc.with_pages(pages).save(
            &PdfSaveOptions {
                optimize: false,
                subset_fonts: false,
                secure: false,
                image_optimization: None,
            },
            &mut warn,
        );

        Ok(bytes)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonPosition {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerticalAlignment {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonBoundingBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,
}

impl BoundingBox {
    pub fn new(x: Mm, y: Mm, width: Mm, height: Mm) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_json(json: JsonBoundingBox) -> Self {
        Self {
            x: Mm(json.x),
            y: Mm(json.y),
            width: Mm(json.width),
            height: Mm(json.height),
        }
    }
}

impl TryFrom<JsonBoundingBox> for BoundingBox {
    type Error = Error;

    fn try_from(json: JsonBoundingBox) -> Result<Self, Self::Error> {
        Ok(Self {
            x: Mm(json.x),
            y: Mm(json.y),
            width: Mm(json.width),
            height: Mm(json.height),
        })
    }
}
