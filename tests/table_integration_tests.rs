use pdforge::font::FontMap;
use pdforge::schemas::table::{JsonTableSchema, Table};
use printpdf::{ParsedFont, PdfDocument};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn create_test_font_map() -> FontMap {
    static FONT: OnceLock<Arc<ParsedFont>> = OnceLock::new();

    let parsed_font = FONT
        .get_or_init(|| {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("fonts")
                .join("NotoSansJP-Regular.ttf");
            let font_bytes = std::fs::read(path).expect("test font should be readable");
            let parsed_font = ParsedFont::from_bytes(&font_bytes, 0, &mut Vec::new())
                .expect("test font should parse");
            Arc::new(parsed_font)
        })
        .clone();

    let mut doc = PdfDocument::new("test");
    let font_id = doc.add_font(parsed_font.as_ref());
    let mut font_map = FontMap::default();
    font_map.add_font("TestFont".to_string(), font_id, parsed_font.as_ref());
    font_map
}

fn create_simple_table_json() -> serde_json::Value {
    serde_json::json!({
        "name": "simple_table",
        "position": { "x": 10.0, "y": 250.0 },
        "width": 190.0,
        "height": 100.0,
        "content": "[[\"Product\",\"Price\"],[\"Item A\",\"$100\"],[\"Item B\",\"$200\"]]",
        "showHead": true,
        "tableStyles": {
            "borderWidth": 0.3,
            "borderColor": "#000000"
        },
        "headStyles": {
            "fontSize": 12.0,
            "fontName": "TestFont",
            "characterSpacing": 0.0,
            "alignment": "center",
            "verticalAlignment": "middle",
            "lineHeight": 1.0,
            "fontColor": "#ffffff",
            "borderColor": "#000000",
            "backgroundColor": "#2980ba",
            "borderWidth": {
                "top": 0.0,
                "right": 0.0,
                "bottom": 0.0,
                "left": 0.0
            },
            "padding": {
                "top": 5.0,
                "right": 5.0,
                "bottom": 5.0,
                "left": 5.0
            }
        },
        "bodyStyles": {
            "fontSize": 11.0,
            "fontName": "TestFont",
            "characterSpacing": 0.0,
            "alignment": "left",
            "verticalAlignment": "middle",
            "lineHeight": 1.5,
            "fontColor": "#000000",
            "borderColor": "#888888",
            "backgroundColor": "#FFFFFF",
            "alternateBackgroundColor": "#f8f8f8",
            "borderWidth": {
                "top": 0.1,
                "right": 0.1,
                "bottom": 0.1,
                "left": 0.1
            },
            "padding": {
                "top": 5.0,
                "right": 5.0,
                "bottom": 5.0,
                "left": 5.0
            }
        },
        "columnStyles": {},
        "columns": [
            {
                "width": "70%",
                "header": {
                    "content": "Product",
                    "fontSize": 12.0,
                    "alignment": "left"
                },
                "cell": {
                    "type": "text",
                    "name": "product",
                    "position": { "x": 0.0, "y": 0.0 },
                    "width": 0.0,
                    "height": 0.0,
                    "content": "",
                    "fontName": "TestFont",
                    "fontSize": 11.0
                }
            },
            {
                "width": "30%",
                "header": {
                    "content": "Price",
                    "fontSize": 12.0,
                    "alignment": "right"
                },
                "cell": {
                    "type": "text",
                    "name": "price",
                    "position": { "x": 0.0, "y": 0.0 },
                    "width": 0.0,
                    "height": 0.0,
                    "content": "",
                    "fontName": "TestFont",
                    "fontSize": 11.0
                }
            }
        ],
        "fields": [
            ["Product", "Price"],
            ["Item A", "$100"],
            ["Item B", "$200"]
        ],
        "required": false,
        "readOnly": false
    })
}

#[test]
fn test_simple_table_creation_from_json() {
    let json_value = create_simple_table_json();
    let json_schema: JsonTableSchema = serde_json::from_value(json_value).unwrap();
    let font_map = create_test_font_map();

    let table = Table::from_json(json_schema, &font_map).unwrap();

    // BaseSchema fields are private outside the crate, so the observable
    // contract here is that a well-formed table deserializes and converts.
    let _base = table.get_base();
}

#[test]
fn test_table_accepts_mixed_width_notations() {
    let mut json_value = create_simple_table_json();
    json_value["columns"][0]["width"] = serde_json::json!("1fr");
    json_value["columns"][1]["width"] = serde_json::json!(40.0);

    let json_schema: JsonTableSchema = serde_json::from_value(json_value).unwrap();
    let font_map = create_test_font_map();

    assert!(Table::from_json(json_schema, &font_map).is_ok());
}

#[test]
fn test_table_rejects_unparseable_column_width() {
    let mut json_value = create_simple_table_json();
    json_value["columns"][1]["width"] = serde_json::json!("40 pt");

    let json_schema: JsonTableSchema = serde_json::from_value(json_value).unwrap();
    let font_map = create_test_font_map();
    let error = Table::from_json(json_schema, &font_map)
        .unwrap_err()
        .to_string();

    assert!(error.contains("column 1"), "got: {error}");
    assert!(error.contains("40 pt"), "got: {error}");
}

#[test]
fn test_table_rejects_rows_that_do_not_match_the_column_count() {
    let mut json_value = create_simple_table_json();
    json_value["fields"] = serde_json::json!([["Product", "Price"], ["Item A"]]);

    let json_schema: JsonTableSchema = serde_json::from_value(json_value).unwrap();
    let font_map = create_test_font_map();
    let error = Table::from_json(json_schema, &font_map)
        .unwrap_err()
        .to_string();

    assert!(error.contains("row 1"), "got: {error}");
    assert!(error.contains("expected 2"), "got: {error}");
    assert!(error.contains("got 1"), "got: {error}");
}

#[test]
fn test_table_valid_column_widths() {
    let json_value = create_simple_table_json();
    let json_schema: JsonTableSchema = serde_json::from_value(json_value).unwrap();
    let font_map = create_test_font_map();

    let result = Table::from_json(json_schema, &font_map);
    assert!(result.is_ok());
}
