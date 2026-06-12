# PDForge プロジェクト問題点レポート

作成日: 2026-06-12
対象バージョン: 0.11.0 (main ブランチ, commit 8733ec4)

コードベース全体(`src/`, `tests/`, `templates/`, README, CLAUDE.md)のレビュー、
`cargo test`(全 passed)および `cargo clippy --all-targets` の結果に基づく問題点の一覧。

---

## 1. 致命度: 高(バグ・パニック経路)

### 1.1 同一インスタンスで `render()` を複数回呼ぶとページが蓄積される

- 場所: `src/lib.rs:17-47`, `src/schemas/mod.rs:910`
- `PDForge` は単一の `PdfDocument` を保持し続け、レンダリング時に
  `doc.with_pages(pages).save(...)` を呼ぶ。printpdf 0.9.1 の `with_pages` は
  `self.pages.append(&mut pages)` で**追記**するため、2 回目の `render()` の出力には
  1 回目のページが含まれてしまう。
- サーバーで 1 つの `PDForge` を使い回して複数 PDF を生成するという典型的な
  ユースケースが成立しない。出力サイズも呼び出しごとに増え続ける(実質メモリリーク)。
- 対策案: `render()` のたびに新しい `PdfDocument` を作る(フォントは再登録する)か、
  レンダリング状態をドキュメントから分離するステートレスな設計に変更する。

### 1.2 動的フォントサイズ計算で width と height を取り違えている

- 場所: `src/font.rs:367`
- `calculate_dynamic_font_size` の grow ループ内の判定が
  `if new_total_height_in_mm < width` となっており、高さを**幅**と比較している。
  正しくは `height` との比較のはず。`fit: vertical` 指定時にフォントが
  正しく成長しない・しすぎる原因になる。

### 1.3 空コンテンツの `DynamicText` で usize アンダーフローによる panic

- 場所: `src/schemas/dynamic_text.rs:164`
- `let pages_increased = pages.len() - 1;` は、`split_text_to_size` が空の行リストを
  返した場合(空文字列コンテンツなど)に `pages` が空となり、`0 - 1` の
  アンダーフローで panic する(debug ビルド)。テンプレート変数が空文字に
  展開されるのは普通に起こりうる。

### 1.4 ライブラリ内に到達可能な `unimplemented!()` / `todo!()` が残っている

- 場所:
  - `src/schemas/table.rs:493` — テーブルのカラムが `Text` / `QrCode` 以外の型だと
    `unimplemented!()` で panic。テンプレート JSON の内容次第で発生するため、
    ライブラリ利用者の入力でプロセスが落ちる。
  - `src/schemas/group.rs:143` — `TryFrom<JsonGroupSchema> for Schema` が
    `todo!()`。public な trait 実装として panic するコードが公開されている。
- 対策案: いずれも `Err(Error::...)` を返すように変更する。`TryFrom` 実装は
  フォントマップなしでは成立しないので削除も検討。

### 1.5 Tera テンプレート展開による JSON 破壊(エスケープ漏れ)

- 場所: `src/schemas/mod.rs:677-716`(`render_with_inputs_table_data_and_static_inputs`)、
  `src/schemas/mod.rs:502-529`(static schema 側も同様)
- スキーマ全体を JSON 文字列にシリアライズしてから Tera で変数展開し、
  その結果を再度 JSON としてパースしている。入力値に `"`、`\`、改行などが
  含まれると、展開後の JSON が壊れてパースエラーになる(あるいは値が
  隣のフィールドに「注入」される)。ユーザー入力をそのまま渡す用途では
  事実上のインジェクション脆弱性。
- 対策案: Tera 展開を JSON 構造のパース後に文字列フィールド単位で行うか、
  カスタムフィルタで値を JSON エスケープして挿入する。

### 1.6 A4 幅 `Mm(210.0)` のハードコード

- 場所: `src/schemas/mod.rs:276, 291, 306`(`SchemaTrait::render` 内の
  DynamicText / Table / Group 分岐)
- Group 内にネストされた Table / DynamicText をレンダリングする際、親の幅を
  常に 210mm(A4)と仮定している。A4 以外のページサイズ(レシート、ラベル、
  A5 等)でレイアウトが崩れる。

---

## 2. 致命度: 中(API 設計)

### 2.1 入力キーが `&'static str` を強制している

- 場所: `src/lib.rs:20-22`(`render` のシグネチャ)
- `inputs: Vec<Vec<HashMap<&'static str, String>>>` のため、実行時に組み立てた
  キー(DB から取得したフィールド名など)を渡すには `Box::leak` が必要になる。
  `String` または `Cow<'_, str>` キーにすべき。`table_data` / `static_inputs` も同様。

### 2.2 エラーが stringly-typed な `Error::Whatever` に偏っている

- 場所: `src/schemas/mod.rs:84-89` ほか多数
- snafu で構造化エラーを定義しているにもかかわらず、「テンプレートが見つからない」
  「入力数とページ数の不一致」などの主要エラーが `Whatever { message }` で
  返るため、呼び出し側がエラー種別で分岐できない。専用バリアントを追加すべき。

### 2.3 テンプレートはファイルパスのみ、フォントはバイト列 API ありの非対称

- 場所: `src/lib.rs:87`(`load_template`)、`src/schemas/mod.rs:420`(`Template::new`)
- `load_template(template_name, template)` の第 2 引数名は `template` だが実際は
  **ファイルパス**。フォント側は `add_font`(bytes)/`add_font_from_file` の
  2 段構えなのに、テンプレートは文字列(JSON 本文)から読み込む API がない。
  埋め込みテンプレートやネットワーク経由のテンプレートを扱えない。
- 対策案: `load_template_from_str` を追加し、引数名を `path` に改名する。

### 2.4 `schemaVersion` を読み取るだけで検証していない

- 場所: `src/schemas/mod.rs:415, 441`(clippy も `field version is never read` を報告)
- バージョン不一致のテンプレートを黙って受け入れる。互換性チェックがない。

---

## 3. 致命度: 中(機能の不完全さ)

### 3.1 JSON で指定できるのに効かないスタイルが多数ある

clippy の `never read` 警告が示すとおり、デシリアライズされるだけで
レンダリングに使われないフィールドが多い:

- `src/schemas/rect.rs` — `opacity` が未使用
- `src/schemas/table.rs` — `border_color`, `border_width`, `character_spacing`,
  `font_color`(スタイル構造体)などが未使用 → **テーブルの罫線・文字色指定が
  テンプレートに書けるのに反映されない**
- `src/schemas/qrcode.rs` ほか — `rotate` 等が未使用

テンプレート仕様(`docs/schema-spec.md`)と実装の乖離であり、利用者には
サイレントな無視として現れる。実装するか、未対応である旨を明示すべき。

### 3.2 `DynamicText` のスタイルが固定

- 場所: `src/schemas/dynamic_text.rs:192`
- 文字色が `#000000` 固定で、alignment / fontColor / backgroundColor などを
  JSON で指定できない。`Text` との機能差が大きい。

### 3.3 禁則処理後の行幅再検証がない

- 場所: `src/font.rs:89-91`, `filter_start_jp` / `filter_end_jp`
- 折り返し確定後に禁則処理で文字を前行へ移動するため、移動先の行が
  ボックス幅を超過する可能性がある(再計測なし)。仕様上のトレードオフなら
  その旨をドキュメント化すべき。

---

## 4. 致命度: 中(パフォーマンス)

### 4.1 レンダリングのたびに全スキーマを deep clone

- `src/schemas/mod.rs:249`(`SchemaTrait::render` 冒頭の `self.clone()`)、
  `render_schemas_with_static_inputs` 内の各 `obj.clone()`。
  フォント参照は `Arc` なので軽いが、コンテンツ文字列・スタイルは毎回コピーされる。

### 4.2 ページ×入力ごとに Tera インスタンス生成と JSON 再シリアライズ

- `src/schemas/mod.rs:677-716`。テンプレート登録時に 1 回パースして使い回せる
  構造にすれば、大量ページ生成(`examples/table-50pages.rs` のような用途)で
  大幅に改善できる。

### 4.3 `FontMap::add_font` が `ParsedFont` を丸ごと clone

- `src/font.rs:544-546`(`Arc::new(font.clone())`)。日本語フォントは数 MB
  あるため無駄が大きい。`add_font` が最初から `Arc<ParsedFont>` を受け取るか、
  `lib.rs` 側で `Arc` を先に作って渡すべき。

### 4.4 テキスト幅計算が O(n²) 気味

- `src/font.rs:148-158, 198-201` — 行の確定判定のたびに `format!` で結合文字列を
  作り直し、行頭から全幅を再計測している。長い段落で二乗オーダーになる。
  クラスタ幅の累積値を保持すれば線形にできる。

---

## 5. 致命度: 低(コード品質・保守性)

1. **JsonSchema→Schema 変換の 4 重複** — `src/schemas/mod.rs` に 3 箇所
   (532-571, 608-651, 731-778)+ `src/schemas/group.rs:41-64` にほぼ同一の
   match が重複。1 つの変換関数に集約すべき(Group だけサポート型が狭い点は
   引数で表現できる)。
2. **ライブラリ内の `println!` デバッグ出力** — `src/schemas/mod.rs:810, 852`、
   `src/schemas/dynamic_text.rs:120`。ライブラリ利用者の stdout を汚す。
   `log` / `tracing` クレートに置き換えるか削除する。
3. **空モジュール `common.rs`** — `src/common.rs` は空行 1 行のみ。CLAUDE.md には
   「Shared utilities and common types」とあるが実体がない。削除するか実装する。
4. **dead code** — `render_schemas`(mod.rs:787)が未使用、テストヘルパー
   `create_test_text_without_font` 未使用、`examples` の未使用 import など
   clippy 警告約 40 件。`base64::encode` の deprecated 警告も残っている。
5. **rustdoc がほぼ皆無** — `src/lib.rs` にドキュメントコメント 0 件。公開 API
   (`PDForgeBuilder`, `PDForge::render` の 4 引数の意味など)の説明がコード上にない。
6. **コメント言語の混在** — 日本語と英語のコメントが混在(方針を決めて統一推奨)。

---

## 6. 致命度: 低〜中(プロジェクト体制・ドキュメント)

### 6.1 CLAUDE.md / README が実態と乖離している

- `src/main.rs` が**存在しない**ため、CLAUDE.md 記載の
  `cargo run --bin pdforge` や README の CLI 使用例は動作しない。
- `templates/table-test.json` が**存在しない**(README の Quick Start と
  CLAUDE.md が参照)。
- 「Binary vs Library Usage」セクション全体が現状と不一致。

### 6.2 公開準備の不足

- `LICENSE` ファイルがなく、`Cargo.toml` に `license` / `description` /
  `repository` メタデータもない。README は `pdforge = "0.11.0"` と crates.io から
  インストールできるかのように記載しているが、現状では publish できない。

### 6.3 CI がない

- GitHub Actions / Gitea Actions 等の設定が見当たらない。`cargo test` /
  `cargo clippy` / `cargo fmt --check` を回す CI を整備すべき。

### 6.4 テストカバレッジの偏り

- 既存テスト(20 件 + font/text/dynamic_text/table のユニットテスト)は
  パース・計算ロジック中心で、**エンドツーエンドのレンダリング検証がない**
  (生成 PDF のページ数・テキスト内容を検証するテストが薄い)。
- `tests/thread_safety.rs` は `Error` の Send/Sync 確認のみで、`PDForge` 本体の
  並行利用は未検証。
- 1.1〜1.6 のようなバグはレンダリング統合テストがあれば検出できた可能性が高い。

### 6.5 生成物のコミット

- `examples/pdf/` に生成済み PDF がコミットされている。リポジトリ肥大化の
  原因になるため、`.gitignore` への追加を検討(サンプルとして意図的なら可)。

---

## 推奨対応順序

| 優先度 | 項目 | 理由 |
|--------|------|------|
| 1 | 1.1 render 複数回呼び出しのページ蓄積 | ライブラリの基本ユースケースが壊れている |
| 2 | 1.5 Tera 展開のエスケープ漏れ | ユーザー入力起因の障害・注入 |
| 3 | 1.2 / 1.3 / 1.4 のバグ・panic 修正 | 入力次第でプロセスが落ちる |
| 4 | 6.4 レンダリング統合テスト追加 | 上記修正の回帰防止 |
| 5 | 1.6 / 3.1 / 3.2 機能の穴埋め | テンプレート仕様との乖離解消 |
| 6 | 2.x API 改善・5.x リファクタリング | 破壊的変更を伴うため計画的に |
| 7 | 6.1 / 6.2 / 6.3 ドキュメント・体制整備 | 公開・運用準備 |
