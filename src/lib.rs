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
    pub fn render(&mut self, template_name: &str) -> Result<Vec<u8>, Error> {
        match self.template_map.get(template_name) {
            Some(template) => template.render_static(&mut self.doc, &self.font_map),
            None => Err(Error::Whatever {
                message: format!("Template not found: {}", template_name),
                source: None,
            }),
        }
    }

    /// Sets PDF metadata (title, author, etc.) in the generated PDF bytes
    pub fn set_pdf_metadata(pdf_bytes: Vec<u8>, title: Option<&str>, author: Option<&str>) -> Result<Vec<u8>, Error> {
        use lopdf::{Document, Object, dictionary};

        let mut doc = Document::load_mem(&pdf_bytes).map_err(|e| Error::Whatever {
            message: format!("Failed to load PDF for metadata setting: {}", e),
            source: None,
        })?;

        // Create Info dictionary
        let mut info_dict = dictionary! {};

        if let Some(title) = title {
            // Convert title to UTF-16BE for proper Unicode support
            let utf16_title = title
                .encode_utf16()
                .flat_map(|c| c.to_be_bytes())
                .collect::<Vec<u8>>();
            
            // Add BOM for UTF-16BE
            let mut title_bytes = vec![0xFE, 0xFF];
            title_bytes.extend(utf16_title);
            
            info_dict.set("Title", Object::String(title_bytes, lopdf::StringFormat::Literal));
        }

        if let Some(author) = author {
            // Convert author to UTF-16BE for proper Unicode support
            let utf16_author = author
                .encode_utf16()
                .flat_map(|c| c.to_be_bytes())
                .collect::<Vec<u8>>();
            
            // Add BOM for UTF-16BE
            let mut author_bytes = vec![0xFE, 0xFF];
            author_bytes.extend(utf16_author);
            
            info_dict.set("Author", Object::String(author_bytes, lopdf::StringFormat::Literal));
        }

        // Add producer info
        info_dict.set("Producer", Object::string_literal("PDForge"));

        // Add Info dictionary to document
        let info_id = doc.add_object(Object::Dictionary(info_dict));
        doc.trailer.set("Info", info_id);

        // Save modified document
        let mut output = Vec::new();
        doc.save_to(&mut output).map_err(|e| Error::Whatever {
            message: format!("Failed to save PDF with metadata: {}", e),
            source: None,
        })?;

        Ok(output)
    }

    pub fn render_with_inputs(
        &mut self,
        template_name: &str,
        inputs: Vec<Vec<HashMap<&'static str, String>>>,
    ) -> Result<Vec<u8>, Error> {
        match self.template_map.get(template_name) {
            Some(template) => template.render(&mut self.doc, &self.font_map, inputs),
            None => Err(Error::Whatever {
                message: format!("Template not found: {}", template_name),
                source: None,
            }),
        }
    }

    pub fn render_with_table_data(
        &mut self,
        template_name: &str,
        table_data: HashMap<String, Vec<Vec<String>>>,
    ) -> Result<Vec<u8>, Error> {
        match self.template_map.get(template_name) {
            Some(template) => {
                template.render_with_table_data(&mut self.doc, &self.font_map, table_data)
            }
            None => Err(Error::Whatever {
                message: format!("Template not found: {}", template_name),
                source: None,
            }),
        }
    }

    pub fn render_with_inputs_and_table_data(
        &mut self,
        template_name: &str,
        inputs: Vec<Vec<HashMap<&'static str, String>>>,
        table_data: HashMap<String, Vec<Vec<String>>>,
    ) -> Result<Vec<u8>, Error> {
        match self.template_map.get(template_name) {
            Some(template) => template.render_with_inputs_and_table_data(
                &mut self.doc,
                &self.font_map,
                inputs,
                table_data,
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

    pub fn add_font(mut self, font_name: &str, file_name: &str) -> Result<Self, Error> {
        let font_slice = std::fs::read(file_name).map_err(|e| Error::FontFileIo {
            source: e,
            message: format!("Failed to read font file: {}", file_name),
        })?;
        let parsed_font =
            ParsedFont::from_bytes(&font_slice, 0, &mut Vec::new()).ok_or_else(|| {
                Error::FontParsing {
                    message: format!("Failed to parse font file: {}", file_name),
                }
            })?;
        let font_id = self.doc.add_font(&parsed_font);
        self.font_map
            .add_font(String::from(font_name), font_id.clone(), &parsed_font);

        Ok(self)
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
