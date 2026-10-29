#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Firewall - GTK4 / Libadwaita Management Interface
Modern, intuitive firewall control panel with English fallback and Spanish translation.
"""

import logging
import os
import sys

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gdk, Gio, GLib, Gtk

# Ensure module path resolution
sys.path.insert(0, "/usr/lib/pulsaros-firewall")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from core import (
    _,
    get_listening_sockets,
    get_ufw_status,
    load_preferences,
    run_helper,
    save_preferences,
)

logger = logging.getLogger("PulsarFirewall.GUI")
APP_ID = "es.pulsaros.Firewall"

PRESET_SERVICES = [
    {
        "id": "sayri",
        "title": _("Sayri AI Assistant (API)"),
        "subtitle": _("OpenAI-compatible local server on port 8765"),
        "port": 8765,
        "proto": "tcp",
        "icon": "sayri-tray",
        "comment": "Sayri API",
    },
    {
        "id": "bridge",
        "title": _("Pulsar Phone & Bridge"),
        "subtitle": _("Phone pairing and streaming on port 8989"),
        "port": 8989,
        "proto": "tcp",
        "icon": "pulsar-vpn",
        "comment": "Pulsar Bridge",
    },
    {
        "id": "broadway",
        "title": _("Pulsar Broadway Remote Apps"),
        "subtitle": _("GTK HTML5 application streaming on port 8085"),
        "port": 8085,
        "proto": "tcp",
        "icon": "applications-other-symbolic",
        "comment": "Pulsar Broadway",
    },
    {
        "id": "mdns",
        "title": _("Network Discovery (mDNS)"),
        "subtitle": _("Local network auto-discovery on port 5353 (UDP)"),
        "port": 5353,
        "proto": "udp",
        "icon": "network-workgroup-symbolic",
        "comment": "mDNS Avahi",
    },
    {
        "id": "ssh",
        "title": _("Remote Access (SSH)"),
        "subtitle": _("Secure shell terminal access on port 22"),
        "port": 22,
        "proto": "tcp",
        "icon": "terminal-symbolic",
        "comment": "SSH Server",
    },
    {
        "id": "samba",
        "title": _("File Sharing (Samba)"),
        "subtitle": _("Windows file and printer sharing on port 445"),
        "port": 445,
        "proto": "tcp",
        "icon": "folder-remote-symbolic",
        "comment": "Samba Shares",
    },
]


class AddRuleDialog(Adw.Window):
    def __init__(self, parent, on_added_cb):
        super().__init__()
        self.set_transient_for(parent)
        self.set_modal(True)
        self.set_title(_("Add Rule"))
        self.set_default_size(380, 360)
        self.on_added_cb = on_added_cb

        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        content.set_margin_top(16)
        content.set_margin_bottom(16)
        content.set_margin_start(16)
        content.set_margin_end(16)

        group = Adw.PreferencesGroup()
        group.set_title(_("Rule Details"))

        self.port_row = Adw.EntryRow(title=_("Port (1-65535)"))
        group.add(self.port_row)

        # Action: Allow, Deny, Reject
        self.action_row = Adw.ComboRow(title=_("Action"))
        action_model = Gtk.StringList.new([
            _("ALLOW"),
            _("DENY"),
            _("REJECT")
        ])
        self.action_row.set_model(action_model)
        self.action_row.set_selected(0)
        group.add(self.action_row)

        # Protocol: TCP, UDP, Any
        self.proto_row = Adw.ComboRow(title=_("Protocol"))
        proto_model = Gtk.StringList.new([
            _("TCP"),
            _("UDP"),
            _("TCP/UDP (Any)")
        ])
        self.proto_row.set_model(proto_model)
        self.proto_row.set_selected(0)
        group.add(self.proto_row)

        self.comment_row = Adw.EntryRow(title=_("Description / App Name"))
        group.add(self.comment_row)

        content.append(group)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        btn_box.set_halign(Gtk.Align.END)
        btn_box.set_margin_top(8)

        cancel_btn = Gtk.Button(label=_("Cancel"))
        cancel_btn.connect("clicked", lambda b: self.close())
        btn_box.append(cancel_btn)

        add_btn = Gtk.Button(label=_("Save"))
        add_btn.add_css_class("suggested-action")
        add_btn.connect("clicked", self._on_add_clicked)
        btn_box.append(add_btn)

        content.append(btn_box)
        self.set_content(content)

    def _on_add_clicked(self, btn):
        port_text = self.port_row.get_text().strip()
        if not port_text.isdigit() or not (1 <= int(port_text) <= 65535):
            self.port_row.add_css_class("error")
            return

        act_idx = self.action_row.get_selected()
        action_map = ["allow", "deny", "reject"]
        action_val = action_map[act_idx] if act_idx < len(action_map) else "allow"

        proto_idx = self.proto_row.get_selected()
        proto = "tcp" if proto_idx == 0 else ("udp" if proto_idx == 1 else "any")
        comment = self.comment_row.get_text().strip()

        ok, msg = run_helper("rule", action_val, port_text, proto, comment)
        if ok:
            self.on_added_cb()
            self.close()
        else:
            logger.error(f"Failed to add rule: {msg}")


class FirewallMainWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title(_("Pulsar Firewall"))
        self.set_default_size(780, 720)
        self.set_icon_name("pulsaros-firewall")

        self.updating_ui = False
        self._live_rows = []
        self._custom_rule_rows = []
        self._build_ui()
        self.refresh_all()

    def _build_ui(self):
        # Header bar
        header = Adw.HeaderBar()

        refresh_btn = Gtk.Button(icon_name="view-refresh-symbolic")
        refresh_btn.set_tooltip_text(_("Refresh status"))
        refresh_btn.connect("clicked", lambda b: self.refresh_all())
        header.pack_start(refresh_btn)

        defaults_btn = Gtk.Button(label=_("Reset Defaults"))
        defaults_btn.set_tooltip_text(_("Apply recommended Pulsar OS security defaults"))
        defaults_btn.connect("clicked", self._on_reset_defaults)
        header.pack_end(defaults_btn)

        # Main Page layout
        page = Adw.PreferencesPage()

        # 1. Main Status Group
        self.status_group = Adw.PreferencesGroup()
        self.status_group.set_title(_("Status"))

        self.status_row = Adw.SwitchRow()
        self.status_row.set_title(_("Firewall is Active"))
        self.status_row.set_subtitle(_("Incoming traffic blocked, outgoing allowed"))
        self.status_row.set_icon_name("security-high-symbolic")
        self.status_row.connect("notify::active", self._on_status_toggled)
        self.status_group.add(self.status_row)

        self.notify_row = Adw.SwitchRow()
        self.notify_row.set_title(_("Notify when an app listens on a closed port"))
        self.notify_row.set_subtitle(_("Shows an interactive prompt to allow or block the application"))
        self.notify_row.set_icon_name("preferences-system-notifications-symbolic")
        self.notify_row.connect("notify::active", self._on_notify_toggled)
        self.status_group.add(self.notify_row)

        page.add(self.status_group)

        # 2. Preset Services Group
        self.preset_group = Adw.PreferencesGroup()
        self.preset_group.set_title(_("Pulsar & System Services"))
        self.preset_group.set_description(_("Quick toggle for official Pulsar OS features and desktop integration"))

        self.service_switches = {}
        for svc in PRESET_SERVICES:
            row = Adw.SwitchRow()
            row.set_title(svc["title"])
            row.set_subtitle(svc["subtitle"])
            row.set_icon_name(svc["icon"])
            row.connect("notify::active", self._make_service_toggle_handler(svc))
            self.preset_group.add(row)
            self.service_switches[svc["id"]] = row

        page.add(self.preset_group)

        # 3. Live Listening Apps Group
        self.live_group = Adw.PreferencesGroup()
        self.live_group.set_title(_("Active Applications"))
        self.live_group.set_description(_("Applications currently listening on network ports on this computer"))
        page.add(self.live_group)

        # 4. Custom Rules Group
        self.rules_group = Adw.PreferencesGroup()
        self.rules_group.set_title(_("Rules"))
        self.rules_group.set_description(_("Custom open ports and inbound firewall rules"))

        add_rule_btn = Gtk.Button(label=_("Add Rule"))
        add_rule_btn.add_css_class("suggested-action")
        add_rule_btn.connect("clicked", self._on_add_rule_clicked)
        self.rules_group.set_header_suffix(add_rule_btn)

        page.add(self.rules_group)

        # Wrap in Box with HeaderBar
        root_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        root_box.append(header)
        root_box.append(page)
        self.set_content(root_box)

    def refresh_all(self):
        self.updating_ui = True
        try:
            status = get_ufw_status()
            prefs = load_preferences()

            # Main switch
            is_active = status.get("active", False)
            self.status_row.set_active(is_active)
            if is_active:
                self.status_row.set_title(_("Firewall is Active"))
                self.status_row.set_subtitle(_("Incoming traffic blocked, outgoing allowed"))
                self.status_row.set_icon_name("security-high-symbolic")
            else:
                self.status_row.set_title(_("Firewall is Disabled"))
                self.status_row.set_subtitle(_("All network traffic is permitted"))
                self.status_row.set_icon_name("security-low-symbolic")

            # Notification toggle
            self.notify_row.set_active(prefs.get("notify_on_new_ports", True))

            # Service Presets
            rules = status.get("rules", [])
            for svc in PRESET_SERVICES:
                sw = self.service_switches.get(svc["id"])
                if sw:
                    is_allowed = any(
                        r["port"] == svc["port"]
                        and r["proto"] in (svc["proto"], "any")
                        and "ALLOW" in r["action"]
                        for r in rules
                    )
                    sw.set_active(is_allowed)

            # Refresh Live Listening Apps with Switches
            self._refresh_live_apps(status)

            # Refresh Custom Rules
            self._refresh_custom_rules(status)

        finally:
            self.updating_ui = False

    def _refresh_live_apps(self, status: dict):
        for row in getattr(self, "_live_rows", []):
            self.live_group.remove(row)
        self._live_rows = []

        sockets = get_listening_sockets()

        if not sockets:
            empty_row = Adw.ActionRow(title=_("No external listening applications detected"))
            empty_row.set_icon_name("emblem-ok-symbolic")
            self.live_group.add(empty_row)
            self._live_rows.append(empty_row)
            return

        for sock in sockets:
            row = Adw.SwitchRow()
            row.set_title(sock["app_name"])
            port_label = _("Port")
            row.set_subtitle(f"{port_label}: {sock['port']}/{sock['proto'].upper()} • {sock['local_addr']}")
            row.set_icon_name(sock.get("icon") or "application-x-executable")

            # Switch state
            is_allowed = sock.get("is_allowed", False)
            row.set_active(is_allowed)
            row.connect("notify::active", self._make_app_switch_handler(sock["port"], sock["proto"], sock["app_name"]))

            self.live_group.add(row)
            self._live_rows.append(row)

    def _refresh_custom_rules(self, status: dict):
        for row in getattr(self, "_custom_rule_rows", []):
            self.rules_group.remove(row)
        self._custom_rule_rows = []

        rules = status.get("rules", [])
        if not rules:
            empty_row = Adw.ActionRow(title=_("No custom rules configured"))
            self.rules_group.add(empty_row)
            self._custom_rule_rows.append(empty_row)
            return

        # Deduplicate IPv4 and IPv6 for clean display
        seen = set()
        for r in rules:
            key = (r["port"], r["proto"], r["action"])
            if key in seen:
                continue
            seen.add(key)

            row = Adw.ActionRow()
            comment_str = f" • {r['comment']}" if r.get("comment") else ""
            proto_str = r['proto'].upper() if r.get('proto') else 'ANY'
            port_fmt = _("Port %s (%s)") % (r['port'], proto_str)
            row.set_title(f"{port_fmt}{comment_str}")

            action_fmt = _("Action: %s • Source: %s") % (r['action'], r['from_ip'])
            row.set_subtitle(action_fmt)
            row.set_icon_name("network-wired-symbolic")

            del_btn = Gtk.Button(icon_name="user-trash-symbolic")
            del_btn.add_css_class("flat")
            del_btn.set_tooltip_text(_("Delete Rule"))
            del_btn.connect("clicked", self._make_delete_rule_handler(r["port"], r["proto"]))
            row.add_suffix(del_btn)

            self.rules_group.add(row)
            self._custom_rule_rows.append(row)

    def _on_status_toggled(self, row, param):
        if self.updating_ui:
            return
        active = row.get_active()
        if active:
            ok, msg = run_helper("enable")
        else:
            ok, msg = run_helper("disable")
        GLib.idle_add(self.refresh_all)

    def _on_notify_toggled(self, row, param):
        if self.updating_ui:
            return
        prefs = load_preferences()
        prefs["notify_on_new_ports"] = row.get_active()
        save_preferences(prefs)

    def _on_reset_defaults(self, btn):
        ok, msg = run_helper("init-defaults")
        self.refresh_all()

    def _make_service_toggle_handler(self, svc: dict):
        def handler(row, param):
            if self.updating_ui:
                return
            active = row.get_active()
            if active:
                run_helper("allow", str(svc["port"]), svc["proto"], svc["comment"])
            else:
                run_helper("delete", str(svc["port"]), svc["proto"])
            GLib.idle_add(self.refresh_all)
        return handler

    def _make_app_switch_handler(self, port: int, proto: str, app_name: str):
        def handler(row, param):
            if self.updating_ui:
                return
            active = row.get_active()
            if active:
                run_helper("allow", str(port), proto, app_name)
            else:
                run_helper("delete", str(port), proto)
            GLib.idle_add(self.refresh_all)
        return handler

    def _make_delete_rule_handler(self, port: int, proto: str):
        def handler(btn):
            run_helper("delete", str(port), proto)
            GLib.idle_add(self.refresh_all)
        return handler

    def _on_add_rule_clicked(self, btn):
        dlg = AddRuleDialog(self, lambda: GLib.idle_add(self.refresh_all))
        dlg.present()


def main():
    app = Adw.Application(application_id=APP_ID)

    def on_activate(application):
        win = FirewallMainWindow(application)
        win.present()

    app.connect("activate", on_activate)
    app.run(sys.argv)


if __name__ == "__main__":
    main()
