//! Window chrome: which of close, minimize, and maximize to show.
//!
//! The buttons are drawn as red, yellow, and green circles at the top of the
//! sidebar, but which ones appear follows the desktop's `gtk-decoration-layout`
//! setting, so GNOME's default (close only) and a user who enabled minimize and
//! maximize both get what they chose.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowButton {
    Close,
    Minimize,
    Maximize,
}

/// Buttons named in a `gtk-decoration-layout` value such as `appmenu:close` or
/// `:minimize,maximize,close`, in the fixed order close, minimize, maximize.
/// Names this app has no use for (`icon`, `menu`, `appmenu`) are ignored, and
/// close is always shown when the layout names none, so a window can always be closed.
pub fn buttons_from_layout(layout: &str) -> Vec<WindowButton> {
    let named = |name: &str| layout.split([':', ',']).any(|token| token.trim() == name);
    let mut out = Vec::new();
    if named("close") {
        out.push(WindowButton::Close);
    }
    if named("minimize") {
        out.push(WindowButton::Minimize);
    }
    if named("maximize") {
        out.push(WindowButton::Maximize);
    }
    if out.is_empty() {
        out.push(WindowButton::Close);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use WindowButton::*;

    #[test]
    fn gnome_default_shows_close_only() {
        assert_eq!(buttons_from_layout("appmenu:close"), [Close]);
    }

    #[test]
    fn enabled_minimize_and_maximize_appear_in_fixed_order() {
        assert_eq!(
            buttons_from_layout("appmenu:minimize,maximize,close"),
            [Close, Minimize, Maximize]
        );
        assert_eq!(
            buttons_from_layout("close,minimize,maximize:"),
            [Close, Minimize, Maximize]
        );
        assert_eq!(buttons_from_layout(":maximize"), [Maximize]);
    }

    #[test]
    fn layouts_without_buttons_still_get_close() {
        assert_eq!(buttons_from_layout(""), [Close]);
        assert_eq!(buttons_from_layout("icon:"), [Close]);
        assert_eq!(buttons_from_layout("menu:appmenu"), [Close]);
    }

    #[test]
    fn names_must_match_exactly() {
        assert_eq!(buttons_from_layout("appmenu:closer"), [Close]);
        assert_eq!(buttons_from_layout("unminimize:"), [Close]);
    }
}
