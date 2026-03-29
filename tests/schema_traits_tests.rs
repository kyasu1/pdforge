//! Tests for SchemaTrait and HasBaseSchema traits
//!
//! This file follows Test-Driven Development (TDD) principles:
//! 1. Write failing tests first (RED)
//! 2. Implement minimal code to pass (GREEN)
//! 3. Refactor while keeping tests green

use pdforge::schemas::base::BaseSchema;
use pdforge::schemas::{SchemaTrait, HasBaseSchema, BoundingBox, Schema};
use pdforge::schemas::qrcode::QrCode;
use pdforge::schemas::rect::Rect;
use printpdf::{Mm, PdfDocument};
use pdforge::utils::OpBuffer;

// Mock schema for testing the traits
#[derive(Debug, Clone)]
struct MockSchema {
    base: BaseSchema,
    custom_value: i32,
}

impl MockSchema {
    fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            base: BaseSchema::new(
                "mock_schema".to_string(),
                Mm(x),
                Mm(y),
                Mm(width),
                Mm(height),
            ),
            custom_value: 42,
        }
    }
}

// Implement HasBaseSchema for MockSchema
impl HasBaseSchema for MockSchema {
    fn base(&self) -> &BaseSchema {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseSchema {
        &mut self.base
    }
}

// Implement SchemaTrait for MockSchema
impl SchemaTrait for MockSchema {
    fn render(
        &self,
        _parent_height: Mm,
        _doc: &mut PdfDocument,
        _page: usize,
        _buffer: &mut OpBuffer,
    ) -> Result<(), pdforge::schemas::Error> {
        Ok(())
    }

    fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }

    fn set_height(&mut self, height: Mm) {
        self.base.height = height;
    }
}

#[test]
fn test_has_base_schema_returns_base_reference() {
    // Arrange
    let schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    let base = schema.base();

    // Assert
    assert_eq!(base.name, "mock_schema");
    assert_eq!(base.x, Mm(10.0));
    assert_eq!(base.y, Mm(20.0));
    assert_eq!(base.width, Mm(100.0));
    assert_eq!(base.height, Mm(50.0));
}

#[test]
fn test_has_base_schema_returns_mut_reference() {
    // Arrange
    let mut schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    let base_mut = schema.base_mut();
    base_mut.x = Mm(15.0);
    base_mut.y = Mm(25.0);

    // Assert
    assert_eq!(schema.base.x, Mm(15.0));
    assert_eq!(schema.base.y, Mm(25.0));
}

#[test]
fn test_schema_trait_position_method() {
    // Arrange
    let schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    let (x, y) = schema.position();

    // Assert
    assert_eq!(x, Mm(10.0));
    assert_eq!(y, Mm(20.0));
}

#[test]
fn test_schema_trait_size_method() {
    // Arrange
    let schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    let (width, height) = schema.size();

    // Assert
    assert_eq!(width, Mm(100.0));
    assert_eq!(height, Mm(50.0));
}

#[test]
fn test_schema_trait_bounds_method() {
    // Arrange
    let schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    let bounds = schema.bounds();

    // Assert
    assert_eq!(bounds.x, Mm(10.0));
    assert_eq!(bounds.y, Mm(20.0));
    assert_eq!(bounds.width, Mm(100.0));
    assert_eq!(bounds.height, Mm(50.0));
}

#[test]
fn test_schema_trait_set_y_modifies_base() {
    // Arrange
    let mut schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    schema.set_y(Mm(30.0));

    // Assert
    assert_eq!(schema.base.y, Mm(30.0));
}

#[test]
fn test_schema_trait_set_height_modifies_base() {
    // Arrange
    let mut schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    schema.set_height(Mm(60.0));

    // Assert
    assert_eq!(schema.base.height, Mm(60.0));
}

#[test]
fn test_schema_trait_name_method() {
    // Arrange
    let schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    let name = schema.name();

    // Assert
    assert_eq!(name, "mock_schema");
}

#[test]
fn test_generic_function_with_schema_trait() {
    // Test that we can write generic code using SchemaTrait
    fn render_all(schemas: &[&dyn SchemaTrait]) -> Result<(), pdforge::schemas::Error> {
        for schema in schemas {
            schema.render(Mm(297.0), &mut PdfDocument::new("test"), 0, &mut OpBuffer::default())?;
        }
        Ok(())
    }

    // Arrange
    let schema1 = MockSchema::new(10.0, 20.0, 100.0, 50.0);
    let schema2 = MockSchema::new(50.0, 60.0, 80.0, 40.0);
    let schemas: Vec<&dyn SchemaTrait> = vec![&schema1, &schema2];

    // Act
    let result = render_all(&schemas);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn test_has_base_schema_can_modify_values() {
    // Arrange
    let mut schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act - Modify through base_mut
    schema.base_mut().x = Mm(100.0);
    schema.base_mut().y = Mm(200.0);
    schema.base_mut().width = Mm(150.0);
    schema.base_mut().height = Mm(80.0);

    // Assert
    assert_eq!(schema.base.x, Mm(100.0));
    assert_eq!(schema.base.y, Mm(200.0));
    assert_eq!(schema.base.width, Mm(150.0));
    assert_eq!(schema.base.height, Mm(80.0));
}

#[test]
fn test_bounding_box_creation() {
    // Arrange
    let bounds = BoundingBox::new(Mm(10.0), Mm(20.0), Mm(100.0), Mm(50.0));

    // Assert
    assert_eq!(bounds.x, Mm(10.0));
    assert_eq!(bounds.y, Mm(20.0));
    assert_eq!(bounds.width, Mm(100.0));
    assert_eq!(bounds.height, Mm(50.0));
}

#[test]
fn test_bounding_box_from_json() {
    // Arrange
    let json_bounds = pdforge::schemas::JsonBoundingBox {
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
    };

    // Act
    let bounds = BoundingBox::from_json(json_bounds);

    // Assert
    assert_eq!(bounds.x, Mm(10.0));
    assert_eq!(bounds.y, Mm(20.0));
    assert_eq!(bounds.width, Mm(100.0));
    assert_eq!(bounds.height, Mm(50.0));
}

#[test]
fn test_generic_position_and_size_operations() {
    // Test generic function that uses position() and size()
    fn calculate_center<S: SchemaTrait>(schema: &S) -> (Mm, Mm) {
        let (x, y) = schema.position();
        let (width, height) = schema.size();
        (x + width / 2.0, y + height / 2.0)
    }

    // Arrange
    let schema = MockSchema::new(10.0, 20.0, 100.0, 50.0);

    // Act
    let center = calculate_center(&schema);

    // Assert
    assert_eq!(center.0, Mm(60.0)); // 10 + 100/2
    assert_eq!(center.1, Mm(45.0)); // 20 + 50/2
}
// ---- RED phase: Schema enum set_y/set_height coverage tests ----

#[test]
fn test_schema_qrcode_set_y_and_set_height() {
    // QrCode variant must support set_y and set_height without panicking
    let qr = QrCode::new(
        "qr_test".to_string(),
        Mm(10.0), Mm(20.0), Mm(50.0), Mm(50.0),
        "test_content".to_string(),
    );
    let mut schema = Schema::QrCode(qr);

    schema.set_y(Mm(99.0));
    schema.set_height(Mm(30.0));

    assert_eq!(schema.position().1, Mm(99.0));
    assert_eq!(schema.size().1, Mm(30.0));
}

#[test]
fn test_schema_rect_set_y_and_set_height() {
    // Rect variant must support set_y and set_height without panicking
    use serde_json::json;
    let json_val = json!({
        "name": "rect_test",
        "position": {"x": 10.0, "y": 20.0},
        "width": 100.0,
        "height": 50.0,
        "borderWidth": 1.0,
        "borderColor": "#000000",
        "color": "#ffffff"
    });
    let json_rect: pdforge::schemas::rect::JsonRectSchema = serde_json::from_value(json_val).unwrap();
    let mut schema: Schema = json_rect.try_into().unwrap();

    schema.set_y(Mm(77.0));
    schema.set_height(Mm(25.0));

    assert_eq!(schema.position().1, Mm(77.0));
    assert_eq!(schema.size().1, Mm(25.0));
}
