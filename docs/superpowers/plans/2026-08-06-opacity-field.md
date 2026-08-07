# Opacity Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the existing-but-unused `opacity` field in the `Rectangle` and `Line` schemas into PDF rendering, so an element with `opacity` < 1 renders translucent.

**Architecture:** PDF transparency is expressed through an Extended Graphics State (ExtGState) dictionary carrying the stroking (`CA`) and non-stroking (`ca`) alpha constants. In printpdf 0.12.3 that is `ExtendedGraphicsState::default().with_current_fill_alpha(a).with_current_stroke_alpha(a)` registered via `PdfDocument::add_graphics_state` → `ExtendedGraphicsStateId`, applied with `Op::LoadGraphicsState { gs }` and scoped by `SaveGraphicsState`/`RestoreGraphicsState` so the alpha does not leak to later elements. We add one shared helper in `pdf_utils.rs` that wraps an existing `Vec<Op>` with this, then call it from `Rect::render` and `Line::render` (both already receive `doc: &mut PdfDocument`, currently ignored as `_doc`). `None` (field absent) or alpha >= 1.0 is a no-op: the generated ops are returned unchanged and no ExtGState is registered, so existing templates render exactly as before. (Note: we do not claim byte-for-byte identical output — the PDF carries metadata/generation IDs outside these ops — only that the ops for existing templates are unchanged.)

**Tech Stack:** Rust, printpdf 0.12.3 (`Op`, `ExtendedGraphicsState`, `PdfDocument`), serde JSON templates, existing `OpBuffer` render pipeline.

## Global Constraints

- The `opacity` field already exists as `Option<f32>` on `JsonRectSchema`/`Rect` (`src/schemas/rect.rs:40,50,73`) and `JsonLineSchema`/`Line` (`src/schemas/line.rs:38,47,68`); do NOT re-add it — only start reading it.
- Keep `draw_rectangle` (`pdf_utils.rs:168`) and `draw_line` (`line.rs:147`) as pure `fn(...) -> Vec<Op>` — do not add `doc` parameters or opacity to their `DrawRectangle`/`DrawLine` structs. The shared helper wraps their output instead.
- Alpha value: `0.0` = fully transparent, `1.0` = fully opaque. Clamp to `[0.0, 1.0]`. `None` or alpha >= 1.0 must return the input ops unchanged and register no ExtGState.
- Apply the same alpha to both fill (`ca`) and stroke (`CA`) — "opacity" means the whole element is uniformly translucent.
- Each element with alpha < 1.0 registers its own ExtGState; many translucent elements grow the PDF resource dictionary. Deduping/caching identical ExtGStates is a future optimization and out of scope here.
- Resolves 2 of the 7 build warnings ("field `opacity` is never read" for `Rect` and `Line`); the other 5 warnings are out of scope for this plan.
- Every task must keep the full test suite green: `cargo test` (currently 156 passing).

---

### Task 1: Shared `wrap_ops_with_opacity` helper in `pdf_utils.rs`

**Files:**
- Modify: `src/schemas/pdf_utils.rs` (add helper after `draw_rectangle`, add unit tests in the existing `mod tests` block)
- Test: `src/schemas/pdf_utils.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces: `pub fn wrap_ops_with_opacity(doc: &mut PdfDocument, alpha: Option<f32>, ops: Vec<Op>) -> Vec<Op>` — used by Tasks 2 and 3. When `alpha` is `Some(a)` with `a < 1.0`, registers an ExtGState with fill+stroke alpha `= a.clamp(0.0, 1.0)` and returns `[SaveGraphicsState, LoadGraphicsState{gs}, ...ops, RestoreGraphicsState]`; otherwise returns `ops` unchanged.

- [ ] **Step 1: Write the failing unit tests** (append to `mod tests` in `src/schemas/pdf_utils.rs`)

```rust
#[test]
fn wrap_ops_with_opacity_registers_extgstate_and_wraps() {
    let mut doc = PdfDocument::new("opacity_test");
    let inner = vec![Op::SetFillColor {
        col: Color::Rgb(Rgb {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            icc_profile: None,
        }),
    }];

    let wrapped = wrap_ops_with_opacity(&mut doc, Some(0.5), inner.clone());

    // Exact structure: [q, LoadGraphicsState, ...inner, Q].
    assert_eq!(wrapped.len(), inner.len() + 3);
    assert!(matches!(wrapped[0], Op::SaveGraphicsState));
    assert!(matches!(wrapped[1], Op::LoadGraphicsState { .. }));
    assert_eq!(wrapped[2], inner[0]);
    assert!(matches!(wrapped.last(), Some(Op::RestoreGraphicsState)));
    // Exactly one LoadGraphicsState, scoped between the save/restore.
    assert_eq!(
        wrapped
            .iter()
            .filter(|op| matches!(op, Op::LoadGraphicsState { .. }))
            .count(),
        1
    );

    // The ExtGState is registered once, with both alpha constants set to 0.5.
    assert_eq!(doc.resources.extgstates.map.len(), 1);
    for gs in doc.resources.extgstates.map.values() {
        assert!((gs.current_fill_alpha() - 0.5).abs() < 1e-6);
        assert!((gs.current_stroke_alpha() - 0.5).abs() < 1e-6);
    }
}

#[test]
fn wrap_ops_with_opacity_is_noop_without_alpha_or_full_alpha() {
    let inner = vec![Op::SaveGraphicsState, Op::RestoreGraphicsState];

    // None (absent) and any alpha that clamps to >= 1.0 are no-ops: ops are
    // returned unchanged and nothing is registered.
    for a in [None, Some(1.0), Some(1.2)] {
        let mut doc = PdfDocument::new("opacity_test");
        let out = wrap_ops_with_opacity(&mut doc, a, inner.clone());
        assert_eq!(out, inner, "alpha {a:?} must be a no-op");
        assert_eq!(doc.resources.extgstates.map.len(), 0, "alpha {a:?}");
    }
}

#[test]
fn wrap_ops_with_opacity_clamps_alpha() {
    // -0.2 -> 0.0, 0.0 -> 0.0, 0.5 -> 0.5, 1.0 -> no-op (>= 1.0), 1.2 -> no-op.
    for (input, expected) in [(Some(-0.2), 0.0), (Some(0.0), 0.0), (Some(0.5), 0.5)] {
        let mut doc = PdfDocument::new("opacity_test");
        let _ = wrap_ops_with_opacity(
            &mut doc,
            input,
            vec![Op::SaveGraphicsState, Op::RestoreGraphicsState],
        );
        assert_eq!(doc.resources.extgstates.map.len(), 1, "alpha {input:?}");
        for gs in doc.resources.extgstates.map.values() {
            assert!((gs.current_fill_alpha() - expected).abs() < 1e-6);
            assert!((gs.current_stroke_alpha() - expected).abs() < 1e-6);
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib pdf_utils` (and `cargo test --lib -- --nocapture` to see the compile error)
Expected: COMPILE ERROR — `wrap_ops_with_opacity` not found.

- [ ] **Step 3: Implement the helper** (add after `draw_rectangle` in `src/schemas/pdf_utils.rs`)

```rust
/// Wrap `ops` so the element draws at the given `alpha` (0.0 = fully
/// transparent, 1.0 = fully opaque).
///
/// A non-`None` alpha < 1.0 registers an Extended Graphics State with both the
/// stroking (`CA`) and non-stroking (`ca`) alpha constants set to `alpha`, then
/// scopes it around `ops` with `SaveGraphicsState` / `LoadGraphicsState` /
/// `RestoreGraphicsState` so the transparency does not leak to elements drawn
/// afterwards. `None` (field absent) or an alpha >= 1.0 returns `ops`
/// unchanged and registers nothing.
pub fn wrap_ops_with_opacity(
    doc: &mut PdfDocument,
    alpha: Option<f32>,
    ops: Vec<Op>,
) -> Vec<Op> {
    let Some(alpha) = alpha else {
        return ops;
    };
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha >= 1.0 {
        return ops;
    }
    let gs = ExtendedGraphicsState::default()
        .with_current_fill_alpha(alpha)
        .with_current_stroke_alpha(alpha);
    let gs_id = doc.add_graphics_state(gs);
    let mut wrapped = vec![Op::SaveGraphicsState, Op::LoadGraphicsState { gs: gs_id }];
    wrapped.extend(ops);
    wrapped.push(Op::RestoreGraphicsState);
    wrapped
}
```

`ExtendedGraphicsState` is already in scope via the file's `use printpdf::*;` (line 3).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib pdf_utils::tests::wrap_ops_with_opacity`
Expected: 3 PASS (registers_extgstate_and_wraps, is_noop_without_alpha_or_full_alpha, clamps_alpha).

- [ ] **Step 5: Commit**

```bash
git add src/schemas/pdf_utils.rs
git commit -m "feat: add wrap_ops_with_opacity helper for schema transparency"
```

---

### Task 2: Wire `opacity` into `Rect::render`

**Files:**
- Modify: `src/schemas/rect.rs:86-125` (`Rect::render`, rename `_doc` → `doc`)
- Test: create `tests/opacity_test.rs`

**Interfaces:**
- Consumes: `wrap_ops_with_opacity(doc, alpha, ops)` from Task 1.
- Produces: `Rect` renders with alpha applied; integration test asserts the rendered `OpBuffer` contains a `LoadGraphicsState` and `doc.resources.extgstates` carries alpha 0.5.

- [ ] **Step 1: Write the failing integration test** (create `tests/opacity_test.rs`)

```rust
use pdforge::schemas::rect::JsonRectSchema;
use pdforge::schemas::{Schema, SchemaTrait};
use pdforge::utils::OpBuffer;
use printpdf::{Mm, Op, PdfDocument};

fn render_rect_into(opacity: Option<f32>, doc: &mut PdfDocument, buffer: &mut OpBuffer) {
    let opacity = opacity.map_or(String::new(), |v| format!(", \"opacity\": {v}"));
    let json = format!(
        r#"{{
            "name": "rect",
            "position": {{"x": 10.0, "y": 20.0}},
            "width": 50.0,
            "height": 30.0,
            "color": "#FFFFFF",
            "borderColor": "#000000"{opacity}
        }}"#
    );
    let schema: Schema = serde_json::from_str::<JsonRectSchema>(&json)
        .unwrap()
        .try_into()
        .unwrap();
    schema
        .render(Mm(150.0), Mm(210.0), doc, 0, buffer)
        .unwrap();
}

fn render_rect(opacity: Option<f32>) -> (PdfDocument, OpBuffer) {
    let mut doc = PdfDocument::new("opacity_test");
    let mut buffer = OpBuffer::default();
    render_rect_into(opacity, &mut doc, &mut buffer);
    (doc, buffer)
}

#[test]
fn rect_opacity_emits_load_graphics_state_and_registers_alpha() {
    let (doc, buffer) = render_rect(Some(0.5));

    assert_eq!(
        buffer.buffer[0]
            .iter()
            .filter(|op| matches!(op, Op::LoadGraphicsState { .. }))
            .count(),
        1
    );
    assert_eq!(doc.resources.extgstates.map.len(), 1);
    for gs in doc.resources.extgstates.map.values() {
        assert!((gs.current_fill_alpha() - 0.5).abs() < 1e-6);
        assert!((gs.current_stroke_alpha() - 0.5).abs() < 1e-6);
    }
}

#[test]
fn rect_without_opacity_registers_nothing() {
    let (doc, buffer) = render_rect(None);

    assert!(
        buffer.buffer[0]
            .iter()
            .all(|op| !matches!(op, Op::LoadGraphicsState { .. }))
    );
    assert_eq!(doc.resources.extgstates.map.len(), 0);
}

#[test]
fn rect_opacity_scope_restores_before_next_element() {
    // Same buffer: an opacity element followed by a plain element. The first
    // element's wrapper must close with its outer RestoreGraphicsState as the
    // very last op of that render, immediately before the second element draws,
    // so the alpha does not leak into the opaque element.
    let mut doc = PdfDocument::new("opacity_test");
    let mut buffer = OpBuffer::default();
    render_rect_into(Some(0.5), &mut doc, &mut buffer);
    let first_element_len = buffer.buffer[0].len();
    render_rect_into(None, &mut doc, &mut buffer);

    let ops = &buffer.buffer[0];
    // The opacity element's wrapper ends with its outer RestoreGraphicsState...
    assert!(matches!(
        ops[first_element_len - 1],
        Op::RestoreGraphicsState
    ));
    // ...and the opaque second element starts immediately after, drawing no
    // ExtGState of its own (its first op is the inner draw's SaveGraphicsState).
    assert!(matches!(ops[first_element_len], Op::SaveGraphicsState));
    assert!(ops[first_element_len..]
        .iter()
        .all(|op| !matches!(op, Op::LoadGraphicsState { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test opacity_test`
Expected: FAIL — `rect_opacity_emits_load_graphics_state_and_registers_alpha` fails because `Rect::render` ignores `opacity` (no `LoadGraphicsState` emitted).

- [ ] **Step 3: Implement — wrap the rectangle ops with opacity** in `Rect::render` (`src/schemas/rect.rs:86`)

Change the signature param `_doc: &mut PdfDocument` → `doc: &mut PdfDocument`, and replace the body from `let ops = draw_rectangle(rect);` onward:

```rust
        let ops = draw_rectangle(rect);
        let ops = wrap_ops_with_opacity(doc, self.opacity, ops);
        buffer.insert(page, ops);

        Ok(())
```

Add the import at the top of `src/schemas/rect.rs` (line 2 area):

```rust
use crate::schemas::pdf_utils::{draw_rectangle, wrap_ops_with_opacity, DrawRectangle};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test opacity_test`
Expected: 3 PASS (rect_opacity_emits..., rect_without_opacity..., rect_opacity_scope_restores...).

- [ ] **Step 5: Commit**

```bash
git add src/schemas/rect.rs tests/opacity_test.rs
git commit -m "feat: apply opacity to rectangle schema rendering"
```

---

### Task 3: Wire `opacity` into `Line::render`

**Files:**
- Modify: `src/schemas/line.rs:80-104` (`Line::render`, rename `_doc` → `doc`)
- Test: extend `tests/opacity_test.rs`

**Interfaces:**
- Consumes: `wrap_ops_with_opacity(doc, alpha, ops)` from Task 1.
- Produces: `Line` renders with alpha applied; integration test asserts `LoadGraphicsState` emitted + `doc.resources.extgstates` carries alpha 0.5.

- [ ] **Step 1: Write the failing integration test** (append to `tests/opacity_test.rs`)

```rust
use pdforge::schemas::line::JsonLineSchema;

fn render_line(opacity: Option<f32>) -> (PdfDocument, OpBuffer) {
    let opacity = opacity.map_or(String::new(), |v| format!(", \"opacity\": {v}"));
    let json = format!(
        r#"{{
            "name": "line",
            "position": {{"x": 10.0, "y": 20.0}},
            "width": 50.0,
            "color": "#000000"{opacity}
        }}"#
    );
    let schema: Schema = serde_json::from_str::<JsonLineSchema>(&json)
        .unwrap()
        .try_into()
        .unwrap();
    let mut doc = PdfDocument::new("opacity_test");
    let mut buffer = OpBuffer::default();
    schema
        .render(Mm(150.0), Mm(210.0), &mut doc, 0, &mut buffer)
        .unwrap();
    (doc, buffer)
}

#[test]
fn line_opacity_emits_load_graphics_state_and_registers_alpha() {
    let (doc, buffer) = render_line(Some(0.5));

    assert_eq!(
        buffer.buffer[0]
            .iter()
            .filter(|op| matches!(op, Op::LoadGraphicsState { .. }))
            .count(),
        1
    );
    assert_eq!(doc.resources.extgstates.map.len(), 1);
    for gs in doc.resources.extgstates.map.values() {
        assert!((gs.current_fill_alpha() - 0.5).abs() < 1e-6);
        assert!((gs.current_stroke_alpha() - 0.5).abs() < 1e-6);
    }
}

#[test]
fn line_without_opacity_registers_nothing() {
    let (doc, buffer) = render_line(None);

    assert!(
        buffer.buffer[0]
            .iter()
            .all(|op| !matches!(op, Op::LoadGraphicsState { .. }))
    );
    assert_eq!(doc.resources.extgstates.map.len(), 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test opacity_test`
Expected: FAIL — `line_opacity_emits_load_graphics_state_and_registers_alpha` fails because `Line::render` ignores `opacity`.

- [ ] **Step 3: Implement — wrap the line ops with opacity** in `Line::render` (`src/schemas/line.rs:80`)

Change `_doc: &mut PdfDocument` → `doc: &mut PdfDocument`, and replace the body from `let ops = draw_line(DrawLine { ... });` onward:

```rust
        let ops = draw_line(DrawLine {
            x: self.base.x,
            y: self.base.y,
            width: self.base.width,
            rotate: self.rotate,
            page_height: parent_height,
            color: Color::Rgb(Rgb {
                r: self.color.r,
                g: self.color.g,
                b: self.color.b,
                icc_profile: None,
            }),
            border_width: Mm(self.border_width.0),
        });
        let ops = wrap_ops_with_opacity(doc, self.opacity, ops);

        buffer.insert(page, ops);
        Ok(())
```

Add the import at the top of `src/schemas/line.rs` (line 2 area):

```rust
use crate::schemas::pdf_utils::{
    calculate_transform_matrix_with_center_pivot, wrap_ops_with_opacity,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test opacity_test`
Expected: 5 PASS total (3 rect + 2 line).

- [ ] **Step 5: Commit**

```bash
git add src/schemas/line.rs tests/opacity_test.rs
git commit -m "feat: apply opacity to line schema rendering"
```

---

### Task 4: Docs, CHANGELOG, and full verification

**Files:**
- Modify: `CHANGELOG.md` (add `Unreleased` entry)
- Modify: `docs/schema-spec.md` (document `opacity` for rectangle/line if a schema reference section exists — see Step 3 note)

**Interfaces:**
- Consumes: completed Tasks 1–3.
- Produces: released-quality changelog + docs; confirmed zero new warnings and a green suite.

- [ ] **Step 1: Add a CHANGELOG entry** under `## [Unreleased]` (currently empty, `CHANGELOG.md:11`)

```markdown
## [Unreleased]

### Added
- `Rectangle` and `Line` schemas now honour the `opacity` field: a value between 0 (fully transparent) and 1 (fully opaque) renders the element translucent via a PDF ExtGState (`ca`/`CA`). The field is optional and defaults to fully opaque; `1` and absent both render exactly as before.
```

- [ ] **Step 2: Check the schema spec** — grep `docs/schema-spec.md` for a rectangle/line field table

Run: `grep -n "opacity\|Rectangle\|Line\|borderWidth" docs/schema-spec.md | head -30`
Expected: either a field list to extend, or no schema-reference section (then skip Step 3).

- [ ] **Step 3: Document the field** — only if Step 2 found a rectangle/line field table

Add an `opacity` row matching the surrounding table style, e.g.:
`| `opacity` | `number` | optional | 1 | Element transparency, 0 (transparent) to 1 (opaque). Applied to both fill and stroke. |

- [ ] **Step 4: Full verification — build and run the whole suite**

Run: `cargo build 2>&1 | grep -E "^warning" | grep -c opacity`
Expected: `0` (both "field `opacity` is never read" warnings are gone; the other 5 unrelated warnings remain).

Run: `cargo test 2>&1 | grep -E "test result"`
Expected: all suites pass; `opacity_test` shows 5 passed; total = 156 baseline + 3 unit + 5 integration = 164 passing.

- [ ] **Step 5: Commit**

```bash
git add CHANGELOG.md docs/schema-spec.md
git commit -m "docs: document opacity support for rectangle and line schemas"
```

---

## Self-Review

**1. Spec coverage**
- Opacity wired into `Rect` → Task 2. Into `Line` → Task 3.
- Shared, DRY helper so both schemas use one code path → Task 1.
- No-op when absent / >= 1.0 so existing output is unchanged → Task 1 helper + Global Constraints; verified by `wrap_ops_with_opacity_is_noop_without_alpha_or_full_alpha` (covers `None`, `1.0`, `1.2`), `wrap_ops_with_opacity_clamps_alpha` (boundaries `-0.2`/`0.0`/`0.5`), and `rect_without_opacity_registers_nothing`.
- ExtGState scoping does not leak to later elements → `wrap_ops_with_opacity_registers_extgstate_and_wraps` asserts exact `[q, Load, ...inner, Q]` structure, and `rect_opacity_scope_restores_before_next_element` renders an opaque element right after a translucent one in the same buffer.
- Resolve the 2 "never read" warnings → Tasks 2/3 (verified in Task 4 Step 4).
- CHANGELOG + docs → Task 4.

**2. Placeholder scan:** No TBD/TODO; every code step has full source. Step 3 of Task 4 is conditionally gated on a real grep result, not a placeholder — it says exactly what to do in each branch.

**3. Type consistency**
- Helper signature `wrap_ops_with_opacity(&mut PdfDocument, Option<f32>, Vec<Op>) -> Vec<Op>` is identical in Task 1 (definition) and Tasks 2/3 (call sites).
- `ExtendedGraphicsState::default().with_current_fill_alpha(a).with_current_stroke_alpha(a)` — confirmed present in printpdf 0.12.3 (`graphics.rs:1015,1021`, `Default` at `1039`).
- `Op::LoadGraphicsState { gs: ExtendedGraphicsStateId }` — confirmed (`ops.rs:433`); serializes to `gs` operator (`serialize.rs:539`).
- `doc.add_graphics_state(gs) -> ExtendedGraphicsStateId` — confirmed (`lib.rs:337`).
- Test accessors `doc.resources.extgstates.map` (`lib.rs:783,792`), `current_fill_alpha()`/`current_stroke_alpha()` (`graphics.rs:719,723`) — confirmed pub.
- `Op` derives `Clone, PartialEq, Debug` (`ops.rs:405`) so `assert_eq!(none, inner)` and `contains(...)` are valid.
- `Rect::render` / `Line::render` signatures already take `&mut PdfDocument` (currently `_doc`) — only renamed, no signature change; `SchemaTrait::render` (mod.rs:148) unchanged.
