# Spec slice 11b: ツールバー SVG 化

## Context7
- `egui_extras` with `all_loaders` + `install_image_loaders` loads SVG via embedded resvg
- Prefer `Image::from_bytes("bytes://….svg", include_bytes!(…))` over adding `resvg` to build.rs for toolbar (simpler, runtime-cached textures)

## Assets (in-repo, simple MIT stroke icons)
- `assets/icons/toolbar/chevron_left.svg`
- `chevron_right.svg`, `arrow_up.svg`, `refresh.svg`, `home.svg`

## Module
`src/toolbar_icons.rs` — `ToolbarIcon` + `icon_button(ui, icon, enabled, compact)`

## Wire-up
```bash
python3 scripts/apply_toolbar_svg.py
cargo test
```

## 受け入れ
- [ ] 戻る/進む/上へ/更新/ホームが SVG
- [ ] compact_ui で 14px / 通常 16px
- [ ] hover text 維持
- [ ] cargo test 緑
