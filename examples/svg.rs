fn main() {
    let mut pdfme = pdforge::PDFme::new("SVG Example".to_string());
    pdfme.load_template("svg", "./templates/tiger-svg.json");

    let bytes: Vec<u8> = pdfme.render("svg");

    std::fs::write("./svg.pdf", bytes).unwrap();
}
