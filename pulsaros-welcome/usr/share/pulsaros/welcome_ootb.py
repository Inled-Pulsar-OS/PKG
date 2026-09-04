#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# ==============================================================================
# Pulsar OS - OOTB Setup Assistant (macOS Setup Assistant GTK4 & Libadwaita Style)
# ==============================================================================

import sys
import os

# Check OOTB witness flag before loading graphical libraries, bypass in test mode
if not os.path.exists("/etc/pulsar-need-setup") and "TEST_MODE" not in os.environ:
    print("OOTB setup not required. Exiting.")
    sys.exit(0)

import subprocess
import threading
import time
import re
import json
import shutil
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')

from gi.repository import Gtk, Gdk, GLib, Adw, Gio

# Custom CSS for Apple macOS OOTB Setup Look-and-Feel (LIGHT MODE)
CSS_DATA = """
/* ── Design System Variables ──────────────────────────────────────── */
window {
    --bg-color: #f5f5f7;
    --card-bg: #ffffff;
    --card-border: #d2d2d7;
    --text-color: #1d1d1f;
    --text-secondary: #86868b;
    --list-hover: #f5f5f7;
    --entry-bg: #ffffff;
    --avatar-hover: rgba(0,0,0,0.05);
}
.dark-theme {
    --bg-color: #1e1e1f;
    --card-bg: #2d2d2d;
    --card-border: #424242;
    --text-color: #f5f5f7;
    --text-secondary: #a1a1a6;
    --list-hover: #3a3a3c;
    --entry-bg: #1e1e1f;
    --avatar-hover: rgba(255,255,255,0.08);
}
/* ── Base & Fonts ── */
window, window * {
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
window, window label, window entry, window list, window list row {
    color: var(--text-color);
}
window {
    background-color: var(--bg-color);
}
window .apple-box {
    background-color: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 16px;
    padding: 32px 48px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.05);
}
/* ── Typography ───────────────────────────────────────────────────── */
window label.welcome-title {
    font-size: 24px;
    font-weight: 700;
    color: var(--text-color);
    margin-top: 10px;
    margin-bottom: 6px;
}
window label.welcome-subtitle {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 20px;
}
/* ── Buttons ──────────────────────────────────────────────────────── */
window .apple-box button.pulsar-continue-btn,
window button.pulsar-continue-btn {
    background-color: #0071e3;
    background-image: none;
    background: #0071e3;
    border-radius: 8px;
    font-weight: bold;
    padding: 8px 22px;
    border: none;
    box-shadow: none;
}
window .apple-box button.pulsar-continue-btn label,
window .apple-box button.pulsar-continue-btn *,
window button.pulsar-continue-btn label,
window button.pulsar-continue-btn * {
    color: #ffffff;
}
window .apple-box button.pulsar-continue-btn:hover,
window button.pulsar-continue-btn:hover {
    background-color: #007bf5;
    background-image: none;
    background: #007bf5;
    box-shadow: none;
}
window .apple-box button.pulsar-continue-btn:active,
window button.pulsar-continue-btn:active {
    background-color: #0063c6;
    background-image: none;
    background: #0063c6;
    box-shadow: none;
}
window .apple-box button.pulsar-continue-btn:disabled,
window button.pulsar-continue-btn:disabled {
    background-color: #e8e8ed;
    background-image: none;
    background: #e8e8ed;
    border: 1px solid #d2d2d7;
    box-shadow: none;
}
window .apple-box button.pulsar-continue-btn:disabled label,
window .apple-box button.pulsar-continue-btn:disabled *,
window button.pulsar-continue-btn:disabled label,
window button.pulsar-continue-btn:disabled * {
    color: #86868b;
}
window button.secondary-action {
    background-color: var(--bg-color);
    color: var(--text-color);
    border-radius: 8px;
    font-weight: bold;
    padding: 8px 22px;
    border: 1px solid var(--card-border);
}
window button.secondary-action:hover  { background-color: var(--list-hover); }
window button.secondary-action:active { background-color: var(--card-border); }
/* ── Back arrow ───────────────────────────────────────────────────── */
window button.back-arrow-btn {
    border-radius: 9999px;
    padding: 0;
    min-width: 32px;
    min-height: 32px;
    background: transparent;
    border: none;
    color: var(--text-color);
}
window button.back-arrow-btn:hover { background-color: var(--list-hover); }
/* ── Lists / Scrolled Windows (Ultra-high Specificity Overrides) ───── */
window scrolledwindow,
window scrolledwindow,
window scrolledwindow viewport,
window scrolledwindow list,
window scrolledwindow list row,
window list,
window list row,
window listview,
window listview row {
    background-color: var(--card-bg);
    background: var(--card-bg);
    color: var(--text-color);
}
window list row label,
window list row .country-row-label,
window listview row label {
    background-color: transparent;
    background: transparent;
    color: var(--text-color);
}
window list row,
window listview row {
    padding: 8px 14px;
    border-bottom: 1px solid var(--card-border);
    transition: background-color 0.1s ease;
}
window list row:last-child { border-bottom: none; }
window list row:hover,
window listview row:hover {
    background-color: var(--list-hover);
}
/* Blue selection — override Adw accent completely */
window list row:selected,
window listview row:selected,
window scrolledwindow list row:selected {
    background-color: #0071e3;
    background: #0071e3;
    color: #ffffff;
}
window list row:selected label,
window listview row:selected label,
window list row:selected .country-row-label,
window scrolledwindow list row:selected label {
    color: #ffffff;
}
/* ── Entries ──────────────────────────────────────────────────────── */
window entry {
    background-color: var(--entry-bg);
    border: 1px solid var(--card-border);
    border-radius: 6px;
    color: var(--text-color);
    padding: 6px 10px;
    font-size: 13px;
}
window entry:focus {
    border-color: #0071e3;
    box-shadow: 0 0 0 2px rgba(0,113,227,0.2);
}
/* ── Avatar: ring outside the Adw.Avatar circle ──────────────────── */
window button.avatar-btn {
    border-radius: 9999px;
    padding: 6px;
    border: 3.5px solid transparent;
    background-color: transparent;
    background: transparent;
    box-shadow: none;
    transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1), border-color 0.2s ease, background-color 0.2s ease;
}
window button.avatar-btn:hover {
    background-color: var(--avatar-hover);
    transform: scale(1.04);
}
window button.avatar-btn.selected {
    border: 3.5px solid #0071e3;
    background-color: transparent;
    background: transparent;
    transform: scale(1.1);
}
/* ── Theme Selection ─────────────────────────────────────────────── */
window .theme-card {
    border-radius: 12px;
    border: 2px solid var(--card-border);
    background-color: var(--card-bg);
    padding: 16px;
    transition: all 0.2s ease;
}
window .theme-card.selected {
    border-color: #0071e3;
    box-shadow: 0 0 0 2px rgba(0,113,227,0.25);
}
window label.theme-card-title {
    font-size: 14px;
    font-weight: bold;
    margin-top: 8px;
    color: var(--text-color);
}
window .theme-preview-light {
    min-width: 140px;
    min-height: 90px;
    border-radius: 8px;
    background-color: #f5f5f7;
    border: 1px solid #d2d2d7;
}
window .theme-preview-dark {
    min-width: 140px;
    min-height: 90px;
    border-radius: 8px;
    background-color: #1e1e1f;
    border: 1px solid #424242;
}
/* ── Misc ─────────────────────────────────────────────────────────── */
.search-box    { margin-bottom: 6px; }
.symbolic-blue { color: #0071e3; }
.input-label   { font-size: 13px; font-weight: 600; color: #1d1d1f; }
.input-subtext { font-size: 11px; color: #86868b; }
.error-text    { font-size: 11px; font-weight: bold; margin-top: 2px; }
.wifi-badge {
    background-color: rgba(0, 113, 227, 0.08);
    color: #0071e3;
    border-radius: 8px;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 600;
}
.wifi-badge.connected {
    background-color: rgba(52, 199, 89, 0.15);
    color: #248a3d;
}
.wifi-badge.error {
    background-color: rgba(255, 59, 48, 0.15);
    color: #d70015;
}
"""



COMMON_LANGS = {
    "es_ES": "Español (España)",
    "en_US": "English (United States)",
    "en_GB": "English (United Kingdom)",
    "fr_FR": "Français (France)",
    "de_DE": "Deutsch (Deutschland)",
    "it_IT": "Italiano (Italia)",
    "pt_BR": "Português (Brasil)",
    "pt_PT": "Português (Portugal)",
    "ru_RU": "Русский (Россия)",
    "zh_CN": "中文 (中国)",
    "ja_JP": "日本語 (日本)",
}

def get_all_countries():
    countries = []
    json_path = "/usr/share/iso-codes/json/iso_3166-1.json"
    if not os.path.exists(json_path):
        raise FileNotFoundError(f"Country database not found: {json_path}")
    with open(json_path, "r") as f:
        data = json.load(f)
        for entry in data.get("3166-1", []):
            name = entry.get("name")
            if name:
                countries.append(name)
    if not countries:
        raise ValueError("No countries found in country database.")
    return sorted(list(set(countries)))

def get_all_supported_locales():
    locales = []
    supported_path = "/usr/share/i18n/SUPPORTED"
    if not os.path.exists(supported_path):
        raise FileNotFoundError(f"Supported locales database not found: {supported_path}")
    with open(supported_path, "r") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) >= 1:
                loc = parts[0]
                if "UTF-8" in loc or "utf8" in loc or "utf-8" in loc:
                    locales.append(loc)
    if not locales:
        raise ValueError("No UTF-8 locales found in supported locales.")
    return sorted(list(set(locales)))

def get_readable_locale_name(loc):
    code = loc.split(".")[0]
    if code in COMMON_LANGS:
        return COMMON_LANGS[code]
    parts = code.split("_")
    if len(parts) == 2:
        return f"{parts[0].lower()} ({parts[1].upper()})"
    return code

def get_all_keymaps():
    out = subprocess.check_output(["localectl", "list-x11-keymap-layouts"], text=True)
    layouts = [line.strip() for line in out.split("\n") if line.strip()]
    if not layouts:
        raise ValueError("No keyboard layouts returned by localectl.")
    return sorted(list(set(layouts)))

def get_all_timezones():
    out = subprocess.check_output(["timedatectl", "list-timezones"], text=True)
    timezones = [line.strip() for line in out.split("\n") if line.strip()]
    if not timezones:
        raise ValueError("No timezones returned by timedatectl.")
    return sorted(list(set(timezones)))

def get_system_face_images():
    # Debian ships default GNOME faces in /usr/share/pixmaps/faces/.
    # Arch has no system faces, so we fall back to the bundled set shipped
    # by pulsaros-welcome under /usr/share/pulsaros/avatars/.
    faces = []
    for faces_dir in ("/usr/share/pixmaps/faces/", "/usr/share/pulsaros/avatars/"):
        if not os.path.isdir(faces_dir):
            continue
        for file in sorted(os.listdir(faces_dir)):
            if file.endswith(".jpg") or file.endswith(".png"):
                faces.append(os.path.join(faces_dir, file))
        if faces:
            break
    if not faces:
        return faces
    return sorted(faces)[:6]


def get_existing_groups():
    groups = set()
    try:
        with open("/etc/group", "r") as f:
            for line in f:
                if ":" in line:
                    groups.add(line.split(":")[0])
    except Exception:
        pass
    return groups


def get_active_network():
    """Returns dict with active connection info or None: {'type': 'wifi'|'ethernet', 'name': str, 'device': str}"""
    if "TEST_MODE" in os.environ:
        return {"type": "wifi", "name": "Skynet-5", "device": "wlan0"}
    try:
        res = subprocess.run(
            ["nmcli", "-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "dev"],
            capture_output=True, text=True, timeout=3
        )
        if res.returncode == 0:
            for line in res.stdout.strip().splitlines():
                parts = line.split(":")
                if len(parts) >= 4:
                    dev, dev_type, state, conn = parts[0], parts[1], parts[2], parts[3]
                    if state == "connected":
                        return {"type": dev_type, "name": conn or dev, "device": dev}
    except Exception:
        pass
    return None


def get_wifi_scan_results():
    """Returns list of unique wifi networks sorted by signal strength."""
    if "TEST_MODE" in os.environ:
        return [
            {"ssid": "Skynet-5", "signal": 100, "security": "WPA3", "in_use": True},
            {"ssid": "Office_WiFi", "signal": 80, "security": "WPA2", "in_use": False},
            {"ssid": "Guest_Network", "signal": 60, "security": "", "in_use": False},
        ]
    networks = []
    seen = set()
    try:
        res = subprocess.run(
            ["nmcli", "-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "dev", "wifi", "list"],
            capture_output=True, text=True, timeout=6
        )
        if res.returncode == 0:
            for line in res.stdout.strip().splitlines():
                parts = line.split(":")
                if len(parts) >= 4:
                    ssid = parts[0].replace("\\:", ":").strip()
                    if not ssid:
                        continue
                    if ssid in seen:
                        continue
                    seen.add(ssid)
                    try:
                        sig = int(parts[1])
                    except Exception:
                        sig = 50
                    sec = parts[2].strip()
                    in_use = "*" in parts[3]
                    networks.append({
                        "ssid": ssid,
                        "signal": sig,
                        "security": sec,
                        "in_use": in_use
                    })
    except Exception:
        pass
    networks.sort(key=lambda x: (not x["in_use"], -x["signal"]))
    return networks


def nmcli_connect_wifi(ssid, password=""):
    """Connect to a Wi-Fi network. Returns (success: bool, message: str)"""
    if "TEST_MODE" in os.environ:
        return True, "Connected successfully."
    try:
        cmd = ["nmcli", "dev", "wifi", "connect", ssid]
        if password:
            cmd.extend(["password", password])
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=25)
        if res.returncode == 0:
            return True, res.stdout.strip() or "Connected successfully."
        else:
            return False, res.stderr.strip() or res.stdout.strip() or "Connection failed."
    except Exception as e:
        return False, str(e)


class OOTBWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Pulsar OS Setup Assistant")
        self.set_default_size(720, 560)
        self.set_resizable(False)

        if "TEST_MODE" not in os.environ:
            self.set_decorated(False)
            display = Gdk.Display.get_default()
            monitors = display.get_monitors()
            if monitors and len(monitors) > 0:
                geo = monitors[0].get_geometry()
                self.set_default_size(geo.width, geo.height)
            else:
                self.set_default_size(1920, 1080)
            self.fullscreen()

        self.apply_css()

        # Center layout
        root_overlay = Gtk.CenterBox()
        root_overlay.set_hexpand(True)
        root_overlay.set_vexpand(True)

        self.card_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.card_box.add_css_class("apple-box")
        self.card_box.set_size_request(700, 520)
        self.card_box.set_valign(Gtk.Align.CENTER)
        self.card_box.set_halign(Gtk.Align.CENTER)

        # Custom Header Bar with back arrow on top-left of the card
        self.header_bar_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.header_bar_box.set_margin_bottom(10)

        self.btn_header_back = Gtk.Button()
        self.btn_header_back.set_child(Gtk.Image.new_from_icon_name("go-previous-symbolic"))
        self.btn_header_back.add_css_class("back-arrow-btn")
        self.btn_header_back.connect("clicked", self.on_back_clicked)
        self.btn_header_back.set_visible(False)
        self.header_bar_box.append(self.btn_header_back)

        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        self.header_bar_box.append(spacer)

        self.card_box.append(self.header_bar_box)

        # ViewStack for pages
        self.stack = Adw.ViewStack()
        self.card_box.append(self.stack)

        # Navigation bar
        self.nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.nav_box.set_margin_top(16)
        self.nav_box.set_margin_bottom(10)

        self.btn_back = Gtk.Button(label="Back")
        self.btn_back.add_css_class("secondary-action")
        self.btn_back.connect("clicked", self.on_back_clicked)
        self.nav_box.append(self.btn_back)

        spacer2 = Gtk.Box()
        spacer2.set_hexpand(True)
        self.nav_box.append(spacer2)

        # Next Button
        self.btn_next = Gtk.Button(label="Continue")
        self.btn_next.add_css_class("pulsar-continue-btn")
        self.btn_next.connect("clicked", self.on_next_clicked)
        self.btn_next.set_sensitive(False)
        self.nav_box.append(self.btn_next)

        self.card_box.append(self.nav_box)

        root_overlay.set_center_widget(self.card_box)
        self.set_content(root_overlay)

        # Load system configurations dynamically
        self.country_list = get_all_countries()
        self.locale_list = get_all_supported_locales()
        self.keymap_list = get_all_keymaps()
        self.timezone_list = get_all_timezones()
        self.face_images = get_system_face_images()

        # Build pages
        self.build_country_page()
        self.build_wifi_page()
        self.build_language_page()
        self.build_keymap_page()
        self.build_timezone_page()
        self.build_account_page()
        self.build_theme_page()
        self.build_progress_page()
        self.build_finished_page()

        # Show first page
        self.stack.set_visible_child_name("country_select")
        self.btn_back.set_visible(False)
        self.btn_header_back.set_visible(False)

        # States
        self.selected_country = None
        self.selected_wifi_ssid = None
        self.selected_language = None
        self.selected_keymap = None
        self.selected_timezone = None
        self.selected_avatar_path = self.face_images[0] if self.face_images else None
        self.selected_theme = "light"

    def apply_css(self):
        # Force light palette so Adw never overrides our white/blue colours
        Adw.StyleManager.get_default().set_color_scheme(Adw.ColorScheme.FORCE_LIGHT)
        display = Gdk.Display.get_default()
        if not display:
            return

        # 1. Load the system theme (MacTahoe-Dark) stylesheet directly to ensure consistency in chroots
        theme_paths = [
            "/usr/share/themes/MacTahoe-Dark/gtk-4.0/gtk.css",
            "/etc/skel/.config/gtk-4.0/gtk.css"
        ]
        for path in theme_paths:
            if os.path.exists(path):
                try:
                    theme_provider = Gtk.CssProvider()
                    theme_provider.load_from_path(path)
                    Gtk.StyleContext.add_provider_for_display(
                        display,
                        theme_provider,
                        Gtk.STYLE_PROVIDER_PRIORITY_THEME
                    )
                    break
                except Exception as ex:
                    print(f"Failed to load theme from {path}: {ex}")

        # 2. Load our custom app overrides
        provider = Gtk.CssProvider()
        provider.load_from_data(CSS_DATA.encode('utf-8'))
        Gtk.StyleContext.add_provider_for_display(
            display,
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_USER
        )

    def create_input_field(self, label_text, entry_widget, subtext=None):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        box.set_margin_bottom(8)

        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        lbl = Gtk.Label(label=label_text)
        lbl.set_size_request(130, -1)
        lbl.set_halign(Gtk.Align.END)
        lbl.add_css_class("input-label")
        row.append(lbl)

        entry_widget.set_hexpand(True)
        row.append(entry_widget)
        box.append(row)

        if subtext:
            sub_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
            spacer = Gtk.Box()
            spacer.set_size_request(130, -1)
            sub_row.append(spacer)

            sub_lbl = Gtk.Label(label=subtext)
            sub_lbl.add_css_class("input-subtext")
            sub_lbl.set_halign(Gtk.Align.START)
            sub_row.append(sub_lbl)
            box.append(sub_row)

        return box

    def build_country_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        globe_icon = Gtk.Image.new_from_icon_name("preferences-desktop-locale-symbolic")
        globe_icon.set_pixel_size(72)
        globe_icon.add_css_class("symbolic-blue")
        box.append(globe_icon)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Select Your Country or Region</span>")
        title.add_css_class("welcome-title")
        box.append(title)

        # Theme Selector (Light / Dark) right at the start
        theme_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        theme_box.set_halign(Gtk.Align.CENTER)
        theme_box.set_margin_bottom(8)
        theme_box.add_css_class("linked")

        self.btn_light = Gtk.ToggleButton(label="Light Mode")
        self.btn_light.set_active(True)
        self.btn_light.connect("toggled", self.on_theme_toggled, False)
        theme_box.append(self.btn_light)

        self.btn_dark = Gtk.ToggleButton(label="Dark Mode")
        self.btn_dark.connect("toggled", self.on_theme_toggled, True)
        theme_box.append(self.btn_dark)

        self.btn_dark.set_group(self.btn_light)
        box.append(theme_box)

        # Search Entry for filtering countries
        self.country_search = Gtk.SearchEntry()
        self.country_search.set_placeholder_text("Search country...")
        self.country_search.add_css_class("search-box")
        self.country_search.set_size_request(320, -1)
        self.country_search.connect("search-changed", self.on_country_search_changed)
        box.append(self.country_search)

        scrolled = Gtk.ScrolledWindow()
        scrolled.set_size_request(320, 180)
        scrolled.add_css_class("country-scroll")
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)

        self.country_listbox = Gtk.ListBox()
        self.country_listbox.connect("row-selected", self.on_country_row_selected)
        self.country_listbox.set_filter_func(self.filter_country_row)
        scrolled.set_child(self.country_listbox)
        box.append(scrolled)

        for c in self.country_list:
            row = Gtk.ListBoxRow()
            row.country_name = c
            row.add_css_class("country-row")
            lbl = Gtk.Label(label=c)
            lbl.add_css_class("country-row-label")
            lbl.set_halign(Gtk.Align.START)
            row.set_child(lbl)
            self.country_listbox.append(row)

        self.stack.add_named(box, "country_select")

    def filter_country_row(self, row):
        query = self.country_search.get_text().lower().strip()
        if not query:
            return True
        return query in row.country_name.lower()

    def on_theme_toggled(self, button, is_dark):
        if button.get_active():
            self.set_theme_dark(is_dark)

    def on_country_search_changed(self, entry):
        self.country_listbox.invalidate_filter()

    def on_country_row_selected(self, listbox, row):
        if row is not None:
            self.selected_country = row.country_name
            self.btn_next.set_sensitive(True)

    def build_wifi_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        icon = Gtk.Image.new_from_icon_name("network-wireless-symbolic")
        icon.set_pixel_size(72)
        icon.add_css_class("symbolic-blue")
        box.append(icon)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Wi-Fi Connection</span>")
        title.add_css_class("welcome-title")
        box.append(title)

        desc = Gtk.Label(label="Connect to the internet to download language packages and system components.")
        desc.add_css_class("welcome-subtitle")
        desc.set_max_width_chars(45)
        desc.set_wrap(True)
        desc.set_justify(Gtk.Justification.CENTER)
        box.append(desc)

        # Status Badge
        self.wifi_status_badge = Gtk.Label(label="Checking network status...")
        self.wifi_status_badge.add_css_class("wifi-badge")
        self.wifi_status_badge.set_halign(Gtk.Align.CENTER)
        box.append(self.wifi_status_badge)

        # Open GNOME Settings Button
        settings_btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        settings_btn_box.set_halign(Gtk.Align.CENTER)
        settings_btn_box.set_margin_top(8)

        self.open_wifi_btn = Gtk.Button()
        btn_content = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        btn_icon = Gtk.Image.new_from_icon_name("network-wireless-symbolic")
        btn_lbl = Gtk.Label(label="Open Wi-Fi Settings")
        btn_content.append(btn_icon)
        btn_content.append(btn_lbl)
        self.open_wifi_btn.set_child(btn_content)
        self.open_wifi_btn.add_css_class("pulsar-continue-btn")
        self.open_wifi_btn.connect("clicked", self.on_open_wifi_settings_clicked)
        settings_btn_box.append(self.open_wifi_btn)

        # Refresh / Check button
        self.wifi_refresh_btn = Gtk.Button()
        self.wifi_refresh_btn.set_icon_name("view-refresh-symbolic")
        self.wifi_refresh_btn.set_tooltip_text("Check connection status")
        self.wifi_refresh_btn.add_css_class("back-arrow-btn")
        self.wifi_refresh_btn.connect("clicked", lambda b: self.refresh_wifi_status())
        settings_btn_box.append(self.wifi_refresh_btn)

        box.append(settings_btn_box)

        hint = Gtk.Label(label="Configure your network in GNOME Settings and click Continue.")
        hint.add_css_class("input-subtext")
        hint.set_halign(Gtk.Align.CENTER)
        hint.set_margin_top(4)
        box.append(hint)

        self.stack.add_named(box, "wifi_select")

    def on_open_wifi_settings_clicked(self, btn):
        try:
            subprocess.Popen(["gnome-control-center", "wifi"])
        except Exception:
            try:
                subprocess.Popen(["gnome-control-center", "network"])
            except Exception:
                pass
        # Periodically check status after opening settings
        GLib.timeout_add(1500, self.refresh_wifi_status)

    def refresh_wifi_status(self):
        def _check():
            active = get_active_network()
            GLib.idle_add(self._update_wifi_status_ui, active)

        threading.Thread(target=_check, daemon=True).start()
        return False

    def _update_wifi_status_ui(self, active):
        if active:
            conn_type = "Wi-Fi" if active["type"] == "wifi" else "Ethernet"
            name = active["name"]
            self.wifi_status_badge.set_label(f"✓ Connected to {name} ({conn_type})")
            self.wifi_status_badge.remove_css_class("error")
            self.wifi_status_badge.add_css_class("connected")
        else:
            self.wifi_status_badge.set_label("Not connected — You can connect via Settings or skip")
            self.wifi_status_badge.remove_css_class("connected")
            self.wifi_status_badge.remove_css_class("error")
        self.btn_next.set_sensitive(True)

    def build_language_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        icon_theme = Gtk.IconTheme.get_for_display(Gdk.Display.get_default())
        icon_name = "languages-symbolic" if icon_theme.has_icon("languages-symbolic") else "preferences-desktop-locale-symbolic"

        icon = Gtk.Image.new_from_icon_name(icon_name)
        icon.set_pixel_size(72)
        icon.add_css_class("symbolic-blue")
        box.append(icon)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Primary Language</span>")
        title.add_css_class("welcome-title")
        box.append(title)

        desc = Gtk.Label(label="Select the primary language to use for the system.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)

        # Search Box
        self.lang_search = Gtk.SearchEntry()
        self.lang_search.set_placeholder_text("Search language...")
        self.lang_search.add_css_class("search-box")
        self.lang_search.set_size_request(320, -1)
        self.lang_search.connect("search-changed", self.on_lang_search_changed)
        box.append(self.lang_search)

        scrolled = Gtk.ScrolledWindow()
        scrolled.set_size_request(320, 140)
        scrolled.add_css_class("country-scroll")
        self.lang_listbox = Gtk.ListBox()
        self.lang_listbox.connect("row-selected", self.on_lang_row_selected)
        self.lang_listbox.set_filter_func(self.filter_lang_row)
        scrolled.set_child(self.lang_listbox)
        box.append(scrolled)

        for loc in self.locale_list:
            row = Gtk.ListBoxRow()
            row.lang_val = loc
            row.lang_name = get_readable_locale_name(loc)
            row.add_css_class("country-row")
            lbl = Gtk.Label(label=row.lang_name)
            lbl.add_css_class("country-row-label")
            lbl.set_halign(Gtk.Align.START)
            row.set_child(lbl)
            self.lang_listbox.append(row)

        self.stack.add_named(box, "language_select")

    def filter_lang_row(self, row):
        query = self.lang_search.get_text().lower().strip()
        if not query:
            return True
        name = row.lang_name.lower()
        val = row.lang_val.lower()
        return (query in name) or (query in val)

    def on_lang_search_changed(self, entry):
        self.lang_listbox.invalidate_filter()
        if self.selected_language:
            self.btn_next.set_sensitive(True)

    def on_lang_row_selected(self, listbox, row):
        if row is not None:
            self.selected_language = row.lang_val
            self.btn_next.set_sensitive(True)

    def build_keymap_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        icon = Gtk.Image.new_from_icon_name("input-keyboard-symbolic")
        icon.set_pixel_size(72)
        icon.add_css_class("symbolic-blue")
        box.append(icon)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Keyboard Layout</span>")
        title.add_css_class("welcome-title")
        box.append(title)

        desc = Gtk.Label(label="Select the input method for your keyboard.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)

        # Search Box
        self.keymap_search = Gtk.SearchEntry()
        self.keymap_search.set_placeholder_text("Search keyboard...")
        self.keymap_search.add_css_class("search-box")
        self.keymap_search.set_size_request(320, -1)
        self.keymap_search.connect("search-changed", self.on_keymap_search_changed)
        box.append(self.keymap_search)

        scrolled = Gtk.ScrolledWindow()
        scrolled.set_size_request(320, 140)
        scrolled.add_css_class("country-scroll")
        self.keymap_listbox = Gtk.ListBox()
        self.keymap_listbox.connect("row-selected", self.on_keymap_row_selected)
        self.keymap_listbox.set_filter_func(self.filter_keymap_row)
        scrolled.set_child(self.keymap_listbox)
        box.append(scrolled)

        for k in self.keymap_list:
            row = Gtk.ListBoxRow()
            row.key_val = k
            row.add_css_class("country-row")
            lbl = Gtk.Label(label=k)
            lbl.add_css_class("country-row-label")
            lbl.set_halign(Gtk.Align.START)
            row.set_child(lbl)
            self.keymap_listbox.append(row)

        self.stack.add_named(box, "keymap_select")

    def filter_keymap_row(self, row):
        query = self.keymap_search.get_text().lower().strip()
        if not query:
            return True
        return query in row.key_val.lower()

    def on_keymap_search_changed(self, entry):
        self.keymap_listbox.invalidate_filter()
        if self.selected_keymap:
            self.btn_next.set_sensitive(True)

    def on_keymap_row_selected(self, listbox, row):
        if row is not None:
            self.selected_keymap = row.key_val
            self.btn_next.set_sensitive(True)

    def build_timezone_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        icon = Gtk.Image.new_from_icon_name("mark-location-symbolic")
        icon.set_pixel_size(72)
        icon.add_css_class("symbolic-blue")
        box.append(icon)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Time Zone</span>")
        title.add_css_class("welcome-title")
        box.append(title)

        desc = Gtk.Label(label="Configure your local time zone.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)

        # Search Box
        self.tz_search = Gtk.SearchEntry()
        self.tz_search.set_placeholder_text("Search timezone...")
        self.tz_search.add_css_class("search-box")
        self.tz_search.set_size_request(320, -1)
        self.tz_search.connect("search-changed", self.on_tz_search_changed)
        box.append(self.tz_search)

        scrolled = Gtk.ScrolledWindow()
        scrolled.set_size_request(320, 140)
        scrolled.add_css_class("country-scroll")
        self.tz_listbox = Gtk.ListBox()
        self.tz_listbox.connect("row-selected", self.on_tz_row_selected)
        self.tz_listbox.set_filter_func(self.filter_tz_row)
        scrolled.set_child(self.tz_listbox)
        box.append(scrolled)

        for tz in self.timezone_list:
            row = Gtk.ListBoxRow()
            row.tz_val = tz
            row.add_css_class("country-row")
            lbl = Gtk.Label(label=tz)
            lbl.add_css_class("country-row-label")
            lbl.set_halign(Gtk.Align.START)
            row.set_child(lbl)
            self.tz_listbox.append(row)

        self.stack.add_named(box, "timezone")

    def filter_tz_row(self, row):
        query = self.tz_search.get_text().lower().strip()
        if not query:
            return True
        return query in row.tz_val.lower()

    def on_tz_search_changed(self, entry):
        self.tz_listbox.invalidate_filter()
        if self.selected_timezone:
            self.btn_next.set_sensitive(True)

    def on_tz_row_selected(self, listbox, row):
        if row is not None:
            self.selected_timezone = row.tz_val
            self.btn_next.set_sensitive(True)

    def build_account_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        box.set_margin_start(20)
        box.set_margin_end(20)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Create a Pulsar Account</span>")
        title.add_css_class("welcome-title")
        box.append(title)

        desc = Gtk.Label(label="The password you create here will be used to log in to this computer.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)

        # Horizontal User Avatars (real JPG system faces styled inside circular Adw.Avatar)
        avatar_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        avatar_box.set_halign(Gtk.Align.CENTER)
        avatar_box.set_margin_bottom(16)
        box.append(avatar_box)

        self.avatar_buttons = []
        if self.face_images:
            for idx, av_path in enumerate(self.face_images):
                btn = Gtk.Button()
                btn.set_has_frame(False)
                btn.avatar_path = av_path
                btn.add_css_class("avatar-btn")

                # Use Adw.Avatar for circular cropping
                avatar = Adw.Avatar()
                avatar.set_size(60)
                avatar.set_can_target(False)
                try:
                    file = Gio.File.new_for_path(av_path)
                    texture = Gdk.Texture.new_from_file(file)
                    avatar.set_custom_image(texture)
                except Exception as e:
                    print(f"Error loading avatar image: {e}")
                    avatar.set_text("👤")

                btn.set_child(avatar)
                btn.connect("clicked", self.on_avatar_clicked)
                avatar_box.append(btn)
                self.avatar_buttons.append(btn)
        else:
            fallback_faces = ["🐼", "🦊", "🦉", "👤"]
            for av in fallback_faces:
                btn = Gtk.Button()
                btn.set_has_frame(False)
                btn.avatar_path = av
                btn.add_css_class("avatar-btn")
                lbl = Gtk.Label(label=av)
                lbl.add_css_class("avatar-label")
                lbl.set_can_target(False)
                btn.set_child(lbl)
                btn.connect("clicked", self.on_avatar_clicked)
                avatar_box.append(btn)
                self.avatar_buttons.append(btn)

        if self.avatar_buttons:
            self.avatar_buttons[0].add_css_class("selected")
            # Pre-set avatar so Continue works even if user doesn't click
            self.selected_avatar_path = self.avatar_buttons[0].avatar_path

        # Aligned form inputs
        form_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        form_box.set_size_request(420, -1)
        box.append(form_box)

        self.fullname_entry = Gtk.Entry()
        self.fullname_entry.connect("changed", self.on_fullname_changed)
        form_box.append(self.create_input_field("Full name", self.fullname_entry))

        self.username_entry = Gtk.Entry()
        form_box.append(self.create_input_field("Account name", self.username_entry, "This will be the name of your home folder."))

        self.password_entry = Gtk.PasswordEntry()
        self.password_entry.connect("changed", self.validate_passwords)
        form_box.append(self.create_input_field("Password", self.password_entry))

        self.confirm_entry = Gtk.PasswordEntry()
        self.confirm_entry.connect("changed", self.validate_passwords)
        form_box.append(self.create_input_field("Verify", self.confirm_entry))

        # Real-time password validation warning label
        self.error_label = Gtk.Label(label="")
        self.error_label.add_css_class("error-text")
        self.error_label.set_halign(Gtk.Align.START)
        self.error_label.set_margin_start(146)
        form_box.append(self.error_label)

        self.hint_entry = Gtk.Entry()
        form_box.append(self.create_input_field("Hint", self.hint_entry))

        self.stack.add_named(box, "account")

    def validate_passwords(self, entry):
        p1 = self.password_entry.get_text()
        p2 = self.confirm_entry.get_text()
        fullname = self.fullname_entry.get_text().strip()
        username = self.username_entry.get_text().strip()

        self.btn_next.set_sensitive(False)

        if not p1 or not p2:
            self.error_label.set_label("")
            return

        if p1 != p2:
            self.error_label.set_markup("<span color='#ff3b30'>Passwords do not match</span>")
        elif len(p1) < 4:
            self.error_label.set_markup("<span color='#ff3b30'>Password is too short</span>")
        else:
            self.error_label.set_markup("<span color='#34c759'>Passwords match</span>")
            if fullname and username:
                self.btn_next.set_sensitive(True)

    def on_fullname_changed(self, entry):
        fullname = entry.get_text()
        sanitized = re.sub(r'[^a-z0-9_-]', '', fullname.lower().replace(' ', ''))
        self.username_entry.set_text(sanitized[:16])
        self.validate_passwords(None)

    def on_avatar_clicked(self, btn):
        for b in self.avatar_buttons:
            b.remove_css_class("selected")
        btn.add_css_class("selected")
        self.selected_avatar_path = btn.avatar_path

    def build_theme_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        icon = Gtk.Image.new_from_icon_name("preferences-desktop-display-symbolic")
        icon.set_pixel_size(72)
        icon.add_css_class("symbolic-blue")
        box.append(icon)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Choose Your Look</span>")
        title.add_css_class("welcome-title")
        box.append(title)

        desc = Gtk.Label(label="Select an appearance style for Pulsar OS.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)

        # Horizontal Box for Light / Dark options
        cards_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=24)
        cards_box.set_halign(Gtk.Align.CENTER)
        box.append(cards_box)

        # Light Theme Card
        self.light_card = Gtk.Button()
        self.light_card.add_css_class("theme-card")
        self.light_card.add_css_class("selected") # default
        
        light_vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        light_vbox.set_halign(Gtk.Align.CENTER)
        
        light_preview = Gtk.Frame()
        light_preview.add_css_class("theme-preview-light")
        light_vbox.append(light_preview)
        
        light_lbl = Gtk.Label(label="Light")
        light_lbl.add_css_class("theme-card-title")
        light_vbox.append(light_lbl)
        
        self.light_card.set_child(light_vbox)
        self.light_card.connect("clicked", self.on_light_theme_selected)
        cards_box.append(self.light_card)

        # Dark Theme Card
        self.dark_card = Gtk.Button()
        self.dark_card.add_css_class("theme-card")
        
        dark_vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        dark_vbox.set_halign(Gtk.Align.CENTER)
        
        dark_preview = Gtk.Frame()
        dark_preview.add_css_class("theme-preview-dark")
        dark_vbox.append(dark_preview)
        
        dark_lbl = Gtk.Label(label="Dark")
        dark_lbl.add_css_class("theme-card-title")
        dark_vbox.append(dark_lbl)
        
        self.dark_card.set_child(dark_vbox)
        self.dark_card.connect("clicked", self.on_dark_theme_selected)
        cards_box.append(self.dark_card)

        self.stack.add_named(box, "theme_select")

    def on_light_theme_selected(self, btn):
        self.light_card.add_css_class("selected")
        self.dark_card.remove_css_class("selected")
        self.set_theme_dark(False)

    def on_dark_theme_selected(self, btn):
        self.dark_card.add_css_class("selected")
        self.light_card.remove_css_class("selected")
        self.set_theme_dark(True)

    def set_theme_dark(self, is_dark):
        if is_dark:
            self.add_css_class("dark-theme")
            Adw.StyleManager.get_default().set_color_scheme(Adw.ColorScheme.FORCE_DARK)
            self.selected_theme = "dark"
        else:
            self.remove_css_class("dark-theme")
            Adw.StyleManager.get_default().set_color_scheme(Adw.ColorScheme.FORCE_LIGHT)
            self.selected_theme = "light"
    def build_progress_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=24)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        # Large beautiful loading spinner
        self.progress_spinner = Gtk.Spinner()
        self.progress_spinner.set_size_request(64, 64)
        self.progress_spinner.add_css_class("suggested-action")
        box.append(self.progress_spinner)

        # Title
        self.progress_title = Gtk.Label(label="Configuring your system...")
        self.progress_title.add_css_class("welcome-title")
        box.append(self.progress_title)

        # Subtitle
        sub_label = Gtk.Label(label="Please wait while Pulsar OS sets up your user account and system files.")
        sub_label.add_css_class("welcome-subtitle")
        box.append(sub_label)

        self.stack.add_named(box, "setup_progress")

    def build_finished_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)

        image = Gtk.Image.new_from_icon_name("object-select-symbolic")
        image.set_pixel_size(80)
        image.add_css_class("suggested-action")
        box.append(image)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>All Set</span>")
        title.add_css_class("welcome-title")
        box.append(title)

        desc = Gtk.Label(label="Your computer is ready to use. Below is the configuration log.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)

        # Monospace Scrollable Text Log Area
        self.log_buffer = Gtk.TextBuffer()
        self.log_view = Gtk.TextView(buffer=self.log_buffer)
        self.log_view.set_editable(False)
        self.log_view.set_cursor_visible(False)
        self.log_view.set_monospace(True)
        self.log_view.set_wrap_mode(Gtk.WrapMode.WORD)

        log_scroll = Gtk.ScrolledWindow()
        log_scroll.set_size_request(480, 140)
        log_scroll.set_child(self.log_view)
        log_scroll.add_css_class("country-scroll")
        box.append(log_scroll)

        # Copy Log Button
        copy_btn = Gtk.Button(label="Copy Setup Log")
        copy_btn.add_css_class("secondary-action")
        copy_btn.connect("clicked", self.on_copy_log_clicked)
        copy_btn.set_halign(Gtk.Align.CENTER)
        box.append(copy_btn)

        self.stack.add_named(box, "finished")

    def on_copy_log_clicked(self, btn):
        clipboard = self.get_clipboard()
        text = self.log_buffer.get_text(self.log_buffer.get_start_iter(), self.log_buffer.get_end_iter(), True)
        clipboard.set_text(text)

    def load_log_to_view(self):
        log_path = "/tmp/pulsar-ootb.log"
        content = ""
        if os.path.exists(log_path):
            try:
                with open(log_path, "r") as f:
                    content = f.read()
            except Exception as e:
                content = f"Error reading log: {e}"
        else:
            content = "Log file not found."
        
        self.log_buffer.set_text(content)

    def on_back_clicked(self, btn):
        current_page = self.stack.get_visible_child_name()

        if current_page == "wifi_select":
            self.btn_next.set_sensitive(bool(self.selected_country))
            self.stack.set_visible_child_name("country_select")
            self.btn_back.set_visible(False)
            self.btn_header_back.set_visible(False)
        elif current_page == "language_select":
            self.btn_next.set_sensitive(True)
            self.stack.set_visible_child_name("wifi_select")
        elif current_page == "keymap_select":
            self.btn_next.set_sensitive(bool(self.selected_language))
            self.stack.set_visible_child_name("language_select")
        elif current_page == "timezone":
            self.btn_next.set_sensitive(bool(self.selected_keymap))
            self.stack.set_visible_child_name("keymap_select")
        elif current_page == "account":
            self.btn_next.set_sensitive(bool(self.selected_timezone))
            self.stack.set_visible_child_name("timezone")
        elif current_page == "theme_select":
            self.btn_next.set_sensitive(True)
            self.btn_next.set_label("Continue")
            self.stack.set_visible_child_name("account")

    def show_error(self, message):
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="Setup Error",
            body=message
        )
        dialog.add_response("ok", "OK")
        dialog.add_response("view_log", "View Log")
        dialog.set_default_response("ok")

        def on_response(d, response_id):
            if response_id == "view_log":
                subprocess.Popen(["xdg-open", "/tmp/pulsar-ootb.log"])
            d.destroy()

        dialog.connect("response", on_response)
        dialog.present()

    def select_listbox_row_by_value(self, listbox, key_attr, target_value):
        row = listbox.get_first_child()
        while row is not None:
            val = getattr(row, key_attr, None)
            if val == target_value:
                listbox.select_row(row)
                break
            row = row.get_next_sibling()

    def select_listbox_row_by_index(self, listbox, index):
        idx = 0
        row = listbox.get_first_child()
        while row is not None:
            if idx == index:
                listbox.select_row(row)
                break
            row = row.get_next_sibling()
            idx += 1

    def on_next_clicked(self, btn):
        current_page = self.stack.get_visible_child_name()

        if current_page == "country_select":
            if not self.selected_country:
                return
            self.btn_next.set_sensitive(True)
            self.stack.set_visible_child_name("wifi_select")
            self.btn_back.set_visible(True)
            self.btn_header_back.set_visible(True)
            self.refresh_wifi_status()

        elif current_page == "wifi_select":
            self.btn_next.set_sensitive(bool(self.selected_language))
            self.stack.set_visible_child_name("language_select")

        elif current_page == "language_select":
            if not self.selected_language:
                return
            self.btn_next.set_sensitive(bool(self.selected_keymap))
            self.stack.set_visible_child_name("keymap_select")

        elif current_page == "keymap_select":
            if not self.selected_keymap:
                return
            self.btn_next.set_sensitive(bool(self.selected_timezone))
            self.stack.set_visible_child_name("timezone")

        elif current_page == "timezone":
            if not self.selected_timezone:
                return
            # Account page: enable if passwords already filled (e.g. going back and forward)
            self.btn_next.set_sensitive(False)
            self.stack.set_visible_child_name("account")

        elif current_page == "account":
            fullname = self.fullname_entry.get_text().strip()
            username = self.username_entry.get_text().strip()
            password = self.password_entry.get_text().strip()
            confirm = self.confirm_entry.get_text().strip()

            if not fullname or not username or not password:
                self.show_error("All fields are required.")
                return
            if password != confirm:
                self.show_error("Passwords do not match.")
                return
            if not re.match(r"^[a-z0-9_-]{3,16}$", username):
                self.show_error("Account name must be 3-16 characters and alphanumeric.")
                return

            # Disable navigation buttons to prevent double click/concurrency
            self.btn_next.set_sensitive(False)
            self.btn_back.set_sensitive(False)
            self.btn_header_back.set_sensitive(False)

            # Show progress page and start spinner
            self.stack.set_visible_child_name("setup_progress")
            self.progress_spinner.start()

            # Start configuration in a background thread
            threading.Thread(
                target=self.run_setup_backend,
                args=(fullname, username, password),
                daemon=True
            ).start()

        elif current_page == "finished":
            self.run_final_cleanup()

    def run_setup_backend(self, fullname, username, password):
        try:
            with open("/tmp/pulsar-ootb.log", "w") as log:
                log.write(f"Pulsar OS OOTB Config Log - {time.ctime()}\n")

            def log_msg(msg):
                with open("/tmp/pulsar-ootb.log", "a") as log:
                    log.write(f"{msg}\n")

            def run_cmd(cmd, check=True):
                cmd_str = ' '.join(cmd) if isinstance(cmd, list) else cmd
                log_msg(f"Executing: {cmd_str}")
                if "TEST_MODE" in os.environ:
                    log_msg("[TEST_MODE] Bypassed.")
                    return subprocess.CompletedProcess(cmd, 0, "", "")
                res = subprocess.run(cmd, capture_output=True, text=True)
                log_msg(f"  exit={res.returncode} stdout={res.stdout.strip()} stderr={res.stderr.strip()}")
                if check and res.returncode != 0:
                    raise Exception(f"Command failed (exit {res.returncode}): {cmd_str}\n{res.stderr.strip()}")
                return res

            def write_temp_and_move(content, dest):
                tmp = dest + ".tmp"
                with open(tmp, "w") as f:
                    f.write(content)
                os.replace(tmp, dest)

            # ── Locale ─────────────────────────────────────────────
            # selected_language is like "es_ES"; the generated locale name
            # needs the encoding suffix (es_ES.UTF-8) to be valid.
            locale_code = self.selected_language
            locale_full = f"{locale_code}.UTF-8"
            # Make sure the locale is enabled in /etc/locale.gen and generated.
            # Both Debian and Arch read /etc/locale.gen (locale-gen takes no args).
            try:
                locale_gen_path = "/etc/locale.gen"
                with open(locale_gen_path, "r") as f:
                    content = f.read()
                new_lines = []
                found = False
                for line in content.splitlines():
                    stripped = line.strip()
                    cand = stripped.lstrip("#").strip()
                    if cand and cand.split() and cand.split()[0] == locale_full:
                        found = True
                        new_lines.append(f"{locale_full} UTF-8")
                    else:
                        new_lines.append(line)
                if not found:
                    new_lines.append(f"{locale_full} UTF-8")
                write_temp_and_move("\n".join(new_lines) + "\n", locale_gen_path)
                run_cmd(["locale-gen"], check=False)
            except Exception as e:
                log_msg(f"Warning: could not enable locale in /etc/locale.gen: {e}")

            try:
                run_cmd(["localectl", "set-locale", f"LANG={locale_full}"])
            except Exception:
                if os.path.exists("/etc/debian_version"):
                    write_temp_and_move(f'LANG="{locale_full}"\n', "/etc/default/locale")
                    run_cmd(["chown", "root:root", "/etc/default/locale"], check=False)
                    run_cmd(["chmod", "644", "/etc/default/locale"], check=False)
                else:
                    # Arch reads /etc/locale.conf (not /etc/default/locale)
                    write_temp_and_move(f"LANG={locale_full}\n", "/etc/locale.conf")
                    run_cmd(["chown", "root:root", "/etc/locale.conf"], check=False)
                    run_cmd(["chmod", "644", "/etc/locale.conf"], check=False)

            # ── OCR & Tesseract Language Pack ──────────────────────
            try:
                TESS_MAP = {
                    "es": "spa", "en": "eng", "fr": "fra", "de": "deu",
                    "it": "ita", "pt": "por", "zh": "chi_sim", "ja": "jpn",
                    "ru": "rus", "ar": "ara", "ca": "cat", "gl": "glg", "eu": "eus"
                }
                lang_prefix = locale_code.split("_")[0].lower() if "_" in locale_code else locale_code.lower()
                tess_pkg = TESS_MAP.get(lang_prefix, "eng")
                log_msg(f"Configuring Tesseract OCR data for language: {tess_pkg} (locale: {locale_code})")
                active_net = get_active_network()
                if active_net:
                    log_msg("Network connected, installing OCR data package...")
                    if os.path.exists("/etc/debian_version"):
                        run_cmd(["apt-get", "install", "-y", f"tesseract-ocr-{tess_pkg}"], check=False)
                    else:
                        run_cmd(["pacman", "-S", "--noconfirm", "--needed", f"tesseract-data-{tess_pkg}"], check=False)
                else:
                    log_msg("Offline mode: skipping remote package installation for tesseract data.")
            except Exception as e:
                log_msg(f"Warning: failed to install tesseract language pack: {e}")

            # ── Keyboard ──────────────────────────────────────────
            # console-setup / keyboard-configuration only exist on Debian.
            # On Arch, localectl set-keymap works out of the box and kbd is
            # already part of the base system, so we skip the package install.
            if os.path.exists("/etc/debian_version"):
                run_cmd(["apt-get", "install", "-y", "console-setup", "keyboard-configuration", "kbd"], check=False)
            try:
                run_cmd(["localectl", "set-keymap", self.selected_keymap])
            except Exception:
                log_msg("Fallback: writing /etc/default/keyboard directly")
                kb_content = (
                    'XKBMODEL="pc105"\n'
                    f'XKBLAYOUT="{self.selected_keymap}"\n'
                    'XKBVARIANT=""\n'
                    'XKBOPTIONS=""\n'
                    'BACKSPACE="guess"\n'
                )
                write_temp_and_move(kb_content, "/etc/default/keyboard")

            # ── Timezone ──────────────────────────────────────────
            try:
                run_cmd(["timedatectl", "set-timezone", self.selected_timezone])
            except Exception:
                log_msg("Fallback: manual timezone symlink")
                run_cmd(["ln", "-sf", f"/usr/share/zoneinfo/{self.selected_timezone}", "/etc/localtime"], check=False)
                write_temp_and_move(f"{self.selected_timezone}\n", "/etc/timezone")
                run_cmd(["chown", "root:root", "/etc/timezone"], check=False)
                run_cmd(["chmod", "644", "/etc/timezone"], check=False)

            if "TEST_MODE" in os.environ:
                log_msg("[TEST_MODE] Skipping user creation.")
            else:
                # ── Lock live user to prevent login ───────────────
                log_msg(f"Locking live user account...")
                run_cmd(["usermod", "-L", "-s", "/usr/sbin/nologin", "live"], check=False)

                # ── Create real user with useradd ─────────────────
                log_msg(f"Creating user '{username}' via useradd...")
                user_home = f"/home/{username}"
                # Some groups are distro/package specific (plugdev exists on
                # Debian but not Arch; docker only if docker is installed), so
                # only add secondary groups that actually exist.
                desired_groups = ["sudo", "wheel", "audio", "video", "plugdev", "docker"]
                existing_groups = get_existing_groups()
                extra_groups = ",".join(g for g in desired_groups if g in existing_groups)
                run_cmd([
                    "useradd", "-m",
                    "-d", user_home,
                    "-s", "/bin/bash",
                    "-G", extra_groups,
                    "-c", fullname,
                    username
                ])

                # Verify user was created in /etc/passwd
                res = run_cmd(["grep", f"^{username}:", "/etc/passwd"])
                if f"{username}:" not in res.stdout:
                    raise Exception(f"User '{username}' not found in /etc/passwd after useradd")

                # Verify home directory exists
                if not os.path.isdir(user_home):
                    raise Exception(f"Home directory {user_home} does not exist after useradd")

                # ── Grant sudo to the new user ─────────────────────
                # Debian's sudoers grants %sudo, but Arch's Pulsar sudoers
                # does not grant %wheel or %sudo, so we also drop a sudoers
                # rule for the user. Both distros include /etc/sudoers.d.
                sudoers_user_file = f"/etc/sudoers.d/pulsaros-user-{username}"
                write_temp_and_move(f"{username} ALL=(ALL:ALL) ALL\n", sudoers_user_file)
                run_cmd(["chown", "root:root", sudoers_user_file])
                run_cmd(["chmod", "0440", sudoers_user_file])
                log_msg(f"Granted sudo to '{username}' via {sudoers_user_file}")

                log_msg(f"User '{username}' verified in /etc/passwd, home={user_home}")

                # ── Update /etc/hosts ─────────────────────────────
                try:
                    with open("/etc/hosts", "r") as f:
                        hosts = f.read()
                    if "pulsaros" not in hosts:
                        lines = hosts.splitlines()
                        new_lines = []
                        for line in lines:
                            if line.startswith("127.0.0.1"):
                                new_lines.append(line + " pulsaros")
                            else:
                                new_lines.append(line)
                        write_temp_and_move("\n".join(new_lines) + "\n", "/etc/hosts")
                        log_msg("Updated /etc/hosts with 'pulsaros' alias")
                except Exception as e:
                    log_msg(f"Warning: Failed to update /etc/hosts: {e}")

                # ── Set passwords ─────────────────────────────────
                res = subprocess.run(
                    ["chpasswd"],
                    input=f"{username}:{password}\n",
                    capture_output=True, text=True
                )
                if res.returncode != 0:
                    raise Exception(f"Failed to set user password: {res.stderr}")
                log_msg("User password set successfully")

                res = subprocess.run(
                    ["chpasswd"],
                    input=f"root:{password}\n",
                    capture_output=True, text=True
                )
                if res.returncode != 0:
                    raise Exception(f"Failed to set root password: {res.stderr}")
                log_msg("Root password set successfully")

                # ── Avatar ────────────────────────────────────────
                as_icon_dest = ""
                if self.selected_avatar_path and os.path.exists(self.selected_avatar_path):
                    if os.path.isdir(user_home):
                        face_dest = os.path.join(user_home, ".face")
                        run_cmd(["cp", self.selected_avatar_path, face_dest])
                        run_cmd(["chown", f"{username}:{username}", face_dest])

                    as_icons_dir = "/var/lib/AccountsService/icons"
                    run_cmd(["mkdir", "-p", as_icons_dir])
                    as_icon_dest = os.path.join(as_icons_dir, username)
                    run_cmd(["cp", self.selected_avatar_path, as_icon_dest])
                    run_cmd(["chown", "root:root", as_icon_dest])

                # ── AccountsService ───────────────────────────────
                as_user_file = f"/var/lib/AccountsService/users/{username}"
                as_content = (
                    f"[User]\n"
                    f"Language={self.selected_language}\n"
                    f"Session=gnome\n"
                    f"XSession=gnome\n"
                    f"SystemAccount=false\n"
                )
                if as_icon_dest:
                    as_content += f"Icon={as_icon_dest}\n"

                run_cmd(["mkdir", "-p", "/var/lib/AccountsService/users"])
                write_temp_and_move(as_content, as_user_file)
                run_cmd(["chown", "root:root", as_user_file])
                run_cmd(["chmod", "600", as_user_file])
                log_msg(f"AccountsService config written to {as_user_file}")

                # Restart accounts-daemon so it picks up the new user
                res = run_cmd(["systemctl", "restart", "accounts-daemon.service"], check=False)
                if res.returncode != 0:
                    log_msg("accounts-daemon not available, trying accounts-daemon")
                    run_cmd(["systemctl", "restart", "accounts-daemon"], check=False)

                # ── GNOME keymap for the new user ─────────────────
                run_cmd([
                    "sudo", "-u", username, "dbus-run-session", "gsettings", "set",
                    "org.gnome.desktop.input-sources", "sources", f"[('xkb', '{self.selected_keymap}')]"
                ], check=False)

                # ── macOS keybindings + spotlight for the new user ──
                # Mirrors the PulsarOS system dconf DB (/etc/dconf/db/local) so the
                # macOS Super<->Ctrl swap (XKB xkb-options), the spotlight on
                # <Ctrl>space and the rest of the macOS shortcuts survive in the
                # user DB even if system dconf is absent or a later tool overwrites
                # them (e.g. the old gnome-macos-remap install.sh used to reset
                # xkb-options and bind <Primary>space to Show Applications).
                mac_user_settings = [
                    ("org.gnome.desktop.input-sources", "xkb-options",
                     "['ctrl:swap_lwin_lctl', 'ctrl:swap_rwin_rctl']"),
                    ("org.gnome.mutter", "overlay-key", "'Super_R'"),
                    ("org.gnome.desktop.wm.keybindings", "minimize", "['<Primary>m']"),
                    ("org.gnome.desktop.wm.keybindings", "show-desktop", "['<Primary>d']"),
                    ("org.gnome.desktop.wm.keybindings", "switch-applications", "['<Primary>Tab']"),
                    ("org.gnome.desktop.wm.keybindings", "switch-applications-backward", "['<Primary><Shift>Tab']"),
                    ("org.gnome.desktop.wm.keybindings", "switch-group", "['<Primary>grave']"),
                    ("org.gnome.desktop.wm.keybindings", "switch-group-backward", "['<Primary><Shift>grave']"),
                    ("org.gnome.mutter.keybindings", "toggle-tiled-left", "[]"),
                    ("org.gnome.mutter.keybindings", "toggle-tiled-right", "[]"),
                    ("org.gnome.desktop.wm.keybindings", "switch-to-workspace-left", "['<Super>Left']"),
                    ("org.gnome.desktop.wm.keybindings", "switch-to-workspace-right", "['<Super>Right']"),
                    ("org.gnome.shell.keybindings", "toggle-overview", "['LaunchA']"),
                    ("org.gnome.shell.keybindings", "toggle-application-view", "['LaunchB']"),
                    ("org.gnome.shell.keybindings", "toggle-message-tray", "[]"),
                    ("org.gnome.shell.keybindings", "screenshot", "['<Primary><Shift>numbersign']"),
                    ("org.gnome.shell.keybindings", "show-screenshot-ui", "['Print', '<Shift><Control>dollar', '<Shift><Super>4', '<Shift><Super>5']"),
                    ("org.gnome.shell.keybindings", "screenshot-window", "['<Shift><Control>percent']"),
                    ("org.gnome.settings-daemon.plugins.media-keys", "screensaver", "['<Super>l', '<Control>l']"),
                    ("org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings",
                     "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/', '/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom1/']"),
                ]
                for schema, key, value in mac_user_settings:
                    run_cmd([
                        "sudo", "-u", username, "dbus-run-session", "gsettings", "set",
                        schema, key, value
                    ], check=False)

                spotlight_path_ctrl = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/"
                for key, value in [
                    ("name", "'Spotlight'"),
                    ("command", "'pulsaros-spotlight'"),
                    ("binding", "'<Ctrl>space'"),
                ]:
                    run_cmd([
                        "sudo", "-u", username, "dbus-run-session", "gsettings", "set",
                        "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:" + spotlight_path_ctrl,
                        key, value
                    ], check=False)

                spotlight_path_super = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom1/"
                for key, value in [
                    ("name", "'Spotlight'"),
                    ("command", "'pulsaros-spotlight'"),
                    ("binding", "'<Super>space'"),
                ]:
                    run_cmd([
                        "sudo", "-u", username, "dbus-run-session", "gsettings", "set",
                        "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:" + spotlight_path_super,
                        key, value
                    ], check=False)

            GLib.idle_add(self.on_setup_completed)

        except Exception as err:
            GLib.idle_add(self.on_setup_failed, str(err))

    def on_setup_completed(self):
        self.progress_spinner.stop()
        self.load_log_to_view()
        self.stack.set_visible_child_name("finished")
        self.btn_next.set_label("Start using Pulsar OS")
        self.btn_back.set_visible(False)
        self.btn_header_back.set_visible(False)
        self.btn_next.set_sensitive(True)

    def on_setup_failed(self, error_message):
        self.progress_spinner.stop()
        self.show_error(error_message)
        self.stack.set_visible_child_name("account")
        self.btn_next.set_sensitive(True)
        self.btn_back.set_sensitive(True)
        self.btn_header_back.set_sensitive(True)

    def run_final_cleanup(self):
        try:
            if "TEST_MODE" in os.environ:
                print("[TEST_MODE] Simulating final system cleanup and reboot...")
                self.close()
                sys.exit(0)
            else:
                username = self.username_entry.get_text().strip()
                log_path = "/tmp/pulsar-ootb.log"

                def log_msg(msg):
                    with open(log_path, "a") as log:
                        log.write(f"{msg}\n")

                # 1. Remove temporary sudoers grant for live user
                log_msg("Cleaning up temporary sudoers for live user...")
                try:
                    os.remove("/etc/sudoers.d/pulsar-ootb-live")
                except FileNotFoundError:
                    pass

                # 2. Remove residual display manager config files
                try:
                    subprocess.run(
                        "rm -f /etc/sddm.conf /etc/sddm.conf.d/live"
                        " /etc/lightdm/lightdm.conf.d/* 2>/dev/null",
                        shell=True
                    )
                except Exception:
                    pass

                # 3. Configure SDDM autologin for the new real user
                os.makedirs("/etc/sddm.conf.d", exist_ok=True)
                with open("/etc/sddm.conf.d/autologin.conf", "w") as f:
                    f.write(f"[Autologin]\nUser={username}\nSession=gnome\nRelogin=false\n")
                os.chmod("/etc/sddm.conf.d/autologin.conf", 0o644)
                log_msg(f"SDDM autologin configured for '{username}'")

                # 4. Delete OOTB witness file
                if os.path.exists("/etc/pulsar-need-setup"):
                    os.remove("/etc/pulsar-need-setup")
                    log_msg("Removed /etc/pulsar-need-setup")

                # 5. Create cleanup sentinel (will be picked up on first login)
                with open("/etc/pulsar-need-cleanup", "w") as f:
                    f.write(username)
                log_msg("Created /etc/pulsar-need-cleanup")

                # 6. Disable pulsar-ootb service (no longer needed)
                subprocess.run(
                    ["systemctl", "disable", "pulsar-ootb.service"],
                    capture_output=True
                )
                log_msg("Disabled pulsar-ootb.service")

                # 6. Restart SDDM so it re-reads AccountsService and shows the new user
                log_msg("Restarting SDDM to pick up new user...")
                res = subprocess.run(
                    ["systemctl", "restart", "sddm"],
                    capture_output=True, text=True
                )
                if res.returncode != 0:
                    log_msg(f"SDDM restart returned non-zero: {res.stderr.strip()}")

                log_msg("Setup complete. Rebooting in 3 seconds...")
                time.sleep(3)

                subprocess.Popen(["systemctl", "reboot"], start_new_session=True)

                self.close()
                sys.exit(0)
        except Exception as e:
            with open("/tmp/pulsar-ootb.log", "a") as log:
                log.write(f"Exception during final cleanup: {e}\n")
            self.show_error(f"Error during cleanup:\n{e}")


class OOTBApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id="es.inled.pulsaros.welcome_ootb",
            flags=Gio.ApplicationFlags.FLAGS_NONE
        )

    def do_activate(self):
        try:
            win = OOTBWindow(self)
            win.present()
        except Exception as e:
            # Create a minimal window to anchor the error dialog
            win = Adw.ApplicationWindow(application=self)
            win.set_default_size(480, 240)
            win.present()
            
            dialog = Adw.MessageDialog(
                transient_for=win,
                heading="Initialization Error",
                body=f"Failed to start Pulsar OS Setup Assistant:\n\n{e}\n\nThis setup cannot continue."
            )
            dialog.add_response("exit", "Exit")
            dialog.set_default_response("exit")
            
            def on_response(d, response_id):
                d.destroy()
                win.destroy()
                sys.exit(1)
                
            dialog.connect("response", on_response)
            dialog.present()


if __name__ == "__main__":
    app = OOTBApp()
    sys.exit(app.run(sys.argv))
