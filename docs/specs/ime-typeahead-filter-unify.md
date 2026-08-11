# Spec: 日本語IME + フィルタ統合（実装ブランチ fix/ime-filter-impl）

## Context7

`egui-winit::on_keyboard_input` は:
1. `Event::Key` を push
2. printable な text があれば `!is_cmd && pressed` のときだけ `Event::Text` を push

日本語IME確定は `Event::Text("てすと")` として来る。`Key::A-Z` フォールバックだけだと欠落する。

出典: https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs

## 実装方針

```rust
// handle_typeahead
if wants_keyboard_input() { return; } // TextEdit がフォーカス中は譲る
// Event::Text を第一ソースで収集
if !typed.is_empty() {
    focus_request = Filter;
    tab.filter.push_str(&typed);
    request_refresh_async(false);
    return; // typeahead との競合を解消
}
```

## 受け入れ
- [ ] フォーカスなしで「てすと」→ フィルタに入り絞り込み
- [ ] フィルタフォーカス中は従来通り入力継続
- [ ] ダイアログ/リネーム中は奪わない
- [ ] GHA cargo test 緑
