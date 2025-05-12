use base64::prelude::*;
use pdfium_render::prelude::*;
use printpdf::{ParsedFont, PdfParseErrorSeverity};
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

    let mut contexts = vec![];

    let mut context = tera::Context::new();
    context.insert("name", "小松原 康行");
    context.insert("pawnDate", "2025-05-10");
    context.insert("expiryDate", "2025-08-10");
    context.insert("amount", "50,000");
    context.insert("interest", "5%(2,500円)");
    context.insert("count", "1");
    context.insert("item", "K18 ネックレス 5.2g");
    context.insert("message1", "夏季休業 Summer Holiday");
    context.insert("message2", "2025-07-30 ~ 2025-08-06");
    context.insert("pawnSequence", "31-0001");
    context.insert(
        "qrCode",
        &BASE64_STANDARD.encode(r##"{"c": "31-0001", "p": "31-0001"}"##),
    );
    contexts.push(context);

    let mut context = tera::Context::new();
    context.insert("name", "佐々木 ローズマリー");
    context.insert("pawnDate", "2025-03-10");
    context.insert("expiryDate", "");
    context.insert("amount", "350,000");
    context.insert("interest", "4%(14,000円)");
    context.insert("count", "2");
    context.insert("item", "K18 ネックレス・ブレスレット 箱付き");
    context.insert("message1", "夏季休業 Summer Holiday");
    context.insert("message2", "2025-07-30 ~ 2025-08-06");
    context.insert("pawnSequence", "25-0001");
    context.insert(
        "qrCode",
        &BASE64_STANDARD.encode(r##"{"c": "25-0001", "p": "25-0001"}"##),
    );
    contexts.push(context);

    match rust_pdfme::schemas::Template::from_str(
        &font_map,
        "./templates/pawn-ticket.json",
        contexts,
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
