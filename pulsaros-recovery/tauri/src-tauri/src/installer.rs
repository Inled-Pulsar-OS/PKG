use std::fs;
use std::io::{BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};

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

fn cleanup_mounts(is_efi: bool) {
    let _ = exec_cmd("fuser -k -9 -M /mnt 2>/dev/null || true");

    for mount_path in &[
        "/mnt/etc/resolv.conf",
        "/mnt/run",
        "/mnt/sys",
        "/mnt/proc",
        "/mnt/dev/pts",
        "/mnt/dev",
        "/mnt/recovery",
        "/mnt/boot/efi",
        "/mnt/home",
    ] {
        if Path::new(mount_path).exists() {
            let _ = exec_cmd(&format!("umount -f {} 2>/dev/null || true", mount_path));
        }
    }

    if is_efi {
        let _ = exec_cmd("umount -f /mnt/boot/efi 2>/dev/null || true");
    }

    let _ = exec_cmd("umount -f /mnt 2>/dev/null || true");
    let _ = exec_cmd("umount -f -l -R /mnt 2>/dev/null || true");
}

fn get_partition_uuid(part: &str) -> Result<String, String> {
    let out = exec_cmd(&format!("blkid -o value -s UUID {}", part))?;
    Ok(out.trim().to_string())
}

fn is_nvme_or_mmc(path: &str) -> bool {
    path.contains("nvme") || path.contains("mmcblk") || path.contains("loop")
}

fn part_path(disk: &str, num: usize) -> String {
    if is_nvme_or_mmc(disk) {
        format!("{}p{}", disk, num)
    } else {
        format!("{}{}", disk, num)
    }
}

fn deploy_recovery_squashfs(log_fn: &dyn Fn(String)) {
    let sources = [
        "/recovery/filesystem.squashfs",
        "/usr/share/pulsaros-recovery/recovery-filesystem.squashfs",
        "/mnt/usr/share/pulsaros-recovery/recovery-filesystem.squashfs",
        "/run/archiso/bootmnt/recovery/filesystem.squashfs",
        "/run/live/medium/recovery/filesystem.squashfs",
        "/lib/live/mount/medium/recovery/filesystem.squashfs",
        "/run/live/medium/live/filesystem.squashfs",
        "/lib/live/mount/medium/live/filesystem.squashfs",
        "/run/archiso/bootmnt/live/x86_64/airootfs.sfs",
        "/run/archiso/bootmnt/live/filesystem.squashfs",
        "/run/archiso/airootfs.sfs",
        "/live/filesystem.squashfs",
    ];

    let found = sources.iter().find(|p| Path::new(p).is_file());
    let Some(src) = found else {
        log_fn("WARNING: No recovery squashfs found".into());
        return;
    };

    let _ = exec_cmd("mkdir -p /mnt/recovery/live /mnt/recovery/recovery /mnt/live");
    let _ = exec_cmd(&format!("cp -f {} /mnt/recovery/live/filesystem.squashfs", src));
    let _ = exec_cmd(&format!(
        "cp -f {} /mnt/recovery/recovery/filesystem.squashfs",
        src
    ));
    let _ = exec_cmd(&format!(
        "cp -f {} /mnt/recovery/filesystem.squashfs",
        src
    ));
    let _ = exec_cmd(&format!(
        "cp -f {} /mnt/live/filesystem.squashfs",
        src
    ));
    log_fn(format!("Recovery squashfs deployed from {}", src));
}

fn deploy_base_squashfs(log_fn: &dyn Fn(String)) {
    let sources = [
        "/run/archiso/bootmnt/images/pulsaros-base.squashfs",
        "/run/archiso/bootmnt/arch/x86_64/airootfs.sfs",
        "/run/archiso/bootmnt/live/x86_64/airootfs.sfs",
        "/run/archiso/airootfs.sfs",
        "/run/live/medium/images/pulsaros-base.squashfs",
        "/recovery/images/pulsaros-base.squashfs",
    ];

    let found = sources.iter().find(|p| {
        Path::new(p).is_file()
            && fs::metadata(p)
                .map(|m| m.len() > 500 * 1024 * 1024)
                .unwrap_or(false)
    });

    let Some(src) = found else {
        log_fn("WARNING: No base squashfs found".into());
        return;
    };

    let _ = exec_cmd("mkdir -p /mnt/recovery/images/x86_64");
    let _ = exec_cmd(&format!("cp -f {} /mnt/recovery/images/pulsaros-base.squashfs", src));
    let _ = exec_cmd(
        "ln -f /mnt/recovery/images/pulsaros-base.squashfs /mnt/recovery/images/x86_64/airootfs.sfs",
    );
    log_fn(format!("Base squashfs deployed from {}", src));
}

fn deploy_recovery_kernel_and_initrd(log_fn: &dyn Fn(String)) {
    let kernel_sources = [
        "/recovery/vmlinuz-recovery",
        "/usr/share/pulsaros-recovery/vmlinuz-recovery",
        "/run/archiso/bootmnt/recovery/vmlinuz-recovery",
        "/run/live/medium/recovery/vmlinuz-recovery",
        "/mnt/usr/share/pulsaros-recovery/vmlinuz-recovery",
        "/mnt/recovery/vmlinuz-recovery",
    ];

    let initrd_sources = [
        "/recovery/initramfs-recovery.img",
        "/usr/share/pulsaros-recovery/initramfs-recovery.img",
        "/run/archiso/bootmnt/recovery/initramfs-recovery.img",
        "/run/live/medium/recovery/initramfs-recovery.img",
        "/mnt/usr/share/pulsaros-recovery/initramfs-recovery.img",
        "/mnt/recovery/initramfs-recovery.img",
    ];

    let esp_root = "/mnt/boot/efi";
    let _ = exec_cmd(&format!("mkdir -p /mnt/boot /mnt/recovery/boot {}/EFI/recovery", esp_root));

    if let Some(k) = kernel_sources.iter().find(|p| {
        let path = Path::new(p);
        path.is_file() && !p.ends_with(".kver") && fs::metadata(p).map(|m| m.len() > 1024).unwrap_or(false)
    }) {
        let _ = exec_cmd(&format!("cp -f {} /mnt/boot/vmlinuz-recovery", k));
        let _ = exec_cmd(&format!("cp -f {} /mnt/recovery/boot/vmlinuz-recovery", k));
        let _ = exec_cmd(&format!("cp -f {} /mnt/recovery/boot/vmlinuz-linux", k));
        let _ = exec_cmd(&format!("cp -f {} /mnt/recovery/vmlinuz-recovery", k));
        let _ = exec_cmd(&format!("cp -f {} {}/EFI/recovery/vmlinuz-recovery.efi", k, esp_root));
        let _ = exec_cmd(&format!("cp -f {} {}/EFI/recovery/vmlinuz-recovery", k, esp_root));
        let _ = exec_cmd(&format!("cp -f {} {}/EFI/recovery/vmlinuz.efi", k, esp_root));
        let _ = exec_cmd(&format!("cp -f {} {}/EFI/recovery/vmlinuz", k, esp_root));

        let rec_opts = "boot=live components username=live autologin cow_spacesize=4G live-media=/dev/disk/by-label/PULSAR_RECOVERY live-media-path=live fsck.mode=skip quiet splash";
        let refind_linux = format!(
            "\"Boot Pulsar OS Recovery\"  \"{}\"\n\"Boot Recovery (Debug)\"     \"{}\"\n",
            rec_opts,
            rec_opts.replace("quiet splash", "loglevel=7 live-debug")
        );
        let _ = fs::write("/mnt/recovery/boot/refind_linux.conf", &refind_linux);

        let esp_refind_opts = rec_opts.replace(
            "live-media=/dev/disk/by-label/PULSAR_RECOVERY",
            "live-media=any",
        );
        let _ = fs::write(
            format!("{}/EFI/recovery/refind_linux.conf", esp_root),
            format!(
                "\"Boot Pulsar OS Recovery\"  \"{}\"\n",
                esp_refind_opts
            ),
        );

        log_fn(format!("Recovery kernel deployed from {}", k));
    } else {
        log_fn("WARNING: Recovery kernel not found".into());
    }

    if let Some(i) = initrd_sources.iter().find(|p| {
        let path = Path::new(p);
        path.is_file() && fs::metadata(p).map(|m| m.len() > 1024).unwrap_or(false)
    }) {
        let _ = exec_cmd(&format!("cp -f {} /mnt/boot/initramfs-recovery.img", i));
        let _ = exec_cmd(&format!(
            "cp -f {} /mnt/recovery/boot/initramfs-recovery.img",
            i
        ));
        let _ = exec_cmd(&format!("cp -f {} /mnt/recovery/initramfs-recovery.img", i));
        let _ = exec_cmd(&format!(
            "cp -f {} {}/EFI/recovery/initramfs-recovery.img",
            i, esp_root
        ));
        let _ = exec_cmd(&format!(
            "cp -f {} {}/EFI/recovery/initrd.img",
            i, esp_root
        ));
        log_fn(format!("Recovery initramfs deployed from {}", i));
    } else {
        log_fn("WARNING: Recovery initramfs not found".into());
    }
}

fn configure_refind_menus(root_uuid: &str, log_fn: &dyn Fn(String)) {
    let esp_root = "/mnt/boot/efi";

    let k_name = fs::read_dir("/mnt/boot")
        .ok()
        .and_then(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.path())
                .find(|p| {
                    p.file_name()
                        .map(|n| {
                            let s = n.to_string_lossy();
                            s.starts_with("vmlinuz") && !s.ends_with(".kver")
                        })
                        .unwrap_or(false)
                })
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "vmlinuz-linux".into());

    let initrd_name = fs::read_dir("/mnt/boot")
        .ok()
        .and_then(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.path())
                .find(|p| {
                    p.file_name()
                        .map(|n| {
                            let s = n.to_string_lossy();
                            (s.starts_with("initramfs-") || s.starts_with("initrd"))
                                && !s.contains("fallback")
                                && !s.contains("ucode")
                                && !s.ends_with(".kver")
                        })
                        .unwrap_or(false)
                })
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "initramfs-linux.img".into());

    let ucode_lines: String = ["amd-ucode.img", "intel-ucode.img"]
        .iter()
        .filter(|uc| Path::new(&format!("/mnt/boot/{}", uc)).exists())
        .map(|uc| format!("    initrd /@/boot/{}\n", uc))
        .collect();

    let rec_opts_rec = "boot=live components username=live autologin cow_spacesize=4G live-media=/dev/disk/by-label/PULSAR_RECOVERY live-media-path=live fsck.mode=skip quiet splash";
    let rec_opts_auto = "boot=live components username=live autologin cow_spacesize=4G live-media-path=live fsck.mode=skip quiet splash";
    let rec_net_opts = "boot=live components username=live autologin cow_spacesize=4G internet_recovery=1 quiet splash";

    let menu_block = format!(
        r#"
# PULSAR-MENU-BEGIN
scanfor manual,external,optical
default_selection 1

menuentry "Pulsar OS" {{
    icon /EFI/refind/themes/rEFInd-Regular-Dark/icons/os_pulsaros_normal.png
    volume PULSAR_OS
    loader /@/boot/{k_name}
{ucode_lines}    initrd /@/boot/{initrd_name}
    options "root=UUID={root_uuid} rootflags=subvol=@ rw quiet splash"
    submenuentry "Boot to single-user mode" {{
        options "root=UUID={root_uuid} rootflags=subvol=@ rw single"
    }}
}}

menuentry "Pulsar OS Recovery" {{
    icon /EFI/refind/themes/rEFInd-Regular-Dark/icons/os_recovery.png
    volume PULSAR_OS
    loader /@/boot/vmlinuz-recovery
    initrd /@/boot/initramfs-recovery.img
    options "{rec_opts_rec}"
    submenuentry "Boot Recovery from ESP" {{
        loader /EFI/recovery/vmlinuz-recovery
        initrd /EFI/recovery/initramfs-recovery.img
        options "{rec_opts_rec}"
    }}
    submenuentry "Boot Recovery (Auto-Detect Drive)" {{
        volume PULSAR_OS
        loader /@/boot/vmlinuz-recovery
        initrd /@/boot/initramfs-recovery.img
        options "{rec_opts_auto}"
    }}
    submenuentry "Boot Recovery (Debug Mode)" {{
        volume PULSAR_OS
        loader /@/boot/vmlinuz-recovery
        initrd /@/boot/initramfs-recovery.img
        options "{rec_opts_debug}"
    }}
    submenuentry "Internet Recovery" {{
        options "{rec_net_opts}"
    }}
}}
# PULSAR-MENU-END
"#,
        rec_opts_debug = rec_opts_rec.replace("quiet splash", "loglevel=7 live-debug"),
    );

    for dir in &[
        format!("{}/EFI/refind", esp_root),
        format!("{}/EFI/BOOT", esp_root),
    ] {
        let conf_path = format!("{}/refind.conf", dir);
        if !Path::new(dir).is_dir() {
            log_fn(format!("Notice: {} absent, skipping menu config", dir));
            continue;
        }

        let mut content = fs::read_to_string(&conf_path).unwrap_or_default();

        if !content.contains("include themes/rEFInd-Regular-Dark/theme.conf")
            && Path::new(&format!("{}/themes/rEFInd-Regular-Dark/theme.conf", dir)).exists()
        {
            content.push_str("\ninclude themes/rEFInd-Regular-Dark/theme.conf\n");
        }

        let begin = "# PULSAR-MENU-BEGIN";
        let end = "# PULSAR-MENU-END";
        if let Some(start_idx) = content.find(begin) {
            if let Some(end_idx) = content[start_idx..].find(end) {
                let full_end = start_idx + end_idx + end.len() + 1;
                content.replace_range(start_idx..full_end.min(content.len()), "");
            }
        }

        content.push_str(&menu_block);

        let _ = fs::write(&conf_path, &content);
        log_fn(format!("rEFInd menu configured: {}", conf_path));
    }
}

/// Run the full installation pipeline.
pub fn run_installation(
    disk_path: &str,
    install_broadcom: bool,
    log_fn: &dyn Fn(String),
    progress_fn: &dyn Fn(f64, String),
) -> Result<(), String> {
    let log_file = "/tmp/pulsaros-install.log";
    let _ = fs::write(log_file, "Pulsar OS Installation started\n");

    let _ = exec_cmd("systemctl stop udisks2.service 2>/dev/null || true");

    let is_efi = Path::new("/sys/firmware/efi").exists();
    let is_arch = Path::new("/etc/pacman.conf").exists();

    // Unmount busy partitions on target disk
    if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0].starts_with(disk_path) {
                log_fn(format!("Unmounting busy partition: {} from {}", parts[0], parts[1]));
                let _ = exec_cmd(&format!("umount -l {} 2>/dev/null || true", parts[1]));
            }
        }
    }
    let _ = exec_cmd("swapoff -a 2>/dev/null || true");

    let (root_part, recovery_part, efi_part) = if is_efi {
        progress_fn(0.05, "Partitioning disk (GPT: EFI, Recovery, Btrfs)...".into());
        log_fn("Wiping and partitioning (UEFI)...".into());

        exec_cmd(&format!("wipefs -a -f {}", disk_path))?;
        exec_cmd(&format!("sgdisk --zap-all {}", disk_path))?;
        exec_cmd(&format!("sgdisk --clear {}", disk_path))?;
        exec_cmd(&format!(
            "sgdisk --new=1:0:+512M --typecode=1:ef00 --change-name=1:EFI {}",
            disk_path
        ))?;
        exec_cmd(&format!(
            "sgdisk --new=2:0:+4G --typecode=2:8300 --change-name=2:PulsarRecovery {}",
            disk_path
        ))?;
        exec_cmd(&format!(
            "sgdisk --new=3:0:0 --typecode=3:8300 --change-name=3:PulsarOS {}",
            disk_path
        ))?;
        exec_cmd("sync")?;
        exec_cmd("udevadm settle")?;
        let _ = exec_cmd(&format!("partprobe {} 2>/dev/null || true", disk_path));
        exec_cmd("udevadm settle")?;

        let ep = part_path(disk_path, 1);
        let rp = part_path(disk_path, 2);
        let rp3 = part_path(disk_path, 3);

        exec_cmd(&format!("wipefs -a -f {}", ep))?;
        exec_cmd(&format!("wipefs -a -f {}", rp))?;
        exec_cmd(&format!("wipefs -a -f {}", rp3))?;

        progress_fn(0.10, "Formatting partitions...".into());
        exec_cmd(&format!("mkfs.vfat -F32 -n EFI {}", ep))?;
        exec_cmd(&format!("mkfs.ext4 -F -F -L PULSAR_RECOVERY {}", rp))?;
        exec_cmd(&format!("mkfs.btrfs -f -L PULSAR_OS {}", rp3))?;
        exec_cmd("sync")?;
        exec_cmd("udevadm settle")?;

        (rp3, rp, ep)
    } else {
        progress_fn(0.05, "Partitioning disk (MBR: Recovery, Btrfs)...".into());
        log_fn("Wiping and partitioning (BIOS)...".into());

        exec_cmd(&format!("wipefs -a -f {}", disk_path))?;
        exec_cmd(&format!("dd if=/dev/zero of={} bs=512 count=2048", disk_path))?;
        exec_cmd(&format!(
            "echo 'label: dos\\nsize=4096M, type=83\\nsize=+, type=83, bootable\\n' | sfdisk {}",
            disk_path
        ))?;
        exec_cmd("sync")?;
        exec_cmd("udevadm settle")?;
        let _ = exec_cmd(&format!("sfdisk --activate {} 2 2>/dev/null || true", disk_path));
        let _ = exec_cmd(&format!("partprobe {} 2>/dev/null || true", disk_path));
        exec_cmd("udevadm settle")?;

        let rp = part_path(disk_path, 1);
        let rp2 = part_path(disk_path, 2);

        exec_cmd(&format!("wipefs -a -f {}", rp))?;
        exec_cmd(&format!("wipefs -a -f {}", rp2))?;

        progress_fn(0.10, "Formatting partitions...".into());
        exec_cmd(&format!("mkfs.ext4 -F -F -L PULSAR_RECOVERY {}", rp))?;
        exec_cmd(&format!("mkfs.btrfs -f -L PULSAR_OS {}", rp2))?;
        exec_cmd("sync")?;
        exec_cmd("udevadm settle")?;

        (rp2, rp, String::new())
    };

    let _ = exec_cmd("modprobe btrfs 2>/dev/null || true");
    let _ = exec_cmd("modprobe ext4 2>/dev/null || true");
    if is_efi {
        let _ = exec_cmd("modprobe vfat 2>/dev/null || true");
    }

    progress_fn(0.15, "Creating Btrfs subvolumes...".into());
    let _ = exec_cmd("umount -l /mnt 2>/dev/null || true");
    let _ = exec_cmd("mkdir -p /mnt");
    exec_cmd(&format!("mount -t btrfs {} /mnt", root_part))?;
    exec_cmd("btrfs subvolume create /mnt/@")?;
    exec_cmd("btrfs subvolume create /mnt/@home")?;
    exec_cmd("umount /mnt")?;

    progress_fn(0.18, "Mounting Btrfs subvolumes...".into());
    exec_cmd(&format!(
        "mount -t btrfs -o subvol=@,compress=zstd:1 {} /mnt",
        root_part
    ))?;
    exec_cmd("mount --make-rprivate /mnt")?;
    let _ = exec_cmd("mkdir -p /mnt/home");
    exec_cmd(&format!(
        "mount -t btrfs -o subvol=@home,compress=zstd:1 {} /mnt/home",
        root_part
    ))?;
    if is_efi {
        let _ = exec_cmd("mkdir -p /mnt/boot/efi");
        exec_cmd(&format!("mount -t vfat {} /mnt/boot/efi", efi_part))?;
    }
    let _ = exec_cmd("mkdir -p /mnt/recovery");
    exec_cmd(&format!(
        "mount -t ext4 {} /mnt/recovery",
        recovery_part
    ))?;

    // ── Rsync ──
    progress_fn(0.25, "Replicating system files...".into());
    log_fn("Starting rsync...".into());

    let rsync_cmd = "rsync -aHAXx \
        --info=progress2 \
        --exclude=/dev/* \
        --exclude=/proc/* \
        --exclude=/sys/* \
        --exclude=/tmp/* \
        --exclude=/run/* \
        --exclude=/mnt/* \
        --exclude=/media/* \
        --exclude=/lost+found \
        --exclude=/var/tmp/* \
        --exclude=/var/log/* \
        --exclude=/home/*/.local/share/gvfs-metadata/* \
        --exclude=/home/*/.cache/* \
        --exclude=/root/.cache/* \
        / /mnt";

    let mut child = Command::new("sudo")
        .args(["-n", "sh", "-c", rsync_cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rsync: {}", e))?;

    {
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);
        let mut buffer = String::new();

        for byte in reader.bytes().map_while(Result::ok) {
            if byte == b'\r' || byte == b'\n' {
                let line = buffer.trim().to_string();
                buffer.clear();
                if let Some(m) = line.find('%') {
                    if let Some(pct_str) = line[..m].split_whitespace().last() {
                        if let Ok(pct) = pct_str.parse::<f64>() {
                            let frac = 0.25 + (pct / 100.0) * 0.55;
                            progress_fn(frac, format!("Copying files: {}%", pct as i64));
                        }
                    }
                }
            } else {
                buffer.push(byte as char);
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for rsync: {}", e))?;

    // Exit code 24 = vanished source files (normal for running live system)
    if !status.success() && status.code() != Some(24) {
        let stderr = child.stderr.take().map(|s| {
            let mut buf = String::new();
            BufReader::new(s).read_to_string(&mut buf).ok();
            buf
        });
        return Err(format!(
            "rsync failed (code {:?}): {}",
            status.code(),
            stderr.unwrap_or_default()
        ));
    }
    log_fn("rsync completed".into());

    // ── Deploy squashfs images ──
    progress_fn(0.80, "Deploying recovery images...".into());
    deploy_recovery_squashfs(log_fn);
    deploy_base_squashfs(log_fn);

    // ── Fstab + udev ──
    progress_fn(0.85, "Configuring filesystem mounts...".into());

    let root_uuid = get_partition_uuid(&root_part)?;
    let efi_uuid = if is_efi {
        get_partition_uuid(&efi_part)?
    } else {
        String::new()
    };

    let _ = exec_cmd("mkdir -p /mnt/etc /mnt/dev /mnt/proc /mnt/sys /mnt/run /mnt/etc/udev/rules.d");

    let fstab = if is_efi {
        format!(
            "# /etc/fstab: Pulsar OS Btrfs Configuration\n\
             # <file system>             <mount point>   <type>  <options>                                       <dump>  <pass>\n\
             UUID={:<24}/               btrfs   subvol=@,compress=zstd:1,space_cache=v2         0       0\n\
             UUID={:<24}/home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2     0       0\n\
             UUID={:<24}/boot/efi       vfat    umask=0077                                      0       2\n",
            root_uuid, root_uuid, efi_uuid
        )
    } else {
        format!(
            "# /etc/fstab: Pulsar OS Btrfs Configuration (BIOS)\n\
             # <file system>             <mount point>   <type>  <options>                                       <dump>  <pass>\n\
             UUID={:<24}/               btrfs   subvol=@,compress=zstd:1,space_cache=v2         0       0\n\
             UUID={:<24}/home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2     0       0\n",
            root_uuid, root_uuid
        )
    };

    fs::write("/mnt/etc/fstab", &fstab)
        .map_err(|e| format!("Failed to write fstab: {}", e))?;
    fs::write(
        "/mnt/etc/udev/rules.d/99-pulsaros-hide-recovery.rules",
        "# Hide PULSAR_RECOVERY partition from file managers and desktop\nENV{ID_FS_LABEL}==\"PULSAR_RECOVERY\", ENV{UDISKS_IGNORE}=\"1\", ENV{UDISKS_AUTO}=\"0\"\n",
    )
    .map_err(|e| format!("Failed to write udev rules: {}", e))?;

    // ── Deploy kernel + initrd ──
    progress_fn(0.88, "Deploying boot and recovery kernels...".into());
    deploy_recovery_kernel_and_initrd(log_fn);

    // ── Bind mount for chroot ──
    progress_fn(0.90, "Installing bootloader...".into());
    exec_cmd("mount --bind /dev /mnt/dev")?;
    let _ = exec_cmd("mkdir -p /mnt/dev/pts");
    exec_cmd("mount -t devpts devpts /mnt/dev/pts")?;
    exec_cmd("mount --bind /proc /mnt/proc")?;
    exec_cmd("mount --bind /sys /mnt/sys")?;
    exec_cmd("mount -t tmpfs tmpfs /mnt/run")?;

    // ── Bootloader ──
    let refind_available = Path::new("/mnt/usr/bin/refind-install").exists()
        || Path::new("/mnt/usr/sbin/refind-install").exists()
        || Path::new("/mnt/bin/refind-install").exists();

    let mut refind_installed = false;

    if is_efi && refind_available {
        log_fn("Installing rEFInd bootloader...".into());
        let live_refind = ["/usr/bin/refind-install", "/usr/sbin/refind-install", "/bin/refind-install"]
            .iter()
            .find(|p| Path::new(p).exists());

        if let Some(ri) = live_refind {
            match exec_cmd(&format!("{} --root /mnt --yes", ri)) {
                Ok(_) => {
                    refind_installed = true;
                    log_fn("rEFInd installed successfully".into());

                    // Clean stale dirs
                    for rel in &["EFI/Linux", "EFI/systemd", "EFI/tools", "loader", "grub"] {
                        let _ = exec_cmd(&format!("rm -rf /mnt/boot/efi/{}", rel));
                    }
                    for rel in &[
                        "vmlinuz-linux",
                        "initramfs-linux.img",
                        "initramfs-linux-fallback.img",
                        "amd-ucode.img",
                        "refind_linux.conf",
                    ] {
                        let _ = exec_cmd(&format!("rm -f /mnt/boot/efi/{}", rel));
                    }

                    // Write refind_linux.conf
                    let conf_content = format!(
                        "\"Boot with standard options\"  \"root=UUID={} rootflags=subvol=@ rw quiet splash\"\n\
                         \"Boot to single-user mode\"    \"root=UUID={} rootflags=subvol=@ rw single\"\n\
                         \"Boot with minimal options\"   \"ro root=UUID={} rootflags=subvol=@\"\n",
                        root_uuid, root_uuid, root_uuid
                    );
                    let _ = fs::write("/mnt/boot/refind_linux.conf", &conf_content);

                    // Copy refind_x64.efi as BOOTX64.EFI
                    let refind_efi = ["/mnt/usr/share/refind/refind_x64.efi", "/mnt/usr/share/refind/refind/refind_x64.efi"]
                        .iter()
                        .find(|p| Path::new(p).is_file());

                    if let Some(efi) = refind_efi {
                        let _ = exec_cmd("mkdir -p /mnt/boot/efi/EFI/BOOT");
                        let _ = exec_cmd(&format!("cp -f {} /mnt/boot/efi/EFI/BOOT/BOOTX64.EFI", efi));
                    }

                    // Copy filesystem drivers
                    let driver_dirs = [
                        "/usr/share/refind/drivers_x64",
                        "/usr/share/refind/refind/drivers_x64",
                        "/run/archiso/bootmnt/EFI/refind/drivers_x64",
                        "/run/live/medium/EFI/BOOT/drivers_x64",
                        "/run/live/medium/EFI/refind/drivers_x64",
                    ];
                    for drv_name in &["ext4_x64.efi", "btrfs_x64.efi", "iso9660_x64.efi"] {
                        if let Some(src_dir) = driver_dirs.iter().find(|d| {
                            Path::new(&format!("{}/{}", d, drv_name)).is_file()
                        }) {
                            let src = format!("{}/{}", src_dir, drv_name);
                            for target_dir in &[
                                "/mnt/boot/efi/EFI/refind/drivers_x64",
                                "/mnt/boot/efi/EFI/refind/drivers",
                                "/mnt/boot/efi/EFI/BOOT/drivers_x64",
                                "/mnt/boot/efi/EFI/BOOT/drivers",
                                "/mnt/boot/efi/drivers_x64",
                                "/mnt/boot/efi/drivers",
                            ] {
                                let _ = exec_cmd(&format!("mkdir -p {}", target_dir));
                                let _ = exec_cmd(&format!("cp -f {} {}/{}", src, target_dir, drv_name));
                            }
                        }
                    }
                }
                Err(e) => {
                    log_fn(format!("Warning: rEFInd failed ({}), falling back to GRUB", e));
                    let _ = exec_cmd(&format!(
                        "chroot /mnt grub-install --force --removable {}",
                        disk_path
                    ));
                }
            }
        }
    } else if is_efi {
        log_fn("Installing GRUB bootloader...".into());
        let _ = exec_cmd(&format!(
            "chroot /mnt grub-install --force {}",
            disk_path
        ));
        exec_cmd(&format!(
            "chroot /mnt grub-install --force --removable {}",
            disk_path
        ))?;
    } else {
        log_fn("Installing GRUB bootloader (BIOS)...".into());
        exec_cmd(&format!(
            "chroot /mnt grub-install --target=i386-pc --force {}",
            disk_path
        ))?;
    }

    // ── Post-bootloader config ──
    if is_arch {
        // Remove archiso/live configs
        for f_live in &[
            "/mnt/etc/mkinitcpio.conf.d/archiso.conf",
            "/mnt/etc/mkinitcpio.conf.d/live.conf",
        ] {
            let _ = fs::remove_file(f_live);
        }

        // Add btrfs hook
        let mkinit_path = "/mnt/etc/mkinitcpio.conf";
        if let Ok(content) = fs::read_to_string(mkinit_path) {
            if !content.contains("btrfs") && content.contains("HOOKS=") {
                let updated = content.replace("HOOKS=(", "HOOKS=(btrfs ");
                let _ = fs::write(mkinit_path, &updated);
            }
        }

        // Set GRUB_DISTRIBUTOR
        let grub_default = "/mnt/etc/default/grub";
        if let Ok(content) = fs::read_to_string(grub_default) {
            let updated = content
                .lines()
                .map(|line| {
                    if line.contains("GRUB_DISTRIBUTOR") {
                        "GRUB_DISTRIBUTOR=\"Pulsar OS\""
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let _ = fs::write(grub_default, &updated);
        }

        let _ = exec_cmd("chroot /mnt mkinitcpio -P 2>/dev/null || true");

        if !refind_installed {
            let _ = exec_cmd("chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg");
        } else {
            configure_refind_menus(&root_uuid, log_fn);
        }
    } else if !refind_installed {
        let _ = exec_cmd("chroot /mnt update-grub");
    } else {
        configure_refind_menus(&root_uuid, log_fn);
    }

    // ── dconf update ──
    progress_fn(0.92, "Applying system settings...".into());
    let _ = exec_cmd("chroot /mnt dconf update 2>/dev/null || true");

    // ── Broadcom drivers ──
    if install_broadcom {
        progress_fn(0.93, "Installing Broadcom drivers...".into());
        let _ = exec_cmd("mount --bind /etc/resolv.conf /mnt/etc/resolv.conf");

        let has_net = Command::new("ping")
            .args(["-c", "1", "-W", "3", "1.1.1.1"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || Command::new("curl")
                .args(["-s", "-I", "-m", "3", "https://archlinux.org"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

        if has_net {
            let blacklist = "blacklist b43\nblacklist b43legacy\nblacklist ssb\nblacklist bcm43xx\nblacklist brcm80211\nblacklist brcmfmac\nblacklist brcmsmac\n";

            if is_arch {
                let _ = exec_cmd("chroot /mnt pacman -Sy --noconfirm");
                let _ = exec_cmd("chroot /mnt pacman -S --noconfirm --needed broadcom-wl-dkms linux-headers");
                let _ = exec_cmd("mkdir -p /mnt/etc/modprobe.d");
                let _ = fs::write("/mnt/etc/modprobe.d/broadcom-sta-blacklist.conf", blacklist);
            } else {
                let policy_file = "/mnt/usr/sbin/policy-rc.d";
                let _ = exec_cmd("mkdir -p /mnt/usr/sbin");
                let _ = fs::write(policy_file, "#!/bin/sh\nexit 101\n");
                let _ = exec_cmd("chmod 755 /mnt/usr/sbin/policy-rc.d");

                // Enable non-free repos
                if let Ok(content) = fs::read_to_string("/mnt/etc/apt/sources.list") {
                    let updated: Vec<String> = content
                        .lines()
                        .map(|line| {
                            let s = line.trim();
                            if !s.starts_with('#') && s.contains("main") {
                                let mut l = line.to_string();
                                for comp in &["contrib", "non-free", "non-free-firmware"] {
                                    if !l.contains(comp) {
                                        l.push_str(&format!(" {}", comp));
                                    }
                                }
                                l
                            } else {
                                line.to_string()
                            }
                        })
                        .collect();
                    let _ = fs::write("/mnt/etc/apt/sources.list", updated.join("\n"));
                }

                let _ = exec_cmd("chroot /mnt apt-get update");
                let _ = exec_cmd("chroot /mnt apt-get install -y broadcom-sta-dkms linux-headers-amd64");
                let _ = exec_cmd("mkdir -p /mnt/etc/modprobe.d");
                let _ = fs::write("/mnt/etc/modprobe.d/broadcom-sta-blacklist.conf", blacklist);
                let _ = fs::remove_file(policy_file);
            }
        } else {
            log_fn("No internet connection, Broadcom driver install skipped".into());
        }

        let _ = exec_cmd("umount -l /mnt/etc/resolv.conf 2>/dev/null || true");
    }

    // ── OOTB flag ──
    progress_fn(0.95, "Creating setup flag...".into());
    let _ = exec_cmd("mkdir -p /mnt/etc");
    let _ = fs::write("/mnt/etc/pulsar-need-setup", "");

    // ── Cleanup ──
    cleanup_mounts(is_efi);

    progress_fn(1.0, "Installation complete!".into());
    log_fn("Pulsar OS installed successfully.".into());

    let _ = exec_cmd("systemctl start udisks2.service 2>/dev/null || true");

    Ok(())
}
