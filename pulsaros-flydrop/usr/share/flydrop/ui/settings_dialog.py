#!/usr/bin/env python3
"""
FlyDrop Settings / Preferences Window (GTK4 / Libadwaita)
Allows user to rename device, toggle auto-accept direct downloads, and view/manage nearby devices.
"""

import sys
import os
import json

# Add project root to sys.path
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
gi.require_version("GLib", "2.0")
from gi.repository import Gtk, Adw, Gio, GLib

import dbus
import dbus.mainloop.glib
dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

from daemon.config import Config
from ui.i18n import _


class SettingsWindow(Adw.PreferencesWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.config = Config.get()
        self.set_title(_("settings_title"))
        self.set_default_size(520, 520)

        # Main Page
        page = Adw.PreferencesPage()
        self.add(page)

        # 1. General Group
        group_general = Adw.PreferencesGroup()
        group_general.set_title(_("general_section"))
        page.add(group_general)

        # Name Entry Row
        self.name_row = Adw.EntryRow()
        self.name_row.set_title(_("device_name"))
        self.name_row.set_text(self.config.alias)
        self.name_row.connect("apply", self.on_name_changed)
        self.name_row.connect("notify::text", self.on_name_text_changed)
        group_general.add(self.name_row)

        # 2. Receiving Group
        group_receiving = Adw.PreferencesGroup()
        group_receiving.set_title(_("receiving_section"))
        page.add(group_receiving)

        # Auto Accept Switch Row
        self.auto_accept_row = Adw.SwitchRow()
        self.auto_accept_row.set_title(_("auto_accept"))
        self.auto_accept_row.set_subtitle(_("auto_accept_subtitle"))
        self.auto_accept_row.set_active(self.config.auto_accept)
        self.auto_accept_row.connect("notify::active", self.on_auto_accept_toggled)
        group_receiving.add(self.auto_accept_row)

        # Auto Open Switch Row
        self.auto_open_row = Adw.SwitchRow()
        self.auto_open_row.set_title(_("auto_open"))
        self.auto_open_row.set_subtitle(_("auto_open_subtitle"))
        self.auto_open_row.set_active(self.config.auto_open)
        self.auto_open_row.connect("notify::active", self.on_auto_open_toggled)
        group_receiving.add(self.auto_open_row)

        # Download Directory Action Row
        self.dir_row = Adw.ActionRow()
        self.dir_row.set_title(_("save_destination"))
        self.dir_row.set_subtitle(self.config.download_dir)
        self.dir_row.set_activatable(True)

        change_folder_btn = Gtk.Button(label=_("settings"))
        change_folder_btn.set_valign(Gtk.Align.CENTER)
        change_folder_btn.connect("clicked", self.on_choose_folder_clicked)
        self.dir_row.add_suffix(change_folder_btn)
        group_receiving.add(self.dir_row)

        # 3. Discovered Devices Group
        group_devices = Adw.PreferencesGroup()
        group_devices.set_title(_("nearby_devices"))
        group_devices.set_description(_("nearby_devices_subtitle"))
        page.add(group_devices)

        self.devices_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        group_devices.add(self.devices_box)

        self._setup_dbus()
        self._refresh_devices_list()

        # Trigger initial scan and schedule periodic refresh
        self._trigger_scan()
        self._poll_timer_id = GLib.timeout_add_seconds(3, self._refresh_devices_list_timer)

    def _setup_dbus(self):
        try:
            bus = dbus.SessionBus()
            bus.add_signal_receiver(
                self._on_device_signal,
                signal_name="DeviceFound",
                dbus_interface="es.pulsaros.FlyDrop"
            )
            bus.add_signal_receiver(
                self._on_device_signal,
                signal_name="DeviceLost",
                dbus_interface="es.pulsaros.FlyDrop"
            )
        except Exception as e:
            print(f"Error setting up D-Bus signal receivers in Settings: {e}", file=sys.stderr)

    def _on_device_signal(self, *args):
        GLib.idle_add(self._refresh_devices_list)

    def _refresh_devices_list_timer(self):
        self._refresh_devices_list()
        return GLib.SOURCE_CONTINUE

    def _trigger_scan(self):
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            iface.TriggerScan()
        except Exception:
            pass

    def on_name_text_changed(self, entry, pspec):
        text = entry.get_text().strip()
        if text:
            self.config.alias = text
            self._notify_dbus_config("alias", text)

    def on_name_changed(self, entry):
        text = entry.get_text().strip()
        if text:
            self.config.alias = text
            self._notify_dbus_config("alias", text)

    def on_auto_accept_toggled(self, switch, pspec):
        val = switch.get_active()
        self.config.auto_accept = val
        self._notify_dbus_config("auto_accept", val)

    def on_auto_open_toggled(self, switch, pspec):
        val = switch.get_active()
        self.config.auto_open = val
        self._notify_dbus_config("auto_open", val)

    def on_choose_folder_clicked(self, btn):
        dialog = Gtk.FileDialog()
        dialog.set_title(_("save_destination"))
        dialog.select_folder(self, None, self.on_folder_selected)

    def on_folder_selected(self, dialog, result):
        try:
            folder = dialog.select_folder_finish(result)
            if folder:
                path = folder.get_path()
                self.config.download_dir = path
                self.dir_row.set_subtitle(path)
                self._notify_dbus_config("download_dir", path)
        except Exception:
            pass

    def _notify_dbus_config(self, key, value):
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            iface.SetConfig(key, json.dumps(value))
        except Exception:
            pass

    def _refresh_devices_list(self):
        while child := self.devices_box.get_first_child():
            self.devices_box.remove(child)

        devices = []
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            raw = iface.GetDiscoveredDevices()
            devices = json.loads(raw)
        except Exception:
            pass

        if not devices:
            empty_row = Adw.ActionRow()
            empty_row.set_title(_("no_devices"))
            empty_row.set_subtitle(_("scanning"))
            self.devices_box.append(empty_row)
            return

        for dev in devices:
            row = Adw.ActionRow()
            row.set_title(dev.get("alias", "Dispositivo"))
            model = dev.get("deviceModel", "LocalSend")
            ip = dev.get("ip", "")
            row.set_subtitle(f"{ip} • {model}")

            dtype = dev.get("deviceType", "desktop")
            icon_name = "phone-symbolic" if dtype == "mobile" else "computer-symbolic"
            row.add_prefix(Gtk.Image.new_from_icon_name(icon_name))

            send_btn = Gtk.Button(label=_("send_file"))
            send_btn.set_valign(Gtk.Align.CENTER)
            send_btn.add_css_class("flat")
            send_btn.connect("clicked", lambda b, d=dev: self._open_file_chooser_for_device(d))
            row.add_suffix(send_btn)

            self.devices_box.append(row)

    def _open_file_chooser_for_device(self, dev):
        dialog = Gtk.FileDialog()
        dialog.set_title(f"{_('send_file')} → {dev.get('alias')}")
        dialog.open_multiple(self, None, lambda d, res: self._on_files_selected_for_send(d, res, dev))

    def _on_files_selected_for_send(self, dialog, result, dev):
        try:
            files_list = dialog.open_multiple_finish(result)
            if files_list:
                paths = [f.get_path() for f in files_list if f.get_path()]
                if paths:
                    bus = dbus.SessionBus()
                    proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
                    iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
                    iface.SendFiles(dev.get("ip"), json.dumps(paths))
        except Exception as e:
            print(f"Error selecting files: {e}", file=sys.stderr)


class SettingsApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id="es.pulsaros.FlyDrop.Settings")

    def do_activate(self):
        win = SettingsWindow(self)
        win.present()


def main():
    app = SettingsApp()
    app.run(sys.argv)


if __name__ == "__main__":
    main()
