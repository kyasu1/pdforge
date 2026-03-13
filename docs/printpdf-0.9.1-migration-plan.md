# printpdf 0.9.1 移行プラン

## 概要

本ドキュメントは、pdforgeが依存する printpdf クレートを現在のピン留めコミット (`a88db12`) から バージョン `0.9.1` (masterブランチ最新: `8a554fc`) へ移行するための計画です。SVGサポートを含む完全移行を対象とします。

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
    "svg",
] }
```

`svg` feature は `html` feature → `azul-layout` (text_layout_hyphenation) + `svg2pdf = "0.13.0"` を引き込む。
依存クレートはすべて crates.io に公開済みのため、git 依存なしで解決可能。

---

## 破壊的変更と対応が必要な箇所

### 1. `Op::SetFontSize` → `Op::SetFont` への変更

**変更前:**
```rust
Op::SetFontSize {
    size: font_size,
    font: font_id.clone(),
},
```

**変更後:**

旧 `SetFontSize` と `SetFont` が統合され、新 `Op::SetFont` は `PdfFontHandle` enum を受け取る。

```rust
Op::SetFont {
    font: PdfFontHandle::External(font_id.clone()),
    size: font_size,
},
```

**対象ファイル:** `src/schemas/pdf_utils.rs`

---

### 2. `Op::WriteText` → `Op::ShowText` への変更

**変更前:**
```rust
Op::WriteText {
    items: vec![TextItem::Text(sanitized_line)],
    font: font_id.clone(),
},
```

**変更後:**

新 `ShowText` はフォント参照を持たない。フォントは直前の `SetFont` で設定済みのため不要。

```rust
Op::ShowText {
    items: vec![TextItem::Text(sanitized_line)],
},
```

**対象ファイル:** `src/schemas/pdf_utils.rs`

---

### 3. `lopdf` バージョンの更新

0.9.1 は `lopdf = "0.39.0"` を使用。pdforge が直接依存している `lopdf = "0.37.0"` をあわせて更新する。

**対象ファイル:** `Cargo.toml`

---

## SVG サポートについて

`svg` feature を有効にすることで `printpdf::svg::Svg::parse()` は引き続き同じシグネチャで使用可能。**`src/schemas/svg.rs` のコード変更は不要。**

`svg` feature を有効にすると `azul-layout` 経由で lyon, tiny-skia, usvg, resvg 等の依存が引き込まれ、初回ビルド時間が増加するが、機能的な問題はない。

---

## 互換性が確認できた API（変更不要）

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

### Step 1: `Cargo.toml` の更新

- `printpdf` を git rev 指定から `version = "0.9.1"` に変更し、`"svg"` feature を追加
- `lopdf` を `"0.37.0"` から `"0.39.0"` に更新

### Step 2: `src/schemas/pdf_utils.rs` の修正

`create_text_ops_with_font` 関数内の2箇所を変更:

- `Op::SetFontSize { size, font }` → `Op::SetFont { font: PdfFontHandle::External(font_id), size }`
- `Op::WriteText { items, font }` → `Op::ShowText { items }`

### Step 3: ビルド確認・テスト

```bash
cargo build
cargo test
cargo test --test thread_safety
cargo run --example simple
cargo run --example font_from_bytes
```

---

## リスク評価

| リスク | 重要度 | 対応 |
|--------|--------|------|
| `Op::SetFontSize`/`WriteText` の名称変更 | 高 | Step 2 で対応済み |
| `lopdf` バージョン競合 | 低 | Cargo.toml を 0.39.0 に更新 |
| svg feature による初回ビルド時間増加 | 低 | 許容範囲 |
