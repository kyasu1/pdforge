pub mod base;
pub mod image;
pub mod qrcode;
pub mod table;
pub mod text;

use crate::font::{self, FontMap};
use crate::utils::OpBuffer;
use printpdf::{Mm, PdfDocument, PdfPage, PdfSaveOptions};
use qrcode::QrCode;
// use rust_pdfme::font::*;
use image::Image;
use serde::Deserialize;
use snafu::prelude::*;
use std::fs::File;
use std::io::BufReader;
use table::Table;
use text::Text;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not read file {filename}"))]
    TemplateFile {
        source: std::io::Error,
        filename: String,
    },
    #[snafu(display("Could not parse json file"))]
    TemplateDeserialize { source: serde_json::Error },

    #[snafu(display("Invalid HEX color string specified"))]
    InvalidColorString { source: palette::rgb::FromHexError },

    #[snafu(display("Font erro"))]
    FontError { source: font::Error },

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
    FlowingText(text::JsonTextSchema),
    Table(table::JsonTableSchema),
    QrCode(qrcode::JsonQrCodeSchema),
    Image(image::JsonImageSchema),
}

#[derive(Debug, Clone, Deserialize)]
struct JsonBasePdf {
    width: f32,
    height: f32,
    padding: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonTemplate {
    schemas: Vec<Vec<JsonSchema>>,
    base_pdf: JsonBasePdf,
    #[serde(rename = "pdfmeVersion")]
    version: String,
}
impl JsonTemplate {
    fn read_from_file(filename: &str) -> Result<JsonTemplate, Error> {
        let file = File::open(filename).context(TemplateFileSnafu { filename })?;
        let reader = BufReader::new(file);
        let template: JsonTemplate =
            serde_json::from_reader(reader).context(TemplateDeserializeSnafu)?;

        Ok(template)
    }
}

//
//
//
//
pub trait SchemaTrait {
    fn render(
        &self,
        page_height: Mm,
        current_top_mm: Option<Mm>,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error>;
    // fn set_x(&mut self, x: Mm);
    fn set_y(&mut self, y: Mm);
    // fn get_width(&self) -> Mm;
}

#[derive(Debug, Clone)]
pub enum Schema {
    Text(text::Text),
    DynamicText(text::DynamicText),
    Table(table::Table),
    QrCode(qrcode::QrCode),
    Image(image::Image),
}

impl SchemaTrait for Schema {
    fn render(
        &self,
        page_height: Mm,
        current_top_mm: Option<Mm>,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error> {
        match self.clone() {
            Schema::Text(mut schema) => {
                schema.render(page_height, page, buffer)?;
                Ok((page, current_top_mm))
            }
            Schema::DynamicText(mut schema) => {
                schema.render(page_height, page, current_top_mm, buffer)?;
                Ok((page, current_top_mm))
            }
            Schema::Table(mut table) => {
                table.render(page_height, doc, page, current_top_mm, buffer)
            }
            Schema::QrCode(qr_code) => {
                qr_code.draw(page_height, doc, page, buffer)?;
                Ok((page, current_top_mm))
            }
            Schema::Image(image) => {
                image.render(page_height, doc, page, buffer)?;
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
}

// impl Schema {
//     pub from_json()
// }

#[derive(Debug, Clone)]
pub struct BasePdf {
    width: Mm,
    height: Mm,
    padding: Vec<Mm>,
}

#[derive(Debug, Clone)]
pub struct Template {
    pub schemas: Vec<Vec<Schema>>,
    base_pdf: BasePdf,
    version: String,
}

impl Template {
    pub fn read_from_file(filename: &str, font_map: &FontMap) -> Result<Template, Error> {
        let json = JsonTemplate::read_from_file(filename)?;

        let base_pdf = BasePdf {
            width: Mm(json.base_pdf.width),
            height: Mm(json.base_pdf.height),
            padding: json.base_pdf.padding.into_iter().map(|v| Mm(v)).collect(),
        };
        let schemas = json
            .schemas
            .into_iter()
            .map(|page| {
                page.into_iter()
                    .map(|schema| match schema {
                        JsonSchema::Text(s) => Schema::Text(Text::from_json(s, font_map).unwrap()),
                        JsonSchema::FlowingText(s) => {
                            Schema::DynamicText(text::DynamicText::from_json(s, font_map).unwrap())
                        }
                        JsonSchema::Table(json) => {
                            Schema::Table(Table::from_json(json, font_map).unwrap())
                        }
                        JsonSchema::QrCode(json) => {
                            Schema::QrCode(QrCode::from_json(json).unwrap())
                        }
                        JsonSchema::Image(json) => Schema::Image(Image::from_json(json).unwrap()),
                    })
                    .collect()
            })
            .collect();
        let template = Template {
            schemas,
            base_pdf,
            version: json.version,
        };
        Ok(template)
    }

    pub fn render(&self, mut doc: &mut PdfDocument) -> Result<(), Error> {
        let mut buffer = OpBuffer::default();

        let mut p = 0;
        let mut y: Option<Mm> = None;
        let page_height = self.base_pdf.height;
        for (page_index, page) in self.schemas.iter().enumerate() {
            for schema in page {
                match schema.clone() {
                    Schema::Text(mut text) => {
                        println!("A");
                        text.render(page_height, page_index, &mut buffer).unwrap();
                    }
                    Schema::DynamicText(mut text) => {
                        println!("B");
                        (p, y) = text.render(page_height, p, y, &mut buffer).unwrap();
                    }
                    Schema::Table(mut table) => {
                        println!("C");
                        (p, y) = table.render(page_height, doc, p, y, &mut buffer).unwrap();
                    }
                    Schema::QrCode(obj) => {
                        println!("D");
                        obj.draw(page_height, &mut doc, page_index, &mut buffer)
                            .unwrap();
                    }
                    Schema::Image(obj) => obj.render(page_height, doc, page_index, &mut buffer)?,
                }

                println!("page {} next_y {:?}", p, y);
            }
        }

        let mut pages: Vec<PdfPage> = Vec::new();
        for ops in buffer.buffer {
            let page = PdfPage::new(Mm(210.0), Mm(297.0), ops.to_vec());
            pages.push(page)
        }

        let bytes = doc.with_pages(pages).save(&PdfSaveOptions {
            optimize: false,
            subset_fonts: false,
        });
        std::fs::write("./simple.pdf", bytes).unwrap();

        Ok(())
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
