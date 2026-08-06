//! Japanese / CJK font loading and runtime font switching for egui.
//!
//! egui's default fonts do not include CJK glyphs, so Japanese UI text and
//! file names would show as ``□``. We load system fonts (Windows: Yu Gothic /
//! Meiryo, Linux: Noto CJK, macOS: Hiragino) and put them first in the
//! Proportional / Monospace fallback chains.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId, TextStyle};

use crate::config::{AppConfig, FontPreset};

const CJK_FONT_KEY: &str = "explorer_cjk";
const CUSTOM_FONT_KEY: &str = "explorer_custom";

/// Candidate system font files for Japanese, ordered by preference per OS.
fn system_cjk_candidates(preset: FontPreset) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    match preset {
        FontPreset::Default => return paths,
        FontPreset::Custom => return paths,
        FontPreset::Auto => {
            #[cfg(windows)]
            {
                paths.extend(windows_font_dirs().into_iter().flat_map(|dir| {
                    [
                        dir.join("YuGothM.ttc"),
                        dir.join("YuGothR.ttc"),
                        dir.join("meiryo.ttc"),
                        dir.join("msgothic.ttc"),
                        dir.join("msmincho.ttc"),
                        dir.join("malgun.ttf"), // Korean fallback
                        dir.join("msyh.ttc"),   // Chinese fallback
                    ]
                }));
            }
            #[cfg(target_os = "macos")]
            {
                paths.push(PathBuf::from(
                    "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
                ));
                paths.push(PathBuf::from("/System/Library/Fonts/Hiragino Sans GB.ttc"));
                paths.push(PathBuf::from("/Library/Fonts/Arial Unicode.ttf"));
                paths.push(PathBuf::from("/System/Library/Fonts/AppleSDGothicNeo.ttc"));
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                for p in [
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf",
                    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
                ] {
                    paths.push(PathBuf::from(p));
                }
            }
        }
        FontPreset::YuGothic => {
            #[cfg(windows)]
            {
                for dir in windows_font_dirs() {
                    paths.push(dir.join("YuGothM.ttc"));
                    paths.push(dir.join("YuGothR.ttc"));
                }
            }
        }
        FontPreset::Meiryo => {
            #[cfg(windows)]
            {
                for dir in windows_font_dirs() {
                    paths.push(dir.join("meiryo.ttc"));
                    paths.push(dir.join("meiryob.ttc"));
                }
            }
        }
        FontPreset::YuMincho => {
            #[cfg(windows)]
            {
                for dir in windows_font_dirs() {
                    paths.push(dir.join("yumin.ttf"));
                    paths.push(dir.join("YuMincho.ttc"));
                    paths.push(dir.join("msmincho.ttc"));
                }
            }
        }
        FontPreset::MsGothic => {
            #[cfg(windows)]
            {
                for dir in windows_font_dirs() {
                    paths.push(dir.join("msgothic.ttc"));
                }
            }
        }
        FontPreset::NotoSansCjk => {
            for p in [
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                "C:\\Windows\\Fonts\\NotoSansCJKjp-Regular.otf",
                "C:\\Windows\\Fonts\\NotoSansJP-Regular.otf",
            ] {
                paths.push(PathBuf::from(p));
            }
            #[cfg(windows)]
            {
                for dir in windows_font_dirs() {
                    paths.push(dir.join("NotoSansCJKjp-Regular.otf"));
                    paths.push(dir.join("NotoSansJP-Regular.otf"));
                }
            }
        }
    }

    paths
}

#[cfg(windows)]
fn windows_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
    dirs.push(PathBuf::from(r"C:\WINDOWS\Fonts"));
    dirs
}

fn load_font_bytes(path: &Path) -> Option<Vec<u8>> {
    if !path.is_file() {
        return None;
    }
    std::fs::read(path).ok().filter(|b| b.len() > 1000)
}

fn find_first_font(candidates: &[PathBuf]) -> Option<(PathBuf, Vec<u8>)> {
    for p in candidates {
        if let Some(bytes) = load_font_bytes(p) {
            return Some((p.clone(), bytes));
        }
    }
    None
}

/// Build FontDefinitions from config and apply to context.
/// Returns a short status message for the UI.
pub fn apply_fonts(ctx: &egui::Context, config: &AppConfig) -> String {
    let mut fonts = FontDefinitions::default();
    let mut loaded_name = String::from("egui default (CJK なし)");

    // 1) Optional user custom font (highest priority when preset is Custom or path set)
    let custom_path = config
        .font_custom_path
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty());

    let mut primary_key: Option<String> = None;

    if let Some(ref path) = custom_path {
        if let Some(bytes) = load_font_bytes(path) {
            fonts.font_data.insert(
                CUSTOM_FONT_KEY.to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            primary_key = Some(CUSTOM_FONT_KEY.to_owned());
            loaded_name = format!(
                "カスタム: {}",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("font")
            );
        }
    }

    // 2) System CJK for Auto / named presets (and as fallback even with custom)
    let preset = if config.font_preset == FontPreset::Custom && primary_key.is_some() {
        FontPreset::Auto // still load system CJK as secondary fallback
    } else {
        config.font_preset
    };

    if config.font_preset != FontPreset::Default {
        let candidates = system_cjk_candidates(if primary_key.is_some() {
            FontPreset::Auto
        } else {
            preset
        });
        if let Some((path, bytes)) = find_first_font(&candidates) {
            fonts.font_data.insert(
                CJK_FONT_KEY.to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            if primary_key.is_none() {
                primary_key = Some(CJK_FONT_KEY.to_owned());
                loaded_name = format!(
                    "{}",
                    path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("system CJK")
                );
            }
        } else if primary_key.is_none() {
            loaded_name = "日本語フォント未検出 (□ になる場合あり)".into();
        }
    }

    // Insert primary (+ CJK fallback) at the front of both families
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let list = fonts.families.entry(family).or_default();
        // Remove previous keys if re-applying
        list.retain(|n| n != CJK_FONT_KEY && n != CUSTOM_FONT_KEY);
        if fonts.font_data.contains_key(CJK_FONT_KEY) {
            list.insert(0, CJK_FONT_KEY.to_owned());
        }
        if fonts.font_data.contains_key(CUSTOM_FONT_KEY) {
            list.insert(0, CUSTOM_FONT_KEY.to_owned());
        }
    }

    ctx.set_fonts(fonts);
    apply_text_styles(ctx, config.font_size);
    format!("フォント: {loaded_name}")
}

pub fn apply_text_styles(ctx: &egui::Context, size: f32) {
    let size = size.clamp(10.0, 28.0);
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new((size * 0.85).max(9.0), FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(size, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(size, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(size * 1.35, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(size, FontFamily::Monospace),
        ),
    ]
    .into();
    style.interaction.selectable_labels = false;
    ctx.set_style(style);
}

/// Human-readable labels for the font preset combo.
pub fn preset_label(p: FontPreset) -> &'static str {
    match p {
        FontPreset::Auto => "自動 (日本語優先)",
        FontPreset::YuGothic => "游ゴシック",
        FontPreset::Meiryo => "メイリオ",
        FontPreset::YuMincho => "游明朝",
        FontPreset::MsGothic => "ＭＳ ゴシック",
        FontPreset::NotoSansCjk => "Noto Sans CJK",
        FontPreset::Default => "egui 標準 (日本語なし)",
        FontPreset::Custom => "カスタムファイル…",
    }
}

pub fn all_presets() -> &'static [FontPreset] {
    &[
        FontPreset::Auto,
        FontPreset::YuGothic,
        FontPreset::Meiryo,
        FontPreset::YuMincho,
        FontPreset::MsGothic,
        FontPreset::NotoSansCjk,
        FontPreset::Custom,
        FontPreset::Default,
    ]
}
