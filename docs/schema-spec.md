# PDForge テンプレート スキーマ仕様

このドキュメントは PDForge のテンプレート JSON における全スキーマ型のプロパティ仕様を定義します。

---

## テンプレートの基本構造

```json
{
    "schemas": [ /* ページごとのスキーマ配列 */ ],
    "basePdf": { /* ページ設定 */ },
    "schemaVersion": "1.0.0"
}
```

### `basePdf`

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `width` | `number` | ✓ | ページ幅 (mm) |
| `height` | `number` | ✓ | ページ高さ (mm) |
| `padding` | `[top, right, bottom, left]` | ✓ | ページ余白 (mm)、4要素の配列 |
| `staticSchema` | `Schema[]` | - | 全ページに共通で描画されるスキーマ |

### `schemas`

各ページはスキーマオブジェクトの配列。複数ページの場合は配列の配列になる。

```json
"schemas": [
    [ /* 1ページ目のスキーマ */ ],
    [ /* 2ページ目のスキーマ */ ]
]
```

---

## 共通プロパティ

すべてのスキーマ型に共通するプロパティ：

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `name` | `string` | ✓ | スキーマの識別子 |
| `type` | `string` | ✓ | スキーマの種類（後述） |
| `position` | `{ x: number, y: number }` | ✓ | 左上を原点とした座標 (mm) |
| `width` | `number` | ✓ | 幅 (mm) |
| `height` | `number` | ✓ | 高さ (mm) |

---

## スキーマ種別一覧

| `type` 値 | 説明 |
|---|---|
| `text` | 静的テキスト |
| `dynamicText` | テンプレート変数付きテキスト（複数ページ対応） |
| `table` | 表 |
| `qrCode` | QRコード |
| `image` | 画像（PNG/JPEG/BMP） |
| `svg` | SVGグラフィック |
| `rectangle` | 矩形 |
| `line` | 線 |
| `group` | 複数スキーマのグループ |

---

## 1. `text` — 静的テキスト

```json
{
    "type": "text",
    "name": "myText",
    "position": { "x": 10, "y": 20 },
    "width": 100,
    "height": 30,
    "content": "表示するテキスト",
    "fontName": "NotoSansJP",
    "fontSize": 12
}
```

| プロパティ | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `content` | `string` | ✓ | | 表示するテキスト内容 |
| `fontName` | `string` | ✓ | | 使用フォント名（事前ロードが必要） |
| `fontSize` | `number \| FontSizeObject` | ✓ | | フォントサイズ (pt)、または動的サイズ設定（後述） |
| `alignment` | `"left" \| "center" \| "right" \| "justify"` | - | `"left"` | 水平方向のテキスト配置 |
| `verticalAlignment` | `"top" \| "middle" \| "bottom"` | - | `"top"` | 垂直方向のテキスト配置 |
| `fontColor` | `string` | - | `"#000000"` | フォント色（CSS色指定） |
| `backgroundColor` | `string` | - | なし | 背景色（CSS色指定） |
| `borderColor` | `string` | - | なし | 枠線色（CSS色指定）。`borderWidth` と組み合わせて使用 |
| `borderWidth` | `number` | - | なし | 枠線の太さ (pt)。`borderColor` と組み合わせて使用 |
| `padding` | `Frame` | - | なし | テキスト内側の余白（後述） |
| `characterSpacing` | `number` | - | `0.0` | 文字間隔 (pt) |
| `lineHeight` | `number` | - | `1.0` | 行の高さ倍率 |
| `rotate` | `number` | - | なし | 回転角度（度数） |
| `scaleX` | `number` | - | なし | X軸方向の拡縮率 |
| `scaleY` | `number` | - | なし | Y軸方向の拡縮率 |

### `fontSize` — 動的サイズ指定

固定サイズの代わりに、ボックス内に収まるよう自動調整する場合：

```json
"fontSize": {
    "min": 8,
    "max": 24,
    "fit": true
}
```

| プロパティ | 型 | 説明 |
|---|---|---|
| `min` | `number` | 最小フォントサイズ (pt) |
| `max` | `number` | 最大フォントサイズ (pt) |
| `fit` | `boolean` | `true` の場合、ボックスに収まるよう自動調整 |

### `Frame` — 余白・枠線幅

```json
{
    "top": 5,
    "right": 5,
    "bottom": 5,
    "left": 5
}
```

各辺に個別の値を設定できる (mm)。

---

## 2. `dynamicText` — 動的テキスト（テンプレート変数対応）

テキスト内容に [Tera テンプレート構文](https://keats.github.io/tera/) を使用できる。
コンテンツがページを超える場合、自動的に次のページに折り返す。

```json
{
    "type": "dynamicText",
    "name": "myDynamicText",
    "position": { "x": 10, "y": 20 },
    "width": 100,
    "height": 200,
    "content": "こんにちは、{{ name }} さん！",
    "fontName": "NotoSansJP",
    "fontSize": 12
}
```

| プロパティ | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `content` | `string` | ✓ | | テキスト内容（Tera 構文使用可） |
| `fontName` | `string` | ✓ | | 使用フォント名 |
| `fontSize` | `number` | ✓ | | フォントサイズ (pt) |
| `characterSpacing` | `number` | - | `0.0` | 文字間隔 (pt) |
| `lineHeight` | `number` | - | `1.0` | 行の高さ倍率 |

> **制限事項:**
> - `fontSize` は必須。省略するとパニックが発生する（未実装）
> - `fontColor`、`alignment`、`verticalAlignment`、`padding` は未対応（テキスト色は常に黒）

---

## 3. `table` — 表

```json
{
    "type": "table",
    "name": "myTable",
    "position": { "x": 10, "y": 30 },
    "width": 190,
    "height": 100,
    "showHead": true,
    "headWidthPercentages": [...],
    "headStyles": {...},
    "bodyStyles": {...},
    "tableStyles": {...},
    "columns": [...],
    "fields": [...]
}
```

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `showHead` | `boolean` | ✓ | ヘッダー行を表示するか |
| `headWidthPercentages` | `HeadColumn[]` | ✓ | 列定義（幅のパーセンテージ合計は 100 でなければならない） |
| `headStyles` | `HeadStyles` | ✓ | ヘッダー行のスタイル |
| `bodyStyles` | `BodyStyles` | ✓ | データ行のスタイル |
| `tableStyles` | `TableStyles` | ✓ | テーブル全体の枠線スタイル |
| `columns` | `CellStyle[]` | ✓ | 列ごとのセルスキーマ定義 |
| `fields` | `string[][]` | ✓ | テーブルデータ（行 × 列の文字列配列）。API からの動的注入も可能 |

### `HeadColumn` — 列定義

```json
{
    "percent": 30,
    "content": "列タイトル",
    "fontSize": 12,
    "fontName": "NotoSansJP",
    "alignment": "center",
    "verticalAlignment": "middle",
    "characterSpacing": 0
}
```

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `percent` | `number` | ✓ | 列の幅（全列合計 = 100） |
| `content` | `string` | ✓ | ヘッダーのテキスト |
| `fontSize` | `number` | - | 個別フォントサイズ（未指定時は `headStyles.fontSize` を使用） |
| `fontName` | `string` | - | 個別フォント名（未指定時は `headStyles.fontName` を使用） |
| `alignment` | `Alignment` | - | 個別水平配置（未指定時は `headStyles.alignment` を使用） |
| `verticalAlignment` | `VerticalAlignment` | - | 個別垂直配置（未指定時は `headStyles.verticalAlignment` を使用） |
| `characterSpacing` | `number` | - | 個別文字間隔 |

### `HeadStyles` — ヘッダースタイル

| プロパティ | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `fontSize` | `number` | ✓ | | フォントサイズ (pt) |
| `fontName` | `string` | ✓ | | フォント名 |
| `fontColor` | `string` | ✓ | | フォント色（CSS色） |
| `backgroundColor` | `string` | ✓ | | 背景色（CSS色） |
| `borderColor` | `string` | ✓ | | 枠線色（CSS色） |
| `borderWidth` | `Frame` | ✓ | | 各辺の枠線幅 (mm) |
| `padding` | `Frame` | ✓ | | 各辺の余白 (mm) |
| `alignment` | `Alignment` | - | `"left"` | 水平配置 |
| `verticalAlignment` | `VerticalAlignment` | - | `"middle"` | 垂直配置 |
| `lineHeight` | `number` | - | `1.0` | 行の高さ倍率 |
| `characterSpacing` | `number` | - | `0.0` | 文字間隔 (pt) |

### `BodyStyles` — データ行スタイル

| プロパティ | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `fontSize` | `number` | ✓ | | フォントサイズ (pt) |
| `fontName` | `string` | ✓ | | フォント名 |
| `fontColor` | `string` | ✓ | | フォント色（CSS色） |
| `backgroundColor` | `string` | ✓ | | 偶数行（0, 2, 4...）の背景色（CSS色） |
| `borderColor` | `string` | ✓ | | 枠線色（CSS色） |
| `borderWidth` | `Frame` | ✓ | | 各辺の枠線幅 (mm) |
| `padding` | `Frame` | ✓ | | 各辺の余白 (mm) |
| `alignment` | `Alignment` | ✓ | | 水平配置 |
| `verticalAlignment` | `VerticalAlignment` | ✓ | | 垂直配置 |
| `lineHeight` | `number` | ✓ | | 行の高さ倍率 |
| `alternateBackgroundColor` | `string` | - | なし | 奇数行（1, 3, 5...）の背景色（交互カラー行） |
| `characterSpacing` | `number` | - | `0.0` | 文字間隔 (pt) |

### `TableStyles` — テーブルスタイル

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `borderWidth` | `number` | ✓ | 外枠の太さ (mm) |
| `borderColor` | `string` | ✓ | 外枠の色（CSS色） |

### `CellStyle` — セル定義

```json
{
    "schema": { "type": "text", ... },
    "height": 10.0
}
```

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `schema` | `TextSchema \| QrCodeSchema` | ✓ | セルのスキーマ（`text` または `qrCode` のみ対応） |
| `height` | `number` | - | セルの最小高さ (mm) |

> **制限事項:** セルに使えるスキーマは `text` と `qrCode` のみ。それ以外はパニックが発生する。

---

## 4. `qrCode` — QRコード

```json
{
    "type": "qrCode",
    "name": "myQr",
    "position": { "x": 10, "y": 20 },
    "width": 30,
    "height": 30,
    "content": "https://example.com"
}
```

| プロパティ | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `content` | `string` | ✓ | | QRコードに埋め込むデータ |
| `alignment` | `Alignment` | - | `"left"` | バウンディングボックス内での水平配置 |
| `verticalAlignment` | `VerticalAlignment` | - | `"top"` | バウンディングボックス内での垂直配置 |
| `padding` | `Frame` | - | なし | 内側の余白 |
| `rotate` | `number` | - | なし | 回転角度（度数） |

---

## 5. `image` — 画像

```json
{
    "type": "image",
    "name": "myImage",
    "position": { "x": 10, "y": 20 },
    "width": 60,
    "height": 40,
    "content": "data:image/png;base64,iVBORw0KGgo...",
    "objectFit": "contain"
}
```

| プロパティ | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `content` | `string` | ✓ | | 画像データ（Base64 データURL形式: `data:image/png;base64,...`） |
| `objectFit` | `ObjectFit` | - | `"fill"` | 画像のスケーリング方法 |

### `objectFit` の値

| 値 | 説明 |
|---|---|
| `"fill"` | 幅・高さに合わせて伸縮（アスペクト比が変わる場合あり） |
| `"contain"` | アスペクト比を保ちながら収まるよう縮小・拡大（余白が生じる） |
| `"cover"` | アスペクト比を保ちながらボックス全体を埋める（はみ出た部分はクリップ） |
| `"none"` | 元のピクセルサイズで表示（300 DPI 基準） |
| `"scale-down"` | 画像がボックスより大きい場合のみ `contain` と同様に縮小、小さい場合はそのまま表示 |

> **対応フォーマット:** PNG、JPEG、BMP

---

## 6. `svg` — SVGグラフィック

```json
{
    "type": "svg",
    "name": "mySvg",
    "position": { "x": 10, "y": 20 },
    "width": 80,
    "height": 80,
    "content": "<svg xmlns=\"http://www.w3.org/2000/svg\">...</svg>"
}
```

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `content` | `string` | ✓ | SVG マークアップ文字列 |

> **制限事項:** 回転・配置・パディング等のオプションは未対応。SVG 内部のスタイルのみが適用される。

---

## 7. `rectangle` — 矩形

```json
{
    "type": "rectangle",
    "name": "myRect",
    "position": { "x": 10, "y": 20 },
    "width": 80,
    "height": 40,
    "color": "#FFFFFF",
    "borderColor": "#000000",
    "borderWidth": 1
}
```

| プロパティ | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `color` | `string` | ✓ | | 塗りつぶし色（CSS色指定） |
| `borderColor` | `string` | ✓ | | 枠線色（CSS色指定） |
| `borderWidth` | `number` | - | `1.0` | 枠線の太さ (pt) |
| `rotate` | `number` | - | なし | 回転角度（度数） |
| `opacity` | `number` | - | なし | **注意: 解析されるが描画には反映されない（未実装）** |

---

## 8. `line` — 線

```json
{
    "type": "line",
    "name": "myLine",
    "position": { "x": 10, "y": 50 },
    "width": 190,
    "color": "#000000"
}
```

| プロパティ | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `color` | `string` | ✓ | | 線の色（CSS色指定） |
| `borderWidth` | `number` | - | `1.0` | 線の太さ (pt) |
| `height` | `number` | - | `0.0` | 通常 0（1次元要素のため） |
| `rotate` | `number` | - | なし | 回転角度（度数） |
| `opacity` | `number` | - | なし | **注意: 解析されるが描画には反映されない（未実装）** |

---

## 9. `group` — グループ

複数のスキーマをまとめ、グループとして変換（回転など）を適用できる。

```json
{
    "type": "group",
    "name": "myGroup",
    "position": { "x": 10, "y": 20 },
    "width": 100,
    "height": 50,
    "rotate": 45,
    "schemas": [
        { "type": "text", ... },
        { "type": "rectangle", ... }
    ]
}
```

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `schemas` | `Schema[]` | ✓ | グループに含めるスキーマの配列 |
| `rotate` | `number` | - | グループ全体の回転角度（度数） |

> **制限事項:** グループ内に `group` をネストすることはできない（パニックが発生する）。
> `line` スキーマもグループ内では非対応（パニックが発生する）。

### グループ内で使用可能なスキーマ

| スキーマ | 対応 |
|---|---|
| `text` | ✓ |
| `dynamicText` | ✓ |
| `table` | ✓ |
| `qrCode` | ✓ |
| `image` | ✓ |
| `svg` | ✓ |
| `rectangle` | ✓ |
| `line` | ✗（パニック） |
| `group` | ✗（パニック） |

---

## `staticSchema` — 全ページ共通スキーマ

`basePdf.staticSchema` に定義したスキーマは、すべてのページに描画される。
テンプレート変数として以下の特殊変数が使用可能：

| 変数 | 説明 | 例 |
|---|---|---|
| `{{ currentPage }}` | 現在のページ番号（1始まり） | `1` |
| `{{ totalPages }}` | 総ページ数 | `5` |
| `{{ date }}` | 現在の日付 | `2026-03-15` |
| `{{ dateTime }}` | 現在の日時 | `2026-03-15 10:30:00` |

加えて、`render()` 呼び出し時の `static_inputs` で渡したカスタム変数も使用できる。

```json
"staticSchema": [
    {
        "name": "pageNumber",
        "type": "text",
        "content": "{{ currentPage }} / {{ totalPages }}",
        "position": { "x": 180, "y": 280 },
        "width": 20,
        "height": 10,
        "fontSize": 9,
        "fontName": "NotoSansJP"
    }
]
```

---

## 色指定

色プロパティはすべて CSS の色文字列を受け付ける。以下の形式が有効：

- 16進数: `"#RRGGBB"` または `"#RGB"`（例: `"#FF0000"`, `"#f00"`）
- RGB関数: `"rgb(255, 0, 0)"`
- 色名: `"red"`, `"white"` など

---

## 既知の制限・注意事項

| 種別 | 内容 |
|---|---|
| `dynamicText.fontSize` | 省略不可。省略するとパニックが発生する（未実装） |
| `rectangle.opacity` / `line.opacity` | JSON として解析されるが、実際の描画には反映されない |
| `table` のセル型 | `text` と `qrCode` のみ対応。他のスキーマ型を指定するとパニックが発生する |
| `group` 内のスキーマ | `line` と `group` は非対応。指定するとエラーになる |
| `dynamicText` のスタイル | `fontColor`、`alignment`、`verticalAlignment`、`padding` は未対応 |

---

## テンプレート変数（Tera 構文）

`content` フィールドに Tera テンプレート変数を使用できる（`dynamicText`、`staticSchema`）。

```
{{ variable_name }}
```

変数は `render()` API の `inputs` パラメータ、または `static_inputs` で渡す。

```rust
pdforge.render(
    "template",
    vec![vec![
        HashMap::from([("name", "田中太郎".to_string())])
    ]],
    None,
    Some(HashMap::from([("company", "PDForge株式会社".to_string())]))
)
```
