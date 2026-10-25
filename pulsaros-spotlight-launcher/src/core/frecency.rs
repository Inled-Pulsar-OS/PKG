//! Frecency: +100 per launch, 10-day half-life. Learned terms: query -> id launch counts.
//! Time is injected (`now` = unix seconds) so tests use a fake clock.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::store::Store;

pub const LAUNCH_BONUS: f64 = 100.0;
pub const HALF_LIFE_SECS: f64 = 10.0 * 86400.0;
const FILE: &str = "frecency";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Entry {
    pub score: f64,
    pub ts: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frecency {
    #[serde(default)]
    pub items: BTreeMap<String, Entry>,
    /// normalized query -> (id -> times chosen)
    #[serde(default)]
    pub learned: BTreeMap<String, BTreeMap<String, u32>>,
}

fn norm(q: &str) -> String {
    q.trim().to_lowercase()
}

impl Frecency {
    pub fn load(store: &Store) -> Self {
        store.load(FILE)
    }

    pub fn save(&self, store: &Store) -> std::io::Result<()> {
        store.save(FILE, self)
    }

    /// Decayed score of `id` at `now`.
    pub fn score(&self, id: &str, now: i64) -> f64 {
        self.items.get(id).map_or(0.0, |e| {
            let dt = (now - e.ts).max(0) as f64;
            e.score * 0.5f64.powf(dt / HALF_LIFE_SECS)
        })
    }

    pub fn record_launch(&mut self, id: &str, query: &str, now: i64) {
        let s = self.score(id, now) + LAUNCH_BONUS;
        self.items.insert(id.to_string(), Entry { score: s, ts: now });
        let q = norm(query);
        if !q.is_empty() {
            *self.learned.entry(q).or_default().entry(id.to_string()).or_insert(0) += 1;
        }
    }

    /// Times `id` was chosen for a query that starts with the typed `query`
    /// (exact query counts double).
    pub fn learned(&self, query: &str, id: &str) -> u32 {
        let q = norm(query);
        if q.is_empty() {
            return 0;
        }
        self.learned
            .iter()
            .filter(|(k, _)| k.starts_with(&q))
            .map(|(k, m)| m.get(id).copied().unwrap_or(0) * if *k == q { 2 } else { 1 })
            .sum()
    }

    /// Ids with decayed score >= `min`, best first (ties by id), at most `n`.
    pub fn top(&self, now: i64, min: f64, n: usize) -> Vec<String> {
        let mut v: Vec<(String, f64)> = self
            .items
            .keys()
            .map(|k| (k.clone(), self.score(k, now)))
            .filter(|(_, s)| *s >= min)
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
        v.into_iter().take(n).map(|x| x.0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::store::temp_store;

    const DAY: i64 = 86400;

    #[test]
    fn launch_adds_100() {
        let mut f = Frecency::default();
        f.record_launch("a", "", 1000);
        assert_eq!(f.score("a", 1000), 100.0);
        f.record_launch("a", "", 1000);
        assert_eq!(f.score("a", 1000), 200.0);
    }

    #[test]
    fn half_life_ten_days() {
        let mut f = Frecency::default();
        f.record_launch("a", "", 0);
        assert!((f.score("a", 10 * DAY) - 50.0).abs() < 1e-9);
        assert!((f.score("a", 20 * DAY) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn decay_applied_before_adding() {
        let mut f = Frecency::default();
        f.record_launch("a", "", 0);
        f.record_launch("a", "", 10 * DAY);
        assert!((f.score("a", 10 * DAY) - 150.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_is_zero_and_clock_backwards_safe() {
        let mut f = Frecency::default();
        assert_eq!(f.score("x", 5), 0.0);
        f.record_launch("a", "", 100);
        assert_eq!(f.score("a", 50), 100.0);
    }

    #[test]
    fn learned_prefix_and_exact() {
        let mut f = Frecency::default();
        f.record_launch("ff", "ff", 0);
        f.record_launch("ff", "ff", 0);
        assert_eq!(f.learned("ff", "ff"), 4);
        assert_eq!(f.learned("f", "ff"), 2);
        assert_eq!(f.learned("fi", "ff"), 0);
        assert_eq!(f.learned("", "ff"), 0);
        assert_eq!(f.learned("ff", "other"), 0);
    }

    #[test]
    fn learned_is_case_insensitive_and_skips_empty_query() {
        let mut f = Frecency::default();
        f.record_launch("a", " FF ", 0);
        f.record_launch("a", "", 0);
        assert_eq!(f.learned("ff", "a"), 2);
        assert_eq!(f.learned.len(), 1);
    }

    #[test]
    fn top_filters_and_sorts() {
        let mut f = Frecency::default();
        f.record_launch("a", "", 0);
        f.record_launch("b", "", 0);
        f.record_launch("b", "", 0);
        f.record_launch("old", "", -100 * DAY);
        assert_eq!(f.top(0, 50.0, 6), vec!["b", "a"]);
        assert_eq!(f.top(0, 50.0, 1), vec!["b"]);
    }

    #[test]
    fn persists() {
        let s = temp_store("frec");
        let mut f = Frecency::default();
        f.record_launch("a", "q", 7);
        f.save(&s).unwrap();
        let g = Frecency::load(&s);
        assert_eq!(g.score("a", 7), 100.0);
        assert_eq!(g.learned("q", "a"), 2);
    }
}
