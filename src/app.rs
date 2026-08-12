use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui;

use crate::commands::{self, BgEvent, BgReceiver, BgSender, Command};
use crate::config::{AppConfig, Bookmark, FontPreset, SortBy};
use crate::fonts;
use crate::fs_ops::{self, Clipboard, ClipboardMode};
use crate::shell;
use crate::tab::{FileEntry, Tab};
use crate::watch;

pub struct ExplorerApp {
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub pane2: Option<Tab>,
    pub active_pane: u8,
    pub address: String,
    pub search_query: String,
    pub search_results: Vec<FileEntry>,
    pub searching: bool,
    pub pasting: bool,
    pub status: String,
    pub config: AppConfig,
    pub clipboard: Option<Clipboard>,
    pub show_hidden: bool,
    pub dual_pane: bool,
    pub preview_text: String,
    pub rename_buffer: String,
    pub renaming: bool,
    pub new_folder_name: String,
    pub show_new_folder_dialog: bool,
    pub last_error: Option<String>,
    pub confirm_delete: Option<ConfirmDelete>,
    pub font_path_edit: String,
    typeahead: String,
    typeahead_at: Option<Instant>,
    watcher: Option<watch::WatchedDir>,
    watcher2: Option<watch::WatchedDir>,
    list_gen: u64,
    list_gen_p2: u64,
    listing: bool,
    listing_p2: bool,
    bg_tx: BgSender,
    bg_rx: BgReceiver,
    icon_cache: RefCell<crate::file_icons::LruIconCache>,
    focus_request: Option<FocusTarget>,
    scroll_to_row: Option<usize>,
    rename_focus_once: bool,
    new_folder_focus_once: bool,
    pending_clipboard_text: Option<String>,
}

#[derive(Clone, Copy)]
enum FocusTarget {
    Address,
    Search,
    Filter,
}

#[derive(Clone)]
pub struct ConfirmDelete {
    pub paths: Vec<PathBuf>,
    pub permanent: bool,
}

impl ExplorerApp {
    pub fn new(start_path: PathBuf, config: AppConfig, _cc: &eframe::CreationContext<'_>) -> Self {
        let show_hidden = config.show_hidden;
        let dual_pane = config.dual_pane;
        let sort_by = config.sort_by;
        let sort_desc = config.sort_desc;
        let font_path_edit = config.font_custom_path.clone().unwrap_or_default();
        let tab = Tab::new(start_path.clone(), show_hidden, sort_by, sort_desc);
        let pane2 = if dual_pane {
            Some(Tab::new(start_path.clone(), show_hidden, sort_by, sort_desc))
        } else {
            None
        };
        let (bg_tx, bg_rx) = commands::channel();
        let mut app = Self {
            tabs: vec![tab],
            active: 0,
            pane2,
            active_pane: 0,
            address: start_path.display().to_string(),
            search_query: String::new(),
            search_results: vec![],
            searching: false,
            pasting: false,
            status: "準備完了".into(),
            config,
            clipboard: None,
            show_hidden,
            dual_pane,
            preview_text: String::new(),
            rename_buffer: String::new(),
            renaming: false,
            new_folder_name: "新しいフォルダ".into(),
            show_new_folder_dialog: false,
            last_error: None,
            confirm_delete: None,
            font_path_edit,
            typeahead: String::new(),
            typeahead_at: None,
            watcher: None,
            watcher2: None,
            list_gen: 0,
            list_gen_p2: 0,
            listing: false,
            listing_p2: false,
            bg_tx,
            bg_rx,
            icon_cache: RefCell::new(crate::file_icons::LruIconCache::with_capacity(512)),
            focus_request: None,
            scroll_to_row: None,
            rename_focus_once: false,
            new_folder_focus_once: false,
            pending_clipboard_text: None,
        };
        app.config.push_recent(start_path);
        app.sync_watchers();
        app.update_preview();
        app
    }

    fn current_tab(&self) -> &Tab {
        if self.active_pane == 1 {
            if let Some(p) = &self.pane2 {
                return p;
            }
        }
        &self.tabs[self.active]
    }
    fn current_tab_mut(&mut self) -> &mut Tab {
        if self.active_pane == 1 {
            if let Some(p) = self.pane2.as_mut() {
                return p;
            }
        }
        &mut self.tabs[self.active]
    }
    fn sync_address(&mut self) {
        self.address = self.current_tab().current.display().to_string();
    }
    fn note_navigation(&mut self) {
        let cur = self.current_tab().current.clone();
        self.config.push_recent(cur);
    }
    fn sync_config_from_state(&mut self) {
        self.config.show_hidden = self.show_hidden;
        self.config.dual_pane = self.dual_pane;
        let (sort_by, sort_desc, last_path) = {
            let tab = self.current_tab();
            (tab.sort_by, tab.sort_desc, tab.current.clone())
        };
        self.config.sort_by = sort_by;
        self.config.sort_desc = sort_desc;
        self.config.last_path = Some(last_path.clone());
        self.config.push_recent(last_path);
        let path = self.font_path_edit.trim();
        self.config.font_custom_path = if path.is_empty() {
            None
        } else {
            Some(path.to_string())
        };
    }
    fn apply_font_settings(&mut self, ctx: &egui::Context) {
        let path = self.font_path_edit.trim();
        self.config.font_custom_path = if path.is_empty() {
            None
        } else {
            Some(path.to_string())
        };
        if self.config.font_preset == FontPreset::Custom && path.is_empty() {
            self.status = "カスタムフォントのパスを指定してください".into();
            return;
        }
        let msg = fonts::apply_fonts(ctx, &self.config);
        self.status = format!("適用しました — {msg}");
        let _ = self.config.save();
    }
    fn sync_watchers(&mut self) {
        let cur = self.tabs[self.active].current.clone();
        let need = self.watcher.as_ref().map(|w| &w.path) != Some(&cur);
        if need {
            self.watcher = watch::watch_dir(self.bg_tx.clone(), cur);
        }
        if let Some(p2) = &self.pane2 {
            let cur2 = p2.current.clone();
            let need2 = self.watcher2.as_ref().map(|w| &w.path) != Some(&cur2);
            if need2 {
                self.watcher2 = watch::watch_dir(self.bg_tx.clone(), cur2);
            }
        } else {
            self.watcher2 = None;
        }
    }
    fn request_refresh_async(&mut self, is_pane2: bool) {
        if is_pane2 {
            if let Some(p2) = &self.pane2 {
                self.list_gen_p2 += 1;
                let gen = self.list_gen_p2;
                self.listing_p2 = true;
                commands::spawn_list(
                    self.bg_tx.clone(),
                    gen,
                    true,
                    p2.current.clone(),
                    self.show_hidden,
                    p2.filter.clone(),
                    p2.sort_by,
                    p2.sort_desc,
                );
            }
        } else {
            let tab = &self.tabs[self.active];
            self.list_gen += 1;
            let gen = self.list_gen;
            self.listing = true;
            commands::spawn_list(
                self.bg_tx.clone(),
                gen,
                false,
                tab.current.clone(),
                self.show_hidden,
                tab.filter.clone(),
                tab.sort_by,
                tab.sort_desc,
            );
        }
    }
    fn poll_bg(&mut self, ctx: &egui::Context) {
        let mut events = Vec::new();
        while let Ok(ev) = self.bg_rx.try_recv() {
            events.push(ev);
        }
        for ev in events {
            match ev {
                BgEvent::SearchDone { query, results } => {
                    if query == self.search_query {
                        let n = results.len();
                        self.search_results = results;
                        self.searching = false;
                        self.status = format!("検索結果: {n} 件");
                    }
                }
                BgEvent::PasteDone {
                    message,
                    ok: _,
                    err,
                } => {
                    self.pasting = false;
                    if err > 0 {
                        self.last_error = Some(format!("失敗 {err} 件"));
                    }
                    self.status = message;
                    self.request_refresh_async(false);
                    if self.dual_pane {
                        self.request_refresh_async(true);
                    }
                    self.sync_watchers();
                    self.update_preview();
                }
                BgEvent::FsChanged => {
                    self.request_refresh_async(false);
                    if self.dual_pane {
                        self.request_refresh_async(true);
                    }
                }
                BgEvent::ListDone {
                    generation,
                    is_pane2,
                    entries,
                    error,
                } => {
                    if is_pane2 {
                        if generation == self.list_gen_p2 {
                            self.listing_p2 = false;
                            if let Some(p2) = self.pane2.as_mut() {
                                p2.apply_list(entries, error);
                            }
                            self.update_preview();
                        }
                    } else if generation == self.list_gen {
                        self.listing = false;
                        self.tabs[self.active].apply_list(entries, error);
                        self.sync_address();
                        self.update_preview();
                    }
                }
            }
            ctx.request_repaint();
        }
        if self.searching || self.pasting || self.listing || self.listing_p2 {
            ctx.request_repaint();
        }
    }
    fn open_path(&mut self, path: &Path) {
        if path.is_dir() {
            self.clear_typeahead();
            self.current_tab_mut().navigate_to(path.to_path_buf());
            self.sync_address();
            self.note_navigation();
            self.sync_watchers();
            self.update_preview();
        } else {
            match shell::open_with_shell(path) {
                Ok(_) => self.status = format!("開きました: {}", path.display()),
                Err(e) => self.status = format!("開けませんでした: {e}"),
            }
        }
    }
    fn clear_typeahead(&mut self) {
        self.typeahead.clear();
        self.typeahead_at = None;
    }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "exe" || ext == "ico" || ext == "lnk" {
            return format!("file:{}", path.display());
        }
        format!("ext:{ext}")
    }
    fn get_or_load_icon(
        &self,
        ctx: &egui::Context,
        path: &Path,
        is_dir: bool,
    ) -> Option<egui::TextureHandle> {
        // Context7: file_icons LRU 512 + Windows SHGetFileInfo + MIT SVG fallback
        crate::file_icons::load_for_path(ctx, &self.icon_cache, path, is_dir)
    }
    fn handle_typeahead(&mut self, ctx: &egui::Context) {
        // Context7: Event::Text is primary (IME commit); unfocused printable → filter.
        // https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs
        if self.renaming || self.show_new_folder_dialog || self.confirm_delete.is_some() {
            return;
        }
        let wants = ctx.wants_keyboard_input();
        let frame = crate::typeahead_input::collect_frame_typed(ctx);

        if frame.backspace && !self.typeahead.is_empty() {
            self.typeahead.pop();
            self.typeahead_at = Some(Instant::now());
            if self.typeahead.is_empty() {
                self.status = "検索クリア".into();
                ctx.request_repaint();
            }
            return;
        }

        use crate::typeahead_input::TypeaheadAction;
        match crate::typeahead_input::decide_action(
            wants,
            &frame,
            &self.typeahead,
            self.typeahead_at,
        ) {
            TypeaheadAction::None => {}
            TypeaheadAction::ClearTypeahead { status } => {
                self.clear_typeahead();
                if let Some(s) = status {
                    self.status = s;
                }
            }
            TypeaheadAction::RouteToFilter { typed } => {
                self.focus_request = Some(FocusTarget::Filter);
                self.current_tab_mut().filter.push_str(&typed);
                self.request_refresh_async(false);
                self.status = format!("フィルタ: '{}'", self.current_tab().filter);
                self.clear_typeahead();
                ctx.request_repaint();
            }
            TypeaheadAction::RepaintAfter(d) => {
                ctx.request_repaint_after(d);
            }
        }
    }

    fn update_preview(&mut self) {
        if let Some(entry) = self.current_tab().primary_selected().cloned() {
            if entry.is_dir {
                let count = std::fs::read_dir(&entry.path)
                    .map(|rd| rd.count())
                    .unwrap_or(0);
                self.preview_text = format!(
                    "フォルダ: {}\nパス: {}\n\n含まれる項目: {} 個",
                    entry.name,
                    entry.path.display(),
                    count
                );
            } else {
                let ext = entry.ext.to_lowercase();
                let text_exts = [
                    "txt", "md", "rs", "toml", "json", "yaml", "yml", "csv", "log", "ini", "cfg",
                    "py", "js", "ts", "html", "css", "xml", "bat", "ps1", "sh", "go", "java", "c",
                    "cpp", "h", "hpp", "cs",
                ];
                if text_exts.contains(&ext.as_str()) {
                    match std::fs::read_to_string(&entry.path) {
                        Ok(s) => self.preview_text = fs_ops::truncate_chars(&s, 4000),
                        Err(e) => self.preview_text = format!("読み込み失敗: {e}"),
                    }
                } else if ["png", "jpg", "jpeg", "gif", "bmp", "webp", "ico"].contains(&ext.as_str())
                {
                    self.preview_text = format!(
                        "画像ファイル: {}\nサイズ: {}\nパス: {}",
                        entry.name,
                        fs_ops::humansize(entry.size),
                        entry.path.display()
                    );
                } else {
                    self.preview_text = format!(
                        "ファイル: {}\nサイズ: {}\n種類: {}\n更新: {}\nパス: {}",
                        entry.name,
                        fs_ops::humansize(entry.size),
                        entry.ext,
                        fs_ops::fmt_time(entry.modified),
                        entry.path.display()
                    );
                }
            }
        } else {
            let cur = self.current_tab().current.clone();
            let free = fs_ops::free_space(&cur).unwrap_or_default();
            let n = self.current_tab().entries.len();
            let tip = if self.typeahead.is_empty() {
                "文字を入力でフィルタ".to_string()
            } else {
                format!("検索中: '{}'", self.typeahead)
            };
            self.preview_text = format!(
                "場所: {cur}\n\n項目数: {n}\n{free}\n\nヒント:\n• {tip}\n• Ctrl+L アドレス / Ctrl+F 検索\n• Home/End/PgUp/PgDn · Shift+矢印 範囲\n• Ctrl+I 反転 · Ctrl+Shift+C パスコピー\n• Del=ごみ箱 · Shift+Del=完全削除 · F10 2ペイン · Ctrl+H 隠し",
                cur = cur.display(),
                tip = tip
            );
        }
    }
    fn run_command(&mut self, cmd: Command) {
        match cmd {
            Command::Copy => self.do_clipboard(ClipboardMode::Copy),
            Command::Cut => self.do_clipboard(ClipboardMode::Cut),
            Command::Paste => self.do_paste(),
            Command::Delete { permanent } => self.ask_delete(permanent),
            Command::ConfirmDelete { permanent } => self.do_delete_confirmed(permanent),
            Command::CancelDialog => {
                self.confirm_delete = None;
                self.show_new_folder_dialog = false;
                self.renaming = false;
            }
            Command::Rename => {
                if let Some(e) = self.current_tab().primary_selected() {
                    self.rename_buffer = e.name.clone();
                    self.renaming = true;
                    self.rename_focus_once = true;
                } else {
                    self.status = "選択がありません".into();
                }
            }
            Command::NewFolder => {
                self.new_folder_name = "新しいフォルダ".into();
                self.show_new_folder_dialog = true;
                self.new_folder_focus_once = true;
            }
            Command::NewTextFile => self.do_new_text_file(),
            Command::SelectAll => {
                self.current_tab_mut().select_all();
                self.update_preview();
                self.status = format!("{} 件選択", self.current_tab().selected.len());
            }
            Command::InvertSelection => {
                self.current_tab_mut().invert_selection();
                self.update_preview();
                self.status = format!("選択反転: {} 件", self.current_tab().selected.len());
            }
            Command::Refresh => {
                self.request_refresh_async(false);
                if self.dual_pane {
                    self.request_refresh_async(true);
                }
                self.status = "更新中…".into();
            }
            Command::GoUp => {
                self.clear_typeahead();
                if self.current_tab_mut().go_up() {
                    self.sync_address();
                    self.note_navigation();
                    self.sync_watchers();
                    self.update_preview();
                }
            }
            Command::GoBack => {
                self.clear_typeahead();
                if self.current_tab_mut().go_back() {
                    self.sync_address();
                    self.note_navigation();
                    self.sync_watchers();
                    self.update_preview();
                }
            }
            Command::GoForward => {
                self.clear_typeahead();
                if self.current_tab_mut().go_forward() {
                    self.sync_address();
                    self.note_navigation();
                    self.sync_watchers();
                    self.update_preview();
                }
            }
            Command::GoHome => {
                if let Some(home) = dirs::home_dir() {
                    self.clear_typeahead();
                    self.current_tab_mut().navigate_to(home);
                    self.sync_address();
                    self.note_navigation();
                    self.sync_watchers();
                    self.update_preview();
                    self.status = "ホームへ移動".into();
                }
            }
            Command::OpenPrimary => {
                if let Some(p) = self.current_tab_mut().enter_primary() {
                    let _ = shell::open_with_shell(&p);
                } else {
                    self.clear_typeahead();
                    self.sync_address();
                    self.note_navigation();
                    self.sync_watchers();
                    self.update_preview();
                }
            }
            Command::AddBookmark => self.add_bookmark(),
            Command::Search => self.do_search(),
            Command::CopyPath => self.copy_selected_paths(false),
            Command::CopyName => self.copy_selected_paths(true),
            Command::ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                for t in &mut self.tabs {
                    t.show_hidden = self.show_hidden;
                }
                if let Some(p) = self.pane2.as_mut() {
                    p.show_hidden = self.show_hidden;
                }
                self.request_refresh_async(false);
                if self.dual_pane {
                    self.request_refresh_async(true);
                }
                self.status = if self.show_hidden {
                    "隠しファイルを表示".into()
                } else {
                    "隠しファイルを非表示".into()
                };
            }
            Command::TogglePreview => {
                self.config.show_preview = !self.config.show_preview;
            }
            Command::ToggleDualPane => {
                let on = !self.dual_pane;
                self.set_dual_pane(on);
                self.status = if on {
                    "2ペイン ON".into()
                } else {
                    "2ペイン OFF".into()
                };
            }
            Command::FocusAddress => self.focus_request = Some(FocusTarget::Address),
            Command::FocusSearch => self.focus_request = Some(FocusTarget::Search),
            Command::FocusFilter => self.focus_request = Some(FocusTarget::Filter),
        }
    }
    fn copy_selected_paths(&mut self, names_only: bool) {
        let paths = self.current_tab().selected_paths();
        let s = if paths.is_empty() {
            if names_only {
                self.current_tab()
                    .current
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| self.current_tab().current.display().to_string())
            } else {
                self.current_tab().current.display().to_string()
            }
        } else {
            paths
                .iter()
                .map(|p| {
                    if names_only {
                        p.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.display().to_string())
                    } else {
                        p.display().to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let n = if paths.is_empty() { 1 } else { paths.len() };
        self.status = if names_only {
            format!("名前をコピー: {n} 件")
        } else {
            format!("パスをコピー: {n} 件")
        };
        self.pending_clipboard_text = Some(s);
    }
    fn do_clipboard(&mut self, mode: ClipboardMode) {
        let paths = self.current_tab().selected_paths();
        if paths.is_empty() {
            self.status = "選択がありません".into();
            return;
        }
        let n = paths.len();
        self.clipboard = Some(Clipboard::new(paths, mode));
        let m = if mode == ClipboardMode::Copy {
            "コピー"
        } else {
            "切り取り"
        };
        self.status = format!("{m}しました: {n} 件");
    }
    fn do_paste(&mut self) {
        let Some(cb) = self.clipboard.clone() else {
            self.status = "クリップボードが空です".into();
            return;
        };
        if self.pasting {
            self.status = "貼り付け処理中です…".into();
            return;
        }
        let dest = self.current_tab().current.clone();
        self.pasting = true;
        self.status = format!("貼り付け中… ({} 件)", cb.paths.len());
        commands::spawn_paste(self.bg_tx.clone(), cb.paths, cb.mode, dest);
        if matches!(cb.mode, ClipboardMode::Cut) {
            self.clipboard = None;
        }
    }
    fn ask_delete(&mut self, permanent: bool) {
        let paths = self.current_tab().selected_paths();
        if paths.is_empty() {
            self.status = "選択がありません".into();
            return;
        }
        self.confirm_delete = Some(ConfirmDelete { paths, permanent });
    }
    fn do_delete_confirmed(&mut self, permanent: bool) {
        let Some(conf) = self.confirm_delete.take() else {
            return;
        };
        let permanent = permanent || conf.permanent;
        let res = if permanent {
            fs_ops::permanent_delete(&conf.paths)
        } else {
            fs_ops::trash_paths(&conf.paths)
        };
        match res {
            Ok(()) => {
                let n = conf.paths.len();
                self.status = if permanent {
                    format!("完全に削除: {n} 件")
                } else {
                    format!("ごみ箱へ移動: {n} 件")
                };
                self.request_refresh_async(false);
                if self.dual_pane {
                    self.request_refresh_async(true);
                }
            }
            Err(e) => {
                self.last_error = Some(e.to_string());
                self.status = format!("削除失敗: {e}");
            }
        }
    }
    fn do_rename(&mut self) {
        let Some(entry) = self.current_tab().primary_selected().cloned() else {
            self.renaming = false;
            return;
        };
        let Some(new_name) = fs_ops::sanitize_filename(&self.rename_buffer) else {
            self.status = "不正な名前です（\\/:*?\"<>| や予約名は使えません）".into();
            return;
        };
        if new_name == entry.name {
            self.renaming = false;
            return;
        }
        let parent = entry.path.parent().unwrap_or(Path::new("."));
        let new_path = parent.join(&new_name);
        if new_path.exists() {
            self.status = "同名のファイル/フォルダが既に存在します".into();
            return;
        }
        match std::fs::rename(&entry.path, &new_path) {
            Ok(()) => {
                self.status = format!("名前変更: {} → {new_name}", entry.name);
                self.renaming = false;
                self.request_refresh_async(false);
                if self.dual_pane {
                    self.request_refresh_async(true);
                }
            }
            Err(e) => self.status = format!("名前変更失敗: {e}"),
        }
    }
    fn do_new_folder(&mut self) {
        let Some(name) = fs_ops::sanitize_filename(&self.new_folder_name) else {
            self.status = "不正なフォルダ名です".into();
            return;
        };
        let p = self.current_tab().current.join(&name);
        if p.exists() {
            self.status = "同名のフォルダが既に存在します".into();
            return;
        }
        match std::fs::create_dir(&p) {
            Ok(()) => {
                self.status = format!("作成しました: {}", p.display());
                self.show_new_folder_dialog = false;
                self.request_refresh_async(false);
            }
            Err(e) => self.status = format!("作成失敗: {e}"),
        }
    }
    fn do_new_text_file(&mut self) {
        let dir = self.current_tab().current.clone();
        let mut p = dir.join("新しいテキスト ドキュメント.txt");
        let mut c = 1;
        while p.exists() {
            p = dir.join(format!("新しいテキスト ドキュメント ({c}).txt"));
            c += 1;
            if c > 999 {
                break;
            }
        }
        match std::fs::write(&p, "") {
            Ok(()) => {
                self.status = format!("作成: {}", p.display());
                self.request_refresh_async(false);
            }
            Err(e) => self.status = format!("作成失敗: {e}"),
        }
    }
    fn do_search(&mut self) {
        let q = self.search_query.trim().to_string();
        if q.is_empty() {
            self.search_results.clear();
            return;
        }
        if self.searching {
            return;
        }
        self.searching = true;
        self.status = format!("検索中: {q} …");
        let root = self.current_tab().current.clone();
        commands::spawn_search(self.bg_tx.clone(), root, q);
    }
    fn add_bookmark(&mut self) {
        let cur = self.current_tab().current.clone();
        let name = cur
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("ブックマーク")
            .to_string();
        if self.config.bookmarks.iter().any(|b| b.path == cur) {
            self.status = "既にブックマーク済みです".into();
            return;
        }
        self.config.bookmarks.push(Bookmark {
            name: name.clone(),
            path: cur,
        });
        match self.config.save() {
            Ok(()) => self.status = format!("ブックマーク追加: {name}"),
            Err(e) => self.status = e,
        }
    }
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.renaming || self.show_new_folder_dialog || self.confirm_delete.is_some() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.run_command(Command::CancelDialog);
            }
            if self.renaming
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            {
                self.do_rename();
            }
            if self.show_new_folder_dialog
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            {
                self.do_new_folder();
            }
            return;
        }
        // Delete / Shift+Delete: handle before typing guard so permanent delete works even when filter focused
        {
            let del_plain = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
            let del_shift = ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Delete));
            if del_plain || del_shift {
                let permanent = del_shift || ctx.input(|i| i.modifiers.shift);
                let typing_now = ctx.wants_keyboard_input();
                if !typing_now || permanent {
                    self.run_command(Command::Delete { permanent });
                }
                // if plain Delete while typing, let TextEdit handle it (don't trigger file delete)
            }
        }
        let typing = ctx.wants_keyboard_input();

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::L)) {
            self.run_command(Command::FocusAddress);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.run_command(Command::FocusSearch);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::T)) {
            let cur = self.current_tab().current.clone();
            let sh = self.show_hidden;
            let sb = self.current_tab().sort_by;
            let sd = self.current_tab().sort_desc;
            self.tabs.push(Tab::new(cur, sh, sb, sd));
            self.active = self.tabs.len() - 1;
            self.active_pane = 0;
            self.sync_address();
            self.sync_watchers();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::W))
            && self.tabs.len() > 1
        {
            self.tabs.remove(self.active);
            if self.active >= self.tabs.len() {
                self.active = self.tabs.len() - 1;
            }
            self.active_pane = 0;
            self.sync_address();
            self.sync_watchers();
            self.update_preview();
        }
        if typing {
            return;
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
            self.run_command(Command::SelectAll);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::I)) {
            self.run_command(Command::InvertSelection);
        }
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::C)
        }) {
            self.run_command(Command::CopyPath);
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::C)) {
            self.run_command(Command::Copy);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::X)) {
            self.run_command(Command::Cut);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::V)) {
            self.run_command(Command::Paste);
        }
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::N)
        }) {
            self.run_command(Command::NewFolder);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
            self.run_command(Command::Rename);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F5)) {
            self.run_command(Command::Refresh);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F10)) {
            self.run_command(Command::ToggleDualPane);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::H)) {
            self.run_command(Command::ToggleHidden);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::P)) {
            self.run_command(Command::TogglePreview);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowUp)) {
            self.run_command(Command::GoUp);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft)) {
            self.run_command(Command::GoBack);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowRight)) {
            self.run_command(Command::GoForward);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::Home)) {
            self.run_command(Command::GoHome);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            self.run_command(Command::OpenPrimary);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if !self.typeahead.is_empty() {
                self.clear_typeahead();
                self.status = "検索クリア".into();
            } else if !self.current_tab().selected.is_empty() {
                self.current_tab_mut().clear_selection();
                self.update_preview();
            } else {
                self.search_results.clear();
                self.search_query.clear();
                self.update_preview();
            }
        }

        let shift = ctx.input(|i| i.modifiers.shift);
        let mut moved = false;
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowDown))
        {
            self.current_tab_mut().move_focus_by(1, shift);
            moved = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp))
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowUp))
        {
            self.current_tab_mut().move_focus_by(-1, shift);
            moved = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::PageDown))
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::PageDown))
        {
            self.current_tab_mut().move_focus_by(20, shift);
            moved = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::PageUp))
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::PageUp))
        {
            self.current_tab_mut().move_focus_by(-20, shift);
            moved = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home))
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Home))
        {
            self.current_tab_mut().focus_first(shift);
            moved = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::End))
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::End))
        {
            self.current_tab_mut().focus_last(shift);
            moved = true;
        }
        if moved {
            self.scroll_to_row = self.current_tab().focus;
            self.update_preview();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace))
            && self.typeahead.is_empty()
        {
            self.run_command(Command::GoUp);
        }
    }
    fn set_dual_pane(&mut self, on: bool) {
        self.dual_pane = on;
        if on {
            if self.pane2.is_none() {
                let cur = self.current_tab().current.clone();
                let sh = self.show_hidden;
                let sb = self.current_tab().sort_by;
                let sd = self.current_tab().sort_desc;
                self.pane2 = Some(Tab::new(cur, sh, sb, sd));
            }
        } else {
            self.pane2 = None;
            self.active_pane = 0;
        }
        self.sync_watchers();
    }
    fn ui_display_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.collapsing("表示", |ui| {
            ui.checkbox(&mut self.config.show_preview, "プレビュー");
            let mut dark = self.config.theme_dark;
            if ui.checkbox(&mut dark, "ダークテーマ").changed() {
                self.config.theme_dark = dark;
                if dark {
                    ctx.set_visuals(egui::Visuals::dark());
                } else {
                    ctx.set_visuals(egui::Visuals::light());
                }
            }
            ui.checkbox(&mut self.config.compact_ui, "コンパクトUI");
            ui.horizontal(|ui| {
                ui.label("行の高さ");
                let mut scale = self.config.row_height_scale;
                if ui
                    .add(egui::Slider::new(&mut scale, 0.85..=1.35).fixed_decimals(2))
                    .changed()
                {
                    self.config.row_height_scale = scale;
                }
            });
            ui.separator();
            ui.strong("フォント（日本語）");
            egui::ComboBox::from_id_source("font_preset")
                .selected_text(fonts::preset_label(self.config.font_preset))
                .width(220.0)
                .show_ui(ui, |ui| {
                    for p in fonts::all_presets() {
                        ui.selectable_value(
                            &mut self.config.font_preset,
                            *p,
                            fonts::preset_label(*p),
                        );
                    }
                });
            ui.horizontal(|ui| {
                ui.label("サイズ");
                let mut size = self.config.font_size;
                if ui
                    .add(egui::Slider::new(&mut size, 10.0..=24.0).suffix(" pt"))
                    .changed()
                {
                    self.config.font_size = size;
                    fonts::apply_text_styles(ctx, size);
                }
            });
            if self.config.font_preset == FontPreset::Custom
                || !self.font_path_edit.trim().is_empty()
            {
                ui.label("カスタムフォント (.ttf / .otf / .ttc)");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.font_path_edit)
                            .desired_width(160.0)
                            .hint_text("C:\\Windows\\Fonts\\..."),
                    );
                    if ui.small_button("参照…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("フォント", &["ttf", "otf", "ttc", "TTF", "OTF", "TTC"])
                            .set_title("日本語フォントを選択")
                            .pick_file()
                        {
                            self.font_path_edit = path.display().to_string();
                            self.config.font_preset = FontPreset::Custom;
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.button("フォントを適用").clicked() {
                    self.apply_font_settings(ctx);
                }
                if ui.button("設定を保存").clicked() {
                    self.sync_config_from_state();
                    match self.config.save() {
                        Ok(()) => self.status = "設定を保存しました".into(),
                        Err(e) => self.status = e,
                    }
                }
            });
        });
    }
    fn navigate_address_bar(&mut self) {
        let p = shell::normalize_path_input(self.address.trim());
        if p.as_os_str().is_empty() {
            self.status = "パスを入力してください".into();
            return;
        }
        if p.exists() {
            if p.is_dir() {
                self.clear_typeahead();
                self.current_tab_mut().navigate_to(p);
                self.sync_address();
                self.note_navigation();
                self.sync_watchers();
                self.update_preview();
            } else {
                match shell::open_with_shell(&p) {
                    Ok(_) => self.status = format!("開きました: {}", p.display()),
                    Err(e) => self.status = format!("開けませんでした: {e}"),
                }
            }
        } else {
            self.status = format!("見つかりません: {}", self.address);
        }
    }
}

impl eframe::App for ExplorerApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_config_from_state();
        if let Err(e) = self.config.save() {
            eprintln!("config save failed: {e}");
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_bg(ctx);
        self.sync_watchers();

        if let Some(text) = self.pending_clipboard_text.take() {
            ctx.output_mut(|o| o.copied_text = text);
        }

        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = "ドロップでコピー:\n".to_owned();
                for f in &i.raw.hovered_files {
                    if let Some(path) = &f.path {
                        text.push_str(&format!("\n{}", path.display()));
                    } else {
                        text.push_str("\n???");
                    }
                }
                text
            });
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("file_drop_target"),
            ));
            let rect = ctx.screen_rect();
            painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(160));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(18.0),
                egui::Color32::WHITE,
            );
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            let dest_dir = self.current_tab().current.clone();
            let mut paths = Vec::new();
            for f in dropped {
                if let Some(path) = f.path {
                    paths.push(path);
                }
            }
            if !paths.is_empty() {
                self.pasting = true;
                self.status = format!("ドロップをコピー中… ({} 件)", paths.len());
                commands::spawn_paste(self.bg_tx.clone(), paths, ClipboardMode::Copy, dest_dir);
            }
        }
        self.handle_typeahead(ctx);
        self.handle_shortcuts(ctx);

        // Delete confirm dialog
        if let Some(conf) = self.confirm_delete.clone() {
            let permanent = conf.permanent;
            let n = conf.paths.len();
            let names: Vec<String> = conf
                .paths
                .iter()
                .take(8)
                .map(|p| {
                    p.file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| p.display().to_string())
                })
                .collect();
            let mut open = true;
            let mut confirmed = false;
            let mut cancelled = false;
            egui::Window::new(if permanent {
                "完全に削除しますか？"
            } else {
                "ごみ箱へ移動しますか？"
            })
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                if permanent {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 80, 80),
                        "この操作は取り消せません。",
                    );
                }
                ui.label(format!("{n} 件の項目:"));
                for name in &names {
                    ui.label(format!("  • {name}"));
                }
                if n > names.len() {
                    ui.label(format!("  … 他 {} 件", n - names.len()));
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button(if permanent {
                            "完全に削除"
                        } else {
                            "ごみ箱へ"
                        })
                        .clicked()
                    {
                        confirmed = true;
                    }
                    if ui.button("キャンセル").clicked() {
                        cancelled = true;
                    }
                });
            });
            if confirmed {
                self.run_command(Command::ConfirmDelete { permanent });
            } else if cancelled || !open {
                self.confirm_delete = None;
            }
        }

        let compact = self.config.compact_ui;
        let btn = |label: &str| {
            if compact {
                egui::RichText::new(label).small()
            } else {
                egui::RichText::new(label)
            }
        };

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            if compact {
                ui.spacing_mut().item_spacing.y = 2.0;
            }
            ui.horizontal_wrapped(|ui| {
                let back_ok = !self.current_tab().history_back.is_empty();
                let fwd_ok = !self.current_tab().history_forward.is_empty();
                ui.add_enabled_ui(back_ok, |ui| {
                    if ui.button(btn("◀")).on_hover_text("戻る").clicked() {
                        self.run_command(Command::GoBack);
                    }
                });
                ui.add_enabled_ui(fwd_ok, |ui| {
                    if ui.button(btn("▶")).on_hover_text("進む").clicked() {
                        self.run_command(Command::GoForward);
                    }
                });
                if ui.button(btn("⬆")).on_hover_text("上へ Alt+↑").clicked() {
                    self.run_command(Command::GoUp);
                }
                if ui.button(btn("⟳")).on_hover_text("更新 F5").clicked() {
                    self.run_command(Command::Refresh);
                }
                if ui.button(btn("🏠")).on_hover_text("ホーム Alt+Home").clicked() {
                    self.run_command(Command::GoHome);
                }
                ui.separator();
                if ui.button(btn("＋タブ")).on_hover_text("Ctrl+T").clicked() {
                    let cur = self.current_tab().current.clone();
                    let sh = self.show_hidden;
                    let sb = self.current_tab().sort_by;
                    let sd = self.current_tab().sort_desc;
                    self.tabs.push(Tab::new(cur, sh, sb, sd));
                    self.active = self.tabs.len() - 1;
                    self.active_pane = 0;
                    self.sync_address();
                    self.sync_watchers();
                }
                ui.separator();
                ui.label("アドレス:");
                let addr_edit = egui::TextEdit::singleline(&mut self.address)
                    .desired_width(if compact { 320.0 } else { 420.0 })
                    .hint_text("パス… (\\ と / 可)  Ctrl+L");
                let resp = ui.add(addr_edit);
                if matches!(self.focus_request, Some(FocusTarget::Address)) {
                    resp.request_focus();
                    self.focus_request = None;
                }
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let should_navigate = (resp.has_focus() && enter_pressed)
                    || ui.small_button("移動").clicked();
                if should_navigate {
                    self.navigate_address_bar();
                }
                ui.separator();
                ui.label("🔍");
                let sresp = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .desired_width(140.0)
                        .hint_text("検索 Ctrl+F"),
                );
                if matches!(self.focus_request, Some(FocusTarget::Search)) {
                    sresp.request_focus();
                    self.focus_request = None;
                }
                let search_enter = sresp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if search_enter || ui.small_button("検索").clicked() {
                    self.run_command(Command::Search);
                }
                if !self.search_query.is_empty() && ui.small_button("×").clicked() {
                    self.search_query.clear();
                    self.search_results.clear();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut dual = self.dual_pane;
                    if ui.checkbox(&mut dual, "2ペイン").on_hover_text("F10").changed() {
                        self.set_dual_pane(dual);
                    }
                    let mut hidden = self.show_hidden;
                    if ui
                        .checkbox(&mut hidden, "隠し")
                        .on_hover_text("Ctrl+H")
                        .changed()
                    {
                        self.show_hidden = hidden;
                        for t in &mut self.tabs {
                            t.show_hidden = hidden;
                        }
                        if let Some(p) = self.pane2.as_mut() {
                            p.show_hidden = hidden;
                        }
                        self.request_refresh_async(false);
                        if self.dual_pane {
                            self.request_refresh_async(true);
                        }
                    }
                    if !self.typeahead.is_empty() {
                        ui.colored_label(
                            egui::Color32::from_rgb(80, 160, 255),
                            format!("⌕ '{}'", self.typeahead),
                        );
                    }
                });
            });
            // Tabs
            let mut switch_to: Option<usize> = None;
            let mut close_idx: Option<usize> = None;
            let mut tab_action: Option<(usize, &'static str)> = None;
            ui.horizontal_wrapped(|ui| {
                for (i, tab) in self.tabs.iter().enumerate() {
                    let is_active = i == self.active && self.active_pane == 0;
                    let name = tab
                        .current
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or_else(|| tab.current.to_str().unwrap_or("/"));
                    let label = format!("{} {name}", if is_active { "●" } else { "○" });
                    let resp = ui.selectable_label(is_active, label);
                    if resp.clicked() {
                        switch_to = Some(i);
                    }
                    if ui.small_button("×").on_hover_text("Ctrl+W").clicked() && self.tabs.len() > 1
                    {
                        close_idx = Some(i);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("このタブを複製").clicked() {
                            tab_action = Some((i, "dup"));
                            ui.close_menu();
                        }
                        if ui.button("他のタブを全て閉じる").clicked() {
                            tab_action = Some((i, "close_others"));
                            ui.close_menu();
                        }
                    });
                }
            });
            if let Some(i) = switch_to {
                self.clear_typeahead();
                self.active = i;
                self.active_pane = 0;
                self.sync_address();
                self.sync_watchers();
                self.update_preview();
            }
            if let Some(i) = close_idx {
                self.tabs.remove(i);
                if self.active >= self.tabs.len() {
                    self.active = self.tabs.len() - 1;
                }
                self.active_pane = 0;
                self.sync_address();
                self.sync_watchers();
                self.update_preview();
            }
            if let Some((i, act)) = tab_action {
                match act {
                    "dup" => {
                        let t = &self.tabs[i];
                        let nt = Tab::new(t.current.clone(), t.show_hidden, t.sort_by, t.sort_desc);
                        self.tabs.push(nt);
                        self.sync_watchers();
                    }
                    "close_others" => {
                        let t = self.tabs[i].clone();
                        self.tabs = vec![Tab::new(t.current, t.show_hidden, t.sort_by, t.sort_desc)];
                        self.active = 0;
                        self.active_pane = 0;
                        self.sync_address();
                        self.sync_watchers();
                        self.update_preview();
                    }
                    _ => {}
                }
            }
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.searching || self.pasting || self.listing || self.listing_p2 {
                    ui.spinner();
                }
                ui.label(&self.status);
                if !self.typeahead.is_empty() {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(80, 160, 255),
                        format!("type: '{}'", self.typeahead),
                    );
                }
                if let Some(e) = &self.last_error {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), e);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tab = self.current_tab();
                    let sel = tab.selected.len();
                    if sel > 0 {
                        let size: u64 = tab
                            .selected
                            .iter()
                            .filter_map(|&i| tab.entries.get(i))
                            .filter(|e| !e.is_dir)
                            .map(|e| e.size)
                            .sum();
                        if size > 0 {
                            ui.label(format!("選択: {sel} ({})", fs_ops::humansize(size)));
                        } else {
                            ui.label(format!("選択: {sel}"));
                        }
                    }
                    ui.label(format!("{} 項目", tab.entries.len()));
                    if let Some(err) = &tab.error {
                        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
                    }
                    if let Some(cb) = &self.clipboard {
                        let mode = match cb.mode {
                            ClipboardMode::Copy => "コピー",
                            ClipboardMode::Cut => "切り取り",
                        };
                        ui.label(format!("CB:{n} {mode}", n = cb.paths.len()));
                    }
                    if self.listing || self.listing_p2 {
                        ui.label("読み込み中…");
                    }
                });
            });
        });

        egui::SidePanel::left("left")
            .resizable(true)
            .default_width(if compact { 220.0 } else { 250.0 })
            .width_range(180.0..=400.0)
            .show(ctx, |ui| {
                ui.heading("クイックアクセス");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.collapsing("ドライブ", |ui| {
                        #[cfg(windows)]
                        {
                            for letter in 'A'..='Z' {
                                let d = format!("{letter}:\\");
                                let p = PathBuf::from(&d);
                                if p.exists() {
                                    let is_cur = self.current_tab().current == p;
                                    if ui.selectable_label(is_cur, format!("💾 {d}")).clicked() {
                                        self.clear_typeahead();
                                        self.current_tab_mut().navigate_to(p);
                                        self.sync_address();
                                        self.note_navigation();
                                        self.sync_watchers();
                                        self.update_preview();
                                    }
                                }
                            }
                        }
                        #[cfg(not(windows))]
                        {
                            for d in ["/", "/home", "/tmp"] {
                                let p = PathBuf::from(d);
                                if p.exists() {
                                    let is_cur = self.current_tab().current == p;
                                    if ui.selectable_label(is_cur, d).clicked() {
                                        self.clear_typeahead();
                                        self.current_tab_mut().navigate_to(p);
                                        self.sync_address();
                                        self.note_navigation();
                                        self.sync_watchers();
                                        self.update_preview();
                                    }
                                }
                            }
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.strong("ブックマーク");
                        if ui.small_button("＋").on_hover_text("現在地を追加").clicked() {
                            self.run_command(Command::AddBookmark);
                        }
                    });
                    let mut nav: Option<PathBuf> = None;
                    let mut rm: Option<usize> = None;
                    for (i, bm) in self.config.bookmarks.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let is_cur = self.current_tab().current == bm.path;
                            if ui
                                .selectable_label(is_cur, format!("⭐ {}", bm.name))
                                .clicked()
                            {
                                nav = Some(bm.path.clone());
                            }
                            if ui.small_button("×").clicked() {
                                rm = Some(i);
                            }
                        });
                    }
                    if let Some(i) = rm {
                        self.config.bookmarks.remove(i);
                        let _ = self.config.save();
                        self.status = "ブックマークを削除しました".into();
                    }
                    if let Some(p) = nav {
                        if p.exists() {
                            self.clear_typeahead();
                            self.current_tab_mut().navigate_to(p);
                            self.sync_address();
                            self.note_navigation();
                            self.sync_watchers();
                            self.update_preview();
                        } else {
                            self.status = "ブックマークのパスが見つかりません".into();
                        }
                    }
                    // Recent paths
                    if !self.config.recent_paths.is_empty() {
                        ui.separator();
                        ui.collapsing("最近", |ui| {
                            let mut rnav: Option<PathBuf> = None;
                            for p in self.config.recent_paths.iter().take(12) {
                                let name = p
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or_else(|| p.to_str().unwrap_or("?"));
                                let is_cur = self.current_tab().current == *p;
                                if ui
                                    .selectable_label(is_cur, format!("🕒 {name}"))
                                    .on_hover_text(p.display().to_string())
                                    .clicked()
                                {
                                    rnav = Some(p.clone());
                                }
                            }
                            if let Some(p) = rnav {
                                if p.exists() {
                                    self.clear_typeahead();
                                    self.current_tab_mut().navigate_to(p);
                                    self.sync_address();
                                    self.note_navigation();
                                    self.sync_watchers();
                                    self.update_preview();
                                }
                            }
                        });
                    }
                    ui.separator();
                    ui.strong("階層");
                    let cur = self.current_tab().current.clone();
                    let ancestors: Vec<PathBuf> =
                        cur.ancestors().map(|p| p.to_path_buf()).collect();
                    for anc in ancestors.iter().rev() {
                        let name = anc
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or_else(|| anc.to_str().unwrap_or("/"));
                        let is_cur = *anc == cur;
                        if ui.selectable_label(is_cur, format!("📁 {name}")).clicked() {
                            self.clear_typeahead();
                            self.current_tab_mut().navigate_to(anc.clone());
                            self.sync_address();
                            self.note_navigation();
                            self.sync_watchers();
                            self.update_preview();
                        }
                    }
                    ui.separator();
                    ui.collapsing("操作", |ui| {
                        if ui.button("📁 新しいフォルダ").clicked() {
                            self.run_command(Command::NewFolder);
                        }
                        if ui.button("📄 新しいテキスト").clicked() {
                            self.run_command(Command::NewTextFile);
                        }
                        ui.separator();
                        let has = !self.current_tab().selected.is_empty();
                        ui.add_enabled_ui(has, |ui| {
                            if ui.button("F2 名前変更").clicked() {
                                self.run_command(Command::Rename);
                            }
                            if ui.button("コピー").clicked() {
                                self.run_command(Command::Copy);
                            }
                            if ui.button("切り取り").clicked() {
                                self.run_command(Command::Cut);
                            }
                            if ui.button("パスをコピー").clicked() {
                                self.run_command(Command::CopyPath);
                            }
                            if ui.button("削除 (ごみ箱)").clicked() {
                                self.run_command(Command::Delete { permanent: false });
                            }
                            if ui.button("完全削除").clicked() {
                                self.run_command(Command::Delete { permanent: true });
                            }
                        });
                        ui.add_enabled_ui(self.clipboard.is_some() && !self.pasting, |ui| {
                            if ui.button("貼り付け").clicked() {
                                self.run_command(Command::Paste);
                            }
                        });
                    });
                    ui.separator();
                    self.ui_display_settings(ui, ctx);
                });
            });

        if self.config.show_preview {
            egui::SidePanel::right("preview")
                .resizable(true)
                .default_width(300.0)
                .width_range(200.0..=480.0)
                .show(ctx, |ui| {
                    ui.heading("プレビュー / 詳細");
                    ui.separator();
                    if self.renaming {
                        ui.group(|ui| {
                            ui.strong("名前変更 (Enter確定 / Esc取消)");
                            let r = ui.text_edit_singleline(&mut self.rename_buffer);
                            if self.rename_focus_once {
                                r.request_focus();
                                self.rename_focus_once = false;
                            }
                            ui.horizontal(|ui| {
                                if ui.button("確定").clicked() {
                                    self.do_rename();
                                }
                                if ui.button("取消").clicked() {
                                    self.renaming = false;
                                }
                            });
                        });
                        ui.separator();
                    }
                    if self.show_new_folder_dialog {
                        ui.group(|ui| {
                            ui.strong("新しいフォルダ");
                            let r = ui.text_edit_singleline(&mut self.new_folder_name);
                            if self.new_folder_focus_once {
                                r.request_focus();
                                self.new_folder_focus_once = false;
                            }
                            ui.horizontal(|ui| {
                                if ui.button("作成").clicked() {
                                    self.do_new_folder();
                                }
                                if ui.button("取消").clicked() {
                                    self.show_new_folder_dialog = false;
                                }
                            });
                        });
                        ui.separator();
                    }
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(&self.preview_text);
                        if let Some(entry) = self.current_tab().primary_selected().cloned() {
                            ui.separator();
                            ui.strong("プロパティ");
                            ui.label(format!("名前: {}", entry.name));
                            ui.label(format!("パス: {}", entry.path.display()));
                            if let Some(t) = shell::os_type_name(&entry.path) {
                                ui.label(format!("種類(OS): {t}"));
                            } else {
                                ui.label(format!("種類: {}", entry.ext));
                            }
                            ui.label(format!(
                                "サイズ: {}",
                                if entry.is_dir {
                                    "-".into()
                                } else {
                                    fs_ops::humansize(entry.size)
                                }
                            ));
                            ui.label(format!("更新: {}", fs_ops::fmt_time(entry.modified)));
                            ui.horizontal(|ui| {
                                if ui.small_button("パスをコピー").clicked() {
                                    let s = entry.path.display().to_string();
                                    ui.output_mut(|o| o.copied_text = s.clone());
                                    self.status = format!("パスをコピー: {s}");
                                }
                                if ui.small_button("Explorerで表示").clicked() {
                                    let _ = shell::reveal_in_explorer(&entry.path);
                                }
                                if ui.small_button("開く").clicked() {
                                    self.open_path(&entry.path);
                                }
                            });
                            let ext = entry.ext.to_lowercase();
                            if ["png", "jpg", "jpeg", "gif", "bmp", "webp"].contains(&ext.as_str())
                            {
                                ui.separator();
                                ui.label("画像プレビュー:");
                                let uri = format!("file://{}", entry.path.display());
                                ui.add(
                                    egui::Image::from_uri(uri).max_size(egui::vec2(280.0, 280.0)),
                                );
                            }
                        }
                        if !self.search_results.is_empty() {
                            ui.separator();
                            ui.heading(format!("検索結果 {}件", self.search_results.len()));
                            let results = self.search_results.clone();
                            for r in results {
                                let icon = shell::icon_emoji_for_path(&r.path, r.is_dir);
                                if ui
                                    .selectable_label(
                                        false,
                                        format!("{icon} {} — {}", r.name, r.path.display()),
                                    )
                                    .clicked()
                                {
                                    self.open_path(&r.path);
                                }
                            }
                            if ui.small_button("結果をクリア").clicked() {
                                self.search_results.clear();
                                self.search_query.clear();
                            }
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                let cur = self.current_tab().current.clone();
                let mut acc = PathBuf::new();
                let comps: Vec<_> = cur.components().collect();
                for (i, comp) in comps.iter().enumerate() {
                    acc.push(comp.as_os_str());
                    let is_last = i + 1 == comps.len();
                    let name = comp.as_os_str().to_string_lossy().to_string();
                    if is_last {
                        ui.strong(name);
                    } else if ui.small_button(&name).clicked() {
                        self.clear_typeahead();
                        self.current_tab_mut().navigate_to(acc.clone());
                        self.sync_address();
                        self.note_navigation();
                        self.sync_watchers();
                        self.update_preview();
                    }
                    if !is_last {
                        ui.label("›");
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("クリア").clicked() {
                        self.current_tab_mut().filter.clear();
                        self.request_refresh_async(false);
                    }
                    let mut f = self.current_tab().filter.clone();
                    let fresp = ui.add(
                        egui::TextEdit::singleline(&mut f)
                            .desired_width(120.0)
                            .hint_text("フィルタ"),
                    );
                    if matches!(self.focus_request, Some(FocusTarget::Filter)) {
                        fresp.request_focus();
                        self.focus_request = None;
                    }
                    if fresp.changed() {
                        self.current_tab_mut().filter = f;
                        self.request_refresh_async(false);
                        self.update_preview();
                    }
                    ui.label("フィルタ:");
                });
            });
            ui.separator();
            if self.dual_pane {
                if self.pane2.is_none() {
                    self.set_dual_pane(true);
                }
                ui.columns(2, |cols| {
                    {
                        let entries = self.tabs[self.active].entries.clone();
                        let selected = self.tabs[self.active].selected.clone();
                        let focus = self.tabs[self.active].focus;
                        let title = self.tabs[self.active].current.display().to_string();
                        let is_act = self.active_pane == 0;
                        cols[0].push_id("pane0", |ui| {
                            ui.horizontal(|ui| {
                                ui.strong(format!(
                                    "{}ペイン1: {title}",
                                    if is_act { "● " } else { "" }
                                ));
                                if ui.small_button("アクティブ").clicked() {
                                    self.active_pane = 0;
                                    self.sync_address();
                                }
                            });
                            self.draw_table(ui, PaneId::Tab, &entries, &selected, focus);
                        });
                    }
                    if let Some(p2) = &self.pane2 {
                        let entries = p2.entries.clone();
                        let selected = p2.selected.clone();
                        let focus = p2.focus;
                        let title = p2.current.display().to_string();
                        let is_act = self.active_pane == 1;
                        cols[1].push_id("pane1", |ui| {
                            ui.horizontal(|ui| {
                                ui.strong(format!(
                                    "{}ペイン2: {title}",
                                    if is_act { "● " } else { "" }
                                ));
                                if ui.small_button("アクティブ").clicked() {
                                    self.active_pane = 1;
                                    self.sync_address();
                                }
                            });
                            self.draw_table(ui, PaneId::Pane2, &entries, &selected, focus);
                        });
                    }
                });
            } else {
                let entries = self.tabs[self.active].entries.clone();
                let selected = self.tabs[self.active].selected.clone();
                let focus = self.tabs[self.active].focus;
                ui.push_id("pane_single", |ui| {
                    self.draw_table(ui, PaneId::Tab, &entries, &selected, focus);
                });
            }
        });
    }
}

#[derive(Clone, Copy)]
enum PaneId {
    Tab,
    Pane2,
}

impl ExplorerApp {
    fn tab_for_mut(&mut self, pane: PaneId) -> &mut Tab {
        match pane {
            PaneId::Tab => &mut self.tabs[self.active],
            PaneId::Pane2 => self.pane2.as_mut().expect("pane2"),
        }
    }
    fn draw_table(
        &mut self,
        ui: &mut egui::Ui,
        pane: PaneId,
        entries: &[FileEntry],
        selected: &BTreeSet<usize>,
        focus: Option<usize>,
    ) {
        let mut sort_clicked: Option<SortBy> = None;
        let mut open_path: Option<PathBuf> = None;
        let mut select_action: Option<SelectAction> = None;
        let mut cmd: Option<Command> = None;
        let (sort_by, sort_desc) = {
            let t = match pane {
                PaneId::Tab => &self.tabs[self.active],
                PaneId::Pane2 => self.pane2.as_ref().unwrap(),
            };
            (t.sort_by, t.sort_desc)
        };
        let row_h = 22.0 * self.config.row_height_scale;
        let num_rows = entries.len().max(1);
        let scroll_to = self.scroll_to_row.take();
        let egui_ctx = ui.ctx().clone();

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::auto().at_least(280.0).resizable(true))
            .column(egui_extras::Column::auto().at_least(90.0))
            .column(egui_extras::Column::auto().at_least(140.0))
            .column(egui_extras::Column::remainder().at_least(70.0))
            .auto_shrink([false, false])
            .vscroll(true)
            .header(24.0, |mut header| {
                header.col(|ui| {
                    let label = header_label("名前", sort_by == SortBy::Name, sort_desc);
                    if ui.selectable_label(false, label).clicked() {
                        sort_clicked = Some(SortBy::Name);
                    }
                });
                header.col(|ui| {
                    let label = header_label("サイズ", sort_by == SortBy::Size, sort_desc);
                    if ui.selectable_label(false, label).clicked() {
                        sort_clicked = Some(SortBy::Size);
                    }
                });
                header.col(|ui| {
                    let label = header_label("更新日時", sort_by == SortBy::Modified, sort_desc);
                    if ui.selectable_label(false, label).clicked() {
                        sort_clicked = Some(SortBy::Modified);
                    }
                });
                header.col(|ui| {
                    let label = header_label("種類", sort_by == SortBy::Type, sort_desc);
                    if ui.selectable_label(false, label).clicked() {
                        sort_clicked = Some(SortBy::Type);
                    }
                });
            })
            .body(|body| {
                if entries.is_empty() {
                    body.rows(row_h, 1, |mut row| {
                        row.col(|ui| {
                            ui.label("（空のフォルダ）");
                        });
                        row.col(|ui| {
                            ui.label("-");
                        });
                        row.col(|ui| {
                            ui.label("-");
                        });
                        row.col(|ui| {
                            ui.label("-");
                        });
                    });
                    return;
                }
                body.rows(row_h, num_rows, |mut row| {
                    let idx = row.index();
                    let Some(entry) = entries.get(idx) else {
                        return;
                    };
                    let is_sel = selected.contains(&idx);
                    let is_focus = focus == Some(idx);
                    if scroll_to == Some(idx) {
                        row.set_selected(true);
                    }
                    let tex = self.get_or_load_icon(&egui_ctx, &entry.path, entry.is_dir);
                    let emoji = shell::icon_emoji_for_path(&entry.path, entry.is_dir).to_string();
                    row.col(|ui| {
                        if is_focus && !is_sel {
                            let rect = ui.max_rect();
                            ui.painter().rect_stroke(
                                rect,
                                0.0,
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(80, 160, 255)),
                            );
                        }
                        let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                        let shift = ui.input(|i| i.modifiers.shift);
                        ui.horizontal(|ui| {
                            if let Some(t) = tex.clone() {
                                ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::new(
                                    t.id(),
                                    egui::vec2(16.0, 16.0),
                                )));
                            } else {
                                ui.label(emoji.clone());
                            }
                            let label = entry.name.clone();
                            let resp = ui.selectable_label(is_sel, label);
                            if resp.clicked() {
                                if ctrl {
                                    select_action = Some(SelectAction::Toggle(idx));
                                } else if shift {
                                    select_action = Some(SelectAction::Range(idx));
                                } else {
                                    select_action = Some(SelectAction::Only(idx));
                                }
                            }
                            if resp.double_clicked() {
                                select_action = Some(SelectAction::Only(idx));
                                if entry.is_dir {
                                    open_path = Some(entry.path.clone());
                                } else {
                                    let _ = shell::open_with_shell(&entry.path);
                                }
                            }
                            if resp.secondary_clicked() && !selected.contains(&idx) {
                                select_action = Some(SelectAction::Only(idx));
                            }
                            resp.context_menu(|ui| {
                                if ui.button("開く").clicked() {
                                    if entry.is_dir {
                                        open_path = Some(entry.path.clone());
                                    } else {
                                        let _ = shell::open_with_shell(&entry.path);
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Explorerで表示").clicked() {
                                    let _ = shell::reveal_in_explorer(&entry.path);
                                    ui.close_menu();
                                }
                                if ui.button("パスをコピー").clicked() {
                                    ui.output_mut(|o| {
                                        o.copied_text = entry.path.display().to_string()
                                    });
                                    ui.close_menu();
                                }
                                if ui.button("名前をコピー").clicked() {
                                    ui.output_mut(|o| o.copied_text = entry.name.clone());
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("コピー (Ctrl+C)").clicked() {
                                    cmd = Some(Command::Copy);
                                    ui.close_menu();
                                }
                                if ui.button("切り取り (Ctrl+X)").clicked() {
                                    cmd = Some(Command::Cut);
                                    ui.close_menu();
                                }
                                if ui.button("貼り付け (Ctrl+V)").clicked() {
                                    cmd = Some(Command::Paste);
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("名前変更 (F2)").clicked() {
                                    cmd = Some(Command::Rename);
                                    ui.close_menu();
                                }
                                if ui.button("ごみ箱へ (Del)").clicked() {
                                    cmd = Some(Command::Delete { permanent: false });
                                    ui.close_menu();
                                }
                                if ui.button("完全削除 (Shift+Del)").clicked() {
                                    cmd = Some(Command::Delete { permanent: true });
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("新しいフォルダ").clicked() {
                                    cmd = Some(Command::NewFolder);
                                    ui.close_menu();
                                }
                                if ui.button("全選択 (Ctrl+A)").clicked() {
                                    cmd = Some(Command::SelectAll);
                                    ui.close_menu();
                                }
                                if ui.button("選択を反転 (Ctrl+I)").clicked() {
                                    cmd = Some(Command::InvertSelection);
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                    row.col(|ui| {
                        ui.label(if entry.is_dir {
                            "-".into()
                        } else {
                            fs_ops::humansize(entry.size)
                        });
                    });
                    row.col(|ui| {
                        ui.label(fs_ops::fmt_time(entry.modified));
                    });
                    row.col(|ui| {
                        if let Some(t) = shell::os_type_name(&entry.path) {
                            ui.label(t);
                        } else {
                            ui.label(&entry.ext);
                        }
                    });
                });
            });

        if let Some(s) = sort_clicked {
            let t = self.tab_for_mut(pane);
            if t.sort_by == s {
                t.sort_desc = !t.sort_desc;
            } else {
                t.sort_by = s;
                t.sort_desc = false;
            }
            t.sort();
        }
        if let Some(a) = select_action {
            self.active_pane = match pane {
                PaneId::Tab => 0,
                PaneId::Pane2 => 1,
            };
            let t = self.tab_for_mut(pane);
            match a {
                SelectAction::Only(i) => t.select_only(i),
                SelectAction::Toggle(i) => t.toggle_select(i),
                SelectAction::Range(i) => t.select_range_to(i),
            }
            self.sync_address();
            self.update_preview();
        }
        if let Some(p) = open_path {
            self.active_pane = match pane {
                PaneId::Tab => 0,
                PaneId::Pane2 => 1,
            };
            self.tab_for_mut(pane).navigate_to(p);
            self.sync_address();
            self.note_navigation();
            self.sync_watchers();
            self.update_preview();
        }
        if let Some(c) = cmd {
            self.active_pane = match pane {
                PaneId::Tab => 0,
                PaneId::Pane2 => 1,
            };
            self.run_command(c);
        }
    }
}

enum SelectAction {
    Only(usize),
    Toggle(usize),
    Range(usize),
}

fn header_label(base: &str, active: bool, desc: bool) -> String {
    if !active {
        return base.to_string();
    }
    if desc {
        format!("{base} ▼")
    } else {
        format!("{base} ▲")
    }
}
