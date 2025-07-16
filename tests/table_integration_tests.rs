use pdforge::font::FontMap;
use pdforge::schemas::table::{JsonTableSchema, Table};
use std::collections::HashMap;

fn create_test_font_map() -> FontMap {
    // Create an empty FontMap for testing purposes
    FontMap::default()
}

fn create_simple_table_json() -> serde_json::Value {
    serde_json::json!({
        "name": "simple_table",
        "position": { "x": 10.0, "y": 250.0 },
        "width": 190.0,
        "height": 100.0,
        "content": "[[\"Product\",\"Price\"],[\"Item A\",\"$100\"],[\"Item B\",\"$200\"]]",
        "showHead": true,
        "headWidthPercentages": [
            {
                "content": "Product",
                "percent": 70.0,
                "fontSize": 12.0,
                "alignment": "left"
            },
            {
                "content": "Price",
                "percent": 30.0,
                "fontSize": 12.0,
                "alignment": "right"
            }
        ],
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
        "columns": [],
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

    let _base = table.get_base();
    // Note: BaseSchema fields are private, so we can only test basic construction success
    assert!(true); // Table was created successfully
}

#[test]
fn test_table_column_width_percentages_validation() {
    let mut json_value = create_simple_table_json();

    // Set invalid percentages that don't add up to 100%
    json_value["headWidthPercentages"] = serde_json::json!([
        {
            "content": "Product",
            "percent": 40.0,
            "fontSize": 12.0,
            "alignment": "left"
        },
        {
            "content": "Price",
            "percent": 40.0, // Total = 80%, not 100%
            "fontSize": 12.0,
            "alignment": "right"
        }
    ]);

    let json_schema: JsonTableSchema = serde_json::from_value(json_value).unwrap();
    let font_map = create_test_font_map();
    let result = Table::from_json(json_schema, &font_map);

    assert!(result.is_err());
    let error_message = format!("{}", result.unwrap_err());
    assert!(error_message.contains("total of column width must be 100%"));
}

#[test]
fn test_table_valid_column_width_percentages() {
    let json_value = create_simple_table_json();
    let json_schema: JsonTableSchema = serde_json::from_value(json_value).unwrap();
    let font_map = create_test_font_map();

    let result = Table::from_json(json_schema, &font_map);
    assert!(result.is_ok());
}
