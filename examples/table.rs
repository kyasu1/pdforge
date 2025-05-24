fn main() {
    let mut pdforge = pdforge::PDForge::new("TEST".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf");

    pdforge.load_template("table", "./templates/table.json");

    let bytes: Vec<u8> = pdforge.render("table");

    std::fs::write("./table.pdf", bytes).unwrap();
}
