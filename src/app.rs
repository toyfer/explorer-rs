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
pub struct ExplorerApp { pub tabs: Vec<Tab>, pub active: usize, pub pane2: Option<Tab>, pub active_pane: u8, pub address: String, pub search_query: String, pub search_results: Vec<FileEntry>, pub searching: bool, pub pasting: bool, pub status: String, pub config: AppConfig, pub clipboard: Option<Clipboard>, pub show_hidden: bool, pub dual_pane: bool, pub preview_text: String, pub rename_buffer: String, pub renaming: bool, pub new_folder_name: String, pub show_new_folder_dialog: bool, pub last_error: Option<String>, pub confirm_delete: Option<ConfirmDelete>, pub font_path_edit: String, typeahead: String, typeahead_at: Option<Instant>, watcher: Option<watch::WatchedDir>, watcher2: Option<watch::WatchedDir>, list_gen: u64, list_gen_p2: u64, listing: bool, listing_p2: bool, bg_tx: BgSender, bg_rx: BgReceiver, icon_cache: RefCell<HashMap<String, egui::TextureHandle>>, focus_request: Option<FocusTarget>, scroll_to_row: Option<usize>, rename_focus_once: bool, new_folder_focus_once: bool, pending_clipboard_text: Option<String>, }
#[derive(Clone, Copy)] enum FocusTarget { Address, Search, Filter, }
#[derive(Clone)] pub struct ConfirmDelete { pub paths: Vec<PathBuf>, pub permanent: bool, }
impl ExplorerApp {
fn current_tab(&self) -> &Tab { if self.active_pane==1 { if let Some(p)=&self.pane2 { return p; } } &self.tabs[self.active] }
fn current_tab_mut(&mut self) -> &mut Tab { if self.active_pane==1 { if let Some(p)=self.pane2.as_mut() { return p; } } &mut self.tabs[self.active] }
fn sync_address(&mut self){ self.address=self.current_tab().current.display().to_string(); }
fn note_navigation(&mut self){ let cur=self.current_tab().current.clone(); self.config.push_recent(cur); }
fn handle_typeahead(&mut self, ctx: &egui::Context) {
    // Context7 egui-winit: on_keyboard_input splits Text vs Key, printable Text is !is_cmd && pressed
    // Spec B: focusなしで日本語が来たらフィルタへ直行, wants_keyboard_input==trueなら譲る
    if self.renaming || self.show_new_folder_dialog || self.confirm_delete.is_some() { return; }
    let is_typing = ctx.wants_keyboard_input();
    // 既にフィルタ/アドレス等がフォーカスを持っていれば typeahead は発火しない — 自然に TextEdit が入力を持つ
    let mut typed = String::new();
    let mut has_text = false;
    let mut backspace=false;
    ctx.input(|i|{ for ev in &i.events { if let egui::Event::Key{key: egui::Key::Backspace, pressed:true, modifiers, ..}=ev { if !modifiers.ctrl && !modifiers.command && !modifiers.alt { backspace=true; } } } });
    if backspace && !self.typeahead.is_empty() { self.typeahead.pop(); self.typeahead_at=Some(Instant::now()); if self.typeahead.is_empty(){ self.status="検索クリア".into(); ctx.request_repaint(); return; } } else if backspace { return; }
    ctx.input(|i|{
        for ev in &i.events { if let egui::Event::Text(t)=ev { if !t.is_empty() && t.chars().all(|c|!c.is_control()) { typed.push_str(t); has_text=true; } } }
        if !has_text { for ev in &i.events { if let egui::Event::Key{key, pressed:true, modifiers, ..}=ev { if modifiers.ctrl||modifiers.command||modifiers.alt { continue; } let ch=match key{ egui::Key::A=>'a', egui::Key::B=>'b', egui::Key::C=>'c', egui::Key::D=>'d', egui::Key::E=>'e', egui::Key::F=>'f', egui::Key::G=>'g', egui::Key::H=>'h', egui::Key::I=>'i', egui::Key::J=>'j', egui::Key::K=>'k', egui::Key::L=>'l', egui::Key::M=>'m', egui::Key::N=>'n', egui::Key::O=>'o', egui::Key::P=>'p', egui::Key::Q=>'q', egui::Key::R=>'r', egui::Key::S=>'s', egui::Key::T=>'t', egui::Key::U=>'u', egui::Key::V=>'v', egui::Key::W=>'w', egui::Key::X=>'x', egui::Key::Y=>'y', egui::Key::Z=>'z', egui::Key::Num0=>'0', egui::Key::Num1=>'1', egui::Key::Num2=>'2', egui::Key::Num3=>'3', egui::Key::Num4=>'4', egui::Key::Num5=>'5', egui::Key::Num6=>'6', egui::Key::Num7=>'7', egui::Key::Num8=>'8', egui::Key::Num9=>'9', egui::Key::Minus=>'-', egui::Key::Period=>'.', _=>continue, } ; typed.push(ch); has_text=true; } } }
    });
    if typed.is_empty() && !backspace { if let Some(at)=self.typeahead_at { if at.elapsed().as_millis()>1500 { self.clear_typeahead(); } else { ctx.request_repaint_after(std::time::Duration::from_millis(100)); } } return; }
    // フィルタへ直行: focusなしで printable が来たら typeahead ではなく filter に入れる (Context7 Text優先)
    if !is_typing && !typed.is_empty() {
        // 日本語IMEの確定文字もここで Text として来る
        self.focus_request = Some(FocusTarget::Filter);
        let tab = self.current_tab_mut();
        tab.filter.push_str(&typed);
        self.request_refresh_async(false);
        self.status = format!("フィルタ: '{}'", tab.filter);
        ctx.request_repaint();
        return;
    }
    if is_typing { return; }
    // 以下は従来の typeahead 継続 (現在はフィルタ優先のため到達しにくいが互換で残す)
    if !typed.is_empty() { if let Some(at)=self.typeahead_at { if at.elapsed().as_millis()>1500 { self.typeahead.clear(); } } self.typeahead.push_str(&typed); self.typeahead_at=Some(Instant::now()); } else { self.typeahead_at=Some(Instant::now()); }
    let q=self.typeahead.to_lowercase(); let entries=self.current_tab().entries.clone(); if entries.is_empty(){ return; }
    let start=self.current_tab().focus.map(|f|f+1).unwrap_or(0); let mut found=None; for o in 0..entries.len(){ let idx=(start+o)%entries.len(); if entries[idx].name.to_lowercase().starts_with(&q){ found=Some(idx); break; } } if found.is_none(){ for (idx,e) in entries.iter().enumerate(){ if e.name.to_lowercase().contains(&q){ found=Some(idx); break; } } }
    if let Some(idx)=found{ self.current_tab_mut().select_only(idx); self.scroll_to_row=Some(idx); self.update_preview(); self.status=format!("インクリメント検索: '{}' → {}", self.typeahead, entries[idx].name); } else { self.status=format!("該当なし: '{}'", self.typeahead); } ctx.request_repaint();
}
fn clear_typeahead(&mut self){ self.typeahead.clear(); self.typeahead_at=None; }
fn get_or_load_icon(&self, ctx:&egui::Context, path:&Path, is_dir:bool)->Option<egui::TextureHandle>{ let key=Self::icon_cache_key(path,is_dir); if let Some(h)=self.icon_cache.borrow().get(&key){ return Some(h.clone()); } #[cfg(windows)]{ if let Some((rgba,w,h))=shell::icon_rgba(path,is_dir){ if w>0&&h>0&&rgba.len()==(w*h*4) as usize{ let img=egui::ColorImage::from_rgba_unmultiplied([w as usize,h as usize],&rgba); let tex=ctx.load_texture(key.clone(),img,egui::TextureOptions::LINEAR); self.icon_cache.borrow_mut().insert(key.clone(),tex.clone()); return Some(tex); } } } let _=(ctx,path,is_dir); None }
fn icon_cache_key(path:&Path,is_dir:bool)->String{ if is_dir{ return "__dir__".to_string(); } let ext=path.extension().and_then(|s|s.to_str()).unwrap_or("").to_lowercase(); if ext=="exe"||ext=="ico"||ext=="lnk"{ return format!("file:{}",path.display()); } format!("ext:{ext}") }
}
