use std::{collections::HashMap, hash::Hash};

use printpdf::*;
use rust_pdfme::font::*;

fn main() {
    let mut pdfme = rust_pdfme::PDFMe::new("TEST".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf");

    pdfme.load_template("table-test", "./templates/table-test.json");

    let bytes = pdfme.render("table-text", Vec::new());

    let mut doc = PdfDocument::new("TEST");

    let mut font_map = FontMap::default();
    let mut warnings = Vec::new();

    let font_slice = include_bytes!(".././assets/fonts/NotoSansJP-Regular.ttf");
    let parsed_font = ParsedFont::from_bytes(font_slice, 0, &mut warnings).unwrap();
    let font_id = doc.add_font(&parsed_font);
    font_map.add_font(String::from("NotoSerifJP"), font_id.clone(), &parsed_font);

    let font_slice = include_bytes!(".././assets/fonts/NotoSansJP-Regular.ttf");
    let parsed_font = ParsedFont::from_bytes(font_slice, 0, &mut warnings).unwrap();
    let font_id = doc.add_font(&parsed_font);
    font_map.add_font(String::from("NotoSansJP"), font_id.clone(), &parsed_font);

    match rust_pdfme::schemas::Template::read_from_file(
        &font_map,
        "./templates/table-test.json",
        vec![HashMap::new()],
    ) {
        Ok(template) => {
            let bytes = template.render(&mut doc).unwrap();

            std::fs::write("./simple.pdf", bytes).unwrap();
        }
        Err(err) => {
            println!("{}", err);
        }
    }
}
