//! Japanese / CJK font loading and runtime font switching for egui.
//! Default is BIZ UD Gothic (Windows standard, highly readable for file names).

use std::path::{Path, PathBuf};
use eframe::egui::{FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use crate::config::{AppConfig, FontPreset};

const CJK_FONT_KEY: &str = "explorer_cjk";
const CUSTOM_FONT_KEY: &str = "explorer_custom";

/// Candidate system font files for Japanese, ordered by preference per OS.
fn system_cjk_candidates(preset: FontPreset) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    match preset {
        FontPreset::BizUdGothic => {
            #[cfg(windows)]
            for dir in windows_font_dirs() {
                for f in ["BIZ-UDGothicB.ttc", "BIZ-UDGothicR.ttc", "BIZUDGothic-Regular.ttf", "BIZUDGothic-Bold.ttf", "BIZ-UDGothic.ttf", "BIZUDGothic.ttf"] { paths.push(dir.join(f)); }
            }
            #[cfg(not(windows))]
            for p in ["/usr/share/fonts/truetype/bizud-gothic/BIZUDGothic-Regular.ttf", "/usr/share/fonts/opentype/bizud/BIZUDGothic-Regular.ttf"] { paths.push(PathBuf::from(p)); }
        }
        FontPreset::BizUdpGothic => {
            #[cfg(windows)]
            for dir in windows_font_dirs() {
                for f in ["BIZ-UDPGothicB.ttc", "BIZ-UDPGothicR.ttc", "BIZUDP Gothic.ttf", "BIZUDPGothic-Regular.ttf", "BIZ-UDPGothic.ttf"] { paths.push(dir.join(f)); }
                paths.push(dir.join("BIZ-UDPGothic.ttc"));
            }
        }
        FontPreset::Default | FontPreset::Custom => return paths,
        FontPreset::Auto => {
            #[cfg(windows)]
            for dir in windows_font_dirs() {
                for f in [
                    "BIZ-UDGothicR.ttc", "BIZUDGothic-Regular.ttf", "BIZ-UDGothic.ttf",
                    "BIZ-UDPGothicR.ttc", "BIZUDPGothic-Regular.ttf",
                    "YuGothM.ttc", "YuGothR.ttc", "meiryo.ttc", "msgothic.ttc", "msmincho.ttc", "yumin.ttf", "YuMincho.ttc"
                ] { paths.push(dir.join(f)); }
            }
            #[cfg(target_os = "macos")]
            {
                paths.push(PathBuf::from("/System/Library/Fonts/Hiragino Sans GB.ttc"));
                paths.push(PathBuf::from("/Library/Fonts/Arial Unicode.ttf"));
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            for p in ["/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf", "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc"] { paths.push(PathBuf::from(p)); }
        }
        FontPreset::YuGothic => { #[cfg(windows)] for dir in windows_font_dirs() { paths.push(dir.join("YuGothM.ttc")); paths.push(dir.join("YuGothR.ttc")); } }
        FontPreset::Meiryo => { #[cfg(windows)] for dir in windows_font_dirs() { paths.push(dir.join("meiryo.ttc")); paths.push(dir.join("meiryob.ttc")); } }
        FontPreset::YuMincho => { #[cfg(windows)] for dir in windows_font_dirs() { paths.push(dir.join("yumin.ttf")); paths.push(dir.join("YuMincho.ttc")); } }
        FontPreset::MsGothic => { #[cfg(windows)] for dir in windows_font_dirs() { paths.push(dir.join("msgothic.ttc")); } }
        FontPreset::NotoSansCjk => {
            for p in ["/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", "C:\\Windows\\Fonts\\NotoSansCJKjp-Regular.otf"] { paths.push(PathBuf::from(p)); }
            #[cfg(windows)] for dir in windows_font_dirs() { paths.push(dir.join("NotoSansCJKjp-Regular.otf")); }
        }
    }
    paths
}

#[cfg(windows)]
fn windows_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") { dirs.push(PathBuf::from(windir).join("Fonts")); }
    dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
    dirs
}

fn load_font_bytes(path: &Path) -> Option<Vec<u8>> {
    if !path.is_file() { return None; }
    std::fs::read(path).ok().filter(|b| b.len() > 1000)
}
fn find_first_font(candidates: &[PathBuf]) -> Option<(PathBuf, Vec<u8>)> {
    for p in candidates { if let Some(bytes) = load_font_bytes(p) { return Some((p.clone(), bytes)); } }
    None
}

pub fn apply_fonts(ctx: &eframe::egui::Context, config: &AppConfig) -> String {
    let mut fonts = FontDefinitions::default();
    let mut loaded_name = String::from("egui default (CJK なし)");
    let mut has_custom = false; let mut has_cjk = false;
    let custom_path = config.font_custom_path.as_ref().map(PathBuf::from).filter(|p| !p.as_os_str().is_empty());
    if let Some(ref path) = custom_path { if let Some(bytes) = load_font_bytes(path) { fonts.font_data.insert(CUSTOM_FONT_KEY.to_owned(), FontData::from_owned(bytes)); has_custom = true; loaded_name = format!("カスタム: {}", path.file_name().and_then(|s| s.to_str()).unwrap_or("font")); } }
    if config.font_preset != FontPreset::Default {
        let load_preset = if has_custom { FontPreset::Auto } else { config.font_preset };
        let candidates = system_cjk_candidates(load_preset);
        if let Some((path, bytes)) = find_first_font(&candidates) {
            fonts.font_data.insert(CJK_FONT_KEY.to_owned(), FontData::from_owned(bytes));
            has_cjk = true;
            if !has_custom { loaded_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("system CJK").to_string(); }
        } else if !has_custom { loaded_name = "日本語フォント未検出".into(); }
    }
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let list = fonts.families.entry(family).or_default();
        list.retain(|n| n != CJK_FONT_KEY && n != CUSTOM_FONT_KEY);
        if has_cjk { list.insert(0, CJK_FONT_KEY.to_owned()); }
        if has_custom { list.insert(0, CUSTOM_FONT_KEY.to_owned()); }
    }
    ctx.set_fonts(fonts);

    // Context7 (egui FullOutput / Context::tessellate):
    // pixels_per_point is used for feathering (anti-aliasing) when Tessellator::new(...)
    // builds triangle meshes. Ensure feathering is on so CJK glyphs at ~14pt are less jagged.
    // Do NOT call set_pixels_per_point here — eframe already follows the OS scale factor.
    // See: https://github.com/emilk/egui/blob/main/crates/egui/src/data/output.rs
    ctx.tessellation_options_mut(|o| {
        o.feathering = true;
    });

    apply_text_styles(ctx, config.font_size);
    format!("フォント: {loaded_name} (ppp={:.2})", ctx.pixels_per_point())
}

pub fn apply_text_styles(ctx: &eframe::egui::Context, size: f32) {
    let size = size.clamp(10.0, 28.0);
    let mut style = (*ctx.style()).clone();
    // Context7: Style::text_styles is BTreeMap<TextStyle, FontId>; keep Body/Button aligned with font_size.
    style.text_styles = [
        (TextStyle::Small, FontId::new((size * 0.85).max(9.0), FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(size, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(size, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(size * 1.35, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(size, FontFamily::Monospace)),
    ]
    .into();
    style.interaction.selectable_labels = false;
    ctx.set_style(style);
}

pub fn preset_label(p: FontPreset) -> &'static str {
    match p {
        FontPreset::BizUdGothic => "BIZ UDゴシック (標準)",
        FontPreset::BizUdpGothic => "BIZ UDPゴシック",
        FontPreset::Auto => "自動 (BIZ UD優先)",
        FontPreset::YuGothic => "游ゴシック", FontPreset::Meiryo => "メイリオ", FontPreset::YuMincho => "游明朝", FontPreset::MsGothic => "ＭＳ ゴシック", FontPreset::NotoSansCjk => "Noto Sans CJK", FontPreset::Default => "egui 標準 (日本語なし)", FontPreset::Custom => "カスタムファイル…",
    }
}
pub fn all_presets() -> &'static [FontPreset] { &[FontPreset::BizUdGothic, FontPreset::BizUdpGothic, FontPreset::Auto, FontPreset::YuGothic, FontPreset::Meiryo, FontPreset::YuMincho, FontPreset::MsGothic, FontPreset::NotoSansCjk, FontPreset::Custom, FontPreset::Default] }
