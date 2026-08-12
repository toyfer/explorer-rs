// Context7 wiring for 11c
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod commands;
mod config;
mod file_icons;
mod fonts;
mod fs_ops;
mod nat_sort;
mod shell;
mod tab;
mod toolbar_icons;
mod typeahead_input;
mod watch;

use app::ExplorerApp;
use eframe::egui;

fn main() -> eframe::Result {
    #[allow(unused_mut)]
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 780.0])
        .with_min_inner_size([900.0, 600.0])
        .with_title("explorer-rs — Explorer++ inspired")
        .with_drag_and_drop(true);
    #[cfg(windows)]
    {
        viewport = viewport.with_icon(load_icon());
    }
    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let config = config::AppConfig::load();
    let start_path = config.last_path.clone().unwrap_or_else(|| {
        dirs::home_dir().unwrap_or_else(|| {
            std::path::PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" })
        })
    });
    eframe::run_native(
        "explorer-rs",
        native_options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let font_status = fonts::apply_fonts(&cc.egui_ctx, &config);
            if config.theme_dark {
                cc.egui_ctx.set_visuals(egui::Visuals::dark());
            } else {
                cc.egui_ctx.set_visuals(egui::Visuals::light());
            }
            let mut app = ExplorerApp::new(start_path, config, cc);
            app.status = format!("準備完了 — {font_status}");
            Ok(Box::new(app))
        }),
    )
}

#[cfg(windows)]
fn load_icon() -> egui::IconData {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let (r, g, b, a) = icon_pixel(x, y);
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    egui::IconData {
        rgba,
        width: 32,
        height: 32,
    }
}

#[cfg(windows)]
fn icon_pixel(x: u32, y: u32) -> (u8, u8, u8, u8) {
    let is_tab = y < 6 && x >= 2 && x < 14;
    let is_body = y >= 6 && y < 28 && x >= 2 && x < 30;
    let is_border =
        (is_tab || is_body) && (x == 2 || x == 29 || y == 6 || y == 27 || (is_tab && y == 2));
    if is_tab || is_body {
        if is_border {
            return (30, 58, 138, 255);
        }
        let shade = if y < 12 {
            (59, 130, 246)
        } else {
            (37, 99, 235)
        };
        if y >= 12 && y <= 13 && x >= 6 && x < 26 {
            return (255, 255, 255, 230);
        }
        if y >= 16 && y <= 17 && x >= 6 && x < 26 {
            return (255, 255, 255, 230);
        }
        if y >= 20 && y <= 21 && x >= 6 && x < 20 {
            return (255, 255, 255, 230);
        }
        return (shade.0, shade.1, shade.2, 255);
    }
    (0, 0, 0, 0)
}
