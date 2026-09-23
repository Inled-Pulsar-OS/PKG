//! Tipos y modelos de datos compartidos entre módulos.
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MirrorInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TargetImageInfo {
    pub squashfs: String,
    #[serde(default)]
    pub iso: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

/// Una versión concreta de la distribución (1.0, 1.1-beta, ...).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VersionEntry {
    #[serde(default)]
    pub arch: HashMap<String, TargetImageInfo>,
    #[serde(default)]
    pub debian: HashMap<String, TargetImageInfo>,
    #[serde(default)]
    pub branch: Option<String>,
}

/// Una edición publicada (bittenfruit, ...) dentro del manifiesto remoto.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EditionInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub latest_version: String,
    #[serde(default)]
    pub versions: HashMap<String, VersionEntry>,
}

/// Manifiesto de releases servido por downloads-os.inled.es (releases.json).
///
/// Esquema real de la API / de `ISO/configs/releases.json`:
/// ```json
/// {
///   "latest_edition": "bittenfruit",
///   "mirrors": [...],
///   "editions": {
///     "bittenfruit": {
///       "id": "bittenfruit", "name": "Bittenfruit",
///       "latest_version": "1.1-beta",
///       "versions": { "1.0": {"arch": {...}, "debian": {...}} , ... }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ManifestData {
    pub latest_edition: String,
    #[serde(default)]
    pub mirrors: Vec<MirrorInfo>,
    #[serde(default)]
    pub editions: HashMap<String, EditionInfo>,
}

impl ManifestData {
    /// Edición actual (la marcada como `latest_edition` en el manifiesto).
    pub fn current_edition(&self) -> EditionInfo {
        self.editions
            .get(&self.latest_edition)
            .cloned()
            .unwrap_or_default()
    }

    /// Última versión publicada de la edición actual ("1.1-beta", ...).
    pub fn latest_version(&self) -> String {
        let edition = self.current_edition();
        if !edition.latest_version.is_empty() {
            return edition.latest_version.clone();
        }
        // Fallback: la clave más alta de versions.
        edition.versions.keys().max().cloned().unwrap_or_default()
    }

    /// Todas las versiones de la edición actual, nuevas primero, con la
    /// versión "latest" al frente cuando no aparece como clave.
    pub fn available_versions(&self) -> Vec<String> {
        let edition = self.current_edition();
        let mut versions: Vec<String> = edition.versions.keys().cloned().collect();
        versions.sort();
        versions.reverse();
        let latest = edition.latest_version.clone();
        if !latest.is_empty() && !versions.contains(&latest) {
            versions.insert(0, latest);
        }
        versions
    }

    /// Imagen (squashfs) para una versión, base ("arch"/"debian") y
    /// bootloader ("grub"/"refind") concretos.
    pub fn image_for(&self, version: &str, base: &str, bootloader: &str) -> Option<TargetImageInfo> {
        let edition = self.current_edition();
        let entry = edition.versions.get(version)?;
        let base_map = if base == "debian" { &entry.debian } else { &entry.arch };
        base_map.get(bootloader).cloned()
    }
}

#[derive(Debug, Clone)]
pub enum DownloadMsg {
    Progress {
        downloaded: u64,
        total: u64,
        speed_mb_s: f64,
        fraction: f64,
        eta_secs: u64,
    },
    Verifying,
    Done,
    Cancelled,
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum NetConnType {
    Wifi,
    Ethernet,
    None,
}

#[derive(Clone, Debug)]
pub struct NetworkStatus {
    pub is_connected: bool,
    pub conn_type: NetConnType,
    pub conn_name: String,
}

#[derive(Clone, Debug)]
pub struct WifiNetwork {
    pub in_use: bool,
    pub ssid: String,
    pub signal: u8,
    pub security: String,
}

#[derive(Clone, Debug)]
pub struct BtrfsTarget {
    pub _disk_path: String,
    pub part_path: String,
    pub label: String,
    pub uuid: String,
    pub size: String,
}

#[derive(Clone, Debug)]
pub struct DiscoveredImage {
    pub file_path: String,
    pub filename: String,
    pub size_str: String,
    pub device_label: String,
}

#[derive(Clone, Debug)]
pub enum RecoveryMode {
    Local,
    CustomImage(String),
}

#[derive(Debug)]
pub enum RecoveryUpdate {
    Progress(f64, String),
    Log(String),
    Finished(Result<(), String>),
}
