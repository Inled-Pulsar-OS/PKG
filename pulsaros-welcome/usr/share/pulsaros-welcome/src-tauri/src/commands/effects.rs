use crate::effects;

#[tauri::command]
pub fn get_effects_state() -> Result<bool, String> {
    effects::get_effects_state()
}

#[tauri::command]
pub fn set_effects(use_liquid_glass: bool) -> Result<(), String> {
    effects::set_effects(use_liquid_glass)
}
