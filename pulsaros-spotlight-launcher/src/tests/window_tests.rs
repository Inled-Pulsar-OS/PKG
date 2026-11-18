use super::*;

#[test]
fn calc_result_becomes_calc_row() {
    let r = row_from_result(&SearchResult {
        url: "calc://4".into(),
        title: "4".into(),
        mime: "application/x-calculator".into(),
        snippet: "2+2".into(),
        app: None,
    });
    assert_eq!(r.row.kind, RowKind::Calc);
    assert_eq!(primary_title(&r), "Copy Answer");
}

#[test]
fn uninstall_message_is_cleaned() {
    assert!(clean_uninstall_message("foo\nerror: bar baz", "X").contains("bar baz"));
    assert!(clean_uninstall_message("unmet dependencies", "X").starts_with("Cannot remove X"));
}

#[test]
fn zoom_css_scales() {
    assert!(zoom_css(1.5).contains("min-height: 84px"));
}
