//! IME-safe typed input for filter / typeahead.
//!
//! Context7 (egui-winit):
//! `on_keyboard_input` pushes `Event::Key` and, for printable text when
//! `pressed && !is_cmd`, also `Event::Text`. Japanese IME commit arrives as
//! `Event::Text`, so Text must be the primary source.
//! https://github.com/emilk/egui/blob/main/crates/egui-winit/src/lib.rs

use eframe::egui;
use std::time::{Duration, Instant};

/// Characters typed this frame (Text first, then Key fallback).
#[derive(Debug, Default, Clone)]
pub struct FrameTyped {
    pub text: String,
    pub backspace: bool,
}

/// What the app should do after processing unfocused typing.
#[derive(Debug, Clone)]
pub enum TypeaheadAction {
    None,
    /// Clear typeahead buffer and optionally set status.
    ClearTypeahead { status: Option<String> },
    /// Append to filter, focus filter field, refresh list.
    RouteToFilter { typed: String },
    /// Request repaint after timeout check.
    RepaintAfter(Duration),
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
pub fn should_route_to_filter(wants_keyboard: bool, typed: &str) -> bool {
    !wants_keyboard && !typed.is_empty()
}

/// Decide next action for unfocused keyboard input (IME-safe filter unify).
///
/// Call only when no dialog owns the UI. Pass `wants_keyboard` from
/// `ctx.wants_keyboard_input()` — when true, TextEdit owns input.
pub fn decide_action(
    wants_keyboard: bool,
    frame: &FrameTyped,
    typeahead: &str,
    typeahead_at: Option<Instant>,
) -> TypeaheadAction {
    if wants_keyboard {
        return TypeaheadAction::None;
    }

    if frame.backspace && !typeahead.is_empty() {
        // Caller pops one char; we only signal clear-when-empty via status path.
        // Prefer filter routing for new input; backspace on legacy typeahead clears.
        return TypeaheadAction::None;
    }

    if !frame.text.is_empty() {
        // Spec B + Context7 Text-first: unfocused printable → filter
        return TypeaheadAction::RouteToFilter {
            typed: frame.text.clone(),
        };
    }

    if let Some(at) = typeahead_at {
        if at.elapsed().as_millis() > 1500 {
            return TypeaheadAction::ClearTypeahead { status: None };
        }
        return TypeaheadAction::RepaintAfter(Duration::from_millis(100));
    }

    TypeaheadAction::None
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

    #[test]
    fn decide_routes_ime_text_to_filter() {
        let frame = FrameTyped {
            text: "てすと".into(),
            backspace: false,
        };
        match decide_action(false, &frame, "", None) {
            TypeaheadAction::RouteToFilter { typed } => assert_eq!(typed, "てすと"),
            other => panic!("expected RouteToFilter, got {other:?}"),
        }
    }

    #[test]
    fn decide_defers_when_textedit_focused() {
        let frame = FrameTyped {
            text: "あ".into(),
            backspace: false,
        };
        assert!(matches!(
            decide_action(true, &frame, "", None),
            TypeaheadAction::None
        ));
    }

    #[test]
    fn decide_clears_stale_typeahead() {
        let frame = FrameTyped::default();
        let old = Instant::now() - Duration::from_secs(3);
        assert!(matches!(
            decide_action(false, &frame, "x", Some(old)),
            TypeaheadAction::ClearTypeahead { .. }
        ));
    }
}
