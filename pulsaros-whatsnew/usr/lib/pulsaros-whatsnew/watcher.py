#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS - What's New: session watcher.

Runs inside the user's graphical session (started by autostart) and:

  * on the very first run, seeds the state so a brand-new installation does
    not pop the window (unless this is an existing system installing the
    feature for the first time, in which case the latest note is shown);
  * watches /var/lib/pulsaros/whatsnew/pending (written by the root hook after
    any package transaction) and opens the viewer once per new set of notes.

It is intentionally forgiving: any error is swallowed so it never disturbs the
desktop session.
"""

import os
import signal
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import common  # noqa: E402

VIEWER = "/usr/bin/pulsaros-whatsnew"
DEBOUNCE_MS = 4000
POLL_SECONDS = 60


class Watcher:
    def __init__(self):
        from gi.repository import GLib, Gio  # local import: fail softly

        self.GLib = GLib
        self.Gio = Gio
        self.loop = GLib.MainLoop()
        self.pending_token = ""
        self.monitor = None
        self.debounce_id = 0

    # -- state ------------------------------------------------------------
    def read_pending_token(self):
        path = os.path.join(common.SYSTEM_STATE_DIR, "pending")
        try:
            with open(path, "r", encoding="utf-8") as fh:
                data = fh.read().strip()
            import json

            marker = json.loads(data)
            return str(marker.get("token") or marker.get("ts") or "")
        except (OSError, ValueError):
            return ""

    def start(self):
        GLib = self.GLib
        config = common.load_config()
        if not config["_enabled"]:
            return 0

        self.evaluate()
        self.pending_token = self.read_pending_token()

        self._install_monitor()
        GLib.timeout_add_seconds(POLL_SECONDS, self._poll)
        try:
            self.loop.run()
        except KeyboardInterrupt:
            pass
        return 0

    def _install_monitor(self):
        Gio = self.Gio
        try:
            gfile = Gio.File.new_for_path(common.SYSTEM_STATE_DIR)
            self.monitor = gfile.monitor_directory(Gio.FileMonitorFlags.NONE, None)
            self.monitor.connect("changed", self._on_dir_changed)
        except Exception:
            self.monitor = None

    def _on_dir_changed(self, *_args):
        # Debounce: the root hook writes a temp file and renames it, so several
        # events arrive in bursts. Reset a single countdown on each event.
        GLib = self.GLib
        if self.debounce_id:
            try:
                GLib.source_remove(self.debounce_id)
            except Exception:
                pass
        self.debounce_id = GLib.timeout_add(DEBOUNCE_MS, self._on_debounce)
        return False

    def _on_debounce(self):
        self.debounce_id = 0
        new_token = self.read_pending_token()
        if new_token and new_token != getattr(self, "pending_token", ""):
            self.pending_token = new_token
            self.evaluate()
        return False

    def _poll(self):
        new_token = self.read_pending_token()
        if new_token and new_token != getattr(self, "pending_token", ""):
            self.pending_token = new_token
            self.evaluate()
        return True

    # -- logic ------------------------------------------------------------
    def evaluate(self):
        state = common.load_state()

        if state.get("disabled"):
            return

        notes = common.load_notes()
        if not notes:
            return

        if not state.get("seeded"):
            if common.fresh_install_signal():
                state["seen"] = [n["id"] for n in notes]
                state["seeded"] = True
                common.save_state(state)
                return
            # Existing system installing the feature for the first time: show
            # only the newest note, treat the rest as history.
            state["seen"] = [n["id"] for n in notes[1:]]
            state["seeded"] = True
            common.save_state(state)

        seen = set(state.get("seen", []))
        unseen = [n for n in notes if n["id"] not in seen]
        if not unseen:
            return

        if not common.is_graphical():
            # No graphical session right now (e.g. an update from a TTY).
            # Leave the notes unseen so the next graphical login shows them.
            return

        if self.launch([n["id"] for n in unseen]):
            state["seen"] = sorted(seen | {n["id"] for n in unseen})
            state["last_shown"] = common.now()
            common.save_state(state)

    def launch(self, ids):
        try:
            env = dict(os.environ)
            if not env.get("WAYLAND_DISPLAY"):
                socket = self._detect_wayland_socket()
                if socket:
                    env["WAYLAND_DISPLAY"] = socket
            subprocess.Popen(
                [VIEWER, "--ids", ",".join(ids)],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            return True
        except Exception:
            return False

    def _detect_wayland_socket(self):
        runtime = os.environ.get("XDG_RUNTIME_DIR")
        if not runtime or not os.path.isdir(runtime):
            return None
        try:
            for entry in sorted(os.listdir(runtime)):
                if entry.startswith("wayland-") and not entry.endswith(".lock"):
                    return entry
        except OSError:
            return None
        return None


def _acquire_lock():
    """Return a locked file object, or None if another watcher is running."""
    import fcntl

    try:
        os.makedirs(common.user_state_dir(), exist_ok=True)
        path = os.path.join(common.user_state_dir(), "watcher.lock")
        handle = open(path, "w")
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return handle
    except OSError:
        return None


def main():
    try:
        from gi.repository import GLib  # noqa: F401
    except Exception as exc:  # pragma: no cover
        print(f"pulsaros-whatsnew-watch: GTK/GLib not available: {exc}", file=sys.stderr)
        return 0

    # Single instance per user session.
    lock = _acquire_lock()
    if lock is None:
        return 0
    try:
        return Watcher().start()
    finally:
        try:
            lock.close()
        except Exception:
            pass


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    sys.exit(main())
