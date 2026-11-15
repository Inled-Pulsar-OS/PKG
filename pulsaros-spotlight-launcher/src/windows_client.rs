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
    rows.into_iter().map(row_to_entry).collect()
}

/// Pure mapping from a raw D-Bus row to `WindowEntry`, kept separate from
/// `list_windows` so it's directly unit-testable without a session bus.
fn row_to_entry(row: (u32, String, String, String, String)) -> WindowEntry {
    let (id, title, wm_class, app_id, workspace) = row;
    WindowEntry { id, title, wm_class, app_id, workspace }
}

pub fn activate(id: u32) -> zbus::Result<()> {
    proxy(&Connection::session()?)?.call("Activate", &(id,))
}

pub fn close(id: u32) -> zbus::Result<()> {
    proxy(&Connection::session()?)?.call("Close", &(id,))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_to_entry_maps_fields_in_order() {
        let row = (
            7u32,
            "Title".to_string(),
            "WmClass".to_string(),
            "app.id".to_string(),
            "workspace-1".to_string(),
        );

        let entry = row_to_entry(row);

        assert_eq!(entry.id, 7);
        assert_eq!(entry.title, "Title");
        assert_eq!(entry.wm_class, "WmClass");
        assert_eq!(entry.app_id, "app.id");
        assert_eq!(entry.workspace, "workspace-1");
    }
}
