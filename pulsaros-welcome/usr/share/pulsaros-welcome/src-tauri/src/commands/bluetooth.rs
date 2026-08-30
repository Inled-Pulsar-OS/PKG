use crate::bluetooth;

#[tauri::command]
pub fn launch_bluetooth_settings() -> Result<(), String> {
    bluetooth::launch_bluetooth_settings()
}
