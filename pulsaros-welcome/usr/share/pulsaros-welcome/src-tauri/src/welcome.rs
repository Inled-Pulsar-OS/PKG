use std::path::Path;
use std::process::Command;

const BLUR_MY_SHELL_UUID: &str = "blur-my-shell@aunetx";
const LIQUID_GLASS_UUID: &str = "liquid-glass@thinkingcoding1231.gmail.com";

// ── System Detection ──

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
    false
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
    false
}

// ── Sentinel ──

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

// ── Resolution ──

#[derive(Clone, serde::Serialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    pub active: bool,
}

pub fn get_resolutions() -> Vec<Resolution> {
    let output = Command::new("xrandr")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut resolutions = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        if !parts[0].contains('x') {
            continue;
        }
        let dims: Vec<&str> = parts[0].split('x').collect();
        if dims.len() != 2 {
            continue;
        }
        let w: u32 = match dims[0].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let h: u32 = match dims[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let active = parts[1..].iter().any(|s| *s == "*");
        if !resolutions.iter().any(|r: &Resolution| r.width == w && r.height == h) {
            resolutions.push(Resolution {
                width: w,
                height: h,
                active,
            });
        }
    }
    resolutions
}

// ── App Launcher ──

pub fn launch_app_with_fallback(primary: &str, fallback: Option<&str>) -> Result<(), String> {
    if which(primary) {
        return Command::new(primary)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to launch {}: {}", primary, e));
    }
    if let Some(fb) = fallback {
        if which(fb) {
            return Command::new(fb)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Failed to launch {}: {}", fb, e));
        }
    }
    Err(format!("{} not found in PATH", primary))
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── Display Settings ──

pub fn launch_display_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("display")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn launch_wifi_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("wifi")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn launch_bluetooth_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("bluetooth")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── ADB ──

pub fn check_adb_devices() -> Result<String, String> {
    let out = Command::new("adb")
        .arg("devices")
        .output()
        .map_err(|e| format!("Error running adb: {}", e))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// ── GSettings Helpers ──

fn gsettings_get(schema: &str, key: &str) -> Result<String, String> {
    let out = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn gsettings_set(schema: &str, key: &str, value: &str) -> Result<(), String> {
    Command::new("gsettings")
        .args(["set", schema, key, value])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn get_enabled_extensions() -> Vec<String> {
    let raw = gsettings_get("org.gnome.shell", "enabled-extensions").unwrap_or_default();
    let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn set_enabled_extensions(exts: &[String]) -> Result<(), String> {
    let formatted: Vec<String> = exts.iter().map(|e| format!("'{}'", e)).collect();
    let value = format!("[{}]", formatted.join(", "));
    gsettings_set("org.gnome.shell", "enabled-extensions", &value)
}

fn set_extension_state(uuid: &str, enable: bool) -> Result<(), String> {
    let mut exts = get_enabled_extensions();
    if enable {
        if !exts.contains(&uuid.to_string()) {
            exts.push(uuid.to_string());
        }
    } else {
        exts.retain(|e| e != uuid);
    }
    set_enabled_extensions(&exts)
}

// ── Desktop Effects ──

pub fn get_effects_state() -> Result<bool, String> {
    let exts = get_enabled_extensions();
    Ok(exts.contains(&LIQUID_GLASS_UUID.to_string()))
}

pub fn set_effects(use_liquid_glass: bool) -> Result<(), String> {
    if use_liquid_glass {
        set_extension_state(BLUR_MY_SHELL_UUID, false)?;
        set_extension_state(LIQUID_GLASS_UUID, true)?;
        apply_glass_settings()?;
    } else {
        set_extension_state(LIQUID_GLASS_UUID, false)?;
        set_extension_state(BLUR_MY_SHELL_UUID, true)?;
        apply_blur_settings()?;
    }
    Ok(())
}

fn apply_blur_settings() -> Result<(), String> {
    let schema = "org.gnome.shell.extensions.dash-to-dock";
    gsettings_set(schema, "background-opacity", "0.8")?;
    gsettings_set(schema, "custom-theme-shrink", "false")?;
    gsettings_set(schema, "show-show-apps-button", "false")?;
    gsettings_set(schema, "height-fraction", "0.9")?;
    gsettings_set(schema, "apply-custom-theme", "true")?;
    gsettings_set(schema, "transparency-mode", "'FIXED'")?;
    gsettings_set(schema, "customize-alphas", "false")?;
    Ok(())
}

fn apply_glass_settings() -> Result<(), String> {
    let dock = "org.gnome.shell.extensions.dash-to-dock";
    gsettings_set(dock, "background-opacity", "0.0")?;
    gsettings_set(dock, "custom-theme-shrink", "false")?;
    gsettings_set(dock, "show-show-apps-button", "false")?;
    gsettings_set(dock, "height-fraction", "0.9")?;
    gsettings_set(dock, "apply-custom-theme", "false")?;
    gsettings_set(dock, "transparency-mode", "'FIXED'")?;
    gsettings_set(dock, "customize-alphas", "true")?;
    gsettings_set(dock, "min-alpha", "0.0")?;
    gsettings_set(dock, "max-alpha", "0.0")?;

    let glass = "org.gnome.shell.extensions.liquid-glass";
    gsettings_set(glass, "application-blur-radius", "9")?;
    gsettings_set(glass, "application-content-opacity", "1.0")?;
    gsettings_set(glass, "application-corner-radius", "17.0")?;
    gsettings_set(glass, "application-glass-all-windows", "false")?;
    gsettings_set(glass, "application-tint-color", "'#000000'")?;
    gsettings_set(glass, "application-tint-strength", "0.06")?;
    gsettings_set(glass, "application-window-whitelist", "[]")?;
    gsettings_set(glass, "dock-corner-radius", "24.0")?;
    gsettings_set(glass, "dock-glass-expand", "3")?;
    gsettings_set(glass, "dock-tint-color", "'#000000'")?;
    gsettings_set(glass, "enable-application-glass", "false")?;
    gsettings_set(glass, "enable-menu-glass", "true")?;
    gsettings_set(glass, "enable-quick-settings-glass", "false")?;
    gsettings_set(glass, "menu-tint-color", "'#000000'")?;
    gsettings_set(glass, "notification-tint-color", "'#000000'")?;
    gsettings_set(glass, "osd-tint-color", "'#000000'")?;
    gsettings_set(glass, "output-logs", "false")?;
    Ok(())
}

// ── Cleanup ──

pub fn run_cleanup() -> Result<(), String> {
    let scripts = [
        vec!["sudo", "/usr/libexec/pulsar-cleanup-live.sh"],
        vec!["pkexec", "/usr/libexec/pulsar-cleanup-live.sh"],
    ];
    for cmd in &scripts {
        let out = Command::new(cmd[0])
            .args(&cmd[1..])
            .output();
        if let Ok(o) = out {
            if o.status.success() {
                return Ok(());
            }
        }
    }
    Err("Could not run cleanup with sudo or pkexec".into())
}
