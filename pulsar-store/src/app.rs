//! Application shell: window, sidebar, router wiring, module registry.

use adw::prelude::*;

pub const APP_ID: &str = "es.inled.PulsarStore";

pub fn run() -> gtk4::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Pulsar Store")
            .default_width(1100)
            .default_height(720)
            .build();
        window.present();
    });
    app.run()
}
