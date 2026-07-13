#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# ==============================================================================
# Pulsar OS - OOTB Setup Assistant (macOS Setup Assistant GTK4 & Libadwaita Style)
# ==============================================================================

import sys
import os

# CRITICAL REGULATION: Check OOTB witness flag before loading graphical libraries
if not os.path.exists("/etc/pulsar-need-setup"):
    print("OOTB setup not required. Exiting.")
    sys.exit(0)

import subprocess
import threading
import time
import re
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')

from gi.repository import Gtk, Gdk, GLib, Adw, Gio

# Custom CSS for Apple macOS OOTB Setup Look-and-Feel
CSS_DATA = """
window {
    background-color: #121212; /* Neutral deep dark background */
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
.apple-box {
    background-color: #1e1e1e;
    border: 1px solid #303030;
    border-radius: 16px;
    padding: 32px 48px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
}
.welcome-title {
    font-size: 26px;
    font-weight: 700;
    color: #ffffff;
    margin-top: 10px;
    margin-bottom: 6px;
}
.welcome-subtitle {
    font-size: 13px;
    color: #8e8e93;
    margin-bottom: 20px;
}
.suggested-action {
    background-color: #0071e3; /* Apple Blue */
    color: #ffffff;
    border-radius: 8px;
    font-weight: bold;
    padding: 8px 22px;
    border: none;
}
.suggested-action:hover {
    background-color: #007bf5;
}
.suggested-action:active {
    background-color: #0063c6;
}
.secondary-action {
    background-color: #2c2c2e;
    color: #ffffff;
    border-radius: 8px;
    font-weight: bold;
    padding: 8px 22px;
    border: 1px solid #3e3e42;
}
.secondary-action:hover {
    background-color: #3a3a3c;
}
.secondary-action:active {
    background-color: #1c1c1e;
}
.header-bar {
    background-color: transparent;
    border-bottom: none;
}
"""

class OOTBWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Asistente de Configuración de Pulsar OS")
        self.set_default_size(720, 560)
        self.set_resizable(False)
        
        # Lock screen into kiosk / borderless fullscreen mode
        self.fullscreen()
        
        # Apply CSS
        self.apply_css()
        
        # Main layout container
        root_overlay = Gtk.CenterBox()
        root_overlay.set_hexpand(True)
        root_overlay.set_vexpand(True)
        
        # Setup Assistant Main Card Box
        self.card_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.card_box.add_css_class("apple-box")
        self.card_box.set_size_request(700, 520)
        self.card_box.set_valign(Gtk.Align.CENTER)
        self.card_box.set_halign(Gtk.Align.CENTER)
        
        # Header bar inside card
        header_bar = Adw.HeaderBar()
        header_bar.set_show_end_title_buttons(False)
        header_bar.set_show_start_title_buttons(False)
        header_bar.add_css_class("header-bar")
        
        window_title = Adw.WindowTitle(title="Asistente de Configuración")
        header_bar.set_title_widget(window_title)
        self.card_box.append(header_bar)
        
        # ViewStack for pages
        self.stack = Adw.ViewStack()
        self.card_box.append(self.stack)
        
        # Setup Navigation buttons container
        self.nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.nav_box.set_margin_top(16)
        self.nav_box.set_margin_bottom(10)
        
        # Back Button
        self.btn_back = Gtk.Button(label="Atrás")
        self.btn_back.add_css_class("secondary-action")
        self.btn_back.connect("clicked", self.on_back_clicked)
        self.nav_box.append(self.btn_back)
        
        # Spacer
        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        self.nav_box.append(spacer)
        
        # Next Button
        self.btn_next = Gtk.Button(label="Continuar")
        self.btn_next.add_css_class("suggested-action")
        self.btn_next.connect("clicked", self.on_next_clicked)
        self.nav_box.append(self.btn_next)
        
        self.card_box.append(self.nav_box)
        
        root_overlay.set_center_widget(self.card_box)
        self.set_content(root_overlay)
        
        # Create wizard steps
        self.build_lang_page()
        self.build_timezone_page()
        self.build_account_page()
        self.build_finished_page()
        
        # Show first step
        self.stack.set_visible_child_name("language")
        self.btn_back.set_sensitive(False)

    def apply_css(self):
        provider = Gtk.CssProvider()
        provider.load_from_data(CSS_DATA.encode('utf-8'))
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def build_lang_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        
        # Heading
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold'>Idioma y Teclado</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        # Description
        desc = Gtk.Label(label="Selecciona tu idioma preferido y distribución de teclado.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)
        
        # Preferences group card
        group = Adw.PreferencesGroup()
        group.set_title("Configuración de Entrada")
        box.append(group)
        
        # Language Select Combo
        self.lang_row = Adw.ComboRow(title="Idioma del Sistema")
        self.lang_list = Gtk.StringList.new([])
        self.lang_list.append("Español (España)")
        self.lang_list.append("English (United States)")
        self.lang_row.set_model(self.lang_list)
        group.add(self.lang_row)
        
        # Keyboard Layout Select Combo
        self.keymap_row = Adw.ComboRow(title="Distribución del Teclado")
        self.keymap_list = Gtk.StringList.new([])
        self.keymap_list.append("Español")
        self.keymap_list.append("English (US)")
        self.keymap_row.set_model(self.keymap_list)
        group.add(self.keymap_row)
        
        self.stack.add_named(box, "language")

    def build_timezone_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        
        # Heading
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold'>Fecha y Hora</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        # Description
        desc = Gtk.Label(label="Configura tu ubicación horaria local.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)
        
        # Preferences group card
        group = Adw.PreferencesGroup()
        group.set_title("Zona Horaria")
        box.append(group)
        
        # Timezone Combo
        self.tz_row = Adw.ComboRow(title="Región Horaria")
        self.tz_list = Gtk.StringList.new([])
        self.tz_list.append("Europe/Madrid")
        self.tz_list.append("America/New_York")
        self.tz_list.append("UTC")
        self.tz_row.set_model(self.tz_list)
        group.add(self.tz_row)
        
        self.stack.add_named(box, "timezone")

    def build_account_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        
        # Heading
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold'>Crear Cuenta de Usuario</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        # Description
        desc = Gtk.Label(label="Crea una cuenta local de administrador para el sistema.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)
        
        # Preferences Group Card (macOS Account form style)
        group = Adw.PreferencesGroup()
        group.set_title("Detalles de la Cuenta")
        box.append(group)
        
        self.fullname_row = Adw.EntryRow(title="Nombre Completo")
        group.add(self.fullname_row)
        
        self.username_row = Adw.EntryRow(title="Nombre de Usuario")
        group.add(self.username_row)
        
        self.password_row = Adw.EntryRow(title="Contraseña")
        self.password_row.set_visibility(False)
        group.add(self.password_row)
        
        self.confirm_row = Adw.EntryRow(title="Verificar Contraseña")
        self.confirm_row.set_visibility(False)
        group.add(self.confirm_row)
        
        self.stack.add_named(box, "account")

    def build_finished_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        # Finalization Image Icon
        image = Gtk.Image.new_from_icon_name("object-select-symbolic")
        image.set_pixel_size(80)
        image.add_css_class("suggested-action")
        box.append(image)
        
        # Heading
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold'>Configuración Completada</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        # Description
        desc = Gtk.Label(label="Tu equipo está listo para usarse. Haz clic en el botón de abajo para comenzar.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)
        
        self.stack.add_named(box, "finished")

    def on_back_clicked(self, btn):
        current_page = self.stack.get_visible_child_name()
        if current_page == "timezone":
            self.stack.set_visible_child_name("language")
            self.btn_back.set_sensitive(False)
        elif current_page == "account":
            self.stack.set_visible_child_name("timezone")

    def show_error(self, message):
        dialog = Gtk.MessageDialog(
            transient_for=self,
            flags=Gtk.DialogFlags.MODAL,
            message_type=Gtk.MessageType.ERROR,
            buttons=Gtk.ButtonsType.OK,
            text="Datos Incorrectos"
        )
        dialog.format_secondary_text(message)
        dialog.connect("response", lambda d, r: d.destroy())
        dialog.present()

    def on_next_clicked(self, btn):
        current_page = self.stack.get_visible_child_name()
        
        if current_page == "language":
            # Apply language and keyboard configs
            lang_idx = self.lang_row.get_selected()
            key_idx = self.keymap_row.get_selected()
            
            locale = "es_ES.UTF-8" if lang_idx == 0 else "en_US.UTF-8"
            keymap = "es" if key_idx == 0 else "us"
            
            subprocess.run(["localectl", "set-locale", f"LANG={locale}"])
            subprocess.run(["localectl", "set-x11-keymap", keymap])
            
            self.stack.set_visible_child_name("timezone")
            self.btn_back.set_sensitive(True)
            
        elif current_page == "timezone":
            # Apply timezone
            tz_idx = self.tz_row.get_selected()
            tz_mapping = ["Europe/Madrid", "America/New_York", "UTC"]
            tz = tz_mapping[tz_idx] if tz_idx >= 0 and tz_idx < len(tz_mapping) else "Europe/Madrid"
            
            subprocess.run(["timedatectl", "set-timezone", tz])
            self.stack.set_visible_child_name("account")
            
        elif current_page == "account":
            # Validate input fields
            fullname = self.fullname_row.get_text().strip()
            username = self.username_row.get_text().strip()
            password = self.password_row.get_text().strip()
            confirm = self.confirm_row.get_text().strip()
            
            if not fullname or not username or not password:
                self.show_error("Todos los campos de la cuenta son obligatorios.")
                return
            if password != confirm:
                self.show_error("Las contraseñas ingresadas no coinciden.")
                return
            if not re.match(r"^[a-z0-9_-]{3,16}$", username):
                self.show_error("El nombre de usuario no es válido (solo minúsculas y números de 3 a 16 caract.).")
                return
                
            # Create user account
            res = subprocess.run([
                "useradd", "-m", "-G", "sudo,audio,video,plugdev",
                "-s", "/bin/bash", username
            ], capture_output=True, text=True)
            
            if res.returncode != 0:
                self.show_error(f"Fallo al crear la cuenta:\n{res.stderr}")
                return
                
            # Configure passwords
            p1 = subprocess.Popen(["echo", f"{username}:{password}"], stdout=subprocess.PIPE)
            p2 = subprocess.Popen(["chpasswd"], stdin=p1.stdout, stderr=subprocess.PIPE)
            p1.stdout.close()
            p2.communicate()
            
            p1 = subprocess.Popen(["echo", f"root:{password}"], stdout=subprocess.PIPE)
            p2 = subprocess.Popen(["chpasswd"], stdin=p1.stdout, stderr=subprocess.PIPE)
            p1.stdout.close()
            p2.communicate()
            
            # Save username to class to use in cleanup
            self.new_username = username
            
            # Transition to finished step
            self.stack.set_visible_child_name("finished")
            self.btn_next.set_label("Comenzar a usar Pulsar OS")
            self.btn_back.set_visible(False)
            
        elif current_page == "finished":
            # Finalization and Absolute Cleanup
            self.run_final_cleanup()

    def run_final_cleanup(self):
        try:
            # 1. Eliminate live temporary user
            subprocess.run(["userdel", "-f", "-r", "live"])
            
            # 2. Eliminate need-setup OOTB witness flag
            if os.path.exists("/etc/pulsar-need-setup"):
                os.remove("/etc/pulsar-need-setup")
                
            # 3. Disable systemd OOTB service
            subprocess.run(["systemctl", "disable", "pulsar-ootb.service"])
            
            # 4. Restart session manager to login to new user session
            subprocess.run(["systemctl", "restart", "display-manager"])
            
            self.close()
            sys.exit(0)
        except Exception as e:
            self.show_error(f"Error durante la limpieza final:\n{e}")


class OOTBApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id="es.inled.pulsaros.welcome_ootb",
            flags=Gio.ApplicationFlags.FLAGS_NONE
        )

    def do_activate(self):
        win = OOTBWindow(self)
        win.present()


if __name__ == "__main__":
    app = OOTBApp()
    sys.exit(app.run(sys.argv))
