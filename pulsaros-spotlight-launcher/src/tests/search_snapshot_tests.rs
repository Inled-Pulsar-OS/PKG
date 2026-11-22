//! Records the section titles and row ids that `Window::do_search` builds for the
//! All category, so later provider slices (#225) can prove behavior is unchanged.
//! `do_search` needs GTK, so this drives the same core call (`RootSearch::sections`)
//! with the same extra rows (calc first, then clipboard, then file) and the same
//! "calc hides Search the web" rule.
use std::collections::HashMap;

use crate::core::commands::builtin;
use crate::core::frecency::Frecency;
use crate::core::root_search::{AppEntry, Ctx, Row, RowKind, RootSearch};
use crate::core::settings::Settings;

fn app(id: &str, name: &str, kw: &[&str]) -> AppEntry {
    AppEntry { id: id.into(), name: name.into(), comment: String::new(), icon: id.into(), keywords: kw.iter().map(|s| s.to_string()).collect() }
}

fn snapshot(query: &str) -> String {
    let apps = vec![app("app:firefox", "Firefox", &["browser"]), app("app:zed", "Zed", &[])];
    let cmds = builtin();
    let (states, settings, fre) = (HashMap::new(), Settings::default(), Frecency::default());
    let mut extra = Vec::new();
    match query.trim() {
        "2+2" => extra.push(Row::simple(RowKind::Calc, "calc:4", "4", "2+2", "accessories-calculator-symbolic")),
        "banana" => extra.push(Row::simple(RowKind::Clipboard, "clip:1", "banana split", "Clipboard", "edit-paste-symbolic")),
        "report" => extra.push(Row::simple(RowKind::File, "file:/home/u/report.txt", "report.txt", "/home/u", "text-x-generic")),
        _ => {}
    }
    let ctx = Ctx { apps: &apps, commands: &cmds, states: &states, settings: &settings, frecency: &fre, now: 1000, extra: &extra };
    let mut secs = RootSearch::sections(query, &ctx);
    if extra.iter().any(|r| r.kind == RowKind::Calc) {
        secs.retain(|s| s.title != "Search the web");
    }
    // Long sections (all builtin commands) keep their first 3 ids plus a count.
    secs.iter()
        .map(|s| {
            let ids: Vec<&str> = s.rows.iter().map(|r| r.id.as_str()).collect();
            match ids.len() > 3 {
                true => format!("{}: {},... ({} rows)", s.title, ids[..3].join(","), ids.len()),
                false => format!("{}: {}", s.title, ids.join(",")),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn empty_query_lists_commands_then_apps() {
    let want = format!(
        "Commands: gnome:settings.wifi,gnome:settings.network,gnome:settings.bluetooth,... ({} rows)\nApplications: app:firefox,app:zed",
        builtin().len()
    );
    assert_eq!(snapshot(""), want);
}

#[test]
fn app_name_gives_app_then_web_fallbacks() {
    assert_eq!(snapshot("firefox"), "Results: app:firefox\nSearch the web: fallback:google,fallback:duckduckgo");
}

#[test]
fn calc_query_hides_web_fallbacks() {
    assert_eq!(snapshot("2+2"), "Results: calc:4");
}

#[test]
fn clipboard_word_lands_in_results() {
    assert_eq!(snapshot("banana"), "Results: clip:1\nSearch the web: fallback:google,fallback:duckduckgo");
}

#[test]
fn file_word_lands_in_results() {
    assert_eq!(
        snapshot("report"),
        "Results: file:/home/u/report.txt\nSearch the web: fallback:google,fallback:duckduckgo"
    );
}
