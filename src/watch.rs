use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{Config, EventKind, RecursiveMode, Watcher};
use notify::RecommendedWatcher;

use crate::commands::{BgEvent, BgSender};

/// Holds a `RecommendedWatcher` so the watch stays alive.
/// Dropping it stops watching.
pub struct WatchedDir {
    _watcher: RecommendedWatcher,
    pub path: PathBuf,
}

/// Start watching `path` (non-recursive) and forward coalesced
/// `FsChanged` events to the UI via `BgSender`.
/// Debounces bursts (e.g. copy of many files) to avoid refresh storms.
pub fn watch_dir(bg_tx: BgSender, path: PathBuf) -> Option<WatchedDir> {
    if !path.exists() {
        return None;
    }
    let (tx, rx) = mpsc::channel();
    let mut watcher = match RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default(),
    ) {
        Ok(w) => w,
        Err(_) => return None,
    };
    if watcher.watch(&path, RecursiveMode::NonRecursive).is_err() {
        return None;
    }
    let bg = bg_tx.clone();
    thread::spawn(move || {
        let debounce = Duration::from_millis(400);
        let mut pending = false;
        loop {
            match rx.recv_timeout(debounce) {
                Ok(Ok(event)) => {
                    match event.kind {
                        EventKind::Create(_)
                        | EventKind::Remove(_)
                        | EventKind::Modify(_)
                        | EventKind::Any => pending = true,
                        _ => {}
                    }
                    while let Ok(Ok(ev)) = rx.try_recv() {
                        match ev.kind {
                            EventKind::Create(_)
                            | EventKind::Remove(_)
                            | EventKind::Modify(_)
                            | EventKind::Any => pending = true,
                            _ => {}
                        }
                    }
                    if pending {
                        let _ = bg.send(BgEvent::FsChanged);
                        pending = false;
                        thread::sleep(Duration::from_millis(300));
                        while rx.try_recv().is_ok() {}
                    }
                }
                Ok(Err(_)) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if pending {
                        let _ = bg.send(BgEvent::FsChanged);
                        pending = false;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
    Some(WatchedDir { _watcher: watcher, path })
}
