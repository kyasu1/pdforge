# テーブルスタイリング マイグレーションガイド（次リリース）

次リリースで `table` スキーマのスタイリングを整理し、これまで**JSON に書けるのに描画へ反映されていなかったフィールド**を実際に機能させました。あわせて、効果を持たなかったフィールドをスキーマから削除しました。

このガイドは既存テンプレートの更新手順をまとめたものです。

## TL;DR

- パースは寛容なままです（**未知フィールドは無視**）。削除したフィールドを残していてもエラーにはなりません。ただし効果はありません。
- 一部の**見た目が変わります**。特にヘッダー行の背景・枠・文字色。意図した表示を保つには下記の対応が必要です。

---

## 1. 削除したフィールド

以下は解析されるだけで一切効果がなかったため、スキーマから削除しました。テンプレートから削除して構いません（残っていても無視されます）。

| 場所 | 削除フィールド | 理由 |
|---|---|---|
| `bodyStyles` | `fontSize` / `fontName` | 本文フォントは各列（`columns[].schema`）が必ず持つため未使用だった |
| `bodyStyles` | `borderColor` / `borderWidth` | データ行の枠線は `tableStyles` が描画するため未使用だった |
| `columns[]`（`CellStyle`） | `height` | 未実装。行の高さはセル内容から自動計算される |

> リポジトリ同梱の `templates/*.json` はすでにこれらを削除済みです。

---

## 2. 挙動が変わるフィールド（要確認）

これまで無視されていた値が描画に反映されるようになりました。既存テンプレートの見た目が変わる可能性があります。

### 2-1. ヘッダー背景 — `headStyles.backgroundColor`

- **旧:** ヘッダー背景は誤って `bodyStyles.backgroundColor` を使用していた。
- **新:** `headStyles.backgroundColor` を使用する。
- **対応:** ヘッダー背景を意図した色にするには `headStyles.backgroundColor` を設定する（多くのテンプレートは既に設定済み）。

### 2-2. ヘッダー文字色・文字間隔・行高 — `headStyles.fontColor` / `characterSpacing` / `lineHeight`

- **旧:** ヘッダー文字は常に黒・文字間隔0で描画されていた。
- **新:** `headStyles.fontColor` / `characterSpacing` / `lineHeight` が反映される。`characterSpacing` は列（`HeadColumn.characterSpacing`）で個別上書き可能。`fontColor` と `lineHeight` は `headStyles` 側のみで、列ごとの上書きはできない。
- **対応:** 白抜きヘッダー等を使う場合は `headStyles.fontColor` を設定する。

### 2-3. ヘッダー枠線 — `headStyles.borderColor` / `borderWidth`（**最重要**）

- **旧:** ヘッダー枠線は `tableStyles.borderWidth` の黒枠で描かれていた（`headStyles.borderWidth` は無視）。
- **新:** `headStyles.borderColor` / `borderWidth`（四辺個別の `Frame`）で描かれる。**`borderWidth` が 0 ならヘッダーに枠線は付かない。**
- **対応:** 従来どおりヘッダーに枠線を付けたい場合は `headStyles.borderWidth` と `borderColor` を設定する。例：

  ```jsonc
  // 旧: ヘッダーは tableStyles の 0.3mm 黒枠で描かれていた
  "headStyles": {
    "borderColor": "",
    "borderWidth": { "top": 0, "right": 0, "bottom": 0, "left": 0 }
  }

  // 新: 同じ見た目にするには明示する
  "headStyles": {
    "borderColor": "#000000",
    "borderWidth": { "top": 0.3, "right": 0.3, "bottom": 0.3, "left": 0.3 }
  }
  ```

  非対称な枠（例：下罫線だけ太く）も四辺個別に指定できます。

### 2-4. データ行の枠線色 — `tableStyles.borderColor`

- **旧:** データ行の枠線は色指定に関わらず常に黒だった。
- **新:** `tableStyles.borderColor` の色で描かれる。
- **対応:** 黒以外にしたい場合は `tableStyles.borderColor` を設定する（従来から黒指定なら変化なし）。

### 2-5. データ行のスタイル継承 — `bodyStyles`

- **新:** 列（`columns[].schema`）が `alignment` / `verticalAlignment` / `characterSpacing` / `lineHeight` / `fontColor` / `padding` / `lineBreakMode` を省略した場合、`bodyStyles` の対応値が既定として使われる。
- 背景色・枠線は従来どおりセル矩形側で描画され、`bodyStyles` の値が列テキストへ注入されることはない（交互背景の消失や枠の二重描画を避けるため）。
- **対応:** 列側で明示している場合は従来どおり列の値が優先されるため、通常は対応不要。

---

## 3. 空文字カラーの扱い

`headStyles` の色（`fontColor` / `borderColor` / `backgroundColor`）に空文字 `""` または空白のみを指定した場合、「未指定」とみなしてフォールバックします（`fontColor` / `borderColor` → `#000000`、`backgroundColor` → `#ffffff`）。

- `borderWidth: 0` と `borderColor: ""` の組み合わせ（＝枠なし）という既存テンプレートの慣例をそのまま扱えます。
- **空でない不正な CSS 色**（例：`"not-a-color"`）は従来どおりエラーになります。

---

## 4. チェックリスト

- [ ] `bodyStyles` から `fontSize` / `fontName` / `borderColor` / `borderWidth` を削除
- [ ] `columns[]` から `height` を削除
- [ ] ヘッダーに枠線が必要なら `headStyles.borderWidth` / `borderColor` を明示
- [ ] ヘッダー背景・文字色が意図どおりか確認（`headStyles.backgroundColor` / `fontColor`）
- [ ] データ行の枠線色を使うなら `tableStyles.borderColor` を確認
- [ ] 生成 PDF を目視確認
