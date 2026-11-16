use super::*;
use crate::core::store::temp_store;

#[test]
fn alias_set_get_clear() {
    let mut s = Settings::default();
    s.set_alias("a", " FF ").unwrap();
    assert_eq!(s.alias("a"), Some("ff"));
    s.set_alias("a", "").unwrap();
    assert_eq!(s.alias("a"), None);
}

#[test]
fn alias_duplicate_rejected_same_id_ok() {
    let mut s = Settings::default();
    s.set_alias("a", "ff").unwrap();
    let e = s.set_alias("b", "FF").unwrap_err();
    assert!(e.contains("already used"));
    assert_eq!(s.alias("b"), None);
    s.set_alias("a", "ff").unwrap();
    s.set_alias("a", "fx").unwrap();
    s.set_alias("b", "ff").unwrap();
}

#[test]
fn favorites_toggle_and_reorder() {
    let mut s = Settings::default();
    assert!(s.toggle_favorite("a"));
    s.toggle_favorite("b");
    s.toggle_favorite("c");
    assert_eq!(s.favorites(), ["a", "b", "c"]);
    s.move_favorite("c", -1);
    assert_eq!(s.favorites(), ["a", "c", "b"]);
    s.move_favorite("c", -5);
    assert_eq!(s.favorites(), ["c", "a", "b"]);
    s.move_favorite("c", 99);
    assert_eq!(s.favorites(), ["a", "b", "c"]);
    s.move_favorite("zzz", 1);
    assert!(!s.toggle_favorite("b"));
    assert!(!s.is_favorite("b"));
    assert_eq!(s.favorites(), ["a", "c"]);
}

#[test]
fn move_in_empty_is_safe() {
    let mut s = Settings::default();
    s.move_favorite("a", 1);
}

#[test]
fn disable_undo() {
    let mut s = Settings::default();
    s.disable("a");
    s.disable("a");
    assert_eq!(s.disabled(), ["a"]);
    assert!(s.is_disabled("a"));
    s.enable("a");
    assert!(!s.is_disabled("a"));
    assert!(s.disabled().is_empty());
}

#[test]
fn history_dedupe_cap_clear() {
    let mut s = Settings::default();
    s.push_history("one");
    s.push_history("two");
    s.push_history("ONE");
    s.push_history("  ");
    assert_eq!(s.history(), ["ONE", "two"]);
    for i in 0..100 {
        s.push_history(&format!("q{i}"));
    }
    assert_eq!(s.history().len(), HISTORY_CAP);
    assert_eq!(s.history()[0], "q99");
    s.clear_history();
    assert!(s.history().is_empty());
}

#[test]
fn default_fallbacks_present() {
    let s = Settings::default();
    assert_eq!(s.fallbacks().len(), 2);
    assert_eq!(s.fallbacks()[0].name, "Google");
}

#[test]
fn persists_roundtrip() {
    let st = temp_store("settings");
    let mut s = Settings::default();
    s.set_alias("a", "x").unwrap();
    s.toggle_favorite("a");
    s.disable("b");
    s.push_history("hello");
    s.save(&st).unwrap();
    let l = Settings::load(&st);
    assert_eq!(l.alias("a"), Some("x"));
    assert_eq!(l.favorites(), ["a"]);
    assert!(l.is_disabled("b"));
    assert_eq!(l.history(), ["hello"]);
    assert_eq!(l.fallbacks().len(), 2);
}

#[test]
fn load_corrupt_gives_defaults() {
    let st = temp_store("settings-bad");
    std::fs::create_dir_all(st.path("settings").parent().unwrap()).unwrap();
    std::fs::write(st.path("settings"), "garbage").unwrap();
    let l = Settings::load(&st);
    assert!(l.favorites().is_empty());
    assert_eq!(l.fallbacks().len(), 2);
}

#[test]
fn custom_fallbacks_survive() {
    let st = temp_store("settings-fb");
    std::fs::create_dir_all(st.path("settings").parent().unwrap()).unwrap();
    std::fs::write(
        st.path("settings"),
        r#"{"version":1,"data":{"fallbacks":[{"name":"Kagi","url":"https://kagi.com/search?q={q}"}]}}"#,
    )
    .unwrap();
    let l = Settings::load(&st);
    assert_eq!(l.fallbacks().len(), 1);
    assert_eq!(l.fallbacks()[0].name, "Kagi");
}
