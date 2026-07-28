# 半角スペース幅の計測誤りによる早すぎる折り返しの修正 — 実装プラン

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 半角スペースの幅を描画側と同じ `hmtx` 実測 advance で計測させ、セル内テキストが実幅より早く折り返されて右側に不自然な余白が残る不具合を解消する。

**Architecture:** `FontSpec::glyph_width_for_char` (`src/font.rs:245-259`) にあるスペース専用の分岐を削除し、スペースも他の全文字と同じ `lookup_glyph_index` → `get_horizontal_advance` 経路に通す。これは printpdf の描画パスが使う値と同一なので、「計測値 == 描画幅」という不変条件が回復する。ロジックの追加はなく、誤った特別扱いの除去のみ。

**Tech Stack:** Rust / printpdf 0.12.3（内部で azul-layout 0.0.12 の `ParsedFont`）/ icu_segmenter / 標準の `cargo test`

**Review:** codex によるレビュー済み（2026-07-28）。方針は承認、文言の限定・残課題の明記・commit 粒度について指摘を受けて反映済み。指摘のうち1点（E2E テストを Word 1本に削減）は不採用（理由は Task 1 Step 1 の注記を参照）。

## Global Constraints

- 変更対象は `src/font.rs` の計測ロジックのみ。折り返しアルゴリズム本体（`get_splitted_lines` / `push_grapheme_wrapped_lines`）、禁則処理、テーブルの列幅解決には手を入れない。
- テストフォントは既存のヘルパ `test_font()` (`src/font.rs:816`) が読む `assets/fonts/NotoSansJP-Regular.ttf` を使う。新しいフォントアセットは追加しない。
- テスト内でフォント固有の数値（`224` 等）をハードコードしない。必ず `ParsedFont` から実測して期待値を組み立てる（`units_per_em` 非依存にするため）。
- ライブラリ本体のコメントに特定フォント由来の数値（`2.76pt` 等）を書かない。汎用ライブラリのコメントとして陳腐化するため。
- 既存 134 テストは全て green のまま維持する。
- コミットは Conventional Commits 形式。

---

## 背景 — 根本原因（調査済み）

`src/font.rs:250-252`:

```rust
if ch == ' ' {
    return self.font.space_width.unwrap_or(0) as f32 * percentage_font_scaling;
}
```

現在の printpdf 0.12.3 / azul-layout 0.0.12 では、**実フォントを `ParsedFont::from_bytes`（および同系の byte parse）で構築した場合、`space_width` は `Some(0)` にキャッシュされる**。azul 側にその理由が明記されている（`azul-layout-0.0.12/src/font.rs:1390-1395`）:

> During `from_bytes_internal` the source bytes are not attached yet, so `hmtx` is unreadable and `get_space_width_internal` reads back 0

※ これは「あらゆる `ParsedFont` で常に 0」という意味ではない。mock-backed な `ParsedFont` は `MockFont::with_space_width`（`azul-layout-0.0.12/src/font.rs:98`）で非0を返せるし、`printpdf::ParsedFont` は `from_azul` と `DerefMut`（`printpdf-0.12.3/src/font.rs:55,155`）を公開しているので構築・変更経路自体は存在する。ただし **どの経路であれ描画で使われるのは `get_horizontal_advance` の値**（mock でも `glyph_advances` を引く、`azul-layout-0.0.12/src/font.rs:2029`）なので、計測もそちらを正とするのが正しい。

`space_width` が `0` を返すと `width_of_text_at_size` (`src/font.rs:422-426`) の

```rust
total_width += if cluster_width > 0.0 {
    cluster_width
} else {
    tofu_width * percentage_font_scaling   // tofu_width = 500.0
};
```

が発動し、**スペース1個が 500 units として計上される**。実測値との差:

| フォント | `font.space_width`（計測に使用中） | `hmtx` 実測 advance（描画で使われる値） | 10pt 時の誤差 |
|---|---|---|---|
| NotoSansJP-Regular.ttf | `Some(0)` → tofu 500 | **224** | +2.76pt / スペース |
| NotoSerifJP-Regular.ttf | `Some(0)` → tofu 500 | **256** | +2.44pt / スペース |

生成済み `examples/pdf/quote.pdf`（商品名列 = 95mm、padding 左右 1mm → 折り返し幅 93mm = 263.62pt）の 05 行目1行目を `pdftotext -bbox-layout` で実測した比較:

```
"腕時計"             ours  30.000  pdf  30.000  diff  +0.000
" "                  ours   5.000  pdf   2.240  diff  +2.760   ← スペースのみズレる
"カルティエ"         ours  50.000  pdf  50.000  diff  +0.000
" "                  ours   5.000  pdf   2.240  diff  +2.760
"タンクフランセーズ" ours  90.000  pdf  90.000  diff  +0.000
" "                  ours   5.000  pdf   2.240  diff  +2.760
"W343234"            ours  42.080  pdf  42.080  diff  +0.000
" "                  ours   5.000  pdf   2.240  diff  +2.760
"#9872"              ours  27.750  pdf  27.750  diff  +0.000
------------------------------------------------------------
total:               ours 259.830  pdf 248.790  diff +11.040
```

漢字・カナ・英数字は誤差ゼロ。ズレはスペースのみ。結果として折り返しは 263.62pt で打ち切るが実描画は 248.79pt しかなく、**14.83pt ≒ 5.2mm の余白が右に残る**。スペース数が行ごとに違うため余白量もバラつき、「折り返し位置が不自然」に見える。

**修正を当てた状態での実測（検証済み）**: 同じ行が `xMax` 333.83 → **344.93** に伸び、右余白は 14.83pt → **3.73pt (1.3mm)** に縮小。`cargo test` は 134 passed / 0 failed。

---

## File Structure

| ファイル | 責務 | 変更内容 |
|---|---|---|
| `src/font.rs:245-259` | 1文字あたりの advance 取得 | スペース専用分岐を削除。`space_width` を使わない理由をコメントで残す |
| `src/font.rs:807-` (`mod tests`) | 計測・折り返しの単体テスト | ヘルパ `hmtx_width_of` を追加し、テストを**計4本**追加（直接計測2本 + 折り返し E2E 2本） |
| `examples/quote.rs:51-52` | 動作確認用サンプルデータ | 文字列リテラル中に紛れ込んだ改行の除去（Task 2・別原因、任意） |

---

### Task 1: スペース幅を `hmtx` 実測 advance で計測する

修正本体・直接計測テスト・折り返し回帰テストを**1つの原子的なコミット**にまとめる。途中状態でも実装と全回帰ガードが揃い、`git bisect` が自然に機能するため。

**Files:**
- Modify: `src/font.rs:245-259` （`glyph_width_for_char`）
- Test: `src/font.rs` の既存 `mod tests`（`src/font.rs:807` 以降）に追記

**Interfaces:**
- Consumes: `printpdf::ParsedFont::lookup_glyph_index(u32) -> Option<u16>`、`ParsedFont::get_horizontal_advance(u16) -> u16`、`ParsedFont::font_metrics.units_per_em: u16`
- Consumes: 既存テストヘルパ `test_font() -> Arc<ParsedFont>` (`src/font.rs:816`)、`test_font_spec(LineBreakMode) -> FontSpec` (`src/font.rs:832`)
- Produces: 新テストヘルパ `fn hmtx_width_of(font: &ParsedFont, text: &str, font_size: Pt) -> Pt` — `FontSpec` を一切経由せず `hmtx` から期待幅を組み立てる非循環アンカー

- [ ] **Step 1: 失敗するテストを書く**

`src/font.rs` の `mod tests` 内、`fn test_font_spec(...)` の直後にヘルパとテスト4本を追加する。

> **Word と Char の両方を残す理由**（レビューで「Word 1本で十分では」と指摘された点への回答）:
> `src/schemas/table.rs:245` は `json.line_break_mode.unwrap_or(LineBreakMode::Char)` で、**テーブル本文のデフォルトは `Char`**。今回報告された不具合が実際に踏んでいるのは Char 経路であり、Word 側だけを残すと再現経路そのものを取りこぼす。計測ロジックは共有だが、`get_splitted_lines` は Word と Char で異なる分岐（`split_text_by_word_segmenter` vs `split_text_by_grapheme_cluster`、および `push_grapheme_wrapped_lines` への落ち方）を通るため、統合経路としては別物。

```rust
    /// Expected advance width straight from `hmtx`, bypassing `FontSpec`
    /// entirely. This is the value the renderer actually draws with, so it is
    /// the non-circular reference for what measurement must return.
    fn hmtx_width_of(font: &ParsedFont, text: &str, font_size: Pt) -> Pt {
        let scaling = 1000.0 / font.font_metrics.units_per_em as f32;
        let units: f32 = text
            .chars()
            .map(|ch| {
                let glyph_index = font
                    .lookup_glyph_index(ch as u32)
                    .unwrap_or_else(|| panic!("test font should have a glyph for {ch:?}"));
                font.get_horizontal_advance(glyph_index) as f32 * scaling
            })
            .sum();

        Pt(units * font_size.0 / 1000.0)
    }

    #[test]
    fn space_is_measured_with_the_font_advance_not_the_tofu_width() {
        // Byte-parsed faces cache `space_width` as 0 (upstream reads `hmtx`
        // before the source bytes are attached), which used to fall through to
        // the 500-unit tofu width and over-measure every space.
        let font = test_font();
        let spec = test_font_spec(LineBreakMode::Word);
        let font_size = Pt(10.0);
        let expected = hmtx_width_of(&font, " ", font_size);

        let actual = spec.width_of_text_at_size(" ", font_size, Pt(0.0)).unwrap();

        assert!(
            (actual.0 - expected.0).abs() < 1e-4,
            "space measured as {}pt, hmtx advance is {}pt",
            actual.0,
            expected.0
        );
    }

    #[test]
    fn spaced_text_is_measured_exactly_as_the_renderer_draws_it() {
        let font = test_font();
        let spec = test_font_spec(LineBreakMode::Word);
        let font_size = Pt(10.0);
        let text = "腕時計 カルティエ タンクフランセーズ";
        let expected = hmtx_width_of(&font, text, font_size);

        let actual = spec
            .width_of_text_at_size(text, font_size, Pt(0.0))
            .unwrap();

        assert!(
            (actual.0 - expected.0).abs() < 1e-4,
            "measured {}pt, renderer draws {}pt",
            actual.0,
            expected.0
        );
    }

    #[test]
    fn word_mode_does_not_wrap_a_line_that_exactly_fits_the_box() {
        // Box width comes from `hmtx`, i.e. the width the renderer will draw.
        // Over-measuring the two spaces used to push the computed width past
        // this box and split the line in two.
        let font = test_font();
        let spec = test_font_spec(LineBreakMode::Word);
        let font_size = Pt(10.0);
        let text = "腕時計 カルティエ タンクフランセーズ";
        let box_width = hmtx_width_of(&font, text, font_size);

        let lines = spec
            .split_text_to_size(text, font_size, box_width, Pt(0.0))
            .unwrap();

        assert_eq!(lines, vec![text.to_string()]);
    }

    #[test]
    fn char_mode_does_not_wrap_a_line_that_exactly_fits_the_box() {
        // `Char` is what table bodies default to (`table.rs`
        // `line_break_mode.unwrap_or(LineBreakMode::Char)`), so this is the
        // path the reported quote-table regression actually took.
        let font = test_font();
        let spec = test_font_spec(LineBreakMode::Char);
        let font_size = Pt(10.0);
        let text = "腕時計 カルティエ タンクフランセーズ";
        let box_width = hmtx_width_of(&font, text, font_size);

        let lines = spec
            .split_text_to_size(text, font_size, box_width, Pt(0.0))
            .unwrap();

        assert_eq!(lines, vec![text.to_string()]);
    }
```

- [ ] **Step 2: テストを実行して失敗を確認する**

```bash
cargo test --lib font::tests -- --nocapture
```

Expected: 追加した4本すべて FAIL。
- `space_is_measured_...`: `space measured as 5pt, hmtx advance is 2.24pt`
- `spaced_text_is_measured_...`: `measured 180pt, renderer draws 174.48pt`
  - 内訳: 和文17文字 × 1000 units = 170.00pt は一致。スペース2個が実測 224×2 = 4.48pt のところ tofu 500×2 = 10.00pt で計上され +5.52pt ずれる。
- `word_mode_does_not_wrap_...` / `char_mode_does_not_wrap_...`: `lines` が2要素に分割され `assert_eq!` が失敗。

4本すべてが赤いことを確認すること。1本でも緑ならテストが不具合を捕まえていないので、Step 1 に戻る。

- [ ] **Step 3: 最小の実装を書く**

`src/font.rs:245-259` の `glyph_width_for_char` を次に置き換える。スペース専用分岐を削除し、理由をコメントで残す。

```rust
    fn glyph_width_for_char(&self, ch: char, percentage_font_scaling: f32, tofu_width: f32) -> f32 {
        if is_non_rendering_cluster_char(ch) {
            return 0.0;
        }

        // Space deliberately goes through the same `hmtx` lookup as every other
        // character. `ParsedFont::space_width` is not authoritative here: for
        // byte-parsed faces upstream computes it inside `from_bytes_internal`,
        // before the source bytes are attached, so `hmtx` is unreadable and it
        // caches `Some(0)`. A 0 here fell through to the tofu width in
        // `width_of_text_at_size`, over-measuring spaces and wrapping lines
        // before the box was full. `get_horizontal_advance` is what the
        // renderer draws with, so measuring through it keeps
        // "measured width == drawn width" true on every construction path.
        self.font
            .lookup_glyph_index(ch as u32)
            .map(|glyph_index| self.font.get_horizontal_advance(glyph_index))
            .map(|width| width as f32 * percentage_font_scaling)
            .unwrap_or(tofu_width * percentage_font_scaling)
    }
```

> `space_width` が非0の場合に備えた分岐は**あえて残さない**。mock-backed な face でも描画側は `glyph_advances` を引く（`azul-layout-0.0.12/src/font.rs:2029`）ため、`space_width` と `glyph_advances` が食い違う場合は後者が正。production コードに分岐を残す理由にならない。

- [ ] **Step 4: テストを実行して成功を確認する**

```bash
cargo test --lib font::tests -- --nocapture
```

Expected: `font::tests` 全件 ok（追加4本を含む）。

- [ ] **Step 5: テストスイート全体を実行する**

```bash
cargo test
```

Expected: 全スイート ok、failed 0（本修正前の基準値は 134 passed）。

- [ ] **Step 6: コミット**

```bash
git add src/font.rs
git commit -m "fix: measure spaces with the font's hmtx advance instead of the tofu width"
```

---

### Task 2: `examples/quote.rs` の文字列リテラルに混入した改行を除去する（任意・独立）

**Files:**
- Modify: `examples/quote.rs:51-52`

**Interfaces:**
- Consumes: なし
- Produces: なし

**これはライブラリの不具合ではなく、Task 1 とは別原因。** サンプルデータ側の貼り付け事故である。`split_text_to_size` (`src/font.rs:323`) は `content.lines()` で段落分割するため、リテラル中の生の改行はそこで強制改行になり、PDF 上で `... F27W4` / `CNVQ5 iPhone 17e, W` / `hite, 256GB ...` と不自然に切れる。ハード改行の挙動を意図的に確認するためのデータであればこのタスクはスキップしてよい。Task 1 とは必ず別コミットにする。

- [ ] **Step 1: 現状を確認する**

```bash
sed -n '51,52p' examples/quote.rs
```

Expected: 文字列リテラルが `iPhone 17e, W` で改行され、次行が `hite, 256GB ...` から始まっている。

- [ ] **Step 2: 改行を除去する**

`examples/quote.rs:51-52` を次に置き換える。

```rust
        (
            "ガジェット アップル スマートフォン MHRP4J/A F27W4CNVQ5 iPhone 17e, White, 256GB 350870650569421 N",
            "95,000",
            "80",
        ),
```

- [ ] **Step 3: 生成して確認する**

```bash
cargo run --example quote
```

Expected: `Quote PDF generated successfully at ./examples/pdf/quote.pdf`。生成物の 11 行目に `W` / `hite` の分断がないこと。

- [ ] **Step 4: コミット**

```bash
git add examples/quote.rs
git commit -m "fix(examples): remove a stray newline inside a quote sample item name"
```

---

### Task 3: 生成 PDF での最終検証

**Files:**
- 変更なし（検証のみ）

**Interfaces:**
- Consumes: `examples/quote.rs`、`templates/quote.json`
- Produces: なし

座標値は poppler / フォントの更新で動きうるため、自動テストにはせず手動検証に留める。

- [ ] **Step 1: PDF を再生成する**

```bash
cargo run --example quote
```

Expected: `Quote PDF generated successfully at ./examples/pdf/quote.pdf`

- [ ] **Step 2: 実描画幅を測る**

```bash
pdftotext -f 1 -l 1 -bbox-layout examples/pdf/quote.pdf /tmp/quote-bbox.html
grep -n "W343234" -B4 -A2 /tmp/quote-bbox.html | head -12
```

Expected: 05 行目1行目の `<line>` が `xMin="85.039386"` / `xMax="344.929386"` 付近。
- 行の実幅 = 344.93 − 85.04 = **259.89pt**
- 折り返し幅（95mm − padding 2mm = 93mm）= **263.62pt**
- 右余白 = **3.73pt (1.3mm)** ← 修正前は 14.83pt (5.2mm)

`xMax` が 333.83 付近のままなら修正が効いていない。Task 1 Step 3 を見直すこと。

- [ ] **Step 3: 目視で確認する**

```bash
pdftoppm -png -r 110 -f 1 -l 1 examples/pdf/quote.pdf /tmp/quote-page
```

`/tmp/quote-page-1.png` を開き、商品名列のテキスト右端が列の罫線近くまで達していること、行ごとの右余白のバラつきが解消していることを確認する。

- [ ] **Step 4: 最終のフルテスト**

```bash
cargo build && cargo test
```

Expected: ビルド成功、全スイート ok、failed 0。

---

## 既知の残課題（本件のスコープ外）

- **cmap に U+0020 を持たないフォントでは「計測値 == 描画幅」が成立しない。**
  `cluster_is_supported_by_font` (`src/font.rs:621-624`) は `cluster == " "` を無条件で `true` とするため、スペースが `sanitize_cluster_for_font` の置換対象にならない。その結果、スペースグリフのないフォントでは計測側が `lookup_glyph_index` → `None` → tofu 500 に落ちる。一方 printpdf の書き出しは `lookup_glyph_index(c).unwrap_or(0)`（`printpdf-0.12.3/src/serialize.rs:975`）で GID 0 (.notdef) を出すため、実幅は `.notdef` の `/W`（または `DW`）依存であり 500 である保証はない。**別 issue として起票すること。**
- **`tofu_width = 500.0` のマジックナンバー** (`src/font.rs:411`): グリフ未収録時のフォールバック幅。`width_of_text_at_size` はクラスタを `sanitize_cluster_for_font` で置換してから計測するため、**通常のフォントでは**このフォールバックに到達する経路はほぼない（上記のスペース例外を除く）。本件とは独立した論点。
- **Word モードでの日本語の折り返し**: ICU の word segmenter は「スマートフォン」のような語の内部で折らないため、和文でも語幅ぶんの余白が残りうる。これは仕様であり、`lineBreakMode: "char"` で回避できる（`docs/line-break-mode-plan.md`）。本件の 5mm 級の余白とは原因が別。
- **上流 (`printpdf` / `azul-layout`) への報告**: byte-parse 経路で `space_width` が 0 にキャッシュされる件は上流のコード内コメントに残された既知の制約。本修正は `space_width` に一切依存しないので、上流が直っても壊れない。
