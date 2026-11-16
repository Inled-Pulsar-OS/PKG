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
#[path = "../tests/root_search_tests.rs"]
mod tests;
