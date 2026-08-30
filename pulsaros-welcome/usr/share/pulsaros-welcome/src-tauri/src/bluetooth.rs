use std::process::Command;

pub fn launch_bluetooth_settings() -> Result<(), String> {
    Command::new("gnome-control-center")
        .arg("bluetooth")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
