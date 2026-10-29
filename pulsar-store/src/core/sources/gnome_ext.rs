//! GNOME Shell extensions, installed under `~/.local/share/gnome-shell/extensions`.

use crate::core::archive::{copy_dir, extract_zip, safe_component, verify_sha256};
use crate::core::error::{Result, StoreError};
use crate::core::http::Downloader;
use crate::core::model::{Edition, Item, Phase, Progress, SourceKind};
use crate::core::ports::{CancelToken, InstallOutcome, PackageSource, ProgressSink};
use crate::core::process::{argv, Runner};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct GnomeExtSource {
    runner: Arc<dyn Runner>,
    downloader: Arc<dyn Downloader>,
    user_dir: PathBuf,
    system_dir: PathBuf,
}

impl GnomeExtSource {
    pub fn new(
        runner: Arc<dyn Runner>,
        downloader: Arc<dyn Downloader>,
        user_dir: PathBuf,
        system_dir: PathBuf,
    ) -> Self {
        Self {
            runner,
            downloader,
            user_dir,
            system_dir,
        }
    }

    /// Default locations: the user's extensions folder and `/usr/share/gnome-shell/extensions`.
    pub fn with_default_dirs(runner: Arc<dyn Runner>, downloader: Arc<dyn Downloader>) -> Self {
        let user = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from(".local/share"))
            .join("gnome-shell/extensions");
        Self::new(
            runner,
            downloader,
            user,
            PathBuf::from("/usr/share/gnome-shell/extensions"),
        )
    }

    fn path_of(&self, uuid: &str) -> Option<PathBuf> {
        safe_component(uuid).ok()?;
        [&self.user_dir, &self.system_dir]
            .into_iter()
            .map(|base| base.join(uuid))
            .find(|p| p.join("metadata.json").is_file())
    }

    /// Refuses an extension that lists supported GNOME versions without the running one.
    fn check_shell_compatibility(&self, extracted: &Path) -> Result<()> {
        let supported: Vec<String> = fs::read_to_string(extracted.join("metadata.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("shell-version")?.as_array().cloned())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if supported.is_empty() {
            return Ok(());
        }
        match self.running_shell_version() {
            Some(running) if !shell_version_supported(&supported, &running) => {
                Err(StoreError::Unsupported(format!(
                    "this extension supports GNOME {}, but you run GNOME {running}",
                    supported.join(", ")
                )))
            }
            _ => Ok(()),
        }
    }

    /// Runs a `busctl --json=short` command and returns the first value of its reply.
    fn shell_json(&self, args: &[&str]) -> Option<serde_json::Value> {
        let mut cmd = argv(["busctl", "--user", "--json=short"]);
        cmd.extend(args.iter().map(|a| a.to_string()));
        let out = self.runner.capture(&cmd).ok().filter(|c| c.success)?;
        serde_json::from_str(&out.stdout).ok()
    }

    /// The running GNOME Shell version such as `46.0`, or `None` when not on GNOME.
    pub fn running_shell_version(&self) -> Option<String> {
        let v = self.shell_json(&[
            "get-property",
            "org.gnome.Shell",
            "/org/gnome/Shell",
            "org.gnome.Shell",
            "ShellVersion",
        ])?;
        v.get("data")?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    /// The Shell's state number for an extension (1 is enabled), `Some(None)` when
    /// the Shell does not know the extension, and `None` when there is no Shell.
    fn shell_state(&self, uuid: &str) -> Option<Option<f64>> {
        let v = self.shell_json(&[
            "call",
            "org.gnome.Shell.Extensions",
            "/org/gnome/Shell/Extensions",
            "org.gnome.Shell.Extensions",
            "GetExtensionInfo",
            "s",
            uuid,
        ])?;
        let info = v.get("data")?.get(0)?;
        Some(
            info.get("state")
                .and_then(|s| s.get("data"))
                .and_then(|d| d.as_f64()),
        )
    }

    /// Asks GNOME Shell to enable or disable an extension. Failure is not fatal:
    /// the files are in place and the shell may pick them up after a restart.
    fn set_enabled(&self, uuid: &str, enable: bool) {
        let method = if enable {
            "EnableExtension"
        } else {
            "DisableExtension"
        };
        let _ = self.runner.capture(&argv([
            "busctl",
            "--user",
            "call",
            "org.gnome.Shell.Extensions",
            "/org/gnome/Shell/Extensions",
            "org.gnome.Shell.Extensions",
            method,
            "s",
            uuid,
        ]));
    }
}

/// True when `running` (such as `46.1`) matches an entry of an extension's
/// `shell-version`. GNOME 40 and later compare by major number (`46`); older
/// entries such as `3.38` never match a modern Shell.
pub fn shell_version_supported(supported: &[String], running: &str) -> bool {
    let major = |v: &str| v.split('.').next().unwrap_or("").to_string();
    let run = major(running);
    supported.iter().any(|s| major(s) == run)
}

fn metadata_field(dir: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(dir.join("metadata.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    match v.get(key)? {
        serde_json::Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

impl PackageSource for GnomeExtSource {
    fn kind(&self) -> SourceKind {
        SourceKind::GnomeExtension
    }

    fn is_available(&self) -> bool {
        true
    }

    fn installed_version(&self, item: &Item) -> Result<Option<String>> {
        Ok(self
            .path_of(item.extension_uuid())
            .map(|p| metadata_field(&p, "version").unwrap_or_else(|| "installed".into())))
    }

    fn is_active(&self, item: &Item) -> Result<Option<bool>> {
        let uuid = safe_component(item.extension_uuid())?;
        Ok(self.shell_state(uuid).map(|state| state == Some(1.0)))
    }

    fn install(
        &self,
        item: &Item,
        edition: Option<&Edition>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome> {
        let uuid = safe_component(item.extension_uuid())?;
        let url = edition
            .map(|e| e.url.as_str())
            .filter(|u| !u.is_empty())
            .unwrap_or(&item.download_url);
        if url.is_empty() {
            return Err(StoreError::NotFound(format!("no download URL for {uuid}")));
        }
        progress.report(Progress {
            phase: Phase::Preparing,
            fraction: None,
        });

        let tmp = tempfile::tempdir()?;
        let zip_path = tmp.path().join("extension.zip");
        self.downloader.download(url, &zip_path, cancel, &mut |f| {
            progress.report(Progress {
                phase: Phase::Downloading,
                fraction: f,
            });
        })?;
        verify_sha256(&zip_path, item.declared_sha256())?;

        progress.report(Progress {
            phase: Phase::Installing,
            fraction: None,
        });
        let extracted = tmp.path().join("extracted");
        extract_zip(&zip_path, &extracted)?;

        self.check_shell_compatibility(&extracted)?;

        // The archive's own metadata names the real UUID, which decides the folder.
        let target = metadata_field(&extracted, "uuid").unwrap_or_else(|| uuid.to_string());
        let target = safe_component(&target)?.to_string();
        let dest = self.user_dir.join(&target);
        if dest.exists() {
            fs::remove_dir_all(&dest)?;
        }
        copy_dir(&extracted, &dest)?;

        let schemas = dest.join("schemas");
        if schemas.is_dir() && self.runner.which("glib-compile-schemas") {
            let _ = self
                .runner
                .capture(&argv(["glib-compile-schemas", &schemas.to_string_lossy()]));
        }
        self.set_enabled(&target, true);
        progress.report(Progress {
            phase: Phase::Finalizing,
            fraction: Some(1.0),
        });
        Ok(InstallOutcome::default())
    }

    fn uninstall(&self, item: &Item) -> Result<()> {
        let uuid = safe_component(item.extension_uuid())?;
        self.set_enabled(uuid, false);
        let dir = self.user_dir.join(uuid);
        if dir.is_dir() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fakes::{FakeRunner, Reply, VecSink};
    use crate::core::http::Downloader;
    use std::io::Write;

    /// Writes a prepared zip as the "download".
    struct ZipDownloader(Vec<u8>);

    impl Downloader for ZipDownloader {
        fn download(
            &self,
            _url: &str,
            dest: &Path,
            _c: &CancelToken,
            p: &mut dyn FnMut(Option<f32>),
        ) -> Result<()> {
            fs::write(dest, &self.0)?;
            p(Some(1.0));
            Ok(())
        }
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

    fn setup(zip: Vec<u8>) -> (tempfile::TempDir, Arc<FakeRunner>, GnomeExtSource) {
        let dir = tempfile::tempdir().unwrap();
        let runner = FakeRunner::new(&["glib-compile-schemas"], |_| Reply::ok(""));
        let src = GnomeExtSource::new(
            runner.clone(),
            Arc::new(ZipDownloader(zip)),
            dir.path().join("user"),
            dir.path().join("system"),
        );
        (dir, runner, src)
    }

    fn item() -> Item {
        serde_json::from_str(
            r#"{"id":"bar","type":"gnome_extension","download_url":"https://x/e.zip","metadata":{"uuid":"bar@example.org"}}"#,
        )
        .unwrap()
    }

    #[test]
    fn install_enables_and_reports_version() {
        let zip = zip_bytes(&[
            ("metadata.json", r#"{"uuid":"bar@example.org","version":3}"#),
            ("extension.js", ""),
            ("schemas/a.gschema.xml", ""),
        ]);
        let (dir, runner, src) = setup(zip);
        assert_eq!(src.installed_version(&item()).unwrap(), None);
        src.install(&item(), None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert!(dir
            .path()
            .join("user/bar@example.org/extension.js")
            .is_file());
        assert_eq!(
            src.installed_version(&item()).unwrap().as_deref(),
            Some("3")
        );
        assert!(runner.called("EnableExtension s bar@example.org"));
        assert!(runner.called("glib-compile-schemas"));
    }

    #[test]
    fn uninstall_disables_and_removes() {
        let zip = zip_bytes(&[("metadata.json", r#"{"uuid":"bar@example.org"}"#)]);
        let (dir, runner, src) = setup(zip);
        src.install(&item(), None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert_eq!(
            src.installed_version(&item()).unwrap().as_deref(),
            Some("installed")
        );
        src.uninstall(&item()).unwrap();
        assert!(!dir.path().join("user/bar@example.org").exists());
        assert!(runner.called("DisableExtension s bar@example.org"));
    }

    fn shell_runner(version: &'static str, state: Option<f64>) -> Arc<FakeRunner> {
        FakeRunner::new(&[], move |cmd| {
            let j = cmd.join(" ");
            if j.contains("ShellVersion") {
                Reply::ok(&format!(r#"{{"type":"s","data":"{version}"}}"#))
            } else if j.contains("GetExtensionInfo") {
                let info = match state {
                    Some(s) => format!(r#"{{"state":{{"type":"d","data":{s}}}}}"#),
                    None => "{}".to_string(),
                };
                Reply::ok(&format!(r#"{{"type":"a{{sv}}","data":[{info}]}}"#))
            } else {
                Reply::ok("")
            }
        })
    }

    #[test]
    fn version_matching() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(shell_version_supported(&s(&["45", "46"]), "46.1"));
        assert!(shell_version_supported(&s(&["46.1"]), "46.0"));
        assert!(!shell_version_supported(&s(&["3.38", "40"]), "46.0"));
        assert!(!shell_version_supported(&s(&["44"]), "47.2"));
    }

    #[test]
    fn rejects_an_extension_for_another_gnome_version() {
        let zip = zip_bytes(&[(
            "metadata.json",
            r#"{"uuid":"bar@example.org","shell-version":["44"]}"#,
        )]);
        let dir = tempfile::tempdir().unwrap();
        let src = GnomeExtSource::new(
            shell_runner("47.1", None),
            Arc::new(ZipDownloader(zip)),
            dir.path().join("user"),
            dir.path().join("system"),
        );
        let err = src
            .install(&item(), None, &VecSink::default(), &CancelToken::new())
            .unwrap_err();
        assert!(matches!(err, StoreError::Unsupported(m) if m.contains("47.1")));
        assert!(!dir.path().join("user/bar@example.org").exists());
    }

    #[test]
    fn compatible_extension_installs_and_without_a_shell_the_check_is_skipped() {
        let zip = zip_bytes(&[(
            "metadata.json",
            r#"{"uuid":"bar@example.org","shell-version":["46"]}"#,
        )]);
        for runner in [shell_runner("46.2", Some(1.0)), shell_runner("", None)] {
            let dir = tempfile::tempdir().unwrap();
            let src = GnomeExtSource::new(
                runner,
                Arc::new(ZipDownloader(zip.clone())),
                dir.path().join("user"),
                dir.path().join("system"),
            );
            src.install(&item(), None, &VecSink::default(), &CancelToken::new())
                .unwrap();
        }
    }

    #[test]
    fn reports_whether_the_shell_has_it_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let mk = |r| {
            GnomeExtSource::new(
                r,
                Arc::new(ZipDownloader(vec![])),
                dir.path().join("u"),
                dir.path().join("s"),
            )
        };
        assert_eq!(
            mk(shell_runner("46.0", Some(1.0)))
                .is_active(&item())
                .unwrap(),
            Some(true)
        );
        assert_eq!(
            mk(shell_runner("46.0", Some(2.0)))
                .is_active(&item())
                .unwrap(),
            Some(false)
        );
        // The Shell answers but has never heard of the extension: needs a login.
        assert_eq!(
            mk(shell_runner("46.0", None)).is_active(&item()).unwrap(),
            Some(false)
        );
        // No Shell at all (not GNOME): unknown.
        let none = FakeRunner::new(&[], |_| Reply::fail(""));
        assert_eq!(mk(none).is_active(&item()).unwrap(), None);
    }

    #[test]
    fn archive_uuid_with_traversal_is_rejected() {
        let zip = zip_bytes(&[("metadata.json", r#"{"uuid":"../evil"}"#)]);
        let (dir, _, src) = setup(zip);
        assert!(src
            .install(&item(), None, &VecSink::default(), &CancelToken::new())
            .is_err());
        assert!(!dir.path().join("evil").exists());
    }

    #[test]
    fn unsafe_item_uuid_is_rejected() {
        let (_, _, src) = setup(zip_bytes(&[("metadata.json", "{}")]));
        let bad: Item =
            serde_json::from_str(r#"{"id":"../x","type":"gnome_extension","download_url":"u"}"#)
                .unwrap();
        assert!(src
            .install(&bad, None, &VecSink::default(), &CancelToken::new())
            .is_err());
        assert_eq!(src.installed_version(&bad).unwrap(), None);
    }

    #[test]
    fn missing_url_is_an_error() {
        let (_, _, src) = setup(vec![]);
        let it: Item = serde_json::from_str(r#"{"id":"a@b","type":"gnome_extension"}"#).unwrap();
        assert!(matches!(
            src.install(&it, None, &VecSink::default(), &CancelToken::new()),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn finds_system_installed_extension() {
        let (dir, _, src) = setup(vec![]);
        let sys = dir.path().join("system/bar@example.org");
        fs::create_dir_all(&sys).unwrap();
        fs::write(sys.join("metadata.json"), r#"{"version":"7"}"#).unwrap();
        assert_eq!(
            src.installed_version(&item()).unwrap().as_deref(),
            Some("7")
        );
    }
}
