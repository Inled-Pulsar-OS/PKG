//! Which kind of Linux the store runs on.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distro {
    Arch,
    Debian,
}

impl Distro {
    /// Debian if `/etc/debian_version` exists, Arch otherwise (as the Python store did).
    pub fn detect() -> Self {
        Self::detect_in(Path::new("/"))
    }

    pub fn detect_in(root: &Path) -> Self {
        if root.join("etc/debian_version").exists() {
            Self::Debian
        } else {
            Self::Arch
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_marker_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Distro::detect_in(dir.path()), Distro::Arch);
        std::fs::create_dir_all(dir.path().join("etc")).unwrap();
        std::fs::write(dir.path().join("etc/debian_version"), "12").unwrap();
        assert_eq!(Distro::detect_in(dir.path()), Distro::Debian);
    }
}
