# Project Issues Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the high-priority correctness and panic issues reported in `docs/project-issues.md`, then add regression coverage so the same failures stay fixed.

**Architecture:** Keep the first pass narrowly scoped to behavior-preserving bug fixes in the existing public API. Make rendering stateless with respect to `PdfDocument`, move template substitution away from whole-JSON string rendering, replace reachable panics with `Error` values, and add integration/unit tests for each failure mode.

**Tech Stack:** Rust 2021, `printpdf` 0.9.1, `snafu`, `serde_json`, `tera`, `lopdf`, standard Rust test framework.

---

## Scope

This plan covers the issues that should be fixed first:

- `1.1` repeated `PDForge::render()` appends old pages.
- `1.5` Tera substitution can break JSON or inject fields.
- `1.2` dynamic font growth compares height with width.
- `1.3` empty `DynamicText` can underflow.
- `1.4` reachable `unimplemented!()` / `todo!()` panics.
- `1.6` nested render paths hard-code A4 width.
- `6.4` add rendering-oriented regression tests for the above.

The remaining medium/low priority items from `docs/project-issues.md` are planned as follow-up phases at the end of this file, because some of them are API-breaking or broader feature work.

## File Map

- Modify `src/lib.rs`
  - Store the document name instead of a long-lived mutable `PdfDocument`.
  - Create a fresh `PdfDocument` per `render()` call and register loaded fonts into it.
  - Keep existing public method signatures in this phase.

- Modify `src/font.rs`
  - Store enough font data in `FontMap` or expose a helper so fonts can be registered into a fresh document.
  - Fix the dynamic font-size height comparison.
  - Add focused unit coverage for the grow path.

- Modify `src/schemas/mod.rs`
  - Add specific error variants for template rendering, unsupported schemas, and input mismatch.
  - Replace whole-JSON Tera rendering with recursive JSON value rendering for string fields.
  - Thread parent width through nested schema rendering.
  - Keep existing template parsing behavior for compatibility.

- Modify `src/schemas/dynamic_text.rs`
  - Return a stable result for empty content instead of computing `pages.len() - 1`.
  - Add a unit test that renders empty content.

- Modify `src/schemas/table.rs`
  - Replace `unimplemented!()` with a structured `Error`.
  - Add a unit test for unsupported column schema.

- Modify `src/schemas/group.rs`
  - Remove or replace the public `TryFrom<JsonGroupSchema> for Schema` panic path.
  - Pass group width to nested schema rendering.

- Create `tests/render_regression_tests.rs`
  - Add end-to-end rendering regressions using a real font and `lopdf` page-count inspection.
  - Cover repeated renders and JSON-safe template substitution.

---

## Task 1: Rendering Regression Test Harness

**Files:**

- Create: `tests/render_regression_tests.rs`
- Read: `assets/fonts/NotoSansJP-Regular.ttf`

- [ ] **Step 1: Write shared test helpers**

Create `tests/render_regression_tests.rs` with this starting content:

```rust
use lopdf::Document;
use pdforge::PDForgeBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("fonts")
        .join("NotoSansJP-Regular.ttf")
}

fn write_template(name: &str, json: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "pdforge-{}-{}.json",
        name,
        std::process::id()
    ));
    std::fs::write(&path, json).expect("template should be writable");
    path
}

fn page_count(pdf_bytes: &[u8]) -> usize {
    let doc = Document::load_mem(pdf_bytes).expect("rendered PDF should parse");
    doc.get_pages().len()
}

fn single_text_template(content: &str) -> String {
    serde_json::json!({
        "schemaVersion": "1.0",
        "base_pdf": {
            "width": 210.0,
            "height": 297.0,
            "padding": [0.0, 0.0, 0.0, 0.0],
            "staticSchema": []
        },
        "schemas": [[{
            "type": "text",
            "name": "message",
            "position": { "x": 10.0, "y": 10.0 },
            "width": 180.0,
            "height": 20.0,
            "content": content,
            "fontName": "TestFont",
            "fontSize": 12.0,
            "alignment": "left",
            "verticalAlignment": "top",
            "characterSpacing": 0.0,
            "lineHeight": 1.0,
            "fontColor": "#000000"
        }]]
    })
    .to_string()
}

fn builder_for_template(template_path: &Path) -> pdforge::PDForge {
    PDForgeBuilder::new("regression".to_string())
        .add_font_from_file("TestFont", font_path().to_str().unwrap())
        .expect("test font should load")
        .load_template("main", template_path.to_str().unwrap())
        .expect("template should load")
        .build()
}
```

- [ ] **Step 2: Run the new empty test file**

Run:

```bash
cargo test --test render_regression_tests -- --nocapture
```

Expected: PASS with zero tests, or PASS after compiling the helper module.

---

## Task 2: Fix Repeated `render()` Page Accumulation

**Files:**

- Modify: `tests/render_regression_tests.rs`
- Modify: `src/lib.rs`
- Modify: `src/font.rs`

- [ ] **Step 1: Write the failing regression test**

Append this test to `tests/render_regression_tests.rs`:

```rust
#[test]
fn repeated_render_calls_do_not_accumulate_pages() {
    let template_path = write_template("repeated-render", &single_text_template("{{ name }}"));
    let mut forge = builder_for_template(&template_path);

    let mut first_input = HashMap::new();
    first_input.insert("name", "first".to_string());

    let mut second_input = HashMap::new();
    second_input.insert("name", "second".to_string());

    let first = forge
        .render("main", vec![vec![first_input]], None, None)
        .expect("first render should succeed");
    let second = forge
        .render("main", vec![vec![second_input]], None, None)
        .expect("second render should succeed");

    assert_eq!(page_count(&first), 1);
    assert_eq!(page_count(&second), 1);
}
```

- [ ] **Step 2: Run the test and verify it fails before the fix**

Run:

```bash
cargo test --test render_regression_tests repeated_render_calls_do_not_accumulate_pages -- --nocapture
```

Expected before the fix: FAIL because `page_count(&second)` is `2`.

- [ ] **Step 3: Change `FontMap` so fonts can be re-registered**

In `src/font.rs`, ensure the stored font value is `Arc<ParsedFont>` and add a method like this to `FontMap`:

```rust
impl FontMap {
    pub fn register_fonts_for_document(&self, doc: &mut printpdf::PdfDocument) -> FontMap {
        let mut registered = FontMap::default();
        for (name, (_, parsed_font)) in &self.0 {
            let font_id = doc.add_font(parsed_font.as_ref());
            registered.add_font(name.clone(), font_id, parsed_font.as_ref());
        }
        registered
    }
}
```

If the tuple fields are not public inside the impl, use the actual private field names already defined in `FontMap`.

- [ ] **Step 4: Store document name instead of a mutable document in `PDForge`**

Update `src/lib.rs` so the structs look like this:

```rust
#[derive(Debug, Clone)]
pub struct PDForge {
    name: String,
    font_map: font::FontMap,
    template_map: HashMap<String, schemas::Template>,
}

pub struct PDForgeBuilder {
    name: String,
    doc: PdfDocument,
    font_map: font::FontMap,
    template_map: HashMap<String, schemas::Template>,
}
```

Update `PDForgeBuilder::new`:

```rust
pub fn new(name: String) -> Self {
    PDForgeBuilder {
        doc: PdfDocument::new(&name),
        name,
        font_map: font::FontMap::default(),
        template_map: HashMap::new(),
    }
}
```

Update `PDForgeBuilder::build`:

```rust
pub fn build(self) -> PDForge {
    PDForge {
        name: self.name,
        font_map: self.font_map,
        template_map: self.template_map,
    }
}
```

- [ ] **Step 5: Create a fresh document inside `PDForge::render`**

In `src/lib.rs`, change the success path in `PDForge::render` to:

```rust
Some(template) => {
    let mut doc = PdfDocument::new(&self.name);
    let font_map = self.font_map.register_fonts_for_document(&mut doc);
    template.render_with_inputs_table_data_and_static_inputs(
        &mut doc,
        &font_map,
        inputs,
        table_data,
        static_inputs,
    )
}
```

- [ ] **Step 6: Run the focused regression**

Run:

```bash
cargo test --test render_regression_tests repeated_render_calls_do_not_accumulate_pages -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/lib.rs src/font.rs tests/render_regression_tests.rs
git commit -m "fix: avoid render page accumulation"
```

---

## Task 3: Make Template Substitution JSON-Safe

**Files:**

- Modify: `tests/render_regression_tests.rs`
- Modify: `src/schemas/mod.rs`

- [ ] **Step 1: Write the failing regression test**

Append this test to `tests/render_regression_tests.rs`:

```rust
#[test]
fn template_inputs_with_quotes_backslashes_and_newlines_render_safely() {
    let template_path = write_template("escaped-input", &single_text_template("{{ value }}"));
    let mut forge = builder_for_template(&template_path);

    let mut input = HashMap::new();
    input.insert(
        "value",
        "quote: \" backslash: \\ newline:\n end".to_string(),
    );

    let pdf = forge
        .render("main", vec![vec![input]], None, None)
        .expect("escaped input should not break JSON parsing");

    assert_eq!(page_count(&pdf), 1);
}
```

- [ ] **Step 2: Run the test and verify it fails before the fix**

Run:

```bash
cargo test --test render_regression_tests template_inputs_with_quotes_backslashes_and_newlines_render_safely -- --nocapture
```

Expected before the fix: FAIL with `Failed to parse rendered template`.

- [ ] **Step 3: Add recursive string-field rendering helpers**

In `src/schemas/mod.rs`, add private helpers on `Template`:

```rust
fn render_json_value_strings(
    value: &serde_json::Value,
    context: &tera::Context,
) -> Result<serde_json::Value, Error> {
    match value {
        serde_json::Value::String(raw) => {
            let rendered = tera::Tera::one_off(raw, context, false).map_err(|e| {
                Error::Whatever {
                    message: "Failed to render template string".to_string(),
                    source: Some(Box::new(e)),
                }
            })?;
            Ok(serde_json::Value::String(rendered))
        }
        serde_json::Value::Array(items) => items
            .iter()
            .map(|item| Self::render_json_value_strings(item, context))
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array),
        serde_json::Value::Object(map) => {
            let rendered = map
                .iter()
                .map(|(key, value)| {
                    Ok((key.clone(), Self::render_json_value_strings(value, context)?))
                })
                .collect::<Result<serde_json::Map<String, serde_json::Value>, Error>>()?;
            Ok(serde_json::Value::Object(rendered))
        }
        _ => Ok(value.clone()),
    }
}

fn render_schema_json_values(
    values: &[serde_json::Value],
    context: &tera::Context,
) -> Result<Vec<JsonSchema>, Error> {
    values
        .iter()
        .map(|value| {
            let rendered = Self::render_json_value_strings(value, context)?;
            serde_json::from_value(rendered).map_err(|e| Error::TemplateDeserialize {
                source: e,
                message: "Failed to parse rendered template".to_string(),
            })
        })
        .collect()
}
```

- [ ] **Step 4: Replace whole-JSON Tera rendering in dynamic schemas**

In `Template::render_with_inputs_table_data_and_static_inputs`, replace the `serde_json::to_string`, `Tera::default`, `add_raw_template`, and `tera.render("schema_template", ...)` path with:

```rust
let mut context = tera::Context::new();
for (key, value) in input.iter() {
    context.insert(*key, value);
}

let mut parsed = Self::render_schema_json_values(&self.schemas[index], &context)?;
```

Keep the table-data injection loop immediately after `parsed` is created.

- [ ] **Step 5: Apply the same rendering helper to static schemas**

In `Template::render_static_schemas_for_page`, replace the stringification and reparse path for `self.static_schema_json` with:

```rust
let context = Self::create_special_context(current_page, total_pages, static_inputs);
let json_schemas = Self::render_schema_json_values(&self.static_schema_json, &context)?;
```

Then reuse the existing `JsonSchema` to `Schema` conversion logic.

- [ ] **Step 6: Run the focused regression**

Run:

```bash
cargo test --test render_regression_tests template_inputs_with_quotes_backslashes_and_newlines_render_safely -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Run repeated render regression again**

Run:

```bash
cargo test --test render_regression_tests -- --nocapture
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/schemas/mod.rs tests/render_regression_tests.rs
git commit -m "fix: render template strings without breaking json"
```

---

## Task 4: Fix Dynamic Font Growth Height Check

**Files:**

- Modify: `src/font.rs`

- [ ] **Step 1: Add a unit test for vertical fit growth**

In `src/font.rs` inside `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn dynamic_font_growth_respects_height_limit() {
    use super::{DynamicFontSize, FitMode};
    use printpdf::Mm;

    let spec = test_font_spec(LineBreakMode::Word);
    let dynamic = DynamicFontSize {
        min: Some(Pt(8.0)),
        max: Some(Pt(40.0)),
        fit: FitMode::Vertical,
    };

    let size = spec
        .calculate_dynamic_font_size(
            &dynamic,
            Pt(1.0),
            Pt(0.0),
            Mm(180.0),
            Mm(12.0),
            "short",
        )
        .expect("dynamic font calculation should succeed");

    assert!(size.0 <= 12.0, "size should not grow beyond the 12mm height");
}
```

If `FitMode` is not public in the test module path, import it from the module where it is defined.

- [ ] **Step 2: Run the focused test**

Run:

```bash
cargo test font::tests::dynamic_font_growth_respects_height_limit -- --nocapture
```

Expected before the fix: FAIL because the size grows too far when width is larger than height.

- [ ] **Step 3: Fix the comparison**

In `src/font.rs`, change:

```rust
if new_total_height_in_mm < width {
```

to:

```rust
if new_total_height_in_mm < height {
```

- [ ] **Step 4: Run the focused test**

Run:

```bash
cargo test font::tests::dynamic_font_growth_respects_height_limit -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/font.rs
git commit -m "fix: compare dynamic font height with height"
```

---

## Task 5: Handle Empty `DynamicText`

**Files:**

- Modify: `src/schemas/dynamic_text.rs`

- [ ] **Step 1: Add a unit test for empty content**

In `src/schemas/dynamic_text.rs` inside its `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn render_empty_dynamic_text_does_not_panic() {
    let base_pdf = create_test_base_pdf();
    let mut text = create_test_dynamic_text("");
    let mut buffer = OpBuffer::default();

    let result = text.render(&base_pdf, 0, None, &mut buffer);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (0, Some(text.base.y)));
    assert!(buffer.buffer.is_empty());
}
```

If the existing test helpers use different names, use the helper that constructs a `BasePdf` and `DynamicText` with the test font already used in that module.

- [ ] **Step 2: Run the focused test**

Run:

```bash
cargo test dynamic_text::tests::render_empty_dynamic_text_does_not_panic -- --nocapture
```

Expected before the fix: FAIL or panic with subtraction overflow.

- [ ] **Step 3: Return early when no lines are produced**

In `DynamicText::render`, after `lines` is computed and before `pages` is used, add:

```rust
if lines.is_empty() {
    return Ok((current_page, Some(y_top_mm)));
}
```

Keep the existing rendering behavior for non-empty content.

- [ ] **Step 4: Run the focused test**

Run:

```bash
cargo test dynamic_text::tests::render_empty_dynamic_text_does_not_panic -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/schemas/dynamic_text.rs
git commit -m "fix: handle empty dynamic text"
```

---

## Task 6: Replace Reachable Panics With Errors

**Files:**

- Modify: `src/schemas/mod.rs`
- Modify: `src/schemas/table.rs`
- Modify: `src/schemas/group.rs`

- [ ] **Step 1: Add an unsupported schema error variant**

In `src/schemas/mod.rs`, add this variant to `Error`:

```rust
#[snafu(display("Unsupported schema type in {context}: {schema_type}"))]
UnsupportedSchema {
    context: String,
    schema_type: String,
},
```

- [ ] **Step 2: Replace `table.rs` `unimplemented!()`**

In `src/schemas/table.rs`, change the wildcard arm in the column rendering match to:

```rust
_ => {
    return Err(Error::UnsupportedSchema {
        context: "table column".to_string(),
        schema_type: format!("{:?}", col),
    });
}
```

Use the existing matched variable name if it is not `col`.

- [ ] **Step 3: Replace `group.rs` `todo!()`**

In `src/schemas/group.rs`, change `TryFrom<JsonGroupSchema> for Schema` to return an error:

```rust
fn try_from(_json: JsonGroupSchema) -> Result<Self, Self::Error> {
    Err(Error::UnsupportedSchema {
        context: "JsonGroupSchema TryFrom without FontMap".to_string(),
        schema_type: "Group".to_string(),
    })
}
```

- [ ] **Step 4: Add a regression test for the group conversion path**

In `src/schemas/group.rs` inside `#[cfg(test)] mod tests`, add a test that deserializes a minimal group JSON and asserts that `Schema::try_from(json)` returns `Error::UnsupportedSchema` instead of panicking:

```rust
#[test]
fn try_from_json_group_returns_error_instead_of_panicking() {
    let json: JsonGroupSchema = serde_json::from_value(serde_json::json!({
        "name": "group",
        "position": { "x": 0.0, "y": 0.0 },
        "width": 100.0,
        "height": 100.0,
        "schemas": []
    }))
    .expect("group json should deserialize");

    let result = Schema::try_from(json);

    assert!(matches!(result, Err(Error::UnsupportedSchema { .. })));
}
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test group::tests::try_from_json_group_returns_error_instead_of_panicking -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/schemas/mod.rs src/schemas/table.rs src/schemas/group.rs
git commit -m "fix: return errors for unsupported schemas"
```

---

## Task 7: Remove A4 Width Assumption From Nested Rendering

**Files:**

- Modify: `src/schemas/mod.rs`
- Modify: `src/schemas/group.rs`

- [ ] **Step 1: Change the schema render trait to accept parent width**

In `src/schemas/mod.rs`, change `SchemaTrait::render` from:

```rust
fn render(
    &self,
    parent_height: Mm,
    doc: &mut PdfDocument,
    page: usize,
    buffer: &mut OpBuffer,
) -> Result<(), Error>;
```

to:

```rust
fn render(
    &self,
    parent_width: Mm,
    parent_height: Mm,
    doc: &mut PdfDocument,
    page: usize,
    buffer: &mut OpBuffer,
) -> Result<(), Error>;
```

- [ ] **Step 2: Update the `Schema` implementation**

In the `impl SchemaTrait for Schema`, update the method signature and replace each `BasePdf { width: Mm(210.0), ... }` with:

```rust
BasePdf {
    width: parent_width,
    height: parent_height,
    padding: Frame {
        top: Mm(0.0),
        right: Mm(0.0),
        bottom: Mm(0.0),
        left: Mm(0.0),
    },
    static_schema: vec![],
}
```

- [ ] **Step 3: Update call sites**

Update every direct `schema.render(...)` call to pass width and height:

```rust
schema.render(self.base.width, self.base.height, doc, page, buffer)?;
```

for group children, and:

```rust
schema.render(self.base_pdf.width, self.base_pdf.height, doc, page_index, &mut buffer)?;
```

for top-level template render paths.

- [ ] **Step 4: Update integration test mock**

In `tests/schema_traits_tests.rs`, update the mock implementation signature:

```rust
fn render(
    &self,
    _parent_width: Mm,
    _parent_height: Mm,
    _doc: &mut PdfDocument,
    _page: usize,
    _buffer: &mut OpBuffer,
) -> Result<(), pdforge::schemas::Error> {
    Ok(())
}
```

Update the `render_all` helper call to pass `Mm(210.0), Mm(297.0)`.

- [ ] **Step 5: Run trait and render regressions**

Run:

```bash
cargo test --test schema_traits_tests -- --nocapture
cargo test --test render_regression_tests -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/schemas/mod.rs src/schemas/group.rs tests/schema_traits_tests.rs
git commit -m "fix: use parent width for nested schemas"
```

---

## Task 8: Full Verification Pass

**Files:**

- Read all modified files.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --check
```

Expected: PASS. If it fails, run `cargo fmt`, inspect the diff, then rerun `cargo fmt --check`.

- [ ] **Step 2: Run all tests**

Run:

```bash
cargo test -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run clippy**

Run:

```bash
cargo clippy --all-targets
```

Expected: No new warnings caused by the files changed in this plan. Existing warnings from unrelated items in `docs/project-issues.md` may remain and should be recorded in the final notes.

- [ ] **Step 4: Inspect the final diff**

Run:

```bash
git diff --stat
git diff
```

Expected: Diff only contains the planned bug fixes and tests.

- [ ] **Step 5: Commit verification cleanup**

If formatting produced changes after the previous commits:

```bash
git add src tests
git commit -m "chore: format project issue fixes"
```

If formatting did not produce changes, do not create an empty commit.

---

## Follow-Up Phase A: API and Error Design

These changes are useful but may be API-breaking, so do them after the high-priority bug fixes have shipped.

- Change render input maps from `HashMap<&'static str, String>` to `HashMap<String, String>` or `HashMap<Cow<'_, str>, String>`.
- Add specific `Error` variants for empty inputs, template not found, input/page length mismatch, and template rendering failures.
- Add `PDForgeBuilder::load_template_from_str(template_name, json)` and rename the current second argument documentation from `template` to `path`.
- Validate `schemaVersion` in `Template::new` and `load_template_from_str`.

Recommended tests:

```bash
cargo test render_accepts_owned_input_keys -- --nocapture
cargo test load_template_from_str_loads_valid_template -- --nocapture
cargo test unsupported_schema_version_returns_error -- --nocapture
```

Recommended commits:

```bash
git commit -m "refactor: use owned render input keys"
git commit -m "feat: load templates from strings"
git commit -m "fix: validate schema versions"
```

## Follow-Up Phase B: Specification Gaps

Implement JSON fields that are currently deserialized but ignored, or remove them from the documented supported schema.

- `src/schemas/rect.rs`: apply `opacity`.
- `src/schemas/table.rs`: apply `borderColor`, `borderWidth`, `characterSpacing`, and style `fontColor`.
- `src/schemas/qrcode.rs`, `src/schemas/image.rs`, and related schema types: apply `rotate` where the spec promises it.
- `src/schemas/dynamic_text.rs`: add `fontColor`, `alignment`, and background support to narrow the behavior gap with `Text`.
- `src/font.rs`: either revalidate line widths after Japanese kinsoku adjustment or document the overflow tradeoff in `docs/schema-spec.md`.

Recommended verification:

```bash
cargo test -- --nocapture
cargo run --example table
```

For visual changes, generate before/after PDFs and attach them to the PR.

## Follow-Up Phase C: Performance and Maintenance

Do these after correctness work, because they are easier to review when behavior is already pinned down by tests.

- Deduplicate `JsonSchema` to `Schema` conversion in `src/schemas/mod.rs` and `src/schemas/group.rs`.
- Remove library `println!` calls or replace them with `log` / `tracing`.
- Remove empty `src/common.rs` or add real shared utilities.
- Reduce rendering-time clones in `SchemaTrait::render` and static schema rendering.
- Cache or precompile reusable template rendering state where it does not compromise JSON-safe substitution.
- Optimize text wrapping by tracking cumulative cluster width.
- Add rustdoc for public APIs in `src/lib.rs`.

Recommended verification:

```bash
cargo fmt --check
cargo test -- --nocapture
cargo clippy --all-targets
```

## Follow-Up Phase D: Project Hygiene

- Fix README and CLAUDE.md references to missing `src/main.rs`, `cargo run --bin pdforge`, and `templates/table-test.json`.
- Add `LICENSE`, `description`, `repository`, and `license` metadata to `Cargo.toml`.
- Add CI for `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets`.
- Decide whether `examples/pdf/` generated files should be committed; if not, add them to `.gitignore`.

Recommended commits:

```bash
git commit -m "docs: align readme with library usage"
git commit -m "chore: add package metadata"
git commit -m "chore: add rust ci"
```

