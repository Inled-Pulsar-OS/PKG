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
    assert_eq!(lookup("c", CS), Some(Act::CopyFull));
    assert_eq!(lookup("d", C), Some(Act::DelItem));
    assert_eq!(lookup("d", CS), Some(Act::DelAll));
    assert_eq!(lookup("k", NONE), None);
    assert_eq!(lookup("1", A), Some(Act::Mode(1)));
    assert_eq!(lookup("4", A), Some(Act::Mode(4)));
    assert_eq!(lookup("1", C), Some(Act::Quick(0)));
}

#[test]
fn labels() {
    assert_eq!(label(Act::Favorite), vec!["Ctrl", "Shift", "F"]);
    assert_eq!(label(Act::Disable), vec!["Alt", "Shift", "D"]);
    assert_eq!(label(Act::Run(0)), vec!["\u{21b5}"]);
}

#[test]
fn alt_digits_map_to_applications_files_actions_clipboard() {
    let ids: Vec<_> = (1..=4)
        .map(|d| match lookup(&d.to_string(), A) {
            Some(Act::Mode(n)) => mode_id(n),
            other => panic!("Alt+{d} -> {other:?}"),
        })
        .collect();
    assert_eq!(ids, [Some("applications"), Some("files"), Some("actions"), Some("clipboard")]);
    assert_eq!(mode_id(0), None);
    assert_eq!(mode_id(5), None);
}

#[test]
fn quick_pick_covers_nine() {
    for i in 0..9 {
        assert!(TABLE.iter().any(|x| x.act == Act::Quick(i)));
    }
}
