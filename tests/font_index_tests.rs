use pdforge::schemas::Error;
use pdforge::PDForgeBuilder;
use printpdf::ParsedFont;
use std::path::PathBuf;

fn two_face_ttc_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("two-face-test.ttc")
}

fn two_face_ttc_bytes() -> Vec<u8> {
    std::fs::read(two_face_ttc_path()).expect("fixture TTC should be readable")
}

#[test]
fn two_face_fixture_faces_are_actually_distinct() {
    // Sanity-check the fixture itself: index 0 ("TestFaceA") has 1 glyph
    // beyond .notdef, index 1 ("TestFaceB") has 2, so a correct font_index
    // selection is observable via glyph count.
    let bytes = two_face_ttc_bytes();
    let mut warnings = Vec::new();
    let face0 =
        ParsedFont::from_bytes(&bytes, 0, &mut warnings).expect("face 0 of fixture should parse");
    let face1 =
        ParsedFont::from_bytes(&bytes, 1, &mut warnings).expect("face 1 of fixture should parse");

    assert_ne!(
        face0.num_glyphs, face1.num_glyphs,
        "fixture faces should be distinguishable by glyph count"
    );
}

#[test]
fn add_font_with_index_loads_each_face_of_a_collection() {
    let bytes = two_face_ttc_bytes();

    let builder = PDForgeBuilder::new("font-index-test".to_string())
        .add_font_with_index("TestFaceA", &bytes, 0)
        .expect("index 0 should load")
        .add_font_with_index("TestFaceB", &bytes, 1)
        .expect("index 1 should load");

    // build() just moves data into PDForge; reaching here without panicking
    // confirms both indices were accepted.
    let _ = builder.build();
}

#[test]
fn add_font_from_file_with_index_loads_each_face_of_a_collection() {
    let path = two_face_ttc_path();
    let path = path.to_str().unwrap();

    let builder = PDForgeBuilder::new("font-index-test".to_string())
        .add_font_from_file_with_index("TestFaceA", path, 0)
        .expect("index 0 should load")
        .add_font_from_file_with_index("TestFaceB", path, 1)
        .expect("index 1 should load");

    let _ = builder.build();
}

#[test]
fn add_font_with_index_rejects_out_of_range_index() {
    let bytes = two_face_ttc_bytes();

    let result = PDForgeBuilder::new("font-index-test".to_string()).add_font_with_index(
        "TooFarFace",
        &bytes,
        2,
    );

    match result {
        Err(Error::FontParsing { message }) => {
            assert!(
                message.contains("TooFarFace"),
                "error message should name the font: {message}"
            );
            assert!(
                message.contains("font_index: 2"),
                "error message should mention the out-of-range index: {message}"
            );
        }
        Ok(_) => panic!("expected Error::FontParsing, got Ok"),
        Err(other) => panic!("expected Error::FontParsing, got {other:?}"),
    }
}

#[test]
fn add_font_defaults_to_index_zero() {
    // add_font() must keep loading face 0 (unchanged public behavior) after
    // being refactored to delegate to add_font_with_index.
    let bytes = two_face_ttc_bytes();

    let builder = PDForgeBuilder::new("font-index-test".to_string())
        .add_font("DefaultFace", &bytes)
        .expect("add_font should still load index 0 by default");

    let _ = builder.build();
}

#[test]
fn add_font_from_file_defaults_to_index_zero() {
    // add_font_from_file() must keep loading face 0 (unchanged public
    // behavior) after being refactored to delegate to
    // add_font_from_file_with_index.
    let path = two_face_ttc_path();
    let path = path.to_str().unwrap();

    let builder = PDForgeBuilder::new("font-index-test".to_string())
        .add_font_from_file("DefaultFileFace", path)
        .expect("add_font_from_file should still load index 0 by default");

    let _ = builder.build();
}
