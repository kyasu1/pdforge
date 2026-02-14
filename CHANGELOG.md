# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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