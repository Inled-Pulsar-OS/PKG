#!/usr/bin/env python3
# ==============================================================================
# Pulsar OS - Recovery and Installation Selector UI (Apple-Style Setup Assistant)
# ==============================================================================
# English: Python script that manages the macOS-style Recovery and Setup Assistant.
#          Delegates all partitioning, formatting, extraction and bootloader decisions
#          to Calamares by running it headlessly and parsing its progress output from
#          the Calamares session log. Removes the fake macOS top menu bar.
# Español: Script en Python que gestiona la interfaz de recuperación y asistente de configuración
#          estilo macOS. Delega todas las decisiones de particionado, formateo, extracción y
#          gestor de arranque en Calamares ejecutándolo de forma oculta y analizando su progreso
#          desde el log de sesión. Elimina la barra superior falsa de macOS.
# ==============================================================================

import json
import os
import re
import subprocess
import sys
import threading
import time

import gi

# Require specific GTK, Gdk and GdkPixbuf versions
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
gi.require_version("GdkPixbuf", "2.0")
from gi.repository import Gdk, GdkPixbuf, GLib, Gtk

CSS_DATA = """
window {
    background-color: #1c1c1e;
}

list, row {
    background-color: transparent;
    border: none;
    box-shadow: none;
}

/* Caja del Recovery Utilities / Recovery Utilities Box */
.recovery-box {
    background-color: #2c2c2e;
    border: 1px solid #3a3a3c;
    border-radius: 12px;
    padding: 24px;
    box-shadow: 0 10px 40px rgba(0,0,0,0.6);
}

.recovery-row {
    padding: 12px 16px;
    border-radius: 8px;
    margin-bottom: 4px;
    background-color: transparent;
    transition: all 0.15s ease;
}

.recovery-row:hover {
    background-color: #3a3a3c;
}

.recovery-row:selected {
    background-color: #0071e3;
    color: #ffffff;
}

.recovery-title {
    font-size: 13px;
    font-weight: 700;
    color: #ffffff;
}

.recovery-desc {
    font-size: 11px;
    color: #8e8e93;
}

.recovery-row:selected .recovery-desc {
    color: #d1d1d6;
}

/* Caja del Instalador / Installer Assistant Box */
.installer-box {
    background-color: #2c2c2e;
    border: 1px solid #3a3a3c;
    border-radius: 16px;
    padding: 40px 50px;
    box-shadow: 0 10px 40px rgba(0,0,0,0.6);
}

.installer-title {
    font-size: 26px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 8px;
}

.installer-desc {
    font-size: 13px;
    color: #8e8e93;
    margin-bottom: 24px;
}

/* Tarjeta de disco / Disk Card */
.disk-card {
    background-color: #3a3a3c;
    border: 1px solid #48484a;
    border-radius: 12px;
    padding: 16px;
    min-width: 140px;
    margin: 0 8px;
    transition: all 0.15s ease;
}

.disk-card:hover {
    background-color: #444446;
    border-color: #545456;
}

.disk-card.selected {
    background-color: #444446;
    border-color: #0071e3;
    box-shadow: 0 0 0 2px #0071e3;
}

.disk-name {
    font-size: 12px;
    font-weight: 700;
    color: #ffffff;
    margin-top: 8px;
}

.disk-info {
    font-size: 10px;
    color: #aeaeb2;
}

/* ListBox Scrollable Areas */
.scroll-list {
    background-color: #1c1c1e;
    border: 1px solid #3a3a3c;
    border-radius: 8px;
}

.list-item-row {
    padding: 10px 14px;
    border-bottom: 1px solid #2c2c2e;
}

.list-item-row:hover {
    background-color: #2c2c2e;
}

.list-item-row:selected {
    background-color: #0071e3;
}

.list-item-label {
    font-size: 13px;
    color: #ffffff;
}

/* Forms & Entries */
label.form-label {
    font-size: 13px;
    font-weight: 600;
    color: #e5e5e7;
}

entry {
    background-color: #1c1c1e;
    border: 1px solid #3a3a3c;
    border-radius: 6px;
    color: #ffffff;
    padding: 6px 10px;
}

entry:focus {
    border-color: #0071e3;
    box-shadow: 0 0 0 1px #0071e3;
}

/* Blue tinted symbolic icons helper */
.symbolic-blue {
    color: #0071e3;
}

/* Botones / Buttons */
button {
    font-size: 13px;
    font-weight: 500;
    padding: 6px 18px;
    border-radius: 6px;
    outline: none;
    transition: all 0.15s ease;
}

button.action-btn {
    background-image: none;
    background-color: #3a3a3c;
    border: 1px solid #48484a;
    color: #ffffff;
}

button.action-btn:hover {
    background-color: #444446;
}

button.btn-continue {
    background-image: none;
    background-color: #0071e3;
    border: none;
    color: #ffffff;
    font-weight: 700;
}

button.btn-continue:hover {
    background-color: #0077ed;
}

button.btn-continue:disabled {
    background-color: #3a3a3c;
    color: #8e8e93;
}

progressbar trough, progressbar progress {
    min-height: 8px;
    border-radius: 4px;
}

progressbar progress {
    background-color: #0071e3;
    background-image: none;
}
"""

COUNTRIES = [
    ("Spain (España)", "Europe/Madrid", "es"),
    ("United States", "America/New_York", "us"),
    ("United Kingdom", "Europe/London", "gb"),
    ("France (France)", "Europe/Paris", "fr"),
    ("Germany (Deutschland)", "Europe/Berlin", "de"),
    ("Italy (Italia)", "Europe/Rome", "it"),
    ("Portugal (Portugal)", "Europe/Lisbon", "pt"),
    ("Mexico (México)", "America/Mexico_City", "es"),
    ("Argentina (Argentina)", "America/Argentina/Buenos_Aires", "es"),
    ("Brazil (Brasil)", "America/Sao_Paulo", "br"),
]

KEYBOARDS = [
    ("Spanish (es)", "es"),
    ("English (US - us)", "us"),
    ("English (UK - gb)", "gb"),
    ("French (fr)", "fr"),
    ("German (de)", "de"),
    ("Italian (it)", "it"),
    ("Portuguese (pt)", "pt"),
]


def get_physical_disks():
    """Queries block devices using lsblk."""
    try:
        output = subprocess.check_output(
            "lsblk -J -d -o NAME,MODEL,SIZE,TYPE,RO", shell=True, text=True
        )
        data = json.loads(output)
        disks = []
        for dev in data.get("blockdevices", []):
            ro_val = dev.get("ro")
            is_ro = ro_val in (True, 1, "1", "true", "True")
            if dev.get("type") == "disk" and not is_ro:
                name = dev.get("name")
                model = (dev.get("model") or "").strip() or "Local Drive"
                size = dev.get("size", "0 GB")
                disks.append(
                    {"path": f"/dev/{name}", "name": name, "model": model, "size": size}
                )
        return disks
    except Exception as e:
        print("Error fetching physical disks:", e)
    return []


def run_as_real_user(cmd, wait=False):
    """Runs a graphical command as the host user instead of root."""
    if os.geteuid() == 0:
        real_uid = os.environ.get("PKEXEC_UID") or os.environ.get("SUDO_UID")
        if real_uid:
            try:
                import pwd
                username = pwd.getpwuid(int(real_uid)).pw_name
                display = os.environ.get("DISPLAY", "")
                xauth = os.environ.get("XAUTHORITY", "")
                wayland = os.environ.get("WAYLAND_DISPLAY", "")
                xdg_runtime = os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{real_uid}")

                env_str = f'DISPLAY="{display}"'
                if xauth:
                    env_str += f' XAUTHORITY="{xauth}"'
                if wayland:
                    env_str += f' WAYLAND_DISPLAY="{wayland}"'
                if xdg_runtime:
                    env_str += f' XDG_RUNTIME_DIR="{xdg_runtime}"'

                full_cmd = f"sudo -u {username} env {env_str} {cmd}"
                if wait:
                    return subprocess.run(full_cmd, shell=True)
                return subprocess.Popen(full_cmd, shell=True)
            except Exception as e:
                print("Failed to run command as user, executing normal fallback:", e)
    if wait:
        return subprocess.run(cmd, shell=True)
    return subprocess.Popen(cmd, shell=True)


def restore_calamares_configs():
    """Restores settings.conf and partition.conf back to their backup states."""
    settings_path = "/etc/calamares/settings.conf"
    bak_path = "/etc/calamares/settings.conf.bak"
    if os.path.exists(bak_path):
        try:
            import shutil
            shutil.copy(bak_path, settings_path)
            print("Restored settings.conf from backup.")
        except Exception as e:
            print(f"Failed to restore settings.conf backup: {e}")
            
    partition_conf = "/etc/calamares/modules/partition.conf"
    partition_bak = "/etc/calamares/modules/partition.conf.bak"
    if os.path.exists(partition_bak):
        try:
            import shutil
            shutil.copy(partition_bak, partition_conf)
            print("Restored partition.conf from backup.")
        except Exception as e:
            print(f"Failed to restore partition.conf backup: {e}")


class RecoveryApp(Gtk.Window):
    def __init__(self):
        super().__init__(type=Gtk.WindowType.TOPLEVEL)
        self.set_title("Pulsar OS Recovery")
        self.fullscreen()

        # State Variables
        self.selected_utility = None
        self.selected_disk_path = None
        self.selected_country = COUNTRIES[0]
        self.selected_keyboard = KEYBOARDS[0]

        # Load Custom GTK CSS
        style_provider = Gtk.CssProvider()
        style_provider.load_from_data(CSS_DATA.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        )

        # Center Align Container (Removed macOS fake top bar)
        center_align = Gtk.Alignment.new(0.5, 0.5, 0, 0)
        self.add(center_align)

        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self.stack.set_transition_duration(300)
        center_align.add(self.stack)

        self.init_slides()

        self.connect("destroy", self.on_destroy)
        self.show_all()

    def on_destroy(self, widget):
        restore_calamares_configs()
        Gtk.main_quit()

    def init_slides(self):
        # ----------------------------------------------------------------------
        # Slide 0: Recovery Utilities Box
        # ----------------------------------------------------------------------
        slide_0 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        slide_0.get_style_context().add_class("recovery-box")
        slide_0.set_size_request(540, 420)
        self.stack.add_named(slide_0, "recovery")

        header_lbl = Gtk.Label(label="Pulsar OS Utilities")
        header_lbl.get_style_context().add_class("recovery-title")
        header_lbl.set_margin_bottom(12)
        slide_0.pack_start(header_lbl, False, False, 0)

        self.util_list = Gtk.ListBox()
        self.util_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.util_list.connect("row-selected", self.on_utility_row_selected)
        self.util_list.connect("row-activated", self.on_utility_row_activated)
        slide_0.pack_start(self.util_list, True, True, 8)

        self.is_live = os.path.exists("/usr/local/bin/launch-calamares") or os.path.exists("/usr/bin/calamares")

        self.add_utility_row("Restore from Backup", "Restore your Pulsar OS installation from a local system backup.", "timemachine")
        if self.is_live:
            self.add_utility_row("Install Pulsar OS", "Install a new copy of the Pulsar OS desktop on your computer.", "logo")
        self.add_utility_row("Seafari Browser", "Browse the web to search for online support and configuration guides.", "safari")
        self.add_utility_row("Disk Utility", "Partition, format, or check your connected storage drives.", "gnome-disk-utility")

        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        btn_box.set_margin_top(16)
        slide_0.pack_end(btn_box, False, False, 0)

        self.btn_try_system = Gtk.Button(label="Try System" if self.is_live else "Close")
        self.btn_try_system.get_style_context().add_class("action-btn")
        self.btn_try_system.connect("clicked", lambda b: self.close())
        btn_box.pack_start(self.btn_try_system, False, False, 0)

        self.btn_util_continue = Gtk.Button(label="Continue")
        self.btn_util_continue.get_style_context().add_class("btn-continue")
        self.btn_util_continue.set_sensitive(False)
        self.btn_util_continue.connect("clicked", self.on_utility_continue_clicked)
        btn_box.pack_end(self.btn_util_continue, False, False, 0)

        # ----------------------------------------------------------------------
        # Slide 1: Welcome Installer Screen
        # ----------------------------------------------------------------------
        slide_1 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        slide_1.get_style_context().add_class("installer-box")
        slide_1.set_size_request(680, 520)
        self.stack.add_named(slide_1, "welcome")

        logo_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        logo_box.set_halign(Gtk.Align.CENTER)
        logo_box.set_margin_top(30)
        logo_box.set_margin_bottom(12)
        slide_1.pack_start(logo_box, False, False, 0)

        logo_path = "/usr/share/pulsaros-recovery/pulsar-logo.png"
        if not os.path.exists(logo_path):
            logo_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "pulsar-logo.png")

        if os.path.exists(logo_path):
            pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 150, 150, True)
            logo_img = Gtk.Image.new_from_pixbuf(pixbuf)
        else:
            logo_img = Gtk.Image.new_from_icon_name("computer-symbolic", Gtk.IconSize.DIALOG)
            logo_img.get_style_context().add_class("symbolic-blue")
            logo_img.set_pixel_size(150)
        logo_box.pack_start(logo_img, True, True, 0)

        lbl_welcome_title = Gtk.Label(label="Pulsar OS")
        lbl_welcome_title.get_style_context().add_class("installer-title")
        slide_1.pack_start(lbl_welcome_title, False, False, 0)

        lbl_welcome_desc = Gtk.Label(label="To set up the installation of Pulsar OS, click Continue.")
        lbl_welcome_desc.get_style_context().add_class("installer-desc")
        slide_1.pack_start(lbl_welcome_desc, False, False, 0)

        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        nav_box.set_margin_top(20)
        nav_box.set_halign(Gtk.Align.CENTER)
        slide_1.pack_end(nav_box, False, False, 0)

        btn_welcome_back = Gtk.Button(label="Back")
        btn_welcome_back.get_style_context().add_class("action-btn")
        btn_welcome_back.connect("clicked", lambda b: self.stack.set_visible_child_name("recovery"))
        nav_box.pack_start(btn_welcome_back, False, False, 0)

        btn_welcome_continue = Gtk.Button(label="Continue")
        btn_welcome_continue.get_style_context().add_class("btn-continue")
        btn_welcome_continue.connect("clicked", lambda b: self.stack.set_visible_child_name("country"))
        nav_box.pack_start(btn_welcome_continue, False, False, 0)

        # ----------------------------------------------------------------------
        # Slide 2: Country Selection (Select Your Country or Region)
        # ----------------------------------------------------------------------
        slide_2 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_2.get_style_context().add_class("installer-box")
        slide_2.set_size_request(680, 520)
        self.stack.add_named(slide_2, "country")

        globe_img = Gtk.Image.new_from_icon_name("preferences-desktop-locale-symbolic", Gtk.IconSize.DIALOG)
        globe_img.get_style_context().add_class("symbolic-blue")
        globe_img.set_pixel_size(100)
        slide_2.pack_start(globe_img, False, False, 0)

        lbl_country_title = Gtk.Label(label="Select Your Country or Region")
        lbl_country_title.get_style_context().add_class("installer-title")
        slide_2.pack_start(lbl_country_title, False, False, 0)

        scroll = Gtk.ScrolledWindow()
        scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll.set_size_request(300, 200)
        scroll.set_halign(Gtk.Align.CENTER)
        scroll.get_style_context().add_class("scroll-list")
        slide_2.pack_start(scroll, True, True, 0)

        self.country_listbox = Gtk.ListBox()
        self.country_listbox.connect("row-selected", self.on_country_row_selected)
        scroll.add(self.country_listbox)

        for c_name, _, _ in COUNTRIES:
            row = Gtk.ListBoxRow()
            row.get_style_context().add_class("list-item-row")
            lbl = Gtk.Label(label=c_name)
            lbl.get_style_context().add_class("list-item-label")
            lbl.set_xalign(0.0)
            row.add(lbl)
            self.country_listbox.add(row)

        nav_box_c = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        nav_box_c.set_margin_top(20)
        nav_box_c.set_halign(Gtk.Align.CENTER)
        slide_2.pack_end(nav_box_c, False, False, 0)

        btn_c_back = Gtk.Button(label="Back")
        btn_c_back.get_style_context().add_class("action-btn")
        btn_c_back.connect("clicked", lambda b: self.stack.set_visible_child_name("welcome"))
        nav_box_c.pack_start(btn_c_back, False, False, 0)

        btn_c_continue = Gtk.Button(label="Continue")
        btn_c_continue.get_style_context().add_class("btn-continue")
        btn_c_continue.connect("clicked", lambda b: self.stack.set_visible_child_name("keyboard"))
        nav_box_c.pack_start(btn_c_continue, False, False, 0)

        # ----------------------------------------------------------------------
        # Slide 3: Keyboard Selection
        # ----------------------------------------------------------------------
        slide_3 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_3.get_style_context().add_class("installer-box")
        slide_3.set_size_request(680, 520)
        self.stack.add_named(slide_3, "keyboard")

        kbd_img = Gtk.Image.new_from_icon_name("input-keyboard-symbolic", Gtk.IconSize.DIALOG)
        kbd_img.get_style_context().add_class("symbolic-blue")
        kbd_img.set_pixel_size(100)
        slide_3.pack_start(kbd_img, False, False, 0)

        lbl_kbd_title = Gtk.Label(label="Select Your Keyboard Layout")
        lbl_kbd_title.get_style_context().add_class("installer-title")
        slide_3.pack_start(lbl_kbd_title, False, False, 0)

        scroll_k = Gtk.ScrolledWindow()
        scroll_k.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll_k.set_size_request(300, 200)
        scroll_k.set_halign(Gtk.Align.CENTER)
        scroll_k.get_style_context().add_class("scroll-list")
        slide_3.pack_start(scroll_k, True, True, 0)

        self.kbd_listbox = Gtk.ListBox()
        self.kbd_listbox.connect("row-selected", self.on_keyboard_row_selected)
        scroll_k.add(self.kbd_listbox)

        for k_name, _ in KEYBOARDS:
            row = Gtk.ListBoxRow()
            row.get_style_context().add_class("list-item-row")
            lbl = Gtk.Label(label=k_name)
            lbl.get_style_context().add_class("list-item-label")
            lbl.set_xalign(0.0)
            row.add(lbl)
            self.kbd_listbox.add(row)

        nav_box_k = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        nav_box_k.set_margin_top(20)
        nav_box_k.set_halign(Gtk.Align.CENTER)
        slide_3.pack_end(nav_box_k, False, False, 0)

        btn_k_back = Gtk.Button(label="Back")
        btn_k_back.get_style_context().add_class("action-btn")
        btn_k_back.connect("clicked", lambda b: self.stack.set_visible_child_name("country"))
        nav_box_k.pack_start(btn_k_back, False, False, 0)

        btn_k_continue = Gtk.Button(label="Continue")
        btn_k_continue.get_style_context().add_class("btn-continue")
        btn_k_continue.connect("clicked", lambda b: self.stack.set_visible_child_name("account"))
        nav_box_k.pack_start(btn_k_continue, False, False, 0)

        # ----------------------------------------------------------------------
        # Slide 4: User Account Creation (Apple style macOS Account)
        # ----------------------------------------------------------------------
        slide_4 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_4.get_style_context().add_class("installer-box")
        slide_4.set_size_request(680, 520)
        self.stack.add_named(slide_4, "account")

        user_img = Gtk.Image.new_from_icon_name("avatar-default-symbolic", Gtk.IconSize.DIALOG)
        user_img.get_style_context().add_class("symbolic-blue")
        user_img.set_pixel_size(70)
        slide_4.pack_start(user_img, False, False, 0)

        lbl_acc_title = Gtk.Label(label="Create a Mac Account")
        lbl_acc_title.get_style_context().add_class("installer-title")
        slide_4.pack_start(lbl_acc_title, False, False, 0)

        lbl_acc_desc = Gtk.Label(label="The password you create here will be used to log in to this Mac.")
        lbl_acc_desc.get_style_context().add_class("installer-desc")
        slide_4.pack_start(lbl_acc_desc, False, False, 0)

        # Form Layout
        grid = Gtk.Grid()
        grid.set_column_spacing(16)
        grid.set_row_spacing(10)
        grid.set_halign(Gtk.Align.CENTER)
        slide_4.pack_start(grid, True, True, 0)

        # Full Name
        lbl_fullname = Gtk.Label(label="Full name:")
        lbl_fullname.get_style_context().add_class("form-label")
        lbl_fullname.set_halign(Gtk.Align.END)
        self.entry_fullname = Gtk.Entry()
        self.entry_fullname.connect("changed", self.on_fullname_changed)
        grid.attach(lbl_fullname, 0, 0, 1, 1)
        grid.attach(self.entry_fullname, 1, 0, 1, 1)

        # Account Name (Username)
        lbl_username = Gtk.Label(label="Account name:")
        lbl_username.get_style_context().add_class("form-label")
        lbl_username.set_halign(Gtk.Align.END)
        self.entry_username = Gtk.Entry()
        self.entry_username.connect("changed", self.on_account_fields_changed)
        grid.attach(lbl_username, 0, 1, 1, 1)
        grid.attach(self.entry_username, 1, 1, 1, 1)

        # Hostname (PC Name)
        lbl_hostname = Gtk.Label(label="Computer name:")
        lbl_hostname.get_style_context().add_class("form-label")
        lbl_hostname.set_halign(Gtk.Align.END)
        self.entry_hostname = Gtk.Entry()
        self.entry_hostname.connect("changed", self.on_account_fields_changed)
        grid.attach(lbl_hostname, 0, 2, 1, 1)
        grid.attach(self.entry_hostname, 1, 2, 1, 1)

        # Password
        lbl_password = Gtk.Label(label="Password:")
        lbl_password.get_style_context().add_class("form-label")
        lbl_password.set_halign(Gtk.Align.END)
        self.entry_password = Gtk.Entry()
        self.entry_password.set_visibility(False)
        self.entry_password.connect("changed", self.on_account_fields_changed)
        grid.attach(lbl_password, 0, 3, 1, 1)
        grid.attach(self.entry_password, 1, 3, 1, 1)

        # Verify password
        lbl_verify = Gtk.Label(label="Verify:")
        lbl_verify.get_style_context().add_class("form-label")
        lbl_verify.set_halign(Gtk.Align.END)
        self.entry_verify = Gtk.Entry()
        self.entry_verify.set_visibility(False)
        self.entry_verify.connect("changed", self.on_account_fields_changed)
        grid.attach(lbl_verify, 0, 4, 1, 1)
        grid.attach(self.entry_verify, 1, 4, 1, 1)

        # Root Password / Use Same Password checkbox
        self.chk_same_password = Gtk.CheckButton(label="Use same password for Administrator/Root")
        self.chk_same_password.set_active(True)
        self.chk_same_password.connect("toggled", self.on_same_pwd_toggled)
        grid.attach(self.chk_same_password, 1, 5, 1, 1)

        self.lbl_root_pwd = Gtk.Label(label="Root Password:")
        self.lbl_root_pwd.get_style_context().add_class("form-label")
        self.lbl_root_pwd.set_halign(Gtk.Align.END)
        self.entry_root_pwd = Gtk.Entry()
        self.entry_root_pwd.set_visibility(False)
        self.entry_root_pwd.connect("changed", self.on_account_fields_changed)
        grid.attach(self.lbl_root_pwd, 0, 6, 1, 1)
        grid.attach(self.entry_root_pwd, 1, 6, 1, 1)

        # Hide root password by default
        self.lbl_root_pwd.hide()
        self.entry_root_pwd.hide()

        # Warning/validation label
        self.lbl_acc_warn = Gtk.Label()
        self.lbl_acc_warn.set_markup("<span size='small' foreground='#ff453a'></span>")
        grid.attach(self.lbl_acc_warn, 1, 7, 1, 1)

        nav_box_a = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        nav_box_a.set_margin_top(20)
        nav_box_a.set_halign(Gtk.Align.CENTER)
        slide_4.pack_end(nav_box_a, False, False, 0)

        btn_a_back = Gtk.Button(label="Back")
        btn_a_back.get_style_context().add_class("action-btn")
        btn_a_back.connect("clicked", lambda b: self.stack.set_visible_child_name("keyboard"))
        nav_box_a.pack_start(btn_a_back, False, False, 0)

        self.btn_a_continue = Gtk.Button(label="Continue")
        self.btn_a_continue.get_style_context().add_class("btn-continue")
        self.btn_a_continue.set_sensitive(False)
        self.btn_a_continue.connect("clicked", lambda b: self.stack.set_visible_child_name("disk_selection"))
        nav_box_a.pack_start(self.btn_a_continue, False, False, 0)

        # ----------------------------------------------------------------------
        # Slide 5: Disk Selection & Partitioning Mode
        # ----------------------------------------------------------------------
        slide_5 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        slide_5.get_style_context().add_class("installer-box")
        slide_5.set_size_request(680, 520)
        self.stack.add_named(slide_5, "disk_selection")

        logo_box_md = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        logo_box_md.set_halign(Gtk.Align.CENTER)
        logo_box_md.set_margin_top(10)
        slide_5.pack_start(logo_box_md, False, False, 0)

        if os.path.exists(logo_path):
            pixbuf_md = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 60, 60, True)
            logo_img_md = Gtk.Image.new_from_pixbuf(pixbuf_md)
        else:
            logo_img_md = Gtk.Image.new_from_icon_name("computer-symbolic", Gtk.IconSize.DIALOG)
            logo_img_md.get_style_context().add_class("symbolic-blue")
            logo_img_md.set_pixel_size(60)
        logo_box_md.pack_start(logo_img_md, True, True, 0)

        lbl_disk_title = Gtk.Label(label="Select Target Disk and Layout")
        lbl_disk_title.get_style_context().add_class("installer-title")
        slide_5.pack_start(lbl_disk_title, False, False, 0)

        self.disks_hbox = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        self.disks_hbox.set_halign(Gtk.Align.CENTER)
        slide_5.pack_start(self.disks_hbox, True, True, 8)

        # Partitioning Options
        options_vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        options_vbox.set_halign(Gtk.Align.CENTER)
        slide_5.pack_start(options_vbox, False, False, 8)

        self.rad_erase = Gtk.RadioButton.new_with_label_from_widget(None, "Erase entire disk and install Pulsar OS (Recommended)")
        self.rad_erase.set_active(True)
        options_vbox.pack_start(self.rad_erase, False, False, 0)

        self.rad_alongside = Gtk.RadioButton.new_with_label_from_widget(self.rad_erase, "Install alongside another operating system (Dual boot)")
        options_vbox.pack_start(self.rad_alongside, False, False, 0)

        self.rad_manual = Gtk.RadioButton.new_with_label_from_widget(self.rad_erase, "Manual partitioning (Opens advanced editor in Calamares)")
        options_vbox.pack_start(self.rad_manual, False, False, 0)

        nav_box_disk = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        nav_box_disk.set_margin_top(14)
        nav_box_disk.set_halign(Gtk.Align.CENTER)
        slide_5.pack_end(nav_box_disk, False, False, 0)

        btn_disk_back = Gtk.Button(label="Back")
        btn_disk_back.get_style_context().add_class("action-btn")
        btn_disk_back.connect("clicked", lambda b: self.stack.set_visible_child_name("account"))
        nav_box_disk.pack_start(btn_disk_back, False, False, 0)

        self.btn_disk_continue = Gtk.Button(label="Install Now")
        self.btn_disk_continue.get_style_context().add_class("btn-continue")
        self.btn_disk_continue.set_sensitive(False)
        self.btn_disk_continue.connect("clicked", self.on_disk_continue_clicked)
        nav_box_disk.pack_start(self.btn_disk_continue, False, False, 0)

        # ----------------------------------------------------------------------
        # Slide 6: Custom Installation Progress Screen (Replaces Calamares GUI on Silent Erase)
        # ----------------------------------------------------------------------
        self.slide_progress = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        self.slide_progress.get_style_context().add_class("installer-box")
        self.slide_progress.set_size_request(680, 520)
        self.stack.add_named(self.slide_progress, "progress")

        prog_logo_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        prog_logo_box.set_halign(Gtk.Align.CENTER)
        prog_logo_box.set_margin_top(40)
        prog_logo_box.set_margin_bottom(12)
        self.slide_progress.pack_start(prog_logo_box, False, False, 0)

        if os.path.exists(logo_path):
            pixbuf_prog = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 120, 120, True)
            prog_img = Gtk.Image.new_from_pixbuf(pixbuf_prog)
        else:
            prog_img = Gtk.Image.new_from_icon_name("system-software-install-symbolic", Gtk.IconSize.DIALOG)
            prog_img.get_style_context().add_class("symbolic-blue")
            prog_img.set_pixel_size(120)
        prog_logo_box.pack_start(prog_img, True, True, 0)

        self.lbl_progress_title = Gtk.Label(label="Installing Pulsar OS")
        self.lbl_progress_title.get_style_context().add_class("installer-title")
        self.slide_progress.pack_start(self.lbl_progress_title, False, False, 0)

        self.lbl_progress_status = Gtk.Label(label="Preparing installation...")
        self.lbl_progress_status.get_style_context().add_class("installer-desc")
        self.slide_progress.pack_start(self.lbl_progress_status, False, False, 0)

        self.progress_bar = Gtk.ProgressBar()
        self.progress_bar.set_size_request(400, 10)
        self.progress_bar.set_halign(Gtk.Align.CENTER)
        self.progress_bar.set_fraction(0.0)
        self.slide_progress.pack_start(self.progress_bar, False, False, 10)

        self.progress_nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        self.progress_nav_box.set_margin_top(20)
        self.progress_nav_box.set_halign(Gtk.Align.CENTER)
        self.slide_progress.pack_end(self.progress_nav_box, False, False, 0)

        self.btn_prog_reboot = Gtk.Button(label="Restart Now")
        self.btn_prog_reboot.get_style_context().add_class("btn-continue")
        self.btn_prog_reboot.connect("clicked", lambda b: subprocess.run("reboot", shell=True))
        self.progress_nav_box.pack_start(self.btn_prog_reboot, False, False, 0)
        self.progress_nav_box.hide()

        # Always-visible fallback: lets the user open Calamares GUI at any point during installation
        self.btn_open_calamares = Gtk.Button(label="Open Calamares Installer")
        self.btn_open_calamares.get_style_context().add_class("action-btn")
        self.btn_open_calamares.connect("clicked", self._launch_calamares_gui)
        self.slide_progress.pack_end(self.btn_open_calamares, False, False, 8)

        self.populate_disks()

    def add_utility_row(self, title, desc, icon_name):
        row = Gtk.ListBoxRow()
        row.get_style_context().add_class("recovery-row")
        row.title = title

        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        row.add(box)

        # Uses standard colored application icons for the main utilities screen (Slide 0)
        if icon_name == "logo":
            logo_path = "/usr/share/pulsaros-recovery/logo.png"
            if not os.path.exists(logo_path):
                logo_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "logo.png")
            if os.path.exists(logo_path):
                pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 42, 42, True)
                image = Gtk.Image.new_from_pixbuf(pixbuf)
            else:
                image = Gtk.Image.new_from_icon_name("system-software-install", Gtk.IconSize.DND)
        elif icon_name == "safari":
            icon_theme = Gtk.IconTheme.get_default()
            name = "safari" if icon_theme.has_icon("safari") else "web-browser"
            image = Gtk.Image.new_from_icon_name(name, Gtk.IconSize.DND)
        elif icon_name == "timemachine":
            icon_theme = Gtk.IconTheme.get_default()
            name = "time-machine" if icon_theme.has_icon("time-machine") else ("deja-dup" if icon_theme.has_icon("deja-dup") else "document-revert")
            image = Gtk.Image.new_from_icon_name(name, Gtk.IconSize.DND)
        else:
            icon_theme = Gtk.IconTheme.get_default()
            name = "gnome-disks" if icon_theme.has_icon("gnome-disks") else "gnome-disk-utility"
            image = Gtk.Image.new_from_icon_name(name, Gtk.IconSize.DND)

        box.pack_start(image, False, False, 0)

        text_vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        box.pack_start(text_vbox, True, True, 0)

        lbl_title = Gtk.Label(label=title)
        lbl_title.get_style_context().add_class("recovery-title")
        lbl_title.set_xalign(0.0)
        text_vbox.pack_start(lbl_title, False, False, 0)

        lbl_desc = Gtk.Label(label=desc)
        lbl_desc.get_style_context().add_class("recovery-desc")
        lbl_desc.set_xalign(0.0)
        lbl_desc.set_line_wrap(True)
        lbl_desc.set_max_width_chars(50)
        text_vbox.pack_start(lbl_desc, False, False, 0)

        self.util_list.add(row)

    def on_utility_row_selected(self, listbox, row):
        if row is not None:
            self.selected_utility = row.title
            self.btn_util_continue.set_sensitive(True)

    def on_utility_row_activated(self, listbox, row):
        if row is not None:
            self.on_utility_continue_clicked(None)

    def on_utility_continue_clicked(self, button):
        if self.selected_utility == "Restore from Backup":
            self.hide()
            def run_deja_dup():
                run_as_real_user("deja-dup --restore || deja-dup", wait=True)
                GLib.idle_add(self.show)
            import threading
            threading.Thread(target=run_deja_dup, daemon=True).start()
        elif self.selected_utility == "Install Pulsar OS":
            self.stack.set_visible_child_name("welcome")
        elif self.selected_utility == "Seafari Browser":
            run_as_real_user("seafari || firefox || xdg-open https://google.com")
        elif self.selected_utility == "Disk Utility":
            subprocess.Popen("gnome-disks || gnome-disk-utility", shell=True)

    def on_country_row_selected(self, listbox, row):
        if row is not None:
            index = row.get_index()
            self.selected_country = COUNTRIES[index]
            target_lang = self.selected_country[2]
            for i, (_, k_code) in enumerate(KEYBOARDS):
                if k_code == target_lang:
                    self.kbd_listbox.select_row(self.kbd_listbox.get_row_at_index(i))
                    break

    def on_keyboard_row_selected(self, listbox, row):
        if row is not None:
            index = row.get_index()
            self.selected_keyboard = KEYBOARDS[index]

    def on_fullname_changed(self, entry):
        name = entry.get_text()
        username = re.sub(r"[^a-zA-Z0-9]", "", name).lower()
        self.entry_username.set_text(username)
        self.entry_hostname.set_text(f"{username}-pc" if username else "")
        self.on_account_fields_changed(None)

    def on_same_pwd_toggled(self, button):
        active = button.get_active()
        if active:
            self.lbl_root_pwd.hide()
            self.entry_root_pwd.hide()
        else:
            self.lbl_root_pwd.show()
            self.entry_root_pwd.show()
        self.on_account_fields_changed(None)

    def on_account_fields_changed(self, entry):
        fullname = self.entry_fullname.get_text().strip()
        username = self.entry_username.get_text().strip()
        hostname = self.entry_hostname.get_text().strip()
        pwd = self.entry_password.get_text()
        verify = self.entry_verify.get_text()
        same_pwd = self.chk_same_password.get_active()
        root_pwd = self.entry_root_pwd.get_text()

        valid = True
        warn_msg = ""

        if not fullname or not username or not hostname or not pwd or not verify:
            valid = False
        elif pwd != verify:
            valid = False
            warn_msg = "Passwords do not match."
        elif not same_pwd and not root_pwd:
            valid = False

        self.lbl_acc_warn.set_markup(f"<span size='small' foreground='#ff453a'>{warn_msg}</span>")
        self.btn_a_continue.set_sensitive(valid)

    def populate_disks(self):
        for child in self.disks_hbox.get_children():
            self.disks_hbox.remove(child)

        self.btn_disk_continue.set_label("Install Now")
        self.btn_disk_continue.set_sensitive(False)

        self.disk_widgets = []
        disks = get_physical_disks()

        if not disks:
            warning_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
            warning_box.set_halign(Gtk.Align.CENTER)
            warning_box.set_valign(Gtk.Align.CENTER)

            img_warn = Gtk.Image.new_from_icon_name("dialog-warning-symbolic", Gtk.IconSize.DIALOG)
            img_warn.get_style_context().add_class("symbolic-blue")
            img_warn.set_pixel_size(48)
            warning_box.pack_start(img_warn, False, False, 0)

            lbl_warn = Gtk.Label()
            lbl_warn.set_markup("<span font_desc='13' weight='bold' foreground='#ff453a'>No storage disks found</span>")
            warning_box.pack_start(lbl_warn, False, False, 0)

            lbl_warn_desc = Gtk.Label(label="You can continue to launch Calamares in auto-detect mode.")
            lbl_warn_desc.set_line_wrap(True)
            lbl_warn_desc.set_max_width_chars(50)
            warning_box.pack_start(lbl_warn_desc, False, False, 5)

            self.disks_hbox.pack_start(warning_box, True, True, 20)
            self.btn_disk_continue.set_label("Continue Anyway")
            self.btn_disk_continue.set_sensitive(True)
            self.selected_disk_path = None
        else:
            for disk in disks:
                event_box = Gtk.EventBox()
                card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
                card.get_style_context().add_class("disk-card")
                card.set_border_width(12)
                event_box.add(card)

                # Standard colored hard drive icon on Slide 5
                img = Gtk.Image.new_from_icon_name("drive-harddisk", Gtk.IconSize.DIALOG)
                img.set_pixel_size(64)
                card.pack_start(img, False, False, 0)

                lbl_name = Gtk.Label(label=disk["name"])
                lbl_name.get_style_context().add_class("disk-name")
                card.pack_start(lbl_name, False, False, 0)

                lbl_info1 = Gtk.Label(label=disk["model"])
                lbl_info1.get_style_context().add_class("disk-info")
                lbl_info1.set_line_wrap(True)
                lbl_info1.set_max_width_chars(15)
                card.pack_start(lbl_info1, False, False, 0)

                lbl_info2 = Gtk.Label(label=f"{disk['size']} total")
                lbl_info2.get_style_context().add_class("disk-info")
                card.pack_start(lbl_info2, False, False, 0)

                event_box.connect("button-press-event", self.on_disk_clicked, disk["path"], card)
                self.disks_hbox.pack_start(event_box, False, False, 10)
                self.disk_widgets.append((card, disk["path"]))

        self.show_all()

    def on_disk_clicked(self, widget, event, path, card):
        self.selected_disk_path = path
        for c, p in self.disk_widgets:
            c.get_style_context().remove_class("selected")
        card.get_style_context().add_class("selected")
        self.btn_disk_continue.set_sensitive(True)

    def on_disk_continue_clicked(self, button):
        partition_mode = "erase"
        if self.rad_alongside.get_active():
            partition_mode = "alongside"
        elif self.rad_manual.get_active():
            partition_mode = "manual"

        # 1. Save user details to JSON for prefill module
        settings_data = {
            "timezone": self.selected_country[1],
            "keyboardLayout": self.selected_keyboard[1],
            "username": self.entry_username.get_text().strip(),
            "fullName": self.entry_fullname.get_text().strip(),
            "hostname": self.entry_hostname.get_text().strip(),
            "password": self.entry_password.get_text(),
            "rootPassword": self.entry_password.get_text() if self.chk_same_password.get_active() else self.entry_root_pwd.get_text(),
            "autologin": True
        }

        try:
            with open("/tmp/recovery-settings.json", "w") as f:
                json.dump(settings_data, f, indent=4)
        except Exception as e:
            print(f"Failed to write settings JSON: {e}")

        # If user chose manual partitioning or install alongside, run Calamares GUI normally
        if partition_mode == "manual" or partition_mode == "alongside":
            self.configure_calamares_gui(self.selected_disk_path, partition_mode)
            print("Launching Calamares GUI installer...")
            subprocess.Popen(
                "/usr/local/bin/launch-calamares || pkexec calamares || calamares &",
                shell=True,
            )
            Gtk.main_quit()
            return

        # If user chose Erase Disk, launch Calamares silently and monitor its progress
        self.stack.set_visible_child_name("progress")
        self.configure_calamares_silent(self.selected_disk_path)

        threading.Thread(
            target=self.run_calamares_silent_installation,
            daemon=True
        ).start()

    def update_progress(self, fraction, message):
        # English: Switch from pulse (activity) mode to fraction mode when real progress arrives.
        # Español: Cambiar de modo pulso (actividad) a modo fracción cuando llega progreso real.
        if self.progress_bar.get_pulse_step() > 0.0:
            self.progress_bar.set_pulse_step(0.0)
        # Monotonically update progress bar and status text
        if fraction > self.progress_bar.get_fraction():
            self.progress_bar.set_fraction(fraction)

        # Human readable job names
        friendly_message = message
        if "unpackfs" in message.lower() or "extract" in message.lower():
            friendly_message = "Extracting system files..."
        elif "bootloader" in message.lower():
            friendly_message = "Configuring system bootloader..."
        elif "users" in message.lower() or "user" in message.lower():
            friendly_message = "Configuring user accounts..."
        elif "fstab" in message.lower():
            friendly_message = "Writing disk configuration..."

        self.lbl_progress_status.set_text(friendly_message)

        if fraction >= 1.0:
            self.lbl_progress_title.set_text("Installation Complete")
            self.progress_nav_box.show_all()

    def show_error(self, message):
        self.lbl_progress_title.set_text("Installation Failed")
        self.lbl_progress_status.set_markup(f"<span foreground='#ff453a'>{message}</span>")

        for child in self.progress_nav_box.get_children():
            self.progress_nav_box.remove(child)

        # English: Back button returns to disk selection to retry silent install
        # Español: El botón Volver regresa a la selección de disco para reintentar la instalación silenciosa
        btn_retry = Gtk.Button(label="Back")
        btn_retry.get_style_context().add_class("action-btn")
        btn_retry.connect("clicked", lambda b: self.stack.set_visible_child_name("disk_selection"))
        self.progress_nav_box.pack_start(btn_retry, False, False, 0)

        # English: Fallback button opens the full Calamares GUI installer so the user
        #          can complete installation manually if the silent backend fails.
        # Español: El botón de instalación manual abre el instalador GUI completo de Calamares
        #          para que el usuario pueda completar la instalación si el backend silencioso falla.
        btn_calamares = Gtk.Button(label="Open Calamares Installer")
        btn_calamares.get_style_context().add_class("btn-continue")
        btn_calamares.connect("clicked", self._launch_calamares_gui)
        self.progress_nav_box.pack_start(btn_calamares, False, False, 0)

        self.progress_nav_box.show_all()

    def _launch_calamares_gui(self, _btn):
        """English: Restore Calamares configs and open the full GUI installer.
           Español: Restaurar configs de Calamares y abrir el instalador GUI completo."""
        restore_calamares_configs()
        launcher = "/usr/local/bin/launch-calamares"
        fallback = "/usr/bin/calamares"
        cmd = launcher if os.path.exists(launcher) else fallback
        subprocess.Popen(["bash", "-c", cmd])
        self.destroy()

    def run_calamares_silent_installation(self):
        try:
            GLib.idle_add(self.update_progress, 0.02, "Initializing Calamares installer...")
            
            # English: Calamares must run as root so its log lands in /root/.cache/calamares/session.log.
            #          Running as the live user would write to ~/.cache/calamares/session.log (different path)
            #          and the progress monitor would hang forever waiting for a file that never appears.
            # Español: Calamares debe ejecutarse como root para que su log se escriba en /root/.cache/calamares/session.log.
            #          Si corre como el usuario live el log iría a ~/.cache/calamares/session.log (ruta distinta)
            #          y el monitor de progreso se quedaría bloqueado esperando un archivo que nunca aparece.
            log_path = "/root/.cache/calamares/session.log"

            # Clean old log so we only parse fresh output
            # Eliminar el log antiguo para analizar solo salida nueva
            try:
                os.makedirs("/root/.cache/calamares", exist_ok=True)
                if os.path.exists(log_path):
                    os.remove(log_path)
            except Exception as e:
                print(f"Could not clear old Calamares log: {e}")

            # English: Run Calamares headlessly via sudo with QT_QPA_PLATFORM=offscreen.
            # Español: Ejecutar Calamares sin GUI via sudo con QT_QPA_PLATFORM=offscreen.
            cmd = "sudo -H bash -c 'QT_QPA_PLATFORM=offscreen calamares -d'"

            print(f"Executing silent installer: {cmd}")
            process = subprocess.Popen(
                cmd,
                shell=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )

            # English: Wait up to 30s for Calamares to create its session log.
            #          Pulse the progress bar during init so the user sees activity.
            # Español: Esperar hasta 30s a que Calamares cree su log de sesión.
            #          Hacer pulsar la barra de progreso durante la init para que el usuario vea actividad.
            start_time = time.time()
            while not os.path.exists(log_path):
                if process.poll() is not None:
                    stdout_data = process.stdout.read() if process.stdout else ""
                    stderr_data = process.stderr.read() if process.stderr else ""
                    print(f"Calamares exited immediately with code {process.returncode}")
                    print(f"stdout: {stdout_data}")
                    print(f"stderr: {stderr_data}")
                    GLib.idle_add(self.show_error, f"Installer failed to start (code {process.returncode}).")
                    restore_calamares_configs()
                    return
                elapsed = time.time() - start_time
                if elapsed > 30:
                    process.kill()
                    GLib.idle_add(self.show_error, "Calamares took too long to start (30s timeout).")
                    restore_calamares_configs()
                    return
                # Pulse the progress bar every 0.5s to indicate activity
                GLib.idle_add(self.progress_bar.pulse)
                time.sleep(0.5)
                
            # Parse Calamares logs for monotonic progress bar updates
            last_pct = 0.02
            self.current_job_name = "Preparing disk layout..."
            
            with open(log_path, "r", errors="replace") as f:
                f.seek(0)
                while True:
                    ret = process.poll()
                    lines = f.readlines()
                    for line in lines:
                        # Extract progress fraction or percentage
                        match_pct = re.search(r"JobQueue\s+progress:\s*([0-9.]+)", line, re.IGNORECASE)
                        if match_pct:
                            val = float(match_pct.group(1))
                            pct = val if val <= 1.0 else val / 100.0
                            if pct > last_pct:
                                last_pct = pct
                                GLib.idle_add(self.update_progress, last_pct, self.current_job_name)
                                
                        # Extract current running job name
                        match_job = re.search(r"running\s+job\s+\"([^\"]+)\"", line, re.IGNORECASE)
                        if not match_job:
                            match_job = re.search(r"Running\s+\"([^\"]+)\"", line, re.IGNORECASE)
                        if match_job:
                            self.current_job_name = match_job.group(1)
                            GLib.idle_add(self.update_progress, last_pct, self.current_job_name)
                            
                    if ret is not None:
                        # Read final logs
                        lines = f.readlines()
                        for line in lines:
                            match_pct = re.search(r"JobQueue\s+progress:\s*([0-9.]+)", line, re.IGNORECASE)
                            if match_pct:
                                val = float(match_pct.group(1))
                                pct = val if val <= 1.0 else val / 100.0
                                if pct > last_pct:
                                    last_pct = pct
                        break
                    time.sleep(0.1)
                    
            # Completed
            if process.returncode == 0:
                GLib.idle_add(self.update_progress, 1.0, "Installation completed successfully!")
            else:
                stderr_out = process.stderr.read() if process.stderr else ""
                GLib.idle_add(self.show_error, f"Installation failed with code {process.returncode}. {stderr_out}")
                
            restore_calamares_configs()
            
        except Exception as e:
            print(f"Silent Calamares execution failed: {e}")
            GLib.idle_add(self.show_error, f"Installation failed: {e}")
            restore_calamares_configs()

    def configure_calamares_gui(self, disk_path, partition_mode):
        settings_path = "/etc/calamares/settings.conf"
        bak_path = "/etc/calamares/settings.conf.bak"

        if not os.path.exists(bak_path) and os.path.exists(settings_path):
            try:
                import shutil
                shutil.copy(settings_path, bak_path)
            except Exception as e:
                print(f"Failed to backup settings.conf: {e}")

        if os.path.exists(settings_path):
            try:
                if partition_mode == "manual":
                    if os.path.exists(bak_path):
                        import shutil
                        shutil.copy(bak_path, settings_path)
                else:
                    with open(settings_path, "r") as f:
                        content = f.read()

                    # Only show Partition and Finished pages on GUI mode
                    lines = content.splitlines(keepends=True)
                    new_lines = []
                    in_show = False
                    for line in lines:
                        if line.startswith("  - show:"):
                            in_show = True
                            new_lines.append(line)
                            new_lines.append("      - partition\n")
                            new_lines.append("      - finished\n")
                            continue
                        if in_show:
                            if line.startswith("  - exec:") or line.startswith("  - show:"):
                                in_show = False
                            elif not line.startswith("      -"):
                                in_show = False
                            else:
                                continue
                        new_lines.append(line)
                    content = "".join(new_lines)

                    if "- prefill" not in content:
                        content = content.replace("  - partition\n  - mount", "  - prefill\n  - partition\n  - mount")

                    with open(settings_path, "w") as f:
                        f.write(content)
            except Exception as e:
                print(f"Failed to configure settings.conf: {e}")

        if disk_path:
            partition_conf = "/etc/calamares/modules/partition.conf"
            try:
                if os.path.exists(partition_conf):
                    with open(partition_conf, "r") as f:
                        lines = f.readlines()
                    new_lines = []
                    found = False
                    for line in lines:
                        if line.strip().startswith("defaultDisk:"):
                            new_lines.append(f'defaultDisk: "{disk_path}"\n')
                            found = True
                        else:
                            new_lines.append(line)
                    if not found:
                        new_lines.append(f'\ndefaultDisk: "{disk_path}"\n')
                    with open(partition_conf, "w") as f:
                        f.writelines(new_lines)
            except Exception as e:
                print(f"Failed to set default disk in partition.conf: {e}")

    def configure_calamares_silent(self, disk_path):
        settings_path = "/etc/calamares/settings.conf"
        bak_path = "/etc/calamares/settings.conf.bak"

        if not os.path.exists(bak_path) and os.path.exists(settings_path):
            try:
                import shutil
                shutil.copy(settings_path, bak_path)
            except Exception as e:
                print(f"Failed to backup settings.conf: {e}")

        if os.path.exists(settings_path):
            try:
                with open(settings_path, "r") as f:
                    content = f.read()

                # Clean up the first show block in sequence (unattended install)
                # but keep the final show block containing the 'finished' module.
                lines = content.splitlines(keepends=True)
                new_lines = []
                in_show = False
                for line in lines:
                    if line.startswith("  - show:"):
                        if not any(nl.startswith("  - exec:") for nl in new_lines):
                            in_show = True
                            continue
                    if in_show:
                        if line.startswith("  - exec:") or line.startswith("  - show:"):
                            in_show = False
                        elif not line.startswith("      -"):
                            in_show = False
                        else:
                            continue
                    new_lines.append(line)
                content = "".join(new_lines)

                if "- prefill" not in content:
                    content = content.replace("  - partition\n  - mount", "  - prefill\n  - partition\n  - mount")

                with open(settings_path, "w") as f:
                    f.write(content)
            except Exception as e:
                print(f"Failed to configure settings.conf: {e}")

        # Configure partition.conf to handle auto-partitioning on the chosen disk
        partition_conf = "/etc/calamares/modules/partition.conf"
        partition_bak = "/etc/calamares/modules/partition.conf.bak"
        if not os.path.exists(partition_bak) and os.path.exists(partition_conf):
            try:
                import shutil
                shutil.copy(partition_conf, partition_bak)
            except Exception as e:
                print(f"Failed to backup partition.conf: {e}")

        if os.path.exists(partition_conf):
            try:
                with open(partition_conf, "r") as f:
                    lines = f.readlines()
                new_lines = []
                for line in lines:
                    if line.strip().startswith("defaultDisk:"):
                        new_lines.append(f'defaultDisk: "{disk_path}"\n')
                    elif line.strip().startswith("partitionLayout:"):
                        break
                    else:
                        new_lines.append(line)

                # OEM Silent Partition Layout settings in partition.conf
                layout_yaml = (
                    "\npartitionLayout:\n"
                    "    - name: \"efi\"\n"
                    "      size: 512M\n"
                    "      filesystem: \"fat32\"\n"
                    "      mountPoint: \"/boot/efi\"\n"
                    "      flags: [ boot, esp ]\n"
                    "    - name: \"root\"\n"
                    "      size: 100%\n"
                    "      filesystem: \"ext4\"\n"
                    "      mountPoint: \"/\"\n"
                )
                new_lines.append(layout_yaml)
                with open(partition_conf, "w") as f:
                    f.writelines(new_lines)
            except Exception as e:
                print(f"Failed to configure partition.conf for auto install: {e}")


def main():
    RecoveryApp()
    Gtk.main()


if __name__ == "__main__":
    main()
