use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::disk::{self, BtrfsTarget};
use crate::fstab;
use crate::users;

#[derive(Clone, Debug, serde::Serialize)]
pub enum RecoveryMode {
    Local,
    Internet(String),
}

fn exec_cmd(cmd: &str) -> Result<String, String> {
    let out = Command::new("sudo")
        .args(["-n", "sh", "-c", cmd])
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

fn exec_cmd_stream(
    cmd: &str,
    log_fn: &dyn Fn(String),
) -> Result<String, String> {
    let mut child = Command::new("sudo")
        .args(["-n", "sh", "-c", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {}", cmd, e))?;

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut full_output = String::new();

    for line in reader.lines().map_while(Result::ok) {
        log_fn(line.clone());
        full_output.push_str(&line);
        full_output.push('\n');
    }

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for '{}': {}", cmd, e))?;

    if !status.success() {
        return Err(format!(
            "Command '{}' failed ({})",
            cmd,
            status.code().unwrap_or(-1)
        ));
    }

    Ok(full_output)
}

pub fn is_valid_base_squashfs(path: &str) -> bool {
    let metadata = match fs::metadata(path) {
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

pub fn run_restoration(
    target: &BtrfsTarget,
    mode: RecoveryMode,
    log_fn: &dyn Fn(String),
    progress_fn: &dyn Fn(f64, String),
) -> Result<(), String> {
    let btrfs_mnt = "/tmp/pulsar_btrfs_pool";
    let _ = fs::create_dir_all(btrfs_mnt);

    let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", btrfs_mnt));
    let _ = exec_cmd(&format!(
        "umount -l {}* 2>/dev/null || true",
        target.part_path
    ));

    progress_fn(0.10, "Mounting Btrfs pool...".into());
    log_fn("Mounting Btrfs root pool without subvolume...".into());
    exec_cmd(&format!(
        "mount -t btrfs {} {}",
        target.part_path, btrfs_mnt
    ))?;

    progress_fn(0.20, "Preserving user accounts...".into());
    log_fn("Backing up user accounts (UID >= 1000)...".into());
    let old_root = format!("{}/@", btrfs_mnt);
    let mut preserved = if Path::new(&old_root).exists() {
        let p = users::preserve_users(&old_root);
        log_fn(format!("Preserved {} user account(s).", p.passwd.len()));
        p
    } else {
        users::PreservedUsers {
            passwd: Vec::new(),
            shadow: Vec::new(),
            group: Vec::new(),
            gshadow: Vec::new(),
            usernames: Vec::new(),
            group_memberships: std::collections::HashMap::new(),
        }
    };
    users::discover_home_users(btrfs_mnt, &mut preserved);

    let squashfs_path = match mode {
        RecoveryMode::Local => {
            progress_fn(0.30, "Locating local recovery image...".into());
            log_fn("Searching for local recovery squashfs...".into());
            match disk::detect_local_squashfs() {
                Some(p) => {
                    if !is_valid_base_squashfs(&p) {
                        log_fn("Local squashfs invalid, attempting internet recovery...".into());
                        download_recovery_image(log_fn, progress_fn)?
                    } else {
                        p
                    }
                }
                None => {
                    log_fn("No local recovery image found, downloading...".into());
                    download_recovery_image(log_fn, progress_fn)?
                }
            }
        }
        RecoveryMode::Internet(url) => {
            progress_fn(0.25, "Downloading image from GitHub Releases...".into());
            log_fn(format!("Downloading from: {}", url));
            let dl_path = "/tmp/pulsaros-remote-recovery.squashfs";
            exec_cmd_stream(
                &format!("curl -L -C - --retry 3 -o {} {}", dl_path, url),
                log_fn,
            )?;
            if !is_valid_base_squashfs(dl_path) {
                return Err(
                    "Downloaded recovery image is corrupt or invalid.\nNo changes were made to your disk.".into()
                );
            }
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
    if let Err(unsquash_err) = exec_cmd_stream(
        &format!("unsquashfs -f -d {}/@ {}", btrfs_mnt, squashfs_path),
        log_fn,
    ) {
        log_fn(format!(
            "Local unsquash failed ({}). Attempting internet recovery fallback...",
            unsquash_err
        ));
        let fallback_url = "https://github.com/Inled-Pulsar-OS/ISO/releases/download/latest/pulsaros-stable-arch-refind.squashfs";
        let dl_path = "/tmp/pulsaros-remote-recovery.squashfs";
        progress_fn(0.60, "Downloading fresh system image...".into());
        exec_cmd_stream(
            &format!("curl -L -C - --retry 3 -o {} {}", dl_path, fallback_url),
            log_fn,
        )?;
        if !is_valid_base_squashfs(dl_path) {
            return Err("Downloaded fallback image is corrupt or invalid.".into());
        }
        log_fn(format!(
            "Unsquashing downloaded image {} into {}/@...",
            dl_path, btrfs_mnt
        ));
        exec_cmd_stream(
            &format!("unsquashfs -f -d {}/@ {}", btrfs_mnt, dl_path),
            log_fn,
        )?;
    }

    progress_fn(0.85, "Re-injecting user credentials...".into());
    log_fn("Restoring user accounts into clean /etc...".into());
    let new_root = format!("{}/@", btrfs_mnt);
    users::restore_users(&new_root, &preserved, log_fn)?;

    progress_fn(0.90, "Configuring filesystem mounts...".into());
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

    progress_fn(0.91, "Deploying udev rules...".into());
    deploy_udev_rules(&new_root);

    progress_fn(0.92, "Deploying SDDM wallpaper...".into());
    deploy_sddm_wallpaper(&new_root, log_fn);

    progress_fn(0.93, "Cleaning up GNOME extensions...".into());
    cleanup_gnome_extensions(&new_root);

    progress_fn(0.95, "Deploying boot and recovery kernels...".into());
    crate::boot::deploy_boot_and_recovery_kernels(&new_root, &btrfs_uuid, log_fn);

    progress_fn(0.98, "Synchronizing disks...".into());
    log_fn("Syncing disks...".into());
    let _ = exec_cmd("sync");
    let _ = exec_cmd(&format!("umount -l {}", btrfs_mnt));

    progress_fn(1.0, "Restoration complete!".into());
    log_fn("System successfully restored.".into());
    Ok(())
}

fn download_recovery_image(
    log_fn: &dyn Fn(String),
    progress_fn: &dyn Fn(f64, String),
) -> Result<String, String> {
    progress_fn(0.25, "Downloading image from GitHub Releases...".into());
    let url = "https://github.com/Inled-Pulsar-OS/ISO/releases/download/latest/pulsaros-stable-arch-refind.squashfs";
    let dl_path = "/tmp/pulsaros-remote-recovery.squashfs";
    log_fn(format!("Downloading from: {}", url));
    exec_cmd_stream(
        &format!("curl -L -C - --retry 3 -o {} {}", dl_path, url),
        log_fn,
    )?;
    if !is_valid_base_squashfs(dl_path) {
        return Err("Downloaded image is corrupt or invalid.".into());
    }
    Ok(dl_path.to_string())
}

fn deploy_udev_rules(new_root: &str) {
    let udev_dir = format!("{}/etc/udev/rules.d", new_root);
    let _ = fs::create_dir_all(&udev_dir);
    let _ = fs::write(
        format!("{}/99-pulsaros-hide-recovery.rules", udev_dir),
        "# Hide PULSAR_RECOVERY partition from file managers and desktop\nENV{ID_FS_LABEL}==\"PULSAR_RECOVERY\", ENV{UDISKS_IGNORE}=\"1\", ENV{UDISKS_AUTO}=\"0\"\n",
    );
}

fn deploy_sddm_wallpaper(new_root: &str, log_fn: &dyn Fn(String)) {
    let sddm_dir = format!("{}/var/lib/pulsar-sddm", new_root);
    let _ = fs::create_dir_all(&sddm_dir);
    let _ = exec_cmd(&format!("chmod 777 {}", sddm_dir));
    let wallpaper_sources = [
        format!("{}/usr/share/backgrounds/pulsar-os-tahoe.png", new_root),
        format!(
            "{}/usr/share/sddm/themes/Apple.Tahoe/pulsar-os-tahoe.png",
            new_root
        ),
        format!(
            "{}/usr/share/backgrounds/gnome/pulsar-wallpaper.png",
            new_root
        ),
    ];
    for ws in &wallpaper_sources {
        if Path::new(ws).exists() {
            let _ = exec_cmd(&format!(
                "cp -f {} {}/pulsar-wallpaper.png",
                ws, sddm_dir
            ));
            let _ = exec_cmd(&format!(
                "chmod 666 {}/pulsar-wallpaper.png",
                sddm_dir
            ));
            log_fn(format!(
                "Deployed default SDDM wallpaper to {} from {}",
                sddm_dir, ws
            ));
            break;
        }
    }
}

fn cleanup_gnome_extensions(new_root: &str) {
    let _ = exec_cmd(&format!(
        "rm -rf {}/usr/share/gnome-shell/extensions/places-menu@gnome-shell-extensions.gcampax.github.com \
                {}/usr/share/gnome-shell/extensions/window-list@gnome-shell-extensions.gcampax.github.com \
                {}/usr/share/gnome-shell/extensions/search-light@icedman.github.com 2>/dev/null || true",
        new_root, new_root, new_root
    ));
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
            vec!["sudo", "-E", "gparted"],
            vec!["gnome-disks"],
            vec!["gnome-disk-utility"],
        ],
        "terminal" => vec![
            vec!["sh", "-c", "gnome-terminal -- sudo bash"],
            vec!["sh", "-c", "kgx -e 'sudo bash'"],
            vec!["sh", "-c", "alacritty -e sudo bash"],
            vec!["sh", "-c", "xfce4-terminal -e 'sudo bash'"],
            vec!["sh", "-c", "konsole -e sudo bash"],
            vec!["sh", "-c", "kitty sudo bash"],
            vec!["sh", "-c", "xterm -title 'Pulsar OS Recovery Terminal' -bg '#18181b' -fg '#ffffff' -fa Monospace -fs 11 -e sudo bash"],
            vec!["sh", "-c", "x-terminal-emulator -e sudo bash"],
            vec!["sh", "-c", "xterm -e sudo bash"],
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
    let _ = Command::new("sudo")
        .args(["-n", "systemctl", "reboot", "-i", "-f"])
        .spawn();
    let _ = Command::new("sudo")
        .args(["-n", "reboot", "-f"])
        .spawn();
    Ok(())
}
