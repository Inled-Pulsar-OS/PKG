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
    font-size: 24px;
    font-weight: 700;
    color: #ffffff;
    margin-top: 10px;
    margin-bottom: 6px;
    text-align: center;
}
.welcome-subtitle {
    font-size: 13px;
    color: #8e8e93;
    margin-bottom: 20px;
    text-align: center;
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
.country-scroll {
    border: 1px solid #3c3c3c;
    border-radius: 8px;
    background-color: #2a2a2a;
}
.country-row {
    padding: 8px 14px;
    border-bottom: 1px solid #323236;
}
.country-row-label {
    font-size: 13px;
    color: #ffffff;
}
.avatar-btn {
    border-radius: 9999px;
    padding: 0;
    width: 56px;
    height: 56px;
    border: 2px solid transparent;
    background-color: #2a2a2a;
    transition: all 0.15s ease;
}
.avatar-btn:hover {
    background-color: #3a3a3c;
}
.avatar-btn.selected {
    border-color: #0071e3;
    background-color: #323236;
}
.avatar-label {
    font-size: 28px;
}
.back-arrow-btn {
    border-radius: 9999px;
    padding: 0;
    width: 32px;
    height: 32px;
    background: transparent;
    border: none;
}
.back-arrow-btn:hover {
    background-color: #2c2c2e;
}
"""

COUNTRIES = [
    "España",
    "Argentina",
    "México",
    "Colombia",
    "Chile",
    "Estados Unidos",
    "Reino Unido",
    "Francia",
    "Alemania",
    "Italia",
    "Portugal",
    "Brasil",
    "Uruguay",
    "Perú",
    "Ecuador"
]

AVATARS = [
    "🐼",
    "🐮",
    "🐓",
    "🦊",
    "🦉",
    "👤"
]

class OOTBWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title("Asistente de Configuración de Pulsar OS")
        self.set_default_size(720, 560)
        self.set_resizable(False)
        
        # Kiosk fullscreen mode
        self.fullscreen()
        self.apply_css()
        
        # Center layout
        root_overlay = Gtk.CenterBox()
        root_overlay.set_hexpand(True)
        root_overlay.set_vexpand(True)
        
        self.card_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.card_box.add_css_class("apple-box")
        self.card_box.set_size_request(700, 520)
        self.card_box.set_valign(Gtk.Align.CENTER)
        self.card_box.set_halign(Gtk.Align.CENTER)
        
        # Custom Header Bar with back arrow on top-left of the card
        self.header_bar_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.header_bar_box.set_margin_bottom(10)
        
        self.btn_header_back = Gtk.Button()
        self.btn_header_back.set_child(Gtk.Image.new_from_icon_name("go-previous-symbolic"))
        self.btn_header_back.add_css_class("back-arrow-btn")
        self.btn_header_back.connect("clicked", self.on_back_clicked)
        self.btn_header_back.set_visible(False) # Only show after first screen
        self.header_bar_box.append(self.btn_header_back)
        
        # Center spacer
        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        self.header_bar_box.append(spacer)
        
        self.card_box.append(self.header_bar_box)
        
        # ViewStack for pages
        self.stack = Adw.ViewStack()
        self.card_box.append(self.stack)
        
        # Navigation bar
        self.nav_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.nav_box.set_margin_top(16)
        self.nav_box.set_margin_bottom(10)
        
        # Back Button (Bottom alternative)
        self.btn_back = Gtk.Button(label="Atrás")
        self.btn_back.add_css_class("secondary-action")
        self.btn_back.connect("clicked", self.on_back_clicked)
        self.nav_box.append(self.btn_back)
        
        spacer2 = Gtk.Box()
        spacer2.set_hexpand(True)
        self.nav_box.append(spacer2)
        
        # Next Button
        self.btn_next = Gtk.Button(label="Continuar")
        self.btn_next.add_css_class("suggested-action")
        self.btn_next.connect("clicked", self.on_next_clicked)
        self.nav_box.append(self.btn_next)
        
        self.card_box.append(self.nav_box)
        
        root_overlay.set_center_widget(self.card_box)
        self.set_content(root_overlay)
        
        # Build pages
        self.build_country_page()
        self.build_language_page()
        self.build_timezone_page()
        self.build_account_page()
        self.build_finished_page()
        
        # Show first page
        self.stack.set_visible_child_name("country_select")
        self.btn_back.set_visible(False)
        self.btn_header_back.set_visible(False)
        self.selected_country = None
        self.selected_avatar = "🐼"

    def apply_css(self):
        provider = Gtk.CssProvider()
        provider.load_from_data(CSS_DATA.encode('utf-8'))
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def build_country_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        # Blue globe icon
        globe_icon = Gtk.Image.new_from_icon_name("input-dial-symbolic")
        globe_icon.set_pixel_size(72)
        globe_icon.add_css_class("symbolic-blue")
        box.append(globe_icon)
        
        # Title
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Selecciona tu país o región</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        # Country Scrolled List
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_size_request(280, 180)
        scrolled.add_css_class("country-scroll")
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        
        self.country_listbox = Gtk.ListBox()
        self.country_listbox.connect("row-selected", self.on_country_row_selected)
        scrolled.set_child(self.country_listbox)
        box.append(scrolled)
        
        # Populate countries
        for c in COUNTRIES:
            row = Gtk.ListBoxRow()
            row.country_name = c
            row.add_css_class("country-row")
            lbl = Gtk.Label(label=c)
            lbl.add_css_class("country-row-label")
            lbl.set_halign(Gtk.Align.START)
            row.set_child(lbl)
            self.country_listbox.append(row)
            
        self.stack.add_named(box, "country_select")

    def on_country_row_selected(self, listbox, row):
        if row is not None:
            self.selected_country = row.country_name
            self.btn_next.set_sensitive(True)

    def build_language_page(self):
        # Spoken & Written languages screen
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        # Circular text bubbles / input symbol icon
        icon = Gtk.Image.new_from_icon_name("document-send-symbolic")
        icon.set_pixel_size(72)
        icon.add_css_class("symbolic-blue")
        box.append(icon)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Idiomas y Teclado</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        desc = Gtk.Label(label="Configura el idioma principal y el método de entrada de texto.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)
        
        group = Adw.PreferencesGroup()
        group.set_title("Opciones de Idioma y Entrada")
        group.set_size_request(320, -1)
        box.append(group)
        
        self.lang_row = Adw.ComboRow(title="Idioma Principal")
        self.lang_list = Gtk.StringList.new([])
        self.lang_list.append("Español (España)")
        self.lang_list.append("English (United States)")
        self.lang_row.set_model(self.lang_list)
        group.add(self.lang_row)
        
        self.keymap_row = Adw.ComboRow(title="Distribución de Teclado")
        self.keymap_list = Gtk.StringList.new([])
        self.keymap_list.append("Español")
        self.keymap_list.append("English (US)")
        self.keymap_row.set_model(self.keymap_list)
        group.add(self.keymap_row)
        
        self.stack.add_named(box, "language_select")

    def build_timezone_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        # Map/Location icon
        icon = Gtk.Image.new_from_icon_name("mark-location-symbolic")
        icon.set_pixel_size(72)
        icon.add_css_class("symbolic-blue")
        box.append(icon)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Zona Horaria</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        desc = Gtk.Label(label="Configura tu ubicación horaria local.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)
        
        group = Adw.PreferencesGroup()
        group.set_title("Zona Horaria del Sistema")
        group.set_size_request(320, -1)
        box.append(group)
        
        self.tz_row = Adw.ComboRow(title="Región Horaria")
        self.tz_list = Gtk.StringList.new([])
        self.tz_list.append("Europe/Madrid")
        self.tz_list.append("America/New_York")
        self.tz_list.append("UTC")
        self.tz_row.set_model(self.tz_list)
        group.add(self.tz_row)
        
        self.stack.add_named(box, "timezone")

    def build_account_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        box.set_margin_start(20)
        box.set_margin_end(20)
        
        # Title
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Crear una cuenta de Pulsar</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        # Subtitle
        desc = Gtk.Label(label="La contraseña que crees aquí se utilizará para iniciar sesión en este equipo.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)
        
        # Row of Selectable User Avatars
        avatar_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        avatar_box.set_halign(Gtk.Align.CENTER)
        avatar_box.set_margin_bottom(12)
        box.append(avatar_box)
        
        self.avatar_buttons = []
        for av in AVATARS:
            btn = Gtk.Button()
            btn.avatar_char = av
            btn.add_css_class("avatar-btn")
            lbl = Gtk.Label(label=av)
            lbl.add_css_class("avatar-label")
            btn.set_child(lbl)
            btn.connect("clicked", self.on_avatar_clicked)
            avatar_box.append(btn)
            self.avatar_buttons.append(btn)
            
        # Select first avatar by default
        self.avatar_buttons[0].add_css_class("selected")
        
        # Account fields list using Adw
        group = Adw.PreferencesGroup()
        group.set_size_request(420, -1)
        box.append(group)
        
        self.fullname_row = Adw.EntryRow(title="Nombre completo")
        self.fullname_row.connect("changed", self.on_fullname_changed)
        group.add(self.fullname_row)
        
        self.username_row = Adw.EntryRow(title="Nombre de cuenta")
        group.add(self.username_row)
        
        self.password_row = Adw.EntryRow(title="Contraseña")
        self.password_row.set_visibility(False)
        group.add(self.password_row)
        
        self.confirm_row = Adw.EntryRow(title="Verificar")
        self.confirm_row.set_visibility(False)
        group.add(self.confirm_row)
        
        self.hint_row = Adw.EntryRow(title="Indicación de contraseña")
        group.add(self.hint_row)
        
        self.stack.add_named(box, "account")

    def on_fullname_changed(self, entry):
        fullname = entry.get_text()
        sanitized = re.sub(r'[^a-z0-9_-]', '', fullname.lower().replace(' ', ''))
        self.username_row.set_text(sanitized[:16])

    def on_avatar_clicked(self, btn):
        for b in self.avatar_buttons:
            b.remove_css_class("selected")
        btn.add_css_class("selected")
        self.selected_avatar = btn.avatar_char

    def build_finished_page(self):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        box.set_valign(Gtk.Align.CENTER)
        box.set_halign(Gtk.Align.CENTER)
        
        image = Gtk.Image.new_from_icon_name("object-select-symbolic")
        image.set_pixel_size(80)
        image.add_css_class("suggested-action")
        box.append(image)
        
        title = Gtk.Label()
        title.set_markup("<span font_weight='bold' size='16000'>Todo listo</span>")
        title.add_css_class("welcome-title")
        box.append(title)
        
        desc = Gtk.Label(label="Tu equipo está listo para usarse. Haz clic en el botón de abajo para comenzar.")
        desc.add_css_class("welcome-subtitle")
        box.append(desc)
        
        self.stack.add_named(box, "finished")

    def on_back_clicked(self, btn):
        current_page = self.stack.get_visible_child_name()
        if current_page == "language_select":
            self.stack.set_visible_child_name("country_select")
            self.btn_back.set_visible(False)
            self.btn_header_back.set_visible(False)
        elif current_page == "timezone":
            self.stack.set_visible_child_name("language_select")
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
        
        if current_page == "country_select":
            if not self.selected_country:
                return
            
            # Map country to pre-selected language and keyboard indexes
            if self.selected_country == "Estados Unidos" or self.selected_country == "Reino Unido":
                self.lang_row.set_selected(1) # English
                self.keymap_row.set_selected(1) # US Keyboard
            else:
                self.lang_row.set_selected(0) # Spanish
                self.keymap_row.set_selected(0) # Spanish Keyboard
                
            self.stack.set_visible_child_name("language_select")
            self.btn_back.set_visible(True)
            self.btn_header_back.set_visible(True)
            
        elif current_page == "language_select":
            # Apply language and keyboard layout
            lang_idx = self.lang_row.get_selected()
            key_idx = self.keymap_row.get_selected()
            
            locale = "es_ES.UTF-8" if lang_idx == 0 else "en_US.UTF-8"
            keymap = "es" if key_idx == 0 else "us"
            
            subprocess.run(["localectl", "set-locale", f"LANG={locale}"])
            subprocess.run(["localectl", "set-x11-keymap", keymap])
            
            self.stack.set_visible_child_name("timezone")
            
        elif current_page == "timezone":
            tz_idx = self.tz_row.get_selected()
            tz_mapping = ["Europe/Madrid", "America/New_York", "UTC"]
            tz = tz_mapping[tz_idx] if tz_idx >= 0 and tz_idx < len(tz_mapping) else "Europe/Madrid"
            
            subprocess.run(["timedatectl", "set-timezone", tz])
            self.stack.set_visible_child_name("account")
            
        elif current_page == "account":
            fullname = self.fullname_row.get_text().strip()
            username = self.username_row.get_text().strip()
            password = self.password_row.get_text().strip()
            confirm = self.confirm_row.get_text().strip()
            
            if not fullname or not username or not password:
                self.show_error("Todos los campos son obligatorios.")
                return
            if password != confirm:
                self.show_error("Las contraseñas no coinciden.")
                return
            if not re.match(r"^[a-z0-9_-]{3,16}$", username):
                self.show_error("El nombre de cuenta debe ser alfanumérico y de 3 a 16 caracteres.")
                return
                
            # Create user account
            res = subprocess.run([
                "useradd", "-m", "-G", "sudo,audio,video,plugdev",
                "-s", "/bin/bash", username
            ], capture_output=True, text=True)
            
            if res.returncode != 0:
                self.show_error(f"Error creando cuenta:\n{res.stderr}")
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
            
            # Transition to final step
            self.stack.set_visible_child_name("finished")
            self.btn_next.set_label("Comenzar a usar Pulsar OS")
            self.btn_back.set_visible(False)
            self.btn_header_back.set_visible(False)
            
        elif current_page == "finished":
            self.run_final_cleanup()

    def run_final_cleanup(self):
        try:
            # 1. Eliminate live user
            subprocess.run(["userdel", "-f", "-r", "live"])
            
            # 2. Delete OOTB witness file
            if os.path.exists("/etc/pulsar-need-setup"):
                os.remove("/etc/pulsar-need-setup")
                
            # 3. Disable service
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
