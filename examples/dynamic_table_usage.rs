use std::collections::HashMap;
use pdforge::PDForgeBuilder;

/// 実行時にテーブルデータを動的に注入する例
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PDForge - Dynamic Table Data Injection Example");

    // PDForgeを初期化
    let mut pdforge = PDForgeBuilder::new("Dynamic Table Example".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .load_template("table_template", "./templates/table.json")?
        .build();

    // 従来の方法: JSONテンプレートのハードコードされたデータを使用
    println!("Generating PDF with static data...");
    let static_pdf = pdforge.render("table_template", vec![vec![]], None, None)?;
    std::fs::write("examples/pdf/static_table.pdf", static_pdf)?;
    println!("Static PDF generated: examples/pdf/static_table.pdf");

    // 新しい方法: 実行時にテーブルデータを動的に注入
    println!("Generating PDF with dynamic data...");
    
    // テーブル名とデータのマップを作成
    let mut table_data = HashMap::new();
    
    // "field1"という名前のテーブルにデータを注入
    // この例では、社員情報テーブルを作成
    table_data.insert("field1", vec![
        // ヘッダー行
        vec![
            "名前".to_string(), 
            "部署".to_string(), 
            "職種".to_string(), 
            "スキル".to_string(), 
            "経験年数".to_string(), 
            "プロジェクト".to_string(), 
            "評価".to_string()
        ],
        // データ行
        vec![
            "田中 太郎".to_string(), 
            "開発部".to_string(), 
            "シニアエンジニア".to_string(), 
            "Rust, Go, Python".to_string(), 
            "8年".to_string(), 
            "PDForge開発".to_string(), 
            "S".to_string()
        ],
        vec![
            "佐藤 花子".to_string(), 
            "UX部".to_string(), 
            "UXデザイナー".to_string(), 
            "Figma, React, TypeScript".to_string(), 
            "5年".to_string(), 
            "PDFビューア改善".to_string(), 
            "A".to_string()
        ],
        vec![
            "鈴木 一郎".to_string(), 
            "QA部".to_string(), 
            "テストエンジニア".to_string(), 
            "Selenium, Jest, Cypress".to_string(), 
            "6年".to_string(), 
            "自動テスト構築".to_string(), 
            "A".to_string()
        ],
        vec![
            "山田 美咲".to_string(), 
            "マーケティング部".to_string(), 
            "プロダクトマネージャー".to_string(), 
            "Analytics, SQL, Tableau".to_string(), 
            "4年".to_string(), 
            "ユーザー分析".to_string(), 
            "B".to_string()
        ],
    ]);

    // 動的データを使用してPDFを生成
    let dynamic_pdf = pdforge.render("table_template", vec![vec![]], Some(table_data), None)?;
    std::fs::write("examples/pdf/dynamic_table.pdf", dynamic_pdf)?;
    println!("Dynamic PDF generated: examples/pdf/dynamic_table.pdf");

    println!("両方のPDFが生成されました！内容を比較してみてください。");
    println!("- static_table.pdf: JSONテンプレートのハードコードされたデータ");
    println!("- dynamic_table.pdf: プログラムで動的に注入されたデータ");

    Ok(())
}