#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# ==============================================================================
# Pulsar OS - Recovery Utility Screen (GTK4 & Libadwaita macOS Sonoma Style)
# ==============================================================================

import sys
import os
import subprocess
import threading
import time
import re
import datetime
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
gi.require_version('GdkPixbuf', '2.0')

from gi.repository import Gtk, Gdk, GLib, Adw, Gio, GdkPixbuf

# Custom CSS for Apple macOS Recovery and Installer Look-and-Feel
CSS_DATA = """
window, .root-container {
    background-color: #1e1e1e; /* dark theme base */
}
window, .root-container, * {
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
.welcome-title {
    font-size: 26px;
    font-weight: 700;
    color: #ffffff;
    margin-top: 12px;
    margin-bottom: 4px;
}
.welcome-subtitle {
    font-size: 13px;
    color: #8e8e93;
    margin-bottom: 24px;
}
.apple-box {
    background-color: #2a2a2a;
    border: 1px solid #3c3c3c;
    border-radius: 12px;
    padding: 24px;
    box-shadow: 0 10px 40px rgba(0,0,0,0.6);
}
.suggested-action {
    background-color: #0071e3; /* Apple Blue */
    color: #ffffff;
    border-radius: 8px;
    font-weight: bold;
    padding: 10px 24px;
    border: none;
}
.suggested-action:hover {
    background-color: #007bf5;
}
.suggested-action:active {
    background-color: #0063c6;
}
.suggested-action:disabled {
    background-color: #3a3a3c;
    color: #8e8e93;
}
.secondary-action {
    background-color: #323236;
    color: #ffffff;
    border-radius: 8px;
    font-weight: bold;
    padding: 10px 24px;
    border: 1px solid #48484a;
}
.secondary-action:hover {
    background-color: #3e3e42;
}
.secondary-action:active {
    background-color: #2c2c2e;
}
.progress-bar-thin {
    min-height: 4px;
    margin-top: 12px;
    margin-bottom: 12px;
}
.progress-text {
    font-size: 12px;
    color: #aeaeb2;
}
list, listbox {
    background-color: transparent;
    border: none;
}
listrow, listboxrow {
    background-color: #2a2a2a;
    border: none;
    transition: background-color 0.15s ease;
}
listrow:hover, listboxrow:hover {
    background-color: #323236;
}
listrow:selected, listboxrow:selected {
    background-color: #323236;
}
.utility-row-box {
    padding: 12px;
    border-bottom: none;
}
.utility-title-lbl {
    font-size: 14px;
    font-weight: bold;
    color: #ffffff;
}
.utility-desc-lbl {
    font-size: 11px;
    color: #aeaeb2;
}
.disk-card {
    background-color: #2a2a2a;
    border: 1px solid #3c3c3c;
    border-radius: 12px;
    padding: 20px;
    min-width: 140px;
    margin: 8px;
    transition: all 0.15s ease;
}
.disk-card:hover {
    background-color: #323236;
}
.disk-card.selected {
    background-color: #323236;
    border-color: #0071e3;
    box-shadow: 0 0 0 2px #0071e3;
}
.disk-name {
    font-size: 13px;
    font-weight: bold;
    color: #ffffff;
    margin-top: 6px;
    text-align: center;
}
.disk-info {
    font-size: 10px;
    color: #8e8e93;
    text-align: center;
}
"""

def get_system_disks():
    disks = []
    try:
        out = subprocess.check_output(
            ["lsblk", "-dno", "NAME,SIZE,MODEL,TYPE"], text=True
        )
        for line in out.strip().split("\n"):
            if not line:
                continue
            parts = [p for p in line.split() if p]
            if len(parts) >= 2:
                name = parts[0]
                size = parts[1]
                model = " ".join(parts[2:-1]) if len(parts) > 3 else "Generic Disk"
                dev_type = parts[-1]
                
                if dev_type == "disk" and not name.startswith("loop") and not name.startswith("sr"):
                    disks.append({
                        "path": f"/dev/{name}",
                        "name": f"/dev/{name} - {model} ({size})"
                    })
    except Exception as e:
        print(f"Error parsing lsblk: {e}")
        try:
            with open("/proc/partitions", "r") as f:
                for line in f:
                    parts = line.split()
                    if len(parts) == 4:
                        name = parts[3]
                        if re.match(r"^(sd[a-z]|nvme[0-9]n[0-9]|vd[a-z])$", name):
                            disks.append({
                                "path": f"/dev/{name}",
                                "name": f"/dev/{name} (Unknown)"
                            })
        except Exception as ex:
            print(f"Fallback reading /proc/partitions failed: {ex}")
            
    if not disks:
        disks.append({"path": "/dev/sda", "name": "/dev/sda - Simulated Disk (30 GB)"})
        
    return disks


class DiskCard(Gtk.Box):
    def __init__(self, disk_info, select_callback):
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.disk_info = disk_info
        self.select_callback = select_callback
        self.add_css_class("disk-card")
        self.set_valign(Gtk.Align.CENTER)
        self.set_halign(Gtk.Align.CENTER)
        
        icon = Gtk.Image.new_from_icon_name("drive-harddisk")
        icon.set_pixel_size(48)
        self.append(icon)
        
        name_lbl = Gtk.Label(label=disk_info["path"].replace("/dev/", ""))
        name_lbl.add_css_class("disk-name")
        self.append(name_lbl)
        
        size_match = re.search(r"\(([^)]+)\)", disk_info["name"])
        size_str = size_match.group(1) if size_match else "Unknown"
        
        details_lbl = Gtk.Label(label=f"{size_str} total\nAvailable")
        details_lbl.add_css_class("disk-info")
        self.append(details_lbl)
        
        gesture = Gtk.GestureClick()
        gesture.connect("released", self.on_clicked)
        self.add_controller(gesture)
        
    def on_clicked(self, gesture, n_press, x, y):
        self.select_callback(self)


class RecoveryWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Pulsar OS Recovery")
        self.set_default_size(720, 560)
        self.set_resizable(True)
        
        self.apply_css()
        
        # Always fullscreen
        self.fullscreen()
            
        # Centered container
        center_container = Gtk.CenterBox()
        center_container.add_css_class("root-container")
        center_container.set_hexpand(True)
        center_container.set_vexpand(True)
        
        self.card_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.card_box.add_css_class("apple-box")
        self.card_box.set_size_request(480, 380)
        self.card_box.set_valign(Gtk.Align.CENTER)
        self.card_box.set_halign(Gtk.Align.CENTER)
        
        # Crossfade transition Gtk.Stack (macos styled crossfade transition)
        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self.stack.set_transition_duration(500)
        self.card_box.append(self.stack)
        
        center_container.set_center_widget(self.card_box)
        self.set_content(center_container)
        
        # Build views
        self.build_utilities_screen()
        self.build_install_welcome_screen()
        self.build_install_disk_select_screen()
        self.build_install_progress_screen()
        
        self.stack.set_visible_child_name("utilities")
        self.selected_action = None
        self.selected_disk_card = None

    def apply_css(self):
        provider = Gtk.CssProvider()
        provider.load_from_data(CSS_DATA.encode('utf-8'))
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def create_row_icon(self, icon_name):
        img = Gtk.Image()
        img.set_pixel_size(42)
        
        script_dir = os.path.dirname(os.path.abspath(__file__))
        
        def get_path(filename):
            for base in (script_dir, "/usr/share/pulsaros-recovery"):
                p = os.path.join(base, filename)
                if os.path.exists(p):
                    return p
            return None

        def try_set_from_file(path):
            if not path:
                return False
            try:
                gfile = Gio.File.new_for_path(path)
                texture = Gdk.Texture.new_from_file(gfile)
                img.set_from_paintable(texture)
                return True
            except Exception as e:
                print(f"Failed to load icon {path}: {e}")
                return False

        if icon_name == "logo":
            icon_path = get_path("installer-logo.png") or get_path("logo.png")
            if not try_set_from_file(icon_path):
                img.set_from_icon_name("system-software-install")
        elif icon_name == "timemachine":
            icon_path = get_path("timemachine.png")
            if not try_set_from_file(icon_path):
                icon_path = get_path("org.gnome.DejaDup.svg")
                if not try_set_from_file(icon_path):
                    img.set_from_icon_name("document-revert")
        elif icon_name == "safari":
            icon_path = get_path("safari.png")
            if not icon_path or os.path.getsize(icon_path) < 100:
                temp_icon = "/tmp/seafari.png"
                if not os.path.exists(temp_icon):
                    try:
                        import urllib.request
                        urllib.request.urlretrieve("https://hosted.inled.es/seafari.png", temp_icon)
                        icon_path = temp_icon
                    except Exception as e:
                        print(f"Failed to download seafari icon: {e}")
                else:
                    icon_path = temp_icon
            if not try_set_from_file(icon_path):
                img.set_from_icon_name("web-browser")
        elif icon_name == "disk":
            icon_path = get_path("diskutility.png")
            if not try_set_from_file(icon_path):
                img.set_from_icon_name("drive-harddisk")
        else:
            img.set_from_icon_name("drive-harddisk")
            
        return img

    def get_logo_image(self, pixel_size, is_installer=True):
        image = Gtk.Image()
        image.set_pixel_size(pixel_size)
        script_dir = os.path.dirname(os.path.abspath(__file__))
        
        if is_installer:
            candidates = ["installer-logo.png", "logo.png"]
        else:
            candidates = ["pulsar-logo.png", "logo.png"]
        
        path_to_load = None
        for name in candidates:
            for base in (script_dir, "/usr/share/pulsaros-recovery"):
                p = os.path.join(base, name)
                if os.path.exists(p):
                    path_to_load = p
                    break
            if path_to_load:
                break
        
        if path_to_load:
            try:
                gfile = Gio.File.new_for_path(path_to_load)
                texture = Gdk.Texture.new_from_file(gfile)
                image.set_from_paintable(texture)
            except Exception as e:
                print(f"Failed to load logo {path_to_load}: {e}")
                image.set_from_icon_name("system-software-install")
        else:
            image.set_from_icon_name("system-software-install")
            
        return image

    def add_utility_row(self, listbox, action_id, title, desc, icon_name):
        row = Gtk.ListBoxRow()
        row.action_id = action_id
        
        row_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        row_box.add_css_class("utility-row-box")
        
        icon_img = self.create_row_icon(icon_name)
        row_box.append(icon_img)
        
        text_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        text_box.set_valign(Gtk.Align.CENTER)
        
        title_lbl = Gtk.Label(label=title)
        title_lbl.add_css_class("utility-title-lbl")
        title_lbl.set_halign(Gtk.Align.START)
        text_box.append(title_lbl)
        
        desc_lbl = Gtk.Label(label=desc)
        desc_lbl.add_css_class("utility-desc-lbl")
        desc_lbl.set_halign(Gtk.Align.START)
        desc_lbl.set_wrap(True)
        text_box.append(desc_lbl)
        
        row_box.append(text_box)
        row.set_child(row_box)
        listbox.append(row)

    def build_utilities_screen(self):
        screen_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        screen_box.set_valign(Gtk.Align.CENTER)
        
        self.listbox = Gtk.ListBox()
        self.listbox.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.listbox.connect("row-selected", self.on_utility_row_selected)
        screen_box.append(self.listbox)
        
        self.add_utility_row(self.listbox, "backup", "Restore from Time Machine", 
                             "If you have backup of your system that you want to restore.", "timemachine")
        self.add_utility_row(self.listbox, "install", "Reinstall Pulsar OS", 
                             "Install a new copy of Pulsar OS onto your computer.", "logo")
        self.add_utility_row(self.listbox, "safari", "Seafari", 
                             "Browse the web to get help with your computer.", "safari")
        self.add_utility_row(self.listbox, "disk", "Disk Utility", 
                             "Repair or erase a disk using Disk Utility.", "disk")
                             
        bottom_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        bottom_box.set_margin_top(12)
        
        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        bottom_box.append(spacer)
        
        self.btn_continue = Gtk.Button(label="Continue")
        self.btn_continue.add_css_class("suggested-action")
        self.btn_continue.set_sensitive(False)
        self.btn_continue.connect("clicked", self.on_utility_continue_clicked)
        bottom_box.append(self.btn_continue)
        
        screen_box.append(bottom_box)
        self.stack.add_named(screen_box, "utilities")

    def on_utility_row_selected(self, listbox, row):
        if row is not None:
            self.selected_action = row.action_id
            self.btn_continue.set_sensitive(True)

    def _get_real_user(self):
        import pwd

        # 1. Check environment variables set by pkexec/sudo
        for var in ("SUDO_USER", "PKEXEC_UID"):
            val = os.environ.get(var)
            if val:
                if var == "PKEXEC_UID":
                    try:
                        return pwd.getpwuid(int(val)).pw_name
                    except (KeyError, ValueError):
                        pass
                elif val != "root":
                    return val

        # 2. Walk /proc to find the first non-root, non-this-process UID
        my_pid = os.getpid()
        my_uid = os.getuid()
        try:
            for entry in sorted(os.listdir("/proc")):
                if not entry.isdigit() or int(entry) == my_pid:
                    continue
                try:
                    with open(f"/proc/{entry}/status") as f:
                        for line in f:
                            if line.startswith("Uid:"):
                                uid = int(line.split()[1])
                                if uid >= 1000 and uid != my_uid:
                                    return pwd.getpwuid(uid).pw_name
                                break
                except (FileNotFoundError, PermissionError, IndexError):
                    continue
        except FileNotFoundError:
            pass

        # 3. Fallback: owner of the XDG_RUNTIME_DIR
        xdg = os.environ.get("XDG_RUNTIME_DIR", "")
        if xdg and xdg.startswith("/run/user/"):
            try:
                uid = int(xdg.split("/")[3])
                return pwd.getpwuid(uid).pw_name
            except (KeyError, ValueError, IndexError):
                pass

        return None

    def _popen_as_user(self, cmd):
        user = self._get_real_user()
        if user:
            home = f"/home/{user}"
            display = os.environ.get("DISPLAY", "")
            wayland = os.environ.get("WAYLAND_DISPLAY", "")
            xauth = os.environ.get("XAUTHORITY", "")
            xdg = os.environ.get("XDG_RUNTIME_DIR", "")
            env_parts = []
            if home:       env_parts.append(f"HOME={home}")
            if display:    env_parts.append(f"DISPLAY={display}")
            if wayland:    env_parts.append(f"WAYLAND_DISPLAY={wayland}")
            if xauth:      env_parts.append(f"XAUTHORITY={xauth}")
            if xdg:        env_parts.append(f"XDG_RUNTIME_DIR={xdg}")
            env_str = " ".join(env_parts)
            # sudo -u <user>: run as real user (passwordless via sudoers)
            # env: pass display/session vars so the app can render
            full_cmd = f"sudo -u {user} env {env_str} {cmd}"
            subprocess.Popen(full_cmd, shell=True)
        else:
            subprocess.Popen(cmd, shell=True)

    def on_utility_continue_clicked(self, btn):
        if not self.selected_action:
            return
            
        if self.selected_action == "backup":
            self._popen_as_user("deja-dup --restore || deja-dup")
        elif self.selected_action == "install":
            self.show_installer_selector_dialog()
        elif self.selected_action == "safari":
            self._popen_as_user("seafari")
        elif self.selected_action == "disk":
            self._popen_as_user("gnome-disks || gnome-disk-utility")

    def show_installer_selector_dialog(self):
        # Emergent selector popup matching Apple design
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="Install Pulsar OS",
            body="Choose the installation method you want to use for your computer."
        )
        dialog.add_response("quick", "Quick Install (Recommended)")
        dialog.add_response("guided", "Guided Install (Calamares)")
        dialog.add_response("cancel", "Cancel")
        
        dialog.set_response_appearance("quick", Adw.ResponseAppearance.SUGGESTED)
        dialog.set_response_appearance("guided", Adw.ResponseAppearance.DEFAULT)
        dialog.set_response_appearance("cancel", Adw.ResponseAppearance.DESTRUCTIVE)
        
        def on_response(d, response_id):
            if response_id == "quick":
                self.stack.set_visible_child_name("install_welcome")
            elif response_id == "guided":
                self.on_guided_install_clicked(None)
            d.destroy()
            
        dialog.connect("response", on_response)
        dialog.present()

    def build_install_welcome_screen(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        # Large Pulsar OS Logo (160px size)
        image = self.get_logo_image(160, is_installer=False)
        box.append(image)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='22000'>Pulsar OS</span>")
        box.append(title)
        
        subtext = Gtk.Label(label="To set up the installation of Pulsar OS, click Continue.")
        subtext.add_css_class("welcome-subtitle")
        subtext.set_margin_top(4)
        subtext.set_margin_bottom(12)
        box.append(subtext)
        
        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        nav_box.set_halign(Gtk.Align.CENTER)
        
        btn_back = Gtk.Button(label="Back")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("utilities"))
        nav_box.append(btn_back)
        
        btn_continue = Gtk.Button(label="Continue")
        btn_continue.add_css_class("suggested-action")
        btn_continue.connect("clicked", self.on_welcome_continue_clicked)
        nav_box.append(btn_continue)
        
        box.append(nav_box)
        self.stack.add_named(box, "install_welcome")

    def on_welcome_continue_clicked(self, btn):
        self.refresh_disk_cards()
        self.stack.set_visible_child_name("install_disk_select")

    def build_install_disk_select_screen(self):
        self.disk_select_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        self.disk_select_box.set_valign(Gtk.Align.CENTER)
        self.disk_select_box.set_halign(Gtk.Align.CENTER)
        
        image = self.get_logo_image(100, is_installer=True)
        self.disk_select_box.append(image)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='18000'>Pulsar OS</span>")
        self.disk_select_box.append(title)
        
        self.disk_select_subtitle = Gtk.Label(label="Pulsar OS will be installed on the selected disk.")
        self.disk_select_subtitle.add_css_class("welcome-subtitle")
        self.disk_select_box.append(self.disk_select_subtitle)
        
        self.disk_cards_flow = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.disk_cards_flow.set_halign(Gtk.Align.CENTER)
        self.disk_select_box.append(self.disk_cards_flow)
        
        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        nav_box.set_halign(Gtk.Align.CENTER)
        nav_box.set_margin_top(12)
        
        btn_back = Gtk.Button(label="Back")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("install_welcome"))
        nav_box.append(btn_back)
        
        self.btn_disk_continue = Gtk.Button(label="Continue")
        self.btn_disk_continue.add_css_class("suggested-action")
        self.btn_disk_continue.set_sensitive(False)
        self.btn_disk_continue.connect("clicked", self.on_disk_continue_clicked)
        nav_box.append(self.btn_disk_continue)
        
        self.disk_select_box.append(nav_box)
        self.stack.add_named(self.disk_select_box, "install_disk_select")

    def refresh_disk_cards(self):
        while (child := self.disk_cards_flow.get_first_child()):
            self.disk_cards_flow.remove(child)
            
        self.disk_cards = []
        self.selected_disk_card = None
        self.btn_disk_continue.set_sensitive(False)
        
        disks = get_system_disks()
        for disk_info in disks:
            card = DiskCard(disk_info, self.on_disk_card_selected)
            self.disk_cards_flow.append(card)
            self.disk_cards.append(card)

    def on_disk_card_selected(self, selected_card):
        for card in self.disk_cards:
            card.remove_css_class("selected")
            
        selected_card.add_css_class("selected")
        self.selected_disk_card = selected_card
        
        disk_path = selected_card.disk_info["path"].replace("/dev/", "")
        self.disk_select_subtitle.set_label(f"Pulsar OS will be installed on disk \"{disk_path}\".")
        self.btn_disk_continue.set_sensitive(True)

    def on_disk_continue_clicked(self, btn):
        if not self.selected_disk_card:
            return

        disk_path = self.selected_disk_card.disk_info["path"]
        disk_name = disk_path.replace("/dev/", "")
        self.pending_disk_path = disk_path
        self.pending_disk_name = disk_name
        self.install_nvidia = False
        self.install_broadcom = False
        self.nvidia_info = self._detect_nvidia()
        self._show_nvidia_dialog()

    def build_install_progress_screen(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        image = self.get_logo_image(100, is_installer=True)
        box.append(image)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='18000'>Pulsar OS</span>")
        box.append(title)
        
        self.progress_subtitle = Gtk.Label(label="Pulsar OS will be installed on the disk.")
        self.progress_subtitle.add_css_class("progress-text")
        box.append(self.progress_subtitle)
        
        self.progress_bar = Gtk.ProgressBar()
        self.progress_bar.add_css_class("progress-bar-thin")
        self.progress_bar.set_size_request(280, -1)
        box.append(self.progress_bar)
        
        self.progress_label = Gtk.Label(label="Preparing installation...")
        self.progress_label.add_css_class("progress-text")
        box.append(self.progress_label)
        
        self.btn_install_action = Gtk.Button(label="Cancel")
        self.btn_install_action.add_css_class("secondary-action")
        self.btn_install_action.set_margin_top(12)
        self.btn_install_action.set_halign(Gtk.Align.CENTER)
        self.btn_install_action.set_size_request(140, -1)
        self.btn_install_action.connect("clicked", self.on_progress_cancel_clicked)
        box.append(self.btn_install_action)
        
        self.stack.add_named(box, "install_progress")

    def on_progress_cancel_clicked(self, btn):
        if btn.get_label() == "Restart System":
            if "TEST_MODE" in os.environ:
                print("[TEST_MODE] Simulating systemctl reboot...")
                self.close()
            else:
                subprocess.Popen(["systemctl", "reboot"])
                self.close()
        else:
            self.stack.set_visible_child_name("install_disk_select")

    def update_progress(self, fraction, text):
        self.progress_bar.set_fraction(fraction)
        self.progress_label.set_label(text)

    # ──────────────────────────────────────────────────────────────
    # Hardware detection helpers
    # ──────────────────────────────────────────────────────────────

    def _detect_nvidia(self):
        """Returns dict with keys: found, name, is_new_gen (Turing/Ampere/Ada/Blackwell ≥ GTX 1600/RTX)"""
        try:
            out = subprocess.check_output(
                ["lspci", "-nn"], text=True, stderr=subprocess.DEVNULL
            )
        except Exception:
            return {"found": False}
        for line in out.splitlines():
            if "NVIDIA" in line and ("VGA" in line or "3D" in line or "Display" in line):
                name = line.split(":", 2)[-1].strip()
                # Turing = RTX 20xx / GTX 1660, Ampere = RTX 30xx,
                # Ada = RTX 40xx, Blackwell = RTX 50xx  → considered "new"
                import re as _re
                m = _re.search(r"(?:RTX|GTX)\s*(\d+)", name, _re.IGNORECASE)
                if m:
                    num = int(m.group(1))
                    is_new = num >= 1600
                else:
                    # Quadro / Tesla / older numbering – treat as old
                    is_new = False
                return {"found": True, "name": name, "is_new": is_new}
        return {"found": False}

    def _detect_broadcom(self):
        """Returns True if a Broadcom WiFi/BT chip is detected via lspci/lsusb."""
        try:
            pci = subprocess.check_output(["lspci", "-nn"], text=True, stderr=subprocess.DEVNULL)
            if "Broadcom" in pci and ("Network" in pci or "Wireless" in pci or "BCM" in pci):
                return True
        except Exception:
            pass
        try:
            usb = subprocess.check_output(["lsusb"], text=True, stderr=subprocess.DEVNULL)
            if "Broadcom" in usb or "BCM" in usb:
                return True
        except Exception:
            pass
        return False

    def _get_pulsar_channel(self):
        """Reads /etc/pulsar-channel (values: stable | forky | rolling)."""
        try:
            with open("/etc/pulsar-channel") as f:
                return f.read().strip().lower()
        except Exception:
            pass
        # Fallback: check VERSION_CODENAME in os-release
        try:
            with open("/etc/os-release") as f:
                for line in f:
                    if line.startswith("VERSION_CODENAME"):
                        val = line.split("=", 1)[1].strip().strip('"').lower()
                        if val in ("forky", "rolling"):
                            return val
        except Exception:
            pass
        return "stable"

    # ──────────────────────────────────────────────────────────────
    # Hardware dialog chain
    # ──────────────────────────────────────────────────────────────

    def _show_nvidia_dialog(self):
        nvidia = self.nvidia_info
        channel = self._get_pulsar_channel()

        if not nvidia.get("found"):
            # No NVIDIA → skip straight to Broadcom
            self._show_broadcom_dialog()
            return

        gpu_name = nvidia.get("name", "NVIDIA GPU")
        is_new   = nvidia.get("is_new", False)

        if is_new and channel == "stable":
            heading = "New NVIDIA GPU Detected"
            body = (
                f"<b>{gpu_name}</b>\n\n"
                "Your GPU requires recent NVIDIA drivers that work best on "
                "<b>Pulsar OS Forky</b> or <b>Pulsar OS Rolling</b>.\n\n"
                "Continuing on <b>Stable</b> may result in a black screen or "
                "degraded performance. We recommend switching channels after install.\n\n"
                "⚠️  Ethernet recommended — WiFi may not work until drivers are installed."
            )
        else:
            heading = "NVIDIA GPU Detected"
            body = (
                f"<b>{gpu_name}</b>\n\n"
                "Would you like to install NVIDIA proprietary drivers?\n\n"
                "⚠️  Ethernet recommended — WiFi may not work until drivers are installed."
            )

        dialog = Adw.MessageDialog(
            transient_for=self,
            heading=heading,
            body=body,
        )
        dialog.set_body_use_markup(True)
        dialog.add_response("skip",    "Skip")
        dialog.add_response("install", "Install NVIDIA Drivers")
        dialog.set_response_appearance("install", Adw.ResponseAppearance.SUGGESTED)
        dialog.set_default_response("install")

        def on_response(d, resp):
            d.destroy()
            if resp == "install":
                self.install_nvidia = True
            self._show_broadcom_dialog()

        dialog.connect("response", on_response)
        dialog.present()

    def _show_broadcom_dialog(self):
        auto_detected = self._detect_broadcom()
        if auto_detected:
            heading = "Broadcom Hardware Detected"
            body = (
                "A Broadcom WiFi or Bluetooth adapter was detected on this system.\n\n"
                "Would you like to install Broadcom drivers (<tt>broadcom-sta-dkms</tt>)?\n\n"
                "⚠️  Ethernet recommended during installation."
            )
        else:
            heading = "Broadcom Drivers"
            body = (
                "No Broadcom adapter was automatically detected.\n\n"
                "Do you have a Broadcom WiFi or Bluetooth chip? "
                "(Common in some laptops and older Mac hardware.)\n\n"
                "If unsure, choose \"No\"."
            )

        dialog = Adw.MessageDialog(
            transient_for=self,
            heading=heading,
            body=body,
        )
        dialog.set_body_use_markup(True)
        dialog.add_response("no",  "No")
        dialog.add_response("yes", "Yes, install Broadcom drivers")
        dialog.set_response_appearance("yes", Adw.ResponseAppearance.SUGGESTED)
        dialog.set_default_response("yes" if auto_detected else "no")

        def on_response(d, resp):
            d.destroy()
            if resp == "yes":
                self.install_broadcom = True
            self._start_installation()

        dialog.connect("response", on_response)
        dialog.present()

    def _start_installation(self):
        disk_path = self.pending_disk_path
        disk_name = self.pending_disk_name
        self.progress_subtitle.set_label(f"Pulsar OS is installing on disk \"{disk_name}\".")
        self.stack.set_visible_child_name("install_progress")
        threading.Thread(
            target=self.installation_backend,
            args=(disk_path,),
            daemon=True
        ).start()

    def installation_backend(self, disk_path):
        import datetime
        log_file = "/tmp/pulsaros-install.log"
        try:
            with open(log_file, "w") as lf:
                lf.write(f"{datetime.datetime.now()} - Pulsar OS Installation started\n")
                lf.write(f"Target disk: {disk_path}\n")
            # Stop udisks2 automount daemon during formatting and system replication
            if "TEST_MODE" not in os.environ:
                subprocess.run(["systemctl", "stop", "udisks2.service"], capture_output=True)

            def log_msg(msg):
                ts = datetime.datetime.now().strftime("%H:%M:%S")
                with open(log_file, "a") as lf:
                    lf.write(f"[{ts}] {msg}\n")
                print(msg)

            def exec_cmd(cmd, shell=False):
                cmd_str = ' '.join(cmd) if isinstance(cmd, list) else cmd
                log_msg(f"Running: {cmd_str}")
                if "TEST_MODE" in os.environ:
                    log_msg(f"[TEST_MODE] Simulating: {cmd_str}")
                    return ""
                res = subprocess.run(cmd, shell=shell, capture_output=True, text=True)
                if res.returncode != 0:
                    err_msg = f"Failed: {cmd_str}\n{res.stderr}"
                    log_msg(f"ERROR: {err_msg}")
                    raise Exception(err_msg)
                if res.stdout:
                    log_msg(f"Output: {res.stdout[:200]}")
                return res.stdout
                
            def cleanup_mounts(is_efi_boot):
                if "TEST_MODE" not in os.environ:
                    # 1. Kill any processes holding files open inside /mnt (chroot and host) surgically using fuser
                    subprocess.run(["fuser", "-k", "-9", "-M", "/mnt"], capture_output=True)
                    
                    # 2. Unmount nested virtual filesystems first, then root, in strict reverse order of mounting
                    for mount_path in [
                        "/mnt/etc/resolv.conf",
                        "/mnt/run",
                        "/mnt/sys",
                        "/mnt/proc",
                        "/mnt/dev/pts",
                        "/mnt/dev",
                    ]:
                        if os.path.exists(mount_path):
                            res = subprocess.run(["umount", "-f", mount_path], capture_output=True)
                            if res.returncode != 0:
                                subprocess.run(["umount", "-l", mount_path])
                    
                    if is_efi_boot:
                        res = subprocess.run(["umount", "-f", "/mnt/boot/efi"], capture_output=True)
                        if res.returncode != 0:
                            subprocess.run(["umount", "-l", "/mnt/boot/efi"])
                                
                    res = subprocess.run(["umount", "-f", "/mnt"], capture_output=True)
                    if res.returncode != 0:
                        subprocess.run(["umount", "-l", "/mnt"])
                        
                    # 3. Final recursive lazy unmount sweep to ensure absolutely nothing remains bound in the VFS
                    subprocess.run(["umount", "-f", "-l", "-R", "/mnt"], capture_output=True)
                
            is_efi = os.path.exists("/sys/firmware/efi")
            is_arch = os.path.exists("/etc/pacman.conf")
            
            # Unmount any active mounts on the selected disk first to prevent device busy errors
            if "TEST_MODE" not in os.environ:
                try:
                    with open("/proc/mounts", "r") as f:
                        for line in f:
                            parts = line.split()
                            if len(parts) >= 2 and parts[0].startswith(disk_path):
                                print(f"Unmounting busy partition: {parts[0]} from {parts[1]}")
                                subprocess.run(["umount", "-l", parts[1]], capture_output=True)
                except Exception as umount_err:
                    print(f"Warning during unmount prep: {umount_err}")
                subprocess.run(["swapoff", "-a"], capture_output=True)

            if is_efi:
                GLib.idle_add(self.update_progress, 0.05, "Cleaning and partitioning (GPT for UEFI)...")
                exec_cmd(["sgdisk", "--zap-all", disk_path])
                exec_cmd(["sgdisk", "--new=1:0:+512M", "--typecode=1:ef00", "--change-name=1:EFI", disk_path])
                exec_cmd(["sgdisk", "--new=2:0:0", "--typecode=2:8300", "--change-name=2:PulsarOS", disk_path])
                exec_cmd(["udevadm", "settle"])
                try:
                    exec_cmd(["partprobe", disk_path])
                except Exception:
                    try:
                        exec_cmd(["blockdev", "--rereadpt", disk_path])
                    except Exception:
                        pass
                
                if "nvme" in disk_path or "mmcblk" in disk_path or "loop" in disk_path:
                    efi_part = f"{disk_path}p1"
                    root_part = f"{disk_path}p2"
                else:
                    efi_part = f"{disk_path}1"
                    root_part = f"{disk_path}2"
                    
                GLib.idle_add(self.update_progress, 0.12, "Formatting partitions (EFI and ext4)...")
                exec_cmd(["mkfs.vfat", "-F32", efi_part])
                exec_cmd(["mkfs.ext4", "-F", root_part])
                
                GLib.idle_add(self.update_progress, 0.18, "Mounting file systems...")
                if "TEST_MODE" not in os.environ:
                    subprocess.run(["umount", "-l", "/mnt/boot/efi"])
                    subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", root_part, "/mnt"])
                exec_cmd(["mount", "--make-rprivate", "/mnt"])
                os.makedirs("/mnt/boot/efi", exist_ok=True)
                exec_cmd(["mount", efi_part, "/mnt/boot/efi"])
            else:
                GLib.idle_add(self.update_progress, 0.05, "Cleaning and partitioning (MBR for BIOS)...")
                exec_cmd(["dd", "if=/dev/zero", f"of={disk_path}", "bs=512", "count=1"])
                sfdisk_script = "label: dos\nsize=+, type=83, bootable\n"
                if "TEST_MODE" in os.environ:
                    print(f"[TEST_MODE] Simulating sfdisk partitioning script:\n{sfdisk_script}")
                else:
                    res_sf = subprocess.run(["sfdisk", disk_path], input=sfdisk_script, capture_output=True, text=True)
                    if res_sf.returncode != 0:
                        raise Exception(f"Failed to partition disk {disk_path} with sfdisk:\n{res_sf.stderr}")
                exec_cmd(["udevadm", "settle"])
                if "TEST_MODE" not in os.environ:
                    subprocess.run(["sfdisk", "--activate", disk_path, "1"], capture_output=True)
                try:
                    exec_cmd(["partprobe", disk_path])
                except Exception:
                    try:
                        exec_cmd(["blockdev", "--rereadpt", disk_path])
                    except Exception:
                        pass
                
                if "nvme" in disk_path or "mmcblk" in disk_path or "loop" in disk_path:
                    root_part = f"{disk_path}p1"
                else:
                    root_part = f"{disk_path}1"
                    
                GLib.idle_add(self.update_progress, 0.12, "Formatting root partition (ext4)...")
                exec_cmd(["mkfs.ext4", "-F", root_part])
                
                GLib.idle_add(self.update_progress, 0.18, "Mounting file systems...")
                if "TEST_MODE" not in os.environ:
                    subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", root_part, "/mnt"])
                exec_cmd(["mount", "--make-rprivate", "/mnt"])
            
            GLib.idle_add(self.update_progress, 0.25, "Replicating system files... (this may take a while)")
            
            if "TEST_MODE" in os.environ:
                for progress_fraction in range(26, 81):
                    GLib.idle_add(
                        self.update_progress, 
                        progress_fraction / 100.0, 
                        f"Installing system files... ({progress_fraction}%)"
                    )
                    time.sleep(0.08)
            else:
                rsync_cmd = [
                    "rsync", "-aHAXx",
                    "--exclude=/dev/*",
                    "--exclude=/proc/*",
                    "--exclude=/sys/*",
                    "--exclude=/tmp/*",
                    "--exclude=/run/*",
                    "--exclude=/mnt/*",
                    "--exclude=/media/*",
                    "--exclude=/lost+found",
                    "/", "/mnt"
                ]
                proc = subprocess.Popen(rsync_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                for progress_fraction in range(26, 81):
                    if proc.poll() is not None:
                        break
                    GLib.idle_add(
                        self.update_progress, 
                        progress_fraction / 100.0, 
                        f"Installing system files... ({progress_fraction}%)"
                    )
                    time.sleep(2)
                proc.wait()
                if proc.returncode != 0:
                    raise Exception(f"System replication failed (code {proc.returncode})\n{proc.stderr.read()}")
                
            GLib.idle_add(self.update_progress, 0.85, "Configuring bootloader (fstab)...")
            def get_partition_uuid(part):
                if "TEST_MODE" in os.environ:
                    return "simulated-uuid-1234-abcd"
                val = exec_cmd(["blkid", "-o", "value", "-s", "UUID", part])
                return val.strip()
                
            root_uuid = get_partition_uuid(root_part)
            
            if is_efi:
                efi_uuid = get_partition_uuid(efi_part)
                fstab_content = f"""# /etc/fstab: static file system information.
#
# <file system>             <mount point>   <type>  <options>       <dump>  <pass>
UUID={root_uuid} /               ext4    errors=remount-ro 0       1
UUID={efi_uuid} /boot/efi       vfat    umask=0077      0       2
"""
            else:
                fstab_content = f"""# /etc/fstab: static file system information.
#
# <file system>             <mount point>   <type>  <options>       <dump>  <pass>
UUID={root_uuid} /               ext4    errors=remount-ro 0       1
"""
            if "TEST_MODE" in os.environ:
                print(f"[TEST_MODE] Simulating writing fstab content:\n{fstab_content}")
            else:
                os.makedirs("/mnt/etc", exist_ok=True)
                with open("/mnt/etc/fstab", "w") as f:
                    f.write(fstab_content)
                
            GLib.idle_add(self.update_progress, 0.90, "Installing GRUB bootloader...")
            exec_cmd(["mount", "--bind", "/dev", "/mnt/dev"])
            if "TEST_MODE" not in os.environ:
                os.makedirs("/mnt/dev/pts", exist_ok=True)
            exec_cmd(["mount", "-t", "devpts", "devpts", "/mnt/dev/pts"])
            exec_cmd(["mount", "--bind", "/proc", "/mnt/proc"])
            exec_cmd(["mount", "--bind", "/sys", "/mnt/sys"])
            exec_cmd(["mount", "-t", "tmpfs", "tmpfs", "/mnt/run"])
            
            if is_efi:
                # Run grub-install twice:
                # 1. Without --removable to register the UEFI NVRAM boot entry (so the UEFI firmware boots the hard drive by default)
                try:
                    exec_cmd(["chroot", "/mnt", "grub-install", "--force", disk_path])
                except Exception as g_err:
                    print(f"Warning: Standard grub-install failed: {g_err}. Proceeding with removable fallback...")
                # 2. With --removable to create fallback loader in /EFI/BOOT/BOOTX64.EFI
                exec_cmd(["chroot", "/mnt", "grub-install", "--force", "--removable", disk_path])
                refind_postinst = "/mnt/var/lib/dpkg/info/pulsaros-refind.postinst"
                if os.path.exists(refind_postinst) or "TEST_MODE" in os.environ:
                    GLib.idle_add(self.update_progress, 0.92, "Configuring rEFInd dual-boot bootloader...")
                    try:
                        exec_cmd(["chroot", "/mnt", "/var/lib/dpkg/info/pulsaros-refind.postinst", "configure"])
                    except Exception as ref_err:
                        print(f"Warning: rEFInd dual-boot setup encountered an issue: {ref_err}. Falling back to GRUB.")
            else:
                exec_cmd(["chroot", "/mnt", "grub-install", "--target=i386-pc", "--force", disk_path])
                
            if is_arch:
                # The ISO live rootfs carries live-only boot artifacts that must
                # be removed on a fixed-disk install:
                #  - /etc/mkinitcpio.conf.d/archiso.conf: enables the 'archiso'
                #    hooks so the initramfs boots from the ISO. If left in place
                #    the installed system waits 30s for the ISO device and drops
                #    to an emergency shell instead of mounting the root partition.
                #  - GRUB_DISTRIBUTOR="Arch" in /etc/default/grub makes GRUB show
                #    "Arch Linux" instead of "Pulsar OS".
                live_conf = "/mnt/etc/mkinitcpio.conf.d/archiso.conf"
                if os.path.exists(live_conf):
                    os.remove(live_conf)
                grub_default = "/mnt/etc/default/grub"
                if os.path.exists(grub_default):
                    with open(grub_default) as f:
                        content = f.read()
                    content = re.sub(
                        r"^#?\s*GRUB_DISTRIBUTOR=.*$",
                        'GRUB_DISTRIBUTOR="Pulsar OS"',
                        content,
                        flags=re.MULTILINE,
                    )
                    with open(grub_default, "w") as f:
                        f.write(content)
                # Regenerate the initramfs with the standard (non-live) hooks
                exec_cmd(["chroot", "/mnt", "mkinitcpio", "-P"])
                exec_cmd(["chroot", "/mnt", "grub-mkconfig", "-o", "/boot/grub/grub.cfg"])
            else:
                exec_cmd(["chroot", "/mnt", "update-grub"])

            # ── Driver installation ────────────────────────────────────
            if self.install_nvidia or self.install_broadcom:
                # Bind network-related paths so package manager can reach the internet
                exec_cmd(["mount", "--bind", "/etc/resolv.conf", "/mnt/etc/resolv.conf"])
                policy_file = "/mnt/usr/sbin/policy-rc.d"
                try:
                    if is_arch:
                        # Arch Linux path
                        GLib.idle_add(self.update_progress, 0.93, "Updating package sources...")
                        exec_cmd(["chroot", "/mnt", "pacman", "-Sy"])
                        
                        if self.install_nvidia:
                            GLib.idle_add(self.update_progress, 0.94, "Installing NVIDIA drivers...")
                            exec_cmd(["chroot", "/mnt", "pacman", "-S", "--noconfirm",
                                      "nvidia", "nvidia-utils", "linux-headers"])
                            
                        if self.install_broadcom:
                            GLib.idle_add(self.update_progress, 0.95, "Installing Broadcom drivers...")
                            exec_cmd(["chroot", "/mnt", "pacman", "-S", "--noconfirm",
                                      "broadcom-wl-dkms", "linux-headers"])
                            # blacklist conflicting drivers
                            blacklist = (
                                "blacklist b43\n"
                                "blacklist b43legacy\n"
                                "blacklist ssb\n"
                                "blacklist bcm43xx\n"
                                "blacklist brcm80211\n"
                                "blacklist brcmfmac\n"
                                "blacklist brcmsmac\n"
                            )
                            if "TEST_MODE" not in os.environ:
                                os.makedirs("/mnt/etc/modprobe.d", exist_ok=True)
                                with open("/mnt/etc/modprobe.d/broadcom-sta-blacklist.conf", "w") as f:
                                    f.write(blacklist)
                    else:
                        # Debian path
                        try:
                            # Create policy-rc.d to prevent service start in chroot
                            if "TEST_MODE" not in os.environ:
                                os.makedirs(os.path.dirname(policy_file), exist_ok=True)
                                with open(policy_file, "w") as f:
                                    f.write("#!/bin/sh\nexit 101\n")
                                os.chmod(policy_file, 0o755)

                            # Enable non-free repositories on target sources.list
                            sources_file = "/mnt/etc/apt/sources.list"
                            if "TEST_MODE" not in os.environ and os.path.exists(sources_file):
                                try:
                                    with open(sources_file, "r") as f:
                                        content = f.read()
                                    modified = False
                                    lines = []
                                    for line in content.splitlines():
                                        s_line = line.strip()
                                        if s_line and not s_line.startswith("#") and "main" in s_line:
                                            for comp in ["contrib", "non-free", "non-free-firmware"]:
                                                if comp not in s_line:
                                                    line += f" {comp}"
                                                    modified = True
                                        lines.append(line)
                                    if modified:
                                        with open(sources_file, "w") as f:
                                            f.write("\n".join(lines) + "\n")
                                except Exception as list_err:
                                    print(f"Warning: Failed to update sources.list: {list_err}")

                            # Run apt update to fetch package indices
                            GLib.idle_add(self.update_progress, 0.93, "Updating package sources...")
                            exec_cmd(["chroot", "/mnt", "apt-get", "update"])
                            
                            if self.install_nvidia:
                                nvidia = self.nvidia_info
                                is_new = nvidia.get("is_new", False)
                                GLib.idle_add(self.update_progress, 0.94, "Installing NVIDIA drivers...")
                                if is_new:
                                    # Turing / Ampere / Ada / Blackwell → nvidia-driver (current)
                                    exec_cmd(["chroot", "/mnt", "apt-get", "install", "-y",
                                              "nvidia-driver", "nvidia-settings",
                                              "linux-headers-amd64"])
                                else:
                                    # Kepler / Maxwell / Pascal and older → legacy 470 series
                                    exec_cmd(["chroot", "/mnt", "apt-get", "install", "-y",
                                              "nvidia-tesla-470-driver", "nvidia-settings",
                                              "linux-headers-amd64"])

                            if self.install_broadcom:
                                GLib.idle_add(self.update_progress, 0.95, "Installing Broadcom drivers...")
                                exec_cmd(["chroot", "/mnt", "apt-get", "install", "-y",
                                          "broadcom-sta-dkms", "linux-headers-amd64"])
                                # blacklist conflicting drivers
                                blacklist = (
                                    "blacklist b43\n"
                                    "blacklist b43legacy\n"
                                    "blacklist ssb\n"
                                    "blacklist bcm43xx\n"
                                    "blacklist brcm80211\n"
                                    "blacklist brcmfmac\n"
                                    "blacklist brcmsmac\n"
                                )
                                if "TEST_MODE" not in os.environ:
                                    os.makedirs("/mnt/etc/modprobe.d", exist_ok=True)
                                    with open("/mnt/etc/modprobe.d/broadcom-sta-blacklist.conf", "w") as f:
                                        f.write(blacklist)
                        except Exception as deb_err:
                            raise deb_err
                        finally:
                            # Remove policy-rc.d
                            if "TEST_MODE" not in os.environ and os.path.exists(policy_file):
                                try:
                                    os.remove(policy_file)
                                except Exception:
                                    pass
                except Exception as drv_err:
                    print(f"Warning: Driver installation failed: {drv_err}")
                finally:
                    subprocess.run(["umount", "-l", "/mnt/etc/resolv.conf"])
            # ──────────────────────────────────────────────────────────
            
            # 1. Touch the setup flag file first while everything is still mounted cleanly
            GLib.idle_add(self.update_progress, 0.95, "Creating setup flag...")
            if "TEST_MODE" not in os.environ:
                try:
                    os.makedirs("/mnt/etc", exist_ok=True)
                    with open("/mnt/etc/pulsar-need-setup", "w") as f:
                        pass
                except Exception as touch_err:
                    print(f"Warning: Failed to create setup flag: {touch_err}")

            # 2. Cleanup and unmount all filesystems
            cleanup_mounts(is_efi)
            
            GLib.idle_add(self.on_installation_completed)
            
        except Exception as err:
            log_msg(f"INSTALLATION FAILED: {err}")
            cleanup_mounts(is_efi)
            GLib.idle_add(self.on_installation_failed, str(err))
        finally:
            # Restart udisks2 automount daemon so the system goes back to normal state
            if "TEST_MODE" not in os.environ:
                subprocess.run(["systemctl", "start", "udisks2.service"], capture_output=True)

    def on_installation_completed(self):
        self.update_progress(1.0, "Pulsar OS has been successfully installed!")
        self.btn_install_action.set_label("Restart System")
        self.btn_install_action.add_css_class("suggested-action")
        self.btn_install_action.remove_css_class("secondary-action")

    def on_installation_failed(self, error):
        self.update_progress(0.0, "Installation failed.")
        self.show_error_dialog(error)
        self.stack.set_visible_child_name("install_disk_select")

    def on_guided_install_clicked(self, btn):
        if "TEST_MODE" in os.environ:
            print("[TEST_MODE] Simulating Calamares installer GUI launcher...")
            self.close()
        else:
            installer_cmd = ["/usr/local/bin/launch-calamares"]
            if not os.path.exists(installer_cmd[0]):
                installer_cmd = ["/usr/bin/launch-calamares"]
            if not os.path.exists(installer_cmd[0]):
                installer_cmd = ["sudo", "calamares", "-platform", "xcb"]
            subprocess.Popen(installer_cmd)
            self.close()

    def show_error_dialog(self, message):
        dialog = Gtk.MessageDialog(
            transient_for=self,
            flags=Gtk.DialogFlags.MODAL,
            message_type=Gtk.MessageType.ERROR,
            buttons=Gtk.ButtonsType.NONE,
            text="Installation Error"
        )
        dialog.format_secondary_text(message)
        dialog.add_button("OK", Gtk.ResponseType.OK)
        dialog.add_button("View Log", Gtk.ResponseType.HELP)
        
        def on_response(d, response_id):
            if response_id == Gtk.ResponseType.HELP:
                subprocess.Popen(["xdg-open", "/tmp/pulsaros-install.log"])
            d.destroy()
            
        dialog.connect("response", on_response)
        dialog.present()


class RecoveryApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id="es.inled.pulsaros.recovery",
            flags=Gio.ApplicationFlags.FLAGS_NONE
        )

    def do_activate(self):
        win = RecoveryWindow(self)
        win.present()


if __name__ == "__main__":
    app = RecoveryApp()
    sys.exit(app.run(sys.argv))
