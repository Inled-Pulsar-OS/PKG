//! The install pipeline: picks the right source and edition, keeps the ledger,
//! and runs work on a background thread with a progress stream and a cancel handle.

use crate::core::editions::{best_edition, editions};
use crate::core::error::{Result, StoreError};
use crate::core::host::Distro;
use crate::core::model::{
    Edition, EditionKey, InstallState, Item, LedgerEntry, Progress, SourceKind,
};
use crate::core::ports::{CancelToken, InstallOutcome, Ledger, PackageSource, ProgressSink};
use crate::core::version;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// The five sources the pipeline chooses between.
pub struct Sources {
    pub flatpak: Arc<dyn PackageSource>,
    pub system: Arc<dyn PackageSource>,
    pub gnome_ext: Arc<dyn PackageSource>,
    pub sayri_skill: Arc<dyn PackageSource>,
    pub sayri_plugin: Arc<dyn PackageSource>,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub item: Item,
    pub installed: String,
    pub available: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Install, optionally forcing an edition.
    Install(Option<EditionKey>),
    Uninstall,
    Update,
}

pub enum Event {
    Progress(Progress),
    /// Always the last event.
    Finished(Result<()>),
}

/// A running background operation.
pub struct InstallHandle {
    pub events: Receiver<Event>,
    pub cancel: CancelToken,
}

struct ChannelSink(Mutex<Sender<Event>>);

impl ProgressSink for ChannelSink {
    fn report(&self, progress: Progress) {
        let _ = self.0.lock().unwrap().send(Event::Progress(progress));
    }
}

pub struct Installer {
    sources: Sources,
    ledger: Arc<dyn Ledger>,
    distro: Distro,
}

/// A copy of `item` that points at a native package by name.
fn native_probe(item: &Item, name: &str) -> Item {
    let mut p = item.clone();
    p.kind = SourceKind::System;
    p.package_name = Some(name.to_string());
    p
}

impl Installer {
    pub fn new(sources: Sources, ledger: Arc<dyn Ledger>, distro: Distro) -> Self {
        Self {
            sources,
            ledger,
            distro,
        }
    }

    fn native_edition(&self) -> EditionKey {
        match self.distro {
            Distro::Arch => EditionKey::Arch,
            Distro::Debian => EditionKey::Debian,
        }
    }

    fn source_for(&self, kind: SourceKind) -> Result<&Arc<dyn PackageSource>> {
        match kind {
            SourceKind::Flatpak => Ok(&self.sources.flatpak),
            SourceKind::System => Ok(&self.sources.system),
            SourceKind::GnomeExtension => Ok(&self.sources.gnome_ext),
            SourceKind::SayriSkill => Ok(&self.sources.sayri_skill),
            SourceKind::SayriPlugin => Ok(&self.sources.sayri_plugin),
            SourceKind::Unknown => Err(StoreError::Unsupported("unknown item type".into())),
        }
    }

    /// Names a native package for this item might have: explicit name, id, and
    /// the file names of its Arch and Debian download URLs.
    fn native_candidates(&self, item: &Item) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        names.extend(item.package_name.clone());
        names.push(item.id.clone());
        for e in editions(item) {
            if matches!(e.key, EditionKey::Arch | EditionKey::Debian) {
                let base = e
                    .url
                    .split('?')
                    .next()
                    .unwrap_or("")
                    .rsplit('/')
                    .next()
                    .unwrap_or("");
                let base = [".pkg.tar.zst", ".pkg.tar.xz", ".pkg.tar", ".deb"]
                    .iter()
                    .find_map(|s| base.strip_suffix(s))
                    .unwrap_or(base);
                names.push(base.to_string());
            }
        }
        let mut seen: Vec<String> = Vec::new();
        for n in names.into_iter().filter(|n| !n.is_empty()) {
            if !seen.contains(&n) {
                seen.push(n);
            }
        }
        seen
    }

    /// Finds a native package that matches the item: explicit names first, then
    /// a scan of installed packages by name similarity (names of 4+ characters).
    fn find_installed_native(&self, item: &Item) -> Result<Option<String>> {
        let system = &self.sources.system;
        let installed = system.list_installed()?;
        let candidates = self.native_candidates(item);
        if installed.is_empty() {
            // The listing is unavailable; ask about each name instead.
            for name in candidates {
                if system
                    .installed_version(&native_probe(item, &name))?
                    .is_some()
                {
                    return Ok(Some(name));
                }
            }
            return Ok(None);
        }
        if let Some(name) = candidates.into_iter().find(|n| installed.contains(n)) {
            return Ok(Some(name));
        }
        if item.id.is_empty() {
            return Ok(None);
        }
        Ok(installed.into_iter().find(|pkg| {
            pkg.len() >= 4
                && (*pkg == item.id || item.id.contains(pkg.as_str()) || pkg.contains(&item.id))
        }))
    }

    /// For Flatpak-kind items: how it is installed (ledger first, then native scan, then Flatpak).
    pub fn installed_edition(&self, item: &Item) -> Result<Option<EditionKey>> {
        if item.kind != SourceKind::Flatpak {
            return Ok(None);
        }
        if let Some(entry) = self.ledger.get(&item.id)? {
            return Ok(Some(entry.edition.unwrap_or(EditionKey::Flatpak)));
        }
        if self.find_installed_native(item)?.is_some() {
            return Ok(Some(self.native_edition()));
        }
        if self.sources.flatpak.is_installed(item)? {
            return Ok(Some(EditionKey::Flatpak));
        }
        Ok(None)
    }

    /// Installed version; `"installed"` when installed but the version is unknown.
    pub fn installed_version(&self, item: &Item) -> Result<Option<String>> {
        match item.kind {
            SourceKind::Flatpak => {
                let entry = self.ledger.get(&item.id)?;
                let edition = match entry.as_ref().and_then(|e| e.edition) {
                    Some(e) => Some(e),
                    None => self.installed_edition(item)?,
                };
                match edition {
                    Some(EditionKey::Arch | EditionKey::Debian) => {
                        let names = entry
                            .as_ref()
                            .and_then(|e| e.package_name.clone())
                            .into_iter()
                            .chain(self.find_installed_native(item)?);
                        for name in names {
                            if let Some(v) = self
                                .sources
                                .system
                                .installed_version(&native_probe(item, &name))?
                            {
                                return Ok(Some(v));
                            }
                        }
                    }
                    Some(EditionKey::Flatpak) => {
                        if let Some(v) = self.sources.flatpak.installed_version(item)? {
                            return Ok(Some(v));
                        }
                    }
                    None => {}
                }
                Ok(entry
                    .and_then(|e| e.version)
                    .or_else(|| edition.map(|_| "installed".to_string())))
            }
            SourceKind::System => {
                if let Some(name) = self.find_installed_native(item)? {
                    if let Some(v) = self
                        .sources
                        .system
                        .installed_version(&native_probe(item, &name))?
                    {
                        return Ok(Some(v));
                    }
                }
                self.sources.flatpak.installed_version(item)
            }
            kind => self.source_for(kind)?.installed_version(item),
        }
    }

    pub fn state(&self, item: &Item) -> Result<InstallState> {
        let Some(current) = self.installed_version(item)? else {
            return Ok(InstallState::NotInstalled);
        };
        match item.version.as_deref() {
            Some(available)
                if current != "installed" && version::is_update(&current, available) =>
            {
                Ok(InstallState::UpdateAvailable {
                    installed: current,
                    available: available.to_string(),
                })
            }
            _ => Ok(InstallState::Installed {
                version: Some(current),
            }),
        }
    }

    /// Items with a newer catalog version than the installed one. Items whose state
    /// cannot be read are skipped.
    pub fn updates(&self, items: &[Item]) -> Vec<UpdateInfo> {
        items
            .iter()
            .filter_map(|item| match self.state(item) {
                Ok(InstallState::UpdateAvailable {
                    installed,
                    available,
                }) => Some(UpdateInfo {
                    item: item.clone(),
                    installed,
                    available,
                }),
                _ => None,
            })
            .collect()
    }

    pub fn install(
        &self,
        item: &Item,
        edition: Option<EditionKey>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<()> {
        if item.kind != SourceKind::Flatpak {
            self.source_for(item.kind)?
                .install(item, None, progress, cancel)?;
            return Ok(());
        }
        let available = editions(item);
        let chosen: Edition = edition
            .and_then(|k| available.iter().find(|e| e.key == k))
            .or_else(|| best_edition(&available, self.distro))
            .cloned()
            .ok_or_else(|| StoreError::Unsupported("no installable edition".into()))?;

        match chosen.key {
            EditionKey::Arch | EditionKey::Debian => {
                if chosen.url.is_empty() {
                    return Err(StoreError::Unsupported(format!(
                        "this package does not provide a {:?} edition",
                        chosen.key
                    )));
                }
                let outcome = self
                    .sources
                    .system
                    .install(item, Some(&chosen), progress, cancel)?;
                let version = match &outcome.package_name {
                    Some(n) => self
                        .sources
                        .system
                        .installed_version(&native_probe(item, n))?,
                    None => None,
                };
                self.record(item, chosen.key, outcome, version)
            }
            EditionKey::Flatpak => {
                if !self.sources.flatpak.is_available() {
                    return Err(StoreError::SourceUnavailable(SourceKind::Flatpak));
                }
                let outcome =
                    self.sources
                        .flatpak
                        .install(item, Some(&chosen), progress, cancel)?;
                let version = self.sources.flatpak.installed_version(item)?;
                self.record(item, EditionKey::Flatpak, outcome, version)
            }
        }
    }

    fn record(
        &self,
        item: &Item,
        edition: EditionKey,
        outcome: InstallOutcome,
        version: Option<String>,
    ) -> Result<()> {
        self.ledger.record(LedgerEntry {
            id: item.id.clone(),
            edition: Some(edition),
            package_name: outcome.package_name,
            version,
        })
    }

    pub fn uninstall(&self, item: &Item) -> Result<()> {
        if item.kind != SourceKind::Flatpak {
            return self.source_for(item.kind)?.uninstall(item);
        }
        let entry = self.ledger.get(&item.id)?;
        let edition = match entry.as_ref().and_then(|e| e.edition) {
            Some(e) => Some(e),
            None => self.installed_edition(item)?,
        };
        if matches!(edition, Some(EditionKey::Arch | EditionKey::Debian)) {
            let name = match entry.and_then(|e| e.package_name) {
                Some(n) => Some(n),
                None => self.find_installed_native(item)?,
            };
            let name = name.ok_or_else(|| {
                StoreError::NotFound("could not determine the native package name to remove".into())
            })?;
            self.sources.system.uninstall(&native_probe(item, &name))?;
        } else {
            self.sources.flatpak.uninstall(item)?;
        }
        self.ledger.forget(&item.id)
    }

    /// Reinstalls the best edition.
    pub fn update(
        &self,
        item: &Item,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<()> {
        self.install(item, None, progress, cancel)
    }

    /// Updates items one after another; one failure does not stop the rest.
    pub fn update_all(
        &self,
        updates: &[UpdateInfo],
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Vec<(String, Result<()>)> {
        updates
            .iter()
            .map(|u| {
                let r = if cancel.is_cancelled() {
                    Err(StoreError::Cancelled)
                } else {
                    self.update(&u.item, progress, cancel)
                };
                (u.item.id.clone(), r)
            })
            .collect()
    }

    /// Runs `action` on a worker thread. Read `events` until `Event::Finished`;
    /// call `cancel.cancel()` to stop it.
    pub fn start(self: &Arc<Self>, action: Action, item: Item) -> InstallHandle {
        let (tx, rx) = mpsc::channel();
        let cancel = CancelToken::new();
        let (me, token) = (Arc::clone(self), cancel.clone());
        std::thread::spawn(move || {
            let sink = ChannelSink(Mutex::new(tx.clone()));
            let result = match action {
                Action::Install(edition) => me.install(&item, edition, &sink, &token),
                Action::Uninstall => me.uninstall(&item),
                Action::Update => me.update(&item, &sink, &token),
            };
            let _ = tx.send(Event::Finished(result));
        });
        InstallHandle { events: rx, cancel }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fakes::{FakeSource, MemoryLedger, VecSink};

    struct Rig {
        installer: Arc<Installer>,
        flatpak: Arc<FakeSource>,
        system: Arc<FakeSource>,
        ledger: Arc<MemoryLedger>,
    }

    fn rig() -> Rig {
        let flatpak = Arc::new(FakeSource::new(SourceKind::Flatpak));
        let system = Arc::new(FakeSource::new(SourceKind::System));
        let ledger = Arc::new(MemoryLedger::default());
        let installer = Arc::new(Installer::new(
            Sources {
                flatpak: flatpak.clone(),
                system: system.clone(),
                gnome_ext: Arc::new(FakeSource::new(SourceKind::GnomeExtension)),
                sayri_skill: Arc::new(FakeSource::new(SourceKind::SayriSkill)),
                sayri_plugin: Arc::new(FakeSource::new(SourceKind::SayriPlugin)),
            },
            ledger.clone(),
            Distro::Arch,
        ));
        Rig {
            installer,
            flatpak,
            system,
            ledger,
        }
    }

    fn item(json: &str) -> Item {
        serde_json::from_str(json).unwrap()
    }

    const FLAT: &str = r#"{"id":"org.x.App","type":"flatpak","version":"2.0"}"#;
    const BOTH: &str = r#"{"id":"mytool","type":"flatpak","version":"2.0","formats":{"arch":"https://x/mytool.pkg.tar.zst","flatpak":"f"}}"#;

    #[test]
    fn flatpak_install_records_ledger_and_state() {
        let r = rig();
        let it = item(FLAT);
        assert_eq!(r.installer.state(&it).unwrap(), InstallState::NotInstalled);
        r.installer
            .install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        let entry = r.ledger.get("org.x.App").unwrap().unwrap();
        assert_eq!(entry.edition, Some(EditionKey::Flatpak));
        assert_eq!(
            r.installer.installed_edition(&it).unwrap(),
            Some(EditionKey::Flatpak)
        );
        assert_eq!(
            r.installer.state(&it).unwrap(),
            InstallState::Installed {
                version: Some("2.0".into())
            }
        );
    }

    #[test]
    fn prefers_native_edition_then_honors_override() {
        let r = rig();
        let it = item(BOTH);
        r.installer
            .install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        let e = r.ledger.get("mytool").unwrap().unwrap();
        assert_eq!(e.edition, Some(EditionKey::Arch));
        assert_eq!(e.package_name.as_deref(), Some("mytool"));
        assert!(r.system.is_installed(&native_probe(&it, "mytool")).unwrap());

        let r2 = rig();
        r2.installer
            .install(
                &it,
                Some(EditionKey::Flatpak),
                &VecSink::default(),
                &CancelToken::new(),
            )
            .unwrap();
        assert_eq!(
            r2.ledger.get("mytool").unwrap().unwrap().edition,
            Some(EditionKey::Flatpak)
        );
    }

    #[test]
    fn native_edition_without_url_is_rejected() {
        let r = rig();
        let it = item(r#"{"id":"a","type":"flatpak","formats":{"arch":""}}"#);
        let err = r
            .installer
            .install(
                &it,
                Some(EditionKey::Arch),
                &VecSink::default(),
                &CancelToken::new(),
            )
            .unwrap_err();
        assert!(matches!(err, StoreError::Unsupported(_)));
    }

    #[test]
    fn failed_install_does_not_touch_ledger() {
        let r = rig();
        *r.flatpak.fail_with.lock().unwrap() = Some("boom".into());
        let it = item(FLAT);
        assert!(r
            .installer
            .install(&it, None, &VecSink::default(), &CancelToken::new())
            .is_err());
        assert_eq!(r.ledger.get("org.x.App").unwrap(), None);
    }

    #[test]
    fn uninstall_native_uses_ledger_name_and_forgets() {
        let r = rig();
        let it = item(BOTH);
        r.installer
            .install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        r.installer.uninstall(&it).unwrap();
        assert_eq!(r.ledger.get("mytool").unwrap(), None);
        assert_eq!(r.installer.state(&it).unwrap(), InstallState::NotInstalled);
    }

    #[test]
    fn detects_native_install_made_outside_the_store() {
        let r = rig();
        let it = item(r#"{"id":"org.gimp.GIMP","type":"flatpak"}"#);
        let outside = native_probe(&it, "gimp");
        r.system
            .install(&outside, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert_eq!(
            r.installer.installed_edition(&it).unwrap(),
            Some(EditionKey::Arch)
        );
        // Short names are ignored to avoid false matches.
        let r2 = rig();
        r2.system
            .install(
                &native_probe(&it, "gi"),
                None,
                &VecSink::default(),
                &CancelToken::new(),
            )
            .unwrap();
        assert_eq!(r2.installer.installed_edition(&it).unwrap(), None);
    }

    #[test]
    fn update_available_when_catalog_is_newer() {
        let r = rig();
        let it = item(FLAT);
        r.installer
            .install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        let newer = item(r#"{"id":"org.x.App","type":"flatpak","version":"3.0"}"#);
        // The fake installs the catalog version it is given, so simulate an older install.
        let older = item(r#"{"id":"org.x.App","type":"flatpak","version":"1.0"}"#);
        let r2 = rig();
        r2.installer
            .install(&older, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        let ups = r2.installer.updates(&[
            newer.clone(),
            item(r#"{"id":"other","type":"flatpak","version":"9"}"#),
        ]);
        assert_eq!(ups.len(), 1);
        assert_eq!(
            (ups[0].installed.as_str(), ups[0].available.as_str()),
            ("1.0", "3.0")
        );
        let results = r2
            .installer
            .update_all(&ups, &VecSink::default(), &CancelToken::new());
        assert!(results.iter().all(|(_, r)| r.is_ok()));
    }

    #[test]
    fn other_kinds_go_to_their_source() {
        let r = rig();
        let ext = item(r#"{"id":"e@x","type":"gnome_extension"}"#);
        r.installer
            .install(&ext, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert!(matches!(
            r.installer.state(&ext).unwrap(),
            InstallState::Installed { .. }
        ));
        r.installer.uninstall(&ext).unwrap();
        assert_eq!(r.installer.state(&ext).unwrap(), InstallState::NotInstalled);
        let unknown = item(r#"{"id":"u","type":"hologram"}"#);
        assert!(r
            .installer
            .install(&unknown, None, &VecSink::default(), &CancelToken::new())
            .is_err());
    }

    #[test]
    fn start_streams_progress_then_finishes() {
        let r = rig();
        let h = r.installer.start(Action::Install(None), item(FLAT));
        let mut progress = 0;
        let result = loop {
            match h.events.recv().unwrap() {
                Event::Progress(_) => progress += 1,
                Event::Finished(res) => break res,
            }
        };
        assert!(result.is_ok());
        assert!(progress >= 1);
        assert!(r.ledger.get("org.x.App").unwrap().is_some());
    }

    #[test]
    fn cancelled_install_leaves_ledger_untouched() {
        let r = rig();
        let (tx_done, rx_done) = std::sync::mpsc::channel();
        let it = item(FLAT);
        let installer = r.installer.clone();
        // The token is already set, so the source stops before doing any work.
        let cancel = CancelToken::new();
        cancel.cancel();
        std::thread::spawn(move || {
            tx_done
                .send(installer.install(&it, None, &VecSink::default(), &cancel))
                .unwrap();
        });
        assert!(matches!(
            rx_done.recv().unwrap(),
            Err(StoreError::Cancelled)
        ));
        assert_eq!(r.ledger.get("org.x.App").unwrap(), None);
    }
}
