use std::process::Command;

pub fn launch_wifi_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("wifi")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
