use crate::effects;

#[tauri::command]
pub fn get_effects_state() -> Result<bool, String> {
    effects::get_effects_state()
}

#[tauri::command]
pub fn set_effects(use_liquid_glass: bool) -> Result<(), String> {
    effects::set_effects(use_liquid_glass)
}

#[tauri::command]
pub fn get_global_menu_state() -> Result<bool, String> {
    effects::get_global_menu_state()
}

#[tauri::command]
pub fn set_global_menu(enable: bool) -> Result<(), String> {
    effects::set_global_menu(enable)
}

#[tauri::command]
pub fn get_dark_mode() -> Result<bool, String> {
    effects::get_dark_mode()
}

#[tauri::command]
pub fn set_dark_mode(dark: bool) -> Result<(), String> {
    effects::set_dark_mode(dark)
}

#[tauri::command]
pub fn get_bootsound_state() -> Result<bool, String> {
    effects::get_bootsound_state()
}

#[tauri::command]
pub fn set_bootsound_state(enable: bool) -> Result<(), String> {
    effects::set_bootsound_state(enable)
}

#[tauri::command]
pub fn get_optimizer_state() -> Result<bool, String> {
    effects::get_optimizer_state()
}

#[tauri::command]
pub fn set_optimizer_state(enable: bool) -> Result<(), String> {
    effects::set_optimizer_state(enable)
}

#[tauri::command]
pub fn launch_optimizer_gui() -> Result<(), String> {
    effects::launch_optimizer_gui()
}
