use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_RECENT: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub last_path: Option<PathBuf>,
    pub bookmarks: Vec<Bookmark>,
    /// Recently visited directories (most recent first).
    pub recent_paths: Vec<PathBuf>,
    pub show_hidden: bool,
    pub show_preview: bool,
    pub dual_pane: bool,
    pub sort_by: SortBy,
    pub sort_desc: bool,
    pub theme_dark: bool,
    /// Font family preset for Japanese / UI text.
    pub font_preset: FontPreset,
    /// Optional path to a custom .ttf / .otf / .ttc file.
    pub font_custom_path: Option<String>,
    /// Base UI font size in points (Body).
    pub font_size: f32,
    /// Row height scale for the file list (0.85–1.35).
    pub row_height_scale: f32,
    /// Compact toolbar / denser spacing.
    pub compact_ui: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FontPreset {
    /// BIZ UD Gothic — default for Japanese Windows, highly legible UI font.
    #[default]
    BizUdGothic,
    /// BIZ UDP Gothic variant (proportional)
    BizUdpGothic,
    /// Prefer system Japanese fonts automatically.
    Auto,
    YuGothic,
    Meiryo,
    YuMincho,
    MsGothic,
    NotoSansCjk,
    /// Use `font_custom_path` only (plus system CJK fallback if available).
    Custom,
    /// egui built-in fonts only (no CJK).
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SortBy {
    #[default]
    Name,
    Size,
    Modified,
    Type,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_path: None,
            bookmarks: vec![],
            recent_paths: vec![],
            show_hidden: false,
            show_preview: true,
            dual_pane: false,
            sort_by: SortBy::Name,
            sort_desc: false,
            theme_dark: true,
            font_preset: FontPreset::BizUdGothic,
            font_custom_path: None,
            font_size: 14.0,
            row_height_scale: 1.0,
            compact_ui: false,
        }
    }
}

impl AppConfig {
    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("explorer-rs").join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(p) = Self::path() {
            if let Ok(s) = std::fs::read_to_string(&p) {
                if let Ok(mut c) = serde_json::from_str::<AppConfig>(&s) {
                    if c.font_size < 10.0 || c.font_size > 28.0 {
                        c.font_size = 14.0;
                    }
                    if c.row_height_scale < 0.8 || c.row_height_scale > 1.5 {
                        c.row_height_scale = 1.0;
                    }
                    // Keep existing directories only, preserving most-recent order.
                    let mut seen = std::collections::HashSet::new();
                    c.recent_paths
                        .retain(|p| p.is_dir() && seen.insert(p.clone()));
                    c.recent_paths.truncate(MAX_RECENT);
                    return c;
                }
            }
        }
        let mut c = Self::default();
        if let Some(home) = dirs::home_dir() {
            c.bookmarks.push(Bookmark {
                name: "ホーム".into(),
                path: home,
            });
        }
        if let Some(d) = dirs::desktop_dir() {
            c.bookmarks.push(Bookmark {
                name: "デスクトップ".into(),
                path: d,
            });
        }
        if let Some(d) = dirs::document_dir() {
            c.bookmarks.push(Bookmark {
                name: "ドキュメント".into(),
                path: d,
            });
        }
        if let Some(d) = dirs::download_dir() {
            c.bookmarks.push(Bookmark {
                name: "ダウンロード".into(),
                path: d,
            });
        }
        #[cfg(windows)]
        {
            for d in ["C:\\", "D:\\", "E:\\"] {
                let p = PathBuf::from(d);
                if p.exists() && !c.bookmarks.iter().any(|b| b.path == p) {
                    c.bookmarks.push(Bookmark {
                        name: d.into(),
                        path: p,
                    });
                }
            }
        }
        c
    }

    pub fn save(&self) -> Result<(), String> {
        let p = Self::path().ok_or_else(|| "設定ディレクトリを解決できません".to_string())?;
        let parent = p
            .parent()
            .ok_or_else(|| "設定パスが不正です".to_string())?;
        std::fs::create_dir_all(parent).map_err(|e| format!("設定フォルダ作成失敗: {e}"))?;
        let s = serde_json::to_string_pretty(self).map_err(|e| format!("設定シリアライズ失敗: {e}"))?;
        std::fs::write(&p, s).map_err(|e| format!("設定書き込み失敗: {e}"))?;
        Ok(())
    }

    /// Push a directory to the front of recent_paths (deduped, capped).
    pub fn push_recent(&mut self, path: PathBuf) {
        if !path.is_dir() {
            return;
        }
        self.recent_paths.retain(|p| p != &path);
        self.recent_paths.insert(0, path);
        self.recent_paths.truncate(MAX_RECENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sort() {
        assert_eq!(SortBy::default(), SortBy::Name);
    }

    #[test]
    fn default_font_is_biz_ud_gothic() {
        assert_eq!(AppConfig::default().font_preset, FontPreset::BizUdGothic);
    }

    #[test]
    fn roundtrip_json() {
        let c = AppConfig {
            last_path: Some(PathBuf::from("/tmp")),
            bookmarks: vec![Bookmark {
                name: "t".into(),
                path: PathBuf::from("/tmp"),
            }],
            recent_paths: vec![PathBuf::from("/tmp")],
            show_hidden: true,
            show_preview: false,
            dual_pane: true,
            sort_by: SortBy::Size,
            sort_desc: true,
            theme_dark: false,
            font_preset: FontPreset::BizUdGothic,
            font_custom_path: Some("C:\\Fonts\\x.ttf".into()),
            font_size: 16.0,
            row_height_scale: 1.1,
            compact_ui: true,
        };
        let s = serde_json::to_string(&c).unwrap();
        let back: AppConfig = serde_json::from_str(&s).unwrap();
        assert!(back.show_hidden);
        assert!(back.dual_pane);
        assert_eq!(back.sort_by, SortBy::Size);
        assert!(!back.theme_dark);
        assert_eq!(back.font_preset, FontPreset::BizUdGothic);
        assert_eq!(back.font_size, 16.0);
        assert!(back.compact_ui);
        assert_eq!(back.recent_paths.len(), 1);
    }

    #[test]
    fn old_config_without_font_fields_deserializes() {
        let s = r#"{"last_path":null,"bookmarks":[],"show_hidden":false,"show_preview":true,"dual_pane":false,"sort_by":"Name","sort_desc":false,"theme_dark":true}"#;
        let c: AppConfig = serde_json::from_str(s).unwrap();
        assert_eq!(c.font_preset, FontPreset::BizUdGothic);
        assert_eq!(c.font_size, 14.0);
        assert!(c.recent_paths.is_empty());
        assert!((c.row_height_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn push_recent_dedupes_and_caps() {
        let mut c = AppConfig::default();
        // Use temp dirs that exist
        let base = std::env::temp_dir();
        c.push_recent(base.clone());
        c.push_recent(base.clone());
        assert_eq!(c.recent_paths.len(), 1);
        assert_eq!(c.recent_paths[0], base);
    }
}
