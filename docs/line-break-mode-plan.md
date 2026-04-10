# `lineBreakMode` 追加計画

## Summary

狭い幅では `WordSegmenter` 固定だと語をまとめすぎて大きな空白が出やすい。

対策として、テキスト系 schema に optional な `lineBreakMode` を追加し、`word` と `char` を選べるようにする。未指定時の既定値は `Text` / `DynamicText` では互換優先で `word`、Table の body では狭幅前提を優先して `char` とする。`char` はユーザー向けの短い名前で、内部実装は grapheme cluster 単位とする。

あわせて、`char` モードの意味を分割器の切替だけに留めず、複合絵文字や variation selector を含む grapheme cluster を途中で壊さない方針を明文化する。`👨‍👩‍👧‍👦`、`👍🏽`、`❤️` のような見た目上 1 文字の単位は、改行候補・fallback 分割・サニタイズで一貫して 1 単位として扱う。

## Key Changes

- `[src/font.rs](/Users/yako/Projects/pdforge/src/font.rs)` に public enum `LineBreakMode` を追加する。
- serde は `camelCase`、値は `word` / `char`。
- `char` の意味は Rust の `char` ではなく grapheme cluster 単位としてコメントで明記する。
- `FontSpec` は concrete な `LineBreakMode` を保持できるようにする。
  - `new()` は `word` 既定
  - `with_line_break_mode(...)` を追加
- `split_text_to_size()` の内部で分割器を切り替える。
  - `word`: まず現在どおり単語境界で分割
  - `char`: 最初から grapheme cluster 単位で分割
  - 両モードとも日本語禁則は現状どおり後段で適用
- 過大セグメントの fallback も grapheme cluster ベースに統一する。
  - `word` で 1 セグメントが幅超過した場合は、そのセグメント内を grapheme cluster 単位で詰める
  - `char` で 1 cluster 自体が箱幅超過なら、その cluster を単独行に置く
- 分割処理では grapheme cluster の途中で改行しないことを保証する。
  - ZWJ 絵文字、skin tone modifier 付き絵文字、variation selector 付き記号を途中分割しない
  - `word` モードの fallback でも cluster 境界を壊さない
- `[src/font.rs](/Users/yako/Projects/pdforge/src/font.rs)` の幅計算も grapheme cluster 前提に寄せる。
  - `width_of_text_at_size()` の走査を `chars()` ベースから見直す
  - 少なくとも文字間隔付与の単位数と fallback 分割の単位数を一致させる
  - 複合絵文字の正確な shaping が無い前提でも、途中分割や過剰な文字間隔で見た目を壊さないことを優先する
- `[src/schemas/pdf_utils.rs](/Users/yako/Projects/pdforge/src/schemas/pdf_utils.rs)` の `sanitize_text_for_font()` も grapheme cluster 単位で見直す。
  - unsupported な cluster をコードポイント単位で部分的に置換しない
  - 必要なら cluster 全体を 1 個の fallback 文字に置き換える
- `Text` と `DynamicText` の描画経路をそろえる。
  - `[src/schemas/text.rs](/Users/yako/Projects/pdforge/src/schemas/text.rs)` と同様に、`[src/schemas/dynamic_text.rs](/Users/yako/Projects/pdforge/src/schemas/dynamic_text.rs)` でもフォント付きの描画経路を使い、同じサニタイズ方針を適用する
- schema に `lineBreakMode` を追加する。
  - `text.lineBreakMode`
  - `dynamicText.lineBreakMode`
  - `table.headStyles.lineBreakMode`
  - `table.headWidthPercentages[].lineBreakMode`
  - `table.bodyStyles.lineBreakMode`
  - `table.columns[].schema` が `text` の場合は既存 `JsonTextSchema` の `lineBreakMode` をそのまま使う
- 解決順を固定する。
  - `Text` / `DynamicText`: schema 指定 → 既定 `word`
  - Table header: `headWidthPercentages[].lineBreakMode` → `headStyles.lineBreakMode` → `word`
  - Table body text cell: column の text schema `lineBreakMode` → `bodyStyles.lineBreakMode` → `char`
- `Text` に `line_break_mode: Option<LineBreakMode>` を保持させる。
  - `Text::from_json` は schema 値を格納
  - `Text::new` は既定 `None`
  - `set_line_break_mode(...)` と `has_line_break_mode()` を追加して Table の fallback 適用に使う
- `DynamicText` も同様に `line_break_mode: Option<LineBreakMode>` を持たせ、`from_json` で受ける。
- Table は次の方針で対応する。
  - header は `headStyles.lineBreakMode` を既定にし、`headWidthPercentages[]` に個別指定があればそれを優先
  - body は `bodyStyles.lineBreakMode` を既定にし、column の text schema に個別指定があればそれを優先
  - `bodyStyles.lineBreakMode` が未指定なら body 側の既定値は `char` とする

## Test Plan

- `src/font.rs` に unit test を追加し、`word` と `char` で分割結果が変わることを確認する。
- `src/font.rs` に unit test を追加し、`word` の幅超過セグメント fallback が grapheme cluster 単位で動くことを確認する。
- `src/font.rs` に unit test を追加し、`👨‍👩‍👧‍👦`、`👍🏽`、`❤️` が `char` モードでも途中分割されないことを確認する。
- `src/font.rs` に unit test を追加し、`word` モードでも幅超過時 fallback が grapheme cluster を壊さないことを確認する。
- `[src/schemas/text.rs](/Users/yako/Projects/pdforge/src/schemas/text.rs)` と `[src/schemas/dynamic_text.rs](/Users/yako/Projects/pdforge/src/schemas/dynamic_text.rs)` に parsing/default テストを追加する。
  - 未指定なら `word`
  - `lineBreakMode: "char"` を受け取れる
- `[src/schemas/pdf_utils.rs](/Users/yako/Projects/pdforge/src/schemas/pdf_utils.rs)` に unit test を追加し、unsupported な複合絵文字が部分的に崩れず 1 単位で fallback されることを確認する。
- `DynamicText` の描画経路が `Text` と同じサニタイズを通ることを確認するテストを追加する。
- `[src/schemas/table.rs](/Users/yako/Projects/pdforge/src/schemas/table.rs)` に優先順位テストを追加する。
  - header item override > headStyles default
  - body column text override > bodyStyles default
  - bodyStyles のみ指定時は body cell に反映される
- 実フォントを使う focused integration test を 1 本追加し、狭幅の日本語+英数字混在テキストで `word` と `char` の改行差を確認する。
- 実フォントを使う focused integration test では、絵文字対応フォントが無い場合でも複合絵文字が途中で砕けず、必要時は 1 文字分の fallback として扱われることを確認する。
- `cargo test` で全体確認する。
  - 既知の `table_integration_tests` の FontMap 問題は今回の受け入れ判定から切り分ける

## Assumptions

- field 名は `lineBreakMode`
- 値名は `word` / `char`
- `char` は grapheme cluster を意味する
- `char` モードはラベル用紙などの狭幅前提で使用し、英数字が文字単位で分割されるのは意図した動作とする
- `char` モードでは絵文字や combining sequence も grapheme cluster 単位で保持し、途中で壊さない
- plain `Text` / `DynamicText` の未指定既定値は `word`
- Table body の未指定既定値は `char`
- Table body でも column の text schema は個別 override 可能にする
- 日本語禁則の常時適用方針は維持し、分割方式の違いは「どこで切るか」に限定する
- 複合絵文字の line break 安全性は今回のスコープに含めるが、カラー絵文字や高度な shaping を伴う「見た目どおりの描画」は使用フォントと PDF 描画系の制約を受ける
- 実際の emoji glyph 表示保証が必要になった場合は、別途フォント fallback chain の導入を次段の課題とする
