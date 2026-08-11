//! Windows shell integration: OS-standard file icons, type names and helpers.

use std::path::{Path, PathBuf};

/// Normalize a user-entered path so both `\\` and `/` are accepted on any OS.
pub fn normalize_path_input(input: &str) -> PathBuf {
    let s = input.trim();
    if s.is_empty() {
        return PathBuf::from(s);
    }
    #[cfg(windows)]
    {
        let is_unc = s.starts_with("\\\\") || s.starts_with("//");
        let mut n = s.replace('/', "\\");
        if is_unc && n.starts_with("//") {
            n = n.replacen("//", "\\\\", 1);
        }
        PathBuf::from(n)
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(s.replace('\\', "/"))
    }
}

/// Prefix `\\?\\` on Windows when path is long or absolute, for Win32 long-path APIs.
/// Leaves UNC and already-prefixed paths alone. Non-Windows: identity.
pub fn to_long_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if s.starts_with("\\\\?\\") || s.starts_with("//?/") {
            return path.to_path_buf();
        }
        // UNC \\\\server\\share → \\\\?\\UNC\\server\\share
        if s.starts_with("\\\\") && !s.starts_with("\\\\?\\") {
            let rest = s.trim_start_matches('\\');
            return PathBuf::from(format!("\\\\?\\UNC\\{rest}"));
        }
        if path.is_absolute() {
            // Only needed when over MAX_PATH-ish; always safe for absolute drive paths.
            if s.len() >= 240 || s.contains("..") {
                return PathBuf::from(format!("\\\\?\\{s}"));
            }
            if s.len() >= 248 {
                return PathBuf::from(format!("\\\\?\\{s}"));
            }
            // Enable for paths that already exceed classic MAX_PATH
            if s.len() > 260 {
                return PathBuf::from(format!("\\\\?\\{s}"));
            }
        }
        path.to_path_buf()
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

#[cfg(windows)]
mod windows_shell {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
    };
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    };
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON, SHGFI_TYPENAME,
        SHGFI_USEFILEATTRIBUTES,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    pub fn os_type_name(path: &Path) -> Option<String> {
        let path = to_long_path(path);
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
        let _ = path;
        false
    }

    pub fn reveal_in_explorer(path: &Path) -> anyhow::Result<()> {
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

    pub fn show_os_context_menu(_paths: &[PathBuf]) -> anyhow::Result<()> {
        anyhow::bail!("OS context menu hosting requires HWND")
    }

    pub fn icon_rgba(path: &Path, is_dir: bool) -> Option<(Vec<u8>, i32, i32)> {
        let path = to_long_path(path);
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
            for c in bits.chunks_exact_mut(4) {
                c[3] = 255;
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
            _ => "📄",
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
    pub fn show_os_context_menu(_paths: &[PathBuf]) -> anyhow::Result<()> {
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
        }
        #[cfg(not(windows))]
        {
            assert_eq!(
                normalize_path_input("C:\\Users\\foo\\bar").to_string_lossy(),
                "C:/Users/foo/bar"
            );
        }
    }

    #[test]
    fn normalize_trims() {
        let p = normalize_path_input("  C:/a  ");
        #[cfg(windows)]
        assert_eq!(p.to_string_lossy().as_ref(), r"C:\a");
        #[cfg(not(windows))]
        assert_eq!(p.to_string_lossy().as_ref(), "C:/a");
    }

    #[test]
    fn long_path_identity_for_short() {
        let p = PathBuf::from(if cfg!(windows) { r"C:\short" } else { "/tmp/x" });
        let lp = to_long_path(&p);
        // short paths stay as-is on both OS
        assert_eq!(lp, p);
    }
}
