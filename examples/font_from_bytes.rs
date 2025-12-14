use std::env;
use std::path::Path;

/// Example demonstrating font loading from byte slices instead of file paths
/// This is useful when fonts are embedded in the binary, loaded from network, or cached in memory
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <template_file>", args[0]);
        std::process::exit(1);
    }

    let template_file = &args[1];
    let template_path = Path::new(template_file);

    let file_stem = template_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // Load font files into byte arrays
    // This demonstrates the add_font() method which accepts byte slices
    let noto_serif_bytes = std::fs::read("./assets/fonts/NotoSerifJP-Regular.ttf")?;
    let noto_sans_bytes = std::fs::read("./assets/fonts/NotoSansJP-Regular.ttf")?;

    // Build PDForge using byte slices instead of file paths
    // add_font() is the primary API that accepts byte slices, eliminating redundant disk I/O
    let mut pdforge = pdforge::PDForgeBuilder::new("Example".to_string())
        .add_font("NotoSerifJP", &noto_serif_bytes)?
        .add_font("NotoSansJP", &noto_sans_bytes)?
        .load_template("template", template_file)?
        .build();

    let bytes: Vec<u8> = pdforge.render("template", vec![vec![]], None, None)?;

    let output_file = format!("./examples/pdf/{}_from_bytes.pdf", file_stem);
    std::fs::write(&output_file, bytes)?;

    println!("PDF generated using byte-based font loading: {}", output_file);
    Ok(())
}
