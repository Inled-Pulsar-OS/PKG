//! Minimal D-Bus service the GNOME Shell extension calls to tell the launcher a click landed
//! outside its window. A regular app window can't see clicks on other windows or the desktop
//! under Wayland — only the compositor (the Shell extension) can — so for this one case the
//! usual direction (Rust as D-Bus client of the extension, see `windows_client.rs`) inverts.
//!
//! This is intentionally one method and no extra state: the extension decides "that click was
//! outside," and this service just forwards the fact to the GTK main thread, which still makes
//! the actual hide/keep-open decision (see `SpotlightWindow::handle_outside_click`).
use std::sync::Arc;

use zbus::interface;

pub const NAME: &str = "es.inled.PulsarosSpotlight.Control";
const PATH: &str = "/es/inled/PulsarosSpotlight/Control";

struct ControlIface {
    on_hide: Arc<dyn Fn() + Send + Sync>,
    on_pointer: Arc<dyn Fn(f64, f64) + Send + Sync>,
}

#[interface(name = "es.inled.PulsarosSpotlight.Control")]
impl ControlIface {
    /// Called by the Shell extension when a pointer press lands outside the Spotlight
    /// window's current bounds while it's visible.
    fn hide(&self) {
        (self.on_hide)();
    }

    /// Called by the Shell extension for pointer motion anywhere on the screen (global
    /// coordinates) while the Spotlight window is visible. A regular window only sees the
    /// pointer over itself, so the compositor reports the rest.
    fn pointer_moved(&self, x: f64, y: f64) {
        (self.on_pointer)(x, y);
    }
}

/// Starts the control service on a background thread. `on_hide` runs on that thread once per
/// `Hide` call, so it must only hand the fact to the GTK main thread (e.g. through
/// `glib::MainContext::invoke`) and never touch widgets itself. No polling: the main loop is
/// woken only when a call arrives.
///
/// Failures (no session bus, name already owned, etc.) are logged and degrade to "the
/// click-outside path never fires" rather than crashing the launcher.
pub fn spawn(on_hide: impl Fn() + Send + Sync + 'static, on_pointer: impl Fn(f64, f64) + Send + Sync + 'static) {
    let on_hide: Arc<dyn Fn() + Send + Sync> = Arc::new(on_hide);
    let on_pointer: Arc<dyn Fn(f64, f64) + Send + Sync> = Arc::new(on_pointer);
    std::thread::spawn(move || {
        let iface = ControlIface { on_hide, on_pointer };
        let conn = match zbus::blocking::Connection::session() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("pulsaros-spotlight: control service: session bus unavailable: {e}");
                return;
            }
        };
        if let Err(e) = conn.object_server().at(PATH, iface) {
            eprintln!("pulsaros-spotlight: control service: failed to register object: {e}");
            return;
        }
        if let Err(e) = conn.request_name(NAME) {
            eprintln!("pulsaros-spotlight: control service: failed to own {NAME}: {e}");
            return;
        }
        // zbus::blocking::Connection dispatches incoming calls on its own executor thread
        // once registered; park this thread for the process lifetime just to keep `conn`
        // (and the name ownership) alive.
        loop {
            std::thread::park();
        }
    });
}

#[cfg(test)]
#[path = "tests/control_service_tests.rs"]
mod tests;
