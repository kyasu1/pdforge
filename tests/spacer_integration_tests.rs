use pdforge::PDForgeBuilder;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

fn write_template(name: &str, schemas: serde_json::Value) -> PathBuf {
    write_template_with_static(name, schemas, json!([]))
}

fn write_template_with_static(
    name: &str,
    schemas: serde_json::Value,
    static_schemas: serde_json::Value,
) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("pdforge-spacer-{name}-{}.json", std::process::id()));
    let template = json!({
        "schemas": [schemas],
        "basePdf": {
            "width": 210.0,
            "height": 297.0,
            "padding": [10.0, 10.0, 10.0, 10.0],
            "staticSchema": static_schemas
        },
        "schemaVersion": "1.0.0"
    });
    std::fs::write(&path, template.to_string()).expect("template should be writable");
    path
}

fn render_template(path: &PathBuf) -> Result<Vec<u8>, pdforge::schemas::Error> {
    let forge = PDForgeBuilder::new("spacer-test".to_string())
        .load_template("main", path.to_str().unwrap())?
        .build();

    forge.render("main", vec![vec![HashMap::new()]], None, None)
}

#[test]
fn spacer_schema_accepts_height_only() {
    let path = write_template("height-only", json!([{ "type": "spacer", "height": 5.0 }]));

    let result = render_template(&path);

    assert!(
        result.is_ok(),
        "height-only spacer should render: {result:?}"
    );
}

#[test]
fn spacer_schema_rejects_negative_height() {
    let path = write_template(
        "negative-height",
        json!([{ "type": "spacer", "height": -1.0 }]),
    );

    let error = render_template(&path).expect_err("negative spacer height should fail");

    assert!(error.to_string().contains("non-negative"));
}

#[test]
fn spacer_is_rejected_inside_group() {
    let path = write_template(
        "group",
        json!([{
            "type": "group",
            "name": "group",
            "position": { "x": 0.0, "y": 0.0 },
            "width": 100.0,
            "height": 100.0,
            "schemas": [{ "type": "spacer", "height": 5.0 }]
        }]),
    );

    let error = render_template(&path).expect_err("group spacer should fail");

    assert!(error
        .to_string()
        .contains("Group schema does not support this type"));
}

#[test]
fn spacer_is_rejected_inside_static_schema() {
    let path = write_template_with_static(
        "static-schema",
        json!([{
            "type": "rectangle",
            "name": "page",
            "position": { "x": 10.0, "y": 10.0 },
            "width": 10.0,
            "height": 10.0,
            "borderColor": "#000000",
            "color": "#ffffff"
        }]),
        json!([{ "type": "spacer", "height": 5.0 }]),
    );

    let error = render_template(&path).expect_err("static spacer should fail");

    assert!(error
        .to_string()
        .contains("Unsupported schema type in staticSchema: Spacer"));
}
