use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub security: String,
}

pub fn scan_networks() -> Vec<WifiNetwork> {
    let output = match Command::new("nmcli")
        .args(["-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    if !output.status.success() {
        return vec![];
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen: HashMap<String, WifiNetwork> = HashMap::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 2 {
            continue;
        }

        let ssid = parts[0].trim().to_string();
        if ssid.is_empty() {
            continue;
        }

        let signal: u8 = parts[1].trim().parse().unwrap_or(0);
        let security = if parts.len() > 2 {
            parts[2].trim().to_string()
        } else {
            String::new()
        };

        // Keep the entry with the strongest signal per SSID
        match seen.get(&ssid) {
            Some(existing) if existing.signal >= signal => {}
            _ => {
                seen.insert(
                    ssid.clone(),
                    WifiNetwork {
                        ssid,
                        signal,
                        security,
                    },
                );
            }
        }
    }

    let mut networks: Vec<WifiNetwork> = seen.into_values().collect();
    networks.sort_by(|a, b| b.signal.cmp(&a.signal));
    networks
}

pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    let mut args = vec!["dev", "wifi", "connect", ssid];

    if let Some(pw) = password {
        if !pw.is_empty() {
            args.push("password");
            args.push(pw);
        }
    }

    let output = Command::new("nmcli")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run nmcli: {}", e))?;

    if output.status.success() {
        return Ok(());
    }

    // nmcli failed — fall back to native GNOME dialog
    let _ = Command::new("gnome-control-center").arg("wifi").spawn();

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("Connection failed: {}", stderr.trim()))
}

pub fn launch_wifi_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("wifi")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}