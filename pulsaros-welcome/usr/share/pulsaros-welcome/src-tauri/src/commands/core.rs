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

/// Wi-Fi configurator slide is disabled by default; re-enable it by launching
/// the welcome app with PULSAROS_ENABLE_WIFI_SLIDE=1.
#[tauri::command]
pub fn wifi_slide_enabled() -> bool {
    std::env::var("PULSAROS_ENABLE_WIFI_SLIDE").map(|v| v == "1").unwrap_or(false)
}

#[tauri::command]
pub fn write_sentinel() -> Result<(), String> {
    core::write_sentinel()
}

#[tauri::command]
pub fn close(app: tauri::AppHandle, window: tauri::Window) {
    let _ = window.destroy();
    app.exit(0);
    std::process::exit(0);
}
