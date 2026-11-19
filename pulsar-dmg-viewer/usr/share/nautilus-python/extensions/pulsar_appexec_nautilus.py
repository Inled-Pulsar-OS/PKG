"""
Pulsar AppExec Nautilus Extension
Adds context actions for macOS applications (.app), disk images (.dmg),
and custom visualizers for Applications folders.
"""

import os
import subprocess
import urllib.parse
import locale
import gi

gi.require_version("Nautilus", "4.0")
gi.require_version("GObject", "2.0")
gi.require_version("Gio", "2.0")
from gi.repository import Nautilus, GObject, Gio


def is_spanish():
    try:
        lang = os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES") or os.environ.get("LANG") or locale.getdefaultlocale()[0] or "en"
        return lang.lower().startswith("es")
    except Exception:
        return False

IS_ES = is_spanish()


class PulsarAppExecMenuProvider(GObject.GObject, Nautilus.MenuProvider):
    def __init__(self):
        super().__init__()

    def get_file_items(self, files):
        if not files or len(files) != 1:
            return []

        file_obj = files[0]
        uri = file_obj.get_uri()
        if not uri.startswith("file://"):
            return []

        path = urllib.parse.unquote(uri[7:])
        items = []

        # 1. Handling .dmg disk images
        if path.lower().endswith(".dmg"):
            label = "Abrir con Visor DMG de Pulsar" if IS_ES else "Open with Pulsar DMG Viewer"
            tip = "Visualizar y montar imagen de disco con Pulsar DMG Viewer" if IS_ES else "View and mount disk image with Pulsar DMG Viewer"
            item = Nautilus.MenuItem(
                name="PulsarAppExec::OpenDMG",
                label=label,
                tip=tip,
                icon="drive-removable-media"
            )
            item.connect("activate", lambda m, p: subprocess.Popen(["/usr/bin/pulsar-dmg-viewer", p]), path)
            items.append(item)

        # 2. Handling .app bundles
        elif path.endswith(".app") or os.path.exists(os.path.join(path, "Contents", "MacOS")):
            label = "Ejecutar aplicación (AppExec)" if IS_ES else "Run application (AppExec)"
            tip = "Ejecutar con el motor Pulsar AppExec" if IS_ES else "Run with Pulsar AppExec engine"
            item = Nautilus.MenuItem(
                name="PulsarAppExec::RunApp",
                label=label,
                tip=tip,
                icon="pulsar-logo"
            )
            item.connect("activate", lambda m, p: subprocess.Popen(["/usr/bin/pulsar-appexec", p]), path)
            items.append(item)

        # 3. Handling Applications folder
        elif os.path.basename(path) == "Applications" and os.path.isdir(path):
            label = "Abrir en el Visor de Aplicaciones" if IS_ES else "Open in Applications Viewer"
            tip = "Abrir la vista nativa de aplicaciones de Pulsar" if IS_ES else "Open Pulsar native applications view"
            item = Nautilus.MenuItem(
                name="PulsarAppExec::OpenAppsViewer",
                label=label,
                tip=tip,
                icon="folder-applications"
            )
            item.connect("activate", lambda m: subprocess.Popen(["/usr/bin/pulsar-apps-viewer"]))
            items.append(item)

        return items

    def get_background_items(self, current_folder):
        uri = current_folder.get_uri()
        if not uri.startswith("file://"):
            return []

        path = urllib.parse.unquote(uri[7:])
        items = []

        # If inside ~/Applications or /Applications
        if os.path.basename(path) == "Applications":
            label = "Abrir en el Visor de Aplicaciones" if IS_ES else "Open in Applications Viewer"
            tip = "Ver aplicaciones en la vista de cuadrícula nativa" if IS_ES else "View applications in native grid view"
            item = Nautilus.MenuItem(
                name="PulsarAppExec::BgAppsViewer",
                label=label,
                tip=tip,
                icon="folder-applications"
            )
            item.connect("activate", lambda m: subprocess.Popen(["/usr/bin/pulsar-apps-viewer"]))
            items.append(item)

        return items
