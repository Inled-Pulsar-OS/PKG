//! Minimal D-Bus service the GNOME Shell extension calls to tell the launcher a click landed
//! outside its window. A regular app window can't see clicks on other windows or the desktop
//! under Wayland — only the compositor (the Shell extension) can — so for this one case the
//! usual direction (Rust as D-Bus client of the extension, see `windows_client.rs`) inverts.
//!
//! This is intentionally one method and no extra state: the extension decides "that click was
//! outside," and this service just forwards the fact to the GTK main thread, which still makes
//! the actual hide/keep-open decision (see `SpotlightWindow::handle_outside_click`).
use std::sync::mpsc::{channel, Receiver, Sender};

use zbus::interface;

pub const NAME: &str = "es.inled.PulsarosSpotlight.Control";
const PATH: &str = "/es/inled/PulsarosSpotlight/Control";

struct ControlIface {
    hide_tx: Sender<()>,
}

#[interface(name = "es.inled.PulsarosSpotlight.Control")]
impl ControlIface {
    /// Called by the Shell extension when a pointer press lands outside the Spotlight
    /// window's current bounds while it's visible.
    fn hide(&self) {
        let _ = self.hide_tx.send(());
    }
}

/// Starts the control service on a background thread and returns the receiving end of a
/// channel that fires once per `Hide` call. GTK widgets aren't thread-safe, so the receiver
/// is meant to be polled from the main thread (e.g. via `glib::timeout_add_local`), same
/// pattern as `SpotlightWindow::check_indexing_status`'s mpsc channel.
///
/// Failures (no session bus, name already owned, etc.) are logged and degrade to "the
/// click-outside path never fires" rather than crashing the launcher.
pub fn spawn() -> Receiver<()> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let iface = ControlIface { hide_tx: tx };
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
    rx
}
