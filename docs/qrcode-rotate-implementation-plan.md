# QR Code `rotate` Implementation Plan

## Overview

Enable `rotate` field support for QR Code schema, consistent with existing rotation behavior in `Group`, `Rectangle`, and `Line` schemas.

## Requirements

- **Pivot Point**: Center of QR code image (matches Group/Rect/Line behavior: center of the element's bounding box)
- **Unit**: Degrees (`f32`)
- **Behavior**: No clipping — rotated QR code should remain centered in its allocated space

## Files to Change

| File | Purpose |
|---|---|
| `src/schemas/base.rs` | Add new `get_matrix_with_rotation()` helper; keep original 2-arg `get_matrix()` (non-breaking) |
| `src/schemas/image.rs` | Keep original 2-arg `get_matrix()` — no rotation |
| `src/schemas/svg.rs` | Keep original 2-arg `get_matrix()` — no rotation |
| `src/schemas/qrcode.rs` | Use `get_matrix_with_rotation()` with `Some(XObjectRotation)` when `rotate` set |
| `templates/qrcode-rotate-test.json` | New test template |
| `examples/qrcode-rotate-test.rs` | New example runner |
| `tests/qr_code_rotation_test.rs` | New auto-test suite |

## Current State

- `JsonQrCodeSchema` has `rotate: Option<f32>` (line 18)
- `QrCode` struct stores `rotate: Option<f32>` (line 28)
- `render()` previously ignored `rotate` entirely; it used `BaseSchema::get_matrix()` which had no rotation parameter
- `BaseSchema::get_matrix()` previously built `XObjectTransform` with fixed `rotate: None`

```rust
pub fn get_matrix(&self, page_height: Mm, original_width: Option<Px>) -> XObjectTransform {
    XObjectTransform {
        translate_x: Some(self.x.into()),
        translate_y: Some((page_height - self.y - self.height).into()),
        rotate: None,  // <-- always None
        scale_x: Some(ratio * self.width.0),
        scale_y: Some(ratio * self.height.0),
        dpi: Some(dpi),
        no_auto_scale: false,
    }
}
```

## Dismissed Approaches

### ~~Option B (Original): Center-Pivot via CTM~~

**DISCARDED** due to blockers:

1. **Double translation**: `calculate_transform_matrix_with_center_pivot` already includes x/y translation, and `UseXobject`'s `transform` also contains absolute coordinates. Combining both applies translation twice.
2. **Incorrect tx/ty formula**: The original formula expressions are wrong.
3. **Unnecessary complexity**: `printpdf::XObjectTransform.rotate` already supports center-specifiable rotation.

## Implementation (Final): Non-breaking — add `get_matrix_with_rotation`

**Codex review (REQUEST CHANGES)** flagged that changing `get_matrix()`'s signature is a breaking public-API change. Final approach keeps the original 2-arg `get_matrix()` untouched and adds a new `get_matrix_with_rotation()` helper that the original delegates to with `rotation = None`.

### Call Sites

Current `get_matrix()` / `get_matrix_with_rotation()` callers:
1. `svg.rs` — uses basic `get_matrix()` (no rotation)
2. `image.rs` — uses basic `get_matrix()` (no rotation)
3. `qrcode.rs` — uses `get_matrix_with_rotation()` (rotation when `rotate` set)

Only 3 call sites; all are image-like schemas that will need rotation eventually.

### Rationale

- `printpdf` 0.12.3's `XObjectTransform.rotate` expects `XObjectRotation` (not `Angle`)
- `XObjectRotation` has fields:
  - `angle_ccw_degrees: f32`
  - `rotation_center_x: Px`
  - `rotation_center_y: Px`
- Rotation center is specified **in pixels relative to the image XObject's intrinsic size**
- `BaseSchema` cannot compute its own rotation center — it only has layout dimensions, not image dimensions; each caller must provide it based on their actual XObject size
- Passing `Option<XObjectRotation>` centralizes rotation logic in one place while keeping callers in charge of rotation center
- When Image/SVG add rotation support later, just add `rotate` field to their schema and pass `XObjectRotation` to `get_matrix()` — no CTM duplication

### Code Change (`src/schemas/base.rs`)

```rust
// Existing public 2-arg API — unchanged (non-breaking)
pub fn get_matrix(&self, page_height: Mm, original_width: Option<Px>) -> XObjectTransform {
    self.get_matrix_with_rotation(page_height, original_width, None)
}

// New helper centralizing rotation logic
pub fn get_matrix_with_rotation(
    &self,
    page_height: Mm,
    original_width: Option<Px>,
    rotation: Option<XObjectRotation>,
) -> XObjectTransform {
    let dpi: f32 = 300.0;
    let ratio: f32 = match original_width {
        Some(original_width) => dpi / 25.4 / (original_width.0 as f32),
        None => 1.0,
    };
    XObjectTransform {
        translate_x: Some(self.x.into()),
        translate_y: Some((page_height - self.y - self.height).into()),
        rotate: rotation,
        scale_x: Some(ratio * self.width.0),
        scale_y: Some(ratio * self.height.0),
        dpi: Some(dpi),
        no_auto_scale: false,
    }
}
```

### Code Change (`src/schemas/svg.rs`)

Keeps original 2-arg call — no rotation for SVG:

```rust
let transform = self.base.get_matrix(parent_height, self.content.width);
```

### Code Change (`src/schemas/image.rs`)

`calculate_object_fit_transform` for Fill mode keeps the original 2-arg call:

```rust
self.base.get_matrix(parent_height, Some(Px(image.width)))
```

Other object-fit modes already construct their own `XObjectTransform`; update them to keep `rotate: None`:

```rust
XObjectTransform {
    // ... existing translate_x/y, scale_x/y, etc.
    rotate: None,  // explicitly None; future Image rotation support would set this
```

### Code Change (`src/schemas/qrcode.rs::render()`)

Apply rotation **after** alignment + padding placement. `printpdf` composes the
image and layout scales before applying the pivot, so express the center of the
final QR square as pixels at the transform DPI:

```rust
use printpdf::{XObjectTransform, XObjectRotation};

let rotation_center = Px(
    (qr_side.0 * XOBJECT_DPI / 25.4 / 2.0)
        .round()
        .max(0.0) as usize,
);
let rotation: Option<XObjectRotation> = self.rotate.map(|angle_deg| XObjectRotation {
    angle_ccw_degrees: angle_deg,
    rotation_center_x: rotation_center,
    rotation_center_y: rotation_center,
});

let transform = temp_base.get_matrix_with_rotation(parent_height, Some(qrcode_width), rotation);
```

**Key properties:**

- `rotation_center_x/y` represent the final rendered QR center converted to pixels at `XOBJECT_DPI`; using the source image center would shift a scaled QR during rotation
- `Px(pub usize)` requires `usize`, not `f32`
- Rotation logic centralized in `BaseSchema::get_matrix_with_rotation()`
- Image/SVG rotation support is just: add `rotate` field, compute rotation from image w/h, pass to `get_matrix_with_rotation()`
- `qrcode_width` uses the **actual intrinsic image width** `Px(w as usize)` (not a hardcoded `Px(256)`), fixing the ~3.1% overscaling bug

## Test Strategy

1. **Visual Verification**:
   - Generate QR with `rotate: 30`, `rotate: 45`, `rotate: 90`, `rotate: 360`
   - Confirm: box-level center is the rotation pivot; QR code rotates inside bounding box
   - Confirm: with `align` = Left/Center/Right + `v_align` = Top/Middle/Bottom + `padding` present — rotated QR stays correctly positioned

2. **Auto-tests (REQUIRED)**:
   - `rotate: None` → `Op::UseXobject.transform.rotate == None` (identical to current no-rotation behavior)
   - `rotate: 90` → the final CTM has the same bounds as the unrotated QR at the same position and size
   - `get_matrix()` keeps original 2-arg signature; `get_matrix_with_rotation(.., None)` matches original
   - Rotation direction matches Group/Rect direction at same angle
   - Scale regression: with `original_width` = intrinsic image width, `scale_x` matches `300/25.4/w*width`
   - Tests use table cell + padding + various alignment combinations

3. **Code Validation**:
   - Build: `cargo build --quiet`
   - Test: `cargo test`
   - Example: `cargo run --example qrcode-rotate-test`

## New Assets

### 1. `/templates/qrcode-rotate-test.json`

Tests QR codes in table cells with observable padding and different alignment values.
Uses current `JsonTableSchema` structure:
- `JsonFrame` is object form `{top, right, bottom, left}`, not array `[..]`
- Each QR cell has directly specified content and padding; bodyStyles.padding is NOT inherited by QR cells
- First column uses height=60 to set the row height while every QR renders as a
  40 mm square; the remaining whitespace makes vertical alignment observable
- Rotated cells use 2 mm padding, which fits around the 40 mm QR without changing
  its rendered size
- table height=100 < page height=150

```jsonc
{
  "schemas": [
    [
      {
        "type": "table",
        "name": "test_table",
        "position": { "x": 10, "y": 10 },
        "width": 190,
        "height": 100,
        "showHead": true,
        "headStyles": {
          "fontSize": 8,
          "fontName": "NotoSansJP",
          "alignment": "center",
          "verticalAlignment": "middle",
          "lineHeight": 1.0,
          "fontColor": "#000000",
          "borderColor": "#000000",
          "backgroundColor": "#f0f0f0",
          "borderWidth": {
            "top": 0.2,
            "right": 0.2,
            "bottom": 0.2,
            "left": 0.2
          },
          "padding": {
            "top": 2,
            "right": 2,
            "bottom": 2,
            "left": 2
          }
        },
        "bodyStyles": {
          "alignment": "center",
          "verticalAlignment": "middle",
          "lineHeight": 1.0,
          "fontColor": "#000000",
          "backgroundColor": "#ffffff",
          "padding": {
            "top": 0,
            "right": 0,
            "bottom": 0,
            "left": 0
          },
          "lineBreakMode": "char"
        },
        "tableStyles": {
          "borderWidth": 0.2,
          "borderColor": "#000000"
        },
        "columns": [
          {
            "width": "3fr",
            "header": { "content": "no rotate (40 square)" },
            "cell": {
              "type": "qrCode",
              "name": "no_rotate",
              "content": "qr_rotate_test_no_rotate",
              "position": { "x": 0, "y": 0 },
              "width": 40,
              "height": 60,
              "alignment": "center",
              "verticalAlignment": "middle"
            }
          },
          {
            "width": "3fr",
            "header": { "content": "rotate: 30 (40 square)" },
            "cell": {
              "type": "qrCode",
              "name": "rotated_30_center",
              "content": "qr_rotate_test_30_center",
              "position": { "x": 0, "y": 0 },
              "width": 40,
              "height": 40,
              "rotate": 30,
              "alignment": "center",
              "verticalAlignment": "middle",
              "padding": {
                "top": 2,
                "right": 2,
                "bottom": 2,
                "left": 2
              }
            }
          },
          {
            "width": "3fr",
            "header": { "content": "rotate: 90 top (40)" },
            "cell": {
              "type": "qrCode",
              "name": "rotated_90_top",
              "content": "qr_rotate_test_90_top",
              "position": { "x": 0, "y": 0 },
              "width": 40,
              "height": 40,
              "rotate": 90,
              "alignment": "left",
              "verticalAlignment": "top",
              "padding": {
                "top": 2,
                "right": 2,
                "bottom": 2,
                "left": 2
              }
            }
          },
          {
            "width": "3fr",
            "header": { "content": "rotate: 90 bottom (40)" },
            "cell": {
              "type": "qrCode",
              "name": "rotated_90_bottom",
              "content": "qr_rotate_test_90_bottom",
              "position": { "x": 0, "y": 0 },
              "width": 40,
              "height": 40,
              "rotate": 90,
              "alignment": "left",
              "verticalAlignment": "bottom",
              "padding": {
                "top": 2,
                "right": 2,
                "bottom": 2,
                "left": 2
              }
            }
          }
        ],
        "fields": [
          ["", "", "", ""]
        ]
      }
    ]
  ],
  "basePdf": {
    "width": 210,
    "height": 150,
    "padding": [10, 10, 10, 10]
  },
  "schemaVersion": "1.0.0"
}
```

### 2. `/examples/qrcode-rotate-test.rs`

```rust
use std::collections::HashMap;
use pdforge::PDForgeBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdforge = PDForgeBuilder::new("QR Code Rotation Test".to_string())
        .add_font_from_file("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("qr_rotate", "./templates/qrcode-rotate-test.json")?
        .build();

    let bytes = pdforge.render("qr_rotate", vec![vec![HashMap::new()]], None, None)?;

    std::fs::create_dir_all("examples/pdf").ok();
    std::fs::write("examples/pdf/qrcode-rotate-test.pdf", bytes)?;

    println!("PDF generated: examples/pdf/qrcode-rotate-test.pdf");
    Ok(())
}
```

### 3. `/tests/qr_code_rotation_test.rs`

Auto-test covering:
- `rotate: None` → transform unchanged (no rotation applied)
- `rotate: Some(90.0)` → the transform rotates around the center of the final rendered QR square without changing its bounds
- Rotation direction and center match Group/Rect behavior

## Change Checklist

- [x] Plan documented in `docs/qrcode-rotate-implementation-plan.md`
- [x] Approach: Extend `BaseSchema::get_matrix()` with optional `XObjectRotation` parameter
- [x] Add rotation parameter to `BaseSchema::get_matrix()` in `src/schemas/base.rs`
- [x] Update call sites: `svg.rs`, `image.rs` pass `None`
- [x] Implement rotation logic in `src/schemas/qrcode.rs::render()` — construct `XObjectRotation` from `self.rotate` + image `w/h`
- [x] Create test template + example (with HashMap import)
- [x] Write auto-tests in `tests/qr_code_rotation_test.rs`
- [x] `cargo build` / verify example + visual inspection
- [x] `cargo test` passes

---

End of plan.
