//! Quicklinks: `alias -> URL template` with `{query}` and `{clipboard}` placeholders (both URL-encoded).
use super::store::Store;
use serde::{Deserialize, Serialize};

const STORE_NAME: &str = "quicklinks";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quicklink {
    pub alias: String,
    /// Display name; falls back to the alias when empty.
    #[serde(default)]
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Quicklinks {
    pub items: Vec<Quicklink>,
}

impl Quicklinks {
    pub fn load(store: &Store) -> Self {
        store.load(STORE_NAME)
    }

    pub fn save(&self, store: &Store) -> std::io::Result<()> {
        store.save(STORE_NAME, self)
    }

    /// Rejects empty/whitespace aliases, empty urls and duplicate aliases (case-insensitive).
    pub fn add(&mut self, alias: &str, name: &str, url: &str) -> Result<(), String> {
        let alias = alias.trim();
        let url = url.trim();
        if alias.is_empty() || alias.contains(char::is_whitespace) {
            return Err("alias must be a single word".into());
        }
        if url.is_empty() {
            return Err("url is empty".into());
        }
        if self.find(alias).is_some() {
            return Err(format!("alias \"{alias}\" already exists"));
        }
        self.items.push(Quicklink { alias: alias.into(), name: name.trim().into(), url: url.into() });
        Ok(())
    }

    pub fn remove(&mut self, alias: &str) -> bool {
        let n = self.items.len();
        self.items.retain(|q| !q.alias.eq_ignore_ascii_case(alias.trim()));
        self.items.len() != n
    }

    pub fn find(&self, alias: &str) -> Option<&Quicklink> {
        self.items.iter().find(|q| q.alias.eq_ignore_ascii_case(alias))
    }

    /// `gh rust` -> the `gh` quicklink with query `rust`. Clipboard is empty.
    #[cfg(test)]
    pub fn resolve(&self, input: &str) -> Option<Resolved> {
        self.resolve_with_clipboard(input, "")
    }

    pub fn resolve_with_clipboard(&self, input: &str, clipboard: &str) -> Option<Resolved> {
        let input = input.trim();
        let (alias, query) = match input.split_once(char::is_whitespace) {
            Some((a, q)) => (a, q.trim()),
            None => (input, ""),
        };
        let q = self.find(alias)?;
        let name = if q.name.is_empty() { &q.alias } else { &q.name };
        let title = if query.is_empty() { format!("Open {name}") } else { format!("Open {name}: {query}") };
        Some(Resolved { title, url: expand(&q.url, query, clipboard) })
    }
}

pub fn expand(template: &str, query: &str, clipboard: &str) -> String {
    template
        .replace("{query}", &urlencoding::encode(query))
        .replace("{clipboard}", &urlencoding::encode(clipboard))
}

/// Process argv for opening a url (never a shell string).
pub fn open_argv(url: &str) -> Vec<String> {
    vec!["xdg-open".into(), url.into()]
}

#[cfg(test)]
#[path = "../tests/quicklinks_tests.rs"]
mod tests;
