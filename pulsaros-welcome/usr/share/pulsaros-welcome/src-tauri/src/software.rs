use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

pub fn launch_app_with_fallback(primary: &str, fallback: Option<&str>) -> Result<(), String> {
    let candidates: Vec<String> = std::iter::once(primary)
        .chain(fallback)
        .map(|s| s.to_string())
        .collect();
    for cmd in &candidates {
        if which(cmd) {
            return Command::new(cmd)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Failed to launch {}: {}", cmd, e));
        }
    }
    Err(format!("{} not found in PATH", primary))
}

fn which(cmd: &str) -> bool {
    if cmd.contains('/') {
        // Absolute path — check the file exists and is executable.
        let perms = std::fs::metadata(cmd)
            .map(|m| m.permissions())
            .ok();
        let executable = perms
            .as_ref()
            .map(|p| p.mode() & 0o111 != 0)
            .unwrap_or(false);
        return perms.is_some() && executable;
    }
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn launch_display_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("display")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn launch_appearance_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("background")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn check_adb_devices() -> Result<String, String> {
    let out = Command::new("adb")
        .arg("devices")
        .output()
        .map_err(|e| format!("Error running adb: {}", e))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn launch_run_cleanup() -> Result<(), String> {
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

pub fn launch_ootb() -> Result<(), String> {
    if which("pulsaros-ootb") {
        return Command::new("pulsaros-ootb")
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to launch pulsaros-ootb: {}", e));
    }
    let fallback = Path::new("/usr/share/pulsaros/welcome_ootb.py");
    if fallback.exists() {
        return Command::new("sudo")
            .args(["-E", "/usr/bin/python3", "/usr/share/pulsaros/welcome_ootb.py"])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to launch welcome_ootb.py: {}", e));
    }
    Err("pulsaros-ootb or welcome_ootb.py not found".into())
}

pub fn launch_recovery() -> Result<(), String> {
    launch_app_with_fallback("pulsaros-recovery", Some("pulsaros-recovery-window"))
}
