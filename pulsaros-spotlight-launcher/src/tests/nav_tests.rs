use super::*;
use NavEvent as E;

fn nav(b: Browse) -> Nav {
    Nav { browse: b, ..Nav::default() }
}

fn after(b: Browse, e: NavEvent) -> Browse {
    let mut n = nav(b);
    n.apply(e);
    n.browse
}

#[test]
fn starts_at_root() {
    let n = Nav::default();
    assert!(n.is_root());
    assert_eq!(n.opened, Opened::None);
    assert!(!n.typed);
}

#[test]
fn open_dir_from_any_list() {
    for b in [Browse::Root, Browse::RecentSearches, Browse::CalcHistory, Browse::Dir("/a".into())] {
        assert_eq!(after(b, E::OpenDir("/b".into())), Browse::Dir("/b".into()));
    }
}

#[test]
fn parent_dir_goes_up_and_stops_at_top() {
    assert_eq!(after(Browse::Dir("/a/b".into()), E::ParentDir), Browse::Dir("/a".into()));
    assert_eq!(after(Browse::Dir("/a".into()), E::ParentDir), Browse::Dir("/".into()));
    assert_eq!(after(Browse::Dir("/".into()), E::ParentDir), Browse::Dir("/".into()));
}

#[test]
fn parent_dir_ignored_outside_a_folder() {
    for b in [Browse::Root, Browse::RecentSearches, Browse::CalcHistory] {
        assert_eq!(after(b.clone(), E::ParentDir), b);
    }
}

#[test]
fn show_recent_and_calc_history_replace_the_list() {
    assert_eq!(after(Browse::Root, E::ShowRecent), Browse::RecentSearches);
    assert_eq!(after(Browse::CalcHistory, E::ShowRecent), Browse::RecentSearches);
    assert_eq!(after(Browse::RecentSearches, E::ShowCalcHistory), Browse::CalcHistory);
    // Never two lists at once: a folder is dropped.
    assert_eq!(after(Browse::Dir("/a".into()), E::ShowCalcHistory), Browse::CalcHistory);
}

#[test]
fn query_change_ends_recent_searches_only() {
    assert_eq!(after(Browse::RecentSearches, E::QueryChanged), Browse::Root);
    assert_eq!(after(Browse::CalcHistory, E::QueryChanged), Browse::CalcHistory);
    assert_eq!(after(Browse::Dir("/a".into()), E::QueryChanged), Browse::Dir("/a".into()));
    assert_eq!(after(Browse::Root, E::QueryChanged), Browse::Root);
}

#[test]
fn back_and_to_root_return_to_root() {
    for b in [Browse::Root, Browse::RecentSearches, Browse::CalcHistory, Browse::Dir("/a".into())] {
        assert_eq!(after(b.clone(), E::Back), Browse::Root);
        assert_eq!(after(b, E::ToRoot), Browse::Root);
    }
}

#[test]
fn typed_and_opened_are_independent_of_the_list() {
    let mut n = nav(Browse::CalcHistory);
    n.apply(E::Typed(true));
    n.apply(E::PanelOpened(Opened::Home));
    assert!(n.typed);
    assert_eq!(n.opened, Opened::Home);
    assert_eq!(n.browse, Browse::CalcHistory);
    n.apply(E::Typed(false));
    assert!(!n.typed);
}

#[test]
fn collapse_forgets_how_the_panel_opened_but_keeps_the_list() {
    let mut n = nav(Browse::Dir("/a".into()));
    n.apply(E::PanelOpened(Opened::History));
    n.apply(E::Typed(true));
    n.apply(E::Collapsed);
    assert_eq!(n.opened, Opened::None);
    assert!(!n.typed);
    assert_eq!(n.browse, Browse::Dir("/a".into()));
}
