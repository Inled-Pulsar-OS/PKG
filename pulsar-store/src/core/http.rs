//! Blocking HTTP helpers. Call from worker threads only (ADR 0007).

use crate::core::config::USER_AGENT;
use crate::core::error::{Result, StoreError};
use crate::core::ports::CancelToken;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

/// Largest in-memory response we accept.
const MAX_BODY: u64 = 64 * 1024 * 1024;

/// Largest file a package download may be.
const MAX_DOWNLOAD: u64 = 4 * 1024 * 1024 * 1024;

/// True when a request to `requested` ended up at `landed` over plain HTTP
/// after starting on HTTPS, which would expose the download to tampering.
pub fn is_downgrade(requested: &str, landed: &str) -> bool {
    requested.to_ascii_lowercase().starts_with("https://")
        && !landed.to_ascii_lowercase().starts_with("https://")
}

fn check_redirects(requested: &str, landed: &str) -> Result<()> {
    if is_downgrade(requested, landed) {
        return Err(StoreError::Network(format!(
            "refusing a redirect from HTTPS to {landed}"
        )));
    }
    Ok(())
}

pub enum Fetched {
    Body {
        bytes: Vec<u8>,
        etag: Option<String>,
    },
    NotModified,
}

/// Fetches a URL, optionally conditional on an ETag. Faked in tests.
pub trait Fetch: Send + Sync {
    fn get(&self, url: &str, etag: Option<&str>, timeout: Duration) -> Result<Fetched>;
}

pub struct UreqFetch;

impl Fetch for UreqFetch {
    fn get(&self, url: &str, etag: Option<&str>, timeout: Duration) -> Result<Fetched> {
        let mut req = ureq::get(url)
            .set("User-Agent", USER_AGENT)
            .timeout(timeout);
        if let Some(e) = etag {
            req = req.set("If-None-Match", e);
        }
        match req.call() {
            Ok(resp) => {
                check_redirects(url, resp.get_url())?;
                let etag = resp.header("ETag").map(str::to_string);
                let mut bytes = Vec::new();
                resp.into_reader().take(MAX_BODY).read_to_end(&mut bytes)?;
                Ok(Fetched::Body { bytes, etag })
            }
            Err(ureq::Error::Status(304, _)) => Ok(Fetched::NotModified),
            Err(ureq::Error::Status(code, _)) => Err(StoreError::Network(format!("HTTP {code}"))),
            Err(e) => Err(StoreError::Network(e.to_string())),
        }
    }
}

/// Downloads a URL into a file. Faked in tests.
pub trait Downloader: Send + Sync {
    fn download(
        &self,
        url: &str,
        dest: &Path,
        cancel: &CancelToken,
        on_progress: &mut dyn FnMut(Option<f32>),
    ) -> Result<()>;
}

pub struct UreqDownloader {
    pub timeout: Duration,
}

impl Default for UreqDownloader {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
        }
    }
}

impl Downloader for UreqDownloader {
    fn download(
        &self,
        url: &str,
        dest: &Path,
        cancel: &CancelToken,
        on_progress: &mut dyn FnMut(Option<f32>),
    ) -> Result<()> {
        download(url, dest, self.timeout, cancel, on_progress)
    }
}

/// Streams `url` into `dest`. Reports a 0..1 fraction when the size is known.
pub fn download(
    url: &str,
    dest: &Path,
    timeout: Duration,
    cancel: &CancelToken,
    on_progress: &mut dyn FnMut(Option<f32>),
) -> Result<()> {
    let resp = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .timeout(timeout)
        .call()
        .map_err(|e| StoreError::Network(e.to_string()))?;
    check_redirects(url, resp.get_url())?;
    let total: Option<u64> = resp.header("Content-Length").and_then(|v| v.parse().ok());
    if total.is_some_and(|t| t > MAX_DOWNLOAD) {
        return Err(StoreError::Network("download is too large".into()));
    }
    let mut reader = resp.into_reader();
    let mut file = File::create(dest)?;
    let mut buf = [0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        if cancel.is_cancelled() {
            return Err(StoreError::Cancelled);
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        if done > MAX_DOWNLOAD {
            return Err(StoreError::Network("download is too large".into()));
        }
        on_progress(total.map(|t| (done as f32 / t as f32).min(1.0)));
    }
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_https_to_http_downgrade() {
        assert!(is_downgrade("https://a/x", "http://b/x"));
        assert!(is_downgrade("HTTPS://a/x", "ftp://b/x"));
        assert!(!is_downgrade("https://a/x", "https://b/x"));
        assert!(!is_downgrade("http://a/x", "http://b/x"));
        assert!(!is_downgrade("http://a/x", "https://b/x"));
        assert!(check_redirects("https://a", "http://a").is_err());
        assert!(check_redirects("https://a", "https://b").is_ok());
    }
}
