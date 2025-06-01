use super::{
    Alignment, BasePdf, Error, FontSnafu, Frame, InvalidColorSnafu, JsonPosition, VerticalAlignment,
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
    font_color: csscolorparser::Color,
    background_color: Option<csscolorparser::Color>,
    padding: Option<Frame>,
    rotate: Option<f32>,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
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
            font_color: "#000"
                .parse::<csscolorparser::Color>()
                .context(InvalidColorSnafu)?,
            background_color: None,
            padding,
            rotate: None,
            scale_x: None,
            scale_y: None,
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

        let character_spacing = json.character_spacing.map(|f| Pt(f)).unwrap_or(Pt(0.0));
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
            font_spec: Arc::new(font_spec.clone()),
            font_color,
            background_color,
            padding: json.padding,
            rotate: json.rotate,
            scale_x: json.scale_x,
            scale_y: json.scale_y,
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

        // 各行のテキストを描画
        for (index, line) in splitted_paragraphs.iter().enumerate() {
            let line_width: Mm = self
                .font_spec
                .width_of_text_at_size(line.clone(), font_size, self.character_spacing)
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
                        + FontSpec::calculate_character_spacing(line.to_string(), residual).into()
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
        super::pdf_utils::create_text_ops(
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
            _text: String,
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

    fn create_test_text() -> Text {
        let base = BaseSchema::new("test".to_string(), Mm(10.0), Mm(10.0), Mm(100.0), Mm(50.0));
        let font_id = FontId::new();
        let font_spec = MockFontSpec {
            width_result: Pt(20.0),
            height: Pt(12.0),
        };

        Text {
            base,
            content: "Test content".to_string(),
            alignment: Alignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            character_spacing: Pt(0.0),
            line_height: Some(1.2),
            font_size: FontSize::Fixed(Pt(12.0)),
            font_id,
            font_spec: Arc::new(font_spec),
            font_color: "#000".parse().unwrap(),
            background_color: None,
            padding: None,
            rotate: None,
            scale_x: None,
            scale_y: None,
        }
    }

    #[test]
    fn test_get_effective_box_size_no_padding() {
        let text = create_test_text();
        let (width, height) = text.get_effective_box_size();

        assert_eq!(width, Mm(100.0));
        assert_eq!(height, Mm(50.0));
    }

    #[test]
    fn test_get_effective_box_size_with_padding() {
        let mut text = create_test_text();
        text.padding = Some(Frame {
            top: Mm(5.0),
            right: Mm(10.0),
            bottom: Mm(5.0),
            left: Mm(10.0),
        });

        let (width, height) = text.get_effective_box_size();

        assert_eq!(width, Mm(80.0)); // 100 - (10 + 10)
        assert_eq!(height, Mm(40.0)); // 50 - (5 + 5)
    }

    #[test]
    fn test_vertical_alignment_top() {
        let mut text = create_test_text();
        text.vertical_alignment = VerticalAlignment::Top;

        let offset = text.calculate_vertical_offset(Mm(50.0), Mm(30.0));
        assert_eq!(offset, Mm(0.0));
    }

    #[test]
    fn test_vertical_alignment_middle() {
        let mut text = create_test_text();
        text.vertical_alignment = VerticalAlignment::Middle;

        let offset = text.calculate_vertical_offset(Mm(50.0), Mm(30.0));
        assert_eq!(offset, Mm(10.0)); // (50 - 30) / 2
    }

    #[test]
    fn test_vertical_alignment_bottom() {
        let mut text = create_test_text();
        text.vertical_alignment = VerticalAlignment::Bottom;

        let offset = text.calculate_vertical_offset(Mm(50.0), Mm(30.0));
        assert_eq!(offset, Mm(20.0)); // 50 - 30
    }

    #[test]
    fn test_horizontal_alignment_left() {
        let mut text = create_test_text();
        text.alignment = Alignment::Left;

        let (x, spacing) = text.calculate_horizontal_alignment(Mm(100.0), Mm(70.0), "Test");
        assert_eq!(x, Mm(10.0)); // base.x (padding.left = 0)
        assert_eq!(spacing, Pt(0.0));
    }

    #[test]
    fn test_horizontal_alignment_center() {
        let mut text = create_test_text();
        text.alignment = Alignment::Center;

        let (x, spacing) = text.calculate_horizontal_alignment(Mm(100.0), Mm(70.0), "Test");
        assert_eq!(x, Mm(25.0)); // 10 + (100 - 70) / 2
        assert_eq!(spacing, Pt(0.0));
    }

    #[test]
    fn test_horizontal_alignment_right() {
        let mut text = create_test_text();
        text.alignment = Alignment::Right;

        let (x, spacing) = text.calculate_horizontal_alignment(Mm(100.0), Mm(70.0), "Test");
        assert_eq!(x, Mm(40.0)); // 10 + (100 - 70)
        assert_eq!(spacing, Pt(0.0));
    }

    #[test]
    fn test_horizontal_alignment_justify() {
        let mut text = create_test_text();
        text.alignment = Alignment::Justify;

        // モッキングが必要かもしれませんが、基本的なテスト
        let (x, _) = text.calculate_horizontal_alignment(Mm(100.0), Mm(70.0), "Test");
        assert_eq!(x, Mm(10.0)); // base.x
                                 // 文字間隔は実際の計算結果に依存するため、ここではテストしない
    }
}
