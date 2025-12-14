use std::collections::HashMap;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    let mut pdforge =
        pdforge::PDForgeBuilder::new("利上台帳 株式会社オフイスイコー 2025".to_string())
            .add_font_from_file("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
            .load_template("print-renews", "./templates/print-renews.json")?
            .build();

    let setup_time = start.elapsed();
    println!("Setup time: {:?}", setup_time);

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
    table_data.insert("renewsTable", rows);

    let data_prep_time = start.elapsed();
    println!("Data preparation time: {:?}", data_prep_time - setup_time);

    let render_start = Instant::now();
    let bytes: Vec<u8> =
        pdforge.render("print-renews", vec![inputs], Some(table_data), None)?;
    let render_time = render_start.elapsed();
    println!("Render time: {:?}", render_time);

    let write_start = Instant::now();
    std::fs::write("./examples/pdf/print-renews.pdf", bytes.clone()).unwrap();
    let write_time = write_start.elapsed();
    println!("File write time: {:?}", write_time);

    let total_time = start.elapsed();
    println!("Total time: {:?}", total_time);

    Ok(())
}
