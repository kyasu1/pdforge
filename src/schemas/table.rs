use std::cmp::max;

use super::{base::BaseSchema, BasePdf, InvalidColorStringSnafu, Schema};
use super::{qrcode, SchemaTrait};
use crate::font::FontMap;
use crate::schemas::text;
use crate::{
    schemas::{Alignment, Error, JsonPosition},
    utils::OpBuffer,
};
use palette::Srgb;
use printpdf::{
    Color, LinePoint, Mm, Op, PaintMode, PdfDocument, Point, Polygon, PolygonRing, Pt, Rgb,
    WindingOrder,
};
use serde::Deserialize;
use snafu::prelude::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonColumnStyles {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSides {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTableStyles {
    border_width: f32,
    border_color: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsonColumn {
    Text(text::JsonTextSchema),
    QrCode(qrcode::JsonQrCodeSchema),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonHeadStyles {
    font_size: f32,
    character_spacing: f32,
    alignment: Alignment,
    line_height: f32,
    border_width: JsonSides,
    padding: JsonSides,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonBodyStyles {
    font_size: f32,
    character_spacing: f32,
    alignment: Alignment,
    line_height: f32,
    border_width: JsonSides,
    padding: JsonSides,
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
    head: Vec<String>,
    head_width_percentages: Vec<f32>,
    table_styles: JsonTableStyles,
    head_styles: JsonHeadStyles,
    body_styles: JsonBodyStyles,
    column_styles: JsonColumnStyles,

    //
    columns: Vec<JsonColumn>,
    fields: Vec<Vec<String>>,
}

//
// Table Schemas
//
#[derive(Debug, Clone)]
pub struct TableStyles {
    border_width: Mm,
    border_color: Srgb<u8>,
}

impl TableStyles {
    pub fn from_json(json: JsonTableStyles) -> Result<Self, Error> {
        let border_color: Srgb<u8> = json.border_color.parse().context(InvalidColorStringSnafu)?;
        Ok(Self {
            border_width: Mm(json.border_width),
            border_color,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Table {
    base_pdf: Box<BasePdf>,
    base: BaseSchema,
    content: String,
    show_head: bool,
    head_width_percentages: Vec<f32>,
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
            .map(|percent| Mm(box_width.0 * percent / 100.0))
            .collect()
    }

    pub fn from_json(
        json: JsonTableSchema,
        font_map: &FontMap,
        base_pdf: &BasePdf,
    ) -> Result<Self, Error> {
        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );

        let table_styles = TableStyles::from_json(json.table_styles)?;
        // let character_spacing = json.character_spacing.map(|f| Pt(f)).unwrap_or(Pt(0.0));
        // let line_height = json.line_height;
        // let font_size = match json.font_size {
        //     Some(f) => FontSize::Fixed(Pt(f)),
        //     None => {
        //         unimplemented!()
        //     }
        // };
        //
        //
        if json.head_width_percentages.iter().sum::<f32>() != 100.0 {
            whatever!("total of column width must be 100%");
        }

        let mut columns = Vec::new();
        for json_column in json.columns {
            let column = match json_column {
                JsonColumn::Text(schema) => Schema::Text(text::Text::from_json(schema, font_map)?),
                JsonColumn::QrCode(schema) => schema.into(),
            };

            columns.push(column)
        }
        let table = Table {
            base_pdf: Box::new(base_pdf.clone()),
            base,
            content: json.content,
            show_head: json.show_head,
            head_width_percentages: json.head_width_percentages,
            table_styles,
            columns,
            fields: json.fields.clone(),
        };
        Ok(table)
    }

    pub fn get_base(&self) -> &BaseSchema {
        &self.base
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
        let mut internal_page_counter = 0;
        let y_top_mm: Mm = current_top_mm.unwrap_or(self.base.y);
        let y_bottom_mm = base_pdf.height - bottom_margin_in_mm;
        let y_offset: Pt = Pt(0.0);
        let mut y_line_mm: Mm = y_top_mm;
        let cell_widths = self.cell_widths();

        let mut pages = vec![];
        for row in self.fields.iter() {
            let mut cols: Vec<Schema> = vec![];
            let mut x = self.base.x;
            let mut max_height = Mm(0.0);

            for (col_index, col) in row.into_iter().enumerate() {
                let cell = self.columns[col_index].clone();
                let width = cell_widths[col_index];
                match cell.clone() {
                    Schema::Text(mut schema) => {
                        schema.set_x(x);
                        schema.set_y(y_line_mm);
                        schema.set_width(width);
                        schema.set_content(col.to_string());

                        let height = schema.get_height()?;
                        max_height = max(max_height, height);

                        x += width;
                        cols.push(Schema::Text(schema));
                    }
                    Schema::QrCode(mut qr_code) => {
                        qr_code.set_x(x);
                        qr_code.set_y(y_line_mm);
                        max_height = max(max_height, qr_code.get_height());

                        x += width;
                        cols.push(Schema::QrCode(qr_code));
                    }
                    _ => {
                        unimplemented!();
                    }
                }
            }
            y_line_mm = y_line_mm + max_height;

            if y_line_mm > y_bottom_mm {
                internal_page_counter += 1;
                // colsのyをリセットする必要がある
                let updated: Vec<Schema> = cols
                    .into_iter()
                    .map(|mut col| {
                        col.set_y(top_margin_in_mm);
                        col.set_height(max_height);
                        return col;
                    })
                    .collect();
                pages.push(vec![updated]);
                y_line_mm = top_margin_in_mm + max_height;
            } else {
                let updated: Vec<Schema> = cols
                    .into_iter()
                    .map(|mut col| {
                        col.set_height(max_height);
                        return col;
                    })
                    .collect();

                if let Some(page) = pages.get_mut(internal_page_counter) {
                    page.push(updated);
                } else {
                    pages.push(vec![updated])
                }
            }
        }

        let gray = Color::Rgb(Rgb {
            r: 0.9,
            g: 0.9,
            b: 0.9,
            icc_profile: None,
        });

        for (page_index, page) in pages.into_iter().enumerate() {
            for rows in page {
                for (col_index, schema) in rows.into_iter().enumerate() {
                    let base = schema.get_base();
                    schema.render(base_pdf, None, doc, page_index, buffer)?;

                    let width = cell_widths[col_index];
                    let rect = DrawRectangle {
                        x: base.x,
                        y: base.y,
                        width,
                        height: base.height,
                        page_height: base_pdf.height,
                        color: Some(gray.clone()),
                        border_width: Some(self.table_styles.border_width),
                        border_color: None,
                    };
                    let ops = draw_rectangle(rect);

                    buffer.insert(page_index, ops);

                    schema.render(base_pdf, None, doc, page_index, buffer)?;
                }
            }
        }

        Ok((current_page + internal_page_counter, Some(y_line_mm)))
    }
}

#[derive(Debug, Clone)]
pub struct DrawRectangle {
    x: Mm,
    y: Mm,
    width: Mm,
    height: Mm,
    page_height: Mm,
    color: Option<Color>,
    border_width: Option<Mm>,
    border_color: Option<Color>,
}
fn draw_rectangle(props: DrawRectangle) -> Vec<Op> {
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

    let x1: Pt = props.x.into();
    let y1: Pt = (props.page_height - props.y).into();
    let x2: Pt = (props.x + props.width).into();
    let y2: Pt = (props.page_height - props.y + props.height).into();

    let polygon = Polygon {
        rings: vec![PolygonRing {
            points: vec![
                LinePoint {
                    p: Point { x: x1, y: y1 },
                    bezier: false,
                },
                LinePoint {
                    p: Point { x: x2, y: y1 },
                    bezier: false,
                },
                LinePoint {
                    p: Point { x: x2, y: y2 },
                    bezier: false,
                },
                LinePoint {
                    p: Point { x: x1, y: y2 },
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
