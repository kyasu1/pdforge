use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pdforge = pdforge::PDForgeBuilder::new("INVENTORY_TAG_EXAMPLE".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("inventory_tag", "./templates/inventory_tag.json")?
        .build();

    let mut inputs: Vec<HashMap<&'static str, String>> = vec![];

    let mut input: HashMap<&'static str, String> = HashMap::new();

    input.insert("content", "時計 / セイコー / SBEC019 374/600 プロスペックス ステンレス スピードダイマー クロノグラフ 黒文字盤 自動巻 箱付 保付".to_string());
    input.insert("inventoryId", "3100-0000-011".to_string());
    input.insert("qrCode", "31000000011".to_string());

    inputs.push(input);

    let bytes: Vec<u8> = pdforge.render("inventory_tag", vec![inputs], None, None)?;

    std::fs::write("./examples/pdf/inventory_tag.pdf", bytes.clone()).unwrap();

    println!("Inventory tag PDF generated successfully at ./examples/pdf/inventory_tag.pdf");

    Ok(())
}
