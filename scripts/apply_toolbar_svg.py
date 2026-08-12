#!/usr/bin/env python3
"""Replace emoji toolbar buttons with toolbar_icons SVG buttons.

Idempotent. Avoids bulk-uploading app.rs.
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
APP = ROOT / "src" / "app.rs"

MARKER = "crate::toolbar_icons::icon_button"

OLD_BLOCK = r'''
                let back_ok = !self.current_tab().history_back.is_empty();
                let fwd_ok = !self.current_tab().history_forward.is_empty();
                ui.add_enabled_ui(back_ok, |ui| {
                    if ui.button(btn("◀")).on_hover_text("戻る").clicked() {
                        self.run_command(Command::GoBack);
                    }
                });
                ui.add_enabled_ui(fwd_ok, |ui| {
                    if ui.button(btn("▶")).on_hover_text("進む").clicked() {
                        self.run_command(Command::GoForward);
                    }
                });
                if ui.button(btn("⬆")).on_hover_text("上へ Alt+↑").clicked() {
                    self.run_command(Command::GoUp);
                }
                if ui.button(btn("⟳")).on_hover_text("更新 F5").clicked() {
                    self.run_command(Command::Refresh);
                }
                if ui.button(btn("🏠")).on_hover_text("ホーム Alt+Home").clicked() {
                    self.run_command(Command::GoHome);
                }
'''.strip("\n")

NEW_BLOCK = r'''
                let back_ok = !self.current_tab().history_back.is_empty();
                let fwd_ok = !self.current_tab().history_forward.is_empty();
                // Context7: SVG via egui_extras image loaders (install_image_loaders in main)
                if crate::toolbar_icons::icon_button(
                    ui,
                    crate::toolbar_icons::ToolbarIcon::Back,
                    back_ok,
                    compact,
                ) {
                    self.run_command(Command::GoBack);
                }
                if crate::toolbar_icons::icon_button(
                    ui,
                    crate::toolbar_icons::ToolbarIcon::Forward,
                    fwd_ok,
                    compact,
                ) {
                    self.run_command(Command::GoForward);
                }
                if crate::toolbar_icons::icon_button(
                    ui,
                    crate::toolbar_icons::ToolbarIcon::Up,
                    true,
                    compact,
                ) {
                    self.run_command(Command::GoUp);
                }
                if crate::toolbar_icons::icon_button(
                    ui,
                    crate::toolbar_icons::ToolbarIcon::Refresh,
                    true,
                    compact,
                ) {
                    self.run_command(Command::Refresh);
                }
                if crate::toolbar_icons::icon_button(
                    ui,
                    crate::toolbar_icons::ToolbarIcon::Home,
                    true,
                    compact,
                ) {
                    self.run_command(Command::GoHome);
                }
'''.strip("\n")


def main() -> int:
    src = APP.read_text(encoding="utf-8")
    if MARKER in src:
        print("toolbar SVG already wired; nothing to do")
        return 0
    if "btn(\"◀\")" not in src and "btn(\"◀\")" not in src:
        # try unicode escape forms
        pass
    if OLD_BLOCK not in src:
        # more tolerant: replace line-by-line patterns
        print("Exact block not found; trying regex…", file=sys.stderr)
        pat = re.compile(
            r"let back_ok = !self\.current_tab\(\)\.history_back\.is_empty\(\);\n"
            r"[\s\S]*?"
            r"if ui\.button\(btn\(\"🏠\"\)\)\.on_hover_text\(\"ホーム Alt\+Home\"\)\.clicked\(\) \{\n"
            r"\s*self\.run_command\(Command::GoHome\);\n"
            r"\s*\}\n",
        )
        new_src, n = pat.subn(NEW_BLOCK + "\n", src, count=1)
        if n != 1:
            print("ERROR: could not locate toolbar emoji block", file=sys.stderr)
            return 1
    else:
        new_src = src.replace(OLD_BLOCK, NEW_BLOCK, 1)

    APP.write_text(new_src, encoding="utf-8")
    print(f"Updated {APP}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
