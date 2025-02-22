use crate::font::*;
use crate::schemas::base::BaseSchema;
use crate::utils::OpBuffer;
use derive_new::new;
use icu_segmenter::WordSegmenter;
use printpdf::*;
use std::cmp::max;

#[derive(Debug, Clone, new)]
pub struct TextSchema {
    base: BaseSchema,
    content: String,
    font_name: String,
    character_spacing: Option<Pt>,
    line_height: Option<Pt>,
    font_size: FontSize,
}

impl TextSchema {
    pub fn flowing(
        name: String,
        content: String,
        font_name: String,
        font_size: Pt,
        x: f32,
        y: f32,
        width: f32,
    ) -> Self {
        Self {
            base: BaseSchema::flowing(name, String::from("text"), x, y, width),
            content,
            font_name,
            character_spacing: None,
            line_height: None,
            font_size: FontSize::Fixed(font_size),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    schema: TextSchema,
    font: FontSpec,
    font_id: FontId,
}

impl Text {
    pub fn new(schema: TextSchema, font_map: &FontMap) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .find(schema.font_name.clone())
            .ok_or(Error::FontNotFound)?;

        Ok(Self {
            schema,
            font: FontSpec::new(font),
            font_id: font_id.clone(),
        })
    }

    fn split_text_to_size(
        font: &FontSpec,
        content: String,
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
            let text_width =
                font.width_of_text_at_size(segment.clone(), font_size, character_spacing)?;

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
                current_text_size = Pt(text_width.0 + character_spacing.0);
            } else {
                for char in segment.chars() {
                    let size = font.width_of_text_at_size(
                        char.clone().to_string(),
                        font_size,
                        character_spacing,
                    )?;

                    if current_text_size.0 + size.0 <= box_width.0 {
                        match lines.get(line_count) {
                            Some(current) => {
                                lines[line_count] = format!("{}{}", current, segment);
                                current_text_size = current_text_size + size + character_spacing;
                            }
                            None => {
                                lines.push(char.to_string());
                                current_text_size = size + character_spacing;
                            }
                        }
                    } else {
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
        &self,
        font: &FontSpec,
        width: Mm,
        height: Option<Mm>,
    ) -> Result<Pt, Error> {
        match self.schema.font_size.clone() {
            FontSize::Fixed(font_size) => Ok(font_size),
            FontSize::Dynamic(dynamic) => {
                let height = match height {
                    Some(height) => height,
                    None => return Err(Error::InvalidFontSize),
                };
                let mut font_size = dynamic.get_min();
                let (mut total_width_in_mm, mut total_height_in_mm) = self.calculate_constraints(
                    font,
                    font_size,
                    width.clone(),
                    height.clone(),
                    dynamic.clone(),
                )?;

                while dynamic.should_font_grow_to_fit(
                    font_size.clone(),
                    width,
                    height,
                    total_width_in_mm,
                    total_height_in_mm,
                ) {
                    font_size += Pt(0.25);
                    let (new_total_width_in_mm, new_total_height_in_mm) = self
                        .calculate_constraints(
                            font,
                            font_size.clone(),
                            width.clone(),
                            height.clone(),
                            dynamic.clone(),
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
                    (total_width_in_mm, total_height_in_mm) = self.calculate_constraints(
                        font,
                        font_size,
                        width.clone(),
                        height.clone(),
                        dynamic.clone(),
                    )?;
                }

                Ok(font_size)
            }
        }
    }

    fn calculate_constraints(
        &self,
        font: &FontSpec,
        font_size: Pt,
        width: Mm,
        height: Mm,
        dynamic: DynamicFontSize,
    ) -> Result<(Mm, Mm), Error> {
        let mut total_width_in_mm: Mm = Mm(0.0);
        let mut total_height_in_mm: Mm = Mm(0.0);
        let line_height = self.schema.line_height.unwrap_or(Pt(1.0));
        let first_line_text_height: Pt = font.height_of_font_at_size(font_size);
        let first_line_height_in_mm: Mm = (first_line_text_height * line_height.0).into();
        let other_row_height_in_mm: Mm = (font_size * line_height.0).into();
        let character_spacing = self.schema.character_spacing.unwrap_or(Pt(0.0));
        let splitted_paragraphs = Self::split_text_to_size(
            font,
            self.schema.content.clone(),
            font_size,
            width.clone().into(),
            character_spacing,
        )?;

        for (line_index, line) in splitted_paragraphs.into_iter().enumerate() {
            if dynamic.is_fit_verfical() {
                let text_width = font.width_of_text_at_size(
                    line.replace("\n", ""),
                    font_size,
                    character_spacing,
                )?;
                total_width_in_mm = max(total_width_in_mm, text_width.into());
            }
            if line_index == 0 {
                total_height_in_mm += first_line_height_in_mm;
            } else {
                total_height_in_mm += other_row_height_in_mm;
            }

            if dynamic.is_fit_horizontal() {
                let text_width =
                    font.width_of_text_at_size(line.to_string(), font_size, character_spacing)?;
                total_width_in_mm = max(total_width_in_mm, text_width.into());
            }
        }

        Ok((total_width_in_mm, total_height_in_mm))
    }

    pub fn draw(
        &mut self,
        page_height: Pt,
        current_page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let font_size: Pt = self
            .calculate_dynamic_font_size(
                &self.font,
                self.schema.base.width(),
                self.schema.base.height(),
            )
            .unwrap();
        let character_spacing = self.schema.character_spacing.unwrap_or(Pt(0.0));

        let splitted_paragraphs: Vec<String> = Self::split_text_to_size(
            &self.font,
            self.schema.content.clone(),
            font_size,
            self.schema.base.width().into(),
            character_spacing,
        )?;

        let mut ops: Vec<Op> = vec![
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: self.schema.base.x().into(),
                    y: page_height - self.schema.base.y().into(),
                },
            },
            Op::SetLineHeight {
                lh: self.schema.line_height.unwrap_or(font_size),
            },
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

    pub fn draw2(
        &mut self,
        page_height_in_mm: Mm,
        current_page: usize,
        current_top_mm: Option<Mm>,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Mm), Error> {
        let top_margin_in_mm = Mm(20.0);
        let bottom_margin_in_mm = Mm(20.0);
        let page_height: Pt = page_height_in_mm.into();

        let mut index_offset = 0;
        let mut page_counter = 0;
        let mut y_top_mm: Mm = current_top_mm.unwrap_or(self.schema.base.y());
        // let mut y_bottom_mm = self.schema.base.get_y() + self.schema.base.get_height();
        let y_bottom_mm = page_height_in_mm - bottom_margin_in_mm;

        let font_size: Pt = self
            .calculate_dynamic_font_size(
                &self.font,
                self.schema.base.width(),
                self.schema.base.height(),
            )
            .unwrap();
        let line_height = self.schema.line_height.unwrap_or(font_size);
        let character_spacing = self.schema.character_spacing.unwrap_or(Pt(0.0));

        let lines: Vec<String> = Self::split_text_to_size(
            &self.font,
            self.schema.content.clone(),
            font_size,
            self.schema.base.width().into(),
            character_spacing,
        )?;

        let y_offset: Pt = Pt(0.0);
        let mut y_line: Pt = Pt(0.0);
        let lines_splitted_to_pages: Vec<Vec<String>> = lines.into_iter().enumerate().fold(
            Vec::new(),
            |mut acc: Vec<Vec<String>>, (index, current)| {
                let row_y_offset = font_size * (index as f32 - index_offset as f32);
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

        let mut pages_increased = 0;
        let mut y_start = page_height - (current_top_mm.unwrap_or(self.schema.base.y()).into());

        for (page_index, page) in lines_splitted_to_pages.into_iter().enumerate() {
            let mut ops: Vec<Op> = vec![
                Op::StartTextSection,
                Op::SetTextCursor {
                    pos: Point {
                        x: self.schema.base.x().into(),
                        y: y_start,
                    },
                },
                Op::SetLineHeight {
                    lh: self.schema.line_height.unwrap_or(font_size),
                },
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
                        size: font_size.clone(),
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
            (page_height - y_line + top_margin_in_mm.into()).into(),
        ))
    }
}
