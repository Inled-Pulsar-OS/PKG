// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Fix WebKitGTK transparent window black background bug on Linux (DMABUF
        // renderer) and disable the bubblewrap sandbox so the WebKitWebProcess
        // can load the gstreamer plugins needed to decode the slideshow videos.
        std::env::set_var("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS", "1");
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    pulsaros_welcome_lib::run();
}
