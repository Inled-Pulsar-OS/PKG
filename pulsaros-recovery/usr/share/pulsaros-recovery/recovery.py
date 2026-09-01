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
import json
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
.net-status {
    font-size: 15px;
    font-weight: bold;
    margin-top: 6px;
}
.net-status.ok {
    color: #30d158;
}
.net-status.warn {
    color: #ff9f0a;
}
.net-status.error {
    color: #ff453a;
}
.net-detail {
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
.mode-card {
    background-color: #2a2a2a;
    border: 1px solid #3c3c3c;
    border-radius: 12px;
    padding: 16px 20px;
    margin: 6px 0;
    transition: all 0.15s ease;
}
.mode-card:hover {
    background-color: #323236;
}
.mode-card.selected {
    background-color: #323236;
    border-color: #0071e3;
    box-shadow: 0 0 0 2px #0071e3;
}
.mode-title {
    font-size: 14px;
    font-weight: bold;
    color: #ffffff;
}
.mode-desc {
    font-size: 11px;
    color: #aeaeb2;
    margin-top: 2px;
}
.partition-card {
    background-color: #2a2a2a;
    border: 1px solid #3c3c3c;
    border-radius: 10px;
    padding: 10px 14px;
    margin: 3px 0;
    transition: all 0.15s ease;
}
.partition-card:hover {
    background-color: #323236;
}
.partition-card.selected {
    background-color: #323236;
    border-color: #0071e3;
    box-shadow: 0 0 0 2px #0071e3;
}
.partition-name {
    font-size: 13px;
    font-weight: 700;
    color: #ffffff;
}
.partition-desc {
    font-size: 11px;
    color: #aeaeb2;
}
.partition-badge {
    background-color: #3a3a3c;
    color: #30d158;
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 10px;
    font-weight: bold;
}
.partition-badge-win {
    background-color: #1e3a5f;
    color: #5ac8fa;
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 10px;
    font-weight: bold;
}
.partition-badge-efi {
    background-color: #3a3a3c;
    color: #ff9f0a;
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 10px;
    font-weight: bold;
}
.efi-info-box {
    background-color: #222224;
    border: 1px solid #333336;
    border-radius: 8px;
    padding: 8px 14px;
    margin-top: 6px;
}
.efi-info-text {
    font-size: 11px;
    color: #30d158;
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
.demo-banner {
    background-color: #ff9f0a;
    color: #000000;
    border-radius: 8px;
    padding: 8px 16px;
    font-size: 13px;
    font-weight: bold;
    text-align: center;
    margin-bottom: 4px;
}
.live-log-view-flat {
    background-color: transparent;
    border: none;
    border-radius: 0;
    padding: 0;
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


def get_system_partitions(disk_path):
    partitions = []
    try:
        out = subprocess.check_output(
            ["lsblk", "-J", "-o", "NAME,PATH,SIZE,TYPE,FSTYPE,LABEL,PARTLABEL,MOUNTPOINT,UUID", disk_path],
            text=True, stderr=subprocess.DEVNULL
        )
        data = json.loads(out)
        devices = data.get("blockdevices", [])
        
        def collect_parts(dev_list):
            for dev in dev_list:
                dtype = dev.get("type", "")
                if dtype in ("part", "lvm"):
                    partitions.append(dev)
                if "children" in dev and dev["children"]:
                    collect_parts(dev["children"])
                    
        collect_parts(devices)
    except Exception as e:
        print(f"Error parsing partitions with lsblk for {disk_path}: {e}")
        try:
            disk_base = os.path.basename(disk_path)
            with open("/proc/partitions", "r") as f:
                for line in f:
                    parts = line.split()
                    if len(parts) == 4:
                        pname = parts[3]
                        if pname.startswith(disk_base) and pname != disk_base:
                            partitions.append({
                                "name": pname,
                                "path": f"/dev/{pname}",
                                "size": f"{int(parts[2]) // 1024}M",
                                "type": "part",
                                "fstype": "",
                                "label": "",
                                "partlabel": "",
                                "mountpoint": None,
                                "uuid": None
                            })
        except Exception as ex:
            print(f"Fallback reading /proc/partitions failed: {ex}")

    return partitions


def detect_efi_partition(disk_path=None):
    """Find the EFI System Partition (ESP) on a given disk, or across the whole system."""
    candidate_parts = []
    try:
        cmd = ["lsblk", "-J", "-o", "NAME,PATH,SIZE,TYPE,FSTYPE,LABEL,PARTLABEL,PARTTYPE,PARTTYPENAME,MOUNTPOINT,UUID"]
        if disk_path:
            cmd.append(disk_path)
        out = subprocess.check_output(cmd, text=True, stderr=subprocess.DEVNULL)
        data = json.loads(out)
        
        def collect_parts(dev_list):
            for dev in dev_list:
                if dev.get("type") in ("part", "lvm"):
                    candidate_parts.append(dev)
                if "children" in dev and dev["children"]:
                    collect_parts(dev["children"])
                    
        collect_parts(data.get("blockdevices", []))
    except Exception as e:
        print(f"Error detecting EFI partition: {e}")
        
    def is_live_usb(p):
        lbl = (p.get("label") or "").upper()
        mp = (p.get("mountpoint") or "")
        return "PULSAR_ISO" in lbl or "ARCHISO" in lbl or "/run/archiso" in mp or "/run/live" in mp

    # First pass: explicit EFI typecode, GUID, label, partlabel or mountpoint
    for p in candidate_parts:
        if is_live_usb(p):
            continue
        fstype = (p.get("fstype") or "").lower()
        label = (p.get("label") or "").lower()
        partlabel = (p.get("partlabel") or "").lower()
        parttype = (p.get("parttype") or "").lower()
        parttypename = (p.get("parttypename") or "").lower()
        mountpoint = (p.get("mountpoint") or "").lower()
        
        if "c12a7328" in parttype or parttype in ("ef00", "0xef") or "efi" in parttypename:
            return p.get("path") or f"/dev/{p.get('name')}"
            
        if fstype in ("vfat", "fat32", "fat16"):
            if "efi" in label or "efi" in partlabel or "boot" in mountpoint or "efi" in mountpoint or "system" in label:
                return p.get("path") or f"/dev/{p.get('name')}"
                
    # Second pass: any vfat partition on internal target disk
    for p in candidate_parts:
        if is_live_usb(p):
            continue
        fstype = (p.get("fstype") or "").lower()
        if fstype in ("vfat", "fat32", "fat16"):
            return p.get("path") or f"/dev/{p.get('name')}"
            
    if disk_path:
        return detect_efi_partition(None)
        
    return None


def format_partition_display(part):
    name = part.get("name", "")
    path = part.get("path") or f"/dev/{name}"
    size = part.get("size", "Unknown")
    fstype = part.get("fstype") or "unformatted"
    label = part.get("label") or ""
    partlabel = part.get("partlabel") or ""
    mountpoint = part.get("mountpoint") or ""
    
    fstype_lower = fstype.lower()
    label_lower = label.lower()
    partlabel_lower = partlabel.lower()
    
    is_efi = False
    is_recovery = False
    is_windows = False
    is_linux = False
    
    if "efi" in label_lower or "efi" in partlabel_lower or (fstype_lower in ("vfat", "fat32", "fat16") and ("boot" in mountpoint or "efi" in mountpoint)):
        is_efi = True
        os_desc = "EFI System Partition"
    elif "recovery" in label_lower or "recovery" in partlabel_lower or label == "PULSAR_RECOVERY":
        is_recovery = True
        os_desc = "Recovery Partition"
    elif fstype_lower in ("ntfs", "exfat") or "msftdata" in partlabel_lower or "windows" in label_lower:
        is_windows = True
        os_desc = f"Windows ({fstype.upper()})"
    elif fstype_lower in ("ext4", "ext3", "ext2", "btrfs", "xfs", "f2fs"):
        is_linux = True
        if label == "PULSAR_OS":
            os_desc = "Pulsar OS (Btrfs)"
        else:
            os_desc = f"Linux ({fstype.upper()})"
    elif fstype_lower == "swap":
        os_desc = "Linux Swap"
    elif fstype_lower in ("apfs", "hfsplus"):
        os_desc = "macOS (APFS/HFS+)"
    else:
        os_desc = f"{fstype.upper()}" if fstype != "unformatted" else "Unformatted / Free Space"

    title = f"{path} ({size})"
    detail = os_desc
    if label and label != os_desc and label != "PULSAR_OS":
        detail += f" • {label}"
    if mountpoint and mountpoint not in ("None", ""):
        detail += f" • {mountpoint}"

    return {
        "path": path,
        "name": name,
        "size": size,
        "fstype": fstype,
        "label": label,
        "partlabel": partlabel,
        "mountpoint": mountpoint,
        "os_desc": os_desc,
        "title": title,
        "detail": detail,
        "is_efi": is_efi,
        "is_recovery": is_recovery,
        "is_windows": is_windows,
        "is_linux": is_linux,
    }


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


class InstallModeCard(Gtk.Box):
    def __init__(self, mode_id, title, desc, icon_name, select_callback):
        super().__init__(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        self.mode_id = mode_id
        self.select_callback = select_callback
        self.add_css_class("mode-card")
        
        icon = Gtk.Image.new_from_icon_name(icon_name)
        icon.set_pixel_size(36)
        icon.set_valign(Gtk.Align.CENTER)
        self.append(icon)
        
        text_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        text_box.set_valign(Gtk.Align.CENTER)
        text_box.set_hexpand(True)
        
        title_lbl = Gtk.Label(label=title)
        title_lbl.add_css_class("mode-title")
        title_lbl.set_halign(Gtk.Align.START)
        text_box.append(title_lbl)
        
        desc_lbl = Gtk.Label(label=desc)
        desc_lbl.add_css_class("mode-desc")
        desc_lbl.set_halign(Gtk.Align.START)
        desc_lbl.set_wrap(True)
        desc_lbl.set_max_width_chars(38)
        text_box.append(desc_lbl)
        
        self.append(text_box)
        
        gesture = Gtk.GestureClick()
        gesture.connect("released", lambda g, n, x, y: self.select_callback(self))
        self.add_controller(gesture)


class PartitionCardRow(Gtk.ListBoxRow):
    def __init__(self, part_info):
        super().__init__()
        self.part_info = part_info
        
        row_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        row_box.add_css_class("partition-card")
        
        icon_name = "drive-harddisk"
        if part_info["is_efi"]:
            icon_name = "emblem-system-symbolic"
        elif part_info["is_windows"]:
            icon_name = "preferences-system-windows"
        elif part_info["is_linux"]:
            icon_name = "system-software-install"
            
        icon = Gtk.Image.new_from_icon_name(icon_name)
        icon.set_pixel_size(28)
        icon.set_valign(Gtk.Align.CENTER)
        row_box.append(icon)
        
        text_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        text_box.set_valign(Gtk.Align.CENTER)
        text_box.set_hexpand(True)
        
        header_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        name_lbl = Gtk.Label(label=part_info["title"])
        name_lbl.add_css_class("partition-name")
        name_lbl.set_halign(Gtk.Align.START)
        header_box.append(name_lbl)
        
        if part_info["is_efi"]:
            badge = Gtk.Label(label="EFI")
            badge.add_css_class("partition-badge-efi")
            header_box.append(badge)
        elif part_info["is_windows"]:
            badge = Gtk.Label(label="WINDOWS")
            badge.add_css_class("partition-badge-win")
            header_box.append(badge)
        elif part_info["label"] == "PULSAR_OS":
            badge = Gtk.Label(label="PULSAR OS")
            badge.add_css_class("partition-badge")
            header_box.append(badge)
            
        text_box.append(header_box)
        
        desc_lbl = Gtk.Label(label=part_info["detail"])
        desc_lbl.add_css_class("partition-desc")
        desc_lbl.set_halign(Gtk.Align.START)
        text_box.append(desc_lbl)
        
        row_box.append(text_box)
        self.set_child(row_box)



class RecoveryWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        title = "Pulsar OS Recovery"
        if "DEMO_MODE" in os.environ:
            title += " [DEMO MODE]"
        self.set_title(title)
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
        
        self.center_container = center_container

        self.card_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.card_box.add_css_class("apple-box")
        self.card_box.set_size_request(500, 420)
        self.card_box.set_valign(Gtk.Align.CENTER)
        self.card_box.set_halign(Gtk.Align.CENTER)
        
        # Crossfade transition Gtk.Stack (macos styled crossfade transition)
        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self.stack.set_transition_duration(500)
        self.card_box.append(self.stack)
        
        center_container.set_center_widget(self.card_box)
        self.set_content(center_container)
        
        # State variables
        self.install_mode = "clean"  # "clean" or "dualboot"
        self.pending_disk_path = None
        self.pending_disk_name = None
        self.target_partition = None
        self.target_partition_info = None
        self.target_efi_partition = None
        self.install_broadcom = False
        self.install_extra_packages = False
        self.selected_action = None
        self.selected_disk_card = None
        self.selected_install_mode = None
        
        # Build views
        self.build_utilities_screen()
        self.build_network_check_screen()
        self.build_install_welcome_screen()
        self.build_install_disk_select_screen()
        self.build_install_mode_select_screen()
        self.build_install_partition_select_screen()
        self.build_install_progress_screen()
        self.build_install_error_screen()
        
        self.stack.set_visible_child_name("utilities")

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
        
        # DEMO MODE banner (visible on the first screen)
        if "DEMO_MODE" in os.environ:
            demo_banner = Gtk.Label(label="⚠ DEMO MODE — No changes will be made to your system")
            demo_banner.add_css_class("demo-banner")
            demo_banner.set_halign(Gtk.Align.CENTER)
            screen_box.append(demo_banner)
        
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
        self.add_utility_row(self.listbox, "packages", "Install Extra Packages",
                             "Install Docker, drivers, firmware, and apps on the installed system.", "logo")
                             
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
            self.stack.set_visible_child_name("install_welcome")
        elif self.selected_action == "safari":
            self._popen_as_user("seafari || epiphany || firefox")
        elif self.selected_action == "disk":
            subprocess.Popen("gparted || pkexec gparted || gnome-disks || gnome-disk-utility", shell=True)
        elif self.selected_action == "packages":
            self._show_network_check_screen()

    def _show_install_packages_dialog(self):
        """Show a confirmation dialog before installing extra packages."""
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="Install Extra Packages",
            body=(
                "This will install the full set of packages that were excluded\n"
                "from the minimal ISO:\n\n"
                "• Docker\n• Full firmware (GPU, audio)\n• VM guest tools\n"
                "• Multimedia apps (VLC, Totem)\n• NVIDIA drivers\n• GNOME apps\n\n"
                "⚠️ <b>Internet connection required</b> (WiFi or Ethernet).\n"
                "The system will be mounted and packages installed via pacman."
            ),
        )
        dialog.set_body_use_markup(True)
        dialog.add_response("cancel", "Cancel")
        dialog.add_response("install", "Install Packages")
        dialog.set_response_appearance("install", Adw.ResponseAppearance.SUGGESTED)
        dialog.set_response_appearance("cancel", Adw.ResponseAppearance.DEFAULT)

        def on_response(d, resp):
            d.destroy()
            if resp == "install":
                self._start_package_installation()

        dialog.connect("response", on_response)
        dialog.present()

    def _start_package_installation(self):
        """Switch to the progress screen and install extra packages in a thread."""
        self.progress_subtitle.set_label("Installing extra packages on the installed system.")
        self.target_disk_name_lbl.set_label("Extra Packages")
        self.image.set_visible(True)
        self.title_label.set_visible(True)
        self.stack.set_visible_child_name("install_progress")
        threading.Thread(target=self._packages_installation_backend, daemon=True).start()

    def _packages_installation_backend(self):
        """Install extra packages directly on the running system.

        Detects whether the system is Arch (pacman) or Debian (apt) and
        runs the appropriate package manager.  The app is already running
        on the installed system, so no chroot or mounting is needed.
        pkexec is used for root privileges.
        """
        import datetime
        log_file = "/tmp/pulsaros-packages.log"
        try:
            with open(log_file, "w") as lf:
                lf.write(f"{datetime.datetime.now()} - Extra package installation started\n")

            def log_msg(msg):
                ts = datetime.datetime.now().strftime("%H:%M:%S")
                line = f"[{ts}] {msg}"
                with open(log_file, "a") as lf:
                    lf.write(line + "\n")
                print(msg)
                GLib.idle_add(self.append_log, line)

            # ── Detect distro ──
            GLib.idle_add(self.update_progress, 0.05, "Detecting system type...")
            is_arch = os.path.exists("/etc/pacman.conf")
            is_debian = os.path.exists("/etc/apt/sources.list") or os.path.exists("/etc/apt/sources.list.d")

            if is_arch:
                log_msg("Detected Arch Linux system — using pacman")
            elif is_debian:
                log_msg("Detected Debian system — using apt")
            else:
                log_msg("ERROR: Cannot detect system type (no pacman.conf or apt sources)")
                GLib.idle_add(self.on_installation_failed, "Cannot detect system type. Is Pulsar OS installed?")
                return

            # ── Package lists ──
            arch_packages = [
                "docker", "linux-firmware", "sof-firmware", "alsa-firmware",
                "open-vm-tools", "virtualbox-guest-utils", "xf86-video-qxl",
                "xf86-video-ati", "xfsprogs", "p7zip", "inxi", "wl-clipboard",
                "python-yaml", "vlc", "totem", "imagemagick",
                "gvfs-smb", "gvfs-gphoto2",
                "geary", "gnome-music", "gnome-contacts", "gnome-weather",
                "gnome-clocks", "xournalpp", "papers", "loupe",
                "gnome-disk-utility", "gnome-logs", "baobab",
                "vim", "webkitgtk-6.0",
                "nvidia-open", "nvidia-settings",
                "dkms", "linux-headers",
                "appmenu-gtk-module", "python-xlib",
                "python-setuptools", "python-pip",
            ]
            # Packages that live only in the AUR (not in official or Inled repos)
            arch_aur_packages = [
                "localsend-bin",
            ]
            debian_packages = [
                "docker.io",
                "firmware-linux", "firmware-sof-signed", "firmware-misc-nonfree",
                "open-vm-tools", "xserver-xorg-video-qxl",
                "vlc", "totem", "imagemagick",
                "gvfs-fuse", "gvfs-backends",
                "geary", "gnome-music", "gnome-contacts", "gnome-weather",
                "gnome-clocks",
                "nvidia-driver", "dkms", "linux-headers-amd64",
                "xdotool", "python3-xlib",
                # LocalSend is NOT packaged in Debian stable; it is installed
                # separately from its official .deb below (_install_localsend_debian).
            ]

            if is_arch:
                packages = arch_packages
                # Ensure Inled repo key is imported
                GLib.idle_add(self.update_progress, 0.10, "Setting up package manager...")
                log_msg("Importing Inled repository key...")
                keyring_cmd = (
                    "mkdir -p /etc/pacman.d/gnupg && "
                    "pacman-key --init 2>/dev/null; "
                    "pacman-key --populate archlinux 2>/dev/null; "
                    "curl -s https://apt.inled.es/archive.key | pacman-key -a - 2>/dev/null; "
                    "pacman-key --lsign-key 89F828A9675B63CD0077CE9965AA57CF36E2018F 2>/dev/null"
                )
                subprocess.run(["pkexec", "bash", "-c", keyring_cmd], capture_output=True, text=True)

                # Sync databases
                GLib.idle_add(self.update_progress, 0.15, "Syncing package databases...")
                log_msg("Syncing package databases...")
                subprocess.run(["pkexec", "pacman", "-Sy", "--noconfirm"], capture_output=True, text=True)

                # Install AUR packages (not in official/Inled repos). We build
                # them from source with makepkg when no helper (yay/paru) is
                # installed, which is the common case in a fresh Pulsar OS.
                if arch_aur_packages:
                    log_msg(f"Installing {len(arch_aur_packages)} AUR package(s)...")
                    GLib.idle_add(self.update_progress, 0.18, f"Installing {len(arch_aur_packages)} AUR packages...")
                    aur_helper = None
                    for helper in ("yay", "paru"):
                        if subprocess.run(["which", helper], capture_output=True).returncode == 0:
                            aur_helper = helper
                            break
                    if aur_helper:
                        aur_cmd = [aur_helper, "-S", "--noconfirm", "--needed"] + arch_aur_packages
                        aur_res = subprocess.run(aur_cmd, capture_output=True, text=True, timeout=300)
                        if aur_res.returncode != 0:
                            log_msg(f"WARNING: {aur_helper} failed: {aur_res.stderr.strip()}")
                        else:
                            log_msg(f"AUR packages installed successfully via {aur_helper}.")
                    else:
                        # No helper available: clone each AUR repo, build with
                        # makepkg, and install the resulting .pkg.tar.zst.
                        real_user = self._get_real_user() or "root"
                        aur_tmp = "/tmp/pulsaros-aur-build"
                        os.makedirs(aur_tmp, exist_ok=True)
                        for pkg in arch_aur_packages:
                            log_msg(f"Building {pkg} from AUR (no helper available)...")
                            pkg_dir = os.path.join(aur_tmp, pkg)
                            shutil.rmtree(pkg_dir, ignore_errors=True)
                            clone_res = subprocess.run(
                                ["git", "clone", "--depth", "1",
                                 f"https://aur.archlinux.org/{pkg}.git", pkg_dir],
                                capture_output=True, text=True, timeout=60,
                            )
                            if clone_res.returncode != 0:
                                log_msg(f"WARNING: Could not clone AUR repo for {pkg}: {clone_res.stderr.strip()}")
                                continue
                            # Build as the real user (makepkg refuses root)
                            build_res = subprocess.run(
                                ["sudo", "-u", real_user, "makepkg", "-s", "--noconfirm",
                                 "--skippgpcheck", "--nocheck"],
                                cwd=pkg_dir, capture_output=True, text=True, timeout=600,
                            )
                            if build_res.returncode != 0:
                                log_msg(f"WARNING: makepkg failed for {pkg}: {build_res.stdout[-200:]}")
                                continue
                            # Find and install the built package
                            import glob as glob_mod
                            built = glob_mod.glob(os.path.join(pkg_dir, "*.pkg.tar.zst"))
                            if not built:
                                log_msg(f"WARNING: No .pkg.tar.zst found for {pkg} after build.")
                                continue
                            install_res = subprocess.run(
                                ["pkexec", "pacman", "-U", "--noconfirm", built[0]],
                                capture_output=True, text=True, timeout=120,
                            )
                            if install_res.returncode != 0:
                                log_msg(f"WARNING: pacman -U failed for {pkg}: {install_res.stderr.strip()}")
                            else:
                                log_msg(f"{pkg} installed successfully from AUR.")
                        shutil.rmtree(aur_tmp, ignore_errors=True)

                # Install packages with streaming progress
                GLib.idle_add(self.update_progress, 0.20, f"Installing {len(packages)} packages...")
                log_msg(f"Installing {len(packages)} extra packages via pacman...")
                install_cmd = ["pkexec", "pacman", "-S", "--noconfirm", "--needed"] + packages
                proc = subprocess.Popen(install_cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
                _buf = ""
                while True:
                    ch = proc.stdout.read(1)
                    if not ch:
                        break
                    if ch in ('\r', '\n'):
                        line = _buf.strip()
                        _buf = ""
                        if not line:
                            continue
                        log_msg(f"  pacman: {line}")
                        m_dl = re.search(r"(\d+)%", line)
                        if m_dl and '[#' in line:
                            pct = int(m_dl.group(1))
                            frac = 0.20 + (pct / 100.0) * 0.70
                            GLib.idle_add(self.update_progress, frac, f"Downloading: {pct}%")
                        m_inst = re.search(r"\((\d+)/(\d+)\)\s+installing\s+", line)
                        if m_inst:
                            cur, total = int(m_inst.group(1)), int(m_inst.group(2))
                            frac = 0.90 + (cur / total) * 0.05
                            GLib.idle_add(self.update_progress, frac, f"Installing {cur}/{total}")
                    else:
                        _buf += ch
                proc.wait()
                if proc.returncode != 0:
                    log_msg(f"WARNING: pacman finished with code {proc.returncode}")
                else:
                    log_msg("All extra packages installed successfully.")

            elif is_debian:
                packages = debian_packages
                # Update and install with streaming progress
                GLib.idle_add(self.update_progress, 0.10, "Updating package lists...")
                log_msg("Running apt-get update...")
                subprocess.run(["pkexec", "apt-get", "update"], capture_output=True, text=True)

                GLib.idle_add(self.update_progress, 0.20, f"Installing {len(packages)} packages...")
                log_msg(f"Installing {len(packages)} extra packages via apt...")
                install_cmd = ["pkexec", "apt-get", "install", "-y", "--no-install-recommends"] + packages
                proc = subprocess.Popen(install_cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
                _buf = ""
                while True:
                    ch = proc.stdout.read(1)
                    if not ch:
                        break
                    if ch in ('\r', '\n'):
                        line = _buf.strip()
                        _buf = ""
                        if not line:
                            continue
                        log_msg(f"  apt: {line}")
                        m_inst = re.search(r"(\d+)/(\d+)", line)
                        if m_inst:
                            cur, total = int(m_inst.group(1)), int(m_inst.group(2))
                            frac = 0.20 + (cur / total) * 0.70
                            GLib.idle_add(self.update_progress, frac, f"Installing {cur}/{total}")
                    else:
                        _buf += ch
                proc.wait()
                if proc.returncode != 0:
                    log_msg(f"WARNING: apt finished with code {proc.returncode}")
                else:
                    log_msg("All extra packages installed successfully.")
                # LocalSend is not in Debian stable; install from its .deb.
                self._install_localsend_debian(
                    log_msg,
                    ["pkexec", "apt-get"],
                    "/tmp/LocalSend-latest-linux-x86-64.deb",
                )

            GLib.idle_add(self.update_progress, 1.0, "Extra packages installed successfully!")
            log_msg("Done! All extra packages have been installed.")
            GLib.idle_add(self.on_installation_completed)

        except Exception as err:
            log_msg(f"FAILED: {err}")
            GLib.idle_add(self.on_installation_failed, str(err))

    def _install_localsend_debian(self, log_msg, apt_prefix, deb_install_path, deb_host_path=None):
        """Install LocalSend from its official x86-64 .deb (best-effort).

        LocalSend is NOT packaged in Debian stable, so `apt-get install
        localsend` would fail and abort the whole extra-packages batch. We
        instead download the latest linux-x86-64 .deb from the official
        GitHub releases and install it with `apt-get install ./...deb` so any
        dependencies are resolved automatically. Never raises: a LocalSend
        failure must not fail the whole install (e.g. when offline).

        - apt_prefix: list that prefixes apt (e.g. ["pkexec","apt-get"] for
          the live system, or ["chroot","/mnt","apt-get"] inside the chroot).
        - deb_install_path: path apt will operate on as seen from apt's root
          (e.g. "/tmp/LocalSend-...deb" inside the chroot).
        - deb_host_path: host filesystem location to write the .deb. Defaults
          to deb_install_path (correct for the live system); for a chroot at
          /mnt it must be "/mnt" + deb_install_path.
        """
        import json
        import urllib.request
        if deb_host_path is None:
            deb_host_path = deb_install_path
        try:
            req = urllib.request.Request(
                "https://api.github.com/repos/localsend/localsend/releases/latest",
                headers={"User-Agent": "PulsarOS-Recovery/1.0"},
            )
            with urllib.request.urlopen(req, timeout=30) as resp:
                data = json.load(resp)
            asset = next(
                (a for a in data.get("assets", [])
                 if a.get("name", "").endswith("linux-x86-64.deb")),
                None,
            )
            if not asset:
                raise RuntimeError("No linux-x86-64.deb asset found in latest release")
            tag = data.get("tag_name", "latest")
            os.makedirs(os.path.dirname(deb_host_path) or "/tmp", exist_ok=True)
            log_msg(f"Downloading LocalSend {tag} .deb ...")
            urllib.request.urlretrieve(asset["browser_download_url"], deb_host_path)
            if not os.path.isfile(deb_host_path) or os.path.getsize(deb_host_path) == 0:
                raise RuntimeError("Downloaded .deb is missing or empty")
            log_msg(f"Installing LocalSend {tag} ...")
            res = subprocess.run(
                apt_prefix + ["install", "-y", "./" + deb_install_path],
                capture_output=True, text=True,
            )
            if res.returncode != 0:
                log_msg(f"WARNING: LocalSend install failed: {res.stderr.strip()}")
            else:
                log_msg("LocalSend installed successfully.")
        except Exception as loc_err:
            log_msg(f"WARNING: could not install LocalSend: {loc_err}")

    def build_install_welcome_screen(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        if "DEMO_MODE" in os.environ:
            demo_banner = Gtk.Label(label="⚠ DEMO MODE — No changes will be made to your system")
            demo_banner.add_css_class("demo-banner")
            box.append(demo_banner)
        
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
        
        if "DEMO_MODE" in os.environ:
            demo_banner = Gtk.Label(label="⚠ DEMO MODE — No changes will be made to your system")
            demo_banner.add_css_class("demo-banner")
            self.disk_select_box.append(demo_banner)
        
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
        self.install_extra_packages = False
        self.install_mode = "clean"
        self.target_partition = None
        self.target_partition_info = None
        self.target_efi_partition = None

        # Reset selection on mode cards
        self.card_clean.remove_css_class("selected")
        self.card_dual.remove_css_class("selected")
        self.selected_install_mode = None
        self.btn_mode_continue.set_sensitive(False)
        self.mode_subtitle.set_label(f"Choose an installation method for \"{disk_name}\".")

        self.stack.set_visible_child_name("install_mode_select")

    def build_install_mode_select_screen(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        box.set_size_request(460, -1)

        if "DEMO_MODE" in os.environ:
            demo_banner = Gtk.Label(label="⚠ DEMO MODE — No changes will be made to your system")
            demo_banner.add_css_class("demo-banner")
            box.append(demo_banner)

        image = self.get_logo_image(80, is_installer=True)
        box.append(image)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='18000'>Installation Type</span>")
        box.append(title)

        self.mode_subtitle = Gtk.Label(label="Choose how to install Pulsar OS on this computer.")
        self.mode_subtitle.add_css_class("welcome-subtitle")
        box.append(self.mode_subtitle)

        modes_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        modes_box.set_margin_top(4)
        modes_box.set_margin_bottom(8)

        self.card_clean = InstallModeCard(
            "clean",
            "Erase Entire Disk",
            "Erase all contents on the disk and perform a clean installation with dedicated recovery.",
            "drive-harddisk",
            self.on_mode_selected
        )
        modes_box.append(self.card_clean)

        self.card_dual = InstallModeCard(
            "dualboot",
            "Install Alongside (Dual Boot)",
            "Install Pulsar OS on a specific partition while preserving your existing operating systems (e.g. Windows or Linux).",
            "preferences-system-windows",
            self.on_mode_selected
        )
        modes_box.append(self.card_dual)

        box.append(modes_box)

        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        nav_box.set_halign(Gtk.Align.CENTER)
        nav_box.set_margin_top(8)

        btn_back = Gtk.Button(label="Back")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("install_disk_select"))
        nav_box.append(btn_back)

        self.btn_mode_continue = Gtk.Button(label="Continue")
        self.btn_mode_continue.add_css_class("suggested-action")
        self.btn_mode_continue.set_sensitive(False)
        self.btn_mode_continue.connect("clicked", self.on_mode_continue_clicked)
        nav_box.append(self.btn_mode_continue)

        box.append(nav_box)
        self.stack.add_named(box, "install_mode_select")

    def on_mode_selected(self, selected_card):
        self.card_clean.remove_css_class("selected")
        self.card_dual.remove_css_class("selected")
        selected_card.add_css_class("selected")
        self.selected_install_mode = selected_card.mode_id
        self.btn_mode_continue.set_sensitive(True)

    def on_mode_continue_clicked(self, btn):
        if not self.selected_install_mode:
            return
            
        self.install_mode = self.selected_install_mode
        if self.install_mode == "clean":
            self._show_broadcom_dialog()
        elif self.install_mode == "dualboot":
            self.refresh_partitions()
            self.stack.set_visible_child_name("install_partition_select")

    def build_install_partition_select_screen(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        box.set_size_request(460, -1)

        if "DEMO_MODE" in os.environ:
            demo_banner = Gtk.Label(label="⚠ DEMO MODE — No changes will be made to your system")
            demo_banner.add_css_class("demo-banner")
            box.append(demo_banner)

        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='17000'>Select Target Partition</span>")
        box.append(title)

        subtitle = Gtk.Label(label="Choose a partition for Pulsar OS (will be formatted as Btrfs).")
        subtitle.add_css_class("progress-text")
        box.append(subtitle)

        # Scrolled list of partitions
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_min_content_height(160)
        scrolled.set_max_content_height(200)
        scrolled.set_min_content_width(440)
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scrolled.add_css_class("live-log-view-flat")

        self.partition_listbox = Gtk.ListBox()
        self.partition_listbox.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.partition_listbox.connect("row-selected", self.on_partition_row_selected)
        scrolled.set_child(self.partition_listbox)
        box.append(scrolled)

        # EFI status banner
        self.efi_info_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.efi_info_box.add_css_class("efi-info-box")
        self.efi_info_icon = Gtk.Image.new_from_icon_name("emblem-system-symbolic")
        self.efi_info_icon.set_pixel_size(18)
        self.efi_info_box.append(self.efi_info_icon)
        self.efi_info_lbl = Gtk.Label(label="Detecting EFI System Partition...")
        self.efi_info_lbl.add_css_class("efi-info-text")
        self.efi_info_lbl.set_halign(Gtk.Align.START)
        self.efi_info_box.append(self.efi_info_lbl)
        box.append(self.efi_info_box)

        # Extra utility tools row (GParted, Refresh)
        tools_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        tools_box.set_halign(Gtk.Align.CENTER)
        tools_box.set_margin_top(4)

        btn_gparted = Gtk.Button(label="Open Disk Utility (GParted)...")
        btn_gparted.add_css_class("secondary-action")
        btn_gparted.connect("clicked", lambda x: subprocess.Popen("gparted || pkexec gparted || gnome-disks || gnome-disk-utility", shell=True))
        tools_box.append(btn_gparted)

        btn_refresh_parts = Gtk.Button(label="Refresh")
        btn_refresh_parts.add_css_class("secondary-action")
        btn_refresh_parts.connect("clicked", lambda x: self.refresh_partitions())
        tools_box.append(btn_refresh_parts)

        box.append(tools_box)

        # Navigation row
        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        nav_box.set_halign(Gtk.Align.CENTER)
        nav_box.set_margin_top(8)

        btn_back = Gtk.Button(label="Back")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("install_mode_select"))
        nav_box.append(btn_back)

        self.btn_partition_continue = Gtk.Button(label="Continue")
        self.btn_partition_continue.add_css_class("suggested-action")
        self.btn_partition_continue.set_sensitive(False)
        self.btn_partition_continue.connect("clicked", self.on_partition_continue_clicked)
        nav_box.append(self.btn_partition_continue)

        box.append(nav_box)
        self.stack.add_named(box, "install_partition_select")

    def refresh_partitions(self):
        while (child := self.partition_listbox.get_first_child()):
            self.partition_listbox.remove(child)

        self.btn_partition_continue.set_sensitive(False)
        self.target_partition = None
        self.target_partition_info = None

        disk_path = self.pending_disk_path
        partitions = get_system_partitions(disk_path)

        detected_efi = detect_efi_partition(disk_path)
        self.target_efi_partition = detected_efi

        if detected_efi:
            self.efi_info_lbl.set_markup(f"Preserving EFI System Partition: <b>{detected_efi}</b>")
            self.efi_info_box.set_visible(True)
        else:
            self.efi_info_lbl.set_markup("<i>No dedicated EFI partition detected (BIOS/MBR mode).</i>")
            self.efi_info_box.set_visible(True)

        has_candidates = False
        for raw_part in partitions:
            part_info = format_partition_display(raw_part)
            row = PartitionCardRow(part_info)
            self.partition_listbox.append(row)
            has_candidates = True

        if not has_candidates:
            empty_row = Gtk.ListBoxRow()
            empty_lbl = Gtk.Label(label="No partitions found. Click 'Disk Utility' to create partitions.")
            empty_lbl.add_css_class("progress-text")
            empty_lbl.set_margin_top(16)
            empty_lbl.set_margin_bottom(16)
            empty_row.set_child(empty_lbl)
            self.partition_listbox.append(empty_row)

    def on_partition_row_selected(self, listbox, row):
        if row is not None and hasattr(row, 'part_info'):
            part_info = row.part_info
            if part_info.get("is_efi"):
                self.btn_partition_continue.set_sensitive(False)
                self.target_partition = None
                self.target_partition_info = None
                return
            self.target_partition = part_info["path"]
            self.target_partition_info = part_info
            self.btn_partition_continue.set_sensitive(True)
        else:
            self.btn_partition_continue.set_sensitive(False)
            self.target_partition = None
            self.target_partition_info = None

    def on_partition_continue_clicked(self, btn):
        if not self.target_partition or not self.target_partition_info:
            return
        self._show_broadcom_dialog()


    def build_install_progress_screen(self):
        self.progress_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.progress_box.set_valign(Gtk.Align.CENTER)
        self.progress_box.set_halign(Gtk.Align.CENTER)
        box = self.progress_box
        
        self.image = self.get_logo_image(90, is_installer=True)
        box.append(self.image)
        
        self.title_label = Gtk.Label()
        self.title_label.set_markup("<span font_weight='bold' size='18000'>Pulsar OS</span>")
        box.append(self.title_label)
        
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
        
        # ── Inline log panel (hidden by default, toggled by terminal button) ──
        self.log_panel = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.log_panel.set_vexpand(True)
        self.log_panel.set_hexpand(True)
        self.log_panel.set_visible(False)

        log_scrolled = Gtk.ScrolledWindow()
        log_scrolled.set_hexpand(True)
        log_scrolled.set_vexpand(True)
        log_scrolled.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        log_scrolled.add_css_class("live-log-view-flat")

        self.log_text_view = Gtk.TextView()
        self.log_text_view.set_editable(False)
        self.log_text_view.set_monospace(True)
        self.log_text_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        self.log_text_view.add_css_class("live-log-text")
        self.log_buffer = self.log_text_view.get_buffer()
        log_scrolled.set_child(self.log_text_view)
        self.log_panel.append(log_scrolled)
        box.append(self.log_panel)

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
        
        self.stack.add_named(self.progress_box, "install_progress")

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

    def build_network_check_screen(self):
        """Connectivity slide shown before installing extra packages.

        Detects whether the machine has an active network, tests whether the
        Inled repository (apt.inled.es) is reachable, and offers the user the
        chance to open the GNOME network settings or enable Cloudflare WARP
        (bundled in the minimal ISO) when connectivity is missing or blocked.
        """
        self.net_check_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        self.net_check_box.set_valign(Gtk.Align.CENTER)
        self.net_check_box.set_halign(Gtk.Align.CENTER)

        self.net_icon = Gtk.Image.new_from_icon_name("network-wireless")
        self.net_icon.set_pixel_size(64)
        self.net_check_box.append(self.net_icon)

        self.net_title = Gtk.Label()
        self.net_title.set_markup("<span font_weight='bold' size='18000'>Check Internet Connection</span>")
        self.net_check_box.append(self.net_title)

        # Spinner shown while checks run (or WARP connects)
        self.net_spinner = Gtk.Spinner()
        self.net_spinner.set_size_request(40, 40)
        self.net_spinner.set_halign(Gtk.Align.CENTER)
        self.net_spinner.set_visible(False)
        self.net_check_box.append(self.net_spinner)

        self.net_status_lbl = Gtk.Label(label="Checking your Internet connection...")
        self.net_status_lbl.add_css_class("net-status")
        self.net_status_lbl.set_wrap(True)
        self.net_check_box.append(self.net_status_lbl)

        self.net_detail_lbl = Gtk.Label(label="")
        self.net_detail_lbl.add_css_class("net-detail")
        self.net_detail_lbl.set_wrap(True)
        self.net_detail_lbl.set_max_width_chars(45)
        self.net_check_box.append(self.net_detail_lbl)

        # Button to open GNOME network settings (shown when no connectivity)
        self.btn_open_network = Gtk.Button(label="Open Network Settings")
        self.btn_open_network.add_css_class("secondary-action")
        self.btn_open_network.set_visible(False)
        self.btn_open_network.set_halign(Gtk.Align.CENTER)
        self.btn_open_network.connect("clicked", self._open_network_settings)
        self.net_check_box.append(self.btn_open_network)

        # Button to enable/activate the Cloudflare WARP VPN (shown when apt.inled.es is blocked)
        self.btn_enable_warp = Gtk.Button(label="Enable Cloudflare WARP VPN")
        self.btn_enable_warp.add_css_class("suggested-action")
        self.btn_enable_warp.set_visible(False)
        self.btn_enable_warp.set_halign(Gtk.Align.CENTER)
        self.btn_enable_warp.connect("clicked", self._enable_warp_vpn)
        self.net_check_box.append(self.btn_enable_warp)

        self.net_warp_lbl = Gtk.Label(label="")
        self.net_warp_lbl.add_css_class("net-detail")
        self.net_warp_lbl.set_wrap(True)
        self.net_warp_lbl.set_max_width_chars(45)
        self.net_warp_lbl.set_visible(False)
        self.net_check_box.append(self.net_warp_lbl)

        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        nav_box.set_halign(Gtk.Align.CENTER)
        nav_box.set_margin_top(14)

        btn_back = Gtk.Button(label="Back")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self._on_network_check_back())
        nav_box.append(btn_back)

        self.btn_net_continue = Gtk.Button(label="Continue")
        self.btn_net_continue.add_css_class("suggested-action")
        self.btn_net_continue.set_sensitive(False)
        self.btn_net_continue.connect("clicked", self._net_continue_clicked)
        nav_box.append(self.btn_net_continue)

        self.net_check_box.append(nav_box)
        self.stack.add_named(self.net_check_box, "network_check")

    def _set_net_spinner(self, running):
        """Toggle the connectivity spinner (safe to call from worker threads)."""
        def _apply():
            self.net_spinner.set_visible(running)
            if running:
                self.net_spinner.start()
            else:
                self.net_spinner.stop()
        GLib.idle_add(_apply)

    def _show_network_check_screen(self):
        """Reset the connectivity slide and start the network checks."""
        self.net_status_lbl.set_text("Checking your Internet connection...")
        self.net_status_lbl.remove_css_class("ok")
        self.net_status_lbl.remove_css_class("warn")
        self.net_status_lbl.remove_css_class("error")
        self.net_detail_lbl.set_text("")
        self.net_warp_lbl.set_text("")
        self.net_warp_lbl.set_visible(False)
        self.btn_open_network.set_visible(False)
        self.btn_enable_warp.set_visible(False)
        self.btn_net_continue.set_sensitive(False)
        self.stack.set_visible_child_name("network_check")
        self._set_net_spinner(True)

        # Run checks in a background thread so the UI does not freeze
        threading.Thread(target=self._network_check_backend, daemon=True).start()

    def _network_check_backend(self):
        """Test global connectivity and reachability of the Inled repository.

        Connectivity is only offered (Continue enabled) when apt.inled.es is
        actually reachable over HTTPS. If there is no network at all we point
        the user at the GNOME network settings; if there is network but the
        Inled repo is blocked/unreachable (censorship) we recommend WARP.
        """
        import socket

        def set_status(css, text):
            def _apply():
                self.net_status_lbl.set_text(text)
                self.net_status_lbl.remove_css_class("ok")
                self.net_status_lbl.remove_css_class("warn")
                self.net_status_lbl.remove_css_class("error")
                self.net_status_lbl.add_css_class(css)
            GLib.idle_add(_apply)

        def set_detail(text):
            GLib.idle_add(lambda: self.net_detail_lbl.set_text(text))

        # 1) Is the machine connected at all (wifi / ethernet)?
        online = False
        try:
            out = subprocess.run(
                ["nmcli", "-t", "-f", "STATE", "general"],
                capture_output=True, text=True, timeout=8,
            )
            online = "connected" in out.stdout
        except Exception:
            pass

        # Fallback: try to reach a well-known host even if nmcli is unavailable
        if not online:
            try:
                socket.setdefaulttimeout(5)
                socket.create_connection(("1.1.1.1", 443), timeout=5)
                online = True
            except Exception:
                online = False

        if not online:
            self._set_net_spinner(False)
            set_status("error", "⚠️ No Internet connection detected.")
            set_detail(
                "Connect to a Wi-Fi or Ethernet network and tap \"Open Network Settings\" to configure it.\n\n"
                "Once connected we will automatically retry the connection to the Inled repository."
            )
            GLib.idle_add(lambda: self.btn_open_network.set_visible(True))
            # Keep polling so the Continue button unlocks as soon as the user connects
            self._poll_until_inled_reachable()
            return

        # 2) Check whether the Inled repository is reachable over HTTPS
        set_status("warn", "✅ Internet detected — checking the Pulsar repository...")
        repo_reachable = self._inled_reachable()
        self._set_net_spinner(False)

        if repo_reachable:
            set_status("ok", "✅ Internet OK — repository reachable.")
            set_detail(
                "You have an Internet connection and the Pulsar OS repository (apt.inled.es) "
                "is reachable. You can now install the extra packages."
            )
            GLib.idle_add(lambda: self.btn_net_continue.set_sensitive(True))
        else:
            set_status("warn", "⚠️ Internet OK, but the repository seems blocked.")
            set_detail(
                "You appear to be online, but the Pulsar OS repository (apt.inled.es) "
                "could not be reached. This is common in regions where Cloudflare is censored.\n\n"
                "Enable the bundled Cloudflare WARP VPN to route around the block, or "
                "check your network settings if you believe the connection is wrong."
            )
            GLib.idle_add(lambda: self.btn_enable_warp.set_visible(True))
            # Keep polling so Continue unlocks if connectivity is restored
            self._poll_until_inled_reachable()

    def _inled_reachable(self):
        """Return True if https://apt.inled.es answers a quick HTTPS request.

        NOTE: we use curl on purpose. Cloudflare's edge blocks urllib/libcurl's
        default user-agent (it returns 403 Forbidden), but a real browser-like
        user-agent succeeds with 200. Using curl with a normal UA makes the
        check accurate: the repo is reachable exactly when curl gets a 2xx/3xx.
        """
        try:
            res = subprocess.run(
                ["curl", "-skI", "--max-time", "8", "-A", "Mozilla/5.0",
                 "-o", "/dev/null", "-w", "%{http_code}", "https://apt.inled.es/"],
                capture_output=True, text=True, timeout=12,
            )
            code = res.stdout.strip()
            # Accept any 2xx or 3xx; reject 4xx/5xx (notably 403 from UA-blocking)
            return code.startswith("2") or code.startswith("3")
        except Exception:
            return False

    def _poll_until_inled_reachable(self):
        """Periodically re-check the repository so the Continue button unlocks on its own."""
        def poll():
            import time
            for _ in range(60):  # poll for up to ~5 minutes
                if self._inled_reachable():
                    GLib.idle_add(self._mark_repository_ok)
                    break
                time.sleep(5)
        threading.Thread(target=poll, daemon=True).start()

    def _mark_repository_ok(self):
        self.net_status_lbl.set_text("✅ Internet OK — repository reachable.")
        self.net_status_lbl.remove_css_class("error")
        self.net_status_lbl.remove_css_class("warn")
        self.net_status_lbl.add_css_class("ok")
        self.net_detail_lbl.set_text("You have an Internet connection and the Pulsar OS repository is reachable.")
        self.btn_open_network.set_visible(False)
        self.btn_enable_warp.set_visible(False)
        self.btn_net_continue.set_sensitive(True)

    def _open_network_settings(self, btn=None):
        self._popen_as_user("gnome-control-center network || nm-connection-editor || nmtui")

    def _enable_warp_vpn(self, btn=None):
        """Enable and activate the bundled Cloudflare WARP VPN (via warp-cli).

        WARP is bundled in the minimal ISO precisely so we can reach the Inled
        repository in regions where Cloudflare (or the repo) is censored.
        Activating it needs root, so we run through pkexec. We pass the
        --accept-tos flag (works fully headless, no TTY/prompt needed) and
        clear any stale registration first, since WARP refuses to re-register
        while an old one is still around.
        """
        def run():
            warp_svc = "systemctl enable --now warp-svc 2>/dev/null; sleep 2; "
            reg_clear = "warp-cli --accept-tos registration delete 2>/dev/null || true; "
            reg_new = "warp-cli --accept-tos registration new 2>/dev/null; "
            connect = "warp-cli --accept-tos connect 2>/dev/null || true"
            cmd = warp_svc + reg_clear + reg_new + connect
            output = ""
            try:
                res = subprocess.run(
                    ["pkexec", "bash", "-c", cmd],
                    timeout=90, capture_output=True, text=True,
                )
                output = (res.stdout + "\n" + res.stderr).strip()
            except Exception as e:
                print(f"WARP enable failed: {e}")
                GLib.idle_add(self._warp_failed, "WARP could not be enabled.")
                return

            if "Error" in output:
                print(f"WARP output had an error:\n{output}")

            # Re-test the repository a few times; WARP needs a moment to connect
            for _ in range(12):
                time.sleep(4)
                if self._inled_reachable():
                    GLib.idle_add(self._warp_success)
                    return
            GLib.idle_add(self._warp_failed)

        GLib.idle_add(lambda: self.btn_enable_warp.set_sensitive(False))
        GLib.idle_add(lambda: self.net_warp_lbl.set_text("Enabling WARP VPN... this takes a few seconds."))
        GLib.idle_add(lambda: self.net_warp_lbl.set_visible(True))
        GLib.idle_add(lambda: self.net_spinner.set_visible(True))
        GLib.idle_add(lambda: self.net_spinner.start())
        threading.Thread(target=run, daemon=True).start()

    def _warp_success(self):
        self.net_spinner.stop()
        self.net_spinner.set_visible(False)
        self.net_warp_lbl.set_text("✅ WARP VPN connected — repository reachable.")
        self.net_warp_lbl.remove_css_class("warn")
        self.net_warp_lbl.add_css_class("ok")
        self.btn_enable_warp.set_sensitive(True)
        self._mark_repository_ok()

    def _warp_failed(self, detail=None):
        self.net_spinner.stop()
        self.net_spinner.set_visible(False)
        self.net_warp_lbl.remove_css_class("ok")
        self.net_warp_lbl.add_css_class("warn")
        msg = "⚠️ Could not connect via WARP. Try again or check your network."
        if detail:
            msg += f"\n{detail}"
        self.net_warp_lbl.set_text(msg)
        self.btn_enable_warp.set_sensitive(True)

    def _on_network_check_back(self):
        if self.install_mode == "dualboot":
            self.stack.set_visible_child_name("install_partition_select")
        else:
            self.stack.set_visible_child_name("install_disk_select")

    def _net_continue_clicked(self, btn):
        if self.install_mode == "dualboot":
            self._show_dualboot_confirmation_dialog()
        else:
            self._show_clean_install_warning_dialog()

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
            self._show_extra_packages_dialog()

        dialog.connect("response", on_response)
        dialog.present()

    def _show_extra_packages_dialog(self):
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="Install Extra Packages",
            body=(
                "The ISO was built minimal to stay small (~3 GB). "
                "You can now install the full set of packages that were excluded:\n\n"
                "• Docker\n• Full firmware (GPU, audio)\n• VM guest tools\n"
                "• Multimedia apps (VLC, Totem)\n• NVIDIA drivers\n• GNOME apps\n\n"
                "⚠️ <b>Internet connection required</b> (WiFi or Ethernet).\n"
                "These packages will be downloaded and installed on disk."
            ),
        )
        dialog.set_body_use_markup(True)
        dialog.add_response("no",  "No, keep minimal")
        dialog.add_response("yes", "Yes, install everything")
        dialog.set_response_appearance("yes", Adw.ResponseAppearance.SUGGESTED)
        dialog.set_default_response("yes")

        def on_response(d, resp):
            d.destroy()
            if resp == "yes":
                self.install_extra_packages = True
                self._show_network_check_screen()
            else:
                self.install_extra_packages = False
                if self.install_mode == "dualboot":
                    self._show_dualboot_confirmation_dialog()
                else:
                    self._show_clean_install_warning_dialog()

        dialog.connect("response", on_response)
        dialog.present()

    def _show_clean_install_warning_dialog(self):
        disk_name = self.pending_disk_name
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="Erase Disk & Install Pulsar OS",
            body=(
                f"⚠️ <b>Warning: All data on /dev/{disk_name} will be permanently erased!</b>\n\n"
                "The entire disk will be partitioned for Pulsar OS (EFI, Recovery, and Btrfs root).\n\n"
                "Are you sure you want to proceed?"
            )
        )
        dialog.set_body_use_markup(True)
        dialog.add_response("cancel", "Cancel")
        dialog.add_response("erase", "Erase and Install")
        dialog.set_response_appearance("erase", Adw.ResponseAppearance.DESTRUCTIVE)
        dialog.set_response_appearance("cancel", Adw.ResponseAppearance.DEFAULT)
        dialog.set_default_response("cancel")

        def on_response(d, resp):
            d.destroy()
            if resp == "erase":
                self._start_installation()

        dialog.connect("response", on_response)
        dialog.present()

    def _show_dualboot_confirmation_dialog(self):
        part = self.target_partition
        part_info = self.target_partition_info or {}
        size = part_info.get("size", "")
        desc = part_info.get("os_desc", "")
        efi_desc = f"<b>{self.target_efi_partition}</b>" if self.target_efi_partition else "System EFI"

        body = (
            f"Pulsar OS will be installed on partition <b>{part}</b> ({size}, {desc}).\n\n"
            f"• <b>Only {part} will be formatted as Btrfs</b> with subvolumes <tt>@</tt> and <tt>@home</tt>.\n"
            f"• Your EFI bootloader ({efi_desc}) will be preserved and configured with rEFInd dual-boot.\n"
            f"• All other partitions on your disk (including Windows/Linux) will remain intact.\n\n"
            f"Do you want to proceed with the installation?"
        )

        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="Confirm Dual Boot Installation",
            body=body
        )
        dialog.set_body_use_markup(True)
        dialog.add_response("cancel", "Cancel")
        dialog.add_response("install", "Install Pulsar OS")
        dialog.set_response_appearance("install", Adw.ResponseAppearance.SUGGESTED)
        dialog.set_response_appearance("cancel", Adw.ResponseAppearance.DEFAULT)
        dialog.set_default_response("install")

        def on_response(d, resp):
            d.destroy()
            if resp == "install":
                self._start_installation()

        dialog.connect("response", on_response)
        dialog.present()

    def _start_installation(self):
        disk_path = self.pending_disk_path
        disk_name = self.pending_disk_name
        
        if self.install_mode == "dualboot" and self.target_partition:
            display_name = f"Pulsar OS (Dual Boot • {self.target_partition})"
            self.progress_subtitle.set_label(f"Installing Pulsar OS alongside other systems on {self.target_partition}.")
        else:
            if hasattr(self, 'selected_disk_card') and self.selected_disk_card:
                name_info = self.selected_disk_card.disk_info.get("name", "")
                size_match = re.search(r"\(([^)]+)\)", name_info)
                if size_match:
                    display_name = f"Pulsar OS ({disk_name} \u2022 {size_match.group(1)})"
                else:
                    display_name = f"Pulsar OS ({disk_name})"
            else:
                display_name = f"Pulsar OS ({disk_name})"
            self.progress_subtitle.set_label("Pulsar OS will be installed on the selected disk.")
            
        self.target_disk_name_lbl.set_label(display_name)
        self.image.set_visible(True)
        self.title_label.set_visible(True)
        self.target_disk_box.set_visible(True)
        self.progress_label.set_visible(True)
        self.log_panel.set_visible(False)
        self.btn_log.set_tooltip_text("Show Installer Log")
        self.stack.set_visible_child_name("install_progress")

        # ── DEMO MODE: never touch the real system ──────────────────────
        if "DEMO_MODE" in os.environ:
            threading.Thread(
                target=self._demo_backend,
                args=(disk_path,),
                daemon=True,
            ).start()
            return

        threading.Thread(
            target=self.installation_backend,
            args=(disk_path,),
            daemon=True
        ).start()

    # ──────────────────────────────────────────────────────────────────
    # DEMO MODE backend — zero side-effects, reads no disks, writes
    # nothing.  Purely cosmetic progress + log output for UI testing.
    # ──────────────────────────────────────────────────────────────────
    def _demo_backend(self, disk_path):
        import datetime
        log_file = "/tmp/pulsaros-install.log"
        with open(log_file, "w") as lf:
            lf.write(f"{datetime.datetime.now()} - DEMO MODE (no real changes)\n")
            lf.write(f"Target disk: {disk_path}\n")
            if self.install_mode == "dualboot":
                lf.write(f"Mode: Dual Boot on {self.target_partition}\n")

        def log_msg(msg):
            ts = datetime.datetime.now().strftime("%H:%M:%S")
            line = f"[{ts}] {msg}"
            with open(log_file, "a") as lf:
                lf.write(line + "\n")
            print(msg)
            GLib.idle_add(self.append_log, line)

        def demo_sleep(secs=0.15):
            time.sleep(secs)

        log_msg("══ DEMO MODE: no real changes will be made ══")
        
        if self.install_mode == "dualboot":
            log_msg(f"Target partition: {self.target_partition} (NOT TOUCHED)")
            log_msg(f"Preserving EFI partition: {self.target_efi_partition}")
            GLib.idle_add(self.update_progress, 0.05, f"[DEMO] Formatting target partition {self.target_partition} as Btrfs...")
            demo_sleep()
            GLib.idle_add(self.update_progress, 0.15, "[DEMO] Creating Btrfs subvolumes (@ and @home)...")
            log_msg("[DEMO] btrfs subvolume create /mnt/@")
            log_msg("[DEMO] btrfs subvolume create /mnt/@home")
            demo_sleep()
            GLib.idle_add(self.update_progress, 0.18, "[DEMO] Mounting subvolumes and existing EFI...")
            demo_sleep()
        else:
            log_msg(f"Target disk: {disk_path} (NOT TOUCHED)")
            # ── Partitioning (simulated) ──
            GLib.idle_add(self.update_progress, 0.03, "[DEMO] Cleaning disk...")
            log_msg(f"[DEMO] wipefs -a -f {disk_path}")
            demo_sleep()
            GLib.idle_add(self.update_progress, 0.05, "[DEMO] Partitioning (GPT/UEFI)...")
            for part in ["EFI", "PulsarRecovery", "PulsarOS"]:
                log_msg(f"[DEMO] Creating partition: {part}")
                demo_sleep()
            GLib.idle_add(self.update_progress, 0.10, "[DEMO] Formatting partitions...")
            for fs in ["mkfs.vfat EFI", "mkfs.ext4 PULSAR_RECOVERY", "mkfs.btrfs PULSAR_OS"]:
                log_msg(f"[DEMO] {fs}")
                demo_sleep()
            GLib.idle_add(self.update_progress, 0.15, "[DEMO] Creating Btrfs subvolumes...")
            log_msg("[DEMO] btrfs subvolume create /mnt/@")
            log_msg("[DEMO] btrfs subvolume create /mnt/@home")
            demo_sleep()
            GLib.idle_add(self.update_progress, 0.18, "[DEMO] Mounting subvolumes...")
            demo_sleep()

        # ── Rsync (simulated) ──
        GLib.idle_add(self.update_progress, 0.25, "[DEMO] Copying system files...")
        log_msg("[DEMO] rsync -aAXx / -> /mnt (SIMULATED)")
        for pct in range(26, 81, 2):
            speed = f"{300 + pct * 2}MB/s" if pct < 50 else f"{200 + (100 - pct) * 5}MB/s"
            GLib.idle_add(self.update_progress, pct / 100.0, f"[DEMO] Copying files: {pct}% at {speed}")
            demo_sleep(0.04)

        # ── Post-install packages (simulated) ──
        if self.install_extra_packages:
            GLib.idle_add(self.update_progress, 0.80, "[DEMO] Installing extra packages...")
            log_msg("[DEMO] Minimal ISO detected — installing excluded packages...")
            GLib.idle_add(self.update_progress, 0.81, "[DEMO] Setting up package manager...")
            log_msg("[DEMO] pacman-key --init")
            log_msg("[DEMO] pacman-key --populate archlinux")
            log_msg("[DEMO] Importing Inled GPG key...")
            demo_sleep(0.3)
            GLib.idle_add(self.update_progress, 0.82, "[DEMO] Syncing package databases...")
            log_msg("[DEMO] pacman -Sy --noconfirm")
            demo_sleep(0.2)

            demo_pkgs = [
                "docker", "linux-firmware", "sof-firmware",
                "open-vm-tools", "vlc", "totem", "imagemagick",
                "geary", "gnome-music", "nvidia-open", "dkms", "linux-headers",
                "localsend-bin",
            ]
            GLib.idle_add(self.update_progress, 0.83, f"[DEMO] Installing {len(demo_pkgs)} packages...")
            for i, pkg in enumerate(demo_pkgs, 1):
                for dl_pct in range(0, 101, 20):
                    frac = 0.83 + ((i - 1) / len(demo_pkgs)) * 0.07 + (dl_pct / 100.0) * (0.07 / len(demo_pkgs))
                    GLib.idle_add(self.update_progress, frac, f"[DEMO] Downloading {pkg}: {dl_pct}%")
                    demo_sleep(0.02)
                frac = 0.83 + (i / len(demo_pkgs)) * 0.07
                GLib.idle_add(self.update_progress, frac, f"[DEMO] Installing package {i}/{len(demo_pkgs)}: {pkg}")
                log_msg(f"[DEMO] ({i}/{len(demo_pkgs)}) installing {pkg}")
                demo_sleep(0.08)
        else:
            log_msg("[DEMO] Full ISO detected — no extra packages needed.")

        # ── SquashFS regeneration (simulated) ──
        GLib.idle_add(self.update_progress, 0.92, "[DEMO] Regenerating recovery image...")
        log_msg("[DEMO] mksquashfs /mnt -> /mnt/recovery/images/pulsaros-base.squashfs (SIMULATED)")
        for sq_pct in range(0, 101, 2):
            frac = 0.92 + (sq_pct / 100.0) * 0.06
            GLib.idle_add(self.update_progress, frac, f"[DEMO] Compressing system image: {sq_pct}%")
            demo_sleep(0.02)
        log_msg("[DEMO] Recovery base image regenerated: ~3.2GB (NOT ACTUAL)")

        # ── Bootloader (simulated) ──
        GLib.idle_add(self.update_progress, 0.98, "[DEMO] Configuring bootloader (rEFInd / GRUB)...")
        log_msg("[DEMO] Writing /etc/fstab")
        log_msg("[DEMO] Deploying vmlinuz-recovery + initramfs to ESP")
        log_msg("[DEMO] Configuring multi-boot menu entries (rEFInd / GRUB with os-prober)")
        log_msg("[DEMO] Generating bootloader configuration...")
        demo_sleep(0.2)

        GLib.idle_add(self.update_progress, 1.0, "[DEMO] Demo complete — nothing was changed!")
        log_msg("══ DEMO MODE: simulation finished, disk untouched ══")
        GLib.idle_add(self.on_installation_completed)

    def on_show_live_log_clicked(self, btn):
        visible = self.log_panel.get_visible()
        self.log_panel.set_visible(not visible)
        self.btn_log.set_tooltip_text("Hide Installer Log" if not visible else "Show Installer Log")
        show_log = not visible
        # Hide everything except progress bar + terminal + bottom controls
        self.image.set_visible(not show_log)
        self.title_label.set_visible(not show_log)
        self.progress_subtitle.set_visible(not show_log)
        self.target_disk_box.set_visible(not show_log)
        self.progress_label.set_visible(not show_log)
        if show_log:
            # Progress bar: thin strip at the top, full width of the card
            self.progress_bar.set_size_request(-1, 4)
            self.progress_bar.set_margin_top(0)
            self.progress_bar.set_margin_bottom(0)
            self.progress_bar.set_hexpand(True)
            # Inner box: fill the card area
            self.progress_box.set_valign(Gtk.Align.FILL)
            self.progress_box.set_halign(Gtk.Align.FILL)
            self.progress_box.set_hexpand(True)
            self.progress_box.set_vexpand(True)
            # Log panel: take all remaining space inside the card
            self.log_panel.set_vexpand(True)
            self.log_panel.set_hexpand(True)
        else:
            # Restore inner progress screen box
            self.progress_box.set_valign(Gtk.Align.CENTER)
            self.progress_box.set_halign(Gtk.Align.CENTER)
            self.progress_box.set_hexpand(False)
            self.progress_box.set_vexpand(False)
            # Restore progress bar
            self.progress_bar.set_size_request(280, -1)
            self.progress_bar.set_margin_top(12)
            self.progress_bar.set_margin_bottom(12)
            self.progress_bar.set_hexpand(False)

    def append_log(self, msg):
        """Write a line to the inline log panel and auto-scroll."""
        if not hasattr(self, 'log_buffer'):
            return
        try:
            iter_end = self.log_buffer.get_end_iter()
            self.log_buffer.insert(iter_end, msg + "\n")
            mark = self.log_buffer.create_mark(None, self.log_buffer.get_end_iter(), False)
            self.log_text_view.scroll_to_mark(mark, 0.0, True, 0.0, 1.0)
        except Exception:
            pass

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
                line = f"[{ts}] {msg}"
                with open(log_file, "a") as lf:
                    lf.write(line + "\n")
                print(msg)
                GLib.idle_add(self.append_log, line)

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
                        "/mnt/sys/firmware/efi/efivars",
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

            if self.install_mode == "dualboot":
                root_part = self.target_partition
                efi_part = self.target_efi_partition or detect_efi_partition(disk_path)
                recovery_part = None
                try:
                    parts = get_system_partitions(disk_path)
                    for p in parts:
                        if (p.get("label") or "") == "PULSAR_RECOVERY":
                            recovery_part = p.get("path") or f"/dev/{p.get('name')}"
                            break
                except Exception:
                    pass

                GLib.idle_add(self.update_progress, 0.05, f"Formatting target partition {root_part} as Btrfs...")
                exec_cmd(["wipefs", "-a", "-f", root_part])
                exec_cmd(["mkfs.btrfs", "-f", "-L", "PULSAR_OS", root_part])
                exec_cmd(["sync"])
                exec_cmd(["udevadm", "settle"])
                time.sleep(1)

                subprocess.run(["modprobe", "btrfs"], capture_output=True)
                if is_efi:
                    subprocess.run(["modprobe", "vfat"], capture_output=True)
                if recovery_part:
                    subprocess.run(["modprobe", "ext4"], capture_output=True)

                GLib.idle_add(self.update_progress, 0.15, "Creating Btrfs subvolumes (@ and @home)...")
                if "TEST_MODE" not in os.environ:
                    subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", "-t", "btrfs", root_part, "/mnt"])
                exec_cmd(["btrfs", "subvolume", "create", "/mnt/@"])
                exec_cmd(["btrfs", "subvolume", "create", "/mnt/@home"])
                exec_cmd(["umount", "/mnt"])

                GLib.idle_add(self.update_progress, 0.18, "Mounting Btrfs subvolumes and EFI...")
                exec_cmd(["mount", "-t", "btrfs", "-o", "subvol=@,compress=zstd:1", root_part, "/mnt"])
                exec_cmd(["mount", "--make-rprivate", "/mnt"])
                os.makedirs("/mnt/home", exist_ok=True)
                exec_cmd(["mount", "-t", "btrfs", "-o", "subvol=@home,compress=zstd:1", root_part, "/mnt/home"])
                if is_efi:
                    if not efi_part:
                        efi_part = detect_efi_partition(disk_path) or detect_efi_partition(None)
                    if efi_part:
                        os.makedirs("/mnt/boot/efi", exist_ok=True)
                        exec_cmd(["mount", "-t", "vfat", efi_part, "/mnt/boot/efi"])
                        log_msg(f"Dual Boot: Mounted EFI partition {efi_part} at /mnt/boot/efi")
                    else:
                        log_msg("Warning: No EFI partition could be detected for dual boot!")
                if recovery_part:
                    os.makedirs("/mnt/recovery", exist_ok=True)
                    exec_cmd(["mount", "-t", "ext4", recovery_part, "/mnt/recovery"])
                else:
                    os.makedirs("/mnt/recovery", exist_ok=True)
            elif is_efi:
                GLib.idle_add(self.update_progress, 0.05, "Cleaning and partitioning (GPT for UEFI: EFI, Recovery, Btrfs)...")
                exec_cmd(["wipefs", "-a", "-f", disk_path])
                exec_cmd(["sgdisk", "--zap-all", disk_path])
                exec_cmd(["sgdisk", "--clear", disk_path])
                exec_cmd(["sgdisk", "--new=1:0:+512M", "--typecode=1:ef00", "--change-name=1:EFI", disk_path])
                exec_cmd(["sgdisk", "--new=2:0:+8G", "--typecode=2:8300", "--change-name=2:PulsarRecovery", disk_path])
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
                # Recovery partition sized to hold the regenerated base SquashFS
                # (~3.5-4GB) plus the Debian recovery environment (374MB) and kernel.
                sfdisk_script = "label: dos\nsize=8192M, type=83\nsize=+, type=83, bootable\n"
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
                    "rsync", "-aAXx",
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
                    "--exclude=/recovery/*",
                    "--exclude=/live/*",
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
                
            # ── Post-install: install packages removed from minimal ISO ───────────
            # In minimal ISO mode, heavy packages (Docker, VM tools, full firmware,
            # multimedia apps) were excluded to keep the ISO under 3GB. Install them
            # now that the system is on disk, then regenerate the recovery base
            # SquashFS so the deployed image carries the full system (not the
            # trimmed one shipped on the ISO).
            extra_packages_installed = False
            if "TEST_MODE" not in os.environ:
                # Detect whether this came from a minimal build by checking for a marker
                minimal_marker = "/mnt/etc/pulsaros-minimal-build"
                is_minimal = os.path.exists(minimal_marker)

                if is_minimal:
                    log_msg("Minimal ISO detected — installing excluded packages...")
                else:
                    log_msg("Full ISO detected — no extra packages needed.")

                if is_arch:
                    # ── Arch: install via pacman from the [inled] + Arch repos ──
                    if is_minimal and self.install_extra_packages:
                        GLib.idle_add(self.update_progress, 0.80, "Installing extra packages (Docker, firmware, drivers, apps)...")
                        log_msg("Post-install: installing packages removed from minimal ISO via pacman...")
                        try:
                            exec_cmd(["mount", "--bind", "/etc/resolv.conf", "/mnt/etc/resolv.conf"])
                            # Every package removed from base-arch.list when building
                            # base-arch-minimal.list, grouped for readability.
                            extra_packages = [
                                # Docker container runtime
                                "docker",
                                # Full firmware (replaces the split sub-packages with the meta)
                                "linux-firmware",
                                "sof-firmware",
                                "alsa-firmware",
                                # VM guest tools (for VM installs)
                                "open-vm-tools",
                                "virtualbox-guest-utils",
                                "xf86-video-qxl",
                                # Old (pre-GCN) AMD video driver and misc system utils
                                "xf86-video-ati",
                                "xfsprogs",
                                "p7zip",
                                "inxi",
                                "wl-clipboard",
                                "python-yaml",
                                # Multimedia & image tools
                                "vlc",
                                "totem",
                                "imagemagick",
                                # Network shares & portal GVFS backends
                                "gvfs-smb",
                                "gvfs-gphoto2",
                                # GNOME apps
                                "geary",
                                "gnome-music",
                                "gnome-contacts",
                                "gnome-weather",
                                "gnome-clocks",
                                "xournalpp",
                                "papers",
                                "loupe",
                                "gnome-disk-utility",
                                "gnome-logs",
                                "baobab",
                                # Editor and WebKit/GTK3 extras pulled by the welcome app
                                "vim",
                                "webkitgtk-6.0",
                                # NVIDIA proprietary drivers
                                "nvidia-open",
                                "nvidia-settings",
                                # Broadcom & DKMS driver support
                                "dkms",
                                "linux-headers",
                                # Global menu / Fildem dependencies
                                "appmenu-gtk-module",
                                "python-xlib",
                                "python-setuptools",
                                "python-pip",
                            ]
                            # Initialize pacman keyring inside the chroot so
                            # signature verification works (avoids GPGME errors),
                            # then import the Inled repository GPG key.
                            GLib.idle_add(self.update_progress, 0.81, "Setting up package manager...")
                            log_msg("Initializing pacman keyring and Inled repo key...")
                            exec_cmd(["chroot", "/mnt", "bash", "-c",
                                       "mkdir -p /etc/pacman.d/gnupg && "
                                       "pacman-key --init && "
                                       "pacman-key --populate archlinux && "
                                       "curl -s https://apt.inled.es/archive.key | pacman-key -a - && "
                                       "pacman-key --lsign-key 89F828A9675B63CD0077CE9965AA57CF36E2018F"])
                            GLib.idle_add(self.update_progress, 0.82, "Syncing package databases...")
                            exec_cmd(["chroot", "/mnt", "pacman", "-Sy", "--noconfirm"])
                            GLib.idle_add(self.update_progress, 0.83, f"Installing {len(extra_packages)} extra packages...")
                            # ── Streaming pacman install (parse download progress) ──
                            pacman_cmd = [
                                "chroot", "/mnt", "pacman", "-S", "--noconfirm", "--needed",
                                *extra_packages
                            ]
                            pacman_proc = subprocess.Popen(
                                pacman_cmd,
                                stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT,
                                text=True,
                                bufsize=1,
                            )
                            _pbuf = ""
                            while True:
                                ch = pacman_proc.stdout.read(1)
                                if not ch:
                                    break
                                if ch in ('\r', '\n'):
                                    line = _pbuf.strip()
                                    _pbuf = ""
                                    if not line:
                                        continue
                                    log_msg(f"  pacman: {line}")
                                    # Download progress: "pkgname  123.4 KiB  0.5 MiB/s 00:00 [####] 100%"
                                    m_dl = re.search(r"(\d+)%", line)
                                    if m_dl and '[#' in line:
                                        pct = int(m_dl.group(1))
                                        frac = 0.83 + (pct / 100.0) * 0.07
                                        GLib.idle_add(self.update_progress, frac, f"Downloading packages: {pct}%")
                                    # Install progress: "(1/3) installing pkg..."
                                    m_inst = re.search(r"\((\d+)/(\d+)\)\s+installing\s+", line)
                                    if m_inst:
                                        cur, total = int(m_inst.group(1)), int(m_inst.group(2))
                                        frac = 0.90 + (cur / total) * 0.02
                                        GLib.idle_add(self.update_progress, frac, f"Installing package {cur}/{total}: {line.split('installing ')[-1]}")
                                else:
                                    _pbuf += ch
                            pacman_proc.wait()
                            if pacman_proc.returncode != 0:
                                raise Exception(f"pacman -S failed (code {pacman_proc.returncode})")
                            log_msg("Post-install: extra packages installed successfully.")
                            extra_packages_installed = True
                        except Exception as post_err:
                            log_msg(f"Post-install package installation: {post_err}")
                        finally:
                            subprocess.run(["umount", "-l", "/mnt/etc/resolv.conf"], capture_output=True)

                else:
                    # ── Debian: install via apt from Debian + Inled repositories ──
                    if is_minimal and self.install_extra_packages:
                        GLib.idle_add(self.update_progress, 0.80, "Installing extra packages (Docker, firmware, drivers, apps)...")
                        log_msg("Post-install: installing packages removed from minimal ISO via apt...")
                        try:
                            exec_cmd(["mount", "--bind", "/etc/resolv.conf", "/mnt/etc/resolv.conf"])
                            extra_packages = [
                                "docker.io",
                                "firmware-linux",
                                "firmware-sof-signed",
                                "firmware-misc-nonfree",
                                "open-vm-tools",
                                "xserver-xorg-video-qxl",
                                "vlc",
                                "totem",
                                "imagemagick",
                                "gvfs-fuse",
                                "gvfs-backends",
                                "geary",
                                "gnome-music",
                                "gnome-contacts",
                                "gnome-weather",
                                "gnome-clocks",
                                "nvidia-driver",
                                "dkms",
                                "linux-headers-amd64",
                                "xdotool",
                                "python3-xlib",
                                # LocalSend is not in Debian stable; installed
                                # separately from its official .deb below.
                            ]
                            exec_cmd(["chroot", "/mnt", "apt-get", "update"])
                            # ── Streaming apt install (parse progress) ──
                            apt_cmd = [
                                "chroot", "/mnt", "apt-get", "install", "-y",
                                "--no-install-recommends",
                                *extra_packages,
                            ]
                            apt_proc = subprocess.Popen(
                                apt_cmd,
                                stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT,
                                text=True,
                                bufsize=1,
                            )
                            _abuf = ""
                            while True:
                                ch = apt_proc.stdout.read(1)
                                if not ch:
                                    break
                                if ch in ('\r', '\n'):
                                    line = _abuf.strip()
                                    _abuf = ""
                                    if not line:
                                        continue
                                    log_msg(f"  apt: {line}")
                                    # Progress: "Do you want to continue? [Y/n]"
                                    # or "Unpacking pkg (1/23)..."
                                    m_inst = re.search(r"(\d+)/(\d+)", line)
                                    if m_inst:
                                        cur, total = int(m_inst.group(1)), int(m_inst.group(2))
                                        frac = 0.83 + (cur / total) * 0.07
                                        GLib.idle_add(self.update_progress, frac, f"Installing package {cur}/{total}")
                                else:
                                    _abuf += ch
                            apt_proc.wait()
                            if apt_proc.returncode != 0:
                                raise Exception(f"apt-get install failed (code {apt_proc.returncode})")
                            log_msg("Post-install: extra packages installed successfully (apt).")
                            extra_packages_installed = True
                            # LocalSend is not in Debian stable; install from its .deb
                            # (downloaded to /mnt/tmp so the chroot can see it at /tmp).
                            self._install_localsend_debian(
                                log_msg,
                                ["chroot", "/mnt", "apt-get"],
                                "/tmp/LocalSend-latest-linux-x86-64.deb",
                                "/mnt/tmp/LocalSend-latest-linux-x86-64.deb",
                            )
                        except Exception as post_err:
                            log_msg(f"Post-install apt installation: {post_err}")
                        finally:
                            subprocess.run(["umount", "-l", "/mnt/etc/resolv.conf"], capture_output=True)

                # Remove the marker so extra packages are not re-installed on reboot.
                try:
                    if os.path.exists(minimal_marker):
                        os.remove(minimal_marker)
                except Exception:
                    pass

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
                        os.makedirs("/mnt/recovery/recovery", exist_ok=True)
                        shutil.copy2(found_rec_squash, "/mnt/recovery/recovery/filesystem.squashfs")
                        shutil.copy2(found_rec_squash, "/mnt/recovery/filesystem.squashfs")
                    except Exception:
                        pass
                    try:
                        os.makedirs("/mnt/live", exist_ok=True)
                        os.makedirs("/mnt/recovery", exist_ok=True)
                        shutil.copy2(found_rec_squash, "/mnt/live/filesystem.squashfs")
                    except Exception:
                        pass
                    arch_dst = "/mnt/recovery/images/x86_64/airootfs.sfs"
                    log_msg(f"Recovery OS squashfs deployed from {found_rec_squash} -> {deb_dst}")

                    # Verify the critical live-boot path exists
                    if not os.path.isfile(deb_dst) or os.path.getsize(deb_dst) == 0:
                        raise Exception(f"SquashFS verification failed: {deb_dst} is missing or empty after copy")
                    log_msg(f"Live-boot SquashFS verified at {deb_dst} ({os.path.getsize(deb_dst)} bytes)")
                elif not found_rec_squash:
                    log_msg("WARNING: No recovery squashfs found in any search path — live-boot will fail")
                    log_msg(f"Searched: {rec_squash_sources}")

                # 2. Base System SquashFS (for restoring root @ subvolume - must be the Arch Linux image)
                base_squash_sources = [
                    "/run/archiso/bootmnt/images/pulsaros-base.squashfs",
                    "/run/archiso/bootmnt/arch/x86_64/airootfs.sfs",
                    "/run/archiso/bootmnt/live/x86_64/airootfs.sfs",
                    "/run/archiso/airootfs.sfs",
                    "/run/live/medium/images/pulsaros-base.squashfs",
                    "/recovery/images/pulsaros-base.squashfs",
                ]
                base_dst = "/mnt/recovery/images/pulsaros-base.squashfs"
                # If we already regenerated the base image with post-install
                # packages above, keep it — do not overwrite it with the trimmed
                # ISO-provided copy. Only backfill the archiso-style .sfs link.
                regenerated_existing = os.path.isfile(base_dst) and os.path.getsize(base_dst) > 500 * 1024 * 1024
                found_base_squash = next((p for p in base_squash_sources if os.path.isfile(p) and os.path.getsize(p) > 500 * 1024 * 1024), None)
                if (found_base_squash or regenerated_existing) and "TEST_MODE" not in os.environ:
                    if not regenerated_existing:
                        shutil.copy2(found_base_squash, base_dst)
                        log_msg(f"Base system restoration image deployed from {found_base_squash} -> {base_dst}")
                    else:
                        log_msg(f"Base system restoration image already regenerated at {base_dst} — keeping it.")
                    # Ensure the archiso-style path also points at the base image.
                    try:
                        arch_dst = "/mnt/recovery/images/x86_64/airootfs.sfs"
                        if os.path.isfile(base_dst) and not os.path.isfile(arch_dst):
                            os.link(base_dst, arch_dst)
                    except Exception:
                        pass
            except Exception as rec_copy_err:
                print(f"Notice: Recovery squashfs copy: {rec_copy_err}")

            GLib.idle_add(self.update_progress, 0.85, "Configuring bootloader (fstab)...")
            def get_partition_uuid(part):
                if "TEST_MODE" in os.environ:
                    return "simulated-uuid-1234-abcd"
                val = exec_cmd(["blkid", "-o", "value", "-s", "UUID", part])
                return val.strip()
                
            root_uuid = get_partition_uuid(root_part)
            rec_uuid = get_partition_uuid(recovery_part) if recovery_part else None
            
            if is_efi:
                efi_uuid = get_partition_uuid(efi_part) if efi_part else None
                fstab_lines = [
                    "# /etc/fstab: Pulsar OS Btrfs Configuration",
                    "# <file system>             <mount point>   <type>  <options>                                       <dump>  <pass>",
                    f"UUID={root_uuid}            /               btrfs   subvol=@,compress=zstd:1,space_cache=v2         0       0",
                    f"UUID={root_uuid}            /home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2     0       0",
                ]
                if efi_uuid:
                    fstab_lines.append(f"UUID={efi_uuid}             /boot/efi       vfat    umask=0077                                      0       2")
                if rec_uuid:
                    fstab_lines.append(f"UUID={rec_uuid}            /recovery       ext4    defaults,noatime                                0       2")
                fstab_content = "\n".join(fstab_lines) + "\n"
                
                if "TEST_MODE" not in os.environ:
                    os.makedirs("/mnt/etc", exist_ok=True)
                    os.makedirs("/mnt/dev", exist_ok=True)
                    os.makedirs("/mnt/proc", exist_ok=True)
                    os.makedirs("/mnt/sys", exist_ok=True)
                    os.makedirs("/mnt/run", exist_ok=True)
                    os.makedirs("/mnt/etc/udev/rules.d", exist_ok=True)
                    with open("/mnt/etc/fstab", "w") as f:
                        f.write(fstab_content)
                    with open("/mnt/etc/udev/rules.d/99-pulsaros-hide-recovery.rules", "w") as f:
                        f.write('# Hide PULSAR_RECOVERY partition from file managers and desktop\nENV{ID_FS_LABEL}=="PULSAR_RECOVERY", ENV{UDISKS_IGNORE}="1", ENV{UDISKS_AUTO}="0"\n')
            else:
                fstab_lines = [
                    "# /etc/fstab: Pulsar OS Btrfs Configuration (BIOS)",
                    "# <file system>             <mount point>   <type>  <options>                                       <dump>  <pass>",
                    f"UUID={root_uuid}            /               btrfs   subvol=@,compress=zstd:1,space_cache=v2         0       0",
                    f"UUID={root_uuid}            /home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2     0       0",
                ]
                if rec_uuid:
                    fstab_lines.append(f"UUID={rec_uuid}            /recovery       ext4    defaults,noatime                                0       2")
                fstab_content = "\n".join(fstab_lines) + "\n"
                
                if "TEST_MODE" not in os.environ:
                    os.makedirs("/mnt/etc", exist_ok=True)
                    os.makedirs("/mnt/dev", exist_ok=True)
                    os.makedirs("/mnt/proc", exist_ok=True)
                    os.makedirs("/mnt/sys", exist_ok=True)
                    os.makedirs("/mnt/run", exist_ok=True)
                    os.makedirs("/mnt/etc/udev/rules.d", exist_ok=True)
                    with open("/mnt/etc/fstab", "w") as f:
                        f.write(fstab_content)
                    with open("/mnt/etc/udev/rules.d/99-pulsaros-hide-recovery.rules", "w") as f:
                        f.write('# Hide PULSAR_RECOVERY partition from file managers and desktop\nENV{ID_FS_LABEL}=="PULSAR_RECOVERY", ENV{UDISKS_IGNORE}="1", ENV{UDISKS_AUTO}="0"\n')
                
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
                        # Search only for dedicated recovery initramfs images
                        for root_dir in ("/run/archiso", "/run/live", "/lib/live", "/mnt"):
                            if os.path.exists(root_dir):
                                for p in glob.glob(f"{root_dir}/**/initramfs-recovery.img", recursive=True) + glob.glob(f"{root_dir}/**/initrd.img-*+deb*", recursive=True):
                                    if os.path.isfile(p) and os.path.getsize(p) > 1024:
                                        candidates.append(p)
                        src = next((p for p in candidates if os.path.isfile(p) and os.path.getsize(p) > 1024), None)

                    if not src:
                        # Fallback to installed system's initramfs
                        for fb_dir in ("/mnt/boot", "/boot"):
                            for p in glob.glob(f"{fb_dir}/initramfs-*.img") + glob.glob(f"{fb_dir}/initrd.img*"):
                                if os.path.isfile(p) and "fallback" not in p and "ucode" not in p and os.path.getsize(p) > 1024:
                                    src = p
                                    break
                            if src:
                                break

                    if not src:
                        log_msg("WARNING: dedicated recovery initramfs not found.")
                    else:
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
                    for root_dir in ("/run/archiso", "/run/live", "/lib/live", "/mnt"):
                        if os.path.exists(root_dir):
                            for p in glob.glob(f"{root_dir}/**/vmlinuz-recovery*", recursive=True) + glob.glob(f"{root_dir}/**/vmlinuz-*+deb*", recursive=True):
                                if os.path.isfile(p) and not p.endswith(".kver") and os.path.getsize(p) > 1024:
                                    kernel_cand.append(p)
                    found_k = next((k for k in kernel_cand if os.path.isfile(k) and not k.endswith(".kver") and os.path.getsize(k) > 1024), None)
                try:
                    if not found_k:
                        # Fallback to installed system's kernel
                        for fb_dir in ("/mnt/boot", "/boot"):
                            for p in glob.glob(f"{fb_dir}/vmlinuz-linux") + glob.glob(f"{fb_dir}/vmlinuz-*") + glob.glob(f"{fb_dir}/vmlinuz"):
                                if os.path.isfile(p) and not p.endswith(".kver") and os.path.getsize(p) > 1024:
                                    found_k = p
                                    break
                            if found_k:
                                break

                    if not found_k:
                        log_msg("WARNING: dedicated recovery kernel not found.")
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
                    
                    rec_opts = "boot=live components username=live autologin cow_spacesize=4G live-media=/dev/disk/by-label/PULSAR_RECOVERY live-media-path=live fsck.mode=skip quiet splash"
                    
                    with open("/mnt/recovery/boot/refind_linux.conf", "w") as f:
                        f.write(f'"Boot Pulsar OS Recovery"  "{rec_opts}"\n')
                        f.write(f'"Boot Recovery (Debug)"     "{rec_opts.replace("quiet splash", "loglevel=7 live-debug")}"\n')

                    with open(f"{esp_root}/EFI/recovery/refind_linux.conf", "w") as f:
                        f.write(f'"Boot Pulsar OS Recovery"  "{rec_opts.replace("live-media=/dev/disk/by-label/PULSAR_RECOVERY", "live-media=any")}"\n')

                    subprocess.run(["sync"])
                    log_msg(f"Recovery kernel deployed to PULSAR_OS, PULSAR_RECOVERY, and ESP from {found_k}")
                except Exception as cp_err:
                    log_msg(f"ERROR deploying recovery kernel: {cp_err}")

            def configure_refind_menus():
                """Write the deterministic multi-boot menu to EVERY rEFInd
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

                rec_opts_rec = "boot=live components username=live autologin cow_spacesize=4G live-media=/dev/disk/by-label/PULSAR_RECOVERY live-media-path=live fsck.mode=skip quiet splash"
                rec_opts_auto = "boot=live components username=live autologin cow_spacesize=4G live-media-path=live fsck.mode=skip quiet splash"
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
                            "# Enable auto-detection of internal systems (Windows, other Linux) and optical/external drives\n"
                            "scanfor internal,external,optical,manual\n"
                            'default_selection "Pulsar OS"\n'
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
                            f'    options "{rec_opts_rec}"\n'
                            '    submenuentry "Boot Recovery from ESP" {\n'
                            "        loader /EFI/recovery/vmlinuz-recovery\n"
                            "        initrd /EFI/recovery/initramfs-recovery.img\n"
                            f'        options "{rec_opts_rec}"\n'
                            "    }\n"
                            '    submenuentry "Boot Recovery (Auto-Detect Drive)" {\n'
                            "        volume PULSAR_OS\n"
                            "        loader /@/boot/vmlinuz-recovery\n"
                            "        initrd /@/boot/initramfs-recovery.img\n"
                            f'        options "{rec_opts_auto}"\n'
                            "    }\n"
                            '    submenuentry "Boot Recovery (Debug Mode)" {\n'
                            "        volume PULSAR_OS\n"
                            "        loader /@/boot/vmlinuz-recovery\n"
                            "        initrd /@/boot/initramfs-recovery.img\n"
                            f'        options "{rec_opts_rec.replace("quiet splash", "loglevel=7 live-debug")}"\n'
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

            def configure_grub_menus():
                """Configure GRUB default parameters, os-prober for Dual Boot, theme, and recovery entry."""
                if "TEST_MODE" in os.environ:
                    return

                grub_default = "/mnt/etc/default/grub"
                grub_params = {
                    "GRUB_DISTRIBUTOR": '"Pulsar OS"',
                    "GRUB_DISABLE_OS_PROBER": "false",
                    "GRUB_TIMEOUT": "5",
                    "GRUB_TIMEOUT_STYLE": "menu",
                    "GRUB_GFXMODE": '"1920x1080,1280x720,1024x768,auto"',
                    "GRUB_GFXPAYLOAD_LINUX": '"keep"',
                }

                # Check if GRUB theme exists
                theme_candidates = [
                    "/mnt/boot/grub/themes/Particle-circle-window/theme.txt",
                    "/mnt/boot/grub/themes/grub-theme/theme.txt",
                ]
                theme_found = next((t for t in theme_candidates if os.path.isfile(t)), None)
                if theme_found:
                    rel_theme = theme_found.replace("/mnt", "")
                    grub_params["GRUB_THEME"] = f'"{rel_theme}"'

                content = ""
                if os.path.isfile(grub_default):
                    with open(grub_default, "r") as f:
                        content = f.read()

                for key, val in grub_params.items():
                    pattern = rf"^#?\s*{re.escape(key)}=.*$"
                    if re.search(pattern, content, flags=re.MULTILINE):
                        content = re.sub(pattern, f"{key}={val}", content, flags=re.MULTILINE)
                    else:
                        content += f"\n{key}={val}\n"

                os.makedirs(os.path.dirname(grub_default), exist_ok=True)
                with open(grub_default, "w") as f:
                    f.write(content)

                # Deploy Recovery Entry to /etc/grub.d/15_pulsar_recovery
                grub_d = "/mnt/etc/grub.d"
                os.makedirs(grub_d, exist_ok=True)
                rec_script = f"""#!/bin/sh
exec tail -n +3 $0
# Pulsar OS Recovery Mode Menu Entry
menuentry "Pulsar OS Recovery" --class recovery --class os {{
    insmod btrfs
    insmod part_gpt
    insmod part_msdos
    insmod ext2
    search --no-floppy --fs-uuid --set=root {root_uuid}
    if [ -f /@/boot/vmlinuz-recovery ]; then
        linux /@/boot/vmlinuz-recovery boot=live components username=live autologin cow_spacesize=4G live-media-path=live fsck.mode=skip quiet splash
        initrd /@/boot/initramfs-recovery.img
    elif [ -f /boot/vmlinuz-recovery ]; then
        linux /boot/vmlinuz-recovery boot=live components username=live autologin cow_spacesize=4G live-media-path=live fsck.mode=skip quiet splash
        initrd /boot/initramfs-recovery.img
    elif [ -f /@/boot/vmlinuz-linux ]; then
        linux /@/boot/vmlinuz-linux root=UUID={root_uuid} rootflags=subvol=@ rw quiet splash single
        if [ -f /@/boot/initramfs-linux.img ]; then
            initrd /@/boot/initramfs-linux.img
        fi
    else
        linux /boot/vmlinuz-linux root=UUID={root_uuid} rw quiet splash single
        if [ -f /boot/initramfs-linux.img ]; then
            initrd /boot/initramfs-linux.img
        fi
    fi
}}
"""
                rec_script_path = f"{grub_d}/15_pulsar_recovery"
                with open(rec_script_path, "w") as f:
                    f.write(rec_script)
                os.chmod(rec_script_path, 0o755)

                # Run update-grub or grub-mkconfig
                GLib.idle_add(self.update_progress, 0.94, "Generating GRUB multi-boot configuration...")
                try:
                    if is_arch:
                        exec_cmd(["chroot", "/mnt", "grub-mkconfig", "-o", "/boot/grub/grub.cfg"])
                    else:
                        if os.path.exists("/mnt/usr/sbin/update-grub") or os.path.exists("/mnt/usr/bin/update-grub"):
                            exec_cmd(["chroot", "/mnt", "update-grub"])
                        else:
                            exec_cmd(["chroot", "/mnt", "grub-mkconfig", "-o", "/boot/grub/grub.cfg"])
                    log_msg("GRUB configuration updated with dual-boot (os-prober) and recovery support.")
                except Exception as g_cfg_err:
                    log_msg(f"Warning: GRUB config generation error: {g_cfg_err}")

            GLib.idle_add(self.update_progress, 0.90, "Installing bootloader...")
            exec_cmd(["mount", "--bind", "/dev", "/mnt/dev"])
            if "TEST_MODE" not in os.environ:
                os.makedirs("/mnt/dev/pts", exist_ok=True)
            exec_cmd(["mount", "-t", "devpts", "devpts", "/mnt/dev/pts"])
            exec_cmd(["mount", "--bind", "/proc", "/mnt/proc"])
            exec_cmd(["mount", "--rbind", "/sys", "/mnt/sys"])
            exec_cmd(["mount", "--make-rslave", "/mnt/sys"])
            if os.path.exists("/sys/firmware/efi/efivars"):
                os.makedirs("/mnt/sys/firmware/efi/efivars", exist_ok=True)
                subprocess.run(["mount", "-t", "efivarfs", "efivarfs", "/mnt/sys/firmware/efi/efivars"], capture_output=True)
            exec_cmd(["mount", "-t", "tmpfs", "tmpfs", "/mnt/run"])

            if is_efi:
                os.makedirs("/mnt/boot/efi", exist_ok=True)
                if not os.path.ismount("/mnt/boot/efi") and efi_part:
                    try:
                        exec_cmd(["mount", "-t", "vfat", efi_part, "/mnt/boot/efi"])
                        log_msg(f"Verified EFI partition mounted at /mnt/boot/efi: {efi_part}")
                    except Exception as me_err:
                        log_msg(f"Warning: Re-mounting EFI partition failed: {me_err}")

            refind_installed = False
            refind_available = any(
                os.path.exists(p)
                for p in (
                    "/mnt/usr/bin/refind-install",
                    "/mnt/usr/sbin/refind-install",
                    "/mnt/bin/refind-install",
                )
            )

            if is_efi and efi_part and refind_available:
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
                    try:
                        exec_cmd([live_refind_install, "--root", "/mnt", "--yes"])
                    except Exception as r_std_err:
                        log_msg(f"Notice: Standard refind-install failed ({r_std_err}), attempting --usedefault fallback...")
                        exec_cmd([live_refind_install, "--root", "/mnt", "--yes", "--usedefault", efi_part or disk_path])
                    refind_installed = True
                except Exception as ref_err:
                    print(f"Warning: refind-install failed: {ref_err}. Falling back to GRUB.")
                    if efi_part:
                        try:
                            exec_cmd(["chroot", "/mnt", "grub-install", "--target=x86_64-efi", "--efi-directory=/boot/efi", "--bootloader-id=PulsarOS", "--recheck"])
                        except Exception:
                            pass
                        exec_cmd(["chroot", "/mnt", "grub-install", "--target=x86_64-efi", "--efi-directory=/boot/efi", "--removable", "--recheck"])
                    else:
                        exec_cmd(["chroot", "/mnt", "grub-install", "--target=i386-pc", "--force", disk_path])
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
            elif is_efi and efi_part:
                GLib.idle_add(self.update_progress, 0.90, "Installing GRUB bootloader (UEFI)...")
                try:
                    exec_cmd(["chroot", "/mnt", "grub-install", "--target=x86_64-efi", "--efi-directory=/boot/efi", "--bootloader-id=PulsarOS", "--recheck"])
                except Exception as g_err:
                    print(f"Warning: Standard grub-install failed: {g_err}. Proceeding with removable fallback...")
                try:
                    exec_cmd(["chroot", "/mnt", "grub-install", "--target=x86_64-efi", "--efi-directory=/boot/efi", "--removable", "--recheck"])
                except Exception as g_rem_err:
                    print(f"Warning: Removable fallback grub-install: {g_rem_err}")
            else:
                GLib.idle_add(self.update_progress, 0.90, "Installing GRUB bootloader (BIOS/MBR)...")
                log_msg(f"Installing BIOS/MBR GRUB onto {disk_path}...")
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

                preserve_live_initramfs_for_recovery()

                try:
                    exec_cmd(["chroot", "/mnt", "mkinitcpio", "-P"])
                except Exception as mki_err:
                    log_msg(f"Warning: mkinitcpio -P failed (non-fatal): {mki_err}")

                deploy_kernel_to_recovery()

                if refind_installed:
                    configure_refind_menus()
                else:
                    configure_grub_menus()
            else:
                preserve_live_initramfs_for_recovery()
                deploy_kernel_to_recovery()
                if refind_installed:
                    configure_refind_menus()
                else:
                    configure_grub_menus()




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
