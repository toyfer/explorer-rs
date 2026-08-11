# Spec: アドレス入力してエンターで確定

## 現状
- `src/app.rs` の `navigate_address_bar()` は既に存在するが、トップバーの `TextEdit::singleline` で Enter がアドレスにフォーカスがない状態でも拾われる可能性がある。
- `should_navigate = (resp.has_focus() && enter_pressed) || move_button.clicked()` でガードしているが、他フィールド（検索・フィルタ・ダイアログ）にフォーカスがある場合の挙動が不安定。

## 要件
- アドレス欄にフォーカスがあるときのみ `Enter` で確定（`navigate_address_bar()`）を実行すること。
- アドレス以外の入力欄（検索、フィルタ、名前変更、新規フォルダ）にフォーカスがあるときは、それぞれの確定処理を優先し、アドレス遷移は発火しないこと。
- `移動` ボタンは常時クリックで遷移できること。
- IME 確定中の Enter は誤遷移しないこと（`Event::Text` vs `Key::Enter` の競合を避ける）。
- 存在しないパス入力時は `status = "見つかりません: ..."` を表示し、クラッシュしない。
- 存在するファイルパス入力時は `shell::open_with_shell`、ディレクトリは `navigate_to` + `note_navigation` + `sync_watchers`。

## 実装方針（Context7 egui）
- `egui::Response::has_focus()` はウィジェット単位のフォーカス。`Context::wants_keyboard_input()` はどのTextEditでもtrueなので粗すぎる — `resp.has_focus()` で厳密に判定する。
- `egui-winit` は `winit::KeyEvent` を `Event::Key` と `Event::Text` に分岐してpushする。IME確定の日本語は `Text` として来るため `Key::Enter` と混同しない。
- `TopBar` の `resp.has_focus()` ガードを維持しつつ、`ui.input(|i| i.key_pressed(Enter))` を `resp.has_focus()` とANDで判定。他の `TextEdit` が `has_focus()` を持つ場合は早期return。
- `egui 0.35` の `lost_focus` 修正と `owns_ime_events` 改善を踏まえ、フォーカス移動の中途半端なフレームでも `has_focus` が安定する。
- テスト: `normalize_path_input` が `/` と `\` を正規化する既存テストに加え、`address="C:/tmp"` → Enter → `current == "C:\tmp"` を GHA Windows で検証。
- レジストリ非使用・ポータブル config に影響なし。

## 受け入れ条件
- [ ] アドレス欄フォーカス + Enter でフォルダ遷移できる
- [ ] 検索/フィルタにフォーカスがある状態の Enter はアドレス遷移しない
- [ ] IME 確定 Enter で誤遷移しない
- [ ] GHA `cargo test` 緑

> 実装済み: `src/app.rs` TopBarで `resp.has_focus() && enter_pressed` を厳格化。Context7 `Response::has_focus` / `egui-winit Text/Key split` 準拠。
