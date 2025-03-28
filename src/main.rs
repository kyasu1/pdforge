use printpdf::*;
use rust_pdfme::font::*;

fn main() {
    let mut doc = PdfDocument::new("TEST");

    let mut font_map = FontMap::default();
    let mut warnings = Vec::new();

    let font_slice = include_bytes!(".././assets/fonts/NotoSerifJP-Regular.ttf");
    let parsed_font = ParsedFont::from_bytes(font_slice, 0, &mut warnings).unwrap();
    let font_id = doc.add_font(&parsed_font);
    font_map.add_font(String::from("NotoSerif"), font_id.clone(), &parsed_font);

    let font_slice = include_bytes!(".././assets/fonts/NotoSansJP-Regular.ttf");
    let parsed_font = ParsedFont::from_bytes(font_slice, 0, &mut warnings).unwrap();
    let font_id = doc.add_font(&parsed_font);
    font_map.add_font(String::from("NotoSans"), font_id.clone(), &parsed_font);

    match rust_pdfme::schemas::Template::read_from_file("./templates/table-test.json", &font_map) {
        Ok(template) => {
            let bytes = template.render(&mut doc).unwrap();

            std::fs::write("./simple.pdf", bytes).unwrap();
        }
        Err(err) => {
            println!("{}", err);
        }
    }
}
