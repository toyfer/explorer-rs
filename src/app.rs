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
    fn get_or_load_icon(
        &self,
        ctx: &egui::Context,
        path: &Path,
        is_dir: bool,
    ) -> Option<egui::TextureHandle> {
        crate::file_icons::load_for_path(ctx, &self.icon_cache, path, is_dir)
    }
    fn handle_typeahead(&mut self, ctx: &egui::Context) {
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
