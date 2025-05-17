fn main() {
    let mut pdfme = rust_pdfme::PDFme::new("TEST".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf");

    pdfme.load_template("table", "./templates/table.json");

    let bytes: Vec<u8> = pdfme.render("table");

    std::fs::write("./table.pdf", bytes).unwrap();
}
