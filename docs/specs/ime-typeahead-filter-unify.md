# Spec: 日本語IMEでのインクリメントサーチ + フィルタ競合解消

## 現状の問題
1. **何もフォーカスがない状態の文字入力 → typeahead** は `handle_typeahead()` で `ctx.wants_keyboard_input() == false` の時のみ発火するため、TextEdit にフォーカスがないときは動作するが、IME 有効時に `Event::Text` が正しく来ない/ローマ字の `Key::A` などが二重で取られてしまう。
2. **日本語IME有効時も入力できるように** したいが、現在は `Event::Text` を優先しつつフォールバックで `Key::A-Z` を `a-z` に変換しており、IME確定の合成中文字が欠落する。
3. **インクリメントサーチとフィルタが競合**: 何か打つと typeahead が動くが、ユーザーはフィルタに打ちたい場合もあり、どちらに流すべきか曖昧。`フィルタ` TextEdit にフォーカスがないとフィルタに入らない。

## 要件
### A. 日本語IME対応の typeahead
- `egui::Event::Text(t)` を第一ソースとし、IME確定で渡される日本語（例: "あいう"）をそのまま `typeahead` に蓄積して前方一致/部分一致検索できること。
- `Event::Text` が空の時のみ `Key` フォールバックを使い、フォールバックは `ctrl/cmd/alt` 修飾時は無視。
- `wants_keyboard_input()` が true のときは typeahead を発火しない（フィルタ/アドレス/検索にフォーカスがあるときはそちらに譲る）。
- IME を有効にできるようにする: egui デフォルトで IME は有効だが、`winit` 側で `ime_allowed` が off になっていないか確認。必要なら `ctx.input_mut(|i| ...)` で IME ハンドリングを明示的に許可。
- 1.5s タイムアウトで `typeahead` クリア、`Backspace` で1文字削除、`Esc` でクリアは維持。

### B. フィルタ統合 — Context7 egui-winit準拠
- **Context7根拠**: `egui-winit::on_keyboard_input` は `winit::KeyEvent` を `Event::Key` と `Event::Text` に分岐してpushする。printableなtextがあれば `!is_cmd && pressed` のときのみ `Text` をpush [egui-winit](https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs) 。日本語IME確定は `Text("てすと")` として来るため `Text` 優先でないと欠落する
- **入力したらフィルタに記入されてフィルタが実行** してもいい、という要望を満たすため、フォーカスがない状態で文字（特に日本語）が打たれたら typeahead ではなく **フィルタ TextEdit にフォーカスを移して直接入力** する方式に変更する。
- 具体的には:
  - フォーカスなし + `Event::Text(t)`（1文字以上、controlでない）→ `focus_request = Filter` を立て、次フレームで `filter = t` をセットし `request_refresh_async(false)`。
  - 以降の連続入力はフィルタ TextEdit が `has_focus()` を持つので自然にフィルタに入る。
  - 英字1文字だけの軽い typeahead が必要な場合は `Ctrl+?` 的な明示操作に分離するか、フィルタが空のときのみ typeahead に流すオプションを残す（P1で検討）。
- `typeahead` と `filter` の責務を分離: typeahead は `Esc` クリア・タイムアウトで消える一時的選択、filter は `Tab::filter` として永続し `request_refresh_async` で一覧を絞り込む。

## 非スコープ
- プラグイン/高度な検索（P1の Everything 連携は別PR）。
- レジストリは触らない（ポータブル方針と同様）。

## 実装方針（実装済み）
- `handle_typeahead()` の先頭で `if ctx.wants_keyboard_input() { return; }` を維持しつつ、フォーカスなしで `Event::Text` が来たら `focus_request = Some(Filter)` + `tab.filter.push_str(&t)` する分岐を追加。Context7の `Event::Text` 優先 + `!is_cmd` ガードをそのまま適用
- IME 合成中は `egui` が `ImePreedit` 的イベントを出す場合があるため、Windows GHA で実機確認: 日本語キーボードレイアウトで `"か" -> "あ"` がフィルタに入るかログ取得。
- `ui_display_settings` や他ダイアログがフォーカスを持っているときは typeahead/filter どちらも発火しない。

## 受け入れ条件
- [ ] フォーカスなしで日本語入力（例: "てすと"）→ フィルタに "てすと" が入り一覧が絞り込まれる
- [ ] IME OFFの英字入力もフィルタに入る（typeahead へのフォールバックは廃止 or オプション）
- [ ] フィルタにフォーカスがあるときは従来通りフィルタ入力が継続する
- [ ] ダイアログ/リネーム中は入力が奪われない
- [ ] GHA Windows で `cargo test` 緑 + 手動 IME テスト（スクリーンショット or 動画）

> Context7 egui-winit Text/Key split準拠で実装。`Event::Text` を第一ソースにしフィルタへ直行させる方式で競合解消。
