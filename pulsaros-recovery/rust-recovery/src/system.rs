//! Detección del sistema, ejecución privilegiada y escaneo de medios.
use crate::models::{BtrfsTarget, DiscoveredImage, EncryptedTarget};
use regex::Regex;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
pub(crate) fn detect_system_base() -> String {
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

pub(crate) fn detect_system_bootloader() -> String {
    if Path::new("/boot/efi/EFI/refind").exists() || Path::new("/boot/refind").exists() {
        return "refind".to_string();
    }
    if Path::new("/run/media/pulsar_btrfs_pool/@/boot/refind").exists() {
        return "refind".to_string();
    }
    "grub".to_string()
}

pub(crate) fn log_msg(msg: &str) {
    let log_path = "/tmp/pulsaros-recovery.log";
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(f, "{}", msg);
    }
    println!("{}", msg);
}

pub(crate) fn exec_cmd_stream<L>(cmd: &str, log: &L) -> Result<(), String>
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

pub(crate) fn exec_cmd(cmd: &str) -> Result<String, String> {
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

pub(crate) fn find_btrfs_targets() -> Vec<BtrfsTarget> {
    let mut targets = Vec::new();
    if let Ok(out) = Command::new("sudo").args(&["-n", "lsblk", "-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME,TYPE"]).output() {
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

                let part_path = if Path::new(&format!("/dev/mapper/{}", name)).exists() {
                    format!("/dev/mapper/{}", name)
                } else if name.starts_with('/') {
                    name
                } else {
                    format!("/dev/{}", name)
                };

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

pub(crate) fn find_encrypted_targets() -> Vec<EncryptedTarget> {
    let mut targets = Vec::new();
    if let Ok(out) = Command::new("sudo").args(&["-n", "lsblk", "-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME,TYPE"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("FSTYPE=\"crypto_LUKS\"") {
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

                // Check if already mapped/unlocked in /sys/class/block/{name}/holders
                let mut is_unlocked = false;
                let mut mapper_name = "pulsar_cryptroot".to_string();
                if let Ok(holders) = fs::read_dir(format!("/sys/class/block/{}/holders", name)) {
                    for h in holders.flatten() {
                        let hname = h.file_name().to_string_lossy().to_string();
                        if !hname.is_empty() {
                            is_unlocked = true;
                            mapper_name = hname;
                            break;
                        }
                    }
                }

                targets.push(EncryptedTarget {
                    disk_path,
                    part_path,
                    label: if label.is_empty() { "Encrypted Pulsar OS".to_string() } else { label },
                    uuid,
                    size,
                    is_unlocked,
                    mapper_name,
                });
            }
        }
    }
    targets
}

pub(crate) fn unlock_luks_partition(part_path: &str, passphrase: &str, mapper_name: &str) -> Result<String, String> {
    log_msg(&format!("Attempting to unlock LUKS partition: {} -> /dev/mapper/{}", part_path, mapper_name));
    
    if crate::demo::is_demo_mode() {
        log_msg(&format!("[DEMO] Simulating LUKS unlock for: {} -> /dev/mapper/{}", part_path, mapper_name));
        if passphrase.is_empty() {
            return Err("Passphrase cannot be empty.".to_string());
        }
        return Ok(format!("/dev/mapper/{}", mapper_name));
    }

    let mut child = Command::new("sudo")
        .args(&["-n", "cryptsetup", "open", part_path, mapper_name, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch cryptsetup: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(passphrase.as_bytes());
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().map_err(|e| format!("Failed waiting on cryptsetup: {}", e))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let err_msg = if stderr.contains("No key available") || stderr.contains("passphrase") || stderr.contains("key") {
            "Incorrect encryption password. Please try again.".to_string()
        } else {
            format!("Could not unlock {}: {}", part_path, stderr.trim())
        };
        log_msg(&format!("ERROR unlocking LUKS: {}", err_msg));
        return Err(err_msg);
    }

    let _ = Command::new("sudo").args(&["-n", "udevadm", "settle"]).status();
    let mapper_dev = format!("/dev/mapper/{}", mapper_name);
    log_msg(&format!("Successfully unlocked LUKS volume: {}", mapper_dev));
    Ok(mapper_dev)
}

pub(crate) fn lock_luks_partition(mapper_name: &str) -> Result<(), String> {
    log_msg(&format!("Closing LUKS mapped volume: {}", mapper_name));
    let _ = Command::new("sudo").args(&["-n", "cryptsetup", "close", mapper_name]).status();
    Ok(())
}

pub(crate) fn is_valid_base_squashfs(path: &str) -> bool {
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

pub(crate) fn format_file_size(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1.0 {
        format!("{:.2} GB", gb)
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.1} MB", mb)
    }
}

pub(crate) fn scan_dir_for_squashfs(dir: &Path, depth: usize, dev_label: &str, out: &mut Vec<DiscoveredImage>) {
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

pub(crate) fn scan_usb_devices() -> Vec<DiscoveredImage> {
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

pub(crate) fn detect_local_squashfs<L>(log: &L) -> Option<String>
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

    log("[Notice] No local base image found on built-in recovery partition.");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypted_target_structure() {
        let enc = EncryptedTarget {
            disk_path: "/dev/nvme0n1".to_string(),
            part_path: "/dev/nvme0n1p2".to_string(),
            label: "Encrypted Pulsar OS".to_string(),
            uuid: "test-luks-uuid-1234".to_string(),
            size: "500G".to_string(),
            is_unlocked: false,
            mapper_name: "pulsar_cryptroot".to_string(),
        };

        assert_eq!(enc.part_path, "/dev/nvme0n1p2");
        assert_eq!(enc.mapper_name, "pulsar_cryptroot");
        assert!(!enc.is_unlocked);
    }

    #[test]
    fn test_demo_mode_luks_unlock() {
        crate::demo::set_demo_mode(true);

        // In demo mode, empty password should return Err and non-empty Ok
        let res_err = unlock_luks_partition("/dev/demo1", "", "pulsar_cryptroot");
        assert!(res_err.is_err());

        let res_ok = unlock_luks_partition("/dev/demo1", "ValidPassphrase123", "pulsar_cryptroot");
        assert!(res_ok.is_ok());
        assert_eq!(res_ok.unwrap(), "/dev/mapper/pulsar_cryptroot");

        crate::demo::set_demo_mode(false);
    }
}
