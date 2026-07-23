# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.1] - 2026-07-23

### Changed
- Updated `printpdf` to 0.12.3. The new `XObjectTransform.no_auto_scale` field is set to `false` everywhere, preserving image/SVG placement behavior.

## [0.13.0] - 2026-07-23

### Added
- Table headers now honor `headStyles` styling that was previously parsed but ignored: `backgroundColor`, `fontColor`, `characterSpacing`, `lineHeight`, `borderColor`, and per-side `borderWidth` (`Frame`).
- Table cell borders now use `tableStyles.borderColor` (previously always black).
- Body cells now inherit unspecified `alignment`, `verticalAlignment`, `characterSpacing`, `lineHeight`, `fontColor`, `padding`, and `lineBreakMode` from `bodyStyles`.
- Empty (`""`) color strings in `headStyles` are treated as "unset" and fall back to defaults.

### Changed
- Table header background now comes from `headStyles.backgroundColor` instead of erroneously reusing `bodyStyles.backgroundColor`.
- Header borders now come from `headStyles.borderWidth`/`borderColor`; a `borderWidth` of `0` yields no header border (previously headers inherited `tableStyles.borderWidth`). See [docs/table-styling-migration.md](docs/table-styling-migration.md).

### Removed
- Removed unused schema fields with no rendering effect: `bodyStyles.fontSize`, `bodyStyles.fontName`, `bodyStyles.borderColor`, `bodyStyles.borderWidth`, and the column `CellStyle.height`. JSON parsing remains lenient (unknown fields are ignored).

## [0.12.0] - 2026-06-27

### Added
- Added a height-only `spacer` schema for explicit vertical gaps between flowing tables and dynamic text.

### Changed
- Tables, dynamic text, and spacers now share page and vertical-position flow state within each template page.

## [0.11.1] - 2026-06-12

### Changed
- Template variables are now rendered only inside JSON string values. This prevents user input from breaking or injecting JSON structure, but templates that relied on Tera expanding outside string values to generate JSON arrays or objects must be rewritten.

### Fixed
- Prevented repeated `PDForge::render()` calls from accumulating pages from previous renders.
- Escaped quotes, backslashes, and newlines in template input values safely.
- Fixed dynamic font-size growth to compare rendered height with the height constraint.
- Prevented empty `DynamicText` content from panicking.
- Replaced reachable unsupported-schema panics with structured errors.
- Removed the A4-width assumption from nested schema rendering.

## [0.11.0] - 2026-04-10

### Added
- Added `lineBreakMode` with `word` and `char` options for `text`, `dynamicText`, and table text cells
- Added grapheme-safe wrapping tests for emoji and combined characters

### Changed
- Japanese kinsoku processing is now always applied during text wrapping
- Table body text now defaults to `char` line breaking, while plain `text` and `dynamicText` keep `word` as the default
- `word` mode now falls back to grapheme clusters when a segment is too wide for the available width
- `DynamicText` now uses the same font-aware sanitization path as `Text`

### Fixed
- Prevented complex grapheme clusters such as family emoji, emoji modifiers, and variation selector sequences from being split mid-wrap
- Replaced unsupported grapheme clusters as a single fallback unit instead of partially degrading individual code points
- Fixed `tests/table_integration_tests.rs` to use a loaded test font instead of an empty `FontMap`

## [0.10.2] - 2026-03-13

### Changed
- Upgraded `printpdf` dependency from pinned git rev (`a88db12`) to published version `0.9.1`
- Added `svg` feature for `printpdf`, maintaining full SVG support via `svg2pdf 0.13.0`
- Upgraded `lopdf` from `0.37.0` to `0.39.0` to align with `printpdf 0.9.1`

### Internal
- `Op::SetFontSize` replaced with `Op::SetFont { font: PdfFontHandle::External(...), size }` (`src/schemas/pdf_utils.rs`)
- `Op::WriteText` replaced with `Op::ShowText` — font reference removed from text op per new printpdf API

## [0.10.1] - 2026-02-14

### Fixed
- `bodyStyles.lineHeight` not being applied to table body cells; column schema's `lineHeight` takes priority, with `bodyStyles.lineHeight` as fallback
- `Text::get_height()` not accounting for `lineHeight`, causing cell height to be smaller than rendered content
- Text vertical positioning now uses CSS half-leading model, distributing leading space equally above and below each line

### Changed
- Removed 12 temporary test/debug example files

## [0.7.0] - 2025-01-15

### Added
- **Text Border Support**: Text elements now support customizable borders
  - New `borderColor` field for border color in CSS format (hex, rgb, named colors)
  - New `borderWidth` field for border thickness in points
  - Borders work seamlessly with existing background colors
  - Both features are optional and backwards compatible

### Changed
- Enhanced text styling capabilities with border rendering
- Improved visual design options for text elements

### Examples
- Added `text-border-test.json` template demonstrating border functionality

## [0.6.0] - 2025-01-15

### Added
- Line schema implementation with comprehensive examples
- Enhanced line rendering capabilities

### Changed
- Updated project structure and documentation

## [0.5.0] - Previous Release

### Added
- Multi-page table support with automatic spanning
- Static schema support for headers, footers, and page elements
- Template variables (currentPage, totalPages, date, dateTime)
- Image object-fit support (fill, contain, cover, none, scale-down)
- Comprehensive font management with Japanese font support
- QR code generation
- SVG graphics support
- Group schema for transformations and grouping

### Changed
- Enhanced table rendering with proper pagination
- Improved template engine integration

### Fixed
- Table footer overlap issues
