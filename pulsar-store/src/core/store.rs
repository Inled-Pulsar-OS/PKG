//! Wires the real catalog, sources, ledger, and icon cache together.

use crate::core::catalog::HttpCatalog;
use crate::core::config::Paths;
use crate::core::host::Distro;
use crate::core::http::{UreqDownloader, UreqFetch};
use crate::core::icons::IconCache;
use crate::core::install::{Installer, Sources};
use crate::core::ledger::FileLedger;
use crate::core::process::SystemRunner;
use crate::core::sources::flatpak::FlatpakSource;
use crate::core::sources::gnome_ext::GnomeExtSource;
use crate::core::sources::sayri::{SayriPluginSource, SayriSkillSource};
use crate::core::sources::system::SystemSource;
use std::sync::Arc;

pub struct Store {
    pub paths: Paths,
    pub distro: Distro,
    pub catalog: Arc<HttpCatalog>,
    pub installer: Arc<Installer>,
    pub icons: Arc<IconCache>,
}

impl Store {
    /// The production wiring, using the user's cache directory.
    pub fn new() -> Self {
        Self::with_paths(Paths::user())
    }

    pub fn with_paths(paths: Paths) -> Self {
        let distro = Distro::detect();
        let runner = Arc::new(SystemRunner);
        let dl = Arc::new(UreqDownloader::default());
        let sources = Sources {
            flatpak: Arc::new(FlatpakSource::new(runner.clone(), dl.clone())),
            system: Arc::new(SystemSource::new(runner.clone(), dl.clone(), distro)),
            gnome_ext: Arc::new(GnomeExtSource::with_default_dirs(runner, dl.clone())),
            sayri_skill: Arc::new(SayriSkillSource::with_default_dir(dl.clone())),
            sayri_plugin: Arc::new(SayriPluginSource::with_default_dirs(dl)),
        };
        let ledger = Arc::new(FileLedger::new(paths.ledger.clone()));
        Self {
            catalog: Arc::new(HttpCatalog::new(paths.clone())),
            installer: Arc::new(Installer::new(sources, ledger, distro)),
            icons: Arc::new(IconCache::new(paths.clone(), Arc::new(UreqFetch))),
            distro,
            paths,
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}
