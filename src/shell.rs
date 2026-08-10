//! Windows shell integration: OS-standard file icons, type names and helpers.
//!
//! - Icons: uses `SHGetFileInfoW` with `SHGFI_ICON` to obtain the real HICON
//!   Explorer shows, converts it to RGBA via `GetIconInfo` + `GetDIBits`, and
//!   returns raw bytes for egui to cache as textures. Single-binary, no assets.
//! - Type name: `SHGFI_TYPENAME` gives "テキスト ドキュメント", "PNG ファイル" etc.
//! - Helpers: `normalize_path_input` accepts both `\\` and `/` on any OS.

use std::path::Path;

/// Normalize a user-entered path so both `\\` and `/` are accepted on any OS.
/// On Windows, `/` is treated as `\\` (except UNC `\\\\` is preserved).
/// On Unix, `\\` is treated as `/` so `C:\\Users` style input still works.
pub fn normalize_path_input(input: &str) -> std::path::PathBuf {
    let s = input.trim();
    if s.is_empty() {
        return std::path::PathBuf::from(s);
    }
    #[cfg(windows)]
    {
        let is_unc = s.starts_with("\\\\") || s.starts_with("//");
        let mut n = s.replace('/', "\\");
        if is_unc && n.starts_with("//") {
            n = n.replacen("//", "\\\\", 1);
        }
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
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
    };
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    };
    use windows::Win32::UI::Shell::{
        SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON, SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES,
        SHGetFileInfoW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO, HICON};

    /// OS display type name, e.g. "テキスト ドキュメント", "PNG ファイル"
    pub fn os_type_name(path: &Path) -> Option<String> {
        let w: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut info = SHFILEINFOW::default();
        let ret = unsafe {
            SHGetFileInfoW(
                windows::core::PCWSTR(w.as_ptr()),
                FILE_ATTRIBUTE_NORMAL,
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
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }

    pub fn is_dir_via_attr(path: &Path) -> bool {
        let w: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut info = SHFILEINFOW::default();
        let ret = unsafe {
            SHGetFileInfoW(
                windows::core::PCWSTR(w.as_ptr()),
                FILE_ATTRIBUTE_DIRECTORY,
                Some(&mut info),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
            )
        };
        ret != 0
    }

    pub fn reveal_in_explorer(path: &Path) -> anyhow::Result<()> {
        // explorer /select,C:\\path\\to\\file — single arg form
        let path_str = path.display().to_string();
        let select_arg = format!("/select,{path_str}");
        std::process::Command::new("explorer")
            .arg(select_arg)
            .spawn()?;
        Ok(())
    }

    pub fn open_with_shell(path: &Path) -> anyhow::Result<()> {
        open::that(path)?;
        Ok(())
    }

    pub fn show_os_context_menu(_paths: &[std::path::PathBuf]) -> anyhow::Result<()> {
        anyhow::bail!(
            "OS context menu hosting requires HWND; use right-click -> Open / Reveal instead"
        )
    }

    /// Try to obtain the true OS icon as RGBA bytes (16x16). Returns (rgba, w, h).
    pub fn icon_rgba(path: &Path, is_dir: bool) -> Option<(Vec<u8>, i32, i32)> {
        let w: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut info = SHFILEINFOW::default();
        let attr = if is_dir {
            FILE_ATTRIBUTE_DIRECTORY
        } else {
            FILE_ATTRIBUTE_NORMAL
        };
        let use_attr = !path.exists();
        let mut flags = SHGFI_ICON | SHGFI_SMALLICON;
        if use_attr {
            flags |= SHGFI_USEFILEATTRIBUTES;
        }
        let ret = unsafe {
            SHGetFileInfoW(
                windows::core::PCWSTR(w.as_ptr()),
                attr,
                Some(&mut info),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                flags,
            )
        };
        let hicon = if ret == 0 || info.hIcon.is_invalid() {
            if !use_attr {
                let flags2 = SHGFI_ICON | SHGFI_SMALLICON | SHGFI_USEFILEATTRIBUTES;
                let ret2 = unsafe {
                    SHGetFileInfoW(
                        windows::core::PCWSTR(w.as_ptr()),
                        attr,
                        Some(&mut info),
                        std::mem::size_of::<SHFILEINFOW>() as u32,
                        flags2,
                    )
                };
                if ret2 == 0 || info.hIcon.is_invalid() {
                    return None;
                }
                info.hIcon
            } else {
                return None;
            }
        } else {
            info.hIcon
        };
        let res = unsafe { hicon_to_rgba(hicon) };
        unsafe {
            let _ = DestroyIcon(hicon);
        }
        res
    }

    unsafe fn hicon_to_rgba(hicon: HICON) -> Option<(Vec<u8>, i32, i32)> {
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(hicon, &mut icon_info).is_err() {
            return None;
        }
        let hbm_color = icon_info.hbmColor;
        let hbm_mask = icon_info.hbmMask;
        let is_color = !hbm_color.is_invalid() && hbm_color.0 != std::ptr::null_mut();
        let hbmp: HBITMAP = if is_color { hbm_color } else { hbm_mask };
        let mut bm = BITMAP::default();
        if GetObjectW(
            HGDIOBJ(hbmp.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut std::ffi::c_void),
        ) == 0
        {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
            return None;
        }
        let mut width = bm.bmWidth;
        let mut height = bm.bmHeight;
        if !is_color {
            height /= 2;
        }
        if width <= 0 || height <= 0 {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
            return None;
        }
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };
        let mut bits = vec![0u8; (width * height * 4) as usize];
        let hdc: HDC = GetDC(HWND(std::ptr::null_mut()));
        if hdc.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
            return None;
        }
        let scan = GetDIBits(
            hdc,
            hbmp,
            0,
            height as u32,
            Some(bits.as_mut_ptr() as *mut std::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(HWND(std::ptr::null_mut()), hdc);
        if scan == 0 {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
            return None;
        }
        for chunk in bits.chunks_exact_mut(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
        }
        let has_alpha = bits.chunks_exact(4).any(|c| c[3] != 0);
        if !has_alpha && is_color {
            let mut mask_bits = vec![0u8; (width * height * 4) as usize];
            let mut mask_bmi = bmi;
            let hdc2 = GetDC(HWND(std::ptr::null_mut()));
            if !hdc2.is_invalid() {
                let ret2 = GetDIBits(
                    hdc2,
                    hbm_mask,
                    0,
                    height as u32,
                    Some(mask_bits.as_mut_ptr() as *mut std::ffi::c_void),
                    &mut mask_bmi,
                    DIB_RGB_COLORS,
                );
                ReleaseDC(HWND(std::ptr::null_mut()), hdc2);
                if ret2 != 0 {
                    for i in 0..(width * height) as usize {
                        let is_white = mask_bits[i * 4] == 255
                            && mask_bits[i * 4 + 1] == 255
                            && mask_bits[i * 4 + 2] == 255;
                        if is_white {
                            bits[i * 4 + 3] = 0;
                        } else if bits[i * 4 + 3] == 0 {
                            bits[i * 4 + 3] = 255;
                        }
                    }
                } else {
                    for c in bits.chunks_exact_mut(4) {
                        c[3] = 255;
                    }
                }
            }
        } else if !is_color {
            for c in bits.chunks_exact_mut(4) {
                let is_white = c[0] == 255 && c[1] == 255 && c[2] == 255;
                if is_white {
                    c[0] = 0;
                    c[1] = 0;
                    c[2] = 0;
                    c[3] = 0;
                } else {
                    c[3] = 255;
                }
            }
        }
        let _ = DeleteObject(HGDIOBJ(hbm_color.0));
        let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
        Some((bits, width, height))
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
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "svg" => "🖼️",
            "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" => "📦",
            "exe" | "msi" | "dll" | "sys" => "⚙️",
            "txt" | "md" | "log" | "ini" | "cfg" => "📄",
            "rs" | "toml" | "json" | "yaml" | "yml" | "py" | "js" | "ts" | "go" | "c"
            | "cpp" | "h" | "cs" | "java" => "📝",
            "pdf" => "📕",
            "doc" | "docx" => "📘",
            "xls" | "xlsx" => "📗",
            "ppt" | "pptx" => "📙",
            _ => {
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
    pub fn os_type_name(_path: &Path) -> Option<String> {
        None
    }
    pub fn is_dir_via_attr(_path: &Path) -> bool {
        false
    }
    pub fn reveal_in_explorer(_path: &Path) -> anyhow::Result<()> {
        anyhow::bail!("Windows only")
    }
    pub fn open_with_shell(path: &Path) -> anyhow::Result<()> {
        open::that(path)?;
        Ok(())
    }
    pub fn show_os_context_menu(_paths: &[std::path::PathBuf]) -> anyhow::Result<()> {
        anyhow::bail!("Windows only")
    }
    pub fn icon_rgba(_path: &Path, _is_dir: bool) -> Option<(Vec<u8>, i32, i32)> {
        None
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
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" => "🖼️",
            "zip" | "7z" | "rar" | "tar" | "gz" => "📦",
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
            assert_eq!(
                normalize_path_input("C:/Users/foo/bar").to_string_lossy(),
                "C:\\Users\\foo\\bar"
            );
            assert_eq!(
                normalize_path_input("C:\\Users\\foo/bar\\baz").to_string_lossy(),
                "C:\\Users\\foo\\bar\\baz"
            );
            assert_eq!(
                normalize_path_input("\\\\server\\share/folder").to_string_lossy(),
                "\\\\server\\share\\folder"
            );
        }
        #[cfg(not(windows))]
        {
            assert_eq!(
                normalize_path_input("C:\\Users\\foo\\bar").to_string_lossy(),
                "C:/Users/foo/bar"
            );
            assert_eq!(normalize_path_input("a/b\\c").to_string_lossy(), "a/b/c");
        }
    }
    #[test]
    fn normalize_trims() {
        let p = normalize_path_input("  C:/a  ");
        #[cfg(windows)]
        assert_eq!(p.to_string_lossy().as_ref(), r"C:\\a");
        #[cfg(not(windows))]
        assert_eq!(p.to_string_lossy().as_ref(), "C:/a");
    }
}
