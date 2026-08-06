# explorer-rs — Explorer++ inspired File Manager

Windows 向けの高性能・高安定・**単一バイナリ**なファイルマネージャー。Explorer++ の思想を Rust + egui で再実装。

**リポジトリ:** https://github.com/toyfer/explorer-rs

## 重要: Windows 用 .exe の入手方法

このリポジトリの GitHub Actions **Release** ワークフローが `windows-latest` 上で `explorer-rs.exe` をビルドします。

1. タグを push: `git tag v0.1.1 && git push origin v0.1.1`
2. Actions → Release が完了するのを待つ
3. [Releases](https://github.com/toyfer/explorer-rs/releases) から `explorer-rs.exe` または zip をダウンロード

> Linux 上で `cargo build` したバイナリは **ELF** であり、拡張子を `.exe` にしても Windows では動きません。

## 特徴

- **単一バイナリ**: `cargo build --release` でポータブル exe（Windows）
- **日本語対応**: システム日本語フォント（游ゴシック / メイリオ 等）を自動読み込み
- **フォント変更**: プリセット選択・サイズ変更・任意 TTF/OTF 指定
- **高性能**: 仮想テーブル行 (`TableBuilder::rows`)、BG 検索/貼り付け
- **Explorer++ 互換**: タブ、2ペイン、ブックマーク、フィルタ、検索、プレビュー、ごみ箱、D&D

## 日本語フォントの使い方

1. 左サイドバー **表示** を開く
2. **フォント（日本語）** コンボで選択:
   - **自動 (日本語優先)** … OS の日本語フォントを自動検出（既定）
   - **游ゴシック / メイリオ / 游明朝 / ＭＳ ゴシック / Noto Sans CJK**
   - **カスタムファイル…** … 任意の `.ttf` / `.otf` / `.ttc`
   - **egui 標準** … CJK なし（デバッグ用）
3. **サイズ** スライダー（10–24 pt）
4. カスタム時は **参照…** でファイル選択、またはパスを直接入力
5. **フォントを適用** → 即時反映＆設定保存

設定は `%APPDATA%\explorer-rs\config.json` に保存されます。

```json
{
  "font_preset": "auto",
  "font_custom_path": null,
  "font_size": 14.0
}
```

### OS 別の自動検出先

| OS | 検出するフォント例 |
|---|---|
| Windows | `YuGothM.ttc`, `meiryo.ttc`, `msgothic.ttc`（`%WINDIR%\Fonts`） |
| Linux | Noto Sans CJK（`fonts-noto-cjk` パッケージ推奨） |
| macOS | ヒラギノ角ゴ |

Linux で日本語が □ になる場合:

```bash
sudo apt install fonts-noto-cjk
```

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
# 日本語表示用
sudo apt install fonts-noto-cjk
cargo build --release
```

## 設定

- Windows: `%APPDATA%\explorer-rs\config.json`
- Linux: `~/.config/explorer-rs/config.json`

## ライセンス

MIT OR Apache-2.0

## 変更履歴

### v0.1.1

- **日本語フォント対応**: システム CJK フォントを Proportional/Monospace の先頭に登録
- **フォント変更 UI**: プリセット・サイズ・カスタム TTF/OTF（参照ダイアログ）
- 設定に `font_preset` / `font_custom_path` / `font_size` を永続化

### v0.1.0

- Explorer++ 互換の基本機能一式
- 複数選択、削除確認、BG 検索/貼り付け、仮想スクロール
- Windows D&D、GitHub Actions による Windows 単一バイナリ配布
