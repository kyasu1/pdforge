use pdforge::PDForgeBuilder;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdforge = PDForgeBuilder::new("QR Code 90 Degree Rotation".to_string())
        .add_font_from_file("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template(
            "qr_rotate_same_size",
            "./templates/qrcode-rotate-same-size.json",
        )?
        .build();

    let inputs = vec![vec![HashMap::new()], vec![HashMap::new()]];
    let bytes = pdforge.render("qr_rotate_same_size", inputs, None, None)?;

    std::fs::create_dir_all("examples/pdf").ok();
    std::fs::write("examples/pdf/qrcode-rotate-same-size.pdf", bytes)?;

    println!("PDF generated: examples/pdf/qrcode-rotate-same-size.pdf");
    println!("Pages 1 and 2 use the same QR content, position, and 80 mm square size.");
    println!("Only page 2 applies a 90 degree rotation around the QR center.");
    Ok(())
}
