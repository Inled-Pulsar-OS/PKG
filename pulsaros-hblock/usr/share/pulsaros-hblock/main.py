#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import os, sys, threading, locale, gettext
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

try:
    locale.setlocale(locale.LC_ALL, "")
except Exception:
    pass

_LANG = (os.environ.get("LANG") or "").split(".")[0].split(":")[0] or "es"
_LOCALE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "locale")
try:
    _t = gettext.translation("pulsaros-hblock", localedir=_LOCALE_DIR, languages=[_LANG] if _LANG.startswith("es") else [_LANG, "en"], fallback=True)
    _ = _t.gettext
except Exception:
    _ = lambda x: x

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, GLib, Gdk
from hblock_core import get_status, run_action

class Win(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title=_("Ad Block"))
        self.set_default_size(420, 420)
        self.set_size_request(360, 360)
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=24)
        box.set_margin_top(36); box.set_margin_bottom(36); box.set_margin_start(24); box.set_margin_end(24)
        box.set_hexpand(True); box.set_vexpand(True); box.set_valign(Gtk.Align.CENTER)

        self.label = Gtk.Label()
        self.label.set_markup("<b>" + _("Ad Block") + "</b>")
        self.label.set_halign(Gtk.Align.CENTER)
        self.label.add_css_class("title-1")

        self.count = Gtk.Label()
        self.count.set_halign(Gtk.Align.CENTER)
        self.count.add_css_class("heading")

        self.btn = Gtk.Button()
        self.btn.set_size_request(260, 72)
        self.btn.add_css_class("pill")
        self.btn.connect("clicked", self.on_click)
        self.btn.set_halign(Gtk.Align.CENTER)
        self.btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.btn_box.set_halign(Gtk.Align.CENTER)
        self.btn_box.set_valign(Gtk.Align.CENTER)
        self.btn_text = Gtk.Label()
        self.btn_text.set_text(_("NO ESTÁS PROTEGIDO"))
        self.btn_text.set_halign(Gtk.Align.CENTER)
        self.btn_spinner = Gtk.Spinner()
        self.btn_spinner.set_size_request(28, 28)
        self.btn_spinner.set_visible(False)
        self.btn_box.append(self.btn_text)
        self.btn_box.append(self.btn_spinner)
        self.btn.set_child(self.btn_box)

        css = b"""
        .pill { border-radius: 36px; border: 0; color: #fff; font-weight: 800; font-size: 1.15rem; box-shadow: 0 8px 24px rgba(0,0,0,.15); }
        .red { background: #ff0000; }
        .green { background: #00cc00; }
        .activando { background: #ff8c00; animation: pulse 1s infinite; }
        @keyframes pulse { 0% { opacity: 1; } 50% { opacity: .7; } 100% { opacity: 1; } }
        """
        provider = Gtk.CssProvider(); provider.load_from_data(css)
        Gtk.StyleContext.add_provider_for_display(Gdk.Display.get_default(), provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)

        box.append(self.label)
        box.append(self.count)
        box.append(self.btn)
        self.set_content(box)
        self.action_active = False
        self.refresh()

    def refresh(self):
        if self.action_active:
            return True
        s = get_status()
        on = s.get("enabled", False)
        blocked = s.get("blocked") or 0
        self.count.set_text(f"{blocked:,} {_('Dominios bloqueados')}" if blocked else "—")
        self.btn_text.set_text(_("PROTEGIDO") if on else _("NO ESTÁS PROTEGIDO"))
        self.btn.remove_css_class("red"); self.btn.remove_css_class("green"); self.btn.remove_css_class("activando")
        self.btn.add_css_class("green" if on else "red")
        return True

    def on_click(self, btn):
        if self.action_active:
            return
        s = get_status()
        target = "enable" if not s.get("enabled") else "disable"
        self.action_active = True
        self.btn_text.set_visible(False)
        self.btn_spinner.set_visible(True)
        self.btn_spinner.start()
        self.btn.remove_css_class("red"); self.btn.remove_css_class("green"); self.btn.remove_css_class("activando")
        self.btn.add_css_class("activando")
        threading.Thread(target=self.run, args=(target,), daemon=True).start()

    def run(self, act):
        try:
            ok, msg = run_action(act)
        except Exception as exc:
            ok, msg = False, str(exc)
        def done():
            self.spinner_stop()
            self.refresh()
            return False
        GLib.idle_add(done)

    def spinner_stop(self):
        self.action_active = False
        self.btn_spinner.stop()
        self.btn_spinner.set_visible(False)
        self.btn_text.set_visible(True)
        self.btn.remove_css_class("activando")
        self.refresh()

class App(Adw.Application):
    def __init__(self):
        super().__init__(application_id="es.inled.PulsarHBlock")
    def do_activate(self):
        self.win = Win(self)
        self.win.present()

if __name__ == "__main__":
    app = App()
    app.run(sys.argv)
