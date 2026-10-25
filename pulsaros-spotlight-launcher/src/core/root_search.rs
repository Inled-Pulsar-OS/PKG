//! Root search: turns (query, apps, commands, settings, frecency) into display sections.
use std::collections::HashMap;

use super::commands::Command;
use super::frecency::Frecency;
use super::ranker::{self, Item};
use super::settings::Settings;

pub const SUGGESTION_MIN: f64 = 50.0;
pub const SUGGESTION_MAX: usize = 6;
const LEARNED_STEP: i64 = 3000;
const LEARNED_MAX: i64 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    App,
    Command,
    File,
    Clipboard,
    Calc,
    Fallback,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionDesc {
    /// `open`, `run`, `open_url`, `favorite`, `alias`, `disable`, ...
    pub id: String,
    pub title: String,
    /// `Some` for plain process spawns; `None` when the UI handles the id itself.
    pub argv: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub breadcrumb: String,
    pub icon_name: String,
    pub kind: RowKind,
    pub checked: Option<bool>,
    pub alias: Option<String>,
    pub favorite: bool,
    /// Show an alert before running (destructive command).
    pub confirm: bool,
    pub actions: Vec<ActionDesc>,
}

impl Row {
    /// Minimal row for UI-provided results (files, clipboard, calculator).
    pub fn simple(kind: RowKind, id: &str, title: &str, subtitle: &str, icon: &str) -> Row {
        Row {
            id: id.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            breadcrumb: String::new(),
            icon_name: icon.into(),
            kind,
            checked: None,
            alias: None,
            favorite: false,
            confirm: false,
            actions: vec![ActionDesc { id: "open".into(), title: "Open".into(), argv: None }],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub title: String,
    pub rows: Vec<Row>,
}

/// Plain app input (convert from `crate::apps::DesktopApp` in the UI layer).
#[derive(Clone, Debug, Default)]
pub struct AppEntry {
    /// Stable id, e.g. `app:firefox.desktop`.
    pub id: String,
    pub name: String,
    pub comment: String,
    pub icon: String,
    pub keywords: Vec<String>,
}

pub struct Ctx<'a> {
    pub apps: &'a [AppEntry],
    pub commands: &'a [Command],
    /// Command id -> checked, from `commands::probe_states`.
    pub states: &'a HashMap<String, bool>,
    pub settings: &'a Settings,
    pub frecency: &'a Frecency,
    /// Unix seconds.
    pub now: i64,
    /// UI-provided rows (Calc first, then File/Clipboard) added to Results.
    pub extra: &'a [Row],
}

pub struct RootSearch;

fn actions(settings: &Settings, id: &str, primary: ActionDesc) -> Vec<ActionDesc> {
    let fav = if settings.is_favorite(id) { "Remove from Favorites" } else { "Add to Favorites" };
    vec![
        primary,
        ActionDesc { id: "favorite".into(), title: fav.into(), argv: None },
        ActionDesc { id: "alias".into(), title: "Set Alias".into(), argv: None },
        ActionDesc { id: "disable".into(), title: "Disable".into(), argv: None },
    ]
}

fn app_row(a: &AppEntry, s: &Settings) -> Row {
    Row {
        id: a.id.clone(),
        title: a.name.clone(),
        subtitle: a.comment.clone(),
        breadcrumb: String::new(),
        icon_name: a.icon.clone(),
        kind: RowKind::App,
        checked: None,
        alias: s.alias(&a.id).map(String::from),
        favorite: s.is_favorite(&a.id),
        confirm: false,
        actions: actions(s, &a.id, ActionDesc { id: "open".into(), title: "Open".into(), argv: None }),
    }
}

fn cmd_row(c: &Command, ctx: &Ctx) -> Row {
    Row {
        id: c.id.clone(),
        title: c.title.clone(),
        subtitle: c.breadcrumb.clone(),
        breadcrumb: c.breadcrumb.clone(),
        icon_name: c.icon.clone(),
        kind: RowKind::Command,
        checked: ctx.states.get(&c.id).copied(),
        alias: ctx.settings.alias(&c.id).map(String::from),
        favorite: ctx.settings.is_favorite(&c.id),
        confirm: c.confirm,
        actions: actions(
            ctx.settings,
            &c.id,
            ActionDesc { id: "run".into(), title: "Run".into(), argv: Some(c.argv.clone()) },
        ),
    }
}

impl RootSearch {
    fn row_by_id(id: &str, ctx: &Ctx) -> Option<Row> {
        if ctx.settings.is_disabled(id) {
            return None;
        }
        if let Some(a) = ctx.apps.iter().find(|a| a.id == id) {
            return Some(app_row(a, ctx.settings));
        }
        ctx.commands.iter().find(|c| c.id == id).map(|c| cmd_row(c, ctx))
    }

    pub fn sections(query: &str, ctx: &Ctx) -> Vec<Section> {
        let q = query.trim();
        let mut out = Vec::new();
        if q.is_empty() {
            Self::root(ctx, &mut out);
        } else {
            Self::query(q, ctx, &mut out);
        }
        out.retain(|s| !s.rows.is_empty());
        out
    }

    fn root(ctx: &Ctx, out: &mut Vec<Section>) {
        let fav_rows: Vec<Row> =
            ctx.settings.favorites().iter().filter_map(|id| Self::row_by_id(id, ctx)).collect();
        let suggestions: Vec<Row> = ctx
            .frecency
            .top(ctx.now, SUGGESTION_MIN, usize::MAX)
            .iter()
            .filter(|id| !ctx.settings.is_favorite(id))
            .filter_map(|id| Self::row_by_id(id, ctx))
            .take(SUGGESTION_MAX)
            .collect();
        out.push(Section { title: "Favorites".into(), rows: fav_rows });
        out.push(Section { title: "Suggestions".into(), rows: suggestions });
        out.push(Section {
            title: "Commands".into(),
            rows: ctx
                .commands
                .iter()
                .filter(|c| !ctx.settings.is_disabled(&c.id))
                .map(|c| cmd_row(c, ctx))
                .collect(),
        });
        let mut apps: Vec<&AppEntry> = ctx.apps.iter().filter(|a| !ctx.settings.is_disabled(&a.id)).collect();
        apps.sort_by_key(|a| (a.name.to_lowercase(), a.id.clone()));
        out.push(Section { title: "Applications".into(), rows: apps.iter().map(|a| app_row(a, ctx.settings)).collect() });
    }

    fn query(q: &str, ctx: &Ctx, out: &mut Vec<Section>) {
        let items: Vec<Item> = ctx
            .apps
            .iter()
            .map(|a| (a.id.as_str(), a.name.as_str(), &a.keywords))
            .chain(ctx.commands.iter().map(|c| (c.id.as_str(), c.title.as_str(), &c.keywords)))
            .filter(|(id, _, _)| !ctx.settings.is_disabled(id))
            .map(|(id, title, kw)| Item {
                id: id.into(),
                title: title.into(),
                keywords: kw.clone(),
                alias: ctx.settings.alias(id).map(String::from),
            })
            .collect();

        let mut scored: Vec<(&Item, i64)> = ranker::rank(q, &items)
            .into_iter()
            .map(|(i, s)| {
                let learned = (ctx.frecency.learned(q, &i.id) as i64).min(LEARNED_MAX) * LEARNED_STEP;
                let fre = (ctx.frecency.score(&i.id, ctx.now).min(300.0) * 3.0) as i64;
                (i, s + learned + fre)
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()))
                .then_with(|| a.0.id.cmp(&b.0.id))
        });

        let mut rows: Vec<Row> = ctx.extra.iter().filter(|r| r.kind == RowKind::Calc).cloned().collect();
        rows.extend(scored.iter().filter_map(|(i, _)| Self::row_by_id(&i.id, ctx)));
        rows.extend(ctx.extra.iter().filter(|r| r.kind != RowKind::Calc).cloned());
        out.push(Section { title: "Results".into(), rows });

        let enc = urlencoding::encode(q);
        out.push(Section {
            title: "Search the web".into(),
            rows: ctx
                .settings
                .fallbacks()
                .iter()
                .map(|f| {
                    let url = f.url.replace("{q}", &enc);
                    let mut r = Row::simple(RowKind::Fallback, &format!("fallback:{}", f.name.to_lowercase()), &format!("Use \"{q}\" with {}", f.name), &url, "web-browser-symbolic");
                    r.actions = vec![ActionDesc { id: "open_url".into(), title: format!("Search {}", f.name), argv: Some(vec!["xdg-open".into(), url]) }];
                    r
                })
                .collect(),
        });
    }

    /// Rows for a "Disabled commands" view: pick one to `Settings::enable` it.
    pub fn disabled_rows(ctx: &Ctx) -> Vec<Row> {
        ctx.settings
            .disabled()
            .iter()
            .filter_map(|id| {
                if let Some(a) = ctx.apps.iter().find(|a| &a.id == id) {
                    let mut r = app_row(a, ctx.settings);
                    r.actions = vec![ActionDesc { id: "enable".into(), title: "Enable".into(), argv: None }];
                    return Some(r);
                }
                ctx.commands.iter().find(|c| &c.id == id).map(|c| {
                    let mut r = cmd_row(c, ctx);
                    r.actions = vec![ActionDesc { id: "enable".into(), title: "Enable".into(), argv: None }];
                    r
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
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
}
