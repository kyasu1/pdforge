# PDForge

A powerful and flexible PDF generation library written in Rust, inspired by [pdfme](https://pdfme.com/). PDForge allows you to create sophisticated PDFs from JSON templates with support for multiple content types including text, tables, images, QR codes, and SVG graphics.

## Features

- **Template-based PDF generation** - Define PDF layouts using JSON templates
- **Multiple content types** - Support for text, dynamic text, tables, images, QR codes, SVG graphics, and rectangles
- **Font management** - Easy font loading and management with support for Japanese fonts (Noto Sans JP, Noto Serif JP)
- **Dynamic content** - Template rendering with variable substitution using Tera templating engine
- **Multi-page support** - Generate PDFs with multiple pages from single templates
- **Advanced table rendering** - Tables can span multiple pages automatically with proper pagination
- **Flexible styling** - Comprehensive styling options for colors, borders, alignment, and spacing
- **High-quality output** - Built on the robust printpdf library

## Supported Schema Types

- **Text**: Static text with font styling and positioning
- **Dynamic Text**: Template-driven text with variable substitution
- **Table**: Structured tables with headers, borders, cell styling, and multi-page spanning
- **QR Code**: QR code generation with customizable size and positioning
- **Image**: Image embedding with PNG, JPEG, and BMP support, plus CSS-like object-fit controls
- **SVG**: Scalable vector graphics rendering
- **Rectangle**: Geometric shapes with customizable styling
- **Group**: Container for grouping and transforming multiple schemas together

## Installation

Add PDForge to your `Cargo.toml`:

```toml
[dependencies]
pdforge = "0.2.2"
```

## Quick Start

### Basic Usage

```rust
use pdforge::PDForgeBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new PDF generator using the builder pattern
    let mut pdforge = PDForgeBuilder::new("My Document".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("table-test", "./templates/table-test.json")?
        .build();

    // Generate PDF
    let bytes: Vec<u8> = pdforge.render("table-test")?;

    // Save to file
    std::fs::write("./output.pdf", bytes)?;
    Ok(())
}
```

### With Command Line Arguments

```rust
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <template_file>", args[0]);
        std::process::exit(1);
    }
    
    let template_file = &args[1];
    let template_path = Path::new(template_file);
    
    // Extract filename without extension for output
    let file_stem = template_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    
    let mut pdforge = pdforge::PDForgeBuilder::new("CLI Example".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("template", template_file)?
        .build();

    let bytes: Vec<u8> = pdforge.render("template")?;

    // Generate output filename based on input template
    let output_file = format!("{}.pdf", file_stem);
    std::fs::write(&output_file, bytes)?;
    
    println!("PDF generated: {}", output_file);
    Ok(())
}
```

### With Dynamic Data

```rust
use std::collections::HashMap;
use pdforge::PDForgeBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pdforge = PDForgeBuilder::new("Dynamic Document".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("pawn-ticket", "./templates/pawn-ticket.json")?
        .build();

    // Prepare input data
    let mut input: HashMap<&'static str, String> = HashMap::new();
    input.insert("name", "田中太郎".to_string());
    input.insert("date", "2025-05-24".to_string());
    input.insert("amount", "100,000".to_string());

    let inputs = vec![vec![input]];
    let bytes: Vec<u8> = pdforge.render_with_inputs("pawn-ticket", inputs)?;

    std::fs::write("./dynamic_output.pdf", bytes)?;
    Ok(())
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

**Multi-Page Tables**: Tables automatically span multiple pages when content exceeds the available space. The library handles pagination, headers, and proper content flow across pages automatically.

#### Image Schema

```json
{
  "type": "image",
  "name": "photo",
  "content": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQEAYABgAAD...",
  "position": { "x": 10, "y": 50 },
  "width": 80,
  "height": 60,
  "objectFit": "contain"
}
```

**Object-Fit Control**: The `objectFit` property controls how images are scaled and positioned within their containers, following CSS object-fit specifications:

- **`fill`** (default): Stretches the image to fill the container exactly, may distort aspect ratio
- **`contain`**: Scales image to fit within container while preserving aspect ratio, may leave empty space
- **`cover`**: Scales image to cover entire container while preserving aspect ratio, may crop parts of the image
- **`none`**: Displays image at natural size (1:1 pixel mapping), may overflow or leave empty space
- **`scale-down`**: Behaves like `none` if image fits naturally, otherwise behaves like `contain`

Examples:
```json
{
  "type": "image",
  "name": "logo",
  "content": "data:image/png;base64,...",
  "position": { "x": 10, "y": 10 },
  "width": 100,
  "height": 80,
  "objectFit": "contain"
}
```

```json
{
  "type": "image", 
  "name": "background",
  "content": "data:image/jpeg;base64,...",
  "position": { "x": 0, "y": 0 },
  "width": 210,
  "height": 150,
  "objectFit": "cover"
}
```

The object-fit feature provides precise control over image display, ensuring your images look exactly as intended regardless of their original dimensions or aspect ratios.

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

#### Group Schema

```json
{
  "type": "group",
  "name": "header_group",
  "position": { "x": 10, "y": 10 },
  "width": 190,
  "height": 50,
  "rotate": 15,
  "schemas": [
    {
      "type": "text",
      "name": "title",
      "content": "Grouped Title",
      "position": { "x": 0, "y": 0 },
      "width": 100,
      "height": 20,
      "fontSize": 16,
      "fontName": "NotoSansJP"
    },
    {
      "type": "qrCode",
      "name": "group_qr",
      "content": "https://example.com",
      "position": { "x": 120, "y": 0 },
      "width": 30,
      "height": 30
    }
  ]
}
```

The Group schema allows you to:
- **Group multiple elements**: Combine different schema types into a single container
- **Apply transformations**: Rotate the entire group around its center point
- **Coordinate positioning**: Position child elements relative to the group's origin
- **Maintain hierarchy**: Organize complex layouts with nested structures

## Examples

The project includes several examples and templates:

### Running Examples

Use the `simple` example to generate PDFs from any template:

```bash
# Generate PDF from template
cargo run --example simple templates/table.json

# Multi-page table examples
cargo run --example simple templates/large-tables-spanning.json
cargo run --example simple templates/multi-table-fixed.json

# Image examples
cargo run --example image assets/images/test-image-1.jpg assets/images/test-image-2.jpg
cargo run --example object_fit_test

# Other examples  
cargo run --example simple templates/multipage.json
cargo run --example simple templates/tiger-svg.json
```

### Available Templates

- **`table.json`** - Basic table example
- **`large-tables-spanning.json`** - Comprehensive 4-page document with two large tables spanning multiple pages
  - Financial report table (76 rows) across pages 1-2
  - Inventory management table (57 rows) across pages 3-4
- **`multi-table-fixed.json`** - 3-page document with multiple separate tables
- **`multipage.json`** - Multi-page document example
- **`tiger-svg.json`** - SVG graphics example
- **`svg.json`** - Basic SVG example
- **`image-test.json`** - Image embedding with object-fit controls
- **`object-fit-test.json`** - Demonstrates all five object-fit modes side by side

### Generating PDFs

All PDF files are generated locally and are not tracked in the repository. To create a PDF:

```bash
cargo run --example simple <template-file>
```

This will create a PDF file in the `examples/pdf/` directory with the same name as your template.

## Font Support

PDForge includes comprehensive font support with a focus on Japanese typography:

### Included Fonts

- **Noto Sans JP**: Modern sans-serif Japanese font family
- **Noto Serif JP**: Traditional serif Japanese font family
- **Hannari Mincho**: Elegant Japanese mincho font
- **IPA Gothic**: Standard Japanese gothic font

### Adding Custom Fonts

```rust
let pdforge = PDForgeBuilder::new("Document".to_string())
    .add_font("CustomFont", "./path/to/custom-font.ttf")?
    .build();
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
# Generate PDFs from templates
cargo run --example simple templates/table.json
cargo run --example simple templates/large-tables-spanning.json
cargo run --example simple templates/multi-table-fixed.json

# Legacy examples (may require specific setup)
cargo run --example table
cargo run --example svg
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
│       ├── table.rs        # Table schema with multi-page support
│       ├── qrcode.rs       # QR code schema
│       ├── image.rs        # Image schema
│       ├── svg.rs          # SVG schema
│       └── rect.rs         # Rectangle schema
├── examples/               # Usage examples and CLI tools
│   ├── simple.rs          # Main CLI for template processing
│   └── ...                # Other specialized examples
├── templates/              # JSON template files
│   ├── large-tables-spanning.json  # Comprehensive multi-page tables
│   ├── multi-table-fixed.json      # Multiple tables example
│   └── ...                # Other template examples
├── examples/pdf/          # Generated PDF output (gitignored)
├── assets/fonts/          # Font files (gitignored)
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
