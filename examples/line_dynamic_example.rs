use std::collections::HashMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Line Schema Dynamic Example - Generating PDF with template variables...");

    // PDForgeインスタンスを作成し、フォントを読み込む
    let mut pdforge = pdforge::PDForgeBuilder::new("Line Dynamic Example".to_string())
        .add_font_from_file("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .add_font_from_file("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("line_dynamic_template", "./templates/line-dynamic.json")?
        .build();

    // テンプレート変数のデータを準備
    let mut template_data = HashMap::new();
    template_data.insert("document_title", "Dynamic Line Example".to_string());
    template_data.insert("title_line_color", "#2980ba".to_string());
    template_data.insert("section_1_title", "Section 1: Introduction".to_string());
    template_data.insert("section_1_content", "This section demonstrates how line schemas can be used with dynamic templates. The lines can change color, width, and position based on template variables.".to_string());
    template_data.insert("section_2_title", "Section 2: Implementation".to_string());
    template_data.insert("section_2_content", "Lines are rendered using the PDForge line schema with configurable properties. This allows for flexible document layouts with dynamic styling.".to_string());
    template_data.insert("highlight_color", "#e74c3c".to_string());
    template_data.insert("date", "2024-01-01".to_string());
    template_data.insert("author", "PDForge".to_string());

    // データを配列形式で準備（単一ページの例）
    let inputs = vec![vec![template_data]];

    // テンプレートをレンダリングしてPDFを生成
    let bytes: Vec<u8> = pdforge.render("line_dynamic_template", inputs, None, None)?;

    // PDFファイルを保存
    let output_file = "./examples/pdf/line_dynamic_example.pdf";
    std::fs::write(output_file, bytes)?;

    println!(
        "✅ Dynamic line example PDF generated successfully: {}",
        output_file
    );
    println!("📄 The PDF demonstrates:");
    println!("   • Template-driven line colors and properties");
    println!("   • Dynamic line positioning and styling");
    println!("   • Configurable highlight lines");
    println!("   • Variable-based content positioning");

    Ok(())
}
