use super::*;

#[test]
fn calc_result_becomes_calc_row() {
    let r = result_row("calc://4".into(), "4".into(), "application/x-calculator".into(), "2+2".into(), None);
    assert_eq!(r.row.kind, RowKind::Calc);
    assert_eq!(primary_title(&r), "Copy Answer");
}

#[test]
fn clipboard_result_keeps_payload_for_paste() {
    let r = result_row("clipboard://3".into(), "hi".into(), "text/plain-clipboard".into(), "hi there".into(), None);
    assert_eq!(r.row.kind, RowKind::Clipboard);
    assert_eq!(r.row.id, "clipboard://3");
    assert_eq!(r.payload.unwrap().url, "clipboard://3");
}

#[test]
fn file_and_folder_results_keep_url_and_mime() {
    let f = result_row("file:///tmp/a.txt".into(), "a.txt".into(), "text/plain".into(), String::new(), None);
    assert_eq!(f.row.kind, RowKind::File);
    assert_eq!(f.row.subtitle, "/tmp/a.txt");
    assert_eq!(primary_title(&f), "Open");
    let d = result_row("file:///tmp".into(), "tmp".into(), "inode/directory".into(), "/tmp".into(), None);
    assert_eq!(primary_title(&d), "Open Folder");
    assert_eq!(d.payload.unwrap().mime, "inode/directory");
}

#[test]
fn history_and_web_results_open_in_browser() {
    let h = result_row("https://a.test".into(), "A".into(), "text/html".into(), "https://a.test".into(), None);
    assert_eq!(h.row.kind, RowKind::File);
    assert_eq!(primary_title(&h), "Open in Browser");
    assert_eq!(h.payload.unwrap().url, "https://a.test");
}

#[test]
fn app_result_carries_its_desktop_url() {
    let a = result_row("app://foo.desktop".into(), "Foo".into(), "application/x-desktop".into(), "foo".into(), None);
    assert_eq!(a.payload.unwrap().url, "app://foo.desktop");
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

#[test]
fn hyprland_cursor_reply_parses() {
    assert_eq!(parse_cursor_pos("1870, 521\n"), Some((1870.0, 521.0)));
    assert_eq!(parse_cursor_pos("garbage"), None);
}
