use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

pub struct PreservedUsers {
    pub passwd: Vec<String>,
    pub shadow: Vec<String>,
    pub group: Vec<String>,
    pub gshadow: Vec<String>,
    pub usernames: Vec<String>,
    pub group_memberships: HashMap<String, Vec<String>>,
}

const TEMP_USERS: &[&str] = &["live", "root", "pulsar-live", "archiso", "nobody"];

const ESSENTIAL_GROUPS: &[&str] = &[
    "wheel", "sudo", "video", "audio", "input", "storage", "network", "optical",
    "power", "rfkill", "autologin", "users", "lp", "scanner", "kvm",
];

fn is_temp_user(name: &str) -> bool {
    TEMP_USERS.contains(&name)
}

pub fn preserve_users(root_path: &str) -> PreservedUsers {
    let mut preserved = PreservedUsers {
        passwd: Vec::new(),
        shadow: Vec::new(),
        group: Vec::new(),
        gshadow: Vec::new(),
        usernames: Vec::new(),
        group_memberships: HashMap::new(),
    };

    let passwd_path = format!("{}/etc/passwd", root_path);
    if let Ok(file) = fs::File::open(&passwd_path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let uname = parts[0].to_string();
                if is_temp_user(&uname) {
                    continue;
                }
                if let Ok(uid) = parts[2].parse::<u32>() {
                    if (1000..65534).contains(&uid) {
                        preserved.usernames.push(uname);
                        preserved.passwd.push(line);
                    }
                }
            }
        }
    }

    let shadow_path = format!("{}/etc/shadow", root_path);
    if let Ok(file) = fs::File::open(&shadow_path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let uname = line.split(':').next().unwrap_or_default();
            if preserved.usernames.iter().any(|u| u == uname) {
                preserved.shadow.push(line);
            }
        }
    }

    let group_path = format!("{}/etc/group", root_path);
    if let Ok(file) = fs::File::open(&group_path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                let gname = parts[0].to_string();
                if is_temp_user(&gname) {
                    continue;
                }
                for m in parts[3].split(',') {
                    let m_trim = m.trim().to_string();
                    if !m_trim.is_empty() && !is_temp_user(&m_trim) {
                        preserved
                            .group_memberships
                            .entry(m_trim)
                            .or_default()
                            .push(gname.clone());
                    }
                }
            }
            if parts.len() >= 3 {
                let gname = parts[0].to_string();
                if is_temp_user(&gname) {
                    continue;
                }
                if let Ok(gid) = parts[2].parse::<u32>() {
                    if (1000..65534).contains(&gid) {
                        preserved.group.push(line);
                    }
                }
            }
        }
    }

    let gshadow_path = format!("{}/etc/gshadow", root_path);
    if let Ok(file) = fs::File::open(&gshadow_path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let gname = line.split(':').next().unwrap_or_default();
            if !is_temp_user(gname) {
                preserved.gshadow.push(line);
            }
        }
    }

    preserved
}

pub fn discover_home_users(btrfs_mnt: &str, preserved: &mut PreservedUsers) {
    let home_dir = format!("{}/@home", btrfs_mnt);
    if let Ok(entries) = fs::read_dir(&home_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let uname = entry.file_name().to_string_lossy().to_string();
                    if is_temp_user(&uname) || uname == "lost+found" {
                        continue;
                    }
                    if !preserved.usernames.contains(&uname) {
                        let home = format!("/home/{}", uname);
                        preserved.passwd.push(format!(
                            "{}:x:1000:1000::{home}:/bin/bash",
                            uname
                        ));
                        preserved.shadow.push(format!("{}:!!:19700:0:99999:7:::", uname));
                        preserved.group.push(format!("{}:x:1000:", uname));
                        preserved.usernames.push(uname);
                    }
                }
            }
        }
    }
}

pub fn restore_users(
    root_path: &str,
    users: &PreservedUsers,
    log_fn: &dyn Fn(String),
) -> Result<(), String> {
    if users.passwd.is_empty() {
        return Ok(());
    }

    let _ = exec_cmd(&format!(
        "sed -i '/^live:/d' {}/etc/passwd {}/etc/shadow {}/etc/group {}/etc/gshadow 2>/dev/null || true",
        root_path, root_path, root_path, root_path
    ));

    for uname in &users.usernames {
        let _ = exec_cmd(&format!(
            "sed -i '/^{}:/d' {}/etc/passwd 2>/dev/null || true",
            uname, root_path
        ));
    }
    write_temp_file("/tmp/pulsar_preserved_passwd", &users.passwd)?;
    let _ = exec_cmd(&format!(
        "cat /tmp/pulsar_preserved_passwd >> {}/etc/passwd",
        root_path
    ));

    for uname in &users.usernames {
        let _ = exec_cmd(&format!(
            "sed -i '/^{}:/d' {}/etc/shadow 2>/dev/null || true",
            uname, root_path
        ));
    }
    let mut shadow_lines = users.shadow.clone();
    for uname in &users.usernames {
        if !users.shadow.iter().any(|s| s.starts_with(&format!("{}:", uname))) {
            log_fn(format!("Adding fallback shadow entry for user: {}", uname));
            shadow_lines.push(format!("{}::19700:0:99999:7:::", uname));
        }
    }
    write_temp_file("/tmp/pulsar_preserved_shadow", &shadow_lines)?;
    let _ = exec_cmd(&format!(
        "cat /tmp/pulsar_preserved_shadow >> {}/etc/shadow",
        root_path
    ));

    write_temp_file("/tmp/pulsar_preserved_group", &users.group)?;
    let _ = exec_cmd(&format!(
        "cat /tmp/pulsar_preserved_group >> {}/etc/group",
        root_path
    ));

    write_temp_file("/tmp/pulsar_preserved_gshadow", &users.gshadow)?;
    let _ = exec_cmd(&format!(
        "cat /tmp/pulsar_preserved_gshadow >> {}/etc/gshadow",
        root_path
    ));

    let sudoers_d = format!("{}/etc/sudoers.d", root_path);
    let _ = fs::create_dir_all(&sudoers_d);
    let _ = exec_cmd(&format!("chmod 750 {}", sudoers_d));

    let wheel_rule = format!("{}/10-admin-wheel", sudoers_d);
    fs::write(&wheel_rule, "%wheel ALL=(ALL:ALL) ALL\n%sudo ALL=(ALL:ALL) ALL\n")
        .map_err(|e| format!("Failed to write {}: {}", wheel_rule, e))?;
    let _ = exec_cmd(&format!("chmod 0440 {}", wheel_rule));

    for uname in &users.usernames {
        let mut target_groups: Vec<String> = ESSENTIAL_GROUPS.iter().map(|s| s.to_string()).collect();
        if let Some(custom_grps) = users.group_memberships.get(uname) {
            for cg in custom_grps {
                if !target_groups.contains(cg) {
                    target_groups.push(cg.clone());
                }
            }
        }

        for grp in &target_groups {
            let _ = exec_cmd(&format!(
                "grep -q '^{}:' {}/etc/group || echo '{}:x:999:' >> {}/etc/group",
                grp, root_path, grp, root_path
            ));
            let _ = exec_cmd(&format!(
                "sed -i -E 's/^({}:[^:]*:[^:]*:)(.*)$/\\1\\2,{}/' {}/etc/group 2>/dev/null || true",
                grp, uname, root_path
            ));
            let _ = exec_cmd(&format!(
                "sed -i -E 's/,+/,/g; s/:,/:/g; s/,$//' {}/etc/group 2>/dev/null || true",
                root_path
            ));
        }

        let user_rule = format!("{}/pulsaros-user-{}", sudoers_d, uname);
        fs::write(&user_rule, format!("{} ALL=(ALL:ALL) ALL\n", uname))
            .map_err(|e| format!("Failed to write {}: {}", user_rule, e))?;
        let _ = exec_cmd(&format!("chmod 0440 {}", user_rule));
        log_fn(format!(
            "Granted full sudo privileges to user '{}' via sudoers and wheel group",
            uname
        ));
    }

    let _ = exec_cmd("rm -f /tmp/pulsar_preserved_passwd /tmp/pulsar_preserved_shadow /tmp/pulsar_preserved_group /tmp/pulsar_preserved_gshadow");

    Ok(())
}

fn write_temp_file(path: &str, lines: &[String]) -> Result<(), String> {
    let mut f = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("Failed to open {}: {}", path, e))?;
    for line in lines {
        writeln!(f, "{}", line)
            .map_err(|e| format!("Failed to write to {}: {}", path, e))?;
    }
    Ok(())
}

fn exec_cmd(cmd: &str) -> Result<String, String> {
    let out = std::process::Command::new("sudo")
        .args(["-n", "sh", "-c", cmd])
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
