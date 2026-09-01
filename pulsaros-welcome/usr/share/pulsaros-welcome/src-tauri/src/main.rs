// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Fix WebKitGTK transparent window black background bug on Linux (DMABUF renderer)
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    pulsaros_welcome_lib::run();
}
