use pdforge::{PDForge, PDForgeBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple PDForge instance
    let mut forge = PDForgeBuilder::new("Test Document".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("test", "./templates/test_metadata.json")?
        .build();

    // Generate PDF
    let pdf_bytes = forge.render("test")?;
    
    // Set metadata with Japanese title and author
    let pdf_with_metadata = PDForge::set_pdf_metadata(
        pdf_bytes,
        Some("日本語タイトルテスト - Japanese Title Test"),
        Some("田中太郎 - Taro Tanaka")
    )?;

    // Save to file
    std::fs::write("examples/pdf/test_metadata.pdf", pdf_with_metadata)?;
    println!("Generated PDF with Japanese metadata: examples/pdf/test_metadata.pdf");

    Ok(())
}