use crate::core;

#[tauri::command]
pub fn is_live_system() -> bool {
    core::is_live_system()
}

#[tauri::command]
pub fn is_arch_system() -> bool {
    core::is_arch_system()
}

#[tauri::command]
pub fn check_sentinel() -> bool {
    core::check_sentinel()
}

#[tauri::command]
pub fn is_ootb_pending() -> bool {
    core::is_ootb_pending()
}

#[tauri::command]
pub fn write_sentinel() -> Result<(), String> {
    core::write_sentinel()
}

#[tauri::command]
pub fn close(window: tauri::Window) {
    let _ = window.close();
}
