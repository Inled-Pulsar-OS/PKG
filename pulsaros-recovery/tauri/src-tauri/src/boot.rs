use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

fn exec_cmd(cmd: &str) -> Result<String, String> {
    let out = Command::new("sudo")
        .args(["-n", "sh", "-c", cmd])
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn deploy_boot_and_recovery_kernels(
    new_root: &str,
    btrfs_uuid: &str,
    log_fn: &dyn Fn(String),
) {
    log_fn("Verifying and deploying boot and recovery kernels...".into());

    let boot_dir = format!("{}/boot", new_root);
    let _ = fs::create_dir_all(&boot_dir);

    deploy_recovery_kernel(&boot_dir, log_fn);
    deploy_recovery_initrd(&boot_dir, log_fn);
    create_kernel_aliases(&boot_dir, log_fn);
    enforce_boot_permissions(&boot_dir);
    deploy_microcode(&boot_dir);
    align_refind(new_root, btrfs_uuid, &boot_dir, log_fn);
}

fn deploy_recovery_kernel(boot_dir: &str, log_fn: &dyn Fn(String)) {
    let sources = [
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

    let mut found: Option<String> = None;
    for src in &sources {
        if Path::new(src).exists() {
            found = Some(src.to_string());
            break;
        }
    }
    if found.is_none() {
        if let Ok(entries) = fs::read_dir("/boot") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("vmlinuz") && !name.ends_with(".kver") {
                    found = Some(entry.path().to_string_lossy().to_string());
                    break;
                }
            }
        }
    }

    if let Some(src) = found {
        let dest = format!("{}/vmlinuz-recovery", boot_dir);
        let _ = exec_cmd(&format!("cp -f {} {}", src, dest));
        log_fn(format!("Restored recovery kernel to {} from {}", dest, src));
    }
}

fn deploy_recovery_initrd(boot_dir: &str, log_fn: &dyn Fn(String)) {
    let sources = [
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

    let mut found: Option<String> = None;
    for src in &sources {
        if Path::new(src).exists() {
            found = Some(src.to_string());
            break;
        }
    }
    if found.is_none() {
        if let Ok(entries) = fs::read_dir("/boot") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("initrd") || name.starts_with("initramfs") {
                    found = Some(entry.path().to_string_lossy().to_string());
                    break;
                }
            }
        }
    }

    if let Some(src) = found {
        let dest = format!("{}/initramfs-recovery.img", boot_dir);
        let _ = exec_cmd(&format!("cp -f {} {}", src, dest));
        log_fn(format!(
            "Restored recovery initramfs to {} from {}",
            dest, src
        ));
    }
}

fn create_kernel_aliases(boot_dir: &str, log_fn: &dyn Fn(String)) {
    let mut found_kernel: Option<String> = None;
    let mut found_initrd: Option<String> = None;

    if let Ok(entries) = fs::read_dir(boot_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name.starts_with("vmlinuz")
                && !name.contains("recovery")
                && !name.ends_with(".kver")
            {
                found_kernel = Some(p.to_string_lossy().to_string());
            }
            if (name.starts_with("initramfs") || name.starts_with("initrd"))
                && !name.contains("recovery")
                && !name.contains("fallback")
                && !name.contains("ucode")
            {
                found_initrd = Some(p.to_string_lossy().to_string());
            }
        }
    }

    if found_initrd.is_none() {
        let alt_sources = [
            "/boot/initramfs-6.1-x86_64.img",
            "/boot/initramfs-linux.img",
            "/tmp/pulsar_recovery/boot/initramfs-6.1-x86_64.img",
            "/run/live/medium/boot/initramfs-6.1-x86_64.img",
            "/run/live/medium/boot/initramfs-linux.img",
        ];
        for alt in &alt_sources {
            if Path::new(alt).exists() {
                found_initrd = Some(alt.to_string());
                break;
            }
        }
    }

    if let Some(k) = &found_kernel {
        log_fn(format!("Detected main OS kernel: {}", k));
        for t in &["vmlinuz-6.1-x86_64", "vmlinuz-linux", "vmlinuz"] {
            let dest = format!("{}/{}", boot_dir, t);
            if !Path::new(&dest).exists() || &dest != k {
                let _ = exec_cmd(&format!("cp -f {} {}", k, dest));
                log_fn(format!("Created kernel alias: {} -> {}", dest, k));
            }
        }
    }

    if let Some(i) = &found_initrd {
        log_fn(format!("Detected main OS initrd: {}", i));
        for t in &["initramfs-6.1-x86_64.img", "initramfs-linux.img"] {
            let dest = format!("{}/{}", boot_dir, t);
            if !Path::new(&dest).exists() || &dest != i {
                let _ = exec_cmd(&format!("cp -f {} {}", i, dest));
                log_fn(format!("Created initrd alias: {} -> {}", dest, i));
            }
        }
    }
}

fn enforce_boot_permissions(boot_dir: &str) {
    let _ = exec_cmd(&format!("chmod 755 {}", boot_dir));
    let _ = exec_cmd(&format!("chmod 644 {}/*", boot_dir));
    let _ = exec_cmd(&format!("chown -R 0:0 {}", boot_dir));
}

fn deploy_microcode(boot_dir: &str) {
    let sources = [
        "/tmp/pulsar_recovery/amd-ucode.img",
        "/run/live/medium/amd-ucode.img",
        "/boot/amd-ucode.img",
        "/tmp/pulsar_recovery/intel-ucode.img",
        "/run/live/medium/intel-ucode.img",
        "/boot/intel-ucode.img",
    ];
    for u in &sources {
        if Path::new(u).exists() {
            let fname = Path::new(u)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let dest = format!("{}/{}", boot_dir, fname);
            if !Path::new(&dest).exists() {
                let _ = exec_cmd(&format!("cp -f {} {}", u, dest));
            }
        }
    }
}

fn align_refind(
    _new_root: &str,
    btrfs_uuid: &str,
    boot_dir: &str,
    log_fn: &dyn Fn(String),
) {
    let esp_mnt = "/tmp/pulsar_esp_mount";
    let _ = fs::create_dir_all(esp_mnt);
    let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", esp_mnt));

    let efi_dev = match exec_cmd("blkid -t TYPE=vfat -o device | head -n 1") {
        Ok(out) => out.trim().to_string(),
        Err(_) => return,
    };

    if efi_dev.is_empty() {
        return;
    }

    if exec_cmd(&format!("mount {} {}", efi_dev, esp_mnt)).is_err() {
        return;
    }

    log_fn("Mounted ESP for bootloader alignment...".into());

    let efi_rec_dir = format!("{}/EFI/recovery", esp_mnt);
    let _ = fs::create_dir_all(&efi_rec_dir);
    let _ = exec_cmd(&format!(
        "cp -f {}/vmlinuz-recovery {}/vmlinuz-recovery 2>/dev/null || true",
        boot_dir, efi_rec_dir
    ));
    let _ = exec_cmd(&format!(
        "cp -f {}/initramfs-recovery.img {}/initramfs-recovery.img 2>/dev/null || true",
        boot_dir, efi_rec_dir
    ));

    let refind_confs = [
        format!("{}/EFI/refind/refind.conf", esp_mnt),
        format!("{}/EFI/BOOT/refind.conf", esp_mnt),
    ];
    let re = Regex::new(r"root=UUID=[a-fA-F0-9-]+").unwrap();
    for rc in &refind_confs {
        if Path::new(rc).exists() {
            if let Ok(content) = fs::read_to_string(rc) {
                let updated = re
                    .replace_all(&content, &format!("root=UUID={}", btrfs_uuid))
                    .to_string();
                let _ = fs::write(rc, updated);
                log_fn(format!("Updated root UUID in {} to {}", rc, btrfs_uuid));
            }
        }
    }

    let _ = exec_cmd(&format!("umount -l {}", esp_mnt));
}
