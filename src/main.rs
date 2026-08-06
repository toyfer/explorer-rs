#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod commands;
mod config;
mod fonts;
mod fs_ops;
mod tab;
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
    egui::IconData::default()
}
