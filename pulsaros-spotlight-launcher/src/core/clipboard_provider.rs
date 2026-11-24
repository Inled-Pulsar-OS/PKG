//! Clipboard history as a `Provider`: rows `clipboard://<index>` for entries matching the query.
use super::provider::{Provider, Query};
use super::root_search::{Row, RowKind};

pub const MIME: &str = "text/plain-clipboard";

/// Rows for the history entries (newest first) that contain `query`; an empty query matches all.
pub fn clipboard_rows(items: &[String], query: &str) -> Vec<Row> {
    let q = query.trim().to_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, text)| q.is_empty() || text.to_lowercase().contains(&q))
        .map(|(idx, text)| {
            let mut title = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or(text).to_string();
            if title.chars().count() > 60 {
                title = title.chars().take(57).collect::<String>() + "...";
            }
            let mut preview = text.replace('\n', " ⏎ ");
            if preview.chars().count() > 100 {
                preview = preview.chars().take(97).collect::<String>() + "...";
            }
            Row::simple(RowKind::Clipboard, &format!("clipboard://{idx}"), &title, &preview, "edit-paste-symbolic")
        })
        .collect()
}

pub struct ClipboardProvider {
    items: Box<dyn Fn() -> Vec<String>>,
    limit: usize,
}

impl ClipboardProvider {
    /// `items` yields the history texts, newest first; at most `limit` rows come back.
    pub fn new(items: impl Fn() -> Vec<String> + 'static, limit: usize) -> Self {
        ClipboardProvider { items: Box::new(items), limit }
    }
}

impl Provider for ClipboardProvider {
    fn instant(&self, query: &Query) -> Vec<Row> {
        let mut rows = clipboard_rows(&(self.items)(), &query.text);
        rows.truncate(self.limit);
        rows
    }
}

#[cfg(test)]
#[path = "../tests/clipboard_provider_tests.rs"]
mod tests;
