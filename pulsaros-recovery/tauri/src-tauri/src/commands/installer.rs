use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

use crate::installer;

pub struct InstallState {
    pub running: Mutex<bool>,
}

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    progress: f64,
    status: String,
}

#[derive(Clone, serde::Serialize)]
struct LogPayload {
    message: String,
}

#[tauri::command]
pub async fn start_install(
    app: AppHandle,
    disk_path: String,
    install_broadcom: bool,
    state: State<'_, InstallState>,
) -> Result<(), String> {
    {
        let mut running = state.running.lock().map_err(|e| e.to_string())?;
        if *running {
            return Err("Installation already in progress".into());
        }
        *running = true;
    }

    let state_clone = state.inner();
    let app_clone = app.clone();

    let result = tokio::task::spawn_blocking(move || {
        let log_fn = {
            let app = app_clone.clone();
            move |msg: String| {
                let _ = app.emit("install-log", LogPayload { message: msg });
            }
        };

        let progress_fn = {
            let app = app_clone.clone();
            move |pct: f64, msg: String| {
                let _ = app.emit(
                    "install-progress",
                    ProgressPayload {
                        progress: pct,
                        status: msg,
                    },
                );
            }
        };

        installer::run_installation(&disk_path, install_broadcom, &log_fn, &progress_fn)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let mut running = state_clone.running.lock().map_err(|e| e.to_string())?;
    *running = false;

    result
}

#[tauri::command]
pub fn detect_broadcom() -> bool {
    // Check lspci for Broadcom WiFi/BT
    let out = std::process::Command::new("lspci")
        .args(["-nn"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    if out.to_lowercase().contains("broadcom") {
        return true;
    }

    // Check lsusb
    let out = std::process::Command::new("lsusb")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    out.to_lowercase().contains("broadcom")
}
