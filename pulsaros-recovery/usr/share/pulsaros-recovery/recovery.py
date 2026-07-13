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
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
gi.require_version('GdkPixbuf', '2.0')

from gi.repository import Gtk, Gdk, GLib, Adw, Gio, GdkPixbuf

# Custom CSS for Apple macOS Recovery and Installer Look-and-Feel
CSS_DATA = """
window {
    background-color: #1e1e1e; /* dark theme base */
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
.macos-window {
    border-radius: 20px;
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
.header-bar {
    background-color: #1e1e1e;
    border-bottom: 1px solid #2d2d2d;
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
.utility-row-box {
    padding: 12px;
    border-bottom: 1px solid #3a3a3c;
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
                                "name": f"/dev/{name} (Desconocido)"
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
        
        # HDD icon
        icon = Gtk.Image.new_from_icon_name("drive-harddisk-symbolic")
        icon.set_pixel_size(48)
        self.append(icon)
        
        # Disk name
        name_lbl = Gtk.Label(label=disk_info["path"].replace("/dev/", ""))
        name_lbl.add_css_class("disk-name")
        self.append(name_lbl)
        
        # Disk size
        size_match = re.search(r"\(([^)]+)\)", disk_info["name"])
        size_str = size_match.group(1) if size_match else "Desconocido"
        
        details_lbl = Gtk.Label(label=f"{size_str} total\nDisponible")
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
        self.set_title("Recuperación de Pulsar OS")
        self.set_default_size(700, 520)
        self.set_resizable(False)
        
        self.apply_css()
        
        # Header bar
        header_bar = Adw.HeaderBar()
        header_bar.set_show_end_title_buttons(False)
        header_bar.set_show_start_title_buttons(False)
        header_bar.add_css_class("header-bar")
        
        window_title = Adw.WindowTitle(title="Recuperación")
        header_bar.set_title_widget(window_title)
        
        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        vbox.append(header_bar)
        
        self.stack = Adw.ViewStack()
        vbox.append(self.stack)
        
        self.set_content(vbox)
        
        # Build views
        self.build_utilities_screen()
        self.build_install_selector_screen()
        
        # New Welcome and Disk Selection screens matching Apple Monterey/Sequoia style
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
        
        if icon_name == "logo":
            logo_path = "/usr/share/pulsaros-recovery/logo.png"
            logo_fallback = "/usr/share/pulsaros-recovery/pulsar-logo.png"
            if os.path.exists(logo_path):
                try:
                    pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 42, 42, True)
                    texture = Gdk.Texture.new_for_pixbuf(pixbuf)
                    img.set_from_paintable(texture)
                except:
                    img.set_from_file(logo_path)
            elif os.path.exists(logo_fallback):
                try:
                    pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_fallback, 42, 42, True)
                    texture = Gdk.Texture.new_for_pixbuf(pixbuf)
                    img.set_from_paintable(texture)
                except:
                    img.set_from_file(logo_fallback)
            else:
                img.set_from_icon_name("system-software-install-symbolic")
        elif icon_name == "timemachine":
            img.set_from_icon_name("document-revert-symbolic")
        elif icon_name == "safari":
            img.set_from_icon_name("web-browser-symbolic")
        else:
            img.set_from_icon_name("gnome-disks")
            
        return img

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
        screen_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        screen_box.set_margin_top(30)
        screen_box.set_margin_bottom(30)
        screen_box.set_margin_start(40)
        screen_box.set_margin_end(40)
        screen_box.set_valign(Gtk.Align.CENTER)
        
        card_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        card_box.add_css_class("apple-box")
        screen_box.append(card_box)
        
        self.listbox = Gtk.ListBox()
        self.listbox.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.listbox.connect("row-selected", self.on_utility_row_selected)
        card_box.append(self.listbox)
        
        self.add_utility_row(self.listbox, "backup", "Restaurar desde Copia de Seguridad", 
                             "Restaura tu instalación de Pulsar OS a partir de una copia de seguridad (Deja Dup).", "timemachine")
        self.add_utility_row(self.listbox, "install", "Instalar Pulsar OS", 
                             "Instala una copia del sistema operativo Pulsar OS en tu ordenador.", "logo")
        self.add_utility_row(self.listbox, "safari", "Seafari Browser", 
                             "Navega por la web para encontrar guías de configuración y soporte en línea.", "safari")
        self.add_utility_row(self.listbox, "disk", "Utilidad de Discos", 
                             "Modifica, formatea o comprueba tus unidades de almacenamiento conectadas.", "disk")
                             
        bottom_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        bottom_box.set_margin_top(16)
        
        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        bottom_box.append(spacer)
        
        self.btn_continue = Gtk.Button(label="Continuar")
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

    def on_utility_continue_clicked(self, btn):
        if not self.selected_action:
            return
            
        if self.selected_action == "backup":
            subprocess.Popen("deja-dup --restore || deja-dup", shell=True)
        elif self.selected_action == "install":
            self.stack.set_visible_child_name("install_selector")
        elif self.selected_action == "safari":
            subprocess.Popen("seafari || firefox || xdg-open https://google.com", shell=True)
        elif self.selected_action == "disk":
            subprocess.Popen("gnome-disks || gnome-disk-utility", shell=True)

    def build_install_selector_screen(self):
        selector_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        selector_box.set_valign(Gtk.Align.CENTER)
        selector_box.set_halign(Gtk.Align.CENTER)
        selector_box.set_margin_top(40)
        selector_box.set_margin_bottom(40)
        selector_box.set_margin_start(40)
        selector_box.set_margin_end(40)
        
        logo_path = "/usr/share/pulsaros-recovery/logo.png"
        logo_fallback = "/usr/share/pulsaros-recovery/pulsar-logo.png"
        
        image = Gtk.Image()
        if os.path.exists(logo_path):
            image.set_from_file(logo_path)
        elif os.path.exists(logo_fallback):
            image.set_from_file(logo_fallback)
        else:
            image.set_from_icon_name("system-software-install-symbolic")
        image.set_pixel_size(96)
        selector_box.append(image)
        
        title_label = Gtk.Label()
        title_label.set_markup("<span font_weight='bold'>Asistente de Pulsar OS</span>")
        title_label.add_css_class("welcome-title")
        selector_box.append(title_label)
        
        subtitle_label = Gtk.Label(label="Elige cómo deseas instalar Pulsar OS en tu equipo.")
        subtitle_label.add_css_class("welcome-subtitle")
        selector_box.append(subtitle_label)
        
        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        btn_box.set_halign(Gtk.Align.CENTER)
        
        btn_quick = Gtk.Button(label="Instalación Rápida Pulsar (Recomendado)")
        btn_quick.add_css_class("suggested-action")
        btn_quick.connect("clicked", lambda x: self.stack.set_visible_child_name("install_welcome"))
        btn_box.append(btn_quick)

        btn_guided = Gtk.Button(label="Instalación Guiada (Calamares)")
        btn_guided.add_css_class("secondary-action")
        btn_guided.connect("clicked", self.on_guided_install_clicked)
        btn_box.append(btn_guided)
        
        selector_box.append(btn_box)
        
        back_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        btn_back = Gtk.Button(label="Atrás")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("utilities"))
        back_box.append(btn_back)
        selector_box.append(back_box)
        
        self.stack.add_named(selector_box, "install_selector")

    def build_install_welcome_screen(self):
        # Corresponds to macOS Monterey Install welcome screen
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        # Centered round logo
        logo_path = "/usr/share/pulsaros-recovery/logo.png"
        logo_fallback = "/usr/share/pulsaros-recovery/pulsar-logo.png"
        image = Gtk.Image()
        if os.path.exists(logo_path):
            image.set_from_file(logo_path)
        elif os.path.exists(logo_fallback):
            image.set_from_file(logo_fallback)
        else:
            image.set_from_icon_name("system-software-install-symbolic")
        image.set_pixel_size(110)
        box.append(image)
        
        # Title "Pulsar OS"
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='22000'>Pulsar OS</span>")
        box.append(title)
        
        # Subtext
        subtext = Gtk.Label(label="Para configurar la instalación de Pulsar OS, haz clic en Continuar.")
        subtext.add_css_class("welcome-subtitle")
        subtext.set_margin_top(8)
        subtext.set_margin_bottom(20)
        box.append(subtext)
        
        # Navigation buttons
        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=20)
        nav_box.set_halign(Gtk.Align.CENTER)
        
        btn_back = Gtk.Button(label="Atrás")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("install_selector"))
        nav_box.append(btn_back)
        
        btn_continue = Gtk.Button(label="Continuar")
        btn_continue.add_css_class("suggested-action")
        btn_continue.connect("clicked", self.on_welcome_continue_clicked)
        nav_box.append(btn_continue)
        
        box.append(nav_box)
        self.stack.add_named(box, "install_welcome")

    def on_welcome_continue_clicked(self, btn):
        # Refresh and rebuild disk list before showing selection screen
        self.refresh_disk_cards()
        self.stack.set_visible_child_name("install_disk_select")

    def build_install_disk_select_screen(self):
        # Corresponds to macOS Sequoia Target Disk select screen
        self.disk_select_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        self.disk_select_box.set_valign(Gtk.Align.CENTER)
        self.disk_select_box.set_halign(Gtk.Align.CENTER)
        self.disk_select_box.set_margin_start(40)
        self.disk_select_box.set_margin_end(40)
        
        logo_path = "/usr/share/pulsaros-recovery/logo.png"
        logo_fallback = "/usr/share/pulsaros-recovery/pulsar-logo.png"
        image = Gtk.Image()
        if os.path.exists(logo_path):
            image.set_from_file(logo_path)
        elif os.path.exists(logo_fallback):
            image.set_from_file(logo_fallback)
        else:
            image.set_from_icon_name("system-software-install-symbolic")
        image.set_pixel_size(90)
        self.disk_select_box.append(image)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='18000'>Pulsar OS</span>")
        self.disk_select_box.append(title)
        
        self.disk_select_subtitle = Gtk.Label(label="Pulsar OS se instalará en el disco seleccionado.")
        self.disk_select_subtitle.add_css_class("welcome-subtitle")
        self.disk_select_box.append(self.disk_select_subtitle)
        
        # Horizontal container for Disk Cards
        self.disk_cards_flow = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        self.disk_cards_flow.set_halign(Gtk.Align.CENTER)
        self.disk_select_box.append(self.disk_cards_flow)
        
        # Nav buttons
        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=20)
        nav_box.set_halign(Gtk.Align.CENTER)
        nav_box.set_margin_top(20)
        
        btn_back = Gtk.Button(label="Atrás")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("install_welcome"))
        nav_box.append(btn_back)
        
        self.btn_disk_continue = Gtk.Button(label="Continuar")
        self.btn_disk_continue.add_css_class("suggested-action")
        self.btn_disk_continue.set_sensitive(False)
        self.btn_disk_continue.connect("clicked", self.on_disk_continue_clicked)
        nav_box.append(self.btn_disk_continue)
        
        self.disk_select_box.append(nav_box)
        self.stack.add_named(self.disk_select_box, "install_disk_select")

    def refresh_disk_cards(self):
        # Clear existing disk cards
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
        # Clear previous selection style
        for card in self.disk_cards:
            card.remove_css_class("selected")
            
        # Select clicked card
        selected_card.add_css_class("selected")
        self.selected_disk_card = selected_card
        
        # Update UI subtitle details
        disk_path = selected_card.disk_info["path"].replace("/dev/", "")
        self.disk_select_subtitle.set_label(f"Pulsar OS se instalará en el disco \"{disk_path}\".")
        self.btn_disk_continue.set_sensitive(True)

    def on_disk_continue_clicked(self, btn):
        if not self.selected_disk_card:
            return
            
        disk_path = self.selected_disk_card.disk_info["path"]
        disk_name = disk_path.replace("/dev/", "")
        
        # Set text on progress screen
        self.progress_subtitle.set_label(f"Pulsar OS se está instalando en el disco \"{disk_name}\".")
        self.stack.set_visible_child_name("install_progress")
        
        # Start installation backend
        threading.Thread(target=self.installation_backend, args=(disk_path,), daemon=True).start()

    def build_install_progress_screen(self):
        # Corresponds to macOS Monterey Progress screen
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        box.set_margin_start(45)
        box.set_margin_end(45)
        
        logo_path = "/usr/share/pulsaros-recovery/logo.png"
        logo_fallback = "/usr/share/pulsaros-recovery/pulsar-logo.png"
        image = Gtk.Image()
        if os.path.exists(logo_path):
            image.set_from_file(logo_path)
        elif os.path.exists(logo_fallback):
            image.set_from_file(logo_fallback)
        else:
            image.set_from_icon_name("system-software-install-symbolic")
        image.set_pixel_size(100)
        box.append(image)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='20000'>Pulsar OS</span>")
        box.append(title)
        
        self.progress_subtitle = Gtk.Label(label="Pulsar OS se instalará en el disco.")
        self.progress_subtitle.add_css_class("progress-text")
        box.append(self.progress_subtitle)
        
        # Thin Apple-style blue progress bar
        self.progress_bar = Gtk.ProgressBar()
        self.progress_bar.add_css_class("progress-bar-thin")
        self.progress_bar.set_size_request(320, -1)
        box.append(self.progress_bar)
        
        # Status details label
        self.progress_label = Gtk.Label(label="Preparando instalación...")
        self.progress_label.add_css_class("progress-text")
        box.append(self.progress_label)
        
        # Cancel / Restart Button
        self.btn_install_action = Gtk.Button(label="Cancelar")
        self.btn_install_action.add_css_class("secondary-action")
        self.btn_install_action.set_margin_top(16)
        self.btn_install_action.connect("clicked", self.on_progress_cancel_clicked)
        box.append(self.btn_install_action)
        
        self.stack.add_named(box, "install_progress")

    def on_progress_cancel_clicked(self, btn):
        # If successfully completed, this button changes label to "Reiniciar" and triggers reboot
        if btn.get_label() == "Reiniciar Sistema":
            subprocess.Popen(["systemctl", "reboot"])
            self.close()
        else:
            # Cancel installer and return to disk selector (only if safe/before critical steps)
            self.stack.set_visible_child_name("install_disk_select")

    def installation_backend(self, disk_path):
        try:
            def exec_cmd(cmd, shell=False):
                print(f"Running command: {cmd}")
                res = subprocess.run(cmd, shell=shell, capture_output=True, text=True)
                if res.returncode != 0:
                    raise Exception(f"Comando fallido: {' '.join(cmd) if isinstance(cmd, list) else cmd}\n{res.stderr}")
                return res.stdout
                
            is_efi = os.path.exists("/sys/firmware/efi")
            
            if is_efi:
                GLib.idle_add(self.update_progress, 0.05, "Limpiando y particionando (GPT para UEFI)...")
                exec_cmd(["sgdisk", "--zap-all", disk_path])
                exec_cmd(["sgdisk", "--new=1:0:+512M", "--typecode=1:ef00", "--change-name=1:EFI", disk_path])
                exec_cmd(["sgdisk", "--new=2:0:0", "--typecode=2:8300", "--change-name=2:PulsarOS", disk_path])
                exec_cmd(["udevadm", "settle"])
                
                if "nvme" in disk_path or "mmcblk" in disk_path or "loop" in disk_path:
                    efi_part = f"{disk_path}p1"
                    root_part = f"{disk_path}p2"
                else:
                    efi_part = f"{disk_path}1"
                    root_part = f"{disk_path}2"
                    
                GLib.idle_add(self.update_progress, 0.12, "Formateando particiones (EFI y ext4)...")
                exec_cmd(["mkfs.vfat", "-F32", efi_part])
                exec_cmd(["mkfs.ext4", "-F", root_part])
                
                GLib.idle_add(self.update_progress, 0.18, "Montando sistema de archivos...")
                subprocess.run(["umount", "-l", "/mnt/boot/efi"])
                subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", root_part, "/mnt"])
                os.makedirs("/mnt/boot/efi", exist_ok=True)
                exec_cmd(["mount", efi_part, "/mnt/boot/efi"])
            else:
                GLib.idle_add(self.update_progress, 0.05, "Limpiando y particionando (MBR para BIOS)...")
                exec_cmd(["dd", "if=/dev/zero", f"of={disk_path}", "bs=512", "count=1"])
                sfdisk_script = "label: dos\nsize=+, type=83, bootable\n"
                p = subprocess.Popen(["sfdisk", disk_path], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                p.communicate(input=sfdisk_script)
                exec_cmd(["udevadm", "settle"])
                
                if "nvme" in disk_path or "mmcblk" in disk_path or "loop" in disk_path:
                    root_part = f"{disk_path}p1"
                else:
                    root_part = f"{disk_path}1"
                    
                GLib.idle_add(self.update_progress, 0.12, "Formateando partición raíz (ext4)...")
                exec_cmd(["mkfs.ext4", "-F", root_part])
                
                GLib.idle_add(self.update_progress, 0.18, "Montando sistema de archivos...")
                subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", root_part, "/mnt"])
            
            GLib.idle_add(self.update_progress, 0.25, "Replicando archivos... (esto puede tardar)")
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
                    f"Instalando archivos... ({progress_fraction}%)"
                )
                time.sleep(2)
                
            proc.wait()
            if proc.returncode != 0:
                raise Exception(f"Replicación rsync fallida (código {proc.returncode})\n{proc.stderr.read()}")
                
            GLib.idle_add(self.update_progress, 0.85, "Configurando arranque (fstab)...")
            def get_partition_uuid(part):
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
            os.makedirs("/mnt/etc", exist_ok=True)
            with open("/mnt/etc/fstab", "w") as f:
                f.write(fstab_content)
                
            GLib.idle_add(self.update_progress, 0.90, "Instalando cargador de arranque GRUB...")
            exec_cmd(["mount", "--bind", "/dev", "/mnt/dev"])
            exec_cmd(["mount", "--bind", "/proc", "/mnt/proc"])
            exec_cmd(["mount", "--bind", "/sys", "/mnt/sys"])
            exec_cmd(["mount", "--bind", "/run", "/mnt/run"])
            
            if is_efi:
                exec_cmd(["chroot", "/mnt", "grub-install", disk_path])
                refind_postinst = "/mnt/var/lib/dpkg/info/pulsaros-refind.postinst"
                if os.path.exists(refind_postinst):
                    GLib.idle_add(self.update_progress, 0.92, "Configurando arranque dual rEFInd...")
                    try:
                        exec_cmd(["chroot", "/mnt", "/var/lib/dpkg/info/pulsaros-refind.postinst", "configure"])
                    except Exception as ref_err:
                        print(f"Warning: rEFInd dual-boot setup encountered an issue: {ref_err}. Falling back to GRUB.")
            else:
                exec_cmd(["chroot", "/mnt", "grub-install", "--target=i386-pc", disk_path])
                
            exec_cmd(["chroot", "/mnt", "update-grub"])
            
            subprocess.run(["umount", "-l", "/mnt/dev"])
            subprocess.run(["umount", "-l", "/mnt/proc"])
            subprocess.run(["umount", "-l", "/mnt/sys"])
            subprocess.run(["umount", "-l", "/mnt/run"])
            
            GLib.idle_add(self.update_progress, 0.95, "Creando flag de primer arranque...")
            exec_cmd(["touch", "/mnt/etc/pulsar-need-setup"])
            
            if is_efi:
                subprocess.run(["umount", "-l", "/mnt/boot/efi"])
            subprocess.run(["umount", "-l", "/mnt"])
            
            GLib.idle_add(self.on_installation_completed)
            
        except Exception as err:
            subprocess.run(["umount", "-l", "/mnt/dev"])
            subprocess.run(["umount", "-l", "/mnt/proc"])
            subprocess.run(["umount", "-l", "/mnt/sys"])
            subprocess.run(["umount", "-l", "/mnt/run"])
            if is_efi:
                subprocess.run(["umount", "-l", "/mnt/boot/efi"])
            subprocess.run(["umount", "-l", "/mnt"])
            
            GLib.idle_add(self.on_installation_failed, str(err))

    def on_installation_completed(self):
        self.update_progress(1.0, "¡Pulsar OS se ha instalado correctamente!")
        self.btn_install_action.set_label("Reiniciar Sistema")
        self.btn_install_action.add_css_class("suggested-action")
        self.btn_install_action.remove_css_class("secondary-action")

    def on_installation_failed(self, error):
        self.update_progress(0.0, "Fallo en la instalación.")
        self.show_error_dialog(error)
        self.stack.set_visible_child_name("install_disk_select")


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
