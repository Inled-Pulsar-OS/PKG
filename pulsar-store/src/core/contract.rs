//! A reusable test suite for `PackageSource` implementations (see docs: "Add a source").
//!
//! Give `run` a factory that builds a fresh, clean fixture each call: a source
//! wired to fakes and an item it can install. The suite then checks the
//! behavior every source must share.

use crate::core::error::StoreError;
use crate::core::fakes::VecSink;
use crate::core::model::{Edition, Item};
use crate::core::ports::{CancelToken, PackageSource};

pub struct Fixture {
    pub source: Box<dyn PackageSource>,
    /// An item this source can install. It must install through the downloader
    /// or runner (not from inline content), so cancellation can be observed.
    pub item: Item,
    pub edition: Option<Edition>,
}

/// Runs every contract check; panics with the failing rule on a violation.
pub fn run(make: &dyn Fn() -> Fixture) {
    starts_not_installed(make());
    install_makes_it_installed_and_reports_progress(make());
    install_twice_is_idempotent(make());
    cancelled_install_fails_and_leaves_nothing(make());
    uninstall_removes(make());
    uninstall_when_absent_does_not_panic(make());
    update_keeps_it_installed(make());
}

fn install(f: &Fixture) -> Result<VecSink, StoreError> {
    let sink = VecSink::default();
    f.source
        .install(&f.item, f.edition.as_ref(), &sink, &CancelToken::new())?;
    Ok(sink)
}

fn version(f: &Fixture) -> Option<String> {
    f.source
        .installed_version(&f.item)
        .expect("installed_version must not fail")
}

fn starts_not_installed(f: Fixture) {
    assert_eq!(
        version(&f),
        None,
        "a clean fixture must start not installed"
    );
    assert!(!f.source.is_installed(&f.item).unwrap());
}

fn install_makes_it_installed_and_reports_progress(f: Fixture) {
    let sink = install(&f).expect("install must succeed");
    assert!(
        version(&f).is_some(),
        "installed_version must be Some after install"
    );
    assert!(f.source.is_installed(&f.item).unwrap());
    assert!(
        !sink.0.lock().unwrap().is_empty(),
        "install must report at least one progress event"
    );
}

fn install_twice_is_idempotent(f: Fixture) {
    install(&f).expect("first install");
    install(&f).expect("a second install must also succeed");
    assert!(version(&f).is_some());
}

fn cancelled_install_fails_and_leaves_nothing(f: Fixture) {
    let cancel = CancelToken::new();
    cancel.cancel();
    let r = f
        .source
        .install(&f.item, f.edition.as_ref(), &VecSink::default(), &cancel);
    assert!(
        matches!(r, Err(StoreError::Cancelled)),
        "a cancelled install must return StoreError::Cancelled"
    );
    assert_eq!(
        version(&f),
        None,
        "a cancelled install must not leave the item installed"
    );
}

fn uninstall_removes(f: Fixture) {
    install(&f).expect("install");
    f.source.uninstall(&f.item).expect("uninstall must succeed");
    assert_eq!(
        version(&f),
        None,
        "item must not be installed after uninstall"
    );
}

fn uninstall_when_absent_does_not_panic(f: Fixture) {
    // Either result is acceptable; the call just must not panic or hang.
    let _ = f.source.uninstall(&f.item);
}

fn update_keeps_it_installed(f: Fixture) {
    install(&f).expect("install");
    f.source
        .update(&f.item, &VecSink::default(), &CancelToken::new())
        .expect("update must succeed");
    assert!(version(&f).is_some());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fakes::{FakeDownloader, FakeRunner, Reply};
    use crate::core::host::Distro;
    use crate::core::http::Downloader;
    use crate::core::model::EditionKey;
    use crate::core::sources::{
        flatpak::FlatpakSource,
        gnome_ext::GnomeExtSource,
        sayri::{SayriPluginSource, SayriSkillSource},
        system::SystemSource,
    };
    use std::io::Write;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    fn item(json: &str) -> Item {
        serde_json::from_str(json).unwrap()
    }

    fn zip_bytes(files: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zw = zip::ZipWriter::new(&mut buf);
        for (n, b) in files {
            zw.start_file(*n, zip::write::SimpleFileOptions::default())
                .unwrap();
            zw.write_all(b.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn flatpak_source_meets_contract() {
        run(&|| {
            let state: Arc<Mutex<Vec<String>>> = Arc::default();
            let runner = FakeRunner::new(&["flatpak"], move |cmd| {
                let j = cmd.join(" ");
                let last = cmd.last().cloned().unwrap_or_default();
                if j.starts_with("flatpak list") {
                    let rows: Vec<String> = state
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|i| format!("{i}\t1.0\tApp"))
                        .collect();
                    Reply::ok(&rows.join("\n"))
                } else if j.starts_with("flatpak install") {
                    let mut s = state.lock().unwrap();
                    if !s.contains(&last) {
                        s.push(last);
                    }
                    Reply::ok("Installing… 10%")
                } else if j.starts_with("flatpak uninstall") {
                    state.lock().unwrap().retain(|i| *i != last);
                    Reply::ok("")
                } else {
                    Reply::ok("")
                }
            });
            Fixture {
                source: Box::new(FlatpakSource::new(runner, FakeDownloader::new(b""))),
                item: item(r#"{"id":"org.x.App","type":"flatpak","name":"App"}"#),
                edition: None,
            }
        });
    }

    #[test]
    fn system_source_meets_contract() {
        run(&|| {
            let state: Arc<Mutex<Vec<String>>> = Arc::default();
            let runner = FakeRunner::new(&["pacman"], move |cmd| {
                let j = cmd.join(" ");
                let last = cmd.last().cloned().unwrap_or_default();
                if j.contains("pacman -Q ") {
                    if state.lock().unwrap().contains(&last) {
                        Reply::ok(&format!("{last} 1.0-1"))
                    } else {
                        Reply::fail("not found")
                    }
                } else if j.contains("pacman -S") {
                    let mut s = state.lock().unwrap();
                    if !s.contains(&last) {
                        s.push(last);
                    }
                    Reply::ok("")
                } else if j.contains("pacman -R") {
                    state.lock().unwrap().retain(|p| *p != last);
                    Reply::ok("")
                } else {
                    Reply::ok("")
                }
            });
            Fixture {
                source: Box::new(SystemSource::new(
                    runner,
                    FakeDownloader::new(b""),
                    Distro::Arch,
                )),
                item: item(r#"{"id":"htop","type":"system"}"#),
                edition: None,
            }
        });
    }

    #[test]
    fn gnome_extension_source_meets_contract() {
        struct Zip(Vec<u8>);
        impl Downloader for Zip {
            fn download(
                &self,
                _u: &str,
                d: &Path,
                c: &CancelToken,
                p: &mut dyn FnMut(Option<f32>),
            ) -> crate::core::error::Result<()> {
                if c.is_cancelled() {
                    return Err(StoreError::Cancelled);
                }
                std::fs::write(d, &self.0)?;
                p(Some(1.0));
                Ok(())
            }
        }
        let keep = tempfile::tempdir().unwrap();
        let base = keep.path().to_path_buf();
        let n = std::sync::atomic::AtomicUsize::new(0);
        run(&|| {
            let dir = base.join(
                n.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    .to_string(),
            );
            let zip = zip_bytes(&[("metadata.json", r#"{"uuid":"e@x.org","version":"1"}"#)]);
            let runner = FakeRunner::new(&[], |_| Reply::ok(""));
            Fixture {
                source: Box::new(GnomeExtSource::new(
                    runner,
                    Arc::new(Zip(zip)),
                    dir.join("u"),
                    dir.join("s"),
                )),
                item: item(
                    r#"{"id":"e","type":"gnome_extension","download_url":"https://x/e.zip","metadata":{"uuid":"e@x.org"}}"#,
                ),
                edition: None,
            }
        });
    }

    #[test]
    fn sayri_skill_source_meets_contract() {
        let keep = tempfile::tempdir().unwrap();
        let base = keep.path().to_path_buf();
        let n = std::sync::atomic::AtomicUsize::new(0);
        run(&|| {
            let dir = base.join(
                n.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    .to_string(),
            );
            Fixture {
                source: Box::new(SayriSkillSource::new(FakeDownloader::new(b"# skill"), dir)),
                item: item(
                    r#"{"id":"s","type":"sayri_skill","download_url":"https://x/SKILL.md"}"#,
                ),
                edition: None,
            }
        });
    }

    #[test]
    fn sayri_plugin_source_meets_contract() {
        let keep = tempfile::tempdir().unwrap();
        let base = keep.path().to_path_buf();
        let n = std::sync::atomic::AtomicUsize::new(0);
        run(&|| {
            let dir = base.join(
                n.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    .to_string(),
            );
            let zip = zip_bytes(&[("manifest.json", r#"{"version":"1"}"#)]);
            Fixture {
                source: Box::new(SayriPluginSource::new(
                    FakeDownloader::new(&zip),
                    dir.join("plugins"),
                    dir.clone(),
                )),
                item: item(r#"{"id":"p","type":"sayri_plugin","download_url":"https://x/p.zip"}"#),
                edition: None,
            }
        });
    }

    #[test]
    fn contract_catches_a_broken_source() {
        // A source that never reports installed must fail the suite.
        struct Broken;
        impl PackageSource for Broken {
            fn kind(&self) -> crate::core::model::SourceKind {
                crate::core::model::SourceKind::Flatpak
            }
            fn is_available(&self) -> bool {
                true
            }
            fn installed_version(&self, _: &Item) -> crate::core::error::Result<Option<String>> {
                Ok(None)
            }
            fn install(
                &self,
                _: &Item,
                _: Option<&Edition>,
                _: &dyn crate::core::ports::ProgressSink,
                _: &CancelToken,
            ) -> crate::core::error::Result<crate::core::ports::InstallOutcome> {
                Ok(Default::default())
            }
            fn uninstall(&self, _: &Item) -> crate::core::error::Result<()> {
                Ok(())
            }
            fn update(
                &self,
                i: &Item,
                p: &dyn crate::core::ports::ProgressSink,
                c: &CancelToken,
            ) -> crate::core::error::Result<crate::core::ports::InstallOutcome> {
                self.install(i, None, p, c)
            }
        }
        let r = std::panic::catch_unwind(|| {
            run(&|| Fixture {
                source: Box::new(Broken),
                item: item(r#"{"id":"x","type":"flatpak"}"#),
                edition: Some(Edition {
                    key: EditionKey::Flatpak,
                    url: String::new(),
                }),
            })
        });
        assert!(r.is_err());
    }
}
