use printpdf::*;

pub fn create_text_ops(
    font_id: &FontId,
    font_size: Pt,
    x_line: Mm,
    y: Mm,
    character_spacing: Pt,
    line: &str,
    line_height: Option<f32>,
    font_color: &csscolorparser::Color,
) -> Vec<Op> {
    vec![
        Op::StartTextSection,
        Op::SetLineHeight {
            lh: font_size * line_height.unwrap_or(1.0),
        },
        Op::SetFillColor {
            col: Color::Rgb(Rgb {
                r: font_color.r,
                g: font_color.g,
                b: font_color.b,
                icc_profile: None,
            }),
        },
        Op::SetFontSize {
            size: font_size,
            font: font_id.clone(),
        },
        Op::SetTextCursor {
            pos: Point {
                x: x_line.into(),
                y: y.into(),
            },
        },
        Op::SetCharacterSpacing {
            multiplier: character_spacing.0,
        },
        Op::WriteText {
            items: vec![TextItem::Text(line.to_string())],
            font: font_id.clone(),
        },
        Op::EndTextSection,
    ]
}

#[derive(Debug, Clone)]
pub struct DrawRectangle {
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,
    pub page_height: Mm,
    pub color: Option<Color>,
    pub border_width: Option<Mm>,
    pub border_color: Option<Color>,
}

pub fn draw_rectangle(props: DrawRectangle) -> Vec<Op> {
    let mode = match props.color {
        Some(_) => PaintMode::FillStroke,
        None => PaintMode::Stroke,
    };

    let mode = if props.color == props.border_color {
        PaintMode::Fill
    } else {
        PaintMode::FillStroke
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
    let y2: Pt = (props.page_height - props.y - props.height).into();

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

    vec![
        Op::SetOutlineColor { col: border_color },
        Op::SetFillColor { col: color },
        Op::SetOutlineThickness {
            pt: border_width.into(),
        },
        Op::DrawPolygon {
            polygon: polygon.clone(),
        },
    ]
}
