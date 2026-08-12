#!/usr/bin/env python3
"""Wire ExplorerApp::get_or_load_icon to file_icons::load_for_path.

Replaces the HashMap cache + Windows HICON block with the LRU-backed
file_icons::load_for_path that also handles the SVG fallback.
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
APP = ROOT / "src" / "app.rs"

MARKER = "crate::file_icons::load_for_path"


OLD_BODY = r'''    fn get_or_load_icon(
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
    }'''.lstrip("\n")


NEW_BODY = r'''    fn get_or_load_icon(
        &self,
        ctx: &egui::Context,
        path: &Path,
        is_dir: bool,
    ) -> Option<egui::TextureHandle> {
        // Context7: file_icons LRU 512 + Windows SHGetFileInfo + MIT SVG fallback
        crate::file_icons::load_for_path(ctx, &self.icon_cache, path, is_dir)
    }'''.lstrip("\n")


def main() -> int:
    src = APP.read_text(encoding="utf-8")
    if MARKER in src:
        print("file_icons already wired; nothing to do")
        return 0
    if OLD_BODY not in src:
        print("ERROR: could not find old get_or_load_icon block", file=sys.stderr)
        return 1
    new_src = src.replace(OLD_BODY, NEW_BODY, 1)

    # Replace icon_cache field type HashMap<...> -> file_icons::LruIconCache
    new_src = re.sub(
        r"icon_cache: RefCell<HashMap<String, egui::TextureHandle>>,",
        "icon_cache: RefCell<crate::file_icons::LruIconCache>,",
        new_src,
        count=1,
    )
    new_src = re.sub(
        r"icon_cache: RefCell::new\(HashMap::new\(\)\),",
        "icon_cache: RefCell::new(crate::file_icons::LruIconCache::with_capacity(512)),",
        new_src,
        count=1,
    )

    # Drop the now-unused icon_cache_key helper (optional cleanup)
    new_src = re.sub(
        r"    fn icon_cache_key\([\s\S]+?    \}\n",
        "",
        new_src,
        count=1,
    )

    APP.write_text(new_src, encoding="utf-8")
    print(f"Updated {APP}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
