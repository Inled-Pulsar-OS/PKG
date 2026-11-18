use super::*;
use crate::core::store::temp_store;

fn sample() -> Quicklinks {
    let mut q = Quicklinks::default();
    q.add("gh", "GitHub search", "https://github.com/search?q={query}").unwrap();
    q.add("docs", "", "https://docs.rs").unwrap();
    q.add("paste", "Paste", "https://x.test/?c={clipboard}&q={query}").unwrap();
    q
}

#[test]
fn resolves_and_encodes() {
    let r = sample().resolve("gh rust & c++").unwrap();
    assert_eq!(r.url, "https://github.com/search?q=rust%20%26%20c%2B%2B");
    assert_eq!(r.title, "Open GitHub search: rust & c++");
}

#[test]
fn case_insensitive_alias() {
    assert!(sample().resolve("GH x").is_some());
}

#[test]
fn unknown_alias_none() {
    assert!(sample().resolve("zz x").is_none());
    assert!(sample().resolve("").is_none());
    assert!(sample().resolve("   ").is_none());
}

#[test]
fn missing_placeholder_ignores_query() {
    let r = sample().resolve("docs serde").unwrap();
    assert_eq!(r.url, "https://docs.rs");
    assert_eq!(r.title, "Open docs: serde");
}

#[test]
fn bare_alias_empty_query() {
    let r = sample().resolve("gh").unwrap();
    assert_eq!(r.url, "https://github.com/search?q=");
    assert_eq!(r.title, "Open GitHub search");
}

#[test]
fn clipboard_placeholder() {
    let r = sample().resolve_with_clipboard("paste a b", "x y/z").unwrap();
    assert_eq!(r.url, "https://x.test/?c=x%20y%2Fz&q=a%20b");
    assert_eq!(sample().resolve("paste q").unwrap().url, "https://x.test/?c=&q=q");
}

#[test]
fn duplicate_alias_rejected() {
    let mut q = sample();
    assert!(q.add("GH", "", "https://a").is_err());
    assert_eq!(q.items.len(), 3);
}

#[test]
fn invalid_add_rejected() {
    let mut q = Quicklinks::default();
    assert!(q.add("", "", "https://a").is_err());
    assert!(q.add("a b", "", "https://a").is_err());
    assert!(q.add("a", "", "  ").is_err());
}

#[test]
fn remove_works() {
    let mut q = sample();
    assert!(q.remove("gh"));
    assert!(!q.remove("gh"));
    assert!(q.resolve("gh x").is_none());
}

#[test]
fn store_roundtrip() {
    let s = temp_store("quicklinks");
    assert!(Quicklinks::load(&s).items.is_empty());
    sample().save(&s).unwrap();
    assert_eq!(Quicklinks::load(&s), sample());
}

#[test]
fn open_argv_is_xdg_open() {
    assert_eq!(open_argv("https://a.b/?q=1 2"), vec!["xdg-open", "https://a.b/?q=1 2"]);
}
