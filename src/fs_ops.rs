use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardMode {
    Copy,
    Cut,
}

#[derive(Debug, Clone)]
pub struct Clipboard {
    pub paths: Vec<PathBuf>,
    pub mode: ClipboardMode,
}

impl Clipboard {
    pub fn new(paths: Vec<PathBuf>, mode: ClipboardMode) -> Self {
        Self { paths, mode }
    }
}

/// Sanitize a user-supplied rename target: reject path separators, `..`, and Windows reserved names.
pub fn sanitize_filename(name: &str) -> Option<String> {
    // Validate raw input before trim so leading tabs / trailing spaces are not silently accepted.
    if name.ends_with(' ')
        || name.chars().any(|c| {
            matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control()
        })
    {
        return None;
    }
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return None;
    }
    if Path::new(name).components().count() != 1 {
        return None;
    }
    // Windows reserved device names (CON, PRN, AUX, NUL, COM1.., LPT1.., plus superscript variants)
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .to_ascii_uppercase();
    const RESERVED: &[&str] = &[
        "CON",
        "PRN",
        "AUX",
        "NUL",
        "COM1",
        "COM2",
        "COM3",
        "COM4",
        "COM5",
        "COM6",
        "COM7",
        "COM8",
        "COM9",
        "COM\u{00B9}",
        "COM\u{00B2}",
        "COM\u{00B3}",
        "LPT1",
        "LPT2",
        "LPT3",
        "LPT4",
        "LPT5",
        "LPT6",
        "LPT7",
        "LPT8",
        "LPT9",
        "LPT\u{00B9}",
        "LPT\u{00B2}",
        "LPT\u{00B3}",
    ];
    if RESERVED.contains(&stem.as_str()) {
        return None;
    }
    // Trailing dots are invalid on Windows
    if name.ends_with('.') {
        return None;
    }
    Some(name.to_string())
}

/// Normalize user input so both `\\` and `/` are accepted as separators.
pub fn normalize_path_input(input: &str) -> PathBuf {
    crate::shell::normalize_path_input(input)
}

pub fn copy_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let e = entry?;
            let s = e.path();
            let d = dst.join(e.file_name());
            copy_recursive(&s, &d)?;
        }
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

pub fn move_path(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_recursive(src, dst)?;
    if !dst.exists() {
        anyhow::bail!("コピー後に宛先が存在しません: {}", dst.display());
    }
    if src.is_dir() {
        std::fs::remove_dir_all(src).map_err(|e| {
            anyhow::anyhow!(
                "コピーは成功しましたが元の削除に失敗しました ({}): {e}",
                src.display()
            )
        })?;
    } else {
        std::fs::remove_file(src).map_err(|e| {
            anyhow::anyhow!(
                "コピーは成功しましたが元の削除に失敗しました ({}): {e}",
                src.display()
            )
        })?;
    }
    Ok(())
}

pub fn trash_paths(paths: &[PathBuf]) -> anyhow::Result<()> {
    let mut failures = Vec::new();
    for p in paths {
        if let Err(e) = trash::delete(p) {
            failures.push(format!("{}: {e}", p.display()));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "ごみ箱へ移動できませんでした（ごみ箱が無効/利用不可の可能性。Shift+Delで完全削除を試してください）: {}",
            failures.join(", ")
        )
    }
}

pub fn permanent_delete(paths: &[PathBuf]) -> anyhow::Result<()> {
    for p in paths {
        if p.is_dir() {
            std::fs::remove_dir_all(p)?;
        } else {
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
}

pub fn humansize(n: u64) -> String {
    const U: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i + 1 < U.len() {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} {}", U[i])
    } else {
        format!("{v:.1} {}", U[i])
    }
}

/// Sum sizes of selected file entries (directories count as 0).
pub fn selected_total_size<'a, I>(entries: I) -> u64
where
    I: IntoIterator<Item = &'a crate::tab::FileEntry>,
{
    entries
        .into_iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.size)
        .sum()
}

pub fn fmt_time(t: Option<std::time::SystemTime>) -> String {
    let Some(st) = t else {
        return "-".into();
    };
    let datetime: chrono::DateTime<chrono::Local> = st.into();
    datetime.format("%Y/%m/%d %H:%M").to_string()
}

/// Truncate a string by Unicode scalar values (safe for multi-byte UTF-8).
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars).collect();
    format!("{truncated}…\n\n({max_chars}文字で切り捨て)")
}

pub fn free_space(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        use windows::core::HSTRING;
        let hs = HSTRING::from(path.display().to_string());
        let mut free: u64 = 0;
        let mut total: u64 = 0;
        let mut avail: u64 = 0;
        unsafe {
            if GetDiskFreeSpaceExW(&hs, Some(&mut free), Some(&mut total), Some(&mut avail)).is_ok()
            {
                return Some(format!(
                    "空き {} / 全体 {}",
                    humansize(free),
                    humansize(total)
                ));
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

/// Unique destination path that does not overwrite existing files.
pub fn unique_dest(dest_dir: &Path, src: &Path) -> PathBuf {
    let file_name = src.file_name().unwrap_or_default();
    let mut dst = dest_dir.join(file_name);
    if !dst.exists() {
        return dst;
    }
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    for counter in 1..=999 {
        dst = dest_dir.join(format!("{stem} - コピー ({counter}){ext}"));
        if !dst.exists() {
            return dst;
        }
    }
    dest_dir.join(format!("{stem} - コピー ({}){ext}", uuid_like()))
}

fn uuid_like() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn humansize_basic() {
        assert_eq!(humansize(0), "0 B");
        assert_eq!(humansize(512), "512 B");
        assert!(humansize(2048).contains("KB"));
        assert!(humansize(5 * 1024 * 1024).contains("MB"));
    }

    #[test]
    fn sanitize_rejects_traversal() {
        assert!(sanitize_filename("ok.txt").is_some());
        assert!(sanitize_filename("../etc/passwd").is_none());
        assert!(sanitize_filename("a/b").is_none());
        assert!(sanitize_filename("a\\b").is_none());
        assert!(sanitize_filename("..").is_none());
        assert!(sanitize_filename("").is_none());
        assert!(sanitize_filename("  ").is_none());
        assert!(sanitize_filename("CON").is_none());
        assert!(sanitize_filename("con.txt").is_none());
        assert!(sanitize_filename("nul").is_none());
        assert!(sanitize_filename("bad:name").is_none());
        assert!(sanitize_filename("trailing.").is_none());
        assert!(sanitize_filename("\tfoo").is_none());
        assert!(sanitize_filename("foo ").is_none());
        assert!(sanitize_filename("COM\u{00B9}.txt").is_none());
        assert!(sanitize_filename("LPT\u{00B3}").is_none());
    }

    #[test]
    fn truncate_chars_safe() {
        let s = "あいうえお日本語テスト";
        let t = truncate_chars(s, 3);
        assert!(t.starts_with("あいう"));
        assert!(!t.contains('\u{FFFD}'));
    }

    #[test]
    fn normalize_accepts_both_separators() {
        let p1 = normalize_path_input("C:/Users/foo/bar");
        let p2 = normalize_path_input("C:\\Users\\foo\\bar");
        assert!(!p1.as_os_str().is_empty());
        assert!(!p2.as_os_str().is_empty());
        let mixed = normalize_path_input("C:\\Users/foo\\bar/baz");
        assert!(!mixed.as_os_str().is_empty());
    }

    #[test]
    fn copy_and_move_roundtrip() {
        let dir = tempfile_dir();
        let src = dir.join("a.txt");
        let mut f = fs::File::create(&src).unwrap();
        writeln!(f, "hello").unwrap();
        let dst = dir.join("b.txt");
        copy_recursive(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(&dst).unwrap().trim(), "hello");
        let dst2 = dir.join("c.txt");
        move_path(&dst, &dst2).unwrap();
        assert!(!dst.exists());
        assert!(dst2.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    fn tempfile_dir() -> PathBuf {
        let p = std::env::temp_dir().join(format!("explorer-rs-test-{}", uuid_like()));
        fs::create_dir_all(&p).unwrap();
        p
    }
}
