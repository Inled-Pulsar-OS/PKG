"""Pure GTK3 Tray AppIndicator for Sayri.

Provides a system tray indicator with fast toggle, mode selection,
settings launcher, and clean lifecycle management.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import threading

import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk

try:
    gi.require_version("AyatanaAppIndicator3", "0.1")
    from gi.repository import AyatanaAppIndicator3 as AppIndicator
except Exception:
    try:
        gi.require_version("AppIndicator3", "0.1")
        from gi.repository import AppIndicator3 as AppIndicator
    except Exception:
        AppIndicator = None

from sayri import config, paths

Gtk.init(sys.argv)


def send_sock_command(cmd: str) -> bool:
    sock_path = os.path.join(paths.state_dir(), "sayri.sock")
    if not os.path.exists(sock_path):
        return False
    try:
        import socket
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(1.0)
        s.connect(sock_path)
        s.sendall(f"{cmd}\n".encode("utf-8"))
        s.recv(1024)
        s.close()
        return True
    except Exception:
        return False


class SayriIndicator:
    def __init__(self) -> None:
        self.cfg = config.config
        self._sayri_proc: subprocess.Popen | None = None
        self._settings_proc: subprocess.Popen | None = None

        self._create_menu()
        self._create_indicator()

    def _create_menu(self) -> None:
        self.menu = Gtk.Menu()

        # Title / Status
        self.title_item = Gtk.MenuItem(label="Sayri Assistant")
        self.title_item.set_sensitive(False)
        self.menu.append(self.title_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # Toggle Sayri
        self.toggle_item = Gtk.MenuItem(label="Toggle Sayri (Show / Hide)")
        self.toggle_item.connect("activate", self._on_toggle_sayri)
        self.menu.append(self.toggle_item)

        self.listen_item = Gtk.MenuItem(label="🎙️ Ask Sayri (Listen)")
        self.listen_item.connect("activate", self._on_listen)
        self.menu.append(self.listen_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # Mode Submenu
        mode_menu = Gtk.Menu()
        mode_item = Gtk.MenuItem(label="Listening Mode")
        mode_item.set_submenu(mode_menu)

        curr_mode = self.cfg.get_string("stt", "mode")

        self.radio_wake = Gtk.RadioMenuItem(label="Wake Word (Hey Sayri)")
        self.radio_wake.set_active(curr_mode == "wakeword")
        self.radio_wake.connect("toggled", lambda w: w.get_active() and self._set_mode("wakeword"))
        mode_menu.append(self.radio_wake)

        self.radio_always = Gtk.RadioMenuItem.new_from_widget(self.radio_wake)
        self.radio_always.set_label("Always Listening")
        self.radio_always.set_active(curr_mode == "always")
        self.radio_always.connect("toggled", lambda w: w.get_active() and self._set_mode("always"))
        mode_menu.append(self.radio_always)

        self.radio_manual = Gtk.RadioMenuItem.new_from_widget(self.radio_wake)
        self.radio_manual.set_label("Manual (Click to talk)")
        self.radio_manual.set_active(curr_mode == "manual")
        self.radio_manual.connect("toggled", lambda w: w.get_active() and self._set_mode("manual"))
        mode_menu.append(self.radio_manual)

        self.menu.append(mode_item)

        # Settings
        self.settings_item = Gtk.MenuItem(label="⚙ Settings…")
        self.settings_item.connect("activate", self._on_open_settings)
        self.menu.append(self.settings_item)

        self.menu.append(Gtk.SeparatorMenuItem())

        # Quit
        self.quit_item = Gtk.MenuItem(label="Quit Sayri")
        self.quit_item.connect("activate", self._on_quit)
        self.menu.append(self.quit_item)

        self.menu.show_all()

    def _create_indicator(self) -> None:
        if AppIndicator is not None:
            self.indicator = AppIndicator.Indicator.new(
                "sayri-indicator",
                "microphone-sensitivity-high-symbolic",
                AppIndicator.IndicatorCategory.APPLICATION_STATUS,
            )
            self.indicator.set_status(AppIndicator.IndicatorStatus.ACTIVE)
            self.indicator.set_title("Sayri")
            self.indicator.set_menu(self.menu)
        else:
            self.status_icon = Gtk.StatusIcon.new_from_icon_name("microphone-sensitivity-high-symbolic")
            self.status_icon.set_tooltip_text("Sayri Voice Assistant")
            self.status_icon.connect("popup-menu", lambda _i, btn, time: self.menu.popup(None, None, None, None, btn, time))
            self.status_icon.connect("activate", lambda _i: self._on_toggle_sayri(None))

    def _set_mode(self, mode: str) -> None:
        self.cfg.set("stt", "mode", mode)

    def _on_toggle_sayri(self, _item) -> None:
        if not send_sock_command("toggle"):
            self._ensure_sayri()

    def _on_listen(self, _item) -> None:
        if not send_sock_command("listen"):
            self._ensure_sayri()

    def _ensure_sayri(self) -> None:
        if self._sayri_proc and self._sayri_proc.poll() is None:
            return
        env = dict(os.environ)
        lib_path = os.path.dirname(os.path.dirname(__file__))
        env["PYTHONPATH"] = lib_path + (":" + env["PYTHONPATH"] if "PYTHONPATH" in env else "")
        self._sayri_proc = subprocess.Popen([sys.executable, "-m", "sayri"], env=env)

    def _on_open_settings(self, _item) -> None:
        if self._settings_proc and self._settings_proc.poll() is None:
            return
        env = dict(os.environ)
        lib_path = os.path.dirname(os.path.dirname(__file__))
        env["PYTHONPATH"] = lib_path + (":" + env["PYTHONPATH"] if "PYTHONPATH" in env else "")
        env.pop("LD_PRELOAD", None)
        self._settings_proc = subprocess.Popen([sys.executable, "-m", "sayri.settings_gtk3"], env=env)

    def _on_quit(self, _item) -> None:
        send_sock_command("quit")
        if self._sayri_proc and self._sayri_proc.poll() is None:
            self._sayri_proc.terminate()
        if self._settings_proc and self._settings_proc.poll() is None:
            self._settings_proc.terminate()
        Gtk.main_quit()


def main() -> None:
    app = SayriIndicator()
    Gtk.main()


if __name__ == "__main__":
    main()
