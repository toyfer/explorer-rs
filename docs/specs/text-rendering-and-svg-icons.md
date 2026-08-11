# Spec: 文字レンダリング改善 + ツールバー/ファイルアイコンをSVG化

## 現状の問題
- **文字のレンダリングが汚い**: `egui` デフォルトフォント + 日本語フォントのスケーリングでアンチエイリアスが弱く、特に 14pt 前後でジャギー/にじみが目立つ。`fonts::apply_fonts` で `FontDefinitions` を差し替えているが `TessellationOptions` や `pixels_per_point` の調整が未実施。
- **ホーム/戻る/進むアイコンが汚い**: `btn("◀") / "▶" / "⬆" / "⟳" / "🏠"` など絵文字/Unicode 記号をボタンラベルに直接使っており、DPI によっては欠け/ボケ/色ずれ。SVG で描くべき。
- **ファイルアイコン**: `shell::icon_emoji_for_path` / `shell::icon_rgba` で絵文字フォールバック + Windows `SHGetFileInfo` の HICON をテクスチャ化しているが、HICON 取得失敗時に絵文字のみで Windows 標準アイコンと乖離。アイコンパック導入も未検討。

## 要件
### A. 文字レンダリング改善
- `egui::Context::set_pixels_per_point` / `tessellation_options` 調整（例: `feathering: true`, `round_text_to_pixels: true`）。
- 日本語フォント（BIZ UD Gothic / 游ゴシック等）のヒンティングを活かすため `font_size` 14.0 を基準に `pixels_per_point` を `ctx.pixels_per_point()` の推奨値に追従。
- `egui::Style::override_text_style` で `Body`/`Button`/`Heading` のサイズを統一し、行高 `row_height_scale` と連動。
- 画像スケーリングは `TextureOptions::LINEAR` → テキストは `NEAREST` にならないように分離。
- GHA Windows のスクリーンショット（`egui` の `screenshot` 取得 or 手動）で before/after を比較。汚れの主因がフォント埋め込みか DPI スケールかを切り分け。

### B. ツールバーアイコン SVG化
- 戻る `◀` → `chevron_left.svg`, 進む `▶` → `chevron_right.svg`, 上へ `⬆` → `arrow_up.svg`, 更新 `⟳` → `refresh.svg`, ホーム `🏠` → `home.svg` を `egui` の `Image` + `include_bytes!` で埋め込み。
- `egui_extras::image` ではなく `egui::ImageSource` + `svg` クレート（例: `resvg` / `egui_svg`）でラスタライズ、または事前に `cargo build` で 16/24/32px PNG に焼く（ビルド時コード生成）。
- ボタンは `Button::image_and_text` 的にアイコンのみ or アイコン+テキストを選択可能にし、`compact_ui` 時はアイコンのみで省スペース。
- ホバー時の `on_hover_text` は維持。

### C. ファイルアイコンは Windows 踏襲 + フォールバック
- 基本は `SHGetFileInfo` (`shell::icon_rgba`) を優先。取得失敗時のみ絵文字ではなく **アイコンパック**（例: `vscode-icons` / `papirus` の MIT ライセンス subset）を同梱し拡張子マップでフォールバック。
- `icon_cache_key` は `ext:xxx` + `file:fullpath` の二層を維持し、LRU 512 程度でキャッシュ上限（Phase A 方針）。
- アイコンパックは `assets/icons/*.svg` に配置し、ビルド時に `include_bytes` で埋め込み、SVG→PNG ラスタライズは B と共通処理。
- ライセンス明記: 使用するアイコンパックのライセンスを `LICENSE-THIRD-PARTY` に追記。

## 非スコープ
- 内蔵プレビューでの動画/PDF レンダリング（PRINCIPLES.md の Out）。
- プラグインによるアイコン差し替え（将来 P1）。

## 実装方針（後で対応）
1. `src/fonts.rs` で `apply_fonts` 後に `ctx.tessellation_options_mut(|o| o.feathering = true)` 等を追加。`egui::Context::set_zoom_factor` は触らない。
2. `assets/icons/toolbar/*.svg` を新規追加し、`build.rs` で `winresource` とは別に SVG→PNG 生成（`resvg` を build-dependencies に追加）。
3. `src/app.rs` のトップバー `btn("◀")` 等を `egui::Button::image(ImageSource::... )` に置換。`btn` クロージャは `icon_button(path, label)` に置換。
4. `shell::icon_emoji_for_path` はフォールバック専用に格下げし、`icon_rgba` 失敗時は `assets/icons/files/{ext}.svg` を見に行く。
5. GHA Windows で `cargo test` + 目視スクショを `ci-logs` に保存して検証。

## 受け入れ条件
- [ ] 日本語テキスト（UIラベル/ファイル名/プレビュー）が GHA スクショでジャギーが目立たなくなる
- [ ] 戻る/進む/上へ/更新/ホームが SVG でシャープに表示される（DPI 100%/150% で確認）
- [ ] ファイルアイコンが Windows 標準に近い（exe/フォルダ等は HICON、txt/md/rs 等はパックで適切）
- [ ] レジストリ不使用・ポータブル config に影響なし
- [ ] GHA `cargo test` 緑

> 本 PR は説明のみ。実装は後続コミットで対応。SVG は MIT ライセンスのものに限定。
