use super::*;
use crate::core::commands::builtin;

fn app(id: &str, name: &str, kw: &[&str]) -> AppEntry {
    AppEntry { id: id.into(), name: name.into(), comment: format!("{name} app"), icon: id.into(), keywords: kw.iter().map(|s| s.to_string()).collect() }
}

struct World {
    apps: Vec<AppEntry>,
    cmds: Vec<Command>,
    states: HashMap<String, bool>,
    settings: Settings,
    fre: Frecency,
    extra: Vec<Row>,
}

impl World {
    fn new() -> Self {
        World {
            apps: vec![app("app:firefox", "Firefox", &["browser"]), app("app:fonts", "Fast Fonts", &[]), app("app:zed", "Zed", &[])],
            cmds: builtin(),
            states: HashMap::new(),
            settings: Settings::default(),
            fre: Frecency::default(),
            extra: vec![],
        }
    }
    fn run(&self, q: &str) -> Vec<Section> {
        RootSearch::sections(q, &Ctx { apps: &self.apps, commands: &self.cmds, states: &self.states, settings: &self.settings, frecency: &self.fre, now: 1000, extra: &self.extra })
    }
    fn titles(&self, q: &str) -> Vec<String> {
        self.run(q).iter().map(|s| s.title.clone()).collect()
    }
    fn ids(&self, q: &str, section: &str) -> Vec<String> {
        self.run(q).iter().find(|s| s.title == section).map(|s| s.rows.iter().map(|r| r.id.clone()).collect()).unwrap_or_default()
    }
}

#[test]
fn fresh_profile_root_has_commands_and_apps_only() {
    let w = World::new();
    assert_eq!(w.titles(""), ["Commands", "Applications"]);
    assert_eq!(w.titles("   "), ["Commands", "Applications"]);
}

#[test]
fn applications_sorted_by_name() {
    let w = World::new();
    assert_eq!(w.ids("", "Applications"), ["app:fonts", "app:firefox", "app:zed"]);
}

#[test]
fn root_section_order_full() {
    let mut w = World::new();
    w.settings.toggle_favorite("app:zed");
    w.fre.record_launch("app:firefox", "", 1000);
    assert_eq!(w.titles(""), ["Favorites", "Suggestions", "Commands", "Applications"]);
    assert_eq!(w.ids("", "Favorites"), ["app:zed"]);
    assert_eq!(w.ids("", "Suggestions"), ["app:firefox"]);
}

#[test]
fn suggestions_threshold_cap_and_exclude_favorites() {
    let mut w = World::new();
    // decayed below 50
    w.fre.record_launch("app:zed", "", 1000 - 11 * 86400);
    assert!(w.ids("", "Suggestions").is_empty());
    for i in 0..8 {
        w.cmds.push(Command { id: format!("c{i}"), ..w.cmds[0].clone() });
        w.fre.record_launch(&format!("c{i}"), "", 1000);
    }
    assert_eq!(w.ids("", "Suggestions").len(), SUGGESTION_MAX);
    w.settings.toggle_favorite("c0");
    assert!(!w.ids("", "Suggestions").contains(&"c0".to_string()));
    assert_eq!(w.ids("", "Favorites"), ["c0"]);
}

#[test]
fn favorites_keep_order_and_skip_unknown() {
    let mut w = World::new();
    w.settings.toggle_favorite("app:zed");
    w.settings.toggle_favorite("gone");
    w.settings.toggle_favorite("app:firefox");
    assert_eq!(w.ids("", "Favorites"), ["app:zed", "app:firefox"]);
    assert!(w.run("")[0].rows[0].favorite);
}

#[test]
fn query_has_results_then_fallbacks() {
    let w = World::new();
    let s = w.run("firefox");
    assert_eq!(s[0].title, "Results");
    assert_eq!(s[0].rows[0].id, "app:firefox");
    assert_eq!(s[1].rows.len(), 2);
    assert_eq!(s[1].rows[0].title, "Use \"firefox\" with Google");
    assert_eq!(s[1].rows[0].kind, RowKind::Fallback);
}

#[test]
fn fallback_url_encoded_argv() {
    let w = World::new();
    let s = w.run("a b&c");
    let s = [s.last().unwrap().clone()];
    let a = &s[0].rows[1].actions[0];
    assert_eq!(a.argv.as_ref().unwrap(), &vec!["xdg-open".to_string(), "https://duckduckgo.com/?q=a%20b%26c".to_string()]);
}

#[test]
fn no_match_still_shows_fallbacks_only() {
    let w = World::new();
    assert_eq!(w.titles("qqqzzz"), ["Search the web"]);
}

#[test]
fn fallbacks_not_shown_for_empty_query() {
    let w = World::new();
    assert!(!w.titles("").contains(&"Search the web".to_string()));
}

#[test]
fn custom_fallbacks_used() {
    let mut w = World::new();
    let st = crate::core::store::temp_store("rs-fb");
    std::fs::create_dir_all(st.path("settings").parent().unwrap()).unwrap();
    std::fs::write(st.path("settings"), r#"{"version":1,"data":{"fallbacks":[{"name":"Kagi","url":"https://k/?q={q}"}]}}"#).unwrap();
    w.settings = Settings::load(&st);
    assert_eq!(w.run("x")[1].rows.len(), 1);
    assert_eq!(w.run("x")[1].rows[0].title, "Use \"x\" with Kagi");
}

#[test]
fn alias_exact_goes_top() {
    let mut w = World::new();
    w.settings.set_alias("app:zed", "ff").unwrap();
    let r = w.ids("ff", "Results");
    assert_eq!(r[0], "app:zed");
    assert_eq!(w.run("ff")[0].rows[0].alias.as_deref(), Some("ff"));
}

#[test]
fn learned_term_beats_text_score() {
    let mut w = World::new();
    // text alone: "Fast Fonts" (initials) beats Firefox (subsequence)
    assert_eq!(w.ids("ff", "Results")[0], "app:fonts");
    w.fre.record_launch("app:firefox", "ff", 500);
    w.fre.record_launch("app:firefox", "ff", 600);
    assert_eq!(w.ids("ff", "Results")[0], "app:firefox");
}

#[test]
fn learned_prefix_boost() {
    let mut w = World::new();
    w.fre.record_launch("app:firefox", "ff", 500);
    w.fre.record_launch("app:firefox", "ff", 600);
    assert_eq!(w.ids("f", "Results")[0], "app:firefox");
}

#[test]
fn disabled_hidden_everywhere_and_listed() {
    let mut w = World::new();
    w.settings.disable("app:firefox");
    w.settings.toggle_favorite("app:firefox");
    assert!(!w.ids("firefox", "Results").contains(&"app:firefox".to_string()));
    assert!(!w.ids("", "Applications").contains(&"app:firefox".to_string()));
    assert!(w.ids("", "Favorites").is_empty());
    let ctx = Ctx { apps: &w.apps, commands: &w.cmds, states: &w.states, settings: &w.settings, frecency: &w.fre, now: 0, extra: &[] };
    let d = RootSearch::disabled_rows(&ctx);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].actions[0].id, "enable");
    w.settings.enable("app:firefox");
    assert!(w.ids("firefox", "Results").contains(&"app:firefox".to_string()));
}

#[test]
fn dark_command_checked_state_and_breadcrumb() {
    let mut w = World::new();
    w.states.insert("gnome:appearance.dark".into(), true);
    let s = w.run("dark");
    let r = &s[0].rows[0];
    assert_eq!(r.id, "gnome:appearance.dark");
    assert_eq!(r.checked, Some(true));
    assert_eq!(r.breadcrumb, "Appearance > Dark Style");
    assert_eq!(r.kind, RowKind::Command);
    assert_eq!(r.actions[0].argv.as_ref().unwrap()[0], "gsettings");
}

#[test]
fn confirm_flag_on_rows() {
    let w = World::new();
    let s = w.run("shut down");
    assert_eq!(s[0].rows[0].id, "gnome:session.shutdown");
    assert!(s[0].rows[0].confirm);
    assert!(!w.run("lock screen")[0].rows[0].confirm);
}

#[test]
fn extras_calc_first_files_last() {
    let mut w = World::new();
    w.extra = vec![
        Row::simple(RowKind::File, "file:/a", "a.txt", "/a", "text-x-generic"),
        Row::simple(RowKind::Calc, "calc", "4", "2+2", "accessories-calculator"),
    ];
    let ids = w.ids("firefox", "Results");
    assert_eq!(ids.first().unwrap(), "calc");
    assert_eq!(ids.last().unwrap(), "file:/a");
    assert!(ids.contains(&"app:firefox".to_string()));
}

#[test]
fn row_actions_and_favorite_label() {
    let mut w = World::new();
    let r = &w.run("zed")[0].rows[0];
    let ids: Vec<_> = r.actions.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["open", "favorite", "alias", "disable"]);
    assert_eq!(r.actions[1].title, "Add to Favorites");
    w.settings.toggle_favorite("app:zed");
    assert_eq!(w.run("zed")[0].rows[0].actions[1].title, "Remove from Favorites");
}

#[test]
fn deterministic_order() {
    let w = World::new();
    assert_eq!(w.run("s"), w.run("s"));
}

#[test]
fn prebuilt_index_matches_and_beats_per_query_build_with_1000_apps() {
    let mut w = World::new();
    w.apps = (0..1000).map(|i| app(&format!("app:a{i}"), &format!("Sample Tool Number {i}"), &["utility", "helper"])).collect();
    let ctx = Ctx { apps: &w.apps, commands: &w.cmds, states: &w.states, settings: &w.settings, frecency: &w.fre, now: 1000, extra: &[] };
    let index = SearchIndex::new(&w.apps, &w.cmds);
    assert_eq!(RootSearch::sections("stn", &ctx), RootSearch::sections_indexed("stn", &ctx, &index));

    let time = |f: &dyn Fn() -> Vec<Section>| {
        let t = std::time::Instant::now();
        for _ in 0..50 {
            std::hint::black_box(f());
        }
        t.elapsed() / 50
    };
    let rebuilt = time(&|| RootSearch::sections("stn", &ctx));
    let indexed = time(&|| RootSearch::sections_indexed("stn", &ctx, &index));
    eprintln!("per query, 1000 apps: build+query {rebuilt:?}, prebuilt index {indexed:?}");
    assert!(indexed < rebuilt);
}
