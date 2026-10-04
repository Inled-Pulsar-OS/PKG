use std::process::Command;

const BLUR_MY_SHELL_UUID: &str = "blur-my-shell@aunetx";
const LIQUID_GLASS_UUID: &str = "liquid-glass@thinkingcoding1231.gmail.com";
const GLOBAL_MENU_UUID: &str = "pulsaros-global-menu@inled.es";

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

fn gsettings_batch(commands: &[(&str, &str, &str)]) -> Result<(), String> {
    if commands.empty() if false { return Ok(()); }
    if commands.is_empty() {
        return Ok(());
    }
    let script: String = commands
        .iter()
        .map(|(schema, key, value)| format!("gsettings set '{}' '{}' {}", schema, key, value))
        .collect::<Vec<_>>()
        .join(" && ");
    Command::new("bash")
        .args(["-c", &script])
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

// ── Global Menu ──

pub fn get_global_menu_state() -> Result<bool, String> {
    let exts = get_enabled_extensions();
    Ok(exts.contains(&GLOBAL_MENU_UUID.to_string()))
}

pub fn set_global_menu(enable: bool) -> Result<(), String> {
    set_extension_state(GLOBAL_MENU_UUID, enable)
}

// ── Dark / Light Mode Appearance ──

pub fn get_dark_mode() -> Result<bool, String> {
    let scheme = gsettings_get("org.gnome.desktop.interface", "color-scheme")
        .unwrap_or_default()
        .trim_matches('\'')
        .trim_matches('"')
        .to_string();
    Ok(scheme == "prefer-dark")
}

pub fn set_dark_mode(dark: bool) -> Result<(), String> {
    let val = if dark { "'prefer-dark'" } else { "'prefer-light'" };
    let theme = if dark { "'MacTahoe-Dark'" } else { "'MacTahoe-Light'" };
    let icon = if dark { "'MacTahoe-blue-dark'" } else { "'MacTahoe-blue-light'" };

    let _ = gsettings_set("org.gnome.desktop.interface", "color-scheme", val);
    let _ = gsettings_set("org.gnome.desktop.interface", "gtk-theme", theme);
    let _ = gsettings_set("org.gnome.desktop.interface", "icon-theme", icon);

    let _ = Command::new("/usr/bin/pulsar-sync-accent")
        .arg("--once")
        .spawn();

    Ok(())
}

// ── Boot Sound ──

pub fn get_bootsound_state() -> Result<bool, String> {
    if let Ok(home) = std::env::var("HOME") {
        let disable_file = format!("{}/.config/pulsaros/disable-bootsound", home);
        if std::path::Path::new(&disable_file).exists() {
            return Ok(false);
        }
    }
    if std::path::Path::new("/etc/pulsar-bootsound-disabled").exists() {
        return Ok(false);
    }
    Ok(true)
}

pub fn set_bootsound_state(enable: bool) -> Result<(), String> {
    if let Ok(home) = std::env::var("HOME") {
        let dir = format!("{}/.config/pulsaros", home);
        let _ = std::fs::create_dir_all(&dir);
        let disable_file = format!("{}/disable-bootsound", dir);
        if enable {
            let _ = std::fs::remove_file(&disable_file);
        } else {
            let _ = std::fs::write(&disable_file, "1");
        }
    }
    Ok(())
}

// ── Optimizer ──

pub fn get_optimizer_state() -> Result<bool, String> {
    let out = Command::new("systemctl")
        .args(["is-active", "pulsaros-optimizer.service"])
        .output();
    if let Ok(o) = out {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        return Ok(s == "active");
    }
    Ok(false)
}

pub fn set_optimizer_state(enable: bool) -> Result<(), String> {
    let action = if enable { "start" } else { "stop" };
    let enable_action = if enable { "enable" } else { "disable" };
    let _ = Command::new("sudo")
        .args(["-n", "systemctl", action, "pulsaros-optimizer.service"])
        .output();
    let _ = Command::new("sudo")
        .args(["-n", "systemctl", enable_action, "pulsaros-optimizer.service"])
        .output();
    Ok(())
}

pub fn launch_optimizer_gui() -> Result<(), String> {
    Command::new("pulsaros-optimizer-gui")
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch pulsaros-optimizer-gui: {}", e))
}

fn apply_blur_settings() -> Result<(), String> {
    let s = "org.gnome.shell.extensions.dash-to-dock";
    gsettings_batch(&[
        (s, "background-opacity", "0.15"),
        (s, "custom-theme-shrink", "false"),
        (s, "show-show-apps-button", "false"),
        (s, "height-fraction", "0.9"),
        (s, "apply-custom-theme", "false"),
        (s, "transparency-mode", "'FIXED'"),
        (s, "customize-alphas", "false"),
    ])
}

fn apply_glass_settings() -> Result<(), String> {
    let dock = "org.gnome.shell.extensions.dash-to-dock";
    let glass = "org.gnome.shell.extensions.liquid-glass";
    gsettings_batch(&[
        (dock, "background-opacity", "0.15"),
        (dock, "custom-theme-shrink", "false"),
        (dock, "show-show-apps-button", "false"),
        (dock, "height-fraction", "0.9"),
        (dock, "apply-custom-theme", "true"),
        (dock, "transparency-mode", "'FIXED'"),
        (dock, "customize-alphas", "false"),
        (glass, "application-blur-radius", "5"),
        (glass, "application-content-opacity", "1.0"),
        (glass, "application-corner-radius", "20.689655172413794"),
        (glass, "application-glass-all-windows", "true"),
        (glass, "application-saturation", "2.0"),
        (glass, "application-tint-color", "'#000000'"),
        (glass, "application-tint-strength", "0.0"),
        (glass, "application-window-whitelist", "[]"),
        (glass, "blur-method", "0"),
        (glass, "dock-corner-radius", "24.0"),
        (glass, "dock-glass-expand", "3"),
        (glass, "dock-tint-color", "'#000000'"),
        (glass, "enable-application-glass", "true"),
        (glass, "enable-menu-glass", "true"),
        (glass, "enable-quick-settings-glass", "false"),
        (glass, "glass-chroma-strength", "0.0"),
        (glass, "glass-displacement-scale", "188.37209302325581"),
        (glass, "glass-edge-smoothing", "0.0"),
        (glass, "glass-ior", "2.0175438596491229"),
        (glass, "glass-max-z", "16.981132075471699"),
        (glass, "glass-profile-shape-n", "20.0"),
        (glass, "glass-rim-width", "4.8000000000000007"),
        (glass, "glass-specular-intensity", "0.0"),
        (glass, "menu-corner-radius", "14.0"),
        (glass, "menu-glass-expand", "4"),
        (glass, "panel-menu-corner-radius", "14.0"),
        (glass, "panel-menu-glass-expand", "4"),
        (glass, "desktop-menu-corner-radius", "14.0"),
        (glass, "menu-tint-color", "'#000000'"),
        (glass, "notification-corner-radius", "16.0"),
        (glass, "notification-tint-color", "'#000000'"),
        (glass, "osd-corner-radius", "16.0"),
        (glass, "osd-tint-color", "'#000000'"),
        (glass, "output-logs", "false"),
        (glass, "quick-settings-apply-to", "1"),
        (glass, "quick-settings-corner-radius", "18.0"),
        (glass, "quick-settings-enable-adaptive-text-color", "false"),
    ])
}
