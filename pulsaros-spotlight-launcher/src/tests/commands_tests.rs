use super::*;
use std::collections::HashSet;

#[test]
fn at_least_60_commands() {
    assert!(builtin().len() >= 60, "got {}", builtin().len());
}

#[test]
fn ids_unique_and_prefixed() {
    let c = builtin();
    let ids: HashSet<_> = c.iter().map(|c| c.id.clone()).collect();
    assert_eq!(ids.len(), c.len());
    assert!(c.iter().all(|c| c.id.starts_with("gnome:")));
}

#[test]
fn fields_complete_and_argv_not_shell() {
    for c in builtin() {
        assert!(!c.title.is_empty() && !c.breadcrumb.is_empty() && !c.icon.is_empty(), "{}", c.id);
        assert!(!c.keywords.is_empty(), "{}", c.id);
        assert!(!c.argv.is_empty(), "{}", c.id);
        assert!(!["sh", "bash", "zsh"].contains(&c.argv[0].as_str()), "{}", c.id);
    }
}

#[test]
fn confirm_flags() {
    let c = builtin();
    let confirmed: HashSet<_> = c.iter().filter(|c| c.confirm).map(|c| c.title.as_str()).collect();
    for t in ["Shut Down", "Restart", "Log Out", "Empty Trash"] {
        assert!(confirmed.contains(t), "{t}");
    }
    assert!(!c.iter().find(|c| c.title == "Lock Screen").unwrap().confirm);
}

#[test]
fn dark_style_state_probe() {
    let c = builtin();
    let dark = c.iter().find(|c| c.id == "gnome:appearance.dark").unwrap();
    assert_eq!(dark.breadcrumb, "Appearance > Dark Style");
    let on: &Runner = &|a: &[String]| {
        assert_eq!(a[0], "gsettings");
        Some("'prefer-dark'\n".to_string())
    };
    assert_eq!(probe(dark, on), Some(true));
    let off: &Runner = &|_: &[String]| Some("'default'\n".to_string());
    assert_eq!(probe(dark, off), Some(false));
    let fail: &Runner = &|_: &[String]| None;
    assert_eq!(probe(dark, fail), None);
}

#[test]
fn no_probe_for_plain_command() {
    let c = builtin();
    let lock = c.iter().find(|c| c.id == "gnome:session.lock").unwrap();
    assert_eq!(probe(lock, &|_: &[String]| Some("x".into())), None);
}

#[test]
fn probe_states_map() {
    let c = builtin();
    let r: &Runner = &|a: &[String]| match a.first().map(|s| s.as_str()) {
        Some("nmcli") => Some("enabled\n".into()),
        Some("gsettings") => Some("'prefer-dark'".into()),
        _ => None,
    };
    let m = probe_states(&c, r);
    assert!(m["gnome:wifi.on"]);
    assert!(!m["gnome:wifi.off"]);
    assert!(m["gnome:appearance.dark"]);
    assert!(!m["gnome:appearance.light"]);
    assert!(!m.contains_key("gnome:bluetooth.on"));
}

#[test]
fn system_runner_works_and_fails_cleanly() {
    assert_eq!(system_runner(&v(&["echo", "hi"])).unwrap().trim(), "hi");
    assert!(system_runner(&v(&["definitely-not-a-binary-xyz"])).is_none());
    assert!(system_runner(&[]).is_none());
}

#[test]
fn dark_is_findable_by_ranker() {
    use crate::core::ranker::{rank, Item};
    let items: Vec<Item> = builtin()
        .iter()
        .map(|c| Item { id: c.id.clone(), title: c.title.clone(), keywords: c.keywords.clone(), alias: None })
        .collect();
    let r = rank("dark", &items);
    assert_eq!(r[0].0.id, "gnome:appearance.dark");
}
