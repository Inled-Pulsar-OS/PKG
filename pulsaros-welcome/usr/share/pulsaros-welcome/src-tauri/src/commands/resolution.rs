use crate::resolution;

#[tauri::command]
pub fn get_resolutions() -> Vec<resolution::Resolution> {
    resolution::get_resolutions()
}

#[tauri::command]
pub fn set_resolution(width: u32, height: u32) -> Result<(), String> {
    resolution::set_resolution(width, height)
}
