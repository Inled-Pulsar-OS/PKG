//! File search, web links and browser history as `Provider`s.
//! Rows are built without GTK; `search::run_job` adds the open payload on the main thread.
use crate::category::Category;
use crate::core::provider::{Job, Provider, Query};
use crate::core::root_search::{Row, RowKind};
use crate::core::worker::JobKind;
use crate::search::{query_browser_history, search_nautilus, DEFAULT_LIMIT};

/// Nautilus file search, run in the background. Its section is named after the category.
pub struct FilesProvider {
    category: Category,
}

impl FilesProvider {
    pub fn new(category: Category) -> Self {
        FilesProvider { category }
    }
}

impl Provider for FilesProvider {
    fn id(&self) -> &str {
        "files"
    }

    fn instant(&self, _query: &Query) -> Vec<Row> {
        Vec::new()
    }

    fn background(&self, query: &Query) -> Option<Job> {
        let category = self.category;
        if !category.needs_file_search() || (query.is_empty() && !category.is_file_tab()) {
            return None;
        }
        let raw = query.raw.clone();
        Some(Job {
            kind: JobKind::Files,
            title: category.section_title(),
            run: Box::new(move |ctx| {
                search_nautilus(ctx, &raw, category, DEFAULT_LIMIT).into_iter().map(|r| r.row).collect()
            }),
        })
    }
}

/// The Google search link for the query, or a few quick links when it is empty. Web mode only.
pub struct WebProvider {
    category: Category,
}

impl WebProvider {
    pub fn new(category: Category) -> Self {
        WebProvider { category }
    }
}

fn link(id: &str, title: &str, subtitle: &str) -> Row {
    Row::simple(RowKind::File, id, title, subtitle, "text-x-generic")
}

impl Provider for WebProvider {
    fn id(&self) -> &str {
        "web"
    }

    fn instant(&self, query: &Query) -> Vec<Row> {
        if self.category != Category::Web {
            return Vec::new();
        }
        if !query.is_empty() {
            let q = &query.text;
            let url = format!("https://www.google.com/search?q={}", urlencoding::encode(q));
            return vec![link(&url, &format!("Search '{q}' on the Web"), &format!("Open Google search for '{q}'"))];
        }
        [("https://www.google.com", "Google"), ("https://github.com", "GitHub"), ("https://youtube.com", "YouTube"), ("https://wikipedia.org", "Wikipedia")]
            .into_iter()
            .map(|(url, title)| link(url, title, url))
            .collect()
    }
}

/// Firefox history matches, run in the background: 6 in Web mode, 3 under a typed query in All.
pub struct HistoryProvider {
    category: Category,
}

impl HistoryProvider {
    pub fn new(category: Category) -> Self {
        HistoryProvider { category }
    }
}

impl Provider for HistoryProvider {
    fn id(&self) -> &str {
        "history"
    }

    fn instant(&self, _query: &Query) -> Vec<Row> {
        Vec::new()
    }

    fn background(&self, query: &Query) -> Option<Job> {
        let web = self.category == Category::Web;
        if !web && (self.category != Category::All || query.is_empty()) {
            return None;
        }
        let (raw, limit) = (query.raw.clone(), if web { 6 } else { 3 });
        Some(Job {
            kind: JobKind::History,
            title: if web { "Web" } else { "Results" },
            run: Box::new(move |ctx| query_browser_history(ctx, &raw, limit).into_iter().map(|r| r.row).collect()),
        })
    }
}

#[cfg(test)]
#[path = "tests/search_providers_tests.rs"]
mod tests;
