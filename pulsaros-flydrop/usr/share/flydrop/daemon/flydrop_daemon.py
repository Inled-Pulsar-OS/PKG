"""
FlyDrop Main Daemon Service
Integrates UDP Discovery, HTTP/HTTPS LocalSend v2 Server, D-Bus Interface,
AyatanaAppIndicator3 System Tray Icon, and Visual Dialog Launchers.
"""

import os
import sys
import time
import json
import logging
import threading
import subprocess

# Ensure base paths are in sys.path
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

import gi
gi.require_version("GLib", "2.0")
gi.require_version("Gio", "2.0")
gi.require_version("Gtk", "3.0")

try:
    gi.require_version("AyatanaAppIndicator3", "0.1")
    from gi.repository import AyatanaAppIndicator3 as AppIndicator
except (ValueError, ImportError):
    try:
        gi.require_version("AppIndicator3", "0.1")
        from gi.repository import AppIndicator3 as AppIndicator
    except (ValueError, ImportError):
        AppIndicator = None

from gi.repository import GLib, Gio, Gtk

import dbus
import dbus.service
import dbus.mainloop.glib

from daemon.config import Config
from daemon.discovery import DeviceDiscovery
from daemon.localsend_protocol import LocalSendServer, LocalSendClient
from ui.i18n import _

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] [%(levelname)s]: %(message)s")
logger = logging.getLogger("FlyDrop.Daemon")

DBUS_SERVICE_NAME = "es.pulsaros.FlyDrop"
DBUS_OBJECT_PATH = "/es/pulsaros/FlyDrop"
DBUS_INTERFACE_NAME = "es.pulsaros.FlyDrop"


def get_gui_env():
    env = dict(os.environ)
    if "DISPLAY" not in env:
        env["DISPLAY"] = ":0"
    if "WAYLAND_DISPLAY" not in env:
        xdg_runtime = env.get("XDG_RUNTIME_DIR", "/run/user/1000")
        if os.path.exists(os.path.join(xdg_runtime, "wayland-0")):
            env["WAYLAND_DISPLAY"] = "wayland-0"
    return env


class FlyDropDBusService(dbus.service.Object):
    def __init__(self, bus, daemon_instance):
        self.daemon = daemon_instance
        bus_name = dbus.service.BusName(DBUS_SERVICE_NAME, bus)
        super().__init__(bus_name, DBUS_OBJECT_PATH)

    @dbus.service.method(DBUS_INTERFACE_NAME, out_signature="s")
    def GetDiscoveredDevices(self):
        devices = self.daemon.discovery.get_devices()
        return json.dumps(devices, ensure_ascii=False)

    @dbus.service.method(DBUS_INTERFACE_NAME, in_signature="ss", out_signature="s")
    def SendFiles(self, target_ip, file_paths_json):
        try:
            file_paths = json.loads(file_paths_json)
            session_id = self.daemon.send_files(target_ip, file_paths)
            return session_id
        except Exception as e:
            logger.error(f"Error in SendFiles D-Bus call: {e}")
            return f"error: {e}"

    @dbus.service.method(DBUS_INTERFACE_NAME, in_signature="ss", out_signature="s")
    def SendText(self, target_ip, text):
        try:
            session_id = self.daemon.send_text(target_ip, text)
            return session_id
        except Exception as e:
            logger.error(f"Error in SendText D-Bus call: {e}")
            return f"error: {e}"

    @dbus.service.method(DBUS_INTERFACE_NAME, out_signature="s")
    def GetConfig(self):
        return json.dumps(self.daemon.config.data, ensure_ascii=False)

    @dbus.service.method(DBUS_INTERFACE_NAME, in_signature="ss", out_signature="b")
    def SetConfig(self, key, value_json):
        try:
            val = json.loads(value_json)
            self.daemon.config.set_value(key, val)
            if key == "auto_accept" and self.daemon.indicator_auto_accept_item:
                self.daemon.indicator_auto_accept_item.set_active(bool(val))
            return True
        except Exception as e:
            logger.error(f"Error in SetConfig D-Bus call: {e}")
            return False

    @dbus.service.method(DBUS_INTERFACE_NAME, in_signature="sb", out_signature="b")
    def RespondTransfer(self, session_id, accept):
        return self.daemon.respond_transfer(session_id, accept)

    @dbus.service.method(DBUS_INTERFACE_NAME, in_signature="s", out_signature="s")
    def GetTransferStatus(self, session_id):
        status_info = self.daemon.transfer_states.get(session_id, {
            "session_id": session_id,
            "status": "unknown",
            "progress": 0.0,
            "speed_str": "",
            "current_file": "",
            "total_bytes_str": "",
            "success": False,
            "message": ""
        })
        return json.dumps(status_info, ensure_ascii=False)

    @dbus.service.method(DBUS_INTERFACE_NAME, out_signature="b")
    def TriggerScan(self):
        self.daemon.discovery.send_announcement(is_announcement=False)
        return True

    @dbus.service.method(DBUS_INTERFACE_NAME, out_signature="b")
    def OpenDownloadsFolder(self):
        dest = self.daemon.config.download_dir
        try:
            subprocess.Popen(["xdg-open", dest], env=get_gui_env())
            return True
        except Exception as e:
            logger.error(f"Error opening downloads folder: {e}")
            return False

    @dbus.service.method(DBUS_INTERFACE_NAME, in_signature="s", out_signature="b")
    def OpenSendDialog(self, file_paths_json):
        ui_script = os.path.join(BASE_DIR, "ui", "send_dialog.py")
        if not os.path.exists(ui_script):
            ui_script = "/usr/share/flydrop/ui/send_dialog.py"
        try:
            subprocess.Popen([sys.executable, ui_script, "--files", file_paths_json], env=get_gui_env())
            return True
        except Exception as e:
            logger.error(f"Error opening Send Dialog: {e}")
            return False

    @dbus.service.method(DBUS_INTERFACE_NAME, out_signature="b")
    def OpenSettingsDialog(self):
        ui_script = os.path.join(BASE_DIR, "ui", "settings_dialog.py")
        if not os.path.exists(ui_script):
            ui_script = "/usr/share/flydrop/ui/settings_dialog.py"
        try:
            subprocess.Popen([sys.executable, ui_script], env=get_gui_env())
            return True
        except Exception as e:
            logger.error(f"Error opening Settings Dialog: {e}")
            return False

    # Signals
    @dbus.service.signal(DBUS_INTERFACE_NAME, signature="s")
    def DeviceFound(self, device_json):
        pass

    @dbus.service.signal(DBUS_INTERFACE_NAME, signature="s")
    def DeviceLost(self, fingerprint):
        pass

    @dbus.service.signal(DBUS_INTERFACE_NAME, signature="ssss")
    def IncomingTransferRequest(self, session_id, sender_alias, files_summary, total_size_str):
        pass

    @dbus.service.signal(DBUS_INTERFACE_NAME, signature="sdsss")
    def TransferProgress(self, session_id, progress, speed_str, current_file, total_bytes_str):
        pass

    @dbus.service.signal(DBUS_INTERFACE_NAME, signature="sbs")
    def TransferCompleted(self, session_id, success, message):
        pass


class FlyDropDaemon:
    def __init__(self):
        self.config = Config.get()
        self.pending_decisions = {}
        self.active_transfers = {}
        self.transfer_states = {}
        self.dbus_service = None
        self.main_loop = None
        self.indicator = None
        self.indicator_auto_accept_item = None

        self.discovery = DeviceDiscovery(
            on_device_found=self._on_device_found,
            on_device_lost=self._on_device_lost
        )

        self.server = LocalSendServer(
            discovery_service=self.discovery,
            on_transfer_request=self._on_incoming_transfer_request,
            on_progress=self._on_transfer_progress,
            on_completed=self._on_transfer_completed
        )

        self.client = LocalSendClient()

    def _setup_indicator(self):
        if not AppIndicator:
            logger.warning("AyatanaAppIndicator3 / AppIndicator3 not available, skipping panel tray indicator")
            return

        try:
            self.indicator = AppIndicator.Indicator.new(
                "flydrop-indicator",
                "flydrop-symbolic",
                AppIndicator.IndicatorCategory.APPLICATION_STATUS
            )
            self.indicator.set_status(AppIndicator.IndicatorStatus.ACTIVE)

            menu = Gtk.Menu()

            # 1. Receive without asking (checkbox)
            self.indicator_auto_accept_item = Gtk.CheckMenuItem(label=_("receive_without_asking"))
            self.indicator_auto_accept_item.set_active(self.config.auto_accept)
            self.indicator_auto_accept_item.connect("toggled", self._on_indicator_auto_accept_toggled)
            menu.append(self.indicator_auto_accept_item)

            # Separator
            menu.append(Gtk.SeparatorMenuItem())

            # 2. Open Downloads
            open_downloads_item = Gtk.MenuItem(label=_("open_downloads"))
            open_downloads_item.connect("activate", lambda w: self._open_downloads())
            menu.append(open_downloads_item)

            # 3. Settings
            settings_item = Gtk.MenuItem(label=_("settings"))
            settings_item.connect("activate", lambda w: self._open_settings())
            menu.append(settings_item)

            # Separator
            menu.append(Gtk.SeparatorMenuItem())

            # 4. Quit
            quit_item = Gtk.MenuItem(label=_("quit"))
            quit_item.connect("activate", lambda w: self.shutdown())
            menu.append(quit_item)

            menu.show_all()
            self.indicator.set_menu(menu)
            logger.info("AyatanaAppIndicator tray icon initialized successfully")
        except Exception as e:
            logger.error(f"Failed to setup AppIndicator: {e}")

    def _on_indicator_auto_accept_toggled(self, widget):
        new_val = widget.get_active()
        self.config.set_value("auto_accept", new_val)
        logger.info(f"Auto-accept changed via indicator: {new_val}")

    def _open_downloads(self):
        dest = self.config.download_dir
        try:
            subprocess.Popen(["xdg-open", dest], env=get_gui_env())
        except Exception as e:
            logger.error(f"Error opening downloads folder: {e}")

    def _open_settings(self):
        ui_script = os.path.join(BASE_DIR, "ui", "settings_dialog.py")
        if not os.path.exists(ui_script):
            ui_script = "/usr/share/flydrop/ui/settings_dialog.py"
        try:
            subprocess.Popen([sys.executable, ui_script], env=get_gui_env())
        except Exception as e:
            logger.error(f"Error opening settings dialog: {e}")

    def _on_device_found(self, device):
        logger.info(f"Device found: {device.get('alias')} ({device.get('ip')}) [{device.get('deviceModel')}]")
        if self.dbus_service:
            GLib.idle_add(self.dbus_service.DeviceFound, json.dumps(device, ensure_ascii=False))

    def _on_device_lost(self, device):
        logger.info(f"Device lost: {device.get('alias')} ({device.get('fingerprint')})")
        if self.dbus_service:
            GLib.idle_add(self.dbus_service.DeviceLost, device.get("fingerprint"))

    def _on_incoming_transfer_request(self, session, decision_callback=None, accepted=False):
        sender_alias = session.sender_info.get("alias", "Dispositivo LocalSend")
        files_count = len(session.files_dict)
        first_file = next(iter(session.files_dict.values()), {}).get("fileName", "Archivo")
        summary = f"{first_file}" if files_count == 1 else f"{first_file} (+{files_count - 1})"
        size_mb = session.total_size / (1024 * 1024)
        size_str = f"{size_mb:.1f} MB" if size_mb >= 1.0 else f"{session.total_size / 1024:.1f} KB"

        logger.info(f"Incoming transfer request from '{sender_alias}': {summary} ({size_str})")

        self.transfer_states[session.session_id] = {
            "session_id": session.session_id,
            "status": "in_progress" if accepted else "prompt",
            "progress": 0.0,
            "speed_str": "0 KB/s",
            "current_file": summary,
            "total_bytes_str": size_str,
            "success": False,
            "message": ""
        }

        if accepted:
            self._launch_transfer_ui(session.session_id, sender_alias, summary, is_incoming=True)
            return

        if decision_callback:
            self.pending_decisions[session.session_id] = decision_callback

        if self.dbus_service:
            GLib.idle_add(
                self.dbus_service.IncomingTransferRequest,
                session.session_id,
                sender_alias,
                summary,
                size_str
            )

        # Launch visual prompt dialog without showing GNOME system notifications
        prompt_script = os.path.join(BASE_DIR, "ui", "prompt_dialog.py")
        if not os.path.exists(prompt_script):
            prompt_script = "/usr/share/flydrop/ui/prompt_dialog.py"
        try:
            subprocess.Popen([
                sys.executable, prompt_script,
                "--session", session.session_id,
                "--sender", sender_alias,
                "--summary", summary,
                "--size", size_str
            ], env=get_gui_env())
        except Exception as e:
            logger.error(f"Error launching prompt dialog: {e}")

    def respond_transfer(self, session_id, accept):
        cb = self.pending_decisions.pop(session_id, None)
        if cb:
            cb(accept)
            if accept:
                session = self.server.active_sessions.get(session_id)
                sender_alias = session.sender_info.get("alias", "Dispositivo") if session else "Remoto"
                summary = "Recibiendo archivos..."
                if session_id in self.transfer_states:
                    self.transfer_states[session_id]["status"] = "in_progress"
                self._launch_transfer_ui(session_id, sender_alias, summary, is_incoming=True)
            else:
                if session_id in self.transfer_states:
                    self.transfer_states[session_id]["status"] = "rejected"
            return True
        return False

    def _on_transfer_progress(self, session):
        # Nunca por encima de 1: los reintentos de un archivo descartan el
        # progreso anterior, pero por si acaso se salta el techo.
        fraction = session.transferred_bytes / session.total_size if session.total_size > 0 else 0.0
        fraction = min(1.0, fraction)
        speed_mb = session.speed / (1024 * 1024)
        speed_str = f"{speed_mb:.1f} MB/s" if speed_mb >= 1.0 else f"{session.speed / 1024:.0f} KB/s"
        total_mb = session.total_size / (1024 * 1024)
        total_str = f"{total_mb:.1f} MB"

        self.transfer_states[session.session_id] = {
            "session_id": session.session_id,
            "status": "in_progress",
            "progress": float(fraction),
            "speed_str": speed_str,
            "current_file": session.current_file_name,
            "total_bytes_str": total_str,
            "success": False,
            "message": ""
        }

        if self.dbus_service:
            GLib.idle_add(
                self.dbus_service.TransferProgress,
                session.session_id,
                float(fraction),
                speed_str,
                session.current_file_name,
                total_str
            )

    def _on_transfer_completed(self, session, success, message):
        logger.info(f"Transfer {session.session_id} completed: success={success}, msg={message}")

        self.transfer_states[session.session_id] = {
            "session_id": session.session_id,
            "status": "completed" if success else "failed",
            "progress": 1.0 if success else 0.0,
            "speed_str": "",
            "current_file": session.current_file_name,
            "total_bytes_str": "",
            "success": bool(success),
            "message": str(message)
        }

        if self.dbus_service:
            GLib.idle_add(
                self.dbus_service.TransferCompleted,
                session.session_id,
                success,
                message
            )

        # If incoming transfer was successful, auto-open the received file
        if success and session.is_incoming and session.saved_files:
            last_file = session.saved_files[-1]
            try:
                subprocess.Popen(["xdg-open", last_file], env=get_gui_env())
            except Exception as e:
                logger.error(f"Error auto-opening received file: {e}")

    def _launch_transfer_ui(self, session_id, peer_name, summary, is_incoming=True):
        ui_script = os.path.join(BASE_DIR, "ui", "transfer_window.py")
        if not os.path.exists(ui_script):
            ui_script = "/usr/share/flydrop/ui/transfer_window.py"

        mode = "incoming" if is_incoming else "outgoing"
        env = get_gui_env()

        try:
            subprocess.Popen([
                sys.executable, ui_script,
                "--session", session_id,
                "--peer", peer_name,
                "--summary", summary,
                "--mode", mode
            ], env=env)
        except Exception as e:
            logger.error(f"Error launching transfer UI: {e}")

    def send_files(self, target_ip_or_id, file_paths):
        target = None
        devices = self.discovery.get_devices()
        for d in devices:
            if d.get("ip") == target_ip_or_id or d.get("fingerprint") == target_ip_or_id:
                target = d
                break

        if not target:
            target = {"ip": target_ip_or_id, "port": 53317, "alias": target_ip_or_id, "protocol": "https"}

        session_id = f"out-{int(time.time()*1000)}"
        peer_name = target.get("alias", "Dispositivo")
        summary = f"{len(file_paths)} archivo(s)" if len(file_paths) > 1 else os.path.basename(file_paths[0])
        target_proto = target.get("protocol", "https")

        self.transfer_states[session_id] = {
            "session_id": session_id,
            "status": "in_progress",
            "progress": 0.0,
            "speed_str": "0 KB/s",
            "current_file": summary,
            "total_bytes_str": "",
            "success": False,
            "message": ""
        }

        self._launch_transfer_ui(session_id, peer_name, summary, is_incoming=False)

        def on_prog(fraction, speed_bps, current_file, transferred, total):
            fraction = min(1.0, float(fraction))
            speed_mb = speed_bps / (1024 * 1024)
            speed_str = f"{speed_mb:.1f} MB/s" if speed_mb >= 1.0 else f"{speed_bps / 1024:.0f} KB/s"
            total_mb = total / (1024 * 1024)
            total_str = f"{total_mb:.1f} MB"

            self.transfer_states[session_id] = {
                "session_id": session_id,
                "status": "in_progress",
                "progress": float(fraction),
                "speed_str": speed_str,
                "current_file": current_file,
                "total_bytes_str": total_str,
                "success": False,
                "message": ""
            }

            if self.dbus_service:
                GLib.idle_add(
                    self.dbus_service.TransferProgress,
                    session_id,
                    float(fraction),
                    speed_str,
                    current_file,
                    total_str
                )

        def on_comp(success, msg):
            self.transfer_states[session_id] = {
                "session_id": session_id,
                "status": "completed" if success else "failed",
                "progress": 1.0 if success else 0.0,
                "speed_str": "",
                "current_file": "",
                "total_bytes_str": "",
                "success": bool(success),
                "message": str(msg)
            }

            if self.dbus_service:
                GLib.idle_add(
                    self.dbus_service.TransferCompleted,
                    session_id,
                    bool(success),
                    str(msg)
                )

        self.client.send_files_async(
            target.get("ip"),
            target.get("port", 53317),
            file_paths,
            target_protocol=target_proto,
            on_progress=on_prog,
            on_completed=on_comp
        )
        return session_id

    def send_text(self, target_ip_or_id, text):
        target = None
        devices = self.discovery.get_devices()
        for d in devices:
            if d.get("ip") == target_ip_or_id or d.get("fingerprint") == target_ip_or_id:
                target = d
                break

        if not target:
            target = {"ip": target_ip_or_id, "port": 53317, "alias": target_ip_or_id, "protocol": "https"}

        session_id = f"out-txt-{int(time.time()*1000)}"
        peer_name = target.get("alias", "Dispositivo")
        summary = f"Texto ({len(text)} caracteres)"
        target_proto = target.get("protocol", "https")

        self.transfer_states[session_id] = {
            "session_id": session_id,
            "status": "in_progress",
            "progress": 0.0,
            "speed_str": "0 KB/s",
            "current_file": summary,
            "total_bytes_str": "",
            "success": False,
            "message": ""
        }

        self._launch_transfer_ui(session_id, peer_name, summary, is_incoming=False)

        def on_prog(fraction, speed_bps, current_file, transferred, total):
            fraction = min(1.0, float(fraction))
            self.transfer_states[session_id] = {
                "session_id": session_id,
                "status": "in_progress",
                "progress": float(fraction),
                "speed_str": "0 KB/s",
                "current_file": current_file,
                "total_bytes_str": f"{total} B",
                "success": False,
                "message": ""
            }
            if self.dbus_service:
                GLib.idle_add(
                    self.dbus_service.TransferProgress,
                    session_id,
                    float(fraction),
                    "0 KB/s",
                    current_file,
                    f"{total} B"
                )

        def on_comp(success, msg):
            self.transfer_states[session_id] = {
                "session_id": session_id,
                "status": "completed" if success else "failed",
                "progress": 1.0 if success else 0.0,
                "speed_str": "",
                "current_file": "",
                "total_bytes_str": "",
                "success": bool(success),
                "message": str(msg)
            }
            if self.dbus_service:
                GLib.idle_add(
                    self.dbus_service.TransferCompleted,
                    session_id,
                    bool(success),
                    str(msg)
                )

        self.client.send_text_async(
            target.get("ip"),
            target.get("port", 53317),
            text,
            target_protocol=target_proto,
            on_progress=on_prog,
            on_completed=on_comp
        )
        return session_id

    def run(self):
        dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
        session_bus = dbus.SessionBus()

        # Single-instance guard. The daemon is started by both systemd and an
        # autostart .desktop, so without this the second start finds the discovery
        # socket and port 53317 already taken, falls through to the next free port
        # and registers a duplicate tray icon.
        #
        # This has to run after DBusGMainLoop is set as the default main loop:
        # SessionBus() caches one shared connection, and building it before the
        # mainloop is set leaves that connection without a main loop, so the
        # later add_match / async call in discovery.start() raises.
        try:
            if session_bus.name_has_owner(DBUS_SERVICE_NAME):
                logger.info(
                    "Another FlyDrop daemon already owns %s, exiting.",
                    DBUS_SERVICE_NAME,
                )
                return
        except Exception as e:
            logger.warning("Could not check for an existing FlyDrop instance: %s", e)

        self.dbus_service = FlyDropDBusService(session_bus, self)

        self.discovery.start()
        self.server.start()
        self._setup_indicator()

        logger.info(f"FlyDrop Daemon started successfully as '{self.config.alias}' on port {self.config.port}")

        self.main_loop = GLib.MainLoop()
        try:
            self.main_loop.run()
        except KeyboardInterrupt:
            self.shutdown()

    def shutdown(self):
        logger.info("Shutting down FlyDrop Daemon...")
        self.discovery.stop()
        self.server.stop()
        if self.main_loop:
            self.main_loop.quit()


def main():
    daemon = FlyDropDaemon()
    daemon.run()


if __name__ == "__main__":
    main()
