# PDForge スキーマアーキテクチャの改善案

## 現状の課題

### 現在の実装構造

```rust
// BaseSchema - 共通ベース
pub struct BaseSchema {
    pub(crate) name: String,
    pub(crate) x: Mm,
    pub(crate) y: Mm,
    pub(crate) width: Mm,
    pub(crate) height: Mm,
}

// 各スキーマは独自の構造体で定義
pub struct Text { base, content, alignment, ... }
pub struct DynamicText { base, content, font_size, ... }
pub struct Table { base, show_head, head_styles, ... }
pub struct Image { base, content, object_fit, ... }
pub struct QrCode { base, content, bounding_box, ... }
pub struct Svg { base, content, ... }
pub struct Rect { base, rotate, opacity, ... }
pub struct Line { base, rotate, opacity, ... }
pub struct Group { base, rotate, schemas, ... }
```

### 問題点

1. **コードの重複**: 全スキーマで共通の操作（`set_x`, `set_y`, `set_height`, `get_width`, `get_height`）が繰り返し実装されている
2. **拡張性の欠如**: 新しいスキーマを追加するたびに、`Schema` enum、`SchemaTrait`、`render_schemas` など複数の箇所を修正が必要
3. **型安全性と柔軟性のトレードオフ**: `Schema` enum による分岐は型安全だが、新しいスキーマ追加時に全パターンの修正が必要
4. **共通機能の分散**: 共通機能（ベース変換、描画前処理など）が各スキーマで個別実装

---

## 改善案: Trait-based アーキテクチャ

### 基本設計

```rust
use printpdf::{Mm, Pt, PdfDocument};
use crate::utils::OpBuffer;

/// スキーマの共通インターフェース
pub trait SchemaTrait {
    /// スキーマ名を取得
    fn name(&self) -> &str;

    /// 位置情報を取得
    fn position(&self) -> (Mm, Mm);

    /// サイズ情報を取得
    fn size(&self) -> (Mm, Mm);

    // 基本的な位置・サイズ操作
    fn set_x(&mut self, x: Mm);
    fn set_y(&mut self, y: Mm);
    fn set_width(&mut self, width: Mm);
    fn set_height(&mut self, height: Mm);

    // 描画処理
    fn render(
        &self,
        parent_height: Mm,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error>;

    // 共通ヘルパー（デフォルト実装）
    fn bounds(&self) -> BoundingBox {
        let (x, y) = self.position();
        let (w, h) = self.size();
        BoundingBox::new(x, y, w, h)
    }
}

/// ベーススキーマを内包するトレイト
pub trait HasBaseSchema {
    fn base(&self) -> &BaseSchema;
    fn base_mut(&mut self) -> &mut BaseSchema;
}

/// 描画可能なすべてのスキーマを表すエンム
#[derive(Debug, Clone)]
pub enum Schema {
    Text(Text),
    DynamicText(DynamicText),
    Table(Table),
    QrCode(QrCode),
    Image(Image),
    Svg(Svg),
    Rect(Rect),
    Line(Line),
    Group(Group),
}
```

### 各スキーマの実装例

#### Text スキーマ

```rust
pub struct Text {
    base: BaseSchema,
    content: String,
    alignment: Alignment,
    vertical_alignment: VerticalAlignment,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: FontSize,
    font_id: FontId,
    font_spec: Arc<dyn FontSpecTrait>,
    font: Arc<ParsedFont>,
    font_color: csscolorparser::Color,
    background_color: Option<csscolorparser::Color>,
    padding: Option<Frame>,
    rotate: Option<f32>,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
    border_color: Option<csscolorparser::Color>,
    border_width: Option<Pt>,
}

impl HasBaseSchema for Text {
    fn base(&self) -> &BaseSchema { &self.base }
    fn base_mut(&mut self) -> &mut BaseSchema { &mut self.base }
}

impl SchemaTrait for Text {
    fn name(&self) -> &str { &self.base.name }
    fn position(&self) -> (Mm, Mm) { (self.base.x, self.base.y) }
    fn size(&self) -> (Mm, Mm) { (self.base.width, self.base.height) }

    fn set_x(&mut self, x: Mm) { self.base.x = x; }
    fn set_y(&mut self, y: Mm) { self.base.y = y; }
    fn set_width(&mut self, width: Mm) { self.base.width = width; }
    fn set_height(&mut self, height: Mm) { self.base.height = height; }

    fn render(&self, parent_height: Mm, doc: &mut PdfDocument, page: usize, buffer: &mut OpBuffer) -> Result<(), Error> {
        // 現在のText::render実装をそのまま使用
        // ...
        # Ok(())
    }
}
```

#### DynamicText スキーマ

```rust
pub struct DynamicText {
    base: BaseSchema,
    content: String,
    character_spacing: Pt,
    line_height: Option<f32>,
    font_size: Pt,
    font_id: FontId,
    font_spec: Arc<dyn FontSpecTrait>,
}

impl HasBaseSchema for DynamicText {
    fn base(&self) -> &BaseSchema { &self.base }
    fn base_mut(&mut self) -> &mut BaseSchema { &mut self.base }
}

impl SchemaTrait for DynamicText {
    fn name(&self) -> &str { &self.base.name }
    fn position(&self) -> (Mm, Mm) { (self.base.x, self.base.y) }
    fn size(&self) -> (Mm, Mm) { (self.base.width, self.base.height) }

    fn set_x(&mut self, x: Mm) { self.base.x = x; }
    fn set_y(&mut self, y: Mm) { self.base.y = y; }
    fn set_width(&mut self, width: Mm) { self.base.width = width; }
    fn set_height(&mut self, height: Mm) { self.base.height = height; }

    fn render(&self, parent_height: Mm, doc: &mut PdfDocument, page: usize, buffer: &mut OpBuffer) -> Result<(), Error> {
        // 現在のDynamicText::render実装をそのまま使用
        // ...
        # Ok(())
    }
}
```

#### Table スキーマ

```rust
pub struct Table {
    base: BaseSchema,
    show_head: bool,
    head_width_percentages: Vec<Head>,
    body_styles: BodyStyles,
    table_styles: TableStyles,
    columns: Vec<Schema>,
    fields: Vec<Vec<String>>,
}

impl HasBaseSchema for Table {
    fn base(&self) -> &BaseSchema { &self.base }
    fn base_mut(&mut self) -> &mut BaseSchema { &mut self.base }
}

impl SchemaTrait for Table {
    fn name(&self) -> &str { &self.base.name }
    fn position(&self) -> (Mm, Mm) { (self.base.x, self.base.y) }
    fn size(&self) -> (Mm, Mm) { (self.base.width, self.base.height) }

    fn set_x(&mut self, x: Mm) { self.base.x = x; }
    fn set_y(&mut self, y: Mm) { self.base.y = y; }
    fn set_width(&mut self, width: Mm) { self.base.width = width; }
    fn set_height(&mut self, height: Mm) { self.base.height = height; }

    fn render(&self, parent_height: Mm, doc: &mut PdfDocument, page: usize, buffer: &mut OpBuffer) -> Result<(), Error> {
        // 現在のTable::render実装をそのまま使用
        // ...
        # Ok(())
    }
}
```

### 共通トレイトの追加（オプション）

#### 可視性トレイト

```rust
/// 不透明度や表示/非表示を制御するトレイト
pub trait HasVisibility {
    fn opacity(&self) -> f32;
    fn set_opacity(&mut self, opacity: f32);
    fn is_visible(&self) -> bool;
    fn set_visible(&mut self, visible: bool);
}

// 実装例
impl HasVisibility for Rect {
    fn opacity(&self) -> f32 { self.opacity.unwrap_or(1.0) }
    fn set_opacity(&mut self, opacity: f32) { self.opacity = Some(opacity); }
    fn is_visible(&self) -> bool { self.opacity.unwrap_or(1.0) > 0.0 }
    fn set_visible(&mut self, visible: bool) {
        self.opacity = if visible { None } else { Some(0.0) };
    }
}
```

#### 回転トレイト

```rust
/// 回転を制御するトレイト
pub trait HasRotation {
    fn rotation(&self) -> f32;
    fn set_rotation(&mut self, angle: f32);
    fn rotate_by(&mut self, angle: f32);
}

impl HasRotation for Rect {
    fn rotation(&self) -> f32 { self.rotate.unwrap_or(0.0) }
    fn set_rotation(&mut self, angle: f32) { self.rotate = Some(angle); }
    fn rotate_by(&mut self, angle: f32) {
        self.rotate = Some(self.rotate.unwrap_or(0.0) + angle);
    }
}
```

#### ボーダートレイト

```rust
/// 枠線を制御するトレイト
pub trait HasBorder {
    fn border_color(&self) -> Option<&Color>;
    fn border_width(&self) -> Option<Mm>;
    fn set_border_color(&mut self, color: Option<Color>);
    fn set_border_width(&mut self, width: Option<Mm>);
}
```

---

## 実装の利点

### 1. 型安全なジェネリック処理

```rust
/// すべてのスキーマに共通する処理をジェネリックで記述可能
fn apply_common_transforms<S: SchemaTrait>(schemas: &mut [S], page_height: Mm) {
    for schema in schemas {
        // 共通の変換処理
        let (x, y) = schema.position();
        let (w, h) = schema.size();

        // ページ境界チェック
        if y + h > page_height {
            // ページオーバー処理
        }

        // 共通のスタイル適用
        // ...
    }
}
```

### 2. ダイナミックなスキーマ処理

```rust
/// ランタイムでスキーマタイプを判別して処理
fn process_schema(schema: &mut Schema) {
    match schema {
        Schema::Text(t) => {
            // Text固有の処理
            t.set_content("Updated");
        }
        Schema::DynamicText(dt) => {
            // DynamicText固有の処理
            // ...
        }
        Schema::Table(t) => {
            // Table固有の処理
            // ...
        }
        // ... 他のスキーマ
    }
}

/// トレイトオブジェクトとして処理
fn render_all(schemas: &[&dyn SchemaTrait], doc: &mut PdfDocument) {
    for (i, schema) in schemas.iter().enumerate() {
        schema.render(297.0, doc, i, &mut buffer)?;
    }
}
```

### 3. 拡張性の向上

```rust
/// 新しいスキーマを追加するだけで済む
pub struct Barcode {
    base: BaseSchema,
    content: String,
    format: BarcodeFormat,
}

impl HasBaseSchema for Barcode {
    fn base(&self) -> &BaseSchema { &self.base }
    fn base_mut(&mut self) -> &mut BaseSchema { &mut self.base }
}

impl SchemaTrait for Barcode {
    fn name(&self) -> &str { &self.base.name }
    fn position(&self) -> (Mm, Mm) { (self.base.x, self.base.y) }
    fn size(&self) -> (Mm, Mm) { (self.base.width, self.base.height) }

    fn set_x(&mut self, x: Mm) { self.base.x = x; }
    fn set_y(&mut self, y: Mm) { self.base.y = y; }
    fn set_width(&mut self, width: Mm) { self.base.width = width; }
    fn set_height(&mut self, height: Mm) { self.base.height = height; }

    fn render(&self, parent_height: Mm, doc: &mut PdfDocument, page: usize, buffer: &mut OpBuffer) -> Result<(), Error> {
        // Barcode専用の描画ロジック
        # Ok(())
    }
}

// Schema enum に追加するだけで使用可能
#[derive(Debug, Clone)]
pub enum Schema {
    // ... 既存のvariants
    Barcode(Barcode),
}
```

---

## 移行計画

### ステップ1: トレイトの定義

```rust
// src/schemas/traits.rs (新規作成)
pub mod traits {
    use printpdf::{Mm, PdfDocument};
    use crate::utils::OpBuffer;

    pub trait SchemaTrait {
        fn name(&self) -> &str;
        fn position(&self) -> (Mm, Mm);
        fn size(&self) -> (Mm, Mm);
        fn bounds(&self) -> BoundingBox;
        fn set_x(&mut self, x: Mm);
        fn set_y(&mut self, y: Mm);
        fn set_width(&mut self, width: Mm);
        fn set_height(&mut self, height: Mm);
        fn render(&self, parent_height: Mm, doc: &mut PdfDocument, page: usize, buffer: &mut OpBuffer) -> Result<(), Error>;
    }

    pub trait HasBaseSchema {
        fn base(&self) -> &BaseSchema;
        fn base_mut(&mut self) -> &mut BaseSchema;
    }

    // ... その他のトレイト
}
```

### ステップ2: 各スキーマの移行

| スキーマ | 移行ステータス |
|---------|--------------|
| Text | TODO |
| DynamicText | TODO |
| Table | TODO |
| QrCode | TODO |
| Image | TODO |
| Svg | TODO |
| Rect | TODO |
| Line | TODO |
| Group | TODO |

### ステップ3: Schema enum の更新

```rust
// src/schemas/mod.rs
pub use traits::{SchemaTrait, HasBaseSchema};

#[derive(Debug, Clone)]
pub enum Schema {
    Text(text::Text),
    DynamicText(dynamic_text::DynamicText),
    Table(table::Table),
    QrCode(qrcode::QrCode),
    Image(image::Image),
    Svg(svg::Svg),
    Rect(rect::Rect),
    Line(line::Line),
    Group(group::Group),
    // 新しいスキーマもここに追加
}
```

### ステップ4: 互換性レイヤーの追加

```rust
// src/schemas/mod.rs
impl Schema {
    pub fn render_legacy(
        &self,
        parent_height: Mm,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        // 既存のrender実装を呼び出す
        match self {
            Schema::Text(s) => s.render(parent_height, page, buffer),
            Schema::DynamicText(s) => s.render(parent_height, doc, page, buffer),
            Schema::Table(s) => s.render(parent_height, doc, page, buffer),
            // ...
        }
    }
}
```

---

## 設計上の考慮事項

### 1. Clone の扱い

```rust
// トレイトメソッドでは &self を使用
fn render(&self, ...) -> Result<(), Error>;

// ただし、内部で mutable な操作が必要な場合は
// または Arc<Mutex<>> の使用を検討
```

### 2. Error の返し方

```rust
// 現在の実装は Result<(), Error> を返す
// トレイトでも同様に統一
```

### 3. パフォーマンス

```rust
// 呼び出しオーバーヘッドを気にする場合は
// inline 属性を追加

#[inline]
fn set_x(&mut self, x: Mm) { self.base.x = x; }
```

### 4. ドキュメント

```rust
/// スキーマの共通インターフェース
///
/// すべてのスキーマタイプはこのトレイトを実装する必要があります。
/// これにより、共通の処理ロジックをジェネリックに記述できます。
///
/// # Example
///
/// ```ignore
/// fn render_schemas<S: SchemaTrait>(schemas: &[S]) {
///     for schema in schemas {
///         schema.render(...)?;
///     }
/// }
/// ```
pub trait SchemaTrait {
    // ...
}
```

---

## まとめ

### 改善後のアーキテクチャの利点

| 項目 | 現状 | 改善後 |
|-----|------|-------|
| コード重複 | 各スキーマで個別実装 | トレイトで共通化 |
| 拡張性 | enum 全パターン修正必要 | 新スキーマ追加のみ |
| 型安全性 | enum による分岐 | trait object による動的_dispatch |
| テスト容易性 | 各実装を個別テスト | trait mock で一括テスト |
| ドキュメント | 各構造体に分散 | trait に集約 |

### 移行のリスクと緩和

| リスク | 缓和策 |
|-------|-------|
| ブレイキングチェンジ | 互換性レイヤーを維持 |
| パフォーマンス低下 | inline 属性で最適化 |
| 移行コスト | ステップごとの段階的移行 |

---

## 参考: 実装チェックリスト

- [ ] `src/schemas/traits.rs` を作成
- [ ] `SchemaTrait` トレイトを定義
- [ ] `HasBaseSchema` トレイトを定義
- [ ] `Text` を移行
- [ ] `DynamicText` を移行
- [ ] `Table` を移行
- [ ] `QrCode` を移行
- [ ] `Image` を移行
- [ ] `Svg` を移行
- [ ] `Rect` を移行
- [ ] `Line` を移行
- [ ] `Group` を移行
- [ ] `Schema` enum を更新
- [ ] 既存の `SchemaTrait` を削除
- [ ] テストを更新
- [ ] ドキュメントを更新