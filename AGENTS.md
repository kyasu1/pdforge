# Repository Guidelines

This document provides guidelines for contributing to the `pdforge` repository. Please follow these standards to maintain code quality and consistency.

## Project Structure & Module Organization

The project is organized as a Rust library with supporting assets and examples:

- `src/`: Core library source code.
  - `src/lib.rs`: Main entry point and `PDForgeBuilder`.
  - `src/schemas/`: Implementations of PDF elements (Text, Table, QR, etc.).
  - `src/font.rs`: Font management and loading.
- `tests/`: Integration tests for verifying core functionality and thread safety.
- `examples/`: Example binaries demonstrating various usage patterns.
- `templates/`: JSON templates used for rendering and testing.
- `assets/fonts/`: Required font files for PDF rendering.
- `docs/`: Architecture specifications and migration plans.

## Build, Test, and Development Commands

Use `cargo` for all build and test operations:

- `cargo build`: Compiles the project.
- `cargo test`: Runs all unit and integration tests.
- `cargo run --example <name>`: Runs a specific example from the `examples/` directory.
- `cargo run --bin pdforge <template_path>`: Tests a specific JSON template.

## Coding Style & Naming Conventions

- **Style**: Follow standard Rust idioms and `rustfmt` formatting.
- **Naming**: 
  - Types (Structs/Enums): `UpperCamelCase`.
  - Functions/Variables: `snake_case`.
- **Error Handling**: Use the `snafu` crate for structured and descriptive error management.
- **API Design**: Prefer the Builder pattern for complex object construction (e.g., `PDForgeBuilder`).

## Testing Guidelines

- **Framework**: Standard Rust test framework.
- **Coverage**: New features must include unit tests in `src/` and integration tests in `tests/`.
- **Naming**: Integration tests should be descriptive (e.g., `table_integration_tests.rs`).
- **Execution**: Use `cargo test -- --nocapture` to see output during failures.

## Commit & Pull Request Guidelines

- **Commit Messages**: Use conventional commit prefixes:
  - `feat:` New features.
  - `fix:` Bug fixes.
  - `docs:` Documentation changes.
  - `chore:` Maintenance and version bumps.
  - `refactor:` Code changes that neither fix a bug nor add a feature.
- **Pull Requests**:
  - Ensure all tests pass.
  - Provide a clear description of the change and link any relevant issues.
  - Include screenshots or PDF outputs for visual changes.
