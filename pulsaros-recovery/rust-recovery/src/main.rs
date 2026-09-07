//! Pulsar OS Recovery Assistant.
mod demo;
mod icons;
mod manifest;
mod models;
mod network;
mod restore;
mod system;
mod theme;
mod ui;

use crate::demo::set_demo_mode;
use crate::ui::build_ui;
use gio::prelude::*;
use gtk4::Application;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has_demo_arg = args.iter().any(|a| a == "--demo" || a == "--dry-run" || a == "-d");
    let has_demo_env = std::env::var("PULSAR_DEMO").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false)
        || std::env::var("PULSAR_DRY_RUN").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false);

    if has_demo_arg || has_demo_env {
        set_demo_mode(true);
        println!("🚀 Pulsar OS Recovery running in DEMO / DRY-RUN mode (Disks are protected, no changes will be made).");
    }

    let app = Application::builder()
        .application_id("es.inled.pulsaros.recovery-assistant")
        .build();

    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[]);
}
