use regex::Regex;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BtrfsTarget {
    pub disk_path: String,
    pub part_path: String,
    pub label: String,
    pub uuid: String,
    pub size: String,
}

pub fn find_btrfs_targets() -> Vec<BtrfsTarget> {
    let mut targets = Vec::new();
    if let Ok(out) = Command::new("lsblk")
        .args(["-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        let re_name = Regex::new(r#"NAME="([^"]*)""#).unwrap();
        let re_label = Regex::new(r#"LABEL="([^"]*)""#).unwrap();
        let re_uuid = Regex::new(r#"UUID="([^"]*)""#).unwrap();
        let re_fstype = Regex::new(r#"FSTYPE="([^"]*)""#).unwrap();
        let re_size = Regex::new(r#"SIZE="([^"]*)""#).unwrap();
        let re_pkname = Regex::new(r#"PKNAME="([^"]*)""#).unwrap();

        for line in text.lines() {
            let fstype = re_fstype
                .captures(line)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let label_raw = re_label
                .captures(line)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            if fstype == "btrfs"
                || label_raw.to_uppercase().contains("PULSAR")
                || label_raw.contains("PulsarOS")
            {
                let name = re_name
                    .captures(line)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let uuid = re_uuid
                    .captures(line)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let size = re_size
                    .captures(line)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let pkname = re_pkname
                    .captures(line)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                let part_path = format!("/dev/{}", name);
                let disk_path = if !pkname.is_empty() {
                    format!("/dev/{}", pkname)
                } else {
                    part_path.clone()
                };

                targets.push(BtrfsTarget {
                    disk_path,
                    part_path,
                    label: if label_raw.is_empty() {
                        "PULSAR_OS".to_string()
                    } else {
                        label_raw
                    },
                    uuid,
                    size,
                });
            }
        }
    }
    targets
}

fn sudo_cmd(args: &[&str]) -> Result<String, String> {
    let out = Command::new("sudo")
        .args(["-n"])
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute: {}", e))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn mount_recovery_partition() -> bool {
    let _ = sudo_cmd(&["umount", "-l", "/tmp/pulsar_recovery"]);
    let _ = sudo_cmd(&["mkdir", "-p", "/tmp/pulsar_recovery"]);

    // Try by label first
    if sudo_cmd(&[
        "mount",
        "-o", "ro",
        "/dev/disk/by-label/PULSAR_RECOVERY",
        "/tmp/pulsar_recovery",
    ])
    .is_ok()
    {
        return true;
    }

    // Find PULSAR_RECOVERY partition via blkid
    if let Ok(out) = Command::new("blkid")
        .args(["-t", "LABEL=PULSAR_RECOVERY", "-o", "device"])
        .output()
    {
        let dev = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !dev.is_empty() {
            return sudo_cmd(&["mount", "-o", "ro", &dev, "/tmp/pulsar_recovery"]).is_ok();
        }
    }

    false
}

fn is_valid_squashfs(path: &str) -> bool {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if metadata.len() < 1_000_000_000 {
        return false;
    }
    Command::new("unsquashfs")
        .args(["-s", path])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn detect_local_squashfs() -> Option<String> {
    let _ = mount_recovery_partition();

    let roots = [
        "/tmp/pulsar_recovery",
        "/run/live/medium",
        "/lib/live/mount/medium",
        "/run/archiso/bootmnt",
        "/run/archiso",
        "/recovery",
        "/mnt/recovery",
    ];

    let names = [
        "images/pulsaros-base.squashfs",
        "images/x86_64/airootfs.sfs",
        "images/airootfs.sfs",
        "arch/x86_64/airootfs.sfs",
        "pulsaros-base.squashfs",
        "airootfs.sfs",
        "live/filesystem.squashfs",
        "live/x86_64/airootfs.sfs",
        "live/filesystem.sfs",
    ];

    for root in &roots {
        for name in &names {
            let path = format!("{}/{}", root, name);
            if Path::new(&path).exists() && is_valid_squashfs(&path) {
                return Some(path);
            }
        }
    }

    // Scan block devices as last resort
    if let Ok(out) = Command::new("blkid")
        .args(["-o", "device"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let dev = line.trim();
            if dev.is_empty() || dev == "/dev/sr0" {
                continue;
            }
            let mnt = "/tmp/pulsar_blkid_scan";
            let _ = sudo_cmd(&["mkdir", "-p", mnt]);
            if sudo_cmd(&["mount", "-o", "ro", dev, mnt]).is_ok() {
                for name in &names {
                    let path = format!("{}/{}", mnt, name);
                    if Path::new(&path).exists() && is_valid_squashfs(&path) {
                        let result = Some(path);
                        let _ = sudo_cmd(&["umount", "-l", mnt]);
                        return result;
                    }
                }
                let _ = sudo_cmd(&["umount", "-l", mnt]);
            }
        }
    }

    None
}

pub fn find_efi_uuid() -> String {
    Command::new("blkid")
        .args(["-t", "TYPE=vfat", "-s", "UUID", "-o", "value"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .map(|s| s.lines().next().unwrap_or("").trim().to_string())
        })
        .unwrap_or_default()
}
