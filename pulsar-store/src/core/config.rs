//! Constants and filesystem locations.

use std::path::{Path, PathBuf};

pub const USER_AGENT: &str = "PulsarStore/2.0";

/// Catalog mirrors, tried in order.
pub const CATALOG_URLS: [&str; 3] = [
    "https://store-os.inled.es/schema/index.json",
    "https://raw.githubusercontent.com/Inled-Pulsar-OS/store/main/schema/index.json",
    "https://pulsar-store.pages.dev/schema/index.json",
];

pub const ICON_BASE_URL: &str = "https://raw.githubusercontent.com/Inled-Pulsar-OS/store/main";

/// Installed copy of the bundled catalog (see `prepare-assets.sh`).
pub const BUNDLED_CATALOG_PATH: &str = "/usr/share/pulsar-store/catalog/catalog.json";

/// Catalog compiled into the binary, used when nothing else is available.
pub const EMBEDDED_CATALOG: &str = include_str!("../../assets/catalog/catalog.json");

/// Where the store keeps its cache. Everything lives under one directory so
/// tests can point it at a temporary folder.
#[derive(Debug, Clone)]
pub struct Paths {
    pub cache_dir: PathBuf,
    pub icons_dir: PathBuf,
    pub catalog_cache: PathBuf,
    pub catalog_etag: PathBuf,
    /// Same file name as the Python store, so existing installs carry over.
    pub ledger: PathBuf,
}

impl Paths {
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        Self {
            icons_dir: cache_dir.join("icons"),
            catalog_cache: cache_dir.join("catalog.json"),
            catalog_etag: cache_dir.join("catalog.etag"),
            ledger: cache_dir.join("installed.json"),
            cache_dir,
        }
    }

    /// `~/.cache/pulsar-store`, honoring `XDG_CACHE_HOME`.
    pub fn user() -> Self {
        let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".cache"));
        Self::new(base.join("pulsar-store"))
    }
}

/// Writes `bytes` to `path` through a temp file in the same directory, then renames.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}
