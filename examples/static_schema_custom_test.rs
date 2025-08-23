use pdforge::PDForgeBuilder;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create PDForge instance with fonts
    let mut pdforge = PDForgeBuilder::new("static-schema-custom-test".to_string())
        .add_font("NotoSansJP-Regular", "./assets/fonts/NotoSansJP-Regular.otf")?
        .add_font("NotoSansJP-Bold", "./assets/fonts/NotoSansJP-Bold.otf")?
        .load_template("static-schema-custom-test", "./templates/static-schema-custom-test.json")?
        .build();

    // Create custom static inputs for static schema
    let mut static_inputs: HashMap<String, String> = HashMap::new();
    static_inputs.insert("company_name".to_string(), "PDForge Corp.".to_string());
    static_inputs.insert("document_type".to_string(), "Test Report".to_string());
    static_inputs.insert("author_name".to_string(), "John Doe".to_string());

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