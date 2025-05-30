pub mod common;
pub mod font;
pub mod schemas;
pub mod utils;
use printpdf::{ParsedFont, PdfDocument};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PDForge {
    doc: PdfDocument,
    font_map: font::FontMap,
    template_map: HashMap<String, schemas::Template>,
}

impl PDForge {
    pub fn render(&mut self, template_name: &str) -> Vec<u8> {
        match self.template_map.get(template_name) {
            Some(template) => template
                .render_static(&mut self.doc, &self.font_map)
                .unwrap(),
            None => {
                println!("Template not found: {}", template_name);
                Vec::new()
            }
        }
    }

    pub fn render_with_inputs(
        &mut self,
        template_name: &str,
        inputs: Vec<Vec<HashMap<&'static str, String>>>,
    ) -> Vec<u8> {
        match self.template_map.get(template_name) {
            Some(template) => template
                .render(&mut self.doc, &self.font_map, inputs)
                .unwrap(),
            None => {
                println!("Template not found: {}", template_name);
                Vec::new()
            }
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

    pub fn add_font(mut self, font_name: &str, file_name: &str) -> Self {
        let font_slice = std::fs::read(file_name).unwrap();
        let parsed_font = ParsedFont::from_bytes(&font_slice, 0, &mut Vec::new()).unwrap();
        let font_id = self.doc.add_font(&parsed_font);
        self.font_map
            .add_font(String::from(font_name), font_id.clone(), &parsed_font);

        self
    }

    pub fn load_template(mut self, template_name: &str, template: &str) -> Self {
        let template = schemas::Template::new(template).unwrap();

        self.template_map
            .insert(template_name.to_string(), template);

        self
    }

    pub fn build(self) -> PDForge {
        PDForge {
            doc: self.doc,
            font_map: self.font_map,
            template_map: self.template_map,
        }
    }
}
