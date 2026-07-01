#!/usr/bin/env python3
# ==============================================================================
# Pulsar OS - Recovery and Installation Selector UI (GTK3 Native macOS Style)
# ==============================================================================
# English: Python script that manages the macOS-style Recovery and disk selector interface.
#          Allows running Gnome Disk Utility, Seafari web browser, or starting the installation
#          process. Passes the selected disk to Calamares and launches it (English only).
# Español: Script en Python que gestiona la interfaz de recuperación y selección de disco
#          estilo macOS. Permite lanzar la utilidad de discos de Gnome, el navegador Seafari o
#          iniciar el proceso de instalación. Pasa el disco seleccionado a Calamares y lo lanza (en inglés).

import json
import os
import subprocess
import sys

import gi

# Requerir versiones específicas de GTK, Gdk y GdkPixbuf para evitar conflictos
# Require specific versions of GTK, Gdk, and GdkPixbuf to prevent conflicts
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
gi.require_version("GdkPixbuf", "2.0")
from gi.repository import Gdk, GdkPixbuf, GLib, Gtk

CSS_DATA = """
window {
    background-color: #101010;
}

list, row {
    background-color: transparent;
    border: none;
    box-shadow: none;
}

/* Barra superior de macOS / macOS Top Bar */
.top-bar {
    background-color: #0c0c0d;
    border-bottom: 1px solid #1a1a1c;
    padding: 0 16px;
    min-height: 24px;
}

.top-bar-menu-item {
    color: #e5e5e7;
    font-size: 12px;
    font-weight: 600;
    margin-right: 18px;
}

.top-bar-clock {
    color: #e5e5e7;
    font-size: 12px;
    font-weight: 500;
}

/* Caja del Recovery Utilities / Recovery Utilities Box */
.recovery-box {
    background-color: #242426;
    border: 1px solid #333335;
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
    background-color: #2e2e30;
}

.recovery-row:selected {
    background-color: #0066cc;
    color: #ffffff;
}

.recovery-title {
    font-size: 13px;
    font-weight: 700;
    color: #ffffff;
}

.recovery-row:selected .recovery-title {
    color: #ffffff;
}

.recovery-desc {
    font-size: 11px;
    color: #8a8a8e;
}

.recovery-row:selected .recovery-desc {
    color: #d1d1d6;
}

/* Caja del Instalador / Installer box (Slides 1 & 2) */
.installer-box {
    background-color: #1e1e1e;
    border: 1px solid #2d2d2d;
    border-radius: 12px;
    padding: 40px 60px;
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
    color: #8a8a8e;
    margin-bottom: 24px;
}

/* Tarjeta de disco / Disk Card */
.disk-card {
    background-color: #242426;
    border: 1px solid #333335;
    border-radius: 10px;
    padding: 16px;
    min-width: 130px;
    margin: 0 8px;
    transition: all 0.15s ease;
}

.disk-card:hover {
    background-color: #2e2e30;
    border-color: #444446;
}

.disk-card.selected {
    background-color: #2e2e30;
    border-color: #0066cc;
    box-shadow: 0 0 0 2px #0066cc;
}

.disk-name {
    font-size: 12px;
    font-weight: 700;
    color: #ffffff;
    margin-top: 8px;
}

.disk-info {
    font-size: 10px;
    color: #8a8a8e;
}

/* Botones / Buttons */
button {
    font-size: 13px;
    font-weight: 500;
    padding: 6px 16px;
    border-radius: 6px;
    outline: none;
    transition: all 0.15s ease;
}

button.action-btn {
    background-image: none;
    background-color: #323234;
    border: 1px solid #444446;
    color: #ffffff;
}

button.action-btn:hover {
    background-color: #3e3e40;
}

button.action-btn:disabled {
    opacity: 0.3;
    color: #8a8a8e;
}

button.btn-continue {
    background-image: none;
    background-color: #0066cc;
    border: none;
    color: #ffffff;
    font-weight: 700;
    box-shadow: 0 1px 2px rgba(0,0,0,0.1);
}

button.btn-continue:hover {
    background-color: #0077ed;
}

button.btn-continue:active {
    background-color: #005bb5;
}

button.btn-continue:disabled {
    background-color: #323234;
    color: #8a8a8e;
    box-shadow: none;
}
"""


def get_physical_disks():
    """
    English: Queries real storage devices using lsblk. Returns an empty list on failure or if no disks found.
    Español: Consulta los dispositivos de almacenamiento reales mediante lsblk. Devuelve una lista vacía si falla o si no encuentra discos.
    """
    try:
        output = subprocess.check_output(
            "lsblk -J -d -o NAME,MODEL,SIZE,TYPE,RO", shell=True, text=True
        )
        data = json.loads(output)
        disks = []
        for dev in data.get("blockdevices", []):
            # Exclude read-only devices like live CD /dev/sr0
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
    """
    English: Runs a command as the real, non-root user who invoked pkexec or sudo,
             preserving the graphical display variables.
    Español: Ejecuta un comando como el usuario real no-root que invocó pkexec o sudo,
             preservando las variables de entorno de pantalla gráfica.
    """
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

                # Command wrapping with sudo -u
                env_str = f'DISPLAY="{display}"'
                if xauth:
                    env_str += f' XAUTHORITY="{xauth}"'
                if wayland:
                    env_str += f' WAYLAND_DISPLAY="{wayland}"'
                if xdg_runtime:
                    env_str += f' XDG_RUNTIME_DIR="{xdg_runtime}"'

                full_cmd = f"sudo -u {username} env {env_str} {cmd}"
                print(f"Running command as user {username}: {full_cmd}")
                if wait:
                    return subprocess.run(full_cmd, shell=True)
                return subprocess.Popen(full_cmd, shell=True)
            except Exception as e:
                print("Failed to run command as user, executing normal fallback:", e)

    if wait:
        return subprocess.run(cmd, shell=True)
    return subprocess.Popen(cmd, shell=True)


class RecoveryApp(Gtk.Window):
    def __init__(self):
        super().__init__(type=Gtk.WindowType.TOPLEVEL)
        self.set_title("Pulsar OS Recovery")
        self.fullscreen()
        self.selected_utility = None
        self.selected_disk_path = None

        # Load Custom GTK CSS
        style_provider = Gtk.CssProvider()
        style_provider.load_from_data(CSS_DATA.encode())
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            style_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        )

        # Main Layout
        main_vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.add(main_vbox)

        # ==============================================================================
        # CENTER CONTENT AREA (Gtk.Stack)
        # ==============================================================================
        center_align = Gtk.Alignment.new(0.5, 0.5, 0, 0)
        main_vbox.pack_start(center_align, True, True, 0)

        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self.stack.set_transition_duration(300)
        center_align.add(self.stack)

        self.init_slides()

        # Connect destroy
        self.connect("destroy", Gtk.main_quit)
        self.show_all()

    def init_slides(self):
        # ----------------------------------------------------------------------
        # Slide 0: Recovery Utilities (Imagen 1)
        # ----------------------------------------------------------------------
        slide_0 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        slide_0.get_style_context().add_class("recovery-box")
        slide_0.set_size_request(540, 420)
        self.stack.add_named(slide_0, "recovery")

        # Titulo de cabecera de la caja (decorativo o limpio)
        header_lbl = Gtk.Label(label="Pulsar OS Utilities")
        header_lbl.get_style_context().add_class("recovery-title")
        header_lbl.set_margin_bottom(12)
        slide_0.pack_start(header_lbl, False, False, 0)

        # ListBox de utilidades
        self.util_list = Gtk.ListBox()
        self.util_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.util_list.connect("row-selected", self.on_utility_row_selected)
        self.util_list.connect("row-activated", self.on_utility_row_activated)
        slide_0.pack_start(self.util_list, True, True, 8)

        # English: Check if Calamares or setup assistant is available (indicating live ISO installer environment)
        # Español: Comprobar si Calamares o el asistente de instalación están disponibles (indica entorno live de instalador)
        self.is_live = os.path.exists("/usr/local/bin/launch-calamares") or os.path.exists("/usr/bin/calamares")

        # Añadir las utilidades / Add utilities
        self.add_utility_row(
            "Restore from Backup",
            "Restore your Pulsar OS installation from a local system backup.",
            "timemachine",
        )
        if self.is_live:
            self.add_utility_row(
                "Install Pulsar OS",
                "Install a new copy of the Pulsar OS desktop on your computer.",
                "logo",  # Cargará el logo físico
            )
        self.add_utility_row(
            "Seafari Browser",
            "Browse the web to search for online support and configuration guides.",
            "safari",
        )
        self.add_utility_row(
            "Disk Utility",
            "Partition, format, or check your connected storage drives.",
            "gnome-disk-utility",
        )

        # Botones inferiores del Recovery
        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        btn_box.set_margin_top(16)
        slide_0.pack_end(btn_box, False, False, 0)

        # Botón a la izquierda / Left action button (Try System or Close)
        if self.is_live:
            self.btn_try_system = Gtk.Button(label="Try System")
        else:
            self.btn_try_system = Gtk.Button(label="Close")
        self.btn_try_system.get_style_context().add_class("action-btn")
        self.btn_try_system.connect("clicked", lambda b: Gtk.main_quit())
        btn_box.pack_start(self.btn_try_system, False, False, 0)

        # Botón "Continue" a la derecha
        self.btn_util_continue = Gtk.Button(label="Continue")
        self.btn_util_continue.get_style_context().add_class("btn-continue")
        self.btn_util_continue.set_sensitive(False)
        self.btn_util_continue.connect("clicked", self.on_utility_continue_clicked)
        btn_box.pack_end(self.btn_util_continue, False, False, 0)

        # ----------------------------------------------------------------------
        # Slide 1: Welcome Installer Screen (Imagen 2)
        # ----------------------------------------------------------------------
        slide_1 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        slide_1.get_style_context().add_class("installer-box")
        slide_1.set_size_request(680, 480)
        self.stack.add_named(slide_1, "welcome")

        # Logo redondo grande
        logo_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        logo_box.set_halign(Gtk.Align.CENTER)
        logo_box.set_margin_top(30)
        logo_box.set_margin_bottom(12)
        slide_1.pack_start(logo_box, False, False, 0)

        logo_path = "/usr/share/pulsaros-recovery/pulsar-logo.png"
        if not os.path.exists(logo_path):
            curr_dir = os.path.dirname(os.path.abspath(__file__))
            logo_path = os.path.join(curr_dir, "pulsar-logo.png")

        if os.path.exists(logo_path):
            pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 150, 150, True)
            logo_img = Gtk.Image.new_from_pixbuf(pixbuf)
        else:
            logo_img = Gtk.Image.new_from_icon_name("computer", Gtk.IconSize.DIALOG)
        logo_box.pack_start(logo_img, True, True, 0)

        # Titulo y desc centrado
        lbl_welcome_title = Gtk.Label()
        lbl_welcome_title.get_style_context().add_class("installer-title")
        lbl_welcome_title.set_text("Pulsar OS")
        lbl_welcome_title.set_halign(Gtk.Align.CENTER)
        slide_1.pack_start(lbl_welcome_title, False, False, 0)

        lbl_welcome_desc = Gtk.Label(
            label="To set up the installation of Pulsar OS, click Continue."
        )
        lbl_welcome_desc.get_style_context().add_class("installer-desc")
        lbl_welcome_desc.set_halign(Gtk.Align.CENTER)
        slide_1.pack_start(lbl_welcome_desc, False, False, 0)

        # Botones de navegación inferior
        nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        nav_box.set_margin_top(20)
        nav_box.set_halign(Gtk.Align.CENTER)
        slide_1.pack_end(nav_box, False, False, 0)

        btn_welcome_back = Gtk.Button(label="Back")
        btn_welcome_back.get_style_context().add_class("action-btn")
        btn_welcome_back.connect(
            "clicked", lambda b: self.stack.set_visible_child_name("recovery")
        )
        nav_box.pack_start(btn_welcome_back, False, False, 0)

        btn_welcome_continue = Gtk.Button(label="Continue")
        btn_welcome_continue.get_style_context().add_class("btn-continue")
        btn_welcome_continue.connect(
            "clicked", lambda b: self.stack.set_visible_child_name("disk_selection")
        )
        nav_box.pack_start(btn_welcome_continue, False, False, 0)

        # ----------------------------------------------------------------------
        # Slide 2: Disk Selection Screen (Imagen 3)
        # ----------------------------------------------------------------------
        slide_2 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        slide_2.get_style_context().add_class("installer-box")
        slide_2.set_size_request(680, 480)
        self.stack.add_named(slide_2, "disk_selection")

        # Logo mediano arriba
        logo_box_md = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        logo_box_md.set_halign(Gtk.Align.CENTER)
        logo_box_md.set_margin_top(16)
        slide_2.pack_start(logo_box_md, False, False, 0)

        if os.path.exists(logo_path):
            pixbuf_md = GdkPixbuf.Pixbuf.new_from_file_at_scale(logo_path, 80, 80, True)
            logo_img_md = Gtk.Image.new_from_pixbuf(pixbuf_md)
        else:
            logo_img_md = Gtk.Image.new_from_icon_name("computer", Gtk.IconSize.DIALOG)
        logo_box_md.pack_start(logo_img_md, True, True, 0)

        # Titulo y desc centrado
        lbl_disk_title = Gtk.Label()
        lbl_disk_title.get_style_context().add_class("installer-title")
        lbl_disk_title.set_text("Pulsar OS")
        lbl_disk_title.set_halign(Gtk.Align.CENTER)
        slide_2.pack_start(lbl_disk_title, False, False, 0)

        lbl_disk_desc = Gtk.Label(
            label="Select the disk where you want to install Pulsar OS."
        )
        lbl_disk_desc.get_style_context().add_class("installer-desc")
        lbl_disk_desc.set_halign(Gtk.Align.CENTER)
        slide_2.pack_start(lbl_disk_desc, False, False, 0)

        # Contenedor de Discos Horizontales
        self.disks_hbox = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=0)
        self.disks_hbox.set_halign(Gtk.Align.CENTER)
        slide_2.pack_start(self.disks_hbox, True, True, 10)

        # Botones de navegación inferior
        nav_box_disk = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        nav_box_disk.set_margin_top(20)
        nav_box_disk.set_halign(Gtk.Align.CENTER)
        slide_2.pack_end(nav_box_disk, False, False, 0)

        btn_disk_back = Gtk.Button(label="Back")
        btn_disk_back.get_style_context().add_class("action-btn")
        btn_disk_back.connect(
            "clicked", lambda b: self.stack.set_visible_child_name("welcome")
        )
        nav_box_disk.pack_start(btn_disk_back, False, False, 0)

        self.btn_disk_continue = Gtk.Button(label="Continue")
        self.btn_disk_continue.get_style_context().add_class("btn-continue")
        self.btn_disk_continue.set_sensitive(False)
        self.btn_disk_continue.connect("clicked", self.on_disk_continue_clicked)
        nav_box_disk.pack_start(self.btn_disk_continue, False, False, 0)

        # Rellenar discos (se ejecuta una vez creado el botón de continuar para evitar AttributeErrors)
        # Populate disks (runs once the continue button has been created to avoid AttributeErrors)
        self.populate_disks()


    def add_utility_row(self, title, desc, icon_name):
        row = Gtk.ListBoxRow()
        row.get_style_context().add_class("recovery-row")
        row.title = title  # Store the title directly on the row to avoid hardcoded index dependency

        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=16)
        row.add(box)

        # Icono
        if icon_name == "logo":
            logo_path = "/usr/share/pulsaros-recovery/logo.png"
            if not os.path.exists(logo_path):
                curr_dir = os.path.dirname(os.path.abspath(__file__))
                logo_path = os.path.join(curr_dir, "logo.png")
            if os.path.exists(logo_path):
                pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(
                    logo_path, 42, 42, True
                )
                image = Gtk.Image.new_from_pixbuf(pixbuf)
            else:
                image = Gtk.Image.new_from_icon_name(
                    "system-software-install", Gtk.IconSize.DND
                )
        elif icon_name == "safari":
            icon_theme = Gtk.IconTheme.get_default()
            if icon_theme.has_icon("safari"):
                image = Gtk.Image.new_from_icon_name("safari", Gtk.IconSize.DND)
            else:
                image = Gtk.Image.new_from_icon_name("web-browser", Gtk.IconSize.DND)
        elif icon_name == "timemachine":
            icon_theme = Gtk.IconTheme.get_default()
            if icon_theme.has_icon("time-machine"):
                image = Gtk.Image.new_from_icon_name("time-machine", Gtk.IconSize.DND)
            elif icon_theme.has_icon("deja-dup"):
                image = Gtk.Image.new_from_icon_name("deja-dup", Gtk.IconSize.DND)
            else:
                image = Gtk.Image.new_from_icon_name(
                    "document-revert", Gtk.IconSize.DND
                )
        else:
            image = Gtk.Image.new_from_icon_name(icon_name, Gtk.IconSize.DND)

        box.pack_start(image, False, False, 0)

        # Textos
        text_vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        box.pack_start(text_vbox, True, True, 0)

        lbl_title = Gtk.Label()
        lbl_title.get_style_context().add_class("recovery-title")
        lbl_title.set_text(title)
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
            self.selected_utility = row.title  # Use dynamic title name instead of list index
            self.btn_util_continue.set_sensitive(True)

    def on_utility_row_activated(self, listbox, row):
        if row is not None:
            self.on_utility_continue_clicked(None)

    def on_utility_continue_clicked(self, button):
        if self.selected_utility == "Restore from Backup":
            # Hide recovery to allow Deja-dup to draw on top (fullscreen blocks other windows)
            # Ocultar el recovery para permitir que Deja-dup se dibuje encima (pantalla completa bloquea otras ventanas)
            self.hide()

            def run_deja_dup():
                run_as_real_user("deja-dup --restore || deja-dup", wait=True)
                # Show recovery again in GTK main thread
                # Mostrar el recovery de nuevo en el hilo principal de GTK
                GLib.idle_add(self.show)

            import threading

            threading.Thread(target=run_deja_dup, daemon=True).start()
        elif self.selected_utility == "Install Pulsar OS":
            # Install Pulsar OS - Ir al welcome
            self.stack.set_visible_child_name("welcome")
        elif self.selected_utility == "Seafari Browser":
            # Seafari
            cmd = "seafari || firefox || xdg-open https://google.com"
            run_as_real_user(cmd)
        elif self.selected_utility == "Disk Utility":
            # Disk Utility
            cmd = "gnome-disks || gnome-disk-utility"
            subprocess.Popen(cmd, shell=True)

    def populate_disks(self):
        # Limpiar
        # English: Clear previous disk widgets in layout
        # Español: Limpiar widgets de disco anteriores en el diseño
        for child in self.disks_hbox.get_children():
            self.disks_hbox.remove(child)

        # Reset continue button state
        # Restablecer el estado del botón continuar
        self.btn_disk_continue.set_label("Continue")
        self.btn_disk_continue.set_sensitive(False)

        self.disk_widgets = []
        disks = get_physical_disks()

        if not disks:
            # English: If no physical disks are found, show a styled warning and enable "Continue Anyway"
            # Español: Si no se encuentran discos físicos, mostrar un aviso estilizado y habilitar "Continuar de todos modos"
            warning_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
            warning_box.set_halign(Gtk.Align.CENTER)
            warning_box.set_valign(Gtk.Align.CENTER)

            # Warning Icon
            img_warn = Gtk.Image.new_from_icon_name("dialog-warning", Gtk.IconSize.DIALOG)
            img_warn.set_pixel_size(48)
            warning_box.pack_start(img_warn, False, False, 0)

            # Warning Labels
            lbl_warn = Gtk.Label()
            lbl_warn.set_markup(
                "<span font_desc='13' weight='bold' foreground='#cc0000'>No storage disks found / No se han encontrado discos</span>"
            )
            lbl_warn.set_halign(Gtk.Align.CENTER)
            warning_box.pack_start(lbl_warn, False, False, 0)

            lbl_warn_desc = Gtk.Label()
            lbl_warn_desc.set_markup(
                "<span font_desc='11'>You can click 'Continue Anyway' to launch Calamares and check if it recognizes your storage.\n"
                "Puede pulsar 'Continuar de todos modos' para lanzar Calamares y comprobar si este reconoce sus discos.</span>"
            )
            lbl_warn_desc.set_justify(Gtk.Justification.CENTER)
            lbl_warn_desc.set_halign(Gtk.Align.CENTER)
            lbl_warn_desc.set_line_wrap(True)
            lbl_warn_desc.set_max_width_chars(60)
            warning_box.pack_start(lbl_warn_desc, False, False, 5)

            self.disks_hbox.pack_start(warning_box, True, True, 20)

            # Enable continue anyway
            self.btn_disk_continue.set_label("Continue Anyway")
            self.btn_disk_continue.set_sensitive(True)
            self.selected_disk_path = None
        else:
            for disk in disks:
                # Crear contenedor clickeable
                event_box = Gtk.EventBox()
                card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
                card.get_style_context().add_class("disk-card")
                card.set_border_width(12)
                event_box.add(card)

                # Icono de disco
                img = Gtk.Image.new_from_icon_name("drive-harddisk", Gtk.IconSize.DIALOG)
                # set size of hard drive icon to large
                img.set_pixel_size(64)
                card.pack_start(img, False, False, 0)

                # Escribir nombre del disco (ej: sda)
                lbl_name = Gtk.Label(label=disk["name"])
                lbl_name.get_style_context().add_class("disk-name")
                card.pack_start(lbl_name, False, False, 0)

                # Escribir modelo e información de capacidad
                lbl_info1 = Gtk.Label(label=disk["model"])
                lbl_info1.get_style_context().add_class("disk-info")
                lbl_info1.set_line_wrap(True)
                lbl_info1.set_max_width_chars(15)
                card.pack_start(lbl_info1, False, False, 0)

                lbl_info2 = Gtk.Label(label=f"{disk['size']} total")
                lbl_info2.get_style_context().add_class("disk-info")
                card.pack_start(lbl_info2, False, False, 0)

                # Conectar click
                event_box.connect(
                    "button-press-event", self.on_disk_clicked, disk["path"], card
                )
                self.disks_hbox.pack_start(event_box, False, False, 10)
                self.disk_widgets.append((card, disk["path"]))

        self.show_all()

    def on_disk_clicked(self, widget, event, path, card):
        self.selected_disk_path = path

        # Deseleccionar todos y seleccionar el actual
        for c, p in self.disk_widgets:
            c.get_style_context().remove_class("selected")

        card.get_style_context().add_class("selected")
        self.btn_disk_continue.set_sensitive(True)

    def on_disk_continue_clicked(self, button):
        if self.selected_disk_path:
            # Escribir en /etc/calamares/modules/partition.conf la preselección del disco
            conf_path = "/etc/calamares/modules/partition.conf"
            try:
                os.makedirs(os.path.dirname(conf_path), exist_ok=True)

                # Escribir el defaultDisk en el archivo yaml de partición
                if os.path.exists(conf_path):
                    with open(conf_path, "r") as f:
                        lines = f.readlines()
                    new_lines = []
                    found = False
                    for line in lines:
                        if line.strip().startswith("defaultDisk:"):
                            new_lines.append(
                                f'defaultDisk: "{self.selected_disk_path}"\n'
                            )
                            found = True
                        else:
                            new_lines.append(line)
                    if not found:
                        new_lines.append(
                            f'\ndefaultDisk: "{self.selected_disk_path}"\n'
                        )
                    with open(conf_path, "w") as f:
                        f.writelines(new_lines)
                else:
                    with open(conf_path, "w") as f:
                        f.write(f'---\ndefaultDisk: "{self.selected_disk_path}"\n')
                print(f"Preselected partition disk written: {self.selected_disk_path}")
            except (PermissionError, Exception) as e:
                print(
                    f"Permission denied or error writing to /etc/calamares ({e}), writing to /tmp/partition.conf instead for debugging..."
                )
                try:
                    with open("/tmp/partition.conf", "w") as f:
                        f.write(f'---\ndefaultDisk: "{self.selected_disk_path}"\n')
                except Exception as ex:
                    print("Failed to write fallback config:", ex)
        else:
            # English: No disk preselected (bypass when no disks found). Launch Calamares in auto-detect mode.
            # Español: Sin disco preseleccionado (omitir cuando no se hallan discos). Lanzar Calamares en auto-detección.
            print("No disk selected. Launching Calamares in auto-detection mode.")

        # Lanzar Calamares en primer plano
        print("Launching Calamares installer...")
        # English: Try launch-calamares first, fallback to pkexec calamares (Gnome auth) or standard calamares
        # Español: Intentar primero launch-calamares, fallback a pkexec calamares (autenticacion Gnome) o calamares estándar
        subprocess.Popen(
            "/usr/local/bin/launch-calamares || pkexec calamares || calamares &",
            shell=True,
        )

        # Cerrar la aplicación de recuperación
        Gtk.main_quit()



def main():
    RecoveryApp()
    Gtk.main()


if __name__ == "__main__":
    main()
