fn main() {
    let mut pdfme = pdforge::PDFme::new("TEST".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf");

    pdfme.load_template("table-test", "./templates/table-test.json");

    let bytes: Vec<u8> = pdfme.render("table-test");

    std::fs::write("./simple.pdf", bytes).unwrap();
}
