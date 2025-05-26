fn main() {
    let mut pdforge = pdforge::PDForgeBuilder::new("TEST".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")
        .load_template("table-test", "./templates/table-test.json")
        .build();

    let bytes: Vec<u8> = pdforge.render("table-test");

    std::fs::write("./simple.pdf", bytes).unwrap();
}
