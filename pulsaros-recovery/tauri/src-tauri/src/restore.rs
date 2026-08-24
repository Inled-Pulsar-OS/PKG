use std::fs;
use std::path::Path;
use std::process::Command;

use crate::disk::{self, BtrfsTarget};
use crate::fstab;
use crate::users;

#[derive(Clone, Debug, serde::Serialize)]
pub enum RecoveryMode {
    Local,
    Internet(String),
}

fn exec_cmd(cmd: &str) -> Result<String, String> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?;

    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if !out.status.success() {
        return Err(format!(
            "Command '{}' failed ({}): {}",
            cmd,
            out.status.code().unwrap_or(-1),
            stderr
        ));
    }

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn run_restoration(
    target: &BtrfsTarget,
    mode: RecoveryMode,
    log_fn: &dyn Fn(String),
    progress_fn: &dyn Fn(f64, String),
) -> Result<(), String> {
    let btrfs_mnt = "/tmp/pulsar_btrfs_pool";
    let _ = fs::create_dir_all(btrfs_mnt);

    let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", btrfs_mnt));
    let _ = exec_cmd(&format!("umount -l {}* 2>/dev/null || true", target.part_path));

    progress_fn(0.10, "Mounting Btrfs pool...".into());
    log_fn("Mounting Btrfs root pool without subvolume...".into());
    exec_cmd(&format!(
        "mount -t btrfs {} {}",
        target.part_path, btrfs_mnt
    ))?;

    progress_fn(0.20, "Preserving user accounts...".into());
    log_fn("Backing up user accounts (UID >= 1000)...".into());
    let old_root = format!("{}/@", btrfs_mnt);
    let preserved = if Path::new(&old_root).exists() {
        let p = users::preserve_users(&old_root);
        log_fn(format!("Preserved {} user account(s).", p.passwd.len()));
        p
    } else {
        users::PreservedUsers {
            passwd: Vec::new(),
            shadow: Vec::new(),
            group: Vec::new(),
            gshadow: Vec::new(),
        }
    };

    let squashfs_path = match mode {
        RecoveryMode::Local => {
            progress_fn(0.30, "Locating local recovery image...".into());
            log_fn("Searching for local recovery squashfs...".into());
            disk::detect_local_squashfs().ok_or_else(|| -> String {
                "No local recovery SquashFS found. Use Internet Recovery.".into()
            })?
        }
        RecoveryMode::Internet(url) => {
            progress_fn(0.25, "Downloading image from GitHub Releases...".into());
            log_fn(format!("Downloading from: {}", url));
            let dl_path = "/tmp/pulsaros-remote-recovery.squashfs";
            exec_cmd(&format!(
                "curl -L -C - --retry 3 -o {} {}",
                dl_path, url
            ))?;
            dl_path.to_string()
        }
    };

    progress_fn(0.45, "Recreating root subvolume...".into());
    log_fn("Removing old @ subvolume...".into());
    let _ = exec_cmd(&format!(
        "btrfs subvolume delete {}/@ 2>/dev/null || rm -rf {}/@",
        btrfs_mnt, btrfs_mnt
    ));
    log_fn("Creating fresh @ subvolume...".into());
    exec_cmd(&format!("btrfs subvolume create {}/@", btrfs_mnt))?;

    let home_path = format!("{}/@home", btrfs_mnt);
    if !Path::new(&home_path).exists() {
        log_fn("Creating @home subvolume...".into());
        exec_cmd(&format!("btrfs subvolume create {}", home_path))?;
    }

    progress_fn(0.55, "Unpacking clean rootfs...".into());
    log_fn(format!(
        "Unsquashing {} into {}/@...",
        squashfs_path, btrfs_mnt
    ));
    exec_cmd(&format!(
        "unsquashfs -f -d {}/@ {}",
        btrfs_mnt, squashfs_path
    ))?;

    progress_fn(0.85, "Re-injecting user credentials...".into());
    log_fn("Restoring user accounts into clean /etc...".into());
    let new_root = format!("{}/@", btrfs_mnt);
    users::restore_users(&new_root, &preserved)?;

    progress_fn(0.92, "Configuring filesystem mounts...".into());
    log_fn("Writing clean /etc/fstab...".into());
    let btrfs_uuid = if !target.uuid.is_empty() {
        target.uuid.clone()
    } else {
        exec_cmd(&format!(
            "blkid -s UUID -o value {}",
            target.part_path
        ))?
        .trim()
        .to_string()
    };
    let efi_uuid = disk::find_efi_uuid();
    fstab::generate_fstab(&new_root, &btrfs_uuid, &efi_uuid)?;

    progress_fn(0.98, "Synchronizing disks...".into());
    log_fn("Syncing disks...".into());
    let _ = exec_cmd("sync");
    let _ = exec_cmd(&format!("umount -l {}", btrfs_mnt));

    progress_fn(1.0, "Restoration complete!".into());
    log_fn("System successfully restored.".into());
    Ok(())
}

pub fn launch_external_app(app: &str) -> Result<(), String> {
    let fallbacks: Vec<Vec<&str>> = match app {
        "timeshift" => vec![
            vec!["timeshift-launcher"],
            vec!["pkexec", "timeshift-gtk"],
            vec!["timeshift-gtk"],
        ],
        "gparted" => vec![
            vec!["gparted"],
            vec!["pkexec", "gparted"],
            vec!["gnome-disks"],
        ],
        _ => return Err(format!("Unknown app: {}", app)),
    };

    for cmds in &fallbacks {
        let mut c = Command::new(cmds[0]);
        if cmds.len() > 1 {
            c.args(&cmds[1..]);
        }
        if c.spawn().is_ok() {
            return Ok(());
        }
    }
    Err(format!("Failed to launch {}", app))
}

pub fn reboot() -> Result<(), String> {
    Command::new("systemctl")
        .arg("reboot")
        .spawn()
        .map_err(|e| format!("Failed to reboot: {}", e))?;
    Ok(())
}
