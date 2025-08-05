fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let template_file = if args.len() > 1 {
        &args[1]
    } else {
        "./templates/table-test.json"
    };

    let mut pdforge = pdforge::PDForgeBuilder::new("TEST".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .add_font("NotoSansJP-Regular", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("test", template_file)?
        .build();

    let bytes: Vec<u8> = pdforge.render("test")?;

    let output_file = format!("./examples/pdf/{}.pdf", 
        std::path::Path::new(template_file)
            .file_stem()
            .unwrap_or(std::ffi::OsStr::new("output"))
            .to_str()
            .unwrap_or("output")
    );
    std::fs::write(&output_file, bytes)?;
    println!("Generated PDF: {}", output_file);
    Ok(())
}
