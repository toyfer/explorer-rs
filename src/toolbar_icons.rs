//! Toolbar SVG icons (PR #11 slice B).
//!
//! Context7: egui_extras `all_loaders` includes SVG via resvg internally when
//! `Image::from_bytes` is used with a `.svg` URI. App already calls
//! `egui_extras::install_image_loaders` in main.
//!
//! Icons are MIT-style simple stroke SVGs (no third-party pack required for toolbar).

use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarIcon {
    Back,
    Forward,
    Up,
    Refresh,
    Home,
}

impl ToolbarIcon {
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Self::Back => include_bytes!("../assets/icons/toolbar/chevron_left.svg"),
            Self::Forward => include_bytes!("../assets/icons/toolbar/chevron_right.svg"),
            Self::Up => include_bytes!("../assets/icons/toolbar/arrow_up.svg"),
            Self::Refresh => include_bytes!("../assets/icons/toolbar/refresh.svg"),
            Self::Home => include_bytes!("../assets/icons/toolbar/home.svg"),
        }
    }

    pub fn uri(self) -> &'static str {
        match self {
            Self::Back => "bytes://toolbar/chevron_left.svg",
            Self::Forward => "bytes://toolbar/chevron_right.svg",
            Self::Up => "bytes://toolbar/arrow_up.svg",
            Self::Refresh => "bytes://toolbar/refresh.svg",
            Self::Home => "bytes://toolbar/home.svg",
        }
    }

    pub fn hover(self) -> &'static str {
        match self {
            Self::Back => "戻る",
            Self::Forward => "進む",
            Self::Up => "上へ Alt+↑",
            Self::Refresh => "更新 F5",
            Self::Home => "ホーム Alt+Home",
        }
    }

    /// egui Image sized for toolbar buttons.
    pub fn image(self, size: f32) -> egui::Image<'static> {
        egui::Image::from_bytes(self.uri().to_owned(), self.bytes()).fit_to_exact_size(egui::vec2(size, size))
    }
}

/// Draw an icon-only toolbar button. Returns whether it was clicked.
pub fn icon_button(ui: &mut egui::Ui, icon: ToolbarIcon, enabled: bool, compact: bool) -> bool {
    let size = if compact { 14.0 } else { 16.0 };
    let img = icon.image(size);
    let mut clicked = false;
    ui.add_enabled_ui(enabled, |ui| {
        let resp = ui.add(egui::Button::image(img)).on_hover_text(icon.hover());
        if resp.clicked() {
            clicked = true;
        }
    });
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_assets_nonempty() {
        for ic in [
            ToolbarIcon::Back,
            ToolbarIcon::Forward,
            ToolbarIcon::Up,
            ToolbarIcon::Refresh,
            ToolbarIcon::Home,
        ] {
            let b = ic.bytes();
            assert!(b.len() > 40, "{ic:?} svg too small");
            assert!(std::str::from_utf8(b).unwrap().contains("<svg"));
        }
    }
}
