use crate::wifi;

#[tauri::command]
pub fn launch_wifi_settings() -> Result<(), String> {
    wifi::launch_wifi_settings()
}
