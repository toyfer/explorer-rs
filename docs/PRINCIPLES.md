# explorer-rs — Product principles

This document is the **source of truth** for scope and engineering.
If a feature is not listed under In-scope, it is **out** until explicitly promoted.

## North star

> **Only essential file-manager features, with stability and performance first.**

Inspired by the intersection of:

| Project | Steal |
|---|---|
| [Explorer++](https://explorerplusplus.com/) | Tabs, bookmarks, portable single binary, familiar Explorer UX |
| [Yazi](https://github.com/sxyazi/yazi) | Async I/O, cancel stale work, keep core tiny |
| [Kiorg](https://github.com/houqp/kiorg) | egui, async long work off UI thread, single binary, function over flash |
| FreeCommander / OneCommander | Dual pane practicality; speed as a product feature |

**Not** competing with Directory Opus, Files (WinUI), or Total Commander feature count.

## In-scope (P0 — must work, must stay fast)

- Tabs, dual pane, bookmarks, recent paths
- List / sort / filter / incremental typeahead search
- Copy, cut, paste (background), delete (trash / permanent with confirm)
- Rename, new folder/file, drag-and-drop
- Light preview (text capped, image capped)
- OS icons (lazy + cache), Japanese fonts, single Windows PE binary
- Async directory listing with generation cancel
- Debounced filesystem watcher
- Virtualized table rows

## P1 (high value, only after P0 is solid)

- Quick jump (recent + bookmarks + path)
- Natural sort (Name) — **done in Phase A**
- Copy/delete progress + cancel
- One-step undo (move/rename)
- Space toggles preview
- Optional Everything / external search delegation
- Persist column widths, window size, pane ratio

## Explicitly out (do not add without a principles update)

- Built-in terminal, plugin system
- FTP/SFTP/cloud
- Folder sync, advanced batch rename
- Full archive suite
- Video/PDF heavy preview
- Full shell context-menu hosting
- Cross-platform parity as a goal (Windows-first; other OS may build but are not gates)

## Architecture rules

1. **UI thread only paints and applies events.** FS and shell heavy work run on workers.
2. **Generation / cancel.** Stale `ListDone` / search results must not overwrite newer state.
3. **Lazy icons.** Prefer extension cache; load SHGetFileInfo for visible rows; cap cache size.
4. **Debounced preview.** Selection changes must not read large files every frame.
5. **Debounced watcher.** Pause or coalesce during bulk paste.
6. **Failures become status text**, not panics.
7. **Long paths on Windows** via `\\?\` when needed.
8. **Minimal config surface.** Prefer defaults over knobs.

## Windows real-machine testing (mandatory)

All validation that matters for releases runs on **GitHub Actions `windows-latest`**.

| Gate | Where |
|---|---|
| `cargo test` | CI Windows job (hard fail) |
| `cargo clippy` | CI (warn → eventually hard fail) |
| `cargo build --release` + PE check | CI + Release |
| Perf bench (`list` 5k entries) | CI → `bench-summary.json` on `ci-logs` |

### Agent feedback loop

1. Push branch / open PR → CI runs on Windows.
2. Logs artifact: `ci-logs-windows`.
3. Branch `ci-logs` path: `logs/<sha7>/` contains:
   - `test.log`, `build.log`, `bench-summary.json`, `meta.txt`, tails
4. Agent fetches `ci-logs` (or Actions artifact) → fix → push → repeat.
5. **No requirement for a local Windows desktop** for the agent.

### Success metrics (Phase A)

- List **5_000** files in temp dir: p50 list+sort **&lt; 500 ms** on GHA Windows (soft gate; recorded in JSON).
- Navigate spam does not apply older generation results.
- Release PE builds and is MZ.

## Versioning

- Patch: stability/perf/bugfix within scope
- Minor: P1 features after principles update
- Do not bump for out-of-scope experiments on feature branches
