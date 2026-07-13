#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# ==============================================================================
# Pulsar OS - Recovery and Installation Selector UI (GTK4 & Libadwaita Sonoma Style)
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

from gi.repository import Gtk, Gdk, GLib, Adw, Gio

# Custom CSS for Apple macOS Look-and-Feel
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
}
.progress-text {
    font-size: 12px;
    color: #8e8e93;
}
"""

def get_system_disks():
    disks = []
    try:
        # Get disks with lsblk
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
                
                # Exclude loop and CD-ROM drives
                if dev_type == "disk" and not name.startswith("loop") and not name.startswith("sr"):
                    disks.append({
                        "path": f"/dev/{name}",
                        "name": f"/dev/{name} - {model} ({size})"
                    })
    except Exception as e:
        print(f"Error parsing lsblk: {e}")
        # Fallback from /proc/partitions
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


class RecoveryWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Recuperación de Pulsar OS")
        self.set_default_size(680, 500)
        self.set_resizable(False)
        
        # Apply Apple-style CSS
        self.apply_css()
        
        # Ultra-clean Title Bar with no window controls
        header_bar = Adw.HeaderBar()
        header_bar.set_show_end_title_buttons(False)
        header_bar.set_show_start_title_buttons(False)
        header_bar.add_css_class("header-bar")
        
        window_title = Adw.WindowTitle(title="Asistente de Recuperación")
        header_bar.set_title_widget(window_title)
        
        # Main layout
        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        vbox.append(header_bar)
        
        # Stack for panel navigation
        self.stack = Adw.ViewStack()
        vbox.append(self.stack)
        
        self.set_content(vbox)
        
        # Build view screens
        self.build_welcome_screen()
        self.build_installer_screen()
        
        # Present the welcome page first
        self.stack.set_visible_child_name("welcome")

    def apply_css(self):
        provider = Gtk.CssProvider()
        provider.load_from_data(CSS_DATA.encode('utf-8'))
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def build_welcome_screen(self):
        welcome_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        welcome_box.set_valign(Gtk.Align.CENTER)
        welcome_box.set_halign(Gtk.Align.CENTER)
        welcome_box.set_margin_top(40)
        welcome_box.set_margin_bottom(40)
        welcome_box.set_margin_start(40)
        welcome_box.set_margin_end(40)
        
        # Icon/Logo Loader
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
        welcome_box.append(image)
        
        # Heading
        title_label = Gtk.Label()
        title_label.set_markup("<span font_weight='bold'>Asistente de Pulsar OS</span>")
        title_label.add_css_class("welcome-title")
        welcome_box.append(title_label)
        
        # Subheading
        subtitle_label = Gtk.Label(label="Elige cómo deseas instalar Pulsar OS en tu equipo.")
        subtitle_label.add_css_class("welcome-subtitle")
        welcome_box.append(subtitle_label)
        
        # Buttons Box (Centered vertically)
        btn_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        btn_box.set_halign(Gtk.Align.CENTER)
        
        # Button 2: Quick Install (Apple Suggested Blue Button)
        btn_quick = Gtk.Button(label="Instalación Rápida Pulsar (Recomendado)")
        btn_quick.add_css_class("suggested-action")
        btn_quick.connect("clicked", self.on_quick_install_clicked)
        btn_box.append(btn_quick)

        # Button 1: Guided Install (Calamares)
        btn_guided = Gtk.Button(label="Instalación Guiada (Calamares)")
        btn_guided.add_css_class("secondary-action")
        btn_guided.connect("clicked", self.on_guided_install_clicked)
        btn_box.append(btn_guided)
        
        welcome_box.append(btn_box)
        
        self.stack.add_named(welcome_box, "welcome")

    def build_installer_screen(self):
        installer_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        installer_box.set_margin_top(40)
        installer_box.set_margin_bottom(40)
        installer_box.set_margin_start(50)
        installer_box.set_margin_end(50)
        
        # Title
        title_label = Gtk.Label()
        title_label.set_markup("<span font_weight='bold'>Instalación Rápida</span>")
        title_label.add_css_class("welcome-title")
        installer_box.append(title_label)
        
        # Subtitle
        subtitle_label = Gtk.Label(label="Selecciona el disco de almacenamiento para el volcado.")
        subtitle_label.add_css_class("welcome-subtitle")
        installer_box.append(subtitle_label)
        
        # Disk Selector Card (PreferencesGroup)
        pref_group = Adw.PreferencesGroup()
        pref_group.set_title("Utilidad de Discos")
        installer_box.append(pref_group)
        
        self.disk_row = Adw.ComboRow(
            title="Disco de Destino", 
            subtitle="Toda la información del disco seleccionado será borrada"
        )
        self.string_list = Gtk.StringList.new([])
        self.disk_row.set_model(self.string_list)
        pref_group.add(self.disk_row)
        
        # Populate system disks
        self.disks = get_system_disks()
        for d in self.disks:
            self.string_list.append(d["name"])
            
        # Progress indicator area
        self.progress_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.progress_box.set_margin_top(12)
        
        self.progress_label = Gtk.Label(label="Listo para instalar")
        self.progress_label.add_css_class("progress-text")
        self.progress_box.append(self.progress_label)
        
        self.progress_bar = Gtk.ProgressBar()
        self.progress_bar.add_css_class("progress-bar-thin")
        self.progress_bar.set_fraction(0.0)
        self.progress_box.append(self.progress_bar)
        
        installer_box.append(self.progress_box)
        
        # Navigation Bar
        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        nav_box.set_margin_top(24)
        
        # Back Button
        btn_back = Gtk.Button(label="Atrás")
        btn_back.add_css_class("secondary-action")
        btn_back.connect("clicked", lambda x: self.stack.set_visible_child_name("welcome"))
        nav_box.append(btn_back)
        
        # Spacer
        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        nav_box.append(spacer)
        
        # Install Button
        self.btn_install = Gtk.Button(label="Instalar Pulsar OS")
        self.btn_install.add_css_class("suggested-action")
        self.btn_install.connect("clicked", self.on_start_install_clicked)
        nav_box.append(self.btn_install)
        
        installer_box.append(nav_box)
        
        self.stack.add_named(installer_box, "installer")

    def on_guided_install_clicked(self, btn):
        print("Ejecutando Instalación Guiada (Calamares) en segundo plano...")
        subprocess.Popen(["sudo", "calamares"])
        self.close()

    def on_quick_install_clicked(self, btn):
        self.stack.set_visible_child_name("installer")

    def on_start_install_clicked(self, btn):
        idx = self.disk_row.get_selected()
        if idx < 0 or idx >= len(self.disks):
            self.show_error_dialog("Por favor, selecciona un disco de la lista.")
            return
            
        disk_path = self.disks[idx]["path"]
        
        # Lock controls
        self.btn_install.set_sensitive(False)
        self.disk_row.set_sensitive(False)
        
        # Launch backend thread
        threading.Thread(target=self.installation_backend, args=(disk_path,), daemon=True).start()

    def update_progress(self, fraction, text):
        self.progress_bar.set_fraction(fraction)
        self.progress_label.set_label(text)

    def show_error_dialog(self, message):
        dialog = Gtk.MessageDialog(
            transient_for=self,
            flags=Gtk.DialogFlags.MODAL,
            message_type=Gtk.MessageType.ERROR,
            buttons=Gtk.ButtonsType.OK,
            text="Error de Instalación"
        )
        dialog.format_secondary_text(message)
        dialog.connect("response", lambda d, r: d.destroy())
        dialog.present()

    def installation_backend(self, disk_path):
        try:
            def exec_cmd(cmd, shell=False):
                print(f"Running command: {cmd}")
                res = subprocess.run(cmd, shell=shell, capture_output=True, text=True)
                if res.returncode != 0:
                    raise Exception(f"Comando fallido: {' '.join(cmd) if isinstance(cmd, list) else cmd}\n{res.stderr}")
                return res.stdout
                
            # Check firmware boot mode: UEFI vs Legacy BIOS
            is_efi = os.path.exists("/sys/firmware/efi")
            print(f"Boot firmware mode detected: {'UEFI' if is_efi else 'Legacy BIOS'}")
            
            if is_efi:
                # UEFI Partitioning (GPT)
                GLib.idle_add(self.update_progress, 0.05, "Limpiando y particionando (GPT para UEFI)...")
                exec_cmd(["sgdisk", "--zap-all", disk_path])
                exec_cmd(["sgdisk", "--new=1:0:+512M", "--typecode=1:ef00", "--change-name=1:EFI", disk_path])
                exec_cmd(["sgdisk", "--new=2:0:0", "--typecode=2:8300", "--change-name=2:PulsarOS", disk_path])
                exec_cmd(["udevadm", "settle"])
                
                # Determine partitions
                if "nvme" in disk_path or "mmcblk" in disk_path or "loop" in disk_path:
                    efi_part = f"{disk_path}p1"
                    root_part = f"{disk_path}p2"
                else:
                    efi_part = f"{disk_path}1"
                    root_part = f"{disk_path}2"
                    
                # Format partitions
                GLib.idle_add(self.update_progress, 0.12, "Formateando particiones (EFI y ext4)...")
                exec_cmd(["mkfs.vfat", "-F32", efi_part])
                exec_cmd(["mkfs.ext4", "-F", root_part])
                
                # Mount system
                GLib.idle_add(self.update_progress, 0.18, "Montando sistema de archivos...")
                subprocess.run(["umount", "-l", "/mnt/boot/efi"])
                subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", root_part, "/mnt"])
                os.makedirs("/mnt/boot/efi", exist_ok=True)
                exec_cmd(["mount", efi_part, "/mnt/boot/efi"])
            else:
                # BIOS Partitioning (MBR / DOS)
                GLib.idle_add(self.update_progress, 0.05, "Limpiando y particionando (MBR para BIOS)...")
                # Wipe MBR sector
                exec_cmd(["dd", "if=/dev/zero", f"of={disk_path}", "bs=512", "count=1"])
                # Partition using sfdisk (single bootable partition)
                sfdisk_script = "label: dos\nsize=+, type=83, bootable\n"
                p = subprocess.Popen(["sfdisk", disk_path], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                p.communicate(input=sfdisk_script)
                exec_cmd(["udevadm", "settle"])
                
                # Determine partitions
                if "nvme" in disk_path or "mmcblk" in disk_path or "loop" in disk_path:
                    root_part = f"{disk_path}p1"
                else:
                    root_part = f"{disk_path}1"
                    
                # Format partitions
                GLib.idle_add(self.update_progress, 0.12, "Formateando partición raíz (ext4)...")
                exec_cmd(["mkfs.ext4", "-F", root_part])
                
                # Mount system
                GLib.idle_add(self.update_progress, 0.18, "Montando sistema de archivos...")
                subprocess.run(["umount", "-l", "/mnt"])
                os.makedirs("/mnt", exist_ok=True)
                exec_cmd(["mount", root_part, "/mnt"])
            
            # Step 4: System Replication (rsync)
            GLib.idle_add(self.update_progress, 0.25, "Copiando archivos del sistema... (25%)")
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
                    f"Copiando archivos del sistema... ({progress_fraction}%)"
                )
                time.sleep(2)
                
            proc.wait()
            if proc.returncode != 0:
                raise Exception(f"Replicación rsync fallida (código {proc.returncode})\n{proc.stderr.read()}")
                
            # Step 5: Boot configurations (fstab)
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
                
            # Step 6: GRUB Installation inside chroot
            GLib.idle_add(self.update_progress, 0.90, "Instalando cargador de arranque GRUB...")
            exec_cmd(["mount", "--bind", "/dev", "/mnt/dev"])
            exec_cmd(["mount", "--bind", "/proc", "/mnt/proc"])
            exec_cmd(["mount", "--bind", "/sys", "/mnt/sys"])
            exec_cmd(["mount", "--bind", "/run", "/mnt/run"])
            
            if is_efi:
                exec_cmd(["chroot", "/mnt", "grub-install", disk_path])
                # Dual boot support: if rEFInd is available in the cloned package database, run its postinst to configure it
                refind_postinst = "/mnt/var/lib/dpkg/info/pulsaros-refind.postinst"
                if os.path.exists(refind_postinst):
                    GLib.idle_add(self.update_progress, 0.92, "Configurando gestor de arranque dual rEFInd...")
                    try:
                        exec_cmd(["chroot", "/mnt", "/var/lib/dpkg/info/pulsaros-refind.postinst", "configure"])
                    except Exception as ref_err:
                        print(f"Warning: rEFInd dual-boot setup encountered an issue: {ref_err}. Falling back to GRUB.")
            else:
                # Force BIOS/i386-pc installation
                exec_cmd(["chroot", "/mnt", "grub-install", "--target=i386-pc", disk_path])
                
            exec_cmd(["chroot", "/mnt", "update-grub"])
            
            # Clean bind mounts
            subprocess.run(["umount", "-l", "/mnt/dev"])
            subprocess.run(["umount", "-l", "/mnt/proc"])
            subprocess.run(["umount", "-l", "/mnt/sys"])
            subprocess.run(["umount", "-l", "/mnt/run"])
            
            # Step 7: Flag witness file for OOTB assistant
            GLib.idle_add(self.update_progress, 0.95, "Creando flag de primer arranque...")
            exec_cmd(["touch", "/mnt/etc/pulsar-need-setup"])
            
            # Final unmounting
            if is_efi:
                subprocess.run(["umount", "-l", "/mnt/boot/efi"])
            subprocess.run(["umount", "-l", "/mnt"])
            
            # Successful end
            GLib.idle_add(self.on_installation_completed)
            
        except Exception as err:
            # Clean bind mounts in case of failure
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
        self.btn_install.set_label("Reiniciar Sistema")
        self.btn_install.set_sensitive(True)
        # Re-bind callback to reboot
        self.btn_install.disconnect_by_func(self.on_start_install_clicked)
        self.btn_install.connect("clicked", self.on_reboot_system)

    def on_reboot_system(self, btn):
        subprocess.Popen(["systemctl", "reboot"])
        self.close()

    def on_installation_failed(self, error):
        self.update_progress(0.0, "Fallo en la instalación.")
        self.show_error_dialog(error)
        self.btn_install.set_sensitive(True)
        self.disk_row.set_sensitive(True)


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
