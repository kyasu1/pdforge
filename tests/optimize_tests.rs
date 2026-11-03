use lopdf::{Document, Object};
use pdforge::PDForgeBuilder;
use std::collections::HashMap;
use std::path::PathBuf;

fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("fonts")
        .join("NotoSansJP-Regular.ttf")
}

fn write_template(name: &str) -> PathBuf {
    let json = serde_json::json!({
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
            "height": 200.0,
            "content": "Optimize me. 圧縮テスト。".repeat(20),
            "fontName": "TestFont",
            "fontSize": 12.0,
            "alignment": "left",
            "verticalAlignment": "top",
            "characterSpacing": 0.0,
            "lineHeight": 1.0,
            "fontColor": "#000000"
        }]]
    })
    .to_string();
    let path = std::env::temp_dir().join(format!(
        "pdforge-optimize-{}-{}.json",
        name,
        std::process::id()
    ));
    std::fs::write(&path, json).expect("template should be writable");
    path
}

fn render(name: &str, optimize: Option<bool>) -> Vec<u8> {
    let template = write_template(name);
    let mut builder = PDForgeBuilder::new("optimize".to_string())
        .add_font_from_file("TestFont", font_path().to_str().unwrap())
        .expect("test font should load")
        .load_template("main", template.to_str().unwrap())
        .expect("template should load");
    if let Some(optimize) = optimize {
        builder = builder.optimize(optimize);
    }
    let bytes = builder
        .build()
        .render("main", vec![vec![HashMap::new()]], None, None)
        .expect("render should succeed");
    let _ = std::fs::remove_file(template);
    bytes
}

/// Whether every content stream of the first page carries a `/Filter`.
fn first_page_content_is_compressed(doc: &Document) -> bool {
    let page_id = *doc.get_pages().values().next().expect("one page");
    let contents = doc.get_page_contents(page_id);
    assert!(!contents.is_empty(), "page should have content streams");
    contents.iter().all(|id| match doc.get_object(*id) {
        Ok(Object::Stream(stream)) => stream.dict.has(b"Filter"),
        _ => false,
    })
}

/// The first page's decompressed content, with font resource names masked.
///
/// Every `PDForgeBuilder` registers fonts under a freshly generated random
/// `FontId` (e.g. `/IFGJFJGDADJHICFHDGBDEIHHDCIDCCFG`), so two renders never
/// share resource names; mask them so only real content differences remain.
fn first_page_decoded_content(doc: &Document) -> String {
    let page_id = *doc.get_pages().values().next().expect("one page");
    let content = String::from_utf8_lossy(&doc.get_page_content(page_id)).into_owned();
    content
        .split_whitespace()
        .map(|token| {
            let is_font_id = token.len() > 17
                && token.starts_with('/')
                && token[1..].bytes().all(|b| b.is_ascii_uppercase());
            if is_font_id { "/FONT" } else { token }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn default_output_is_not_compressed() {
    let doc = Document::load_mem(&render("default", None)).expect("PDF should parse");
    assert!(!first_page_content_is_compressed(&doc));
}

#[test]
fn optimize_false_matches_default() {
    let doc = Document::load_mem(&render("false", Some(false))).expect("PDF should parse");
    assert!(!first_page_content_is_compressed(&doc));
}

#[test]
fn optimize_true_compresses_content_without_changing_it() {
    let plain = Document::load_mem(&render("plain", None)).expect("PDF should parse");
    let optimized =
        Document::load_mem(&render("optimized", Some(true))).expect("optimized PDF should parse");

    assert!(first_page_content_is_compressed(&optimized));
    assert_eq!(optimized.get_pages().len(), plain.get_pages().len());
    assert_eq!(
        first_page_decoded_content(&optimized),
        first_page_decoded_content(&plain),
        "decompressed page content should be identical"
    );
}

#[test]
fn optimize_true_produces_smaller_output() {
    let plain = render("size-plain", None);
    let optimized = render("size-optimized", Some(true));
    assert!(
        optimized.len() < plain.len(),
        "optimized ({} bytes) should be smaller than plain ({} bytes)",
        optimized.len(),
        plain.len()
    );
}
