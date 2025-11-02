use std::collections::HashMap;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    println!("Starting 50-page table PDF generation...");

    let setup_start = Instant::now();

    // Create PDForge instance
    let mut pdforge = pdforge::PDForgeBuilder::new("50ページテーブルテスト".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("print-renews", "./templates/print-renews.json")?
        .build();

    let setup_time = setup_start.elapsed();
    println!("Setup completed: {:?}", setup_time);

    // Prepare data
    let data_start = Instant::now();

    // Create sample row data
    let row = vec![
        "2025-11-02".to_string(),
        "山田 太郎".to_string(),
        "27-0001".to_string(),
        "2023-05-05".to_string(),
        "50,000".to_string(),
        "2".to_string(),
        "7,000".to_string(),
        "2025-11-02".to_string(),
    ];

    // Generate dataset for approximately 50 pages
    // Based on testing: ~31-32 rows per page
    // So for 50 pages we need about 1550-1600 rows
    let total_rows = 1600;
    let mut rows = Vec::with_capacity(total_rows);

    println!("Generating {} rows of data (target: ~50 pages)...", total_rows);
    for i in 0..total_rows {
        let mut row_data = row.clone();
        // Vary some data to make it more realistic
        row_data[1] = format!("顧客 {}", i + 1);
        row_data[2] = format!("{:05}", i + 1);
        row_data[4] = format!("{},000", (i % 100) + 1);
        rows.push(row_data);
    }

    let data_prep_time = data_start.elapsed();
    println!("Data preparation completed: {:?}", data_prep_time);
    println!("Total data size: {} rows x {} columns = {} cells",
             rows.len(), row.len(), rows.len() * row.len());

    // Render PDF
    let render_start = Instant::now();
    println!("\nStarting PDF rendering (memory-efficient mode)...");

    let mut inputs: Vec<HashMap<&'static str, String>> = vec![];
    let print_datetime = format!("2025-11-02 12:00:00");

    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("printDateTime", print_datetime);
    inputs.push(input);

    let mut table_data = HashMap::new();
    table_data.insert("renewsTable", rows);

    let bytes = pdforge.render(
        "print-renews",
        vec![inputs],
        Some(table_data),
        None,
    )?;

    let render_time = render_start.elapsed();
    println!("Rendering completed: {:?}", render_time);

    // Write to file
    let write_start = Instant::now();
    let output_path = "./examples/pdf/table-50pages.pdf";
    std::fs::write(output_path, &bytes)?;
    let write_time = write_start.elapsed();
    println!("File written: {:?}", write_time);

    // Check page count using lopdf
    use lopdf::Document;
    let doc = Document::load_mem(&bytes)?;
    let page_count = doc.get_pages().len();

    // Summary
    let total_time = start.elapsed();
    println!("\n========== Summary ==========");
    println!("Setup time:           {:?}", setup_time);
    println!("Data preparation:     {:?}", data_prep_time);
    println!("PDF rendering:        {:?}", render_time);
    println!("File write:           {:?}", write_time);
    println!("Total time:           {:?}", total_time);
    println!("PDF size:             {:.2} MB", bytes.len() as f64 / 1_048_576.0);
    println!("Total pages:          {}", page_count);
    println!("Rows per page (avg):  {:.1}", total_rows as f64 / page_count as f64);
    println!("Output file:          {}", output_path);
    println!("============================\n");
    println!("Memory-efficient rendering: Constant memory usage regardless of row count");

    Ok(())
}
