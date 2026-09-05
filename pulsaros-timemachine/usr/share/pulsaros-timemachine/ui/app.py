#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Time Machine - GTK4 & Libadwaita Application
macOS Sonoma / Tahoe Styled Backup and Recovery Suite with Lucide Icons, In-Button Spinners,
Live Process Tracking, Cancellation, Pause/Resume, and Manual Snapshot Deletion.
"""

import sys
import os
import threading
import datetime
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
gi.require_version('GdkPixbuf', '2.0')

from gi.repository import Gtk, Gdk, GLib, Adw, Gio, GdkPixbuf, Pango

# Add parent path to resolve core modules
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from core.config_manager import ConfigManager
from core.engine import TimeMachineEngine
from core.storage_manager import StorageManager
from core.scheduler import SchedulerManager
from ui.lucide import create_lucide_icon, get_lucide_svg_path

CSS_DATA = """
window, .root-container {
    background-color: #1e1e20;
    color: #ffffff;
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
.apple-card {
    background-color: #2a2a2e;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 14px;
    padding: 18px 22px;
    margin-bottom: 12px;
}
.hero-title {
    font-size: 24px;
    font-weight: 700;
    color: #ffffff;
}
.hero-subtitle {
    font-size: 13px;
    color: #a1a1a6;
}
.current-file-text {
    font-size: 11px;
    color: #0a84ff;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
}
.suggested-action {
    background-color: #0071e3;
    color: #ffffff;
    border-radius: 8px;
    font-weight: 600;
    padding: 10px 24px;
    border: none;
    font-size: 13px;
}
.suggested-action:hover {
    background-color: #007bf5;
}
.suggested-action:disabled {
    background-color: #38383a;
    color: #636366;
}
.secondary-action {
    background-color: #323236;
    color: #ffffff;
    border-radius: 8px;
    font-weight: 600;
    padding: 8px 18px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    font-size: 12px;
}
.secondary-action:hover {
    background-color: #3e3e42;
}
.destructive-action {
    background-color: #3a1c1a;
    color: #ff453a;
    border-radius: 8px;
    font-weight: 600;
    padding: 8px 18px;
    border: 1px solid rgba(255, 69, 58, 0.3);
    font-size: 12px;
}
.destructive-action:hover {
    background-color: #4a201e;
}
.badge-active {
    background-color: #1b4328;
    color: #30d158;
    border-radius: 6px;
    padding: 3px 10px;
    font-size: 11px;
    font-weight: bold;
}
.badge-inactive {
    background-color: #3a3a3c;
    color: #8e8e93;
    border-radius: 6px;
    padding: 3px 10px;
    font-size: 11px;
    font-weight: bold;
}
.badge-error {
    background-color: #491815;
    color: #ff453a;
    border-radius: 6px;
    padding: 3px 10px;
    font-size: 11px;
    font-weight: bold;
}
.log-view {
    background-color: #121212;
    border: 1px solid #333333;
    border-radius: 10px;
    padding: 10px;
}
.log-view text {
    background-color: #121212;
    color: #30d158;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 11px;
}
.progress-bar-thin trough {
    min-height: 8px;
    border-radius: 9999px;
    background-color: #3a3a3c;
    border: none;
}
.progress-bar-thin progress {
    min-height: 8px;
    border-radius: 9999px;
    background-color: #0071e3;
    border: none;
}
.button-busy {
    opacity: 1.0;
}
.button-busy-box {
    opacity: 1.0;
}
.button-busy-box spinner {
    color: #ffffff;
    min-width: 18px;
    min-height: 18px;
    opacity: 1.0;
}
.button-busy-box label {
    color: #ffffff;
    font-weight: 600;
    opacity: 1.0;
}
"""

class TimeMachineWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Pulsar OS Time Machine")
        self.set_default_size(880, 660)

        self.config_mgr = ConfigManager()
        self.engine = TimeMachineEngine(self.config_mgr)
        self.is_backup_running = False

        self.apply_css()
        self.build_ui()
        self.refresh_state()

    def apply_css(self):
        provider = Gtk.CssProvider()
        provider.load_from_data(CSS_DATA.encode('utf-8'))
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def set_button_busy(self, button: Gtk.Button, busy: bool, busy_text: str = "", idle_text: str = ""):
        """Replaces button text with a high-contrast animated spinner and status label."""
        if busy:
            button.set_can_target(False)
            button.add_css_class("button-busy")
            box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            box.add_css_class("button-busy-box")
            box.set_halign(Gtk.Align.CENTER)
            box.set_valign(Gtk.Align.CENTER)
            spinner = Gtk.Spinner()
            spinner.set_spinning(True)
            spinner.set_size_request(18, 18)
            box.append(spinner)
            if busy_text:
                lbl = Gtk.Label(label=busy_text)
                lbl.add_css_class("button-busy-box")
                box.append(lbl)
            button.set_child(box)
        else:
            button.remove_css_class("button-busy")
            button.set_can_target(True)
            button.set_sensitive(True)
            button.set_child(None)
            if idle_text:
                button.set_label(idle_text)

    def show_toast(self, message: str):
        """Displays an in-app non-intrusive toast notification."""
        def _show():
            toast = Adw.Toast.new(message)
            toast.set_timeout(3)
            self.toast_overlay.add_toast(toast)
        GLib.idle_add(_show)

    def build_ui(self):
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        main_box.add_css_class("root-container")
        self.set_content(main_box)

        # Header Bar
        header = Adw.HeaderBar()
        title = Adw.WindowTitle(title="Time Machine", subtitle="Pulsar OS Protection")
        header.set_title_widget(title)
        main_box.append(header)

        # Toast Overlay Container
        self.toast_overlay = Adw.ToastOverlay()
        self.toast_overlay.set_hexpand(True)
        self.toast_overlay.set_vexpand(True)

        # View Stack & Switcher
        self.stack = Adw.ViewStack()
        switcher_bar = Adw.ViewSwitcher(stack=self.stack)
        switcher_bar.set_policy(Adw.ViewSwitcherPolicy.WIDE)
        header.set_title_widget(switcher_bar)

        # 1. Overview Page
        self.page_overview = self.create_overview_page()
        self.stack.add_titled(self.page_overview, "overview", "Overview")
        self.stack.get_page(self.page_overview).set_icon_name("document-revert-symbolic")

        # 2. Settings & Destinations Page
        self.page_settings = self.create_settings_page()
        self.stack.add_titled(self.page_settings, "settings", "Settings & Storage")
        self.stack.get_page(self.page_settings).set_icon_name("emblem-system-symbolic")

        # 3. Restore & Manage Snapshots Page
        self.page_restore = self.create_restore_page()
        self.stack.add_titled(self.page_restore, "restore", "Snapshots & Restore")
        self.stack.get_page(self.page_restore).set_icon_name("edit-undo-symbolic")

        # 4. Activity Log Page
        self.page_logs = self.create_logs_page()
        self.stack.add_titled(self.page_logs, "logs", "Activity Logs")
        self.stack.get_page(self.page_logs).set_icon_name("utilities-terminal-symbolic")

        self.toast_overlay.set_child(self.stack)
        main_box.append(self.toast_overlay)

    # ─────────────────────────────────────────────────────────────
    # Page 1: Overview
    # ─────────────────────────────────────────────────────────────
    def create_overview_page(self):
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_hexpand(True)
        scrolled.set_vexpand(True)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        box.set_margin_top(20)
        box.set_margin_bottom(20)
        box.set_margin_start(40)
        box.set_margin_end(40)

        # Hero Banner
        hero_card = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=20)
        hero_card.add_css_class("apple-card")

        icon_path = "/usr/share/pulsaros-timemachine/assets/timemachine.png"
        if not os.path.exists(icon_path):
            icon_path = os.path.join(os.path.dirname(__file__), "../assets/timemachine.png")

        img = Gtk.Image()
        if os.path.exists(icon_path):
            img.set_from_file(icon_path)
            img.set_pixel_size(72)
        else:
            img = create_lucide_icon("history", size=64)
        hero_card.append(img)

        vbox_hero = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        vbox_hero.set_valign(Gtk.Align.CENTER)
        vbox_hero.set_hexpand(True)

        lbl_title = Gtk.Label(label="Pulsar OS Time Machine")
        lbl_title.add_css_class("hero-title")
        lbl_title.set_halign(Gtk.Align.START)
        vbox_hero.append(lbl_title)

        lbl_sub = Gtk.Label(label="Btrfs Copy-On-Write snapshots synced securely with Restic.")
        lbl_sub.add_css_class("hero-subtitle")
        lbl_sub.set_halign(Gtk.Align.START)
        vbox_hero.append(lbl_sub)

        hero_card.append(vbox_hero)
        box.append(hero_card)

        # Status Summary Card
        status_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        status_card.add_css_class("apple-card")

        # Row 1: Active Badge, Toggle Pause/Resume, & Destination
        row1 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        lbl_status_head = Gtk.Label(label="Automatic Protection:")
        lbl_status_head.set_halign(Gtk.Align.START)
        row1.append(lbl_status_head)

        self.badge_status = Gtk.Label(label="ACTIVE")
        self.badge_status.add_css_class("badge-active")
        row1.append(self.badge_status)

        self.btn_toggle_protection = Gtk.Button(label="Pause")
        self.btn_toggle_protection.add_css_class("secondary-action")
        self.btn_toggle_protection.connect("clicked", self.on_toggle_protection_clicked)
        row1.append(self.btn_toggle_protection)

        spacer1 = Gtk.Box()
        spacer1.set_hexpand(True)
        row1.append(spacer1)

        self.lbl_dest_type = Gtk.Label(label="Destination: USB Storage")
        self.lbl_dest_type.add_css_class("hero-subtitle")
        row1.append(self.lbl_dest_type)
        status_card.append(row1)

        # Row 2: Last Backup Info
        row2 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.lbl_last_backup = Gtk.Label(label="Last Backup: None")
        self.lbl_last_backup.set_halign(Gtk.Align.START)
        row2.append(self.lbl_last_backup)
        status_card.append(row2)

        # Row 3: Next Backup Info
        row3 = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.lbl_next_backup = Gtk.Label(label="Next Backup: Daily at 03:00")
        self.lbl_next_backup.set_halign(Gtk.Align.START)
        row3.append(self.lbl_next_backup)
        status_card.append(row3)

        # ─── Live Progress Section (Visible during backup) ───
        self.progress_container = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.progress_container.set_margin_top(6)
        self.progress_container.set_visible(False)

        # Progress bar
        self.progress_bar = Gtk.ProgressBar()
        self.progress_bar.add_css_class("progress-bar-thin")
        self.progress_container.append(self.progress_bar)

        # Detailed metrics row: Bytes done + Files done + ETA
        self.row_metrics = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        self.lbl_progress_bytes = Gtk.Label(label="0 MB / 0 MB (0%)")
        self.lbl_progress_bytes.add_css_class("hero-subtitle")
        self.row_metrics.append(self.lbl_progress_bytes)

        self.lbl_progress_files = Gtk.Label(label="Files: 0 / 0")
        self.lbl_progress_files.add_css_class("hero-subtitle")
        self.row_metrics.append(self.lbl_progress_files)

        self.lbl_progress_eta = Gtk.Label(label="ETA: Calculating...")
        self.lbl_progress_eta.add_css_class("hero-subtitle")
        self.row_metrics.append(self.lbl_progress_eta)

        self.progress_container.append(self.row_metrics)

        # Current file path label
        self.lbl_current_file = Gtk.Label(label="Scanning files...")
        self.lbl_current_file.add_css_class("current-file-text")
        self.lbl_current_file.set_halign(Gtk.Align.START)
        self.lbl_current_file.set_ellipsize(Pango.EllipsizeMode.MIDDLE)
        self.progress_container.append(self.lbl_current_file)

        status_card.append(self.progress_container)

        # Action Buttons
        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        btn_box.set_margin_top(10)

        self.btn_backup_now = Gtk.Button(label="Back Up Now")
        self.btn_backup_now.add_css_class("suggested-action")
        self.btn_backup_now.connect("clicked", self.on_backup_now_clicked)
        btn_box.append(self.btn_backup_now)

        self.btn_cancel_backup = Gtk.Button(label="Stop / Cancel Backup")
        self.btn_cancel_backup.add_css_class("destructive-action")
        self.btn_cancel_backup.set_visible(False)
        self.btn_cancel_backup.connect("clicked", self.on_cancel_backup_clicked)
        btn_box.append(self.btn_cancel_backup)

        self.btn_refresh = Gtk.Button(label="Refresh Status")
        self.btn_refresh.add_css_class("secondary-action")
        self.btn_refresh.connect("clicked", lambda b: self.refresh_state())
        btn_box.append(self.btn_refresh)

        status_card.append(btn_box)
        box.append(status_card)

        scrolled.set_child(box)
        return scrolled

    # ─────────────────────────────────────────────────────────────
    # Page 2: Settings & Storage
    # ─────────────────────────────────────────────────────────────
    def create_settings_page(self):
        scrolled = Gtk.ScrolledWindow()
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        box.set_margin_top(20)
        box.set_margin_bottom(20)
        box.set_margin_start(40)
        box.set_margin_end(40)

        # Card 1: Source Selection & Exclusions
        src_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        src_card.add_css_class("apple-card")
        lbl_src_title = Gtk.Label(label="1. Backup Scope & Exclusions")
        lbl_src_title.add_css_class("hero-subtitle")
        lbl_src_title.set_halign(Gtk.Align.START)
        src_card.append(lbl_src_title)

        self.combo_source = Gtk.DropDown.new_from_strings([
            "Both Root (@) and User Home (@home) [Recommended]",
            "Only Root System (@)",
            "Only User Data (@home)",
            "Custom Directory / Folder (Selective & Fast Test)"
        ])
        self.combo_source.connect("notify::selected", self.on_source_selected_changed)
        src_card.append(self.combo_source)

        # Custom Folder Row
        self.custom_folder_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.custom_folder_box.set_visible(False)
        self.entry_custom_path = Adw.EntryRow(title="Folder Path")
        self.entry_custom_path.set_hexpand(True)
        self.custom_folder_box.append(self.entry_custom_path)

        self.btn_browse_folder = Gtk.Button(label="Browse...")
        self.btn_browse_folder.add_css_class("secondary-action")
        self.btn_browse_folder.connect("clicked", self.on_browse_folder_clicked)
        self.custom_folder_box.append(self.btn_browse_folder)
        src_card.append(self.custom_folder_box)

        # Exclusions Section
        self.entry_excludes = Adw.EntryRow(title="Excluded Patterns (comma-separated)")
        self.entry_excludes.set_text("**/.cache/*, **/node_modules/*, **/tmp/*, **/.local/share/Trash/*, **/ISO/build/*")
        src_card.append(self.entry_excludes)

        lbl_exc_hint = Gtk.Label(label="Exclusion patterns prevent backing up bulky temp or cache files (e.g. **/.cache/*, **/Downloads/*).")
        lbl_exc_hint.set_halign(Gtk.Align.START)
        lbl_exc_hint.set_wrap(True)
        lbl_exc_hint.add_css_class("metric-sub")
        src_card.append(lbl_exc_hint)

        box.append(src_card)

        # Card 2: Destination Type
        dest_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        dest_card.add_css_class("apple-card")
        lbl_dest_title = Gtk.Label(label="2. Backup Storage Destination")
        lbl_dest_title.add_css_class("hero-subtitle")
        lbl_dest_title.set_halign(Gtk.Align.START)
        dest_card.append(lbl_dest_title)

        self.dest_stack = Gtk.Stack()
        self.dest_switcher = Gtk.StackSwitcher(stack=self.dest_stack)
        dest_card.append(self.dest_switcher)

        # Subview 2A: USB
        usb_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        usb_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.combo_usb = Gtk.DropDown.new_from_strings(["Scanning for USB disks..."])
        self.combo_usb.set_hexpand(True)
        self.combo_usb.connect("notify::selected", self.on_usb_selected_changed)
        usb_row.append(self.combo_usb)

        self.btn_scan_usb = Gtk.Button(label="Rescan Drives")
        self.btn_scan_usb.add_css_class("secondary-action")
        self.btn_scan_usb.connect("clicked", lambda b: self.on_rescan_usb_clicked())
        usb_row.append(self.btn_scan_usb)
        usb_box.append(usb_row)

        # Unformatted / Format Needed Banner & Action
        self.usb_format_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.usb_format_box.set_visible(False)
        self.usb_format_box.add_css_class("snapshot-card")

        banner_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        icon_warn = create_lucide_icon("hard-drive", size=22)
        banner_row.append(icon_warn)

        self.lbl_usb_warn = Gtk.Label(label="Selected drive has no valid filesystem or partition table. Format it to use with Time Machine.")
        self.lbl_usb_warn.set_wrap(True)
        self.lbl_usb_warn.set_halign(Gtk.Align.START)
        self.lbl_usb_warn.set_hexpand(True)
        self.lbl_usb_warn.add_css_class("metric-sub")
        banner_row.append(self.lbl_usb_warn)
        self.usb_format_box.append(banner_row)

        self.btn_format_usb = Gtk.Button(label="Format USB Drive (ext4 / TIMEMACHINE)")
        self.btn_format_usb.add_css_class("destructive-action")
        self.btn_format_usb.connect("clicked", self.on_format_usb_clicked)
        self.usb_format_box.append(self.btn_format_usb)
        usb_box.append(self.usb_format_box)

        self.entry_usb_subpath = Adw.EntryRow(title="Folder in USB")
        self.entry_usb_subpath.set_text("pulsaros-timemachine-backup")
        usb_box.append(self.entry_usb_subpath)
        self.dest_stack.add_titled(usb_box, "usb", "USB / Local Disk")

        # Subview 2B: Samba
        smb_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.entry_smb_host = Adw.EntryRow(title="Server IP / Hostname (e.g. 127.0.0.1)")
        self.entry_smb_share = Adw.EntryRow(title="Share Name (e.g. timemachine_share)")
        self.entry_smb_user = Adw.EntryRow(title="Username (Optional)")
        self.entry_smb_pass = Adw.PasswordEntryRow(title="Password (Optional)")
        self.entry_smb_domain = Adw.EntryRow(title="Domain / Workgroup (Optional)")
        smb_box.append(self.entry_smb_host)
        smb_box.append(self.entry_smb_share)
        smb_box.append(self.entry_smb_user)
        smb_box.append(self.entry_smb_pass)
        smb_box.append(self.entry_smb_domain)

        self.btn_test_smb = Gtk.Button(label="Test Samba Connection")
        self.btn_test_smb.add_css_class("secondary-action")
        self.btn_test_smb.connect("clicked", self.on_test_samba_clicked)
        smb_box.append(self.btn_test_smb)
        self.dest_stack.add_titled(smb_box, "samba", "Samba / Remote NAS")

        # Subview 2C: Rclone
        rclone_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        rc_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.combo_rclone = Gtk.DropDown.new_from_strings(["Scanning rclone remotes..."])
        self.combo_rclone.set_hexpand(True)
        rc_row.append(self.combo_rclone)

        self.btn_scan_rc = Gtk.Button(label="Refresh Remotes")
        self.btn_scan_rc.add_css_class("secondary-action")
        self.btn_scan_rc.connect("clicked", lambda b: self.on_rescan_rclone_clicked())
        rc_row.append(self.btn_scan_rc)
        rclone_box.append(rc_row)

        self.entry_rclone_path = Adw.EntryRow(title="Cloud Directory Path")
        self.entry_rclone_path.set_text("pulsaros-timemachine-backup")
        rclone_box.append(self.entry_rclone_path)

        rc_btn_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.btn_import_rc = Gtk.Button(label="Import rclone.conf...")
        self.btn_import_rc.add_css_class("secondary-action")
        self.btn_import_rc.connect("clicked", self.on_import_rclone_clicked)
        rc_btn_row.append(self.btn_import_rc)

        self.btn_test_rc = Gtk.Button(label="Test Cloud Remote")
        self.btn_test_rc.add_css_class("secondary-action")
        self.btn_test_rc.connect("clicked", self.on_test_rclone_clicked)
        rc_btn_row.append(self.btn_test_rc)
        rclone_box.append(rc_btn_row)

        self.dest_stack.add_titled(rclone_box, "rclone", "Cloud Storage (Rclone)")
        dest_card.append(self.dest_stack)
        box.append(dest_card)

        # Card 3: Security & Encryption
        sec_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        sec_card.add_css_class("apple-card")
        lbl_sec = Gtk.Label(label="3. Repository Security & Encryption")
        lbl_sec.add_css_class("hero-subtitle")
        lbl_sec.set_halign(Gtk.Align.START)
        sec_card.append(lbl_sec)

        self.entry_repo_pass = Adw.PasswordEntryRow(title="Encryption Password")
        sec_card.append(self.entry_repo_pass)
        box.append(sec_card)

        # Card 4: Scheduling & Retention Policy
        sched_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        sched_card.add_css_class("apple-card")
        lbl_sched = Gtk.Label(label="4. Schedule & Retention")
        lbl_sched.add_css_class("hero-subtitle")
        lbl_sched.set_halign(Gtk.Align.START)
        sched_card.append(lbl_sched)

        row_freq = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        lbl_freq = Gtk.Label(label="Frequency:")
        row_freq.append(lbl_freq)
        self.combo_freq = Gtk.DropDown.new_from_strings([
            "Hourly (Every hour)",
            "Daily (Every night at 03:00)",
            "Weekly (Every Sunday)",
            "Monthly (1st of month)",
            "Manual Only (No automated schedule)"
        ])
        self.combo_freq.set_hexpand(True)
        row_freq.append(self.combo_freq)
        sched_card.append(row_freq)

        row_ret = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        lbl_ret = Gtk.Label(label="Number of snapshots to keep:")
        row_ret.append(lbl_ret)
        self.spin_retention = Gtk.SpinButton.new_with_range(1, 100, 1)
        self.spin_retention.set_value(10)
        row_ret.append(self.spin_retention)
        sched_card.append(row_ret)
        box.append(sched_card)

        # Save Button
        self.btn_save = Gtk.Button(label="Save Configuration")
        self.btn_save.add_css_class("suggested-action")
        self.btn_save.connect("clicked", self.on_save_config_clicked)
        box.append(self.btn_save)

        scrolled.set_child(box)
        return scrolled

    # ─────────────────────────────────────────────────────────────
    # Page 3: Live Restore & Manage Snapshots
    # ─────────────────────────────────────────────────────────────
    def create_restore_page(self):
        scrolled = Gtk.ScrolledWindow()
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        box.set_margin_top(20)
        box.set_margin_bottom(20)
        box.set_margin_start(40)
        box.set_margin_end(40)

        top_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        top_card.add_css_class("apple-card")
        lbl_t = Gtk.Label(label="Manage Snapshots & Restore")
        lbl_t.add_css_class("hero-title")
        lbl_t.set_halign(Gtk.Align.START)
        top_card.append(lbl_t)

        lbl_d = Gtk.Label(label="Browse, delete, or restore individual files and system subvolumes from your repository.")
        lbl_d.add_css_class("hero-subtitle")
        lbl_d.set_halign(Gtk.Align.START)
        top_card.append(lbl_d)
        box.append(top_card)

        # Snapshots list card
        list_card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        list_card.add_css_class("apple-card")

        head_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        lbl_snaps = Gtk.Label(label="Available Snapshots in Destination:")
        lbl_snaps.add_css_class("hero-subtitle")
        lbl_snaps.set_halign(Gtk.Align.START)
        head_row.append(lbl_snaps)

        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        head_row.append(spacer)

        self.btn_load_snaps = Gtk.Button(label="Fetch Snapshots")
        self.btn_load_snaps.add_css_class("secondary-action")
        self.btn_load_snaps.connect("clicked", lambda b: self.load_snapshots_async())
        head_row.append(self.btn_load_snaps)
        list_card.append(head_row)

        self.combo_snapshots = Gtk.DropDown.new_from_strings(["Click 'Fetch Snapshots' to scan repository..."])
        list_card.append(self.combo_snapshots)

        restore_path_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.entry_restore_path = Adw.EntryRow(title="Extract to Destination Directory")
        self.entry_restore_path.set_text("/tmp/pulsar-restore")
        self.entry_restore_path.set_hexpand(True)
        restore_path_box.append(self.entry_restore_path)

        self.btn_browse_restore = Gtk.Button(label="Browse...")
        self.btn_browse_restore.add_css_class("secondary-action")
        self.btn_browse_restore.connect("clicked", self.on_browse_restore_clicked)
        restore_path_box.append(self.btn_browse_restore)
        list_card.append(restore_path_box)

        # Quick Subvolume Target Helper
        self.btn_detect_targets = Gtk.Button(label="Detect System Subvolumes (@ / @home)")
        self.btn_detect_targets.add_css_class("secondary-action")
        self.btn_detect_targets.connect("clicked", self.on_detect_targets_clicked)
        list_card.append(self.btn_detect_targets)

        self.entry_restore_filter = Adw.EntryRow(title="Specific file/path to extract (optional)")
        list_card.append(self.entry_restore_filter)

        # Action buttons row: Restore + Delete manually
        actions_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        actions_row.set_margin_top(8)

        self.btn_start_restore = Gtk.Button(label="Restore Selected Snapshot")
        self.btn_start_restore.add_css_class("suggested-action")
        self.btn_start_restore.connect("clicked", self.on_start_restore_clicked)
        actions_row.append(self.btn_start_restore)

        self.btn_delete_snap = Gtk.Button(label="Delete Snapshot Permanently")
        self.btn_delete_snap.add_css_class("destructive-action")
        self.btn_delete_snap.connect("clicked", self.on_delete_snapshot_clicked)
        actions_row.append(self.btn_delete_snap)

        list_card.append(actions_row)
        box.append(list_card)

        scrolled.set_child(box)
        return scrolled

    # ─────────────────────────────────────────────────────────────
    # Page 4: Activity Logs
    # ─────────────────────────────────────────────────────────────
    def create_logs_page(self):
        scrolled = Gtk.ScrolledWindow()
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        box.set_margin_top(14)
        box.set_margin_bottom(14)
        box.set_margin_start(20)
        box.set_margin_end(20)

        header_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        lbl_log = Gtk.Label(label="Time Machine Execution Log")
        lbl_log.add_css_class("hero-subtitle")
        lbl_log.set_halign(Gtk.Align.START)
        header_box.append(lbl_log)

        sp = Gtk.Box()
        sp.set_hexpand(True)
        header_box.append(sp)

        btn_clear = Gtk.Button(label="Clear")
        btn_clear.add_css_class("secondary-action")
        btn_clear.connect("clicked", lambda b: self.text_buffer.set_text(""))
        header_box.append(btn_clear)
        box.append(header_box)

        log_scroll = Gtk.ScrolledWindow()
        log_scroll.set_vexpand(True)
        log_scroll.add_css_class("log-view")
        log_scroll.set_size_request(-1, 350)

        self.log_view = Gtk.TextView()
        self.log_view.set_editable(False)
        self.log_view.set_monospace(True)
        self.text_buffer = self.log_view.get_buffer()
        log_scroll.set_child(self.log_view)
        box.append(log_scroll)

        scrolled.set_child(box)
        return scrolled

    def log(self, message: str):
        def _append():
            ts = datetime.datetime.now().strftime("%H:%M:%S")
            end_iter = self.text_buffer.get_end_iter()
            self.text_buffer.insert(end_iter, f"[{ts}] {message}\n")
        GLib.idle_add(_append)

    # ─────────────────────────────────────────────────────────────
    # Logic & State Synchronization
    # ─────────────────────────────────────────────────────────────
    def refresh_state(self):
        cfg = self.config_mgr.load()

        # Update Overview
        dtype = cfg.get("destination_type", "usb").upper()
        self.lbl_dest_type.set_text(f"Destination: {dtype}")

        last_time = cfg.get("last_backup_time") or "Never"
        last_status = cfg.get("last_backup_status") or "never"
        last_size = cfg.get("last_backup_size") or ""
        self.lbl_last_backup.set_text(f"Last Backup: {last_time} ({last_status.upper()}) {last_size}")

        freq = cfg.get("schedule_frequency", "daily")
        self.lbl_next_backup.set_text(f"Next Backup: {SchedulerManager.get_next_run_time()} ({freq})")

        is_active = SchedulerManager.is_timer_active() and freq != "manual"
        if is_active:
            self.badge_status.set_text("AUTO PROTECTION ON")
            self.badge_status.remove_css_class("badge-inactive")
            self.badge_status.add_css_class("badge-active")
            self.btn_toggle_protection.set_label("Pause Auto-Backups")
        else:
            self.badge_status.set_text("PAUSED / MANUAL")
            self.badge_status.remove_css_class("badge-active")
            self.badge_status.add_css_class("badge-inactive")
            self.btn_toggle_protection.set_label("Resume Auto-Backups")

        # Load Settings fields
        src_map = {"both": 0, "root": 1, "home": 2, "custom": 3}
        src_val = cfg.get("source", "both")
        self.combo_source.set_selected(src_map.get(src_val, 0))
        self.entry_custom_path.set_text(cfg.get("custom_path", ""))
        self.custom_folder_box.set_visible(src_val == "custom")

        exc_list = cfg.get("exclude_patterns", [])
        if isinstance(exc_list, list):
            self.entry_excludes.set_text(", ".join(exc_list))
        else:
            self.entry_excludes.set_text(str(exc_list))

        self.dest_stack.set_visible_child_name(cfg.get("destination_type", "usb"))
        self.entry_usb_subpath.set_text(cfg.get("usb_repo_subpath", "pulsaros-timemachine-backup"))
        self.entry_smb_host.set_text(cfg.get("samba_host", ""))
        self.entry_smb_share.set_text(cfg.get("samba_share", ""))
        self.entry_smb_user.set_text(cfg.get("samba_user", ""))
        self.entry_smb_pass.set_text(cfg.get("samba_pass", ""))
        self.entry_smb_domain.set_text(cfg.get("samba_domain", ""))
        self.entry_rclone_path.set_text(cfg.get("rclone_path", "pulsaros-timemachine-backup"))
        self.entry_repo_pass.set_text(cfg.get("repo_password", ""))
        self.spin_retention.set_value(cfg.get("retention_count", 10))

        freq_map = {"hourly": 0, "daily": 1, "weekly": 2, "monthly": 3, "manual": 4}
        self.combo_freq.set_selected(freq_map.get(freq, 1))

        self.refresh_usb_list()
        self.refresh_rclone_list()

    def on_toggle_protection_clicked(self, btn):
        is_active = SchedulerManager.is_timer_active()
        if is_active:
            SchedulerManager.disable()
            self.log("Automatic scheduled backups paused.")
        else:
            freq = self.config_mgr.get("schedule_frequency", "daily")
            if freq == "manual":
                freq = "daily"
                self.config_mgr.set("schedule_frequency", "daily")
                self.config_mgr.save()
            SchedulerManager.enable(freq)
            self.log(f"Automatic scheduled backups resumed ({freq}).")
        self.refresh_state()

    def on_rescan_usb_clicked(self):
        self.set_button_busy(self.btn_scan_usb, True, "Scanning...")
        GLib.timeout_add(400, lambda: [self.refresh_usb_list(), self.set_button_busy(self.btn_scan_usb, False, idle_text="Rescan Drives")])

    def on_source_selected_changed(self, dropdown, param=None):
        idx = dropdown.get_selected()
        self.custom_folder_box.set_visible(idx == 3)

    def on_browse_folder_clicked(self, btn):
        dialog = Gtk.FileChooserNative.new(
            "Select Folder to Back Up",
            self,
            Gtk.FileChooserAction.SELECT_FOLDER,
            "Select Folder",
            "Cancel"
        )
        def _response(d, resp):
            if resp == Gtk.ResponseType.ACCEPT:
                f = d.get_file()
                if f:
                    self.entry_custom_path.set_text(f.get_path())
        dialog.connect("response", _response)
        dialog.show()

    def on_usb_selected_changed(self, dropdown, param=None):
        idx = dropdown.get_selected()
        if hasattr(self, "usb_devices") and 0 <= idx < len(self.usb_devices):
            dev = self.usb_devices[idx]
            if dev.get("needs_formatting"):
                self.usb_format_box.set_visible(True)
                self.lbl_usb_warn.set_text(
                    f"Drive {dev.get('path')} ({dev.get('size')}) has no filesystem. "
                    "Format as ext4 (TIMEMACHINE) to use it with Time Machine."
                )
            else:
                self.usb_format_box.set_visible(False)
        else:
            self.usb_format_box.set_visible(False)

    def on_format_usb_clicked(self, btn):
        idx = self.combo_usb.get_selected()
        if not hasattr(self, "usb_devices") or not (0 <= idx < len(self.usb_devices)):
            self.show_dialog("Format Disk", "No USB drive selected.")
            return

        dev = self.usb_devices[idx]
        dev_path = dev.get("path")
        model = dev.get("model") or "USB Drive"

        dialog = Adw.MessageDialog.new(
            self,
            f"Format {model} ({dev.get('size')})?",
            f"WARNING: All existing files on {dev_path} will be permanently destroyed.\n\n"
            "A clean GPT partition table and ext4 filesystem labeled 'TIMEMACHINE' will be created."
        )
        dialog.add_response("cancel", "Cancel")
        dialog.add_response("format", "Format Disk")
        dialog.set_response_appearance("format", Adw.ResponseAppearance.DESTRUCTIVE)

        def _on_response(d, resp):
            if resp == "format":
                self.set_button_busy(self.btn_format_usb, True, "Formatting Disk...")
                self.log(f"Starting format of {dev_path} (ext4 / TIMEMACHINE)...")
                
                def _bg():
                    ok, msg = StorageManager.format_usb_disk(dev_path, fs_type="ext4", label="TIMEMACHINE")
                    def _ui():
                        self.set_button_busy(self.btn_format_usb, False, idle_text="Format USB Drive (ext4 / TIMEMACHINE)")
                        self.log(msg)
                        self.refresh_usb_list()
                        self.show_toast(msg)
                    GLib.idle_add(_ui)

                threading.Thread(target=_bg, daemon=True).start()

        dialog.connect("response", _on_response)
        dialog.present()

    def refresh_usb_list(self):
        self.usb_devices = StorageManager.get_available_usb_disks()
        if not self.usb_devices:
            self.combo_usb.set_model(Gtk.StringList.new(["No external USB drives detected"]))
            self.usb_format_box.set_visible(False)
        else:
            labels = [d["display_name"] for d in self.usb_devices]
            self.combo_usb.set_model(Gtk.StringList.new(labels))
            self.on_usb_selected_changed(self.combo_usb)

    def refresh_rclone_list(self):
        self.rclone_remotes = StorageManager.get_rclone_remotes()
        if not self.rclone_remotes:
            self.combo_rclone.set_model(Gtk.StringList.new(["No Rclone remotes configured (Import rclone.conf)"]))
        else:
            labels = [r["display_name"] for r in self.rclone_remotes]
            self.combo_rclone.set_model(Gtk.StringList.new(labels))

    def on_backup_now_clicked(self, btn):
        # Ensure active USB target is recorded if using USB
        if self.config_mgr.get("destination_type", "usb") == "usb":
            if not self.config_mgr.get("usb_uuid") and hasattr(self, "usb_devices") and self.usb_devices:
                idx = self.combo_usb.get_selected()
                if 0 <= idx < len(self.usb_devices):
                    sel = self.usb_devices[idx]
                    self.config_mgr.set("usb_uuid", sel.get("uuid"))
                    self.config_mgr.set("usb_label", sel.get("label", ""))
                    self.config_mgr.save()

        self.is_backup_running = True
        self.set_button_busy(self.btn_backup_now, True, "Realizando copia...")
        self.btn_cancel_backup.set_visible(True)
        self.progress_container.set_visible(True)
        self.progress_bar.set_fraction(0.0)
        self.lbl_progress_bytes.set_text("Iniciando copia de seguridad...")
        self.lbl_progress_files.set_text("Archivos: 0 / 0")
        self.lbl_progress_eta.set_text("Tiempo restante: Calculando...")
        self.lbl_current_file.set_text("Inicializando instantáneas y repositorio...")

        def _worker():
            self.log("Copia de seguridad Time Machine iniciada manualmente.")
            
            def _prog(event):
                percent = event.get("percent_done", 0.0)
                bytes_done = event.get("bytes_done", 0) / (1024 * 1024)
                total_bytes = event.get("total_bytes", 0) / (1024 * 1024)
                files_done = event.get("files_done", 0)
                total_files = event.get("total_files", 0)
                curr_files = event.get("current_files", [])
                sec_rem = event.get("seconds_remaining")

                def _ui_update():
                    self.progress_bar.set_fraction(percent)
                    pct_str = f"{int(percent * 100)}%"
                    if total_bytes > 0:
                        self.lbl_progress_bytes.set_text(f"{bytes_done:.1f} MB / {total_bytes:.1f} MB ({pct_str})")
                    else:
                        self.lbl_progress_bytes.set_text(f"{bytes_done:.1f} MB procesados ({pct_str})")

                    self.lbl_progress_files.set_text(f"Archivos: {files_done:,} / {total_files:,}")

                    if sec_rem is not None and sec_rem > 0:
                        hours, rem = divmod(sec_rem, 3600)
                        mins, secs = divmod(rem, 60)
                        if hours > 0:
                            eta_str = f"Tiempo restante: {hours} h {mins} min"
                        elif mins > 0:
                            eta_str = f"Tiempo restante: {mins} min {secs} s"
                        else:
                            eta_str = f"Tiempo restante: {secs} s"
                        self.lbl_progress_eta.set_text(eta_str)
                    else:
                        self.lbl_progress_eta.set_text("Tiempo restante: Calculando...")

                    if curr_files:
                        curr_name = os.path.basename(curr_files[0]) if len(curr_files[0]) > 45 else curr_files[0]
                        self.lbl_current_file.set_text(f"Procesando: {curr_name}")

                GLib.idle_add(_ui_update)

            ok, msg = self.engine.perform_backup(
                progress_cb=_prog,
                log_cb=self.log
            )

            def _done():
                self.is_backup_running = False
                self.set_button_busy(self.btn_backup_now, False, idle_text="Back Up Now")
                self.btn_cancel_backup.set_visible(False)
                self.progress_container.set_visible(False)
                self.refresh_state()
                self.show_dialog("Time Machine Backup", msg)

            GLib.idle_add(_done)

        threading.Thread(target=_worker, daemon=True).start()

    def on_cancel_backup_clicked(self, btn):
        self.log("Cancellation requested by user. Terminating backup process...")
        self.lbl_current_file.set_text("Cancelling backup...")
        self.set_button_busy(self.btn_cancel_backup, True, "Cancelling...")
        def _cancel_worker():
            self.engine.cancel_active_backup()
            GLib.idle_add(lambda: self.set_button_busy(self.btn_cancel_backup, False, idle_text="Stop / Cancel Backup"))
        threading.Thread(target=_cancel_worker, daemon=True).start()

    def on_save_config_clicked(self, btn):
        src_opts = ["both", "root", "home", "custom"]
        source = src_opts[self.combo_source.get_selected()]
        custom_path = self.entry_custom_path.get_text().strip()
        excludes_str = self.entry_excludes.get_text().strip()
        exclude_patterns = [p.strip() for p in excludes_str.split(",") if p.strip()]
        dest_type = self.dest_stack.get_visible_child_name()

        # USB
        usb_uuid = ""
        usb_label = ""
        if hasattr(self, "usb_devices") and self.usb_devices and self.combo_usb.get_selected() < len(self.usb_devices):
            selected_usb = self.usb_devices[self.combo_usb.get_selected()]
            usb_uuid = selected_usb["uuid"]
            usb_label = selected_usb["label"]

        # Rclone
        rclone_remote = ""
        if hasattr(self, "rclone_remotes") and self.rclone_remotes and self.combo_rclone.get_selected() < len(self.rclone_remotes):
            rclone_remote = self.rclone_remotes[self.combo_rclone.get_selected()]["name"]

        freq_opts = ["hourly", "daily", "weekly", "monthly", "manual"]
        frequency = freq_opts[self.combo_freq.get_selected()]

        self.config_mgr.update({
            "source": source,
            "custom_path": custom_path,
            "exclude_patterns": exclude_patterns,
            "destination_type": dest_type,
            "usb_uuid": usb_uuid,
            "usb_label": usb_label,
            "usb_repo_subpath": self.entry_usb_subpath.get_text(),
            "samba_host": self.entry_smb_host.get_text(),
            "samba_share": self.entry_smb_share.get_text(),
            "samba_user": self.entry_smb_user.get_text(),
            "samba_pass": self.entry_smb_pass.get_text(),
            "samba_domain": self.entry_smb_domain.get_text(),
            "rclone_remote": rclone_remote,
            "rclone_path": self.entry_rclone_path.get_text(),
            "repo_password": self.entry_repo_pass.get_text(),
            "schedule_frequency": frequency,
            "retention_count": int(self.spin_retention.get_value())
        })

        SchedulerManager.enable(frequency)
        self.refresh_state()
        self.show_dialog("Settings Saved", "Time Machine configuration and backup schedule have been updated.")

    def on_test_samba_clicked(self, btn):
        host = self.entry_smb_host.get_text()
        share = self.entry_smb_share.get_text()
        user = self.entry_smb_user.get_text()
        passw = self.entry_smb_pass.get_text()
        domain = self.entry_smb_domain.get_text()
        self.log(f"Testing connection to Samba share //{host}/{share}...")
        self.set_button_busy(self.btn_test_smb, True, "Testing Connection...")

        def _worker():
            ok, msg = StorageManager.test_samba_connection(host, share, user, passw, domain)
            def _done():
                self.set_button_busy(self.btn_test_smb, False, idle_text="Test Samba Connection")
                self.show_dialog("Samba Test", msg)
            GLib.idle_add(_done)
            self.log(f"Samba test result: {msg}")

        threading.Thread(target=_worker, daemon=True).start()

    def on_test_rclone_clicked(self, btn):
        if not hasattr(self, "rclone_remotes") or not self.rclone_remotes:
            self.show_dialog("Rclone", "No rclone remote selected.")
            return
        remote = self.rclone_remotes[self.combo_rclone.get_selected()]["name"]
        self.log(f"Testing Rclone cloud remote '{remote}'...")
        self.set_button_busy(self.btn_test_rc, True, "Testing Remote...")

        def _worker():
            ok, msg = StorageManager.test_rclone_remote(remote)
            def _done():
                self.set_button_busy(self.btn_test_rc, False, idle_text="Test Cloud Remote")
                self.show_dialog("Cloud Remote Test", msg)
            GLib.idle_add(_done)
            self.log(f"Rclone test result: {msg}")

        threading.Thread(target=_worker, daemon=True).start()

    def on_import_rclone_clicked(self, btn):
        dialog = Gtk.FileDialog()
        dialog.set_title("Select rclone.conf file")
        dialog.open(self, None, self.on_rclone_file_selected)

    def on_rclone_file_selected(self, dialog, result):
        try:
            file = dialog.open_finish(result)
            if file:
                path = file.get_path()
                ok, msg = StorageManager.import_rclone_config(path)
                self.show_dialog("Import rclone.conf", msg)
                self.refresh_rclone_list()
        except Exception as e:
            self.show_dialog("Error", str(e))

    def load_snapshots_async(self):
        self.log("Fetching snapshots from repository...")
        self.set_button_busy(self.btn_load_snaps, True, "Fetching...")

        def _worker():
            snaps = self.engine.get_snapshots_list()
            self.cached_snapshots = snaps
            def _update():
                self.set_button_busy(self.btn_load_snaps, False, idle_text="Fetch Snapshots")
                if not snaps:
                    self.combo_snapshots.set_model(Gtk.StringList.new(["No snapshots found in repository"]))
                else:
                    labels = []
                    for s in snaps:
                        sid = s.get("short_id", s.get("id", "")[:8])
                        stime = s.get("time", "")[:19].replace("T", " ")
                        stags = ", ".join(s.get("tags", []))
                        labels.append(f"{stime} - [{sid}] ({stags})")
                    self.combo_snapshots.set_model(Gtk.StringList.new(labels))
                self.log(f"Fetched {len(snaps)} snapshots.")
            GLib.idle_add(_update)
        threading.Thread(target=_worker, daemon=True).start()

    def on_browse_restore_clicked(self, btn):
        dialog = Gtk.FileChooserNative.new(
            "Select Restore Destination Directory",
            self,
            Gtk.FileChooserAction.SELECT_FOLDER,
            "Select Destination",
            "Cancel"
        )
        def _response(d, resp):
            if resp == Gtk.ResponseType.ACCEPT:
                f = d.get_file()
                if f:
                    self.entry_restore_path.set_text(f.get_path())
        dialog.connect("response", _response)
        dialog.show()

    def on_detect_targets_clicked(self, btn):
        targets = BtrfsManager.discover_restore_targets()
        if not targets:
            self.show_dialog("Subvolume Detection", "No Btrfs system subvolumes detected.")
            return

        lines = ["Detected system Btrfs targets for recovery:"]
        for label, path in targets.items():
            lines.append(f"• {label}:\n  {path}")
        
        # Set primary target
        if "Recovery Target: System Root (@)" in targets:
            self.entry_restore_path.set_text(targets["Recovery Target: System Root (@)"])
        elif "Running System Root (/)" in targets:
            self.entry_restore_path.set_text(targets["Running System Root (/)"])

        self.show_dialog("Detected Subvolumes", "\n\n".join(lines))

    def on_start_restore_clicked(self, btn):
        if not hasattr(self, "cached_snapshots") or not self.cached_snapshots:
            self.show_dialog("Restore", "Please fetch and select a snapshot first.")
            return

        idx = self.combo_snapshots.get_selected()
        if idx >= len(self.cached_snapshots):
            return

        snap = self.cached_snapshots[idx]
        snap_id = snap.get("id")
        target_dir = self.entry_restore_path.get_text()
        include_path = self.entry_restore_filter.get_text() or None

        self.log(f"Starting restore of snapshot {snap_id[:8]} to {target_dir}...")
        self.set_button_busy(self.btn_start_restore, True, "Restoring...")

        def _worker():
            ok, msg = self.engine.perform_restore(
                snapshot_id=snap_id,
                target_dir=target_dir,
                include_path=include_path,
                log_cb=self.log
            )
            def _done():
                self.set_button_busy(self.btn_start_restore, False, idle_text="Restore Selected Snapshot")
                self.show_dialog("Restore Result", msg)
            GLib.idle_add(_done)

        threading.Thread(target=_worker, daemon=True).start()

    def on_delete_snapshot_clicked(self, btn):
        if not hasattr(self, "cached_snapshots") or not self.cached_snapshots:
            self.show_dialog("Delete Snapshot", "Please fetch and select a snapshot first.")
            return

        idx = self.combo_snapshots.get_selected()
        if idx >= len(self.cached_snapshots):
            return

        snap = self.cached_snapshots[idx]
        snap_id = snap.get("id")
        short_id = snap.get("short_id", snap_id[:8])

        confirm_dialog = Adw.MessageDialog(
            transient_for=self,
            heading="Delete Snapshot Permanently?",
            body=f"Are you sure you want to permanently delete snapshot {short_id}?\n\nThis will prune the repository and free storage space. This action cannot be undone."
        )
        confirm_dialog.add_response("cancel", "Cancel")
        confirm_dialog.add_response("delete", "Delete")
        confirm_dialog.set_response_appearance("delete", Adw.ResponseAppearance.DESTRUCTIVE)

        def _on_confirm(d, resp):
            d.destroy()
            if resp == "delete":
                self.log(f"Deleting snapshot {short_id} from repository...")
                self.set_button_busy(self.btn_delete_snap, True, "Deleting...")
                def _del_worker():
                    ok, msg = self.engine.delete_backup(snap_id, log_cb=self.log)
                    def _del_done():
                        self.set_button_busy(self.btn_delete_snap, False, idle_text="Delete Snapshot Permanently")
                        self.show_dialog("Delete Snapshot", msg)
                        self.load_snapshots_async()
                    GLib.idle_add(_del_done)
                threading.Thread(target=_del_worker, daemon=True).start()

        confirm_dialog.connect("response", _on_confirm)
        confirm_dialog.present()

    def show_dialog(self, title: str, message: str):
        dlg = Adw.MessageDialog(
            transient_for=self,
            heading=title,
            body=message
        )
        dlg.add_response("ok", "OK")
        dlg.present()

def main_gui():
    app = Adw.Application(application_id="org.pulsaros.TimeMachine", flags=Gio.ApplicationFlags.NON_UNIQUE)

    def on_activate(app):
        win = TimeMachineWindow(app)
        win.present()

    app.connect("activate", on_activate)
    return app.run([])

if __name__ == "__main__":
    sys.exit(main_gui())
