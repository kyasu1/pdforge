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
