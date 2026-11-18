//! Frecency: +100 per launch, 10-day half-life. Learned terms: query -> id launch counts.
//! Time is injected (`now` = unix seconds) so tests use a fake clock.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

use super::store::Store;

pub const LAUNCH_BONUS: f64 = 100.0;
pub const HALF_LIFE_SECS: f64 = 10.0 * 86400.0;
/// Cap on distinct `learned` queries kept; least-recently-used ones are
/// evicted beyond this so ranking-pass latency stays flat over time.
pub const LEARNED_CAP: usize = 500;
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
    /// `learned` keys ordered oldest-to-most-recently-used, for LRU eviction.
    #[serde(default)]
    learned_recency: VecDeque<String>,
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
            *self.learned.entry(q.clone()).or_default().entry(id.to_string()).or_insert(0) += 1;
            self.touch_learned(q);
        }
    }

    /// Marks `q` as most-recently-used and evicts the least-recently-used
    /// `learned` entries beyond `LEARNED_CAP`.
    ///
    /// `learned_recency` can be shorter than `learned` right after loading a
    /// file persisted before this field existed (it deserializes to an empty
    /// queue via `#[serde(default)]` while `learned` already holds legacy
    /// keys). In that case every legacy key not yet in the queue is backfilled
    /// first, so eviction below treats them as real LRU candidates instead of
    /// leaving them permanently un-evictable while freshly-typed queries get
    /// evicted immediately.
    fn touch_learned(&mut self, q: String) {
        if self.learned_recency.len() < self.learned.len() {
            let present: std::collections::HashSet<&String> = self.learned_recency.iter().collect();
            let missing: Vec<String> = self
                .learned
                .keys()
                .filter(|k| *k != &q && !present.contains(k))
                .cloned()
                .collect();
            self.learned_recency.extend(missing);
        }
        self.learned_recency.retain(|k| k != &q);
        self.learned_recency.push_back(q);
        while self.learned.len() > LEARNED_CAP {
            let Some(oldest) = self.learned_recency.pop_front() else { break };
            self.learned.remove(&oldest);
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
#[path = "../tests/frecency_tests.rs"]
mod tests;
