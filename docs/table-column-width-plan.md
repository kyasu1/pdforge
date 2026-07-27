# テーブル列幅仕様の刷新 — `headWidthPercentages` → `columns[].width`

## Context

現状の table スキーマは列幅を `headWidthPercentages[].percent` だけで指定し、`Table::from_json` (`src/schemas/table.rs:279-287`) が **f32 の完全一致**で合計 100 を要求している。これは pdfme から受け継いだ仕様だが、主要な帳票ライブラリを調査した結果、**パーセントのみを強制する設計は他に一つも存在しない**ことが分かった。

| ライブラリ | 列幅の指定方法 |
|---|---|
| pdfmake | `widths: [100, '*', 'auto', '20%']` |
| Typst | `columns: (3cm, 1fr, auto, 20%)` |
| ReportLab | `colWidths=[80, None, '30%']` |
| dart pdf | `FixedColumnWidth / FlexColumnWidth / IntrinsicColumnWidth` |
| jsPDF-AutoTable | `cellWidth: 'auto' \| 'wrap' \| 50` |
| CSS (WeasyPrint 等) | `width: 20% / 3cm / auto` |
| WPF DataGrid | `Auto \| * \| 2* \| 100` |
| LaTeX tabularx | `p{3cm}` + `X` |

共通する事実上の標準は「**固定長 / 残余按分(fr・star) / 内容依存(auto)** の union を列ごとに指定」。

現行方式の実害:
- 合計 100 の手計算負担。既存 15 テーブル中 8 つが `17/18`、`31/17/14`、`11/12` のような「100 に合わせるために目分量で調整した」値になっている
- `33.33 × 3` が f32 完全一致で弾かれる
- 「この列だけ 25mm 固定、残りは均等」が表現不能
- 列の追加・削除で全列の値を再計算する必要がある

さらに構造上の問題として、**幅・ヘッダ文言・セルスキーマが3つの別配列に散っている**:
`headWidthPercentages[]`（幅＋ヘッダ）、`columns[]`（セルスキーマ）、`fields[][]`（データ）。列数は `headWidthPercentages.len()` が定義するのに、`process_row` は `self.columns[col_index]` を**添字アクセス**しており (`table.rs:495`)、行のセル数が列数を超えると panic する。3者の整合性はどこでも検証されていない。

**目指す結果**: 列定義を `columns[]` に一元化し、幅を `固定mm / fr / %` の union で指定できるようにする。`auto`（内容依存）はスコープ外（後続の別 PR）。

## 決定事項

1. **`columns[]` に一元化する破壊的変更**（pdfme JSON 互換はすでに不要と確認済み）。旧形式のフォールバックは実装しない — パーサに 2 系統を残すと 1300 行の `table.rs` がさらに膨らむため
2. **`width` は `固定mm / fr / %` の3種のみ**。`auto` は除外 — 全行のテキスト測定パス、min/max クランプ、複数ページでの列幅安定化の設計が必要になり、スコープが跳ね上がるため

## 新しい JSON 形式

```json
{
  "type": "table",
  "name": "items",
  "position": { "x": 10, "y": 10 },
  "width": 190,
  "height": 52.9,
  "showHead": true,
  "tableStyles": { "borderWidth": 0.3, "borderColor": "#000000" },
  "headStyles": { "...": "変更なし" },
  "bodyStyles": { "...": "変更なし" },
  "columns": [
    {
      "width": 15,
      "header": { "content": "No", "alignment": "center" },
      "cell":   { "type": "text", "alignment": "right" }
    },
    {
      "width": "2fr",
      "header": { "content": "品名" },
      "cell":   { "type": "text", "alignment": "left" }
    },
    {
      "width": "20%",
      "header": { "content": "金額", "alignment": "center" },
      "cell":   { "type": "text", "alignment": "right" }
    }
  ],
  "fields": [["1", "ボルト", "1,200"]]
}
```

- `header` は旧 `headWidthPercentages[]` の要素から `percent` を除いたもの（`content` / `fontName` / `fontSize` / `alignment` / `verticalAlignment` / `characterSpacing` / `lineBreakMode`）。既存の `JsonHead` をほぼ流用できる
- `cell` は旧 `columns[].schema` を1段フラットにしたもの（`JsonCellStyle` のラッパを削除）
- `headWidthPercentages` と `JsonCellStyle` は削除

### `width` の文法

| 記法 | 意味 |
|---|---|
| `25` (number) | 25mm 固定 |
| `"25mm"` | 同上（明示） |
| `"20%"` | テーブル有効幅の 20% |
| `"2fr"` / `"1fr"` / `"fr"` | 残余幅を fr 比で按分（`"fr"` = `"1fr"`） |

`*`（pdfmake / WPF の star 記法）はサポートしない — `fr` 一本に絞る。

### 解決アルゴリズム

`cell_widths()` (`table.rs:250-262`) を `resolve_column_widths()` に置き換える。純関数 `(&self, &BasePdf) -> Vec<Mm>` のまま、呼び出しは `render` 内の 1 箇所 (`table.rs:559`) のみ。

**内部計算はすべて f64 で行い、最後に `Mm(f32)` へ落とす。** 旧実装との f32 ビット一致は要件ではない（後方互換不要）。f64 で通すことで、合計 100% のテンプレートが f32 の丸め誤差だけで overflow 判定に落ちる事故を構造的に防ぐ。

1. `table_width = min(self.base.width, base_pdf.width - padding.left - padding.right)`（現行の計算をそのまま踏襲）
2. `Fixed(mm)` → そのまま採用
3. `Percent(p)` → `table_width * p / 100`
4. `allocated = Σfixed + Σpercent`
5. **overflow 判定は許容誤差付きで行う**: `allocated > table_width + EPSILON` のときのみ overflow とみなす。`EPSILON` は `table_width` に対する相対値（例: `table_width * 1e-6`）とする
   - 判定を素の `>` にすると、合計 100% でも各列を割り当ててから足し戻した丸め誤差だけで overflow 扱いになり得る。f64 集計でこの余地はほぼ消えるが、判定そのものにも余裕を持たせて二重に防ぐ
6. **overflow のとき**: **fixed と percent のみ**を `table_width / allocated` で比例縮小し、**fr 列は 0 幅**になる（`remaining = 0` のため）。エラーにはしない — `table_width` は `basePdf.padding` に依存するため、用紙設定の変更が実行時エラーに化けるのを避ける
7. **overflow でないとき**: `remaining = table_width - allocated`（`EPSILON` 以下の負値は 0 にクランプ）、`Fraction(f)` → `remaining * f / Σfr`
8. **fr 列が存在せず余りが出る場合**: テーブルは `table_width` より狭くなる。左寄せのままとし、既存の右端クランプ (`table.rs:432-440`, `:482-490`) がそのまま機能する
9. **丸め残差の補正**:
   - **fr 列がある かつ overflow でない場合のみ**補正する。補正先は**最も幅の大きい fr 列**（最後の fr 列ではない）。極小 weight の fr 列に残差を寄せると負幅になり得るため
   - **fr 列がない場合、および overflow の場合は補正しない**。overflow 時は fr = 0 が仕様であり、そこへ残差を寄せると負幅になる
   - fixed / percent 列は宣言された幅そのものを守るため、残差補正の対象にしない
   - **補正は f64 計算の途中ではなく、各幅を `Mm(f32)` に落とした後の出力ベクタに対して行う。** f64 上で合計を合わせてから一括 cast すると、cast 時の f32 丸め誤差が再発して補正が無意味になる。実装は `target_f32 - Σwidths_f32` を最大 fr 列に加算する形にする
   - **不変条件**: 補正後の全列幅が `is_finite()` かつ `>= 0` であること。テストで固定する

### テーブル幅の下限

`BaseSchema.width` はどこでも検証されていないため、負の `width` を持つスキーマは
`table_width` を負にし、overflow の縮小係数を反転させて負の列幅を生む。
`effective_width()` で `.max(Mm(0.0))` にクランプし、「全列幅 >= 0」の不変条件を守る。

### 縮退ケースの扱い

有効幅 0（`basePdf.padding` が用紙幅以上）や、手順 7 で 0 幅になった fr 列では、セル幅 0 でテキスト測定が走る。`FontSpec::push_grapheme_wrapped_lines` (`src/font.rs:190-210`) は先頭クラスタを無条件に受理する（`:193-194`）ため**無限ループにはならない**が、1 グラフェムクラスタ = 1 行になり行高が異常に大きくなる。この挙動は許容し、ハングしないことをテストで固定する。

### パース時バリデーション

- `columns` が空 → エラー
- `width` の文字列がパース不能 → エラー（受理する記法をメッセージに列挙）
- **全 variant で `is_finite()` を必須**とする。`f64::from_str` は `"NaN"` / `"inf"` を受理してしまうため、これを通すと `Σfr = NaN` で全列幅が NaN になる
- **上限 `f32::MAX` も必須**。個々の値が有限でも解決時の合算で無限大に到達し得る（`["1e308fr", "1e308fr"]` で `Σfr = inf`、`inf / inf = NaN`）。最終表現が `Mm(f32)` なのでこの上限が自然で、現実的な列数では f64 の合算も溢れない
- **`Fixed` / `Percent` / `Fraction` はいずれも `> 0` を要求**する。特に `Fraction(0)` は全列が 0fr のとき `Σfr = 0` でゼロ除算になり、`Fixed(0)` / `Percent(0)` は上記の縮退ケース（1グラフェム1行）を招く
- `Percent > 100` は**許可**する。単独では overflow となり手順 6 の比例縮小が効く。この挙動を仕様として明記する
- **合計 100% の制約は撤廃**
- **新規**: `fields` の各行の要素数が `columns.len()` と一致するか検証し、不一致ならエラー。現状は `table.rs:495` の添字アクセスで panic する（今回 `columns` を触るついでに塞ぐ）

## 実装

### `src/schemas/table.rs`（主戦場）

**新規型**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnWidth {
    Fixed(Mm),
    Percent(f32),
    Fraction(f32),
}

// serde 受け口: number か string を受ける
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum JsonColumnWidth { Number(f32), Text(String) }
// -> ColumnWidth への変換関数（"25mm" / "20%" / "2fr" / "fr" をパース）

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonColumn {
    width: JsonColumnWidth,
    header: JsonHead,     // 既存 JsonHead から percent を削除
    cell: JsonSchema,     // JsonCellStyle のラッパを削除
}

// 解決済みの列。旧 Head + columns の2本の Vec を1本に統合
#[derive(Debug, Clone)]
struct Column {
    width: ColumnWidth,
    header: text::Text,
    cell: Schema,
}
```

**変更箇所**

| 場所 | 変更内容 |
|---|---|
| `JsonTableSchema` (`:169-184`) | `head_width_percentages` + `columns` → `columns: Vec<JsonColumn>` |
| `Head` (`:205-209`) | 削除（`Column` に統合） |
| `Table` (`:211-221`) | `head_width_percentages` + `columns` → `columns: Vec<Column>` |
| `cell_widths` (`:250-262`) | `resolve_column_widths` に置換 |
| `from_json` (`:264-350`) | 合計100検証 (`:279-287`) を削除。ヘッダ Text 構築 (`:289-321`) とセルスキーマ構築 (`:324-336`) を `columns` の1ループに統合 |
| `from_json` (`:294-297`) | ヘッダ Text の初期幅が `json.width * percent / 100`（パディング未考慮）というデッドコード兼トラップ。`Mm(0.0)` にして `create_header_row` の `set_width` に一本化する |
| `create_header_row` (`:444`) | `self.head_width_percentages.iter()` → `self.columns.iter()`、`head.text` → `col.header` |
| `process_row` (`:495`) | `&self.columns[col_index]` → `&self.columns[col_index].cell`（長さ検証済みなので安全） |
| `render` (`:559`) | 呼び出し名の差し替えのみ |

`render_row_immediately` / ページ分割ロジック / 行高さ計算は**無変更**。幅は依然としてレンダ開始時に一度確定する `Vec<Mm>` であり、行処理には `&[Mm]` スライスで渡る構造をそのまま維持する。

### テンプレート移行（10 ファイル / 15 テーブル）

機械的変換。全テンプレートが合計ちょうど 100 なので `percent: N` → `"width": "N%"` は**意味的に等価**な変換になる。ただし内部計算が f32 → f64 に変わるため、幅は 1e-5 mm オーダーで動き得る（後方互換は要件ではない）。

対象: `table.json`, `table-with-static-simple.json`, `table-with-static-test.json`, `multipage.json`, `quote.json`, `combined_example.json`, `print-renews.json`, `large-tables-spanning.json`(2), `multi-table-fixed.json`(4), `table-spacer-summary-page-break.json`(2)

変換スクリプトを書いて一括適用し、`"width"` / `"header"` / `"cell"` の3キーに組み替える。適用後に旧形式のキーが残っていないことを grep で確認。

**移行後の任意改善**（別コミット）: `large-tables-spanning.json` の `17/18`、`print-renews.json` の `32/8`、`table-spacer-summary-page-break.json` の `31/17/14` など、100 合わせのために歪んだ値を `fr` に置き換えて意図を明示する。

### テスト

**新規ユニットテスト**（`src/schemas/table.rs` の `mod tests`）— 現状 `cell_widths` の出力 `Mm` を検証するテストは**一つも存在しない**ため、まずここを固める:

**文法パース**
- 成功: `25` / `"25mm"` / `"20%"` / `"2fr"` / `"fr"`、および大小文字（`"2FR"` / `"25MM"`）と前後空白（`" 25mm "`）
- 失敗: `"abc"` / `"20 %"` / `"-5"` / `"-5mm"` / `"0fr"` / `"0mm"` / `"0%"` / `"NaNmm"` / `"inffr"` / `""`

**幅の解決**
- 全 `%`、全 `fr`、混在（`[20mm, "1fr", "25%"]`）で期待 `Mm` 値を検証
- **列の宣言順を入れ替えても各列の割当が変わらない**こと（`[20mm, "1fr", "25%"]` と `["25%", 20mm, "1fr"]`）
- fr あり → `Σcell_widths ≈ table_width`（epsilon 比較）が丸め後も成立
- **fr なし・余りあり → `Σcell_widths == Σallocated`（`table_width` より小さい）。最終列が伸びないこと** — `[20mm, 30mm]` で最終列が 30mm のままであることを明示的に assert する
- overflow（fr なし）→ fixed/percent が比例縮小され合計が `table_width` に収まる
- overflow（fr あり）→ fixed/percent のみ縮小、fr 列は 0 幅。残差補正が走らないこと
- **全列 `%` で合計ちょうど 100 のとき overflow 判定に落ちないこと** — 既存テンプレートの実値で確認する。特に `product_list` の `[14,31,17,10,14,14]` / 190mm、`financial_table` の `[15,25,25,17,18]`、`print-renews` の `[10,32,10,10,10,8,10,10]`
- **残差補正先が極小 weight の fr 列でも負幅にならないこと** — `["100fr", "0.0001fr"]` のようなケース。全列幅 `>= 0` を不変条件として assert する
- `Percent > 100` 単独 → overflow 経路で縮小される
- 単一列、極端に大きい/小さい有限値
- 有効幅 0（`basePdf.padding` が用紙幅以上）でハングしないこと
- `base_pdf.padding` によるクランプ（現行 `min(base.width, available)` の挙動維持）
- assert は `Mm` の exact 比較ではなく epsilon 比較か終端 x 座標で行う（fr 経路は f64 集計のため）

**バリデーション**
- `fields` の行長不一致（短い行 / 長い行の両方）→ panic ではなくエラー。エラーメッセージに **row index / expected / actual** を含める
- `showHead: false` でも `columns[].header` を必須とするか — 仕様として決め、テストで固定する（必須とする方針。`showHead` は描画の有無だけを制御し、列定義の形は変えない）

**更新が必要な既存テスト**:
- `tests/table_integration_tests.rs:129` `test_table_column_width_percentages_validation` — 合計 100 検証を pin している唯一のテスト。削除し、`width` パースエラーのテストに置き換える
- `tests/table_integration_tests.rs:116`, `:158` — フィクスチャ JSON を新形式に
- `src/schemas/table.rs:937` `test_table_from_json_invalid_column_percentages` — 同上
- `src/schemas/table.rs:1007` のインライン JSON、および `:1051`-`:1298` の `LineBreakMode` / 行スタイル系テストのフィクスチャ
- `src/schemas/table.rs:1130`, `:1160` — `FlowCursor.y` を検証する唯一の end-to-end テスト。**幅の解決結果が同じなら数値は変わらないはず**。変わったら回帰のシグナルなので、期待値の書き換えではなく原因追及を先にする

### ドキュメント

現行仕様として更新するもの:
- `docs/schema-spec.md:178-213` — `headWidthPercentages` の節を `columns` に書き換え、`width` の文法表を追加
- `README.md:238, 326, 341, 615` — 4箇所のテーブル例と `lineBreakMode` の説明文（`headWidthPercentages[].lineBreakMode` → `columns[].header.lineBreakMode`）
- `CHANGELOG.md` — BREAKING CHANGE として旧→新のマイグレーション例を記載

歴史資料として残すもの（旧キー参照が残るが更新しない。冒頭に「この文書は当時の仕様に基づく」と1行追記するに留める）:
- `docs/line-break-mode-plan.md:42, 47, 55` — `headWidthPercentages[].lineBreakMode` への言及
- `docs/schema-trait-architecture.md:192` — `head_width_percentages: Vec<Head>` の構造体例

（`docs/table-styling-migration.md` に旧キー参照は無いことを確認済み）

## スコープ外（意図的に手を付けない）

- **`auto` 幅**: 別 PR。ただし `ColumnWidth` を enum にしておくことで `Auto` バリアントの追加は後から非破壊的に行える。前提として下記の測定バグを先に潰す必要がある
- **`Text::get_height` の測定バグ**: `get_height` (`text.rs:583`) は `base.width` 全体で折り返しを計算するが、`render` (`text.rs:270`) は `base.width - padding.left - padding.right` で折り返す。水平パディングのあるセルで測定行高が実レンダより小さくなる。**既存のバグであり本変更とは独立**なので別 issue とする（`auto` 実装のブロッカー）
- 行分割 / widow-orphan 制御、`colspan` / `rowspan`

## 検証

```bash
# 1. ビルドとテスト
cargo build
cargo test
cargo test -- --nocapture   # 幅解決ユニットテストの出力確認

# 2. 移行済みテンプレートのレンダリング（全 10 ファイル）
cargo run --example simple ./templates/quote.json
cargo run --example simple ./templates/table.json
cargo run --example simple ./templates/multi-table-fixed.json
cargo run --example simple ./templates/large-tables-spanning.json
cargo run --example simple ./templates/table-spacer-summary-page-break.json
cargo run --example simple ./templates/print-renews.json
# ... 残りも同様

# 3. 改ページ・大量行の挙動
cargo run --example table-50pages
cargo run --example memory-efficient-table
cargo run --example quote
```

**幾何回帰確認（最重要）**

後方互換は要件ではないため、旧出力とのバイト一致は**求めない**。内部計算が f32 → f64 に変わる以上、列幅は 1e-5 mm オーダーで動く。

確認したいのは「移行が意味的に等価であること」なので、次の3点を見る:

1. **ページ数が変わらないこと** — 変わっていたら行高計算かテキスト折り返しが動いており、幅解決のバグ
2. **テキストの折り返し位置が変わらないこと** — セル内の改行位置は列幅に敏感なので、実質的な幅回帰検出器になる
3. **列境界の x 座標が許容誤差（0.01mm）内で一致すること**

手順:
1. 変更前に全テンプレートをレンダリングし `examples/pdf/*.pdf` を退避
2. 実装・テンプレート移行後に再レンダリング
3. ページ数を比較し、差があれば原因追及。テキスト抽出（`pdftotext -layout` 等）で折り返し位置を比較
4. **列境界の x 座標は数値で比較する。** 0.01mm の差は目視では判定できない。`resolve_column_widths` の出力から累積 x 座標を計算するユニットテストで固定し、目視は補助確認に留める

（参考: `cargo run --example simple ./templates/table.json` を連続 2 回実行して `cmp` した結果はバイト単位で一致した。printpdf の出力自体は決定的なので、将来ゴールデンファイル方式の回帰テストを入れる余地はある。ただし Tera に `date` / `dateTime` が注入される (`src/schemas/mod.rs:513-524`) ため、それらを使うテンプレートは日付をまたぐと差分が出る点に注意）

**新機能の動作確認**: `templates/` に `table-column-widths.json` を新規追加し、`[20, "2fr", "1fr", "25%"]` のような混在指定でレンダリングして、列境界が意図通りか目視確認する。
