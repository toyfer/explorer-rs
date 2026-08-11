# IME filter unify — 実装メモ（Context7）

## 変更箇所
`src/app.rs` の `handle_typeahead` のみ。

## ロジック（Context7 egui-winit 準拠）

1. `renaming` / `show_new_folder_dialog` / `confirm_delete` 中は return
2. `ctx.wants_keyboard_input()` が true → return（フィルタ/アドレス/検索に譲る）
3. `Event::Text` を第一ソースで収集（制御文字除外）
4. Text が空のときのみ `Key::A-Z/0-9` フォールバック（ctrl/cmd/alt 除外）
5. **typed が空でない → `focus_request = Filter` + `tab.filter.push_str` + `request_refresh_async` でフィルタ直行**
6. 旧 typeahead の starts_with 走査は削除（競合解消）

## なぜこれで IME が通るか
IME 確定文字は winit → egui で `Event::Text` になる。Key フォールバックに頼らない。

## 手動テスト（Windows）
1. リストにフォーカスなし
2. IME ON で「てすと」確定
3. フィルタ欄に「てすと」が入り一覧が絞り込まれること
4. フィルタにフォーカスして追加入力できること
