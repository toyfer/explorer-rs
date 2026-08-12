# Spec: アドレス入力してエンターで確定（実装済み）

## Context7
- `Response::has_focus()` はウィジェット単位。`Context::wants_keyboard_input()` はどの TextEdit でも true なのでガードに使わない
- `egui-winit` は `Event::Key` と `Event::Text` を分離。IME 確定 Enter は Key として来るが、アドレス欄に has_focus が無い限り navigate しない
- https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs
- https://github.com/emilk/egui/blob/main/CHANGELOG.md (0.35 lost_focus / IME)

## 実装
```rust
let enter_pressed = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
let should_navigate = enter_pressed || ui.small_button("移動").clicked();
```

## 受け入れ
- [x] アドレス欄フォーカス + Enter で遷移
- [x] 検索/フィルタにフォーカスがある状態の Enter はアドレス遷移しない（has_focus ガード）
- [x] 移動ボタンは常時クリック可
- [ ] GHA `cargo test` 緑
