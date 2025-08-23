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

    let mut pdforge = pdforge::PDForgeBuilder::new("OBJECT_FIT_TEST".to_string())
        .load_template("object-fit-test", "./templates/object-fit-test.json")?
        .build();

    let mut inputs = vec![];
    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("testImage", base64_image);
    inputs.push(input);

    let bytes: Vec<u8> = pdforge.render("object-fit-test", vec![inputs], None, None)?;

    std::fs::write("./object-fit-test.pdf", bytes.clone()).unwrap();

    println!("Object-fit test PDF created successfully: ./object-fit-test.pdf");
    println!("This PDF shows the same image with different object-fit values:");
    println!("- Top row: fill, contain, cover");
    println!("- Bottom row: none, scale-down");

    Ok(())
}