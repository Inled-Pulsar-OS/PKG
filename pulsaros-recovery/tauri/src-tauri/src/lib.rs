mod boot;
mod commands;
mod disk;
mod fstab;
mod installer;
mod mode;
mod ootb;
mod restore;
mod users;

use std::sync::Mutex;

use commands::installer::InstallState;
use commands::ootb::OotbState;
use commands::recovery::RestoreState;

#[tauri::command]
fn get_system_mode() -> &'static str {
    mode::detect_mode()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .manage(RestoreState {
            running: Mutex::new(false),
        })
        .manage(InstallState {
            running: Mutex::new(false),
        })
        .manage(OotbState {
            running: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_mode,
            commands::recovery::get_btrfs_targets,
            commands::recovery::get_local_squashfs,
            commands::recovery::start_restore,
            commands::recovery::launch_app,
            commands::recovery::reboot,
            commands::installer::start_install,
            commands::installer::detect_broadcom,
            commands::ootb::get_ootb_data,
            commands::ootb::start_ootb_setup,
            commands::ootb::ootb_final_cleanup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
