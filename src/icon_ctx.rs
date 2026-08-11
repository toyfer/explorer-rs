//! Frame-scoped egui context holder so table body rows can load textures
//! without TableRow::ctx() (unavailable in egui_extras 0.28).

use std::cell::RefCell;
use eframe::egui;

thread_local! {
    static FRAME_CTX: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
}

pub fn set(ctx: egui::Context) {
    FRAME_CTX.with(|c| *c.borrow_mut() = Some(ctx));
}

pub fn clear() {
    FRAME_CTX.with(|c| *c.borrow_mut() = None);
}

pub fn get() -> Option<egui::Context> {
    FRAME_CTX.with(|c| c.borrow().clone())
}
