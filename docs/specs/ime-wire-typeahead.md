# IME wire-up PR (follow-up to #15)

## Context7
- `egui-winit` pushes `Event::Text` for printable/IME commit when `pressed && !is_cmd`
- https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs

## What lands
1. `scripts/apply_handle_typeahead.py` — idempotent replace of `handle_typeahead`
2. Workflow on this branch runs the script and pushes the app.rs change
3. Result: unfocused typing (incl. Japanese IME) → filter bar + refresh

## After CI bot commit
- `cargo test` on Windows GHA must stay green
- Manual: focus none, type 「てすと」→ filter shows てすと
