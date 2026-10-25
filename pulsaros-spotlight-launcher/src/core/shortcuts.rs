//! One shortcut table for the whole launcher. Keys are lowercase GDK key names
//! (`Key::to_lower().name()`), so this module stays free of gtk types.
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Act {
    ActionPanel,
    /// Run action N (0-based) of the selected row's action list.
    Run(usize),
    /// Quick pick the Nth visible row (0-based).
    Quick(usize),
    SectionPrev,
    SectionNext,
    PageUp,
    PageDown,
    Back,
    Root,
    Favorite,
    FavUp,
    FavDown,
    Disable,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Quit,
    CatNext,
    CatPrev,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

const fn m(ctrl: bool, shift: bool, alt: bool) -> Mods {
    Mods { ctrl, shift, alt }
}
const NONE: Mods = m(false, false, false);
const C: Mods = m(true, false, false);
const CS: Mods = m(true, true, false);
const CA: Mods = m(true, false, true);
const A: Mods = m(false, false, true);
const AS: Mods = m(false, true, true);
const S: Mods = m(false, true, false);

pub struct Binding {
    pub key: &'static str,
    pub mods: Mods,
    pub act: Act,
}

const fn b(key: &'static str, mods: Mods, act: Act) -> Binding {
    Binding { key, mods, act }
}

pub static TABLE: &[Binding] = &[
    b("up", NONE, Act::Up),
    b("down", NONE, Act::Down),
    b("k", C, Act::ActionPanel),
    b("return", NONE, Act::Run(0)),
    b("kp_enter", NONE, Act::Run(0)),
    b("return", C, Act::Run(1)),
    b("kp_enter", C, Act::Run(1)),
    b("return", CS, Act::Run(2)),
    b("kp_enter", CS, Act::Run(2)),
    b("1", C, Act::Quick(0)),
    b("2", C, Act::Quick(1)),
    b("3", C, Act::Quick(2)),
    b("4", C, Act::Quick(3)),
    b("5", C, Act::Quick(4)),
    b("6", C, Act::Quick(5)),
    b("7", C, Act::Quick(6)),
    b("8", C, Act::Quick(7)),
    b("9", C, Act::Quick(8)),
    b("up", C, Act::SectionPrev),
    b("down", C, Act::SectionNext),
    b("page_up", NONE, Act::PageUp),
    b("page_down", NONE, Act::PageDown),
    b("escape", NONE, Act::Back),
    b("escape", S, Act::Root),
    b("f", CS, Act::Favorite),
    b("up", CA, Act::FavUp),
    b("down", CA, Act::FavDown),
    b("d", AS, Act::Disable),
    b("plus", C, Act::ZoomIn),
    b("plus", CS, Act::ZoomIn),
    b("equal", C, Act::ZoomIn),
    b("kp_add", C, Act::ZoomIn),
    b("minus", C, Act::ZoomOut),
    b("kp_subtract", C, Act::ZoomOut),
    b("0", C, Act::ZoomReset),
    b("kp_0", C, Act::ZoomReset),
    b("q", C, Act::Quit),
    b("tab", NONE, Act::CatNext),
    b("iso_left_tab", NONE, Act::CatPrev),
    b("iso_left_tab", S, Act::CatPrev),
    b("tab", S, Act::CatPrev),
    b("right", C, Act::CatNext),
    b("left", C, Act::CatPrev),
    b("right", A, Act::CatNext),
    b("left", A, Act::CatPrev),
];

pub fn lookup(key: &str, mods: Mods) -> Option<Act> {
    TABLE.iter().find(|x| x.key == key && x.mods == mods).map(|x| x.act)
}

/// Human label for the first binding of `act`, e.g. `["Ctrl", "Shift", "F"]`.
pub fn label(act: Act) -> Vec<String> {
    let Some(x) = TABLE.iter().find(|x| x.act == act) else { return Vec::new() };
    let mut out = Vec::new();
    if x.mods.ctrl {
        out.push("Ctrl".to_string());
    }
    if x.mods.alt {
        out.push("Alt".to_string());
    }
    if x.mods.shift {
        out.push("Shift".to_string());
    }
    out.push(match x.key {
        "return" | "kp_enter" => "\u{21b5}".to_string(),
        "up" => "\u{2191}".to_string(),
        "down" => "\u{2193}".to_string(),
        "plus" => "+".to_string(),
        k => k.to_uppercase(),
    });
    out
}

/// Duplicate (key, mods) pairs; must stay empty.
pub fn conflicts() -> Vec<(&'static str, Mods)> {
    let mut seen = HashSet::new();
    TABLE.iter().filter(|x| !seen.insert((x.key, x.mods))).map(|x| (x.key, x.mods)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_conflicts() {
        assert!(conflicts().is_empty(), "{:?}", conflicts());
    }

    #[test]
    fn lookups() {
        assert_eq!(lookup("k", C), Some(Act::ActionPanel));
        assert_eq!(lookup("return", CS), Some(Act::Run(2)));
        assert_eq!(lookup("up", CA), Some(Act::FavUp));
        assert_eq!(lookup("up", C), Some(Act::SectionPrev));
        assert_eq!(lookup("5", C), Some(Act::Quick(4)));
        assert_eq!(lookup("escape", S), Some(Act::Root));
        assert_eq!(lookup("k", NONE), None);
    }

    #[test]
    fn labels() {
        assert_eq!(label(Act::Favorite), vec!["Ctrl", "Shift", "F"]);
        assert_eq!(label(Act::Disable), vec!["Alt", "Shift", "D"]);
        assert_eq!(label(Act::Run(0)), vec!["\u{21b5}"]);
    }

    #[test]
    fn quick_pick_covers_nine() {
        for i in 0..9 {
            assert!(TABLE.iter().any(|x| x.act == Act::Quick(i)));
        }
    }
}
