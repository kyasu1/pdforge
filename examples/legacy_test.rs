use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read test image
    let image_data = fs::read("assets/images/test-image-1.jpg")?;
    let base64_image = format!(
        "data:image/jpeg;base64,{}",
        general_purpose::STANDARD.encode(&image_data)
    );

    let mut pdforge = pdforge::PDForgeBuilder::new("LEGACY_TEST".to_string())
        .load_template("legacy-test", "./templates/legacy-image-test.json")?
        .build();

    let mut inputs = vec![];
    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("testImage", base64_image);
    inputs.push(input);

    let bytes: Vec<u8> = pdforge.render_with_inputs("legacy-test", vec![inputs])?;

    std::fs::write("./legacy-test.pdf", bytes.clone()).unwrap();

    println!("Legacy compatibility test PDF created successfully: ./legacy-test.pdf");
    println!("This PDF uses the default object-fit behavior (fill) for backwards compatibility");

    Ok(())
}