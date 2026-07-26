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
cargo run --example font_from_bytes

# Run example with custom template
cargo run --example simple ./templates/tiger-svg.json
cargo run --example font_from_bytes ./templates/tiger-svg.json
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

PDForge provides four methods for loading fonts, all delegating down to `ParsedFont::from_bytes(bytes, font_index, warnings)` from `printpdf`:

1. **From byte slice** - `PDForgeBuilder.add_font(name, bytes)` **(Primary API)**
   - Accepts font data as `&[u8]`
   - Most flexible and efficient approach
   - Useful for embedded fonts, network-loaded fonts, or cached font data
   - Eliminates redundant disk I/O
   - Loads face index `0`; equivalent to `add_font_with_index(name, bytes, 0)`
   - See `examples/font_from_bytes.rs` for usage

2. **From file path** - `PDForgeBuilder.add_font_from_file(name, path)` **(Convenience wrapper)**
   - Convenient for loading fonts directly from disk
   - Internally reads the file and calls `add_font()`
   - Loads face index `0`; equivalent to `add_font_from_file_with_index(name, path, 0)`
   - See `examples/simple.rs` for usage

3. **From byte slice with face index** - `PDForgeBuilder.add_font_with_index(name, bytes, font_index)`
   - Selects a specific face inside a TrueType/OpenType Collection (`.ttc`/`.otc`)
   - Use `font_index` `0` for single-face `.ttf`/`.otf` fonts
   - To load multiple faces from the same collection, call once per face with a distinct `name` and index
   - An out-of-range `font_index` surfaces as `Error::FontParsing`

4. **From file path with face index** - `PDForgeBuilder.add_font_from_file_with_index(name, path, font_index)`
   - Same face-selection behavior as `add_font_with_index`, reading from disk

- `FontMap` manages font references and metadata
- Default fonts include comprehensive Japanese typography support
- Font files stored in `assets/fonts/`

**Design rationale**: `add_font()` accepts byte slices as the primary API because fonts are ultimately parsed from bytes. This eliminates redundant disk I/O and provides maximum flexibility for different font sources (embedded, network, cache, etc.). The `_with_index` variants exist as separate methods (rather than changing the signature of `add_font`/`add_font_from_file`) to keep the common single-face case ergonomic while remaining backward compatible.

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