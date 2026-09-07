//! Detección de red y gestión de Wi-Fi vía nmcli.
use crate::models::{NetConnType, NetworkStatus, WifiNetwork};
use std::collections::HashMap;
use std::process::Command;
pub fn get_network_status() -> NetworkStatus {
    // 1. Try nmcli to get device and connection info
    if let Ok(out) = Command::new("nmcli")
        .args(&["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "dev"])
        .output()
    {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 4 {
                    let dev_type = parts[1].trim();
                    let state = parts[2].trim();
                    let conn = parts[3].trim();
                    if state == "connected" {
                        let c_type = if dev_type == "wifi" {
                            NetConnType::Wifi
                        } else if dev_type == "ethernet" {
                            NetConnType::Ethernet
                        } else {
                            NetConnType::Ethernet
                        };
                        let name = if !conn.is_empty() && conn != "--" {
                            conn.to_string()
                        } else if c_type == NetConnType::Wifi {
                            "Wi-Fi Network".to_string()
                        } else {
                            "Wired (Ethernet)".to_string()
                        };
                        return NetworkStatus {
                            is_connected: true,
                            conn_type: c_type,
                            conn_name: name,
                        };
                    }
                }
            }
        }
    }

    // 2. Check general connectivity via nmcli
    if let Ok(out) = Command::new("nmcli")
        .args(&["-t", "-f", "STATE", "general"])
        .output()
    {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("connected") {
                return NetworkStatus {
                    is_connected: true,
                    conn_type: NetConnType::Ethernet,
                    conn_name: "Connected".to_string(),
                };
            }
        }
    }

    // 3. Fallback: check if a default route exists
    if let Ok(out) = Command::new("ip").args(&["route", "show", "default"]).output() {
        if out.status.success() && !out.stdout.is_empty() {
            return NetworkStatus {
                is_connected: true,
                conn_type: NetConnType::Ethernet,
                conn_name: "Connected".to_string(),
            };
        }
    }

    NetworkStatus {
        is_connected: false,
        conn_type: NetConnType::None,
        conn_name: "Offline / Disconnected".to_string(),
    }
}

pub fn scan_wifi_networks() -> Vec<WifiNetwork> {
    let output = match Command::new("nmcli")
        .args(&["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "dev", "wifi", "list", "--rescan", "yes"])
        .output()
    {
        Ok(o) => {
            if o.status.success() {
                o
            } else {
                match Command::new("nmcli")
                    .args(&["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "dev", "wifi", "list"])
                    .output()
                {
                    Ok(o2) => o2,
                    Err(_) => return vec![],
                }
            }
        }
        Err(_) => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen: HashMap<String, WifiNetwork> = HashMap::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 3 {
            continue;
        }

        let in_use = parts[0].trim() == "*";
        let ssid = parts[1].trim().to_string();
        if ssid.is_empty() || ssid == "--" {
            continue;
        }

        let signal: u8 = parts[2].trim().parse().unwrap_or(0);
        let security = if parts.len() > 3 {
            let s = parts[3].trim();
            if s.is_empty() || s == "--" {
                "Open".to_string()
            } else {
                s.to_string()
            }
        } else {
            "Open".to_string()
        };

        match seen.get_mut(&ssid) {
            Some(existing) => {
                if in_use {
                    existing.in_use = true;
                }
                if signal > existing.signal {
                    existing.signal = signal;
                    existing.security = security;
                }
            }
            None => {
                seen.insert(
                    ssid.clone(),
                    WifiNetwork {
                        in_use,
                        ssid,
                        signal,
                        security,
                    },
                );
            }
        }
    }

    let mut list: Vec<WifiNetwork> = seen.into_values().collect();
    list.sort_by(|a, b| {
        b.in_use
            .cmp(&a.in_use)
            .then_with(|| b.signal.cmp(&a.signal))
            .then_with(|| a.ssid.cmp(&b.ssid))
    });
    list
}

pub fn connect_wifi(ssid: &str, password: Option<&str>) -> Result<(), String> {
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
        .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let out_msg = String::from_utf8_lossy(&output.stdout);
        let full = format!("{} {}", err_msg.trim(), out_msg.trim()).trim().to_string();
        if full.is_empty() {
            Err("Failed to connect. Please verify password and signal.".to_string())
        } else {
            Err(full)
        }
    }
}

pub fn open_external_network_settings() {
    let _ = Command::new("gnome-control-center")
        .arg("wifi")
        .spawn()
        .or_else(|_| Command::new("nm-connection-editor").spawn())
        .or_else(|_| Command::new("alacritty").args(&["-e", "nmtui"]).spawn())
        .or_else(|_| Command::new("xterm").args(&["-e", "nmtui"]).spawn());
}
