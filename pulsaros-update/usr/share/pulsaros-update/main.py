#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# ==============================================================================
# Pulsar OS - System Update and Migration Helper (GTK4 / Libadwaita UI)
# ==============================================================================

import sys
import os
import threading
import time

AUTOSTART_FLAG_PATH = os.path.expanduser("~/.config/pulsaros/update-v1-done")
if "--autostart" in sys.argv:
    if os.path.exists(AUTOSTART_FLAG_PATH) and "--force" not in sys.argv:
        sys.exit(0)

import gi
gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')

from gi.repository import Gtk, Gdk, GLib, Adw, Gio
from updater_core import UpdateCore, POPULAR_LANGUAGES

CSS_DATA = """
window.pulsar-update-window {
    background-color: #f6f6f6;
    color: #1d1d1f;
}

@media (prefers-color-scheme: dark) {
    window.pulsar-update-window {
        background-color: #1e1e1e;
        color: #f5f5f7;
    }
}

.sidebar-pane {
    background-color: rgba(0, 0, 0, 0.02);
    border-right: 1px solid rgba(0, 0, 0, 0.08);
}

@media (prefers-color-scheme: dark) {
    .sidebar-pane {
        background-color: rgba(255, 255, 255, 0.02);
        border-right: 1px solid rgba(255, 255, 255, 0.08);
    }
}

.sidebar-row {
    padding: 10px 14px;
    border-radius: 8px;
    margin: 2px 8px;
    font-weight: 500;
}

.sidebar-row:selected {
    background-color: #0071e3;
    color: #ffffff;
}

.pulsar-header-title {
    font-size: 18px;
    font-weight: 700;
}

.pulsar-header-desc {
    font-size: 13px;
    color: #86868b;
}

.badge-ok {
    background-color: rgba(52, 199, 89, 0.16);
    color: #248a3d;
    font-weight: 600;
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 12px;
}

@media (prefers-color-scheme: dark) {
    .badge-ok {
        background-color: rgba(48, 209, 88, 0.2);
        color: #30d158;
    }
}

.badge-warn {
    background-color: rgba(255, 149, 0, 0.16);
    color: #c97000;
    font-weight: 600;
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 12px;
}

@media (prefers-color-scheme: dark) {
    .badge-warn {
        background-color: rgba(255, 159, 10, 0.2);
        color: #ff9f0a;
    }
}

.badge-error {
    background-color: rgba(255, 59, 48, 0.16);
    color: #d70015;
    font-weight: 600;
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 12px;
}

@media (prefers-color-scheme: dark) {
    .badge-error {
        background-color: rgba(255, 69, 58, 0.2);
        color: #ff453a;
    }
}

.btn-master-action {
    background-color: #0071e3;
    color: #ffffff;
    font-weight: 600;
    border-radius: 8px;
    padding: 8px 16px;
}

.btn-master-action:hover {
    background-color: #0077ed;
}

.log-view-box {
    font-family: monospace;
    font-size: 12px;
    background-color: #121214;
    color: #38ef7d;
    padding: 12px;
    border-radius: 10px;
}
"""


class PulsarUpdateApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id="es.inled.pulsaros.update",
            flags=Gio.ApplicationFlags.FLAGS_NONE
        )

    def do_activate(self):
        win = self.props.active_window
        if not win:
            win = PulsarUpdateWindow(application=self)
        win.present()


class PulsarUpdateWindow(Adw.ApplicationWindow):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.set_title("Pulsar OS Update Assistant")
        self.set_default_size(960, 640)
        self.add_css_class("pulsar-update-window")

        style_provider = Gtk.CssProvider()
        style_provider.load_from_data(CSS_DATA.encode('utf-8'))
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        self.core = UpdateCore(log_callback=self.on_log_received)
        self.is_running_task = False

        self.setup_ui()
        self.refresh_all_status()

    def on_log_received(self, message: str):
        GLib.idle_add(self._append_log, message)

    def _append_log(self, message: str):
        buf = self.log_text_view.get_buffer()
        end_iter = buf.get_end_iter()
        buf.insert(end_iter, f"{message}\n")
        mark = buf.create_mark(None, buf.get_end_iter(), False)
        self.log_text_view.scroll_to_mark(mark, 0.0, True, 0.0, 1.0)

    def setup_ui(self):
        self.toast_overlay = Adw.ToastOverlay()
        self.set_content(self.toast_overlay)

        self.split_view = Adw.NavigationSplitView()
        self.toast_overlay.set_child(self.split_view)

        # ── CONTENT PANE (Stack) ─────────────────────────────────────
        self.content_stack = Gtk.Stack()
        self.content_stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self.content_stack.set_transition_duration(150)
        self.content_stack.set_vexpand(True)
        self.content_stack.set_hexpand(True)

        self.page_overview = self.build_overview_page()
        self.content_stack.add_named(self.page_overview, "overview")

        self.page_lang = self.build_language_page()
        self.content_stack.add_named(self.page_lang, "language")

        self.page_recovery = self.build_recovery_page()
        self.content_stack.add_named(self.page_recovery, "recovery")

        self.page_sayri = self.build_sayri_page()
        self.content_stack.add_named(self.page_sayri, "sayri")

        self.page_hibernate = self.build_hibernation_page()
        self.content_stack.add_named(self.page_hibernate, "hibernate")

        self.page_extras = self.build_extras_page()
        self.content_stack.add_named(self.page_extras, "extras")

        self.page_logs = self.build_logs_page()
        self.content_stack.add_named(self.page_logs, "logs")

        # Wrap content in ToolbarView
        content_toolbar_view = Adw.ToolbarView()
        content_header = Adw.HeaderBar()
        content_toolbar_view.add_top_bar(content_header)

        btn_refresh = Gtk.Button(icon_name="view-refresh-symbolic")
        btn_refresh.set_tooltip_text("Check for updates and rescan system")
        btn_refresh.connect("clicked", lambda b: self.refresh_all_status())
        content_header.pack_start(btn_refresh)

        self.btn_top_update = Gtk.Button(label="Update All Components")
        self.btn_top_update.set_icon_name("software-update-available-symbolic")
        self.btn_top_update.add_css_class("btn-master-action")
        self.btn_top_update.connect("clicked", self.on_master_update_clicked)
        content_header.pack_end(self.btn_top_update)

        content_toolbar_view.set_content(self.content_stack)
        content_page = Adw.NavigationPage.new(content_toolbar_view, "Details")
        self.split_view.set_content(content_page)

        # ── SIDEBAR PANE ─────────────────────────────────────────────
        sidebar_page = Adw.NavigationPage.new(self.build_sidebar_widget(), "Navigation")
        self.split_view.set_sidebar(sidebar_page)

    def show_toast(self, message: str, timeout: int = 4):
        toast = Adw.Toast.new(message)
        toast.set_timeout(timeout)
        self.toast_overlay.add_toast(toast)

    def build_sidebar_widget(self) -> Gtk.Widget:
        toolbar_view = Adw.ToolbarView()
        toolbar_view.add_css_class("sidebar-pane")

        header = Adw.HeaderBar()
        header.set_show_title(True)
        toolbar_view.add_top_bar(header)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        box.set_vexpand(True)
        toolbar_view.set_content(box)

        # App branding header
        brand_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        brand_box.set_margin_top(14)
        brand_box.set_margin_bottom(12)
        brand_box.set_margin_start(16)
        brand_box.set_margin_end(16)

        icon_img = Gtk.Image.new_from_icon_name("system-software-update-symbolic")
        icon_img.set_pixel_size(24)
        brand_box.append(icon_img)

        lbl_brand = Gtk.Label(label="Pulsar OS Update")
        lbl_brand.add_css_class("pulsar-header-title")
        brand_box.append(lbl_brand)
        box.append(brand_box)

        # Sidebar ListBox
        self.sidebar_list = Gtk.ListBox()
        self.sidebar_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.sidebar_list.add_css_class("navigation-sidebar")
        self.sidebar_list.connect("row-selected", self.on_sidebar_row_selected)
        box.append(self.sidebar_list)

        items = [
            ("overview", "preferences-system-details-symbolic", "Overview"),
            ("language", "preferences-desktop-locale-symbolic", "Language and Apps"),
            ("recovery", "drive-harddisk-symbolic", "Recovery Assistant"),
            ("sayri", "audio-input-microphone-symbolic", "Sayri AI"),
            ("hibernate", "night-light-symbolic", "Hibernation"),
            ("extras", "preferences-other-symbolic", "System Extras"),
            ("logs", "utilities-terminal-symbolic", "Activity Log"),
        ]

        self.sidebar_rows = {}
        for name, icon_name, title in items:
            row = Gtk.ListBoxRow()
            row.name = name
            row.add_css_class("sidebar-row")

            row_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            row_icon = Gtk.Image.new_from_icon_name(icon_name)
            row_icon.set_pixel_size(18)
            row_box.append(row_icon)

            row_lbl = Gtk.Label(label=title, halign=Gtk.Align.START)
            row_lbl.set_hexpand(True)
            row_box.append(row_lbl)

            row.set_child(row_box)
            self.sidebar_list.append(row)
            self.sidebar_rows[name] = row

        self.sidebar_list.select_row(self.sidebar_rows["overview"])

        # Bottom section in sidebar
        bottom_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        bottom_box.set_margin_top(16)
        bottom_box.set_margin_bottom(16)
        bottom_box.set_margin_start(14)
        bottom_box.set_margin_end(14)

        btn_all = Gtk.Button(label="Update All Components")
        btn_all.set_icon_name("software-update-available-symbolic")
        btn_all.add_css_class("btn-master-action")
        btn_all.connect("clicked", self.on_master_update_clicked)
        bottom_box.append(btn_all)

        box.append(Gtk.Box(vexpand=True))
        box.append(bottom_box)

        return toolbar_view

    def on_sidebar_row_selected(self, listbox, row):
        if row is not None and hasattr(row, 'name') and hasattr(self, 'content_stack'):
            self.content_stack.set_visible_child_name(row.name)

    # -------------------------------------------------------------------------
    # 1. OVERVIEW PAGE
    # -------------------------------------------------------------------------
    def build_overview_page(self) -> Gtk.Widget:
        scroll = Gtk.ScrolledWindow()
        scroll.set_vexpand(True)
        scroll.set_hexpand(True)

        clamp = Adw.Clamp(maximum_size=820, tightening_threshold=640)
        clamp.set_vexpand(True)
        scroll.set_child(clamp)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        box.set_margin_top(24)
        box.set_margin_bottom(24)
        box.set_margin_start(20)
        box.set_margin_end(20)
        clamp.set_child(box)

        banner = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        icon_img = Gtk.Image.new_from_icon_name("system-software-update-symbolic")
        icon_img.set_pixel_size(44)
        banner.append(icon_img)

        txt_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        lbl_title = Gtk.Label(label="Pulsar OS System Status", halign=Gtk.Align.START)
        lbl_title.add_css_class("pulsar-header-title")
        lbl_desc = Gtk.Label(
            label="Migration and configuration assistant to align existing installations with the latest Pulsar OS standards.",
            halign=Gtk.Align.START,
            wrap=True
        )
        lbl_desc.add_css_class("pulsar-header-desc")
        txt_box.append(lbl_title)
        txt_box.append(lbl_desc)
        banner.append(txt_box)
        box.append(banner)

        group = Adw.PreferencesGroup(title="System Components")
        box.append(group)

        # 0. Core Packages
        self.row_pkgs = Adw.ActionRow(title="0. Official System Packages (Inled Repo)", subtitle="Checking...")
        self.row_pkgs.add_prefix(Gtk.Image.new_from_icon_name("system-software-update-symbolic"))
        self.badge_pkgs = Gtk.Label(label="...")
        self.row_pkgs.add_suffix(self.badge_pkgs)
        btn_fix_pkgs = Gtk.Button(label="Update")
        btn_fix_pkgs.set_icon_name("software-update-available-symbolic")
        btn_fix_pkgs.connect("clicked", lambda b: self.run_async_task(self.core.update_pulsar_packages, "Updating all Pulsar OS core packages..."))
        self.row_pkgs.add_suffix(btn_fix_pkgs)
        group.add(self.row_pkgs)

        # 1. Recovery
        self.row_rec = Adw.ActionRow(title="1. Native Recovery Assistant (Rust)", subtitle="Checking...")
        self.row_rec.add_prefix(Gtk.Image.new_from_icon_name("drive-harddisk-symbolic"))
        self.badge_rec = Gtk.Label(label="...")
        self.row_rec.add_suffix(self.badge_rec)
        btn_fix_rec = Gtk.Button(label="Update")
        btn_fix_rec.set_icon_name("emblem-system-symbolic")
        btn_fix_rec.connect("clicked", lambda b: self.run_async_task(self.core.update_recovery_assistant, "Updating Native Recovery Assistant..."))
        self.row_rec.add_suffix(btn_fix_rec)
        group.add(self.row_rec)

        # 2. Language & Apps
        self.row_lang = Adw.ActionRow(title="2. UI Language and App Naming", subtitle="Checking...")
        self.row_lang.add_prefix(Gtk.Image.new_from_icon_name("preferences-desktop-locale-symbolic"))
        self.badge_lang = Gtk.Label(label="...")
        self.row_lang.add_suffix(self.badge_lang)
        btn_goto_lang = Gtk.Button(label="Configure")
        btn_goto_lang.set_icon_name("go-next-symbolic")
        btn_goto_lang.connect("clicked", lambda b: self.sidebar_list.select_row(self.sidebar_rows["language"]))
        self.row_lang.add_suffix(btn_goto_lang)
        group.add(self.row_lang)

        # 3. Sayri AI
        self.row_sayri = Adw.ActionRow(title="3. Sayri AI Voice Assistant", subtitle="Checking...")
        self.row_sayri.add_prefix(Gtk.Image.new_from_icon_name("audio-input-microphone-symbolic"))
        self.badge_sayri = Gtk.Label(label="...")
        self.row_sayri.add_suffix(self.badge_sayri)
        btn_fix_sayri = Gtk.Button(label="Update")
        btn_fix_sayri.set_icon_name("software-update-available-symbolic")
        btn_fix_sayri.connect("clicked", lambda b: self.run_async_task(self.core.update_or_install_sayri, "Updating Sayri AI Assistant..."))
        self.row_sayri.add_suffix(btn_fix_sayri)
        group.add(self.row_sayri)

        # 4. Hibernation
        self.row_hib = Adw.ActionRow(title="4. Hibernation and Swapfile Subsystem", subtitle="Checking...")
        self.row_hib.add_prefix(Gtk.Image.new_from_icon_name("night-light-symbolic"))
        self.badge_hib = Gtk.Label(label="...")
        self.row_hib.add_suffix(self.badge_hib)
        btn_fix_hib = Gtk.Button(label="Repair")
        btn_fix_hib.set_icon_name("emblem-system-symbolic")
        btn_fix_hib.connect("clicked", lambda b: self.run_async_task(self.core.fix_and_configure_hibernation, "Configuring Hibernation Subsystem..."))
        self.row_hib.add_suffix(btn_fix_hib)
        group.add(self.row_hib)

        # 5. Extras
        self.row_extras = Adw.ActionRow(title="5. System Enhancements and Optimizations", subtitle="Apple Sounds, Spotlight, Keybindings and Schemas")
        self.row_extras.add_prefix(Gtk.Image.new_from_icon_name("preferences-other-symbolic"))
        self.badge_extras = Gtk.Label(label="Ready")
        self.badge_extras.add_css_class("badge-ok")
        self.row_extras.add_suffix(self.badge_extras)
        btn_fix_extras = Gtk.Button(label="Apply")
        btn_fix_extras.set_icon_name("document-save-symbolic")
        btn_fix_extras.connect("clicked", lambda b: self.run_async_task(self.core.apply_all_extras, "Applying system enhancements..."))
        self.row_extras.add_suffix(btn_fix_extras)
        group.add(self.row_extras)

        # Autostart checkbox
        autostart_group = Adw.PreferencesGroup()
        self.chk_dont_show = Gtk.CheckButton(label="Do not open this assistant automatically on login")
        self.chk_dont_show.set_active(os.path.exists(AUTOSTART_FLAG_PATH))
        self.chk_dont_show.connect("toggled", self.on_autostart_toggled)
        autostart_group.add(self.chk_dont_show)
        box.append(autostart_group)

        return scroll

    # -------------------------------------------------------------------------
    # 2. LANGUAGE & APP NAMES PAGE
    # -------------------------------------------------------------------------
    def build_language_page(self) -> Gtk.Widget:
        scroll = Gtk.ScrolledWindow()
        scroll.set_vexpand(True)
        scroll.set_hexpand(True)

        clamp = Adw.Clamp(maximum_size=820, tightening_threshold=640)
        scroll.set_child(clamp)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        box.set_margin_top(24)
        box.set_margin_bottom(24)
        box.set_margin_start(20)
        box.set_margin_end(20)
        clamp.set_child(box)

        lang_group = Adw.PreferencesGroup(
            title="System Language Selection",
            description="Select the primary language for the graphical interface, system locales, and OCR engine."
        )
        box.append(lang_group)

        self.lang_model = Gtk.StringList()
        for l in POPULAR_LANGUAGES:
            self.lang_model.append(f"{l['name']}  ({l['code']})")

        self.row_lang_combo = Adw.ComboRow(title="Primary Language", model=self.lang_model)
        self.row_lang_combo.add_prefix(Gtk.Image.new_from_icon_name("preferences-desktop-locale-symbolic"))
        lang_group.add(self.row_lang_combo)

        app_names_group = Adw.PreferencesGroup(
            title="Application Naming Convention",
            description="Choose between macOS standard clean names or fully translated application titles."
        )
        box.append(app_names_group)

        self.row_app_mode = Adw.ComboRow(
            title="Application Titles Style",
            model=Gtk.StringList.new([
                "macOS Standard Style (Finder, Terminal, Settings)",
                "Localized Names (Files, Terminal, Settings)"
            ])
        )
        self.row_app_mode.add_prefix(Gtk.Image.new_from_icon_name("application-x-executable-symbolic"))
        app_names_group.add(self.row_app_mode)

        self.row_user_dirs = Adw.SwitchRow(
            title="Update XDG user folders",
            subtitle="Renames standard user directories (Downloads, Documents, Desktop) to match chosen language.",
            active=True
        )
        self.row_user_dirs.add_prefix(Gtk.Image.new_from_icon_name("folder-symbolic"))
        app_names_group.add(self.row_user_dirs)

        btn_apply_lang = Gtk.Button(label="Apply Language and UI Settings")
        btn_apply_lang.set_icon_name("document-save-symbolic")
        btn_apply_lang.add_css_class("btn-master-action")
        btn_apply_lang.set_halign(Gtk.Align.CENTER)
        btn_apply_lang.connect("clicked", self.on_apply_language_clicked)
        box.append(btn_apply_lang)

        return scroll

    # -------------------------------------------------------------------------
    # 3. RECOVERY PAGE
    # -------------------------------------------------------------------------
    def build_recovery_page(self) -> Gtk.Widget:
        scroll = Gtk.ScrolledWindow()
        scroll.set_vexpand(True)
        scroll.set_hexpand(True)

        clamp = Adw.Clamp(maximum_size=820, tightening_threshold=640)
        scroll.set_child(clamp)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        box.set_margin_top(24)
        box.set_margin_bottom(24)
        box.set_margin_start(20)
        box.set_margin_end(20)
        clamp.set_child(box)

        group = Adw.PreferencesGroup(
            title="Native Recovery Assistant (Rust)",
            description="The native pulsar-recovery-assistant delivers the macOS-style recovery and reinstallation workflow from the recovery partition."
        )
        box.append(group)

        self.row_rec_detail = Adw.ActionRow(title="Root System Location (/usr/bin/pulsar-recovery-assistant)", subtitle="Checking...")
        self.row_rec_detail.add_prefix(Gtk.Image.new_from_icon_name("application-x-executable-symbolic"))
        group.add(self.row_rec_detail)

        self.row_rec_part = Adw.ActionRow(title="Recovery Partition and SquashFS Environment", subtitle="Scanning...")
        self.row_rec_part.add_prefix(Gtk.Image.new_from_icon_name("drive-harddisk-symbolic"))
        group.add(self.row_rec_part)

        btn_install_rec = Gtk.Button(label="Update and Sync Recovery Partition")
        btn_install_rec.set_icon_name("emblem-system-symbolic")
        btn_install_rec.add_css_class("btn-master-action")
        btn_install_rec.set_halign(Gtk.Align.CENTER)
        btn_install_rec.connect("clicked", lambda b: self.run_async_task(self.core.update_recovery_assistant, "Updating and Syncing Recovery Partition..."))
        box.append(btn_install_rec)

        return scroll

    # -------------------------------------------------------------------------
    # 4. SAYRI AI PAGE
    # -------------------------------------------------------------------------
    def build_sayri_page(self) -> Gtk.Widget:
        scroll = Gtk.ScrolledWindow()
        scroll.set_vexpand(True)
        scroll.set_hexpand(True)

        clamp = Adw.Clamp(maximum_size=820, tightening_threshold=640)
        scroll.set_child(clamp)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        box.set_margin_top(24)
        box.set_margin_bottom(24)
        box.set_margin_start(20)
        box.set_margin_end(20)
        clamp.set_child(box)

        group = Adw.PreferencesGroup(
            title="Sayri AI Voice Assistant",
            description="Sayri is the integrated voice and AI assistant for Pulsar OS with Whisper speech-to-text and Piper text-to-speech."
        )
        box.append(group)

        self.row_sayri_ver = Adw.ActionRow(title="Installed Version", subtitle="Checking...")
        self.row_sayri_ver.add_prefix(Gtk.Image.new_from_icon_name("system-software-update-symbolic"))
        group.add(self.row_sayri_ver)

        self.row_sayri_latest = Adw.ActionRow(title="Latest Repository Version", subtitle="Checking...")
        self.row_sayri_latest.add_prefix(Gtk.Image.new_from_icon_name("software-update-available-symbolic"))
        group.add(self.row_sayri_latest)

        self.row_sayri_audio = Adw.ActionRow(title="Speech Engines (Whisper / Piper)", subtitle="Checking...")
        self.row_sayri_audio.add_prefix(Gtk.Image.new_from_icon_name("audio-volume-high-symbolic"))
        group.add(self.row_sayri_audio)

        btn_sayri = Gtk.Button(label="Update / Install Sayri AI")
        btn_sayri.set_icon_name("software-update-available-symbolic")
        btn_sayri.add_css_class("btn-master-action")
        btn_sayri.set_halign(Gtk.Align.CENTER)
        btn_sayri.connect("clicked", lambda b: self.run_async_task(self.core.update_or_install_sayri, "Updating Sayri AI Assistant..."))
        box.append(btn_sayri)

        return scroll

    # -------------------------------------------------------------------------
    # 5. HIBERNATION PAGE
    # -------------------------------------------------------------------------
    def build_hibernation_page(self) -> Gtk.Widget:
        scroll = Gtk.ScrolledWindow()
        scroll.set_vexpand(True)
        scroll.set_hexpand(True)

        clamp = Adw.Clamp(maximum_size=820, tightening_threshold=640)
        scroll.set_child(clamp)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        box.set_margin_top(24)
        box.set_margin_bottom(24)
        box.set_margin_start(20)
        box.set_margin_end(20)
        clamp.set_child(box)

        group = Adw.PreferencesGroup(
            title="Hibernation Subsystem Diagnostics",
            description="Hibernation requires an active swapfile with sufficient capacity, calculated resume offset, and systemd sleep services."
        )
        box.append(group)

        self.row_hib_swap = Adw.ActionRow(title="Swapfile and Memory Capacity", subtitle="Checking...")
        self.row_hib_swap.add_prefix(Gtk.Image.new_from_icon_name("drive-harddisk-symbolic"))
        group.add(self.row_hib_swap)

        self.row_hib_offset = Adw.ActionRow(title="Resume Offset (resume_offset)", subtitle="Checking...")
        self.row_hib_offset.add_prefix(Gtk.Image.new_from_icon_name("preferences-system-details-symbolic"))
        group.add(self.row_hib_offset)

        self.row_hib_services = Adw.ActionRow(title="Systemd Hibernation Services", subtitle="Checking...")
        self.row_hib_services.add_prefix(Gtk.Image.new_from_icon_name("emblem-system-symbolic"))
        group.add(self.row_hib_services)

        self.row_hib_hooks = Adw.ActionRow(title="Kernel Initramfs Resume Hook", subtitle="Checking...")
        self.row_hib_hooks.add_prefix(Gtk.Image.new_from_icon_name("application-x-executable-symbolic"))
        group.add(self.row_hib_hooks)

        btn_hib = Gtk.Button(label="Configure and Repair Hibernation")
        btn_hib.set_icon_name("emblem-system-symbolic")
        btn_hib.add_css_class("btn-master-action")
        btn_hib.set_halign(Gtk.Align.CENTER)
        btn_hib.connect("clicked", lambda b: self.run_async_task(self.core.fix_and_configure_hibernation, "Configuring Hibernation Subsystem..."))
        box.append(btn_hib)

        return scroll

    # -------------------------------------------------------------------------
    # 6. EXTRAS PAGE
    # -------------------------------------------------------------------------
    def build_extras_page(self) -> Gtk.Widget:
        scroll = Gtk.ScrolledWindow()
        scroll.set_vexpand(True)
        scroll.set_hexpand(True)

        clamp = Adw.Clamp(maximum_size=820, tightening_threshold=640)
        scroll.set_child(clamp)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        box.set_margin_top(24)
        box.set_margin_bottom(24)
        box.set_margin_start(20)
        box.set_margin_end(20)
        clamp.set_child(box)

        group = Adw.PreferencesGroup(
            title="System Enhancements and Visual Polish",
            description="Additional features and settings to complete the Pulsar OS user experience."
        )
        box.append(group)

        row_sound = Adw.ActionRow(title="Pear / Apple Sound Theme", subtitle="Enables macOS-inspired interface sound effects")
        row_sound.add_prefix(Gtk.Image.new_from_icon_name("audio-volume-high-symbolic"))
        group.add(row_sound)

        row_boot = Adw.ActionRow(title="Startup Bootsound Chime", subtitle="Plays the classic startup chime on boot")
        row_boot.add_prefix(Gtk.Image.new_from_icon_name("audio-speakers-symbolic"))
        group.add(row_boot)

        row_spot = Adw.ActionRow(title="Universal Spotlight Search", subtitle="Configures Super+Space shortcut and dock integration")
        row_spot.add_prefix(Gtk.Image.new_from_icon_name("system-search-symbolic"))
        group.add(row_spot)

        row_remap = Adw.ActionRow(title="macOS Keyboard Shortcuts Remap", subtitle="Enables Cmd+C, Cmd+V, Cmd+Q and macOS navigation in Wayland")
        row_remap.add_prefix(Gtk.Image.new_from_icon_name("input-keyboard-symbolic"))
        group.add(row_remap)

        row_schemas = Adw.ActionRow(title="Recompile GSettings Schemas", subtitle="Ensures all system schemas and application metadata are up to date")
        row_schemas.add_prefix(Gtk.Image.new_from_icon_name("preferences-other-symbolic"))
        group.add(row_schemas)

        btn_extras = Gtk.Button(label="Apply All Enhancements")
        btn_extras.set_icon_name("document-save-symbolic")
        btn_extras.add_css_class("btn-master-action")
        btn_extras.set_halign(Gtk.Align.CENTER)
        btn_extras.connect("clicked", lambda b: self.run_async_task(self.core.apply_all_extras, "Applying system enhancements..."))
        box.append(btn_extras)

        return scroll

    # -------------------------------------------------------------------------
    # 7. LOGS PAGE
    # -------------------------------------------------------------------------
    def build_logs_page(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_margin_top(18)
        box.set_margin_bottom(18)
        box.set_margin_start(18)
        box.set_margin_end(18)
        box.set_vexpand(True)
        box.set_hexpand(True)

        header_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        lbl_log_title = Gtk.Label(label="Operations and Diagnostics Log", halign=Gtk.Align.START)
        lbl_log_title.add_css_class("pulsar-header-title")
        lbl_log_title.set_hexpand(True)
        header_box.append(lbl_log_title)

        btn_clear = Gtk.Button(label="Clear Log")
        btn_clear.set_icon_name("edit-clear-symbolic")
        btn_clear.connect("clicked", lambda b: self.log_text_view.get_buffer().set_text(""))
        header_box.append(btn_clear)
        box.append(header_box)

        scroll = Gtk.ScrolledWindow()
        scroll.set_vexpand(True)
        scroll.set_hexpand(True)

        self.log_text_view = Gtk.TextView()
        self.log_text_view.set_editable(False)
        self.log_text_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self.log_text_view.add_css_class("log-view-box")
        scroll.set_child(self.log_text_view)
        box.append(scroll)

        return box

    # -------------------------------------------------------------------------
    # STATUS REFRESH & ASYNC TASK RUNNER
    # -------------------------------------------------------------------------
    def refresh_all_status(self):
        def worker():
            self.core.log("[Scan] Scanning Pulsar OS components and configurations...")
            pkg_stat = self.core.check_packages_status()
            rec_stat = self.core.check_recovery_assistant()
            loc_info = self.core.get_current_locale_info()
            sayri_stat = self.core.check_sayri_status()
            hib_stat = self.core.check_hibernation_status()
            extras_stat = self.core.check_system_extras()

            GLib.idle_add(self._update_ui_status, pkg_stat, rec_stat, loc_info, sayri_stat, hib_stat, extras_stat)

        threading.Thread(target=worker, daemon=True).start()

    def _update_ui_status(self, pkg_stat, rec_stat, loc_info, sayri_stat, hib_stat, extras_stat):
        # 0. Packages
        if pkg_stat.get("has_updates"):
            self.badge_pkgs.set_text(f"{pkg_stat.get('upgradable_count', 1)} Updates")
            self.badge_pkgs.set_css_classes(["badge-warn"])
            self.row_pkgs.set_subtitle(f"Updates available in repository: {', '.join(pkg_stat.get('upgradable_packages', [])[:3])}")
        else:
            self.badge_pkgs.set_text("Up to Date")
            self.badge_pkgs.set_css_classes(["badge-ok"])
            self.row_pkgs.set_subtitle("All official Pulsar OS core packages are up to date")

        # 1. Recovery
        if rec_stat["installed_root"]:
            if rec_stat.get("partition_is_outdated"):
                self.badge_rec.set_text("Partition Outdated")
                self.badge_rec.set_css_classes(["badge-warn"])
                self.row_rec.set_subtitle(f"Recovery partition build is outdated: {rec_stat.get('partition_version_info')}")
                self.row_rec_detail.set_subtitle("Root system has latest binary, but recovery partition squashfs requires update.")
                self.row_rec_part.set_subtitle(f"Outdated recovery squashfs detected on {', '.join(rec_stat.get('recovery_devices', []))}")
            else:
                self.badge_rec.set_text("Up to Date")
                self.badge_rec.set_css_classes(["badge-ok"])
                self.row_rec.set_subtitle("Native assistant installed and synchronized with recovery partition")
                self.row_rec_detail.set_subtitle("Installed in /usr/bin/pulsar-recovery-assistant")
                self.row_rec_part.set_subtitle(f"Synchronized recovery partition: {', '.join(rec_stat.get('recovery_devices', []))}")
        else:
            self.badge_rec.set_text("Pending")
            self.badge_rec.set_css_classes(["badge-warn"])
            self.row_rec.set_subtitle("Native assistant missing in /usr/bin")
            self.row_rec_detail.set_subtitle("Not found in /usr/bin")
            self.row_rec_part.set_subtitle("Recovery partition pending synchronization")

        # 2. Locale
        curr_loc = loc_info.get("current_locale", "en_US")
        self.badge_lang.set_text(curr_loc)
        self.badge_lang.set_css_classes(["badge-ok"])
        self.row_lang.set_subtitle(f"Active locale: {curr_loc} | App naming: {loc_info.get('app_names_mode')}")
        
        for idx, lang_item in enumerate(POPULAR_LANGUAGES):
            if lang_item["code"].startswith(curr_loc) or curr_loc.startswith(lang_item["code"].split("_")[0]):
                self.row_lang_combo.set_selected(idx)
                break

        # 3. Sayri
        if sayri_stat["installed"]:
            if sayri_stat["is_latest"]:
                self.badge_sayri.set_text(f"v{sayri_stat['installed_version']}")
                self.badge_sayri.set_css_classes(["badge-ok"])
                self.row_sayri.set_subtitle("Sayri AI is up to date")
            else:
                self.badge_sayri.set_text("Update Available")
                self.badge_sayri.set_css_classes(["badge-warn"])
                self.row_sayri.set_subtitle(f"Installed: v{sayri_stat['installed_version']} | Repository: v{sayri_stat['latest_version']}")
        else:
            self.badge_sayri.set_text("Not Installed")
            self.badge_sayri.set_css_classes(["badge-error"])
            self.row_sayri.set_subtitle("Sayri AI voice assistant is not installed")

        self.row_sayri_ver.set_subtitle(sayri_stat["installed_version"])
        self.row_sayri_latest.set_subtitle(sayri_stat["latest_version"])
        self.row_sayri_audio.set_subtitle("Voice models ready" if sayri_stat["has_stt_tts"] else "Whisper / Piper models pending")

        # 4. Hibernation
        if hib_stat["swap_active"] and hib_stat["resume_offset_valid"] and hib_stat["services_enabled"]:
            self.badge_hib.set_text("Active")
            self.badge_hib.set_css_classes(["badge-ok"])
            self.row_hib.set_subtitle(f"Swap active ({hib_stat['swap_size_mb']}MB), offset: {hib_stat['resume_offset_val']}")
        else:
            self.badge_hib.set_text("Needs Setup")
            self.badge_hib.set_css_classes(["badge-warn"])
            self.row_hib.set_subtitle("Swapfile or resume parameters pending configuration")

        self.row_hib_swap.set_subtitle(f"Swap: {hib_stat['swap_size_mb']} MB (RAM: {hib_stat['ram_size_mb']} MB)")
        self.row_hib_offset.set_subtitle(f"Offset: {hib_stat['resume_offset_val']}" if hib_stat['resume_offset_valid'] else "Offset not configured")
        self.row_hib_services.set_subtitle("Systemd services enabled" if hib_stat['services_enabled'] else "Services pending enablement")
        self.row_hib_hooks.set_subtitle("Initramfs resume hook configured" if hib_stat['kernel_hook_ok'] else "Hook pending in initramfs")

    def run_async_task(self, task_fn, title: str):
        if self.is_running_task:
            self.show_toast("A task is already running. Please wait.")
            return
        self.is_running_task = True
        self.btn_top_update.set_sensitive(False)
        self.show_toast(f"Starting: {title}")

        # Switch to logs view so user sees live output immediately
        if hasattr(self, 'sidebar_list') and 'logs' in self.sidebar_rows:
            self.sidebar_list.select_row(self.sidebar_rows["logs"])

        def worker():
            try:
                self.core.log(f"==================================================")
                self.core.log(f"[Task] Starting: {title}")
                self.core.log(f"==================================================")
                result = task_fn()
                if result is False:
                    self.core.log(f"[Task] Finished with warnings: {title}")
                    GLib.idle_add(lambda: self.show_toast(f"Task finished with warnings: {title}"))
                else:
                    self.core.log(f"[Task] Successfully completed: {title}")
                    GLib.idle_add(lambda: self.show_toast(f"Completed: {title}"))
            except Exception as e:
                self.core.log(f"[Error] Task encountered an error: {e}")
                GLib.idle_add(lambda: self.show_toast(f"Error: {e}"))
            finally:
                self.is_running_task = False
                GLib.idle_add(lambda: self.btn_top_update.set_sensitive(True))
                GLib.idle_add(self.refresh_all_status)

        threading.Thread(target=worker, daemon=True).start()

    def on_apply_language_clicked(self, button):
        selected_idx = self.row_lang_combo.get_selected()
        selected_lang = POPULAR_LANGUAGES[selected_idx]["code"] if selected_idx < len(POPULAR_LANGUAGES) else "en_US"
        app_mode = "standard" if self.row_app_mode.get_selected() == 0 else "localized"
        update_dirs = self.row_user_dirs.get_active()

        self.run_async_task(
            lambda: self.core.configure_locale_and_apps(selected_lang, app_names_mode=app_mode, update_user_dirs=update_dirs),
            f"Configuring system language to {selected_lang}..."
        )

    def on_master_update_clicked(self, button):
        selected_idx = self.row_lang_combo.get_selected()
        selected_lang = POPULAR_LANGUAGES[selected_idx]["code"] if selected_idx < len(POPULAR_LANGUAGES) else "en_US"
        app_mode = "standard" if self.row_app_mode.get_selected() == 0 else "localized"

        self.run_async_task(
            lambda: self.core.run_full_migration(target_locale=selected_lang, app_names_mode=app_mode),
            "Full Pulsar OS System Update..."
        )

    def on_autostart_toggled(self, button):
        if button.get_active():
            os.makedirs(os.path.dirname(AUTOSTART_FLAG_PATH), exist_ok=True)
            with open(AUTOSTART_FLAG_PATH, "w") as f:
                f.write("Pulsar OS update wizard completed.\n")
        else:
            if os.path.exists(AUTOSTART_FLAG_PATH):
                try:
                    os.remove(AUTOSTART_FLAG_PATH)
                except Exception:
                    pass


def main():
    app = PulsarUpdateApp()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
