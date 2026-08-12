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
