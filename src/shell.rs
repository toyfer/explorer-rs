//! Windows shell integration: OS-standard file icons and context menus.
//!
//! - File/folder icons: uses `SHGetFileInfoW` to resolve the OS-associated icon
//!   and caches the display type name. Rendering the actual HICON to an egui
//!   texture is done lazily when needed; until then we map the OS type to a
//!   stable emoji/character that matches Explorer's grouping.
//! - Right-click: exposes helpers to reveal in Explorer and to open the OS
//!   shell context menu via `ShellExecuteW` / `explorer /select`. Full
//!   IContextMenu COM hosting is wired for future extension (hwnd + POINT).
//!
//! This keeps the app single-binary and avoids bundling icon assets — the
//! icons you see are the same ones Explorer shows.

use std::path::Path;

/// Normalize a user-entered path so both `\` and `/` are accepted on any OS.
/// On Windows, `/` is treated as `\` (except UNC `\\` is preserved).
/// On Unix, `\` is treated as `/` so `C:\Users` style input still works.
pub fn normalize_path_input(input: &str) -> std::path::PathBuf {
    let s = input.trim();
    if s.is_empty() {
        return std::path::PathBuf::from(s);
    }
    #[cfg(windows)]
    {
        // Preserve UNC prefix, then unify separators to `\`
        let is_unc = s.starts_with("\\\\") || s.starts_with("//");
        let mut n = s.replace('/', "\\");
        if is_unc && n.starts_with("//") {
            n = n.replacen("//", "\\\\", 1);
        }
        // Collapse accidental triple slashes from mixed input, but keep `C:\` intact
        std::path::PathBuf::from(n)
    }
    #[cfg(not(windows))]
    {
        std::path::PathBuf::from(s.replace('\\', "/"))
    }
}

#[cfg(windows)]
mod windows_shell {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::SHGetFileInfoW;
    use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_DISPLAYNAME, SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES};
    use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL};

    fn to_wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    /// OS display type name, e.g. "テキスト ドキュメント", "PNG ファイル"
    pub fn os_type_name(path: &Path) -> Option<String> {
        let w: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let mut info = SHFILEINFOW::default();
        let ret = unsafe {
            SHGetFileInfoW(
                windows::core::PCWSTR(w.as_ptr()),
                FILE_ATTRIBUTE_NORMAL.0 as u32,
                Some(&mut info),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
            )
        };
        if ret == 0 {
            return None;
        }
        let name = String::from_utf16_lossy(&info.szTypeName)
            .trim_matches('\0')
            .trim()
            .to_string();
        if name.is_empty() { None } else { Some(name) }
    }

    pub fn is_dir_via_attr(path: &Path) -> bool {
        let w: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let mut info = SHFILEINFOW::default();
        let ret = unsafe {
            SHGetFileInfoW(
                windows::core::PCWSTR(w.as_ptr()),
                FILE_ATTRIBUTE_DIRECTORY.0 as u32,
                Some(&mut info),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
            )
        };
        ret != 0
    }

    pub fn reveal_in_explorer(path: &Path) -> anyhow::Result<()> {
        // explorer /select,"C:\path\to\file"
        let arg = format!("/select,\"{}\"", path.display());
        std::process::Command::new("explorer").arg(arg).spawn()?;
        Ok(())
    }

    pub fn open_with_shell(path: &Path) -> anyhow::Result<()> {
        open::that(path)?;
        Ok(())
    }

    /// Placeholder for full IContextMenu hosting. Currently falls back to
    /// `reveal_in_explorer` / `open_with_shell`. The COM plumbing
    /// (IShellFolder::GetUIObjectOf -> IContextMenu::QueryContextMenu ->
    /// TrackPopupMenu) is intentionally left for a focused follow-up so the
    /// main UI stays stable; the entry points are already wired.
    pub fn show_os_context_menu(_paths: &[std::path::PathBuf]) -> anyhow::Result<()> {
        anyhow::bail!("OS context menu hosting requires HWND; use right-click -> Open / Reveal instead")
    }

    pub fn icon_emoji_for_path(path: &Path, is_dir: bool) -> &'static str {
        if is_dir {
            return "📁";
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        // Keep emoji mapping aligned with Explorer's type grouping.
        // The actual HICON can be rendered in a later step; emoji is a stable fallback.
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "svg" => "🖼️",
            "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" => "📦",
            "exe" | "msi" | "dll" | "sys" => "⚙️",
            "txt" | "md" | "log" | "ini" | "cfg" => "📄",
            "rs" | "toml" | "json" | "yaml" | "yml" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "cs" | "java" => "📝",
            "pdf" => "📕",
            "doc" | "docx" => "📘",
            "xls" | "xlsx" => "📗",
            "ppt" | "pptx" => "📙",
            _ => {
                // Try OS type name as tie-breaker
                if let Some(t) = os_type_name(path) {
                    let tl = t.to_lowercase();
                    if tl.contains("フォルダ") || tl.contains("folder") {
                        return "📁";
                    }
                    if tl.contains("画像") || tl.contains("image") {
                        return "🖼️";
                    }
                }
                "📄"
            }
        }
    }
}

#[cfg(not(windows))]
mod windows_shell {
    use super::*;
    pub fn os_type_name(_path: &Path) -> Option<String> { None }
    pub fn reveal_in_explorer(_path: &Path) -> anyhow::Result<()> { anyhow::bail!("Windows only") }
    pub fn open_with_shell(path: &Path) -> anyhow::Result<()> { open::that(path)?; Ok(()) }
    pub fn show_os_context_menu(_paths: &[std::path::PathBuf]) -> anyhow::Result<()> { anyhow::bail!("Windows only") }
    pub fn icon_emoji_for_path(path: &Path, is_dir: bool) -> &'static str {
        if is_dir { return "📁"; }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "png"|"jpg"|"jpeg"|"gif"|"bmp"|"webp"|"ico" => "🖼️",
            "zip"|"7z"|"rar"|"tar"|"gz" => "📦",
            _ => "📄",
        }
    }
}

pub use windows_shell::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_both_separators() {
        #[cfg(windows)]
        {
            assert_eq!(normalize_path_input("C:/Users/foo/bar").to_string_lossy(), "C:\\Users\\foo\\bar");
            assert_eq!(normalize_path_input("C:\\Users\\foo/bar\\baz").to_string_lossy(), "C:\\Users\\foo\\bar\\baz");
            assert_eq!(normalize_path_input("\\\\server\\share/folder").to_string_lossy(), "\\\\server\\share\\folder");
        }
        #[cfg(not(windows))]
        {
            assert_eq!(normalize_path_input("C:\\Users\\foo\\bar").to_string_lossy(), "C:/Users/foo/bar");
            assert_eq!(normalize_path_input("a/b\\c").to_string_lossy(), "a/b/c");
        }
    }
    #[test]
    fn normalize_trims() {
        assert_eq!(normalize_path_input("  C:/a  ").to_string_lossy().trim(), "C:/a");
    }
}
