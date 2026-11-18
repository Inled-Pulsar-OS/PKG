use super::*;

fn w(id: u32, title: &str, class: &str, ws: &str) -> WindowEntry {
    WindowEntry { id, title: title.into(), wm_class: class.into(), app_id: String::new(), workspace: ws.into() }
}
fn ids(v: Vec<&WindowEntry>) -> Vec<u32> {
    v.into_iter().map(|w| w.id).collect()
}

#[test]
fn ranks_by_title_and_class() {
    let ws = [w(1, "Inbox - Mail", "thunderbird", "0"), w(2, "notes.md", "Code", "1"), w(3, "Terminal", "kitty", "0")];
    let f = Frecency::default();
    assert_eq!(ids(rank("term", &ws, &f, 0)), vec![3]);
    assert_eq!(ids(rank("code", &ws, &f, 0)), vec![2]);
    assert_eq!(ids(rank("thunder", &ws, &f, 0)), vec![1]);
    assert!(rank("zzz", &ws, &f, 0).is_empty());
}

#[test]
fn frecency_breaks_ties_and_orders_empty_query() {
    let ws = [w(1, "Doc A", "app", "0"), w(2, "Doc B", "app", "0")];
    let mut f = Frecency::default();
    assert_eq!(ids(rank("", &ws, &f, 100)), vec![1, 2]);
    f.record_launch("window:2", "", 100);
    assert_eq!(ids(rank("", &ws, &f, 100)), vec![2, 1]);
    assert_eq!(ids(rank("doc", &ws, &f, 100)), vec![2, 1]);
}

#[test]
fn row_conversion() {
    let r = to_row(&w(7, "Terminal", "kitty", "2"));
    assert_eq!(r.id, "window:7");
    assert_eq!(r.subtitle, "kitty - Workspace 3");
    assert_eq!(r.actions[0].id, "window_activate");
    assert_eq!(WindowEntry::parse_row_id(&r.id), Some(7));
    assert_eq!(WindowEntry::parse_row_id("app:7"), None);
    assert_eq!(to_row(&w(1, "", "x", "-1")).title, "x");
}
