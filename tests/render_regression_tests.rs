use lopdf::Document;
use pdforge::PDForgeBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("fonts")
        .join("NotoSansJP-Regular.ttf")
}

fn write_template(name: &str, json: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pdforge-{}-{}.json", name, std::process::id()));
    std::fs::write(&path, json).expect("template should be writable");
    path
}

fn page_count(pdf_bytes: &[u8]) -> usize {
    let doc = Document::load_mem(pdf_bytes).expect("rendered PDF should parse");
    doc.get_pages().len()
}

fn single_text_template(content: &str) -> String {
    serde_json::json!({
        "schemaVersion": "1.0",
        "basePdf": {
            "width": 210.0,
            "height": 297.0,
            "padding": [0.0, 0.0, 0.0, 0.0],
            "staticSchema": []
        },
        "schemas": [[{
            "type": "text",
            "name": "message",
            "position": { "x": 10.0, "y": 10.0 },
            "width": 180.0,
            "height": 20.0,
            "content": content,
            "fontName": "TestFont",
            "fontSize": 12.0,
            "alignment": "left",
            "verticalAlignment": "top",
            "characterSpacing": 0.0,
            "lineHeight": 1.0,
            "fontColor": "#000000"
        }]]
    })
    .to_string()
}

fn builder_for_template(template_path: &Path) -> pdforge::PDForge {
    PDForgeBuilder::new("regression".to_string())
        .add_font_from_file("TestFont", font_path().to_str().unwrap())
        .expect("test font should load")
        .load_template("main", template_path.to_str().unwrap())
        .expect("template should load")
        .build()
}

#[test]
fn repeated_render_calls_do_not_accumulate_pages() {
    let template_path = write_template("repeated-render", &single_text_template("{{ name }}"));
    let forge = builder_for_template(&template_path);

    let mut first_input = HashMap::new();
    first_input.insert("name", "first".to_string());

    let mut second_input = HashMap::new();
    second_input.insert("name", "second".to_string());

    let first = forge
        .render("main", vec![vec![first_input]], None, None)
        .expect("first render should succeed");
    let second = forge
        .render("main", vec![vec![second_input]], None, None)
        .expect("second render should succeed");

    assert_eq!(page_count(&first), 1);
    assert_eq!(page_count(&second), 1);
}

#[test]
fn template_inputs_with_quotes_backslashes_and_newlines_render_safely() {
    let template_path = write_template("escaped-input", &single_text_template("{{ value }}"));
    let forge = builder_for_template(&template_path);

    let mut input = HashMap::new();
    input.insert(
        "value",
        "quote: \" backslash: \\ newline:\n end".to_string(),
    );

    let pdf = forge
        .render("main", vec![vec![input]], None, None)
        .expect("escaped input should not break JSON parsing");

    assert_eq!(page_count(&pdf), 1);
}
