use std::path::Path;
use std::process::Command;

pub fn is_live_system() -> bool {
    if Path::new("/lib/live/mount").exists() {
        return true;
    }
    if let Ok(user) = std::env::var("USER") {
        if user == "live" {
            return true;
        }
    }
    if let Ok(cmdline) = std::fs::read_to_string("/proc/cmdline") {
        if cmdline.contains("boot=live") || cmdline.contains("rootfstype=9p") {
            return true;
        }
    }
    return false;
}

pub fn is_arch_system() -> bool {
    if let Ok(rel) = std::fs::read_to_string("/etc/os-release") {
        let lower = rel.to_lowercase();
        if lower.contains("id_like=arch") || lower.contains("id=arch") {
            return true;
        }
        if lower.contains("id_like=debian") || lower.contains("id=debian") || lower.contains("id=ubuntu") {
            return false;
        }
    }
    if Command::new("which").arg("pacman").output().map(|o| o.status.success()).unwrap_or(false) {
        return true;
    }
    if Command::new("which").arg("apt-get").output().map(|o| o.status.success()).unwrap_or(false) {
        return false;
    }
    return false;
}

pub fn sentinel_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    format!("{}/.config/pulsaros-welcome.done", home)
}

pub fn check_sentinel() -> bool {
    Path::new(&sentinel_path()).exists()
}

pub fn write_sentinel() -> Result<(), String> {
    let path = sentinel_path();
    if let Some(parent) = Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, "").map_err(|e| e.to_string())
}
