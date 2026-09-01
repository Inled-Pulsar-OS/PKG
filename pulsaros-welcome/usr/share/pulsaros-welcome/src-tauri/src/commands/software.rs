use crate::software;

#[tauri::command]
pub fn launch_app(app: &str, fallback: Option<&str>) -> Result<(), String> {
    software::launch_app_with_fallback(app, fallback)
}

#[tauri::command]
pub fn launch_display_settings() -> Result<(), String> {
    software::launch_display_settings()
}

#[tauri::command]
pub fn launch_appearance_settings() -> Result<(), String> {
    software::launch_appearance_settings()
}

#[tauri::command]
pub fn check_adb_devices() -> Result<String, String> {
    software::check_adb_devices()
}

#[tauri::command]
pub fn run_cleanup() -> Result<(), String> {
    software::launch_run_cleanup()
}

#[tauri::command]
pub fn launch_ootb() -> Result<(), String> {
    software::launch_ootb()
}

#[tauri::command]
pub fn launch_recovery() -> Result<(), String> {
    software::launch_recovery()
}
