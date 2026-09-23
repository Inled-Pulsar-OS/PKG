//! Carga del manifiesto de versiones (remoto + fallback local).
//!
//! El manifiesto remoto (`downloads-os.inled.es/releases.json`) usa el
//! esquema `latest_edition` + `editions` (ver `models::ManifestData`).
//! Se descarga con curl (ya disponible en el sistema de recuperación) y se
//! valida con serde; el primer URL que responda con JSON correcto gana.
use crate::models::{EditionInfo, ManifestData, MirrorInfo, TargetImageInfo, VersionEntry};
use crate::system::log_msg;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

pub(crate) fn fetch_release_manifest() -> Option<ManifestData> {
    let urls = [
        "https://downloads-os.inled.es/releases.json",
        "https://downloads-os.inled.es/isos.json",
        "https://releases.pulsaros.inled.es/releases.json",
        "https://pulsaros-releases.pages.dev/releases.json",
        "https://apt.inled.es/releases.json",
        "https://raw.githubusercontent.com/Inled-Pulsar-OS/ISO/main/configs/releases.json",
    ];

    for url in urls {
        if let Ok(out) = Command::new("curl")
            .args(&["-sSL", "--connect-timeout", "4", "--max-time", "8", url])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                if let Ok(data) = serde_json::from_slice::<ManifestData>(&out.stdout) {
                    log_msg(&format!(
                        "Successfully fetched releases manifest from: {} (latest: {} / {})",
                        url,
                        data.latest_edition,
                        data.latest_version(),
                    ));
                    return Some(data);
                }
            }
        }
    }
    None
}

pub(crate) fn local_manifest_fallback() -> ManifestData {
    // 1. Manifiesto empaquetado con el sistema de recuperación.
    if let Ok(content) = fs::read_to_string("/usr/share/pulsaros-recovery/releases.json") {
        if let Ok(data) = serde_json::from_str::<ManifestData>(&content) {
            return data;
        }
    }
    // 2. Manifiesto generado por generate-manifest.py en el árbol de la ISO
    //    (útil durante desarrollo en la máquina de builds).
    if let Ok(content) = fs::read_to_string("/home/jaime/Documentos/pulsar/ISO/configs/releases.json") {
        if let Ok(data) = serde_json::from_str::<ManifestData>(&content) {
            return data;
        }
    }

    // 3. Fallback embebido: mismo esquema que la API real, con las versiones
    //    públicas conocidas en SourceForge CDN.
    // URLs de SourceForge (el patrón del manifiesto real es
    // pulsaros-{modelo}-{base}-{bootloader}-{sufijo}[.squashfs|.iso]).
    let sf = |model: &str, base: &str, bl: &str, suffix: &str, ext: &str| {
        format!(
            "https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-{}-{}-{}-{}.{}",
            model, base, bl, suffix, ext
        )
    };

    let mut version_map: HashMap<String, VersionEntry> = HashMap::new();

    // (version, branch, modelo SourceForge, sufijo URL, size arch, size debian)
    let published: [(&str, &str, &str, &str, u64, u64); 3] = [
        ("1.1-beta", "unstable", "unstable", "1.1-beta-unstable", 4_724_464_025, 3_435_973_836),
        ("1.0", "stable", "1.0-bittenfruit", "1.0-bittenfruit", 4_724_464_025, 3_435_973_836),
        ("0.3-beta", "unstable", "0.3-beta-bittenfruit", "0.3-beta-bittenfruit", 3_145_728_000, 2_800_000_000),
    ];

    for (version, branch, model, suffix, size_arch, size_debian) in published {
        let mut entry = VersionEntry {
            branch: Some(branch.to_string()),
            ..Default::default()
        };
        for base in ["arch", "debian"] {
            let map = if base == "arch" { &mut entry.arch } else { &mut entry.debian };
            let base_size = if base == "arch" { size_arch } else { size_debian };
            for bl in ["grub", "refind"] {
                map.insert(
                    bl.to_string(),
                    TargetImageInfo {
                        squashfs: sf(model, base, bl, suffix, "squashfs"),
                        iso: Some(sf(model, base, bl, suffix, "iso")),
                        sha256: None,
                        size_bytes: Some(base_size),
                    },
                );
            }
        }
        version_map.insert(version.to_string(), entry);
    }

    let bittenfruit = EditionInfo {
        id: "bittenfruit".to_string(),
        name: "Bittenfruit".to_string(),
        latest_version: "1.1-beta".to_string(),
        versions: version_map,
    };

    let mut editions = HashMap::new();
    editions.insert("bittenfruit".to_string(), bittenfruit);

    let mut mirrors = vec![MirrorInfo {
        id: "auto".to_string(),
        name: "Automático (SourceForge CDN / Fast Anycast)".to_string(),
    }];
    for (id, name) in [
        ("netix", "NetIX (Europa / Internacional)"),
        ("deac-riga", "DEAC Riga (Europa del Norte)"),
        ("altushost-swe", "AltusHost (Suecia)"),
        ("liquidtelecom", "Liquid Telecom (África / Global)"),
        ("cfhcable", "CFH Cable (Norteamérica)"),
    ] {
        mirrors.push(MirrorInfo {
            id: id.to_string(),
            name: name.to_string(),
        });
    }

    ManifestData {
        latest_edition: "bittenfruit".to_string(),
        mirrors,
        editions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prueba real: conexión a downloads-os.inled.es, parseo del manifiesto y
    /// resolución de la última release + squashfs de recuperación.
    #[test]
    fn fetch_and_parse_latest_release_from_api() {
        let manifest = fetch_release_manifest()
            .expect("debería conectarse a la API y parsear el manifiesto");

        println!("latest_edition = {}", manifest.latest_edition);
        println!("latest_version = {}", manifest.latest_version());
        println!("available_versions = {:?}", manifest.available_versions());

        let img = manifest
            .image_for(&manifest.latest_version(), "arch", "grub")
            .expect("debería existir imagen squashfs para la última release");
        println!("squashfs (arch/grub) = {}", img.squashfs);
        assert!(img.squashfs.starts_with("https://"));
        assert!(img.squashfs.ends_with(".squashfs"));

        let deb = manifest
            .image_for(&manifest.latest_version(), "debian", "refind")
            .expect("debería existir imagen debian/refind");
        println!("squashfs (debian/refind) = {}", deb.squashfs);
    }

    /// Fallback local parseable y consistente con el esquema de la API.
    #[test]
    fn local_fallback_uses_api_schema() {
        let fallback = local_manifest_fallback();
        assert_eq!(fallback.latest_edition, "bittenfruit");
        assert!(!fallback.latest_version().is_empty());
        assert!(fallback.mirrors.iter().any(|m| m.id == "auto"));
    }
}