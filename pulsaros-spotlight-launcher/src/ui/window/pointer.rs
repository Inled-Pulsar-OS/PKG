//! Outside-click and pointer tracking: hide-on-click-away, the shell extension pointer feed,
//! and the Hyprland cursor poll.
use super::*;

impl SpotlightWindow {
    /// Same two checks the focus-loss handler has always used: the `--keep-open` debug
    /// escape hatch and the `hide_on_focus_loss` config toggle. Any path that wants to hide
    /// the window because of an "outside" interaction (focus-loss today, click-outside via
    /// the Shell extension) must gate on this first, so the two paths can never drift apart.
    pub(super) fn may_hide_on_outside_event(&self) -> bool {
        let keep_open = std::env::args().any(|a| a == "--keep-open");
        !keep_open && self.config.borrow().hide_on_focus_loss
    }

    /// The one place that actually hides the window and cleans up. Called by both the
    /// focus-loss handler and the click-outside D-Bus request from the Shell extension.
    pub(super) fn hide_window(self: &Rc<Self>) {
        self.window.set_visible(false);
    }

    /// Entry point for the Shell extension's "click landed outside the window" report
    /// (see `src/control_service.rs`). Routes through the exact same gating and hide logic
    /// as the existing focus-loss path — no separate hide implementation.
    pub fn handle_outside_click(self: &Rc<Self>) {
        if self.window.is_visible() && self.may_hide_on_outside_event() {
            self.hide_window();
        }
    }

    /// Pointer motion anywhere on the screen, reported by the Shell extension (global
    /// coordinates). Reveals the mode buttons from any pointer travel, not only over the bar.
    pub fn handle_global_pointer(&self, x: f64, y: f64) {
        if self.window.is_visible() {
            self.bar.global_pointer.set(true);
            self.on_pointer(x, y);
        }
    }

    /// Hyprland has no extension that reports the pointer, so ask its IPC socket for the cursor
    /// position every 30 ms while the window is visible and still in the Idle bar state. The
    /// timer stops itself when the window hides. GNOME gets the position from the Shell
    /// extension instead (`handle_global_pointer`), so this only runs under Hyprland.
    pub(super) fn start_hypr_pointer_poll(self: &Rc<Self>) {
        let Some(path) = hypr_socket_path() else { return };
        if self.bar.hypr_poll.borrow().is_some() {
            return;
        }
        let s = self.clone();
        let id = glib::timeout_add_local(Duration::from_millis(30), move || {
            if !s.window.is_visible() {
                s.bar.hypr_poll.borrow_mut().take();
                return glib::ControlFlow::Break;
            }
            if s.layout.get() == LayoutState::Idle {
                if let Some((x, y)) = hypr_cursor_pos(&path) {
                    s.handle_global_pointer(x, y);
                }
            }
            glib::ControlFlow::Continue
        });
        *self.bar.hypr_poll.borrow_mut() = Some(id);
    }
}

/// Path of Hyprland's command socket, when running under Hyprland.
fn hypr_socket_path() -> Option<std::path::PathBuf> {
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let run = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(std::path::PathBuf::from(run).join("hypr").join(sig).join(".socket.sock"))
}

/// Cursor position from Hyprland (`cursorpos` replies "x, y").
fn hypr_cursor_pos(socket: &Path) -> Option<(f64, f64)> {
    use std::io::{Read, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(50))).ok()?;
    stream.write_all(b"cursorpos").ok()?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).ok()?;
    parse_cursor_pos(&reply)
}

pub(super) fn parse_cursor_pos(reply: &str) -> Option<(f64, f64)> {
    let (x, y) = reply.trim().split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}
