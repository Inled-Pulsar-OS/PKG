use super::*;

fn it(id: &str, title: &str, kw: &[&str]) -> Item {
    Item { id: id.into(), title: title.into(), keywords: kw.iter().map(|s| s.to_string()).collect(), alias: None }
}
fn order(q: &str, items: &[Item]) -> Vec<String> {
    rank(q, items).into_iter().map(|(i, _)| i.id.clone()).collect()
}

#[test]
fn empty_query_no_match() {
    assert_eq!(score("", &it("a", "A", &[])), None);
    assert_eq!(score("  ", &it("a", "A", &[])), None);
}

#[test]
fn no_match() {
    assert_eq!(score("zzz", &it("a", "Firefox", &[])), None);
}

#[test]
fn golden_ff_firefox_over_fast_fonts() {
    // Firefox carries the common "ff" abbreviation as a keyword.
    let items = [it("fonts", "Fast Fonts", &[]), it("ff", "Firefox", &["ff", "browser"])];
    assert_eq!(order("ff", &items), vec!["ff", "fonts"]);
}

#[test]
fn golden_tiers() {
    let items = [
        it("sub", "Xfirex", &[]),
        it("word", "My Fire Thing", &[]),
        it("prefix", "Fireplace Viewer", &[]),
        it("exact", "Fire", &[]),
        it("kw", "Browser", &["fire"]),
    ];
    assert_eq!(order("fire", &items), vec!["exact", "prefix", "word", "kw", "sub"]);
}

#[test]
fn initials() {
    let items = [it("a", "Visual Studio Code", &[]), it("b", "Vivaldi", &[])];
    assert_eq!(order("vsc", &items), vec!["a"]);
    assert_eq!(order("vs", &items)[0], "a");
}

#[test]
fn initials_beat_subsequence() {
    let items = [it("sub", "Fxsx", &[]), it("ini", "Fast Fonts", &[])];
    assert_eq!(order("ff", &items)[0], "ini");
}

#[test]
fn subsequence_tighter_wins() {
    let items = [it("loose", "Firefox Nightly", &[]), it("tight", "Frox", &[])];
    assert_eq!(order("frox", &items)[0], "tight");
    assert!(score("fx", &items[0]).is_some());
}

#[test]
fn case_insensitive() {
    assert_eq!(score("FIRE", &it("a", "firefox", &[])), score("fire", &it("a", "Firefox", &[])));
}

#[test]
fn alias_exact_is_top() {
    let mut a = it("a", "Zed", &[]);
    a.alias = Some("ff".into());
    let b = it("b", "ff", &[]);
    assert_eq!(order("ff", &[b, a]), vec!["a", "b"]);
}

#[test]
fn alias_prefix_matches() {
    let mut a = it("a", "Zed", &[]);
    a.alias = Some("edit".into());
    assert!(score("ed", &a).is_some());
}

#[test]
fn multiword_query() {
    let items = [it("a", "Dark Style", &[]), it("b", "Darkroom", &[])];
    assert_eq!(order("dark s", &items), vec!["a"]);
}

#[test]
fn shorter_title_wins_tie() {
    let items = [it("long", "Terminal Emulator Pro", &[]), it("short", "Terminal Pro", &[])];
    assert_eq!(order("term", &items)[0], "short");
}

#[test]
fn stable_tie_break_by_title_then_id() {
    let items = [it("2", "Same", &[]), it("1", "Same", &[]), it("0", "Same", &[])];
    assert_eq!(order("same", &items), vec!["0", "1", "2"]);
    let items = [it("b", "Bbb Tool", &[]), it("a", "Aaa Tool", &[])];
    assert_eq!(order("tool", &items), vec!["a", "b"]);
}

#[test]
fn keyword_exact_beats_title_substring() {
    let items = [it("t", "Xdarkx", &[]), it("k", "Appearance", &["dark"])];
    assert_eq!(order("dark", &items)[0], "k");
}
