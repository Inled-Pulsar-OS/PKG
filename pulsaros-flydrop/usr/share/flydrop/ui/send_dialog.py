#!/usr/bin/env python3
"""
FlyDrop Send Target Selector (GTK4 / Libadwaita)
Allows user to pick a discovered LocalSend peer to send selected files or text.
"""

import sys
import os
import json
import argparse

# Add project root to sys.path
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, GLib, Gio

import dbus
import dbus.mainloop.glib
dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

from ui.i18n import _


class SendDialogWindow(Adw.Window):
    def __init__(self, app, files_list=None, text_content=""):
        super().__init__(application=app)
        self.files_list = files_list or []
        self.text_content = text_content

        self.set_title(_("app_name"))
        self.set_default_size(440, 480)
        self.set_resizable(False)

        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        main_box.set_margin_top(24)
        main_box.set_margin_bottom(24)
        main_box.set_margin_start(24)
        main_box.set_margin_end(24)

        header_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        title = Gtk.Label()
        title.set_markup(f"<span size='large' weight='bold'>{_('nearby_devices')}</span>")
        header_box.append(title)

        summary_text = ""
        if self.files_list:
            cnt = len(self.files_list)
            first = os.path.basename(self.files_list[0])
            summary_text = f"{first}" if cnt == 1 else f"{first} (+{cnt-1} {_('file_plural')})"
        elif self.text_content:
            summary_text = f"Text ({len(self.text_content)} chars)"

        subtitle = Gtk.Label(label=summary_text or _("nearby_devices_subtitle"))
        subtitle.add_css_class("dim-label")
        header_box.append(subtitle)
        main_box.append(header_box)

        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        scrolled.set_min_content_height(260)

        self.list_box = Gtk.ListBox()
        self.list_box.add_css_class("boxed-list")
        self.list_box.connect("row-activated", self.on_device_selected)
        scrolled.set_child(self.list_box)
        main_box.append(scrolled)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        btn_box.set_halign(Gtk.Align.CENTER)

        self.scan_btn = Gtk.Button(label=_("scanning"))
        self.scan_btn.add_css_class("pill")
        self.scan_btn.connect("clicked", self.on_rescan_clicked)
        btn_box.append(self.scan_btn)

        main_box.append(btn_box)
        self.set_content(main_box)

        self.devices_data = []
        self._load_devices()

        # Rescan and poll
        self._trigger_scan()
        GLib.timeout_add_seconds(2, self._load_devices_timer)

    def _trigger_scan(self):
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            iface.TriggerScan()
        except Exception:
            pass

    def _load_devices_timer(self):
        self._load_devices()
        return GLib.SOURCE_CONTINUE

    def _load_devices(self):
        while child := self.list_box.get_first_child():
            self.list_box.remove(child)

        self.devices_data = []
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            raw = iface.GetDiscoveredDevices()
            self.devices_data = json.loads(raw)
        except Exception as e:
            print(f"Error fetching devices from D-Bus: {e}", file=sys.stderr)

        if not self.devices_data:
            empty_row = Adw.ActionRow()
            empty_row.set_title(_("no_devices"))
            empty_row.set_subtitle(_("scanning"))
            empty_row.set_sensitive(False)
            self.list_box.append(empty_row)
            return

        for dev in self.devices_data:
            row = Adw.ActionRow()
            row.set_title(dev.get("alias", "Dispositivo"))
            row.set_subtitle(f"{dev.get('ip')} • {dev.get('deviceModel', 'LocalSend')}")

            dtype = dev.get("deviceType", "desktop")
            icon_name = "phone-symbolic" if dtype == "mobile" else "computer-symbolic"
            row.add_prefix(Gtk.Image.new_from_icon_name(icon_name))

            send_icon = Gtk.Image.new_from_icon_name("network-transmit-symbolic")
            row.add_suffix(send_icon)
            row.set_activatable(True)
            self.list_box.append(row)

    def on_rescan_clicked(self, btn):
        self._trigger_scan()
        GLib.timeout_add(600, self._load_devices)

    def on_device_selected(self, list_box, row):
        idx = row.get_index()
        if idx >= len(self.devices_data):
            return
        target = self.devices_data[idx]
        target_ip = target.get("ip")

        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")

            if self.files_list:
                iface.SendFiles(target_ip, json.dumps(self.files_list))
            elif self.text_content:
                iface.SendText(target_ip, self.text_content)
        except Exception as e:
            print(f"Error initiating send via D-Bus: {e}", file=sys.stderr)

        self.close()


class SendApp(Adw.Application):
    def __init__(self, files, text):
        super().__init__(application_id="es.pulsaros.FlyDrop.SendSelector")
        self.files = files
        self.text = text

    def do_activate(self):
        win = SendDialogWindow(self, self.files, self.text)
        win.present()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--files", default="")
    parser.add_argument("--text", default="")
    args, unknown = parser.parse_known_args()

    files = []
    if args.files:
        try:
            files = json.loads(args.files)
        except Exception:
            files = [args.files]

    app = SendApp(files, args.text)
    app.run(sys.argv[:1])


if __name__ == "__main__":
    main()
