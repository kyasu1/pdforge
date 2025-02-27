use crate::font::{DynamicFontSize, FontMap, FontSize, FontSpec};
use crate::schemas::base::BaseSchema;
use crate::schemas::{Error, FontSnafu, JsonPosition};
use crate::utils::OpBuffer;
use icu_segmenter::WordSegmenter;
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;
use std::cmp::max;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTextSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
    font_name: String,
    character_spacing: Option<f32>,
    line_height: Option<f32>,
    font_size: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct DynamicText {
    base: BaseSchema,
    content: String,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: Pt,
    font_id: FontId,
    font_spec: FontSpec,
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
            .find(font_name.clone())
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
            font_spec: font_spec.clone(),
        })
    }

    pub fn from_json(json: JsonTextSchema, font_map: &FontMap) -> Result<DynamicText, Error> {
        let (font_id, font) = font_map
            .find(json.font_name.clone())
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
            None => {
                unimplemented!()
            }
        };
        let text = Self {
            base,
            content: json.content,
            character_spacing,
            line_height,
            font_size,
            font_id: font_id.clone(),
            font_spec: font_spec.clone(),
        };

        Ok(text)
    }
    pub fn render(
        &mut self,
        page_height_in_mm: Mm,
        current_page: usize,
        current_top_mm: Option<Mm>,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error> {
        let top_margin_in_mm = Mm(20.0);
        let bottom_margin_in_mm = Mm(20.0);
        let page_height: Pt = page_height_in_mm.into();

        let mut index_offset = 0;
        let mut page_counter = 0;
        let mut y_top_mm: Mm = current_top_mm.unwrap_or(self.base.y);
        // let mut y_bottom_mm = self.schema.base.get_y() + self.schema.base.get_height();
        let y_bottom_mm = page_height_in_mm - bottom_margin_in_mm;

        let line_height = Pt(self.line_height.unwrap_or(1.0) * self.font_size.0);

        let character_spacing = self.character_spacing;

        let lines: Vec<String> = TextUtil::split_text_to_size(
            &self.font_spec,
            &self.content,
            self.font_size.clone(),
            self.base.width.into(),
            character_spacing,
        )?;

        let y_offset: Pt = Pt(0.0);
        let mut y_line: Pt = Pt(0.0);
        let lines_splitted_to_pages: Vec<Vec<String>> = lines.into_iter().enumerate().fold(
            Vec::new(),
            |mut acc: Vec<Vec<String>>, (index, current)| {
                let row_y_offset = self.font_size * (index as f32 - index_offset as f32);
                y_line = page_height - y_top_mm.into() - y_offset - row_y_offset;
                if y_line <= page_height - y_bottom_mm.into() {
                    page_counter += 1;
                    acc.push(vec![current.clone()]);
                    index_offset = index;
                    y_top_mm = top_margin_in_mm;
                } else {
                    if let Some(page) = acc.get_mut(page_counter) {
                        page.push(current.clone());
                    } else {
                        acc.push(vec![current.clone()]);
                    }
                }
                acc
            },
        );

        let next_y_line_mm = page_height_in_mm - y_line.into() + self.font_size.into();

        let mut pages_increased = 0;
        let mut y_start = page_height - (current_top_mm.unwrap_or(self.base.y).into());

        for (page_index, page) in lines_splitted_to_pages.into_iter().enumerate() {
            let mut ops: Vec<Op> = vec![
                Op::StartTextSection,
                Op::SetTextCursor {
                    pos: Point {
                        x: self.base.x.into(),
                        y: y_start,
                    },
                },
                Op::SetLineHeight { lh: line_height },
                Op::SetCharacterSpacing {
                    // multiplier: character_spacing.clone(),
                    multiplier: 0.0,
                },
            ];

            for line in page {
                let line_ops = vec![
                    Op::WriteText {
                        text: line.to_string(),
                        font: self.font_id.clone(),
                        size: self.font_size.clone(),
                    },
                    Op::AddLineBreak,
                ];
                ops.extend_from_slice(&line_ops);
            }

            ops.extend_from_slice(&[Op::EndTextSection]);
            buffer.insert(current_page + page_index, ops);
            pages_increased += 1;
            y_start = page_height - top_margin_in_mm.into();
        }
        Ok((
            current_page + pages_increased - 1,
            // Some((page_height - y_line + top_margin_in_mm.into()).into()),
            // Some(page_height_in_mm - y_line.into()),
            Some(next_y_line_mm),
        ))
    }

    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    base: BaseSchema,
    content: String,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: FontSize,
    font_id: FontId,
    font_spec: FontSpec,
}

impl Text {
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
            .find(font_name.clone())
            .whatever_context("Font specified in the schema is not loaded")?;
        let font_spec = FontSpec::new(font.clone());
        let base = BaseSchema::new(String::from("cell"), x, y, width, height);

        Ok(Self {
            base,
            content,
            character_spacing: Pt(0.0),
            line_height: None,
            font_size: FontSize::Fixed(font_size),
            font_id: font_id.clone(),
            font_spec: font_spec.clone(),
        })
    }

    pub fn from_json(json: JsonTextSchema, font_map: &FontMap) -> Result<Text, Error> {
        let (font_id, font) = font_map
            .find(json.font_name.clone())
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
            Some(f) => FontSize::Fixed(Pt(f)),
            None => {
                unimplemented!()
            }
        };
        let text = Text {
            base,
            content: json.content,
            character_spacing,
            line_height,
            font_size,
            font_id: font_id.clone(),
            font_spec: font_spec.clone(),
        };

        Ok(text)
    }

    pub fn render(
        &mut self,
        page_height_in_mm: Mm,
        current_page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let font_size = self.get_font_size()?;

        let splitted_paragraphs: Vec<String> = TextUtil::split_text_to_size(
            &self.font_spec,
            &self.content,
            font_size,
            self.base.width.into(),
            self.character_spacing,
        )?;

        let line_height = Pt(self.line_height.unwrap_or(1.0) * font_size.0);

        let mut ops: Vec<Op> = vec![
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: self.base.x.into(),
                    y: (page_height_in_mm - self.base.y).into(),
                },
            },
            Op::SetLineHeight { lh: line_height },
            Op::SetCharacterSpacing {
                // multiplier: character_spacing.clone(),
                multiplier: 0.0,
            },
        ];

        for line in splitted_paragraphs {
            let line_ops = vec![
                Op::WriteText {
                    text: line.to_string(),
                    font: self.font_id.clone(),
                    size: font_size.clone(),
                },
                Op::AddLineBreak,
            ];
            ops.extend_from_slice(&line_ops);
        }

        ops.extend_from_slice(&[Op::EndTextSection]);
        buffer.insert(current_page, ops);

        Ok(())
    }

    pub fn set_x(&mut self, x: Mm) {
        self.base.x = x;
    }
    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }
    pub fn set_width(&mut self, width: Mm) {
        self.base.width = width;
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }
    pub fn get_height(&self) -> Result<Mm, Error> {
        let font_size = self.get_font_size()?;

        let lines: Vec<String> = TextUtil::split_text_to_size(
            &self.font_spec,
            &self.content,
            font_size,
            self.base.width.into(),
            self.character_spacing,
        )?;

        Ok(Pt(lines.len() as f32 * font_size.0).into())
    }

    fn get_font_size(&self) -> Result<Pt, Error> {
        match self.font_size.clone() {
            FontSize::Fixed(font_size) => Ok(font_size),
            FontSize::Dynamic(dynamic) => TextUtil::calculate_dynamic_font_size(
                &self.font_spec,
                dynamic,
                self.line_height,
                self.character_spacing,
                self.base.width,
                self.base.height,
                &self.content,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextUtil {
    // schema: TextSchema,
    // font: FontSpec,
    // font_id: FontId,
}

impl TextUtil {
    // pub fn new(schema: &TextSchema, font_map: &FontMap) -> Result<Self, Error> {
    //     let (font_id, font) = font_map
    //         .find(schema.font_name.clone())
    //         .whatever_context("Font specified in the schema is not loaded")?;
    //
    //     Ok(Self {
    //         schema: schema.clone(),
    //         font: FontSpec::new(font),
    //         font_id: font_id.clone(),
    //     })
    // }
    // pub fn set_y(&mut self, y: Mm) {
    //     self.schema.base.set_y(y);
    // }
    //
    // pub fn adjust_width(&mut self, ratio: f32) {
    //     self.schema.base.adjust_width(ratio);
    // }
    //
    // pub fn get_width(&self) -> Mm {
    //     self.schema.base.width()
    // }
    //
    // pub fn get_height(&self) -> Result<Mm, Error> {
    //     let font_size: Pt = self.calculate_dynamic_font_size(
    //         &self.font,
    //         self.schema.base.width(),
    //         self.schema.base.height(),
    //     )?;
    //
    //     let lines: Vec<String> = Self::split_text_to_size(
    //         &self.font,
    //         self.schema.content.clone(),
    //         font_size,
    //         self.schema.base.width().into(),
    //         self.schema.character_spacing,
    //     )?;
    //
    //     Ok(Pt(lines.len() as f32 * font_size.0).into())
    // }
    // pub fn set_content(&mut self, content: String) {
    //     self.schema.content = content;
    // }

    fn split_text_to_size(
        font: &FontSpec,
        content: &str,
        font_size: Pt,
        box_width: Pt,
        character_spacing: Pt,
    ) -> Result<Vec<String>, Error> {
        let mut splitted_paragraphs: Vec<Vec<String>> = Vec::new();

        for paragraph in content.lines().collect::<Vec<&str>>() {
            let lines = Self::get_splitted_lines_by_segmenter(
                font,
                String::from(paragraph),
                font_size.clone(),
                box_width.into(),
                character_spacing.clone(),
            )?;
            splitted_paragraphs.push(lines);
        }

        Ok(splitted_paragraphs.concat())
    }

    fn get_splitted_lines_by_segmenter(
        font: &FontSpec,
        paragraph: String,
        font_size: Pt,
        box_width: Pt,
        character_spacing: Pt,
    ) -> Result<Vec<String>, Error> {
        let segments = Self::split_text_by_segmenter(paragraph);

        let mut lines: Vec<String> = vec![];
        let mut line_count: usize = 0;
        let mut current_text_size = Pt(0.0);

        for segment in segments.iter() {
            let text_width: Pt = font
                .width_of_text_at_size(segment.clone(), font_size, character_spacing)
                .context(FontSnafu)?;

            if current_text_size + text_width <= box_width {
                match lines.get(line_count) {
                    Some(current) => {
                        lines[line_count] = format!("{}{}", current, segment);
                        current_text_size = current_text_size + text_width + character_spacing
                    }
                    None => {
                        lines.push(String::from(segment));
                        current_text_size = text_width + character_spacing
                    }
                }
            } else if segment.is_empty() {
                line_count += 1;
                lines.push(String::from(""));
                current_text_size = Pt(0.0);
            } else if text_width <= box_width {
                line_count += 1;
                lines.push(String::from(segment));
                current_text_size = text_width + character_spacing;
            } else {
                for char in segment.chars() {
                    let size = font
                        .width_of_text_at_size(
                            char.clone().to_string(),
                            font_size,
                            character_spacing,
                        )
                        .context(FontSnafu)?;

                    if current_text_size + size <= box_width {
                        match lines.get(line_count) {
                            Some(current) => {
                                lines[line_count] = format!("{}{}", current, char);
                                current_text_size = current_text_size + size + character_spacing;
                            }
                            None => {
                                lines.push(char.to_string());
                                current_text_size = size + character_spacing;
                            }
                        }
                    } else {
                        line_count += 1;
                        lines.push(char.to_string());
                        current_text_size = size + character_spacing;
                    }
                }
            }
        }

        Ok(lines)
    }

    fn split_text_by_segmenter(paragraph: String) -> Vec<String> {
        let segmenter = WordSegmenter::new_auto();
        let breakpoints: Vec<usize> = segmenter.segment_str(&paragraph).collect();

        let mut segments: Vec<String> = vec![];
        let mut last_breakpoint = 0;
        for breakpoint in breakpoints {
            segments.push(paragraph[last_breakpoint..breakpoint].to_string());
            last_breakpoint = breakpoint;
        }

        if segments.is_empty() {
            vec![]
        } else {
            segments.remove(0);
            segments
        }
    }

    fn calculate_dynamic_font_size(
        font: &FontSpec,
        dynamic: DynamicFontSize,
        line_height: Option<f32>,
        character_spacing: Pt,
        width: Mm,
        height: Mm,
        content: &str,
    ) -> Result<Pt, Error> {
        let mut font_size = dynamic.get_min();
        let (mut total_width_in_mm, mut total_height_in_mm) = Self::calculate_constraints(
            font,
            font_size,
            line_height,
            character_spacing,
            width.clone(),
            dynamic.clone(),
            content,
        )?;

        while dynamic.should_font_grow_to_fit(
            font_size.clone(),
            width,
            height,
            total_width_in_mm,
            total_height_in_mm,
        ) {
            font_size += Pt(0.25);
            let (new_total_width_in_mm, new_total_height_in_mm) = Self::calculate_constraints(
                font,
                font_size.clone(),
                line_height,
                character_spacing,
                width.clone(),
                dynamic.clone(),
                content,
            )?;
            if new_total_height_in_mm <= total_height_in_mm {
                total_width_in_mm = new_total_width_in_mm;
                total_height_in_mm = new_total_height_in_mm
            } else {
                font_size -= Pt(0.25);
                break;
            }
        }

        while dynamic.should_font_shrink_to_fit(
            font_size.clone(),
            width.clone(),
            height.clone(),
            total_width_in_mm,
            total_height_in_mm,
        ) {
            font_size -= Pt(0.25);
            (total_width_in_mm, total_height_in_mm) = Self::calculate_constraints(
                font,
                font_size,
                line_height,
                character_spacing,
                width.clone(),
                dynamic.clone(),
                content,
            )?;
        }

        Ok(font_size)
    }

    fn calculate_constraints(
        font: &FontSpec,
        font_size: Pt,
        line_height: Option<f32>,
        character_spacing: Pt,
        width: Mm,
        dynamic: DynamicFontSize,
        content: &str,
    ) -> Result<(Mm, Mm), Error> {
        let mut total_width_in_mm: Mm = Mm(0.0);
        let mut total_height_in_mm: Mm = Mm(0.0);
        let line_height: f32 = line_height.unwrap_or(1.0);
        let first_line_text_height: Pt = font.height_of_font_at_size(font_size);
        let first_line_height_in_mm: Mm = (first_line_text_height * line_height).into();
        let other_row_height_in_mm: Mm = (font_size * line_height).into();
        let character_spacing = character_spacing;
        let splitted_paragraphs = Self::split_text_to_size(
            font,
            content,
            font_size,
            width.clone().into(),
            character_spacing,
        )?;

        for (line_index, line) in splitted_paragraphs.into_iter().enumerate() {
            if dynamic.is_fit_verfical() {
                let text_width = font
                    .width_of_text_at_size(line.replace("\n", ""), font_size, character_spacing)
                    .context(FontSnafu)?;
                total_width_in_mm = max(total_width_in_mm, text_width.into());
            }
            if line_index == 0 {
                total_height_in_mm += first_line_height_in_mm;
            } else {
                total_height_in_mm += other_row_height_in_mm;
            }

            if dynamic.is_fit_horizontal() {
                let text_width = font
                    .width_of_text_at_size(line.to_string(), font_size, character_spacing)
                    .context(FontSnafu)?;
                total_width_in_mm = max(total_width_in_mm, text_width.into());
            }
        }

        Ok((total_width_in_mm, total_height_in_mm))
    }

    // pub fn draw(
    //     &mut self,
    //     page_height_in_mm: Mm,
    //     current_page: usize,
    //     buffer: &mut OpBuffer,
    // ) -> Result<(), Error> {
    //     let font_size: Pt = self.calculate_dynamic_font_size(
    //         &self.font,
    //         self.schema.base.width(),
    //         self.schema.base.height(),
    //     )?;
    //
    //     let character_spacing = self.schema.character_spacing;
    //
    //     let splitted_paragraphs: Vec<String> = Self::split_text_to_size(
    //         &self.font,
    //         self.schema.content.clone(),
    //         font_size,
    //         self.schema.base.width().into(),
    //         character_spacing,
    //     )?;
    //
    //     let line_height = Pt(self.schema.line_height.unwrap_or(1.0) * font_size.0);
    //
    //     let mut ops: Vec<Op> = vec![
    //         Op::StartTextSection,
    //         Op::SetTextCursor {
    //             pos: Point {
    //                 x: self.schema.base.x().into(),
    //                 y: (page_height_in_mm - self.schema.base.y()).into(),
    //             },
    //         },
    //         Op::SetLineHeight { lh: line_height },
    //         Op::SetCharacterSpacing {
    //             // multiplier: character_spacing.clone(),
    //             multiplier: 0.0,
    //         },
    //     ];
    //
    //     for line in splitted_paragraphs {
    //         let line_ops = vec![
    //             Op::WriteText {
    //                 text: line.to_string(),
    //                 font: self.font_id.clone(),
    //                 size: font_size.clone(),
    //             },
    //             Op::AddLineBreak,
    //         ];
    //         ops.extend_from_slice(&line_ops);
    //     }
    //
    //     ops.extend_from_slice(&[Op::EndTextSection]);
    //     buffer.insert(current_page, ops);
    //
    //     Ok(())
    // }
    //
    // pub fn draw2(
    //     &mut self,
    //     page_height_in_mm: Mm,
    //     current_page: usize,
    //     current_top_mm: Option<Mm>,
    //     buffer: &mut OpBuffer,
    // ) -> Result<(usize, Option<Mm>), Error> {
    //     let top_margin_in_mm = Mm(20.0);
    //     let bottom_margin_in_mm = Mm(20.0);
    //     let page_height: Pt = page_height_in_mm.into();
    //
    //     let mut index_offset = 0;
    //     let mut page_counter = 0;
    //     let mut y_top_mm: Mm = current_top_mm.unwrap_or(self.schema.base.y());
    //     // let mut y_bottom_mm = self.schema.base.get_y() + self.schema.base.get_height();
    //     let y_bottom_mm = page_height_in_mm - bottom_margin_in_mm;
    //
    //     let font_size: Pt = self.calculate_dynamic_font_size(
    //         &self.font,
    //         self.schema.base.width(),
    //         self.schema.base.height(),
    //     )?;
    //
    //     let line_height = Pt(self.schema.line_height.unwrap_or(1.0) * font_size.0);
    //
    //     let character_spacing = self.schema.character_spacing;
    //
    //     let lines: Vec<String> = Self::split_text_to_size(
    //         &self.font,
    //         self.schema.content.clone(),
    //         font_size,
    //         self.schema.base.width().into(),
    //         character_spacing,
    //     )?;
    //
    //     let y_offset: Pt = Pt(0.0);
    //     let mut y_line: Pt = Pt(0.0);
    //     let lines_splitted_to_pages: Vec<Vec<String>> = lines.into_iter().enumerate().fold(
    //         Vec::new(),
    //         |mut acc: Vec<Vec<String>>, (index, current)| {
    //             let row_y_offset = font_size * (index as f32 - index_offset as f32);
    //             y_line = page_height - y_top_mm.into() - y_offset - row_y_offset;
    //             if y_line <= page_height - y_bottom_mm.into() {
    //                 page_counter += 1;
    //                 acc.push(vec![current.clone()]);
    //                 index_offset = index;
    //                 y_top_mm = top_margin_in_mm;
    //             } else {
    //                 if let Some(page) = acc.get_mut(page_counter) {
    //                     page.push(current.clone());
    //                 } else {
    //                     acc.push(vec![current.clone()]);
    //                 }
    //             }
    //             acc
    //         },
    //     );
    //
    //     let next_y_line_mm = page_height_in_mm - y_line.into() + font_size.into();
    //
    //     let mut pages_increased = 0;
    //     let mut y_start = page_height - (current_top_mm.unwrap_or(self.schema.base.y()).into());
    //
    //     for (page_index, page) in lines_splitted_to_pages.into_iter().enumerate() {
    //         let mut ops: Vec<Op> = vec![
    //             Op::StartTextSection,
    //             Op::SetTextCursor {
    //                 pos: Point {
    //                     x: self.schema.base.x().into(),
    //                     y: y_start,
    //                 },
    //             },
    //             Op::SetLineHeight { lh: line_height },
    //             Op::SetCharacterSpacing {
    //                 // multiplier: character_spacing.clone(),
    //                 multiplier: 0.0,
    //             },
    //         ];
    //
    //         for line in page {
    //             let line_ops = vec![
    //                 Op::WriteText {
    //                     text: line.to_string(),
    //                     font: self.font_id.clone(),
    //                     size: font_size.clone(),
    //                 },
    //                 Op::AddLineBreak,
    //             ];
    //             ops.extend_from_slice(&line_ops);
    //         }
    //
    //         ops.extend_from_slice(&[Op::EndTextSection]);
    //         buffer.insert(current_page + page_index, ops);
    //         pages_increased += 1;
    //         y_start = page_height - top_margin_in_mm.into();
    //     }
    //     Ok((
    //         current_page + pages_increased - 1,
    //         // Some((page_height - y_line + top_margin_in_mm.into()).into()),
    //         // Some(page_height_in_mm - y_line.into()),
    //         Some(next_y_line_mm),
    //     ))
    // }
}
