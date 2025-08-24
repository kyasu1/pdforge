use std::collections::HashMap;
use pdforge::PDForgeBuilder;

/// 簡単な組み合わせテスト
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing combined inputs and table data API...");

    // 既存のテンプレートを使用
    let mut pdforge = PDForgeBuilder::new("Simple Combined Test".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .load_template("table_test", "./templates/table.json")?
        .build();

    // inputs（このテンプレートでは動的テキストがないので空）
    let inputs = vec![vec![]];

    // テーブルデータ
    let mut table_data = HashMap::new();
    table_data.insert("field1", vec![
        vec!["名前".to_string(), "場所".to_string(), "説明".to_string(), "名前2".to_string(), "場所2".to_string(), "説明2".to_string(), "価格".to_string()],
        vec!["太郎".to_string(), "東京".to_string(), "エンジニア".to_string(), "花子".to_string(), "大阪".to_string(), "デザイナー".to_string(), "120,000円".to_string()],
        vec!["次郎".to_string(), "京都".to_string(), "アナリスト".to_string(), "三郎".to_string(), "神戸".to_string(), "マネージャー".to_string(), "95,000円".to_string()],
    ]);

    println!("Table data prepared: {:?}", table_data);
    println!("Inputs prepared: {:?}", inputs);

    // 新しいAPIを使ってPDF生成
    let pdf_bytes = pdforge.render("table_test", inputs, Some(table_data), None)?;
    println!("PDF bytes generated: {} bytes", pdf_bytes.len());

    // ファイルに保存
    std::fs::write("examples/pdf/simple_combined_test.pdf", pdf_bytes)?;
    println!("Combined PDF successfully generated: examples/pdf/simple_combined_test.pdf");

    Ok(())
}