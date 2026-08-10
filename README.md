# explorer-rs

Windows 向け・**単一バイナリ**のファイルマネージャー。  
[Explorer++](https://explorerplusplus.com/) の思想を **Rust + egui** で再実装。

**方針（必読）:** [docs/PRINCIPLES.md](docs/PRINCIPLES.md)

> **必要な機能だけ。安定とパフォーマンスを最優先。**  
> Windows 実機相当の検証は **すべて GitHub Actions (`windows-latest`)** で行う。

**リポジトリ:** https://github.com/toyfer/explorer-rs

---

## プロダクト契約（要約）

| 入れる（P0） | 入れない |
|---|---|
| タブ / 2ペイン / ブックマーク / 最近 | 内蔵ターミナル・プラグイン |
| 一覧・ソート（自然順）・フィルタ・typeahead | FTP / クラウド / 同期 |
| コピー・移動・削除・改名・新規・D&amp;D | 高度バッチリネーム |
| 軽量プレビュー（テキスト/画像） | 動画・PDF フルプレビュー |
| 非同期一覧・watcher・仮想スクロール | シェル拡張の完全再現 |
| OS アイコン・日本語フォント・単一 PE | 機能数での Opus / Files 競合 |

詳細な P0 / P1 / Out とアーキテクチャ規約は [docs/PRINCIPLES.md](docs/PRINCIPLES.md)。

---

## Windows 用 .exe の入手

GitHub Actions **Release** が `windows-latest` 上で `explorer-rs.exe` をビルドします。

1. `git tag vX.Y.Z && git push origin vX.Y.Z`
2. Actions → Release 完了を待つ
3. [Releases](https://github.com/toyfer/explorer-rs/releases) から zip / exe を取得

> Linux の `cargo build` 成果物は ELF です。拡張子を `.exe` にしても Windows では動きません。

---

## 特徴

- **単一バイナリ**（ポータブル PE）
- **日本語フォント**（BIZ UD / 游ゴシック / メイリオ 等）
- **高性能**: 仮想テーブル、BG 検索/貼り付け、非同期一覧、自然順ソート
- **Explorer++ 系**: タブ、2ペイン、ブックマーク、フィルタ、検索、プレビュー、ごみ箱、D&amp;D
- **最近使った場所** / コンパクト UI / 行高さ

---

## ショートカット

| 機能 | キー |
|---|---|
| タブ | Ctrl+T / Ctrl+W |
| アドレス / 検索 | Ctrl+L / Ctrl+F |
| 全選択 / 反転 | Ctrl+A / Ctrl+I |
| コピー / 切り取り / 貼り付け | Ctrl+C / X / V |
| パスをコピー | Ctrl+Shift+C |
| 削除 / 完全削除 | Del / Shift+Del |
| 名前変更 / 新規フォルダ | F2 / Ctrl+Shift+N |
| 更新 | F5 |
| 上へ / 戻る / 進む | Alt+↑←→ · Backspace=上へ |
| ホーム | Alt+Home |
| 2ペイン / 隠し / プレビュー | F10 / Ctrl+H / Ctrl+P |
| リスト | ↑↓ Home/End PgUp/PgDn + Shift |
| typeahead | 文字入力（1.5s リセット） |

---

## 開発・テスト（Windows = GitHub Actions）

ローカルに Windows が無くても開発できます。**ゲートは GHA のみ**です。

```text
push / PR → windows-latest
  cargo check / clippy / test  (test は hard fail)
  release build + PE magic
  bench: list 5000 entries → bench-summary.json
  → artifact ci-logs-windows
  → branch ci-logs/logs/<sha7>/
```

### エージェント向けログ取得

```text
https://github.com/toyfer/explorer-rs/tree/ci-logs/logs/<sha7>/
  meta.txt
  test.log / build.log / check.log / clippy.log
  bench-summary.json
  *.tail.txt
```

Actions の Artifacts からも同名ログを取得できます。

### ローカル（任意）

```powershell
# Windows
rustup update
$env:RUSTFLAGS="-C target-feature=+crt-static"
cargo test
cargo build --release
```

```bash
# Linux: 開発用のみ（リリースゲートではない）
sudo apt install fonts-noto-cjk
cargo test
```

---

## 設定

- Windows: `%APPDATA%\explorer-rs\config.json`
- Linux: `~/.config/explorer-rs/config.json`

キー例: `last_path`, `bookmarks`, `recent_paths`, `show_hidden`, `show_preview`, `dual_pane`, `theme_dark`, `font_preset`, `font_size`, `row_height_scale`, `compact_ui`

---

## ロードマップ（原則どおり）

| Phase | 内容 |
|---|---|
| **A（進行中）** | 原則文書化、自然順、長パス、ナビ非同期寄り、GHA ベンチ/ログ |
| **B** | クイックジャンプ、進捗/キャンセル、Undo、Space プレビュー、列幅永続化 |
| **C** | 需要確定後のみ（原則更新必須） |

---

## ライセンス

MIT OR Apache-2.0

## 変更履歴

### main (post-v0.1.4) — Phase A start

- プロダクト原則を README + `docs/PRINCIPLES.md` に固定
- Name ソートを自然順に
- Windows 長パスヘルパ
- ナビのブロッキング refresh 削減（async 前提）
- CI: Windows ベンチ + `bench-summary.json` を ci-logs に公開、test hard-fail

### v0.1.4

- コンパクト UI / 行高 / 最近 / キーボード強化 / サニタイズ強化 など

### v0.1.0–v0.1.3

- 基本機能、日本語フォント、OS アイコン、Windows 単一バイナリ配布
