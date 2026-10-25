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
mod tests {
    use super::*;
    use crate::core::store::temp_store;

    fn sample() -> Quicklinks {
        let mut q = Quicklinks::default();
        q.add("gh", "GitHub search", "https://github.com/search?q={query}").unwrap();
        q.add("docs", "", "https://docs.rs").unwrap();
        q.add("paste", "Paste", "https://x.test/?c={clipboard}&q={query}").unwrap();
        q
    }

    #[test]
    fn resolves_and_encodes() {
        let r = sample().resolve("gh rust & c++").unwrap();
        assert_eq!(r.url, "https://github.com/search?q=rust%20%26%20c%2B%2B");
        assert_eq!(r.title, "Open GitHub search: rust & c++");
    }

    #[test]
    fn case_insensitive_alias() {
        assert!(sample().resolve("GH x").is_some());
    }

    #[test]
    fn unknown_alias_none() {
        assert!(sample().resolve("zz x").is_none());
        assert!(sample().resolve("").is_none());
        assert!(sample().resolve("   ").is_none());
    }

    #[test]
    fn missing_placeholder_ignores_query() {
        let r = sample().resolve("docs serde").unwrap();
        assert_eq!(r.url, "https://docs.rs");
        assert_eq!(r.title, "Open docs: serde");
    }

    #[test]
    fn bare_alias_empty_query() {
        let r = sample().resolve("gh").unwrap();
        assert_eq!(r.url, "https://github.com/search?q=");
        assert_eq!(r.title, "Open GitHub search");
    }

    #[test]
    fn clipboard_placeholder() {
        let r = sample().resolve_with_clipboard("paste a b", "x y/z").unwrap();
        assert_eq!(r.url, "https://x.test/?c=x%20y%2Fz&q=a%20b");
        assert_eq!(sample().resolve("paste q").unwrap().url, "https://x.test/?c=&q=q");
    }

    #[test]
    fn duplicate_alias_rejected() {
        let mut q = sample();
        assert!(q.add("GH", "", "https://a").is_err());
        assert_eq!(q.items.len(), 3);
    }

    #[test]
    fn invalid_add_rejected() {
        let mut q = Quicklinks::default();
        assert!(q.add("", "", "https://a").is_err());
        assert!(q.add("a b", "", "https://a").is_err());
        assert!(q.add("a", "", "  ").is_err());
    }

    #[test]
    fn remove_works() {
        let mut q = sample();
        assert!(q.remove("gh"));
        assert!(!q.remove("gh"));
        assert!(q.resolve("gh x").is_none());
    }

    #[test]
    fn store_roundtrip() {
        let s = temp_store("quicklinks");
        assert!(Quicklinks::load(&s).items.is_empty());
        sample().save(&s).unwrap();
        assert_eq!(Quicklinks::load(&s), sample());
    }

    #[test]
    fn open_argv_is_xdg_open() {
        assert_eq!(open_argv("https://a.b/?q=1 2"), vec!["xdg-open", "https://a.b/?q=1 2"]);
    }
}
