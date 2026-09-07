//! Motor de restauración / reinstalación del sistema.
use crate::demo::is_demo_mode;
use crate::models::{BtrfsTarget, RecoveryMode};
use crate::system::{detect_local_squashfs, exec_cmd, exec_cmd_stream, is_valid_base_squashfs};
use regex::Regex;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::thread;
pub(crate) fn run_restoration<F, L>(
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

pub(crate) fn deploy_boot_and_recovery_kernels<L>(new_root: &str, btrfs_uuid: &str, log: &L)
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
