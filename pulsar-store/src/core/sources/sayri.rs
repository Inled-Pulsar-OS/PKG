//! Sayri skills and plugins, installed under `~/.config/sayri`.

use crate::core::archive::{
    copy_dir, extract_zip, find_dir_with, is_zip, safe_component, verify_sha256,
};
use crate::core::error::{Result, StoreError};
use crate::core::http::Downloader;
use crate::core::model::{Edition, Item, Phase, Progress, SourceKind};
use crate::core::ports::{CancelToken, InstallOutcome, PackageSource, ProgressSink};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn sayri_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("sayri")
}

fn url_for<'a>(item: &'a Item, edition: Option<&'a Edition>) -> &'a str {
    edition
        .map(|e| e.url.as_str())
        .filter(|u| !u.is_empty())
        .unwrap_or(&item.download_url)
}

fn download_to(
    downloader: &dyn Downloader,
    url: &str,
    dest: &Path,
    item: &Item,
    progress: &dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<()> {
    downloader.download(url, dest, cancel, &mut |f| {
        progress.report(Progress {
            phase: Phase::Downloading,
            fraction: f,
        });
    })?;
    verify_sha256(dest, item.declared_sha256())
}

pub struct SayriSkillSource {
    downloader: Arc<dyn Downloader>,
    dir: PathBuf,
}

impl SayriSkillSource {
    pub fn new(downloader: Arc<dyn Downloader>, skills_dir: PathBuf) -> Self {
        Self {
            downloader,
            dir: skills_dir,
        }
    }

    pub fn with_default_dir(downloader: Arc<dyn Downloader>) -> Self {
        Self::new(downloader, sayri_dir().join("skills"))
    }

    fn skill_path(&self, id: &str) -> Option<PathBuf> {
        safe_component(id).ok()?;
        let p = self.dir.join(id);
        p.join("SKILL.md").is_file().then_some(p)
    }
}

/// Reads `version: x` from a skill's front matter; `1.0.0` when absent.
fn skill_version(markdown: &str) -> String {
    markdown
        .lines()
        .find_map(|l| l.strip_prefix("version:"))
        .map(|v| v.trim().trim_matches(['"', '\'']).to_string())
        .unwrap_or_else(|| "1.0.0".into())
}

impl PackageSource for SayriSkillSource {
    fn kind(&self) -> SourceKind {
        SourceKind::SayriSkill
    }

    fn is_available(&self) -> bool {
        true
    }

    fn installed_version(&self, item: &Item) -> Result<Option<String>> {
        Ok(self.skill_path(&item.id).map(|p| {
            fs::read_to_string(p.join("SKILL.md"))
                .map(|t| skill_version(&t))
                .unwrap_or_else(|_| "installed".into())
        }))
    }

    fn install(
        &self,
        item: &Item,
        edition: Option<&Edition>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome> {
        let id = safe_component(&item.id)?;
        progress.report(Progress {
            phase: Phase::Preparing,
            fraction: None,
        });
        let target = self.dir.join(id);
        let skill_file = target.join("SKILL.md");

        if let Some(raw) = item.extra.get("raw_content").and_then(|v| v.as_str()) {
            fs::create_dir_all(&target)?;
            fs::write(&skill_file, raw)?;
            return Ok(InstallOutcome::default());
        }
        let url = url_for(item, edition);
        if url.is_empty() {
            return Err(StoreError::NotFound(format!(
                "no download URL or content for {id}"
            )));
        }

        let tmp = tempfile::tempdir()?;
        let downloaded = tmp.path().join("skill");
        download_to(&*self.downloader, url, &downloaded, item, progress, cancel)?;

        progress.report(Progress {
            phase: Phase::Installing,
            fraction: None,
        });
        fs::create_dir_all(&target)?;
        if is_zip(&downloaded) {
            extract_zip(&downloaded, &target)?;
            if !skill_file.is_file() {
                // The archive wrapped everything in a folder.
                if let Some(dir) = find_dir_with(&target, "SKILL.md") {
                    fs::copy(dir.join("SKILL.md"), &skill_file)?;
                }
            }
            if !skill_file.is_file() {
                return Err(StoreError::Parse("skill archive has no SKILL.md".into()));
            }
        } else {
            fs::copy(&downloaded, &skill_file)?;
        }
        Ok(InstallOutcome::default())
    }

    fn uninstall(&self, item: &Item) -> Result<()> {
        let id = safe_component(&item.id)?;
        let dir = self.dir.join(id);
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

pub struct SayriPluginSource {
    downloader: Arc<dyn Downloader>,
    dir: PathBuf,
    conf_dir: PathBuf,
}

impl SayriPluginSource {
    pub fn new(downloader: Arc<dyn Downloader>, plugins_dir: PathBuf, conf_dir: PathBuf) -> Self {
        Self {
            downloader,
            dir: plugins_dir,
            conf_dir,
        }
    }

    pub fn with_default_dirs(downloader: Arc<dyn Downloader>) -> Self {
        let conf = sayri_dir();
        Self::new(downloader, conf.join("plugins"), conf)
    }

    fn plugin_path(&self, id: &str) -> Option<PathBuf> {
        safe_component(id).ok()?;
        let p = self.dir.join(id);
        p.join("manifest.json").is_file().then_some(p)
    }

    /// Stops running gateways for this plugin (`gateway_<id>*.pid` files).
    fn stop_gateways(&self, id: &str) {
        let Ok(entries) = fs::read_dir(&self.conf_dir) else {
            return;
        };
        let prefix = format!("gateway_{id}");
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !(name.starts_with(&prefix) && name.ends_with(".pid")) {
                continue;
            }
            let pid = fs::read_to_string(entry.path())
                .ok()
                .and_then(|t| t.trim().parse::<i32>().ok())
                .filter(|p| *p > 1);
            if let Some(pid) = pid {
                // SAFETY: kill with a positive pid and SIGTERM has no memory-safety preconditions.
                unsafe {
                    libc::kill(pid, libc::SIGTERM);
                }
            }
            let _ = fs::remove_file(entry.path());
        }
    }
}

impl PackageSource for SayriPluginSource {
    fn kind(&self) -> SourceKind {
        SourceKind::SayriPlugin
    }

    fn is_available(&self) -> bool {
        true
    }

    fn installed_version(&self, item: &Item) -> Result<Option<String>> {
        Ok(self.plugin_path(&item.id).map(|p| {
            fs::read_to_string(p.join("manifest.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .map(|v| match v.get("version") {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                    None => "1.0.0".into(),
                })
                .unwrap_or_else(|| "installed".into())
        }))
    }

    fn install(
        &self,
        item: &Item,
        edition: Option<&Edition>,
        progress: &dyn ProgressSink,
        cancel: &CancelToken,
    ) -> Result<InstallOutcome> {
        let id = safe_component(&item.id)?;
        let url = url_for(item, edition);
        if url.is_empty() {
            return Err(StoreError::NotFound(format!("no download URL for {id}")));
        }
        progress.report(Progress {
            phase: Phase::Preparing,
            fraction: None,
        });

        let tmp = tempfile::tempdir()?;
        let zip_path = tmp.path().join("plugin.zip");
        download_to(&*self.downloader, url, &zip_path, item, progress, cancel)?;

        progress.report(Progress {
            phase: Phase::Installing,
            fraction: None,
        });
        let extracted = tmp.path().join("extracted");
        extract_zip(&zip_path, &extracted)?;
        let root = find_dir_with(&extracted, "manifest.json")
            .ok_or_else(|| StoreError::Parse("plugin archive has no manifest.json".into()))?;

        let target = self.dir.join(id);
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        copy_dir(&root, &target)?;
        make_entrypoint_executable(&target);
        Ok(InstallOutcome::default())
    }

    fn uninstall(&self, item: &Item) -> Result<()> {
        let id = safe_component(&item.id)?;
        self.stop_gateways(id);
        let dir = self.dir.join(id);
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

/// Marks the manifest's `entrypoint` executable, if it stays inside the plugin folder.
fn make_entrypoint_executable(plugin_dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let Some(entry) = fs::read_to_string(plugin_dir.join("manifest.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("entrypoint")?.as_str().map(str::to_string))
    else {
        return;
    };
    let rel = Path::new(&entry);
    let escapes = rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir));
    if escapes {
        return;
    }
    let path = plugin_dir.join(rel);
    if path.is_file() {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fakes::{FakeDownloader, VecSink};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

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
    fn skill_from_raw_content() {
        let dir = tempfile::tempdir().unwrap();
        let src = SayriSkillSource::new(FakeDownloader::new(b""), dir.path().to_path_buf());
        let it = item(
            r#"{"id":"notes","type":"sayri_skill","raw_content":"---\nversion: \"2.0\"\n---\nbody"}"#,
        );
        assert_eq!(src.installed_version(&it).unwrap(), None);
        src.install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert_eq!(src.installed_version(&it).unwrap().as_deref(), Some("2.0"));
        src.uninstall(&it).unwrap();
        assert_eq!(src.installed_version(&it).unwrap(), None);
    }

    #[test]
    fn skill_from_plain_markdown_download() {
        let dir = tempfile::tempdir().unwrap();
        let dl = FakeDownloader::new(b"# skill\n");
        let src = SayriSkillSource::new(dl, dir.path().to_path_buf());
        let it = item(r#"{"id":"s","type":"sayri_skill","download_url":"https://x/SKILL.md"}"#);
        src.install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert_eq!(
            src.installed_version(&it).unwrap().as_deref(),
            Some("1.0.0")
        );
    }

    #[test]
    fn skill_from_nested_zip() {
        let dir = tempfile::tempdir().unwrap();
        let dl = FakeDownloader::new(&zip_bytes(&[("inner/SKILL.md", "version: 3")]));
        let src = SayriSkillSource::new(dl, dir.path().to_path_buf());
        let it = item(r#"{"id":"s","type":"sayri_skill","download_url":"https://x/s.zip"}"#);
        src.install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert_eq!(src.installed_version(&it).unwrap().as_deref(), Some("3"));
    }

    #[test]
    fn skill_without_source_or_with_bad_id_fails() {
        let dir = tempfile::tempdir().unwrap();
        let src = SayriSkillSource::new(FakeDownloader::new(b""), dir.path().to_path_buf());
        let none = item(r#"{"id":"s","type":"sayri_skill"}"#);
        assert!(matches!(
            src.install(&none, None, &VecSink::default(), &CancelToken::new()),
            Err(StoreError::NotFound(_))
        ));
        let bad = item(r#"{"id":"../s","type":"sayri_skill","raw_content":"x"}"#);
        assert!(src
            .install(&bad, None, &VecSink::default(), &CancelToken::new())
            .is_err());
        assert!(src.uninstall(&bad).is_err());
    }

    #[test]
    fn plugin_install_sets_entrypoint_executable_and_uninstall_stops_gateway() {
        let dir = tempfile::tempdir().unwrap();
        let zip = zip_bytes(&[
            (
                "p/manifest.json",
                r#"{"version":"1.2","entrypoint":"run.sh"}"#,
            ),
            ("p/run.sh", "#!/bin/sh"),
        ]);
        let conf = dir.path().join("conf");
        fs::create_dir_all(&conf).unwrap();
        let src = SayriPluginSource::new(
            FakeDownloader::new(&zip),
            conf.join("plugins"),
            conf.clone(),
        );
        let it = item(r#"{"id":"tg","type":"sayri_plugin","download_url":"https://x/p.zip"}"#);
        src.install(&it, None, &VecSink::default(), &CancelToken::new())
            .unwrap();
        assert_eq!(src.installed_version(&it).unwrap().as_deref(), Some("1.2"));
        let mode = fs::metadata(conf.join("plugins/tg/run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);

        // An unrelated pid file is left alone; the plugin's own is removed.
        fs::write(conf.join("gateway_tg.pid"), "999999999").unwrap();
        fs::write(conf.join("gateway_other.pid"), "999999999").unwrap();
        src.uninstall(&it).unwrap();
        assert!(!conf.join("gateway_tg.pid").exists());
        assert!(conf.join("gateway_other.pid").exists());
        assert_eq!(src.installed_version(&it).unwrap(), None);
    }

    #[test]
    fn plugin_entrypoint_cannot_escape() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside.sh");
        fs::write(&outside, "x").unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).unwrap();
        let plugin = dir.path().join("plugin");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(
            plugin.join("manifest.json"),
            r#"{"entrypoint":"../outside.sh"}"#,
        )
        .unwrap();
        make_entrypoint_executable(&plugin);
        assert_eq!(
            fs::metadata(&outside).unwrap().permissions().mode() & 0o111,
            0
        );
    }

    #[test]
    fn plugin_archive_without_manifest_fails_and_keeps_existing() {
        let dir = tempfile::tempdir().unwrap();
        let zip = zip_bytes(&[("readme.txt", "x")]);
        let src = SayriPluginSource::new(
            FakeDownloader::new(&zip),
            dir.path().join("p"),
            dir.path().to_path_buf(),
        );
        let it = item(r#"{"id":"tg","type":"sayri_plugin","download_url":"https://x/p.zip"}"#);
        assert!(matches!(
            src.install(&it, None, &VecSink::default(), &CancelToken::new()),
            Err(StoreError::Parse(_))
        ));
    }
}
