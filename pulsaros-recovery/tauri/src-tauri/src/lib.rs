mod commands;
mod disk;
mod fstab;
mod restore;
mod users;

use std::sync::Mutex;

use commands::recovery::RestoreState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .manage(RestoreState {
            running: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            commands::recovery::get_btrfs_targets,
            commands::recovery::get_local_squashfs,
            commands::recovery::start_restore,
            commands::recovery::launch_app,
            commands::recovery::reboot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
