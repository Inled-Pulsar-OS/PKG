//! Carga del manifiesto de versiones (remoto + fallback local).
use crate::models::{ManifestData, MirrorInfo, TargetImageInfo};
use crate::system::log_msg;
use std::collections::HashMap;
use std::fs;
use std::process::Command;
pub(crate) fn fetch_release_manifest() -> Option<ManifestData> {
    let urls = [
        "https://pulsaros-releases.pages.dev/releases.json",
        "https://releases.pulsaros.inled.es/releases.json",
        "https://inled.github.io/pulsaros-releases/releases.json",
        "https://raw.githubusercontent.com/inled/pulsar/main/ISO/configs/releases.json",
        "https://apt.inled.es/releases.json",
    ];

    for url in urls {
        if let Ok(out) = Command::new("curl")
            .args(&["-sSL", "--connect-timeout", "4", "--max-time", "8", url])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                if let Ok(data) = serde_json::from_slice::<ManifestData>(&out.stdout) {
                    log_msg(&format!("Successfully fetched releases manifest from: {}", url));
                    return Some(data);
                }
            }
        }
    }
    None
}

pub(crate) fn local_manifest_fallback() -> ManifestData {
    if let Ok(content) = fs::read_to_string("/usr/share/pulsaros-recovery/releases.json") {
        if let Ok(data) = serde_json::from_str::<ManifestData>(&content) {
            return data;
        }
    }
    if let Ok(content) = fs::read_to_string("/home/jaime/Documentos/pulsar/ISO/configs/releases.json") {
        if let Ok(data) = serde_json::from_str::<ManifestData>(&content) {
            return data;
        }
    }

    let mut versions = HashMap::new();
    let mut arch_map = HashMap::new();
    let mut debian_map = HashMap::new();

    arch_map.insert("grub".to_string(), TargetImageInfo {
        squashfs: "https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-0.3-beta-bittenfruit-arch-grub-0.3-beta-bittenfruit.squashfs".to_string(),
        iso: Some("https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-0.3-beta-bittenfruit-arch-grub-0.3-beta-bittenfruit.iso".to_string()),
        sha256: None,
        size_bytes: Some(3145728000),
    });
    arch_map.insert("refind".to_string(), TargetImageInfo {
        squashfs: "https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-0.3-beta-bittenfruit-arch-refind-0.3-beta-bittenfruit.squashfs".to_string(),
        iso: Some("https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-0.3-beta-bittenfruit-arch-refind-0.3-beta-bittenfruit.iso".to_string()),
        sha256: None,
        size_bytes: Some(3145728000),
    });

    debian_map.insert("grub".to_string(), TargetImageInfo {
        squashfs: "https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-0.3-beta-bittenfruit-debian-grub-0.3-beta-bittenfruit.squashfs".to_string(),
        iso: Some("https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-0.3-beta-bittenfruit-debian-grub-0.3-beta-bittenfruit.iso".to_string()),
        sha256: None,
        size_bytes: Some(2800000000),
    });
    debian_map.insert("refind".to_string(), TargetImageInfo {
        squashfs: "https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-0.3-beta-bittenfruit-debian-refind-0.3-beta-bittenfruit.squashfs".to_string(),
        iso: Some("https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-0.3-beta-bittenfruit-debian-refind-0.3-beta-bittenfruit.iso".to_string()),
        sha256: None,
        size_bytes: Some(2800000000),
    });

    let mut base_map = HashMap::new();
    base_map.insert("arch".to_string(), arch_map);
    base_map.insert("debian".to_string(), debian_map);
    versions.insert("0.3-beta-bittenfruit".to_string(), base_map);

    let mirrors = vec![
        MirrorInfo { id: "auto".to_string(), name: "Automático (SourceForge CDN / Fast Anycast)".to_string() },
        MirrorInfo { id: "netix".to_string(), name: "NetIX (Europa / Internacional)".to_string() },
        MirrorInfo { id: "deac-riga".to_string(), name: "DEAC Riga (Europa del Norte)".to_string() },
        MirrorInfo { id: "altushost-swe".to_string(), name: "AltusHost (Suecia)".to_string() },
        MirrorInfo { id: "liquidtelecom".to_string(), name: "Liquid Telecom (África / Global)".to_string() },
        MirrorInfo { id: "cfhcable".to_string(), name: "CFH Cable (Norteamérica)".to_string() },
    ];

    ManifestData {
        latest_version: "0.3-beta-bittenfruit".to_string(),
        versions,
        mirrors,
    }
}
