//! Icon cache: downloads item icons into the cache directory.
//! Work runs on a few worker threads; nothing here touches the UI thread.

use crate::core::config::{write_atomic, Paths, ICON_BASE_URL};
use crate::core::http::{Fetch, Fetched};
use crate::core::model::Item;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const WORKERS: usize = 4;
const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 64 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const EXTENSIONS: [&str; 4] = ["png", "svg", "jpg", "jpeg"];

/// Where to download an item's icon from.
pub fn icon_url(item: &Item) -> String {
    match item.extra.get("icon_url").and_then(|v| v.as_str()) {
        None | Some("") => format!("{ICON_BASE_URL}/assets/icons/{}.png", item.id),
        Some(url) if url.starts_with("http://") || url.starts_with("https://") => url.to_string(),
        Some(path) => format!("{ICON_BASE_URL}/{}", path.trim_start_matches('/')),
    }
}

/// A cached icon file for `id`, if one exists and looks valid.
pub fn cached_icon(paths: &Paths, id: &str) -> Option<PathBuf> {
    EXTENSIONS
        .iter()
        .map(|ext| paths.icons_dir.join(format!("{id}.{ext}")))
        .find(|p| p.is_file() && is_valid_image(p))
}

/// Checks file signatures, so a truncated or HTML error page is not kept.
fn is_valid_image(path: &Path) -> bool {
    std::fs::read(path).is_ok_and(|b| looks_like_image(&b))
}

fn looks_like_image(b: &[u8]) -> bool {
    if b.starts_with(&[0x89, b'P', b'N', b'G']) || b.starts_with(&[0xFF, 0xD8]) {
        return true;
    }
    let head = String::from_utf8_lossy(&b[..b.len().min(1024)]);
    head.contains("<svg")
}

pub struct IconCache {
    paths: Paths,
    fetch: Arc<dyn Fetch>,
}

impl IconCache {
    pub fn new(paths: Paths, fetch: Arc<dyn Fetch>) -> Self {
        Self { paths, fetch }
    }

    /// Downloads missing or corrupt icons. Returns how many were fetched.
    pub fn prefetch(&self, items: &[Item]) -> usize {
        let _ = std::fs::create_dir_all(&self.paths.icons_dir);
        let next = AtomicUsize::new(0);
        let fetched = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..WORKERS.min(items.len().max(1)) {
                scope.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(item) = items.get(i) else { break };
                    if self.ensure(item) {
                        fetched.fetch_add(1, Ordering::SeqCst);
                    }
                });
            }
        });
        evict(&self.paths.icons_dir, MAX_CACHE_BYTES);
        fetched.into_inner()
    }

    /// Returns true when a new file was written.
    fn ensure(&self, item: &Item) -> bool {
        if item.id.is_empty() || item.id.contains(['/', '\\']) {
            return false;
        }
        for ext in EXTENSIONS {
            let existing = self.paths.icons_dir.join(format!("{}.{ext}", item.id));
            if existing.is_file() {
                if is_valid_image(&existing) {
                    return false;
                }
                let _ = std::fs::remove_file(&existing);
            }
        }
        let url = icon_url(item);
        let ext = if url.ends_with(".svg") { "svg" } else { "png" };
        match self.fetch.get(&url, None, TIMEOUT) {
            Ok(Fetched::Body { bytes, .. })
                if bytes.len() <= MAX_ICON_BYTES && looks_like_image(&bytes) =>
            {
                let dest = self.paths.icons_dir.join(format!("{}.{ext}", item.id));
                write_atomic(&dest, &bytes).is_ok()
            }
            _ => false,
        }
    }
}

/// Deletes the oldest files until the directory fits within `max_bytes`.
fn evict(dir: &Path, max_bytes: u64) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = read
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            m.is_file().then(|| (m.modified().ok(), m.len(), e.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            total -= len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::{Result, StoreError};
    use std::sync::Mutex;

    struct Fake(Mutex<Vec<String>>, Vec<u8>);

    impl Fetch for Fake {
        fn get(&self, url: &str, _e: Option<&str>, _t: Duration) -> Result<Fetched> {
            self.0.lock().unwrap().push(url.to_string());
            if self.1.is_empty() {
                return Err(StoreError::Network("404".into()));
            }
            Ok(Fetched::Body {
                bytes: self.1.clone(),
                etag: None,
            })
        }
    }

    fn item(json: &str) -> Item {
        serde_json::from_str(json).unwrap()
    }

    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn url_derivation() {
        assert_eq!(
            icon_url(&item(r#"{"id":"x","type":"flatpak"}"#)),
            format!("{ICON_BASE_URL}/assets/icons/x.png")
        );
        assert_eq!(
            icon_url(&item(
                r#"{"id":"x","type":"flatpak","icon_url":"https://a/b.svg"}"#
            )),
            "https://a/b.svg"
        );
        assert_eq!(
            icon_url(&item(
                r#"{"id":"x","type":"flatpak","icon_url":"/assets/i.png"}"#
            )),
            format!("{ICON_BASE_URL}/assets/i.png")
        );
    }

    #[test]
    fn prefetch_downloads_once_and_skips_cached() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let fake = Arc::new(Fake(Mutex::default(), PNG.to_vec()));
        let cache = IconCache::new(paths.clone(), fake.clone());
        let items = vec![
            item(r#"{"id":"a","type":"flatpak"}"#),
            item(r#"{"id":"b","type":"flatpak"}"#),
        ];
        assert_eq!(cache.prefetch(&items), 2);
        assert!(cached_icon(&paths, "a").is_some());
        assert_eq!(cache.prefetch(&items), 0);
        assert_eq!(fake.0.lock().unwrap().len(), 2);
    }

    #[test]
    fn corrupt_cached_file_is_refetched() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        std::fs::create_dir_all(&paths.icons_dir).unwrap();
        std::fs::write(paths.icons_dir.join("a.png"), "<html>error</html>").unwrap();
        assert!(cached_icon(&paths, "a").is_none());
        let cache = IconCache::new(
            paths.clone(),
            Arc::new(Fake(Mutex::default(), PNG.to_vec())),
        );
        assert_eq!(cache.prefetch(&[item(r#"{"id":"a","type":"flatpak"}"#)]), 1);
        assert!(cached_icon(&paths, "a").is_some());
    }

    #[test]
    fn rejects_non_image_and_path_like_ids() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let cache = IconCache::new(
            paths.clone(),
            Arc::new(Fake(Mutex::default(), b"<html>".to_vec())),
        );
        let items = vec![
            item(r#"{"id":"a","type":"flatpak"}"#),
            item(r#"{"id":"../evil","type":"flatpak"}"#),
        ];
        assert_eq!(cache.prefetch(&items), 0);
        assert!(cached_icon(&paths, "a").is_none());
    }

    #[test]
    fn eviction_removes_oldest_first() {
        let dir = tempfile::tempdir().unwrap();
        for (i, name) in ["old", "new"].iter().enumerate() {
            let p = dir.path().join(name);
            std::fs::write(&p, vec![0u8; 10]).unwrap();
            let t = std::time::SystemTime::now() - Duration::from_secs(100 - i as u64 * 50);
            std::fs::File::options()
                .write(true)
                .open(&p)
                .unwrap()
                .set_modified(t)
                .unwrap();
        }
        evict(dir.path(), 10);
        assert!(!dir.path().join("old").exists());
        assert!(dir.path().join("new").exists());
    }
}
