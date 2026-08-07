use pdforge::schemas::line::JsonLineSchema;
use pdforge::schemas::rect::JsonRectSchema;
use pdforge::schemas::{Schema, SchemaTrait};
use pdforge::utils::OpBuffer;
use printpdf::{Mm, Op, PdfDocument};

fn render_rect_into(opacity: Option<f32>, doc: &mut PdfDocument, buffer: &mut OpBuffer) {
    let opacity = opacity.map_or(String::new(), |v| format!(", \"opacity\": {v}"));
    // NOTE: uses r##"..."## (double-hash) because the JSON's "#FFFFFF"/"#000000"
    // would terminate a single-hash r#"..."# raw string at the "# sequence.
    let json = format!(
        r##"{{
            "name": "rect",
            "position": {{"x": 10.0, "y": 20.0}},
            "width": 50.0,
            "height": 30.0,
            "color": "#FFFFFF",
            "borderColor": "#000000"{opacity}
        }}"##
    );
    let schema: Schema = serde_json::from_str::<JsonRectSchema>(&json)
        .unwrap()
        .try_into()
        .unwrap();
    schema
        .render(Mm(150.0), Mm(210.0), doc, 0, buffer)
        .unwrap();
}

fn render_rect(opacity: Option<f32>) -> (PdfDocument, OpBuffer) {
    let mut doc = PdfDocument::new("opacity_test");
    let mut buffer = OpBuffer::default();
    render_rect_into(opacity, &mut doc, &mut buffer);
    (doc, buffer)
}

#[test]
fn rect_opacity_emits_load_graphics_state_and_registers_alpha() {
    let (doc, buffer) = render_rect(Some(0.5));

    assert_eq!(
        buffer.buffer[0]
            .iter()
            .filter(|op| matches!(op, Op::LoadGraphicsState { .. }))
            .count(),
        1
    );
    assert_eq!(doc.resources.extgstates.map.len(), 1);
    for gs in doc.resources.extgstates.map.values() {
        assert!((gs.current_fill_alpha() - 0.5).abs() < 1e-6);
        assert!((gs.current_stroke_alpha() - 0.5).abs() < 1e-6);
    }
}

#[test]
fn rect_without_opacity_registers_nothing() {
    let (doc, buffer) = render_rect(None);

    assert!(
        buffer.buffer[0]
            .iter()
            .all(|op| !matches!(op, Op::LoadGraphicsState { .. }))
    );
    assert_eq!(doc.resources.extgstates.map.len(), 0);
}

#[test]
fn rect_opacity_scope_restores_before_next_element() {
    // Same buffer: an opacity element followed by a plain element. The first
    // element's wrapper must close with its outer RestoreGraphicsState as the
    // very last op of that render, immediately before the second element draws,
    // so the alpha does not leak into the opaque element.
    let mut doc = PdfDocument::new("opacity_test");
    let mut buffer = OpBuffer::default();
    render_rect_into(Some(0.5), &mut doc, &mut buffer);
    let first_element_len = buffer.buffer[0].len();
    render_rect_into(None, &mut doc, &mut buffer);

    let ops = &buffer.buffer[0];
    // The opacity element's wrapper ends with its outer RestoreGraphicsState...
    assert!(matches!(
        ops[first_element_len - 1],
        Op::RestoreGraphicsState
    ));
    // ...and the opaque second element starts immediately after, drawing no
    // ExtGState of its own (its first op is the inner draw's SaveGraphicsState).
    assert!(matches!(ops[first_element_len], Op::SaveGraphicsState));
    assert!(ops[first_element_len..]
        .iter()
        .all(|op| !matches!(op, Op::LoadGraphicsState { .. })));
}

fn render_line_into(opacity: Option<f32>, doc: &mut PdfDocument, buffer: &mut OpBuffer) {
    let opacity = opacity.map_or(String::new(), |v| format!(", \"opacity\": {v}"));
    // NOTE: r##"..."## (double-hash) because "#000000" would terminate r#"..."#.
    let json = format!(
        r##"{{
            "name": "line",
            "position": {{"x": 10.0, "y": 20.0}},
            "width": 50.0,
            "color": "#000000"{opacity}
        }}"##
    );
    let schema: Schema = serde_json::from_str::<JsonLineSchema>(&json)
        .unwrap()
        .try_into()
        .unwrap();
    schema
        .render(Mm(150.0), Mm(210.0), doc, 0, buffer)
        .unwrap();
}

fn render_line(opacity: Option<f32>) -> (PdfDocument, OpBuffer) {
    let mut doc = PdfDocument::new("opacity_test");
    let mut buffer = OpBuffer::default();
    render_line_into(opacity, &mut doc, &mut buffer);
    (doc, buffer)
}

#[test]
fn line_opacity_emits_load_graphics_state_and_registers_alpha() {
    let (doc, buffer) = render_line(Some(0.5));

    assert_eq!(
        buffer.buffer[0]
            .iter()
            .filter(|op| matches!(op, Op::LoadGraphicsState { .. }))
            .count(),
        1
    );
    assert_eq!(doc.resources.extgstates.map.len(), 1);
    for gs in doc.resources.extgstates.map.values() {
        assert!((gs.current_fill_alpha() - 0.5).abs() < 1e-6);
        assert!((gs.current_stroke_alpha() - 0.5).abs() < 1e-6);
    }
}

#[test]
fn line_without_opacity_registers_nothing() {
    let (doc, buffer) = render_line(None);

    assert!(
        buffer.buffer[0]
            .iter()
            .all(|op| !matches!(op, Op::LoadGraphicsState { .. }))
    );
    assert_eq!(doc.resources.extgstates.map.len(), 0);
}
