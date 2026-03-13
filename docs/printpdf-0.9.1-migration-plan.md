# printpdf 0.9.1 移行プラン

## 概要

本ドキュメントは、pdforgeが依存する printpdf クレートを現在のピン留めコミット (`a88db12`) から バージョン `0.9.1` (masterブランチ最新: `8a554fc`) へ移行するための計画です。

## 現状

`Cargo.toml` の現在の依存定義:

```toml
printpdf = { git = "https://github.com/fschutt/printpdf", features = [
    "bmp",
    "png",
    "jpeg",
], rev = "a88db12bdc020e0f3743cc4bb22b08826d733b9c" }
```

## 変更後の依存定義

```toml
printpdf = { version = "0.9.1", features = [
    "bmp",
    "png",
    "jpeg",
] }
```

## 破壊的変更と対応が必要な箇所

### 1. `Op::SetFontSize` → `Op::SetFont` への変更 (最重要)

**変更前 (現在のコード):**
```rust
Op::SetFontSize {
    size: font_size,
    font: font_id.clone(),
},
```

**変更後:**

旧APIの `SetFontSize` と `SetFont` が統合され、新しい `Op::SetFont` は `PdfFontHandle` enum を受け取るようになった。

```rust
Op::SetFont {
    font: PdfFontHandle::External(font_id.clone()),
    size: font_size,
},
```

**対象ファイル:** `src/schemas/pdf_utils.rs:167`

---

### 2. `Op::WriteText` → `Op::ShowText` への変更 (最重要)

**変更前:**
```rust
Op::WriteText {
    items: vec![TextItem::Text(sanitized_line)],
    font: font_id.clone(),
},
```

**変更後:**

新APIでは `ShowText` はフォント参照を持たない。フォントは直前の `SetFont` オペレーションで設定する。

```rust
Op::ShowText {
    items: vec![TextItem::Text(sanitized_line)],
},
```

**対象ファイル:** `src/schemas/pdf_utils.rs:175`

---

### 3. `PdfFontHandle` の import 追加

`Op::SetFont` で使用する `PdfFontHandle` を import する必要がある。

**対象ファイル:** `src/schemas/pdf_utils.rs`

```rust
use printpdf::{Op, PdfFontHandle, TextItem, /* ... */};
```

`use printpdf::*;` を使っている場合は自動的に解決される。

---

## 互換性が確認できた API（変更不要）

以下の API はすべて 0.9.1 でも同じシグネチャで存在する:

| API | ファイル |
|-----|---------|
| `PdfDocument::new()` | `src/lib.rs` |
| `PdfDocument::add_font(&ParsedFont) -> FontId` | `src/lib.rs` |
| `PdfDocument::add_image(&RawImage) -> XObjectId` | `src/schemas/image.rs` |
| `PdfDocument::add_xobject(&ExternalXObject) -> XObjectId` | `src/schemas/svg.rs` |
| `PdfDocument::with_pages(Vec<PdfPage>) -> &mut Self` | `src/schemas/mod.rs` |
| `PdfDocument::save(&PdfSaveOptions, &mut Vec<PdfWarnMsg>) -> Vec<u8>` | `src/schemas/mod.rs` |
| `PdfPage::new(Mm, Mm, Vec<Op>)` | `src/schemas/mod.rs` |
| `PdfSaveOptions { optimize, subset_fonts, secure, image_optimization }` | `src/schemas/mod.rs` |
| `ParsedFont::from_bytes(&[u8], usize, &mut Vec<_>) -> Option<Self>` | `src/lib.rs` |
| `ParsedFont::lookup_glyph_index(u32)` | `src/font.rs`, `src/schemas/pdf_utils.rs` |
| `ParsedFont::get_horizontal_advance(u16)` | `src/font.rs` |
| `RawImage::decode_from_bytes(&[u8], &mut Vec<_>)` | `src/schemas/image.rs` |
| `RawImage { width, height }` | `src/schemas/image.rs` |
| `XObjectTransform { translate_x, translate_y, rotate, scale_x, scale_y, dpi }` | `src/schemas/image.rs`, `src/schemas/base.rs` |
| `Svg::parse(&str, &mut Vec<_>) -> Result<ExternalXObject, _>` | `src/schemas/svg.rs` |
| `Op::SaveGraphicsState` / `RestoreGraphicsState` | 複数ファイル |
| `Op::SetTransformationMatrix` | 複数ファイル |
| `Op::StartTextSection` / `EndTextSection` | `src/schemas/pdf_utils.rs` |
| `Op::SetTextMatrix` | `src/schemas/pdf_utils.rs` |
| `Op::SetLineHeight` | `src/schemas/pdf_utils.rs` |
| `Op::SetFillColor` / `SetOutlineColor` / `SetOutlineThickness` | 複数ファイル |
| `Op::SetCharacterSpacing` | `src/schemas/pdf_utils.rs` |
| `Op::UseXobject { id, transform }` | `src/schemas/svg.rs`, `src/schemas/image.rs` |
| `Op::DrawPolygon { polygon }` | 複数ファイル |
| `Mm`, `Pt`, `Px` | 全体 |
| `Point`, `LinePoint`, `Line`, `Polygon`, `PolygonRing` | 複数ファイル |
| `PaintMode`, `WindingOrder` | 複数ファイル |
| `Color`, `Rgb` | 複数ファイル |
| `CurTransMat`, `TextMatrix` | 複数ファイル |
| `FontId` | 全体 |
| `TextItem::Text(String)` | `src/schemas/pdf_utils.rs` |

---

## 実装手順

### Step 1: Cargo.toml の更新

```toml
# 変更前
printpdf = { git = "https://github.com/fschutt/printpdf", features = [
    "bmp",
    "png",
    "jpeg",
], rev = "a88db12bdc020e0f3743cc4bb22b08826d733b9c" }

# 変更後
printpdf = { version = "0.9.1", features = [
    "bmp",
    "png",
    "jpeg",
] }
```

### Step 2: `src/schemas/pdf_utils.rs` の修正

`create_text_ops_with_font` 関数内の2箇所を変更する:

**L167: `Op::SetFontSize` → `Op::SetFont`**
```rust
// 変更前
Op::SetFontSize {
    size: font_size,
    font: font_id.clone(),
},

// 変更後
Op::SetFont {
    font: PdfFontHandle::External(font_id.clone()),
    size: font_size,
},
```

**L175: `Op::WriteText` → `Op::ShowText`**
```rust
// 変更前
Op::WriteText {
    items: vec![TextItem::Text(sanitized_line)],
    font: font_id.clone(),
},

// 変更後
Op::ShowText {
    items: vec![TextItem::Text(sanitized_line)],
},
```

### Step 3: ビルド確認

```bash
cargo build
```

### Step 4: テスト実行

```bash
cargo test
cargo test --test thread_safety
```

### Step 5: 動作確認

```bash
cargo run --example simple
cargo run --example font_from_bytes
```

---

## 注意事項

### SVG featureについて

0.9.1 では `svg` feature が `html` feature（azul-layout依存）に依存するようになった。pdforgeは現在 SVG に `printpdf::svg::Svg::parse()` を使用しているが、これは `svg` feature が必要。

ただし `svg` feature を有効にすると `azul-layout` など多数の依存が引き込まれる。現時点では `svg` feature を `Cargo.toml` に追加するか、あるいは SVG サポートを削除・代替実装することを検討する必要がある。

**推奨**: まず `svg` feature なしでビルドし、SVG使用箇所のコンパイルエラーを確認してから判断する。

### `lopdf` バージョン

0.9.1 は `lopdf = "0.39.0"` を使用。pdforge も `lopdf = "0.37.0"` を直接依存しているため、バージョン競合が起きる可能性がある。Cargo.toml の `lopdf` バージョンを `0.39.0` に更新するか、削除して printpdf 経由で使うことを検討する。

---

## リスク評価

| リスク | 重要度 | 対応 |
|--------|--------|------|
| `Op::SetFontSize`/`WriteText` の名称変更 | 高 | Step 2 で対応 |
| SVG feature の依存関係変化 | 中 | ビルドエラーを確認して判断 |
| `lopdf` バージョン競合 | 低〜中 | Cargo.lock で Cargo が解決、問題あれば手動調整 |
| crates.io への公開遅延 | 低 | 未公開の場合は git + tag 指定で対応可能 |
