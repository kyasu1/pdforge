fn main() {
    let builder = pdforge::PDForgeBuilder::new("Table Example".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")
        .load_template("table", "./templates/table.json");

    let mut pdforge = builder.build();

    let bytes: Vec<u8> = pdforge.render("table");

    std::fs::write("./table.pdf", bytes).unwrap();
}
