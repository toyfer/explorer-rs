# explorer-rs — Explorer++ inspired File Manager

Windows 向けの高性能・高安定・**単一バイナリ**なファイルマネージャー。Explorer++ の思想を Rust + egui で再実装。

**リポジトリ:** https://github.com/toyfer/explorer-rs

## 重要: Windows 用 .exe の入手方法

このリポジトリの GitHub Actions **Release** ワークフローが `windows-latest` 上で `explorer-rs.exe` をビルドします。

1. タグを push: `git tag v0.1.4 && git push origin v0.1.4`
2. Actions → Release が完了するのを待つ
3. [Releases](https://github.com/toyfer/explorer-rs/releases) から `explorer-rs.exe` または zip をダウンロード

> Linux 上で `cargo build` したバイナリは **ELF** であり、拡張子を `.exe` にしても Windows では動きません。

## 特徴

- **単一バイナリ**: `cargo build --release` でポータブル exe（Windows）
- **日本語対応**: システム日本語フォント（BIZ UDゴシック / 游ゴシック / メイリオ 等）
- **フォント変更**: プリセット選択・サイズ変更・任意 TTF/OTF 指定
- **高性能**: 仮想テーブル行 (`TableBuilder::rows`)、BG 検索/貼り付け、非同期一覧
- **Explorer++ 互換**: タブ、2ペイン、ブックマーク、フィルタ、検索、プレビュー、ごみ箱、D&D
- **最近使った場所**: サイドバーに自動記録（最大15件）
- **コンパクトUI / 行高さ調整**: 表示設定から変更可能

## 機能・ショートカット

| 機能 | ショートカット |
|---|---|
| タブ | Ctrl+T / Ctrl+W |
| アドレスバーへ | Ctrl+L |
| 検索へ | Ctrl+F |
| 全選択 / 選択反転 | Ctrl+A / Ctrl+I |
| 複数選択 | Ctrl+クリック / Shift+範囲 / Shift+矢印 |
| コピー/切り取り/貼り付け | Ctrl+C / X / V |
| パスをコピー | Ctrl+Shift+C |
| 削除 / 完全削除 | Del / Shift+Del（確認あり） |
| 名前変更 | F2 |
| 新規フォルダ | Ctrl+Shift+N |
| 更新 | F5 |
| 上へ / 戻る / 進む | Alt+↑ / ← / → · Backspace=上へ |
| ホーム | Alt+Home |
| 2ペイン切替 | F10 |
| 隠しファイル | Ctrl+H |
| プレビュー切替 | Ctrl+P |
| リスト移動 | ↑↓ · Home/End · PgUp/PgDn |
| インクリメント検索 | 文字入力（1.5秒でリセット） |

## 日本語フォントの使い方

1. 左サイドバー **表示** を開く
2. **フォント（日本語）** コンボで選択
3. **サイズ** スライダー（10–24 pt）
4. **コンパクトUI** / **行の高さ** で一覧密度を調整
5. **フォントを適用** → 即時反映＆設定保存

設定は `%APPDATA%\explorer-rs\config.json` に保存されます。

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
sudo apt install fonts-noto-cjk
cargo build --release
cargo test
```

## 設定

- Windows: `%APPDATA%\explorer-rs\config.json`
- Linux: `~/.config/explorer-rs/config.json`

主なキー: `last_path`, `bookmarks`, `recent_paths`, `show_hidden`, `show_preview`, `dual_pane`, `theme_dark`, `font_preset`, `font_size`, `row_height_scale`, `compact_ui`

## ライセンス

MIT OR Apache-2.0

## 変更履歴

### v0.1.4

**UI**
- コンパクトUI / 行高さスケール
- ステータスバーに選択件数＋合計サイズ
- 最近使った場所（サイドバー）
- フォーカス行ハイライト、パンくず末尾の余分な › を除去
- 自動更新時にステータス文言を上書きしない

**機能**
- Home / End / PgUp / PgDn（+ Shift 範囲選択）
- Shift+矢印 範囲選択、Ctrl+I 選択反転
- Ctrl+Shift+C パスコピー、名前コピー
- Alt+Home ホーム、F10 2ペイン、Ctrl+H 隠し、Ctrl+P プレビュー
- Ctrl+L アドレス / Ctrl+F 検索フォーカス
- Backspace で上へ（typeahead 中でないとき）
- 検索結果上限・深度の拡張

**バグ修正**
- ナビゲート時に選択をクリア
- アドレス Enter はアドレス欄フォーカス時のみ
- 削除確認ダイアログの二重発火防止
- ファイル名サニタイズ強化（予約デバイス名・不正文字）
- typeahead の空フォルダ panic 回避

### v0.1.1–v0.1.3

- 日本語フォント対応・フォント変更 UI
- OS 標準アイコン、IME インクリメント検索
- Windows 単一バイナリ配布

### v0.1.0

- Explorer++ 互換の基本機能一式
- 複数選択、削除確認、BG 検索/貼り付け、仮想スクロール
