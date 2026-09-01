use std::process::Command;

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

pub fn launch_display_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("display")
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
