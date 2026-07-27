use super::{base::BaseSchema, BasePdf, HasBaseSchema, InvalidColorSnafu, Schema};
use super::{qrcode, BoundingBox, Frame, JsonFrame, SchemaTrait, VerticalAlignment};
use crate::font::{FontMap, LineBreakMode};
use crate::schemas::pdf_utils::{draw_table_cell, DrawCell};
use crate::schemas::text;
use crate::{
    schemas::{Alignment, Error, JsonPosition},
    utils::OpBuffer,
};
use printpdf::{Color, Mm, PdfDocument, Pt, Rgb};
use serde::Deserialize;
use snafu::prelude::*;
use std::cmp::max;

/// Relative tolerance used when deciding whether the declared column widths
/// overflow the table. Without it, a table whose percentages sum to exactly 100
/// can tip into the overflow branch on floating-point rounding alone.
const OVERFLOW_EPSILON_RATIO: f64 = 1e-6;

/// How a single column's width is declared in the template.
///
/// Resolution order is fixed → percent → fraction: fixed and percent widths are
/// taken from the table width first, and whatever is left over is split between
/// the fraction columns by weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnWidth {
    /// An absolute width in millimetres.
    Fixed(f64),
    /// A percentage of the table's effective width.
    Percent(f64),
    /// A share of the width left over after fixed and percent columns are placed.
    Fraction(f64),
}

/// Raw `width` value: either a bare number (millimetres) or a suffixed string.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum JsonColumnWidth {
    Number(f32),
    Text(String),
}

impl JsonColumnWidth {
    /// Accepts `25`, `"25mm"`, `"20%"`, `"2fr"` and `"fr"` (= `1fr`). Suffixes are
    /// case-insensitive and surrounding whitespace is ignored.
    fn parse(&self, column_index: usize) -> Result<ColumnWidth, Error> {
        let width = match self {
            JsonColumnWidth::Number(value) => ColumnWidth::Fixed(*value as f64),
            JsonColumnWidth::Text(raw) => {
                let text = raw.trim();
                let lowered = text.to_ascii_lowercase();

                // Bare "fr" means one share, matching the CSS grid shorthand.
                if lowered == "fr" {
                    ColumnWidth::Fraction(1.0)
                } else if let Some(number) = lowered.strip_suffix("fr") {
                    ColumnWidth::Fraction(parse_number(number, text, column_index)?)
                } else if let Some(number) = lowered.strip_suffix('%') {
                    ColumnWidth::Percent(parse_number(number, text, column_index)?)
                } else if let Some(number) = lowered.strip_suffix("mm") {
                    ColumnWidth::Fixed(parse_number(number, text, column_index)?)
                } else {
                    whatever!(
                        "column {column_index}: invalid width {text:?}; \
                         expected a number (mm) or one of \"25mm\", \"20%\", \"2fr\""
                    )
                }
            }
        };

        // A zero fraction would make the fraction weights sum to zero and divide
        // by it; a zero fixed/percent width collapses the cell so far that text
        // wraps to one grapheme cluster per line. Neither is ever intended.
        //
        // The upper bound matters just as much: each value is finite on its own,
        // but the resolver sums them, and two `1e308fr` columns overflow to
        // infinity, whose ratio is NaN. Capping at `f32::MAX` — the precision
        // widths end up in anyway — leaves no room for the f64 aggregates to
        // overflow at any realistic column count.
        let value = match width {
            ColumnWidth::Fixed(v) | ColumnWidth::Percent(v) | ColumnWidth::Fraction(v) => v,
        };
        if !value.is_finite() || value <= 0.0 || value > f32::MAX as f64 {
            whatever!(
                "column {column_index}: width must be a positive number no larger than {}, got {value}",
                f32::MAX
            );
        }

        Ok(width)
    }
}

/// Parse the numeric part of a width literal. `f64::from_str` happily accepts
/// `"NaN"` and `"inf"`, so the caller checks `is_finite` afterwards.
fn parse_number(number: &str, original: &str, column_index: usize) -> Result<f64, Error> {
    // Deliberately not trimmed: leading/trailing space around the whole literal
    // is fine, but `"20 %"` is a typo, not a width.
    number.parse::<f64>().map_err(|_| {
        snafu::FromString::without_source(format!(
            "column {column_index}: invalid width {original:?}; \
             expected a number (mm) or one of \"25mm\", \"20%\", \"2fr\""
        ))
    })
}

/// A column as declared in the template: its width, its header cell, and the
/// schema used to render every body cell in that column.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonColumn {
    width: JsonColumnWidth,
    header: JsonHead,
    cell: JsonSchema,
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
    line_break_mode: Option<LineBreakMode>,
}

#[derive(Debug, Clone)]
pub struct HeadStyles {
    font_size: Pt,
    font_name: String,
    character_spacing: f32,
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    line_height: f32,
    font_color: csscolorparser::Color,
    border_color: csscolorparser::Color,
    background_color: csscolorparser::Color,
    border_width: Frame,
    padding: Frame,
    line_break_mode: Option<LineBreakMode>,
}

impl HeadStyles {
    pub fn from_json(json: JsonHeadStyles) -> Result<Self, Error> {
        let border_width = Frame::from_json(json.border_width)?;
        let font_color = parse_color_or(&json.font_color, "#000000")?;
        let border_color = parse_color_or(&json.border_color, "#000000")?;
        let background_color = parse_color_or(&json.background_color, "#ffffff")?;
        Ok(Self {
            font_size: Pt(json.font_size),
            font_name: json.font_name,
            character_spacing: json.character_spacing.unwrap_or(0.0),
            alignment: json.alignment.unwrap_or(Alignment::Left),
            vertical_alignment: json.vertical_alignment.unwrap_or(VerticalAlignment::Middle),
            line_height: json.line_height.unwrap_or(1.0),
            font_color,
            border_color,
            background_color,
            border_width,
            padding: Frame::from_json(json.padding)?,
            line_break_mode: json.line_break_mode,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonHead {
    content: String,
    font_size: Option<f32>,
    font_name: Option<String>,
    character_spacing: Option<f32>,
    alignment: Option<Alignment>,
    vertical_alignment: Option<VerticalAlignment>,
    line_break_mode: Option<LineBreakMode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonBodyStyles {
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    character_spacing: Option<f32>,
    font_color: String,
    line_height: f32,
    background_color: String,
    alternate_background_color: Option<String>,
    padding: JsonFrame,
    line_break_mode: Option<LineBreakMode>,
}

#[derive(Debug, Clone)]
pub struct BodyStyles {
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    character_spacing: f32,
    font_color: csscolorparser::Color,
    line_height: f32,
    background_color: csscolorparser::Color,
    alternate_background_color: Option<csscolorparser::Color>,
    padding: Frame,
    line_break_mode: LineBreakMode,
}

impl BodyStyles {
    pub fn from_json(json: JsonBodyStyles) -> Result<Self, Error> {
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
            font_color,
            line_height: json.line_height,
            background_color,
            alternate_background_color,
            padding: Frame::from_json(json.padding)?,
            line_break_mode: json.line_break_mode.unwrap_or(LineBreakMode::Char),
        })
    }

    /// Text-responsibility defaults a body column inherits when it leaves a
    /// field unspecified. Background/border stay out — the row painter owns them.
    fn text_defaults(&self) -> text::TextStyleDefaults {
        text::TextStyleDefaults {
            alignment: self.alignment.clone(),
            vertical_alignment: self.vertical_alignment.clone(),
            character_spacing: self.character_spacing,
            line_height: self.line_height,
            font_color: self.font_color.clone(),
            padding: Some(self.padding.clone()),
            line_break_mode: self.line_break_mode,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTableSchema {
    pub name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    show_head: bool,
    head_styles: JsonHeadStyles,
    body_styles: JsonBodyStyles,
    table_styles: JsonTableStyles,
    columns: Vec<JsonColumn>,
    pub fields: Vec<Vec<String>>,
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

/// A resolved column: width declaration, header cell, and body cell template.
/// Keeping the three together means the header, the cell schema and the width
/// can never drift out of sync by index.
#[derive(Debug, Clone)]
pub struct Column {
    width: ColumnWidth,
    header: text::Text,
    cell: Schema,
}

#[derive(Debug, Clone)]
pub struct Table {
    base: BaseSchema,
    show_head: bool,
    head_styles: HeadStyles,
    body_styles: BodyStyles,
    table_styles: TableStyles,
    columns: Vec<Column>,
    fields: Vec<Vec<String>>,
}

/// Visual styling applied to every cell rectangle in a single table row.
#[derive(Debug, Clone)]
struct RowCellStyle {
    background: csscolorparser::Color,
    border_color: csscolorparser::Color,
    border_width: Frame,
}

fn to_rgb(color: &csscolorparser::Color) -> Color {
    Color::Rgb(Rgb {
        r: color.r,
        g: color.g,
        b: color.b,
        icc_profile: None,
    })
}

/// Parse a CSS color, treating an empty/blank string as "unset" and falling
/// back to `default`. Templates commonly use `""` to mean a field was left
/// unspecified (e.g. a header with `borderWidth: 0` and `borderColor: ""`).
fn parse_color_or(value: &str, default: &str) -> Result<csscolorparser::Color, Error> {
    let value = value.trim();
    let source = if value.is_empty() { default } else { value };
    csscolorparser::parse(source).context(InvalidColorSnafu)
}

impl Table {
    /// Width the table actually gets: the template width, capped by what is left
    /// of the page after horizontal padding.
    fn effective_width(&self, base_pdf: &BasePdf) -> Mm {
        let available_width =
            (base_pdf.width - base_pdf.padding.left - base_pdf.padding.right).max(Mm(0.0));
        // The schema's own width is not validated anywhere, and a negative one
        // would flip the overflow scale factor and hand back negative columns.
        self.base.width.min(available_width).max(Mm(0.0))
    }

    /// Turn the declared column widths into concrete millimetre widths.
    ///
    /// Fixed and percent columns are placed first. If they fit, the remainder is
    /// split between the fraction columns by weight; if they overflow, they are
    /// scaled down to fit and the fraction columns collapse to zero. Overflow is
    /// not an error because the table width depends on `basePdf.padding`, so a
    /// page-setup change must not turn into a runtime failure.
    ///
    /// Arithmetic runs in `f64` and only lands in `f32` at the end, so a table
    /// whose percentages sum to 100 does not drift into the overflow branch.
    fn resolve_column_widths(&self, base_pdf: &BasePdf) -> Vec<Mm> {
        let table_width = self.effective_width(base_pdf).0 as f64;

        let mut widths = vec![0.0_f64; self.columns.len()];
        let mut allocated = 0.0_f64;
        let mut fraction_total = 0.0_f64;

        for (index, column) in self.columns.iter().enumerate() {
            match column.width {
                ColumnWidth::Fixed(mm) => {
                    widths[index] = mm;
                    allocated += mm;
                }
                ColumnWidth::Percent(percent) => {
                    let width = table_width * percent / 100.0;
                    widths[index] = width;
                    allocated += width;
                }
                ColumnWidth::Fraction(weight) => fraction_total += weight,
            }
        }

        let overflows = allocated > table_width + table_width.abs() * OVERFLOW_EPSILON_RATIO;

        if overflows {
            // Shrink the declared widths to fit. Fraction columns have no width
            // to shrink and stay at zero.
            let scale = if allocated > 0.0 {
                table_width / allocated
            } else {
                0.0
            };
            for width in widths.iter_mut() {
                *width *= scale;
            }
        } else if fraction_total > 0.0 {
            let remaining = (table_width - allocated).max(0.0);
            for (index, column) in self.columns.iter().enumerate() {
                if let ColumnWidth::Fraction(weight) = column.width {
                    widths[index] = remaining * weight / fraction_total;
                }
            }
        }
        // Without fraction columns and without overflow the table simply ends up
        // narrower than `table_width`; the row painters already left-align it.

        let mut resolved: Vec<Mm> = widths
            .into_iter()
            .map(|w| {
                // Parse-time bounds should make this unreachable; the clamp keeps
                // a bad width from reaching the renderer if one ever slips past.
                debug_assert!(w.is_finite() && w >= 0.0, "resolved column width {w}");
                Mm(if w.is_finite() {
                    w.max(0.0) as f32
                } else {
                    0.0
                })
            })
            .collect();

        // Absorb the f32 rounding residual so column borders meet exactly. This
        // has to happen after the cast — correcting in f64 and casting afterwards
        // reintroduces the very error it is meant to remove.
        //
        // Only a fraction column may absorb it: fixed and percent columns must
        // keep the width they declared. Skipped on overflow, where fraction
        // columns are zero by definition and would go negative.
        if !overflows && fraction_total > 0.0 {
            if let Some(target) = self.widest_fraction_column(&resolved) {
                let sum: f32 = resolved.iter().map(|w| w.0).sum();
                let corrected = resolved[target].0 + (table_width as f32 - sum);
                if corrected.is_finite() && corrected >= 0.0 {
                    resolved[target] = Mm(corrected);
                }
            }
        }

        resolved
    }

    /// Index of the widest fraction column — the safest place to absorb the
    /// rounding residual, since a tiny-weight column could otherwise go negative.
    fn widest_fraction_column(&self, resolved: &[Mm]) -> Option<usize> {
        self.columns
            .iter()
            .enumerate()
            .filter(|(_, column)| matches!(column.width, ColumnWidth::Fraction(_)))
            .max_by(|(a, _), (b, _)| resolved[*a].0.total_cmp(&resolved[*b].0))
            .map(|(index, _)| index)
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

        if json.columns.is_empty() {
            whatever!("table must declare at least one column");
        }

        let body_text_defaults = body_styles.text_defaults();
        let mut columns = Vec::with_capacity(json.columns.len());

        for (index, json_column) in json.columns.into_iter().enumerate() {
            let width = json_column.width.parse(index)?;
            let json_head = json_column.header;

            // The real width is assigned per render in `create_header_row`, once
            // the page geometry is known.
            let mut header = text::Text::new(
                Mm(0.0),
                Mm(0.0),
                Mm(0.0),
                Mm(0.0),
                json_head.font_name.unwrap_or(head_styles.font_name.clone()),
                json_head.font_size.map(Pt).unwrap_or(head_styles.font_size),
                json_head.content.clone(),
                json_head.alignment.unwrap_or(head_styles.clone().alignment),
                json_head
                    .vertical_alignment
                    .unwrap_or(head_styles.clone().vertical_alignment),
                font_map,
                Some(head_styles.padding.clone()),
            )?;
            header.set_line_break_mode(json_head.line_break_mode.or(head_styles.line_break_mode));
            header.set_font_color(head_styles.font_color.clone());
            header.set_character_spacing(Pt(json_head
                .character_spacing
                .unwrap_or(head_styles.character_spacing)));
            header.set_line_height(Some(head_styles.line_height));

            let cell = match json_column.cell {
                JsonSchema::Text(schema) => Schema::Text(text::Text::from_json_with_defaults(
                    schema,
                    &body_text_defaults,
                    font_map,
                )?),
                JsonSchema::QrCode(schema) => schema.into(),
            };

            columns.push(Column {
                width,
                header,
                cell,
            });
        }

        // Row rendering indexes `columns` by cell position, so a row that does
        // not match the column count would panic mid-render.
        for (row_index, row) in json.fields.iter().enumerate() {
            if row.len() != columns.len() {
                whatever!(
                    "row {row_index}: expected {} cells to match the column count, got {}",
                    columns.len(),
                    row.len()
                );
            }
        }

        let table = Table {
            base,
            show_head: json.show_head,
            head_styles,
            body_styles,
            table_styles,
            columns,
            fields: json.fields.clone(),
        };
        Ok(table)
    }

    pub fn get_base(&self) -> BaseSchema {
        self.base.clone()
    }

    /// Background color for a body row, honoring the alternating (zebra) color.
    fn body_row_style(&self, visual_row_index: usize) -> RowCellStyle {
        let background = if visual_row_index % 2 == 1 {
            self.body_styles
                .alternate_background_color
                .clone()
                .unwrap_or_else(|| self.body_styles.background_color.clone())
        } else {
            self.body_styles.background_color.clone()
        };

        RowCellStyle {
            background,
            border_color: self.table_styles.border_color.clone(),
            border_width: Frame::uniform(self.table_styles.border_width),
        }
    }

    /// Visual style for the header row (its own background and border override).
    fn header_row_style(&self) -> RowCellStyle {
        RowCellStyle {
            background: self.head_styles.background_color.clone(),
            border_color: self.head_styles.border_color.clone(),
            border_width: self.head_styles.border_width.clone(),
        }
    }

    /// Render a row immediately to the buffer (memory-efficient approach)
    fn render_row_immediately(
        &self,
        schemas: Vec<Schema>,
        cell_style: &RowCellStyle,
        cell_widths: &[Mm],
        base_pdf: &BasePdf,
        page_index: usize,
        doc: &mut PdfDocument,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let fill = to_rgb(&cell_style.background);
        let border_color = to_rgb(&cell_style.border_color);

        // Render each cell in the row
        for (col_index, schema) in schemas.iter().enumerate() {
            let base = schema.get_base_copy();
            let width = cell_widths[col_index];

            // Draw the cell background and border
            let cell = DrawCell {
                x: base.x,
                y: base.y,
                width,
                height: base.height,
                rotate: None,
                page_height: base_pdf.height,
                fill: fill.clone(),
                border_color: border_color.clone(),
                border_width: cell_style.border_width.clone(),
            };
            buffer.insert(page_index, draw_table_cell(cell));

            // Render cell content
            schema.render(base_pdf.width, base_pdf.height, doc, page_index, buffer)?;
        }

        Ok(())
    }

    fn create_header_row(
        &self,
        y_line_mm: Mm,
        cell_widths: &[Mm],
        base_pdf: &BasePdf,
    ) -> Result<(Vec<Schema>, Mm), Error> {
        let mut cols: Vec<Schema> = vec![];
        // Apply left padding to the table start position
        let mut x = self.base.x.max(base_pdf.padding.left);

        // Ensure table doesn't exceed right padding boundary
        let max_right_x = base_pdf.width - base_pdf.padding.right;
        let total_width: Mm = cell_widths
            .iter()
            .fold(Mm(0.0), |acc, &width| Mm(acc.0 + width.0));
        if x + total_width > max_right_x {
            x = max_right_x - total_width;
            // Ensure x doesn't go below left padding
            x = x.max(base_pdf.padding.left);
        }

        let mut max_height = Mm(0.0);

        for (head_index, column) in self.columns.iter().enumerate() {
            let mut text = column.header.clone();

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
                schema
            })
            .collect();

        Ok((header_row, max_height))
    }

    fn process_row(
        &self,
        row: Vec<String>,
        y_line_mm: Mm,
        cell_widths: &[Mm],
        base_pdf: &BasePdf,
    ) -> Result<(Vec<Schema>, Mm), Error> {
        let mut cols: Vec<Schema> = vec![];
        // Apply left padding to the table start position
        let mut x = self.base.x.max(base_pdf.padding.left);

        // Ensure table doesn't exceed right padding boundary
        let max_right_x = base_pdf.width - base_pdf.padding.right;
        let total_width: Mm = cell_widths
            .iter()
            .fold(Mm(0.0), |acc, &width| Mm(acc.0 + width.0));
        if x + total_width > max_right_x {
            x = max_right_x - total_width;
            // Ensure x doesn't go below left padding
            x = x.max(base_pdf.padding.left);
        }

        let mut max_height = Mm(0.0);

        for (col_index, col) in row.into_iter().enumerate() {
            let schema = &self.columns[col_index].cell;
            let cell_width = cell_widths[col_index];
            match schema {
                Schema::Text(text) => {
                    let mut text = text.clone();
                    text.set_x(x);
                    text.set_y(y_line_mm);
                    text.set_width(cell_width);
                    text.set_content(col.to_string());
                    if !text.has_line_height() {
                        text.set_line_height(Some(self.body_styles.line_height));
                    }
                    if !text.has_line_break_mode() {
                        text.set_line_break_mode(Some(self.body_styles.line_break_mode));
                    }

                    let height = text.get_height()?;
                    max_height = max(max_height, height);

                    x += cell_width;

                    cols.push(Schema::Text(text));
                }
                Schema::QrCode(qr_code) => {
                    let mut qr_code = qr_code.clone();
                    max_height = max_height.max(qr_code.get_height());

                    let bounding_box = BoundingBox {
                        x,
                        y: y_line_mm,
                        width: cell_width,
                        height: max_height,
                    };

                    qr_code.set_bounding_box(bounding_box);

                    x += cell_width;

                    cols.push(Schema::QrCode(qr_code));
                }
                _ => {
                    return Err(Error::UnsupportedSchema {
                        context: "table column".to_string(),
                        schema_type: schema.type_name().to_string(),
                    });
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
        let mut internal_page_counter: usize = 0;
        let y_top_mm: Mm = current_top_mm.unwrap_or(self.base.y);
        let y_bottom_mm = base_pdf.height - base_pdf.padding.bottom;
        let mut y_line_mm: Mm = y_top_mm;
        let cell_widths = self.resolve_column_widths(base_pdf);

        // Track row index for alternating background colors (0-indexed, header is not counted)
        let mut visual_row_index: usize = 0;

        let (header_row, header_height) = if self.show_head {
            let header = self.create_header_row(top_margin_in_mm, &cell_widths, base_pdf)?;
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
                let (cols, max_height) =
                    self.process_row(head.clone(), y_line_mm, &cell_widths, base_pdf)?;

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
                                schema
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
                        schema
                    })
                    .collect();

                y_line_mm += max_height;

                // Render header row immediately if present
                let actual_page_index = current_page + internal_page_counter;
                if let Some(header_row) = updated_header_row {
                    // Header uses its own styling, independent of the zebra colors.
                    self.render_row_immediately(
                        header_row,
                        &self.header_row_style(),
                        &cell_widths,
                        base_pdf,
                        actual_page_index,
                        doc,
                        buffer,
                    )?;
                }

                // Render first data row immediately
                self.render_row_immediately(
                    updated,
                    &self.body_row_style(visual_row_index),
                    &cell_widths,
                    base_pdf,
                    actual_page_index,
                    doc,
                    buffer,
                )?;
                visual_row_index += 1;

                for row in tail.iter() {
                    let (cols, max_height) =
                        self.process_row(row.clone(), y_line_mm, &cell_widths, base_pdf)?;

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
                                        schema
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
                                schema
                            })
                            .collect();

                        // Render header row immediately on new page if present
                        let actual_page_index = current_page + internal_page_counter;
                        if let Some(header_row) = updated_header_row {
                            self.render_row_immediately(
                                header_row,
                                &self.header_row_style(),
                                &cell_widths,
                                base_pdf,
                                actual_page_index,
                                doc,
                                buffer,
                            )?;
                        }

                        // Render data row immediately
                        self.render_row_immediately(
                            updated,
                            &self.body_row_style(visual_row_index),
                            &cell_widths,
                            base_pdf,
                            actual_page_index,
                            doc,
                            buffer,
                        )?;
                        visual_row_index += 1;
                    } else {
                        let updated: Vec<Schema> = cols
                            .into_iter()
                            .map(|mut schema| {
                                schema.set_height(max_height);
                                schema
                            })
                            .collect();

                        // Render row immediately on same page
                        let actual_page_index = current_page + internal_page_counter;
                        self.render_row_immediately(
                            updated,
                            &self.body_row_style(visual_row_index),
                            &cell_widths,
                            base_pdf,
                            actual_page_index,
                            doc,
                            buffer,
                        )?;
                        visual_row_index += 1;
                    }
                    y_line_mm += max_height;
                }
            }
        }

        // All rows have been rendered immediately, no need for post-processing
        Ok((current_page + internal_page_counter, Some(y_line_mm)))
    }
}

impl HasBaseSchema for Table {
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
    use crate::schemas::spacer::{FlowCursor, JsonSpacerSchema, Spacer};
    use printpdf::{ParsedFont, PdfDocument};
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::{Arc, OnceLock};

    fn create_real_test_font_map() -> FontMap {
        static FONT: OnceLock<Arc<ParsedFont>> = OnceLock::new();

        let parsed_font = FONT
            .get_or_init(|| {
                let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("assets")
                    .join("fonts")
                    .join("NotoSansJP-Regular.ttf");
                let font_bytes = std::fs::read(path).expect("test font should be readable");
                let parsed_font = ParsedFont::from_bytes(&font_bytes, 0, &mut Vec::new())
                    .expect("test font should parse");
                Arc::new(parsed_font)
            })
            .clone();

        let mut doc = PdfDocument::new("test");
        let font_id = doc.add_font(parsed_font.as_ref());
        let mut font_map = FontMap::default();
        font_map.add_font("TestFont".to_string(), font_id, parsed_font.as_ref());
        font_map
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
            line_break_mode: None,
        }
    }

    fn create_test_body_styles() -> JsonBodyStyles {
        JsonBodyStyles {
            alignment: Alignment::Left,
            vertical_alignment: VerticalAlignment::Middle,
            character_spacing: Some(0.0),
            font_color: "#000000".to_string(),
            line_height: 1.5,
            background_color: "#ffffff".to_string(),
            alternate_background_color: Some("#f8f8f8".to_string()),
            padding: create_test_json_frame(),
            line_break_mode: None,
        }
    }

    fn create_test_table_styles() -> JsonTableStyles {
        JsonTableStyles {
            border_width: 0.3,
            border_color: "#000000".to_string(),
        }
    }

    fn create_test_json_column(label: &str, width: JsonColumnWidth) -> JsonColumn {
        JsonColumn {
            width,
            header: JsonHead {
                content: label.to_string(),
                font_size: Some(12.0),
                font_name: Some("TestFont".to_string()),
                character_spacing: Some(0.0),
                alignment: Some(Alignment::Center),
                vertical_alignment: Some(VerticalAlignment::Middle),
                line_break_mode: None,
            },
            cell: serde_json::from_value(json!({
                "type": "text",
                "name": label,
                "position": { "x": 0.0, "y": 0.0 },
                "width": 0.0,
                "height": 0.0,
                "content": "",
                "fontName": "TestFont",
                "fontSize": 10.0
            }))
            .unwrap(),
        }
    }

    fn create_test_table_schema() -> JsonTableSchema {
        create_test_table_schema_with_widths(vec![
            JsonColumnWidth::Text("50%".to_string()),
            JsonColumnWidth::Text("50%".to_string()),
        ])
    }

    fn create_test_table_schema_with_widths(widths: Vec<JsonColumnWidth>) -> JsonTableSchema {
        let columns = widths
            .into_iter()
            .enumerate()
            .map(|(index, width)| create_test_json_column(&format!("Column {}", index + 1), width))
            .collect::<Vec<_>>();
        let fields = (0..3)
            .map(|row| {
                (0..columns.len())
                    .map(|col| format!("Data {row}-{col}"))
                    .collect()
            })
            .collect();

        JsonTableSchema {
            name: "test_table".to_string(),
            position: JsonPosition { x: 10.0, y: 50.0 },
            width: 190.0,
            height: 100.0,
            show_head: true,
            head_styles: create_test_head_styles(),
            body_styles: create_test_body_styles(),
            table_styles: create_test_table_styles(),
            columns,
            fields,
        }
    }

    /// Resolve column widths against an A4-width page with no padding, so the
    /// table's own `width` (190mm) is what gets divided up.
    fn resolve_widths(widths: Vec<&str>) -> Vec<f32> {
        let json = create_test_table_schema_with_widths(
            widths
                .into_iter()
                .map(|w| JsonColumnWidth::Text(w.to_string()))
                .collect(),
        );
        let font_map = create_real_test_font_map();
        let table = Table::from_json(json, &font_map).expect("table should parse");
        table
            .resolve_column_widths(&create_test_base_pdf())
            .into_iter()
            .map(|mm| mm.0)
            .collect()
    }

    fn create_test_base_pdf() -> BasePdf {
        BasePdf {
            width: Mm(210.0),
            height: Mm(297.0),
            padding: Frame {
                top: Mm(0.0),
                right: Mm(0.0),
                bottom: Mm(0.0),
                left: Mm(0.0),
            },
            static_schema: vec![],
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
        assert_eq!(
            head_styles.font_color,
            csscolorparser::parse("#ffffff").unwrap()
        );
        assert_eq!(
            head_styles.background_color,
            csscolorparser::parse("#2980ba").unwrap()
        );
        assert_eq!(
            head_styles.border_color,
            csscolorparser::parse("#000000").unwrap()
        );
    }

    #[test]
    fn test_body_styles_from_json() {
        let json = create_test_body_styles();
        let result = BodyStyles::from_json(json);

        assert!(result.is_ok());
        let body_styles = result.unwrap();
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

    //
    // Column width grammar
    //

    #[test]
    fn column_width_accepts_every_supported_notation() {
        let cases = [
            (JsonColumnWidth::Number(25.0), ColumnWidth::Fixed(25.0)),
            (
                JsonColumnWidth::Text("25mm".into()),
                ColumnWidth::Fixed(25.0),
            ),
            (
                JsonColumnWidth::Text("20%".into()),
                ColumnWidth::Percent(20.0),
            ),
            (
                JsonColumnWidth::Text("2fr".into()),
                ColumnWidth::Fraction(2.0),
            ),
            (
                JsonColumnWidth::Text("fr".into()),
                ColumnWidth::Fraction(1.0),
            ),
            // Suffixes are case-insensitive and surrounding space is ignored.
            (
                JsonColumnWidth::Text(" 25MM ".into()),
                ColumnWidth::Fixed(25.0),
            ),
            (
                JsonColumnWidth::Text("2FR".into()),
                ColumnWidth::Fraction(2.0),
            ),
            (
                JsonColumnWidth::Text("  20% ".into()),
                ColumnWidth::Percent(20.0),
            ),
        ];

        for (input, expected) in cases {
            let parsed = input.parse(0);
            assert_eq!(parsed.unwrap(), expected, "failed for {input:?}");
        }
    }

    #[test]
    fn column_width_rejects_malformed_and_non_positive_values() {
        // `f64::from_str` accepts "NaN"/"inf", so those must be caught explicitly.
        let cases = [
            "abc", "-5", "-5mm", "0fr", "0mm", "0%", "NaNmm", "inffr", "", "mm", "%", "20 pt",
            // Space between the number and its suffix is a typo, not a width.
            "20 %", "25 mm", "2 fr",
            // Doubled suffixes: "mm" is stripped, then "2fr" fails to parse.
            "2frmm", "20%mm", "5mmfr",
        ];

        for input in cases {
            let parsed = JsonColumnWidth::Text(input.to_string()).parse(0);
            assert!(parsed.is_err(), "expected {input:?} to be rejected");
        }
    }

    #[test]
    fn column_width_rejects_values_too_large_for_the_output_precision() {
        // Each of these is finite on its own, but the resolver sums them: two
        // `1e308fr` columns overflow to infinity and their ratio is NaN.
        for input in ["1e308fr", "1e308%", "1e308mm", "1e39mm"] {
            let parsed = JsonColumnWidth::Text(input.to_string()).parse(0);
            assert!(parsed.is_err(), "expected {input:?} to be rejected");
        }

        // Anything that still fits in f32 is accepted. (`format!("{}", f32::MAX)`
        // is not usable here: its decimal form rounds up past the real maximum.)
        assert!(JsonColumnWidth::Text("3.4e38mm".into()).parse(0).is_ok());
    }

    #[test]
    fn resolution_never_yields_nan_for_extreme_but_legal_widths() {
        // Guards the aggregate-overflow path end to end: the parse-time bound is
        // what keeps these finite, so a regression there surfaces here.
        for widths in [
            vec!["3e38fr", "3e38fr"],
            vec!["3e38%", "1fr"],
            vec!["3e38mm", "3e38mm"],
            vec!["3e38mm", "1fr"],
        ] {
            for width in resolve_widths(widths.clone()) {
                assert!(width.is_finite(), "{widths:?} produced {width}");
                assert!(width >= 0.0, "{widths:?} produced {width}");
            }
        }
    }

    #[test]
    fn a_negative_schema_width_does_not_produce_negative_columns() {
        // `BaseSchema.width` is not validated anywhere; a negative one used to
        // flip the overflow scale factor and hand back negative columns.
        let mut json = create_test_table_schema_with_widths(vec![
            JsonColumnWidth::Number(1.0),
            JsonColumnWidth::Text("1fr".into()),
        ]);
        json.width = -10.0;

        let font_map = create_real_test_font_map();
        let table = Table::from_json(json, &font_map).unwrap();

        for width in table.resolve_column_widths(&create_test_base_pdf()) {
            assert!(width.0.is_finite(), "got {}", width.0);
            assert!(width.0 >= 0.0, "got {}", width.0);
        }
    }

    #[test]
    fn column_width_error_names_the_offending_column() {
        let error = JsonColumnWidth::Text("nonsense".into())
            .parse(3)
            .unwrap_err()
            .to_string();
        assert!(error.contains("column 3"), "unexpected message: {error}");
    }

    //
    // Column width resolution
    //

    #[test]
    fn percent_columns_divide_the_table_width() {
        let widths = resolve_widths(vec!["70%", "15%", "15%"]);
        assert_eq!(widths, vec![133.0, 28.5, 28.5]);
    }

    #[test]
    fn fixed_columns_keep_their_declared_width() {
        let widths = resolve_widths(vec!["20mm", "30mm"]);
        assert_eq!(widths, vec![20.0, 30.0]);
    }

    #[test]
    fn fraction_columns_split_the_remaining_width() {
        // 190 - 20 - 47.5 = 122.5 left, split 2:1.
        let widths = resolve_widths(vec!["20mm", "2fr", "25%", "1fr"]);
        assert!((widths[0] - 20.0).abs() < 1e-3);
        assert!((widths[2] - 47.5).abs() < 1e-3);
        assert!((widths[1] - 81.6667).abs() < 1e-3, "got {}", widths[1]);
        assert!((widths[3] - 40.8333).abs() < 1e-3, "got {}", widths[3]);
    }

    #[test]
    fn column_order_does_not_change_the_allocation() {
        let straight = resolve_widths(vec!["20mm", "1fr", "25%"]);
        let shuffled = resolve_widths(vec!["25%", "20mm", "1fr"]);

        // Same declarations, different order: each kind keeps its width.
        assert!((straight[0] - shuffled[1]).abs() < 1e-3);
        assert!((straight[1] - shuffled[2]).abs() < 1e-3);
        assert!((straight[2] - shuffled[0]).abs() < 1e-3);
    }

    #[test]
    fn fraction_columns_fill_the_table_width_exactly() {
        // The rounding residual is absorbed so column borders meet.
        for widths in [
            vec!["1fr", "1fr", "1fr"],
            vec!["20mm", "1fr", "1fr"],
            vec!["33%", "1fr", "17mm", "2fr"],
        ] {
            let resolved = resolve_widths(widths.clone());
            let sum: f32 = resolved.iter().sum();
            assert_eq!(sum, 190.0, "widths {widths:?} summed to {sum}");
        }
    }

    #[test]
    fn without_fraction_columns_the_table_stays_narrower() {
        // No fraction column means nothing may absorb the leftover: the last
        // column must not be stretched to fill the table.
        let widths = resolve_widths(vec!["20mm", "30mm"]);
        assert_eq!(widths[1], 30.0);
        assert_eq!(widths.iter().sum::<f32>(), 50.0);
    }

    #[test]
    fn overflowing_fixed_and_percent_columns_are_scaled_to_fit() {
        // 150 + 150 = 300mm declared into a 190mm table.
        let widths = resolve_widths(vec!["150mm", "150mm"]);
        let sum: f32 = widths.iter().sum();
        assert!((sum - 190.0).abs() < 1e-2, "sum was {sum}");
        assert!((widths[0] - widths[1]).abs() < 1e-3);
    }

    #[test]
    fn overflow_collapses_fraction_columns_to_zero() {
        let widths = resolve_widths(vec!["150mm", "150mm", "1fr"]);
        assert_eq!(widths[2], 0.0);
        let sum: f32 = widths.iter().sum();
        assert!((sum - 190.0).abs() < 1e-2, "sum was {sum}");
    }

    #[test]
    fn percent_over_one_hundred_is_allowed_and_scaled_down() {
        let widths = resolve_widths(vec!["150%"]);
        assert!((widths[0] - 190.0).abs() < 1e-2, "got {}", widths[0]);
    }

    #[test]
    fn percentages_summing_to_one_hundred_never_trip_the_overflow_branch() {
        // Real template values. Allocating these in sequence and adding the
        // results back up can overshoot the table width by a rounding step; the
        // overflow test tolerates that so the columns keep their declared share.
        let templates: [&[&str]; 5] = [
            &["14%", "31%", "17%", "10%", "14%", "14%"], // product_list
            &["15%", "25%", "25%", "17%", "18%"],        // financial_table
            &["10%", "32%", "10%", "10%", "10%", "8%", "10%", "10%"], // print-renews
            &["15%", "30%", "20%", "12%", "11%", "12%"], // inventory_table
            &["15%", "15%", "15%", "15%", "15%", "15%", "10%"], // table.json
        ];

        for percents in templates {
            let widths = resolve_widths(percents.to_vec());
            let expected: Vec<f32> = percents
                .iter()
                .map(|p| {
                    let value: f64 = p.trim_end_matches('%').parse().unwrap();
                    (190.0 * value / 100.0) as f32
                })
                .collect();
            for (index, (got, want)) in widths.iter().zip(expected.iter()).enumerate() {
                assert!(
                    (got - want).abs() < 1e-3,
                    "{percents:?} column {index}: got {got}, want {want}"
                );
            }
        }
    }

    #[test]
    fn residual_correction_never_produces_a_negative_width() {
        // A tiny-weight fraction column must not be driven below zero by the
        // rounding correction, which is why the widest one absorbs it.
        for widths in [
            vec!["100fr", "0.0001fr"],
            vec!["189mm", "0.0001fr"],
            vec!["0.001fr", "0.001fr", "100mm"],
        ] {
            for width in resolve_widths(widths.clone()) {
                assert!(width.is_finite(), "{widths:?} produced {width}");
                assert!(width >= 0.0, "{widths:?} produced {width}");
            }
        }
    }

    #[test]
    fn resolution_survives_a_page_with_no_usable_width() {
        // Padding wider than the page leaves nothing to divide up. This must not
        // hang or produce negative widths.
        let json = create_test_table_schema_with_widths(vec![
            JsonColumnWidth::Text("1fr".into()),
            JsonColumnWidth::Text("50%".into()),
        ]);
        let font_map = create_real_test_font_map();
        let table = Table::from_json(json, &font_map).unwrap();

        let base_pdf = BasePdf {
            width: Mm(210.0),
            height: Mm(297.0),
            padding: Frame {
                top: Mm(0.0),
                right: Mm(150.0),
                bottom: Mm(0.0),
                left: Mm(150.0),
            },
            static_schema: vec![],
        };

        for width in table.resolve_column_widths(&base_pdf) {
            assert!(width.0.is_finite());
            assert!(width.0 >= 0.0);
        }
    }

    #[test]
    fn table_width_is_capped_by_the_page_padding() {
        let json = create_test_table_schema_with_widths(vec![JsonColumnWidth::Text("100%".into())]);
        let font_map = create_real_test_font_map();
        let table = Table::from_json(json, &font_map).unwrap();

        // 210 - 20 - 20 = 170mm available, less than the table's own 190mm.
        let base_pdf = BasePdf {
            width: Mm(210.0),
            height: Mm(297.0),
            padding: Frame {
                top: Mm(0.0),
                right: Mm(20.0),
                bottom: Mm(0.0),
                left: Mm(20.0),
            },
            static_schema: vec![],
        };

        let widths = table.resolve_column_widths(&base_pdf);
        assert!((widths[0].0 - 170.0).abs() < 1e-3, "got {}", widths[0].0);
    }

    #[test]
    fn column_boundaries_land_where_expected() {
        // The cumulative x offsets are what actually position the cells, and a
        // 0.01mm drift is invisible by eye — pin them numerically.
        let widths = resolve_widths(vec!["20mm", "1fr", "25%"]);
        let mut x = 0.0_f32;
        let mut boundaries = vec![];
        for width in &widths {
            x += width;
            boundaries.push(x);
        }

        assert!((boundaries[0] - 20.0).abs() < 1e-3);
        assert!(
            (boundaries[1] - 142.5).abs() < 1e-3,
            "got {}",
            boundaries[1]
        );
        assert_eq!(boundaries[2], 190.0);
    }

    //
    // Structural validation
    //

    #[test]
    fn table_requires_at_least_one_column() {
        let mut json = create_test_table_schema();
        json.columns.clear();
        json.fields.clear();

        let font_map = create_real_test_font_map();
        let error = Table::from_json(json, &font_map).unwrap_err().to_string();
        assert!(error.contains("at least one column"), "got: {error}");
    }

    #[test]
    fn rows_must_match_the_column_count() {
        // Row rendering indexes columns by cell position, so a mismatched row
        // used to panic mid-render.
        for row in [
            vec!["only one".to_string()],
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        ] {
            let mut json = create_test_table_schema();
            let actual = row.len();
            json.fields = vec![vec!["a".to_string(), "b".to_string()], row];

            let font_map = create_real_test_font_map();
            let error = Table::from_json(json, &font_map).unwrap_err().to_string();

            assert!(error.contains("row 1"), "missing row index: {error}");
            assert!(error.contains("expected 2"), "missing expected: {error}");
            assert!(
                error.contains(&format!("got {actual}")),
                "missing actual: {error}"
            );
        }
    }

    #[test]
    fn headers_are_required_even_when_the_head_is_hidden() {
        // `showHead` controls painting only; the column shape stays the same.
        let mut json = create_test_table_schema();
        json.show_head = false;

        let font_map = create_real_test_font_map();
        let table = Table::from_json(json, &font_map).unwrap();
        assert_eq!(table.columns.len(), 2);
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

    fn create_base_pdf() -> BasePdf {
        BasePdf {
            width: Mm(200.0),
            height: Mm(300.0),
            padding: Frame {
                top: Mm(0.0),
                right: Mm(0.0),
                bottom: Mm(0.0),
                left: Mm(0.0),
            },
            static_schema: vec![],
        }
    }

    fn create_text_table_schema(
        head_styles_line_break_mode: Option<&str>,
        head_line_break_mode: Option<&str>,
        body_styles_line_break_mode: Option<&str>,
        column_line_break_mode: Option<&str>,
    ) -> JsonTableSchema {
        serde_json::from_value(json!({
            "name": "test_table",
            "position": { "x": 10.0, "y": 50.0 },
            "width": 100.0,
            "height": 100.0,
            "showHead": true,
            "headStyles": {
                "fontSize": 12.0,
                "fontName": "TestFont",
                "characterSpacing": 0.0,
                "alignment": "center",
                "verticalAlignment": "middle",
                "lineHeight": 1.0,
                "fontColor": "#ffffff",
                "borderColor": "#000000",
                "backgroundColor": "#2980ba",
                "borderWidth": { "top": 1.0, "right": 1.0, "bottom": 1.0, "left": 1.0 },
                "padding": { "top": 1.0, "right": 1.0, "bottom": 1.0, "left": 1.0 },
                "lineBreakMode": head_styles_line_break_mode
            },
            "bodyStyles": {
                "alignment": "left",
                "verticalAlignment": "middle",
                "characterSpacing": 0.0,
                "fontColor": "#000000",
                "lineHeight": 1.5,
                "backgroundColor": "#ffffff",
                "alternateBackgroundColor": "#f8f8f8",
                "padding": { "top": 1.0, "right": 1.0, "bottom": 1.0, "left": 1.0 },
                "lineBreakMode": body_styles_line_break_mode
            },
            "tableStyles": {
                "borderWidth": 0.3,
                "borderColor": "#000000"
            },
            "columns": [{
                "width": "100%",
                "header": {
                    "content": "Header Value",
                    "fontSize": 12.0,
                    "fontName": "TestFont",
                    "characterSpacing": 0.0,
                    "alignment": "center",
                    "verticalAlignment": "middle",
                    "lineBreakMode": head_line_break_mode
                },
                "cell": {
                    "type": "text",
                    "name": "body_column",
                    "position": { "x": 0.0, "y": 0.0 },
                    "width": 0.0,
                    "height": 0.0,
                    "content": "",
                    "fontName": "TestFont",
                    "fontSize": 10.0,
                    "lineBreakMode": column_line_break_mode
                }
            }],
            "fields": [["Body Value"]]
        }))
        .unwrap()
    }

    #[test]
    fn body_styles_default_line_break_mode_is_char() {
        let body_styles = BodyStyles::from_json(create_test_body_styles()).unwrap();

        assert_eq!(body_styles.line_break_mode, LineBreakMode::Char);
    }

    #[test]
    fn table_header_uses_head_styles_line_break_mode_by_default() {
        let font_map = create_real_test_font_map();
        let json = create_text_table_schema(Some("char"), None, None, None);
        let table = Table::from_json(json, &font_map).unwrap();

        assert_eq!(
            table.columns[0].header.resolved_line_break_mode(),
            LineBreakMode::Char
        );
    }

    #[test]
    fn table_header_item_line_break_mode_overrides_head_styles_default() {
        let font_map = create_real_test_font_map();
        let json = create_text_table_schema(Some("word"), Some("char"), None, None);
        let table = Table::from_json(json, &font_map).unwrap();

        assert_eq!(
            table.columns[0].header.resolved_line_break_mode(),
            LineBreakMode::Char
        );
    }

    #[test]
    fn table_body_uses_body_styles_line_break_mode_when_column_is_unspecified() {
        let font_map = create_real_test_font_map();
        let json = create_text_table_schema(None, None, None, None);
        let table = Table::from_json(json, &font_map).unwrap();
        let cell_widths = vec![Mm(100.0)];
        let base_pdf = create_base_pdf();
        let (schemas, _) = table
            .process_row(
                vec!["Body Value".to_string()],
                Mm(0.0),
                &cell_widths,
                &base_pdf,
            )
            .unwrap();

        match &schemas[0] {
            Schema::Text(text) => assert_eq!(text.resolved_line_break_mode(), LineBreakMode::Char),
            _ => panic!("expected text schema"),
        }
    }

    #[test]
    fn table_body_column_line_break_mode_overrides_body_styles_default() {
        let font_map = create_real_test_font_map();
        let json = create_text_table_schema(None, None, Some("char"), Some("word"));
        let table = Table::from_json(json, &font_map).unwrap();
        let cell_widths = vec![Mm(100.0)];
        let base_pdf = create_base_pdf();
        let (schemas, _) = table
            .process_row(
                vec!["Body Value".to_string()],
                Mm(0.0),
                &cell_widths,
                &base_pdf,
            )
            .unwrap();

        match &schemas[0] {
            Schema::Text(text) => assert_eq!(text.resolved_line_break_mode(), LineBreakMode::Word),
            _ => panic!("expected text schema"),
        }
    }

    #[test]
    fn spacer_creates_exact_gap_between_flowing_tables() {
        let font_map = create_real_test_font_map();
        let mut first =
            Table::from_json(create_text_table_schema(None, None, None, None), &font_map).unwrap();
        let mut second = first.clone();
        let spacer = Spacer::from_json(JsonSpacerSchema { height: 5.0 }).unwrap();
        let base_pdf = create_base_pdf();
        let mut doc = PdfDocument::new("spacer table test");
        let mut buffer = OpBuffer::default();
        let mut cursor = FlowCursor::new(0);

        (cursor.page, cursor.y) = first
            .render(&base_pdf, &mut doc, cursor.page, cursor.y, &mut buffer)
            .unwrap();
        let first_end = cursor.y.unwrap();

        spacer.advance(&base_pdf, &mut cursor);

        assert_eq!(cursor.y, Some(first_end + Mm(5.0)));

        let second_start = cursor.y.unwrap();
        (cursor.page, cursor.y) = second
            .render(&base_pdf, &mut doc, cursor.page, cursor.y, &mut buffer)
            .unwrap();

        let first_height = first_end - Mm(50.0);
        assert_eq!(cursor.y, Some(second_start + first_height));
    }

    #[test]
    fn dynamic_text_spacer_and_table_share_one_flow_cursor() {
        let font_map = create_real_test_font_map();
        let dynamic_json: crate::schemas::dynamic_text::JsonDynamicTextSchema =
            serde_json::from_value(json!({
                "name": "intro",
                "position": { "x": 10.0, "y": 20.0 },
                "width": 100.0,
                "height": 20.0,
                "content": "Introduction",
                "fontName": "TestFont",
                "fontSize": 10.0,
                "lineHeight": 1.0
            }))
            .unwrap();
        let mut dynamic =
            crate::schemas::dynamic_text::DynamicText::from_json(dynamic_json, &font_map).unwrap();
        let spacer = Spacer::from_json(JsonSpacerSchema { height: 5.0 }).unwrap();
        let mut table =
            Table::from_json(create_text_table_schema(None, None, None, None), &font_map).unwrap();
        let mut measuring_table = table.clone();
        let base_pdf = create_base_pdf();
        let mut doc = PdfDocument::new("mixed flow test");
        let mut buffer = OpBuffer::default();
        let (_, measured_end) = measuring_table
            .render(&base_pdf, &mut doc, 0, Some(Mm(0.0)), &mut buffer)
            .unwrap();
        let table_height = measured_end.unwrap();
        let mut cursor = FlowCursor::new(0);

        (cursor.page, cursor.y) = dynamic
            .render(&base_pdf, cursor.page, cursor.y, &mut buffer)
            .unwrap();
        let dynamic_end = cursor.y.unwrap();
        spacer.advance(&base_pdf, &mut cursor);

        assert_eq!(cursor.y, Some(dynamic_end + Mm(5.0)));

        let table_start = cursor.y.unwrap();
        (cursor.page, cursor.y) = table
            .render(&base_pdf, &mut doc, cursor.page, cursor.y, &mut buffer)
            .unwrap();

        assert_eq!(cursor.y, Some(table_start + table_height));
    }

    #[test]
    fn header_row_style_uses_head_background_distinct_from_body() {
        let font_map = create_real_test_font_map();
        let table = Table::from_json(create_test_table_schema(), &font_map).unwrap();

        let header = table.header_row_style();
        let body = table.body_row_style(0);

        assert_eq!(header.background, csscolorparser::parse("#2980ba").unwrap());
        assert_eq!(body.background, csscolorparser::parse("#ffffff").unwrap());
        assert_ne!(header.background, body.background);
    }

    #[test]
    fn header_text_uses_head_font_color() {
        let font_map = create_real_test_font_map();
        let table = Table::from_json(create_test_table_schema(), &font_map).unwrap();

        assert_eq!(
            *table.columns[0].header.font_color(),
            csscolorparser::parse("#ffffff").unwrap()
        );
    }

    #[test]
    fn body_row_style_uses_configured_table_border_color() {
        let font_map = create_real_test_font_map();
        let mut json = create_test_table_schema();
        json.table_styles.border_color = "#ff0000".to_string();
        let table = Table::from_json(json, &font_map).unwrap();

        assert_eq!(
            table.body_row_style(0).border_color,
            csscolorparser::parse("#ff0000").unwrap()
        );
    }

    #[test]
    fn body_row_style_applies_alternate_background_on_odd_rows() {
        let font_map = create_real_test_font_map();
        let table = Table::from_json(create_test_table_schema(), &font_map).unwrap();

        assert_eq!(
            table.body_row_style(0).background,
            csscolorparser::parse("#ffffff").unwrap()
        );
        assert_eq!(
            table.body_row_style(1).background,
            csscolorparser::parse("#f8f8f8").unwrap()
        );
    }

    #[test]
    fn header_row_style_preserves_non_uniform_border_width() {
        let font_map = create_real_test_font_map();
        let mut json = create_test_table_schema();
        json.head_styles.border_width = JsonFrame {
            top: 2.0,
            right: 0.0,
            bottom: 3.0,
            left: 0.0,
        };
        let table = Table::from_json(json, &font_map).unwrap();

        let border = table.header_row_style().border_width;
        assert_eq!(border.top, Mm(2.0));
        assert_eq!(border.right, Mm(0.0));
        assert_eq!(border.bottom, Mm(3.0));
        assert_eq!(border.left, Mm(0.0));
    }

    #[test]
    fn parse_color_or_falls_back_on_empty_or_blank() {
        assert_eq!(
            parse_color_or("", "#123456").unwrap(),
            csscolorparser::parse("#123456").unwrap()
        );
        assert_eq!(
            parse_color_or("   ", "#123456").unwrap(),
            csscolorparser::parse("#123456").unwrap()
        );
    }

    #[test]
    fn parse_color_or_uses_non_empty_value() {
        assert_eq!(
            parse_color_or("#ff0000", "#000000").unwrap(),
            csscolorparser::parse("#ff0000").unwrap()
        );
    }

    #[test]
    fn parse_color_or_errors_on_invalid_non_empty_color() {
        assert!(parse_color_or("not-a-color", "#000000").is_err());
    }
}
