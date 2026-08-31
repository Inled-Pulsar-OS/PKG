use std::process::Command;

const BLUR_MY_SHELL_UUID: &str = "blur-my-shell@aunetx";
const LIQUID_GLASS_UUID: &str = "liquid-glass@thinkingcoding1231.gmail.com";

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
    return inner
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
    return set_enabled_extensions(&exts);
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
    let s = "org.gnome.shell.extensions.dash-to-dock";
    gsettings_batch(&[
        (s, "background-opacity", "0.8"),
        (s, "custom-theme-shrink", "false"),
        (s, "show-show-apps-button", "false"),
        (s, "height-fraction", "0.9"),
        (s, "apply-custom-theme", "true"),
        (s, "transparency-mode", "'FIXED'"),
        (s, "customize-alphas", "false"),
    ])
}

fn apply_glass_settings() -> Result<(), String> {
    let dock = "org.gnome.shell.extensions.dash-to-dock";
    let glass = "org.gnome.shell.extensions.liquid-glass";
    gsettings_batch(&[
        (dock, "background-opacity", "0.0"),
        (dock, "custom-theme-shrink", "false"),
        (dock, "show-show-apps-button", "false"),
        (dock, "height-fraction", "0.9"),
        (dock, "apply-custom-theme", "false"),
        (dock, "transparency-mode", "'FIXED'"),
        (dock, "customize-alphas", "true"),
        (dock, "min-alpha", "0.0"),
        (dock, "max-alpha", "0.0"),
        (glass, "application-blur-radius", "9"),
        (glass, "application-content-opacity", "1.0"),
        (glass, "application-corner-radius", "17.0"),
        (glass, "application-glass-all-windows", "false"),
        (glass, "application-tint-color", "'#000000'"),
        (glass, "application-tint-strength", "0.06"),
        (glass, "application-window-whitelist", "[]"),
        (glass, "dock-corner-radius", "24.0"),
        (glass, "dock-glass-expand", "3"),
        (glass, "dock-tint-color", "'#000000'"),
        (glass, "enable-application-glass", "false"),
        (glass, "enable-menu-glass", "true"),
        (glass, "enable-quick-settings-glass", "false"),
        (glass, "menu-tint-color", "'#000000'"),
        (glass, "notification-tint-color", "'#000000'"),
        (glass, "osd-tint-color", "'#000000'"),
        (glass, "output-logs", "false"),
    ])
}
