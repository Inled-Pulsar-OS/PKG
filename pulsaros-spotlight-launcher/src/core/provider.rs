//! Provider abstraction for Spotlight search (GTK-free).
//!
//! A `Provider` turns a `Query` into rows. `instant` is cheap and runs on the UI thread;
//! `background` may hand back a `Job` that runs off-thread and returns more rows later.
//! `Registry` holds providers in registration order and merges their rows, first id wins.
use std::collections::HashSet;

use super::root_search::Row;

/// One search request. `text` is `raw` trimmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    pub raw: String,
    pub text: String,
}

impl Query {
    pub fn new(raw: &str) -> Self {
        Query { raw: raw.to_string(), text: raw.trim().to_string() }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Slow work run off the UI thread; returns rows to merge in.
pub type Job = Box<dyn FnOnce() -> Vec<Row> + Send>;

pub trait Provider {
    /// Stable name, used for logs and tests.
    fn id(&self) -> &str;
    fn instant(&self, query: &Query) -> Vec<Row>;
    fn background(&self, _query: &Query) -> Option<Job> {
        None
    }
}

#[derive(Default)]
pub struct Registry {
    providers: Vec<Box<dyn Provider>>,
}

impl Registry {
    pub fn register(&mut self, provider: Box<dyn Provider>) {
        self.providers.push(provider);
    }

    /// Rows of every provider in registration order; a repeated row id keeps its first row.
    pub fn instant(&self, query: &Query) -> Vec<Row> {
        let mut seen = HashSet::new();
        self.providers
            .iter()
            .flat_map(|p| p.instant(query))
            .filter(|r| seen.insert(r.id.clone()))
            .collect()
    }

    /// Background jobs of every provider that has one, in registration order.
    pub fn background(&self, query: &Query) -> Vec<Job> {
        self.providers.iter().filter_map(|p| p.background(query)).collect()
    }
}

#[cfg(test)]
#[path = "../tests/provider_tests.rs"]
mod tests;
