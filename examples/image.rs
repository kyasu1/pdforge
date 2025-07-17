use base64;
use std::collections::HashMap;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <front_image.jpg> <back_image.jpg>", args[0]);
        std::process::exit(1);
    }

    let front_image_path = &args[1];
    let back_image_path = &args[2];

    // ファイルを読み込んでBase64エンコード
    let front_image_data = fs::read(front_image_path)?;
    let back_image_data = fs::read(back_image_path)?;

    let front_base64 = format!(
        "data:image/jpeg;base64,{}",
        base64::encode(&front_image_data)
    );
    let back_base64 = format!(
        "data:image/jpeg;base64,{}",
        base64::encode(&back_image_data)
    );

    let mut pdforge = pdforge::PDForgeBuilder::new("IMAGE_TEST".to_string())
        .load_template("image-test", "./templates/image-test.json")?
        .build();

    let mut inputs = vec![];

    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("frontImage", front_base64);
    input.insert("backImage", back_base64);

    inputs.push(input);

    let bytes: Vec<u8> = pdforge.render_with_inputs("image-test", vec![inputs])?;

    std::fs::write("./image-test.pdf", bytes.clone()).unwrap();

    println!("PDF created successfully: ./image-test.pdf");

    Ok(())
}
