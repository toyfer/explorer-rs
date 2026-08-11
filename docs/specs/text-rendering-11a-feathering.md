# Spec slice 11a: 文字レンダリング改善（tessellation feathering）

PR #11 の A 要件だけを切り出した実装ブランチ。

## Context7

- `FullOutput::pixels_per_point` は `Context::tessellate` → `Tessellator::new(pixels_per_point, tessellation_options, ...)` で feathering に使われる
  - https://github.com/emilk/egui/blob/main/crates/egui/src/data/output.rs
- `set_pixels_per_point` は eframe/winit の scale factor に任せる（触らない）
- `Style::text_styles` は `BTreeMap<TextStyle, FontId>` で Body/Button/Heading を font_size に揃える

## 変更

- `src/fonts.rs` `apply_fonts` 後に `ctx.tessellation_options_mut(|o| o.feathering = true)`
- status に `ppp=` を出し GHA スクショ比較を容易に

## 受け入れ

- [ ] `cargo test` / `cargo check` 緑
- [ ] 日本語ラベルのジャギーが改善（目視 / ci-logs スクショ）
- [ ] ポータブル config / レジストリ非使用に影響なし
