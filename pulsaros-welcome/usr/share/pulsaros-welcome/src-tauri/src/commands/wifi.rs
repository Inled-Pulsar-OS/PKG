use crate::wifi;

#[tauri::command]
pub fn launch_wifi_settings() -> Result<(), String> {
    wifi::launch_wifi_settings()
}

#[tauri::command]
pub fn scan_wifi_networks() -> Vec<wifi::WifiNetwork> {
    wifi::scan_networks()
}

#[tauri::command]
pub fn connect_to_wifi(ssid: String, password: Option<String>) -> Result<(), String> {
    wifi::connect(&ssid, password.as_deref())
}
