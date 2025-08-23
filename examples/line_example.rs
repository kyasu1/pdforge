use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Line Schema Example - Generating PDF with various line styles...");

    // PDForgeインスタンスを作成し、フォントを読み込む
    let mut pdforge = pdforge::PDForgeBuilder::new("Line Example".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("line_template", "./templates/line-example.json")?
        .build();

    // テンプレートをレンダリングしてPDFを生成
    let bytes: Vec<u8> = pdforge.render("line_template", vec![vec![]], None, None)?;

    // PDFファイルを保存
    let output_file = "./examples/pdf/line_example.pdf";
    std::fs::write(output_file, bytes)?;

    println!(
        "✅ Line example PDF generated successfully: {}",
        output_file
    );
    println!("📄 The PDF demonstrates:");
    println!("   • Basic horizontal lines with different thickness");
    println!("   • Colored lines (red, green, blue)");
    println!("   • Rotated lines (15°, 45°, 90°)");
    println!("   • Various line styles and positions");

    Ok(())
}
