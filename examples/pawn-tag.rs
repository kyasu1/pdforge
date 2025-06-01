use std::collections::HashMap;

use base64::prelude::*;
use pdfium_render::prelude::*;

fn main() -> Result<(), PdfiumError> {
    let mut pdforge = pdforge::PDForgeBuilder::new("TEST".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")
        .load_template("pawn-tag", "./templates/pawn-tag.json")
        .build();

    let mut inputs = vec![];

    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("name", "小松原 康行".to_string());
    input.insert("pawnDate", "2025/05/10".to_string());
    input.insert("amount", "50,000".to_string());
    input.insert(
        "desc",
        "ロレックス デイトジャスト 時計 16233G ブルーグラデーション 箱付 保付 計1点".to_string(),
    );
    input.insert("pawnSequence", "31-0001".to_string());

    inputs.push(input);

    let bytes: Vec<u8> = pdforge.render_with_inputs("pawn-tag", vec![inputs]);

    std::fs::write("./pawn-tag.pdf", bytes.clone()).unwrap();

    let pdfium: Pdfium = Pdfium::default();

    let document = pdfium.load_pdf_from_byte_slice(&bytes, None)?;
    let render_config = PdfRenderConfig::new().set_target_size(720, 1062 * 2);
    for (index, page) in document.pages().iter().enumerate() {
        page.render_with_config(&render_config)?
            .as_image()
            .into_luma8()
            .save_with_format(
                format!("pawn-tag-page-{}.png", index),
                image::ImageFormat::Png,
            )
            .map_err(|_| PdfiumError::ImageError)?;
    }

    Ok(())
}
