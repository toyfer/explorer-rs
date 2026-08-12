# Final wire-up for IME filter (1 function in app.rs)

## Why this is separate
Bulk-uploading the full `src/app.rs` via API has truncated the file before.
Apply this **only** to `fn handle_typeahead` after checking out `fix/ime-filter-code`.

## Context7
- IME commit → `Event::Text` ([egui-winit](https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs))
- `wants_keyboard_input()` → TextEdit owns keys
- Unfocused + printable → filter (spec B)

## Replace entire `fn handle_typeahead` body with:

```rust
    fn handle_typeahead(&mut self, ctx: &egui::Context) {
        // Context7: Text-first IME; unfocused typing → filter (PR #10 / #15)
        if self.renaming || self.show_new_folder_dialog || self.confirm_delete.is_some() {
            return;
        }
        let wants = ctx.wants_keyboard_input();
        let frame = crate::typeahead_input::collect_frame_typed(ctx);

        if frame.backspace && !self.typeahead.is_empty() {
            self.typeahead.pop();
            self.typeahead_at = Some(Instant::now());
            if self.typeahead.is_empty() {
                self.status = "検索クリア".into();
                ctx.request_repaint();
            }
            return;
        }

        use crate::typeahead_input::TypeaheadAction;
        match crate::typeahead_input::decide_action(
            wants,
            &frame,
            &self.typeahead,
            self.typeahead_at,
        ) {
            TypeaheadAction::None => {}
            TypeaheadAction::ClearTypeahead { status } => {
                self.clear_typeahead();
                if let Some(s) = status {
                    self.status = s;
                }
            }
            TypeaheadAction::RouteToFilter { typed } => {
                self.focus_request = Some(FocusTarget::Filter);
                self.current_tab_mut().filter.push_str(&typed);
                self.request_refresh_async(false);
                self.status = format!("フィルタ: '{}'", self.current_tab().filter);
                self.clear_typeahead();
                ctx.request_repaint();
            }
            TypeaheadAction::RepaintAfter(d) => {
                ctx.request_repaint_after(d);
            }
        }
    }
```

## Optional: update empty-folder tip in `update_preview`
Change tip from `文字を入力でインクリメント検索` to `文字を入力でフィルタ`.

## Commands
```bash
git fetch origin && git checkout fix/ime-filter-code && git pull
# edit handle_typeahead as above
cargo test
git add src/app.rs && git commit -m "feat(ime): wire handle_typeahead to filter route"
git push
gh pr merge 15 --squash
```
