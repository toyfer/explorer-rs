#!/usr/bin/env python3
"""Replace ExplorerApp::get_or_load_icon with file_icons::load_for_path.

Idempotent. Avoids bulk-uploading app.rs (truncate risk).
"""
from __future__ import annotations

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
APP = ROOT / "src" / "app.rs"

MARKER = "crate::file_icons::load_for_path"

OLD_BLOCK = r'''    fn get_or_load_icon(
        &self,
        ctx: &egui::Context,
        path: &Path,
        is_dir: bool,
    ) -> Option<egui::TextureHandle> {
        let key = Self::icon_cache_key(path, is_dir);
        if let Some(h) = self.icon_cache.borrow().get(&key) {
            return Some(h.clone());
        }
        #[cfg(windows)]
        {
            if let Some((rgba, w, h)) = shell::icon_rgba(path, is_dir) {
                if w > 0 && h > 0 && rgba.len() == (w * h * 4) as usize {
                    let img =
                        egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                    let tex = ctx.load_texture(key.clone(), img, egui::TextureOptions::LINEAR);
                    self.icon_cache.borrow_mut().insert(key.clone(), tex.clone());
                    return Some(tex);
                }
            }
        }
        let _ = (ctx, path, is_dir);
        None
    }'''.strip("\n")

NEW_BLOCK = r'''    fn get_or_load_icon(
        &self,
        ctx: &egui::Context,
        path: &Path,
        is_dir: bool,
    ) -> Option<egui::TextureHandle> {
        // Context7 (PR #11 slice C): SHGetFileInfo via shell::icon_rgba is the
        // preferred path; on miss, file_icons::load_for_path falls back to
        // bundled MIT-licensed SVGs with LRU 512 cache.
        let key = Self::icon_cache_key(path, is_dir);
        if let Some(h) = self.icon_cache.borrow().get(&key) {
            return Some(h.clone());
        }
        if let Some(tex) = crate::file_icons::load_for_path(ctx, &self.icon_cache, path, is_dir) {
            return Some(tex);
        }
        let _ = key;
        None
    }'''.strip("\n")


def main() -> int:
    src = APP.read_text(encoding="utf-8")
    if MARKER in src:
        print("file_icons already wired; nothing to do")
        return 0
    if OLD_BLOCK not in src:
        print("ERROR: get_or_load_icon block not found verbatim; bailing", file=sys.stderr)
        return 1
    new_src = src.replace(OLD_BLOCK, NEW_BLOCK, 1)
    APP.write_text(new_src, encoding="utf-8")
    print(f"Updated {APP}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
