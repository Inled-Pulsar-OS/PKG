//! State of the single search worker thread (GTK-free), shared with provider jobs.
use std::path::PathBuf;

use rusqlite::Connection as SqliteConnection;
use zbus::blocking::Connection as ZbusConnection;

/// Which stream a job belongs to. Only the newest queued job of each kind is worth running.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobKind {
    Files,
    History,
}

/// State owned by the single worker thread, reused across jobs.
#[derive(Default)]
pub struct WorkerCtx {
    pub session: Option<ZbusConnection>,
    /// Read-only handles to each `places.sqlite`, opened on first use.
    pub history_dbs: std::collections::HashMap<PathBuf, SqliteConnection>,
}

impl WorkerCtx {
    /// The session bus connection: created once, kept until a call on it fails.
    pub fn session(&mut self) -> Option<&ZbusConnection> {
        if self.session.is_none() {
            self.session = ZbusConnection::session().ok();
        }
        self.session.as_ref()
    }
}
