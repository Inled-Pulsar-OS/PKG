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
}

#[cfg(test)]
#[path = "../tests/settings_tests.rs"]
mod tests;
