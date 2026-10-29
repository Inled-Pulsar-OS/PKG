//! The install ledger: what the store installed, kept in `installed.json`.
//! Same file and shape as the Python store, so existing installs carry over:
//! `{ "<id>": { "edition": "arch", "package_name": "x", "version": "1.0" } }`.

use crate::core::config::write_atomic;
use crate::core::error::Result;
use crate::core::model::{EditionKey, LedgerEntry};
use crate::core::ports::Ledger;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    edition: Option<String>,
    package_name: Option<String>,
    version: Option<String>,
}

pub struct FileLedger {
    path: PathBuf,
    /// Serializes read-modify-write cycles within this process.
    lock: Mutex<()>,
}

impl FileLedger {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    /// Reads the file. A missing file is empty; an unreadable one is moved aside
    /// to `<name>.corrupt` and treated as empty, so one bad write never blocks installs.
    fn read(&self) -> BTreeMap<String, Stored> {
        let Ok(bytes) = std::fs::read(&self.path) else {
            return BTreeMap::new();
        };
        serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            let mut aside = self.path.clone().into_os_string();
            aside.push(".corrupt");
            let _ = std::fs::rename(&self.path, aside);
            BTreeMap::new()
        })
    }

    fn write(&self, data: &BTreeMap<String, Stored>) -> Result<()> {
        let json = serde_json::to_vec_pretty(data)?;
        Ok(write_atomic(&self.path, &json)?)
    }
}

fn edition_name(e: EditionKey) -> &'static str {
    match e {
        EditionKey::Debian => "debian",
        EditionKey::Arch => "arch",
        EditionKey::Flatpak => "flatpak",
    }
}

fn parse_edition(s: &str) -> Option<EditionKey> {
    match s {
        "debian" => Some(EditionKey::Debian),
        "arch" => Some(EditionKey::Arch),
        "flatpak" => Some(EditionKey::Flatpak),
        _ => None,
    }
}

impl Ledger for FileLedger {
    fn get(&self, id: &str) -> Result<Option<LedgerEntry>> {
        let _guard = self.lock.lock().unwrap();
        Ok(self.read().remove(id).map(|s| LedgerEntry {
            id: id.to_string(),
            edition: s.edition.as_deref().and_then(parse_edition),
            package_name: s.package_name,
            version: s.version,
        }))
    }

    fn record(&self, entry: LedgerEntry) -> Result<()> {
        if entry.id.is_empty() {
            return Ok(());
        }
        let _guard = self.lock.lock().unwrap();
        let mut data = self.read();
        data.insert(
            entry.id,
            Stored {
                edition: entry.edition.map(|e| edition_name(e).to_string()),
                package_name: entry.package_name,
                version: entry.version,
            },
        );
        self.write(&data)
    }

    fn forget(&self, id: &str) -> Result<()> {
        let _guard = self.lock.lock().unwrap();
        let mut data = self.read();
        if data.remove(id).is_some() {
            self.write(&data)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> LedgerEntry {
        LedgerEntry {
            id: id.into(),
            edition: Some(EditionKey::Arch),
            package_name: Some("pkg".into()),
            version: Some("1.0".into()),
        }
    }

    #[test]
    fn record_get_forget() {
        let dir = tempfile::tempdir().unwrap();
        let l = FileLedger::new(dir.path().join("installed.json"));
        assert_eq!(l.get("a").unwrap(), None);
        l.record(entry("a")).unwrap();
        assert_eq!(l.get("a").unwrap(), Some(entry("a")));
        l.forget("a").unwrap();
        assert_eq!(l.get("a").unwrap(), None);
        l.forget("missing").unwrap();
    }

    #[test]
    fn reads_the_python_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("installed.json");
        std::fs::write(
            &path,
            r#"{"org.x.App":{"edition":"flatpak","version":"2.1"},"b":{"edition":"arch","package_name":"bb"}}"#,
        )
        .unwrap();
        let l = FileLedger::new(path);
        let a = l.get("org.x.App").unwrap().unwrap();
        assert_eq!(a.edition, Some(EditionKey::Flatpak));
        assert_eq!(a.version.as_deref(), Some("2.1"));
        assert_eq!(
            l.get("b").unwrap().unwrap().package_name.as_deref(),
            Some("bb")
        );
    }

    #[test]
    fn corrupt_file_is_moved_aside_and_recovered() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("installed.json");
        std::fs::write(&path, "{ not json").unwrap();
        let l = FileLedger::new(path.clone());
        assert_eq!(l.get("a").unwrap(), None);
        assert!(dir.path().join("installed.json.corrupt").is_file());
        l.record(entry("a")).unwrap();
        assert_eq!(l.get("a").unwrap(), Some(entry("a")));
    }

    #[test]
    fn unknown_edition_text_becomes_none_and_survives_other_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("installed.json");
        std::fs::write(&path, r#"{"a":{"edition":"extension"}}"#).unwrap();
        let l = FileLedger::new(path);
        assert_eq!(l.get("a").unwrap().unwrap().edition, None);
        l.record(entry("b")).unwrap();
        assert!(l.get("a").unwrap().is_some());
    }

    #[test]
    fn no_temp_file_left_behind() {
        let dir = tempfile::tempdir().unwrap();
        let l = FileLedger::new(dir.path().join("installed.json"));
        l.record(entry("a")).unwrap();
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1);
    }
}
