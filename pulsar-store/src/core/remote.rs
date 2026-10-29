//! Fetching images and text from URLs the catalog names. Catalog data is
//! untrusted, so only http(s) is allowed and sizes are capped.

use crate::core::config::{write_atomic, Paths};
use crate::core::error::{Result, StoreError};
use crate::core::http::{Fetch, Fetched};
use crate::core::icons::looks_like_image;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;

const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);

/// True for `http://` and `https://` URLs; anything else must never be opened or fetched.
pub fn is_web_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("https://") || u.starts_with("http://")
}

fn get(fetch: &dyn Fetch, url: &str) -> Result<Vec<u8>> {
    if !is_web_url(url) {
        return Err(StoreError::Unsupported(format!("not a web URL: {url}")));
    }
    match fetch.get(url, None, TIMEOUT)? {
        Fetched::Body { bytes, .. } => Ok(bytes),
        Fetched::NotModified => Err(StoreError::Network("unexpected 304".into())),
    }
}

/// Downloads an image once and returns its path in the cache.
pub fn fetch_image_cached(paths: &Paths, fetch: &dyn Fetch, url: &str) -> Result<PathBuf> {
    let digest = Sha256::digest(url.as_bytes());
    let name: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    let path = paths.cache_dir.join("images").join(format!("{name}.img"));
    if std::fs::read(&path).is_ok_and(|b| looks_like_image(&b)) {
        return Ok(path);
    }
    let bytes = get(fetch, url)?;
    if bytes.len() > MAX_IMAGE_BYTES || !looks_like_image(&bytes) {
        return Err(StoreError::Parse("not a usable image".into()));
    }
    write_atomic(&path, &bytes)?;
    Ok(path)
}

/// Downloads text, keeping at most `max_bytes` of it.
pub fn fetch_text(fetch: &dyn Fetch, url: &str, max_bytes: usize) -> Result<String> {
    let mut bytes = get(fetch, url)?;
    bytes.truncate(max_bytes);
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Fake(Mutex<usize>, Vec<u8>);

    impl Fetch for Fake {
        fn get(&self, _u: &str, _e: Option<&str>, _t: Duration) -> Result<Fetched> {
            *self.0.lock().unwrap() += 1;
            Ok(Fetched::Body {
                bytes: self.1.clone(),
                etag: None,
            })
        }
    }

    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn web_urls_only() {
        assert!(is_web_url("https://x/a"));
        assert!(is_web_url(" HTTP://x"));
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ftp://x",
            "x",
            "",
        ] {
            assert!(!is_web_url(bad), "{bad}");
        }
    }

    #[test]
    fn image_is_cached_after_first_fetch() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let fake = Fake(Mutex::new(0), PNG.to_vec());
        let a = fetch_image_cached(&paths, &fake, "https://x/1.png").unwrap();
        let b = fetch_image_cached(&paths, &fake, "https://x/1.png").unwrap();
        assert_eq!(a, b);
        assert_eq!(*fake.0.lock().unwrap(), 1);
    }

    #[test]
    fn rejects_non_images_and_non_web_urls() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        assert!(fetch_image_cached(
            &paths,
            &Fake(Mutex::new(0), b"<html>".to_vec()),
            "https://x/1"
        )
        .is_err());
        let fake = Fake(Mutex::new(0), PNG.to_vec());
        assert!(fetch_image_cached(&paths, &fake, "file:///etc/passwd").is_err());
        assert_eq!(*fake.0.lock().unwrap(), 0);
    }

    #[test]
    fn text_is_truncated() {
        let fake = Fake(Mutex::new(0), b"hello world".to_vec());
        assert_eq!(fetch_text(&fake, "https://x/r.md", 5).unwrap(), "hello");
    }
}
