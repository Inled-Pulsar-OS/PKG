//! User settings: aliases, favorites, disabled ids, search history, web fallbacks.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::store::Store;

const FILE_SETTINGS: &str = "settings";
const FILE_HISTORY: &str = "history";
pub const HISTORY_CAP: usize = 50;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fallback {
    pub name: String,
    /// `{q}` is replaced with the URL-encoded query.
    pub url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Data {
    #[serde(default)]
    aliases: BTreeMap<String, String>, // id -> alias (lowercase)
    #[serde(default)]
    favorites: Vec<String>,
    #[serde(default)]
    disabled: Vec<String>,
    #[serde(default = "default_fallbacks")]
    fallbacks: Vec<Fallback>,
}

fn default_fallbacks() -> Vec<Fallback> {
    vec![
        Fallback { name: "Google".into(), url: "https://www.google.com/search?q={q}".into() },
        Fallback { name: "DuckDuckGo".into(), url: "https://duckduckgo.com/?q={q}".into() },
    ]
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct HistoryData {
    #[serde(default)]
    items: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Settings {
    data: Data,
    history: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { data: Data { fallbacks: default_fallbacks(), ..Data::default() }, history: Vec::new() }
    }
}

impl Settings {
    pub fn load(store: &Store) -> Self {
        let mut data: Data = if store.path(FILE_SETTINGS).exists() {
            store.load(FILE_SETTINGS)
        } else {
            Settings::default().data
        };
        if data.fallbacks.is_empty() {
            data.fallbacks = default_fallbacks();
        }
        let h: HistoryData = store.load(FILE_HISTORY);
        Settings { data, history: h.items }
    }

    pub fn save(&self, store: &Store) -> std::io::Result<()> {
        store.save(FILE_SETTINGS, &self.data)?;
        store.save(FILE_HISTORY, &HistoryData { items: self.history.clone() })
    }

    // ---- aliases ----
    pub fn alias(&self, id: &str) -> Option<&str> {
        self.data.aliases.get(id).map(|s| s.as_str())
    }

    /// Empty alias removes it. Duplicate (owned by another id) is rejected.
    pub fn set_alias(&mut self, id: &str, alias: &str) -> Result<(), String> {
        let a = alias.trim().to_lowercase();
        if a.is_empty() {
            self.data.aliases.remove(id);
            return Ok(());
        }
        if let Some((owner, _)) = self.data.aliases.iter().find(|(k, v)| **v == a && k.as_str() != id) {
            return Err(format!("Alias \"{a}\" is already used by {owner}"));
        }
        self.data.aliases.insert(id.to_string(), a);
        Ok(())
    }

    // ---- favorites ----
    pub fn favorites(&self) -> &[String] {
        &self.data.favorites
    }
    pub fn is_favorite(&self, id: &str) -> bool {
        self.data.favorites.iter().any(|f| f == id)
    }
    /// Returns the new state (true = now a favorite).
    pub fn toggle_favorite(&mut self, id: &str) -> bool {
        if let Some(i) = self.data.favorites.iter().position(|f| f == id) {
            self.data.favorites.remove(i);
            false
        } else {
            self.data.favorites.push(id.to_string());
            true
        }
    }
    /// Move by `delta` positions (negative = up), clamped. No-op if not a favorite.
    pub fn move_favorite(&mut self, id: &str, delta: i32) {
        let Some(i) = self.data.favorites.iter().position(|f| f == id) else { return };
        let j = (i as i64 + delta as i64).clamp(0, self.data.favorites.len() as i64 - 1) as usize;
        let item = self.data.favorites.remove(i);
        self.data.favorites.insert(j, item);
    }

    // ---- disabled ----
    pub fn disabled(&self) -> &[String] {
        &self.data.disabled
    }
    pub fn is_disabled(&self, id: &str) -> bool {
        self.data.disabled.iter().any(|d| d == id)
    }
    pub fn disable(&mut self, id: &str) {
        if !self.is_disabled(id) {
            self.data.disabled.push(id.to_string());
        }
    }
    /// Re-enable; also serves as Undo for `disable`.
    pub fn enable(&mut self, id: &str) {
        self.data.disabled.retain(|d| d != id);
    }

    // ---- fallbacks ----
    pub fn fallbacks(&self) -> &[Fallback] {
        &self.data.fallbacks
    }

    // ---- history ----
    pub fn history(&self) -> &[String] {
        &self.history
    }
    /// Newest first, deduped (case-insensitive), capped at `HISTORY_CAP`.
    pub fn push_history(&mut self, q: &str) {
        let q = q.trim();
        if q.is_empty() {
            return;
        }
        self.history.retain(|h| !h.eq_ignore_ascii_case(q));
        self.history.insert(0, q.to_string());
        self.history.truncate(HISTORY_CAP);
    }
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::store::temp_store;

    #[test]
    fn alias_set_get_clear() {
        let mut s = Settings::default();
        s.set_alias("a", " FF ").unwrap();
        assert_eq!(s.alias("a"), Some("ff"));
        s.set_alias("a", "").unwrap();
        assert_eq!(s.alias("a"), None);
    }

    #[test]
    fn alias_duplicate_rejected_same_id_ok() {
        let mut s = Settings::default();
        s.set_alias("a", "ff").unwrap();
        let e = s.set_alias("b", "FF").unwrap_err();
        assert!(e.contains("already used"));
        assert_eq!(s.alias("b"), None);
        s.set_alias("a", "ff").unwrap();
        s.set_alias("a", "fx").unwrap();
        s.set_alias("b", "ff").unwrap();
    }

    #[test]
    fn favorites_toggle_and_reorder() {
        let mut s = Settings::default();
        assert!(s.toggle_favorite("a"));
        s.toggle_favorite("b");
        s.toggle_favorite("c");
        assert_eq!(s.favorites(), ["a", "b", "c"]);
        s.move_favorite("c", -1);
        assert_eq!(s.favorites(), ["a", "c", "b"]);
        s.move_favorite("c", -5);
        assert_eq!(s.favorites(), ["c", "a", "b"]);
        s.move_favorite("c", 99);
        assert_eq!(s.favorites(), ["a", "b", "c"]);
        s.move_favorite("zzz", 1);
        assert!(!s.toggle_favorite("b"));
        assert!(!s.is_favorite("b"));
        assert_eq!(s.favorites(), ["a", "c"]);
    }

    #[test]
    fn move_in_empty_is_safe() {
        let mut s = Settings::default();
        s.move_favorite("a", 1);
    }

    #[test]
    fn disable_undo() {
        let mut s = Settings::default();
        s.disable("a");
        s.disable("a");
        assert_eq!(s.disabled(), ["a"]);
        assert!(s.is_disabled("a"));
        s.enable("a");
        assert!(!s.is_disabled("a"));
        assert!(s.disabled().is_empty());
    }

    #[test]
    fn history_dedupe_cap_clear() {
        let mut s = Settings::default();
        s.push_history("one");
        s.push_history("two");
        s.push_history("ONE");
        s.push_history("  ");
        assert_eq!(s.history(), ["ONE", "two"]);
        for i in 0..100 {
            s.push_history(&format!("q{i}"));
        }
        assert_eq!(s.history().len(), HISTORY_CAP);
        assert_eq!(s.history()[0], "q99");
        s.clear_history();
        assert!(s.history().is_empty());
    }

    #[test]
    fn default_fallbacks_present() {
        let s = Settings::default();
        assert_eq!(s.fallbacks().len(), 2);
        assert_eq!(s.fallbacks()[0].name, "Google");
    }

    #[test]
    fn persists_roundtrip() {
        let st = temp_store("settings");
        let mut s = Settings::default();
        s.set_alias("a", "x").unwrap();
        s.toggle_favorite("a");
        s.disable("b");
        s.push_history("hello");
        s.save(&st).unwrap();
        let l = Settings::load(&st);
        assert_eq!(l.alias("a"), Some("x"));
        assert_eq!(l.favorites(), ["a"]);
        assert!(l.is_disabled("b"));
        assert_eq!(l.history(), ["hello"]);
        assert_eq!(l.fallbacks().len(), 2);
    }

    #[test]
    fn load_corrupt_gives_defaults() {
        let st = temp_store("settings-bad");
        std::fs::create_dir_all(st.path("settings").parent().unwrap()).unwrap();
        std::fs::write(st.path("settings"), "garbage").unwrap();
        let l = Settings::load(&st);
        assert!(l.favorites().is_empty());
        assert_eq!(l.fallbacks().len(), 2);
    }

    #[test]
    fn custom_fallbacks_survive() {
        let st = temp_store("settings-fb");
        std::fs::create_dir_all(st.path("settings").parent().unwrap()).unwrap();
        std::fs::write(
            st.path("settings"),
            r#"{"version":1,"data":{"fallbacks":[{"name":"Kagi","url":"https://kagi.com/search?q={q}"}]}}"#,
        )
        .unwrap();
        let l = Settings::load(&st);
        assert_eq!(l.fallbacks().len(), 1);
        assert_eq!(l.fallbacks()[0].name, "Kagi");
    }
}
