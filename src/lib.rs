pub mod common;
pub mod font;
pub mod schemas;
pub mod utils;
use printpdf::{ParsedFont, PdfDocument};
use schemas::Error;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PDForge {
    doc: PdfDocument,
    font_map: font::FontMap,
    template_map: HashMap<String, schemas::Template>,
}

impl PDForge {
    pub fn render(
        &mut self,
        template_name: &str,
        inputs: Vec<Vec<HashMap<&'static str, String>>>,
        table_data: Option<HashMap<&'static str, Vec<Vec<String>>>>,
        static_inputs: Option<HashMap<&'static str, String>>,
    ) -> Result<Vec<u8>, Error> {
        if inputs.is_empty() {
            return Err(Error::Whatever {
                message: "Inputs cannot be empty".to_string(),
                source: None,
            });
        }

        let table_data = table_data.unwrap_or_default();
        let static_inputs = static_inputs.unwrap_or_default();

        match self.template_map.get(template_name) {
            Some(template) => template.render_with_inputs_table_data_and_static_inputs(
                &mut self.doc,
                &self.font_map,
                inputs,
                table_data,
                static_inputs,
            ),
            None => Err(Error::Whatever {
                message: format!("Template not found: {}", template_name),
                source: None,
            }),
        }
    }
}

pub struct PDForgeBuilder {
    doc: PdfDocument,
    font_map: font::FontMap,
    template_map: HashMap<String, schemas::Template>,
}

impl PDForgeBuilder {
    pub fn new(name: String) -> Self {
        PDForgeBuilder {
            doc: PdfDocument::new(&name),
            font_map: font::FontMap::default(),
            template_map: HashMap::new(),
        }
    }

    pub fn add_font(mut self, font_name: &str, font_bytes: &[u8]) -> Result<Self, Error> {
        let parsed_font =
            ParsedFont::from_bytes(font_bytes, 0, &mut Vec::new()).ok_or_else(|| {
                Error::FontParsing {
                    message: format!("Failed to parse font bytes for: {}", font_name),
                }
            })?;
        let font_id = self.doc.add_font(&parsed_font);
        self.font_map
            .add_font(String::from(font_name), font_id.clone(), &parsed_font);

        Ok(self)
    }

    pub fn add_font_from_file(self, font_name: &str, file_path: &str) -> Result<Self, Error> {
        let font_bytes = std::fs::read(file_path).map_err(|e| Error::FontFileIo {
            source: e,
            message: format!("Failed to read font file: {}", file_path),
        })?;
        self.add_font(font_name, &font_bytes)
    }

    pub fn load_template(mut self, template_name: &str, template: &str) -> Result<Self, Error> {
        let template = schemas::Template::new(template)?;

        self.template_map
            .insert(template_name.to_string(), template);

        Ok(self)
    }

    pub fn build(self) -> PDForge {
        PDForge {
            doc: self.doc,
            font_map: self.font_map,
            template_map: self.template_map,
        }
    }
}
