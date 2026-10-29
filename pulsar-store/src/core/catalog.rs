//! Catalog loading: cache, mirrors with ETag, bundled fallback, normalization.

use crate::core::config::{
    write_atomic, Paths, BUNDLED_CATALOG_PATH, CATALOG_URLS, EMBEDDED_CATALOG,
};
use crate::core::error::{Result, StoreError};
use crate::core::http::{Fetch, Fetched, UreqFetch};
use crate::core::model::{Catalog, Item, LocalizedText};
use crate::core::ports::CatalogProvider;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const FETCH_TIMEOUT: Duration = Duration::from_secs(6);

/// Parses catalog JSON. Entries that are not valid items are skipped, so one
/// bad entry never hides the rest. The top level must be an object.
pub fn parse_catalog(bytes: &[u8]) -> Result<Catalog> {
    let root: Value = serde_json::from_slice(bytes)?;
    let Value::Object(obj) = root else {
        return Err(StoreError::Parse("catalog is not a JSON object".into()));
    };
    let list = obj
        .get("packages")
        .or_else(|| obj.get("items"))
        .and_then(Value::as_array);
    let packages = list
        .map(|a| {
            a.iter()
                .filter_map(|v| serde_json::from_value::<Item>(v.clone()).ok())
                .map(normalize_item)
                .collect()
        })
        .unwrap_or_default();
    Ok(Catalog {
        packages,
        announcement: obj.get("announcement").cloned(),
    })
}

/// Derives a missing summary from the first sentence of the description.
fn normalize_item(mut item: Item) -> Item {
    if item.summary.is_empty() {
        if let LocalizedText::Plain(desc) = &item.description {
            if !desc.is_empty() {
                let first = desc.split(". ").next().unwrap_or(desc);
                item.summary = LocalizedText::Plain(format!("{first}."));
            }
        }
    }
    item
}

pub struct HttpCatalog {
    paths: Paths,
    urls: Vec<String>,
    fetch: Arc<dyn Fetch>,
    bundled: Vec<PathBuf>,
}

impl HttpCatalog {
    pub fn new(paths: Paths) -> Self {
        Self::with(
            paths,
            CATALOG_URLS.iter().map(|s| s.to_string()).collect(),
            Arc::new(UreqFetch),
        )
    }

    pub fn with(paths: Paths, urls: Vec<String>, fetch: Arc<dyn Fetch>) -> Self {
        Self {
            paths,
            urls,
            fetch,
            bundled: vec![PathBuf::from(BUNDLED_CATALOG_PATH)],
        }
    }

    /// The ETag is stored with the URL it came from; it only applies to that mirror.
    fn stored_etag(&self, url: &str) -> Option<String> {
        if !self.paths.catalog_cache.is_file() {
            return None;
        }
        let text = std::fs::read_to_string(&self.paths.catalog_etag).ok()?;
        let (stored_url, etag) = text.split_once('\n')?;
        (stored_url == url && !etag.trim().is_empty()).then(|| etag.trim().to_string())
    }
}

impl CatalogProvider for HttpCatalog {
    /// Cache first, then an installed copy, then the copy compiled into the binary.
    fn load(&self) -> Result<Catalog> {
        let cached = std::iter::once(&self.paths.catalog_cache).chain(self.bundled.iter());
        for path in cached {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(catalog) = parse_catalog(&bytes) {
                    return Ok(catalog);
                }
            }
        }
        parse_catalog(EMBEDDED_CATALOG.as_bytes())
    }

    fn refresh(&self, force: bool) -> Result<bool> {
        let mut last = StoreError::Network("no catalog mirrors configured".into());
        for url in &self.urls {
            let etag = if force { None } else { self.stored_etag(url) };
            match self.fetch.get(url, etag.as_deref(), FETCH_TIMEOUT) {
                Ok(Fetched::NotModified) => return Ok(false),
                Ok(Fetched::Body { bytes, etag }) => {
                    if let Err(e) = parse_catalog(&bytes) {
                        last = e;
                        continue;
                    }
                    write_atomic(&self.paths.catalog_cache, &bytes)?;
                    match etag {
                        Some(e) => write_atomic(
                            &self.paths.catalog_etag,
                            format!("{url}\n{e}").as_bytes(),
                        )?,
                        None => {
                            let _ = std::fs::remove_file(&self.paths.catalog_etag);
                        }
                    }
                    return Ok(true);
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeFetch(
        Mutex<Vec<(String, Result<Fetched>)>>,
        Mutex<Vec<Option<String>>>,
    );

    impl FakeFetch {
        fn new(replies: Vec<(&str, Result<Fetched>)>) -> Arc<Self> {
            Arc::new(Self(
                Mutex::new(
                    replies
                        .into_iter()
                        .map(|(u, r)| (u.to_string(), r))
                        .collect(),
                ),
                Mutex::default(),
            ))
        }
    }

    impl Fetch for FakeFetch {
        fn get(&self, url: &str, etag: Option<&str>, _t: Duration) -> Result<Fetched> {
            self.1.lock().unwrap().push(etag.map(str::to_string));
            let mut replies = self.0.lock().unwrap();
            let pos = replies
                .iter()
                .position(|(u, _)| u == url)
                .expect("unexpected url");
            replies.remove(pos).1
        }
    }

    fn body(json: &str, etag: Option<&str>) -> Result<Fetched> {
        Ok(Fetched::Body {
            bytes: json.as_bytes().to_vec(),
            etag: etag.map(str::to_string),
        })
    }

    fn catalog(paths: &Paths, urls: &[&str], fetch: Arc<FakeFetch>) -> HttpCatalog {
        let mut c = HttpCatalog::with(
            paths.clone(),
            urls.iter().map(|s| s.to_string()).collect(),
            fetch,
        );
        c.bundled.clear();
        c
    }

    const ONE: &str =
        r#"{"packages":[{"id":"a","type":"flatpak","description":"First. Second."}]}"#;

    #[test]
    fn parse_skips_bad_entries_and_derives_summary() {
        let c = parse_catalog(br#"{"packages":[1,{"id":"a","type":"flatpak","description":"Hi there. More."},{"type":"x"}]}"#).unwrap();
        assert_eq!(c.packages.len(), 1);
        assert_eq!(c.packages[0].summary.resolve(&[]), "Hi there.");
    }

    #[test]
    fn parse_rejects_non_object() {
        assert!(matches!(parse_catalog(b"[1]"), Err(StoreError::Parse(_))));
        assert!(parse_catalog(b"nope").is_err());
    }

    #[test]
    fn load_falls_back_to_embedded_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let c = catalog(&Paths::new(dir.path()), &[], FakeFetch::new(vec![]));
        assert!(!c.load().unwrap().packages.is_empty());
    }

    #[test]
    fn corrupt_cache_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        std::fs::write(&paths.catalog_cache, "{broken").unwrap();
        let c = catalog(&paths, &[], FakeFetch::new(vec![]));
        assert!(!c.load().unwrap().packages.is_empty());
    }

    #[test]
    fn refresh_uses_next_mirror_after_failure_and_caches() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let fetch = FakeFetch::new(vec![
            ("u1", Err(StoreError::Network("down".into()))),
            ("u2", body(ONE, Some("\"v1\""))),
        ]);
        let c = catalog(&paths, &["u1", "u2"], fetch);
        assert!(c.refresh(false).unwrap());
        assert_eq!(c.load().unwrap().packages[0].id, "a");
        assert_eq!(c.stored_etag("u2").as_deref(), Some("\"v1\""));
        assert_eq!(c.stored_etag("u1"), None);
    }

    #[test]
    fn refresh_sends_etag_and_handles_not_modified() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let fetch = FakeFetch::new(vec![
            ("u", body(ONE, Some("\"v1\""))),
            ("u", Ok(Fetched::NotModified)),
        ]);
        let c = catalog(&paths, &["u"], fetch.clone());
        assert!(c.refresh(false).unwrap());
        assert!(!c.refresh(false).unwrap());
        assert_eq!(
            *fetch.1.lock().unwrap(),
            vec![None, Some("\"v1\"".to_string())]
        );
    }

    #[test]
    fn invalid_body_is_not_cached() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let c = catalog(
            &paths,
            &["u"],
            FakeFetch::new(vec![("u", body("[]", None))]),
        );
        assert!(c.refresh(false).is_err());
        assert!(!paths.catalog_cache.exists());
    }

    #[test]
    fn all_mirrors_failing_returns_last_error() {
        let dir = tempfile::tempdir().unwrap();
        let c = catalog(
            &Paths::new(dir.path()),
            &["u"],
            FakeFetch::new(vec![("u", Err(StoreError::Network("down".into())))]),
        );
        assert!(matches!(c.refresh(true), Err(StoreError::Network(_))));
    }
}
