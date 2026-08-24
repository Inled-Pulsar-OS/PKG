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
import glob
import shutil
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
    min-height: 6px;
    margin-top: 12px;
    margin-bottom: 12px;
}
progressbar.progress-bar-thin trough,
.progress-bar-thin trough {
    min-height: 6px;
    border-radius: 9999px;
    background-color: #3a3a3c;
    border: none;
}
progressbar.progress-bar-thin progress,
.progress-bar-thin progress {
    min-height: 6px;
    border-radius: 9999px;
    background-color: #0071e3;
    border: none;
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
.error-icon {
    color: #ff453a;
}
.error-log-view {
    background-color: #121212;
    border: 1px solid #3c3c3c;
    border-radius: 8px;
    padding: 8px;
}
textview.error-log-text {
    background-color: #121212;
}
textview.error-log-text text {
    background-color: #121212;
    color: #ff453a;
    font-size: 11px;
}
.target-disk-box {
    margin-top: 6px;
    margin-bottom: 6px;
}
.target-disk-name {
    font-size: 12px;
    font-weight: 600;
    color: #e5e5ea;
    text-align: center;
}
.terminal-btn {
    background-color: transparent;
    border: none;
    border-radius: 8px;
    padding: 8px 10px;
    color: #8e8e93;
    opacity: 0.7;
    transition: all 0.2s ease;
}
.terminal-btn:hover {
    background-color: rgba(255, 255, 255, 0.12);
    color: #ffffff;
    opacity: 1.0;
}
.log-window {
    background-color: #1a1a1a;
}
.live-log-view {
    background-color: #121212;
    border: 1px solid #333333;
    border-radius: 8px;
    padding: 10px;
}
textview.live-log-text {
    background-color: #121212;
}
textview.live-log-text text {
    background-color: #121212;
    color: #30d158;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 11px;
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


class InstallerLogWindow(Adw.Window):
    def __init__(self, parent_window):
        super().__init__()
        self.parent_win = parent_window
        self.set_transient_for(parent_window)
        self.set_title("Installer Log")
        self.set_default_size(680, 420)
        self.set_modal(False)
        self.add_css_class("log-window")
        
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        
        header = Adw.HeaderBar()
        header.set_show_end_title_buttons(True)
        title_widget = Adw.WindowTitle(title="Pulsar OS Installer Log", subtitle="Live installation output")
        header.set_title_widget(title_widget)
        main_box.append(header)
        
        content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        content_box.set_margin_top(10)
        content_box.set_margin_bottom(12)
        content_box.set_margin_start(14)
        content_box.set_margin_end(14)
        content_box.set_hexpand(True)
        content_box.set_vexpand(True)
        
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_hexpand(True)
        scrolled.set_vexpand(True)
        scrolled.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        scrolled.add_css_class("live-log-view")
        
        self.text_view = Gtk.TextView()
        self.text_view.set_editable(False)
        self.text_view.set_monospace(True)
        self.text_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self.text_view.add_css_class("live-log-text")
        
        self.buffer = self.text_view.get_buffer()
        scrolled.set_child(self.text_view)
        content_box.append(scrolled)
        
        main_box.append(content_box)
        self.set_content(main_box)
        
        self.last_pos = 0
        self.poll_log()
        self.timer_id = GLib.timeout_add(300, self.poll_log)
        self.connect("close-request", self.on_close)
        
    def poll_log(self):
        log_path = "/tmp/pulsaros-install.log"
        if os.path.exists(log_path):
            try:
                with open(log_path, "r", encoding="utf-8", errors="replace") as f:
                    f.seek(self.last_pos)
                    new_text = f.read()
                    if new_text:
                        self.last_pos = f.tell()
                        iter_end = self.buffer.get_end_iter()
                        self.buffer.insert(iter_end, new_text)
                        
                        # Auto-scroll to end
                        mark = self.buffer.create_mark(None, self.buffer.get_end_iter(), False)
                        self.text_view.scroll_to_mark(mark, 0.0, True, 0.0, 1.0)
            except Exception:
                pass
        return True

    def on_close(self, *args):
        if self.timer_id:
            GLib.source_remove(self.timer_id)
            self.timer_id = None
        if hasattr(self.parent_win, 'log_window'):
            self.parent_win.log_window = None
        return False


class RecoveryWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Pulsar OS Recovery")
        self.set_default_size(720, 560)
        self.set_resizable(True)
        
        self.apply_css()
        
        # Always fullscreen (except in test mode)
        if "TEST_MODE" not in os.environ:
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
        self.build_install_error_screen()
        
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
            if not icon_path or not os.path.exists(icon_path) or os.path.getsize(icon_path) < 100:
                for candidate in [
                    "/usr/share/icons/safari.png",
                    "/usr/share/pixmaps/safari.png",
                    "/usr/share/icons/hicolor/scalable/apps/org.gnome.Epiphany.svg",
                    "/usr/share/icons/hicolor/48x48/apps/safari.png",
                ]:
                    if os.path.exists(candidate):
                        icon_path = candidate
                        break
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
            subprocess.Popen("timeshift-launcher || pkexec timeshift-gtk || timeshift-gtk || deja-dup --restore || deja-dup", shell=True)
        elif self.selected_action == "install":
            self.show_installer_selector_dialog()
        elif self.selected_action == "safari":
            self._popen_as_user("seafari || epiphany || firefox")
        elif self.selected_action == "disk":
            subprocess.Popen("gparted || pkexec gparted || gnome-disks || gnome-disk-utility", shell=True)


    def show_installer_selector_dialog(self):
        # Emergent selector popup matching Apple design
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="Install Pulsar OS",
            body="Choose the installation method you want to use for your computer."
        )
        dialog.add_response("quick", "MacOS like UI (recommended)")
        dialog.add_response("guided", "Dual boot and more reliable (calamares)")
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
        self.install_broadcom = False
        self._show_broadcom_dialog()

    def build_install_progress_screen(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        image = self.get_logo_image(90, is_installer=True)
        box.append(image)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='18000'>Pulsar OS</span>")
        box.append(title)
        
        self.progress_subtitle = Gtk.Label(label="Pulsar OS will be installed on the selected disk.")
        self.progress_subtitle.add_css_class("progress-text")
        box.append(self.progress_subtitle)
        
        # Target disk visual box (Apple-style disk icon + disk name between description and progress bar)
        self.target_disk_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=3)
        self.target_disk_box.add_css_class("target-disk-box")
        self.target_disk_box.set_halign(Gtk.Align.CENTER)
        
        self.target_disk_icon = Gtk.Image.new_from_icon_name("drive-harddisk")
        self.target_disk_icon.set_pixel_size(42)
        self.target_disk_box.append(self.target_disk_icon)
        
        self.target_disk_name_lbl = Gtk.Label(label="Target Disk")
        self.target_disk_name_lbl.add_css_class("target-disk-name")
        self.target_disk_box.append(self.target_disk_name_lbl)
        
        box.append(self.target_disk_box)
        
        self.progress_bar = Gtk.ProgressBar()
        self.progress_bar.add_css_class("progress-bar-thin")
        self.progress_bar.set_size_request(280, -1)
        box.append(self.progress_bar)
        
        self.progress_label = Gtk.Label(label="Preparing installation...")
        self.progress_label.add_css_class("progress-text")
        box.append(self.progress_label)
        
        # Bottom controls row (Cancel button centered, flat terminal log button on bottom right)
        bottom_row = Gtk.CenterBox()
        bottom_row.set_margin_top(8)
        bottom_row.set_size_request(340, -1)
        
        self.btn_install_action = Gtk.Button(label="Cancel")
        self.btn_install_action.add_css_class("secondary-action")
        self.btn_install_action.set_size_request(130, -1)
        self.btn_install_action.connect("clicked", self.on_progress_cancel_clicked)
        bottom_row.set_center_widget(self.btn_install_action)
        
        self.btn_log = Gtk.Button()
        self.btn_log.set_icon_name("utilities-terminal-symbolic")
        self.btn_log.set_tooltip_text("Show Installer Log")
        self.btn_log.add_css_class("terminal-btn")
        self.btn_log.connect("clicked", self.on_show_live_log_clicked)
        bottom_row.set_end_widget(self.btn_log)
        
        box.append(bottom_row)
        
        self.stack.add_named(box, "install_progress")

    def build_install_error_screen(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        box.set_size_request(460, -1)
        
        # Error icon
        icon = Gtk.Image.new_from_icon_name("dialog-error")
        icon.set_pixel_size(72)
        icon.add_css_class("error-icon")
        box.append(icon)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='18000' color='#ff453a'>Installation Failed</span>")
        box.append(title)
        
        desc = Gtk.Label(label="An error occurred during the installation process.")
        desc.add_css_class("progress-text")
        box.append(desc)
        
        # Scrolled window for log viewer
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_min_content_height(160)
        scrolled.set_min_content_width(420)
        scrolled.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        scrolled.add_css_class("error-log-view")
        
        self.error_log_text = Gtk.TextView()
        self.error_log_text.set_editable(False)
        self.error_log_text.set_monospace(True)
        self.error_log_text.set_wrap_mode(Gtk.WrapMode.WORD)
        self.error_log_text.add_css_class("error-log-text")
        
        scrolled.set_child(self.error_log_text)
        box.append(scrolled)
        
        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        nav_box.set_halign(Gtk.Align.CENTER)
        nav_box.set_margin_top(12)
        
        btn_back = Gtk.Button(label="Back to Disk Select")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("install_disk_select"))
        nav_box.append(btn_back)
        
        btn_reboot = Gtk.Button(label="Restart System")
        btn_reboot.add_css_class("suggested-action")
        btn_reboot.connect("clicked", self.on_progress_cancel_clicked)
        nav_box.append(btn_reboot)
        
        box.append(nav_box)
        self.stack.add_named(box, "install_error")

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

    def _show_broadcom_dialog(self):
        auto_detected = self._detect_broadcom()
        if auto_detected:
            heading = "Broadcom Hardware Detected"
            body = (
                "A Broadcom WiFi or Bluetooth adapter was detected on this computer.\n\n"
                "Would you like to install the Broadcom wireless driver (<tt>broadcom-wl / broadcom-sta-dkms</tt>)?\n\n"
                "⚠️ <b>Active Internet connection required</b>:\n"
                "An active Internet connection (Ethernet cable or USB tethering) is required during installation to download and compile the driver.\n\n"
                "If you do not have Internet access right now, choose \"No\" (you can install it later with Driver Manager)."
            )
        else:
            heading = "Broadcom Wireless Drivers"
            body = (
                "No Broadcom adapter was automatically detected.\n\n"
                "Do you have a Broadcom WiFi or Bluetooth chip? (Common in older MacBooks and select laptops).\n\n"
                "⚠️ <b>Active Internet connection required</b>:\n"
                "An active Internet connection (Ethernet cable or USB tethering) is required to download the driver.\n\n"
                "If unsure or without Internet, choose \"No\"."
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
        self.progress_subtitle.set_label("Pulsar OS will be installed on the selected disk.")
        
        display_name = disk_name
        if hasattr(self, 'selected_disk_card') and self.selected_disk_card:
            name_info = self.selected_disk_card.disk_info.get("name", "")
            size_match = re.search(r"\(([^)]+)\)", name_info)
            if size_match:
                display_name = f"Pulsar OS ({disk_name} • {size_match.group(1)})"
            else:
                display_name = f"Pulsar OS ({disk_name})"
        else:
            display_name = f"Pulsar OS ({disk_name})"
            
        self.target_disk_name_lbl.set_label(display_name)
        self.stack.set_visible_child_name("install_progress")
        threading.Thread(
            target=self.installation_backend,
            args=(disk_path,),
            daemon=True
        ).start()

    def on_show_live_log_clicked(self, btn):
        if not hasattr(self, 'log_window') or self.log_window is None:
            self.log_window = InstallerLogWindow(self)
        self.log_window.present()

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
                        "/mnt/recovery",
                        "/mnt/boot/efi",
                        "/mnt/home",
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
            esp_root = "/mnt/boot/efi"
            
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
                GLib.idle_add(self.update_progress, 0.05, "Cleaning and partitioning (GPT for UEFI: EFI, Recovery, Btrfs)...")
                exec_cmd(["wipefs", "-a", "-f", disk_path])
                exec_cmd(["sgdisk", "--zap-all", disk_path])
                exec_cmd(["sgdisk", "--clear", disk_path])
                exec_cmd(["sgdisk", "--new=1:0:+512M", "--typecode=1:ef00", "--change-name=1:EFI", disk_path])
                exec_cmd(["sgdisk", "--new=2:0:+4G", "--typecode=2:8300", "--change-name=2:PulsarRecovery", disk_path])
                exec_cmd(["sgdisk", "--new=3:0:0", "--typecode=3:8300", "--change-name=3:PulsarOS", disk_path])
                exec_cmd(["sync"])
                exec_cmd(["udevadm", "settle"])
                try:
                    exec_cmd(["partprobe", disk_path])
                except Exception:
                    try:
                        exec_cmd(["blockdev", "--rereadpt", disk_path])
                    except Exception:
                        pass
                exec_cmd(["udevadm", "settle"])
                time.sleep(1)
                
                if "nvme" in disk_path or "mmcblk" in disk_path or "loop" in disk_path:
                    efi_part = f"{disk_path}p1"
                    recovery_part = f"{disk_path}p2"
                    root_part = f"{disk_path}p3"
                else:
                    efi_part = f"{disk_path}1"
                    recovery_part = f"{disk_path}2"
                    root_part = f"{disk_path}3"
                    
                exec_cmd(["wipefs", "-a", "-f", efi_part])
                exec_cmd(["wipefs", "-a", "-f", recovery_part])
                exec_cmd(["wipefs", "-a", "-f", root_part])
                
                GLib.idle_add(self.update_progress, 0.10, "Formatting partitions (EFI, Recovery, Btrfs)...")
                exec_cmd(["mkfs.vfat", "-F32", "-n", "EFI", efi_part])
                exec_cmd(["mkfs.ext4", "-F", "-F", "-L", "PULSAR_RECOVERY", recovery_part])
                exec_cmd(["mkfs.btrfs", "-f", "-L", "PULSAR_OS", root_part])
                exec_cmd(["sync"])
                exec_cmd(["udevadm", "settle"])
                time.sleep(1)
                
                subprocess.run(["modprobe", "btrfs"], capture_output=True)
                subprocess.run(["modprobe", "ext4"], capture_output=True)
                subprocess.run(["modprobe", "vfat"], capture_output=True)
                
                GLib.idle_add(self.update_progress, 0.15, "Creating Btrfs subvolumes (@ and @home)...")
                if "TEST_MODE" not in os.environ:
                    subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", "-t", "btrfs", root_part, "/mnt"])
                exec_cmd(["btrfs", "subvolume", "create", "/mnt/@"])
                exec_cmd(["btrfs", "subvolume", "create", "/mnt/@home"])
                exec_cmd(["umount", "/mnt"])
                
                GLib.idle_add(self.update_progress, 0.18, "Mounting Btrfs subvolumes...")
                exec_cmd(["mount", "-t", "btrfs", "-o", "subvol=@,compress=zstd:1", root_part, "/mnt"])
                exec_cmd(["mount", "--make-rprivate", "/mnt"])
                os.makedirs("/mnt/home", exist_ok=True)
                exec_cmd(["mount", "-t", "btrfs", "-o", "subvol=@home,compress=zstd:1", root_part, "/mnt/home"])
                os.makedirs("/mnt/boot/efi", exist_ok=True)
                exec_cmd(["mount", "-t", "vfat", efi_part, "/mnt/boot/efi"])
                os.makedirs("/mnt/recovery", exist_ok=True)
                exec_cmd(["mount", "-t", "ext4", recovery_part, "/mnt/recovery"])
            else:
                GLib.idle_add(self.update_progress, 0.05, "Cleaning and partitioning (MBR for BIOS: Recovery, Btrfs)...")
                exec_cmd(["wipefs", "-a", "-f", disk_path])
                exec_cmd(["dd", "if=/dev/zero", f"of={disk_path}", "bs=512", "count=2048"])
                sfdisk_script = "label: dos\nsize=4096M, type=83\nsize=+, type=83, bootable\n"
                if "TEST_MODE" in os.environ:
                    print(f"[TEST_MODE] Simulating sfdisk partitioning script:\n{sfdisk_script}")
                else:
                    res_sf = subprocess.run(["sfdisk", disk_path], input=sfdisk_script, capture_output=True, text=True)
                    if res_sf.returncode != 0:
                        raise Exception(f"Failed to partition disk {disk_path} with sfdisk:\n{res_sf.stderr}")
                exec_cmd(["sync"])
                exec_cmd(["udevadm", "settle"])
                if "TEST_MODE" not in os.environ:
                    subprocess.run(["sfdisk", "--activate", disk_path, "2"], capture_output=True)
                try:
                    exec_cmd(["partprobe", disk_path])
                except Exception:
                    try:
                        exec_cmd(["blockdev", "--rereadpt", disk_path])
                    except Exception:
                        pass
                exec_cmd(["udevadm", "settle"])
                time.sleep(1)
                
                if "nvme" in disk_path or "mmcblk" in disk_path or "loop" in disk_path:
                    recovery_part = f"{disk_path}p1"
                    root_part = f"{disk_path}p2"
                else:
                    recovery_part = f"{disk_path}1"
                    root_part = f"{disk_path}2"
                    
                exec_cmd(["wipefs", "-a", "-f", recovery_part])
                exec_cmd(["wipefs", "-a", "-f", root_part])
                GLib.idle_add(self.update_progress, 0.10, "Formatting Recovery and Btrfs partitions...")
                exec_cmd(["mkfs.ext4", "-F", "-F", "-L", "PULSAR_RECOVERY", recovery_part])
                exec_cmd(["mkfs.btrfs", "-f", "-L", "PULSAR_OS", root_part])
                exec_cmd(["sync"])
                exec_cmd(["udevadm", "settle"])
                time.sleep(1)
                
                subprocess.run(["modprobe", "btrfs"], capture_output=True)
                subprocess.run(["modprobe", "ext4"], capture_output=True)
                
                GLib.idle_add(self.update_progress, 0.15, "Creating Btrfs subvolumes (@ and @home)...")
                if "TEST_MODE" not in os.environ:
                    subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", "-t", "btrfs", root_part, "/mnt"])
                exec_cmd(["btrfs", "subvolume", "create", "/mnt/@"])
                exec_cmd(["btrfs", "subvolume", "create", "/mnt/@home"])
                exec_cmd(["umount", "/mnt"])
                
                GLib.idle_add(self.update_progress, 0.18, "Mounting Btrfs subvolumes...")
                exec_cmd(["mount", "-t", "btrfs", "-o", "subvol=@,compress=zstd:1", root_part, "/mnt"])
                exec_cmd(["mount", "--make-rprivate", "/mnt"])
                os.makedirs("/mnt/home", exist_ok=True)
                exec_cmd(["mount", "-t", "btrfs", "-o", "subvol=@home,compress=zstd:1", root_part, "/mnt/home"])
                os.makedirs("/mnt/recovery", exist_ok=True)
                exec_cmd(["mount", "-t", "ext4", recovery_part, "/mnt/recovery"])
            
            GLib.idle_add(self.update_progress, 0.25, "Replicating system files... (this may take a while)")
            
            if "TEST_MODE" in os.environ:
                for progress_fraction in range(26, 81):
                    if "SIMULATE_INSTALL_ERROR" in os.environ and progress_fraction >= 45:
                        log_msg("ERROR: Simulated disk read/write error at sector 0x4f32a7b8.")
                        log_msg("ERROR: System replication failed: failed to copy /usr/lib/libgtk-4.so.")
                        raise Exception("Simulated disk read/write error: Sector 0x4f32a7b8 is corrupt. System replication failed to copy /usr/lib/libgtk-4.so.")
                    GLib.idle_add(
                        self.update_progress, 
                        progress_fraction / 100.0, 
                        f"Installing system files... ({progress_fraction}%)"
                    )
                    time.sleep(0.08)
            else:
                rsync_cmd = [
                    "rsync", "-aHAXx",
                    "--info=progress2",
                    "--exclude=/dev/*",
                    "--exclude=/proc/*",
                    "--exclude=/sys/*",
                    "--exclude=/tmp/*",
                    "--exclude=/run/*",
                    "--exclude=/mnt/*",
                    "--exclude=/media/*",
                    "--exclude=/lost+found",
                    "--exclude=/var/tmp/*",
                    "--exclude=/var/log/*",
                    "--exclude=/home/*/.local/share/gvfs-metadata/*",
                    "--exclude=/home/*/.cache/*",
                    "--exclude=/root/.cache/*",
                    "/", "/mnt"
                ]
                proc = subprocess.Popen(rsync_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
                buffer = ""
                while True:
                    char = proc.stdout.read(1)
                    if not char:
                        break
                    if char in ('\r', '\n'):
                        line = buffer.strip()
                        buffer = ""
                        m = re.search(r"(\d+)%\s+([\d\.]+[KMGT]?B/s)", line)
                        if m:
                            pct = int(m.group(1))
                            speed = m.group(2)
                            frac = 0.25 + (pct / 100.0) * 0.55
                            GLib.idle_add(
                                self.update_progress, 
                                frac, 
                                f"Copying files: {pct}% at {speed}"
                            )
                    else:
                        buffer += char

                proc.wait()
                # Exit code 24 = vanished source files during transfer (normal for running live system)
                if proc.returncode not in (0, 24):
                    err_output = proc.stderr.read()
                    raise Exception(f"System replication failed (code {proc.returncode})\n{err_output}")
                
            # Populate Recovery Partition with dedicated Debian Recovery image, clean base image, and assistant.
            try:
                os.makedirs("/mnt/recovery/images/x86_64", exist_ok=True)
                os.makedirs("/mnt/recovery/live", exist_ok=True)
                os.makedirs("/mnt/recovery/boot", exist_ok=True)

                # 1. Recovery OS SquashFS (Debian + Fluxbox + Rust Assistant)
                rec_squash_sources = [
                    "/recovery/filesystem.squashfs",
                    "/usr/share/pulsaros-recovery/recovery-filesystem.squashfs",
                    "/mnt/usr/share/pulsaros-recovery/recovery-filesystem.squashfs",
                    "/run/archiso/bootmnt/recovery/filesystem.squashfs",
                    "/run/live/medium/recovery/filesystem.squashfs",
                    "/lib/live/mount/medium/recovery/filesystem.squashfs",
                    "/run/live/medium/live/filesystem.squashfs",
                    "/lib/live/mount/medium/live/filesystem.squashfs",
                    "/run/archiso/bootmnt/live/x86_64/airootfs.sfs",
                    "/run/archiso/bootmnt/live/filesystem.squashfs",
                    "/run/archiso/airootfs.sfs",
                    "/live/filesystem.squashfs",
                ]
                found_rec_squash = next((p for p in rec_squash_sources if os.path.isfile(p)), None)
                if found_rec_squash and "TEST_MODE" not in os.environ:
                    deb_dst = "/mnt/recovery/live/filesystem.squashfs"
                    shutil.copy2(found_rec_squash, deb_dst)
                    try:
                        shutil.copy2(found_rec_squash, "/mnt/recovery/filesystem.squashfs")
                    except Exception:
                        pass
                    try:
                        os.makedirs("/mnt/live", exist_ok=True)
                        shutil.copy2(found_rec_squash, "/mnt/live/filesystem.squashfs")
                    except Exception:
                        pass
                    arch_dst = "/mnt/recovery/images/x86_64/airootfs.sfs"
                    if not os.path.exists(arch_dst):
                        try:
                            os.link(deb_dst, arch_dst)
                        except Exception:
                            shutil.copy2(deb_dst, arch_dst)
                    log_msg(f"Recovery OS squashfs deployed from {found_rec_squash} -> {deb_dst}")

                # 2. Base System SquashFS (for restoring root @ subvolume)
                base_squash_sources = [
                    "/run/archiso/bootmnt/images/pulsaros-base.squashfs",
                    "/run/live/medium/images/pulsaros-base.squashfs",
                    "/recovery/images/pulsaros-base.squashfs",
                    "/run/archiso/bootmnt/live/x86_64/airootfs.sfs",
                    "/run/archiso/bootmnt/live/filesystem.squashfs",
                    "/run/live/medium/live/filesystem.squashfs",
                    "/lib/live/mount/medium/live/filesystem.squashfs",
                    "/live/filesystem.squashfs",
                ]
                found_base_squash = next((p for p in base_squash_sources if os.path.isfile(p)), None)
                if found_base_squash and "TEST_MODE" not in os.environ:
                    base_dst = "/mnt/recovery/images/pulsaros-base.squashfs"
                    if found_base_squash != deb_dst:
                        shutil.copy2(found_base_squash, base_dst)
                    else:
                        try:
                            os.link(deb_dst, base_dst)
                        except Exception:
                            shutil.copy2(deb_dst, base_dst)
                    log_msg(f"Base system restoration image deployed from {found_base_squash} -> {base_dst}")
            except Exception as rec_copy_err:
                print(f"Notice: Recovery squashfs copy: {rec_copy_err}")

            GLib.idle_add(self.update_progress, 0.85, "Configuring bootloader (fstab)...")
            def get_partition_uuid(part):
                if "TEST_MODE" in os.environ:
                    return "simulated-uuid-1234-abcd"
                val = exec_cmd(["blkid", "-o", "value", "-s", "UUID", part])
                return val.strip()
                
            root_uuid = get_partition_uuid(root_part)
            rec_uuid = get_partition_uuid(recovery_part)
            
            if is_efi:
                efi_uuid = get_partition_uuid(efi_part)
                fstab_content = f"""# /etc/fstab: Pulsar OS Btrfs Configuration
# <file system>             <mount point>   <type>  <options>                                       <dump>  <pass>
UUID={root_uuid}            /               btrfs   subvol=@,compress=zstd:1,space_cache=v2         0       0
UUID={root_uuid}            /home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2     0       0
UUID={efi_uuid}             /boot/efi       vfat    umask=0077                                      0       2
UUID={rec_uuid}             /recovery       ext4    defaults,noatime                                0       2
"""
                if "TEST_MODE" not in os.environ:
                    os.makedirs("/mnt/etc", exist_ok=True)
                    os.makedirs("/mnt/dev", exist_ok=True)
                    os.makedirs("/mnt/proc", exist_ok=True)
                    os.makedirs("/mnt/sys", exist_ok=True)
                    os.makedirs("/mnt/run", exist_ok=True)
                    with open("/mnt/etc/fstab", "w") as f:
                        f.write(fstab_content)
            else:
                fstab_content = f"""# /etc/fstab: Pulsar OS Btrfs Configuration (BIOS)
# <file system>             <mount point>   <type>  <options>                                       <dump>  <pass>
UUID={root_uuid}            /               btrfs   subvol=@,compress=zstd:1,space_cache=v2         0       0
UUID={root_uuid}            /home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2     0       0
UUID={rec_uuid}             /recovery       ext4    defaults,noatime                                0       2
"""
                if "TEST_MODE" not in os.environ:
                    os.makedirs("/mnt/etc", exist_ok=True)
                    os.makedirs("/mnt/dev", exist_ok=True)
                    os.makedirs("/mnt/proc", exist_ok=True)
                    os.makedirs("/mnt/sys", exist_ok=True)
                    os.makedirs("/mnt/run", exist_ok=True)
                    with open("/mnt/etc/fstab", "w") as f:
                        f.write(fstab_content)
                
            def preserve_live_initramfs_for_recovery():
                """Deploy the live/recovery initramfs to PULSAR_OS (@/boot), ESP, and PULSAR_RECOVERY."""
                if "TEST_MODE" in os.environ or not is_efi:
                    return
                try:
                    candidates = [
                        # Dedicated Debian recovery initrd
                        "/recovery/initramfs-recovery.img",
                        "/usr/share/pulsaros-recovery/initramfs-recovery.img",
                        "/run/archiso/bootmnt/recovery/initramfs-recovery.img",
                        "/run/live/medium/recovery/initramfs-recovery.img",
                        "/mnt/usr/share/pulsaros-recovery/initramfs-recovery.img",
                        "/mnt/recovery/initramfs-recovery.img",
                    ]
                    src = next((p for p in candidates if os.path.isfile(p) and os.path.getsize(p) > 1024), None)
                    if not src:
                        # Fallback to general live media paths
                        for root_dir in ("/run/archiso", "/run/live", "/lib/live"):
                            if os.path.exists(root_dir):
                                for p in glob.glob(f"{root_dir}/**/initr*", recursive=True):
                                    if os.path.isfile(p) and not p.endswith(".kver"):
                                        candidates.append(p)
                        for p in sorted(glob.glob("/boot/initramfs-*.img") + glob.glob("/boot/initrd.img*") + glob.glob("/boot/initrd*")):
                            if "fallback" not in p and "ucode" not in p and not p.endswith(".kver"):
                                candidates.append(p)
                        src = next((p for p in candidates if os.path.isfile(p) and os.path.getsize(p) > 1024), None)

                    if not src:
                        log_msg("ERROR: no live initramfs found - the recovery entry will not boot")
                        return
                    esp_root = "/mnt/boot/efi"
                    os.makedirs("/mnt/boot", exist_ok=True)
                    os.makedirs("/mnt/recovery/boot", exist_ok=True)
                    os.makedirs(f"{esp_root}/EFI/recovery", exist_ok=True)
                    shutil.copy2(src, "/mnt/boot/initramfs-recovery.img")
                    shutil.copy2(src, "/mnt/recovery/boot/initramfs-recovery.img")
                    shutil.copy2(src, "/mnt/recovery/initramfs-recovery.img")
                    shutil.copy2(src, f"{esp_root}/EFI/recovery/initramfs-recovery.img")
                    shutil.copy2(src, f"{esp_root}/EFI/recovery/initrd.img")
                    subprocess.run(["sync"])
                    log_msg(f"Recovery initramfs deployed to PULSAR_OS, PULSAR_RECOVERY, and ESP from {src}")
                except Exception as p_err:
                    log_msg(f"ERROR preserving live initramfs for recovery: {p_err}")

            def deploy_kernel_to_recovery():
                """Deploy the live/recovery kernel to PULSAR_OS (@/boot), ESP, and PULSAR_RECOVERY."""
                if "TEST_MODE" in os.environ or not is_efi:
                    return
                kernel_cand = [
                    # Dedicated Debian recovery kernel
                    "/recovery/vmlinuz-recovery",
                    "/usr/share/pulsaros-recovery/vmlinuz-recovery",
                    "/run/archiso/bootmnt/recovery/vmlinuz-recovery",
                    "/run/live/medium/recovery/vmlinuz-recovery",
                    "/mnt/usr/share/pulsaros-recovery/vmlinuz-recovery",
                    "/mnt/recovery/vmlinuz-recovery",
                ]
                found_k = next((k for k in kernel_cand if os.path.isfile(k) and not k.endswith(".kver") and os.path.getsize(k) > 1024), None)
                if not found_k:
                    for root_dir in ("/run/archiso", "/run/live", "/lib/live"):
                        if os.path.exists(root_dir):
                            for p in glob.glob(f"{root_dir}/**/vmlinuz*", recursive=True):
                                if os.path.isfile(p) and not p.endswith(".kver"):
                                    kernel_cand.append(p)
                    kernel_cand += [
                        "/mnt/boot/vmlinuz-linux",
                        "/mnt/boot/vmlinuz",
                    ] + sorted(glob.glob("/mnt/boot/vmlinuz-*")) + sorted(glob.glob("/boot/vmlinuz-*"))
                    found_k = next((k for k in kernel_cand if os.path.isfile(k) and not k.endswith(".kver") and os.path.getsize(k) > 1024), None)
                try:
                    if not found_k:
                        log_msg("ERROR: no kernel found - the recovery entry will not boot")
                        return
                    esp_root = "/mnt/boot/efi"
                    os.makedirs("/mnt/boot", exist_ok=True)
                    os.makedirs("/mnt/recovery/boot", exist_ok=True)
                    os.makedirs(f"{esp_root}/EFI/recovery", exist_ok=True)
                    shutil.copy2(found_k, "/mnt/boot/vmlinuz-recovery")
                    shutil.copy2(found_k, "/mnt/recovery/boot/vmlinuz-recovery")
                    shutil.copy2(found_k, "/mnt/recovery/boot/vmlinuz-linux")
                    shutil.copy2(found_k, "/mnt/recovery/vmlinuz-recovery")
                    shutil.copy2(found_k, f"{esp_root}/EFI/recovery/vmlinuz-recovery.efi")
                    shutil.copy2(found_k, f"{esp_root}/EFI/recovery/vmlinuz-recovery")
                    shutil.copy2(found_k, f"{esp_root}/EFI/recovery/vmlinuz.efi")
                    shutil.copy2(found_k, f"{esp_root}/EFI/recovery/vmlinuz")
                    
                    rec_opts = "boot=live components username=live autologin cow_spacesize=4G live-media=any live-media-path=live quiet splash"
                    
                    with open("/mnt/recovery/boot/refind_linux.conf", "w") as f:
                        f.write(f'"Boot Pulsar OS Recovery"  "{rec_opts}"\n')
                        f.write(f'"Boot Recovery (Debug)"     "{rec_opts.replace("quiet splash", "loglevel=7")}"\n')

                    with open(f"{esp_root}/EFI/recovery/refind_linux.conf", "w") as f:
                        f.write(f'"Boot Pulsar OS Recovery"  "{rec_opts}"\n')

                    subprocess.run(["sync"])
                    log_msg(f"Recovery kernel deployed to PULSAR_OS, PULSAR_RECOVERY, and ESP from {found_k}")
                except Exception as cp_err:
                    log_msg(f"ERROR deploying recovery kernel: {cp_err}")

            def configure_refind_menus():
                """Write the deterministic two-entry boot menu to EVERY rEFInd
                configuration location (NVRAM path and fallback)."""
                if "TEST_MODE" in os.environ or not is_efi:
                    return
                MENU_BEGIN = "# PULSAR-MENU-BEGIN"
                MENU_END = "# PULSAR-MENU-END"

                kernel_cands = sorted(glob.glob("/mnt/boot/vmlinuz-*") + glob.glob("/mnt/boot/vmlinuz"))
                installed_k_file = next((k for k in kernel_cands if os.path.isfile(k) and not k.endswith(".kver")), None)
                k_name = os.path.basename(installed_k_file) if installed_k_file else "vmlinuz-linux"

                initrd_cands = sorted(glob.glob("/mnt/boot/initramfs-*.img") + glob.glob("/mnt/boot/initrd.img*") + glob.glob("/mnt/boot/initrd*"))
                installed_initrd_file = next(
                    (
                        i for i in initrd_cands
                        if os.path.isfile(i) and "fallback" not in i and "ucode" not in i and not i.endswith(".kver")
                    ),
                    None
                )
                initrd_name = os.path.basename(installed_initrd_file) if installed_initrd_file else "initramfs-linux.img"

                ucode_lines = "".join(
                    f"    initrd /@/boot/{uc}\n"
                    for uc in ("amd-ucode.img", "intel-ucode.img")
                    if os.path.exists(f"/mnt/boot/{uc}")
                )

                rec_opts = "boot=live components username=live autologin cow_spacesize=4G live-media=any live-media-path=live quiet splash"
                rec_net_opts = "boot=live components username=live autologin cow_spacesize=4G internet_recovery=1 quiet splash"

                refind_main = f"{esp_root}/EFI/refind"
                boot_fb = f"{esp_root}/EFI/BOOT"

                for rd in (refind_main, boot_fb):
                    conf_path = f"{rd}/refind.conf"
                    if not os.path.isdir(rd):
                        log_msg(f"Notice: {rd} absent - menu config skipped there")
                        continue
                    try:
                        is_main = (rd == refind_main)
                        icon_prefix = "/EFI/refind" if is_main else "/EFI/BOOT"
                        icon_os = f"{icon_prefix}/themes/rEFInd-Regular-Dark/icons/os_pulsaros_normal.png"
                        icon_rec = f"{icon_prefix}/themes/rEFInd-Regular-Dark/icons/os_recovery.png"

                        menu_block = (
                            f"\n{MENU_BEGIN}\n"
                            "# Only show our explicit curated entries: exactly\n"
                            "# 'Pulsar OS' and 'Pulsar OS Recovery'.\n"
                            "scanfor manual,external,optical\n"
                            'dont_scan_volumes "PULSAR_RECOVERY"\n'
                            "default_selection 1\n"
                            "\n"
                            'menuentry "Pulsar OS" {\n'
                            f"    icon {icon_os}\n"
                            "    volume PULSAR_OS\n"
                            f"    loader /@/boot/{k_name}\n"
                            f"{ucode_lines}"
                            f"    initrd /@/boot/{initrd_name}\n"
                            f'    options "root=UUID={root_uuid} rootflags=subvol=@ rw quiet splash"\n'
                            '    submenuentry "Boot to single-user mode" {\n'
                            f'        options "root=UUID={root_uuid} rootflags=subvol=@ rw single"\n'
                            "    }\n"
                            "}\n"
                            "\n"
                            'menuentry "Pulsar OS Recovery" {\n'
                            f"    icon {icon_rec}\n"
                            "    volume PULSAR_OS\n"
                            "    loader /@/boot/vmlinuz-recovery\n"
                            "    initrd /@/boot/initramfs-recovery.img\n"
                            f'    options "{rec_opts}"\n'
                            '    submenuentry "Boot Recovery from ESP" {\n'
                            "        loader /EFI/recovery/vmlinuz-recovery\n"
                            "        initrd /EFI/recovery/initramfs-recovery.img\n"
                            f'        options "{rec_opts}"\n'
                            "    }\n"
                            '    submenuentry "Boot Recovery from PULSAR_RECOVERY partition" {\n'
                            "        volume PULSAR_RECOVERY\n"
                            "        loader /boot/vmlinuz-recovery\n"
                            "        initrd /boot/initramfs-recovery.img\n"
                            f'        options "{rec_opts.replace("live-media=any", "live-media=/dev/disk/by-label/PULSAR_RECOVERY")}"\n'
                            "    }\n"
                            '    submenuentry "Internet Recovery" {\n'
                            f'        options "{rec_net_opts}"\n'
                            "    }\n"
                            "}\n"
                            f"{MENU_END}\n"
                        )

                        content = ""
                        if os.path.isfile(conf_path):
                            with open(conf_path, "r") as f:
                                content = f.read()
                        content = content.replace("#enable_mouse", "enable_mouse")
                        content = content.replace("enable_mouse", "enable_mouse", 1)
                        if os.path.isfile(f"{rd}/themes/rEFInd-Regular-Dark/theme.conf") \
                                and "include themes/rEFInd-Regular-Dark/theme.conf" not in content:
                            content += "\ninclude themes/rEFInd-Regular-Dark/theme.conf\n"
                        content = re.sub(
                            re.escape(MENU_BEGIN) + r".*?" + re.escape(MENU_END) + r"\n?",
                            "",
                            content,
                            flags=re.DOTALL,
                        )
                        if "/EFI/recovery/vmlinuz.efi" in content:
                            content = re.sub(
                                r'\nmenuentry "Pulsar OS Recovery" \{.*?\n\}\n',
                                "\n",
                                content,
                                flags=re.DOTALL,
                            )
                        content += menu_block
                        with open(conf_path, "w") as f:
                            f.write(content)
                        log_msg(f"rEFInd menu configured: {conf_path}")
                    except Exception as cfg_err:
                        log_msg(f"ERROR configuring {conf_path}: {cfg_err}")

            GLib.idle_add(self.update_progress, 0.90, "Installing bootloader...")
            exec_cmd(["mount", "--bind", "/dev", "/mnt/dev"])
            if "TEST_MODE" not in os.environ:
                os.makedirs("/mnt/dev/pts", exist_ok=True)
            exec_cmd(["mount", "-t", "devpts", "devpts", "/mnt/dev/pts"])
            exec_cmd(["mount", "--bind", "/proc", "/mnt/proc"])
            exec_cmd(["mount", "--bind", "/sys", "/mnt/sys"])
            exec_cmd(["mount", "-t", "tmpfs", "tmpfs", "/mnt/run"])

            refind_installed = False
            refind_available = any(
                os.path.exists(p)
                for p in (
                    "/mnt/usr/bin/refind-install",
                    "/mnt/usr/sbin/refind-install",
                    "/mnt/bin/refind-install",
                )
            )

            if is_efi and refind_available:
                GLib.idle_add(self.update_progress, 0.90, "Installing rEFInd bootloader...")
                try:
                    live_refind_install = next(
                        (
                            p
                            for p in (
                                "/usr/bin/refind-install",
                                "/usr/sbin/refind-install",
                                "/bin/refind-install",
                            )
                            if os.path.exists(p)
                        ),
                        None,
                    )
                    if live_refind_install is None:
                        raise RuntimeError("refind-install not found in the live system")
                    exec_cmd([live_refind_install, "--root", "/mnt", "--yes"])
                    refind_installed = True
                except Exception as ref_err:
                    print(f"Warning: refind-install failed: {ref_err}. Falling back to GRUB.")
                    exec_cmd(["chroot", "/mnt", "grub-install", "--force", "--removable", disk_path])
                else:
                    GLib.idle_add(self.update_progress, 0.92, "Configuring rEFInd...")
                    esp_root = "/mnt/boot/efi"
                    try:
                        stale_dirs = (
                            "EFI/Linux",
                            "EFI/systemd",
                            "EFI/tools",
                            "EFI/BOOT",
                            "loader",
                            "grub",
                        )
                        stale_files = (
                            "vmlinuz-linux",
                            "initramfs-linux.img",
                            "initramfs-linux-fallback.img",
                            "amd-ucode.img",
                            "refind_linux.conf",
                        )
                        if "TEST_MODE" not in os.environ:
                            for rel in stale_dirs:
                                subprocess.run(
                                    ["rm", "-rf", f"{esp_root}/{rel}"],
                                    capture_output=True,
                                )
                            for rel in stale_files:
                                subprocess.run(
                                    ["rm", "-f", f"{esp_root}/{rel}"],
                                    capture_output=True,
                                )

                        refind_linux_conf = "/mnt/boot/refind_linux.conf"
                        conf_content = (
                            f'"Boot with standard options"  "root=UUID={root_uuid} rootflags=subvol=@ rw quiet splash"\n'
                            f'"Boot to single-user mode"    "root=UUID={root_uuid} rootflags=subvol=@ rw single"\n'
                            f'"Boot with minimal options"   "ro root=UUID={root_uuid} rootflags=subvol=@"\n'
                        )
                        if "TEST_MODE" not in os.environ:
                            os.makedirs("/mnt/boot", exist_ok=True)
                            with open(refind_linux_conf, "w") as f:
                                f.write(conf_content)

                        refind_share = next(
                            (
                                p
                                for p in (
                                    "/mnt/usr/share/refind/refind_x64.efi",
                                    "/mnt/usr/share/refind/refind/refind_x64.efi",
                                )
                                if os.path.isfile(p)
                            ),
                            None,
                        )
                        if refind_share:
                            if "TEST_MODE" not in os.environ:
                                os.makedirs(f"{esp_root}/EFI/BOOT", exist_ok=True)
                                subprocess.run(
                                    ["cp", refind_share, f"{esp_root}/EFI/BOOT/BOOTX64.EFI"],
                                    capture_output=True,
                                )

                        refind_root = f"{esp_root}/EFI/refind"
                        boot_fb = f"{esp_root}/EFI/BOOT"
                        theme_src = "/mnt/usr/share/refind/themes/rEFInd-Regular-Dark"
                        theme_dest = f"{refind_root}/themes/rEFInd-Regular-Dark"
                        theme_fb_dest = f"{boot_fb}/themes/rEFInd-Regular-Dark"

                        if os.path.isdir(refind_root) and "TEST_MODE" not in os.environ:
                            os.makedirs(theme_dest, exist_ok=True)
                            os.makedirs(theme_fb_dest, exist_ok=True)
                            if os.path.isdir(theme_src):
                                subprocess.run(
                                    ["cp", "-r", f"{theme_src}/.", theme_dest],
                                    capture_output=True,
                                )
                                subprocess.run(
                                    ["cp", "-r", f"{theme_src}/.", theme_fb_dest],
                                    capture_output=True,
                                )

                            rec_icon_cands = [
                                "/mnt/usr/share/pulsar-boot-icons/os_recovery.png",
                                "/mnt/usr/share/pulsar-boot-icons/recovery.png",
                                "/usr/share/pulsar-boot-icons/os_recovery.png",
                                "/usr/share/pulsar-boot-icons/recovery.png",
                                "/mnt/usr/share/pulsaros-recovery/os_recovery.png",
                                "/usr/share/pulsaros-recovery/os_recovery.png",
                            ]
                            rec_icon_src = next((p for p in rec_icon_cands if os.path.isfile(p)), None)
                            if rec_icon_src:
                                for idir in (
                                    f"{theme_dest}/icons",
                                    f"{theme_fb_dest}/icons",
                                    f"{refind_root}/icons",
                                    refind_root,
                                    f"{boot_fb}/icons",
                                    boot_fb,
                                    f"{esp_root}/EFI/recovery",
                                    "/mnt/recovery",
                                    "/mnt/recovery/boot",
                                ):
                                    try:
                                        os.makedirs(idir, exist_ok=True)
                                        shutil.copy2(rec_icon_src, f"{idir}/os_recovery.png")
                                        shutil.copy2(rec_icon_src, f"{idir}/os_recovery-big.png")
                                        shutil.copy2(rec_icon_src, f"{idir}/os_pulsaros_recovery.png")
                                        shutil.copy2(rec_icon_src, f"{idir}/.VolumeIcon.png")
                                    except Exception:
                                        pass

                            main_icon_cands = [
                                "/mnt/usr/share/icons/hicolor/128x128/apps/pulsar-logo.png",
                                "/mnt/usr/share/icons/hicolor/128x128/apps/pulsaros-logo.png",
                                "/usr/share/icons/hicolor/128x128/apps/pulsar-logo.png",
                                "/usr/share/icons/hicolor/128x128/apps/pulsaros-logo.png",
                                "/mnt/usr/share/pulsar-boot-icons/normal.png",
                                "/usr/share/pulsar-boot-icons/normal.png",
                                f"{theme_dest}/icons/os_pulsaros_normal.png",
                                f"{theme_dest}/icons/os_pulsaros.png",
                                "/mnt/usr/share/pulsaros-recovery/logo.png",
                                "/usr/share/pulsaros-recovery/logo.png",
                            ]
                            main_icon_src = next((p for p in main_icon_cands if os.path.isfile(p)), None)
                            if main_icon_src:
                                for idir in (
                                    f"{theme_dest}/icons",
                                    f"{theme_fb_dest}/icons",
                                    f"{refind_root}/icons",
                                    refind_root,
                                    f"{boot_fb}/icons",
                                    boot_fb,
                                    "/mnt/@/boot",
                                    "/mnt/@",
                                ):
                                    try:
                                        os.makedirs(idir, exist_ok=True)
                                        shutil.copy2(main_icon_src, f"{idir}/os_pulsaros_normal.png")
                                        shutil.copy2(main_icon_src, f"{idir}/os_pulsaros.png")
                                        shutil.copy2(main_icon_src, f"{idir}/.VolumeIcon.png")
                                    except Exception:
                                        pass

                            drv_search_dirs = [
                                "/mnt/usr/share/refind/drivers_x64",
                                "/mnt/usr/lib/refind/drivers_x64",
                                "/usr/share/refind/drivers_x64",
                                "/usr/lib/refind/drivers_x64",
                                "/run/archiso/bootmnt/EFI/BOOT/drivers_x64",
                                "/run/archiso/bootmnt/EFI/refind/drivers_x64",
                                "/run/live/medium/EFI/BOOT/drivers_x64",
                                "/run/live/medium/EFI/refind/drivers_x64",
                            ]
                            for drv_name in ["ext4_x64.efi", "btrfs_x64.efi", "iso9660_x64.efi"]:
                                drv_src = next((f"{d}/{drv_name}" for d in drv_search_dirs if os.path.isfile(f"{d}/{drv_name}")), None)
                                if drv_src:
                                    for target_dir in (
                                        f"{refind_root}/drivers_x64",
                                        f"{refind_root}/drivers",
                                        f"{boot_fb}/drivers_x64",
                                        f"{boot_fb}/drivers",
                                        f"{esp_root}/drivers_x64",
                                        f"{esp_root}/drivers",
                                    ):
                                        try:
                                            os.makedirs(target_dir, exist_ok=True)
                                            shutil.copy2(drv_src, f"{target_dir}/{drv_name}")
                                        except Exception:
                                            pass
                    except Exception as theme_err:
                        print(f"Warning: rEFInd theme configuration failed: {theme_err}")
            elif is_efi:
                GLib.idle_add(self.update_progress, 0.90, "Installing GRUB bootloader...")
                try:
                    exec_cmd(["chroot", "/mnt", "grub-install", "--force", disk_path])
                except Exception as g_err:
                    print(f"Warning: Standard grub-install failed: {g_err}. Proceeding with removable fallback...")
                exec_cmd(["chroot", "/mnt", "grub-install", "--force", "--removable", disk_path])
            else:
                GLib.idle_add(self.update_progress, 0.90, "Installing GRUB bootloader...")
                exec_cmd(["chroot", "/mnt", "grub-install", "--target=i386-pc", "--force", disk_path])
                
            esp_root = "/mnt/boot/efi"

            if is_arch:
                for f_live in [
                    "/mnt/etc/mkinitcpio.conf.d/archiso.conf",
                    "/mnt/etc/mkinitcpio.conf.d/live.conf",
                ]:
                    if os.path.exists(f_live):
                        try:
                            os.remove(f_live)
                        except Exception:
                            pass

                mkinit_path = "/mnt/etc/mkinitcpio.conf"
                if os.path.exists(mkinit_path):
                    with open(mkinit_path, "r") as f:
                        mk_content = f.read()
                    if "btrfs" not in mk_content and "HOOKS=" in mk_content:
                        mk_content = re.sub(r'HOOKS=\((.*?)\)', r'HOOKS=(\1 btrfs)', mk_content)
                        with open(mkinit_path, "w") as f:
                            f.write(mk_content)

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

                preserve_live_initramfs_for_recovery()

                try:
                    exec_cmd(["chroot", "/mnt", "mkinitcpio", "-P"])
                except Exception as mki_err:
                    log_msg(f"Warning: mkinitcpio -P failed (non-fatal): {mki_err}")

                deploy_kernel_to_recovery()

                if not refind_installed:
                    exec_cmd(["chroot", "/mnt", "grub-mkconfig", "-o", "/boot/grub/grub.cfg"])
                else:
                    configure_refind_menus()
            else:
                preserve_live_initramfs_for_recovery()
                if not refind_installed:
                    exec_cmd(["chroot", "/mnt", "update-grub"])
                deploy_kernel_to_recovery()
                if refind_installed:
                    configure_refind_menus()



            # ── Recompile the system dconf database ───────────────────
            # The PulsarOS macOS keybindings (XKB Super<->Ctrl swap, spotlight on
            # <Ctrl>space, etc.) live in /etc/dconf/db/local. The rsync copies the
            # compiled DB and its local.d sources; recompiling here guarantees the
            # installed system applies them even if the compiled DB was missing or
            # stale. This is best-effort: dconf may be absent in minimal targets.
            try:
                GLib.idle_add(self.update_progress, 0.90, "Applying system settings...")
                exec_cmd(["chroot", "/mnt", "dconf", "update"])
            except Exception as dconf_err:
                print(f"Warning: dconf update failed (non-fatal): {dconf_err}")

            # ── Driver installation (Best effort, non-fatal for offline installs) ───
            # ── Broadcom Driver installation (Optional, requires Internet) ───
            if self.install_broadcom:
                # Bind network-related paths so package manager can reach the internet if available
                exec_cmd(["mount", "--bind", "/etc/resolv.conf", "/mnt/etc/resolv.conf"])
                policy_file = "/mnt/usr/sbin/policy-rc.d"
                try:
                    # Check network connectivity
                    has_net = subprocess.run(
                        ["ping", "-c", "1", "-W", "3", "1.1.1.1"],
                        capture_output=True
                    ).returncode == 0 or subprocess.run(
                        ["curl", "-s", "-I", "-m", "3", "https://archlinux.org"],
                        capture_output=True
                    ).returncode == 0

                    if not has_net:
                        print("Notice: No active Internet connection. Broadcom driver installation skipped. Install later with Driver Manager.")
                    elif is_arch:
                        # Arch Linux path
                        GLib.idle_add(self.update_progress, 0.93, "Installing Broadcom drivers...")
                        try:
                            exec_cmd(["chroot", "/mnt", "pacman", "-Sy", "--noconfirm"])
                            exec_cmd(["chroot", "/mnt", "pacman", "-S", "--noconfirm", "--needed",
                                      "broadcom-wl-dkms", "linux-headers"])
                        except Exception as arch_net_err:
                            print(f"Notice: Broadcom install error: {arch_net_err}")

                        # blacklist conflicting open-source drivers
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

                            # Run apt update to fetch package indices and install broadcom-sta-dkms
                            GLib.idle_add(self.update_progress, 0.93, "Installing Broadcom drivers...")
                            exec_cmd(["chroot", "/mnt", "apt-get", "update"])
                            exec_cmd(["chroot", "/mnt", "apt-get", "install", "-y",
                                      "broadcom-sta-dkms", "linux-headers-amd64"])

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
                            print(f"Warning: Non-fatal driver step: {deb_err}")
                        finally:
                            if "TEST_MODE" not in os.environ and os.path.exists(policy_file):
                                try:
                                    os.remove(policy_file)
                                except Exception:
                                    pass
                except Exception as drv_err:
                    print(f"Notice: Driver configuration step completed: {drv_err}")
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
        
        # Read the log file to display inside the textview
        log_content = ""
        log_file = "/tmp/pulsaros-install.log"
        if os.path.exists(log_file):
            try:
                with open(log_file, "r") as f:
                    log_content = f.read()
            except Exception as e:
                log_content = f"Could not read log file: {e}\n"
        
        if not log_content:
            log_content = f"Error details:\n{error}"
            
        buffer = self.error_log_text.get_buffer()
        buffer.set_text(log_content)
        
        # Scroll to the bottom of the log
        GLib.idle_add(self._scroll_log_to_bottom)
        
        self.stack.set_visible_child_name("install_error")

    def _scroll_log_to_bottom(self):
        buffer = self.error_log_text.get_buffer()
        mark = buffer.get_insert()
        self.error_log_text.scroll_to_mark(mark, 0.0, True, 0.5, 1.0)
        return False

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
