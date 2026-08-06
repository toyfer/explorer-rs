use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub last_path: Option<PathBuf>,
    pub bookmarks: Vec<Bookmark>,
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FontPreset {
    /// Prefer system Japanese fonts automatically.
    #[default]
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
            show_hidden: false,
            show_preview: true,
            dual_pane: false,
            sort_by: SortBy::Name,
            sort_desc: false,
            theme_dark: true,
            font_preset: FontPreset::Auto,
            font_custom_path: None,
            font_size: 14.0,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sort() {
        assert_eq!(SortBy::default(), SortBy::Name);
    }

    #[test]
    fn roundtrip_json() {
        let c = AppConfig {
            last_path: Some(PathBuf::from("/tmp")),
            bookmarks: vec![Bookmark {
                name: "t".into(),
                path: PathBuf::from("/tmp"),
            }],
            show_hidden: true,
            show_preview: false,
            dual_pane: true,
            sort_by: SortBy::Size,
            sort_desc: true,
            theme_dark: false,
            font_preset: FontPreset::Meiryo,
            font_custom_path: Some("C:\\Fonts\\x.ttf".into()),
            font_size: 16.0,
        };
        let s = serde_json::to_string(&c).unwrap();
        let back: AppConfig = serde_json::from_str(&s).unwrap();
        assert!(back.show_hidden);
        assert!(back.dual_pane);
        assert_eq!(back.sort_by, SortBy::Size);
        assert!(!back.theme_dark);
        assert_eq!(back.font_preset, FontPreset::Meiryo);
        assert_eq!(back.font_size, 16.0);
    }

    #[test]
    fn old_config_without_font_fields_deserializes() {
        let s = r#"{"last_path":null,"bookmarks":[],"show_hidden":false,"show_preview":true,"dual_pane":false,"sort_by":"Name","sort_desc":false,"theme_dark":true}"#;
        let c: AppConfig = serde_json::from_str(s).unwrap();
        assert_eq!(c.font_preset, FontPreset::Auto);
        assert_eq!(c.font_size, 14.0);
    }
}
