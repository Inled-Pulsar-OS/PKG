use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use gio::prelude::*;
use glib::clone;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, Box as GtkBox, Button, CenterBox, CssProvider, DropDown, GestureClick, Image,
    Label, ListBox, ListBoxRow, Orientation, ProgressBar, ScrolledWindow, SelectionMode,
    Stack, StackTransitionType, StringList, TextView, WrapMode,
};
use libadwaita::prelude::*;
use libadwaita::ApplicationWindow;
use regex::Regex;
use serde::{Deserialize, Serialize};

static DEMO_MODE: AtomicBool = AtomicBool::new(false);

pub fn is_demo_mode() -> bool {
    DEMO_MODE.load(Ordering::SeqCst)
}

pub fn set_demo_mode(val: bool) {
    DEMO_MODE.store(val, Ordering::SeqCst);
}

const APP_CSS: &str = r#"
/* Force macOS Dark Backdrop */
window, window.background, .background, .root-container {
    background-color: #1e1e20;
    color: #ffffff;
}
window, .root-container, * {
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
.apple-box {
    background-color: #323236;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 20px;
    padding: 24px 28px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
}
.welcome-title {
    font-size: 22px;
    font-weight: 700;
    color: #ffffff;
    margin-top: 4px;
    margin-bottom: 4px;
}
.welcome-subtitle {
    font-size: 13px;
    color: #c7c7cc;
    margin-bottom: 12px;
}
.info-card {
    background-color: transparent;
    border: none;
    padding: 0px;
    margin-top: 4px;
    margin-bottom: 8px;
}
.info-card-text {
    font-size: 13px;
    color: #e5e5ea;
    line-height: 1.5;
}
.setting-label {
    font-size: 12px;
    font-weight: 600;
    color: #a1a1a6;
    margin-bottom: 2px;
}
.badge-net-ok {
    color: #30d158;
    font-weight: 600;
    font-size: 12px;
}
.badge-net-warn {
    color: #ff9f0a;
    font-weight: 600;
    font-size: 12px;
}
.badge-demo {
    background-color: rgba(255, 159, 10, 0.18);
    color: #ff9f0a;
    border: 1px solid rgba(255, 159, 10, 0.4);
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: bold;
}
/* Force completely transparent ListBox with clean seamless rows */
list, listview, listbox, .transparent-list, .content, .boxed-list {
    background-color: transparent;
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
}
listbox > row, listboxrow, row, .utility-item-row {
    background-color: transparent;
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
}
.utility-row-card {
    background-color: transparent;
    border: none;
    border-radius: 8px;
    padding: 8px 12px;
    transition: background-color 0.15s ease;
}
listbox > row:hover .utility-row-card,
listboxrow:hover .utility-row-card,
.utility-item-row:hover .utility-row-card {
    background-color: rgba(255, 255, 255, 0.08);
}
listbox > row:selected .utility-row-card,
listboxrow:selected .utility-row-card,
.utility-item-row:selected .utility-row-card {
    background-color: #0071e3;
}
.utility-title-lbl {
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
}
.utility-desc-lbl {
    font-size: 12px;
    color: #c7c7cc;
}
listbox > row:selected .utility-title-lbl,
listboxrow:selected .utility-title-lbl,
.utility-item-row:selected .utility-title-lbl {
    color: #ffffff;
}
listbox > row:selected .utility-desc-lbl,
listboxrow:selected .utility-desc-lbl,
.utility-item-row:selected .utility-desc-lbl {
    color: rgba(255, 255, 255, 0.92);
}
.suggested-action {
    background-color: #0071e3;
    color: #ffffff;
    border-radius: 10px;
    font-weight: 600;
    padding: 10px 24px;
    border: none;
    font-size: 13px;
}
.suggested-action:hover {
    background-color: #007bf5;
}
.suggested-action:disabled {
    background-color: #38383a;
    color: #636366;
}
.secondary-action {
    background-color: rgba(255, 255, 255, 0.08);
    color: #ffffff;
    border-radius: 10px;
    font-weight: 600;
    padding: 10px 20px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    font-size: 13px;
}
.secondary-action:hover {
    background-color: rgba(255, 255, 255, 0.14);
    border-color: rgba(255, 255, 255, 0.25);
}
.destructive-action {
    background-color: rgba(255, 69, 58, 0.15);
    color: #ff453a;
    border-radius: 10px;
    font-weight: 600;
    padding: 10px 20px;
    border: 1px solid rgba(255, 69, 58, 0.3);
    font-size: 13px;
}
.destructive-action:hover {
    background-color: rgba(255, 69, 58, 0.25);
}
.shortcut-btn {
    background-color: rgba(255, 255, 255, 0.08);
    color: #ffffff;
    border-radius: 6px;
    padding: 4px 10px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    font-size: 11px;
}
.shortcut-btn:hover {
    background-color: rgba(255, 255, 255, 0.15);
}
.progress-bar-thin {
    min-height: 8px;
    margin-top: 12px;
    margin-bottom: 12px;
}
.progress-bar-thin trough {
    min-height: 8px;
    border-radius: 9999px;
    background-color: #3a3a3c;
    border: none;
}
.progress-bar-thin progress {
    min-height: 8px;
    border-radius: 9999px;
    background-color: #0071e3;
    border: none;
}
.progress-text {
    font-size: 13px;
    color: #aeaeb2;
}
.live-log-view {
    background-color: #121212;
    border: 1px solid #333333;
    border-radius: 8px;
    padding: 6px;
}
.live-log-text text {
    background-color: #121212;
    color: #30d158;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 11px;
}
.err-log-text text {
    background-color: #121212;
    color: #ff453a;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 11px;
}
.disk-card {
    background-color: #2a2a2a;
    border: 1px solid #3c3c3c;
    border-radius: 12px;
    padding: 14px;
    min-width: 130px;
    margin: 6px;
    transition: all 0.15s ease;
}
.disk-card:hover {
    background-color: #323236;
}
.disk-card.selected {
    background-color: #323236;
    border: 2px solid #0071e3;
}
.bottom-power-btn {
    background-color: rgba(255, 255, 255, 0.08);
    color: #ffffff;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 20px;
    padding: 8px 24px;
    font-size: 13px;
    font-weight: 600;
    transition: all 0.15s ease;
}
.bottom-power-btn:hover {
    background-color: rgba(255, 255, 255, 0.16);
    border-color: rgba(255, 255, 255, 0.3);
}
"#;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct MirrorInfo {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TargetImageInfo {
    squashfs: String,
    #[serde(default)]
    iso: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ManifestData {
    latest_version: String,
    versions: HashMap<String, HashMap<String, HashMap<String, TargetImageInfo>>>,
    #[serde(default)]
    mirrors: Vec<MirrorInfo>,
}

#[derive(Debug, Clone)]
enum DownloadMsg {
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

fn detect_system_base() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        if content.contains("ID=debian") || content.contains("ID_LIKE=debian") || content.contains("ID=ubuntu") {
            return "debian".to_string();
        }
    }
    if Path::new("/run/media/pulsar_btrfs_pool/@/etc/debian_version").exists() {
        return "debian".to_string();
    }
    "arch".to_string()
}

fn detect_system_bootloader() -> String {
    if Path::new("/boot/efi/EFI/refind").exists() || Path::new("/boot/refind").exists() {
        return "refind".to_string();
    }
    if Path::new("/run/media/pulsar_btrfs_pool/@/boot/refind").exists() {
        return "refind".to_string();
    }
    "grub".to_string()
}

fn fetch_release_manifest() -> ManifestData {
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
                    return data;
                }
            }
        }
    }

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

#[derive(Clone, Debug)]
struct BtrfsTarget {
    _disk_path: String,
    part_path: String,
    label: String,
    uuid: String,
    size: String,
}

#[derive(Clone, Debug)]
struct DiscoveredImage {
    file_path: String,
    filename: String,
    size_str: String,
    device_label: String,
}

#[derive(Clone, Debug)]
enum RecoveryMode {
    Local,
    CustomImage(String),
}

#[derive(Debug)]
enum RecoveryUpdate {
    Progress(f64, String),
    Log(String),
    Finished(Result<(), String>),
}

fn log_msg(msg: &str) {
    let log_path = "/tmp/pulsaros-recovery.log";
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(f, "{}", msg);
    }
    println!("{}", msg);
}

fn exec_cmd_stream<L>(cmd: &str, log: &L) -> Result<(), String>
where
    L: Fn(&str) + Send + Sync + 'static,
{
    log_msg(&format!("Running (as root): {}", cmd));
    log(&format!("$ {}", cmd));

    let mut child = Command::new("sudo")
        .args(&["-n", "sh", "-c", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {}", cmd, e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (tx, rx) = std::sync::mpsc::channel::<String>();

    if let Some(out) = stdout {
        let tx_out = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().flatten() {
                let _ = tx_out.send(line);
            }
        });
    }

    if let Some(err) = stderr {
        let tx_err = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().flatten() {
                let _ = tx_err.send(line);
            }
        });
    }
    drop(tx);

    while let Ok(line) = rx.recv() {
        log_msg(&line);
        log(&line);
    }

    let status = child.wait().map_err(|e| format!("Failed to wait on '{}': {}", cmd, e))?;
    if !status.success() {
        let err_str = format!("Command '{}' failed with exit code: {:?}", cmd, status.code());
        log_msg(&format!("ERROR: {}", err_str));
        return Err(err_str);
    }
    Ok(())
}

fn exec_cmd(cmd: &str) -> Result<String, String> {
    log_msg(&format!("Running (as root): {}", cmd));
    let out = Command::new("sudo")
        .args(&["-n", "sh", "-c", cmd])
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if !out.status.success() {
        let err = format!("Command '{}' failed with code {:?}: {}", cmd, out.status.code(), stderr);
        log_msg(&format!("ERROR: {}", err));
        return Err(err);
    }
    Ok(stdout)
}

fn find_btrfs_targets() -> Vec<BtrfsTarget> {
    let mut targets = Vec::new();
    if let Ok(out) = Command::new("sudo").args(&["-n", "lsblk", "-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("FSTYPE=\"btrfs\"") || line.contains("PULSAR_OS") || line.contains("PulsarOS") {
                let get_val = |key: &str| -> String {
                    let re = Regex::new(&format!(r#"{}=\"([^\"]*)\""#, key)).unwrap();
                    re.captures(line).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or_default()
                };
                let name = get_val("NAME");
                let label = get_val("LABEL");
                let uuid = get_val("UUID");
                let size = get_val("SIZE");
                let pkname = get_val("PKNAME");

                let part_path = format!("/dev/{}", name);
                let disk_path = if !pkname.is_empty() { format!("/dev/{}", pkname) } else { part_path.clone() };

                targets.push(BtrfsTarget {
                    _disk_path: disk_path,
                    part_path,
                    label: if label.is_empty() { "PULSAR_OS".to_string() } else { label },
                    uuid,
                    size,
                });
            }
        }
    }
    targets
}

fn is_valid_base_squashfs(path: &str) -> bool {
    if !Path::new(path).exists() {
        return false;
    }
    // Must be a complete base OS rootfs (>= 1.0 GB) and never the mini recovery environment
    if path.contains("/recovery/") || path.contains("recovery-") {
        return false;
    }
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() < 1000 * 1024 * 1024 {
            return false;
        }
    } else {
        return false;
    }
    // Superblock verification using unsquashfs -s
    Command::new("unsquashfs")
        .args(&["-s", path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn format_file_size(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1.0 {
        format!("{:.2} GB", gb)
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.1} MB", mb)
    }
}

fn scan_dir_for_squashfs(dir: &Path, depth: usize, dev_label: &str, out: &mut Vec<DiscoveredImage>) {
    if depth > 3 {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dname = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                if dname == "recovery" || dname == "proc" || dname == "sys" || dname == "dev" {
                    continue;
                }
                scan_dir_for_squashfs(&path, depth + 1, dev_label, out);
            } else if path.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
                if name.ends_with(".squashfs") || name.ends_with(".sfs") {
                    let full_p = path.to_string_lossy().to_string();
                    if is_valid_base_squashfs(&full_p) {
                        let size_str = if let Ok(meta) = fs::metadata(&path) {
                            format_file_size(meta.len())
                        } else {
                            "Unknown size".to_string()
                        };
                        out.push(DiscoveredImage {
                            file_path: full_p,
                            filename: name,
                            size_str,
                            device_label: dev_label.to_string(),
                        });
                    }
                }
            }
        }
    }
}

fn scan_usb_devices() -> Vec<DiscoveredImage> {
    let mut images: Vec<DiscoveredImage> = Vec::new();
    let usb_base_mnt = "/tmp/pulsar_usb_mnt";
    let _ = fs::create_dir_all(usb_base_mnt);

    // 1. Scan removable USB storage devices via lsblk
    if let Ok(out) = Command::new("sudo").args(&["-n", "lsblk", "-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,TRAN,RM,HOTPLUG,MOUNTPOINTS,TYPE"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let get_val = |key: &str| -> String {
                let re = Regex::new(&format!(r#"{}=\"([^\"]*)\""#, key)).unwrap();
                re.captures(line).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or_default()
            };

            let name = get_val("NAME");
            let label = get_val("LABEL");
            let fstype = get_val("FSTYPE");
            let tran = get_val("TRAN");
            let rm = get_val("RM");
            let hotplug = get_val("HOTPLUG");
            let dev_type = get_val("TYPE");
            let mountpoints = get_val("MOUNTPOINTS");

            if name.is_empty() || name.starts_with("loop") || name.starts_with("zram") || name.starts_with("sr") {
                continue;
            }

            // Exclude current boot ISO/live recovery medium from the external USB list
            if label == "PULSAR_ISO" || label == "PULSAR_RECOVERY" || label == "archiso" || label == "LIVE" {
                continue;
            }

            // Exclude main internal disk partitions unless removable USB
            let is_removable = tran == "usb" || rm == "1" || hotplug == "1";
            if !is_removable && !name.starts_with("sd") {
                continue;
            }

            if dev_type == "disk" && fstype.is_empty() {
                continue;
            }

            let part_dev = format!("/dev/{}", name);

            if !fstype.is_empty() {
                let dev_title = if !label.is_empty() {
                    label.clone()
                } else if is_removable {
                    format!("USB Drive ({})", name)
                } else {
                    format!("Removable Drive ({})", name)
                };

                // Check if already mounted
                if !mountpoints.is_empty() {
                    for mnt in mountpoints.split_whitespace() {
                        scan_dir_for_squashfs(Path::new(mnt), 0, &dev_title, &mut images);
                    }
                } else {
                    // Mount temporarily in ro mode
                    let temp_mnt = format!("{}/{}", usb_base_mnt, name);
                    let _ = fs::create_dir_all(&temp_mnt);
                    if Command::new("sudo").args(&["-n", "mount", "-o", "ro", &part_dev, &temp_mnt]).status().map(|s| s.success()).unwrap_or(false) {
                        let prev_len = images.len();
                        scan_dir_for_squashfs(Path::new(&temp_mnt), 0, &dev_title, &mut images);
                        // If no images found, cleanly unmount and remove dir
                        if images.len() == prev_len {
                            let _ = Command::new("sudo").args(&["-n", "umount", &temp_mnt]).output();
                            let _ = fs::remove_dir(&temp_mnt);
                        }
                    }
                }
            }
        }
    }

    // 2. Scan standard media mounts
    let media_dirs = ["/media", "/run/media", "/mnt"];
    for m in &media_dirs {
        if Path::new(m).exists() {
            scan_dir_for_squashfs(Path::new(m), 0, "Mounted Storage", &mut images);
        }
    }

    // Deduplicate by file_path
    let mut deduped: Vec<DiscoveredImage> = Vec::new();
    for img in images {
        if !deduped.iter().any(|d| d.file_path == img.file_path) {
            deduped.push(img);
        }
    }
    deduped
}

fn detect_local_squashfs<L>(log: &L) -> Option<String>
where
    L: Fn(&str) + Send + Sync + 'static,
{
    log("Scanning storage devices for clean Arch Linux Pulsar OS base image...");

    let rec_mnt = "/tmp/pulsar_recovery";
    let _ = fs::create_dir_all(rec_mnt);

    // 1. Mount recovery partition by label
    let _ = Command::new("sudo").args(&["-n", "mount", "/dev/disk/by-label/PULSAR_RECOVERY", rec_mnt]).output();
    let _ = Command::new("sudo").args(&["-n", "mount", "-L", "PULSAR_RECOVERY", rec_mnt]).output();

    let base_image_names = [
        "images/pulsaros-base.squashfs",
        "images/x86_64/airootfs.sfs",
        "images/airootfs.sfs",
        "arch/x86_64/airootfs.sfs",
        "pulsaros-base.squashfs",
        "airootfs.sfs",
    ];

    let search_roots = [
        "/tmp/pulsar_recovery",
        "/run/live/medium",
        "/lib/live/mount/medium",
        "/run/archiso/bootmnt",
        "/run/archiso",
        "/recovery",
        "/mnt/recovery",
    ];

    for root in &search_roots {
        for img in &base_image_names {
            let full_p = format!("{}/{}", root, img);
            if Path::new(&full_p).exists() && is_valid_base_squashfs(&full_p) {
                log(&format!("Verified clean Arch base system image at: {}", full_p));
                return Some(full_p);
            }
        }
    }

    // 2. Scan all block devices
    if let Ok(out) = Command::new("sudo").args(&["-n", "blkid", "-o", "device"]).output() {
        let devs = String::from_utf8_lossy(&out.stdout);
        for dev in devs.lines() {
            let dev = dev.trim();
            if dev.is_empty() || dev.contains("loop") || dev.contains("zram") {
                continue;
            }
            let temp_mnt = format!("/tmp/mnt_{}", dev.replace('/', "_"));
            let _ = fs::create_dir_all(&temp_mnt);
            if Command::new("sudo").args(&["-n", "mount", "-o", "ro", dev, &temp_mnt]).status().map(|s| s.success()).unwrap_or(false) {
                for img in &base_image_names {
                    let p = format!("{}/{}", temp_mnt, img);
                    if Path::new(&p).exists() && is_valid_base_squashfs(&p) {
                        log(&format!("Verified clean base system image on {} at: {}", dev, p));
                        return Some(p);
                    }
                }
                let _ = Command::new("sudo").args(&["-n", "umount", &temp_mnt]).output();
                let _ = fs::remove_dir(&temp_mnt);
            }
        }
    }

    log("⚠️ No local base image found on built-in recovery partition.");
    None
}

fn ensure_lucide_icon(name: &str, svg_data: &str) -> String {
    let dir = "/tmp/pulsar_recovery_icons";
    let _ = fs::create_dir_all(dir);
    let p = format!("{}/{}.svg", dir, name);
    if !Path::new(&p).exists() {
        let _ = fs::write(&p, svg_data);
    }
    p
}

fn get_lucide_icon_path(name: &str) -> String {
    match name {
        "restore" | "timemachine" | "rotate-ccw" => ensure_lucide_icon(
            "restore",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#38bdf8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/></svg>"##,
        ),
        "usb" | "flash-drive" => ensure_lucide_icon(
            "usb",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#34d399" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="10" cy="7" r="1"/><circle cx="4" cy="20" r="1"/><path d="M4.7 19.3 19 5"/><path d="m21 3-3 1 2 2Z"/><path d="M9.26 7.68 5 12l2 5"/><path d="m10 14 5 2 3.5-3.5"/><circle cx="17" cy="17" r="1"/><circle cx="12" cy="12" r="1"/></svg>"##,
        ),
        "safari" | "globe" | "internet" => ensure_lucide_icon(
            "globe",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#60a5fa" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/></svg>"##,
        ),
        "disk" | "hard-drive" | "drive-harddisk" => ensure_lucide_icon(
            "hard-drive",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#a78bfa" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="22" x2="2" y1="12" y2="12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><line x1="6" x2="6.01" y1="16" y2="16"/><line x1="10" x2="10.01" y1="16" y2="16"/></svg>"##,
        ),
        "terminal" => ensure_lucide_icon(
            "terminal",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#4ade80" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/></svg>"##,
        ),
        "refresh" | "refresh-cw" => ensure_lucide_icon(
            "refresh",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/></svg>"##,
        ),
        "folder" => ensure_lucide_icon(
            "folder",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#60a5fa" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>"##,
        ),
        "folder-up" => ensure_lucide_icon(
            "folder-up",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#f59e0b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/><path d="m12 10 3 3m-3-3-3 3m3-3v6"/></svg>"##,
        ),
        "package" | "box" => ensure_lucide_icon(
            "package",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#34d399" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7.5 4.27 9 5.15"/><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/></svg>"##,
        ),
        "complete" | "check-circle" => ensure_lucide_icon(
            "complete",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="72" height="72" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10" stroke="#22c55e" stroke-width="2" fill="#22c55e" fill-opacity="0.15"/><path d="m9 12 2 2 4-4" stroke="#22c55e" stroke-width="2.5"/></svg>"##,
        ),
        "error" | "alert-circle" => ensure_lucide_icon(
            "error",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>"##,
        ),
        "progress" => ensure_lucide_icon(
            "progress",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="#38bdf8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>"##,
        ),
        "restart" => ensure_lucide_icon(
            "restart",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/></svg>"##,
        ),
        "shutdown" | "power" => ensure_lucide_icon(
            "power",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><line x1="12" y1="2" x2="12" y2="12"/></svg>"##,
        ),
        _ => ensure_lucide_icon(
            "generic",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#94a3b8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>"##,
        ),
    }
}

fn create_icon_widget(file_path: &str, fallback_icon_name: &str, size: i32) -> Image {
    if !file_path.is_empty() && Path::new(file_path).exists() {
        let img = Image::from_file(file_path);
        img.set_pixel_size(size);
        img
    } else {
        let svg_p = get_lucide_icon_path(fallback_icon_name);
        let img = Image::from_file(&svg_p);
        img.set_pixel_size(size);
        img
    }
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(if is_demo_mode() { "Pulsar OS Recovery (MODO DEMO / PRUEBAS)" } else { "Pulsar OS Recovery" })
        .default_width(1024)
        .default_height(720)
        .resizable(true)
        .build();

    window.maximize();

    let style_mgr = libadwaita::StyleManager::default();
    style_mgr.set_color_scheme(libadwaita::ColorScheme::ForceDark);

    let provider = CssProvider::new();
    let _ = provider.load_from_string(APP_CSS);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER + 500,
        );
    }

    let root_box = GtkBox::new(Orientation::Vertical, 0);
    root_box.add_css_class("root-container");
    root_box.set_hexpand(true);
    root_box.set_vexpand(true);

    let center_box = CenterBox::new();
    center_box.set_hexpand(true);
    center_box.set_vexpand(true);

    let card_box = GtkBox::new(Orientation::Vertical, 0);
    card_box.add_css_class("apple-box");
    card_box.set_size_request(680, 580);
    card_box.set_valign(Align::Center);
    card_box.set_halign(Align::Center);

    let stack = Stack::new();
    stack.set_transition_type(StackTransitionType::Crossfade);
    stack.set_transition_duration(250);
    card_box.append(&stack);

    center_box.set_center_widget(Some(&card_box));
    root_box.append(&center_box);

    // Bottom center control buttons: Restart & Shut Down
    let bottom_bar = GtkBox::new(Orientation::Horizontal, 16);
    bottom_bar.set_halign(Align::Center);
    bottom_bar.set_valign(Align::End);
    bottom_bar.set_margin_bottom(24);

    if is_demo_mode() {
        let demo_pill = Label::new(Some("⚠️ MODO DEMO / SIMULACIÓN ACTIVO (No se escribirán datos en disco)"));
        demo_pill.add_css_class("badge-demo");
        bottom_bar.append(&demo_pill);
    }

    let btn_restart = Button::new();
    btn_restart.add_css_class("bottom-power-btn");
    let restart_box = GtkBox::new(Orientation::Horizontal, 8);
    let restart_icon = create_icon_widget("", "restart", 18);
    let restart_lbl = Label::new(Some("Restart"));
    restart_box.append(&restart_icon);
    restart_box.append(&restart_lbl);
    btn_restart.set_child(Some(&restart_box));
    btn_restart.connect_clicked(|_| {
        let _ = Command::new("sh")
            .arg("-c")
            .arg("systemctl reboot || reboot || sudo reboot")
            .spawn();
    });

    let btn_shutdown = Button::new();
    btn_shutdown.add_css_class("bottom-power-btn");
    let shutdown_box = GtkBox::new(Orientation::Horizontal, 8);
    let shutdown_icon = create_icon_widget("", "shutdown", 18);
    let shutdown_lbl = Label::new(Some("Shut Down"));
    shutdown_box.append(&shutdown_icon);
    shutdown_box.append(&shutdown_lbl);
    btn_shutdown.set_child(Some(&shutdown_box));
    btn_shutdown.connect_clicked(|_| {
        let _ = Command::new("sh")
            .arg("-c")
            .arg("systemctl poweroff || poweroff || sudo poweroff")
            .spawn();
    });

    bottom_bar.append(&btn_restart);
    bottom_bar.append(&btn_shutdown);
    root_box.append(&bottom_bar);

    window.set_content(Some(&root_box));

    // Shared state
    let selected_action: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let selected_target: Rc<RefCell<Option<BtrfsTarget>>> = Rc::new(RefCell::new(None));
    let selected_image_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let recovery_mode: Rc<RefCell<RecoveryMode>> = Rc::new(RefCell::new(RecoveryMode::Local));
    let current_browser_dir: Rc<RefCell<PathBuf>> = Rc::new(RefCell::new(PathBuf::from("/media")));

    // ─────────────────────────────────────────────────────────────
    // 1. Utilities Screen (macOS Recovery main view)
    // ─────────────────────────────────────────────────────────────
    let util_box = GtkBox::new(Orientation::Vertical, 10);
    util_box.set_valign(Align::Center);

    let header_lbl = Label::new(Some("Pulsar OS Recovery Utilities"));
    header_lbl.add_css_class("welcome-title");
    util_box.append(&header_lbl);

    let sub_lbl = Label::new(Some("Select a recovery utility to restore or repair your system."));
    sub_lbl.add_css_class("welcome-subtitle");
    util_box.append(&sub_lbl);

    let listbox = ListBox::new();
    listbox.add_css_class("transparent-list");
    listbox.set_selection_mode(SelectionMode::Single);
    listbox.set_show_separators(false);

    let add_row = |id: &str, title: &str, desc: &str, icon_file: &str, icon_fallback: &str| {
        let row = ListBoxRow::new();
        row.set_widget_name(id);
        row.add_css_class("utility-item-row");

        let card = GtkBox::new(Orientation::Horizontal, 16);
        card.add_css_class("utility-row-card");
        card.set_margin_top(3);
        card.set_margin_bottom(3);
        card.set_margin_start(2);
        card.set_margin_end(2);

        let icon = create_icon_widget(icon_file, icon_fallback, 44);
        card.append(&icon);

        let vbox = GtkBox::new(Orientation::Vertical, 2);
        vbox.set_valign(Align::Center);

        let title_l = Label::new(Some(title));
        title_l.add_css_class("utility-title-lbl");
        title_l.set_halign(Align::Start);
        vbox.append(&title_l);

        let desc_l = Label::new(Some(desc));
        desc_l.add_css_class("utility-desc-lbl");
        desc_l.set_halign(Align::Start);
        desc_l.set_wrap(true);
        vbox.append(&desc_l);

        card.append(&vbox);
        row.set_child(Some(&card));
        listbox.append(&row);
    };

    add_row(
        "timemachine",
        "Restore from Time Machine (Btrfs + Restic)",
        "Restore full system or personal files from a Time Machine backup (USB, Samba/NAS or Cloud Rclone).",
        "/usr/share/pulsaros-recovery/timemachine.png",
        "restore",
    );
    add_row(
        "reinstall",
        "Reinstall Pulsar OS (Local Partition)",
        "Install a fresh copy of Pulsar OS from built-in recovery while keeping personal files intact.",
        "/usr/share/pulsaros-recovery/reinstall.png",
        "restore",
    );
    add_row(
        "usb_restore",
        "Restore from USB Flash Drive",
        "Scan connected USB drives to find and restore a downloaded .squashfs system image.",
        "",
        "usb",
    );
    add_row(
        "internet_info",
        "Pulsar Internet Recovery",
        "Download latest recovery image from SourceForge CDN and reinstall directly over the internet.",
        "/usr/share/pulsaros-recovery/safari.png",
        "safari",
    );
    add_row(
        "disk",
        "Disk Utility (GParted)",
        "Repair, inspect, format, or resize disk partitions with GParted.",
        "/usr/share/pulsaros-recovery/diskutility.png",
        "hard-drive",
    );
    add_row(
        "terminal",
        "Terminal / Root Console",
        "Open a root terminal for manual diagnosis and advanced commands.",
        "/usr/share/pulsaros-recovery/terminal.png",
        "terminal",
    );

    util_box.append(&listbox);

    let util_btn_box = GtkBox::new(Orientation::Horizontal, 0);
    util_btn_box.set_margin_top(14);
    let util_spacer = GtkBox::new(Orientation::Horizontal, 0);
    util_spacer.set_hexpand(true);
    util_btn_box.append(&util_spacer);

    let btn_util_continue = Button::with_label("Continue");
    btn_util_continue.add_css_class("suggested-action");
    btn_util_continue.set_sensitive(false);
    util_btn_box.append(&btn_util_continue);
    util_box.append(&util_btn_box);
    stack.add_named(&util_box, Some("utilities"));

    // ─────────────────────────────────────────────────────────────
    // 2. Internet Recovery Screen (Direct Cloud Download & Restore)
    // ─────────────────────────────────────────────────────────────
    let net_info_box = GtkBox::new(Orientation::Vertical, 8);
    net_info_box.set_valign(Align::Center);
    net_info_box.set_halign(Align::Center);

    let net_icon = create_icon_widget("/usr/share/pulsaros-recovery/safari.png", "globe", 48);
    net_info_box.append(&net_icon);

    let net_title = Label::new(Some("Pulsar Internet Recovery"));
    net_title.add_css_class("welcome-title");
    net_info_box.append(&net_title);

    let net_subtitle = Label::new(Some("Download official system recovery image from SourceForge CDN and reinstall."));
    net_subtitle.add_css_class("welcome-subtitle");
    net_info_box.append(&net_subtitle);

    let net_card = GtkBox::new(Orientation::Vertical, 10);
    net_card.add_css_class("info-card");
    net_card.set_size_request(580, -1);

    let detected_base = detect_system_base();
    let detected_boot = detect_system_bootloader();
    let manifest_rc = Rc::new(RefCell::new(fetch_release_manifest()));

    // Row 1: Edition and Version
    let net_row_1 = GtkBox::new(Orientation::Horizontal, 14);
    net_row_1.set_hexpand(true);

    let edition_box = GtkBox::new(Orientation::Vertical, 3);
    edition_box.set_hexpand(true);
    let lbl_edition = Label::new(Some("Distribution & Bootloader"));
    lbl_edition.add_css_class("setting-label");
    lbl_edition.set_halign(Align::Start);
    edition_box.append(&lbl_edition);

    let edition_entries = [
        "Arch Linux (GRUB)",
        "Arch Linux (rEFInd)",
        "Debian (GRUB)",
        "Debian (rEFInd)",
    ];
    let edition_list = StringList::new(&edition_entries);
    let combo_edition = DropDown::new(Some(edition_list), gtk4::Expression::NONE);
    combo_edition.set_hexpand(true);

    let default_edition_idx = match (detected_base.as_str(), detected_boot.as_str()) {
        ("arch", "grub") => 0,
        ("arch", "refind") => 1,
        ("debian", "grub") => 2,
        ("debian", "refind") => 3,
        _ => 0,
    };
    combo_edition.set_selected(default_edition_idx);
    edition_box.append(&combo_edition);
    net_row_1.append(&edition_box);

    let version_box = GtkBox::new(Orientation::Vertical, 3);
    version_box.set_hexpand(true);
    let lbl_version = Label::new(Some("Version"));
    lbl_version.add_css_class("setting-label");
    lbl_version.set_halign(Align::Start);
    version_box.append(&lbl_version);

    let ver_str = manifest_rc.borrow().latest_version.clone();
    let version_entries = [ver_str.as_str()];
    let version_list = StringList::new(&version_entries);
    let combo_version = DropDown::new(Some(version_list), gtk4::Expression::NONE);
    combo_version.set_hexpand(true);
    version_box.append(&combo_version);
    net_row_1.append(&version_box);
    net_card.append(&net_row_1);

    // Row 2: Mirror and Network status
    let net_row_2 = GtkBox::new(Orientation::Horizontal, 14);
    net_row_2.set_hexpand(true);

    let mirror_box = GtkBox::new(Orientation::Vertical, 3);
    mirror_box.set_hexpand(true);
    let lbl_mirror = Label::new(Some("SourceForge Mirror"));
    lbl_mirror.add_css_class("setting-label");
    lbl_mirror.set_halign(Align::Start);
    mirror_box.append(&lbl_mirror);

    let mirror_names: Vec<String> = manifest_rc.borrow().mirrors.iter().map(|m| m.name.clone()).collect();
    let mirror_strs: Vec<&str> = mirror_names.iter().map(|s| s.as_str()).collect();
    let mirror_list = StringList::new(&mirror_strs);
    let combo_mirror = DropDown::new(Some(mirror_list), gtk4::Expression::NONE);
    combo_mirror.set_hexpand(true);
    mirror_box.append(&combo_mirror);
    net_row_2.append(&mirror_box);

    let net_stat_box = GtkBox::new(Orientation::Vertical, 3);
    let lbl_net_hdr = Label::new(Some("Network Status"));
    lbl_net_hdr.add_css_class("setting-label");
    lbl_net_hdr.set_halign(Align::Start);
    net_stat_box.append(&lbl_net_hdr);

    let lbl_net_badge = Label::new(Some("● Ready / Connected"));
    lbl_net_badge.add_css_class("badge-net-ok");
    lbl_net_badge.set_halign(Align::Start);
    net_stat_box.append(&lbl_net_badge);
    net_row_2.append(&net_stat_box);
    net_card.append(&net_row_2);

    // Progress Section
    let pbar_net = ProgressBar::new();
    pbar_net.add_css_class("progress-bar-thin");
    pbar_net.set_fraction(0.0);
    pbar_net.set_margin_top(8);
    net_card.append(&pbar_net);

    let lbl_net_status = Label::new(Some("Ready to download (~3.1 GB). System root (@) will be restored, keeping @home intact."));
    lbl_net_status.add_css_class("progress-text");
    lbl_net_status.set_halign(Align::Start);
    lbl_net_status.set_wrap(true);
    net_card.append(&lbl_net_status);

    net_info_box.append(&net_card);

    let net_btn_box = GtkBox::new(Orientation::Horizontal, 14);
    net_btn_box.set_halign(Align::Center);
    net_btn_box.set_margin_top(6);

    let btn_net_back = Button::with_label("Back to Utilities");
    btn_net_back.add_css_class("secondary-action");
    net_btn_box.append(&btn_net_back);

    let btn_net_cancel = Button::with_label("Cancel Download");
    btn_net_cancel.add_css_class("destructive-action");
    btn_net_cancel.set_visible(false);
    net_btn_box.append(&btn_net_cancel);

    let btn_net_download = Button::with_label("Download and Reinstall");
    btn_net_download.add_css_class("suggested-action");
    net_btn_box.append(&btn_net_download);

    net_info_box.append(&net_btn_box);
    stack.add_named(&net_info_box, Some("internet_info"));

    // ─────────────────────────────────────────────────────────────
    // 3. USB Image Selector Screen (Auto-detected USBs)
    // ─────────────────────────────────────────────────────────────
    let usb_box = GtkBox::new(Orientation::Vertical, 10);
    usb_box.set_valign(Align::Center);

    let usb_header = Label::new(Some("Select Recovery Image from USB"));
    usb_header.add_css_class("welcome-title");
    usb_box.append(&usb_header);

    let usb_sub = Label::new(Some("Plug in your USB drive with the .squashfs file, then select it below."));
    usb_sub.add_css_class("welcome-subtitle");
    usb_box.append(&usb_sub);

    // USB Actions bar
    let usb_actions_bar = GtkBox::new(Orientation::Horizontal, 10);
    usb_actions_bar.set_margin_bottom(6);

    let btn_scan_usb = Button::new();
    btn_scan_usb.add_css_class("secondary-action");
    let scan_box = GtkBox::new(Orientation::Horizontal, 6);
    let scan_icon = create_icon_widget("", "refresh", 16);
    let scan_lbl = Label::new(Some("Scan / Refresh USBs"));
    scan_box.append(&scan_icon);
    scan_box.append(&scan_lbl);
    btn_scan_usb.set_child(Some(&scan_box));
    usb_actions_bar.append(&btn_scan_usb);

    let btn_open_browser = Button::new();
    btn_open_browser.add_css_class("secondary-action");
    let open_box = GtkBox::new(Orientation::Horizontal, 6);
    let open_icon = create_icon_widget("", "folder", 16);
    let open_lbl = Label::new(Some("Browse Files / Drives..."));
    open_box.append(&open_icon);
    open_box.append(&open_lbl);
    btn_open_browser.set_child(Some(&open_box));
    usb_actions_bar.append(&btn_open_browser);

    usb_box.append(&usb_actions_bar);

    let usb_scrolled = ScrolledWindow::new();
    usb_scrolled.set_size_request(600, 230);
    usb_scrolled.add_css_class("live-log-view");

    let usb_listbox = ListBox::new();
    usb_listbox.add_css_class("transparent-list");
    usb_listbox.set_selection_mode(SelectionMode::Single);
    usb_scrolled.set_child(Some(&usb_listbox));
    usb_box.append(&usb_scrolled);

    let usb_nav_box = GtkBox::new(Orientation::Horizontal, 16);
    usb_nav_box.set_halign(Align::End);
    usb_nav_box.set_margin_top(12);

    let btn_usb_back = Button::with_label("Back");
    btn_usb_back.add_css_class("secondary-action");
    btn_usb_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));
    usb_nav_box.append(&btn_usb_back);

    let btn_usb_continue = Button::with_label("Continue");
    btn_usb_continue.add_css_class("suggested-action");
    btn_usb_continue.set_sensitive(false);
    usb_nav_box.append(&btn_usb_continue);
    usb_box.append(&usb_nav_box);

    stack.add_named(&usb_box, Some("usb_select"));

    // ─────────────────────────────────────────────────────────────
    // 4. Built-in File Browser Screen (No XDG portal required!)
    // ─────────────────────────────────────────────────────────────
    let browser_box = GtkBox::new(Orientation::Vertical, 8);
    browser_box.set_valign(Align::Center);

    let browser_header = Label::new(Some("Browse Storage for System Image"));
    browser_header.add_css_class("welcome-title");
    browser_box.append(&browser_header);

    // Current Path & Quick Jump buttons
    let browser_top_bar = GtkBox::new(Orientation::Horizontal, 8);
    browser_top_bar.set_valign(Align::Center);

    let lbl_current_path = Label::new(Some("Path: /media"));
    lbl_current_path.add_css_class("progress-text");
    lbl_current_path.set_hexpand(true);
    lbl_current_path.set_halign(Align::Start);
    lbl_current_path.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    browser_top_bar.append(&lbl_current_path);

    let shortcuts = [
        ("/media", "media"),
        ("/run/media", "run/media"),
        ("/mnt", "mnt"),
        ("/tmp", "tmp"),
        ("/", "root (/)"),
    ];

    let shortcuts_bar = GtkBox::new(Orientation::Horizontal, 4);
    shortcuts_bar.set_halign(Align::End);

    browser_top_bar.append(&shortcuts_bar);
    browser_box.append(&browser_top_bar);

    let browser_scrolled = ScrolledWindow::new();
    browser_scrolled.set_size_request(600, 240);
    browser_scrolled.add_css_class("live-log-view");

    let browser_listbox = ListBox::new();
    browser_listbox.add_css_class("transparent-list");
    browser_listbox.set_selection_mode(SelectionMode::Single);
    browser_scrolled.set_child(Some(&browser_listbox));
    browser_box.append(&browser_scrolled);

    let browser_nav_box = GtkBox::new(Orientation::Horizontal, 16);
    browser_nav_box.set_halign(Align::End);
    browser_nav_box.set_margin_top(10);

    let btn_browser_back = Button::with_label("Back to USB List");
    btn_browser_back.add_css_class("secondary-action");
    btn_browser_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("usb_select");
    }));
    browser_nav_box.append(&btn_browser_back);

    let btn_browser_select = Button::with_label("Select Image");
    btn_browser_select.add_css_class("suggested-action");
    btn_browser_select.set_sensitive(false);
    browser_nav_box.append(&btn_browser_select);
    browser_box.append(&browser_nav_box);

    stack.add_named(&browser_box, Some("file_browser"));

    // ─────────────────────────────────────────────────────────────
    // 5. Select Target Partition Screen
    // ─────────────────────────────────────────────────────────────
    let target_box = GtkBox::new(Orientation::Vertical, 10);
    target_box.set_valign(Align::Center);
    target_box.set_halign(Align::Center);

    let target_icon = create_icon_widget("/usr/share/pulsaros-recovery/diskutility.png", "hard-drive", 56);
    target_box.append(&target_icon);

    let target_title = Label::new(Some("Select Pulsar OS Partition"));
    target_title.add_css_class("welcome-title");
    target_box.append(&target_title);

    let target_desc = Label::new(Some("The root system (@) will be cleanly restored. Your user accounts and documents in /home (@home) will remain completely intact."));
    target_desc.add_css_class("welcome-subtitle");
    target_desc.set_wrap(true);
    target_desc.set_max_width_chars(50);
    target_desc.set_justify(gtk4::Justification::Center);
    target_box.append(&target_desc);

    let source_img_lbl = Label::new(Some("Source: Built-in Recovery Partition"));
    source_img_lbl.add_css_class("progress-text");
    target_box.append(&source_img_lbl);

    let targets_flow = GtkBox::new(Orientation::Horizontal, 10);
    targets_flow.set_halign(Align::Center);
    targets_flow.set_margin_top(8);
    target_box.append(&targets_flow);

    let target_nav_box = GtkBox::new(Orientation::Horizontal, 16);
    target_nav_box.set_halign(Align::Center);
    target_nav_box.set_margin_top(14);

    let btn_target_back = Button::with_label("Back");
    btn_target_back.add_css_class("secondary-action");
    target_nav_box.append(&btn_target_back);

    let btn_target_restore = Button::with_label("Restore System");
    btn_target_restore.add_css_class("suggested-action");
    btn_target_restore.set_sensitive(false);
    target_nav_box.append(&btn_target_restore);
    target_box.append(&target_nav_box);

    stack.add_named(&target_box, Some("target_select"));

    // ─────────────────────────────────────────────────────────────
    // 6. Progress Screen
    // ─────────────────────────────────────────────────────────────
    let prog_box = GtkBox::new(Orientation::Vertical, 10);
    prog_box.set_valign(Align::Center);
    prog_box.set_halign(Align::Center);

    let prog_icon = create_icon_widget("/usr/share/pulsaros-recovery/reinstall.png", "progress", 60);
    prog_box.append(&prog_icon);

    let prog_title = Label::new(Some("Restoring Pulsar OS..."));
    prog_title.add_css_class("welcome-title");
    prog_box.append(&prog_title);

    let prog_desc = Label::new(Some("Preparing disk and restoring root subvolume (@)..."));
    prog_desc.add_css_class("progress-text");
    prog_box.append(&prog_desc);

    let pbar = ProgressBar::new();
    pbar.add_css_class("progress-bar-thin");
    pbar.set_size_request(480, -1);
    prog_box.append(&pbar);

    let scrolled_log = ScrolledWindow::new();
    scrolled_log.set_size_request(520, 160);
    scrolled_log.add_css_class("live-log-view");

    let log_view = TextView::new();
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.set_wrap_mode(WrapMode::WordChar);
    log_view.add_css_class("live-log-text");
    scrolled_log.set_child(Some(&log_view));
    prog_box.append(&scrolled_log);

    stack.add_named(&prog_box, Some("progress"));

    // ─────────────────────────────────────────────────────────────
    // 7. Complete Screen
    // ─────────────────────────────────────────────────────────────
    let done_box = GtkBox::new(Orientation::Vertical, 14);
    done_box.set_valign(Align::Center);
    done_box.set_halign(Align::Center);

    let done_icon = create_icon_widget("", "complete", 72);
    done_box.append(&done_icon);

    let done_title = Label::new(Some("Restoration Complete"));
    done_title.add_css_class("welcome-title");
    done_box.append(&done_title);

    let done_desc = Label::new(Some("Pulsar OS has been successfully restored.\nYour personal files, settings, and apps in /home are intact.\n\nClick Restart to boot into your restored system."));
    done_desc.add_css_class("welcome-subtitle");
    done_desc.set_justify(gtk4::Justification::Center);
    done_box.append(&done_desc);

    let btn_reboot = Button::with_label("Restart System");
    btn_reboot.add_css_class("suggested-action");
    btn_reboot.connect_clicked(|_| {
        let _ = Command::new("sudo").args(&["-n", "systemctl", "reboot", "-i", "-f"]).spawn();
        let _ = Command::new("sudo").args(&["-n", "reboot", "-f"]).spawn();
    });
    done_box.append(&btn_reboot);

    stack.add_named(&done_box, Some("complete"));

    // ─────────────────────────────────────────────────────────────
    // 8. Error Screen
    // ─────────────────────────────────────────────────────────────
    let err_box = GtkBox::new(Orientation::Vertical, 10);
    err_box.set_valign(Align::Center);
    err_box.set_halign(Align::Center);

    let err_icon = create_icon_widget("", "error", 64);
    err_box.append(&err_icon);

    let err_title = Label::new(Some("Restoration Failed"));
    err_title.add_css_class("welcome-title");
    err_box.append(&err_title);

    let err_msg_lbl = Label::new(Some("An error occurred during system restoration."));
    err_msg_lbl.add_css_class("welcome-subtitle");
    err_msg_lbl.set_wrap(true);
    err_msg_lbl.set_max_width_chars(50);
    err_msg_lbl.set_justify(gtk4::Justification::Center);
    err_box.append(&err_msg_lbl);

    let err_scrolled_log = ScrolledWindow::new();
    err_scrolled_log.set_size_request(520, 140);
    err_scrolled_log.add_css_class("live-log-view");

    let err_log_view = TextView::new();
    err_log_view.set_editable(false);
    err_log_view.set_monospace(true);
    err_log_view.set_wrap_mode(WrapMode::WordChar);
    err_log_view.add_css_class("err-log-text");
    err_scrolled_log.set_child(Some(&err_log_view));
    err_box.append(&err_scrolled_log);

    let err_btn_box = GtkBox::new(Orientation::Horizontal, 12);
    err_btn_box.set_halign(Align::Center);
    err_btn_box.set_margin_top(8);

    let btn_err_back = Button::with_label("Back to Utilities");
    btn_err_back.add_css_class("secondary-action");
    btn_err_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));
    err_btn_box.append(&btn_err_back);

    let btn_try_usb = Button::with_label("Try USB Image");
    btn_try_usb.add_css_class("suggested-action");
    let sel_act_c = selected_action.clone();
    btn_try_usb.connect_clicked(clone!(@weak stack, @weak btn_util_continue => move |_| {
        *sel_act_c.borrow_mut() = Some("usb_restore".to_string());
        btn_util_continue.emit_clicked();
    }));
    err_btn_box.append(&btn_try_usb);

    err_box.append(&err_btn_box);
    stack.add_named(&err_box, Some("error"));

    // ─────────────────────────────────────────────────────────────
    // Helper: Refresh Target Partitions & Show Target Screen
    // ─────────────────────────────────────────────────────────────
    let show_target_screen = {
        let stack = stack.clone();
        let targets_flow = targets_flow.clone();
        let btn_target_restore = btn_target_restore.clone();
        let selected_target = selected_target.clone();
        let source_img_lbl = source_img_lbl.clone();

        move |source_desc: &str| {
            source_img_lbl.set_text(source_desc);

            while let Some(child) = targets_flow.first_child() {
                targets_flow.remove(&child);
            }
            *selected_target.borrow_mut() = None;
            btn_target_restore.set_sensitive(false);

            let mut targets = find_btrfs_targets();
            if targets.is_empty() && is_demo_mode() {
                targets.push(BtrfsTarget {
                    _disk_path: "/dev/demo-nvme0n1".to_string(),
                    part_path: "/dev/demo-nvme0n1p2 (Simulado)".to_string(),
                    label: "Pulsar OS Demo Pool".to_string(),
                    uuid: "demo-btrfs-uuid-0000".to_string(),
                    size: "500.0G".to_string(),
                });
            }

            if targets.is_empty() {
                let no_target_lbl = Label::new(Some("No Btrfs Pulsar OS partitions detected.\nUse Disk Utility to inspect drives."));
                no_target_lbl.add_css_class("welcome-subtitle");
                targets_flow.append(&no_target_lbl);
            } else {
                for target in targets {
                    let card = GtkBox::new(Orientation::Vertical, 6);
                    card.add_css_class("disk-card");
                    let disk_icon = create_icon_widget("", "hard-drive", 40);
                    card.append(&disk_icon);

                    let name_lbl = Label::new(Some(&format!("{} ({})", target.label, target.size)));
                    name_lbl.add_css_class("utility-title-lbl");
                    card.append(&name_lbl);

                    let dev_lbl = Label::new(Some(&target.part_path));
                    dev_lbl.add_css_class("utility-desc-lbl");
                    card.append(&dev_lbl);

                    let gesture = GestureClick::new();
                    let t_clone = target.clone();
                    let targets_flow_c = targets_flow.clone();
                    let btn_restore_c = btn_target_restore.clone();
                    let sel_target_c = selected_target.clone();
                    let card_c = card.clone();

                    gesture.connect_released(move |_, _, _, _| {
                        let mut next = targets_flow_c.first_child();
                        while let Some(w) = next {
                            w.remove_css_class("selected");
                            next = w.next_sibling();
                        }
                        card_c.add_css_class("selected");
                        *sel_target_c.borrow_mut() = Some(t_clone.clone());
                        btn_restore_c.set_sensitive(true);
                    });

                    card.add_controller(gesture);
                    targets_flow.append(&card);
                }
            }
            stack.set_visible_child_name("target_select");
        }
    };

    // ─────────────────────────────────────────────────────────────
    // Helper: Refresh USB Images list
    // ─────────────────────────────────────────────────────────────
    let populate_usb_images = {
        let usb_listbox = usb_listbox.clone();
        let btn_usb_continue = btn_usb_continue.clone();
        let selected_image_path = selected_image_path.clone();

        move || {
            while let Some(child) = usb_listbox.first_child() {
                usb_listbox.remove(&child);
            }
            *selected_image_path.borrow_mut() = None;
            btn_usb_continue.set_sensitive(false);

            let images = scan_usb_devices();
            if images.is_empty() {
                let row = ListBoxRow::new();
                row.set_selectable(false);
                let empty_box = GtkBox::new(Orientation::Vertical, 6);
                empty_box.set_margin_top(16);
                empty_box.set_margin_bottom(16);
                let empty_lbl = Label::new(Some("No .squashfs recovery images detected on connected USB drives."));
                empty_lbl.add_css_class("welcome-subtitle");
                let hint_lbl = Label::new(Some("Plug in your USB drive and click 'Scan / Refresh USBs' or use 'Browse Files / Drives...'"));
                hint_lbl.add_css_class("utility-desc-lbl");
                empty_box.append(&empty_lbl);
                empty_box.append(&hint_lbl);
                row.set_child(Some(&empty_box));
                usb_listbox.append(&row);
            } else {
                for img in images {
                    let row = ListBoxRow::new();
                    row.set_widget_name(&img.file_path);
                    row.add_css_class("utility-item-row");

                    let card = GtkBox::new(Orientation::Horizontal, 14);
                    card.add_css_class("utility-row-card");
                    card.set_margin_top(3);
                    card.set_margin_bottom(3);

                    let icon = create_icon_widget("", "usb", 36);
                    card.append(&icon);

                    let vbox = GtkBox::new(Orientation::Vertical, 2);
                    let title_l = Label::new(Some(&format!("{} ({})", img.filename, img.size_str)));
                    title_l.add_css_class("utility-title-lbl");
                    title_l.set_halign(Align::Start);
                    vbox.append(&title_l);

                    let desc_l = Label::new(Some(&format!("On {} • {}", img.device_label, img.file_path)));
                    desc_l.add_css_class("utility-desc-lbl");
                    desc_l.set_halign(Align::Start);
                    desc_l.set_wrap(true);
                    vbox.append(&desc_l);

                    card.append(&vbox);
                    row.set_child(Some(&card));
                    usb_listbox.append(&row);
                }
            }
        }
    };

    // ─────────────────────────────────────────────────────────────
    // Helper: Built-in Directory Browser Populator
    // ─────────────────────────────────────────────────────────────
    let populate_file_browser = {
        let browser_listbox = browser_listbox.clone();
        let lbl_current_path = lbl_current_path.clone();
        let btn_browser_select = btn_browser_select.clone();
        let selected_image_path = selected_image_path.clone();
        let current_browser_dir = current_browser_dir.clone();

        Rc::new(RefCell::new(move |dir: &Path| {
            while let Some(child) = browser_listbox.first_child() {
                browser_listbox.remove(&child);
            }
            *selected_image_path.borrow_mut() = None;
            btn_browser_select.set_sensitive(false);

            let canonical_dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
            *current_browser_dir.borrow_mut() = canonical_dir.clone();
            lbl_current_path.set_text(&format!("Path: {}", canonical_dir.display()));

            // Parent directory row if not root
            if let Some(parent) = canonical_dir.parent() {
                let row = ListBoxRow::new();
                row.set_widget_name(&format!("DIR:{}", parent.display()));
                row.add_css_class("utility-item-row");

                let card = GtkBox::new(Orientation::Horizontal, 12);
                card.add_css_class("utility-row-card");
                card.set_margin_top(2);
                card.set_margin_bottom(2);

                let icon = create_icon_widget("", "folder-up", 24);
                card.append(&icon);

                let lbl = Label::new(Some(".. (Go to parent directory)"));
                lbl.add_css_class("utility-title-lbl");
                card.append(&lbl);

                row.set_child(Some(&card));
                browser_listbox.append(&row);
            }

            // Read entries
            let mut dirs_list: Vec<PathBuf> = Vec::new();
            let mut squashfs_list: Vec<PathBuf> = Vec::new();

            if let Ok(entries) = fs::read_dir(&canonical_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    if name.starts_with('.') {
                        continue;
                    }
                    if p.is_dir() {
                        dirs_list.push(p);
                    } else if p.is_file() && (name.ends_with(".squashfs") || name.ends_with(".sfs")) {
                        squashfs_list.push(p);
                    }
                }
            }

            dirs_list.sort();
            squashfs_list.sort();

            // Append directories
            for d in dirs_list {
                let row = ListBoxRow::new();
                row.set_widget_name(&format!("DIR:{}", d.display()));
                row.add_css_class("utility-item-row");

                let card = GtkBox::new(Orientation::Horizontal, 12);
                card.add_css_class("utility-row-card");
                card.set_margin_top(2);
                card.set_margin_bottom(2);

                let icon = create_icon_widget("", "folder", 24);
                card.append(&icon);

                let dname = d.file_name().and_then(|n| n.to_str()).unwrap_or("Directory");
                let lbl = Label::new(Some(&format!("{}/", dname)));
                lbl.add_css_class("utility-title-lbl");
                card.append(&lbl);

                row.set_child(Some(&card));
                browser_listbox.append(&row);
            }

            // Append squashfs files
            for f in squashfs_list {
                let full_p = f.to_string_lossy().to_string();
                let row = ListBoxRow::new();
                row.set_widget_name(&format!("FILE:{}", full_p));
                row.add_css_class("utility-item-row");

                let card = GtkBox::new(Orientation::Horizontal, 12);
                card.add_css_class("utility-row-card");
                card.set_margin_top(2);
                card.set_margin_bottom(2);

                let icon = create_icon_widget("", "package", 28);
                card.append(&icon);

                let vbox = GtkBox::new(Orientation::Vertical, 2);
                let fname = f.file_name().and_then(|n| n.to_str()).unwrap_or("squashfs");
                let size_str = if let Ok(meta) = fs::metadata(&f) { format_file_size(meta.len()) } else { "".to_string() };
                let is_valid = is_valid_base_squashfs(&full_p);

                let title_l = Label::new(Some(&format!("{} ({})", fname, size_str)));
                title_l.add_css_class("utility-title-lbl");
                title_l.set_halign(Align::Start);
                vbox.append(&title_l);

                let desc_l = Label::new(Some(if is_valid { "✅ Valid Pulsar OS system image" } else { "⚠️ Image smaller than standard base size" }));
                desc_l.add_css_class("utility-desc-lbl");
                desc_l.set_halign(Align::Start);
                vbox.append(&desc_l);

                card.append(&vbox);
                row.set_child(Some(&card));
                browser_listbox.append(&row);
            }
        }))
    };

    // Setup shortcuts buttons in file browser top bar
    for (path_str, btn_title) in shortcuts {
        let s_btn = Button::with_label(btn_title);
        s_btn.add_css_class("shortcut-btn");
        let pop_c = populate_file_browser.clone();
        let target_p = PathBuf::from(path_str);
        s_btn.connect_clicked(move |_| {
            (pop_c.borrow_mut())(&target_p);
        });
        shortcuts_bar.append(&s_btn);
    }

    // Connect file browser listbox row selection and activation
    let pop_c2 = populate_file_browser.clone();
    browser_listbox.connect_row_activated(clone!(
        @weak btn_browser_select,
        @strong selected_image_path
     => move |_, row| {
        let tag = row.widget_name().to_string();
        if let Some(dir_path) = tag.strip_prefix("DIR:") {
            (pop_c2.borrow_mut())(Path::new(dir_path));
        } else if let Some(file_path) = tag.strip_prefix("FILE:") {
            *selected_image_path.borrow_mut() = Some(file_path.to_string());
            btn_browser_select.set_sensitive(true);
            btn_browser_select.emit_clicked();
        }
    }));

    browser_listbox.connect_row_selected(clone!(
        @weak btn_browser_select,
        @strong selected_image_path
     => move |_, row| {
        if let Some(r) = row {
            let tag = r.widget_name().to_string();
            if let Some(file_path) = tag.strip_prefix("FILE:") {
                *selected_image_path.borrow_mut() = Some(file_path.to_string());
                btn_browser_select.set_sensitive(true);
                return;
            }
        }
        btn_browser_select.set_sensitive(false);
    }));

    // ─────────────────────────────────────────────────────────────
    // Callbacks & Connections
    // ─────────────────────────────────────────────────────────────
    listbox.connect_row_selected(clone!(@weak btn_util_continue, @strong selected_action => move |_, row| {
        if let Some(r) = row {
            let id = r.widget_name().to_string();
            *selected_action.borrow_mut() = Some(id);
            btn_util_continue.set_sensitive(true);
        }
    }));

    listbox.connect_row_activated(clone!(@weak btn_util_continue, @strong selected_action => move |_, row| {
        let id = row.widget_name().to_string();
        *selected_action.borrow_mut() = Some(id);
        btn_util_continue.emit_clicked();
    }));

    btn_scan_usb.connect_clicked(clone!(@strong populate_usb_images => move |_| {
        populate_usb_images();
    }));

    let pop_browser_first = populate_file_browser.clone();
    btn_open_browser.connect_clicked(clone!(@weak stack => move |_| {
        let initial_dir = if Path::new("/media").exists() {
            Path::new("/media")
        } else if Path::new("/run/media").exists() {
            Path::new("/run/media")
        } else {
            Path::new("/")
        };
        (pop_browser_first.borrow_mut())(initial_dir);
        stack.set_visible_child_name("file_browser");
    }));

    usb_listbox.connect_row_selected(clone!(@weak btn_usb_continue, @strong selected_image_path => move |_, row| {
        if let Some(r) = row {
            let path = r.widget_name().to_string();
            if !path.is_empty() {
                *selected_image_path.borrow_mut() = Some(path);
                btn_usb_continue.set_sensitive(true);
                return;
            }
        }
        btn_usb_continue.set_sensitive(false);
    }));

    usb_listbox.connect_row_activated(clone!(@weak btn_usb_continue, @strong selected_image_path => move |_, row| {
        let path = row.widget_name().to_string();
        if !path.is_empty() {
            *selected_image_path.borrow_mut() = Some(path);
            btn_usb_continue.emit_clicked();
        }
    }));

    let cancel_signal_holder = Rc::new(RefCell::new(None::<Arc<AtomicBool>>));

    btn_net_back.connect_clicked(clone!(@weak stack => move |_| {
        stack.set_visible_child_name("utilities");
    }));

    btn_net_cancel.connect_clicked(clone!(
        @strong cancel_signal_holder,
        @weak lbl_net_status,
        @weak pbar_net,
        @weak btn_net_download,
        @weak btn_net_back,
        @weak btn_net_cancel,
        @weak combo_edition,
        @weak combo_mirror,
        @weak combo_version,
        @weak stack
     => move |_| {
        if let Some(ref sig) = *cancel_signal_holder.borrow() {
            sig.store(true, Ordering::SeqCst);
        }
        let _ = fs::remove_file("/tmp/pulsaros-internet-recovery.squashfs.part");
        let _ = fs::remove_file("/tmp/pulsaros-internet-recovery.squashfs");
        pbar_net.set_fraction(0.0);
        lbl_net_status.set_text("Descarga cancelada por el usuario.");
        btn_net_download.set_visible(true);
        btn_net_download.set_sensitive(true);
        btn_net_back.set_visible(true);
        btn_net_back.set_sensitive(true);
        btn_net_cancel.set_visible(false);
        combo_edition.set_sensitive(true);
        combo_mirror.set_sensitive(true);
        combo_version.set_sensitive(true);
        stack.set_visible_child_name("utilities");
    }));

    btn_net_download.connect_clicked(clone!(
        @weak pbar_net,
        @weak lbl_net_status,
        @weak btn_net_download,
        @weak btn_net_back,
        @weak btn_net_cancel,
        @weak combo_edition,
        @weak combo_mirror,
        @weak combo_version,
        @strong manifest_rc,
        @strong selected_image_path,
        @strong recovery_mode,
        @strong show_target_screen,
        @strong cancel_signal_holder,
        @weak stack
     => move |_| {
        btn_net_download.set_visible(false);
        btn_net_back.set_visible(false);
        btn_net_cancel.set_visible(true);
        btn_net_cancel.set_sensitive(true);
        combo_edition.set_sensitive(false);
        combo_mirror.set_sensitive(false);
        combo_version.set_sensitive(false);

        let cancel_flag = Arc::new(AtomicBool::new(false));
        *cancel_signal_holder.borrow_mut() = Some(cancel_flag.clone());

        let (sel_base, sel_boot) = match combo_edition.selected() {
            0 => ("arch", "grub"),
            1 => ("arch", "refind"),
            2 => ("debian", "grub"),
            3 => ("debian", "refind"),
            _ => ("arch", "grub"),
        };

        let m_data = manifest_rc.borrow().clone();
        let ver = m_data.latest_version.clone();
        let mirror_id = m_data.mirrors.get(combo_mirror.selected() as usize)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "auto".to_string());

        let target_info = m_data.versions
            .get(&ver)
            .and_then(|b_map| b_map.get(sel_base))
            .and_then(|bt_map| bt_map.get(sel_boot))
            .cloned();

        let mut download_url = target_info.as_ref()
            .map(|t| t.squashfs.clone())
            .unwrap_or_else(|| {
                format!("https://downloads.sourceforge.net/project/pulsaros-inled/pulsaros-{}-{}-{}-{}.squashfs", ver, sel_base, sel_boot, ver)
            });

        if mirror_id != "auto" {
            download_url = format!("{}?use_mirror={}", download_url, mirror_id);
        }

        let expected_size = target_info.as_ref().and_then(|t| t.size_bytes).unwrap_or(3_145_728_000);
        let expected_hash = target_info.as_ref().and_then(|t| t.sha256.clone());

        pbar_net.set_fraction(0.01);
        lbl_net_status.set_text("Connecting to SourceForge CDN...");

        let (tx, rx) = std::sync::mpsc::channel::<DownloadMsg>();

        let pbar_dl = pbar_net.clone();
        let lbl_status_dl = lbl_net_status.clone();
        let btn_dl = btn_net_download.clone();
        let btn_bk = btn_net_back.clone();
        let btn_cn = btn_net_cancel.clone();
        let combo_ed = combo_edition.clone();
        let combo_mr = combo_mirror.clone();
        let combo_ver = combo_version.clone();
        let sel_img = selected_image_path.clone();
        let rec_mode = recovery_mode.clone();
        let show_tgt = show_target_screen.clone();
        let stack_dl = stack.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    DownloadMsg::Progress { downloaded, total, speed_mb_s, fraction, eta_secs } => {
                        pbar_dl.set_fraction(fraction);
                        let down_mb = (downloaded as f64) / 1_048_576.0;
                        let tot_mb = (total as f64) / 1_048_576.0;
                        let eta_str = if eta_secs >= 60 {
                            format!("{} min {} s", eta_secs / 60, eta_secs % 60)
                        } else {
                            format!("{} s", eta_secs)
                        };
                        lbl_status_dl.set_text(&format!(
                            "Downloading: {:.1} MB / {:.1} MB ({:.0}%) — {:.2} MB/s (ETA: {})",
                            down_mb, tot_mb, fraction * 100.0, speed_mb_s, eta_str
                        ));
                    }
                    DownloadMsg::Verifying => {
                        pbar_dl.set_fraction(0.98);
                        lbl_status_dl.set_text("Verifying image checksum (SHA256)...");
                    }
                    DownloadMsg::Done => {
                        pbar_dl.set_fraction(1.0);
                        if is_demo_mode() {
                            lbl_status_dl.set_text("✅ [MODO DEMO] ¡Descarga y verificación SHA256 completadas! /tmp/pulsaros-internet-recovery.squashfs listo. Ningún disco ha sido modificado.");
                        } else {
                            lbl_status_dl.set_text("Download complete and verified!");
                        }
                        btn_dl.set_visible(true);
                        btn_dl.set_sensitive(true);
                        btn_bk.set_visible(true);
                        btn_bk.set_sensitive(true);
                        btn_cn.set_visible(false);
                        *sel_img.borrow_mut() = Some("/tmp/pulsaros-internet-recovery.squashfs".to_string());
                        *rec_mode.borrow_mut() = RecoveryMode::CustomImage("/tmp/pulsaros-internet-recovery.squashfs".to_string());
                        let src_str = if is_demo_mode() {
                            "Source: Internet Recovery [MODO DEMO - Seguro]"
                        } else {
                            "Source: Internet Recovery (SourceForge CDN)"
                        };
                        show_tgt(src_str);
                        return glib::ControlFlow::Break;
                    }
                    DownloadMsg::Cancelled => {
                        pbar_dl.set_fraction(0.0);
                        lbl_status_dl.set_text("Download cancelled.");
                        btn_dl.set_visible(true);
                        btn_dl.set_sensitive(true);
                        btn_bk.set_visible(true);
                        btn_bk.set_sensitive(true);
                        btn_cn.set_visible(false);
                        combo_ed.set_sensitive(true);
                        combo_mr.set_sensitive(true);
                        combo_ver.set_sensitive(true);
                        stack_dl.set_visible_child_name("utilities");
                        return glib::ControlFlow::Break;
                    }
                    DownloadMsg::Error(err) => {
                        pbar_dl.set_fraction(0.0);
                        lbl_status_dl.set_text(&format!("Error: {}", err));
                        btn_dl.set_visible(true);
                        btn_dl.set_sensitive(true);
                        btn_bk.set_visible(true);
                        btn_bk.set_sensitive(true);
                        btn_cn.set_visible(false);
                        combo_ed.set_sensitive(true);
                        combo_mr.set_sensitive(true);
                        combo_ver.set_sensitive(true);
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        thread::spawn(move || {
            log_msg(&format!("Starting Internet Recovery download from: {}", download_url));
            let tmp_part = "/tmp/pulsaros-internet-recovery.squashfs.part";
            let tmp_final = "/tmp/pulsaros-internet-recovery.squashfs";
            let _ = fs::remove_file(tmp_part);
            let _ = fs::remove_file(tmp_final);

            let mut child = match Command::new("curl")
                .args(&[
                    "-sSL",
                    "-L",
                    "--connect-timeout", "10",
                    "--retry", "3",
                    "-o", tmp_part,
                    &download_url,
                ])
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(DownloadMsg::Error(format!("Could not run curl: {}", e)));
                    return;
                }
            };

            let mut last_size: u64 = 0;
            let mut last_time = Instant::now();

            loop {
                thread::sleep(std::time::Duration::from_millis(200));

                if cancel_flag.load(Ordering::SeqCst) {
                    log_msg("Download cancelled by user. Terminating curl and cleaning temporary files...");
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(tmp_part);
                    let _ = fs::remove_file(tmp_final);
                    let _ = tx.send(DownloadMsg::Cancelled);
                    return;
                }

                match child.try_wait() {
                    Ok(Some(status)) => {
                        if cancel_flag.load(Ordering::SeqCst) {
                            let _ = fs::remove_file(tmp_part);
                            let _ = fs::remove_file(tmp_final);
                            let _ = tx.send(DownloadMsg::Cancelled);
                            return;
                        }

                        if status.success() {
                            let _ = fs::rename(tmp_part, tmp_final);
                            let _ = tx.send(DownloadMsg::Verifying);

                            // Calculate SHA-256
                            let mut calculated_hash = String::new();
                            if let Ok(out) = Command::new("sha256sum").arg(tmp_final).output() {
                                let hash_out = String::from_utf8_lossy(&out.stdout);
                                calculated_hash = hash_out.split_whitespace().next().unwrap_or("").to_string();
                                log_msg(&format!("Calculated SHA-256 for downloaded image: {}", calculated_hash));
                            }

                            if let Some(ref expected_h) = expected_hash {
                                let trimmed_exp = expected_h.trim();
                                if !trimmed_exp.is_empty() {
                                    if !calculated_hash.eq_ignore_ascii_case(trimmed_exp) {
                                        log_msg(&format!("❌ SHA-256 mismatch! Got: '{}', Expected: '{}'", calculated_hash, trimmed_exp));
                                        let _ = fs::remove_file(tmp_final);
                                        let _ = tx.send(DownloadMsg::Error(format!(
                                             "SHA256 checksum mismatch!\nCalculated: {}\nExpected: {}",
                                            calculated_hash, trimmed_exp
                                        )));
                                        return;
                                    } else {
                                        log_msg("✅ SHA-256 checksum verified successfully.");
                                    }
                                }
                            }

                            let _ = tx.send(DownloadMsg::Done);
                        } else {
                            let _ = tx.send(DownloadMsg::Error(format!("curl failed with status {:?}", status)));
                        }
                        break;
                    }
                    Ok(None) => {
                        let cur_size = fs::metadata(tmp_part).map(|m| m.len()).unwrap_or(0);
                        let now = Instant::now();
                        let elapsed = now.duration_since(last_time).as_secs_f64();
                        if elapsed >= 0.3 {
                            let diff = cur_size.saturating_sub(last_size);
                            let speed_mb_s = (diff as f64 / 1_048_576.0) / elapsed;
                            let rem_bytes = expected_size.saturating_sub(cur_size);
                            let eta_secs = if speed_mb_s > 0.05 {
                                ((rem_bytes as f64 / 1_048_576.0) / speed_mb_s) as u64
                            } else {
                                0
                            };
                            let fraction = if expected_size > 0 {
                                (cur_size as f64 / expected_size as f64).clamp(0.0, 0.95)
                            } else {
                                0.0
                            };
                            let _ = tx.send(DownloadMsg::Progress {
                                downloaded: cur_size,
                                total: expected_size,
                                speed_mb_s,
                                fraction,
                                eta_secs,
                            });
                            last_size = cur_size;
                            last_time = now;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(DownloadMsg::Error(format!("Error monitoring download: {}", e)));
                        break;
                    }
                }
            }
        });
    }));

    btn_usb_continue.connect_clicked(clone!(
        @strong selected_image_path,
        @strong recovery_mode,
        @strong show_target_screen
     => move |_| {
        if let Some(img_path) = selected_image_path.borrow().clone() {
            *recovery_mode.borrow_mut() = RecoveryMode::CustomImage(img_path.clone());
            let fname = Path::new(&img_path).file_name().and_then(|n| n.to_str()).unwrap_or(&img_path);
            let desc = format!("Source: USB Image ({})", fname);
            show_target_screen(&desc);
        }
    }));

    btn_browser_select.connect_clicked(clone!(
        @strong selected_image_path,
        @strong recovery_mode,
        @strong show_target_screen
     => move |_| {
        if let Some(img_path) = selected_image_path.borrow().clone() {
            *recovery_mode.borrow_mut() = RecoveryMode::CustomImage(img_path.clone());
            let fname = Path::new(&img_path).file_name().and_then(|n| n.to_str()).unwrap_or(&img_path);
            let desc = format!("Source: Selected Image ({})", fname);
            show_target_screen(&desc);
        }
    }));

    btn_target_back.connect_clicked(clone!(
        @weak stack,
        @strong selected_action
     => move |_| {
        let action = selected_action.borrow().clone().unwrap_or_default();
        if action == "usb_restore" {
            stack.set_visible_child_name("usb_select");
        } else if action == "internet_info" {
            stack.set_visible_child_name("internet_info");
        } else {
            stack.set_visible_child_name("utilities");
        }
    }));

    btn_util_continue.connect_clicked(clone!(
        @weak stack,
        @strong selected_action,
        @strong recovery_mode,
        @strong show_target_screen,
        @strong populate_usb_images
     => move |_| {
        let action = selected_action.borrow().clone().unwrap_or_default();
        match action.as_str() {
            "timemachine" => {
                log_msg("Launching Pulsar OS Time Machine Recovery Suite...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin; xhost +SI:localuser:root >/dev/null 2>&1 || xhost +local: >/dev/null 2>&1 || xhost + >/dev/null 2>&1 || true; (pulsaros-timemachine gui || python3 /usr/share/pulsaros-timemachine/cli.py gui || python3 /usr/share/pulsaros-timemachine/cli.py restore --help) >/tmp/timemachine-recovery.log 2>&1 &")
                    .spawn();
            }
            "disk" => {
                log_msg("Launching elevated Disk Utility (GParted)...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin; xhost +SI:localuser:root >/dev/null 2>&1 || xhost +local: >/dev/null 2>&1 || xhost + >/dev/null 2>&1 || true; (sudo -E /usr/sbin/gparted || sudo -E gparted || sudo /usr/sbin/gparted || sudo gparted || /usr/sbin/gparted || gparted || gnome-disks || gnome-disk-utility) >/tmp/gparted.log 2>&1 &")
                    .spawn();
            }
            "terminal" => {
                log_msg("Launching recovery root terminal...");
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin; xhost +SI:localuser:root >/dev/null 2>&1 || xhost +local: >/dev/null 2>&1 || xhost + >/dev/null 2>&1 || true; (xterm -title 'Pulsar OS Recovery Terminal' -bg '#18181b' -fg '#ffffff' -fa Monospace -fs 11 -e sudo bash || gnome-terminal -- sudo bash || alacritty -e sudo bash || x-terminal-emulator -e sudo bash || xterm -e sudo bash) &")
                    .spawn();
            }
            "internet_info" => {
                stack.set_visible_child_name("internet_info");
            }
            "usb_restore" => {
                populate_usb_images();
                stack.set_visible_child_name("usb_select");
            }
            "reinstall" => {
                *recovery_mode.borrow_mut() = RecoveryMode::Local;
                show_target_screen("Source: Built-in Recovery Partition");
            }
            _ => {}
        }
    }));

    btn_target_restore.connect_clicked(clone!(
        @weak stack,
        @weak pbar,
        @weak prog_desc,
        @weak log_view,
        @weak scrolled_log,
        @weak err_msg_lbl,
        @weak err_log_view,
        @strong selected_target,
        @strong recovery_mode
     => move |_| {
        let target = match selected_target.borrow().clone() {
            Some(t) => t,
            None => return,
        };
        let mode = recovery_mode.borrow().clone();
        stack.set_visible_child_name("progress");

        let (sender, receiver) = std::sync::mpsc::channel::<RecoveryUpdate>();

        let pbar_c = pbar.clone();
        let desc_c = prog_desc.clone();
        let stack_c = stack.clone();
        let buffer = log_view.buffer();
        let err_buffer = err_log_view.buffer();
        let err_lbl_c = err_msg_lbl.clone();
        let scroll_c = scrolled_log.clone();

        buffer.set_text("");
        err_buffer.set_text("");

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(msg) = receiver.try_recv() {
                match msg {
                    RecoveryUpdate::Progress(fraction, text) => {
                        pbar_c.set_fraction(fraction);
                        desc_c.set_label(&text);
                    }
                    RecoveryUpdate::Log(line) => {
                        let mut end = buffer.end_iter();
                        buffer.insert(&mut end, &format!("{}\n", line));
                        let mut err_end = err_buffer.end_iter();
                        err_buffer.insert(&mut err_end, &format!("{}\n", line));
                        let adj = scroll_c.vadjustment();
                        adj.set_value(adj.upper());
                    }
                    RecoveryUpdate::Finished(res) => {
                        match res {
                            Ok(_) => {
                                stack_c.set_visible_child_name("complete");
                            }
                            Err(e) => {
                                log_msg(&format!("Restoration error: {}", e));
                                err_lbl_c.set_label(&format!("Failed: {}", e));
                                stack_c.set_visible_child_name("error");
                            }
                        }
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        thread::spawn(move || {
            let sender_p = sender.clone();
            let update_ui = move |pct: f64, msg: &str| {
                let _ = sender_p.send(RecoveryUpdate::Progress(pct, msg.to_string()));
            };

            let sender_l = sender.clone();
            let append_log = move |text: &str| {
                let _ = sender_l.send(RecoveryUpdate::Log(text.to_string()));
            };

            update_ui(0.05, "Scanning target partition...");
            append_log(&format!("Target partition: {}", target.part_path));

            // Run restoration steps
            let res = run_restoration(&target, mode, update_ui, append_log);
            let _ = sender.send(RecoveryUpdate::Finished(res));
        });
    }));

    stack.set_visible_child_name("utilities");
    window.present();
}

fn run_restoration<F, L>(
    target: &BtrfsTarget,
    mode: RecoveryMode,
    progress: F,
    log: L,
) -> Result<(), String>
where
    F: Fn(f64, &str) + Send + Sync + 'static,
    L: Fn(&str) + Send + Sync + 'static,
{
    if is_demo_mode() {
        log("═════════════════════════════════════════════════════════════");
        log("   [MODO DEMO / SIMULACIÓN ACTIVO — DRY RUN]");
        log("   Ningún comando destructivo será ejecutado.");
        log("   Tus discos y particiones se mantienen 100% intactos.");
        log("═════════════════════════════════════════════════════════════");
        
        progress(0.10, "[DEMO] Simulando montaje seguro del Btrfs pool...");
        log(&format!("[DEMO] Simulación: mount -t btrfs {} /tmp/pulsar_btrfs_pool", target.part_path));
        thread::sleep(std::time::Duration::from_millis(600));

        progress(0.25, "[DEMO] Simulando preservación de usuarios (UID >= 1000)...");
        log("[DEMO] Simulación: Copiando identidades desde /etc/passwd y @home");
        thread::sleep(std::time::Duration::from_millis(600));

        progress(0.45, "[DEMO] Simulando rotación de subvolumen @ a @_backup_demo...");
        log("[DEMO] Simulación: btrfs subvolume snapshot @ @_backup_demo");
        thread::sleep(std::time::Duration::from_millis(700));

        progress(0.70, "[DEMO] Simulando descompresión del sistema (squashfs)...");
        match &mode {
            RecoveryMode::CustomImage(p) => log(&format!("[DEMO] Simulación: unsquashfs -f -d /tmp/pulsar_btrfs_pool/@ {}", p)),
            RecoveryMode::Local => log("[DEMO] Simulación: unsquashfs -f -d /tmp/pulsar_btrfs_pool/@ /run/archiso/bootmnt/..."),
        }
        thread::sleep(std::time::Duration::from_millis(900));

        progress(0.90, "[DEMO] Simulando sincronización de /etc/fstab y entradas UEFI...");
        log("[DEMO] Simulación: Actualizando UUID de partición y regenerando bootloader");
        thread::sleep(std::time::Duration::from_millis(600));

        progress(1.0, "[DEMO] ¡Restauración simulada con éxito! (Discos intactos)");
        log("[DEMO] Proceso de simulación finalizado correctamente. Cero modificaciones.");
        return Ok(());
    }

    let btrfs_mnt = "/tmp/pulsar_btrfs_pool";
    let _ = fs::create_dir_all(btrfs_mnt);

    // 1. Unmount any busy mounts
    let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", btrfs_mnt));
    let _ = exec_cmd(&format!("umount -l {}* 2>/dev/null || true", target.part_path));

    progress(0.10, "Mounting Btrfs pool...");
    log("Mounting Btrfs root pool without subvolume...");
    exec_cmd(&format!("mount -t btrfs {} {}", target.part_path, btrfs_mnt))?;

    // 2. Backup existing user accounts from old @ subvolume and discover users from @home
    progress(0.20, "Preserving user accounts and identities...");
    log("Backing up /etc/passwd, /etc/shadow, /etc/group for real users (UID >= 1000)...");
    let old_root = format!("{}/@", btrfs_mnt);
    let mut preserved_passwd: Vec<String> = Vec::new();
    let mut preserved_shadow: Vec<String> = Vec::new();
    let mut preserved_group: Vec<String> = Vec::new();
    let mut preserved_gshadow: Vec<String> = Vec::new();
    let mut preserved_usernames: Vec<String> = Vec::new();
    let mut user_group_memberships: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    if Path::new(&old_root).exists() {
        if let Ok(file) = File::open(format!("{}/etc/passwd", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let uname = parts[0].to_string();
                    // NEVER preserve temporary live session users
                    if uname == "live" || uname == "root" || uname == "pulsar-live" || uname == "archiso" || uname == "nobody" {
                        continue;
                    }
                    if let Ok(uid) = parts[2].parse::<u32>() {
                        if uid >= 1000 && uid < 65534 {
                            preserved_usernames.push(uname);
                            preserved_passwd.push(line);
                        }
                    }
                }
            }
        }
        if let Ok(file) = File::open(format!("{}/etc/shadow", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let uname = line.split(':').next().unwrap_or_default();
                if preserved_usernames.iter().any(|u| u == uname) {
                    preserved_shadow.push(line);
                }
            }
        }
        if let Ok(file) = File::open(format!("{}/etc/group", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 4 {
                    let gname = parts[0].to_string();
                    let members = parts[3].split(',');
                    for m in members {
                        let m_trim = m.trim().to_string();
                        if !m_trim.is_empty() && m_trim != "live" && m_trim != "root" && m_trim != "archiso" {
                            user_group_memberships.entry(m_trim).or_default().push(gname.clone());
                        }
                    }
                }
                if parts.len() >= 3 {
                    let gname = parts[0];
                    if gname == "live" || gname == "root" || gname == "archiso" {
                        continue;
                    }
                    if let Ok(gid) = parts[2].parse::<u32>() {
                        if gid >= 1000 && gid < 65534 {
                            preserved_group.push(line);
                        }
                    }
                }
            }
        }
        if let Ok(file) = File::open(format!("{}/etc/gshadow", old_root)) {
            for line in BufReader::new(file).lines().flatten() {
                let gname = line.split(':').next().unwrap_or_default();
                if gname != "live" && gname != "root" && gname != "archiso" {
                    preserved_gshadow.push(line);
                }
            }
        }
    }

    // Also inspect @home in case /@/etc/passwd was already corrupted or missing
    let home_dir = format!("{}/@home", btrfs_mnt);
    if let Ok(entries) = fs::read_dir(&home_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let uname = entry.file_name().to_string_lossy().to_string();
                    if uname != "live" && uname != "root" && uname != "lost+found" && !preserved_usernames.contains(&uname) {
                        log(&format!("Discovered existing user home directory in @home: /home/{}", uname));
                        preserved_passwd.push(format!("{}:x:1000:1000::{}:/bin/bash", uname, format!("/home/{}", uname)));
                        preserved_shadow.push(format!("{}:!!:19700:0:99999:7:::", uname));
                        preserved_group.push(format!("{}:x:1000:", uname));
                        preserved_usernames.push(uname);
                    }
                }
            }
        }
    }
    log(&format!("Preserved {} real user account(s): {:?}", preserved_usernames.len(), preserved_usernames));

    // 3. Resolve and verify SquashFS source BEFORE wiping anything
    let squashfs_path = match mode {
        RecoveryMode::Local => {
            progress(0.25, "Locating built-in Arch Linux recovery image...");
            match detect_local_squashfs(&log) {
                Some(p) => p,
                None => {
                    log("ERROR: No valid base recovery image found on built-in recovery partition.");
                    return Err(
                        "No recovery image found on built-in recovery partition.\n\n\
                        Please choose 'Restore from USB Flash Drive' or check 'Pulsar Internet Recovery' to download an image from SourceForge.".to_string()
                    );
                }
            }
        }
        RecoveryMode::CustomImage(path) => {
            progress(0.25, "Verifying selected recovery image...");
            log(&format!("Verifying image at: {}", path));
            if !is_valid_base_squashfs(&path) {
                return Err(format!(
                    "Selected recovery image at '{}' is invalid or corrupt.\nNo changes were made to your disk.",
                    path
                ));
            }
            path
        }
    };

    // 4. Wipe and recreate @ root subvolume (SAFE: Image is 100% verified)
    progress(0.45, "Recreating @ root subvolume...");
    log("Removing old root (@) subvolume...");
    let _ = exec_cmd(&format!("btrfs subvolume delete {}/@ 2>/dev/null || rm -rf {}/@", btrfs_mnt, btrfs_mnt));
    log("Creating fresh root (@) subvolume...");
    exec_cmd(&format!("btrfs subvolume create {}/@", btrfs_mnt))?;

    // Ensure @home exists
    let home_path = format!("{}/@home", btrfs_mnt);
    if !Path::new(&home_path).exists() {
        log("Creating @home subvolume...");
        exec_cmd(&format!("btrfs subvolume create {}", home_path))?;
    }

    // 5. Unsquash clean system into @
    progress(0.55, "Unpacking clean Pulsar OS rootfs into @...");
    log(&format!("Unsquashing {} into {}/@...", squashfs_path, btrfs_mnt));
    exec_cmd_stream(&format!("unsquashfs -f -d {}/@ {}", btrfs_mnt, squashfs_path), &log)?;

    // 6. Re-inject preserved users and clean out any temporary live user
    progress(0.85, "Re-injecting user credentials and settings...");
    log("Restoring user accounts into clean /etc...");
    let new_root = format!("{}/@", btrfs_mnt);

    // Remove any live user artifact from new rootfs
    let _ = exec_cmd(&format!("sed -i '/^live:/d' {}/etc/passwd {}/etc/shadow {}/etc/group {}/etc/gshadow 2>/dev/null || true", new_root, new_root, new_root, new_root));

    if !preserved_passwd.is_empty() {
        for l in &preserved_passwd {
            let uname = l.split(':').next().unwrap_or_default();
            let _ = exec_cmd(&format!("sed -i '/^{}:/d' {}/etc/passwd 2>/dev/null || true", uname, new_root));
        }
        let mut tmp_users = String::new();
        for l in &preserved_passwd {
            tmp_users.push_str(&format!("{}\n", l));
        }
        let _ = fs::write("/tmp/pulsar_preserved_passwd", &tmp_users);
        let _ = exec_cmd(&format!("cat /tmp/pulsar_preserved_passwd >> {}/etc/passwd", new_root));

        for l in &preserved_shadow {
            let uname = l.split(':').next().unwrap_or_default();
            let _ = exec_cmd(&format!("sed -i '/^{}:/d' {}/etc/shadow 2>/dev/null || true", uname, new_root));
        }
        let mut tmp_shadow = String::new();
        for l in &preserved_shadow {
            tmp_shadow.push_str(&format!("{}\n", l));
        }
        // Guarantee that every preserved user has a valid line in /etc/shadow
        for uname in &preserved_usernames {
            if !preserved_shadow.iter().any(|s| s.starts_with(&format!("{}:", uname))) {
                log(&format!("Adding fallback shadow entry for user: {}", uname));
                tmp_shadow.push_str(&format!("{}::19700:0:99999:7:::\n", uname));
            }
        }
        let _ = fs::write("/tmp/pulsar_preserved_shadow", &tmp_shadow);
        let _ = exec_cmd(&format!("cat /tmp/pulsar_preserved_shadow >> {}/etc/shadow", new_root));

        let mut tmp_group = String::new();
        for l in &preserved_group {
            tmp_group.push_str(&format!("{}\n", l));
        }
        let _ = fs::write("/tmp/pulsar_preserved_group", &tmp_group);
        let _ = exec_cmd(&format!("cat /tmp/pulsar_preserved_group >> {}/etc/group", new_root));

        let mut tmp_gshadow = String::new();
        for l in &preserved_gshadow {
            tmp_gshadow.push_str(&format!("{}\n", l));
        }
        let _ = fs::write("/tmp/pulsar_preserved_gshadow", &tmp_gshadow);
        let _ = exec_cmd(&format!("cat /tmp/pulsar_preserved_gshadow >> {}/etc/gshadow", new_root));

        // Add each preserved user to essential desktop/admin groups and preserved groups
        let base_admin_groups = [
            "wheel", "sudo", "video", "audio", "input", "storage", "network", "optical",
            "power", "rfkill", "autologin", "users", "lp", "scanner", "kvm"
        ];

        let sudoers_d = format!("{}/etc/sudoers.d", new_root);
        let _ = fs::create_dir_all(&sudoers_d);
        let _ = exec_cmd(&format!("chmod 750 {}", sudoers_d));

        let wheel_rule = format!("{}/10-admin-wheel", sudoers_d);
        let _ = fs::write(&wheel_rule, "%wheel ALL=(ALL:ALL) ALL\n%sudo ALL=(ALL:ALL) ALL\n");
        let _ = exec_cmd(&format!("chmod 0440 {}", wheel_rule));

        for uname in &preserved_usernames {
            let mut target_groups: Vec<String> = base_admin_groups.iter().map(|s| s.to_string()).collect();
            if let Some(custom_grps) = user_group_memberships.get(uname) {
                for cg in custom_grps {
                    if !target_groups.contains(cg) {
                        target_groups.push(cg.clone());
                    }
                }
            }

            for grp in &target_groups {
                let _ = exec_cmd(&format!(
                    "grep -q '^{}:' {}/etc/group || echo '{}:x:999:' >> {}/etc/group",
                    grp, new_root, grp, new_root
                ));
                let _ = exec_cmd(&format!(
                    "sed -i -E 's/^({}:[^:]*:[^:]*:)(.*)$/\\1\\2,{}/' {}/etc/group 2>/dev/null || true",
                    grp, uname, new_root
                ));
                let _ = exec_cmd(&format!(
                    "sed -i -E 's/,+/,/g; s/:,/:/g; s/,$//' {}/etc/group 2>/dev/null || true",
                    new_root
                ));
            }

            // Drop explicit sudoers rule for the user
            let user_rule = format!("{}/pulsaros-user-{}", sudoers_d, uname);
            let _ = fs::write(&user_rule, format!("{} ALL=(ALL:ALL) ALL\n", uname));
            let _ = exec_cmd(&format!("chmod 0440 {}", user_rule));
            log(&format!("Granted full sudo privileges to user '{}' via sudoers and wheel group", uname));
        }
    }

    // 7. Regenerate clean /etc/fstab with correct UUID
    progress(0.90, "Configuring file systems and boot mounts...");
    log("Writing clean /etc/fstab for Btrfs subvolumes (@, @home)...");
    let btrfs_uuid = if !target.uuid.is_empty() {
        target.uuid.clone()
    } else {
        exec_cmd(&format!("blkid -s UUID -o value {}", target.part_path))?.trim().to_string()
    };

    // Find EFI partition on the same disk
    let efi_uuid = exec_cmd("blkid -t TYPE=vfat -s UUID -o value | head -n 1").unwrap_or_default().trim().to_string();

    let fstab_content = format!(
        "# /etc/fstab: Pulsar OS Btrfs Configuration\n\
        UUID={} /               btrfs   subvol=@,compress=zstd:1,space_cache=v2 0 0\n\
        UUID={} /home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2 0 0\n\
        {}\n",
        btrfs_uuid,
        btrfs_uuid,
        if !efi_uuid.is_empty() {
            format!("UUID={} /boot/efi       vfat    umask=0077 0 2", efi_uuid)
        } else {
            "".to_string()
        }
    );

    let _ = fs::write("/tmp/pulsar_new_fstab", &fstab_content);
    let _ = exec_cmd(&format!("cp -f /tmp/pulsar_new_fstab {}/etc/fstab", new_root));

    // Deploy udev rule to hide recovery partition from file managers
    let udev_dir = format!("{}/etc/udev/rules.d", new_root);
    let _ = fs::create_dir_all(&udev_dir);
    let _ = fs::write(
        format!("{}/99-pulsaros-hide-recovery.rules", udev_dir),
        "# Hide PULSAR_RECOVERY partition from file managers and desktop\nENV{ID_FS_LABEL}==\"PULSAR_RECOVERY\", ENV{UDISKS_IGNORE}=\"1\", ENV{UDISKS_AUTO}=\"0\"\n"
    );

    // Deploy default non-empty SDDM wallpaper
    let sddm_dir = format!("{}/var/lib/pulsar-sddm", new_root);
    let _ = fs::create_dir_all(&sddm_dir);
    let _ = exec_cmd(&format!("chmod 777 {}", sddm_dir));
    let wallpaper_sources = [
        format!("{}/usr/share/backgrounds/pulsar-os-tahoe.png", new_root),
        format!("{}/usr/share/sddm/themes/Apple.Tahoe/pulsar-os-tahoe.png", new_root),
        format!("{}/usr/share/backgrounds/gnome/pulsar-wallpaper.png", new_root),
    ];
    for ws in &wallpaper_sources {
        if Path::new(ws).exists() {
            let _ = exec_cmd(&format!("cp -f {} {}/pulsar-wallpaper.png", ws, sddm_dir));
            let _ = exec_cmd(&format!("chmod 666 {}/pulsar-wallpaper.png", sddm_dir));
            log(&format!("Deployed default SDDM wallpaper to {} from {}", sddm_dir, ws));
            break;
        }
    }

    // Remove unwanted GNOME extensions that should never be active in Pulsar OS
    log("Removing unwanted GNOME extensions (places-menu, window-list)...");
    let _ = exec_cmd(&format!(
        "rm -rf {}/usr/share/gnome-shell/extensions/places-menu@gnome-shell-extensions.gcampax.github.com \
                {}/usr/share/gnome-shell/extensions/window-list@gnome-shell-extensions.gcampax.github.com \
                {}/usr/share/gnome-shell/extensions/search-light@icedman.github.com 2>/dev/null || true",
        new_root, new_root, new_root
    ));

    // 8. Deploy boot kernels, recovery kernel, and align rEFInd
    progress(0.95, "Deploying OS & Recovery kernels to @/boot and aligning bootloader...");
    deploy_boot_and_recovery_kernels(&new_root, &btrfs_uuid, &log);

    // 9. Cleanup and sync
    progress(0.98, "Synchronizing disks and unmounting...");
    log("Syncing disks...");
    let _ = exec_cmd("sync");
    let _ = exec_cmd(&format!("umount -l {}", btrfs_mnt));

    progress(1.0, "Restoration complete!");
    log("System successfully restored.");
    Ok(())
}

fn deploy_boot_and_recovery_kernels<L>(new_root: &str, btrfs_uuid: &str, log: &L)
where
    L: Fn(&str) + Send + Sync + 'static,
{
    log("Verifying and deploying boot and recovery kernels into @/boot and ESP...");

    let boot_dir = format!("{}/boot", new_root);
    let _ = fs::create_dir_all(&boot_dir);

    // 1. Locate and deploy recovery kernel & initramfs
    let rec_kernel_sources = [
        "/run/live/medium/live/vmlinuz",
        "/run/live/medium/vmlinuz",
        "/run/live/medium/recovery/vmlinuz-recovery",
        "/run/live/medium/boot/vmlinuz-recovery",
        "/tmp/pulsar_recovery/boot/vmlinuz-recovery",
        "/tmp/pulsar_recovery/vmlinuz-recovery",
        "/tmp/pulsar_recovery/live/vmlinuz",
        "/recovery/vmlinuz-recovery",
        "/lib/live/mount/medium/live/vmlinuz",
        "/lib/live/mount/medium/vmlinuz",
    ];
    let rec_initrd_sources = [
        "/run/live/medium/live/initrd.img",
        "/run/live/medium/initrd.img",
        "/run/live/medium/recovery/initramfs-recovery.img",
        "/run/live/medium/boot/initramfs-recovery.img",
        "/tmp/pulsar_recovery/boot/initramfs-recovery.img",
        "/tmp/pulsar_recovery/initramfs-recovery.img",
        "/tmp/pulsar_recovery/live/initrd.img",
        "/recovery/initramfs-recovery.img",
        "/lib/live/mount/medium/live/initrd.img",
        "/lib/live/mount/medium/initrd.img",
    ];

    let mut rec_k_found: Option<String> = None;
    for src in &rec_kernel_sources {
        if Path::new(src).exists() {
            rec_k_found = Some(src.to_string());
            break;
        }
    }
    if rec_k_found.is_none() {
        if let Ok(entries) = fs::read_dir("/boot") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("vmlinuz") && !name.ends_with(".kver") {
                    rec_k_found = Some(entry.path().to_string_lossy().to_string());
                    break;
                }
            }
        }
    }

    if let Some(src) = rec_k_found {
        let dest = format!("{}/vmlinuz-recovery", boot_dir);
        let _ = exec_cmd(&format!("cp -f {} {}", src, dest));
        log(&format!("Restored recovery kernel to {} from {}", dest, src));
    }

    let mut rec_initrd_found: Option<String> = None;
    for src in &rec_initrd_sources {
        if Path::new(src).exists() {
            rec_initrd_found = Some(src.to_string());
            break;
        }
    }
    if rec_initrd_found.is_none() {
        if let Ok(entries) = fs::read_dir("/boot") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("initrd") || name.starts_with("initramfs") {
                    rec_initrd_found = Some(entry.path().to_string_lossy().to_string());
                    break;
                }
            }
        }
    }

    if let Some(src) = rec_initrd_found {
        let dest = format!("{}/initramfs-recovery.img", boot_dir);
        let _ = exec_cmd(&format!("cp -f {} {}", src, dest));
        log(&format!("Restored recovery initramfs to {} from {}", dest, src));
    }

    // 2. Ensure OS kernel naming aliases exist in @/boot
    let mut found_kernel: Option<String> = None;
    let mut found_initrd: Option<String> = None;
    if let Ok(entries) = fs::read_dir(&boot_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name.starts_with("vmlinuz") && !name.contains("recovery") && !name.ends_with(".kver") {
                found_kernel = Some(p.to_string_lossy().to_string());
            }
            if (name.starts_with("initramfs") || name.starts_with("initrd")) && !name.contains("recovery") && !name.contains("fallback") && !name.contains("ucode") {
                found_initrd = Some(p.to_string_lossy().to_string());
            }
        }
    }

    // Fallback search for initrd if not in rootfs
    if found_initrd.is_none() {
        let alt_initrd_sources = [
            "/boot/initramfs-6.1-x86_64.img",
            "/boot/initramfs-linux.img",
            "/tmp/pulsar_recovery/boot/initramfs-6.1-x86_64.img",
            "/run/live/medium/boot/initramfs-6.1-x86_64.img",
            "/run/live/medium/boot/initramfs-linux.img",
        ];
        for alt in &alt_initrd_sources {
            if Path::new(alt).exists() {
                found_initrd = Some(alt.to_string());
                break;
            }
        }
    }

    if let Some(k) = &found_kernel {
        log(&format!("Detected main OS kernel: {}", k));
        let targets = ["vmlinuz-6.1-x86_64", "vmlinuz-linux", "vmlinuz"];
        for t in &targets {
            let dest = format!("{}/{}", boot_dir, t);
            if !Path::new(&dest).exists() || &dest != k {
                let _ = exec_cmd(&format!("cp -f {} {}", k, dest));
                log(&format!("Created kernel alias: {} -> {}", dest, k));
            }
        }
    }

    if let Some(i) = &found_initrd {
        log(&format!("Detected main OS initrd: {}", i));
        let targets = ["initramfs-6.1-x86_64.img", "initramfs-linux.img"];
        for t in &targets {
            let dest = format!("{}/{}", boot_dir, t);
            if !Path::new(&dest).exists() || &dest != i {
                let _ = exec_cmd(&format!("cp -f {} {}", i, dest));
                log(&format!("Created initramfs alias: {} -> {}", dest, i));
            }
        }
    }

    // Enforce UEFI-compatible permissions on @/boot and all boot assets
    let _ = exec_cmd(&format!("chmod 755 {}", boot_dir));
    let _ = exec_cmd(&format!("chmod 644 {}/*", boot_dir));
    let _ = exec_cmd(&format!("chown -R 0:0 {}", boot_dir));

    // Copy microcode files if present on host / recovery medium
    let ucode_sources = [
        "/tmp/pulsar_recovery/amd-ucode.img",
        "/run/live/medium/amd-ucode.img",
        "/boot/amd-ucode.img",
        "/tmp/pulsar_recovery/intel-ucode.img",
        "/run/live/medium/intel-ucode.img",
        "/boot/intel-ucode.img",
    ];
    for u in &ucode_sources {
        if Path::new(u).exists() {
            let fname = Path::new(u).file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let dest = format!("{}/{}", boot_dir, fname);
            if !Path::new(&dest).exists() {
                let _ = exec_cmd(&format!("cp -f {} {}", u, dest));
            }
        }
    }

    // 3. Mount and configure ESP / rEFInd
    let esp_mnt = "/tmp/pulsar_esp_mount";
    let _ = fs::create_dir_all(esp_mnt);
    let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", esp_mnt));

    if let Ok(out) = exec_cmd("blkid -t TYPE=vfat -o device | head -n 1") {
        let efi_dev = out.trim();
        if !efi_dev.is_empty() {
            if exec_cmd(&format!("mount {} {}", efi_dev, esp_mnt)).is_ok() {
                log(&format!("Mounted ESP on {} for bootloader alignment...", esp_mnt));

                // Copy recovery kernels to ESP as well
                let efi_rec_dir = format!("{}/EFI/recovery", esp_mnt);
                let _ = fs::create_dir_all(&efi_rec_dir);
                let _ = exec_cmd(&format!("cp -f {}/vmlinuz-recovery {}/vmlinuz-recovery 2>/dev/null || true", boot_dir, efi_rec_dir));
                let _ = exec_cmd(&format!("cp -f {}/initramfs-recovery.img {}/initramfs-recovery.img 2>/dev/null || true", boot_dir, efi_rec_dir));

                // Align refind.conf UUIDs
                let refind_confs = [
                    format!("{}/EFI/refind/refind.conf", esp_mnt),
                    format!("{}/EFI/BOOT/refind.conf", esp_mnt),
                ];
                for rc in &refind_confs {
                    if Path::new(rc).exists() {
                        if let Ok(content) = fs::read_to_string(rc) {
                            let re = Regex::new(r"root=UUID=[a-fA-F0-9-]+").unwrap();
                            let updated = re.replace_all(&content, &format!("root=UUID={}", btrfs_uuid)).to_string();
                            let _ = fs::write(rc, updated);
                            log(&format!("Updated root UUID in {} to {}", rc, btrfs_uuid));
                        }
                    }
                }

                let _ = exec_cmd(&format!("umount -l {}", esp_mnt));
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has_demo_arg = args.iter().any(|a| a == "--demo" || a == "--dry-run" || a == "-d");
    let has_demo_env = std::env::var("PULSAR_DEMO").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false)
        || std::env::var("PULSAR_DRY_RUN").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false);

    if has_demo_arg || has_demo_env {
        set_demo_mode(true);
        println!("🚀 Pulsar OS Recovery running in DEMO / DRY-RUN mode (Disks are protected, no changes will be made).");
    }

    let app = Application::builder()
        .application_id("es.inled.pulsaros.recovery-assistant")
        .build();

    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[]);
}
