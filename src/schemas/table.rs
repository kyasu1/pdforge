use std::cmp::max;

use super::{base::BaseSchema, InvalidColorStringSnafu};
use crate::schemas::qrcode::QrCode;
use crate::schemas::text::Text;
use crate::{font::FontMap, schemas::text::TextSchema};
use crate::{
    schemas::{Alignment, Error, JsonPosition},
    utils::OpBuffer,
};
use palette::Srgb;
use printpdf::{Mm, Pt};
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
#[serde(rename_all = "camelCase")]
pub struct JsonColumn {
    name: String,
    type_: String,
    alignment: Alignment,
    font_name: String,
    font_size: f32,
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
pub struct ColumnSchema {
    name: String,
    type_: String,
    alignment: Alignment,
    font_name: String,
    font_size: Pt,
}

impl ColumnSchema {
    pub fn from_json(json: JsonColumn) -> Result<Self, Error> {
        Ok(ColumnSchema {
            name: json.name,
            type_: json.type_,
            alignment: json.alignment,
            font_name: json.font_name,
            font_size: Pt(json.font_size),
        })
    }
}
#[derive(Debug, Clone)]
pub struct TableSchema {
    base: BaseSchema,
    content: String,
    show_head: bool,
    head_width_percentages: Vec<f32>,
    table_styles: TableStyles,
    columns: Vec<ColumnSchema>,
    fields: Vec<Vec<String>>,
}

impl TableSchema {
    fn cell_widths(&self) -> Vec<Pt> {
        let box_width = self.base.width();
        self.head_width_percentages
            .clone()
            .into_iter()
            .map(|percent| Pt(box_width.0 * percent))
            .collect()
    }

    pub fn from_json(json: JsonTableSchema) -> Result<Self, Error> {
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
            let column = ColumnSchema::from_json(json_column)?;

            columns.push(column)
        }
        let text = TableSchema {
            base,
            content: json.content,
            show_head: json.show_head,
            head_width_percentages: json.head_width_percentages,
            table_styles,
            columns,
            fields: json.fields.clone(),
        };
        Ok(text)
    }
}

pub struct Table {
    schema: TableSchema,
    font_map: FontMap,
}
impl Table {
    pub fn new(schema: &TableSchema, font_map: &FontMap) -> Self {
        Self {
            schema: schema.clone(),
            font_map: font_map.clone(),
        }
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
        let mut page_counter = 0;
        let y_top_mm: Mm = current_top_mm.unwrap_or(self.schema.base.y());
        let y_bottom_mm = page_height_in_mm - bottom_margin_in_mm;
        let y_offset: Pt = Pt(0.0);
        let mut y_line_mm: Mm = y_top_mm;

        let mut pages = vec![];
        for row in self.schema.fields.iter() {
            let mut cols = vec![];
            let mut x = self.schema.base.x();
            let mut max_height = Mm(0.0);
            for (col_index, col) in row.into_iter().enumerate() {
                let width = Mm(self.schema.head_width_percentages[col_index]
                    * self.schema.base.width().0
                    / 100.0);
                let cell = self.schema.columns[col_index].clone();
                let text_schema = TextSchema::new(
                    x,
                    y_line_mm,
                    width,
                    Mm(5.0),
                    cell.font_name,
                    cell.font_size,
                    col.to_string(),
                );
                let text = Text::new(&text_schema, &self.font_map).unwrap();
                let lines_in_cell = text.split().unwrap();

                max_height = max(
                    max_height,
                    Pt(lines_in_cell.len() as f32 * cell.font_size.0).into(),
                );

                cols.push(text);

                x = x + width;
            }
            y_line_mm = y_line_mm + max_height;

            if y_line_mm > y_bottom_mm {
                page_counter += 1;
                // colsのyをリセットする必要がある
                let updated: Vec<Text> = cols
                    .into_iter()
                    .map(|mut col| {
                        col.set_y(top_margin_in_mm);
                        return col;
                    })
                    .collect();
                pages.push(vec![updated]);
                y_line_mm = top_margin_in_mm + max_height;
            } else {
                if let Some(page) = pages.get_mut(page_counter) {
                    page.push(cols);
                } else {
                    pages.push(vec![cols])
                }
            }
        }

        for (page_index, page) in pages.into_iter().enumerate() {
            for rows in page {
                for mut cols in rows {
                    cols.draw(page_height_in_mm, page_index, buffer).unwrap();
                }
            }
        }

        Ok((
            current_page + page_counter,
            // Some((page_height - y_line_mm.into() + top_margin_in_mm.into()).into()),
            Some(y_line_mm),
        ))
    }
}
