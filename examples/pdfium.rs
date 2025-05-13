use std::collections::HashMap;

use base64::prelude::*;
use pdfium_render::prelude::*;
use printpdf::ParsedFont;
use rust_pdfme::font::*;

fn main() -> Result<(), PdfiumError> {
    let mut doc = printpdf::PdfDocument::new("TEST");

    let mut font_map = FontMap::default();
    let mut warnings = Vec::new();

    let font_slice = include_bytes!(".././assets/fonts/NotoSerifJP-Regular.ttf");
    let parsed_font = ParsedFont::from_bytes(font_slice, 0, &mut warnings).unwrap();
    let font_id = doc.add_font(&parsed_font);
    font_map.add_font(String::from("NotoSerifJP"), font_id.clone(), &parsed_font);

    let font_slice = include_bytes!(".././assets/fonts/NotoSansJP-Regular.ttf");
    let parsed_font = ParsedFont::from_bytes(font_slice, 0, &mut warnings).unwrap();
    let font_id = doc.add_font(&parsed_font);
    font_map.add_font(String::from("NotoSansJP"), font_id.clone(), &parsed_font);

    let mut inputs = vec![];

    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("name", "小松原 康行".to_string());
    input.insert("pawnDate", "2025-05-10".to_string());
    input.insert("expiryDate", "2025-08-10".to_string());
    input.insert("amount", "50,000".to_string());
    input.insert("interest", "5%(2,500円)".to_string());
    input.insert("count", "1".to_string());
    input.insert("item", "K18 ネックレス 5.2g".to_string());
    input.insert("message1", "夏季休業 Summer Holiday".to_string());
    input.insert("message2", "2025-07-30 ~ 2025-08-06".to_string());
    input.insert("pawnSequence", "31-0001".to_string());
    input.insert(
        "qrCode",
        BASE64_STANDARD.encode(r##"{"c": "31-0001", "p": "31-0001"}"##),
    );
    inputs.push(input);

    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("name", "佐々木 ローズマリー".to_string());
    input.insert("pawnDate", "2025-03-10".to_string());
    input.insert("expiryDate", "".to_string());
    input.insert("amount", "350,000".to_string());
    input.insert("interest", "4%(14,000円)".to_string());
    input.insert("count", "2".to_string());
    input.insert("item", "K18 ネックレス・ブレスレット 箱付き".to_string());
    input.insert("message1", "夏季休業 Summer Holiday".to_string());
    input.insert("message2", "2025-07-30 ~ 2025-08-06".to_string());
    input.insert("pawnSequence", "25-0001".to_string());
    input.insert(
        "qrCode",
        BASE64_STANDARD.encode(r##"{"c": "25-0001", "p": "25-0001"}"##),
    );
    inputs.push(input);

    match rust_pdfme::schemas::Template::read_from_file(
        &font_map,
        "./templates/pawn-ticket.json",
        inputs,
    ) {
        Ok(template) => {
            let bytes = template.render(&mut doc).unwrap();

            std::fs::write("./pdfium.pdf", bytes.clone()).unwrap();

            let pdfium: Pdfium = Pdfium::default();

            let document = pdfium.load_pdf_from_byte_slice(&bytes, None)?;
            let render_config = PdfRenderConfig::new().set_target_size(720, 1040);

            for (index, page) in document.pages().iter().enumerate() {
                page.render_with_config(&render_config)?
                    .as_image()
                    .into_luma8()
                    .save_with_format(
                        format!("pdfium-page-{}.png", index),
                        image::ImageFormat::Png,
                    )
                    .map_err(|_| PdfiumError::ImageError)?;
            }

            // for warning in warnings {
            //     if warning.severity != PdfParseErrorSeverity::Info {
            //         println!("{:#?}", warning);
            //     }
            // }
        }
        Err(err) => {
            println!("{}", err);
        }
    }

    Ok(())
}
