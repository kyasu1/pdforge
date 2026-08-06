//! Tests for QR Code `rotate` support
//!
//! Verifies that `BaseSchema::get_matrix()` retains its original public
//! signature, that `get_matrix_with_rotation()` accepts an optional
//! `XObjectRotation`, and that `QrCode::render()` produces a `UseXobject`
//! transform with the rotation centered on the QR image's intrinsic size.

use pdforge::schemas::base::BaseSchema;
use pdforge::schemas::qrcode::{JsonQrCodeSchema, QrCode};
use pdforge::schemas::{Schema, SchemaTrait};
use pdforge::utils::OpBuffer;
use printpdf::{Mm, Op, PdfDocument, Px, XObjectRotation, XObjectTransform};

fn qr_dimensions(content: &str) -> (u32, u32) {
    let code = qrcode::QrCode::new(content).unwrap();
    let luma = code.render::<image::Luma<u8>>().build();
    (luma.width(), luma.height())
}

fn rendered_transform(json: &str) -> XObjectTransform {
    let json_schema: JsonQrCodeSchema = serde_json::from_str(json).unwrap();
    let schema: Schema = json_schema.into();
    let mut doc = PdfDocument::new("rotation_test");
    let mut buffer = OpBuffer::default();
    schema
        .render(Mm(150.0), Mm(210.0), &mut doc, 0, &mut buffer)
        .unwrap();

    let ops = &buffer.buffer[0];
    for op in ops {
        if let Op::UseXobject { transform, .. } = op {
            return transform.clone();
        }
    }
    panic!("no UseXobject op found in buffer");
}

#[test]
fn test_get_matrix_keeps_original_signature_with_no_rotation() {
    let base = BaseSchema::new("qr".to_string(), Mm(10.0), Mm(20.0), Mm(50.0), Mm(50.0));

    let transform = base.get_matrix(Mm(150.0), Some(Px(256)));

    assert!(transform.rotate.is_none());
    assert_eq!(transform.translate_x, Some(Mm(10.0).into()));
}

#[test]
fn test_get_matrix_without_original_width_uses_identity_scale() {
    let base = BaseSchema::new("qr".to_string(), Mm(10.0), Mm(20.0), Mm(50.0), Mm(50.0));

    let transform = base.get_matrix(Mm(150.0), None);

    assert_eq!(transform.scale_x, Some(50.0));
    assert_eq!(transform.scale_y, Some(50.0));
    assert!(transform.rotate.is_none());
}

#[test]
fn test_get_matrix_with_rotation_preserves_rotation() {
    let base = BaseSchema::new("qr".to_string(), Mm(10.0), Mm(20.0), Mm(50.0), Mm(50.0));
    let rotation = XObjectRotation {
        angle_ccw_degrees: 90.0,
        rotation_center_x: Px(128),
        rotation_center_y: Px(128),
    };

    let transform = base.get_matrix_with_rotation(Mm(150.0), Some(Px(256)), Some(rotation.clone()));

    assert_eq!(transform.rotate, Some(rotation));
}

#[test]
fn test_get_matrix_with_rotation_none_behaves_like_original() {
    let base = BaseSchema::new("qr".to_string(), Mm(10.0), Mm(20.0), Mm(50.0), Mm(50.0));

    let transform = base.get_matrix_with_rotation(Mm(150.0), Some(Px(256)), None);

    assert!(transform.rotate.is_none());
    assert_eq!(transform.scale_x, Some(300.0 / 25.4 / 256.0 * 50.0));
}

#[test]
fn test_qrcode_no_rotation_produces_rotate_none() {
    let json = r#"{
        "name": "qr",
        "content": "test_no_rotate",
        "position": { "x": 10.0, "y": 20.0 },
        "width": 50.0,
        "height": 50.0
    }"#;

    let transform = rendered_transform(json);

    assert!(transform.rotate.is_none());
}

#[test]
fn test_qrcode_with_rotation_produces_centered_rotation() {
    let content = "test_rotate_45";
    let json = format!(
        r#"{{
            "name": "qr",
            "content": "{}",
            "position": {{ "x": 10.0, "y": 20.0 }},
            "width": 50.0,
            "height": 50.0,
            "rotate": 45.0
        }}"#,
        content
    );

    let transform = rendered_transform(&json);

    let rotation = transform.rotate.expect("rotation must be set");
    let (w, h) = qr_dimensions(content);
    assert_eq!(rotation.angle_ccw_degrees, 45.0);
    assert_eq!(rotation.rotation_center_x, Px((w / 2) as usize));
    assert_eq!(rotation.rotation_center_y, Px((h / 2) as usize));
}

#[test]
fn test_qrcode_with_rotation_uses_intrinsic_width_for_scale() {
    let content = "test_scale_90";
    let json = format!(
        r#"{{
            "name": "qr",
            "content": "{}",
            "position": {{ "x": 10.0, "y": 20.0 }},
            "width": 50.0,
            "height": 50.0,
            "rotate": 90.0
        }}"#,
        content
    );

    let transform = rendered_transform(&json);
    let (w, _h) = qr_dimensions(content);

    assert!(transform.rotate.is_some());
    let expected_scale = 300.0 / 25.4 / (w as f32) * 50.0;
    assert_eq!(transform.scale_x, Some(expected_scale));
}

#[test]
fn test_qrcode_renders_with_rotation_and_alignment_padding() {
    let json = r#"{
        "name": "qr",
        "content": "test_align_pad",
        "position": { "x": 10.0, "y": 20.0 },
        "width": 100.0,
        "height": 100.0,
        "rotate": 90.0,
        "alignment": "center",
        "verticalAlignment": "middle",
        "padding": { "top": 10.0, "right": 10.0, "bottom": 10.0, "left": 10.0 }
    }"#;

    let transform = rendered_transform(json);

    // effective box = 100 - (10+10) = 80; center/middle offsets = (80-100)/2 = -10
    // x = 10 + 10 + (-10) = 10, y = 20 + 10 + (-10) = 20
    let rotation = transform.rotate.expect("rotation must be set");
    assert_eq!(rotation.angle_ccw_degrees, 90.0);
    assert_eq!(transform.translate_x, Some(Mm(10.0).into()));
    assert_eq!(transform.translate_y, Some(Mm(210.0 - 20.0 - 100.0).into()));
}

#[test]
fn test_qrcode_rotate_direction_matches_counter_clockwise_convention() {
    let mut qr = QrCode::new(
        "qr".to_string(),
        Mm(10.0),
        Mm(20.0),
        Mm(50.0),
        Mm(50.0),
        "test_ccw".to_string(),
    );
    qr.set_rotate(90.0);

    let mut doc = PdfDocument::new("rotation_test");
    let mut buffer = OpBuffer::default();
    qr.render(Mm(150.0), &mut doc, 0, &mut buffer).unwrap();

    let ops = &buffer.buffer[0];
    let rotation = ops
        .iter()
        .find_map(|op| match op {
            Op::UseXobject { transform, .. } => transform.rotate.clone(),
            _ => None,
        })
        .expect("rotation must be set");

    assert_eq!(rotation.angle_ccw_degrees, 90.0);
}
