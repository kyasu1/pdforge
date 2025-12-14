use pdforge::PDForgeBuilder;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create PDForge instance with fonts
    let mut pdforge = PDForgeBuilder::new("static-schema-custom-test".to_string())
        .add_font_from_file(
            "NotoSansJP-Regular",
            "./assets/fonts/NotoSansJP-Regular.otf",
        )?
        .add_font_from_file("NotoSansJP-Bold", "./assets/fonts/NotoSansJP-Bold.otf")?
        .load_template(
            "static-schema-custom-test",
            "./templates/static-schema-custom-test.json",
        )?
        .build();

    // Create custom static inputs for static schema
    let mut static_inputs: HashMap<&'static str, String> = HashMap::new();
    static_inputs.insert("company_name", "PDForge Corp.".to_string());
    static_inputs.insert("document_type", "Test Report".to_string());
    static_inputs.insert("author_name", "John Doe".to_string());

    // Create regular page inputs (empty for this test)
    let inputs = vec![vec![HashMap::new()]];

    // Render PDF with static schema custom inputs
    let pdf_bytes = pdforge.render(
        "static-schema-custom-test",
        inputs,
        None,
        Some(static_inputs),
    )?;

    // Save PDF
    std::fs::write("examples/pdf/static-schema-custom-test.pdf", pdf_bytes)?;
    println!("PDF generated successfully with custom static schema variables!");

    Ok(())
}
