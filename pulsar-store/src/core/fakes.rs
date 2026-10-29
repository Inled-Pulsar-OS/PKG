//! In-memory implementations of the ports, for tests and for the source contract suite.

use crate::core::error::{Result, StoreError};
use crate::core::model::{Catalog, Edition, Item, LedgerEntry, Phase, Progress, SourceKind};
use crate::core::ports::{
    CancelToken, CatalogProvider, InstallOutcome, Ledger, PackageSource, ProgressSink,
};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct VecSink(pub Mutex<Vec<Progress>>);

impl ProgressSink for VecSink {
    fn report(&self, progress: Progress) {
        self.0.lock().unwrap().push(progress);
    }
}

pub struct FakeSource {
    kind: SourceKind,
    installed: Mutex<HashMap<String, String>>,
    pub fail_with: Mutex<Option<String>>,
}

impl FakeSource {
    pub fn new(kind: SourceKind) -> Self {
        Self {
            kind,
            installed: Mutex::default(),
            fail_with: Mutex::default(),
        }
    }

    fn check(&self, cancel: &CancelToken) -> Result<()> {
        if cancel.is_cancelled() {
            return Err(StoreError::Cancelled);
        }
        if let Some(msg) = self.fail_with.lock().unwrap().clone() {
            return Err(StoreError::CommandFailed(msg));
        }
        Ok(())
    }
}

impl PackageSource for FakeSource {
    fn kind(&self) -> SourceKind {
        self.kind
    }

    fn is_available(&self) -> bool {
        true
    }

    fn installed_version(&self, item: &Item) -> Result<Option<String>> {
        Ok(self.installed.lock().unwrap().get(&item.id).cloned())
    }

    fn install(
        &self,
        item: &Item,
        _edition: Option<&Edition>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome> {
        progress.report(Progress {
            phase: Phase::Preparing,
            fraction: None,
        });
        self.check(cancel)?;
        progress.report(Progress {
            phase: Phase::Installing,
            fraction: Some(1.0),
        });
        let version = item.version.clone().unwrap_or_else(|| "0".into());
        self.installed
            .lock()
            .unwrap()
            .insert(item.id.clone(), version);
        Ok(InstallOutcome::default())
    }

    fn uninstall(&self, item: &Item) -> Result<()> {
        match self.installed.lock().unwrap().remove(&item.id) {
            Some(_) => Ok(()),
            None => Err(StoreError::NotFound(item.id.clone())),
        }
    }

    fn update(
        &self,
        item: &Item,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome> {
        self.install(item, None, progress, cancel)
    }
}

#[derive(Default)]
pub struct MemoryLedger(Mutex<HashMap<String, LedgerEntry>>);

impl Ledger for MemoryLedger {
    fn get(&self, id: &str) -> Result<Option<LedgerEntry>> {
        Ok(self.0.lock().unwrap().get(id).cloned())
    }
    fn record(&self, entry: LedgerEntry) -> Result<()> {
        self.0.lock().unwrap().insert(entry.id.clone(), entry);
        Ok(())
    }
    fn forget(&self, id: &str) -> Result<()> {
        self.0.lock().unwrap().remove(id);
        Ok(())
    }
}

pub struct StaticCatalog(pub Catalog);

impl CatalogProvider for StaticCatalog {
    fn load(&self) -> Result<Catalog> {
        Ok(self.0.clone())
    }
    fn refresh(&self, _force: bool) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> Item {
        serde_json::from_str(r#"{"id":"x","type":"flatpak","version":"1.2"}"#).unwrap()
    }

    #[test]
    fn install_then_uninstall() {
        let s = FakeSource::new(SourceKind::Flatpak);
        let sink = VecSink::default();
        s.install(&item(), None, &sink, &CancelToken::new())
            .unwrap();
        assert_eq!(
            s.installed_version(&item()).unwrap().as_deref(),
            Some("1.2")
        );
        assert_eq!(sink.0.lock().unwrap().len(), 2);
        s.uninstall(&item()).unwrap();
        assert!(matches!(s.uninstall(&item()), Err(StoreError::NotFound(_))));
    }

    #[test]
    fn cancelled_install_leaves_nothing_installed() {
        let s = FakeSource::new(SourceKind::Flatpak);
        let cancel = CancelToken::new();
        cancel.cancel();
        let r = s.install(&item(), None, &VecSink::default(), &cancel);
        assert!(matches!(r, Err(StoreError::Cancelled)));
        assert_eq!(s.installed_version(&item()).unwrap(), None);
    }

    #[test]
    fn ledger_round_trip() {
        let l = MemoryLedger::default();
        let e = LedgerEntry {
            id: "x".into(),
            edition: None,
            package_name: None,
            version: None,
        };
        l.record(e.clone()).unwrap();
        assert_eq!(l.get("x").unwrap(), Some(e));
        l.forget("x").unwrap();
        assert_eq!(l.get("x").unwrap(), None);
    }
}
