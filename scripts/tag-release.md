# Windows .exe を Release から取る手順

このリポジトリの CI は `windows-latest` で本物の PE（`explorer-rs.exe`）をビルドします。

## 方法 1: タグ push（推奨）

```bash
git clone https://github.com/toyfer/explorer-rs.git
cd explorer-rs
git checkout main
git pull
git tag v0.1.0
git push origin v0.1.0
```

数分後:

1. https://github.com/toyfer/explorer-rs/actions で **Release** ワークフローが成功していること
2. https://github.com/toyfer/explorer-rs/releases から
   - `explorer-rs-windows-x86_64-v0.1.0.zip` または
   - `explorer-rs.exe`
   をダウンロード

## 方法 2: Actions 手動実行

1. https://github.com/toyfer/explorer-rs/actions/workflows/release.yml
2. **Run workflow** → version に `v0.1.0` → Run
3. 完了後 **Artifacts** から `explorer-rs-exe` をダウンロード  
   （手動実行では GitHub Release ページは作られない場合あり。Artifacts を使う）

## 注意

- Linux で `cargo build` したバイナリは **ELF**。`.exe` にリネームしても Windows では動かない
- 本物の Windows バイナリは先頭バイトが `MZ`（PE）
