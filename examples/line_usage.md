# Line Schema Usage Examples

This document demonstrates how to use the Line schema in PDForge to create various types of lines in PDF documents.

## Basic Usage

The Line schema allows you to draw straight lines with customizable properties:

### JSON Schema Format

```json
{
    "name": "my_line",
    "type": "line",
    "position": { "x": 20, "y": 40 },
    "width": 170,
    "height": 0,
    "rotate": 0,
    "borderWidth": 1,
    "color": "#000000"
}
```

### Properties

- **name**: Unique identifier for the line element
- **type**: Must be "line"
- **position**: Starting position (x, y coordinates in mm)
- **width**: Length of the line in mm
- **height**: Should be 0 for lines
- **rotate**: Rotation angle in degrees (0-360)
- **borderWidth**: Thickness of the line in points
- **color**: Line color in hex format (e.g., "#000000" for black)

## Examples

### 1. Basic Horizontal Line

```json
{
    "name": "horizontal_line",
    "type": "line",
    "position": { "x": 20, "y": 40 },
    "width": 170,
    "height": 0,
    "rotate": 0,
    "borderWidth": 1,
    "color": "#000000"
}
```

Creates a 1pt thick black horizontal line.

### 2. Colored Lines

```json
{
    "name": "red_line",
    "type": "line",
    "position": { "x": 30, "y": 60 },
    "width": 100,
    "height": 0,
    "rotate": 0,
    "borderWidth": 3,
    "color": "#e74c3c"
}
```

Creates a 3pt thick red line.

### 3. Rotated Lines

```json
{
    "name": "diagonal_line",
    "type": "line",
    "position": { "x": 50, "y": 80 },
    "width": 50,
    "height": 0,
    "rotate": 45,
    "borderWidth": 2,
    "color": "#3498db"
}
```

Creates a 45-degree diagonal line.

### 4. Vertical Line

```json
{
    "name": "vertical_line",
    "type": "line",
    "position": { "x": 100, "y": 100 },
    "width": 50,
    "height": 0,
    "rotate": 90,
    "borderWidth": 1.5,
    "color": "#27ae60"
}
```

Creates a vertical line using 90-degree rotation.

## Running the Examples

### Static Examples

Run the basic line examples:

```bash
cargo run --example line_example
```

This generates `./examples/pdf/line_example.pdf` demonstrating:
- Different line thicknesses
- Various colors
- Rotated lines at different angles

### Dynamic Templates

Run the dynamic template example:

```bash
cargo run --example line_dynamic_example
```

This generates `./examples/pdf/line_dynamic_example.pdf` demonstrating:
- Template-driven line colors
- Variable-based positioning
- Dynamic content with line separators

### Using with Your Own Templates

```bash
cargo run --example simple ./templates/your-line-template.json
```

## Common Use Cases

### Document Separators

Use horizontal lines to separate sections in documents:

```json
{
    "name": "section_separator",
    "type": "line",
    "position": { "x": 20, "y": 100 },
    "width": 170,
    "height": 0,
    "rotate": 0,
    "borderWidth": 0.5,
    "color": "#cccccc"
}
```

### Headers and Footers

Create underlines for titles:

```json
{
    "name": "title_underline",
    "type": "line",
    "position": { "x": 20, "y": 35 },
    "width": 170,
    "height": 0,
    "rotate": 0,
    "borderWidth": 2,
    "color": "#2980ba"
}
```

### Borders and Frames

Create simple borders by combining multiple lines:

```json
[
    {
        "name": "top_border",
        "type": "line",
        "position": { "x": 20, "y": 20 },
        "width": 170,
        "borderWidth": 1,
        "color": "#000000"
    },
    {
        "name": "left_border",
        "type": "line",
        "position": { "x": 20, "y": 20 },
        "width": 100,
        "rotate": 90,
        "borderWidth": 1,
        "color": "#000000"
    }
]
```

### Highlight Lines

Use thick, colored lines to highlight important sections:

```json
{
    "name": "highlight_line",
    "type": "line",
    "position": { "x": 15, "y": 140 },
    "width": 5,
    "height": 0,
    "rotate": 90,
    "borderWidth": 4,
    "color": "#e74c3c"
}
```

## Tips and Best Practices

1. **Line Positioning**: The position specifies the starting point of the line.

2. **Rotation**: Lines rotate around their starting point. Consider adjusting position when rotating.

3. **Line Width**: Use `borderWidth` to control thickness, not the `width` property (which controls length).

4. **Color Formats**: Use hex colors (#RRGGBB) for consistent color representation.

5. **Thin Lines**: For very thin separator lines, use borderWidth values like 0.5 or 0.25.

6. **Performance**: Line elements are lightweight and render quickly.

## Template Variables

Lines can use template variables for dynamic content:

```json
{
    "name": "dynamic_line",
    "type": "line",
    "position": { "x": 20, "y": 40 },
    "width": 170,
    "height": 0,
    "rotate": 0,
    "borderWidth": 1,
    "color": "{{ line_color }}"
}
```

Then provide the variable in your Rust code:

```rust
let mut template_data = HashMap::new();
template_data.insert("line_color", "#e74c3c".to_string());
```

## Integration with Other Schemas

Lines work well with other PDForge schemas:

- **Text**: Use lines to underline or separate text sections
- **Tables**: Create custom table borders or separators
- **Images**: Frame images with line borders
- **QR Codes**: Add decorative lines around QR codes
- **SVG**: Combine with SVG elements for complex layouts

For more advanced usage and combinations, see the other example files in the `examples/` directory.