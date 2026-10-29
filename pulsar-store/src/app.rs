//! Application shell: window, sidebar, router wiring, module registry.

use adw::prelude::*;
use pulsar_store::core::i18n;
use pulsar_store::tr;

pub const APP_ID: &str = "es.inled.PulsarStore";

pub fn run() -> gtk4::glib::ExitCode {
    i18n::init();
    if i18n::wants_rtl() {
        gtk4::Widget::set_default_direction(gtk4::TextDirection::Rtl);
    }
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(tr!("Pulsar Store"))
            .default_width(1100)
            .default_height(720)
            .build();
        window.present();
    });
    app.run()
}
