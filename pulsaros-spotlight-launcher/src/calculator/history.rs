//! Calculator history: JSON file in the XDG data dir, capped by rows and age.

use super::CalcResult;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_ROWS: usize = 4096;
pub const MAX_AGE_SECS: u64 = 90 * 86400;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HistoryRow {
    pub id: u64,
    pub ts: u64,
    pub question: String,
    pub answer: String,
    pub caption_question: String,
    pub caption_answer: String,
}

pub struct CalcHistory {
    path: PathBuf,
    rows: Vec<HistoryRow>, // oldest first
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl CalcHistory {
    pub fn default_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from(".local/share"))
            .join("pulsaros-spotlight/db/calculator.json")
    }

    pub fn open() -> Self {
        Self::open_at(Self::default_path())
    }

    pub fn open_at(path: PathBuf) -> Self {
        let rows = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let mut h = Self { path, rows };
        h.prune(now());
        h
    }

    fn prune(&mut self, now: u64) {
        self.rows.retain(|r| now.saturating_sub(r.ts) <= MAX_AGE_SECS);
        if self.rows.len() > MAX_ROWS {
            let drop = self.rows.len() - MAX_ROWS;
            self.rows.drain(..drop);
        }
    }

    fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp: &Path = &self.path.with_extension("json.tmp");
        std::fs::write(tmp, serde_json::to_string(&self.rows)?)?;
        std::fs::rename(tmp, &self.path)
    }

    pub fn add(&mut self, r: &CalcResult) -> u64 {
        self.add_at(r, now())
    }

    pub fn add_at(&mut self, r: &CalcResult, ts: u64) -> u64 {
        let id = self.push(r, ts);
        let _ = self.save();
        id
    }

    pub(super) fn push(&mut self, r: &CalcResult, ts: u64) -> u64 {
        // Re-running the same question replaces the old row.
        self.rows.retain(|x| !(x.question == r.question && x.answer == r.answer));
        let id = self.rows.iter().map(|x| x.id).max().unwrap_or(0) + 1;
        self.rows.push(HistoryRow {
            id,
            ts,
            question: r.question.clone(),
            answer: r.answer.clone(),
            caption_question: r.caption_question.clone(),
            caption_answer: r.caption_answer.clone(),
        });
        self.prune(ts);
        id
    }

    pub fn delete(&mut self, id: u64) -> bool {
        let before = self.rows.len();
        self.rows.retain(|r| r.id != id);
        let changed = self.rows.len() != before;
        if changed {
            let _ = self.save();
        }
        changed
    }

    pub fn delete_all(&mut self) {
        self.rows.clear();
        let _ = self.save();
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Newest first; empty query returns everything.
    pub fn search(&self, q: &str) -> Vec<HistoryRow> {
        let q = q.trim().to_lowercase();
        self.rows
            .iter()
            .rev()
            .filter(|r| {
                q.is_empty()
                    || r.question.to_lowercase().contains(&q)
                    || r.answer.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }
}
