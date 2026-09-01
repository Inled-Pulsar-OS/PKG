mod bluetooth;
mod commands;
mod core;
mod effects;
mod feedback;
mod mode;
mod resolution;
mod software;
mod wifi;

#[tauri::command]
fn get_system_mode() -> &'static str {
    mode::detect_mode()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_system_mode,
            commands::core::is_live_system,
            commands::core::is_arch_system,
            commands::core::check_sentinel,
            commands::core::write_sentinel,
            commands::resolution::get_resolutions,
            commands::resolution::set_resolution,
            commands::software::launch_display_settings,
            commands::wifi::launch_wifi_settings,
            commands::bluetooth::launch_bluetooth_settings,
            commands::effects::get_effects_state,
            commands::effects::set_effects,
            commands::software::check_adb_devices,
            commands::software::run_cleanup,
            commands::software::launch_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
