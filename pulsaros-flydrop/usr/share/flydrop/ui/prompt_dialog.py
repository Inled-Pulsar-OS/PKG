#!/usr/bin/env python3
"""
FlyDrop Incoming Transfer Authorization Prompt (GTK4 / Libadwaita)
Appears centered when a device attempts to send files and auto-accept is disabled.
"""

import sys
import os
import argparse

# Add project root to sys.path
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

import dbus
import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, GLib, Gdk

from ui.i18n import _


class PromptWindow(Gtk.Window):
    def __init__(self, app, session_id, sender_name, summary, size_str):
        super().__init__(application=app)
        self.session_id = session_id

        self.add_css_class("flydrop-prompt-window")
        self.set_decorated(False)
        self.set_resizable(False)
        self.set_default_size(360, 240)

        self._load_css()

        card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        card.add_css_class("flydrop-prompt-card")

        # Icon
        icon_img = Gtk.Image.new_from_icon_name("flydrop-symbolic")
        icon_img.set_pixel_size(36)
        card.append(icon_img)

        # Title
        title_label = Gtk.Label()
        prompt_title = f"{sender_name} {_('wants_to_send')}"
        title_label.set_markup(f"<span size='large' weight='bold'>{prompt_title}</span>")
        title_label.set_ellipsize(3)
        card.append(title_label)

        # Details
        desc_label = Gtk.Label(label=f"{summary} • {size_str}")
        desc_label.add_css_class("dim-label")
        desc_label.set_ellipsize(3)
        card.append(desc_label)

        # Action Buttons
        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        btn_box.set_halign(Gtk.Align.CENTER)
        btn_box.set_margin_top(8)

        decline_btn = Gtk.Button(label=_("decline"))
        decline_btn.add_css_class("prompt-decline-btn")
        decline_btn.connect("clicked", self.on_decline)
        btn_box.append(decline_btn)

        accept_btn = Gtk.Button(label=_("accept"))
        accept_btn.add_css_class("prompt-accept-btn")
        accept_btn.connect("clicked", self.on_accept)
        btn_box.append(accept_btn)

        card.append(btn_box)
        self.set_child(card)

    def _load_css(self):
        css_provider = Gtk.CssProvider()
        css = """
        window.flydrop-prompt-window, window.flydrop-prompt-window.background {
            background-color: transparent;
            background: none;
            box-shadow: none;
        }
        window.flydrop-prompt-window .flydrop-prompt-card {
            background: rgba(22, 22, 28, 0.96);
            border: 1px solid rgba(255, 255, 255, 0.16);
            border-radius: 28px;
            /* La sombra tiene que caber entera en el margen transparente:
               si se recorta contra el borde de la ventana queda un marco
               duro con esquinas en punta. */
            box-shadow: 0 12px 28px rgba(0, 0, 0, 0.6);
            padding: 24px;
            margin: 32px 32px 42px;
        }
        window.flydrop-prompt-window .dim-label {
            color: rgba(255, 255, 255, 0.70);
            font-size: 13px;
        }
        window.flydrop-prompt-window .prompt-decline-btn {
            background: rgba(255, 255, 255, 0.12);
            color: #ffffff;
            border-radius: 20px;
            padding: 8px 24px;
            font-size: 13px;
            font-weight: 600;
            border: none;
        }
        window.flydrop-prompt-window .prompt-decline-btn:hover {
            background: rgba(255, 255, 255, 0.20);
        }
        window.flydrop-prompt-window .prompt-accept-btn {
            background: #007AFF;
            color: #ffffff;
            border-radius: 20px;
            padding: 8px 26px;
            font-size: 13px;
            font-weight: 600;
            border: none;
        }
        window.flydrop-prompt-window .prompt-accept-btn:hover {
            background: #0062cc;
        }
        """
        css_provider.load_from_data(css.encode("utf-8"))
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            css_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def on_accept(self, btn):
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            iface.RespondTransfer(self.session_id, True)
        except Exception as e:
            print(f"Error responding accept: {e}", file=sys.stderr)
        self.close()

    def on_decline(self, btn):
        try:
            bus = dbus.SessionBus()
            proxy = bus.get_object("es.pulsaros.FlyDrop", "/es/pulsaros/FlyDrop")
            iface = dbus.Interface(proxy, "es.pulsaros.FlyDrop")
            iface.RespondTransfer(self.session_id, False)
        except Exception as e:
            print(f"Error responding decline: {e}", file=sys.stderr)
        self.close()


class PromptApp(Adw.Application):
    def __init__(self, session_id, sender_name, summary, size_str):
        app_id_suffix = f"t{abs(hash(session_id)) % 100000}"
        super().__init__(application_id=f"es.pulsaros.FlyDrop.Prompt.{app_id_suffix}")
        self.session_id = session_id
        self.sender_name = sender_name
        self.summary = summary
        self.size_str = size_str

    def do_activate(self):
        win = PromptWindow(self, self.session_id, self.sender_name, self.summary, self.size_str)
        win.present()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--session", default="")
    parser.add_argument("--sender", default="Dispositivo")
    parser.add_argument("--summary", default="Archivos")
    parser.add_argument("--size", default="")
    args, unknown = parser.parse_known_args()

    app = PromptApp(args.session, args.sender, args.summary, args.size)
    app.run(sys.argv[:1])


if __name__ == "__main__":
    main()
