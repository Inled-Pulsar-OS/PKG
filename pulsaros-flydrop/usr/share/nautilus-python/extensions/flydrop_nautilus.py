"""
FlyDrop Nautilus Extension
Adds right-click context menu "Share with FlyDrop" / "Compartir con FlyDrop"
with real-time submenu of discovered devices.
"""

import os
import json
import urllib.parse
import locale
import gi

gi.require_version("Nautilus", "4.0")
gi.require_version("GObject", "2.0")
gi.require_version("Gio", "2.0")
from gi.repository import Nautilus, GObject, Gio

import dbus

def is_spanish():
    try:
        lang = os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES") or os.environ.get("LANG") or locale.getdefaultlocale()[0] or "en"
        return lang.lower().startswith("es")
    except Exception:
        return False

IS_ES = is_spanish()


class FlyDropMenuProvider(GObject.GObject, Nautilus.MenuProvider):
    def __init__(self):
        super().__init__()

    def get_file_items(self, files):
        if not files:
            return []

        # Extract local filesystem paths
        file_paths = []
        for file_obj in files:
            uri = file_obj.get_uri()
            if uri.startswith("file://"):
                path = urllib.parse.unquote(uri[7:])
                if os.path.exists(path):
                    file_paths.append(path)

        if not file_paths:
            return []

        main_label = "Compartir con FlyDrop" if IS_ES else "Share with FlyDrop"
        main_tip = "Enviar archivos a través de FlyDrop / LocalSend" if IS_ES else "Send files via FlyDrop / LocalSend"

        # Main Context Menu Item
        main_item = Nautilus.MenuItem(
            name="FlyDropMenuProvider::ShareFlyDrop",
            label=main_label,
            tip=main_tip,
            icon="flydrop-symbolic"
        )

        submenu = Nautilus.Menu()
        main_item.set_submenu(submenu)

        # Get discovered devices from D-Bus and trigger a background refresh
        devices = []
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            iface.TriggerScan()
            raw = iface.GetDiscoveredDevices()
            devices = json.loads(raw)
        except Exception:
            pass

        if devices:
            for dev in devices:
                alias = dev.get("alias", "Dispositivo" if IS_ES else "Device")
                model = dev.get("deviceModel", "")
                label_text = f"{alias} ({model})" if model else alias
                dtype = dev.get("deviceType", "desktop")
                icon_name = "phone-symbolic" if dtype == "mobile" else "computer-symbolic"

                send_to_tip = f"Enviar a {alias}" if IS_ES else f"Send to {alias}"

                dev_item = Nautilus.MenuItem(
                    name=f"FlyDropMenuProvider::Device_{dev.get('fingerprint', dev.get('ip'))}",
                    label=label_text,
                    tip=send_to_tip,
                    icon=icon_name
                )
                dev_item.connect("activate", self._on_send_to_device, dev.get("ip"), file_paths)
                submenu.append_item(dev_item)

        # "Buscar más dispositivos..." action item
        more_label = "Buscar dispositivos..." if IS_ES else "Search devices..."
        more_tip = "Abrir selector de dispositivos FlyDrop" if IS_ES else "Open FlyDrop device selector"

        more_item = Nautilus.MenuItem(
            name="FlyDropMenuProvider::MoreDevices",
            label=more_label,
            tip=more_tip,
            icon="system-search-symbolic"
        )
        more_item.connect("activate", self._on_open_selector, file_paths)
        submenu.append_item(more_item)

        return [main_item]

    def _on_send_to_device(self, menu_item, target_ip, file_paths):
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            iface.SendFiles(target_ip, json.dumps(file_paths))
        except Exception as e:
            print(f"FlyDrop Nautilus error sending files: {e}")

    def _on_open_selector(self, menu_item, file_paths):
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            iface.OpenSendDialog(json.dumps(file_paths))
        except Exception as e:
            print(f"FlyDrop Nautilus error opening dialog: {e}")
