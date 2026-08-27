use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

use crate::disk;
use crate::restore::{self, RecoveryMode};

pub struct RestoreState {
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
pub fn get_btrfs_targets() -> Vec<disk::BtrfsTarget> {
    disk::find_btrfs_targets()
}

#[tauri::command]
pub fn get_local_squashfs() -> Option<String> {
    disk::detect_local_squashfs()
}

#[tauri::command]
pub async fn start_restore(
    app: AppHandle,
    target: disk::BtrfsTarget,
    internet_url: Option<String>,
    state: State<'_, RestoreState>,
) -> Result<(), String> {
    {
        let mut running = state.running.lock().map_err(|e| e.to_string())?;
        if *running {
            return Err("Restore already in progress".into());
        }
        *running = true;
    }

    let mode = match internet_url {
        Some(url) => RecoveryMode::Internet(url),
        None => RecoveryMode::Local,
    };

    let state_clone = state.inner();
    let app_clone = app.clone();

    let result = tokio::task::spawn_blocking(move || {
        let log_fn = {
            let app = app_clone.clone();
            move |msg: String| {
                let _ = app.emit("restore-log", LogPayload { message: msg });
            }
        };

        let progress_fn = {
            let app = app_clone.clone();
            move |pct: f64, msg: String| {
                let _ = app.emit(
                    "restore-progress",
                    ProgressPayload {
                        progress: pct,
                        status: msg,
                    },
                );
            }
        };

        restore::run_restoration(&target, mode, &log_fn, &progress_fn)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let mut running = state_clone.running.lock().map_err(|e| e.to_string())?;
    *running = false;

    result
}

#[tauri::command]
pub fn launch_app(app: &str) -> Result<(), String> {
    restore::launch_external_app(app)
}

#[tauri::command]
pub fn reboot() -> Result<(), String> {
    restore::reboot()
}
