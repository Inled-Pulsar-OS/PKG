//! Which installable editions an item offers, and which suits this host.

use crate::core::host::Distro;
use crate::core::model::{Edition, EditionKey, Item};

const ARCH_SUFFIXES: [&str; 3] = [".pkg.tar.zst", ".pkg.tar.xz", ".pacman"];
const FLATPAK_SUFFIXES: [&str; 2] = [".flatpak", ".flatpakref"];

fn ends_with_any(url: &str, suffixes: &[&str]) -> bool {
    suffixes.iter().any(|s| url.ends_with(s))
}

/// Native (Debian, Arch) editions are listed when the catalog provides them.
/// The Flatpak edition is listed when named explicitly, or when it is the only option.
pub fn editions(item: &Item) -> Vec<Edition> {
    let f = &item.formats;
    let url = item.download_url.as_str();
    let mut out = Vec::new();

    if f.deb.is_some() || url.ends_with(".deb") {
        let u = f
            .deb
            .clone()
            .or_else(|| url.ends_with(".deb").then(|| url.to_string()))
            .unwrap_or_default();
        out.push(Edition {
            key: EditionKey::Debian,
            url: u,
        });
    }

    if f.arch.is_some() || f.pacman.is_some() || ends_with_any(url, &ARCH_SUFFIXES) {
        let u = f
            .arch
            .clone()
            .or_else(|| f.pacman.clone())
            .or_else(|| ends_with_any(url, &ARCH_SUFFIXES).then(|| url.to_string()))
            .unwrap_or_default();
        out.push(Edition {
            key: EditionKey::Arch,
            url: u,
        });
    }

    let ref_url = item.metadata.flatpakref_url.as_deref();
    if f.flatpak.is_some()
        || ref_url.is_some()
        || ends_with_any(url, &FLATPAK_SUFFIXES)
        || out.is_empty()
    {
        let u = f
            .flatpak
            .clone()
            .or_else(|| ref_url.map(str::to_string))
            .or_else(|| ends_with_any(url, &FLATPAK_SUFFIXES).then(|| url.to_string()))
            .unwrap_or_default();
        out.push(Edition {
            key: EditionKey::Flatpak,
            url: u,
        });
    }
    out
}

/// The edition that works on this host: the native one if offered, else the first.
pub fn best_edition(editions: &[Edition], distro: Distro) -> Option<&Edition> {
    let native = match distro {
        Distro::Arch => EditionKey::Arch,
        Distro::Debian => EditionKey::Debian,
    };
    editions
        .iter()
        .find(|e| e.key == native)
        .or_else(|| editions.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(json: &str) -> Item {
        serde_json::from_str(json).unwrap()
    }

    fn keys(es: &[Edition]) -> Vec<EditionKey> {
        es.iter().map(|e| e.key).collect()
    }

    #[test]
    fn flatpak_is_the_default_when_nothing_else_is_offered() {
        let es = editions(&item(r#"{"id":"a","type":"flatpak"}"#));
        assert_eq!(keys(&es), [EditionKey::Flatpak]);
        assert_eq!(es[0].url, "");
    }

    #[test]
    fn native_editions_from_formats() {
        let es = editions(&item(
            r#"{"id":"a","type":"flatpak","formats":{"deb":"https://x/a.deb","pacman":"https://x/a.pkg.tar.zst"}}"#,
        ));
        assert_eq!(keys(&es), [EditionKey::Debian, EditionKey::Arch]);
        assert_eq!(es[1].url, "https://x/a.pkg.tar.zst");
    }

    #[test]
    fn download_url_suffix_selects_edition() {
        let deb = editions(&item(
            r#"{"id":"a","type":"flatpak","download_url":"https://x/a.deb"}"#,
        ));
        assert_eq!(keys(&deb), [EditionKey::Debian]);
        let arch = editions(&item(
            r#"{"id":"a","type":"flatpak","download_url":"https://x/a.pkg.tar.xz"}"#,
        ));
        assert_eq!(keys(&arch), [EditionKey::Arch]);
        let fp = editions(&item(
            r#"{"id":"a","type":"flatpak","download_url":"https://x/a.flatpakref"}"#,
        ));
        assert_eq!(fp[0].url, "https://x/a.flatpakref");
    }

    #[test]
    fn flatpak_listed_alongside_native_when_named() {
        let es = editions(&item(
            r#"{"id":"a","type":"flatpak","formats":{"deb":"d"},"metadata":{"flatpakref_url":"https://x/a.flatpakref"}}"#,
        ));
        assert_eq!(keys(&es), [EditionKey::Debian, EditionKey::Flatpak]);
        assert_eq!(es[1].url, "https://x/a.flatpakref");
    }

    #[test]
    fn best_edition_prefers_native_for_host() {
        let es = editions(&item(
            r#"{"id":"a","type":"flatpak","formats":{"deb":"d","arch":"a","flatpak":"f"}}"#,
        ));
        assert_eq!(
            best_edition(&es, Distro::Arch).unwrap().key,
            EditionKey::Arch
        );
        assert_eq!(
            best_edition(&es, Distro::Debian).unwrap().key,
            EditionKey::Debian
        );
        let only_fp = editions(&item(r#"{"id":"a","type":"flatpak"}"#));
        assert_eq!(
            best_edition(&only_fp, Distro::Arch).unwrap().key,
            EditionKey::Flatpak
        );
        assert!(best_edition(&[], Distro::Arch).is_none());
    }
}
