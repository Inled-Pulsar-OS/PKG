//! Native packages: `.deb` and apt on Debian, `.pkg.tar.*` and pacman on Arch.
//! Everything that changes the system runs through `privilege::elevate`.

use crate::core::archive::{safe_component, verify_sha256};
use crate::core::cache::TtlCache;
use crate::core::error::{Result, StoreError};
use crate::core::host::Distro;
use crate::core::http::Downloader;
use crate::core::model::{Edition, Item, Phase, Progress, SourceKind};
use crate::core::ports::{CancelToken, InstallOutcome, PackageSource, ProgressSink};
use crate::core::privilege::elevate;
use crate::core::process::{argv, parse_percent, RunOutput, Runner};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const ARCH_SUFFIXES: [&str; 4] = [".pkg.tar.zst", ".pkg.tar.xz", ".pkg.tar", ".pacman"];

/// Package names become command arguments, so reject anything that could be an option.
fn safe_pkg_name(name: &str) -> Result<&str> {
    if name.is_empty() || name.starts_with('-') || name.contains(char::is_whitespace) {
        return Err(StoreError::Unsupported(format!(
            "unsafe package name: {name:?}"
        )));
    }
    Ok(name)
}

/// Reads dependency names from `pacman -Qip` output, dropping version constraints.
pub fn parse_depends(output: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut in_deps = false;
    for line in output.lines() {
        if line.to_lowercase().starts_with("depends on") && line.contains(':') {
            in_deps = true;
        } else if line.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && line.contains(':')
        {
            in_deps = false;
        }
        if in_deps {
            if let Some((_, payload)) = line.split_once(':') {
                for token in payload.split_whitespace() {
                    let name = token.split(['<', '>', '=']).next().unwrap_or("").trim();
                    if !name.is_empty() && name != "None" && !names.iter().any(|n| n == name) {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }
    names
}

/// Reads `Name : x` from `pacman -Qip` output.
pub fn parse_pacman_name(output: &str) -> Option<String> {
    output.lines().find_map(|l| {
        let (key, value) = l.split_once(':')?;
        (key.trim() == "Name").then(|| value.trim().to_string())
    })
}

pub struct SystemSource {
    runner: Arc<dyn Runner>,
    downloader: Arc<dyn Downloader>,
    distro: Distro,
    /// Installed package names; listing them is slow and the UI asks often.
    names: TtlCache<Vec<String>>,
}

impl SystemSource {
    pub fn new(runner: Arc<dyn Runner>, downloader: Arc<dyn Downloader>, distro: Distro) -> Self {
        Self {
            runner,
            downloader,
            distro,
            names: TtlCache::new(Duration::from_secs(5)),
        }
    }

    pub fn distro(&self) -> Distro {
        self.distro
    }

    fn run_elevated(
        &self,
        cmd: &[String],
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<RunOutput> {
        let full = elevate(&*self.runner, cmd);
        // Whatever this command does, the installed set may have changed.
        self.names.invalidate();
        let out = self.runner.run(
            &full,
            &mut |line| {
                progress.report(Progress {
                    phase: Phase::Installing,
                    fraction: parse_percent(line),
                });
            },
            cancel,
        )?;
        if out.success {
            Ok(out)
        } else {
            Err(StoreError::CommandFailed(out.tail_text()))
        }
    }

    /// Installed version of a native package, or `None`.
    pub fn version_of(&self, name: &str) -> Option<String> {
        if safe_pkg_name(name).is_err() {
            return None;
        }
        match self.distro {
            Distro::Arch => {
                let c = self.runner.capture(&argv(["pacman", "-Q", name])).ok()?;
                c.success
                    .then(|| c.stdout.split_whitespace().nth(1).map(str::to_string))
                    .flatten()
            }
            Distro::Debian => {
                let status = self
                    .runner
                    .capture(&argv(["dpkg-query", "-W", "-f=${Status}", name]))
                    .ok()?;
                if !status.success || status.stdout.split_whitespace().last() != Some("installed") {
                    return None;
                }
                let v = self
                    .runner
                    .capture(&argv(["dpkg-query", "-W", "-f=${Version}", name]))
                    .ok()?;
                Some(v.stdout.trim().to_string()).filter(|v| !v.is_empty())
            }
        }
    }

    fn arch_package_name(&self, path: &Path) -> Option<String> {
        let p = path.to_string_lossy();
        let direct = self
            .runner
            .capture(&argv(["pacman", "-Qp", "--print-format", "%n", &p]))
            .ok()
            .filter(|c| c.success)
            .and_then(|c| c.stdout.lines().next().map(|l| l.trim().to_string()))
            .filter(|n| !n.is_empty());
        direct.or_else(|| {
            let c = self.runner.capture(&argv(["pacman", "-Qip", &p])).ok()?;
            parse_pacman_name(&c.stdout)
        })
    }

    /// Installs missing dependencies of a local Arch package. Names that are not
    /// real repository packages (virtual ones) come back to be passed as
    /// `--assume-installed`.
    fn ensure_arch_deps(&self, path: &Path) -> Result<Vec<String>> {
        let p = path.to_string_lossy();
        let info = self
            .runner
            .capture(&argv(["pacman", "-Qip", &p]))
            .map(|c| c.stdout)
            .unwrap_or_default();
        let mut assume = Vec::new();
        for dep in parse_depends(&info) {
            if safe_pkg_name(&dep).is_err() {
                continue;
            }
            if self
                .runner
                .capture(&argv(["pacman", "-Q", &dep]))
                .is_ok_and(|c| c.success)
            {
                continue;
            }
            let cmd = elevate(
                &*self.runner,
                &argv(["pacman", "-S", "--needed", "--noconfirm", &dep]),
            );
            let res = self.runner.capture(&cmd)?;
            if res.success {
                continue;
            }
            let text = format!("{}{}", res.stdout, res.stderr);
            if text.contains("target not found") {
                assume.push(dep);
            } else {
                let tail: Vec<&str> = text.lines().rev().take(8).collect();
                return Err(StoreError::CommandFailed(format!(
                    "Could not install dependency '{dep}':\n{}",
                    tail.into_iter().rev().collect::<Vec<_>>().join("\n")
                )));
            }
        }
        Ok(assume)
    }

    fn install_from_url(
        &self,
        url: &str,
        expected_sha256: Option<&str>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome> {
        let filename = url
            .split('?')
            .next()
            .and_then(|u| u.rsplit('/').next())
            .unwrap_or("");
        safe_component(filename)?;

        let tmp = tempfile::tempdir()?;
        let local = tmp.path().join(filename);
        progress.report(Progress {
            phase: Phase::Downloading,
            fraction: None,
        });
        self.downloader.download(url, &local, cancel, &mut |f| {
            progress.report(Progress {
                phase: Phase::Downloading,
                fraction: f,
            });
        })?;
        verify_sha256(&local, expected_sha256)?;
        let path = local.to_string_lossy().into_owned();

        let is_arch_pkg = ARCH_SUFFIXES.iter().any(|s| filename.ends_with(s));
        if is_arch_pkg && self.distro == Distro::Arch {
            let name = self.arch_package_name(&local);
            let assume = self.ensure_arch_deps(&local)?;
            let mut cmd = argv(["pacman", "-U", "--noconfirm", "--overwrite", "*"]);
            cmd.extend(assume.iter().map(|d| format!("--assume-installed={d}")));
            cmd.push(path);
            self.run_elevated(&cmd, progress, cancel)?;
            return Ok(InstallOutcome { package_name: name });
        }
        if filename.ends_with(".deb") {
            let name = self
                .runner
                .capture(&argv(["dpkg-deb", "-f", &path, "Package"]))
                .ok()
                .filter(|c| c.success)
                .map(|c| c.stdout.trim().to_string())
                .filter(|n| !n.is_empty())
                .or_else(|| filename.strip_suffix(".deb").map(str::to_string));
            match self.distro {
                Distro::Debian => {
                    self.run_elevated(
                        &argv(["apt-get", "install", "-y", &path]),
                        progress,
                        cancel,
                    )?;
                }
                Distro::Arch if self.runner.which("dpkg") => {
                    self.run_elevated(&argv(["dpkg", "-i", &path]), progress, cancel)?;
                }
                Distro::Arch => {
                    return Err(StoreError::Unsupported(
                        "a .deb package cannot be installed on this Arch system (no dpkg available)"
                            .into(),
                    ));
                }
            }
            return Ok(InstallOutcome { package_name: name });
        }
        Err(StoreError::Unsupported(format!(
            "package file not supported on this system: {filename}"
        )))
    }

    /// The download URL for this host, from the item's formats or `download_url`.
    fn native_url(&self, item: &Item) -> Option<String> {
        let f = &item.formats;
        let dl = item.download_url.as_str();
        match self.distro {
            Distro::Arch => f.arch.clone().or_else(|| f.pacman.clone()).or_else(|| {
                ARCH_SUFFIXES
                    .iter()
                    .any(|s| dl.ends_with(s))
                    .then(|| dl.to_string())
            }),
            Distro::Debian => f
                .deb
                .clone()
                .or_else(|| dl.ends_with(".deb").then(|| dl.to_string())),
        }
        .filter(|u| !u.is_empty())
    }
}

impl PackageSource for SystemSource {
    fn kind(&self) -> SourceKind {
        SourceKind::System
    }

    fn is_available(&self) -> bool {
        match self.distro {
            Distro::Arch => self.runner.which("pacman"),
            Distro::Debian => self.runner.which("apt-get"),
        }
    }

    fn installed_version(&self, item: &Item) -> Result<Option<String>> {
        Ok(self.version_of(item.native_name()))
    }

    fn list_installed(&self) -> Result<Vec<String>> {
        Ok(self.names.get_or_load(|| {
            let cmd = match self.distro {
                Distro::Arch => argv(["pacman", "-Qq"]),
                Distro::Debian => argv(["dpkg-query", "-W", "-f=${binary:Package}\n"]),
            };
            self.runner
                .capture(&cmd)
                .map(|out| {
                    out.stdout
                        .lines()
                        .filter(|l| !l.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        }))
    }

    fn install(
        &self,
        item: &Item,
        edition: Option<&Edition>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome> {
        progress.report(Progress {
            phase: Phase::Preparing,
            fraction: None,
        });
        let sha = item.declared_sha256();
        if let Some(url) = edition.map(|e| e.url.as_str()).filter(|u| !u.is_empty()) {
            return self.install_from_url(url, sha, progress, cancel);
        }
        if item.kind != SourceKind::System {
            return Err(StoreError::Unsupported(
                "this item has no native package edition".into(),
            ));
        }
        if let Some(url) = self.native_url(item) {
            return self.install_from_url(&url, sha, progress, cancel);
        }
        let name = safe_pkg_name(item.native_name())?;
        match self.distro {
            Distro::Arch => {
                self.run_elevated(
                    &argv(["pacman", "-S", "--needed", "--noconfirm", name]),
                    progress,
                    cancel,
                )?;
            }
            Distro::Debian => {
                // A failed index refresh should not block installing from the cached index.
                let _ = self.run_elevated(&argv(["apt-get", "update"]), progress, cancel);
                self.run_elevated(&argv(["apt-get", "install", "-y", name]), progress, cancel)?;
            }
        }
        Ok(InstallOutcome {
            package_name: Some(name.to_string()),
        })
    }

    fn uninstall(&self, item: &Item) -> Result<()> {
        let name = safe_pkg_name(item.native_name())?;
        let cmd = match self.distro {
            Distro::Arch => argv(["pacman", "-R", "--noconfirm", name]),
            Distro::Debian => argv(["apt-get", "remove", "-y", name]),
        };
        self.run_elevated(&cmd, &NoProgress, &CancelToken::new())?;
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

struct NoProgress;

impl ProgressSink for NoProgress {
    fn report(&self, _progress: Progress) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fakes::{FakeDownloader, FakeRunner, Reply, VecSink};
    use crate::core::model::EditionKey;
    use std::sync::Mutex;

    const QIP: &str = "Name            : mytool\nVersion         : 1.0-1\nDepends On      : glibc  libuuid>=2.0  gtk4\nOptional Deps   : None\n";

    #[test]
    fn parses_depends_and_name() {
        assert_eq!(parse_depends(QIP), ["glibc", "libuuid", "gtk4"]);
        assert_eq!(parse_pacman_name(QIP).as_deref(), Some("mytool"));
        assert!(parse_depends("Depends On      : None\n").is_empty());
    }

    fn item(json: &str) -> Item {
        serde_json::from_str(json).unwrap()
    }

    fn arch_source(installed: Arc<Mutex<Vec<String>>>) -> (Arc<FakeRunner>, SystemSource) {
        let runner = FakeRunner::new(&["pacman"], move |cmd| {
            let j = cmd.join(" ");
            let has = |s: &str| j.contains(s);
            if has("pacman -Qip") {
                return Reply::ok(QIP);
            }
            if has("pacman -Qp --print-format") {
                return Reply::ok("mytool\n");
            }
            if has("pacman -Qq") {
                return Reply::ok(&installed.lock().unwrap().join("\n"));
            }
            if has("pacman -Q ") {
                let name = cmd.last().unwrap();
                return if installed.lock().unwrap().contains(name) {
                    Reply::ok(&format!("{name} 1.0-1"))
                } else {
                    Reply::fail("error: package not found")
                };
            }
            if has("pacman -S --needed --noconfirm libuuid") {
                return Reply::fail("error: target not found: libuuid");
            }
            if has("pacman -S --needed --noconfirm") {
                installed.lock().unwrap().push(cmd.last().unwrap().clone());
                return Reply::ok("");
            }
            if has("pacman -U") {
                installed.lock().unwrap().push("mytool".into());
                return Reply::ok("installing... 80%");
            }
            if has("pacman -R") {
                installed
                    .lock()
                    .unwrap()
                    .retain(|p| p != cmd.last().unwrap());
                return Reply::ok("");
            }
            Reply::fail("unexpected")
        });
        let src = SystemSource::new(runner.clone(), FakeDownloader::new(b"pkg"), Distro::Arch);
        (runner, src)
    }

    fn url_edition(url: &str) -> Edition {
        Edition {
            key: EditionKey::Arch,
            url: url.into(),
        }
    }

    #[test]
    fn arch_package_install_resolves_deps_and_assumes_virtual() {
        let state = Arc::new(Mutex::new(vec!["glibc".to_string()]));
        let (runner, src) = arch_source(state);
        let it = item(r#"{"id":"my-tool","type":"flatpak"}"#);
        let sink = VecSink::default();
        let out = src
            .install(
                &it,
                Some(&url_edition("https://x/mytool-1.0.pkg.tar.zst?token=1")),
                &sink,
                &CancelToken::new(),
            )
            .unwrap();
        assert_eq!(out.package_name.as_deref(), Some("mytool"));
        assert!(runner.called("pacman -S --needed --noconfirm gtk4"));
        assert!(runner.called("--assume-installed=libuuid"));
        assert!(runner.called("pacman -U --noconfirm --overwrite *"));
        assert_eq!(src.version_of("mytool").as_deref(), Some("1.0-1"));
        assert!(sink
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.fraction == Some(0.8)));
    }

    #[test]
    fn dependency_failure_stops_install() {
        let runner = FakeRunner::new(&[], |cmd| {
            let j = cmd.join(" ");
            if j.contains("pacman -Qip") {
                Reply::ok(QIP)
            } else {
                Reply::fail("error: failed retrieving file")
            }
        });
        let src = SystemSource::new(runner.clone(), FakeDownloader::new(b"x"), Distro::Arch);
        let it = item(r#"{"id":"a","type":"flatpak"}"#);
        let err = src
            .install(
                &it,
                Some(&url_edition("https://x/a.pkg.tar.zst")),
                &VecSink::default(),
                &CancelToken::new(),
            )
            .unwrap_err();
        assert!(
            matches!(err, StoreError::CommandFailed(m) if m.contains("Could not install dependency"))
        );
        assert!(!runner.called("pacman -U"));
    }

    #[test]
    fn checksum_mismatch_blocks_install() {
        let (runner, src) = arch_source(Arc::default());
        let it = item(r#"{"id":"a","type":"flatpak","sha256":"00"}"#);
        let err = src
            .install(
                &it,
                Some(&url_edition("https://x/a.pkg.tar.zst")),
                &VecSink::default(),
                &CancelToken::new(),
            )
            .unwrap_err();
        assert!(matches!(err, StoreError::Parse(m) if m.contains("checksum")));
        assert!(!runner.called("pacman -U"));
    }

    #[test]
    fn rejects_unsafe_names_and_filenames() {
        let (_, src) = arch_source(Arc::default());
        let bad = item(r#"{"id":"--help","type":"system"}"#);
        assert!(src
            .install(&bad, None, &VecSink::default(), &CancelToken::new())
            .is_err());
        assert!(src.uninstall(&bad).is_err());
        let it = item(r#"{"id":"a","type":"flatpak"}"#);
        assert!(src
            .install(
                &it,
                Some(&url_edition("https://x/..")),
                &VecSink::default(),
                &CancelToken::new()
            )
            .is_err());
    }

    #[test]
    fn system_item_installs_repo_package_and_uninstalls() {
        let state: Arc<Mutex<Vec<String>>> = Arc::default();
        let (runner, src) = arch_source(state);
        let it = item(r#"{"id":"htop","type":"system"}"#);
        src.install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert!(runner.called("pacman -S --needed --noconfirm htop"));
        assert_eq!(
            src.installed_version(&it).unwrap().as_deref(),
            Some("1.0-1")
        );
        assert_eq!(src.list_installed().unwrap(), ["htop"]);
        src.uninstall(&it).unwrap();
        assert_eq!(src.installed_version(&it).unwrap(), None);
    }

    #[test]
    fn system_item_prefers_native_url_for_host() {
        let (runner, src) = arch_source(Arc::default());
        let it = item(
            r#"{"id":"mytool","type":"system","formats":{"arch":"https://x/mytool.pkg.tar.zst","deb":"https://x/mytool.deb"}}"#,
        );
        src.install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert!(runner.called("pacman -U"));
    }

    #[test]
    fn debian_deb_install_and_status_check() {
        let installed = Arc::new(Mutex::new(false));
        let st = installed.clone();
        let runner = FakeRunner::new(&["apt-get"], move |cmd| {
            let j = cmd.join(" ");
            if j.contains("dpkg-deb -f") {
                Reply::ok("mytool\n")
            } else if j.contains("apt-get install -y /") {
                *st.lock().unwrap() = true;
                Reply::ok("")
            } else if j.contains("${Status}") {
                if *st.lock().unwrap() {
                    Reply::ok("install ok installed")
                } else {
                    Reply::fail("")
                }
            } else if j.contains("${Version}") {
                Reply::ok("2.1\n")
            } else {
                Reply::ok("")
            }
        });
        let src = SystemSource::new(runner.clone(), FakeDownloader::new(b"deb"), Distro::Debian);
        let it = item(
            r#"{"id":"mytool","type":"system","download_url":"https://x/mytool_2.1_amd64.deb"}"#,
        );
        assert_eq!(src.installed_version(&it).unwrap(), None);
        let out = src
            .install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert_eq!(out.package_name.as_deref(), Some("mytool"));
        assert_eq!(src.installed_version(&it).unwrap().as_deref(), Some("2.1"));
    }

    #[test]
    fn deb_on_arch_without_dpkg_is_unsupported() {
        let (_, src) = arch_source(Arc::default());
        let it = item(r#"{"id":"a","type":"flatpak"}"#);
        let ed = Edition {
            key: EditionKey::Debian,
            url: "https://x/a.deb".into(),
        };
        let err = src
            .install(&it, Some(&ed), &VecSink::default(), &CancelToken::new())
            .unwrap_err();
        assert!(matches!(err, StoreError::Unsupported(_)));
    }
}
