fn main() {
    let mut pdforge = pdforge::PDForge::new("SVG Example".to_string());
    pdforge.load_template("svg", "./templates/tiger-svg.json");

    let bytes: Vec<u8> = pdforge.render("svg");

    std::fs::write("./svg.pdf", bytes).unwrap();
}
