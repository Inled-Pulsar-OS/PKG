use super::*;

fn provider(limit: usize) -> ClipboardProvider {
    let items = vec!["alpha one".to_string(), "beta two".to_string(), "alpha three".to_string()];
    ClipboardProvider::new(move || items.clone(), limit)
}

fn ids(p: &ClipboardProvider, q: &str) -> Vec<String> {
    p.instant(&Query::new(q)).into_iter().map(|r| r.id).collect()
}

#[test]
fn matches_query_and_keeps_history_index() {
    assert_eq!(ids(&provider(9), "ALPHA"), ["clipboard://0", "clipboard://2"]);
}

#[test]
fn limit_caps_rows_and_empty_query_matches_all() {
    assert_eq!(ids(&provider(2), ""), ["clipboard://0", "clipboard://1"]);
}

#[test]
fn row_has_first_line_title_and_flat_preview() {
    let rows = clipboard_rows(&["\n  first\nsecond".to_string()], "");
    assert_eq!(rows[0].title, "first");
    assert_eq!(rows[0].subtitle, " ⏎   first ⏎ second");
    assert_eq!(rows[0].kind, RowKind::Clipboard);
}

#[test]
fn long_text_is_cut_with_ellipsis() {
    let rows = clipboard_rows(&["x".repeat(200)], "");
    assert_eq!(rows[0].title.chars().count(), 60);
    assert_eq!(rows[0].subtitle.chars().count(), 100);
}
