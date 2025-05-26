fn main() {
    let mut pdforge = pdforge::PDForgeBuilder::new("SVG Example".to_string())
        .load_template("svg", "./templates/tiger-svg.json")
        .build();

    let bytes: Vec<u8> = pdforge.render("svg");

    std::fs::write("./svg.pdf", bytes).unwrap();
}
