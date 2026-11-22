use super::*;

fn v(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

#[test]
fn parse_flags() {
    let a = Args::parse(&v(&["x", "--dmenu", "--prompt", "Pick", "--placeholder=Type"])).unwrap();
    assert_eq!(a, Args { prompt: Some("Pick".into()), placeholder: Some("Type".into()) });
    assert_eq!(Args::parse(&v(&["x", "--dmenu"])).unwrap(), Args::default());
}

#[test]
fn parse_errors() {
    assert!(Args::parse(&v(&["x", "--dmenu", "--prompt"])).is_err());
    assert!(Args::parse(&v(&["x", "--dmenu", "--bogus"])).is_err());
    assert!(Args::parse(&v(&["x", "--prompt", "a"])).is_err());
}

#[test]
fn wants_dmenu_detect() {
    assert!(Args::wants_dmenu(&v(&["x", "--dmenu"])));
    assert!(!Args::wants_dmenu(&v(&["x", "--hidden"])));
    assert!(!Args::wants_dmenu(&v(&["--dmenu"])));
}

#[test]
fn reads_items() {
    let items = read_items("a\r\n\nb\n  \nc".as_bytes());
    assert_eq!(items, v(&["a", "b", "c"]));
}

#[test]
fn filter_empty_keeps_order_and_fuzzy_ranks() {
    let items = v(&["zeta", "Firefox", "Fast Fonts"]);
    assert_eq!(filter("", &items), items);
    assert_eq!(filter("fire", &items), v(&["Firefox"]));
    assert_eq!(filter("zz", &items), Vec::<String>::new());
    assert!(filter("qqq", &items).is_empty());
}

#[test]
fn outcomes() {
    let s = Outcome::Selected("b".into());
    assert_eq!((s.exit_code(), s.stdout()), (0, Some("b\n".into())));
    assert_eq!((Outcome::Cancelled.exit_code(), Outcome::Cancelled.stdout()), (1, None));
}
