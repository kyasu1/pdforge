use pdforge::PDForgeBuilder;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdforge = PDForgeBuilder::new("QR Code Rotation Test".to_string())
        .add_font_from_file("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("qr_rotate", "./templates/qrcode-rotate-test.json")?
        .build();

    let bytes = pdforge.render("qr_rotate", vec![vec![HashMap::new()]], None, None)?;

    std::fs::create_dir_all("examples/pdf").ok();
    std::fs::write("examples/pdf/qrcode-rotate-test.pdf", bytes)?;

    println!("PDF generated: examples/pdf/qrcode-rotate-test.pdf");
    Ok(())
}
