# Spec slice 11c: ファイルアイコン

## Context7
- Windows: `shell::icon_rgba` → `SHGetFileInfoW` → HICON → `GetDIBits` → RGBA [src/shell.rs](https://github.com/toyfer/explorer-rs/blob/main/src/shell.rs)
- egui: `ctx.load_texture(key, ColorImage, TextureOptions::LINEAR)` [src/app.rs](https://github.com/toyfer/explorer-rs/blob/main/src/app.rs)
- Phase A 方針: LRU 512

## 構成

| ファイル | 役割 |
|---|---|
| `src/file_icons.rs` | `LruIconCache` + `load_for_path` + `fallback_svg_bytes` + tests |
| `src/main.rs` | `mod file_icons;` |
| `src/app.rs` | `get_or_load_icon` を `file_icons::load_for_path` 経由に（配線スクリプト） |
| `scripts/apply_file_icons.py` | idempotent |
| `.github/workflows/apply-file-icons.yml` | bot 自動配線 |
| `LICENSE-THIRD-PARTY.md` | MIT ライセンス明記 |

## フォールバック
`SHGetFileInfo` 失敗 / 非 Windows / キャッシュ未取得のとき bundled SVG を RGBA 化。
- `resvg` 0.44 + `usvg` 0.44 + `tiny-skia` 0.12 を Cargo.toml に追加すると本実装になる
- 現状は `1×1` プレースホルダ（HICON 経路が主）

## 受け入れ
- [x] `LruIconCache::with_capacity` の eviction テスト
- [x] fallback SVG nonempty テスト
- [x] Windows CI 緑（`shell::icon_rgba` 経路）
- [ ] LRU 上限 512 維持（`cargo test` 通過）
- [ ] フォールバック SVG 描画は `resvg` 追加後の follow-up

Refs: #11 (closed draft)
Closes file-icon portion of text-rendering-and-svg-icons spec
