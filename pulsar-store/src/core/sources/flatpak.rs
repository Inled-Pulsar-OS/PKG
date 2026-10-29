//! Flatpak applications, through the `flatpak` command.

use crate::core::cache::TtlCache;
use crate::core::error::{Result, StoreError};
use crate::core::http::Downloader;
use crate::core::model::{Edition, Item, Phase, Progress, SourceKind};
use crate::core::ports::{CancelToken, InstallOutcome, PackageSource, ProgressSink};
use crate::core::process::{argv, parse_percent, RunOutput, Runner};
use std::sync::Arc;
use std::time::Duration;

const FLATHUB_REPO: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub id: String,
    pub version: String,
    pub name: String,
}

/// Lowercases and drops `-`, `_`, `.`, so `com.ios-notes.app` equals `com.iosnotes.app`.
pub fn norm_id(s: &str) -> String {
    s.to_lowercase()
        .replace(['-', '_', '.'], "")
        .trim()
        .to_string()
}

/// Strips a URL path and the `.flatpakref` / `.flatpak` suffix from a reference.
fn clean_ref(app_id: &str) -> String {
    if app_id.contains('/') {
        app_id
            .rsplit('/')
            .next()
            .unwrap_or(app_id)
            .replace(".flatpakref", "")
            .replace(".flatpak", "")
    } else {
        app_id.to_string()
    }
}

/// Parses `flatpak list --app --columns=application,version,name`.
pub fn parse_list(output: &str) -> Vec<InstalledApp> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let id = parts.next()?.trim();
            if id.is_empty() {
                return None;
            }
            let version = parts.next().map(str::trim).filter(|v| !v.is_empty());
            Some(InstalledApp {
                id: id.to_string(),
                version: version.unwrap_or("installed").to_string(),
                name: parts.next().unwrap_or("").trim().to_string(),
            })
        })
        .collect()
}

/// Finds the installed app that matches `app_id`: exact id, then normalized id,
/// then substring or same display name.
pub fn find_match<'a>(
    installed: &'a [InstalledApp],
    app_id: &str,
    item_name: &str,
) -> Option<&'a InstalledApp> {
    let clean = clean_ref(app_id);
    let target = norm_id(&clean);
    let name_norm = norm_id(item_name);

    installed
        .iter()
        .find(|a| a.id == clean || a.id == app_id)
        .or_else(|| installed.iter().find(|a| norm_id(&a.id) == target))
        .or_else(|| {
            installed.iter().find(|a| {
                let n = norm_id(&a.id);
                (!target.is_empty() && (n.contains(&target) || target.contains(&n)))
                    || (!name_norm.is_empty() && name_norm == norm_id(&a.name))
            })
        })
}

pub struct FlatpakSource {
    runner: Arc<dyn Runner>,
    downloader: Arc<dyn Downloader>,
    /// `flatpak list` is slow, and the UI asks about every item.
    installed: TtlCache<Vec<InstalledApp>>,
}

impl FlatpakSource {
    pub fn new(runner: Arc<dyn Runner>, downloader: Arc<dyn Downloader>) -> Self {
        Self {
            runner,
            downloader,
            installed: TtlCache::new(Duration::from_secs(5)),
        }
    }

    fn installed_apps(&self) -> Vec<InstalledApp> {
        if !self.is_available() {
            return Vec::new();
        }
        self.installed.get_or_load(|| self.list_installed_apps())
    }

    fn list_installed_apps(&self) -> Vec<InstalledApp> {
        let cmd = argv([
            "flatpak",
            "list",
            "--app",
            "--columns=application,version,name",
        ]);
        match self.runner.capture(&cmd) {
            Ok(c) if c.success => parse_list(&c.stdout),
            _ => Vec::new(),
        }
    }

    fn display_name(item: &Item) -> String {
        item.name.resolve(&["en"]).to_string()
    }

    fn run_logged(
        &self,
        cmd: &[String],
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<RunOutput> {
        self.runner.run(
            cmd,
            &mut |line| {
                progress.report(Progress {
                    phase: Phase::Installing,
                    fraction: parse_percent(line),
                });
            },
            cancel,
        )
    }

    fn failure(out: &RunOutput) -> StoreError {
        StoreError::CommandFailed(out.tail_text())
    }
}

impl PackageSource for FlatpakSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Flatpak
    }

    fn is_available(&self) -> bool {
        self.runner.which("flatpak")
    }

    fn installed_version(&self, item: &Item) -> Result<Option<String>> {
        let apps = self.installed_apps();
        Ok(find_match(&apps, &item.id, &Self::display_name(item)).map(|a| a.version.clone()))
    }

    fn install(
        &self,
        item: &Item,
        edition: Option<&Edition>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome> {
        if !self.is_available() {
            return Err(StoreError::SourceUnavailable(SourceKind::Flatpak));
        }
        progress.report(Progress {
            phase: Phase::Preparing,
            fraction: None,
        });

        // Best effort: make sure Flathub is configured for this user.
        let _ = self.runner.capture(&argv([
            "flatpak",
            "remote-add",
            "--user",
            "--if-not-exists",
            "flathub",
            FLATHUB_REPO,
        ]));

        let target = edition
            .map(|e| e.url.as_str())
            .filter(|u| !u.is_empty())
            .or(Some(item.download_url.as_str()).filter(|u| !u.is_empty()))
            .unwrap_or(&item.id)
            .to_string();
        let is_url = target.starts_with("http://") || target.starts_with("https://");

        let tmp = tempfile::tempdir()?;
        let base = argv(["flatpak", "install", "--user", "-y", "--noninteractive"]);
        let mut with_remote = false;

        let cmd: Vec<String> = if target.ends_with(".flatpak") {
            let bundle = if is_url {
                let path = tmp.path().join("app.flatpak");
                progress.report(Progress {
                    phase: Phase::Downloading,
                    fraction: None,
                });
                self.downloader.download(&target, &path, cancel, &mut |f| {
                    progress.report(Progress {
                        phase: Phase::Downloading,
                        fraction: f,
                    });
                })?;
                path.to_string_lossy().into_owned()
            } else {
                target.clone()
            };
            [base.clone(), argv(["--bundle"]), vec![bundle]].concat()
        } else if is_url || target.ends_with(".flatpakref") {
            [base.clone(), argv(["--from"]), vec![target.clone()]].concat()
        } else {
            with_remote = true;
            [base.clone(), argv(["flathub"]), vec![target.clone()]].concat()
        };

        let mut out = self.run_logged(&cmd, progress, cancel);
        if matches!(&out, Ok(o) if !o.success) && with_remote {
            // Retry without naming the remote, letting flatpak pick one.
            let retry = [base, vec![target]].concat();
            out = self.run_logged(&retry, progress, cancel);
        }
        self.installed.invalidate();
        let out = out?;
        if !out.success {
            return Err(Self::failure(&out));
        }
        progress.report(Progress {
            phase: Phase::Finalizing,
            fraction: Some(1.0),
        });
        Ok(InstallOutcome::default())
    }

    fn uninstall(&self, item: &Item) -> Result<()> {
        if !self.is_available() {
            return Err(StoreError::SourceUnavailable(SourceKind::Flatpak));
        }
        let apps = self.installed_apps();
        let real_id = find_match(&apps, &item.id, &Self::display_name(item))
            .map_or_else(|| clean_ref(&item.id), |a| a.id.clone());
        let cancel = CancelToken::new();
        let user = argv(["flatpak", "uninstall", "--user", "-y", "--noninteractive"]);
        let out = self.runner.run(
            &[user, vec![real_id.clone()]].concat(),
            &mut |_| {},
            &cancel,
        )?;
        self.installed.invalidate();
        if out.success {
            return Ok(());
        }
        // Fall back to a system-wide installation.
        let sys = argv(["flatpak", "uninstall", "-y", "--noninteractive"]);
        let out = self
            .runner
            .run(&[sys, vec![real_id]].concat(), &mut |_| {}, &cancel)?;
        self.installed.invalidate();
        if out.success {
            Ok(())
        } else {
            Err(Self::failure(&out))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fakes::{FakeDownloader, FakeRunner, Reply, VecSink};
    use std::sync::Mutex;

    fn item(id: &str) -> Item {
        serde_json::from_str(&format!(
            r#"{{"id":"{id}","type":"flatpak","name":"Notes"}}"#
        ))
        .unwrap()
    }

    #[test]
    fn parses_list_with_missing_versions() {
        let apps = parse_list("org.a.App\t1.0\tApp A\norg.b.App\t\tApp B\n\n");
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].version, "1.0");
        assert_eq!(apps[1].version, "installed");
    }

    #[test]
    fn matching_order() {
        let apps = parse_list("com.iosnotes.app\t1\tNotes\norg.other.Thing\t2\tOther\n");
        assert_eq!(
            find_match(&apps, "com.iosnotes.app", "").unwrap().version,
            "1"
        );
        assert_eq!(
            find_match(&apps, "com.ios-notes.app", "").unwrap().version,
            "1"
        );
        assert_eq!(find_match(&apps, "iosnotes", "").unwrap().version, "1");
        assert_eq!(
            find_match(&apps, "zzz.unknown.id", "Other").unwrap().id,
            "org.other.Thing"
        );
        assert!(find_match(&apps, "zzz.unknown.id", "").is_none());
        assert_eq!(
            find_match(&apps, "https://x/com.iosnotes.app.flatpakref", "")
                .unwrap()
                .id,
            "com.iosnotes.app"
        );
    }

    fn source(
        installed: Arc<Mutex<Vec<String>>>,
        ok: bool,
    ) -> (Arc<FakeRunner>, FlatpakSource, Arc<FakeDownloader>) {
        let runner = FakeRunner::new(&["flatpak"], move |cmd| {
            let joined = cmd.join(" ");
            if joined.starts_with("flatpak list") {
                let lines: Vec<String> = installed
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|id| format!("{id}\t1.0\tNotes"))
                    .collect();
                return Reply::ok(&lines.join("\n"));
            }
            if joined.starts_with("flatpak install") && ok {
                installed.lock().unwrap().push(cmd.last().unwrap().clone());
                return Reply::ok("Installing… 50%\nDone");
            }
            if joined.starts_with("flatpak uninstall") && ok {
                installed
                    .lock()
                    .unwrap()
                    .retain(|i| i != cmd.last().unwrap());
                return Reply::ok("");
            }
            if joined.starts_with("flatpak remote-add") {
                return Reply::ok("");
            }
            Reply::fail("error: something broke")
        });
        let dl = FakeDownloader::new(b"bundle");
        let src = FlatpakSource::new(runner.clone(), dl.clone());
        (runner, src, dl)
    }

    #[test]
    fn installs_from_flathub_id_and_reports_progress() {
        let (runner, src, _) = source(Arc::default(), true);
        let sink = VecSink::default();
        src.install(&item("org.x.App"), None, &sink, &CancelToken::new())
            .unwrap();
        assert!(runner.called("flatpak install --user -y --noninteractive flathub org.x.App"));
        assert!(sink
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.fraction == Some(0.5)));
        assert_eq!(
            src.installed_version(&item("org.x.App"))
                .unwrap()
                .as_deref(),
            Some("1.0")
        );
    }

    #[test]
    fn flatpakref_url_uses_from_not_bundle() {
        let (runner, src, _) = source(Arc::default(), true);
        let ed = Edition {
            key: crate::core::model::EditionKey::Flatpak,
            url: "https://x/app.flatpakref".into(),
        };
        src.install(
            &item("org.x.App"),
            Some(&ed),
            &VecSink::default(),
            &CancelToken::new(),
        )
        .unwrap();
        assert!(runner
            .called("flatpak install --user -y --noninteractive --from https://x/app.flatpakref"));
        assert!(!runner.called("flatpak install --user -y --noninteractive --bundle"));
    }

    #[test]
    fn bundle_url_is_downloaded_first() {
        let (runner, src, dl) = source(Arc::default(), true);
        let ed = Edition {
            key: crate::core::model::EditionKey::Flatpak,
            url: "https://x/app.flatpak".into(),
        };
        src.install(
            &item("org.x.App"),
            Some(&ed),
            &VecSink::default(),
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(
            dl.urls.lock().unwrap().as_slice(),
            ["https://x/app.flatpak"]
        );
        assert!(runner.called("flatpak install --user -y --noninteractive --bundle"));
    }

    #[test]
    fn failure_reports_output_tail() {
        let (_, src, _) = source(Arc::default(), false);
        let err = src
            .install(
                &item("org.x.App"),
                None,
                &VecSink::default(),
                &CancelToken::new(),
            )
            .unwrap_err();
        assert!(matches!(err, StoreError::CommandFailed(m) if m.contains("something broke")));
    }

    #[test]
    fn unavailable_without_flatpak() {
        let runner = FakeRunner::new(&[], |_| Reply::fail(""));
        let src = FlatpakSource::new(runner, FakeDownloader::new(b""));
        assert!(matches!(
            src.install(
                &item("a.b.C"),
                None,
                &VecSink::default(),
                &CancelToken::new()
            ),
            Err(StoreError::SourceUnavailable(_))
        ));
        assert_eq!(src.installed_version(&item("a.b.C")).unwrap(), None);
    }

    #[test]
    fn uninstall_removes() {
        let state = Arc::new(Mutex::new(vec!["org.x.App".to_string()]));
        let (_, src, _) = source(state, true);
        src.uninstall(&item("org.x.App")).unwrap();
        assert_eq!(src.installed_version(&item("org.x.App")).unwrap(), None);
    }
}
