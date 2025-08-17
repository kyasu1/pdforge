use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pdforge = pdforge::PDForgeBuilder::new("利上台帳".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("print-renews", "./templates/print-renews.json")?
        .build();

    let mut inputs: Vec<HashMap<&'static str, String>> = vec![];

    // Get current datetime for printing
    let print_datetime = format!("2025-08-05 00:58:00"); // For demo purposes

    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("printDateTime", print_datetime.clone());
    inputs.push(input);

    let row = vec![
        "2025-08-05".to_string(),
        "山田 太郎".to_string(),
        "27-0001".to_string(),
        "2023-05-05".to_string(),
        "50,000".to_string(),
        "2".to_string(),
        "7,000".to_string(),
        "2025-08-05".to_string(),
    ];
    let mut rows = vec![];
    for _ in 0..100 {
        rows.push(row.clone());
    }

    let mut table_data = HashMap::new();
    table_data.insert("renewsTable".to_string(), rows);

    let bytes: Vec<u8> =
        pdforge.render_with_inputs_and_table_data("print-renews", vec![inputs], table_data)?;

    let updated = pdforge::PDForge::set_pdf_metadata(
        bytes,
        Some("利上台帳"),
        Some("株式会社オフイスイコー"),
    )?;
    std::fs::write("./examples/pdf/print-renews.pdf", updated.clone()).unwrap();

    Ok(())
}
