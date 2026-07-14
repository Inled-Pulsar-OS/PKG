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
    width: 140px;
    height: 90px;
    border-radius: 8px;
    background-color: #f5f5f7;
    border: 1px solid #d2d2d7;
}
window .theme-preview-dark {
    width: 140px;
    height: 90px;
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
    faces = []
    faces_dir = "/usr/share/pixmaps/faces/"
    if not os.path.exists(faces_dir):
        raise FileNotFoundError(f"Face images directory not found: {faces_dir}")
    for file in os.listdir(faces_dir):
        if file.endswith(".jpg") or file.endswith(".png"):
            faces.append(os.path.join(faces_dir, file))
    if not faces:
        raise ValueError(f"No avatar images found in {faces_dir}")
    return sorted(faces)[:6]


class OOTBWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Pulsar OS Setup Assistant")
        self.set_default_size(720, 560)
        self.set_resizable(False)

        if "TEST_MODE" not in os.environ:
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
        self.build_language_page()
        self.build_keymap_page()
        self.build_timezone_page()
        self.build_account_page()
        self.build_theme_page()
        self.build_finished_page()

        # Show first page
        self.stack.set_visible_child_name("country_select")
        self.btn_back.set_visible(False)
        self.btn_header_back.set_visible(False)

        # States
        self.selected_country = None
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

        if current_page == "language_select":
            self.btn_next.set_sensitive(bool(self.selected_country))
            self.stack.set_visible_child_name("country_select")
            self.btn_back.set_visible(False)
            self.btn_header_back.set_visible(False)
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
            self.btn_next.set_sensitive(bool(self.selected_language))
            self.stack.set_visible_child_name("language_select")
            self.btn_back.set_visible(True)
            self.btn_header_back.set_visible(True)

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

            # Perform system configurations setup
            with open("/tmp/pulsar-ootb.log", "w") as log:
                log.write(f"Pulsar OS OOTB Config Log - {time.ctime()}\n")

            def run_setup_cmd(cmd):
                if isinstance(cmd, list):
                    if cmd[0] not in ["sudo", "echo"]:
                        cmd = ["sudo"] + cmd
                cmd_str = ' '.join(cmd) if isinstance(cmd, list) else cmd
                with open("/tmp/pulsar-ootb.log", "a") as log:
                    log.write(f"Executing: {cmd_str}\n")
                if "TEST_MODE" in os.environ:
                    with open("/tmp/pulsar-ootb.log", "a") as log:
                        log.write("[TEST_MODE] Bypassed execution.\n")
                    return
                res = subprocess.run(cmd, capture_output=True, text=True)
                with open("/tmp/pulsar-ootb.log", "a") as log:
                    log.write(f"STDOUT: {res.stdout}\n")
                    log.write(f"STDERR: {res.stderr}\n")
                if res.returncode != 0:
                    raise Exception(f"Failed command: {cmd_str}\n{res.stderr}")

            try:
                run_setup_cmd(["locale-gen", self.selected_language])
                run_setup_cmd(["localectl", "set-locale", f"LANG={self.selected_language}"])
                
                # 1. Try to install console-setup to make layout packages available
                try:
                    run_setup_cmd(["apt-get", "install", "-y", "console-setup", "keyboard-configuration"])
                except Exception as apt_err:
                    with open("/tmp/pulsar-ootb.log", "a") as log:
                        log.write(f"Warning: Failed to install console-setup: {apt_err}\n")

                # 2. Try to set keymap via localectl
                try:
                    run_setup_cmd(["localectl", "set-keymap", self.selected_keymap])
                except Exception as key_err:
                    with open("/tmp/pulsar-ootb.log", "a") as log:
                        log.write(f"Warning: localectl set-keymap failed: {key_err}\n")
                    
                    # 3. Fallback: Write directly to /etc/default/keyboard so it applies on reboot
                    try:
                        kb_content = (
                            'XKBMODEL="pc105"\n'
                            f'XKBLAYOUT="{self.selected_keymap}"\n'
                            'XKBVARIANT=""\n'
                            'XKBOPTIONS=""\n'
                            'BACKSPACE="guess"\n'
                        )
                        if "TEST_MODE" not in os.environ:
                            with open("/tmp/keyboard_tmp", "w") as f:
                                f.write(kb_content)
                            run_setup_cmd(["mv", "/tmp/keyboard_tmp", "/etc/default/keyboard"])
                    except Exception as kb_err:
                        with open("/tmp/pulsar-ootb.log", "a") as log:
                            log.write(f"Warning: Failed to write /etc/default/keyboard: {kb_err}\n")

                run_setup_cmd(["timedatectl", "set-timezone", self.selected_timezone])

                if "TEST_MODE" in os.environ:
                    print("[TEST_MODE] Simulating user creation and avatar copy...")
                else:
                    run_setup_cmd([
                        "useradd", "-m", "-G", "sudo,audio,video,plugdev",
                        "-s", "/bin/bash", username
                    ])

                    # Write user and root password securely without pipelines
                    cmd_user = ["sudo", "chpasswd"] if os.geteuid() != 0 else ["chpasswd"]
                    res = subprocess.run(cmd_user, input=f"{username}:{password}\n", capture_output=True, text=True)
                    if res.returncode != 0:
                        raise Exception(f"Failed to set user password: {res.stderr}")

                    cmd_root = ["sudo", "chpasswd"] if os.geteuid() != 0 else ["chpasswd"]
                    res = subprocess.run(cmd_root, input=f"root:{password}\n", capture_output=True, text=True)
                    if res.returncode != 0:
                        raise Exception(f"Failed to set root password: {res.stderr}")

                    # Configure keyboard layout for GNOME Wayland
                    try:
                        run_setup_cmd([
                            "sudo", "-u", username, "dbus-run-session", "gsettings", "set",
                            "org.gnome.desktop.input-sources", "sources", f"[('xkb', '{self.selected_keymap}')]"
                        ])
                    except Exception as gset_err:
                        with open("/tmp/pulsar-ootb.log", "a") as log:
                            log.write(f"Warning: Failed to set GNOME user keymap via gsettings: {gset_err}\n")

                    # Setup Real User Avatar
                    if self.selected_avatar_path and os.path.exists(self.selected_avatar_path):
                        user_home = f"/home/{username}"
                        if os.path.exists(user_home):
                            face_dest = os.path.join(user_home, ".face")
                            run_setup_cmd(["cp", self.selected_avatar_path, face_dest])
                            run_setup_cmd(["chown", f"{username}:{username}", face_dest])

                        as_icons_dir = "/var/lib/AccountsService/icons"
                        run_setup_cmd(["mkdir", "-p", as_icons_dir])
                        as_icon_dest = os.path.join(as_icons_dir, username)
                        run_setup_cmd(["cp", self.selected_avatar_path, as_icon_dest])
                        run_setup_cmd(["chown", "root:root", as_icon_dest])

                        as_user_file = f"/var/lib/AccountsService/users/{username}"
                        as_content = f"[User]\nLanguage={self.selected_language}\nXSession=gnome\nIcon={as_icon_dest}\nSystemAccount=false\n"
                        
                        # Write to tmp and move using sudo to bypass permissions
                        with open("/tmp/as_user_tmp", "w") as f:
                            f.write(as_content)
                        run_setup_cmd(["mkdir", "-p", os.path.dirname(as_user_file)])
                        run_setup_cmd(["mv", "/tmp/as_user_tmp", as_user_file])
                        run_setup_cmd(["chown", "root:root", as_user_file])

            except Exception as err:
                self.show_error(str(err))
                return

            self.load_log_to_view()
            self.stack.set_visible_child_name("finished")
            self.btn_next.set_label("Start using Pulsar OS")
            self.btn_back.set_visible(False)
            self.btn_header_back.set_visible(False)
            self.btn_next.set_sensitive(True)

        elif current_page == "finished":
            self.run_final_cleanup()

    def run_final_cleanup(self):
        try:
            if "TEST_MODE" in os.environ:
                print("[TEST_MODE] Simulating final system cleanup and reboot...")
                self.close()
                sys.exit(0)
            else:
                username = self.username_entry.get_text().strip()
                cleanup_script = f"""#!/bin/bash
sleep 2
echo "=== Background Cleanup Started ===" >> /tmp/pulsar-ootb.log

# 1. Kill any active processes owned by live user
pkill -9 -u live >> /tmp/pulsar-ootb.log 2>&1

# 2. Eliminate live user
userdel -f -r live >> /tmp/pulsar-ootb.log 2>&1

# 3. Delete OOTB witness file
if [ -f /etc/pulsar-need-setup ]; then
    rm -f /etc/pulsar-need-setup
    echo "Deleted /etc/pulsar-need-setup" >> /tmp/pulsar-ootb.log
fi

# 4. Configure SDDM autologin for the new user once
mkdir -p /etc/sddm.conf.d
cat <<EOF > /etc/sddm.conf.d/autologin.conf
[Autologin]
User={username}
Session=gnome
EOF
chmod 644 /etc/sddm.conf.d/autologin.conf
echo "Configured autologin for user {username}" >> /tmp/pulsar-ootb.log

# 5. Disable service
systemctl disable pulsar-ootb.service >> /tmp/pulsar-ootb.log 2>&1

# 6. Restart display manager
echo "Restarting display-manager..." >> /tmp/pulsar-ootb.log
systemctl restart display-manager >> /tmp/pulsar-ootb.log 2>&1
"""
                cleanup_path = "/tmp/pulsar-cleanup.sh"
                with open(cleanup_path, "w") as sf:
                    sf.write(cleanup_script)
                os.chmod(cleanup_path, 0o755)

                with open("/tmp/pulsar-ootb.log", "a") as log:
                    log.write("\n=== Scheduling Detached Background Cleanup ===\n")

                # Launch the script completely detached in a new session group
                subprocess.Popen([cleanup_path], start_new_session=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                
                self.close()
                sys.exit(0)
        except Exception as e:
            with open("/tmp/pulsar-ootb.log", "a") as log:
                log.write(f"Exception during final cleanup schedule: {e}\n")
            self.show_error(f"Error starting final cleanup:\n{e}")


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
