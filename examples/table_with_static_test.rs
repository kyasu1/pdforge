use pdforge::PDForgeBuilder;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create PDForge instance with fonts
    let mut pdforge = PDForgeBuilder::new("table-with-static-test".to_string())
        .add_font("NotoSansJP-Regular", "./assets/fonts/NotoSansJP-Regular.otf")?
        .add_font("NotoSansJP-Bold", "./assets/fonts/NotoSansJP-Bold.otf")?
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.otf")?
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .load_template("table-with-static-simple", "./templates/table-with-static-simple.json")?
        .build();

    // Create page inputs (empty for this test)
    let inputs = vec![vec![HashMap::new()]];

    // Create table data (dynamic table content)
    let mut table_data: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    table_data.insert("sales_table".to_string(), vec![
        vec!["Product".to_string(), "Sales".to_string(), "Region".to_string()],
        vec!["Widget A".to_string(), "1000".to_string(), "Tokyo".to_string()],
        vec!["Widget B".to_string(), "1500".to_string(), "Osaka".to_string()],
        vec!["Widget C".to_string(), "2000".to_string(), "Kyoto".to_string()],
        vec!["Widget D".to_string(), "800".to_string(), "Fukuoka".to_string()],
    ]);

    // Create static schema inputs (custom variables for headers/footers)
    let mut static_inputs: HashMap<String, String> = HashMap::new();
    static_inputs.insert("company_name".to_string(), "ABC Corporation".to_string());
    static_inputs.insert("report_type".to_string(), "Monthly Sales".to_string());
    static_inputs.insert("author_name".to_string(), "Sales Team".to_string());
    static_inputs.insert("confidentiality_level".to_string(), "CONFIDENTIAL".to_string());

    // Render PDF with all three types of inputs: page inputs, table data, and static inputs
    let pdf_bytes = pdforge.render(
        "table-with-static-simple",
        inputs,
        Some(table_data),
        Some(static_inputs),
    )?;

    // Save PDF
    std::fs::write("examples/pdf/table-with-static-test.pdf", pdf_bytes)?;
    println!("PDF generated successfully with table data and static schema variables!");

    Ok(())
}