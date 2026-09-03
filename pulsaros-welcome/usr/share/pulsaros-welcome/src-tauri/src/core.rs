use std::path::Path;
use std::process::Command;

pub fn is_live_system() -> bool {
    // Only detect genuine live environments (cmdline or actual live mountpoints).
    // Do NOT check USER == "live" because the first boot of an installed system
    // runs under the temporary live user before OOTB user setup.
    if Path::new("/run/archiso/bootmnt").exists()
        || Path::new("/run/archiso/airootfs").exists()
        || Path::new("/run/live/medium").exists()
        || Path::new("/run/live/overlay").exists()
        || Path::new("/lib/live/mount/overlay").exists()
    {
        return true;
    }
    if let Ok(cmdline) = std::fs::read_to_string("/proc/cmdline") {
        if cmdline.contains("boot=live")
            || cmdline.contains("archisobasedir=")
            || cmdline.contains("archisolabel=")
            || cmdline.contains("img_dev=")
            || cmdline.contains("rootfstype=9p")
            || cmdline.contains("live-media")
        {
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

pub const OOTB_SENTINEL: &str = "/etc/pulsar-need-setup";

pub fn is_ootb_pending() -> bool {
    Path::new(OOTB_SENTINEL).exists()
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
