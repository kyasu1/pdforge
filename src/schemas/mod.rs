pub mod base;
pub mod image;
pub mod qrcode;
pub mod table;
pub mod text;

use crate::font;
use printpdf::Mm;
use qrcode::QrCode;
use serde::Deserialize;
use snafu::prelude::*;
use std::fs::File;
use std::io::BufReader;
use table::TableSchema;
use text::TextSchema;

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
#[derive(Debug, Clone)]
pub enum Schema {
    Text(text::TextSchema),
    FlowingText(text::TextSchema),
    Table(table::TableSchema),
    QrCode(qrcode::QrCode),
}

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
    pub fn read_from_file(filename: &str) -> Result<Template, Error> {
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
                        JsonSchema::Text(s) => Schema::Text(TextSchema::from_json(s).unwrap()),
                        JsonSchema::FlowingText(s) => {
                            Schema::FlowingText(TextSchema::from_json(s).unwrap())
                        }
                        JsonSchema::Table(json) => {
                            Schema::Table(TableSchema::from_json(json).unwrap())
                        }
                        JsonSchema::QrCode(json) => {
                            Schema::QrCode(QrCode::from_json(json).unwrap())
                        }
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

    pub fn page_heihgt(&self) -> Mm {
        self.base_pdf.height
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
