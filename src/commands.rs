use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use crate::fs_ops::{self, ClipboardMode};
use crate::tab::FileEntry;

/// All user-facing operations funnel through this enum.
#[derive(Debug, Clone)]
pub enum Command {
    Copy,
    Cut,
    Paste,
    Delete { permanent: bool },
    Rename,
    NewFolder,
    NewTextFile,
    SelectAll,
    Refresh,
    GoUp,
    GoBack,
    GoForward,
    OpenPrimary,
    AddBookmark,
    Search,
    ConfirmDelete { permanent: bool },
    CancelDialog,
}

/// Results delivered from background workers back to the UI thread.
#[derive(Debug)]
pub enum BgEvent {
    SearchDone {
        query: String,
        results: Vec<FileEntry>,
    },
    PasteDone {
        ok: usize,
        err: usize,
        message: String,
    },
}

pub type BgSender = mpsc::Sender<BgEvent>;
pub type BgReceiver = mpsc::Receiver<BgEvent>;

pub fn channel() -> (BgSender, BgReceiver) {
    mpsc::channel()
}

pub fn spawn_search(tx: BgSender, root: PathBuf, query: String) {
    thread::spawn(move || {
        let results = crate::tab::Tab::search_blocking(&root, &query, 300);
        let _ = tx.send(BgEvent::SearchDone { query, results });
    });
}

pub fn spawn_paste(
    tx: BgSender,
    paths: Vec<PathBuf>,
    mode: ClipboardMode,
    dest_dir: PathBuf,
) {
    thread::spawn(move || {
        let mut ok = 0usize;
        let mut err = 0usize;
        let mut last_err = String::new();
        for src in &paths {
            let dst = fs_ops::unique_dest(&dest_dir, src);
            let res = match mode {
                ClipboardMode::Copy => fs_ops::copy_recursive(src, &dst),
                ClipboardMode::Cut => fs_ops::move_path(src, &dst),
            };
            match res {
                Ok(()) => ok += 1,
                Err(e) => {
                    err += 1;
                    last_err = e.to_string();
                }
            }
        }
        let message = if err == 0 {
            format!("貼り付け完了: {ok} 件")
        } else {
            format!("貼り付け: 成功 {ok}, 失敗 {err} — {last_err}")
        };
        let _ = tx.send(BgEvent::PasteDone { ok, err, message });
    });
}
