# explorer-rs — Explorer++ inspired File Manager

Windows 向けの高性能・高安定・**単一バイナリ**なファイルマネージャー。Explorer++ の思想を Rust + egui で再実装。

**リポジトリ:** https://github.com/toyfer/explorer-rs

## 重要: Windows 用 .exe の入手方法

このリポジトリの GitHub Actions **Release** ワークフローが `windows-latest` 上で `explorer-rs.exe` をビルドします。

1. タグを push: `git tag v0.1.0 && git push origin v0.1.0`
2. Actions → Release が完了するのを待つ
3. [Releases](https://github.com/toyfer/explorer-rs/releases) から `explorer-rs.exe` または zip をダウンロード

> Linux 上で `cargo build` したバイナリは **ELF** であり、拡張子を `.exe` にしても Windows では動きません。

## 特徴

- **単一バイナリ**: `cargo build --release` でポータブル exe（Windows）
- **高性能**: 仮想テーブル行 (`TableBuilder::rows`)、BG 検索/貼り付け
- **Explorer++ 互換**: タブ、2ペイン、ブックマーク、フィルタ、検索、プレビュー、ごみ箱、D&D
- **安定性**: 削除確認、パスサニタイズ、Rust メモリ安全

## 機能・ショートカット

| 機能 | ショートカット |
|---|---|
| タブ | Ctrl+T / Ctrl+W |
| 全選択 | Ctrl+A |
| 複数選択 | Ctrl+クリック / Shift+範囲 |
| コピー/切り取り/貼り付け | Ctrl+C / X / V |
| 削除 / 完全削除 | Del / Shift+Del（確認あり） |
| 名前変更 | F2 |
| 新規フォルダ | Ctrl+Shift+N |
| 更新 | F5 |
| 上へ / 戻る / 進む | Alt+↑ / ← / → |

## ローカルビルド

### Windows（本物の .exe）

```powershell
rustup update
$env:RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release
# → target\release\explorer-rs.exe
```

### Linux（開発・検証用 ELF）

```bash
cargo build --release
# → target/release/explorer-rs  (Linux only)
```

## 設定

- Windows: `%APPDATA%\explorer-rs\config.json`
- Linux: `~/.config/explorer-rs/config.json`

## ライセンス

MIT OR Apache-2.0

## 変更履歴

### v0.1.0

- Explorer++ 互換の基本機能一式
- 複数選択 (Ctrl/Shift/Ctrl+A)
- 削除確認ダイアログ
- バックグラウンド検索・貼り付け
- 仮想スクロール (TableBuilder rows)
- Windows D&D (`with_drag_and_drop(true)`) + ホバープレビュー
- TableBuilder 内蔵スクロールのみ（外側 ScrollArea 二重化なし）
- GitHub Actions による Windows 単一バイナリ配布
