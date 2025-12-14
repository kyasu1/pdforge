use std::collections::HashMap;
use pdforge::PDForgeBuilder;

/// テンプレート変数（inputs）と動的テーブルデータの両方を使用する例
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PDForge - Inputs + Table Data Combined Example");

    // PDForgeを初期化
    let mut pdforge = PDForgeBuilder::new("Combined Example".to_string())
        .add_font_from_file("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .add_font_from_file("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .load_template("combined", "./templates/combined_example.json")?
        .build();

    // テンプレート変数（inputs）を準備
    let mut inputs = Vec::new();
    let page_inputs = vec![
        {
            let mut input = HashMap::new();
            input.insert("report_title", "月次社員評価レポート".to_string());
            input.insert("created_date", "2024年3月15日".to_string());
            input.insert("department", "開発部".to_string());
            input
        }
    ];
    inputs.push(page_inputs);

    // テーブルデータを準備
    let mut table_data = HashMap::new();
    table_data.insert("employee_table", vec![
        vec![
            "田中 太郎".to_string(),
            "シニアエンジニア".to_string(), 
            "8年".to_string(),
            "S".to_string(),
            "¥850,000".to_string()
        ],
        vec![
            "佐藤 花子".to_string(),
            "UXデザイナー".to_string(),
            "5年".to_string(), 
            "A".to_string(),
            "¥720,000".to_string()
        ],
        vec![
            "鈴木 一郎".to_string(),
            "QAエンジニア".to_string(),
            "6年".to_string(),
            "A".to_string(), 
            "¥680,000".to_string()
        ],
        vec![
            "山田 美咲".to_string(),
            "プロダクトマネージャー".to_string(),
            "4年".to_string(),
            "B".to_string(),
            "¥750,000".to_string()
        ],
        vec![
            "高橋 健".to_string(), 
            "ジュニアエンジニア".to_string(),
            "2年".to_string(),
            "B".to_string(),
            "¥520,000".to_string()
        ]
    ]);

    // 両方を組み合わせてPDFを生成
    println!("Generating PDF with both template variables and dynamic table data...");
    let pdf_bytes = pdforge.render("combined", inputs, Some(table_data), None)?;

    // PDFファイルに保存
    std::fs::write("examples/pdf/inputs_and_table_combined.pdf", pdf_bytes)?;
    println!("Combined PDF generated: examples/pdf/inputs_and_table_combined.pdf");

    println!("成功！以下の内容を含むPDFが生成されました:");
    println!("- テンプレート変数: タイトル、作成日、部署名");
    println!("- 動的テーブルデータ: 社員情報（名前、役職、経験年数、評価、給与）");

    Ok(())
}