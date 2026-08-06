use pdforge::schemas::image::JsonImageSchema;
use pdforge::schemas::svg::JsonSvgSchema;
use pdforge::schemas::{Schema, SchemaTrait};
use pdforge::utils::OpBuffer;
use printpdf::{CurTransMat, Mm, Op, PdfDocument, Px, XObjectTransform};

const IMAGE_DATA_URL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
    <rect width="200" height="100" fill="black"/>
</svg>"#;

fn render_transform(schema: Schema) -> XObjectTransform {
    let mut doc = PdfDocument::new("rotation_test");
    let mut buffer = OpBuffer::default();
    schema
        .render(Mm(150.0), Mm(210.0), &mut doc, 0, &mut buffer)
        .unwrap();

    buffer.buffer[0]
        .iter()
        .find_map(|op| match op {
            Op::UseXobject { transform, .. } => Some(*transform),
            _ => None,
        })
        .expect("UseXobject must be emitted")
}

fn bounds(transform: &XObjectTransform, width: usize, height: usize) -> [f32; 4] {
    let matrix = transform
        .get_ctms(Some((Px(width), Px(height))))
        .into_iter()
        .fold(CurTransMat::Identity.as_array(), |combined, next| {
            CurTransMat::combine_matrix(combined, next.as_array())
        });
    let [a, b, c, d, e, f] = matrix;
    [
        (e, f),
        (a + e, b + f),
        (c + e, d + f),
        (a + c + e, b + d + f),
    ]
    .into_iter()
    .fold(
        [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ],
        |[min_x, min_y, max_x, max_y], (x, y)| {
            [min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y)]
        },
    )
}

fn center(bounds: [f32; 4]) -> (f32, f32) {
    ((bounds[0] + bounds[2]) / 2.0, (bounds[1] + bounds[3]) / 2.0)
}

fn assert_same_center(unrotated: [f32; 4], rotated: [f32; 4]) {
    let (unrotated_x, unrotated_y) = center(unrotated);
    let (rotated_x, rotated_y) = center(rotated);
    assert!((unrotated_x - rotated_x).abs() < 0.3);
    assert!((unrotated_y - rotated_y).abs() < 0.3);
}

#[test]
fn image_rotation_keeps_the_transformed_center() {
    let image_json = |rotate: Option<f32>| {
        format!(
            r#"{{
                "name": "image",
                "position": {{"x": 30.0, "y": 25.0}},
                "width": 40.0,
                "height": 20.0,
                "content": "{IMAGE_DATA_URL}",
                "objectFit": "fill"{}
            }}"#,
            rotate.map_or(String::new(), |angle| format!(", \"rotate\": {angle}"))
        )
    };

    let unrotated: Schema = serde_json::from_str::<JsonImageSchema>(&image_json(None))
        .unwrap()
        .try_into()
        .unwrap();
    let rotated: Schema = serde_json::from_str::<JsonImageSchema>(&image_json(Some(90.0)))
        .unwrap()
        .try_into()
        .unwrap();
    let unrotated = render_transform(unrotated);
    let rotated = render_transform(rotated);

    assert_eq!(
        rotated
            .rotate
            .expect("rotation must be set")
            .angle_ccw_degrees,
        90.0
    );
    assert_same_center(bounds(&unrotated, 1, 1), bounds(&rotated, 1, 1));
}

#[test]
fn svg_rotation_keeps_the_transformed_center() {
    let svg_json = |rotate: Option<f32>| {
        format!(
            r#"{{
                "name": "svg",
                "position": {{"x": 30.0, "y": 25.0}},
                "width": 40.0,
                "height": 20.0,
                "content": {}{}
            }}"#,
            serde_json::to_string(SVG).unwrap(),
            rotate.map_or(String::new(), |angle| format!(", \"rotate\": {angle}"))
        )
    };

    let unrotated: Schema = serde_json::from_str::<JsonSvgSchema>(&svg_json(None))
        .unwrap()
        .try_into()
        .unwrap();
    let rotated: Schema = serde_json::from_str::<JsonSvgSchema>(&svg_json(Some(90.0)))
        .unwrap()
        .try_into()
        .unwrap();
    let unrotated = render_transform(unrotated);
    let rotated = render_transform(rotated);

    assert_eq!(
        rotated
            .rotate
            .expect("rotation must be set")
            .angle_ccw_degrees,
        90.0
    );
    assert_same_center(bounds(&unrotated, 200, 100), bounds(&rotated, 200, 100));
}
