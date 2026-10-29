//! Seams between logic and infrastructure. Adapters implement these traits;
//! `core::fakes` has in-memory versions for tests.

use crate::core::error::Result;
use crate::core::model::{Catalog, Edition, Item, LedgerEntry, Progress, SourceKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Receives progress reports from a running operation. May be called from any thread.
pub trait ProgressSink: Send + Sync {
    fn report(&self, progress: Progress);
}

/// Cooperative cancellation. Clones share state. Sources check it between steps
/// and return `StoreError::Cancelled` when set.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// What a successful install reports back, so the ledger can record it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallOutcome {
    /// Native package name, when the source knows it.
    pub package_name: Option<String>,
}

/// One way to install things: Flatpak, system packages, GNOME extensions, Sayri.
/// Methods block, so call them from a worker thread.
pub trait PackageSource: Send + Sync {
    fn kind(&self) -> SourceKind;
    /// False when the tooling this source needs is missing on the host.
    fn is_available(&self) -> bool;
    /// `Some` when installed; `"installed"` when the version is unknown.
    fn installed_version(&self, item: &Item) -> Result<Option<String>>;
    fn is_installed(&self, item: &Item) -> Result<bool> {
        Ok(self.installed_version(item)?.is_some())
    }
    /// Names of all installed native packages. Only the system source implements this.
    fn list_installed(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    fn install(
        &self,
        item: &Item,
        edition: Option<&Edition>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome>;
    fn uninstall(&self, item: &Item) -> Result<()>;
    fn update(
        &self,
        item: &Item,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome>;
}

/// Loads the catalog from cache, network, or the bundled copy.
pub trait CatalogProvider: Send + Sync {
    fn load(&self) -> Result<Catalog>;
    /// Fetches a fresh catalog; returns true when the stored catalog changed.
    fn refresh(&self, force: bool) -> Result<bool>;
}

/// Records what the store installed, since some sources cannot tell us.
pub trait Ledger: Send + Sync {
    fn get(&self, id: &str) -> Result<Option<LedgerEntry>>;
    fn record(&self, entry: LedgerEntry) -> Result<()>;
    fn forget(&self, id: &str) -> Result<()>;
}
