# Apply IME filter unify in `handle_typeahead` (local 1-function edit)

Do **not** rewrite all of `src/app.rs` via bulk upload (truncate risk).
Edit only `ExplorerApp::handle_typeahead` after merging `typeahead_input` module.

## Context7

- IME commit → `egui::Event::Text` ([egui-winit keyboard](https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs))
- `wants_keyboard_input()` true → TextEdit owns keys; do not steal
- Unfocused + printable → filter bar (spec B)

## Replacement body

```rust
fn handle_typeahead(&mut self, ctx: &egui::Context) {
    if self.renaming || self.show_new_folder_dialog || self.confirm_delete.is_some() {
        return;
    }
    // Context7: TextEdit focus → let filter/address/search handle input
    if ctx.wants_keyboard_input() {
        return;
    }

    let frame = crate::typeahead_input::collect_frame_typed(ctx);

    if frame.backspace && !self.typeahead.is_empty() {
        self.typeahead.pop();
        self.typeahead_at = Some(std::time::Instant::now());
        if self.typeahead.is_empty() {
            self.status = "検索クリア".into();
            ctx.request_repaint();
        }
        return;
    }

    if frame.text.is_empty() {
        if let Some(at) = self.typeahead_at {
            if at.elapsed().as_millis() > 1500 {
                self.clear_typeahead();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        return;
    }

    // Spec B + Context7 Text-first: unfocused typing goes to filter
    if crate::typeahead_input::should_route_to_filter(false, &frame.text) {
        self.focus_request = Some(FocusTarget::Filter);
        self.current_tab_mut().filter.push_str(&frame.text);
        self.request_refresh_async(false);
        self.status = format!("フィルタ: '{}'", self.current_tab().filter);
        self.clear_typeahead();
        ctx.request_repaint();
        return;
    }
}
```

## Commands

```bash
git fetch origin
git checkout fix/ime-filter-code
# edit handle_typeahead as above
cargo test
git add src/app.rs && git commit -m "feat(ime): route unfocused typing to filter"
git push
```
