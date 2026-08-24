use std::sync::Mutex;
use tauri::State;

use crate::disk;
use crate::restore::{self, RecoveryMode};

pub struct RestoreState {
    pub running: Mutex<bool>,
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
pub fn start_restore(
    target: disk::BtrfsTarget,
    internet_url: Option<String>,
    state: State<'_, RestoreState>,
) -> Result<(), String> {
    let mut running = state.running.lock().map_err(|e| e.to_string())?;
    if *running {
        return Err("Restore already in progress".into());
    }
    *running = true;
    drop(running);

    let mode = match internet_url {
        Some(url) => RecoveryMode::Internet(url),
        None => RecoveryMode::Local,
    };

    let result = restore::run_restoration(&target, mode, &|_msg| {}, &|_pct, _msg| {});

    let mut running = state.running.lock().map_err(|e| e.to_string())?;
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
