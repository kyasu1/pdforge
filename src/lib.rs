pub mod common;
pub mod font;
pub mod schemas;
pub mod utils;
use printpdf::{FontId, ParsedFont, PdfDocument, PdfFontParseWarning};
use schemas::Error;
use std::collections::HashMap;
use std::sync::Arc;

/// Formats a `Error::FontParsing` message for a failed `ParsedFont::from_bytes`
/// call, including any parser warnings (e.g. an out-of-range `font_index` for
/// a `.ttc`/`.otc` collection) as supporting detail.
fn format_font_parse_error(
    font_name: &str,
    font_index: usize,
    warnings: &[PdfFontParseWarning],
) -> String {
    if warnings.is_empty() {
        format!(
            "Failed to parse font bytes for: {} (font_index: {})",
            font_name, font_index
        )
    } else {
        let details = warnings
            .iter()
            .map(|w| w.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "Failed to parse font bytes for: {} (font_index: {}): {}",
            font_name, font_index, details
        )
    }
}

#[derive(Debug, Clone)]
pub struct PDForge {
    name: String,
    font_map: font::FontMap,
    template_map: HashMap<String, schemas::Template>,
}

impl PDForge {
    pub fn render(
        &self,
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
            Some(template) => {
                let mut doc = PdfDocument::new(&self.name);
                let font_map = self.font_map.register_fonts_for_document(&mut doc);
                template.render_with_inputs_table_data_and_static_inputs(
                    &mut doc,
                    &font_map,
                    inputs,
                    table_data,
                    static_inputs,
                )
            }
            None => Err(Error::Whatever {
                message: format!("Template not found: {}", template_name),
                source: None,
            }),
        }
    }
}

pub struct PDForgeBuilder {
    name: String,
    font_map: font::FontMap,
    template_map: HashMap<String, schemas::Template>,
}

impl PDForgeBuilder {
    pub fn new(name: String) -> Self {
        PDForgeBuilder {
            name,
            font_map: font::FontMap::default(),
            template_map: HashMap::new(),
        }
    }

    /// Loads a font from a byte slice and registers it under `font_name`.
    ///
    /// Equivalent to `add_font_with_index(font_name, font_bytes, 0)`. This is
    /// the right choice for plain single-face `.ttf`/`.otf` fonts; for a
    /// TrueType/OpenType Collection (`.ttc`/`.otc`) that bundles multiple
    /// faces, use [`Self::add_font_with_index`] to select a face other than
    /// the first.
    pub fn add_font(self, font_name: &str, font_bytes: &[u8]) -> Result<Self, Error> {
        self.add_font_with_index(font_name, font_bytes, 0)
    }

    /// Loads a font from a byte slice, selecting `font_index` as the face to
    /// use, and registers it under `font_name`.
    ///
    /// `font_index` selects the face inside a TrueType/OpenType Collection
    /// (`.ttc`/`.otc`); pass `0` for a plain single-face `.ttf`/`.otf`. To
    /// use multiple faces from the same collection file, call this method
    /// once per face with a distinct `font_name` and the desired index.
    pub fn add_font_with_index(
        mut self,
        font_name: &str,
        font_bytes: &[u8],
        font_index: usize,
    ) -> Result<Self, Error> {
        let mut warnings: Vec<PdfFontParseWarning> = Vec::new();
        let parsed_font = ParsedFont::from_bytes(font_bytes, font_index, &mut warnings)
            .ok_or_else(|| Error::FontParsing {
                message: format_font_parse_error(font_name, font_index, &warnings),
            })?;
        self.font_map.add_font_arc(
            String::from(font_name),
            FontId::new(),
            Arc::new(parsed_font),
        );

        Ok(self)
    }

    /// Loads a font from a file and registers it under `font_name`.
    ///
    /// Equivalent to `add_font_from_file_with_index(font_name, file_path, 0)`.
    /// This is the right choice for plain single-face `.ttf`/`.otf` fonts;
    /// for a TrueType/OpenType Collection (`.ttc`/`.otc`) that bundles
    /// multiple faces, use [`Self::add_font_from_file_with_index`] to select
    /// a face other than the first.
    pub fn add_font_from_file(self, font_name: &str, file_path: &str) -> Result<Self, Error> {
        self.add_font_from_file_with_index(font_name, file_path, 0)
    }

    /// Loads a font from a file, selecting `font_index` as the face to use,
    /// and registers it under `font_name`.
    ///
    /// `font_index` selects the face inside a TrueType/OpenType Collection
    /// (`.ttc`/`.otc`); pass `0` for a plain single-face `.ttf`/`.otf`. To
    /// use multiple faces from the same collection file, call this method
    /// once per face with a distinct `font_name` and the desired index.
    pub fn add_font_from_file_with_index(
        self,
        font_name: &str,
        file_path: &str,
        font_index: usize,
    ) -> Result<Self, Error> {
        let font_bytes = std::fs::read(file_path).map_err(|e| Error::FontFileIo {
            source: e,
            message: format!("Failed to read font file: {}", file_path),
        })?;
        self.add_font_with_index(font_name, &font_bytes, font_index)
    }

    pub fn load_template(mut self, template_name: &str, template: &str) -> Result<Self, Error> {
        let template = schemas::Template::new(template)?;

        self.template_map
            .insert(template_name.to_string(), template);

        Ok(self)
    }

    pub fn build(self) -> PDForge {
        PDForge {
            name: self.name,
            font_map: self.font_map,
            template_map: self.template_map,
        }
    }
}
