use printpdf::*;

/// Replace characters that don't have glyphs in the font with a replacement character
/// Uses .notdef glyph approach by falling back to basic ASCII characters that should exist in most fonts
pub fn sanitize_text_for_font(text: &str, font: &ParsedFont) -> String {
    // We can't directly access .notdef glyph (index 0) through normal text rendering
    // in printpdf library, so we use a fallback hierarchy of replacement characters
    const FALLBACK_CHARS: &[char] = &[
        '□',    // U+25A1 White Square (classic TOFU character)
        '\u{FFFD}', // U+FFFD Replacement Character (Unicode standard replacement)
        '?',    // Question mark (widely supported)
        '.',    // Period (basic punctuation)
        'X',    // Letter X (ASCII fallback)
        ' ',    // Space (should exist in any font)
    ];
    
    text.chars()
        .map(|ch| {
            // Check if the character has a glyph in the font
            match font.lookup_glyph_index(ch as u32) {
                Some(_) => ch, // Character exists in font
                None => {
                    // Try each fallback character in order of preference
                    // This simulates .notdef behavior by using the best available replacement
                    for &fallback in FALLBACK_CHARS {
                        if font.lookup_glyph_index(fallback as u32).is_some() {
                            return fallback;
                        }
                    }
                    // Ultimate fallback - return the original character
                    // This may cause the font's actual .notdef glyph to be used at PDF level
                    ch
                }
            }
        })
        .collect()
}

// 2D変換行列を計算する独立した関数
pub fn calculate_transform_matrix(
    x: Mm,
    y: Mm,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
) -> [f32; 6] {
    let x_in_pt: Pt = x.into();
    let y_in_pt: Pt = y.into();

    // スケール値を取得
    let sx = scale_x.unwrap_or(1.0);
    let sy = scale_y.unwrap_or(1.0);

    // 2D変換行列: [a b c d e f] = [sx*cos -sx*sin sy*sin sy*cos tx ty]
    // 回転とスケールを組み合わせた変換行列
    [
        sx,        // a: x方向のスケール * 回転のcos成分
        0.0,       // b: x方向のスケール * 回転の-sin成分
        0.0,       // c: y方向のスケール * 回転のsin成分
        sy,        // d: y方向のスケール * 回転のcos成分
        x_in_pt.0, // e: x座標の平行移動
        y_in_pt.0, // f: y座標の平行移動
    ]
}
// バウンディングボックスの中心をピボットとして回転を考慮したマトリックスを計算
pub fn calculate_transform_matrix_with_center_pivot(
    x: Mm,
    y: Mm,
    width: Mm,
    height: Mm,
    rotate: Option<f32>,
) -> [f32; 6] {
    let x_in_pt: Pt = x.into();
    let y_in_pt: Pt = y.into();
    let width_in_pt: Pt = width.into();
    let height_in_pt: Pt = height.into();

    // ボックスの中心点を計算（左下座標からの相対位置）
    let center_x = width_in_pt / 2.0;
    let center_y = height_in_pt / 2.0;

    // 回転角度をラジアンに変換
    let rotation_radians = rotate.unwrap_or(0.0) * std::f32::consts::PI / 180.0;
    let cos_theta = rotation_radians.cos();
    let sin_theta = rotation_radians.sin();

    let tx = x_in_pt + center_x - center_x * cos_theta + center_y * sin_theta;
    let ty = y_in_pt + center_y - center_x * sin_theta - center_y * cos_theta;
    [
        cos_theta,  // a: 回転のcos成分
        sin_theta,  // b: 回転の-sin成分
        -sin_theta, // c: 回転のsin成分
        cos_theta,  // d: 回転のcos成分
        tx.0,       // e: 中心基準回転を考慮したx座標の平行移動
        ty.0,       // f: 中心基準回転を考慮したy座標の平行移動
    ]
}

pub fn create_text_ops(
    bounding_matrix: [f32; 6],
    font_id: &FontId,
    font_size: Pt,
    x_line: Mm,
    y: Mm,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
    character_spacing: Pt,
    line: &str,
    line_height: Option<f32>,
    font_color: &csscolorparser::Color,
) -> Vec<Op> {
    create_text_ops_with_font(
        bounding_matrix,
        font_id,
        font_size,
        x_line,
        y,
        scale_x,
        scale_y,
        character_spacing,
        line,
        line_height,
        font_color,
        None,
    )
}

pub fn create_text_ops_with_font(
    bounding_matrix: [f32; 6],
    font_id: &FontId,
    font_size: Pt,
    x_line: Mm,
    y: Mm,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
    character_spacing: Pt,
    line: &str,
    line_height: Option<f32>,
    font_color: &csscolorparser::Color,
    font: Option<&ParsedFont>,
) -> Vec<Op> {
    let matrix_values = calculate_transform_matrix(x_line, y, scale_x, scale_y);
    let matrix = TextMatrix::Raw(matrix_values);

    // Sanitize text if font is provided, otherwise use original text
    let sanitized_line = match font {
        Some(font_ref) => sanitize_text_for_font(line, font_ref),
        None => line.to_string(),
    };

    vec![
        Op::SaveGraphicsState,
        Op::SetTransformationMatrix {
            matrix: CurTransMat::Raw(bounding_matrix),
        },
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
        Op::SetTextMatrix { matrix },
        Op::SetCharacterSpacing {
            multiplier: character_spacing.0,
        },
        Op::WriteText {
            items: vec![TextItem::Text(sanitized_line)],
            font: font_id.clone(),
        },
        Op::EndTextSection,
        Op::RestoreGraphicsState,
    ]
}

#[derive(Debug, Clone)]
pub struct DrawRectangle {
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,
    pub rotate: Option<f32>,
    pub page_height: Mm,
    pub color: Option<Color>,
    pub border_width: Option<Mm>,
    pub border_color: Option<Color>,
}

pub fn draw_rectangle(props: DrawRectangle) -> Vec<Op> {
    let matrix_values = calculate_transform_matrix_with_center_pivot(
        props.x,
        props.page_height - props.y - props.height,
        props.width,
        props.height,
        props.rotate,
    );

    let matrixe: Op = Op::SetTransformationMatrix {
        matrix: CurTransMat::Raw(matrix_values),
    };

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

    let x1: Pt = Pt(0.0);
    let y1: Pt = Pt(0.0);
    let x2: Pt = props.width.into();
    let y2: Pt = props.height.into();

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
        Op::SaveGraphicsState,
        matrixe,
        Op::SetOutlineColor { col: border_color },
        Op::SetFillColor { col: color },
        Op::SetOutlineThickness {
            pt: border_width.into(),
        },
        Op::DrawPolygon {
            polygon: polygon.clone(),
        },
        Op::RestoreGraphicsState,
    ]
}
