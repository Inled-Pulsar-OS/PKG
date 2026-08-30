use crate::welcome;

#[tauri::command]
pub fn is_live_system() -> bool {
    welcome::is_live_system()
}

#[tauri::command]
pub fn is_arch_system() -> bool {
    welcome::is_arch_system()
}

#[tauri::command]
pub fn check_sentinel() -> bool {
    welcome::check_sentinel()
}

#[tauri::command]
pub fn write_sentinel() -> Result<(), String> {
    welcome::write_sentinel()
}

#[tauri::command]
pub fn get_resolutions() -> Vec<welcome::Resolution> {
    welcome::get_resolutions()
}

#[tauri::command]
pub fn set_resolution(width: u32, height: u32) -> Result<(), String> {
    welcome::set_resolution(width, height)
}

#[tauri::command]
pub fn launch_display_settings() -> Result<(), String> {
    welcome::launch_display_settings()
}

#[tauri::command]
pub fn launch_wifi_settings() -> Result<(), String> {
    welcome::launch_wifi_settings()
}

#[tauri::command]
pub fn launch_bluetooth_settings() -> Result<(), String> {
    welcome::launch_bluetooth_settings()
}

#[tauri::command]
pub fn get_effects_state() -> Result<bool, String> {
    welcome::get_effects_state()
}

#[tauri::command]
pub fn set_effects(use_liquid_glass: bool) -> Result<(), String> {
    welcome::set_effects(use_liquid_glass)
}

#[tauri::command]
pub fn check_adb_devices() -> Result<String, String> {
    welcome::check_adb_devices()
}

#[tauri::command]
pub fn run_cleanup() -> Result<(), String> {
    welcome::run_cleanup()
}

#[tauri::command]
pub fn launch_app(app: &str, fallback: Option<&str>) -> Result<(), String> {
    welcome::launch_app_with_fallback(app, fallback)
}
