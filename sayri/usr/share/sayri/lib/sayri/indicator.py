"""Native DBus StatusNotifierItem for Sayri.

Provides a system tray indicator with official Siri iOS 2021 icon,
direct click toggle (no dropdown menus), and clean lifecycle management.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import threading

import gi

gi.require_version("Gio", "2.0")
gi.require_version("GLib", "2.0")
gi.require_version("Gtk", "3.0")
from gi.repository import Gio, GLib, Gtk

from sayri import config, paths

SNI_XML = """
<node>
  <interface name="org.kde.StatusNotifierItem">
    <property name="Category" type="s" access="read"/>
    <property name="Id" type="s" access="read"/>
    <property name="Title" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="IconName" type="s" access="read"/>
    <property name="IconThemePath" type="s" access="read"/>
    <property name="ItemIsMenu" type="b" access="read"/>
    <method name="ContextMenu">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="Activate">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="SecondaryActivate">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="Scroll">
      <arg name="delta" type="i" direction="in"/>
      <arg name="orientation" type="s" direction="in"/>
    </method>
    <signal name="NewTitle"/>
    <signal name="NewIcon"/>
    <signal name="NewStatus">
      <arg name="status" type="s"/>
    </signal>
  </interface>
</node>
"""


def _ensure_local_icon() -> str:
    user_icons = os.path.expanduser("~/.local/share/icons/hicolor")
    src = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "icons", "hicolor", "256x256", "apps", "sayri-tray.png"))
    if os.path.isfile(src):
        for sz in ["256x256", "scalable", "48x48", "32x32"]:
            d = os.path.join(user_icons, sz, "apps")
            os.makedirs(d, exist_ok=True)
            dest = os.path.join(d, "sayri-tray.png")
            if not os.path.exists(dest) or os.path.getsize(dest) != os.path.getsize(src):
                try:
                    shutil.copy2(src, dest)
                except OSError:
                    pass
    return os.path.expanduser("~/.local/share/icons")


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
        self.icon_theme_path = _ensure_local_icon()

        self._init_dbus_sni()

    def _init_dbus_sni(self) -> None:
        self.node_info = Gio.DBusNodeInfo.new_for_xml(SNI_XML)
        self.bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)

        service_name = f"org.kde.StatusNotifierItem-{os.getpid()}-1"
        self.bus.register_object(
            "/StatusNotifierItem",
            self.node_info.interfaces[0],
            self._handle_method_call,
            self._handle_get_property,
            None,
        )

        try:
            self.bus.call_sync(
                "org.kde.StatusNotifierWatcher",
                "/StatusNotifierWatcher",
                "org.kde.StatusNotifierWatcher",
                "RegisterStatusNotifierItem",
                GLib.Variant("(s)", ("/StatusNotifierItem",)),
                None,
                Gio.DBusCallFlags.NONE,
                1000,
                None,
            )
            print("[Sayri] ✓ Native StatusNotifierItem registered (Direct Click Mode)")
        except Exception as exc:
            print(f"[Sayri] StatusNotifierWatcher register warning: {exc}")

    def _handle_method_call(self, conn, sender, path, iface, method, params, invocation) -> None:
        if method in ("Activate", "SecondaryActivate", "ContextMenu"):
            self._on_toggle_sayri()
        invocation.return_value(None)

    def _handle_get_property(self, conn, sender, path, iface, prop):
        if prop == "Category":
            return GLib.Variant("s", "ApplicationStatus")
        elif prop == "Id":
            return GLib.Variant("s", "sayri")
        elif prop == "Title":
            return GLib.Variant("s", "Sayri")
        elif prop == "Status":
            return GLib.Variant("s", "Active")
        elif prop == "IconName":
            return GLib.Variant("s", "sayri-tray")
        elif prop == "IconThemePath":
            return GLib.Variant("s", self.icon_theme_path)
        elif prop == "ItemIsMenu":
            return GLib.Variant("b", False)
        return None

    def _on_toggle_sayri(self) -> None:
        if not send_sock_command("toggle"):
            self._ensure_sayri()

    def _ensure_sayri(self) -> None:
        if self._sayri_proc and self._sayri_proc.poll() is None:
            return
        env = dict(os.environ)
        lib_path = os.path.dirname(os.path.dirname(__file__))
        env["PYTHONPATH"] = lib_path + (":" + env["PYTHONPATH"] if "PYTHONPATH" in env else "")
        self._sayri_proc = subprocess.Popen([sys.executable, "-m", "sayri"], env=env)


def main() -> None:
    app = SayriIndicator()
    loop = GLib.MainLoop()
    loop.run()


if __name__ == "__main__":
    main()
