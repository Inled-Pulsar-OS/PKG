#!/usr/bin/env python3
"""
Pulsar Bridge Daemon
Secure local daemon connecting Pulsar OS with Pulsar Phone over Tailscale.
Features:
- Restricted to Tailscale IP & localhost (ZERO public internet exposure)
- Token-based authentication
- Broadway single-app management
- Notification relay (Android -> Linux Desktop)
- Clipboard sync
- Filesystem REST API for Android DocumentsProvider (SAF Native Mount)
"""

import http.server
import json
import logging
import mimetypes
import os
import queue
import secrets
import shutil
import socket
import socketserver
import subprocess
import sys
import threading
import time
import urllib.parse
from typing import Optional, Dict, Any, List

from tailscale_manager import TailscaleManager
from app_launcher import AppLauncher

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(levelname)s: %(message)s")
logger = logging.getLogger("PulsarBridge")

CONFIG_DIR = os.path.expanduser("~/.config/pulsaros")
TOKEN_FILE = os.path.join(CONFIG_DIR, "bridge_token.key")
DEFAULT_PORT = 8989
BROADWAY_PORT = 8085


def get_or_create_token() -> str:
    os.makedirs(CONFIG_DIR, exist_ok=True)
    if os.path.isfile(TOKEN_FILE):
        try:
            with open(TOKEN_FILE, "r") as f:
                t = f.read().strip()
                if t:
                    return t
        except Exception:
            pass

    token = secrets.token_hex(24)
    with open(TOKEN_FILE, "w") as f:
        f.write(token)
    os.chmod(TOKEN_FILE, 0o600)
    return token


def get_local_ip() -> str:
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
            s.connect(("8.8.8.8", 80))
            return s.getsockname()[0]
    except Exception:
        return "127.0.0.1"


def ask_remote_desktop_consent(requester_hostname: str, requester_ip: str) -> bool:
    """Prompts local desktop user with a modal confirmation dialog to allow remote control."""
    try:
        env = dict(os.environ)
        if "DISPLAY" not in env:
            env["DISPLAY"] = ":1" if os.path.exists("/tmp/.X11-unix/X1") else ":0"
        if "WAYLAND_DISPLAY" not in env and os.path.exists(f"/run/user/{os.getuid()}/wayland-0"):
            env["WAYLAND_DISPLAY"] = "wayland-0"

        msg = (
            f"El equipo <b>{requester_hostname}</b> ({requester_ip}) de tu red privada "
            f"solicita ver tu pantalla, escuchar el audio y controlar tu ratón y teclado.\n\n"
            f"¿Deseas permitir el acceso remoto?"
        )
        cmd = [
            "zenity", "--question",
            "--title=Pulsar OS • Acceso Remoto",
            f"--text={msg}",
            "--ok-label=Permitir Acceso",
            "--cancel-label=Rechazar",
            "--timeout=30",
            "--width=420"
        ]
        res = subprocess.run(cmd, env=env, timeout=32)
        return res.returncode == 0
    except Exception as e:
        logger.error(f"Error asking remote desktop consent: {e}")
        return False


def get_connected_monitors() -> List[Dict[str, Any]]:
    """Queries connected displays from Mutter DisplayConfig via D-Bus."""
    monitors_list = []
    try:
        from gi.repository import Gio
        bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        disp_proxy = Gio.DBusProxy.new_sync(
            bus, 0, None,
            "org.gnome.Mutter.DisplayConfig",
            "/org/gnome/Mutter/DisplayConfig",
            "org.gnome.Mutter.DisplayConfig",
            None
        )
        _, monitors, logical, _ = disp_proxy.call_sync("GetCurrentState", None, 0, 2000, None).unpack()

        primary_connectors = set()
        for lm in logical:
            if lm[4]:  # is_primary boolean
                for mon_spec in lm[5]:
                    primary_connectors.add(mon_spec[0])

        for m in monitors:
            spec = m[0]
            connector = spec[0]
            props = m[2] if len(m) > 2 else {}
            name = props.get("display-name") or f"Pantalla {connector}"
            is_builtin = props.get("is-builtin", False)
            is_primary = connector in primary_connectors
            monitors_list.append({
                "connector": connector,
                "name": name,
                "isPrimary": is_primary,
                "isBuiltin": is_builtin
            })
    except Exception as e:
        logger.warning(f"Error querying monitors from Mutter: {e}")
        monitors_list = [{"connector": "eDP-1", "name": "Pantalla principal", "isPrimary": True, "isBuiltin": True}]
    return monitors_list


class DesktopStreamManager:
    """Streams PC desktop video (Wayland ScreenCast / MJPEG) and audio (MP3) with zero latency."""
    def __init__(self):
        self.video_lock = threading.RLock()
        self.audio_lock = threading.RLock()
        self.pcm_audio_lock = threading.RLock()
        self.video_proc: Optional[subprocess.Popen] = None
        self.audio_proc: Optional[subprocess.Popen] = None
        self.pcm_audio_proc: Optional[subprocess.Popen] = None
        self.video_clients: List[queue.Queue] = []
        self.audio_clients: List[queue.Queue] = []
        self.pcm_audio_clients: List[queue.Queue] = []
        self.video_thread: Optional[threading.Thread] = None
        self.audio_thread: Optional[threading.Thread] = None
        self.pcm_audio_thread: Optional[threading.Thread] = None
        self.mutter_session = None
        self.current_connector: Optional[str] = None

    def is_running(self) -> bool:
        with self.video_lock:
            return (self.video_proc is not None and self.video_proc.poll() is None)

    def start_if_needed(self, connector: Optional[str] = None):
        self.start_video(connector)
        self.start_audio()

    def _start_wayland_stream(self, connector: Optional[str] = None) -> bool:
        try:
            from gi.repository import Gio, GLib
            bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)

            # Determine connector if not provided
            if not connector:
                monitors = get_connected_monitors()
                primary = next((m for m in monitors if m.get("isPrimary")), None)
                if primary:
                    connector = primary["connector"]
                elif monitors:
                    connector = monitors[0]["connector"]
                else:
                    connector = "eDP-1"

            proxy = Gio.DBusProxy.new_sync(
                bus, 0, None,
                "org.gnome.Mutter.ScreenCast",
                "/org/gnome/Mutter/ScreenCast",
                "org.gnome.Mutter.ScreenCast",
                None
            )
            res = proxy.call_sync("CreateSession", GLib.Variant("(a{sv})", ({},)), 0, 2000, None)
            session_path = res.unpack()[0]
            self.mutter_session = Gio.DBusProxy.new_sync(
                bus, 0, None,
                "org.gnome.Mutter.ScreenCast",
                session_path,
                "org.gnome.Mutter.ScreenCast.Session",
                None
            )

            stream_res = self.mutter_session.call_sync(
                "RecordMonitor",
                GLib.Variant("(sa{sv})", (connector, {"cursor-mode": GLib.Variant("u", 1)})),
                0, 2000, None
            )
            stream_path = stream_res.unpack()[0]

            node_id_holder = []
            loop = GLib.MainLoop()

            def on_sig(conn, sender, path, iface, name, params, udata):
                if name == "PipeWireStreamAdded":
                    node_id_holder.append(params.unpack()[0])
                    loop.quit()

            sub_id = bus.signal_subscribe(
                "org.gnome.Mutter.ScreenCast",
                "org.gnome.Mutter.ScreenCast.Stream",
                "PipeWireStreamAdded",
                stream_path,
                None, 0, on_sig, None
            )
            self.mutter_session.call_sync("Start", None, 0, 2000, None)
            GLib.timeout_add_seconds(3, loop.quit)
            loop.run()
            bus.signal_unsubscribe(sub_id)

            if not node_id_holder:
                logger.warning("Failed to obtain PipeWire node ID from Mutter ScreenCast")
                return False

            node_id = node_id_holder[0]
            cmd = [
                "gst-launch-1.0", "-q",
                "pipewiresrc", f"path={node_id}", "keepalive-time=33", "always-copy=true",
                "!", "videoconvert",
                "!", "videoscale",
                "!", "video/x-raw,width=1280,height=720",
                "!", "jpegenc", "quality=75",
                "!", "multipartmux", "boundary=frame",
                "!", "fdsink", "fd=1"
            ]
            self.video_proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            self.video_thread = threading.Thread(target=self._broadcast_multipart_video, daemon=True)
            self.video_thread.start()
            self.current_connector = connector
            logger.info(f"Wayland PipeWire desktop streamer started on {connector} (node {node_id})")
            return True
        except Exception as e:
            logger.warning(f"Could not start Wayland ScreenCast: {e}")
            return False

    def _start_x11_stream(self):
        env = os.environ.copy()
        if "DISPLAY" not in env or not env["DISPLAY"]:
            env["DISPLAY"] = ":1"
        display = env["DISPLAY"]

        cmd = [
            "ffmpeg",
            "-f", "x11grab",
            "-framerate", "30",
            "-i", display,
            "-vf", "scale=1280:-2",
            "-c:v", "mjpeg",
            "-q:v", "5",
            "-f", "image2pipe",
            "-"
        ]
        try:
            self.video_proc = subprocess.Popen(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            self.video_thread = threading.Thread(target=self._broadcast_x11_video, daemon=True)
            self.video_thread.start()
            self.current_connector = "X11"
            logger.info(f"Desktop X11 video streamer started on {display}")
        except Exception as e:
            logger.error(f"Failed to start desktop video: {e}")

    def start_video(self, connector: Optional[str] = None):
        with self.video_lock:
            if self.video_proc and self.video_proc.poll() is None:
                if connector and self.current_connector and connector != self.current_connector:
                    logger.info(f"Switching video stream connector from {self.current_connector} to {connector}")
                    self.stop_video()
                else:
                    return

            if self._start_wayland_stream(connector):
                return

            self._start_x11_stream()

    def _broadcast_multipart_video(self):
        buf = b""
        while True:
            proc = self.video_proc
            if not proc or proc.poll() is not None or not proc.stdout:
                break
            try:
                chunk = proc.stdout.read(4096)
                if not chunk:
                    break
                buf += chunk
                while True:
                    pos = buf.find(b"--frame", 7)
                    if pos == -1:
                        if len(buf) > 500000:
                            buf = buf[-4096:]
                        break
                    packet = buf[:pos]
                    buf = buf[pos:]

                    with self.video_lock:
                        clients = list(self.video_clients)

                    for q in clients:
                        try:
                            if q.full():
                                try:
                                    q.get_nowait()
                                except queue.Empty:
                                    pass
                            q.put_nowait(packet)
                        except Exception:
                            with self.video_lock:
                                if q in self.video_clients:
                                    self.video_clients.remove(q)
            except Exception:
                break
        self.stop_video()

    def _broadcast_x11_video(self):
        buf = b""
        while True:
            proc = self.video_proc
            if not proc or proc.poll() is not None or not proc.stdout:
                break
            try:
                chunk = proc.stdout.read(4096)
                if not chunk:
                    break
                buf += chunk
                while True:
                    soi = buf.find(b"\xff\xd8")
                    if soi == -1:
                        if len(buf) > 32768: buf = buf[-4096:]
                        break
                    eoi = buf.find(b"\xff\xd9", soi)
                    if eoi == -1:
                        break
                    frame = buf[soi:eoi+2]
                    buf = buf[eoi+2:]

                    frame_packet = (
                        b"--frame\r\n"
                        b"Content-Type: image/jpeg\r\n"
                        b"Content-Length: " + str(len(frame)).encode("ascii") + b"\r\n\r\n" +
                        frame + b"\r\n"
                    )

                    with self.video_lock:
                        clients = list(self.video_clients)

                    for q in clients:
                        try:
                            if q.full():
                                try:
                                    q.get_nowait()
                                except queue.Empty:
                                    pass
                            q.put_nowait(frame_packet)
                        except Exception:
                            with self.video_lock:
                                if q in self.video_clients:
                                    self.video_clients.remove(q)
            except Exception:
                break
        self.stop_video()

    def register_video_client(self, connector: Optional[str] = None) -> queue.Queue:
        q = queue.Queue(maxsize=30)
        with self.video_lock:
            self.video_clients.append(q)
        self.start_video(connector=connector)
        return q

    def unregister_video_client(self, q: queue.Queue):
        with self.video_lock:
            if q in self.video_clients:
                self.video_clients.remove(q)
            rem = len(self.video_clients)
        if rem == 0:
            self.stop_video()

    def stop_video(self):
        with self.video_lock:
            proc = self.video_proc
            self.video_proc = None
            self.video_clients.clear()
            session = self.mutter_session
            self.mutter_session = None
            self.current_connector = None

        if session:
            try:
                session.call_sync("Stop", None, 0, 1000, None)
            except Exception:
                pass

        if proc:
            try:
                proc.terminate()
                proc.wait(timeout=0.5)
            except Exception:
                try: proc.kill()
                except Exception: pass

    def start_audio(self):
        with self.audio_lock:
            if self.audio_proc and self.audio_proc.poll() is None:
                return

            # Capture desktop speaker monitor with zero buffering and low delay
            cmd = [
                "ffmpeg",
                "-fflags", "nobuffer",
                "-flags", "low_delay",
                "-f", "pulse",
                "-i", "@DEFAULT_SINK@.monitor",
                "-c:a", "libmp3lame",
                "-b:a", "128k",
                "-flush_packets", "1",
                "-f", "mp3",
                "-"
            ]
            try:
                self.audio_proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
                self.audio_thread = threading.Thread(target=self._broadcast_audio, daemon=True)
                self.audio_thread.start()
                logger.info("Desktop low-latency audio streamer started with @DEFAULT_SINK@.monitor")
            except Exception as e:
                logger.error(f"Failed to start desktop audio: {e}")

    def _broadcast_audio(self):
        while True:
            proc = self.audio_proc
            if not proc or proc.poll() is not None or not proc.stdout:
                break
            try:
                chunk = proc.stdout.read(4096)
                if not chunk:
                    break
                with self.audio_lock:
                    clients = list(self.audio_clients)

                for q in clients:
                    try:
                        while q.qsize() >= 12:
                            try:
                                q.get_nowait()
                            except queue.Empty:
                                break
                        q.put_nowait(chunk)
                    except Exception:
                        with self.audio_lock:
                            if q in self.audio_clients:
                                self.audio_clients.remove(q)
            except Exception:
                break
        self.stop_audio()

    def register_audio_client(self) -> queue.Queue:
        q = queue.Queue(maxsize=15)
        with self.audio_lock:
            self.audio_clients.append(q)
        self.start_audio()
        return q

    def unregister_audio_client(self, q: queue.Queue):
        with self.audio_lock:
            if q in self.audio_clients:
                self.audio_clients.remove(q)
            rem = len(self.audio_clients)
        if rem == 0:
            self.stop_audio()

    def stop_audio(self):
        with self.audio_lock:
            proc = self.audio_proc
            self.audio_proc = None
            self.audio_clients.clear()
        if proc:
            try:
                proc.terminate()
                proc.wait(timeout=0.5)
            except Exception:
                try: proc.kill()
                except Exception: pass

    def start_pcm_audio(self):
        with self.pcm_audio_lock:
            if self.pcm_audio_proc and self.pcm_audio_proc.poll() is None:
                return

            cmd = [
                "ffmpeg",
                "-fflags", "nobuffer",
                "-flags", "low_delay",
                "-f", "pulse",
                "-i", "@DEFAULT_SINK@.monitor",
                "-f", "s16le",
                "-ar", "48000",
                "-ac", "2",
                "-"
            ]
            try:
                self.pcm_audio_proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
                self.pcm_audio_thread = threading.Thread(target=self._broadcast_pcm_audio, daemon=True)
                self.pcm_audio_thread.start()
                logger.info("Desktop low-latency PCM audio streamer started (48000Hz stereo)")
            except Exception as e:
                logger.error(f"Failed to start desktop PCM audio: {e}")

    def _broadcast_pcm_audio(self):
        chunk_size = 1920  # 10ms of 48kHz 16-bit stereo PCM
        while True:
            proc = self.pcm_audio_proc
            if not proc or proc.poll() is not None or not proc.stdout:
                break
            try:
                chunk = proc.stdout.read(chunk_size)
                if not chunk:
                    break
                with self.pcm_audio_lock:
                    clients = list(self.pcm_audio_clients)

                for q in clients:
                    try:
                        while q.qsize() >= 5:
                            try:
                                q.get_nowait()
                            except queue.Empty:
                                break
                        q.put_nowait(chunk)
                    except Exception:
                        with self.pcm_audio_lock:
                            if q in self.pcm_audio_clients:
                                self.pcm_audio_clients.remove(q)
            except Exception:
                break
        self.stop_pcm_audio()

    def register_pcm_client(self) -> queue.Queue:
        q = queue.Queue(maxsize=10)
        with self.pcm_audio_lock:
            self.pcm_audio_clients.append(q)
        self.start_pcm_audio()
        return q

    def unregister_pcm_client(self, q: queue.Queue):
        with self.pcm_audio_lock:
            if q in self.pcm_audio_clients:
                self.pcm_audio_clients.remove(q)
            rem = len(self.pcm_audio_clients)
        if rem == 0:
            self.stop_pcm_audio()

    def stop_pcm_audio(self):
        with self.pcm_audio_lock:
            proc = self.pcm_audio_proc
            self.pcm_audio_proc = None
            self.pcm_audio_clients.clear()
        if proc:
            try:
                proc.terminate()
                proc.wait(timeout=0.5)
            except Exception:
                try: proc.kill()
                except Exception: pass

    def stop(self):
        self.stop_video()
        self.stop_audio()
        self.stop_pcm_audio()


class ScrcpyManager:
    """Manages streaming phone screen & audio to PC desktop using scrcpy with audio."""
    def __init__(self):
        self.proc: Optional[subprocess.Popen] = None

    def is_running(self) -> bool:
        return self.proc is not None and self.proc.poll() is None

    def _find_device_serial(self, phone_ip: Optional[str] = None) -> Optional[str]:
        # Try Wi-Fi / IP first if provided
        if phone_ip and phone_ip not in ("127.0.0.1", "localhost", "0.0.0.0"):
            try:
                subprocess.run(["adb", "connect", f"{phone_ip}:5555"], timeout=2, capture_output=True)
            except Exception:
                pass

        try:
            res = subprocess.run(["adb", "devices"], capture_output=True, text=True, timeout=2)
            lines = [l.strip().split() for l in res.stdout.strip().splitlines()[1:] if l.strip()]
            for dev in lines:
                if len(dev) >= 2 and dev[1] == "device":
                    if phone_ip and phone_ip in dev[0]:
                        return dev[0]
            # Return first available connected device (e.g. ZY32FTXBXD)
            for dev in lines:
                if len(dev) >= 2 and dev[1] == "device":
                    return dev[0]
        except Exception:
            pass
        return None

    def start(self, phone_ip: Optional[str] = None, fullscreen: bool = False) -> bool:
        self.stop()
        device_serial = self._find_device_serial(phone_ip)

        scrcpy_bin = shutil.which("scrcpy") or "scrcpy"
        env = os.environ.copy()
        if "DISPLAY" not in env or not env["DISPLAY"]:
            env["DISPLAY"] = ":1"
        if "WAYLAND_DISPLAY" not in env or not env["WAYLAND_DISPLAY"]:
            env["WAYLAND_DISPLAY"] = "wayland-0"
        env["XDG_RUNTIME_DIR"] = f"/run/user/{os.getuid()}"

        cmd = [
            scrcpy_bin,
            "--audio-source=output",
            "--audio-codec=opus",
            "--audio-bit-rate=128K",
            "--audio-buffer=150",
            "--audio-output-buffer=40",
            "--video-bit-rate=4M",
            "--max-size=1600",
            "--window-title=Pulsar Phone (Pantalla y Sonido)"
        ]
        if fullscreen:
            cmd.append("--fullscreen")
        if device_serial:
            cmd.extend(["-s", device_serial])

        try:
            self.proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            logger.info(f"Launched scrcpy phone stream on device {device_serial} (fullscreen={fullscreen}): {' '.join(cmd)}")
            return True
        except Exception as e:
            logger.error(f"Error launching scrcpy: {e}")
            return False

    def stop(self):
        if self.proc:
            try:
                self.proc.terminate()
                self.proc.wait(timeout=1.0)
            except Exception:
                try:
                    self.proc.kill()
                except Exception:
                    pass
            self.proc = None
            logger.info("Scrcpy phone stream stopped")

    def get_adb_status(self) -> Dict[str, Any]:
        try:
            res = subprocess.run(["adb", "devices"], capture_output=True, text=True, timeout=2)
            lines = [l.strip().split() for l in res.stdout.strip().splitlines()[1:] if l.strip()]
            devices = []
            for dev in lines:
                if len(dev) >= 2:
                    serial, state = dev[0], dev[1]
                    is_wifi = ":5555" in serial
                    devices.append({"serial": serial, "state": state, "type": "wifi" if is_wifi else "usb"})
            return {"devices": devices}
        except Exception as e:
            return {"devices": [], "error": str(e)}

    def enable_adb_tcpip(self) -> Dict[str, Any]:
        status = self.get_adb_status()
        devices = status.get("devices", [])
        usb_devs = [d for d in devices if d["type"] == "usb" and d["state"] == "device"]

        if not usb_devs:
            wifi_devs = [d for d in devices if d["type"] == "wifi" and d["state"] == "device"]
            if wifi_devs:
                return {"success": True, "already_connected": True, "ip": wifi_devs[0]["serial"], "message": "Already connected over Wi-Fi"}
            return {"success": False, "error": "No USB device connected"}

        serial = usb_devs[0]["serial"]
        phone_ip = None
        try:
            import re
            ip_res = subprocess.run(["adb", "-s", serial, "shell", "ip route get 1.1.1.1"], capture_output=True, text=True, timeout=3)
            m = re.search(r"src\s+([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+)", ip_res.stdout)
            if m:
                phone_ip = m.group(1)
            else:
                ip_res2 = subprocess.run(["adb", "-s", serial, "shell", "ip -f inet addr show wlan0"], capture_output=True, text=True, timeout=3)
                m2 = re.search(r"inet\s+([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+)", ip_res2.stdout)
                if m2:
                    phone_ip = m2.group(1)
        except Exception as e:
            logger.warning(f"Failed to query phone IP via ADB: {e}")

        try:
            tcpip_res = subprocess.run(["adb", "-s", serial, "tcpip", "5555"], capture_output=True, text=True, timeout=5)
            if tcpip_res.returncode != 0:
                return {"success": False, "error": tcpip_res.stderr.strip() or "adb tcpip failed"}
        except Exception as e:
            return {"success": False, "error": str(e)}

        time.sleep(1.5)

        if phone_ip:
            try:
                subprocess.run(["adb", "connect", f"{phone_ip}:5555"], capture_output=True, text=True, timeout=5)
                return {"success": True, "ip": phone_ip, "message": f"Connected to {phone_ip}:5555"}
            except Exception as e:
                return {"success": True, "ip": phone_ip, "message": f"Port 5555 enabled. Connect manually: {e}"}

        return {"success": True, "message": "Port 5555 enabled on device"}


class SleepInhibitorManager:
    """Manages system sleep/idle inhibition using systemd-inhibit for 24/7 remote access."""
    def __init__(self):
        self.lock = threading.Lock()
        self.proc: Optional[subprocess.Popen] = None
        self.config_file = os.path.expanduser("~/.config/pulsaros/inhibit_sleep.enabled")
        if os.path.exists(self.config_file):
            self.set_inhibit(True)

    def is_inhibited(self) -> bool:
        with self.lock:
            return self.proc is not None and self.proc.poll() is None

    def set_inhibit(self, enable: bool) -> bool:
        with self.lock:
            if enable:
                if self.proc and self.proc.poll() is None:
                    return True
                try:
                    self.proc = subprocess.Popen([
                        "systemd-inhibit",
                        "--what=idle:sleep:handle-lid-switch",
                        "--who=Pulsar VPN",
                        "--why=Acceso remoto activo y retransmisión",
                        "sleep", "infinity"
                    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    os.makedirs(os.path.dirname(self.config_file), exist_ok=True)
                    with open(self.config_file, "w") as f:
                        f.write("1")
                    logger.info(f"System sleep inhibited successfully (PID: {self.proc.pid})")
                    return True
                except Exception as e:
                    logger.error(f"Failed to inhibit sleep: {e}")
                    return False
            else:
                if self.proc:
                    try:
                        self.proc.terminate()
                        self.proc.wait(timeout=0.5)
                    except Exception:
                        try: self.proc.kill()
                        except Exception: pass
                    self.proc = None
                if os.path.exists(self.config_file):
                    try: os.remove(self.config_file)
                    except Exception: pass
                logger.info("System sleep inhibition released")
                return True


class BridgeContext:
    def __init__(self):
        self.token = get_or_create_token()
        self.tailscale = TailscaleManager()
        self.sleep_inhibitor = SleepInhibitorManager()
        self.bind_ip = "0.0.0.0"
        self.lan_ip = get_local_ip()
        self.tailscale_ip: Optional[str] = None
        self.port = DEFAULT_PORT
        self.broadway_port = BROADWAY_PORT
        self.launcher = AppLauncher(bind_ip="0.0.0.0", port=self.broadway_port)
        self.desktop_streamer = DesktopStreamManager()
        self.scrcpy_mgr = ScrcpyManager()
        self.refresh_ip()

    def refresh_ip(self):
        self.lan_ip = get_local_ip()
        ts_status = self.tailscale.get_status()
        self.tailscale_ip = ts_status.get("ip")
        logger.info(f"Pulsar Bridge IPs - LAN: {self.lan_ip}, Tailscale: {self.tailscale_ip}")

    def get_preferred_ip(self) -> str:
        # Prefer Tailscale IP if active, otherwise LAN IP for Wi-Fi
        if self.tailscale_ip:
            return self.tailscale_ip
        return self.lan_ip


context = BridgeContext()


class BridgeRequestHandler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def send_cors_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, X-Pulsar-Token")

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_cors_headers()
        self.end_headers()

    def do_HEAD(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        if path in ("/api/stream/pc-to-phone/live.mjpeg", "/api/stream/pc-to-phone/audio.mp3", "/api/stream/pc-to-phone/view", "/api/stream/pc-to-phone/live.ts", "/api/download/pulsar-phone.apk", "/api/ping"):
            self.send_response(200)
            self.send_cors_headers()
            if path == "/api/stream/pc-to-phone/audio.mp3":
                self.send_header("Content-Type", "audio/mpeg")
            elif path in ("/api/stream/pc-to-phone/live.mjpeg", "/api/stream/pc-to-phone/live.ts"):
                self.send_header("Content-Type", "multipart/x-mixed-replace; boundary=frame")
            elif path == "/api/stream/pc-to-phone/view":
                self.send_header("Content-Type", "text/html; charset=utf-8")
            elif path == "/api/download/pulsar-phone.apk":
                self.send_header("Content-Type", "application/vnd.android.package-archive")
            self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            self.send_header("Accept-Ranges", "none")
            self.end_headers()
            return
        if not self.check_auth():
            return
        self.send_response(200)
        self.send_cors_headers()
        self.end_headers()

    def check_auth(self) -> bool:
        parsed_path = urllib.parse.urlparse(self.path).path
        if parsed_path == "/api/remote-desktop/request":
            return True

        auth_token = self.headers.get("X-Pulsar-Token")
        if not auth_token:
            # Check query param as fallback
            query = urllib.parse.urlparse(self.path).query
            params = urllib.parse.parse_qs(query)
            auth_token = params.get("token", [None])[0]

        if auth_token != context.token:
            self.send_response(401)
            self.send_cors_headers()
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error": "Unauthorized: Invalid or missing X-Pulsar-Token"}).encode("utf-8"))
            return False

        # Authenticated phone client - store IP and ensure wireless adb
        client_ip = self.client_address[0]
        if client_ip not in ("127.0.0.1", "::1", "localhost"):
            def track_phone_ip(ip):
                try:
                    pfile = os.path.expanduser("~/.config/pulsaros/last_phone_ip.txt")
                    os.makedirs(os.path.dirname(pfile), exist_ok=True)
                    with open(pfile, "w") as f:
                        f.write(ip)
                    subprocess.run(["adb", "connect", f"{ip}:5555"], timeout=1.5, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                except Exception:
                    pass
            threading.Thread(target=track_phone_ip, args=(client_ip,), daemon=True).start()

        return True

    def send_json(self, status_code: int, data: Any):
        payload = json.dumps(data).encode("utf-8")
        self.send_response(status_code)
        self.send_cors_headers()
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        params = urllib.parse.parse_qs(parsed.query)

        # Pair QR check / Ping without token (only returns public alias/status)
        if path == "/api/ping":
            self.send_json(200, {
                "service": "PulsarBridge",
                "hostname": socket.gethostname(),
                "version": "1.0.0",
                "broadwayPort": context.broadway_port
            })
            return

        if path == "/api/download/pulsar-phone.apk":
            apk_path = "/home/jaime/Documentos/pulsar/pulsar-phone/app/build/outputs/apk/debug/app-debug.apk"
            if os.path.isfile(apk_path):
                self._serve_file(apk_path)
            else:
                self.send_json(404, {"error": "APK not found"})
            return

        if not self.check_auth():
            return

        if path == "/api/status":
            ts_status = context.tailscale.get_status()
            active_app = context.launcher.get_active_state()
            self.send_json(200, {
                "hostname": socket.gethostname(),
                "tailscale": ts_status,
                "activeApp": active_app,
                "flydropPort": 53317,
                "broadwayPort": context.broadway_port
            })
        elif path == "/api/apps":
            apps = context.launcher.list_apps()
            self.send_json(200, {"apps": apps})
        elif path == "/api/clipboard":
            text = self._get_pc_clipboard()
            self.send_json(200, {"text": text})
        elif path == "/api/fs/roots":
            roots = self._get_fs_roots()
            self.send_json(200, {"roots": roots})
        elif path == "/api/fs/children":
            target_path = params.get("path", [os.path.expanduser("~")])[0]
            children = self._list_fs_dir(target_path)
            self.send_json(200, {"path": target_path, "items": children})
        elif path == "/api/fs/file":
            target_path = params.get("path", [""])[0]
            self._serve_file(target_path)
        elif path == "/api/stream/pc-to-phone/monitors":
            self.send_json(200, {
                "monitors": get_connected_monitors(),
                "current": context.desktop_streamer.current_connector
            })
        elif path == "/api/stream/pc-to-phone/live.mjpeg":
            target_mon = params.get("monitor", [None])[0]
            self._serve_live_mjpeg(target_mon)
        elif path == "/api/stream/pc-to-phone/audio.pcm":
            self._serve_audio_pcm()
        elif path == "/api/stream/pc-to-phone/audio.mp3":
            self._serve_audio_mp3()
        elif path == "/api/stream/pc-to-phone/view":
            target_mon = params.get("monitor", [None])[0]
            no_audio = params.get("noaudio", ["0"])[0] in ("1", "true")
            control = params.get("control", ["0"])[0] in ("1", "true")
            self._serve_stream_view(target_mon, no_audio=no_audio, control=control)
        elif path == "/api/stream/pc-to-phone/live.ts":
            target_mon = params.get("monitor", [None])[0]
            self._serve_live_mjpeg(target_mon)
        elif path == "/api/stream/pc-to-phone/status":
            self.send_json(200, {
                "active": context.desktop_streamer.is_running(),
                "currentMonitor": context.desktop_streamer.current_connector,
                "videoClients": len(context.desktop_streamer.video_clients),
                "audioClients": len(context.desktop_streamer.audio_clients),
                "pcmAudioClients": len(context.desktop_streamer.pcm_audio_clients),
                "viewUrl": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/view?token={context.token}",
                "mjpegUrl": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/live.mjpeg?token={context.token}",
                "audioUrl": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/audio.mp3?token={context.token}",
                "pcmAudioUrl": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/audio.pcm?token={context.token}"
            })
        elif path == "/api/stream/phone-to-pc/status":
            self.send_json(200, {
                "running": context.scrcpy_mgr.is_running()
            })
        elif path == "/api/adb/status":
            self.send_json(200, context.scrcpy_mgr.get_adb_status())
        elif path == "/api/system/inhibit-sleep":
            self.send_json(200, {
                "enabled": context.sleep_inhibitor.is_inhibited()
            })
        else:
            self.send_json(404, {"error": "Not found"})

    def do_POST(self):
        if not self.check_auth():
            return

        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length > 0 else b"{}"

        try:
            req_data = json.loads(body.decode("utf-8")) if body else {}
        except Exception:
            req_data = {}

        if path == "/api/apps/launch":
            app_id = req_data.get("appId")
            if not app_id:
                self.send_json(400, {"error": "Missing appId"})
                return
            try:
                host_hdr = self.headers.get("Host", "").split(":")[0]
                client_ip = host_hdr if host_hdr and host_hdr not in ("0.0.0.0", "127.0.0.1") else context.get_preferred_ip()
                dark_mode = req_data.get("darkMode", True)
                res = context.launcher.launch_app(app_id, client_host=client_ip, dark_mode=dark_mode)
                self.send_json(200, res)
            except Exception as e:
                logger.error(f"Error launching app {app_id}: {e}")
                self.send_json(500, {"error": str(e)})

        elif path == "/api/apps/close":
            context.launcher.close_current_app()
            self.send_json(200, {"status": "closed"})

        elif path == "/api/notify":
            app_name = req_data.get("appName", "Pulsar Phone")
            title = req_data.get("title", "Notificación")
            text = req_data.get("text", "")
            self._send_desktop_notification(app_name, title, text)
            self.send_json(200, {"status": "ok"})

        elif path == "/api/clipboard":
            text = req_data.get("text", "")
            self._set_pc_clipboard(text)
            self.send_json(200, {"status": "ok"})

        elif path == "/api/fs/mkdir":
            parent = req_data.get("path", "")
            name = req_data.get("name", "")
            new_path = os.path.join(parent, name)
            try:
                os.makedirs(new_path, exist_ok=True)
                self.send_json(200, {"status": "created", "path": new_path})
            except Exception as e:
                self.send_json(500, {"error": str(e)})

        elif path == "/api/stream/phone-to-pc/start":
            phone_ip = req_data.get("phoneIp")
            if not phone_ip:
                # Use client ip from connection
                phone_ip = self.client_address[0]
            fullscreen = req_data.get("fullscreen", False)
            ok = context.scrcpy_mgr.start(phone_ip, fullscreen=fullscreen)
            self.send_json(200, {"status": "started" if ok else "error"})

        elif path == "/api/stream/phone-to-pc/stop":
            context.scrcpy_mgr.stop()
            self.send_json(200, {"status": "stopped"})

        elif path == "/api/stream/pc-to-phone/start":
            target_mon = req_data.get("monitor")
            context.desktop_streamer.start_if_needed(connector=target_mon)
            self.send_json(200, {
                "status": "started",
                "monitor": context.desktop_streamer.current_connector,
                "url": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/view?token={context.token}"
            })

        elif path == "/api/stream/pc-to-phone/stop":
            context.desktop_streamer.stop()
            self.send_json(200, {"status": "stopped"})

        elif path == "/api/adb/tcpip":
            res = context.scrcpy_mgr.enable_adb_tcpip()
            self.send_json(200, res)

        elif path == "/api/phone/apps/sync":
            try:
                import app_sync
                serial = req_data.get("serial")
                ok, count, msg = app_sync.sync_phone_apps(serial)
                self.send_json(200, {"success": ok, "count": count, "message": msg})
            except Exception as e:
                self.send_json(500, {"success": False, "error": str(e)})

        elif path == "/api/system/inhibit-sleep":
            enable = req_data.get("enabled", True)
            ok = context.sleep_inhibitor.set_inhibit(enable)
            self.send_json(200, {
                "enabled": context.sleep_inhibitor.is_inhibited(),
                "success": ok
            })

        elif path == "/api/shell/exec":
            command = req_data.get("command", "").strip()
            cwd = req_data.get("cwd") or os.path.expanduser("~")
            if not command:
                self.send_json(400, {"success": False, "error": "Missing command"})
                return
            try:
                logger.info(f"Executing remote shell command: {command[:60]} (cwd={cwd})")
                proc = subprocess.run(
                    ["/bin/bash", "-c", command],
                    cwd=cwd,
                    capture_output=True,
                    text=True,
                    timeout=30,
                    env=dict(os.environ, TERM="xterm-256color")
                )
                self.send_json(200, {
                    "success": True,
                    "exit_code": proc.returncode,
                    "stdout": proc.stdout,
                    "stderr": proc.stderr,
                    "cwd": cwd
                })
            except subprocess.TimeoutExpired:
                self.send_json(200, {
                    "success": False,
                    "exit_code": 124,
                    "stdout": "",
                    "stderr": "Command execution timed out after 30s",
                    "cwd": cwd
                })
            except Exception as e:
                logger.error(f"Error executing shell command: {e}")
                self.send_json(500, {"success": False, "error": str(e)})

        elif path == "/api/remote-desktop/request":
            req_host = req_data.get("requester_hostname", "Equipo remoto")
            req_ip = req_data.get("requester_ip", self.client_address[0])
            logger.info(f"Received remote desktop request from {req_host} ({req_ip})")
            allowed = ask_remote_desktop_consent(req_host, req_ip)
            if allowed:
                logger.info(f"Remote desktop access granted to {req_host} ({req_ip})")
                context.desktop_streamer.start_if_needed()
                self.send_json(200, {
                    "allowed": True,
                    "token": context.token,
                    "viewUrl": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/view?token={context.token}&control=1",
                    "mjpegUrl": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/live.mjpeg?token={context.token}",
                    "audioUrl": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/audio.mp3?token={context.token}"
                })
            else:
                logger.warning(f"Remote desktop access denied to {req_host} ({req_ip})")
                self.send_json(403, {
                    "allowed": False,
                    "reason": "Acceso rechazado por el usuario del equipo remoto"
                })

        elif path == "/api/remote-desktop/input":
            action = req_data.get("action", "")
            try:
                if action == "click":
                    x = req_data.get("x")
                    y = req_data.get("y")
                    btn = req_data.get("button", 0)  # 0: left, 2: right
                    btn_hex = "0xC1" if btn == 2 else "0xC0"
                    if x is not None and y is not None:
                        subprocess.run(["ydotool", "mousemove", "-a", "-x", str(x), "-y", str(y)], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    subprocess.run(["ydotool", "click", btn_hex], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    self.send_json(200, {"status": "ok"})
                elif action == "move":
                    x = req_data.get("x")
                    y = req_data.get("y")
                    if x is not None and y is not None:
                        subprocess.run(["ydotool", "mousemove", "-a", "-x", str(x), "-y", str(y)], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    self.send_json(200, {"status": "ok"})
                elif action == "type":
                    text = req_data.get("text", "")
                    if text:
                        subprocess.run(["ydotool", "type", text], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    self.send_json(200, {"status": "ok"})
                elif action == "key":
                    key = req_data.get("key", "")
                    if key == "Enter":
                        subprocess.run(["ydotool", "key", "28:1", "28:0"], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    elif key == "Backspace":
                        subprocess.run(["ydotool", "key", "14:1", "14:0"], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    elif key == "Escape":
                        subprocess.run(["ydotool", "key", "1:1", "1:0"], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    elif key in ("Space", " "):
                        subprocess.run(["ydotool", "key", "57:1", "57:0"], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    elif len(key) == 1:
                        subprocess.run(["ydotool", "type", key], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    self.send_json(200, {"status": "ok"})
                else:
                    self.send_json(400, {"error": f"Unknown action: {action}"})
            except Exception as e:
                logger.error(f"Error handling remote input: {e}")
                self.send_json(500, {"error": str(e)})
        else:
            self.send_json(404, {"error": "Not found"})

    def do_PUT(self):
        if not self.check_auth():
            return
        parsed = urllib.parse.urlparse(self.path)
        params = urllib.parse.parse_qs(parsed.query)
        target_path = params.get("path", [""])[0]

        if not target_path or not self._is_path_safe(target_path):
            self.send_json(400, {"error": "Invalid or unsafe path"})
            return

        try:
            length = int(self.headers.get("Content-Length", 0))
            os.makedirs(os.path.dirname(target_path), exist_ok=True)
            with open(target_path, "wb") as f:
                remaining = length
                while remaining > 0:
                    chunk = self.rfile.read(min(remaining, 65536))
                    if not chunk:
                        break
                    f.write(chunk)
                    remaining -= len(chunk)
            self.send_json(200, {"status": "written", "path": target_path})
        except Exception as e:
            logger.error(f"Error writing file {target_path}: {e}")
            self.send_json(500, {"error": str(e)})

    def do_DELETE(self):
        if not self.check_auth():
            return
        parsed = urllib.parse.urlparse(self.path)
        params = urllib.parse.parse_qs(parsed.query)
        target_path = params.get("path", [""])[0]

        if not target_path or not self._is_path_safe(target_path):
            self.send_json(400, {"error": "Invalid or unsafe path"})
            return

        try:
            if os.path.isdir(target_path):
                shutil.rmtree(target_path)
            elif os.path.isfile(target_path):
                os.remove(target_path)
            self.send_json(200, {"status": "deleted", "path": target_path})
        except Exception as e:
            self.send_json(500, {"error": str(e)})

    def _serve_file(self, target_path: str):
        if not self._is_path_safe(target_path) or not os.path.isfile(target_path):
            self.send_json(404, {"error": "File not found"})
            return

        try:
            fsize = os.path.getsize(target_path)
            mime, _ = mimetypes.guess_type(target_path)
            if not mime:
                mime = "application/octet-stream"

            self.send_response(200)
            self.send_cors_headers()
            self.send_header("Content-Type", mime)
            self.send_header("Content-Length", str(fsize))
            self.end_headers()

            with open(target_path, "rb") as f:
                while True:
                    chunk = f.read(65536)
                    if not chunk:
                        break
                    self.wfile.write(chunk)
        except Exception as e:
            logger.error(f"Error serving {target_path}: {e}")

    def _serve_live_mjpeg(self, monitor: Optional[str] = None):
        client_q = context.desktop_streamer.register_video_client(connector=monitor)
        try:
            self.send_response(200)
            self.send_cors_headers()
            self.send_header("Content-Type", "multipart/x-mixed-replace; boundary=frame")
            self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            self.send_header("Pragma", "no-cache")
            self.send_header("Expires", "0")
            self.end_headers()

            while True:
                try:
                    packet = client_q.get(timeout=3.0)
                    self.wfile.write(packet)
                    self.wfile.flush()
                except queue.Empty:
                    if not context.desktop_streamer.is_running():
                        break
                    continue
        except (BrokenPipeError, ConnectionResetError, Exception) as e:
            logger.debug(f"MJPEG client disconnected: {e}")
        finally:
            context.desktop_streamer.unregister_video_client(client_q)

    def _serve_audio_pcm(self):
        client_q = context.desktop_streamer.register_pcm_client()
        try:
            self.send_response(200)
            self.send_cors_headers()
            self.send_header("Content-Type", "audio/l16; rate=48000; channels=2")
            self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            self.send_header("Pragma", "no-cache")
            self.send_header("Expires", "0")
            self.end_headers()

            while True:
                try:
                    chunk = client_q.get(timeout=3.0)
                    self.wfile.write(chunk)
                    self.wfile.flush()
                except queue.Empty:
                    continue
        except (BrokenPipeError, ConnectionResetError, Exception) as e:
            logger.debug(f"PCM audio client disconnected: {e}")
        finally:
            context.desktop_streamer.unregister_pcm_client(client_q)

    def _serve_audio_mp3(self):
        client_q = context.desktop_streamer.register_audio_client()
        try:
            self.send_response(200)
            self.send_cors_headers()
            self.send_header("Content-Type", "audio/mpeg")
            self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            self.send_header("Pragma", "no-cache")
            self.send_header("Expires", "0")
            self.end_headers()

            while True:
                try:
                    chunk = client_q.get(timeout=3.0)
                    self.wfile.write(chunk)
                    self.wfile.flush()
                except queue.Empty:
                    continue
        except (BrokenPipeError, ConnectionResetError, Exception) as e:
            logger.debug(f"Audio client disconnected: {e}")
        finally:
            context.desktop_streamer.unregister_audio_client(client_q)

    def _serve_stream_view(self, monitor: Optional[str] = None, no_audio: bool = False, control: bool = False):
        mon_param = f"&monitor={monitor}" if monitor else ""
        audio_markup = ""
        if not no_audio:
            audio_markup = f"""<audio id="streamAudio" autoplay preload="none" src="/api/stream/pc-to-phone/audio.mp3?token={context.token}"></audio>
<script>
const audio = document.getElementById('streamAudio');
if (audio) {{
    audio.play().catch(() => {{
        window.addEventListener('click', () => audio.play(), {{ once: true }});
        window.addEventListener('touchstart', () => audio.play(), {{ once: true }});
    }});
}}
</script>"""

        control_badge = ""
        control_script = ""
        if control:
            control_badge = """<div id="controlBadge" style="position:fixed; top:12px; left:50%; transform:translateX(-50%); background:rgba(20,20,25,0.85); backdrop-filter:blur(8px); color:#22c55e; padding:6px 16px; border-radius:20px; font-family:sans-serif; font-size:13px; font-weight:600; border:1px solid rgba(34,197,94,0.4); z-index:9999; pointer-events:none; box-shadow:0 4px 16px rgba(0,0,0,0.5);">🟢 Control Remoto Activo • Clic y Teclado Habilitados</div>"""
            control_script = f"""
const token = "{context.token}";
img.addEventListener('contextmenu', e => e.preventDefault());
img.addEventListener('mousedown', (e) => {{
    const rect = img.getBoundingClientRect();
    const scaleX = (img.naturalWidth || 1920) / rect.width;
    const scaleY = (img.naturalHeight || 1080) / rect.height;
    const x = Math.round((e.clientX - rect.left) * scaleX);
    const y = Math.round((e.clientY - rect.top) * scaleY);
    fetch('/api/remote-desktop/input?token=' + encodeURIComponent(token), {{
        method: 'POST',
        headers: {{'Content-Type': 'application/json'}},
        body: JSON.stringify({{action: 'click', x: x, y: y, button: e.button}})
    }}).catch(console.error);
}});
window.addEventListener('keydown', (e) => {{
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    fetch('/api/remote-desktop/input?token=' + encodeURIComponent(token), {{
        method: 'POST',
        headers: {{'Content-Type': 'application/json'}},
        body: JSON.stringify({{action: 'key', key: e.key}})
    }}).catch(console.error);
}});
"""

        html = f"""<!DOCTYPE html>
<html>
<head>
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=3.0, user-scalable=yes">
<title>Pulsar Desktop Stream</title>
<style>
* {{ box-sizing: border-box; }}
html, body {{
    margin: 0; padding: 0; width: 100%; height: 100%; background: #000;
    display: flex; align-items: center; justify-content: center; overflow: hidden;
    user-select: none; -webkit-user-select: none;
}}
img {{
    width: 100%; height: 100%; object-fit: contain; pointer-events: auto; cursor: crosshair;
}}
</style>
</head>
<body>
{control_badge}
<img id="streamImg" src="/api/stream/pc-to-phone/live.mjpeg?token={context.token}{mon_param}" />
{audio_markup}
<script>
const img = document.getElementById('streamImg');
img.onerror = () => {{
    setTimeout(() => {{
        img.src = '/api/stream/pc-to-phone/live.mjpeg?token={context.token}{mon_param}&t=' + Date.now();
    }}, 1500);
}};
{control_script}
</script>
</body>
</html>"""
        payload = html.encode("utf-8")
        self.send_response(200)
        self.send_cors_headers()
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def _send_desktop_notification(self, app_name: str, title: str, text: str):
        logger.info(f"Relaying notification from {app_name}: {title} - {text}")
        try:
            # Try notify-send with custom app-name and category
            subprocess.Popen([
                "notify-send",
                "-a", f"Pulsar Phone ({app_name})",
                "-i", "phone",
                title,
                text
            ])
        except Exception as e:
            logger.error(f"Error invoking notify-send: {e}")

    def _set_pc_clipboard(self, text: str):
        try:
            if shutil.which("wl-copy"):
                proc = subprocess.Popen(["wl-copy"], stdin=subprocess.PIPE)
                proc.communicate(input=text.encode("utf-8"))
            elif shutil.which("xclip"):
                proc = subprocess.Popen(["xclip", "-selection", "clipboard"], stdin=subprocess.PIPE)
                proc.communicate(input=text.encode("utf-8"))
        except Exception as e:
            logger.error(f"Error setting clipboard: {e}")

    def _get_pc_clipboard(self) -> str:
        try:
            if shutil.which("wl-paste"):
                res = subprocess.run(["wl-paste"], stdout=subprocess.PIPE, text=True, timeout=2)
                return res.stdout
            elif shutil.which("xclip"):
                res = subprocess.run(["xclip", "-selection", "clipboard", "-o"], stdout=subprocess.PIPE, text=True, timeout=2)
                return res.stdout
        except Exception:
            pass
        return ""

    def _is_path_safe(self, path: str) -> bool:
        real = os.path.realpath(path)
        home = os.path.realpath(os.path.expanduser("~"))
        # Allow home directory access and media mounts (/media, /mnt)
        return real.startswith(home) or real.startswith("/media") or real.startswith("/mnt")

    def _get_fs_roots(self) -> List[Dict[str, Any]]:
        home = os.path.expanduser("~")
        roots = [
            {"id": "root_home", "name": "Inicio (Home)", "path": home, "icon": "user-home"},
            {"id": "root_downloads", "name": "Descargas", "path": os.path.join(home, "Descargas"), "icon": "folder-download"},
            {"id": "root_documents", "name": "Documentos", "path": os.path.join(home, "Documentos"), "icon": "folder-documents"},
            {"id": "root_pictures", "name": "Imágenes", "path": os.path.join(home, "Imágenes"), "icon": "folder-pictures"}
        ]
        # Filter existing paths
        valid_roots = []
        for r in roots:
            if os.path.exists(r["path"]):
                valid_roots.append(r)
        return valid_roots

    def _list_fs_dir(self, dir_path: str) -> List[Dict[str, Any]]:
        if not self._is_path_safe(dir_path) or not os.path.isdir(dir_path):
            return []

        results = []
        try:
            for entry in os.scandir(dir_path):
                # Skip hidden files
                if entry.name.startswith("."):
                    continue
                try:
                    stat = entry.stat()
                    is_dir = entry.is_dir()
                    mime, _ = mimetypes.guess_type(entry.path) if not is_dir else (None, None)
                    results.append({
                        "name": entry.name,
                        "path": entry.path,
                        "isDir": is_dir,
                        "size": stat.st_size if not is_dir else 0,
                        "modified": int(stat.st_mtime * 1000),
                        "mimeType": "vnd.android.document/directory" if is_dir else (mime or "application/octet-stream")
                    })
                except Exception:
                    continue
        except Exception as e:
            logger.error(f"Error scanning dir {dir_path}: {e}")

        results.sort(key=lambda x: (not x["isDir"], x["name"].lower()))
        return results


class UDPDiscoveryResponder(threading.Thread):
    """Answers UDP broadcast discovery beacons from Pulsar Phone on port 8989."""
    def __init__(self, port: int = DEFAULT_PORT):
        super().__init__(daemon=True, name="UDPDiscoveryResponder")
        self.port = port
        self.running = True

    def run(self):
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            try:
                sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
            except Exception:
                pass
            sock.bind(("0.0.0.0", self.port))
            logger.info(f"UDP Discovery responder active on port {self.port}")
        except Exception as e:
            logger.error(f"Failed to start UDP Discovery responder: {e}")
            return

        while self.running:
            try:
                data, addr = sock.recvfrom(2048)
                if not data:
                    continue
                msg = data.decode("utf-8", errors="ignore").strip()
                if msg.startswith("PULSAR_DISCOVER"):
                    context.refresh_ip()
                    reply = {
                        "service": "PulsarBridge",
                        "hostname": socket.gethostname(),
                        "lanIp": context.lan_ip,
                        "tailscaleIp": context.tailscale_ip,
                        "preferredIp": context.get_preferred_ip(),
                        "port": context.port,
                        "token": context.token,
                        "broadwayPort": context.broadway_port
                    }
                    sock.sendto(json.dumps(reply).encode("utf-8"), addr)
                    logger.info(f"Answered UDP discovery beacon from {addr}")
            except Exception:
                pass


class PhoneAppSyncWatcher(threading.Thread):
    """Watches for connected ADB devices and automatically synchronizes phone apps to desktop menu."""
    def __init__(self):
        super().__init__(daemon=True, name="PhoneAppSyncWatcher")
        self.last_synced_serial: Optional[str] = None
        self.last_sync_time: float = 0.0

    def run(self):
        time.sleep(3)
        while True:
            try:
                status = context.scrcpy_mgr.get_adb_status()
                devs = [d["serial"] for d in status.get("devices", []) if d.get("state") == "device"]
                if devs:
                    current_serial = devs[0]
                    now = time.time()
                    if current_serial != self.last_synced_serial or (now - self.last_sync_time > 900):
                        logger.info(f"Auto-syncing phone apps for device: {current_serial}")
                        self.last_synced_serial = current_serial
                        self.last_sync_time = now
                        try:
                            import app_sync
                            app_sync.sync_phone_apps(serial=current_serial)
                        except Exception as e:
                            logger.error(f"Error during auto-sync: {e}")
                else:
                    self.last_synced_serial = None
            except Exception as e:
                logger.debug(f"Watcher loop error: {e}")
            time.sleep(6)


class ThreadedTCPBridgeServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True


def run_bridge_server():
    context.refresh_ip()
    port = context.port

    # Start UDP discovery beacon responder
    udp_responder = UDPDiscoveryResponder(port)
    udp_responder.start()

    # Start Phone App auto-sync watcher
    app_sync_watcher = PhoneAppSyncWatcher()
    app_sync_watcher.start()

    server_address = ("0.0.0.0", port)
    logger.info(f"Starting Pulsar Bridge on 0.0.0.0:{port} (Auth Token: {context.token[:6]}...)")

    httpd = ThreadedTCPBridgeServer(server_address, BridgeRequestHandler)

    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        logger.info("Stopping Pulsar Bridge")
        context.launcher.close_current_app()
        httpd.server_close()


if __name__ == "__main__":
    run_bridge_server()
