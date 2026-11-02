#!/usr/bin/env python3
"""
FlyDrop Send Target Selector (GTK4 / Libadwaita)
Allows user to send clipboard content or pick files and select a nearby device.
"""

import sys
import os
import json
import argparse
import subprocess

# Add project root to sys.path
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, GLib, Gio, Gdk

import dbus
import dbus.mainloop.glib
dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

from ui.i18n import _


def get_clipboard_text_fallback():
    """Helper to retrieve text from clipboard using wl-paste or xclip"""
    try:
        out = subprocess.check_output(["wl-paste", "-n"], stderr=subprocess.DEVNULL, timeout=0.6)
        text = out.decode("utf-8", errors="replace").strip()
        if text:
            return text
    except Exception:
        pass
    try:
        out = subprocess.check_output(["xclip", "-selection", "clipboard", "-o"], stderr=subprocess.DEVNULL, timeout=0.6)
        text = out.decode("utf-8", errors="replace").strip()
        if text:
            return text
    except Exception:
        pass
    return ""


class SendDialogWindow(Adw.Window):
    MODE_CLIPBOARD = "clipboard"
    MODE_FILES = "files"

    def __init__(self, app, files_list=None, text_content=""):
        super().__init__(application=app)
        self.files_list = files_list or []
        self.text_content = text_content or ""
        self.current_mode = self.MODE_FILES if self.files_list else self.MODE_CLIPBOARD

        self.set_title(_("send"))
        self.set_default_size(480, 560)
        self.set_resizable(False)

        # If no text was passed and no files, read clipboard
        if not self.files_list and not self.text_content:
            self.text_content = get_clipboard_text_fallback()
            if not self.text_content:
                self.current_mode = self.MODE_FILES

        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        main_box.set_margin_top(18)
        main_box.set_margin_bottom(18)
        main_box.set_margin_start(20)
        main_box.set_margin_end(20)

        # Header Title
        header_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        title = Gtk.Label()
        title.set_markup(f"<span size='large' weight='bold'>{_('send')}</span>")
        header_box.append(title)

        self.subtitle = Gtk.Label(label=_("nearby_devices_subtitle"))
        self.subtitle.add_css_class("dim-label")
        header_box.append(self.subtitle)
        main_box.append(header_box)

        # Mode Selector Buttons (Segmented)
        mode_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        mode_box.set_halign(Gtk.Align.CENTER)
        mode_box.add_css_class("linked")

        self.btn_mode_clipboard = Gtk.ToggleButton(label=f"📋 {_('clipboard')}")
        self.btn_mode_clipboard.connect("toggled", self.on_mode_toggled, self.MODE_CLIPBOARD)
        mode_box.append(self.btn_mode_clipboard)

        self.btn_mode_files = Gtk.ToggleButton(label=f"📁 {_('file_plural').capitalize()}")
        self.btn_mode_files.connect("toggled", self.on_mode_toggled, self.MODE_FILES)
        mode_box.append(self.btn_mode_files)

        main_box.append(mode_box)

        # Content Card Stack
        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)

        # Page 1: Clipboard Card
        clip_page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.clip_card = Adw.PreferencesGroup()
        self.clip_card.set_title(_("clipboard_content"))

        self.clip_preview_row = Adw.ActionRow()
        self.clip_preview_row.set_subtitle_lines(3)
        self.clip_card.add(self.clip_preview_row)

        clip_actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        clip_actions.set_halign(Gtk.Align.END)
        btn_refresh_clip = Gtk.Button(label="↻ Refrescar")
        btn_refresh_clip.add_css_class("flat")
        btn_refresh_clip.connect("clicked", lambda b: self.refresh_clipboard())
        clip_actions.append(btn_refresh_clip)
        self.clip_card.add(clip_actions)

        clip_page.append(self.clip_card)
        self.stack.add_named(clip_page, self.MODE_CLIPBOARD)

        # Page 2: Files Card
        files_page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.files_card = Adw.PreferencesGroup()
        self.files_card.set_title(_("file_plural").capitalize())

        self.files_summary_row = Adw.ActionRow()
        self.files_card.add(self.files_summary_row)

        files_actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        files_actions.set_halign(Gtk.Align.END)
        self.pick_btn = Gtk.Button(label=_("pick_files"))
        self.pick_btn.add_css_class("suggested-action")
        self.pick_btn.add_css_class("pill")
        self.pick_btn.connect("clicked", self.on_pick_files_clicked)
        files_actions.append(self.pick_btn)
        self.files_card.add(files_actions)

        files_page.append(self.files_card)
        self.stack.add_named(files_page, self.MODE_FILES)

        main_box.append(self.stack)

        # Discovered Devices Header
        dev_header = Gtk.Label()
        dev_header.set_markup(f"<b>{_('nearby_devices')}</b>")
        dev_header.set_halign(Gtk.Align.START)
        main_box.append(dev_header)

        # Scrolled List of Devices
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        scrolled.set_min_content_height(190)

        self.list_box = Gtk.ListBox()
        self.list_box.add_css_class("boxed-list")
        self.list_box.connect("row-activated", self.on_device_selected)
        scrolled.set_child(self.list_box)
        main_box.append(scrolled)

        # Bottom Scan Button
        bottom_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        bottom_box.set_halign(Gtk.Align.CENTER)

        self.scan_btn = Gtk.Button(label=_("scanning"))
        self.scan_btn.add_css_class("pill")
        self.scan_btn.connect("clicked", self.on_rescan_clicked)
        bottom_box.append(self.scan_btn)

        main_box.append(bottom_box)
        self.set_content(main_box)

        # Initialize Mode UI
        if self.current_mode == self.MODE_FILES:
            self.btn_mode_files.set_active(True)
        else:
            self.btn_mode_clipboard.set_active(True)

        self.update_content_views()

        # Try GTK native clipboard async in case wl-paste was slow
        self.read_gtk_clipboard_async()

        self.devices_data = []
        self._load_devices()

        # Rescan and poll
        self._trigger_scan()
        GLib.timeout_add_seconds(2, self._load_devices_timer)

    def read_gtk_clipboard_async(self):
        try:
            clipboard = Gdk.Display.get_default().get_clipboard()
            clipboard.read_text_async(None, self._on_gtk_clipboard_read)
        except Exception:
            pass

    def _on_gtk_clipboard_read(self, clipboard, result):
        try:
            text = clipboard.read_text_finish(result)
            if text and text.strip():
                self.text_content = text.strip()
                self.update_content_views()
        except Exception:
            pass

    def refresh_clipboard(self):
        text = get_clipboard_text_fallback()
        if text:
            self.text_content = text
        self.read_gtk_clipboard_async()
        self.update_content_views()

    def on_mode_toggled(self, button, mode):
        if not button.get_active():
            return
        self.current_mode = mode
        if mode == self.MODE_CLIPBOARD:
            self.btn_mode_files.set_active(False)
            self.stack.set_visible_child_name(self.MODE_CLIPBOARD)
        else:
            self.btn_mode_clipboard.set_active(False)
            self.stack.set_visible_child_name(self.MODE_FILES)
        self.update_content_views()

    def update_content_views(self):
        # Update Clipboard Card
        if self.text_content:
            preview = self.text_content.replace("\n", " ")
            if len(preview) > 100:
                preview = preview[:97] + "..."
            self.clip_preview_row.set_title(preview)
            self.clip_preview_row.set_subtitle(f"{len(self.text_content)} caracteres")
        else:
            self.clip_preview_row.set_title(_("empty_clipboard"))
            self.clip_preview_row.set_subtitle("Copia cualquier texto en tu equipo para enviarlo")

        # Update Files Card
        if self.files_list:
            cnt = len(self.files_list)
            first = os.path.basename(self.files_list[0])
            summary = f"{first}" if cnt == 1 else f"{first} (+{cnt-1} {_('file_plural')})"
            self.files_summary_row.set_title(summary)
            self.files_summary_row.set_subtitle(f"{cnt} archivo(s) preparado(s)")
        else:
            self.files_summary_row.set_title("Ningún archivo seleccionado")
            self.files_summary_row.set_subtitle("Pulsa 'Elegir archivos' para buscar en el sistema")

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

    def on_pick_files_clicked(self, btn):
        dialog = Gtk.FileDialog()
        dialog.set_title(_("add_files"))
        try:
            dialog.open_multiple(self, None, self._on_files_picked)
        except Exception as e:
            print(f"Error opening file picker: {e}", file=sys.stderr)

    def _on_files_picked(self, dialog, result):
        try:
            files = dialog.open_multiple_finish(result)
        except GLib.Error:
            return
        paths = []
        if files is not None:
            for i in range(files.get_n_items()):
                try:
                    p = files.get_item(i).get_path()
                except Exception:
                    p = None
                if p:
                    paths.append(p)
        if not paths:
            return
        self.files_list = paths
        self.btn_mode_files.set_active(True)
        self.update_content_views()

    def on_device_selected(self, list_box, row):
        idx = row.get_index()
        if idx >= len(self.devices_data):
            return
        target = self.devices_data[idx]
        target_ip = target.get("ip")

        # Determine what to send according to active mode
        if self.current_mode == self.MODE_CLIPBOARD:
            if not self.text_content:
                dialog = Adw.MessageDialog.new(self, _("empty_clipboard"), "Copia texto al portapapeles o cambia a la pestaña de Archivos.")
                dialog.add_response("ok", "Entendido")
                dialog.present()
                return
            try:
                bus = dbus.SessionBus()
                proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
                iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
                iface.SendText(target_ip, self.text_content)
            except Exception as e:
                print(f"Error SendText via D-Bus: {e}", file=sys.stderr)
            self.close()

        else: # MODE_FILES
            if not self.files_list:
                self.on_pick_files_clicked(self.pick_btn)
                return
            try:
                bus = dbus.SessionBus()
                proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
                iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
                iface.SendFiles(target_ip, json.dumps(self.files_list))
            except Exception as e:
                print(f"Error SendFiles via D-Bus: {e}", file=sys.stderr)
            self.close()


class SendApp(Adw.Application):
    def __init__(self, files, text):
        super().__init__(application_id=f"es.pulsaros.FlyDrop.SendSelector.{os.getpid()}")
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
