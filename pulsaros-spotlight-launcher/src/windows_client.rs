//! Blocking D-Bus client for the Shell extension's window switcher
//! (`es.inled.PulsarosSpotlight.Windows`). Degrades to an empty list when the extension is absent.
#![allow(dead_code)]
use zbus::blocking::{Connection, Proxy};

use crate::core::windows::WindowEntry;

const NAME: &str = "es.inled.PulsarosSpotlight.Windows";
const PATH: &str = "/es/inled/PulsarosSpotlight/Windows";

fn proxy(conn: &Connection) -> zbus::Result<Proxy<'_>> {
    Proxy::new(conn, NAME, PATH, NAME)
}

fn has_owner(conn: &Connection) -> bool {
    let Ok(dbus) = zbus::blocking::fdo::DBusProxy::new(conn) else { return false };
    let Ok(name) = zbus::names::BusName::try_from(NAME) else { return false };
    dbus.name_has_owner(name).unwrap_or(false)
}

/// Empty when the extension is not running or the call fails.
pub fn list_windows() -> Vec<WindowEntry> {
    let Ok(conn) = Connection::session() else { return Vec::new() };
    if !has_owner(&conn) {
        return Vec::new();
    }
    let rows: Vec<(u32, String, String, String, String)> =
        match proxy(&conn).and_then(|p| p.call("ListWindows", &())) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
    rows.into_iter()
        .map(|(id, title, wm_class, app_id, workspace)| WindowEntry { id, title, wm_class, app_id, workspace })
        .collect()
}

pub fn activate(id: u32) -> zbus::Result<()> {
    proxy(&Connection::session()?)?.call("Activate", &(id,))
}

pub fn close(id: u32) -> zbus::Result<()> {
    proxy(&Connection::session()?)?.call("Close", &(id,))
}
