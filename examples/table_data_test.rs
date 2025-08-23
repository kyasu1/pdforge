use std::collections::HashMap;
use pdforge::PDForgeBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing dynamic table data injection...");

    let mut pdforge = PDForgeBuilder::new("Table Data Test".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .load_template("table_test", "./templates/table.json")?
        .build();

    // 動的なテーブルデータを準備
    let mut table_data = HashMap::new();
    table_data.insert("field1".to_string(), vec![
        vec!["Alice".to_string(), "Tokyo".to_string(), "Software Engineer".to_string(), "Bob".to_string(), "Osaka".to_string(), "Designer".to_string(), "120,000円".to_string()],
        vec!["Charlie".to_string(), "Kyoto".to_string(), "Product Manager".to_string(), "Diana".to_string(), "Fukuoka".to_string(), "Analyst".to_string(), "95,000円".to_string()],
        vec!["Eve".to_string(), "Sendai".to_string(), "Data Scientist".to_string(), "Frank".to_string(), "Hiroshima".to_string(), "Consultant".to_string(), "110,000円".to_string()],
    ]);

    // 新しいAPIを使ってPDFを生成
    let pdf_bytes = pdforge.render("table_test", vec![vec![]], Some(table_data), None)?;

    // PDFファイルに書き出し
    std::fs::write("examples/pdf/table_dynamic_data.pdf", pdf_bytes)?;

    println!("Dynamic table data PDF generated successfully at examples/pdf/table_dynamic_data.pdf");

    Ok(())
}