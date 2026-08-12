#!/usr/bin/env python3
"""Replace ExplorerApp::handle_typeahead with typeahead_input wiring.

Avoids bulk-uploading app.rs through the API (truncate risk).
Idempotent: safe to re-run.
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
APP = ROOT / "src" / "app.rs"

NEW_FN = r'''
    fn handle_typeahead(&mut self, ctx: &egui::Context) {
        // Context7: Event::Text is primary (IME commit); unfocused printable → filter.
        // https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs
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
'''.lstrip("\n")

MARKER = "crate::typeahead_input::collect_frame_typed"


def main() -> int:
    src = APP.read_text(encoding="utf-8")
    if MARKER in src:
        print("handle_typeahead already wired; nothing to do")
        return 0

    # Match fn handle_typeahead ... until next fn at same indent (4 spaces + fn)
    pat = re.compile(
        r"    fn handle_typeahead\(&mut self, ctx: &egui::Context\) \{[\s\S]*?\n    \}\n(?=    fn )",
        re.MULTILINE,
    )
    new_src, n = pat.subn(NEW_FN + "\n", src, count=1)
    if n != 1:
        print("ERROR: could not find unique handle_typeahead", file=sys.stderr)
        return 1

    # Optional tip string update
    new_src = new_src.replace(
        "文字を入力でインクリメント検索",
        "文字を入力でフィルタ",
        1,
    )

    APP.write_text(new_src, encoding="utf-8")
    print(f"Updated {APP}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
