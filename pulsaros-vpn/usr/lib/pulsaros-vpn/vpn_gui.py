#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar VPN & Phone - GTK4 / Libadwaita Desktop Control Center
Allows easy and 100% secure configuration of Tailscale mesh VPN and Pulsar Phone pairing.
ZERO public internet ports exposed.
"""

import json
import logging
import os
import shutil
import socket
import subprocess
import sys
import threading
import urllib.parse

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gdk, Gio, GLib, Gtk

from tailscale_manager import TailscaleManager
from bridge_daemon import get_or_create_token, get_local_ip, DEFAULT_PORT

APP_ID = "es.pulsaros.Vpn"
QR_CACHE_PATH = "/tmp/pulsar_phone_pair_qr.png"


class PulsarVpnWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Pulsar VPN & Phone")
        self.set_default_size(780, 680)

        self.tailscale = TailscaleManager()
        self.token = get_or_create_token()
        self.hostname = socket.gethostname()

        self._build_ui()
        self._refresh_status()

        # Periodic refresh every 4 seconds
        GLib.timeout_add_seconds(4, self._refresh_status)

    def _build_ui(self):
        # HeaderBar
        header_bar = Adw.HeaderBar()
        self.set_title("Pulsar VPN & Phone")

        # Refresh button
        refresh_btn = Gtk.Button(icon_name="view-refresh-symbolic")
        refresh_btn.set_tooltip_text("Actualizar estado")
        refresh_btn.connect("clicked", lambda _: self._refresh_status())
        header_bar.pack_start(refresh_btn)

        # Main Box
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        main_box.append(header_bar)

        # Scrolled Window
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        main_box.append(scrolled)

        # Preferences Page
        pref_page = Adw.PreferencesPage()
        scrolled.set_child(pref_page)

        # ---------------- GROUP 1: VPN & Tailscale Status ----------------
        vpn_group = Adw.PreferencesGroup()
        vpn_group.set_title("Red Privada Segura (Tailscale WireGuard)")
        vpn_group.set_description(
            "Conexión directa peer-to-peer cifrada de extremo a extremo sin abrir puertos en el router "
            "ni exponer el PC a la Internet pública."
        )
        pref_page.add(vpn_group)

        # Status ActionRow
        self.vpn_status_row = Adw.ActionRow()
        self.vpn_status_row.set_title("Estado de la VPN")
        self.vpn_status_row.set_subtitle("Comprobando conexión...")
        self.vpn_status_icon = Gtk.Image.new_from_icon_name("network-vpn-symbolic")
        self.vpn_status_row.add_prefix(self.vpn_status_icon)

        self.vpn_toggle_btn = Gtk.Button(label="Conectar")
        self.vpn_toggle_btn.set_valign(Gtk.Align.CENTER)
        self.vpn_toggle_btn.connect("clicked", self._on_toggle_vpn)
        self.vpn_status_row.add_suffix(self.vpn_toggle_btn)
        vpn_group.add(self.vpn_status_row)

        # IP Row
        self.ip_row = Adw.ActionRow()
        self.ip_row.set_title("Dirección IP Privada")
        self.ip_row.set_subtitle("No asignada")
        copy_ip_btn = Gtk.Button(icon_name="edit-copy-symbolic")
        copy_ip_btn.set_tooltip_text("Copiar IP")
        copy_ip_btn.set_valign(Gtk.Align.CENTER)
        copy_ip_btn.connect("clicked", self._copy_ip)
        self.ip_row.add_suffix(copy_ip_btn)
        vpn_group.add(self.ip_row)

        # Hostname / MagicDNS Row
        self.dns_row = Adw.ActionRow()
        self.dns_row.set_title("Nombre de Red (MagicDNS)")
        self.dns_row.set_subtitle(self.hostname)
        vpn_group.add(self.dns_row)

        # ---------------- GROUP 2: Pulsar Phone Pairing ----------------
        pair_group = Adw.PreferencesGroup()
        pair_group.set_title("Vincular con Pulsar Phone")
        pair_group.set_description(
            "Abre la app Pulsar Phone en tu teléfono Android y escanea el código QR para "
            "acceder a las aplicaciones del PC, transferir con FlyDrop y montar tus archivos."
        )
        pref_page.add(pair_group)

        # QR Card Row
        qr_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        qr_box.set_margin_top(12)
        qr_box.set_margin_bottom(12)
        qr_box.set_halign(Gtk.Align.CENTER)

        self.qr_image = Gtk.Picture()
        self.qr_image.set_size_request(220, 220)
        self.qr_image.set_can_shrink(True)
        qr_box.append(self.qr_image)

        self.qr_info_label = Gtk.Label()
        self.qr_info_label.set_wrap(True)
        self.qr_info_label.set_justify(Gtk.Justification.CENTER)
        self.qr_info_label.set_text("Generando código QR seguro...")
        qr_box.append(self.qr_info_label)

        # Copy pair URI button
        copy_pair_btn = Gtk.Button(label="Copiar Datos de Vinculación")
        copy_pair_btn.set_halign(Gtk.Align.CENTER)
        copy_pair_btn.add_css_class("suggested-action")
        copy_pair_btn.connect("clicked", self._copy_pair_uri)
        qr_box.append(copy_pair_btn)

        pair_group.add(qr_box)

        # ---------------- GROUP 3: Servicios Integrados ----------------
        services_group = Adw.PreferencesGroup()
        services_group.set_title("Servicios Activos para el Teléfono")
        pref_page.add(services_group)

        # Broadway Apps Row
        self.apps_row = Adw.ActionRow()
        self.apps_row.set_title("Retransmisión Broadway (GTK)")
        self.apps_row.set_subtitle("Listo para proyectar aplicaciones en el teléfono (Puerto 8085)")
        self.apps_row.add_prefix(Gtk.Image.new_from_icon_name("video-display-symbolic"))
        services_group.add(self.apps_row)

        # FlyDrop Row
        flydrop_row = Adw.ActionRow()
        flydrop_row.set_title("Compartir con FlyDrop (LocalSend v2)")
        flydrop_row.set_subtitle("Envío de archivos, fotos y portapapeles a través de FlyDrop (Puerto 53317)")
        flydrop_row.add_prefix(Gtk.Image.new_from_icon_name("send-to-symbolic"))
        services_group.add(flydrop_row)

        # Filesystem Mount Row
        fs_row = Adw.ActionRow()
        fs_row.set_title("Montaje de Archivos (Android SAF)")
        fs_row.set_subtitle("Permite navegar y editar archivos del PC en la app nativa 'Archivos' de Android")
        fs_row.add_prefix(Gtk.Image.new_from_icon_name("folder-remote-symbolic"))
        services_group.add(fs_row)

        # Notifications Relay Row
        notif_row = Adw.ActionRow()
        notif_row.set_title("Sincronización de Notificaciones")
        notif_row.set_subtitle("Las notificaciones de tu teléfono aparecerán en tu escritorio de Pulsar OS")
        notif_row.add_prefix(Gtk.Image.new_from_icon_name("preferences-system-notifications-symbolic"))
        services_group.add(notif_row)

        self.set_content(main_box)

    def _refresh_status(self):
        status = self.tailscale.get_status()
        is_running = status.get("running", False)
        ip = status.get("ip")
        magic_dns = status.get("magic_dns") or self.hostname
        auth_url = status.get("auth_url")
        self.current_auth_url = auth_url

        lan_ip = get_local_ip()

        if not status.get("installed", False):
            self.vpn_status_row.set_subtitle("Tailscale no está instalado en este sistema")
            self.vpn_toggle_btn.set_label("Instalar")
            self.vpn_status_icon.set_from_icon_name("dialog-warning-symbolic")
        elif is_running and ip:
            self.vpn_status_row.set_subtitle(f"Conectado a la Tailnet • {status.get('backend_state')}")
            self.vpn_toggle_btn.set_label("Desconectar")
            self.vpn_status_icon.set_from_icon_name("network-vpn-symbolic")
        elif status.get("backend_state") == "NeedsLogin" or auth_url:
            self.vpn_status_row.set_subtitle("Pendiente de inicio de sesión en Tailscale")
            self.vpn_toggle_btn.set_label("Iniciar Sesión")
            self.vpn_status_icon.set_from_icon_name("dialog-password-symbolic")
        else:
            self.vpn_status_row.set_subtitle("Desconectado de la VPN (Modo Wi-Fi Local Activo)")
            self.vpn_toggle_btn.set_label("Conectar VPN")
            self.vpn_status_icon.set_from_icon_name("network-wireless-symbolic")

        if ip:
            self.ip_row.set_subtitle(f"{ip} (Tailscale WireGuard)")
            self.dns_row.set_subtitle(magic_dns)
            self._generate_qr(ip)
        else:
            self.ip_row.set_subtitle(f"{lan_ip} (Red Local Wi-Fi)")
            self.dns_row.set_subtitle(self.hostname)
            self._generate_qr(lan_ip)

        return True

    def _generate_qr(self, ip_address: str):
        pair_uri = f"pulsar://pair?ip={ip_address}&port={DEFAULT_PORT}&token={self.token}&alias={urllib.parse.quote(self.hostname)}"
        self.current_pair_uri = pair_uri

        # Generate QR code using qrencode if available
        if shutil.which("qrencode"):
            try:
                subprocess.run(
                    ["qrencode", "-s", "6", "-m", "2", "-o", QR_CACHE_PATH, pair_uri],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=3
                )
                if os.path.exists(QR_CACHE_PATH):
                    self.qr_image.set_filename(QR_CACHE_PATH)
                    self.qr_info_label.set_markup(
                        f"<b>PC:</b> {self.hostname} | <b>IP:</b> {ip_address}\n"
                        f"<small>Cifrado WireGuard • Sin exposición pública</small>"
                    )
                    return
            except Exception as e:
                logging.error(f"Error executing qrencode: {e}")

        self.qr_info_label.set_text(f"Vinculación: {pair_uri}")

    def _on_toggle_vpn(self, button):
        status = self.tailscale.get_status()
        auth_url = status.get("auth_url") or getattr(self, "current_auth_url", None)

        # If needs login and we have an auth URL, launch browser immediately
        if auth_url:
            Gio.AppInfo.launch_default_for_uri(auth_url, None)
            return

        button.set_sensitive(False)

        def worker():
            if status.get("running", False):
                self.tailscale.stop_connection()
            else:
                new_auth_url = self.tailscale.start_connection()
                if new_auth_url:
                    GLib.idle_add(lambda: Gio.AppInfo.launch_default_for_uri(new_auth_url, None))

            GLib.idle_add(lambda: (button.set_sensitive(True), self._refresh_status()))

        threading.Thread(target=worker, daemon=True).start()

    def _copy_ip(self, _):
        status = self.tailscale.get_status()
        ip = status.get("ip") or "127.0.0.1"
        clipboard = self.get_display().get_clipboard()
        clipboard.set(ip)

    def _copy_pair_uri(self, _):
        if hasattr(self, "current_pair_uri"):
            clipboard = self.get_display().get_clipboard()
            clipboard.set(self.current_pair_uri)

    def _show_info_dialog(self, title: str, text: str):
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading=title,
            body=text
        )
        dialog.add_response("ok", "Aceptar")
        dialog.present()


class PulsarVpnApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id=APP_ID,
            flags=Gio.ApplicationFlags.FLAGS_NONE
        )

    def do_activate(self):
        # Ensure bridge daemon is running in user session
        self._ensure_bridge_running()

        win = self.props.active_window
        if not win:
            win = PulsarVpnWindow(self)
        win.present()

    def _ensure_bridge_running(self):
        try:
            # Check if bridge daemon is already listening on port 8989
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.settimeout(0.5)
                res = s.connect_ex(("127.0.0.1", DEFAULT_PORT))
                if res == 0:
                    return  # Already running!
        except Exception:
            pass

        # Launch bridge daemon in background
        daemon_script = os.path.join(os.path.dirname(__file__), "bridge_daemon.py")
        if os.path.isfile(daemon_script):
            subprocess.Popen([sys.executable, daemon_script], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def main():
    app = PulsarVpnApp()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
