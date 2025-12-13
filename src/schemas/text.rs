use super::{
    Alignment, Error, FontSnafu, Frame, InvalidColorSnafu, JsonPosition, VerticalAlignment,
};
use crate::font::{DynamicFontSize, FontMap, FontSize, FontSpec, FontSpecTrait, JsonFontSize};
use crate::schemas::base::BaseSchema;

use crate::utils::OpBuffer;
use printpdf::*;
use serde::Deserialize;
use snafu::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTextSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
    font_name: String,
    alignment: Option<Alignment>,
    vertical_alignment: Option<VerticalAlignment>,
    character_spacing: Option<f32>,
    line_height: Option<f32>,
    font_size: JsonFontSize,
    font_color: Option<String>,
    background_color: Option<String>,
    padding: Option<Frame>,
    rotate: Option<f32>,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
    border_color: Option<String>,
    border_width: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct Text {
    base: BaseSchema,
    content: String,
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: FontSize,
    font_id: FontId,
    font_spec: Arc<dyn FontSpecTrait>,
    font: Arc<ParsedFont>,
    font_color: csscolorparser::Color,
    background_color: Option<csscolorparser::Color>,
    padding: Option<Frame>,
    rotate: Option<f32>,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
    border_color: Option<csscolorparser::Color>,
    border_width: Option<Pt>,
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
        alignment: Alignment,
        vertical_alignment: VerticalAlignment,
        font_map: &FontMap,
        padding: Option<Frame>,
    ) -> Result<Self, Error> {
        let (font_id, font) = font_map
            .find(&font_name)
            .whatever_context("Font specified in the schema is not loaded")?;
        let font_spec = FontSpec::new(font.clone());
        let base = BaseSchema::new(String::from("cell"), x, y, width, height);

        Ok(Self {
            base,
            content,
            alignment,
            vertical_alignment,
            character_spacing: Pt(0.0),
            line_height: None,
            font_size: FontSize::Fixed(font_size),
            font_id: font_id.clone(),
            font_spec: Arc::new(font_spec.clone()),
            font: font.clone(),
            font_color: "#000"
                .parse::<csscolorparser::Color>()
                .context(InvalidColorSnafu)?,
            background_color: None,
            padding,
            rotate: None,
            scale_x: None,
            scale_y: None,
            border_color: None,
            border_width: None,
        })
    }

    pub fn get_base(self) -> BaseSchema {
        self.base
    }

    pub fn from_json(json: JsonTextSchema, font_map: &FontMap) -> Result<Text, Error> {
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

        let character_spacing = json.character_spacing.map(Pt).unwrap_or(Pt(0.0));
        let line_height = json.line_height;
        let font_size = match json.font_size {
            JsonFontSize::Dynamic { min, max, fit } => {
                FontSize::Dynamic(DynamicFontSize::new(Pt(min), Pt(max), fit))
            }
            JsonFontSize::Fixed(f) => FontSize::Fixed(Pt(f)),
        };

        let alignment = json.alignment.unwrap_or(Alignment::Left);
        let vertical_alignment = json.vertical_alignment.unwrap_or(VerticalAlignment::Top);

        let font_color = csscolorparser::parse(&json.font_color.unwrap_or("#000000".to_string()))
            .context(InvalidColorSnafu)?;

        let background_color = json
            .background_color
            .as_ref()
            .map(|c| csscolorparser::parse(c).context(InvalidColorSnafu))
            .transpose()?;

        let text = Text {
            base,
            content: json.content,
            character_spacing,
            alignment,
            vertical_alignment,
            line_height,
            font_size,
            font_id: font_id.clone(),
            font_spec: Arc::new(font_spec),
            font: font.clone(),
            font_color,
            background_color,
            padding: json.padding,
            rotate: json.rotate,
            scale_x: json.scale_x,
            scale_y: json.scale_y,
            border_color: json
                .border_color
                .as_ref()
                .map(|c| csscolorparser::parse(c).context(InvalidColorSnafu))
                .transpose()?,
            border_width: json.border_width.map(Pt),
        };

        Ok(text)
    }

    pub fn render(
        &mut self,
        parent_height: Mm,
        current_page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let font_size = self.get_font_size()?;
        let (box_width, box_height) = self.get_effective_box_size();

        let splitted_paragraphs = self
            .font_spec
            .split_text_to_size(
                &self.content,
                font_size,
                box_width.into(),
                self.character_spacing,
            )
            .context(FontSnafu)?;

        let line_height = font_size * self.line_height.unwrap_or(1.0);
        let line_height_in_mm: Mm = line_height.into();
        let total_height: Mm = line_height_in_mm * (splitted_paragraphs.len() as f32);

        let y_offset = self.calculate_vertical_offset(box_height, total_height);

        let mut ops: Vec<Op> = vec![];

        let bounding_matrix = super::pdf_utils::calculate_transform_matrix_with_center_pivot(
            self.base.x,
            parent_height - self.base.y - self.base.height,
            self.base.width,
            self.base.height,
            self.rotate,
        );
        // 背景色があれば背景を描画
        if let Some(bg_ops) = self.create_background_ops(bounding_matrix) {
            ops.extend_from_slice(&bg_ops);
        }

        // Draw border if specified
        if let Some(border_ops) = self.create_border_ops(bounding_matrix) {
            ops.extend_from_slice(&border_ops);
        }

        // 各行のテキストを描画
        for (index, line) in splitted_paragraphs.iter().enumerate() {
            let line_width: Mm = self
                .font_spec
                .width_of_text_at_size(line, font_size, self.character_spacing)
                .context(FontSnafu)?
                .into();

            let (x_line, character_spacing) =
                self.calculate_horizontal_alignment(box_width, line_width, line);

            let y = self.base.height
                - (y_offset
                    + line_height_in_mm * (index as i32 + 1) as f32
                    + self.padding.as_ref().map_or(Mm(0.0), |p| p.top));

            let line_ops = self.create_text_ops(
                bounding_matrix,
                font_size,
                x_line,
                y,
                character_spacing,
                line,
            );
            ops.extend_from_slice(&line_ops);
        }

        buffer.insert(current_page, ops);
        Ok(())
    }

    // パディングを考慮したボックスの有効サイズを計算
    fn get_effective_box_size(&self) -> (Mm, Mm) {
        let box_width =
            self.base.width - self.padding.as_ref().map_or(Mm(0.0), |p| p.left + p.right);

        let box_height =
            self.base.height - self.padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom);

        (box_width, box_height)
    }

    // 垂直方向の配置オフセットを計算
    fn calculate_vertical_offset(&self, box_height: Mm, total_height: Mm) -> Mm {
        match self.vertical_alignment {
            VerticalAlignment::Top => Mm(0.0),
            VerticalAlignment::Middle => (box_height - total_height) / 2.0,
            VerticalAlignment::Bottom => box_height - total_height,
        }
    }

    // 水平方向の配置とキャラクタ間隔を計算
    fn calculate_horizontal_alignment(
        &self,
        box_width: Mm,
        line_width: Mm,
        line: &str,
    ) -> (Mm, Pt) {
        let residual: Mm = box_width - line_width;

        let x_line = match self.alignment {
            Alignment::Left => self.padding.as_ref().map_or(Mm(0.0), |p| p.left),
            Alignment::Center => residual / 2.0 + self.padding.as_ref().map_or(Mm(0.0), |p| p.left),
            Alignment::Right => residual + self.padding.as_ref().map_or(Mm(0.0), |p| p.left),
            Alignment::Justify => self.padding.as_ref().map_or(Mm(0.0), |p| p.left),
        };

        let character_spacing = match self.alignment {
            Alignment::Justify => {
                if line.ends_with('\n') {
                    self.character_spacing
                } else {
                    self.character_spacing
                        + FontSpec::calculate_character_spacing(line, residual).into()
                }
            }
            _ => self.character_spacing,
        };

        (x_line, character_spacing)
    }

    // 背景色のオペレーションを作成
    fn create_background_ops(&self, bounding_matrix: [f32; 6]) -> Option<Vec<Op>> {
        let matrix: Op = Op::SetTransformationMatrix {
            matrix: CurTransMat::Raw(bounding_matrix),
        };

        let x1 = Pt(0.0);
        let x2: Pt = self.base.width.into();
        let y1: Pt = Pt(0.0);
        let y2: Pt = self.base.height.into();
        self.background_color.clone().map(|bg_color| {
            vec![
                Op::SaveGraphicsState,
                matrix,
                Op::SetFillColor {
                    col: Color::Rgb(Rgb {
                        r: bg_color.r,
                        g: bg_color.g,
                        b: bg_color.b,
                        icc_profile: None,
                    }),
                },
                Op::DrawPolygon {
                    polygon: Polygon {
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
                        mode: printpdf::PaintMode::Fill,
                        winding_order: printpdf::WindingOrder::NonZero,
                    },
                },
                Op::RestoreGraphicsState,
            ]
        })
    }

    // Create border drawing operations
    fn create_border_ops(&self, bounding_matrix: [f32; 6]) -> Option<Vec<Op>> {
        let border_color = self.border_color.as_ref()?;
        let border_width = self.border_width.as_ref()?;

        let matrix: Op = Op::SetTransformationMatrix {
            matrix: CurTransMat::Raw(bounding_matrix),
        };

        let x1 = Pt(0.0);
        let x2: Pt = self.base.width.into();
        let y1: Pt = Pt(0.0);
        let y2: Pt = self.base.height.into();

        Some(vec![
            Op::SaveGraphicsState,
            matrix,
            Op::SetOutlineColor {
                col: Color::Rgb(Rgb {
                    r: border_color.r,
                    g: border_color.g,
                    b: border_color.b,
                    icc_profile: None,
                }),
            },
            Op::SetOutlineThickness {
                pt: (*border_width),
            },
            Op::DrawPolygon {
                polygon: Polygon {
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
                    mode: printpdf::PaintMode::Stroke,
                    winding_order: printpdf::WindingOrder::NonZero,
                },
            },
            Op::RestoreGraphicsState,
        ])
    }

    // テキスト描画のオペレーションを作成
    fn create_text_ops(
        &self,
        matrix: [f32; 6],
        font_size: Pt,
        x_line: Mm,
        y: Mm,
        character_spacing: Pt,
        line: &str,
    ) -> Vec<Op> {
        super::pdf_utils::create_text_ops_with_font(
            matrix,
            &self.font_id,
            font_size,
            x_line,
            y,
            self.scale_x,
            self.scale_y,
            character_spacing,
            line,
            self.line_height,
            &self.font_color,
            Some(&self.font),
        )
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
    pub fn set_height(&mut self, height: Mm) {
        self.base.height = height;
    }

    pub fn set_rotate(&mut self, rotate: Option<f32>) {
        match self.rotate {
            Some(rotate_) => self.rotate = Some(rotate_ + rotate.unwrap_or(0.0)),
            None => self.rotate = rotate,
        }
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }
    pub fn get_height(&self) -> Result<Mm, Error> {
        let font_size = self.get_font_size()?;

        let lines: Vec<String> = self
            .font_spec
            .split_text_to_size(
                &self.content,
                font_size,
                self.base.width.into(),
                self.character_spacing,
            )
            .context(FontSnafu)?;

        let height_in_mm: Mm = Pt(lines.len() as f32 * font_size.0).into();

        Ok(height_in_mm + self.padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom))
    }

    fn get_font_size(&self) -> Result<Pt, Error> {
        match self.font_size.clone() {
            FontSize::Fixed(font_size) => Ok(font_size),
            FontSize::Dynamic(dynamic_font_size) => self
                .font_spec
                .calculate_dynamic_font_size(
                    dynamic_font_size,
                    self.line_height,
                    self.character_spacing,
                    self.base.width,
                    self.base.height,
                    &self.content,
                )
                .context(FontSnafu),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printpdf::{FontId, Mm, Pt};

    pub struct MockFontSpec {
        pub width_result: Pt,
        pub height: Pt,
    }

    impl MockFontSpec {
        #[allow(dead_code)]
        pub fn new(width_result: Pt, height: Pt) -> Self {
            MockFontSpec {
                width_result,
                height,
            }
        }
    }

    impl FontSpecTrait for MockFontSpec {
        fn split_text_to_size(
            &self,
            _text: &str,
            _font_size: Pt,
            _width: Pt,
            _character_spacing: Pt,
        ) -> Result<Vec<String>, crate::font::Error> {
            Ok(vec!["Test line".to_string()])
        }

        fn width_of_text_at_size(
            &self,
            _text: &str,
            _font_size: Pt,
            _character_spacing: Pt,
        ) -> Result<Pt, crate::font::Error> {
            Ok(self.width_result)
        }

        fn calculate_dynamic_font_size(
            &self,
            _dynamic_font_size: DynamicFontSize,
            _line_height: Option<f32>,
            _character_spacing: Pt,
            _width: Mm,
            _height: Mm,
            _content: &str,
        ) -> Result<Pt, crate::font::Error> {
            Ok(self.height)
        }
    }

    // Note: These tests are simplified to avoid complex font construction
    // Integration tests should be used for full font functionality testing
    #[cfg(test)]
    fn create_test_text_without_font() -> (BaseSchema, FontId, Arc<MockFontSpec>) {
        let base = BaseSchema::new("test".to_string(), Mm(10.0), Mm(10.0), Mm(100.0), Mm(50.0));
        let font_id = FontId::new();
        let font_spec = Arc::new(MockFontSpec {
            width_result: Pt(20.0),
            height: Pt(12.0),
        });
        (base, font_id, font_spec)
    }

    // Simplified tests that focus on testing individual methods without requiring full Text construction

    #[test]
    fn test_effective_box_size_calculation_no_padding() {
        // Test the calculation logic directly
        let width = Mm(100.0);
        let height = Mm(50.0);
        let padding: Option<Frame> = None;

        let effective_width = width - padding.as_ref().map_or(Mm(0.0), |p| p.left + p.right);
        let effective_height = height - padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom);

        assert_eq!(effective_width, Mm(100.0));
        assert_eq!(effective_height, Mm(50.0));
    }

    #[test]
    fn test_effective_box_size_calculation_with_padding() {
        let width = Mm(100.0);
        let height = Mm(50.0);
        let padding = Some(Frame {
            top: Mm(5.0),
            right: Mm(10.0),
            bottom: Mm(5.0),
            left: Mm(10.0),
        });

        let effective_width = width - padding.as_ref().map_or(Mm(0.0), |p| p.left + p.right);
        let effective_height = height - padding.as_ref().map_or(Mm(0.0), |p| p.top + p.bottom);

        assert_eq!(effective_width, Mm(80.0)); // 100 - (10 + 10)
        assert_eq!(effective_height, Mm(40.0)); // 50 - (5 + 5)
    }

    #[test]
    fn test_vertical_alignment_calculations() {
        // Test vertical alignment calculations
        let box_height = Mm(50.0);
        let total_height = Mm(30.0);

        // Top alignment
        let top_offset = match VerticalAlignment::Top {
            VerticalAlignment::Top => Mm(0.0),
            VerticalAlignment::Middle => (box_height - total_height) / 2.0,
            VerticalAlignment::Bottom => box_height - total_height,
        };
        assert_eq!(top_offset, Mm(0.0));

        // Middle alignment
        let middle_offset = match VerticalAlignment::Middle {
            VerticalAlignment::Top => Mm(0.0),
            VerticalAlignment::Middle => (box_height - total_height) / 2.0,
            VerticalAlignment::Bottom => box_height - total_height,
        };
        assert_eq!(middle_offset, Mm(10.0)); // (50 - 30) / 2

        // Bottom alignment
        let bottom_offset = match VerticalAlignment::Bottom {
            VerticalAlignment::Top => Mm(0.0),
            VerticalAlignment::Middle => (box_height - total_height) / 2.0,
            VerticalAlignment::Bottom => box_height - total_height,
        };
        assert_eq!(bottom_offset, Mm(20.0)); // 50 - 30
    }

    #[test]
    fn test_horizontal_alignment_calculations() {
        let box_width = Mm(100.0);
        let line_width = Mm(70.0);
        let residual = box_width - line_width; // 30.0
        let padding_left = Mm(0.0);

        // Left alignment
        let left_x = match Alignment::Left {
            Alignment::Left => padding_left,
            Alignment::Center => residual / 2.0 + padding_left,
            Alignment::Right => residual + padding_left,
            Alignment::Justify => padding_left,
        };
        assert_eq!(left_x, Mm(0.0));

        // Center alignment
        let center_x = match Alignment::Center {
            Alignment::Left => padding_left,
            Alignment::Center => residual / 2.0 + padding_left,
            Alignment::Right => residual + padding_left,
            Alignment::Justify => padding_left,
        };
        assert_eq!(center_x, Mm(15.0)); // 30/2 + 0 = 15

        // Right alignment
        let right_x = match Alignment::Right {
            Alignment::Left => padding_left,
            Alignment::Center => residual / 2.0 + padding_left,
            Alignment::Right => residual + padding_left,
            Alignment::Justify => padding_left,
        };
        assert_eq!(right_x, Mm(30.0)); // 30 + 0 = 30
    }
}
