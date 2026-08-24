use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

pub struct PreservedUsers {
    pub passwd: Vec<String>,
    pub shadow: Vec<String>,
    pub group: Vec<String>,
    pub gshadow: Vec<String>,
}

pub fn preserve_users(root_path: &str) -> PreservedUsers {
    let mut preserved = PreservedUsers {
        passwd: Vec::new(),
        shadow: Vec::new(),
        group: Vec::new(),
        gshadow: Vec::new(),
    };

    let passwd_path = format!("{}/etc/passwd", root_path);
    if let Ok(file) = fs::File::open(&passwd_path) {
        for line in BufReader::new(file).lines().flatten() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(uid) = parts[2].parse::<u32>() {
                    if uid >= 1000 && uid < 65534 {
                        preserved.passwd.push(line);
                    }
                }
            }
        }
    }

    let shadow_path = format!("{}/etc/shadow", root_path);
    if let Ok(file) = fs::File::open(&shadow_path) {
        for line in BufReader::new(file).lines().flatten() {
            let uname = line.split(':').next().unwrap_or_default();
            if preserved
                .passwd
                .iter()
                .any(|p| p.starts_with(&format!("{}:", uname)))
            {
                preserved.shadow.push(line);
            }
        }
    }

    let group_path = format!("{}/etc/group", root_path);
    if let Ok(file) = fs::File::open(&group_path) {
        for line in BufReader::new(file).lines().flatten() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(gid) = parts[2].parse::<u32>() {
                    if gid >= 1000 && gid < 65534 {
                        preserved.group.push(line);
                    }
                }
            }
        }
    }

    let gshadow_path = format!("{}/etc/gshadow", root_path);
    if let Ok(file) = fs::File::open(&gshadow_path) {
        for line in BufReader::new(file).lines().flatten() {
            let gname = line.split(':').next().unwrap_or_default();
            if preserved
                .group
                .iter()
                .any(|g| g.starts_with(&format!("{}:", gname)))
            {
                preserved.gshadow.push(line);
            }
        }
    }

    preserved
}

pub fn restore_users(root_path: &str, users: &PreservedUsers) -> Result<(), String> {
    if users.passwd.is_empty() {
        return Ok(());
    }

    append_lines(&format!("{}/etc/passwd", root_path), &users.passwd)?;
    append_lines(&format!("{}/etc/shadow", root_path), &users.shadow)?;
    append_lines(&format!("{}/etc/group", root_path), &users.group)?;
    append_lines(&format!("{}/etc/gshadow", root_path), &users.gshadow)?;

    Ok(())
}

fn append_lines(path: &str, lines: &[String]) -> Result<(), String> {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Failed to open {}: {}", path, e))?;
    for line in lines {
        writeln!(f, "{}", line)
            .map_err(|e| format!("Failed to write to {}: {}", path, e))?;
    }
    Ok(())
}
