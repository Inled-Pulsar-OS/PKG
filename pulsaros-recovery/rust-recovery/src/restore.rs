use crate::demo::is_demo_mode;
use crate::models::{AppType, BtrfsTarget, InstalledApp, PreservedSettings, RecoveryMode};
use crate::system::{detect_local_squashfs, exec_cmd, exec_cmd_stream, is_valid_base_squashfs};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, File};
use std::io::BufReader;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::thread;

// Temporary live-session accounts must never survive a restore. `root` is not
// preserved either (the new image has its own), but it is a valid member of
// system groups and must not be stripped from them.
const EXCLUDED_USERS: &[&str] = &["live", "pulsar-live", "archiso", "root", "nobody"];
const EXCLUDED_GROUP_MEMBERS: &[&str] = &["live", "pulsar-live", "archiso", "nobody"];
const EXCLUDED_GROUPS: &[&str] = &["live", "pulsar-live", "archiso"];

// Every restored user gets these. Missing groups are created with a free GID.
const BASE_ADMIN_GROUPS: &[&str] = &[
    "wheel", "sudo", "video", "audio", "input", "storage", "network", "optical",
    "power", "rfkill", "autologin", "users", "lp", "scanner", "kvm",
];

#[derive(Clone, Debug)]
struct AccountRecord {
    name: String,
    uid: u32,
    gid: u32,
    gecos: String,
    home: String,
    shell: String,
}

#[derive(Clone, Debug)]
struct GroupRecord {
    name: String,
    passwd: String,
    gid: u32,
    members: BTreeSet<String>,
}

#[derive(Default)]
struct PreservedAccounts {
    users: Vec<AccountRecord>,
    shadow: BTreeMap<String, String>,
    groups: Vec<GroupRecord>,
    gshadow: BTreeMap<String, String>,
    user_groups: HashMap<String, BTreeSet<String>>,
    usernames: Vec<String>,
}

fn is_excluded_user(name: &str) -> bool {
    EXCLUDED_USERS.iter().any(|n| *n == name)
}

fn is_excluded_group(name: &str) -> bool {
    EXCLUDED_GROUPS.iter().any(|n| *n == name)
}

fn is_excluded_member(name: &str) -> bool {
    EXCLUDED_GROUP_MEMBERS.iter().any(|n| *n == name)
}

fn parse_passwd(content: &str) -> Vec<AccountRecord> {
    let mut users = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 7 {
            continue;
        }
        let Ok(uid) = parts[2].parse::<u32>() else { continue };
        let Ok(gid) = parts[3].parse::<u32>() else { continue };
        users.push(AccountRecord {
            name: parts[0].to_string(),
            uid,
            gid,
            gecos: parts[4].to_string(),
            home: parts[5].to_string(),
            shell: parts[6].to_string(),
        });
    }
    users
}

fn passwd_line(user: &AccountRecord) -> String {
    format!(
        "{}:x:{}:{}:{}:{}:{}",
        user.name, user.uid, user.gid, user.gecos, user.home, user.shell
    )
}

fn parse_groups(content: &str) -> Vec<GroupRecord> {
    let mut groups = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 4 {
            continue;
        }
        let Ok(gid) = parts[2].parse::<u32>() else { continue };
        let members = parts[3]
            .split(',')
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(str::to_string)
            .collect();
        groups.push(GroupRecord {
            name: parts[0].to_string(),
            passwd: parts[1].to_string(),
            gid,
            members,
        });
    }
    groups
}

// One entry per name: the first GID/password wins, members are merged. This
// also repairs files that a previous restore already left with duplicates.
fn merge_group_records(records: Vec<GroupRecord>) -> BTreeMap<String, GroupRecord> {
    let mut groups: BTreeMap<String, GroupRecord> = BTreeMap::new();
    for record in records {
        if let Some(existing) = groups.get_mut(&record.name) {
            existing.members.extend(record.members);
        } else {
            groups.insert(record.name.clone(), record);
        }
    }
    groups
}

// name -> raw line. Duplicate names keep the first line.
fn parse_name_map(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let name = line.split(':').next().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        map.entry(name.to_string()).or_insert_with(|| line.to_string());
    }
    map
}

fn next_free_uid(used: &HashSet<u32>, start: u32) -> u32 {
    let mut candidate = start.max(1000);
    while used.contains(&candidate) || (65534..=65535).contains(&candidate) {
        candidate += 1;
    }
    candidate
}

fn next_free_gid(used: &HashMap<u32, String>, start: u32) -> u32 {
    let mut candidate = start.max(100);
    while used.contains_key(&candidate) || (65534..=65535).contains(&candidate) {
        candidate += 1;
    }
    candidate
}

fn collect_accounts<L>(old_root: &str, home_dir: &str, log: &L) -> PreservedAccounts
where
    L: Fn(&str) + Send + Sync + 'static,
{
    let mut preserved = PreservedAccounts::default();

    if Path::new(old_root).exists() {
        if let Ok(passwd_raw) = fs::read_to_string(format!("{}/etc/passwd", old_root)) {
            for user in parse_passwd(&passwd_raw) {
                if is_excluded_user(&user.name) {
                    continue;
                }
                if user.uid < 1000 || user.uid >= 65534 {
                    continue;
                }
                if preserved.usernames.iter().any(|n| *n == user.name) {
                    continue;
                }
                preserved.usernames.push(user.name.clone());
                preserved.users.push(user);
            }
        }

        if let Ok(shadow_raw) = fs::read_to_string(format!("{}/etc/shadow", old_root)) {
            preserved.shadow = parse_name_map(&shadow_raw);
        }

        if let Ok(group_raw) = fs::read_to_string(format!("{}/etc/group", old_root)) {
            for group in merge_group_records(parse_groups(&group_raw)).into_values() {
                if is_excluded_group(&group.name) {
                    continue;
                }
                for member in &group.members {
                    if is_excluded_member(member) {
                        continue;
                    }
                    preserved
                        .user_groups
                        .entry(member.clone())
                        .or_default()
                        .insert(group.name.clone());
                }
                preserved.groups.push(group);
            }
        }

        if let Ok(gshadow_raw) = fs::read_to_string(format!("{}/etc/gshadow", old_root)) {
            preserved.gshadow = parse_name_map(&gshadow_raw);
        }
    }

    // Homes are authoritative when /etc/passwd was already lost. Use the
    // directory owner instead of a hardcoded 1000:1000, which would both
    // duplicate an existing UID and leave the home owned by another user.
    if let Ok(entries) = fs::read_dir(home_dir) {
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else { continue };
            if !file_type.is_dir() {
                continue;
            }
            let uname = entry.file_name().to_string_lossy().to_string();
            if is_excluded_user(&uname) || uname == "lost+found" {
                continue;
            }
            if preserved.usernames.iter().any(|n| *n == uname) {
                continue;
            }

            let (owner_uid, owner_gid) = fs::metadata(entry.path())
                .map(|meta| (meta.uid(), meta.gid()))
                .unwrap_or((0, 0));
            let uid = if owner_uid >= 1000 && owner_uid < 65534 { owner_uid } else { 0 };
            let gid = if owner_gid >= 1000 && owner_gid < 65534 { owner_gid } else { 0 };

            log(&format!(
                "Discovered existing user home directory in @home: /home/{} (owner {}:{})",
                uname, owner_uid, owner_gid
            ));
            preserved.usernames.push(uname.clone());
            preserved.users.push(AccountRecord {
                name: uname.clone(),
                uid,
                gid,
                gecos: uname.clone(),
                home: format!("/home/{}", uname),
                shell: "/bin/bash".to_string(),
            });
        }
    }

    preserved
}

fn set_mode(path: &str, mode: u32) {
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
}

fn gshadow_line(name: &str, groups: &BTreeMap<String, GroupRecord>, fallback: Option<&String>) -> String {
    let members = groups
        .get(name)
        .map(|group| group.members.iter().cloned().collect::<Vec<_>>().join(","))
        .unwrap_or_default();
    let password = fallback
        .and_then(|line| line.split(':').nth(1))
        .unwrap_or("!");
    let admins = fallback
        .and_then(|line| line.split(':').nth(2))
        .unwrap_or("");
    format!("{}:{}:{}:{}", name, password, admins, members)
}

/// Rebuilds the account databases of `new_root` from the base image plus the
/// accounts collected from the old system and @home.
///
/// The base image is merged instead of appended to: duplicate names and GIDs,
/// stale live-session memberships and repeated usernames are removed, so the
/// resulting files are valid for `getpwnam()`/`getgrnam()` and for the input
/// device groups the desktop session needs.
fn restore_accounts<L>(
    preserved: &PreservedAccounts,
    old_root: &str,
    new_root: &str,
    home_dir: &str,
    log: &L,
) -> Result<(), String>
where
    L: Fn(&str) + Send + Sync + 'static,
{
    let etc = format!("{}/etc", new_root);
    fs::create_dir_all(&etc).map_err(|e| format!("No se pudo crear {}: {}", etc, e))?;

    let base_passwd = fs::read_to_string(format!("{}/passwd", etc)).unwrap_or_default();
    let base_shadow = fs::read_to_string(format!("{}/shadow", etc)).unwrap_or_default();
    let base_group = fs::read_to_string(format!("{}/group", etc)).unwrap_or_default();
    let base_gshadow = fs::read_to_string(format!("{}/gshadow", etc)).unwrap_or_default();

    let base_users = parse_passwd(&base_passwd);
    let base_order: Vec<String> = base_users.iter().map(|user| user.name.clone()).collect();
    let base_group_records = parse_groups(&base_group);
    let base_group_order: Vec<String> = base_group_records.iter().map(|group| group.name.clone()).collect();
    let base_shadow_map = parse_name_map(&base_shadow);
    let base_gshadow_map = parse_name_map(&base_gshadow);

    let mut users_by_name: BTreeMap<String, AccountRecord> = BTreeMap::new();
    for user in base_users {
        if !is_excluded_user(&user.name) {
            users_by_name.insert(user.name.clone(), user);
        }
    }

    let mut used_uids: HashSet<u32> = users_by_name.values().map(|user| user.uid).collect();
    for user in &preserved.users {
        let mut user = user.clone();
        if user.name.is_empty() {
            continue;
        }
        // Never reuse a UID that another account still holds.
        let same_account = users_by_name
            .get(&user.name)
            .map(|existing| existing.uid == user.uid)
            .unwrap_or(false);
        if user.uid < 1000
            || user.uid >= 65534
            || (!same_account && used_uids.contains(&user.uid))
        {
            user.uid = next_free_uid(&used_uids, 1000);
            log(&format!(
                "Assigned free UID {} to preserved user '{}'",
                user.uid, user.name
            ));
        }
        used_uids.insert(user.uid);
        users_by_name.insert(user.name.clone(), user);
    }

    // Merge groups by name. A base group keeps its GID; a preserved group
    // whose GID is already taken gets a free one instead of creating a
    // duplicate GID.
    let mut groups = merge_group_records(base_group_records);
    groups.retain(|name, _| !is_excluded_group(name));
    for group in groups.values_mut() {
        group.members.retain(|member| !is_excluded_member(member));
    }

    let mut used_gids: HashMap<u32, String> = groups
        .iter()
        .map(|(name, group)| (group.gid, name.clone()))
        .collect();

    for group in &preserved.groups {
        if is_excluded_group(&group.name) {
            continue;
        }
        if let Some(existing) = groups.get_mut(&group.name) {
            existing.members.extend(group.members.iter().cloned());
            continue;
        }
        let mut group = group.clone();
        group.members.retain(|member| !is_excluded_member(member));
        if used_gids.contains_key(&group.gid) {
            group.gid = next_free_gid(&used_gids, 1000);
            log(&format!(
                "Group '{}' remapped to free GID {}",
                group.name, group.gid
            ));
        }
        used_gids.insert(group.gid, group.name.clone());
        groups.insert(group.name.clone(), group);
    }

    // Keep each user's primary GID, creating the private group at that GID
    // when needed. Home ownership and file permissions depend on it.
    for uname in &preserved.usernames {
        let Some(user) = users_by_name.get_mut(uname) else { continue };
        let mut gid = user.gid;

        let group_at_gid = groups
            .values()
            .find(|group| group.gid == gid)
            .map(|group| group.name.clone());
        if gid < 1000 || gid >= 65534 || group_at_gid.is_none() {
            if gid >= 1000 && gid < 65534 && !used_gids.contains_key(&gid) {
                // The old primary GID is free: use it for the user's group.
                if let Some(group) = groups.get_mut(uname) {
                    used_gids.remove(&group.gid);
                    group.gid = gid;
                    used_gids.insert(gid, uname.clone());
                } else {
                    groups.insert(
                        uname.clone(),
                        GroupRecord {
                            name: uname.clone(),
                            passwd: "x".to_string(),
                            gid,
                            members: BTreeSet::new(),
                        },
                    );
                    used_gids.insert(gid, uname.clone());
                }
            } else if let Some(group) = groups.get(uname) {
                gid = group.gid;
            } else {
                gid = next_free_gid(&used_gids, 1000);
                groups.insert(
                    uname.clone(),
                    GroupRecord {
                        name: uname.clone(),
                        passwd: "x".to_string(),
                        gid,
                        members: BTreeSet::new(),
                    },
                );
                used_gids.insert(gid, uname.clone());
                log(&format!(
                    "Created private group '{}' with GID {}",
                    uname, gid
                ));
            }
        }

        user.gid = gid;
    }

    // Re-apply the old supplementary memberships, then the desktop groups
    // every restored user needs. Everything is merged in sets, so a user is
    // listed exactly once.
    for (uname, memberships) in &preserved.user_groups {
        if !users_by_name.contains_key(uname) {
            continue;
        }
        let primary_gid = users_by_name[uname].gid;
        for group_name in memberships {
            if is_excluded_group(group_name) {
                continue;
            }
            if let Some(group) = groups.get_mut(group_name) {
                if group.gid == primary_gid {
                    group.members.remove(uname);
                } else {
                    group.members.insert(uname.clone());
                }
            }
        }
    }

    for uname in &preserved.usernames {
        let Some(user) = users_by_name.get(uname) else { continue };
        for group_name in BASE_ADMIN_GROUPS {
            if !groups.contains_key(*group_name) {
                let gid = next_free_gid(&used_gids, 900);
                groups.insert(
                    (*group_name).to_string(),
                    GroupRecord {
                        name: (*group_name).to_string(),
                        passwd: "x".to_string(),
                        gid,
                        members: BTreeSet::new(),
                    },
                );
                used_gids.insert(gid, (*group_name).to_string());
                log(&format!(
                    "Created missing desktop group '{}' with GID {}",
                    group_name, gid
                ));
            }
            let group = groups.get_mut(*group_name).unwrap();
            if group.gid == user.gid {
                group.members.remove(uname);
            } else {
                group.members.insert(uname.clone());
            }
        }
    }

    // Drop members that no longer have an account (live session leftovers)
    // and explicit primary-group memberships, which are implicit.
    for group in groups.values_mut() {
        let gid = group.gid;
        group.members.retain(|member| {
            users_by_name
                .get(member)
                .map(|user| user.gid != gid)
                .unwrap_or(false)
        });
    }

    // /etc/passwd: base order first, then restored users by UID.
    let mut ordered_users: Vec<String> = Vec::new();
    let mut seen_users: HashSet<String> = HashSet::new();
    for name in &base_order {
        if users_by_name.contains_key(name) && seen_users.insert(name.clone()) {
            ordered_users.push(name.clone());
        }
    }
    let mut restored: Vec<&AccountRecord> = preserved
        .usernames
        .iter()
        .filter_map(|name| users_by_name.get(name))
        .collect();
    restored.sort_by_key(|user| user.uid);
    for user in restored {
        if seen_users.insert(user.name.clone()) {
            ordered_users.push(user.name.clone());
        }
    }
    let mut remaining: Vec<&AccountRecord> = users_by_name
        .values()
        .filter(|user| !seen_users.contains(&user.name))
        .collect();
    remaining.sort_by_key(|user| user.uid);
    for user in remaining {
        ordered_users.push(user.name.clone());
    }

    let mut passwd_out = String::new();
    let mut shadow_out = String::new();
    let mut missing_shadow: Vec<String> = Vec::new();
    for name in &ordered_users {
        let Some(user) = users_by_name.get(name) else { continue };
        passwd_out.push_str(&passwd_line(user));
        passwd_out.push('\n');

        let line = preserved
            .shadow
            .get(name)
            .or_else(|| base_shadow_map.get(name))
            .cloned();
        match line {
            Some(line) => {
                shadow_out.push_str(&line);
                shadow_out.push('\n');
            }
            None => {
                // No hash was preserved: lock the account instead of leaving
                // it with an empty password.
                shadow_out.push_str(&format!("{}:!:19700:0:99999:7:::", name));
                shadow_out.push('\n');
                missing_shadow.push(name.clone());
            }
        }
    }
    if !missing_shadow.is_empty() {
        log(&format!(
            "WARNING: no shadow entry preserved for {:?}; the accounts were locked. Set a new password with passwd.",
            missing_shadow
        ));
    }

    // /etc/group: base order first, then restored groups by GID.
    let mut ordered_groups: Vec<String> = Vec::new();
    let mut seen_groups: HashSet<String> = HashSet::new();
    for name in &base_group_order {
        if groups.contains_key(name) && seen_groups.insert(name.clone()) {
            ordered_groups.push(name.clone());
        }
    }
    let mut restored_groups: Vec<&GroupRecord> = groups
        .values()
        .filter(|group| !seen_groups.contains(&group.name))
        .collect();
    restored_groups.sort_by_key(|group| group.gid);
    for group in restored_groups {
        if seen_groups.insert(group.name.clone()) {
            ordered_groups.push(group.name.clone());
        }
    }

    let mut group_out = String::new();
    let mut gshadow_out = String::new();
    for name in &ordered_groups {
        let Some(group) = groups.get(name) else { continue };
        let passwd = if group.passwd.is_empty() { "x" } else { &group.passwd };
        let members = group.members.iter().cloned().collect::<Vec<_>>().join(",");
        group_out.push_str(&format!("{}:{}:{}:{}\n", group.name, passwd, group.gid, members));

        let fallback = preserved
            .gshadow
            .get(name)
            .or_else(|| base_gshadow_map.get(name));
        gshadow_out.push_str(&gshadow_line(name, &groups, fallback));
        gshadow_out.push('\n');
    }

    fs::write(format!("{}/passwd", etc), passwd_out)
        .map_err(|e| format!("No se pudo escribir /etc/passwd: {}", e))?;
    fs::write(format!("{}/shadow", etc), shadow_out)
        .map_err(|e| format!("No se pudo escribir /etc/shadow: {}", e))?;
    fs::write(format!("{}/group", etc), group_out)
        .map_err(|e| format!("No se pudo escribir /etc/group: {}", e))?;
    fs::write(format!("{}/gshadow", etc), gshadow_out)
        .map_err(|e| format!("No se pudo escribir /etc/gshadow: {}", e))?;

    set_mode(&format!("{}/passwd", etc), 0o644);
    set_mode(&format!("{}/group", etc), 0o644);
    set_mode(&format!("{}/shadow", etc), 0o600);
    set_mode(&format!("{}/gshadow", etc), 0o600);

    // Preserve the AccountsService record and avatar so GDM, the lock screen
    // and Settings show the same user instead of a blank/default account.
    let old_accounts = format!("{}/var/lib/AccountsService", old_root);
    if Path::new(&old_accounts).exists() {
        let new_accounts = format!("{}/var/lib/AccountsService", new_root);
        let _ = fs::create_dir_all(format!("{}/users", new_accounts));
        let _ = fs::create_dir_all(format!("{}/icons", new_accounts));
        for uname in &preserved.usernames {
            let src_user = format!("{}/users/{}", old_accounts, uname);
            if Path::new(&src_user).exists() {
                let _ = fs::copy(&src_user, format!("{}/users/{}", new_accounts, uname));
            }
            let src_icon = format!("{}/icons/{}", old_accounts, uname);
            if Path::new(&src_icon).exists() {
                let _ = fs::copy(&src_icon, format!("{}/icons/{}", new_accounts, uname));
            }
            for ext in ["png", "jpg", "jpeg"] {
                let src = format!("{}/icons/{}.{}", old_accounts, uname, ext);
                if Path::new(&src).exists() {
                    let _ = fs::copy(&src, format!("{}/icons/{}.{}", new_accounts, uname, ext));
                }
            }
        }
    }

    // The accounts keep their old UID/GID, so a mismatch here means the home
    // files were already owned by another identity. Make them readable again.
    for uname in &preserved.usernames {
        let Some(user) = users_by_name.get(uname) else { continue };
        let home = format!("{}/{}", home_dir, uname);
        if let Ok(meta) = fs::metadata(&home) {
            if meta.uid() != user.uid || meta.gid() != user.gid {
                log(&format!(
                    "Fixing ownership of {} ({}:{} -> {}:{})",
                    home, meta.uid(), meta.gid(), user.uid, user.gid
                ));
                let _ = std::process::Command::new("chown")
                    .arg("-R")
                    .arg(format!("{}:{}", user.uid, user.gid))
                    .arg(&home)
                    .status();
            }
        }
    }

    log(&format!(
        "Restored {} user account(s): {:?}",
        preserved.usernames.len(),
        preserved.usernames
    ));
    Ok(())
}

pub(crate) fn run_restoration<F, L>(
    target: &BtrfsTarget,
    mode: RecoveryMode,
    apps_to_reinstall: Vec<InstalledApp>,
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

        if !apps_to_reinstall.is_empty() {
            progress(0.85, &format!("[DEMO] Simulating reinstallation of {} user applications and drivers...", apps_to_reinstall.len()));
            log(&format!("[DEMO] Preserved applications to reinstall: {:?}", apps_to_reinstall.iter().map(|a| &a.id).collect::<Vec<_>>()));
            thread::sleep(std::time::Duration::from_millis(600));
        }

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

    // 2. Collect existing user accounts from the old @ subvolume and @home.
    // The account databases are rebuilt after unpacking (see
    // `restore_accounts`) instead of appended to, so duplicate names/GIDs and
    // stale live-session memberships cannot survive a restore.
    progress(0.20, "Preserving user accounts and identities...");
    log("Collecting user accounts from old /etc and @home (UID >= 1000)...");
    let old_root = format!("{}/@", btrfs_mnt);
    let home_dir = format!("{}/@home", btrfs_mnt);
    let preserved = collect_accounts(&old_root, &home_dir, &log);
    log(&format!(
        "Preserved {} real user account(s): {:?}",
        preserved.usernames.len(),
        preserved.usernames
    ));

    // Also check for existing /etc/crypttab to preserve LUKS configuration
    let mut preserved_crypttab: Option<String> = None;
    let old_crypttab = format!("{}/etc/crypttab", old_root);
    if let Ok(c) = exec_cmd(&format!("cat {} 2>/dev/null || true", old_crypttab)) {
        if !c.trim().is_empty() {
            log("Preserved existing /etc/crypttab disk encryption configuration.");
            preserved_crypttab = Some(c);
        }
    }

    // Preserve timezone, locale, language and keyboard layout
    let mut preserved_settings = PreservedSettings::default();
    if Path::new(&old_root).exists() {
        let old_localtime = format!("{}/etc/localtime", old_root);
        if let Ok(link) = fs::read_link(&old_localtime) {
            log(&format!("Preserved timezone symlink: {}", link.display()));
            preserved_settings.localtime_link = Some(link.to_string_lossy().to_string());
        }
        let old_timezone = format!("{}/etc/timezone", old_root);
        if let Ok(tz) = fs::read_to_string(&old_timezone) {
            let tz_trimmed = tz.trim();
            if !tz_trimmed.is_empty() {
                log(&format!("Preserved timezone: {}", tz_trimmed));
                preserved_settings.timezone = Some(tz_trimmed.to_string());
            }
        }
        let old_locale_conf = format!("{}/etc/locale.conf", old_root);
        if let Ok(lc) = fs::read_to_string(&old_locale_conf) {
            if !lc.trim().is_empty() {
                log("Preserved /etc/locale.conf language configuration.");
                preserved_settings.locale_conf = Some(lc);
            }
        }
        let old_locale_gen = format!("{}/etc/locale.gen", old_root);
        if let Ok(lg) = fs::read_to_string(&old_locale_gen) {
            if !lg.trim().is_empty() {
                log("Preserved /etc/locale.gen configuration.");
                preserved_settings.locale_gen = Some(lg);
            }
        }
        let old_def_locale = format!("{}/etc/default/locale", old_root);
        if let Ok(dl) = fs::read_to_string(&old_def_locale) {
            if !dl.trim().is_empty() {
                log("Preserved /etc/default/locale language configuration.");
                preserved_settings.default_locale = Some(dl);
            }
        }
        let old_vconsole = format!("{}/etc/vconsole.conf", old_root);
        if let Ok(vc) = fs::read_to_string(&old_vconsole) {
            if !vc.trim().is_empty() {
                log("Preserved /etc/vconsole.conf keyboard layout.");
                preserved_settings.vconsole_conf = Some(vc);
            }
        }
        let old_def_kb = format!("{}/etc/default/keyboard", old_root);
        if let Ok(dk) = fs::read_to_string(&old_def_kb) {
            if !dk.trim().is_empty() {
                log("Preserved /etc/default/keyboard layout.");
                preserved_settings.default_keyboard = Some(dk);
            }
        }
        let old_x11_kb = format!("{}/etc/X11/xorg.conf.d/00-keyboard.conf", old_root);
        if let Ok(xk) = fs::read_to_string(&old_x11_kb) {
            if !xk.trim().is_empty() {
                log("Preserved X11 keyboard configuration.");
                preserved_settings.x11_keyboard = Some(xk);
            }
        }
    }

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
    log("Cleaning nested mounts and removing old root (@) subvolume...");
    let _ = exec_cmd(&format!("umount -R {}/@ 2>/dev/null || true", btrfs_mnt));
    let _ = exec_cmd(&format!("umount -l {}/@ 2>/dev/null || true", btrfs_mnt));
    let _ = exec_cmd(&format!("btrfs subvolume delete -c {}/@ 2>/dev/null || btrfs subvolume delete {}/@ 2>/dev/null || rm -rf {}/@ 2>/dev/null || true", btrfs_mnt, btrfs_mnt, btrfs_mnt));
    if Path::new(&format!("{}/@", btrfs_mnt)).exists() {
        let _ = exec_cmd(&format!("btrfs subvolume delete -c {}/@ 2>/dev/null || btrfs subvolume delete {}/@ 2>/dev/null || true", btrfs_mnt, btrfs_mnt));
    }
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

    // 6. Rebuild the account databases from the clean image plus the accounts
    // collected from the old system. Merging (instead of appending) removes
    // duplicate names/GIDs and stale live-session memberships, which otherwise
    // leave users without a valid primary group and break input device access,
    // GDM sessions and the lock screen after a restore.
    progress(0.85, "Re-injecting user credentials and settings...");
    log("Restoring user accounts into clean /etc...");
    let new_root = format!("{}/@", btrfs_mnt);

    restore_accounts(&preserved, &old_root, &new_root, &home_dir, &log)?;

    // Keep sudo usable even if the image's /etc/sudoers does not reference the
    // wheel/sudo groups.
    let sudoers_d = format!("{}/etc/sudoers.d", new_root);
    let _ = fs::create_dir_all(&sudoers_d);
    let _ = set_mode(&sudoers_d, 0o750);
    let wheel_rule = format!("{}/10-admin-wheel", sudoers_d);
    let _ = fs::write(&wheel_rule, "%wheel ALL=(ALL:ALL) ALL\n%sudo ALL=(ALL:ALL) ALL\n");
    let _ = set_mode(&wheel_rule, 0o440);
    for uname in &preserved.usernames {
        let user_rule = format!("{}/pulsaros-user-{}", sudoers_d, uname);
        let _ = fs::write(&user_rule, format!("{} ALL=(ALL:ALL) ALL\n", uname));
        let _ = set_mode(&user_rule, 0o440);
    }
    log("Rebuilt /etc/passwd, /etc/shadow, /etc/group and /etc/gshadow.");


    // Restore timezone, locale, language and keyboard configuration
    log("Restoring system timezone, language, and keyboard configuration...");
    if let Some(ref link) = preserved_settings.localtime_link {
        let dest = format!("{}/etc/localtime", new_root);
        let _ = fs::remove_file(&dest);
        let _ = exec_cmd(&format!("ln -s {} {}", link, dest));
        log(&format!("Restored timezone symlink: -> {}", link));
    } else if let Some(ref tz) = preserved_settings.timezone {
        let _ = fs::write(format!("{}/etc/timezone", new_root), tz);
        let tz_trimmed = tz.trim();
        let zoneinfo = format!("/usr/share/zoneinfo/{}", tz_trimmed);
        let dest = format!("{}/etc/localtime", new_root);
        let _ = fs::remove_file(&dest);
        let _ = exec_cmd(&format!("ln -s {} {}", zoneinfo, dest));
        log(&format!("Restored timezone: {}", tz_trimmed));
    }

    if let Some(ref lc) = preserved_settings.locale_conf {
        let _ = fs::write(format!("{}/etc/locale.conf", new_root), lc);
    }
    if let Some(ref lg) = preserved_settings.locale_gen {
        let _ = fs::write(format!("{}/etc/locale.gen", new_root), lg);
        let _ = exec_cmd(&format!("chroot {} locale-gen 2>/dev/null || true", new_root));
    }
    if let Some(ref dl) = preserved_settings.default_locale {
        let _ = fs::write(format!("{}/etc/default/locale", new_root), dl);
    }
    if let Some(ref vc) = preserved_settings.vconsole_conf {
        let _ = fs::write(format!("{}/etc/vconsole.conf", new_root), vc);
    }
    if let Some(ref dk) = preserved_settings.default_keyboard {
        let _ = fs::write(format!("{}/etc/default/keyboard", new_root), dk);
    }
    if let Some(ref xk) = preserved_settings.x11_keyboard {
        let x11_dir = format!("{}/etc/X11/xorg.conf.d", new_root);
        let _ = fs::create_dir_all(&x11_dir);
        let _ = fs::write(format!("{}/00-keyboard.conf", x11_dir), xk);
    }

    // Reinstall retained user applications and drivers
    if !apps_to_reinstall.is_empty() {
        progress(0.88, &format!("Reinstalling {} user applications & drivers...", apps_to_reinstall.len()));
        log(&format!("Reinstalling {} preserved user applications and drivers...", apps_to_reinstall.len()));

        let mut native_pkgs: Vec<String> = Vec::new();
        let mut flatpak_apps: Vec<String> = Vec::new();

        for app in &apps_to_reinstall {
            match app.app_type {
                AppType::Application | AppType::Driver => {
                    native_pkgs.push(app.id.clone());
                }
                AppType::Flatpak => {
                    flatpak_apps.push(app.id.clone());
                }
            }
        }

        if !native_pkgs.is_empty() {
            let is_arch = Path::new(&format!("{}/etc/arch-release", new_root)).exists();
            let is_debian = Path::new(&format!("{}/etc/debian_version", new_root)).exists();

            let _ = exec_cmd(&format!("cp -L /etc/resolv.conf {}/etc/resolv.conf 2>/dev/null || true", new_root));

            let pkg_chunk = native_pkgs.join(" ");
            if is_arch {
                log(&format!("Reinstalling Arch packages via pacman: {}", pkg_chunk));
                let _ = exec_cmd_stream(
                    &format!("chroot {} pacman -Sy --noconfirm --needed {}", new_root, pkg_chunk),
                    &log,
                );
            } else if is_debian {
                log(&format!("Reinstalling Debian packages via apt: {}", pkg_chunk));
                let _ = exec_cmd_stream(
                    &format!("chroot {} apt-get update -y && chroot {} apt-get install -y --no-install-recommends {}", new_root, new_root, pkg_chunk),
                    &log,
                );
            }
        }

        if !flatpak_apps.is_empty() {
            for fp in flatpak_apps {
                log(&format!("Restoring Flatpak application: {}", fp));
                let _ = exec_cmd(&format!("chroot {} flatpak install -y --noninteractive flathub {} 2>/dev/null || true", new_root, fp));
            }
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

    // Restore or generate /etc/crypttab if LUKS encrypted
    if let Some(ref crypttab) = preserved_crypttab {
        let crypttab_path = format!("{}/etc/crypttab", new_root);
        let _ = fs::write(&crypttab_path, crypttab);
        log("Restored /etc/crypttab disk encryption configuration.");
    } else if target.part_path.starts_with("/dev/mapper/") {
        let mapper_name = target.part_path.strip_prefix("/dev/mapper/").unwrap_or("pulsar_cryptroot");
        if let Ok(out) = exec_cmd(&format!("cryptsetup status {} 2>/dev/null | grep device: || true", mapper_name)) {
            let backing_dev = out.split_whitespace().nth(1).unwrap_or("");
            if !backing_dev.is_empty() {
                if let Ok(luks_uuid) = exec_cmd(&format!("blkid -s UUID -o value {} 2>/dev/null || true", backing_dev)) {
                    let uuid_trimmed = luks_uuid.trim();
                    if !uuid_trimmed.is_empty() {
                        let crypttab_content = format!("{} UUID={} none luks,discard,initramfs\n", mapper_name, uuid_trimmed);
                        let _ = fs::write(format!("{}/etc/crypttab", new_root), crypttab_content);
                        log(&format!("Generated /etc/crypttab for mapper {} (LUKS UUID: {})", mapper_name, uuid_trimmed));
                    }
                }
            }
        }
    }

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
