use std::sync::Arc;

use crate::font::{FontMap, FontSpec, FontSpecTrait, LineBreakMode};
use crate::schemas::base::BaseSchema;
use crate::schemas::{Error, FontSnafu, HasBaseSchema, JsonPosition};
use crate::utils::OpBuffer;
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;

use super::BasePdf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonDynamicTextSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
    font_name: String,
    character_spacing: Option<f32>,
    line_height: Option<f32>,
    font_size: Option<f32>,
    line_break_mode: Option<LineBreakMode>,
}

#[derive(Debug, Clone)]
pub struct DynamicText {
    base: BaseSchema,
    content: String,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: Pt,
    font_id: FontId,
    font_spec: Arc<dyn FontSpecTrait>,
    font: Arc<ParsedFont>,
    line_break_mode: Option<LineBreakMode>,
}

impl DynamicText {
    pub fn new(
        x: Mm,
        y: Mm,
        width: Mm,
        height: Mm,
        font_name: String,
        font_size: Pt,
        content: String,
        font_map: &FontMap,
    ) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .find(&font_name)
            .whatever_context("Font specified in the schema is not loaded")?;
        let font_spec = FontSpec::new(font.clone());
        let base = BaseSchema::new(String::from("cell"), x, y, width, height);

        Ok(Self {
            base,
            content,
            character_spacing: Pt(0.0),
            line_height: None,
            font_size,
            font_id: font_id.clone(),
            font_spec: Arc::new(font_spec.clone()),
            font: font.clone(),
            line_break_mode: None,
        })
    }

    pub fn from_json(json: JsonDynamicTextSchema, font_map: &FontMap) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .find(&json.font_name)
            .whatever_context("Font specified in the schema is not loaded")?;
        let font_spec = FontSpec::new(font.clone());

        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );

        let character_spacing = json.character_spacing.map(|f| Pt(f)).unwrap_or(Pt(0.0));
        let line_height = json.line_height;
        let font_size = match json.font_size {
            Some(f) => Pt(f),
            None => Pt(12.0),
        };
        let line_break_mode = json.line_break_mode;
        let text = Self {
            base,
            content: json.content,
            character_spacing,
            line_height,
            font_size,
            font_id: font_id.clone(),
            font_spec: Arc::new(
                font_spec.with_line_break_mode(line_break_mode.unwrap_or_default()),
            ),
            font: font.clone(),
            line_break_mode,
        };

        Ok(text)
    }

    pub fn get_base(&self) -> BaseSchema {
        self.base.clone()
    }

    pub fn render(
        &mut self,
        base_pdf: &BasePdf,
        current_page: usize,
        current_top_mm: Option<Mm>,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error> {
        println!("current_top_mm {:?}", current_top_mm);

        let top_margin_in_mm = base_pdf.padding.top;
        let bottom_margin_in_mm = base_pdf.padding.bottom;
        let page_height_in_mm = base_pdf.height;

        let mut page_counter = 0;
        let y_top_mm: Mm = current_top_mm.unwrap_or(self.base.y);
        let mut y_line_mm: Mm = y_top_mm;
        let y_bottom_mm = base_pdf.height - bottom_margin_in_mm;
        let line_height_in_mm: Mm = Pt(self.line_height.unwrap_or(1.0) * self.font_size.0).into();
        let character_spacing = self.character_spacing;

        let lines: Vec<String> = self
            .font_spec
            .split_text_to_size(
                &self.content,
                self.font_size,
                self.base.width.into(),
                character_spacing,
            )
            .context(FontSnafu)?;

        let mut pages: Vec<Vec<String>> = Vec::new();

        for line in lines.into_iter() {
            y_line_mm += line_height_in_mm;

            if y_line_mm > y_bottom_mm {
                page_counter += 1;
                y_line_mm = top_margin_in_mm;
            }

            if let Some(page) = pages.get_mut(page_counter) {
                page.push(line.clone());
            } else {
                pages.insert(page_counter, vec![line.clone()]);
            }
        }

        let next_y_line_mm = y_line_mm;

        let mut y_start = y_top_mm;

        let pages_increased = pages.len() - 1;

        for (page_index, page) in pages.into_iter().enumerate() {
            let mut ops: Vec<Op> = Vec::new();

            for (index, line) in page.into_iter().enumerate() {
                let y_position =
                    page_height_in_mm - y_start - line_height_in_mm * (index + 1) as f32;

                let matrix = super::pdf_utils::calculate_transform_matrix_with_center_pivot(
                    self.base.x,
                    self.base.y,
                    self.base.width,
                    self.base.height,
                    None,
                );

                let line_ops = super::pdf_utils::create_text_ops_with_font(
                    matrix,
                    &self.font_id,
                    self.font_size,
                    self.base.x,
                    y_position,
                    None,
                    None,
                    character_spacing,
                    &line,
                    self.line_height,
                    &csscolorparser::parse("#000000").map_err(|e| Error::ColorParsing {
                        source: e,
                        message: "Failed to parse default black color".to_string(),
                    })?, // デフォルトの黒色
                    Some(&self.font),
                );

                ops.extend_from_slice(&line_ops);
            }
            buffer.insert(current_page + page_index, ops);
            y_start = top_margin_in_mm;
        }
        Ok((current_page + pages_increased, Some(next_y_line_mm)))
    }

    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }

    pub fn set_height(&mut self, height: Mm) {
        self.base.height = height;
    }

    pub fn has_line_break_mode(&self) -> bool {
        self.line_break_mode.is_some()
    }

    pub fn line_break_mode(&self) -> Option<LineBreakMode> {
        self.line_break_mode
    }

    pub fn resolved_line_break_mode(&self) -> LineBreakMode {
        self.line_break_mode.unwrap_or_default()
    }

    pub fn set_line_break_mode(&mut self, line_break_mode: Option<LineBreakMode>) {
        self.line_break_mode = line_break_mode;
        self.font_spec = Arc::new(
            FontSpec::new(self.font.clone()).with_line_break_mode(self.resolved_line_break_mode()),
        );
    }
}

impl HasBaseSchema for DynamicText {
    fn base(&self) -> &BaseSchema {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BaseSchema {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontMap;
    use crate::schemas::Frame;
    use printpdf::{Op, ParsedFont, PdfDocument, TextItem};
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::{Arc, OnceLock};

    fn test_font() -> Arc<ParsedFont> {
        static FONT: OnceLock<Arc<ParsedFont>> = OnceLock::new();

        FONT.get_or_init(|| {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("fonts")
                .join("NotoSansJP-Regular.ttf");
            let font_bytes = std::fs::read(path).expect("test font should be readable");
            let parsed_font = ParsedFont::from_bytes(&font_bytes, 0, &mut Vec::new())
                .expect("test font should parse");
            Arc::new(parsed_font)
        })
        .clone()
    }

    fn test_font_map() -> FontMap {
        let mut doc = PdfDocument::new("test");
        let parsed_font = test_font();
        let font_id = doc.add_font(parsed_font.as_ref());
        let mut font_map = FontMap::default();
        font_map.add_font("TestFont".to_string(), font_id, parsed_font.as_ref());
        font_map
    }

    #[test]
    fn dynamic_text_from_json_defaults_line_break_mode_to_word() {
        let font_map = test_font_map();
        let json: JsonDynamicTextSchema = serde_json::from_value(json!({
            "name": "dynamicText",
            "position": { "x": 0.0, "y": 0.0 },
            "width": 80.0,
            "height": 20.0,
            "content": "abc def",
            "fontName": "TestFont",
            "fontSize": 12.0
        }))
        .unwrap();

        let text = DynamicText::from_json(json, &font_map).unwrap();

        assert!(!text.has_line_break_mode());
        assert_eq!(text.line_break_mode(), None);
        assert_eq!(text.resolved_line_break_mode(), LineBreakMode::Word);
    }

    #[test]
    fn dynamic_text_from_json_accepts_char_line_break_mode() {
        let font_map = test_font_map();
        let json: JsonDynamicTextSchema = serde_json::from_value(json!({
            "name": "dynamicText",
            "position": { "x": 0.0, "y": 0.0 },
            "width": 80.0,
            "height": 20.0,
            "content": "abc def",
            "fontName": "TestFont",
            "fontSize": 12.0,
            "lineBreakMode": "char"
        }))
        .unwrap();

        let text = DynamicText::from_json(json, &font_map).unwrap();

        assert!(text.has_line_break_mode());
        assert_eq!(text.line_break_mode(), Some(LineBreakMode::Char));
        assert_eq!(text.resolved_line_break_mode(), LineBreakMode::Char);
    }

    #[test]
    fn dynamic_text_render_sanitizes_unsupported_emoji_cluster() {
        let font_map = test_font_map();
        let json: JsonDynamicTextSchema = serde_json::from_value(json!({
            "name": "dynamicText",
            "position": { "x": 0.0, "y": 0.0 },
            "width": 80.0,
            "height": 20.0,
            "content": "a👍🏽b",
            "fontName": "TestFont",
            "fontSize": 12.0
        }))
        .unwrap();
        let mut text = DynamicText::from_json(json, &font_map).unwrap();
        let base_pdf = BasePdf {
            width: Mm(100.0),
            height: Mm(100.0),
            padding: Frame {
                top: Mm(0.0),
                right: Mm(0.0),
                bottom: Mm(0.0),
                left: Mm(0.0),
            },
            static_schema: vec![],
        };
        let mut buffer = OpBuffer::default();

        text.render(&base_pdf, 0, None, &mut buffer).unwrap();

        let rendered = buffer.buffer[0]
            .iter()
            .find_map(|op| match op {
                Op::ShowText { items } => Some(
                    items
                        .iter()
                        .filter_map(|item| match item {
                            TextItem::Text(text) => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<String>(),
                ),
                _ => None,
            })
            .expect("render should emit ShowText");

        assert_eq!(rendered.chars().count(), 3);
        assert!(rendered.starts_with('a'));
        assert!(rendered.ends_with('b'));
        assert!(!rendered.contains('👍'));
        assert!(!rendered.contains('🏽'));
    }
}
