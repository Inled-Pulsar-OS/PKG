mod bluetooth;
mod commands;
mod core;
mod effects;
mod feedback;
mod mode;
mod resolution;
mod software;
mod wifi;

#[tauri::command]
fn get_system_mode() -> &'static str {
    mode::detect_mode()
}

/// Replicates the former `/usr/bin/pulsaros-welcome` bash wrapper so the Tauri
/// binary can live at that path and still decide what to run on each boot.
fn preflight() {
    use std::path::Path;
    use std::process::Command;

    // 1. Live ISO environment: always show the full welcome (last slide = recovery).
    if core::is_live_system() {
        return;
    }

    // 2. Live user cleanup pending: hand over to the cleanup spinner app, which
    //    deletes the live user, removes the sentinel, then relaunches us.
    if Path::new("/etc/pulsar-need-cleanup").exists() {
        let _ = Command::new("/usr/bin/pulsar-cleanup-user").spawn();
        std::process::exit(0);
    }

    // 3. OOTB setup pending: show Hello first; the frontend launches
    //    pulsaros-ootb (or welcome_ootb.py) after the Continue click.
    if core::is_ootb_pending() {
        return;
    }

    // 4. Already done and not forced: exit silently (autostart on later boots).
    let force = std::env::args().any(|a| a == "--force" || a == "-f");
    if core::check_sentinel() && !force {
        std::process::exit(0);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // Skip the wrapper preflight in development builds so `tauri dev` always
    // opens the window regardless of the current system state.
    if !cfg!(debug_assertions) {
        preflight();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_system_mode,
            commands::core::is_live_system,
            commands::core::is_arch_system,
            commands::core::check_sentinel,
            commands::core::is_ootb_pending,
            commands::core::write_sentinel,
            commands::resolution::get_resolutions,
            commands::resolution::set_resolution,
            commands::software::launch_display_settings,
            commands::software::launch_appearance_settings,
            commands::wifi::launch_wifi_settings,
            commands::bluetooth::launch_bluetooth_settings,
            commands::effects::get_effects_state,
            commands::effects::set_effects,
            commands::software::check_adb_devices,
            commands::software::launch_ootb,
            commands::software::launch_recovery,
            commands::software::run_cleanup,
            commands::software::launch_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
