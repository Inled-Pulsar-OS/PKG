//! Window switcher logic: ranking and row conversion for windows reported by the Shell extension.
use super::frecency::Frecency;
use super::ranker::{self, Item};
use super::root_search::{ActionDesc, Row, RowKind};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowEntry {
    pub id: u32,
    pub title: String,
    pub wm_class: String,
    pub app_id: String,
    pub workspace: String,
}

impl WindowEntry {
    pub fn row_id(&self) -> String {
        format!("window:{}", self.id)
    }

    /// Parse a `window:<id>` row id.
    pub fn parse_row_id(id: &str) -> Option<u32> {
        id.strip_prefix("window:")?.parse().ok()
    }
}

const FRECENCY_WEIGHT: f64 = 5.0;

/// Rank windows. Empty query lists all by frecency (then title); otherwise text score on
/// title + wm_class/app_id (keywords) plus a small frecency bonus.
pub fn rank<'a>(query: &str, windows: &'a [WindowEntry], frecency: &Frecency, now: i64) -> Vec<&'a WindowEntry> {
    let fr = |w: &WindowEntry| frecency.score(&w.row_id(), now);
    let mut v: Vec<(&WindowEntry, f64)> = if query.trim().is_empty() {
        windows.iter().map(|w| (w, fr(w))).collect()
    } else {
        windows
            .iter()
            .filter_map(|w| {
                let item = Item {
                    id: w.row_id(),
                    title: w.title.clone(),
                    keywords: vec![w.wm_class.clone(), w.app_id.clone()],
                    alias: None,
                };
                ranker::score(query, &item).map(|s| (w, s as f64 + fr(w).min(100.0) * FRECENCY_WEIGHT))
            })
            .collect()
    };
    v.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
            .then_with(|| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()))
            .then_with(|| a.0.id.cmp(&b.0.id))
    });
    v.into_iter().map(|(w, _)| w).collect()
}

pub fn to_row(w: &WindowEntry) -> Row {
    let class = if w.wm_class.is_empty() { &w.app_id } else { &w.wm_class };
    let sub = match w.workspace.parse::<i32>() {
        Ok(n) if n >= 0 => format!("{class} - Workspace {}", n + 1),
        _ => class.to_string(),
    };
    let icon = if w.app_id.is_empty() {
        w.wm_class.to_lowercase()
    } else {
        w.app_id.trim_end_matches(".desktop").to_string()
    };
    let title = if w.title.is_empty() { class.clone() } else { w.title.clone() };
    let mut r = Row::simple(RowKind::App, &w.row_id(), &title, &sub, &icon);
    r.breadcrumb = "Windows".into();
    r.actions = vec![
        ActionDesc { id: "window_activate".into(), title: "Switch to Window".into(), argv: None },
        ActionDesc { id: "window_close".into(), title: "Close Window".into(), argv: None },
    ];
    r
}

pub fn rows(query: &str, windows: &[WindowEntry], frecency: &Frecency, now: i64) -> Vec<Row> {
    rank(query, windows, frecency, now).into_iter().map(to_row).collect()
}

#[cfg(test)]
mod tests {
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
}
