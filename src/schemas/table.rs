use super::{base::BaseSchema, BasePdf, InvalidColorSnafu, Schema};
use super::{qrcode, BoundingBox, Frame, JsonFrame, SchemaTrait, VerticalAlignment};
use crate::font::FontMap;
use crate::schemas::pdf_utils::{draw_rectangle, DrawRectangle};
use crate::schemas::text;
use crate::{
    schemas::{Alignment, Error, JsonPosition},
    utils::OpBuffer,
};
use printpdf::{
    Color, LinePoint, Mm, Op, PaintMode, PdfDocument, Point, Polygon, PolygonRing, Pt, Rgb,
    WindingOrder,
};
use serde::Deserialize;
use snafu::prelude::*;
use std::cmp::max;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonCellStyle {
    schema: JsonSchema,
    height: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTableStyles {
    border_width: f32,
    border_color: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsonSchema {
    Text(text::JsonTextSchema),
    QrCode(qrcode::JsonQrCodeSchema),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonHeadStyles {
    font_size: f32,
    font_name: String,
    character_spacing: Option<f32>,
    alignment: Option<Alignment>,
    vertical_alignment: Option<VerticalAlignment>,
    line_height: Option<f32>,
    font_color: String,
    border_color: String,
    background_color: String,
    border_width: JsonFrame,
    padding: JsonFrame,
}

#[derive(Debug, Clone)]
pub struct HeadStyles {
    font_size: Pt,
    font_name: String,
    character_spacing: f32,
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    line_height: f32,
    font_color: String,
    border_color: String,
    background_color: String,
    border_width: Frame,
    padding: Frame,
}

impl HeadStyles {
    pub fn from_json(json: JsonHeadStyles) -> Result<Self, Error> {
        let border_width = Frame::from_json(json.border_width)?;
        Ok(Self {
            font_size: Pt(json.font_size),
            font_name: json.font_name,
            character_spacing: json.character_spacing.unwrap_or(0.0),
            alignment: json.alignment.unwrap_or(Alignment::Left),
            vertical_alignment: json.vertical_alignment.unwrap_or(VerticalAlignment::Middle),
            line_height: json.line_height.unwrap_or(1.0),
            font_color: json.font_color,
            border_color: json.border_color,
            background_color: json.background_color,
            border_width,
            padding: Frame::from_json(json.padding)?,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonHead {
    percent: f32,
    content: String,
    font_size: Option<f32>,
    font_name: Option<String>,
    character_spacing: Option<f32>,
    alignment: Option<Alignment>,
    vertical_alignment: Option<VerticalAlignment>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonBodyStyles {
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    character_spacing: Option<f32>,
    font_size: f32,
    font_name: String,
    font_color: String,
    line_height: f32,
    border_color: String,
    background_color: String,
    alternate_background_color: Option<String>,
    border_width: JsonFrame,
    padding: JsonFrame,
}

#[derive(Debug, Clone)]
pub struct BodyStyles {
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    character_spacing: f32,
    font_size: Pt,
    font_name: String,
    font_color: csscolorparser::Color,
    line_height: f32,
    border_color: csscolorparser::Color,
    background_color: csscolorparser::Color,
    alternate_background_color: Option<csscolorparser::Color>,
    border_width: Frame,
    padding: Frame,
}

impl BodyStyles {
    pub fn from_json(json: JsonBodyStyles) -> Result<Self, Error> {
        let border_width = Frame::from_json(json.border_width)?;
        let font_color = csscolorparser::parse(&json.font_color).context(InvalidColorSnafu)?;

        let background_color =
            csscolorparser::parse(&json.background_color).context(InvalidColorSnafu)?;
        let alternate_background_color = match json.alternate_background_color {
            Some(color) => Some(csscolorparser::parse(&color).context(InvalidColorSnafu)?),
            None => None,
        };
        Ok(Self {
            alignment: json.alignment,
            vertical_alignment: json.vertical_alignment,
            character_spacing: json.character_spacing.unwrap_or(0.0),
            font_size: Pt(json.font_size),
            font_name: json.font_name,
            font_color,
            line_height: json.line_height,
            border_color: csscolorparser::parse(&json.border_color).context(InvalidColorSnafu)?,
            background_color,
            alternate_background_color,
            border_width,
            padding: Frame::from_json(json.padding)?,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTableSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
    show_head: bool,
    head_styles: JsonHeadStyles,
    head_width_percentages: Vec<JsonHead>,
    body_styles: JsonBodyStyles,
    table_styles: JsonTableStyles,
    //
    columns: Vec<JsonCellStyle>,
    fields: Vec<Vec<String>>,
}

//
// Table Schemas
//
#[derive(Debug, Clone)]
pub struct TableStyles {
    border_width: Mm,
    border_color: csscolorparser::Color,
}

impl TableStyles {
    pub fn from_json(json: JsonTableStyles) -> Result<Self, Error> {
        let border_color = csscolorparser::parse(&json.border_color).context(InvalidColorSnafu)?;
        Ok(Self {
            border_width: Mm(json.border_width),
            border_color,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Head {
    percent: f32,
    text: text::Text,
}

#[derive(Debug, Clone)]
pub struct Table {
    base: BaseSchema,
    content: String,
    show_head: bool,
    head_width_percentages: Vec<Head>,
    body_styles: BodyStyles,
    table_styles: TableStyles,
    columns: Vec<Schema>,
    fields: Vec<Vec<String>>,
}

impl Table {
    fn cell_widths(&self) -> Vec<Mm> {
        let box_width = self.base.width;
        self.head_width_percentages
            .clone()
            .into_iter()
            .map(|head| Mm(box_width.0 * head.percent / 100.0))
            .collect()
    }

    pub fn from_json(json: JsonTableSchema, font_map: &FontMap) -> Result<Self, Error> {
        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );

        let head_styles = HeadStyles::from_json(json.head_styles)?;

        let table_styles = TableStyles::from_json(json.table_styles)?;

        let body_styles = BodyStyles::from_json(json.body_styles)?;

        if json
            .head_width_percentages
            .iter()
            .map(|json| json.percent)
            .sum::<f32>()
            != 100.0
        {
            whatever!("total of column width must be 100%");
        }

        let heads: Result<Vec<Head>, Error> = json
            .head_width_percentages
            .into_iter()
            .map(|json_head| {
                let text = text::Text::new(
                    Mm(0.0),
                    Mm(0.0),
                    Mm(json.width * json_head.percent / 100.0),
                    Mm(0.0),
                    json_head.font_name.unwrap_or(head_styles.font_name.clone()),
                    json_head
                        .font_size
                        .map(|size| Pt(size))
                        .unwrap_or(head_styles.font_size),
                    json_head.content.clone(),
                    json_head.alignment.unwrap_or(head_styles.clone().alignment),
                    json_head
                        .vertical_alignment
                        .unwrap_or(head_styles.clone().vertical_alignment),
                    font_map,
                    Some(head_styles.padding.clone()),
                )?;

                Ok(Head {
                    percent: json_head.percent,
                    text,
                })
            })
            .collect();
        let heads = heads?;

        let mut columns = Vec::new();
        for json_column in json.columns {
            let schema = match json_column.schema {
                JsonSchema::Text(schema) => Schema::Text(text::Text::from_json(schema, font_map)?),
                JsonSchema::QrCode(schema) => schema.into(),
            };

            columns.push(schema)
        }
        let table = Table {
            base,
            content: json.content,
            show_head: json.show_head,
            head_width_percentages: heads,
            body_styles,
            table_styles,
            columns,
            fields: json.fields.clone(),
        };
        Ok(table)
    }

    pub fn get_base(self) -> BaseSchema {
        self.base
    }

    fn create_header_row(
        &self,
        y_line_mm: Mm,
        cell_widths: &[Mm],
    ) -> Result<(Vec<Schema>, Mm), Error> {
        let mut cols: Vec<Schema> = vec![];
        let mut x = self.base.x;
        let mut max_height = Mm(0.0);

        for (head_index, head) in self.head_width_percentages.iter().enumerate() {
            let mut text = head.text.clone();
            let height = text.get_height()?;

            max_height = max(max_height, height);

            text.set_x(x);
            text.set_y(y_line_mm);
            text.set_width(cell_widths[head_index]);

            let height = text.get_height()?;
            max_height = max(max_height, height);

            x += cell_widths[head_index];

            cols.push(Schema::Text(text));
        }

        let header_row = cols
            .into_iter()
            .map(|mut schema| {
                schema.set_height(max_height);
                return schema;
            })
            .collect();

        Ok((header_row, max_height))
    }

    fn process_row(
        &self,
        row: Vec<String>,
        y_line_mm: Mm,
        cell_widths: &[Mm],
    ) -> Result<(Vec<Schema>, Mm), Error> {
        let mut cols: Vec<Schema> = vec![];
        let mut x = self.base.x;
        let mut max_height = Mm(0.0);

        for (col_index, col) in row.into_iter().enumerate() {
            let schema = self.columns[col_index].clone();
            let cell_width = cell_widths[col_index];
            match schema.clone() {
                Schema::Text(mut text) => {
                    text.set_x(x);
                    text.set_y(y_line_mm);
                    text.set_width(cell_width);
                    text.set_content(col.to_string());

                    let height = text.get_height()?;
                    max_height = max(max_height, height);

                    x += cell_width;

                    cols.push(Schema::Text(text));
                }
                Schema::QrCode(mut qr_code) => {
                    max_height = max_height.max(qr_code.get_height());

                    let bounding_box = BoundingBox {
                        x: x,
                        y: y_line_mm,
                        width: cell_width,
                        height: max_height.clone(),
                    };

                    qr_code.set_bounding_box(bounding_box);

                    x += cell_width;

                    cols.push(Schema::QrCode(qr_code));
                }
                _ => {
                    unimplemented!();
                }
            }
        }
        Ok((cols, max_height))
    }

    pub fn render(
        &mut self,
        base_pdf: &BasePdf,
        doc: &mut PdfDocument,
        current_page: usize,
        current_top_mm: Option<Mm>,
        buffer: &mut OpBuffer,
    ) -> Result<(usize, Option<Mm>), Error> {
        let top_margin_in_mm = base_pdf.padding.top;
        let bottom_margin_in_mm = base_pdf.padding.bottom;
        let mut internal_page_counter: usize = 0;
        let y_top_mm: Mm = current_top_mm.unwrap_or(self.base.y);
        let y_bottom_mm = base_pdf.height - bottom_margin_in_mm;
        let mut y_line_mm: Mm = y_top_mm;
        let cell_widths = self.cell_widths();

        let mut pages: HashMap<usize, Vec<Vec<Schema>>> = HashMap::new();

        let (header_row, header_height) = if self.show_head {
            let header = self.create_header_row(top_margin_in_mm, &cell_widths)?;
            (Some(header.0), header.1)
        } else {
            (None, Mm(0.0))
        };

        match self.fields.as_slice() {
            [] => {
                // If there are no fields, we don't need to render anything
                return Ok((current_page, None));
            }
            [head, tail @ ..] => {
                let (cols, max_height) = self.process_row(head.clone(), y_line_mm, &cell_widths)?;

                if y_line_mm + header_height + max_height > y_bottom_mm {
                    // If the header row exceeds the page height, we need to create a new page
                    internal_page_counter += 1;
                    y_line_mm = top_margin_in_mm;
                }

                let updated_header_row = match header_row.clone() {
                    Some(header_row) => {
                        let updated: Vec<Schema> = header_row
                            .into_iter()
                            .map(|mut schema| {
                                schema.set_y(y_line_mm);
                                return schema;
                            })
                            .collect();
                        Some(updated)
                    }
                    None => None,
                };

                // colsのyをリセットする必要がある
                y_line_mm += header_height;

                let updated: Vec<Schema> = cols
                    .into_iter()
                    .map(|mut schema| {
                        schema.set_y(y_line_mm);
                        schema.set_height(max_height);
                        return schema;
                    })
                    .collect();

                y_line_mm += max_height;

                let head_and_first = match updated_header_row {
                    Some(header_row) => {
                        vec![header_row, updated]
                    }
                    None => {
                        vec![updated]
                    }
                };

                if let Some(page) = pages.get_mut(&internal_page_counter) {
                    page.extend_from_slice(&head_and_first);
                } else {
                    pages.insert(internal_page_counter, head_and_first);
                }

                for row in tail.iter() {
                    let (cols, max_height) =
                        self.process_row(row.clone(), y_line_mm, &cell_widths)?;

                    // Check if the next row will exceed the page height
                    // If it does, create a new page and reset the y_line_mm
                    // If it doesn't, just continue to the next row
                    if y_line_mm + max_height > y_bottom_mm {
                        internal_page_counter += 1;

                        y_line_mm = top_margin_in_mm;

                        let updated_header_row = match header_row.clone() {
                            Some(header_row) => {
                                let updated: Vec<Schema> = header_row
                                    .into_iter()
                                    .map(|mut schema| {
                                        schema.set_y(y_line_mm);
                                        return schema;
                                    })
                                    .collect();
                                Some(updated)
                            }
                            None => None,
                        };

                        y_line_mm = header_height + top_margin_in_mm;

                        // colsのyをリセットする必要がある
                        let updated: Vec<Schema> = cols
                            .into_iter()
                            .map(|mut schema| {
                                schema.set_y(y_line_mm);
                                schema.set_height(max_height);
                                return schema;
                            })
                            .collect();

                        let head_and_first = match updated_header_row {
                            Some(u) => {
                                vec![u, updated]
                            }
                            None => {
                                vec![updated]
                            }
                        };

                        if let Some(page) = pages.get_mut(&internal_page_counter) {
                            page.extend_from_slice(&head_and_first);
                        } else {
                            pages.insert(internal_page_counter, head_and_first);
                        }
                    } else {
                        let updated: Vec<Schema> = cols
                            .into_iter()
                            .map(|mut schema| {
                                schema.set_height(max_height);
                                return schema;
                            })
                            .collect();

                        // Check if the page already exists
                        // If it does, push the updated schema to that page
                        // If it doesn't, create a new page and add the updated schema
                        // to that page
                        if let Some(page) = pages.get_mut(&internal_page_counter) {
                            page.push(updated);
                        } else {
                            pages.insert(internal_page_counter, vec![updated]);
                        }
                    }
                    y_line_mm = y_line_mm + max_height;
                }
            }
        }

        for (page_index, page) in pages.into_iter() {
            for (row_index, rows) in page.into_iter().enumerate() {
                // Determine background color based on row_index
                let color = if row_index % 2 == 1 {
                    // Odd rows use alternate_background_color if available, otherwise use background_color
                    match &self.body_styles.alternate_background_color {
                        Some(alt_color) => Color::Rgb(Rgb {
                            r: alt_color.r as f32,
                            g: alt_color.g as f32,
                            b: alt_color.b as f32,
                            icc_profile: None,
                        }),
                        None => Color::Rgb(Rgb {
                            r: self.body_styles.background_color.r as f32,
                            g: self.body_styles.background_color.g as f32,
                            b: self.body_styles.background_color.b as f32,
                            icc_profile: None,
                        }),
                    }
                } else {
                    // Even rows use background_color
                    Color::Rgb(Rgb {
                        r: self.body_styles.background_color.r as f32,
                        g: self.body_styles.background_color.g as f32,
                        b: self.body_styles.background_color.b as f32,
                        icc_profile: None,
                    })
                };

                for (col_index, schema) in rows.into_iter().enumerate() {
                    let base = schema.clone().get_base();

                    let width = cell_widths[col_index];
                    let rect = DrawRectangle {
                        x: base.x,
                        y: base.y,
                        width,
                        height: base.height,
                        rotate: None,
                        page_height: base_pdf.height,
                        color: Some(color.clone()),
                        border_width: Some(self.table_styles.border_width),
                        border_color: None,
                    };
                    let ops = draw_rectangle(rect);

                    // Fix: Add current_page offset to page_index
                    let actual_page_index = current_page + page_index;
                    buffer.insert(actual_page_index, ops);

                    schema.render(base_pdf.height, doc, actual_page_index, buffer)?;
                }
            }
        }

        Ok((current_page + internal_page_counter, Some(y_line_mm)))
    }
}

#[derive(Debug, Clone)]
pub struct DrawRoundedRectangle {
    x: Mm,
    y: Mm,
    width: Mm,
    height: Mm,
    page_height: Mm,
    color: Option<Color>,
    border_width: Option<Mm>,
    border_color: Option<Color>,
    radius: Mm,
}

fn draw_rounded_rectangle(props: DrawRoundedRectangle) -> Vec<Op> {
    if props.radius.0 > props.width.0 / 2.0 || props.radius.0 > props.height.0 / 2.0 {
        return vec![];
    }

    const C: f32 = 0.551915024494;
    let kappa: Mm = Mm(C * props.radius.0);

    let mode = match props.color {
        Some(_) => PaintMode::FillStroke,
        None => PaintMode::Stroke,
    };

    let color = props.color.unwrap_or(Color::Rgb(Rgb {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        icc_profile: None,
    }));

    let border_color = props.border_color.unwrap_or(Color::Rgb(Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        icc_profile: None,
    }));

    let border_width = props.border_width.unwrap_or(Mm(0.1));

    let bottom_y = props.page_height - props.y;
    let top_y = bottom_y + props.height;
    let right_x = props.x + props.width;

    // start drawing from lower left corner of the box counter clockwise direction
    let p10 = Point {
        x: (props.x).into(),
        y: (bottom_y + props.radius).into(),
    };
    let p11 = Point {
        x: (props.x).into(),
        y: (bottom_y + props.radius - kappa).into(),
    };
    let p12 = Point {
        x: (props.x + props.radius - kappa).into(),
        y: (bottom_y).into(),
    };
    let p13 = Point {
        x: (props.x + props.radius).into(),
        y: (bottom_y).into(),
    };

    let p20 = Point {
        x: (right_x - props.radius).into(),
        y: (bottom_y).into(),
    };

    let p21 = Point {
        x: (right_x - props.radius + kappa).into(),
        y: (bottom_y).into(),
    };

    let p22 = Point {
        x: (right_x).into(),
        y: (bottom_y + props.radius - kappa).into(),
    };

    let p23 = Point {
        x: (right_x).into(),
        y: (bottom_y + props.radius).into(),
    };

    let p30 = Point {
        x: (right_x).into(),
        y: (top_y - props.radius).into(),
    };

    let p31 = Point {
        x: (right_x).into(),
        y: (top_y - props.radius + kappa).into(),
    };

    let p32 = Point {
        x: (right_x - props.radius + kappa).into(),
        y: (top_y).into(),
    };

    let p33 = Point {
        x: (right_x - props.radius).into(),
        y: (top_y).into(),
    };

    let p40 = Point {
        x: (props.x + props.radius).into(),
        y: (top_y).into(),
    };

    let p41 = Point {
        x: (props.x + props.radius - kappa).into(),
        y: (top_y).into(),
    };

    let p42 = Point {
        x: (props.x).into(),
        y: (top_y - props.radius + kappa).into(),
    };

    let p43 = Point {
        x: (props.x).into(),
        y: (top_y - props.radius).into(),
    };

    let polygon = Polygon {
        rings: vec![PolygonRing {
            points: vec![
                LinePoint {
                    p: p10,
                    bezier: false,
                },
                LinePoint {
                    p: p11,
                    bezier: true,
                },
                LinePoint {
                    p: p12,
                    bezier: true,
                },
                LinePoint {
                    p: p13,
                    bezier: false,
                },
                LinePoint {
                    p: p20,
                    bezier: false,
                },
                LinePoint {
                    p: p21,
                    bezier: true,
                },
                LinePoint {
                    p: p22,
                    bezier: true,
                },
                LinePoint {
                    p: p23,
                    bezier: false,
                },
                LinePoint {
                    p: p30,
                    bezier: false,
                },
                LinePoint {
                    p: p31,
                    bezier: true,
                },
                LinePoint {
                    p: p32,
                    bezier: true,
                },
                LinePoint {
                    p: p33,
                    bezier: false,
                },
                LinePoint {
                    p: p40,
                    bezier: false,
                },
                LinePoint {
                    p: p41,
                    bezier: true,
                },
                LinePoint {
                    p: p42,
                    bezier: true,
                },
                LinePoint {
                    p: p43,
                    bezier: false,
                },
            ],
        }],
        mode,
        winding_order: WindingOrder::NonZero,
    };

    let ops: Vec<Op> = vec![
        Op::SetOutlineColor { col: border_color },
        Op::SetFillColor { col: color },
        Op::SetOutlineThickness {
            pt: border_width.into(),
        },
        Op::DrawPolygon {
            polygon: polygon.clone(),
        },
    ];
    ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontMap;
    use std::collections::HashMap;

    fn create_test_font_map() -> FontMap {
        // Create an empty FontMap for testing purposes
        FontMap::default()
    }

    fn create_test_json_frame() -> JsonFrame {
        JsonFrame {
            top: 5.0,
            right: 5.0,
            bottom: 5.0,
            left: 5.0,
        }
    }

    fn create_test_head_styles() -> JsonHeadStyles {
        JsonHeadStyles {
            font_size: 12.0,
            font_name: "TestFont".to_string(),
            character_spacing: Some(0.0),
            alignment: Some(Alignment::Center),
            vertical_alignment: Some(VerticalAlignment::Middle),
            line_height: Some(1.0),
            font_color: "#ffffff".to_string(),
            border_color: "#000000".to_string(),
            background_color: "#2980ba".to_string(),
            border_width: create_test_json_frame(),
            padding: create_test_json_frame(),
        }
    }

    fn create_test_body_styles() -> JsonBodyStyles {
        JsonBodyStyles {
            alignment: Alignment::Left,
            vertical_alignment: VerticalAlignment::Middle,
            character_spacing: Some(0.0),
            font_size: 10.0,
            font_name: "TestFont".to_string(),
            font_color: "#000000".to_string(),
            line_height: 1.5,
            border_color: "#888888".to_string(),
            background_color: "#ffffff".to_string(),
            alternate_background_color: Some("#f8f8f8".to_string()),
            border_width: create_test_json_frame(),
            padding: create_test_json_frame(),
        }
    }

    fn create_test_table_styles() -> JsonTableStyles {
        JsonTableStyles {
            border_width: 0.3,
            border_color: "#000000".to_string(),
        }
    }

    fn create_test_table_schema() -> JsonTableSchema {
        JsonTableSchema {
            name: "test_table".to_string(),
            position: JsonPosition { x: 10.0, y: 50.0 },
            width: 190.0,
            height: 100.0,
            content: "test_content".to_string(),
            show_head: true,
            head_styles: create_test_head_styles(),
            head_width_percentages: vec![
                JsonHead {
                    percent: 50.0,
                    content: "Column 1".to_string(),
                    font_size: Some(12.0),
                    font_name: Some("TestFont".to_string()),
                    character_spacing: Some(0.0),
                    alignment: Some(Alignment::Center),
                    vertical_alignment: Some(VerticalAlignment::Middle),
                },
                JsonHead {
                    percent: 50.0,
                    content: "Column 2".to_string(),
                    font_size: Some(12.0),
                    font_name: Some("TestFont".to_string()),
                    character_spacing: Some(0.0),
                    alignment: Some(Alignment::Center),
                    vertical_alignment: Some(VerticalAlignment::Middle),
                },
            ],
            body_styles: create_test_body_styles(),
            table_styles: create_test_table_styles(),
            columns: vec![],
            fields: vec![
                vec!["Header 1".to_string(), "Header 2".to_string()],
                vec!["Data 1".to_string(), "Data 2".to_string()],
                vec!["Data 3".to_string(), "Data 4".to_string()],
            ],
        }
    }

    #[test]
    fn test_head_styles_from_json() {
        let json = create_test_head_styles();
        let result = HeadStyles::from_json(json);
        
        assert!(result.is_ok());
        let head_styles = result.unwrap();
        assert_eq!(head_styles.font_size, Pt(12.0));
        assert_eq!(head_styles.font_name, "TestFont");
        assert_eq!(head_styles.character_spacing, 0.0);
        assert_eq!(head_styles.line_height, 1.0);
        assert_eq!(head_styles.font_color, "#ffffff");
        assert_eq!(head_styles.background_color, "#2980ba");
        assert_eq!(head_styles.border_color, "#000000");
    }

    #[test]
    fn test_body_styles_from_json() {
        let json = create_test_body_styles();
        let result = BodyStyles::from_json(json);
        
        assert!(result.is_ok());
        let body_styles = result.unwrap();
        assert_eq!(body_styles.font_size, Pt(10.0));
        assert_eq!(body_styles.font_name, "TestFont");
        assert_eq!(body_styles.character_spacing, 0.0);
        assert_eq!(body_styles.line_height, 1.5);
        assert!(body_styles.alternate_background_color.is_some());
    }

    #[test]
    fn test_body_styles_invalid_color() {
        let mut json = create_test_body_styles();
        json.font_color = "invalid_color".to_string();
        
        let result = BodyStyles::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_table_styles_from_json() {
        let json = create_test_table_styles();
        let result = TableStyles::from_json(json);
        
        assert!(result.is_ok());
        let table_styles = result.unwrap();
        assert_eq!(table_styles.border_width, Mm(0.3));
    }

    #[test]
    fn test_table_styles_invalid_color() {
        let mut json = create_test_table_styles();
        json.border_color = "invalid_color".to_string();
        
        let result = TableStyles::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_table_from_json_invalid_column_percentages() {
        let mut json = create_test_table_schema();
        // Make percentages not add up to 100%
        json.head_width_percentages[0].percent = 40.0;
        json.head_width_percentages[1].percent = 40.0; // Total = 80%, not 100%
        
        let font_map = create_test_font_map();
        let result = Table::from_json(json, &font_map);
        
        assert!(result.is_err());
        let error_message = format!("{}", result.unwrap_err());
        assert!(error_message.contains("total of column width must be 100%"));
    }

    #[test]
    fn test_draw_rounded_rectangle_valid_radius() {
        let props = DrawRoundedRectangle {
            x: Mm(10.0),
            y: Mm(20.0),
            width: Mm(100.0),
            height: Mm(50.0),
            page_height: Mm(297.0),
            color: Some(Color::Rgb(Rgb { r: 1.0, g: 0.0, b: 0.0, icc_profile: None })),
            border_width: Some(Mm(1.0)),
            border_color: Some(Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None })),
            radius: Mm(5.0),
        };

        let ops = draw_rounded_rectangle(props);
        assert!(!ops.is_empty());
        assert_eq!(ops.len(), 4); // SetOutlineColor, SetFillColor, SetOutlineThickness, DrawPolygon
    }

    #[test]
    fn test_draw_rounded_rectangle_invalid_radius() {
        let props = DrawRoundedRectangle {
            x: Mm(10.0),
            y: Mm(20.0),
            width: Mm(100.0),
            height: Mm(50.0),
            page_height: Mm(297.0),
            color: Some(Color::Rgb(Rgb { r: 1.0, g: 0.0, b: 0.0, icc_profile: None })),
            border_width: Some(Mm(1.0)),
            border_color: Some(Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None })),
            radius: Mm(60.0), // Too large radius
        };

        let ops = draw_rounded_rectangle(props);
        assert!(ops.is_empty()); // Should return empty when radius is invalid
    }

    #[test]
    fn test_frame_from_json() {
        let json_frame = JsonFrame {
            top: 10.0,
            right: 15.0,
            bottom: 20.0,
            left: 25.0,
        };

        let frame = Frame::from_json(json_frame).unwrap();
        assert_eq!(frame.top, Mm(10.0));
        assert_eq!(frame.right, Mm(15.0));
        assert_eq!(frame.bottom, Mm(20.0));
        assert_eq!(frame.left, Mm(25.0));
    }
}
