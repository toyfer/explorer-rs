# Spec slice 11c: ファイルアイコン (LRU + Windows HICON + MIT SVG fallback)

## Context7
- `shell::icon_rgba` (Windows) = `SHGetFileInfoW` → HICON → `GetDIBits` → RGBA [src/shell.rs](https://github.com/toyfer/explorer-rs/blob/main/src/shell.rs)
- `egui_extras::install_image_loaders` (既存) で SVG バイト URI を `Image::from_bytes` で読める
- `eg ui::Context::load_texture` でキャッシュにテクスチャ登録

## 実装
- `src/file_icons.rs`
  - `LruIconCache` (HashMap + VecDeque, cap 512)
  - `load_for_path(ctx, cache, path, is_dir)` = cache → SHGetFileInfo → bundled SVG fallback
  - `fallback_svg_bytes(ext, is_dir)` = 拡張子別 MIT 風 SVG
- `src/main.rs`: `mod file_icons;`
- `scripts/apply_file_icons.py`: app.rs の `get_or_load_icon` 置換 (idempotent)
- `.github/workflows/apply-file-icons.yml`: bot が app.rs を push (PR #16/#17 と同じパターン)

## 受け入れ
- [x] LRU eviction unit test
- [x] SVG アセット nonempty
- [ ] 配線後、SHGetFileInfo 失敗時 MIT SVG 表示
- [ ] キャッシュ上限 512 維持
