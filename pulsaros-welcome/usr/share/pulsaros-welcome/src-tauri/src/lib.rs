mod commands;
mod mode;
mod welcome;

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
            commands::welcome::is_live_system,
            commands::welcome::is_arch_system,
            commands::welcome::check_sentinel,
            commands::welcome::write_sentinel,
            commands::welcome::get_resolutions,
            commands::welcome::set_resolution,
            commands::welcome::launch_display_settings,
            commands::welcome::launch_wifi_settings,
            commands::welcome::launch_bluetooth_settings,
            commands::welcome::get_effects_state,
            commands::welcome::set_effects,
            commands::welcome::check_adb_devices,
            commands::welcome::run_cleanup,
            commands::welcome::launch_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
