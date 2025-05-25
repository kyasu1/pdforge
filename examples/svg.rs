fn main() {
    let builder = pdforge::PDForgeBuilder::new("SVG Example".to_string())
        .load_template("svg", "./templates/tiger-svg.json");

    let mut pdforge = builder.build();

    let bytes: Vec<u8> = pdforge.render("svg");

    std::fs::write("./svg.pdf", bytes).unwrap();
}
