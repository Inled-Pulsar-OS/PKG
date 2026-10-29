//! Small user preferences: window size and recent searches, in `prefs.json`
//! under the user's config directory.

use crate::core::config::write_atomic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_RECENT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Data {
    window: Option<WindowState>,
    recent_searches: Vec<String>,
}

pub struct Prefs {
    path: PathBuf,
    data: Mutex<Data>,
}

impl Prefs {
    /// Loads the file, treating a missing or unreadable one as empty.
    pub fn load(path: PathBuf) -> Self {
        let data = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            path,
            data: Mutex::new(data),
        }
    }

    /// `~/.config/pulsar-store/prefs.json`.
    pub fn user() -> Self {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
        Self::load(base.join("pulsar-store/prefs.json"))
    }

    fn save(&self, data: &Data) {
        if let Ok(json) = serde_json::to_vec_pretty(data) {
            // Preferences are a convenience; failing to save is not worth an error.
            let _ = write_atomic(&self.path, &json);
        }
    }

    pub fn window(&self) -> Option<WindowState> {
        self.data.lock().unwrap().window
    }

    pub fn set_window(&self, state: WindowState) {
        let mut d = self.data.lock().unwrap();
        d.window = Some(state);
        self.save(&d);
    }

    pub fn recent_searches(&self) -> Vec<String> {
        self.data.lock().unwrap().recent_searches.clone()
    }

    /// Remembers a search, most recent first, without duplicates.
    pub fn add_recent_search(&self, query: &str) {
        let q = query.trim();
        if q.is_empty() {
            return;
        }
        let mut d = self.data.lock().unwrap();
        d.recent_searches.retain(|s| !s.eq_ignore_ascii_case(q));
        d.recent_searches.insert(0, q.to_string());
        d.recent_searches.truncate(MAX_RECENT);
        self.save(&d);
    }

    pub fn clear_recent_searches(&self) {
        let mut d = self.data.lock().unwrap();
        d.recent_searches.clear();
        self.save(&d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_searches_are_deduped_capped_and_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p/prefs.json");
        let p = Prefs::load(path.clone());
        for q in ["a", "b", "A", " ", "c"] {
            p.add_recent_search(q);
        }
        assert_eq!(p.recent_searches(), ["c", "A", "b"]);
        for i in 0..20 {
            p.add_recent_search(&format!("q{i}"));
        }
        assert_eq!(p.recent_searches().len(), MAX_RECENT);
        assert_eq!(Prefs::load(path).recent_searches()[0], "q19");
    }

    #[test]
    fn window_state_and_clear() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs.json");
        let p = Prefs::load(path.clone());
        assert_eq!(p.window(), None);
        let w = WindowState {
            width: 900,
            height: 600,
            maximized: true,
        };
        p.set_window(w);
        p.add_recent_search("x");
        p.clear_recent_searches();
        let again = Prefs::load(path);
        assert_eq!(again.window(), Some(w));
        assert!(again.recent_searches().is_empty());
    }

    #[test]
    fn corrupt_file_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs.json");
        std::fs::write(&path, "nope").unwrap();
        assert!(Prefs::load(path).recent_searches().is_empty());
    }
}
