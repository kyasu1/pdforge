# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PDForge is a Rust PDF generation library inspired by pdfme. It creates PDFs from JSON templates with support for multiple content types including text, tables, images, QR codes, and SVG graphics.

## Commands

### Building and Testing
```bash
# Build the project
cargo build

# Run tests
cargo test

# Build for release
cargo build --release
```

### Running Examples
```bash
# Run simple example (uses templates/table-test.json)
cargo run --bin pdforge

# Run with specific template file
cargo run --bin pdforge ./templates/table.json

# Run example files
cargo run --example simple
cargo run --example test-printpdf

# Run example with custom template
cargo run --example simple ./templates/tiger-svg.json
```

### Testing Individual Files
```bash
# Run specific test file
cargo test --test thread_safety

# Run with output
cargo test -- --nocapture
```

## Architecture

### Core Components

**PDForge Library Structure:**
- `lib.rs` - Main entry point with `PDForgeBuilder` pattern and `PDForge` struct
- `schemas/` - Contains all PDF element implementations (text, table, QR codes, images, SVG, rectangles, groups)
- `font.rs` - Font management and loading system
- `common.rs` - Shared utilities and common types
- `utils.rs` - Helper functions and utilities

**Key Design Patterns:**
- **Builder Pattern**: `PDForgeBuilder` for constructing PDF generators with fonts and templates
- **Schema System**: Each PDF element type implements the `SchemaTrait` for rendering
- **Template Engine**: Uses Tera templating for dynamic content substitution
- **Error Handling**: Uses Snafu for structured error management

### Schema Types

The library supports these PDF elements:
- `Text` - Static text with positioning and styling
- `DynamicText` - Template-driven text with variable substitution
- `Table` - Structured tables with headers and styling
- `QrCode` - QR code generation
- `Image` - PNG/JPEG/BMP image embedding
- `Svg` - Scalable vector graphics
- `Rectangle` - Geometric shapes
- `Group` - Container for grouping and transforming multiple elements

### Template System

**JSON Template Structure:**
- Templates define PDF layouts using JSON
- Each template contains `schemas` (page elements) and `basePdf` (page settings)
- Supports multi-page documents where each page is an array of schemas
- Dynamic content uses Tera templating engine with `{{ variable }}` syntax

**Key Files:**
- `templates/` - JSON template definitions
- `assets/fonts/` - Font files (primarily Japanese fonts: Noto Sans/Serif JP)
- `examples/pdf/` - Generated PDF output examples

### Rendering Flow

1. **Template Loading**: JSON templates parsed into `Template` struct
2. **Schema Conversion**: JSON schemas converted to typed `Schema` enum variants
3. **Dynamic Rendering**: Tera processes templates with input data
4. **PDF Generation**: Schemas render to `OpBuffer`, then converted to PDF via printpdf

### Font Management

- Fonts loaded via `PDForgeBuilder.add_font(name, path)`
- `FontMap` manages font references and metadata
- Default fonts include comprehensive Japanese typography support
- Font files stored in `assets/fonts/`

## Development Notes

### Binary vs Library Usage
- Main binary (`src/main.rs`) provides CLI interface for template processing
- Library interface (`src/lib.rs`) provides programmatic API via `PDForgeBuilder`
- Examples demonstrate both usage patterns

### Template Development
- JSON templates in `templates/` directory follow pdfme-compatible format
- Test templates with: `cargo run ./templates/your-template.json`
- Generated PDFs output to `examples/pdf/` directory

### Thread Safety
- See `tests/thread_safety.rs` for concurrent usage patterns
- Error types use thread-safe design patterns