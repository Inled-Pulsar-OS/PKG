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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestData {
    pub latest_version: String,
    pub versions: HashMap<String, HashMap<String, HashMap<String, TargetImageInfo>>>,
    #[serde(default)]
    pub mirrors: Vec<MirrorInfo>,
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
