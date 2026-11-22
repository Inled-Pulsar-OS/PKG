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
fn bar_event_maps_down_up_and_modes() {
    let none = Mods { ctrl: false, shift: false, alt: false };
    assert_eq!(bar_event(gdk::Key::Down, none, None), Some(LayoutEvent::Down));
    assert_eq!(bar_event(gdk::Key::Up, none, None), Some(LayoutEvent::Up));
    assert_eq!(bar_event(gdk::Key::Left, none, None), None);
    assert_eq!(bar_event(gdk::Key::Down, Mods { ctrl: true, ..none }, None), None);
    assert_eq!(bar_event(gdk::Key::_2, Mods { alt: true, ..none }, Some(Act::Mode(2))), Some(LayoutEvent::Mode(BarMode::Files)));
    assert_eq!(bar_event(gdk::Key::_4, Mods { alt: true, ..none }, Some(Act::Mode(4))), Some(LayoutEvent::Mode(BarMode::Clipboard)));
}

#[test]
fn zoom_css_scales() {
    assert!(zoom_css(1.5).contains("min-height: 84px"));
}
