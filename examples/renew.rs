use std::collections::HashMap;

use base64::prelude::*;
use pdfium_render::prelude::*;

fn main() -> Result<(), PdfiumError> {
    let mut pdforge = pdforge::PDForgeBuilder::new("TEST".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")
        .load_template("renew", "./templates/renew.json")
        .build();

    let mut inputs = vec![];

    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("pawnSequence", "31-0001".to_string());
    input.insert("date", "2025-05-10".to_string());
    input.insert("renewDate", "2025-06-10".to_string());
    input.insert("expiryDate", "2025-09-10".to_string());

    inputs.push(input);

    let bytes: Vec<u8> = pdforge.render_with_inputs("renew", vec![inputs]);

    std::fs::write("./renew.pdf", bytes.clone()).unwrap();

    let pdfium: Pdfium = Pdfium::default();

    let document = pdfium.load_pdf_from_byte_slice(&bytes, None)?;
    let render_config = PdfRenderConfig::new().set_target_size(720, 1040);

    for (index, page) in document.pages().iter().enumerate() {
        page.render_with_config(&render_config)?
            .as_image()
            .into_luma8()
            .save_with_format(format!("renew-page-{}.png", index), image::ImageFormat::Png)
            .map_err(|_| PdfiumError::ImageError)?;
    }

    Ok(())
}
