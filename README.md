# PDForge

A powerful and flexible PDF generation library written in Rust, inspired by [pdfme](https://pdfme.com/). PDForge allows you to create sophisticated PDFs from JSON templates with support for multiple content types including text, tables, images, QR codes, and SVG graphics.

## Features

- **Template-based PDF generation** - Define PDF layouts using JSON templates
- **Multiple content types** - Support for text, dynamic text, tables, images, QR codes, SVG graphics, and rectangles
- **Font management** - Easy font loading and management with support for Japanese fonts (Noto Sans JP, Noto Serif JP)
- **Dynamic content** - Template rendering with variable substitution using Tera templating engine
- **Multi-page support** - Generate PDFs with multiple pages from single templates
- **Flexible styling** - Comprehensive styling options for colors, borders, alignment, and spacing
- **High-quality output** - Built on the robust printpdf library

## Supported Schema Types

- **Text**: Static text with font styling and positioning
- **Dynamic Text**: Template-driven text with variable substitution
- **Table**: Structured tables with headers, borders, and cell styling
- **QR Code**: QR code generation with customizable size and positioning
- **Image**: Image embedding with PNG, JPEG, and BMP support
- **SVG**: Scalable vector graphics rendering
- **Rectangle**: Geometric shapes with customizable styling

## Installation

Add PDForge to your `Cargo.toml`:

```toml
[dependencies]
pdforge = "0.2.1"
```

## Quick Start

### Basic Usage

```rust
use pdforge::PDFme;

fn main() {
    // Create a new PDF generator
    let mut pdfme = PDFme::new("My Document".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf");

    // Load a template from JSON file
    pdfme.load_template("table-test", "./templates/table-test.json");

    // Generate PDF
    let bytes: Vec<u8> = pdfme.render("table-test");

    // Save to file
    std::fs::write("./output.pdf", bytes).unwrap();
}
```

### With Dynamic Data

```rust
use std::collections::HashMap;
use pdforge::PDFme;

fn main() {
    let mut pdfme = PDFme::new("Dynamic Document".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf");

    pdfme.load_template("pawn-ticket", "./templates/pawn-ticket.json");

    // Prepare input data
    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("name", "田中太郎".to_string());
    input.insert("date", "2025-05-24".to_string());
    input.insert("amount", "100,000".to_string());

    let inputs = vec![vec![input]];
    let bytes: Vec<u8> = pdfme.render_with_inputs("pawn-ticket", inputs);

    std::fs::write("./dynamic_output.pdf", bytes).unwrap();
}
```

## Template Structure

PDForge uses JSON templates to define PDF layouts. Here's the basic structure:

```json
{
  "schemas": [
    [
      {
        "type": "text",
        "name": "title",
        "content": "Hello World",
        "position": { "x": 10, "y": 10 },
        "width": 100,
        "height": 20,
        "fontSize": 16,
        "fontName": "NotoSansJP"
      }
    ]
  ],
  "basePdf": {
    "width": 210,
    "height": 297,
    "padding": [20, 10, 20, 10]
  },
  "pdfmeVersion": "5.3.8"
}
```

### Schema Types

#### Text Schema

```json
{
  "type": "text",
  "name": "field1",
  "content": "Static text content",
  "position": { "x": 10, "y": 10 },
  "width": 100,
  "height": 20,
  "fontSize": 12,
  "fontName": "NotoSansJP",
  "fontColor": "#000000",
  "alignment": "left"
}
```

#### Table Schema

```json
{
  "type": "table",
  "name": "data_table",
  "position": { "x": 10, "y": 50 },
  "width": 190,
  "height": 100,
  "content": "[[\"Name\",\"City\",\"Description\"],[\"Alice\",\"New York\",\"Web designer\"]]",
  "showHead": true,
  "headWidthPercentages": [
    { "content": "Name", "percent": 30, "fontSize": 12, "alignment": "center" },
    { "content": "City", "percent": 20, "fontSize": 12, "alignment": "center" },
    {
      "content": "Description",
      "percent": 50,
      "fontSize": 12,
      "alignment": "left"
    }
  ],
  "tableStyles": {
    "borderWidth": 0.3,
    "borderColor": "#000000"
  },
  "headStyles": {
    "fontSize": 12,
    "fontName": "NotoSansJP",
    "fontColor": "#ffffff",
    "backgroundColor": "#2980ba"
  }
}
```

#### QR Code Schema

```json
{
  "type": "qrCode",
  "name": "qr1",
  "content": "https://example.com",
  "position": { "x": 150, "y": 50 },
  "width": 30,
  "height": 30
}
```

#### SVG Schema

```json
{
  "type": "svg",
  "name": "logo",
  "content": "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\"><circle cx=\"50\" cy=\"50\" r=\"40\" fill=\"blue\"/></svg>",
  "position": { "x": 10, "y": 100 },
  "width": 50,
  "height": 50
}
```

## Examples

The project includes several examples in the `examples/` directory:

- **Basic Table**: `cargo run --example table`
- **SVG Graphics**: `cargo run --example svg`
- **PDFium Integration**: `cargo run --example pdfium`

## Font Support

PDForge includes comprehensive font support with a focus on Japanese typography:

### Included Fonts

- **Noto Sans JP**: Modern sans-serif Japanese font family
- **Noto Serif JP**: Traditional serif Japanese font family
- **Hannari Mincho**: Elegant Japanese mincho font
- **IPA Gothic**: Standard Japanese gothic font

### Adding Custom Fonts

```rust
let pdfme = PDFme::new("Document".to_string())
    .add_font("CustomFont", "./path/to/custom-font.ttf");
```

## Template Engine

PDForge uses the [Tera](https://tera.netlify.app/) templating engine for dynamic content rendering. This allows you to:

- Insert variables: `{{ variable_name }}`
- Use conditionals: `{% if condition %}`
- Apply filters: `{{ name | upper }}`
- Loop through data: `{% for item in items %}`

## Dependencies

PDForge is built on top of several excellent Rust crates:

- **printpdf**: Core PDF generation functionality
- **tera**: Template engine for dynamic content
- **serde**: JSON serialization/deserialization
- **qrcode**: QR code generation
- **image**: Image processing and embedding
- **uuid**: Unique identifier generation

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Examples

```bash
# Table example
cargo run --example table

# SVG example
cargo run --example svg

# PDFium example
cargo run --example pdfium
```

## Project Structure

```
pdforge/
├── src/
│   ├── lib.rs              # Main library interface
│   ├── font.rs             # Font management
│   ├── common.rs           # Common utilities
│   ├── utils.rs            # Helper functions
│   └── schemas/            # Schema implementations
│       ├── mod.rs          # Schema definitions
│       ├── text.rs         # Text schema
│       ├── table.rs        # Table schema
│       ├── qrcode.rs       # QR code schema
│       ├── image.rs        # Image schema
│       ├── svg.rs          # SVG schema
│       └── rect.rs         # Rectangle schema
├── examples/               # Usage examples
├── templates/              # JSON template files
├── assets/fonts/          # Font files
└── pdfium/                # PDFium integration
```

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Inspired by [pdfme](https://pdfme.com/) - A powerful PDF generation library
- Built with [printpdf](https://github.com/fschutt/printpdf) - A pure Rust PDF generation library
- Font rendering powered by industry-standard font libraries
