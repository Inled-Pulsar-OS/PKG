use regex::Regex;
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

pub fn detect_local_squashfs() -> Option<String> {
    let candidates = [
        "/recovery/images/pulsaros-base.squashfs",
        "/run/archiso/bootmnt/live/x86_64/airootfs.sfs",
        "/run/archiso/bootmnt/live/filesystem.squashfs",
        "/run/live/medium/live/filesystem.squashfs",
        "/lib/live/mount/medium/live/filesystem.squashfs",
        "/run/archiso/airootfs.sfs",
    ];
    for p in &candidates {
        if std::path::Path::new(p).exists() {
            return Some(p.to_string());
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
