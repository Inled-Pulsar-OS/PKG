//! Detección del sistema, ejecución privilegiada y escaneo de medios.
use crate::models::{AppType, BtrfsTarget, DiscoveredImage, EncryptedTarget, ExternalDrive, InstalledApp};
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

pub(crate) fn is_root() -> bool {
    Command::new("id").arg("-u").output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

pub(crate) fn exec_cmd_stream<L>(cmd: &str, log: &L) -> Result<(), String>
where
    L: Fn(&str) + Send + Sync + 'static,
{
    log_msg(&format!("Running (as root): {}", cmd));
    log(&format!("$ {}", cmd));

    let mut child = if is_root() {
        Command::new("sh")
            .args(&["-c", cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn '{}': {}", cmd, e))?
    } else {
        Command::new("sudo")
            .args(&["-n", "sh", "-c", cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn '{}': {}", cmd, e))?
    };

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
    let out = if is_root() {
        Command::new("sh")
            .args(&["-c", cmd])
            .output()
            .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?
    } else {
        Command::new("sudo")
            .args(&["-n", "sh", "-c", cmd])
            .output()
            .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?
    };

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
    let cmd_res = if is_root() {
        Command::new("lsblk").args(&["-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME,TYPE"]).output()
    } else {
        Command::new("sudo").args(&["-n", "lsblk", "-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME,TYPE"]).output()
            .or_else(|_| Command::new("lsblk").args(&["-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME,TYPE"]).output())
    };

    if let Ok(out) = cmd_res {
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
    let cmd_res = if is_root() {
        Command::new("lsblk").args(&["-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME,TYPE"]).output()
    } else {
        Command::new("sudo").args(&["-n", "lsblk", "-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME,TYPE"]).output()
            .or_else(|_| Command::new("lsblk").args(&["-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,PKNAME,TYPE"]).output())
    };

    if let Ok(out) = cmd_res {
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
                let clean_name = name.replace('/', "_");
                let mut mapper_name = format!("pulsar_crypt_{}", clean_name);
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

    // Check if mapper_name is already active
    if Path::new(&format!("/dev/mapper/{}", mapper_name)).exists() {
        let check_cmd = if is_root() {
            Command::new("cryptsetup").args(&["status", mapper_name]).output()
        } else {
            Command::new("sudo").args(&["-n", "cryptsetup", "status", mapper_name]).output()
        };
        if let Ok(st) = check_cmd {
            if st.status.success() {
                log_msg(&format!("Volume /dev/mapper/{} is already unlocked and active.", mapper_name));
                return Ok(format!("/dev/mapper/{}", mapper_name));
            }
        }
    }

    let mut cmd = if is_root() {
        let mut c = Command::new("cryptsetup");
        c.args(&["open", part_path, mapper_name, "-"]);
        c
    } else {
        let mut c = Command::new("sudo");
        c.args(&["-n", "cryptsetup", "open", part_path, mapper_name, "-"]);
        c
    };

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to launch cryptsetup: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(passphrase.as_bytes());
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().map_err(|e| format!("Failed waiting on cryptsetup: {}", e))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let err_msg = if stderr.contains("No key available") || stderr.contains("passphrase") || stderr.contains("key") {
            "Incorrect encryption passphrase. Please try again.".to_string()
        } else {
            format!("Could not unlock {}: {}", part_path, stderr.trim())
        };
        log_msg(&format!("ERROR unlocking LUKS: {}", err_msg));
        return Err(err_msg);
    }

    if is_root() {
        let _ = Command::new("udevadm").args(&["settle"]).status();
    } else {
        let _ = Command::new("sudo").args(&["-n", "udevadm", "settle"]).status();
    }

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
    let p = Path::new(path);
    if !p.exists() {
        return false;
    }

    // Explicitly REJECT the dedicated recovery OS environment (never restore recovery OS over @ root)
    let p_lower = path.to_lowercase();
    if p_lower.contains("recovery/filesystem.squashfs")
        || p_lower.contains("recovery/live/filesystem.squashfs")
        || p_lower.contains("recovery-filesystem.squashfs")
        || p_lower.contains("recovery.squashfs")
        || p_lower.contains("pulsar_recovery/filesystem.squashfs")
        || p_lower.contains("pulsar_recovery/live/filesystem.squashfs")
    {
        return false;
    }

    // Must not be a kernel or initramfs file
    let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if fname.starts_with("initramfs") || fname.starts_with("vmlinuz") {
        return false;
    }
    if let Ok(meta) = fs::metadata(path) {
        // Base desktop rootfs squashfs must be at least 400 MB
        if meta.len() < 400 * 1024 * 1024 {
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
    if depth > 4 {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dname = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                if dname == "proc" || dname == "sys" || dname == "dev" || dname == "run" {
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
    let media_dirs = ["/media", "/run/media", "/mnt", "/recovery"];
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
    log("Scanning storage devices for clean Pulsar OS base recovery image...");

    let rec_mnt = "/tmp/pulsar_recovery";
    let _ = fs::create_dir_all(rec_mnt);

    // 1. Mount recovery partition by label
    let _ = Command::new("sudo").args(&["-n", "mount", "/dev/disk/by-label/PULSAR_RECOVERY", rec_mnt]).output();
    let _ = Command::new("sudo").args(&["-n", "mount", "-L", "PULSAR_RECOVERY", rec_mnt]).output();

    let base_image_names = [
        "images/pulsaros-base.squashfs",
        "pulsaros-base.squashfs",
        "recovery/images/pulsaros-base.squashfs",
        "live/x86_64/airootfs.sfs",
        "arch/x86_64/airootfs.sfs",
        "images/x86_64/airootfs.sfs",
        "images/airootfs.sfs",
        "airootfs.sfs",
        "live/filesystem.squashfs",
        "live/rootfs.squashfs",
        "rootfs.squashfs",
    ];

    let search_roots = [
        "/tmp/pulsar_recovery",
        "/tmp/pulsar_recovery/images",
        "/recovery/images",
        "/mnt/recovery/images",
        "/run/live/medium/images",
        "/run/live/medium",
        "/lib/live/mount/medium/images",
        "/lib/live/mount/medium",
        "/run/archiso/bootmnt/images",
        "/run/archiso/bootmnt/arch/x86_64",
        "/run/archiso/bootmnt",
        "/var/lib/pulsar/images",
    ];

    for root in &search_roots {
        for img in &base_image_names {
            let full_p = format!("{}/{}", root, img);
            if Path::new(&full_p).exists() && is_valid_base_squashfs(&full_p) {
                log(&format!("Verified clean base system image at: {}", full_p));
                return Some(full_p);
            }
        }
    }

    // 2. Scan block devices and mapper targets
    let mut dev_list = Vec::new();
    if let Ok(out) = Command::new("sudo").args(&["-n", "blkid", "-o", "device"]).output() {
        let devs = String::from_utf8_lossy(&out.stdout);
        for dev in devs.lines() {
            let dev = dev.trim();
            if !dev.is_empty() && !dev.contains("loop") && !dev.contains("zram") {
                dev_list.push(dev.to_string());
            }
        }
    }

    // Also include /dev/mapper devices (e.g. unlocked LUKS containers)
    if let Ok(entries) = fs::read_dir("/dev/mapper") {
        for entry in entries.flatten() {
            let path = entry.path();
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if fname != "control" && !dev_list.iter().any(|d| d == &path.to_string_lossy()) {
                dev_list.push(path.to_string_lossy().to_string());
            }
        }
    }

    for dev in &dev_list {
        let temp_mnt = format!("/tmp/mnt_{}", dev.replace('/', "_"));
        let _ = fs::create_dir_all(&temp_mnt);
        
        // Try regular ro mount or btrfs top-level mount
        let mounted = Command::new("sudo")
            .args(&["-n", "mount", "-o", "ro", dev, &temp_mnt])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            || Command::new("sudo")
            .args(&["-n", "mount", "-o", "ro,subvolid=5", dev, &temp_mnt])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if mounted {
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

    log("[Notice] No local base image found on built-in recovery partition.");
    None
}

pub(crate) fn set_keyboard_layout(layout: &str) -> Result<(), String> {
    log_msg(&format!("Setting keyboard layout to: {}", layout));
    let _ = Command::new("setxkbmap").arg(layout).status();
    let _ = Command::new("sudo").args(&["-n", "localectl", "set-x11-keymap", layout]).status();
    let _ = Command::new("sudo").args(&["-n", "loadkeys", layout]).status();
    Ok(())
}

pub(crate) fn mount_chroot_environment(part_path: &str) -> Result<String, String> {
    let mnt_root = "/mnt/pulsar";
    let _ = fs::create_dir_all(mnt_root);

    log_msg(&format!("Mounting Pulsar OS (@) from {} at {}...", part_path, mnt_root));
    let _ = exec_cmd(&format!("mountpoint -q {} || mount -t btrfs -o subvol=@,noatime,compress=zstd:1 {} {}", mnt_root, part_path, mnt_root));

    let home_path = format!("{}/home", mnt_root);
    let _ = fs::create_dir_all(&home_path);
    let _ = exec_cmd(&format!("mountpoint -q {} || mount -t btrfs -o subvol=@home,noatime,compress=zstd:1 {} {}", home_path, part_path, home_path));

    // Mount EFI partition if present
    if let Ok(out) = exec_cmd("blkid -t TYPE=vfat -o device | head -n 1") {
        let efi_dev = out.trim();
        if !efi_dev.is_empty() {
            let efi_path = format!("{}/boot/efi", mnt_root);
            let _ = fs::create_dir_all(&efi_path);
            let _ = exec_cmd(&format!("mountpoint -q {} || mount {} {}", efi_path, efi_dev, efi_path));
        }
    }

    // Mount virtual filesystems
    for d in &["dev", "proc", "sys", "run"] {
        let target_dir = format!("{}/{}", mnt_root, d);
        let _ = fs::create_dir_all(&target_dir);
        let _ = exec_cmd(&format!("mountpoint -q {} || mount --bind /{} {}", target_dir, d, target_dir));
    }

    if Path::new("/sys/firmware/efi/efivars").exists() {
        let efivars_target = format!("{}/sys/firmware/efi/efivars", mnt_root);
        let _ = fs::create_dir_all(&efivars_target);
        let _ = exec_cmd(&format!("mountpoint -q {} || mount --bind /sys/firmware/efi/efivars {}", efivars_target, efivars_target));
    }

    // Copy DNS config so network works inside chroot
    let _ = exec_cmd(&format!("cp -L /etc/resolv.conf {}/etc/resolv.conf 2>/dev/null || true", mnt_root));

    Ok(mnt_root.to_string())
}

pub(crate) fn launch_terminal_with_command(title: &str, command: &str) -> Result<(), String> {
    let wrapped = format!(
        "trap 'kill 0 2>/dev/null; exit 0' HUP INT TERM; {}; exit 0",
        command
    );

    let candidates: [(&str, Vec<&str>); 6] = [
        ("gnome-terminal", vec!["--title", title, "--", "bash", "-c", &wrapped]),
        ("ptyxis", vec!["--title", title, "--", "bash", "-c", &wrapped]),
        ("xterm", vec!["-title", title, "-bg", "#18181b", "-fg", "#ffffff", "-fa", "Monospace", "-fs", "11", "-geometry", "110x32", "-e", "bash", "-c", &wrapped]),
        ("alacritty", vec!["-t", title, "-e", "bash", "-c", &wrapped]),
        ("konsole", vec!["--title", title, "-e", "bash", "-c", &wrapped]),
        ("x-terminal-emulator", vec!["-e", "bash", "-c", &wrapped]),
    ];

    for (term, args) in &candidates {
        if Command::new("which").arg(term).status().map(|s| s.success()).unwrap_or(false) {
            match Command::new(term).args(args).spawn() {
                Ok(_) => {
                    log_msg(&format!("Terminal launched successfully using {}", term));
                    return Ok(());
                }
                Err(e) => {
                    log_msg(&format!("Failed launching {}: {}", term, e));
                }
            }
        }
    }

    // Direct fallback if 'which' failed
    for term in &["xterm", "gnome-terminal", "ptyxis", "x-terminal-emulator"] {
        if let Ok(_) = Command::new(term).arg("-e").arg(&format!("bash -c '{}'", wrapped)).spawn() {
            log_msg(&format!("Terminal launched using direct fallback {}", term));
            return Ok(());
        }
    }

    Err("Could not launch any terminal emulator (tried gnome-terminal, ptyxis, xterm, alacritty, konsole)".to_string())
}

pub(crate) fn launch_chroot_terminal(part_path: &str) -> Result<(), String> {
    if crate::demo::is_demo_mode() {
        log_msg("[DEMO] Opening simulation terminal for chroot...");
        return launch_terminal_with_command(
            "Pulsar OS Chroot (DEMO MODE)",
            "echo '=== Pulsar OS Chroot (DEMO MODE) ==='; echo 'Root filesystem mounted at /mnt/pulsar (simulated).'; echo; bash",
        );
    }

    let mnt_root = mount_chroot_environment(part_path)?;
    log_msg(&format!("Spawning root terminal inside chroot at {}...", mnt_root));

    let sudo_cmd = if is_root() { "chroot /mnt/pulsar /bin/bash" } else { "sudo chroot /mnt/pulsar /bin/bash" };
    launch_terminal_with_command("Pulsar OS Chroot (Root Console)", sudo_cmd)
}

pub(crate) fn open_system_logs(part_path: Option<&str>) {
    log_msg("Opening Pulsar OS logs directory...");
    let _ = fs::create_dir_all("/var/log/pulsaros");

    if let Some(part) = part_path {
        let log_mnt = "/tmp/pulsar_log_root";
        let _ = fs::create_dir_all(log_mnt);
        log_msg(&format!("Mounting installed system from {} to inspect logs at {}...", part, log_mnt));
        let _ = exec_cmd(&format!("mountpoint -q {} || mount -t btrfs -o ro,subvol=@ {} {} 2>/dev/null || true", log_mnt, part, log_mnt));

        let installed_pulsar_logs = format!("{}/var/log/pulsaros", log_mnt);
        if Path::new(&installed_pulsar_logs).exists() {
            let target_link = "/var/log/pulsaros/installed_system_logs";
            let _ = exec_cmd(&format!("rm -f {} 2>/dev/null; ln -sf {} {} 2>/dev/null || true", target_link, installed_pulsar_logs, target_link));
        }
        let installed_var_log = format!("{}/var/log", log_mnt);
        if Path::new(&installed_var_log).exists() {
            let target_link = "/var/log/pulsaros/installed_var_log";
            let _ = exec_cmd(&format!("rm -f {} 2>/dev/null; ln -sf {} {} 2>/dev/null || true", target_link, installed_var_log, target_link));
        }
    }

    let fm_cmd = "nautilus /var/log/pulsaros || thunar /var/log/pulsaros || pcmanfm /var/log/pulsaros || xdg-open /var/log/pulsaros || gnome-text-editor /var/log/pulsaros/installer.log";
    let spawned = Command::new("sh")
        .arg("-c")
        .arg(format!("export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin; xhost +SI:localuser:root >/dev/null 2>&1 || xhost +local: >/dev/null 2>&1 || true; ({})", fm_cmd))
        .spawn();

    if spawned.is_err() {
        let _ = launch_terminal_with_command("Pulsar OS Logs", "echo === Pulsar OS Logs ===; ls -la /var/log/pulsaros; echo; bash");
    }
}

pub(crate) fn scan_external_drives() -> Vec<ExternalDrive> {
    let mut drives = Vec::new();
    let cmd_res = if is_root() {
        Command::new("lsblk").args(&["-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,TRAN,RM,HOTPLUG,MOUNTPOINTS,TYPE"]).output()
    } else {
        Command::new("sudo").args(&["-n", "lsblk", "-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,TRAN,RM,HOTPLUG,MOUNTPOINTS,TYPE"]).output()
            .or_else(|_| Command::new("lsblk").args(&["-P", "-o", "NAME,LABEL,UUID,FSTYPE,SIZE,TRAN,RM,HOTPLUG,MOUNTPOINTS,TYPE"]).output())
    };

    if let Ok(out) = cmd_res {
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
            let size = get_val("SIZE");

            if name.is_empty() || name.starts_with("loop") || name.starts_with("zram") || name.starts_with("sr") {
                continue;
            }
            if label == "PULSAR_ISO" || label == "PULSAR_RECOVERY" || label == "archiso" || label == "LIVE" {
                continue;
            }
            if dev_type == "disk" && fstype.is_empty() {
                continue;
            }

            let is_removable = tran == "usb" || rm == "1" || hotplug == "1";
            if !is_removable && !name.starts_with("sd") {
                continue;
            }

            if label == "PULSAR_OS" {
                continue;
            }

            let part_dev = format!("/dev/{}", name);
            let mount_point = if !mountpoints.is_empty() {
                mountpoints.split_whitespace().next().unwrap_or("").to_string()
            } else if !fstype.is_empty() {
                let mnt = format!("/media/{}", name);
                let _ = fs::create_dir_all(&mnt);
                let _ = Command::new("sudo").args(&["-n", "mount", &part_dev, &mnt]).output();
                mnt
            } else {
                continue;
            };

            drives.push(ExternalDrive {
                name,
                label: if !label.is_empty() { label } else { format!("Drive ({})", part_dev) },
                mount_point,
                fstype,
                size,
                is_removable,
            });
        }
    }
    drives
}

pub(crate) fn scan_installed_applications(old_root: &str) -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let old_root_p = Path::new(old_root);
    if !old_root_p.exists() {
        return apps;
    }

    // 1. Arch Linux (pacman local database)
    let pacman_local = old_root_p.join("var/lib/pacman/local");
    if pacman_local.exists() {
        if let Ok(entries) = fs::read_dir(&pacman_local) {
            for entry in entries.flatten() {
                let path = entry.path();
                let desc_file = path.join("desc");
                if desc_file.exists() {
                    if let Ok(content) = fs::read_to_string(&desc_file) {
                        let mut name = String::new();
                        let mut version = String::new();
                        let mut desc = String::new();
                        let mut reason = 1;

                        let lines: Vec<&str> = content.lines().collect();
                        let mut i = 0;
                        while i < lines.len() {
                            match lines[i] {
                                "%NAME%" if i + 1 < lines.len() => name = lines[i + 1].to_string(),
                                "%VERSION%" if i + 1 < lines.len() => version = lines[i + 1].to_string(),
                                "%DESC%" if i + 1 < lines.len() => desc = lines[i + 1].to_string(),
                                "%REASON%" if i + 1 < lines.len() => {
                                    if let Ok(r) = lines[i + 1].parse::<u32>() {
                                        reason = r;
                                    }
                                }
                                _ => {}
                            }
                            i += 1;
                        }

                        if reason == 0 && !name.is_empty() {
                            let core_system = [
                                "base", "linux", "linux-firmware", "glibc", "systemd", "coreutils", "bash",
                                "pacman", "btrfs-progs", "e2fsprogs", "dosfstools", "shadow", "util-linux",
                                "grub", "refind", "sudo", "pulsaros-recovery"
                            ];
                            if core_system.contains(&name.as_str()) {
                                continue;
                            }

                            let is_driver = name.starts_with("nvidia")
                                || name.starts_with("broadcom")
                                || name.starts_with("rtl8")
                                || name.starts_with("xf86-video")
                                || name.starts_with("vulkan-")
                                || name.ends_with("-dkms")
                                || name.ends_with("-firmware");

                            let app_type = if is_driver {
                                AppType::Driver
                            } else {
                                AppType::Application
                            };

                            apps.push(InstalledApp {
                                id: name.clone(),
                                name,
                                version,
                                description: desc,
                                app_type,
                                selected: true,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Debian / Ubuntu (dpkg status)
    let dpkg_status = old_root_p.join("var/lib/dpkg/status");
    if dpkg_status.exists() && apps.is_empty() {
        if let Ok(content) = fs::read_to_string(&dpkg_status) {
            let mut cur_pkg = String::new();
            let mut cur_ver = String::new();
            let mut cur_desc = String::new();
            let mut is_installed = false;

            for line in content.lines() {
                if line.is_empty() {
                    if is_installed && !cur_pkg.is_empty() {
                        let is_driver = cur_pkg.starts_with("nvidia")
                            || cur_pkg.starts_with("broadcom")
                            || cur_pkg.ends_with("-dkms")
                            || cur_pkg.starts_with("firmware-");

                        let has_desktop = old_root_p.join(format!("usr/share/applications/{}.desktop", cur_pkg)).exists();
                        if has_desktop || is_driver {
                            apps.push(InstalledApp {
                                id: cur_pkg.clone(),
                                name: cur_pkg.clone(),
                                version: cur_ver.clone(),
                                description: cur_desc.clone(),
                                app_type: if is_driver { AppType::Driver } else { AppType::Application },
                                selected: true,
                            });
                        }
                    }
                    cur_pkg.clear();
                    cur_ver.clear();
                    cur_desc.clear();
                    is_installed = false;
                } else if line.starts_with("Package: ") {
                    cur_pkg = line["Package: ".len()..].trim().to_string();
                } else if line.starts_with("Version: ") {
                    cur_ver = line["Version: ".len()..].trim().to_string();
                } else if line.starts_with("Description: ") {
                    cur_desc = line["Description: ".len()..].trim().to_string();
                } else if line.starts_with("Status: ") {
                    is_installed = line.contains("installed");
                }
            }
        }
    }

    // 3. Flatpaks
    let flatpak_dirs = [
        old_root_p.join("var/lib/flatpak/app"),
        Path::new("/tmp/pulsar_btrfs_pool/@home").to_path_buf(),
    ];
    for fdir in &flatpak_dirs {
        if fdir.exists() {
            if let Ok(entries) = fs::read_dir(fdir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
                    if name.contains('.') && !apps.iter().any(|a| a.id == name) {
                        apps.push(InstalledApp {
                            id: name.clone(),
                            name: name.clone(),
                            version: "Flatpak".to_string(),
                            description: "Sandboxed Flatpak application".to_string(),
                            app_type: AppType::Flatpak,
                            selected: true,
                        });
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| {
        let type_ord_a = match a.app_type { AppType::Driver => 0, AppType::Application => 1, AppType::Flatpak => 2 };
        let type_ord_b = match b.app_type { AppType::Driver => 0, AppType::Application => 1, AppType::Flatpak => 2 };
        type_ord_a.cmp(&type_ord_b).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    apps
}

pub(crate) fn copy_folder_with_progress<F, L>(
    src: &str,
    dst: &str,
    progress: F,
    log: L,
) -> Result<(), String>
where
    F: Fn(f64, &str) + Send + Sync + 'static,
    L: Fn(&str) + Send + Sync + 'static,
{
    log(&format!("Starting copy: {} -> {}", src, dst));
    progress(0.05, "Preparing copy transfer...");

    let src_p = Path::new(src);
    let folder_name = src_p.file_name().and_then(|n| n.to_str()).unwrap_or("backup");
    let target_dest = format!("{}/{}", dst, folder_name);

    let _ = fs::create_dir_all(&target_dest);

    let has_rsync = Command::new("which").arg("rsync").status().map(|s| s.success()).unwrap_or(false);
    let cmd = if has_rsync {
        format!("rsync -aHAX --info=progress2 \"{}/\" \"{}/\"", src, target_dest)
    } else {
        format!("cp -a \"{}/.\" \"{}/\"", src, target_dest)
    };

    progress(0.20, &format!("Copying {} to destination drive...", folder_name));
    exec_cmd_stream(&cmd, &log)?;

    progress(0.95, "Syncing files to storage...");
    let _ = exec_cmd("sync");

    progress(1.0, "Copy completed successfully!");
    log(&format!("Successfully copied {} to {}", src, target_dest));
    Ok(())
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

    #[test]
    fn test_set_keyboard_layout() {
        assert!(set_keyboard_layout("es").is_ok());
        assert!(set_keyboard_layout("us").is_ok());
    }

    #[test]
    fn test_installed_app_model() {
        let app = InstalledApp {
            id: "firefox".to_string(),
            name: "Firefox Web Browser".to_string(),
            version: "128.0".to_string(),
            description: "Fast, Private & Safe Web Browser".to_string(),
            app_type: AppType::Application,
            selected: true,
        };
        assert_eq!(app.id, "firefox");
        assert_eq!(app.app_type, AppType::Application);
        assert!(app.selected);
    }

    #[test]
    fn test_scan_installed_applications_mock() {
        let temp_dir = std::env::temp_dir().join("pulsar_test_mock_root");
        let pacman_dir = temp_dir.join("var/lib/pacman/local/firefox-128.0-1");
        let _ = fs::create_dir_all(&pacman_dir);

        let desc_content = "%NAME%\nfirefox\n\n%VERSION%\n128.0-1\n\n%DESC%\nStandalone web browser\n\n%REASON%\n0\n";
        let _ = fs::write(pacman_dir.join("desc"), desc_content);

        let apps = scan_installed_applications(&temp_dir.to_string_lossy());
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].id, "firefox");
        assert_eq!(apps[0].app_type, AppType::Application);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
