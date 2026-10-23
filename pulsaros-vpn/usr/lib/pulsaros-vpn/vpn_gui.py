#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar VPN & Phone - GTK4 / Libadwaita Desktop Control Center
Simple, elegant interface matching Pulsar Adblocker design philosophy:
- Large centered pill action button (Connect / Disconnect)
- Clear pairing instructions & inline QR code
- Bidirectional screen and sound streaming:
    * PC -> Android (Screen + PulseAudio over low-latency MPEG-TS)
    * Phone -> PC (Screen + Audio via scrcpy)
- Zero public internet ports exposed (100% WireGuard mesh)
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
import urllib.request

import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gdk, Gio, GLib, Gtk

from tailscale_manager import TailscaleManager
from bridge_daemon import get_or_create_token, get_local_ip, DEFAULT_PORT

APP_ID = "es.pulsaros.Vpn"
QR_CACHE_PATH = "/tmp/pulsar_phone_pair_qr.png"
DAEMON_URL = f"http://127.0.0.1:{DEFAULT_PORT}"

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(levelname)s: %(message)s")
logger = logging.getLogger("PulsarVPN.GUI")


class PulsarVpnWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Pulsar VPN")
        self.set_default_size(520, 720)
        self.set_size_request(420, 580)

        self.tailscale = TailscaleManager()
        self.token = get_or_create_token()
        self.hostname = socket.gethostname()
        self.is_busy = False

        self._build_ui()
        self._refresh_status()

        # Periodic refresh every 3 seconds
        GLib.timeout_add_seconds(3, self._refresh_status)

    def _build_ui(self):
        # HeaderBar
        header_bar = Adw.HeaderBar()
        self.set_title("Pulsar VPN")

        refresh_btn = Gtk.Button(icon_name="view-refresh-symbolic")
        refresh_btn.set_tooltip_text("Actualizar estado")
        refresh_btn.connect("clicked", lambda _: self._refresh_status())
        header_bar.pack_start(refresh_btn)

        # Main Layout
        root_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        root_box.append(header_bar)

        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        root_box.append(scrolled)

        clamp = Adw.Clamp()
        clamp.set_maximum_size(540)
        clamp.set_tightening_threshold(420)
        scrolled.set_child(clamp)

        content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20)
        content_box.set_margin_top(28)
        content_box.set_margin_bottom(36)
        content_box.set_margin_start(20)
        content_box.set_margin_end(20)
        content_box.set_halign(Gtk.Align.FILL)
        clamp.set_child(content_box)

        # Top App Header
        self.header_icon = Gtk.Image.new_from_icon_name("network-vpn-symbolic")
        self.header_icon.set_pixel_size(64)
        self.header_icon.set_halign(Gtk.Align.CENTER)
        content_box.append(self.header_icon)

        self.title_label = Gtk.Label()
        self.title_label.set_markup("<b>Pulsar VPN</b>")
        self.title_label.add_css_class("title-1")
        self.title_label.set_halign(Gtk.Align.CENTER)
        content_box.append(self.title_label)

        self.status_label = Gtk.Label()
        self.status_label.set_text("Comprobando conexión...")
        self.status_label.add_css_class("heading")
        self.status_label.set_halign(Gtk.Align.CENTER)
        content_box.append(self.status_label)

        # ----------------- BIG CENTERED PILL BUTTON (Like Adblocker) -----------------
        self.btn = Gtk.Button()
        self.btn.set_size_request(260, 72)
        self.btn.add_css_class("pill")
        self.btn.set_halign(Gtk.Align.CENTER)
        self.btn.connect("clicked", self._on_btn_clicked)

        self.btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        self.btn_box.set_halign(Gtk.Align.CENTER)
        self.btn_box.set_valign(Gtk.Align.CENTER)

        self.btn_icon = Gtk.Image.new_from_icon_name("network-vpn-symbolic")
        self.btn_box.append(self.btn_icon)

        self.btn_text = Gtk.Label()
        self.btn_text.set_markup("<b>CONECTAR VPN</b>")
        self.btn_text.add_css_class("title-2")
        self.btn_box.append(self.btn_text)

        self.btn_spinner = Gtk.Spinner()
        self.btn_spinner.set_size_request(24, 24)
        self.btn_spinner.set_visible(False)
        self.btn_box.append(self.btn_spinner)

        self.btn.set_child(self.btn_box)
        content_box.append(self.btn)

        # ----------------- SIMPLE EXPLANATION -----------------
        self.info_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.info_box.set_halign(Gtk.Align.CENTER)
        self.info_box.set_margin_top(4)

        info_lbl = Gtk.Label()
        info_lbl.set_markup(
            "<span size='medium'>Simplemente <b>inicia sesión con la misma cuenta</b> en otro dispositivo\n"
            "o escanea el código QR que se muestra a continuación.</span>"
        )
        info_lbl.set_wrap(True)
        info_lbl.set_justify(Gtk.Justification.CENTER)
        info_lbl.add_css_class("dim-label")
        self.info_box.append(info_lbl)
        content_box.append(self.info_box)

        # ----------------- QR PAIRING CARD -----------------
        qr_card = Adw.PreferencesGroup()
        qr_card.set_title("Vincular con tu Móvil")
        content_box.append(qr_card)

        qr_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        qr_box.set_halign(Gtk.Align.CENTER)
        qr_box.set_margin_top(8)
        qr_box.set_margin_bottom(12)

        self.qr_image = Gtk.Picture()
        self.qr_image.set_size_request(200, 200)
        self.qr_image.set_can_shrink(True)
        qr_box.append(self.qr_image)

        self.qr_details_label = Gtk.Label()
        self.qr_details_label.set_justify(Gtk.Justification.CENTER)
        self.qr_details_label.set_wrap(True)
        self.qr_details_label.set_markup(f"<b>PC:</b> {self.hostname}")
        qr_box.append(self.qr_details_label)

        copy_pair_btn = Gtk.Button(label="Copiar Enlace de Vinculación")
        copy_pair_btn.set_halign(Gtk.Align.CENTER)
        copy_pair_btn.add_css_class("flat")
        copy_pair_btn.connect("clicked", self._copy_pair_uri)
        qr_box.append(copy_pair_btn)

        qr_card.add(qr_box)

        # ----------------- BIDIRECTIONAL SCREEN & SOUND STREAMING -----------------
        stream_group = Adw.PreferencesGroup()
        stream_group.set_title("Pantalla y Sonido Compartidos")
        stream_group.set_description("Transmite pantalla y audio en tiempo real entre este PC y tu teléfono Android.")
        content_box.append(stream_group)

        # Row 1: PC -> Android Phone (Screen + Sound)
        self.pc_to_phone_row = Adw.ActionRow()
        self.pc_to_phone_row.set_title("Transmitir PC al Teléfono (Pantalla + Sonido)")
        self.pc_to_phone_row.set_subtitle("Envía el escritorio completo y el sonido del sistema al móvil vía Wi-Fi / VPN")
        self.pc_to_phone_row.add_prefix(Gtk.Image.new_from_icon_name("video-display-symbolic"))

        self.btn_stream_pc = Gtk.Button(label="Iniciar")
        self.btn_stream_pc.set_valign(Gtk.Align.CENTER)
        self.btn_stream_pc.add_css_class("suggested-action")
        self.btn_stream_pc.connect("clicked", self._toggle_pc_to_phone_stream)
        self.pc_to_phone_row.add_suffix(self.btn_stream_pc)
        stream_group.add(self.pc_to_phone_row)

        # Row 2: Phone -> PC (Screen + Sound)
        self.phone_to_pc_row = Adw.ActionRow()
        self.phone_to_pc_row.set_title("Ver Teléfono en el PC (Pantalla + Sonido)")
        self.phone_to_pc_row.set_subtitle("Proyecta y escucha la pantalla de tu móvil en una ventana del PC con scrcpy")
        self.phone_to_pc_row.add_prefix(Gtk.Image.new_from_icon_name("phone-symbolic"))

        self.btn_stream_phone = Gtk.Button(label="Abrir")
        self.btn_stream_phone.set_valign(Gtk.Align.CENTER)
        self.btn_stream_phone.connect("clicked", self._toggle_phone_to_pc_stream)
        self.phone_to_pc_row.add_suffix(self.btn_stream_phone)
        stream_group.add(self.phone_to_pc_row)

        # IP details footer
        self.footer_label = Gtk.Label()
        self.footer_label.set_halign(Gtk.Align.CENTER)
        self.footer_label.add_css_class("caption")
        self.footer_label.add_css_class("dim-label")
        content_box.append(self.footer_label)

        self.set_content(root_box)

    def _refresh_status(self):
        if self.is_busy:
            return True

        status = self.tailscale.get_status()
        is_running = status.get("running", False)
        ip = status.get("ip")
        auth_url = status.get("auth_url")
        self.current_auth_url = auth_url

        lan_ip = get_local_ip()

        # Update Main Button & Status Banner
        self.btn.remove_css_class("suggested-action")
        self.btn.remove_css_class("destructive-action")

        if not status.get("installed", False):
            self.status_label.set_text("TAILSCALE NO INSTALADO")
            self.btn_text.set_markup("<b>INSTALAR TAILSCALE</b>")
            self.btn.add_css_class("suggested-action")
            self.header_icon.set_from_icon_name("dialog-warning-symbolic")
        elif is_running and ip:
            self.status_label.set_text("PROTEGIDO Y CONECTADO")
            self.btn_text.set_markup("<b>DESCONECTAR</b>")
            self.btn.add_css_class("destructive-action")
            self.header_icon.set_from_icon_name("network-vpn-symbolic")
            self.btn_icon.set_from_icon_name("network-vpn-acquiring-symbolic")
        elif status.get("backend_state") == "NeedsLogin" or auth_url:
            self.status_label.set_text("INICIO DE SESIÓN REQUERIDO")
            self.btn_text.set_markup("<b>INICIAR SESIÓN</b>")
            self.btn.add_css_class("suggested-action")
            self.header_icon.set_from_icon_name("dialog-password-symbolic")
            self.btn_icon.set_from_icon_name("dialog-password-symbolic")
        else:
            self.status_label.set_text("MODO LOCAL (SIN VPN)")
            self.btn_text.set_markup("<b>CONECTAR VPN</b>")
            self.btn.add_css_class("suggested-action")
            self.header_icon.set_from_icon_name("network-wireless-symbolic")
            self.btn_icon.set_from_icon_name("network-vpn-symbolic")

        # QR and IP display
        active_ip = ip if ip else lan_ip
        self.footer_label.set_text(f"IP VPN: {ip or 'Inactivo'} • IP Local: {lan_ip} • Puerto: {DEFAULT_PORT}")
        self._generate_qr(active_ip, is_vpn=bool(ip))

        # Check streaming status from bridge daemon
        self._refresh_streaming_status()

        return True

    def _generate_qr(self, ip_address: str, is_vpn: bool):
        pair_uri = f"pulsar://pair?ip={ip_address}&port={DEFAULT_PORT}&token={self.token}&alias={urllib.parse.quote(self.hostname)}"
        self.current_pair_uri = pair_uri

        if shutil.which("qrencode"):
            try:
                subprocess.run(
                    ["qrencode", "-s", "5", "-m", "2", "-o", QR_CACHE_PATH, pair_uri],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=3
                )
                if os.path.exists(QR_CACHE_PATH):
                    self.qr_image.set_filename(QR_CACHE_PATH)
                    net_type = "Red Privada Tailscale" if is_vpn else "Red Local Wi-Fi"
                    self.qr_details_label.set_markup(
                        f"<b>{self.hostname}</b> ({ip_address})\n"
                        f"<small>{net_type} • Cifrado de extremo a extremo</small>"
                    )
                    return
            except Exception as e:
                logger.error(f"Error running qrencode: {e}")

        self.qr_details_label.set_text(f"Vinculación: {pair_uri}")

    def _on_btn_clicked(self, _):
        status = self.tailscale.get_status()
        auth_url = status.get("auth_url") or getattr(self, "current_auth_url", None)

        if auth_url:
            Gio.AppInfo.launch_default_for_uri(auth_url, None)
            return

        self._set_busy(True)

        def worker():
            if status.get("running", False):
                self.tailscale.stop_connection()
            else:
                new_auth_url = self.tailscale.start_connection()
                if new_auth_url:
                    GLib.idle_add(lambda: Gio.AppInfo.launch_default_for_uri(new_auth_url, None))

            GLib.idle_add(lambda: (self._set_busy(False), self._refresh_status()))

        threading.Thread(target=worker, daemon=True).start()

    def _set_busy(self, busy: bool):
        self.is_busy = busy
        self.btn.set_sensitive(not busy)
        self.btn_spinner.set_visible(busy)
        if busy:
            self.btn_spinner.start()
        else:
            self.btn_spinner.stop()

    def _copy_pair_uri(self, _):
        if hasattr(self, "current_pair_uri"):
            clipboard = self.get_display().get_clipboard()
            clipboard.set(self.current_pair_uri)

    def _refresh_streaming_status(self):
        def query():
            try:
                req = urllib.request.Request(
                    f"{DAEMON_URL}/api/stream/pc-to-phone/status",
                    headers={"X-Pulsar-Token": self.token}
                )
                with urllib.request.urlopen(req, timeout=1.0) as resp:
                    data = json.loads(resp.read().decode("utf-8"))
                    active = data.get("active", False)
                    GLib.idle_add(lambda: self._update_pc_stream_ui(active))
            except Exception:
                pass

            try:
                req = urllib.request.Request(
                    f"{DAEMON_URL}/api/stream/phone-to-pc/status",
                    headers={"X-Pulsar-Token": self.token}
                )
                with urllib.request.urlopen(req, timeout=1.0) as resp:
                    data = json.loads(resp.read().decode("utf-8"))
                    running = data.get("running", False)
                    GLib.idle_add(lambda: self._update_phone_stream_ui(running))
            except Exception:
                pass

        threading.Thread(target=query, daemon=True).start()

    def _update_pc_stream_ui(self, active: bool):
        if active:
            self.btn_stream_pc.set_label("Detener")
            self.btn_stream_pc.remove_css_class("suggested-action")
            self.btn_stream_pc.add_css_class("destructive-action")
            self.pc_to_phone_row.set_subtitle("Transmitiendo pantalla y sonido en vivo...")
        else:
            self.btn_stream_pc.set_label("Iniciar")
            self.btn_stream_pc.remove_css_class("destructive-action")
            self.btn_stream_pc.add_css_class("suggested-action")
            self.pc_to_phone_row.set_subtitle("Envía el escritorio completo y el sonido del sistema al móvil")

    def _update_phone_stream_ui(self, running: bool):
        if running:
            self.btn_stream_phone.set_label("Cerrar")
            self.btn_stream_phone.add_css_class("destructive-action")
            self.phone_to_pc_row.set_subtitle("Visualizando pantalla y audio del teléfono con scrcpy")
        else:
            self.btn_stream_phone.set_label("Abrir")
            self.btn_stream_phone.remove_css_class("destructive-action")
            self.phone_to_pc_row.set_subtitle("Proyecta y escucha la pantalla de tu móvil en una ventana del PC")

    def _toggle_pc_to_phone_stream(self, btn):
        label = btn.get_label()
        action = "stop" if label == "Detener" else "start"

        def worker():
            try:
                req = urllib.request.Request(
                    f"{DAEMON_URL}/api/stream/pc-to-phone/{action}",
                    data=b"{}",
                    headers={"X-Pulsar-Token": self.token, "Content-Type": "application/json"}
                )
                with urllib.request.urlopen(req, timeout=2.0) as resp:
                    pass
            except Exception as e:
                logger.error(f"Error toggling PC stream: {e}")
            GLib.idle_add(self._refresh_streaming_status)

        threading.Thread(target=worker, daemon=True).start()

    def _toggle_phone_to_pc_stream(self, btn):
        label = btn.get_label()
        action = "stop" if label == "Cerrar" else "start"

        def worker():
            try:
                req = urllib.request.Request(
                    f"{DAEMON_URL}/api/stream/phone-to-pc/{action}",
                    data=b"{}",
                    headers={"X-Pulsar-Token": self.token, "Content-Type": "application/json"}
                )
                with urllib.request.urlopen(req, timeout=2.0) as resp:
                    pass
            except Exception as e:
                logger.error(f"Error toggling Phone stream: {e}")
            GLib.idle_add(self._refresh_streaming_status)

        threading.Thread(target=worker, daemon=True).start()


class PulsarVpnApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id=APP_ID,
            flags=Gio.ApplicationFlags.FLAGS_NONE
        )

    def do_activate(self):
        self._ensure_bridge_running()
        win = self.props.active_window
        if not win:
            win = PulsarVpnWindow(self)
        win.present()

    def _ensure_bridge_running(self):
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.settimeout(0.5)
                res = s.connect_ex(("127.0.0.1", DEFAULT_PORT))
                if res == 0:
                    return
        except Exception:
            pass

        daemon_script = os.path.join(os.path.dirname(__file__), "bridge_daemon.py")
        if os.path.isfile(daemon_script):
            subprocess.Popen([sys.executable, daemon_script], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def main():
    app = PulsarVpnApp()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
