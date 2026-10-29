#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Firewall - Interactive Prompt Dialog
Prompts the user when an application attempts to listen on an unopened port.
"""

import argparse
import sys
import os

sys.path.insert(0, "/usr/lib/pulsaros-firewall")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gtk, Gio, GLib

from core import _, run_helper, save_decision, resolve_process_app_info


class FirewallPromptWindow(Adw.Window):
    def __init__(self, port: int, proto: str, pid: int = 0, app_name: str = "", binary: str = ""):
        super().__init__()
        self.port = port
        self.proto = proto.lower()
        self.pid = pid

        if not app_name or not binary:
            info = resolve_process_app_info(pid)
            self.app_name = app_name or info["name"]
            self.binary = binary or info["binary"]
            self.icon = info["icon"]
        else:
            self.app_name = app_name
            self.binary = binary
            self.icon = "pulsaros-firewall"

        self.set_title(_("Pulsar Firewall"))
        self.set_default_size(440, 480)
        self.set_modal(True)
        self.set_resizable(False)

        self._build_ui()

    def _build_ui(self):
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        main_box.set_margin_top(24)
        main_box.set_margin_bottom(24)
        main_box.set_margin_start(24)
        main_box.set_margin_end(24)

        # Icon and Shield header
        icon_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        icon_box.set_halign(Gtk.Align.CENTER)

        icon_img = Gtk.Image.new_from_icon_name("pulsaros-firewall")
        icon_img.set_pixel_size(64)
        icon_box.append(icon_img)

        title_lbl = Gtk.Label(label=_("Allow incoming connections?"))
        title_lbl.add_css_class("title-2")
        title_lbl.set_wrap(True)
        title_lbl.set_justify(Gtk.Justification.CENTER)
        icon_box.append(title_lbl)

        main_box.append(icon_box)

        # Safe markup escaping
        escaped_app_name = GLib.markup_escape_text(self.app_name)
        escaped_binary = GLib.markup_escape_text(self.binary)

        # Message explanation
        desc_text = (
            f"<b>{escaped_app_name}</b> "
            f"{_('is attempting to accept incoming connections on port')} "
            f"<b>{self.port} ({self.proto.upper()})</b>.\n\n"
            f"{_('If you block this port, devices on your network will not be able to connect to this app.')}"
        )
        desc_lbl = Gtk.Label()
        desc_lbl.set_markup(desc_text)
        desc_lbl.set_wrap(True)
        desc_lbl.set_justify(Gtk.Justification.CENTER)
        desc_lbl.add_css_class("body")
        main_box.append(desc_lbl)

        # Technical details preferences group
        group = Adw.PreferencesGroup()
        group.set_title(_("Technical Details"))

        # Row: Application (Properly escaped for Pango)
        app_row = Adw.ActionRow(title=_("Application"))
        app_row.set_subtitle(escaped_app_name)
        app_row.add_prefix(Gtk.Image.new_from_icon_name(self.icon))
        group.add(app_row)

        # Row: Port & Protocol
        port_row = Adw.ActionRow(title=_("Port"))
        port_row.set_subtitle(f"{self.port} / {self.proto.upper()}")
        port_row.add_prefix(Gtk.Image.new_from_icon_name("network-wired-symbolic"))
        group.add(port_row)

        # Row: Executable path
        if self.binary:
            exe_row = Adw.ActionRow(title=_("Executable"))
            exe_row.set_subtitle(escaped_binary)
            exe_row.add_prefix(Gtk.Image.new_from_icon_name("application-x-executable-symbolic"))
            group.add(exe_row)

        # Row: PID
        if self.pid:
            pid_row = Adw.ActionRow(title=_("Process ID"))
            pid_row.set_subtitle(str(self.pid))
            pid_row.add_prefix(Gtk.Image.new_from_icon_name("utilities-system-monitor-symbolic"))
            group.add(pid_row)

        main_box.append(group)

        # Action Buttons
        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        btn_box.set_margin_top(8)

        # 1. Allow Always button (Primary / Suggested)
        allow_btn = Gtk.Button(label=_("Allow Always"))
        allow_btn.add_css_class("suggested-action")
        allow_btn.add_css_class("pill")
        allow_btn.connect("clicked", self._on_allow_clicked)
        btn_box.append(allow_btn)

        # 2. Block button (Destructive)
        block_btn = Gtk.Button(label=_("Block"))
        block_btn.add_css_class("destructive-action")
        block_btn.add_css_class("pill")
        block_btn.connect("clicked", self._on_block_clicked)
        btn_box.append(block_btn)

        # 3. Only this session
        temp_btn = Gtk.Button(label=_("Only This Session"))
        temp_btn.add_css_class("flat")
        temp_btn.connect("clicked", self._on_temp_clicked)
        btn_box.append(temp_btn)

        main_box.append(btn_box)
        self.set_content(main_box)

    def _record_all_keys(self, decision: str):
        key = f"{self.port}/{self.proto}"
        save_decision(key, decision)
        if self.app_name:
            save_decision(f"app:{self.app_name}", decision)
        if self.binary:
            save_decision(f"bin:{self.binary}", decision)

    def _on_allow_clicked(self, btn):
        self._record_all_keys("allow")
        # Add UFW rule via helper
        comment = self.app_name or "Custom App"
        run_helper("allow", str(self.port), self.proto, comment)
        self.close()

    def _on_block_clicked(self, btn):
        self._record_all_keys("block")
        self.close()

    def _on_temp_clicked(self, btn):
        self._record_all_keys("temporary")
        self.close()


def main():
    parser = argparse.ArgumentParser(description="Pulsar OS Firewall Prompt")
    parser.add_argument("--port", type=int, required=True, help="Port number")
    parser.add_argument("--proto", type=str, default="tcp", help="Protocol (tcp/udp)")
    parser.add_argument("--pid", type=int, default=0, help="Process ID")
    parser.add_argument("--app", type=str, default="", help="Application name")
    parser.add_argument("--binary", type=str, default="", help="Binary executable path")
    args = parser.parse_args()

    app = Adw.Application(application_id="es.pulsaros.FirewallPrompt")

    def on_activate(application):
        win = FirewallPromptWindow(
            port=args.port,
            proto=args.proto,
            pid=args.pid,
            app_name=args.app,
            binary=args.binary,
        )
        application.add_window(win)
        win.present()

    app.connect("activate", on_activate)
    app.run(None)


if __name__ == "__main__":
    main()
