use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::WalkDir;

use crate::config::SortBy;
use crate::nat_sort;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub ext: String,
}

impl FileEntry {
    pub fn from_path(p: &Path) -> Option<Self> {
        let md = std::fs::metadata(p).ok()?;
        let name = p.file_name()?.to_string_lossy().to_string();
        let is_hidden = is_hidden_path(p, &name, &md);
        let ext = if md.is_dir() {
            "フォルダ".to_string()
        } else {
            p.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("ファイル")
                .to_lowercase()
        };
        Some(Self {
            name,
            path: p.to_path_buf(),
            is_dir: md.is_dir(),
            is_hidden,
            size: if md.is_dir() { 0 } else { md.len() },
            modified: md.modified().ok(),
            ext,
        })
    }
}

fn is_hidden_path(p: &Path, name: &str, md: &std::fs::Metadata) -> bool {
    if name.starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
        let attrs = md.file_attributes();
        if attrs & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0 {
            return true;
        }
        let _ = p;
    }
    #[cfg(not(windows))]
    {
        let _ = (p, md);
    }
    false
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub current: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected: BTreeSet<usize>,
    pub history_back: Vec<PathBuf>,
    pub history_forward: Vec<PathBuf>,
    pub filter: String,
    pub sort_by: SortBy,
    pub sort_desc: bool,
    pub show_hidden: bool,
    pub error: Option<String>,
    pub focus: Option<usize>,
    pub anchor: Option<usize>,
}

impl Tab {
    pub fn new(path: PathBuf, show_hidden: bool, sort_by: SortBy, sort_desc: bool) -> Self {
        let mut t = Self {
            current: path,
            entries: vec![],
            selected: BTreeSet::new(),
            history_back: vec![],
            history_forward: vec![],
            filter: String::new(),
            sort_by,
            sort_desc,
            show_hidden,
            error: None,
            focus: None,
            anchor: None,
        };
        t.refresh();
        t
    }

    pub fn list_blocking(
        dir: &Path,
        show_hidden: bool,
        filter: &str,
    ) -> (Vec<FileEntry>, Option<String>) {
        let mut entries = Vec::new();
        let mut error = None;
        match std::fs::read_dir(dir) {
            Ok(rd) => {
                let filter_lc = filter.to_lowercase();
                for e in rd.flatten() {
                    let path = e.path();
                    if let Some(entry) = FileEntry::from_path(&path) {
                        if !show_hidden && entry.is_hidden {
                            continue;
                        }
                        if !filter_lc.is_empty() && !entry.name.to_lowercase().contains(&filter_lc) {
                            continue;
                        }
                        entries.push(entry);
                    }
                }
            }
            Err(e) => error = Some(format!("読み取り失敗: {e}")),
        }
        (entries, error)
    }

    pub fn sort_entries(entries: &mut Vec<FileEntry>, sort_by: SortBy, sort_desc: bool) {
        match sort_by {
            SortBy::Name => entries.sort_by(|a, b| {
                let ord = b
                    .is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| nat_sort::natural_cmp(&a.name, &b.name));
                if sort_desc {
                    ord.reverse()
                } else {
                    ord
                }
            }),
            SortBy::Size => entries.sort_by(|a, b| {
                let ord = b.is_dir.cmp(&a.is_dir).then(a.size.cmp(&b.size));
                if sort_desc {
                    ord.reverse()
                } else {
                    ord
                }
            }),
            SortBy::Modified => entries.sort_by(|a, b| {
                let ord = b.is_dir.cmp(&a.is_dir).then(a.modified.cmp(&b.modified));
                if sort_desc {
                    ord.reverse()
                } else {
                    ord
                }
            }),
            SortBy::Type => entries.sort_by(|a, b| {
                let ord = b
                    .is_dir
                    .cmp(&a.is_dir)
                    .then(a.ext.cmp(&b.ext))
                    .then_with(|| nat_sort::natural_cmp(&a.name, &b.name));
                if sort_desc {
                    ord.reverse()
                } else {
                    ord
                }
            }),
        }
    }

    pub fn apply_list(&mut self, entries: Vec<FileEntry>, error: Option<String>) {
        let selected_paths: Vec<PathBuf> = self
            .selected
            .iter()
            .filter_map(|&i| self.entries.get(i).map(|e| e.path.clone()))
            .collect();
        let focus_path = self
            .focus
            .and_then(|i| self.entries.get(i).map(|e| e.path.clone()));
        self.entries = entries;
        self.error = error;
        self.selected.clear();
        for (i, e) in self.entries.iter().enumerate() {
            if selected_paths.iter().any(|p| p == &e.path) {
                self.selected.insert(i);
            }
        }
        self.focus = focus_path.and_then(|p| self.entries.iter().position(|e| e.path == p));
        self.anchor = self.focus;
    }

    pub fn refresh(&mut self) {
        let (mut entries, error) =
            Self::list_blocking(&self.current, self.show_hidden, &self.filter);
        Self::sort_entries(&mut entries, self.sort_by, self.sort_desc);
        self.apply_list(entries, error);
    }

    pub fn sort(&mut self) {
        let selected_paths: Vec<PathBuf> = self
            .selected
            .iter()
            .filter_map(|&i| self.entries.get(i).map(|e| e.path.clone()))
            .collect();
        let focus_path = self
            .focus
            .and_then(|i| self.entries.get(i).map(|e| e.path.clone()));
        Self::sort_entries(&mut self.entries, self.sort_by, self.sort_desc);
        self.selected.clear();
        for (i, e) in self.entries.iter().enumerate() {
            if selected_paths.iter().any(|p| p == &e.path) {
                self.selected.insert(i);
            }
        }
        self.focus = focus_path.and_then(|p| self.entries.iter().position(|e| e.path == p));
        self.anchor = self.focus;
    }

    /// Non-blocking path change. Caller should `request_refresh_async`.
    pub fn navigate_to_async(&mut self, path: PathBuf) -> bool {
        if path == self.current {
            return false;
        }
        if path.is_dir() {
            self.history_back.push(self.current.clone());
            self.history_forward.clear();
            self.current = path;
            self.selected.clear();
            self.focus = None;
            self.anchor = None;
            self.entries.clear();
            self.error = None;
            true
        } else {
            false
        }
    }

    pub fn navigate_to(&mut self, path: PathBuf) {
        if path == self.current {
            return;
        }
        if path.exists() && path.is_dir() {
            self.history_back.push(self.current.clone());
            self.history_forward.clear();
            self.current = path;
            self.selected.clear();
            self.focus = None;
            self.anchor = None;
            self.refresh();
        }
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(prev) = self.history_back.pop() {
            self.history_forward.push(self.current.clone());
            self.current = prev;
            self.selected.clear();
            self.focus = None;
            self.anchor = None;
            self.refresh();
            true
        } else {
            false
        }
    }

    pub fn go_forward(&mut self) -> bool {
        if let Some(next) = self.history_forward.pop() {
            self.history_back.push(self.current.clone());
            self.current = next;
            self.selected.clear();
            self.focus = None;
            self.anchor = None;
            self.refresh();
            true
        } else {
            false
        }
    }

    pub fn go_up(&mut self) -> bool {
        if let Some(parent) = self.current.parent().map(|p| p.to_path_buf()) {
            if parent != self.current {
                self.navigate_to(parent);
                return true;
            }
        }
        false
    }

    pub fn primary_selected(&self) -> Option<&FileEntry> {
        self.focus.and_then(|i| self.entries.get(i)).or_else(|| {
            self.selected
                .iter()
                .next()
                .and_then(|&i| self.entries.get(i))
        })
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected
            .iter()
            .filter_map(|&i| self.entries.get(i).map(|e| e.path.clone()))
            .collect()
    }

    pub fn select_only(&mut self, idx: usize) {
        self.selected.clear();
        if idx < self.entries.len() {
            self.selected.insert(idx);
            self.focus = Some(idx);
            self.anchor = Some(idx);
        }
    }

    pub fn toggle_select(&mut self, idx: usize) {
        if idx >= self.entries.len() {
            return;
        }
        if self.selected.contains(&idx) {
            self.selected.remove(&idx);
        } else {
            self.selected.insert(idx);
        }
        self.focus = Some(idx);
        self.anchor = Some(idx);
    }

    pub fn select_range_to(&mut self, idx: usize) {
        if idx >= self.entries.len() {
            return;
        }
        let anchor = self.anchor.unwrap_or(idx);
        let (a, b) = if anchor <= idx {
            (anchor, idx)
        } else {
            (idx, anchor)
        };
        self.selected.clear();
        for i in a..=b {
            self.selected.insert(i);
        }
        self.focus = Some(idx);
    }

    pub fn select_all(&mut self) {
        self.selected = (0..self.entries.len()).collect();
        if !self.entries.is_empty() {
            self.focus = Some(0);
            self.anchor = Some(0);
        }
    }

    pub fn invert_selection(&mut self) {
        let all: BTreeSet<usize> = (0..self.entries.len()).collect();
        self.selected = all.difference(&self.selected).copied().collect();
        self.focus = self.selected.iter().next().copied().or(self.focus);
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.focus = None;
        self.anchor = None;
    }

    pub fn move_focus_by(&mut self, delta: isize, extend: bool) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let cur = self.focus.unwrap_or(0) as isize;
        let nxt = (cur + delta).clamp(0, (len - 1) as isize) as usize;
        if extend {
            if self.anchor.is_none() {
                self.anchor = self.focus.or(Some(cur.max(0) as usize));
            }
            self.focus = Some(nxt);
            let anchor = self.anchor.unwrap_or(nxt);
            let (a, b) = if anchor <= nxt {
                (anchor, nxt)
            } else {
                (nxt, anchor)
            };
            self.selected.clear();
            for i in a..=b {
                self.selected.insert(i);
            }
        } else {
            self.select_only(nxt);
        }
    }

    pub fn focus_first(&mut self, extend: bool) {
        if self.entries.is_empty() {
            return;
        }
        if extend {
            if self.anchor.is_none() {
                self.anchor = self.focus.or(Some(0));
            }
            self.focus = Some(0);
            let anchor = self.anchor.unwrap_or(0);
            let lo = anchor.min(0);
            let hi = anchor.max(0);
            self.selected.clear();
            for i in lo..=hi {
                self.selected.insert(i);
            }
        } else {
            self.select_only(0);
        }
    }

    pub fn focus_last(&mut self, extend: bool) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let last = len - 1;
        if extend {
            if self.anchor.is_none() {
                self.anchor = self.focus.or(Some(last));
            }
            self.focus = Some(last);
            let anchor = self.anchor.unwrap_or(last);
            let (a, b) = if anchor <= last {
                (anchor, last)
            } else {
                (last, anchor)
            };
            self.selected.clear();
            for i in a..=b {
                self.selected.insert(i);
            }
        } else {
            self.select_only(last);
        }
    }

    pub fn enter_primary(&mut self) -> Option<PathBuf> {
        let entry = self.primary_selected()?.clone();
        if entry.is_dir {
            self.navigate_to(entry.path);
            None
        } else {
            Some(entry.path)
        }
    }

    pub fn search_blocking(root: &Path, query: &str, max_results: usize) -> Vec<FileEntry> {
        if query.is_empty() {
            return vec![];
        }
        let q = query.to_lowercase();
        let mut results = Vec::new();
        for entry in WalkDir::new(root)
            .max_depth(8)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if results.len() >= max_results {
                break;
            }
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                if name.to_lowercase().contains(&q) {
                    if let Some(fe) = FileEntry::from_path(p) {
                        results.push(fe);
                    }
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SortBy;
    use std::fs;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "explorer-rs-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn sort_dirs_first() {
        let dir = tmp("tab");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.txt"), b"x").unwrap();
        fs::write(dir.join("z.txt"), b"y").unwrap();
        let tab = Tab::new(dir.clone(), true, SortBy::Name, false);
        assert!(tab.entries[0].is_dir);
        assert_eq!(tab.entries[0].name, "sub");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn natural_name_order() {
        let dir = tmp("natsort");
        fs::create_dir_all(&dir).unwrap();
        for n in ["file10.txt", "file2.txt", "file1.txt"] {
            fs::write(dir.join(n), b"x").unwrap();
        }
        let tab = Tab::new(dir.clone(), true, SortBy::Name, false);
        let names: Vec<_> = tab.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["file1.txt", "file2.txt", "file10.txt"]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn multi_select_range() {
        let dir = tmp("sel");
        fs::create_dir_all(&dir).unwrap();
        for i in 0..5 {
            fs::write(dir.join(format!("f{i}.txt")), b"x").unwrap();
        }
        let mut tab = Tab::new(dir.clone(), true, SortBy::Name, false);
        tab.select_only(1);
        tab.select_range_to(3);
        assert_eq!(tab.selected.len(), 3);
        tab.select_all();
        assert_eq!(tab.selected.len(), tab.entries.len());
        tab.invert_selection();
        assert!(tab.selected.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn list_blocking_filters_hidden() {
        let dir = tmp("hidden");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(".hidden"), b"x").unwrap();
        fs::write(dir.join("visible.txt"), b"x").unwrap();
        let (entries, _) = Tab::list_blocking(&dir, false, "");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "visible.txt");
        let (entries2, _) = Tab::list_blocking(&dir, true, "");
        assert_eq!(entries2.len(), 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn move_focus_by_clamps() {
        let dir = tmp("focus");
        fs::create_dir_all(&dir).unwrap();
        for i in 0..3 {
            fs::write(dir.join(format!("f{i}.txt")), b"x").unwrap();
        }
        let mut tab = Tab::new(dir.clone(), true, SortBy::Name, false);
        tab.select_only(0);
        tab.move_focus_by(10, false);
        assert_eq!(tab.focus, Some(2));
        tab.move_focus_by(-100, false);
        assert_eq!(tab.focus, Some(0));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn navigate_async_clears_without_list() {
        let a = tmp("nav-a");
        let b = tmp("nav-b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("x.txt"), b"x").unwrap();
        let mut tab = Tab::new(a.clone(), true, SortBy::Name, false);
        assert!(!tab.entries.is_empty());
        assert!(tab.navigate_to_async(b.clone()));
        assert!(tab.entries.is_empty());
        assert_eq!(tab.current, b);
        let _ = fs::remove_dir_all(a);
        let _ = fs::remove_dir_all(b);
    }
}
