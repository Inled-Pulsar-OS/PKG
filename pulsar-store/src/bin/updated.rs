//! Background update checker. Builds without GTK.
//!
//! Runs as a systemd user service. Every few hours (default 6; set with
//! `--interval-hours N` or `PULSAR_STORE_UPDATE_INTERVAL_HOURS`) it refreshes
//! the catalog, counts available updates, and tells the user once per new
//! update through a desktop notification whose button opens the Updates page.
//! Metered connections are skipped, unless a check is requested over D-Bus.
//!
//! D-Bus (session bus): name and path `es.inled.PulsarStore.Updater`,
//! `/es/inled/PulsarStore/Updater`, interface `es.inled.PulsarStore.Updater`:
//! method `CheckNow()`, property `UpdateCount` (u), signal `UpdatesChanged(u)`.

use pulsar_store::core::ports::CatalogProvider;
use pulsar_store::core::store::Store;
use pulsar_store::core::updater::{interval_from, is_metered, UpdateTracker};
use std::collections::HashMap;
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedValue, Value};

const NAME: &str = "es.inled.PulsarStore.Updater";
const PATH: &str = "/es/inled/PulsarStore/Updater";

struct Updater {
    count: Arc<AtomicU32>,
    trigger: mpsc::Sender<()>,
}

#[zbus::interface(name = "es.inled.PulsarStore.Updater")]
impl Updater {
    /// Checks for updates now, even on a metered connection.
    fn check_now(&self) {
        let _ = self.trigger.send(());
    }

    #[zbus(property)]
    fn update_count(&self) -> u32 {
        self.count.load(Ordering::SeqCst)
    }
}

fn metered(conn: &Connection) -> bool {
    conn.call_method(
        Some("org.freedesktop.NetworkManager"),
        "/org/freedesktop/NetworkManager",
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.freedesktop.NetworkManager", "Metered"),
    )
    .ok()
    .and_then(|m| m.body().deserialize::<OwnedValue>().ok())
    .and_then(|v| u32::try_from(v).ok())
    .is_some_and(is_metered)
}

/// Shows the notification and returns its id.
fn notify(conn: &Connection, count: usize) -> Option<u32> {
    let body = if count == 1 {
        "1 update is available.".to_string()
    } else {
        format!("{count} updates are available.")
    };
    let hints: HashMap<&str, Value> = HashMap::new();
    let reply = conn
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &(
                "Pulsar Store",
                0u32,
                "pulsar-store",
                "Updates available",
                body,
                vec!["open", "Open Updates"],
                hints,
                -1i32,
            ),
        )
        .ok()?;
    reply.body().deserialize::<u32>().ok()
}

/// Opens the Updates page when the notification's button is pressed.
fn listen_for_actions(conn: Connection, last_id: Arc<AtomicU32>) {
    std::thread::spawn(move || {
        let Ok(rule) = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.freedesktop.Notifications")
            .and_then(|b| b.member("ActionInvoked"))
            .map(|b| b.build())
        else {
            return;
        };
        let Ok(iter) = zbus::blocking::MessageIterator::for_match_rule(rule, &conn, None) else {
            return;
        };
        for msg in iter.flatten() {
            if let Ok((id, key)) = msg.body().deserialize::<(u32, String)>() {
                if id == last_id.load(Ordering::SeqCst) && key == "open" {
                    let _ = Command::new("pulsar-store")
                        .args(["--page", "updates"])
                        .spawn();
                }
            }
        }
    });
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let interval = interval_from(
        &args,
        std::env::var("PULSAR_STORE_UPDATE_INTERVAL_HOURS")
            .ok()
            .as_deref(),
    );

    let count = Arc::new(AtomicU32::new(0));
    let (tx, rx) = mpsc::channel();
    let conn = match zbus::blocking::connection::Builder::session()
        .and_then(|b| b.name(NAME))
        .and_then(|b| {
            b.serve_at(
                PATH,
                Updater {
                    count: count.clone(),
                    trigger: tx,
                },
            )
        })
        .and_then(|b| b.build())
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("pulsar-store-updated: cannot use the session bus: {e}");
            return ExitCode::FAILURE;
        }
    };

    let last_notification = Arc::new(AtomicU32::new(0));
    listen_for_actions(conn.clone(), last_notification.clone());

    let store = Store::new();
    let mut tracker = UpdateTracker::default();
    let mut forced = false;
    loop {
        if forced || !metered(&conn) {
            let _ = store.catalog.refresh(false);
            if let Ok(catalog) = store.catalog.load() {
                let updates = store.installer.updates(&catalog.packages);
                let ids: Vec<String> = updates.iter().map(|u| u.item.id.clone()).collect();
                let previous = count.swap(ids.len() as u32, Ordering::SeqCst);
                if previous != ids.len() as u32 {
                    let _ = conn.emit_signal(
                        None::<&str>,
                        PATH,
                        NAME,
                        "UpdatesChanged",
                        &(ids.len() as u32,),
                    );
                }
                if tracker.observe(&ids) {
                    if let Some(id) = notify(&conn, ids.len()) {
                        last_notification.store(id, Ordering::SeqCst);
                    }
                }
            }
        }
        // Sleep until the next check, or until someone asks for one now.
        forced = matches!(rx.recv_timeout(interval), Ok(()));
    }
}
