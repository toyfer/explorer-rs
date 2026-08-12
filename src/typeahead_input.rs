//! IME-safe typed input collection for filter / typeahead.
//!
//! Context7 (egui-winit):
//! `on_keyboard_input` pushes `Event::Key` and, for printable text when
//! `pressed && !is_cmd`, also `Event::Text`. Japanese IME commit arrives as
//! `Event::Text`, so Text must be the primary source.
//! https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs

use eframe::egui;

/// Characters typed this frame (Text first, then Key fallback).
#[derive(Debug, Default, Clone)]
pub struct FrameTyped {
    pub text: String,
    pub backspace: bool,
}

/// Collect printable input for the current frame.
///
/// - Prefer `egui::Event::Text` (IME commit / composed characters).
/// - Fall back to bare `Key::A-Z` / digits only when no Text event fired.
/// - Ignore ctrl/cmd/alt modified keys on the Key fallback path.
pub fn collect_frame_typed(ctx: &egui::Context) -> FrameTyped {
    let mut out = FrameTyped::default();
    let mut has_text = false;

    ctx.input(|i| {
        for ev in &i.events {
            if let egui::Event::Key {
                key: egui::Key::Backspace,
                pressed: true,
                modifiers,
                ..
            } = ev
            {
                if !modifiers.ctrl && !modifiers.command && !modifiers.alt {
                    out.backspace = true;
                }
            }
        }

        for ev in &i.events {
            if let egui::Event::Text(t) = ev {
                if !t.is_empty() && t.chars().all(|c| !c.is_control()) {
                    out.text.push_str(t);
                    has_text = true;
                }
            }
        }

        if !has_text {
            for ev in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = ev
                {
                    if modifiers.ctrl || modifiers.command || modifiers.alt {
                        continue;
                    }
                    let ch = match key {
                        egui::Key::A => 'a',
                        egui::Key::B => 'b',
                        egui::Key::C => 'c',
                        egui::Key::D => 'd',
                        egui::Key::E => 'e',
                        egui::Key::F => 'f',
                        egui::Key::G => 'g',
                        egui::Key::H => 'h',
                        egui::Key::I => 'i',
                        egui::Key::J => 'j',
                        egui::Key::K => 'k',
                        egui::Key::L => 'l',
                        egui::Key::M => 'm',
                        egui::Key::N => 'n',
                        egui::Key::O => 'o',
                        egui::Key::P => 'p',
                        egui::Key::Q => 'q',
                        egui::Key::R => 'r',
                        egui::Key::S => 's',
                        egui::Key::T => 't',
                        egui::Key::U => 'u',
                        egui::Key::V => 'v',
                        egui::Key::W => 'w',
                        egui::Key::X => 'x',
                        egui::Key::Y => 'y',
                        egui::Key::Z => 'z',
                        egui::Key::Num0 => '0',
                        egui::Key::Num1 => '1',
                        egui::Key::Num2 => '2',
                        egui::Key::Num3 => '3',
                        egui::Key::Num4 => '4',
                        egui::Key::Num5 => '5',
                        egui::Key::Num6 => '6',
                        egui::Key::Num7 => '7',
                        egui::Key::Num8 => '8',
                        egui::Key::Num9 => '9',
                        egui::Key::Minus => '-',
                        egui::Key::Period => '.',
                        egui::Key::Slash => '/',
                        egui::Key::Backslash => '\\',
                        egui::Key::Comma => ',',
                        _ => continue,
                    };
                    out.text.push(ch);
                }
            }
        }
    });

    out
}

/// Whether unfocused typing should go to the filter bar (spec B).
///
/// When true: append `typed` to `filter`, request Filter focus, refresh list.
/// When false: either TextEdit owns input, or no printable text this frame.
pub fn should_route_to_filter(wants_keyboard: bool, typed: &str) -> bool {
    !wants_keyboard && !typed.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_to_filter_when_unfocused_and_typed() {
        assert!(should_route_to_filter(false, "てすと"));
        assert!(should_route_to_filter(false, "a"));
        assert!(!should_route_to_filter(true, "てすと"));
        assert!(!should_route_to_filter(false, ""));
    }
}
