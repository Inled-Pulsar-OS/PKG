#!/usr/bin/env python3
"""
FlyDrop Received Text Dialog (GTK4 / Libadwaita)
Displays received text from FlyDrop / LocalSend transfers, allowing easy 1-click
copying to clipboard and cleaning up temporary files.
"""

import sys
import os
import argparse
import subprocess

# Add project root to sys.path
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, GLib, Gdk

from ui.i18n import _


class TextDialogWindow(Adw.Window):
    def __init__(self, app, text="", sender="Dispositivo", file_path=None):
        super().__init__(application=app)
        self.text = text
        self.sender = sender
        self.file_path = file_path
        self.saved_permanently = False

        self.set_title(_("text_received"))
        self.set_default_size(460, 420)
        self.set_resizable(True)

        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        main_box.set_margin_top(20)
        main_box.set_margin_bottom(20)
        main_box.set_margin_start(20)
        main_box.set_margin_end(20)

        # Header
        header_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        title_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        title_box.set_halign(Gtk.Align.CENTER)

        icon = Gtk.Image.new_from_icon_name("edit-copy-symbolic")
        icon.set_pixel_size(24)
        title_box.append(icon)

        title_lbl = Gtk.Label()
        title_lbl.set_markup(f"<span size='large' weight='bold'>{_('text_received')}</span>")
        title_box.append(title_lbl)
        header_box.append(title_box)

        sub_lbl = Gtk.Label(label=f"{_('text_from')} {self.sender}")
        sub_lbl.add_css_class("dim-label")
        header_box.append(sub_lbl)
        main_box.append(header_box)

        # Text View in ScrolledWindow
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        scrolled.set_hexpand(True)
        scrolled.set_min_content_height(180)

        self.text_view = Gtk.TextView()
        self.text_view.set_editable(False)
        self.text_view.set_cursor_visible(False)
        self.text_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self.text_view.set_left_margin(12)
        self.text_view.set_right_margin(12)
        self.text_view.set_top_margin(12)
        self.text_view.set_bottom_margin(12)
        self.text_view.get_buffer().set_text(self.text)

        frame = Gtk.Frame()
        frame.set_child(self.text_view)
        scrolled.set_child(frame)
        main_box.append(scrolled)

        # Bottom Buttons
        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        btn_box.set_halign(Gtk.Align.CENTER)

        self.copy_btn = Gtk.Button(label=_("copy_clipboard"))
        self.copy_btn.add_css_class("pill")
        self.copy_btn.add_css_class("suggested-action")
        self.copy_btn.connect("clicked", self.on_copy_clicked)
        btn_box.append(self.copy_btn)

        if self.file_path and os.path.exists(self.file_path):
            self.save_btn = Gtk.Button(label=_("save_file"))
            self.save_btn.add_css_class("pill")
            self.save_btn.connect("clicked", self.on_save_clicked)
            btn_box.append(self.save_btn)

        self.close_btn = Gtk.Button(label=_("close"))
        self.close_btn.add_css_class("pill")
        self.close_btn.connect("clicked", lambda b: self.close())
        btn_box.append(self.close_btn)

        main_box.append(btn_box)
        self.set_content(main_box)

        # Auto-copy on presentation if desired, or let user click
        self.connect("close-request", self.on_close_request)

    def on_copy_clicked(self, btn):
        clipboard = Gdk.Display.get_default().get_clipboard()
        clipboard.set(self.text)

        # Fallback to wl-copy
        try:
            p = subprocess.Popen(["wl-copy"], stdin=subprocess.PIPE)
            p.communicate(input=self.text.encode("utf-8"), timeout=1)
        except Exception:
            pass

        self.copy_btn.set_label(_("copied"))
        self.copy_btn.remove_css_class("suggested-action")
        self.copy_btn.add_css_class("success")

        # Close after 1.5 seconds automatically
        GLib.timeout_add(1500, self.close)

    def on_save_clicked(self, btn):
        self.saved_permanently = True
        self.save_btn.set_sensitive(False)
        self.save_btn.set_label(_("file_saved"))

    def on_close_request(self, win):
        if not self.saved_permanently and self.file_path and os.path.exists(self.file_path):
            try:
                os.remove(self.file_path)
            except Exception:
                pass
        return False


class TextApp(Adw.Application):
    def __init__(self, text, sender, file_path):
        super().__init__(application_id=f"es.pulsaros.FlyDrop.TextDialog.{os.getpid()}")
        self.text = text
        self.sender = sender
        self.file_path = file_path

    def do_activate(self):
        win = TextDialogWindow(self, self.text, self.sender, self.file_path)
        win.present()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--file", default=None)
    parser.add_argument("--text", default=None)
    parser.add_argument("--sender", default="Dispositivo")
    args, unknown = parser.parse_known_args()

    text_content = args.text or ""
    if args.file and os.path.exists(args.file):
        try:
            with open(args.file, "r", encoding="utf-8", errors="replace") as f:
                text_content = f.read()
        except Exception as e:
            text_content = f"Error leyendo texto: {e}"

    app = TextApp(text_content, args.sender, args.file)
    app.run(sys.argv[:1])


if __name__ == "__main__":
    main()
