# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.16.0] - 2026-07-28

### Fixed
- A table's `position.x` was silently raised to `basePdf.padding.left` and its available width was computed as `page width - padding.left - padding.right`, ignoring where the table actually starts. A table declared at `x: 10` on a page with `padding: [10, 10, 20, 20]` rendered at `x: 20` with its width cut from 190mm to 180mm. `position.x` is now honoured as the absolute coordinate it is, and the available width spans from `x` to `page width - padding.right`.

  This aligns the horizontal axis with what the rest of the library already did: every other schema type uses `position.x` directly, and a table's first page already used `position.y` directly.

  A table wider than the space remaining to its right no longer slides left to fit — its `width` is a maximum and is trimmed to the right boundary instead. Templates that relied on being pushed rightwards to `padding.left`, or on the leftward slide, will render differently.

### Changed
- `basePdf.padding` is documented per side rather than as a uniform page margin: `top`/`bottom` bound pagination, `right` bounds a table's maximum width, and `left` is not consulted by any schema. The field is retained for the 4-element array shape; it does not provide a default horizontal position.

## [0.15.0] - 2026-07-28

### Fixed
- `Text::get_height` wrapped text at the full `base.width` while `Text::render` wraps at `base.width` minus horizontal padding, so any padded element that wrapped was measured one or more lines too short (#39). Both now wrap at the same effective width. Table row heights are computed from `get_height`, so padded cells in narrow columns no longer draw text outside their cell rectangle, and page breaks — which compare the measured row height against the remaining space — are decided from the height that actually gets drawn.
- Dynamic font sizing (`fontSize: { min, max, fit }`) fitted text to the unpadded box for the same reason, picking a size that overflowed. It now sizes against the padded box.

### Added
- Table columns can now declare fixed and flexible widths, not just percentages. A column's `width` accepts `25` / `"25mm"` (fixed millimetres), `"20%"` (share of the table width), and `"2fr"` / `"fr"` (share of the leftover width, like CSS grid). Fixed and percent columns are placed first and the remainder is split between the `fr` columns by weight.
- Rows whose cell count does not match the column count are now rejected at parse time with the offending row index, instead of panicking mid-render.
- `templates/table-column-widths.json` demonstrates the mixed notations.

### Changed
- **BREAKING** — table column definitions are unified under `columns[]`. `headWidthPercentages` is removed; each `columns[]` entry now carries `width`, `header`, and `cell`. The `columns[].schema` wrapper is gone — the cell schema sits directly under `cell`.

  ```json
  // before
  "headWidthPercentages": [
    { "content": "Name", "percent": 70 },
    { "content": "Price", "percent": 30 }
  ],
  "columns": [
    { "schema": { "type": "text", "name": "name",  "...": "..." } },
    { "schema": { "type": "text", "name": "price", "...": "..." } }
  ]

  // after
  "columns": [
    {
      "width": "70%",
      "header": { "content": "Name" },
      "cell": { "type": "text", "name": "name", "...": "..." }
    },
    {
      "width": "30%",
      "header": { "content": "Price" },
      "cell": { "type": "text", "name": "price", "...": "..." }
    }
  ]
  ```

  A percentage `N` migrates to `"N%"` unchanged. `header` stays required even when `showHead` is `false` — it controls painting only, not the column shape.

- **BREAKING** — column widths no longer have to add up to 100%. The previous exact-sum check rejected legitimate layouts such as three columns of `33.33`. Widths that fall short leave the table narrower than its declared width; widths that overflow are scaled down to fit, with any `fr` columns collapsing to zero. Neither case is an error.
- Column widths are resolved in `f64` and converted to `f32` once at the end, so a table whose percentages sum to 100 no longer risks tipping into the overflow path on rounding alone.
- Column `width` values must be positive and no larger than `f32::MAX`. `0fr` divided by zero, a zero-width column wraps text to one grapheme cluster per line, and values that are finite individually can still sum to infinity during resolution — two `1e308fr` columns yielded `NaN` widths.
- Whitespace between a width's number and its suffix is rejected (`"20 %"`); only whitespace around the whole value is ignored.

## [0.14.0] - 2026-07-26

### Added
- `PDForgeBuilder::add_font_with_index` and `add_font_from_file_with_index` for selecting a specific face by index inside a TrueType/OpenType Collection (`.ttc`/`.otc`). `add_font`/`add_font_from_file` continue to load face 0 by default.

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
