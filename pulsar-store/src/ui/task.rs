//! Running blocking work off the main thread and getting the result back on it (ADR 0007).

use crate::core::install::{Event, InstallHandle};
use gtk4::{gio, glib};
use std::time::Duration;

/// Runs `work` on a worker thread, then calls `done` with its result on the main thread.
pub fn run<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) + 'static,
) {
    glib::spawn_future_local(async move {
        if let Ok(value) = gio::spawn_blocking(work).await {
            done(value);
        }
    });
}

/// Follows an install/uninstall/update: `on_progress` for each report, `on_done` once at the end.
pub fn watch(
    handle: InstallHandle,
    on_progress: impl Fn(crate::core::model::Progress) + 'static,
    on_done: impl FnOnce(crate::core::error::Result<()>) + 'static,
) {
    let mut on_done = Some(on_done);
    glib::timeout_add_local(Duration::from_millis(60), move || loop {
        match handle.events.try_recv() {
            Ok(Event::Progress(p)) => on_progress(p),
            Ok(Event::Finished(result)) => {
                if let Some(f) = on_done.take() {
                    f(result);
                }
                return glib::ControlFlow::Break;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => return glib::ControlFlow::Break,
        }
    });
}
