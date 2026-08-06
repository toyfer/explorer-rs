//! Windows shell integration: OS-standard file icons and context menu.
//! On non-Windows, falls back to emoji / custom menu.

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use eframe::egui;

#[cfg(windows)]
mod win_impl {
    use super::*;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject};
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, GetForegroundWindow, GetCursorPos};
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{SHParseDisplayName, SHBindToParent};
    use windows::Win32::UI::Shell::{IContextMenu, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX, CMIC_MASK_UNICODE};
    use windows::Win32::UI::WindowsAndMessaging::{TrackPopupMenuEx, TPM_RETURNCMD, TPM_RIGHTBUTTON, TPM_LEFTALIGN, TPM_TOPALIGN};
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::core::Interface;

    fn to_wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

    /// Extract 16x16 RGBA icon for a path via SHGetFileInfoW. Returns None on failure.
    pub fn icon_rgba(path: &Path, size: i32) -> Option<egui::ColorImage> {
        unsafe {
            let wide: Vec<u16> = to_wide(&path.to_string_lossy());
            let mut shfi: SHFILEINFOW = std::mem::zeroed();
            let ret = SHGetFileInfoW(PCWSTR(wide.as_ptr()), windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut shfi), std::mem::size_of::<SHFILEINFOW>() as u32, SHGFI_ICON | SHGFI_SMALLICON);
            if ret == 0 || shfi.hIcon.is_invalid() { return None; }
            let img = hicon_to_image(shfi.hIcon, size);
            let _ = DestroyIcon(shfi.hIcon);
            img
        }
    }

    unsafe fn hicon_to_image(hicon: windows::Win32::UI::WindowsAndMessaging::HICON, size: i32) -> Option<egui::ColorImage> {
        let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
        if hdc_screen.is_invalid() { return None; }
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_invalid() { ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen); return None; }
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = size;
        bmi.bmiHeader.biHeight = -size; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0 as u32;
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
        if hbmp.is_invalid() || bits.is_null() { DeleteDC(hdc_mem); ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen); return None; }
        let old = SelectObject(hdc_mem, hbmp);
        // Fill transparent
        let rect_ok = DrawIconEx(hdc_mem, 0, 0, hicon, size, size, 0, None, DI_NORMAL);
        if rect_ok == windows::Win32::Foundation::BOOL(0) { SelectObject(hdc_mem, old); DeleteObject(hbmp); DeleteDC(hdc_mem); ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen); return None; }
        let slice = std::slice::from_raw_parts(bits as *const u8, (size * size * 4) as usize);
        let mut rgba = Vec::with_capacity((size*size*4) as usize);
        for chunk in slice.chunks_exact(4) {
            let (b,g,r,a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
            // DIB is BGRA, convert to RGBA
            // If alpha is 0 but color non-zero, make opaque (some icons have no alpha)
            let a = if a==0 && (r!=0 || g!=0 || b!=0) { 255 } else { a };
            rgba.extend_from_slice(&[r,g,b,a]);
        }
        SelectObject(hdc_mem, old);
        DeleteObject(hbmp);
        DeleteDC(hdc_mem);
        ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
        Some(egui::ColorImage::from_rgba_unmultiplied([size as usize, size as usize], &rgba))
    }

    /// Show the OS-native context menu for given paths at cursor position.
    /// Uses SHBindToParent + IContextMenu. Best-effort: returns Err with message on failure.
    pub fn show_context_menu(paths: &[PathBuf]) -> Result<(), String> {
        if paths.is_empty() { return Err("no paths".into()); }
        unsafe {
            let hr_init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let need_uninit = hr_init.is_ok();
            let result = (|| -> Result<(), String> {
                // All paths must share same parent for single menu; otherwise show for first
                let parent = paths[0].parent().ok_or("no parent")?;
                let parent_w: Vec<u16> = to_wide(&parent.to_string_lossy());
                let mut pidl_parent: *mut windows::Win32::UI::Shell::ITEMIDLIST = std::ptr::null_mut();
                let hr = SHParseDisplayName(PCWSTR(parent_w.as_ptr()), None, &mut pidl_parent, 0, None);
                if hr.is_err() { return Err(format!("SHParseDisplayName failed: {hr:?}")); }
                // We need to free pidl later via CoTaskMemFree
                let mut psf: Option<windows::Win32::UI::Shell::IShellFolder> = None;
                let mut pidl_child: *const windows::Win32::UI::Shell::ITEMIDLIST = std::ptr::null();
                let hr2 = SHBindToParent(pidl_parent, &windows::Win32::UI::Shell::IShellFolder::IID, &mut psf as *mut _ as *mut _, &mut pidl_child);
                if hr2.is_err() || psf.is_none() {
                    windows::Win32::System::Com::CoTaskMemFree(Some(pidl_parent as *const _));
                    return Err(format!("SHBindToParent failed: {hr2:?}"));
                }
                let psf = psf.unwrap();
                // Build child pidls for all paths under same parent
                let mut child_pidls: Vec<*const windows::Win32::UI::Shell::ITEMIDLIST> = Vec::new();
                // For single-parent case, we can use the bound child; for others, parse file name
                // Simpler: only handle single file selection for now (Explorer++ single file menu is most common)
                // For multi, we try to collect
                if paths.len() == 1 {
                    child_pidls.push(pidl_child);
                } else {
                    child_pidls.push(pidl_child);
                    for p in paths.iter().skip(1) {
                        let name = p.file_name().and_then(|n| n.to_str()).ok_or("bad filename")?;
                        let name_w: Vec<u16> = to_wide(name);
                        let mut pidl: *mut windows::Win32::UI::Shell::ITEMIDLIST = std::ptr::null_mut();
                        let hrp = psf.ParseDisplayName(HWND(std::ptr::null_mut()), None, PCWSTR(name_w.as_ptr()), None, &mut pidl, None);
                        if hrp.is_ok() && !pidl.is_null() { child_pidls.push(pidl as *const _); }
                    }
                }
                let mut pcm: Option<IContextMenu> = None;
                let hr3 = psf.GetUIObjectOf(HWND(std::ptr::null_mut()), child_pidls.len() as u32, child_pidls.as_ptr(), &IContextMenu::IID, None, &mut pcm as *mut _ as *mut _);
                if hr3.is_err() || pcm.is_none() {
                    if !pidl_parent.is_null() { windows::Win32::System::Com::CoTaskMemFree(Some(pidl_parent as *const _)); }
                    return Err(format!("GetUIObjectOf IContextMenu failed: {hr3:?}"));
                }
                let cm = pcm.unwrap();
                let hmenu = windows::Win32::UI::WindowsAndMessaging::CreatePopupMenu().map_err(|e| format!("{e:?}"))?;
                let hr4 = cm.QueryContextMenu(hmenu, 0, 1, 0x7FFF, 0);
                if hr4.is_err() {
                    windows::Win32::UI::WindowsAndMessaging::DestroyMenu(hmenu);
                    windows::Win32::System::Com::CoTaskMemFree(Some(pidl_parent as *const _));
                    return Err(format!("QueryContextMenu failed: {hr4:?}"));
                }
                let mut pt = POINT { x: 0, y: 0 };
                let _ = GetCursorPos(&mut pt);
                let hwnd = GetForegroundWindow();
                // Make foreground for TrackPopupMenuEx to work
                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
                let cmd = TrackPopupMenuEx(hmenu, TPM_RETURNCMD.0 | TPM_RIGHTBUTTON.0 | TPM_LEFTALIGN.0 | TPM_TOPALIGN.0, pt.x, pt.y, hwnd, None);
                windows::Win32::UI::WindowsAndMessaging::DestroyMenu(hmenu);
                if cmd != 0 {
                    let mut cmi: CMINVOKECOMMANDINFO = std::mem::zeroed();
                    cmi.cbSize = std::mem::size_of::<CMINVOKECOMMANDINFO>() as u32;
                    cmi.hwnd = hwnd;
                    cmi.lpVerb = windows::core::PCSTR((cmd as usize - 1) as *const u8);
                    cmi.nShow = windows::Win32::UI::WindowsAndMessaging::SW_NORMAL.0 as i32;
                    let hr5 = cm.InvokeCommand(&cmi);
                    if hr5.is_err() { /* some verbs fail but not fatal */ }
                }
                if !pidl_parent.is_null() { windows::Win32::System::Com::CoTaskMemFree(Some(pidl_parent as *const _)); }
                // Free extra pidls (except the bound one which is inside parent)
                for &p in child_pidls.iter().skip(1) { if !p.is_null() { windows::Win32::System::Com::CoTaskMemFree(Some(p as *const _)); } }
                Ok(())
            })();
            if need_uninit { CoUninitialize(); }
            result
        }
    }
}

#[cfg(windows)]
pub use win_impl::{icon_rgba, show_context_menu};
#[cfg(not(windows))]
pub fn icon_rgba(_path: &Path, _size: i32) -> Option<egui::ColorImage> { None }
#[cfg(not(windows))]
pub fn show_context_menu(_paths: &[PathBuf]) -> Result<(), String> { Err("not windows".into()) }

/// Simple icon cache keyed by extension / dir flag. Keeps egui textures.
pub struct IconCache {
    map: HashMap<String, egui::TextureHandle>,
}
impl IconCache {
    pub fn new() -> Self { Self { map: HashMap::new() } }
    fn key_for(path: &Path, is_dir: bool) -> String {
        if is_dir { return "__dir__".into(); }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if ext.is_empty() { "__file__".into() } else { ext }
    }
    pub fn get(&mut self, ctx: &egui::Context, path: &Path, is_dir: bool) -> Option<egui::TextureHandle> {
        let key = Self::key_for(path, is_dir);
        if let Some(h) = self.map.get(&key) { return Some(h.clone()); }
        // Try OS icon
        if let Some(img) = icon_rgba(path, 16) {
            let tex = ctx.load_texture(format!("icon:{key}"), img, egui::TextureOptions::LINEAR);
            self.map.insert(key.clone(), tex.clone());
            return Some(tex);
        }
        None
    }
    pub fn clear(&mut self) { self.map.clear(); }
}
