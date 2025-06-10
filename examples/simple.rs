use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <template_file>", args[0]);
        std::process::exit(1);
    }

    let template_file = &args[1];
    let template_path = Path::new(template_file);

    // テンプレートファイル名から拡張子を除いた部分を取得
    let file_stem = template_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let mut pdforge = pdforge::PDForgeBuilder::new("Example".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")
        .load_template("template", template_file)
        .build();

    let bytes: Vec<u8> = pdforge.render("template");

    // 出力ファイル名を元のファイル名をベースに生成
    let output_file = format!("./examples/pdf/{}.pdf", file_stem);
    std::fs::write(&output_file, bytes).unwrap();

    println!("PDF generated: {}", output_file);
}
