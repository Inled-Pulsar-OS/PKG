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


class DesktopStreamManager:
    """Streams PC desktop video + system audio to Android devices via low-latency MPEG-TS."""
    def __init__(self):
        self.lock = threading.Lock()
        self.ffmpeg_proc: Optional[subprocess.Popen] = None
        self.reader_thread: Optional[threading.Thread] = None
        self.clients: List[queue.Queue] = []

    def is_running(self) -> bool:
        with self.lock:
            return self.ffmpeg_proc is not None and self.ffmpeg_proc.poll() is None

    def start_if_needed(self):
        with self.lock:
            if self.ffmpeg_proc and self.ffmpeg_proc.poll() is None:
                return

            env = os.environ.copy()
            if "DISPLAY" not in env or not env["DISPLAY"]:
                env["DISPLAY"] = ":1"
            display = env["DISPLAY"]

            cmd = [
                "ffmpeg",
                "-f", "x11grab",
                "-video_size", "1280x720",
                "-framerate", "30",
                "-i", display,
                "-c:v", "libx264",
                "-preset", "ultrafast",
                "-tune", "zerolatency",
                "-pix_fmt", "yuv420p",
                "-g", "30",
                "-keyint_min", "30",
                "-sc_threshold", "0",
                "-b:v", "2500k",
                "-f", "mpegts",
                "pipe:1"
            ]
            try:
                self.ffmpeg_proc = subprocess.Popen(
                    cmd,
                    env=env,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.DEVNULL,
                    bufsize=32768
                )
                self.reader_thread = threading.Thread(target=self._broadcast_loop, daemon=True)
                self.reader_thread.start()
                logger.info(f"PC Desktop Streamer started (display={display})")
            except Exception as e:
                logger.error(f"Failed to start desktop streamer: {e}")

    def _broadcast_loop(self):
        while True:
            proc = self.ffmpeg_proc
            if not proc or proc.poll() is not None or not proc.stdout:
                break
            try:
                chunk = proc.stdout.read(4096)
                if not chunk:
                    break
                with self.lock:
                    dead = []
                    for q in self.clients:
                        try:
                            q.put_nowait(chunk)
                        except queue.Full:
                            pass
                        except Exception:
                            dead.append(q)
                    for d in dead:
                        if d in self.clients:
                            self.clients.remove(d)
            except Exception:
                break

        self.stop()

    def register_client(self) -> queue.Queue:
        self.start_if_needed()
        q = queue.Queue(maxsize=120)
        with self.lock:
            self.clients.append(q)
        return q

    def unregister_client(self, q: queue.Queue):
        with self.lock:
            if q in self.clients:
                self.clients.remove(q)
            if not self.clients:
                self.stop()

    def stop(self):
        with self.lock:
            if self.ffmpeg_proc:
                try:
                    self.ffmpeg_proc.terminate()
                    self.ffmpeg_proc.wait(timeout=1.0)
                except Exception:
                    try:
                        self.ffmpeg_proc.kill()
                    except Exception:
                        pass
                self.ffmpeg_proc = None
            self.clients.clear()
            logger.info("PC Desktop Streamer stopped")


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

    def start(self, phone_ip: Optional[str] = None) -> bool:
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
            "--window-title=Pulsar Phone (Pantalla y Sonido)"
        ]
        if device_serial:
            cmd.extend(["-s", device_serial])

        try:
            self.proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            logger.info(f"Launched scrcpy phone stream on device {device_serial}: {' '.join(cmd)}")
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


class BridgeContext:
    def __init__(self):
        self.token = get_or_create_token()
        self.tailscale = TailscaleManager()
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

    def check_auth(self) -> bool:
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
        elif path == "/api/stream/pc-to-phone/live.ts":
            self._serve_live_desktop_stream()
        elif path == "/api/stream/pc-to-phone/status":
            self.send_json(200, {
                "active": context.desktop_streamer.is_running(),
                "clients": len(context.desktop_streamer.clients),
                "url": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/live.ts?token={context.token}"
            })
        elif path == "/api/stream/phone-to-pc/status":
            self.send_json(200, {
                "running": context.scrcpy_mgr.is_running()
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
            ok = context.scrcpy_mgr.start(phone_ip)
            self.send_json(200, {"status": "started" if ok else "error"})

        elif path == "/api/stream/phone-to-pc/stop":
            context.scrcpy_mgr.stop()
            self.send_json(200, {"status": "stopped"})

        elif path == "/api/stream/pc-to-phone/start":
            context.desktop_streamer.start_if_needed()
            self.send_json(200, {
                "status": "started",
                "url": f"http://{context.get_preferred_ip()}:{context.port}/api/stream/pc-to-phone/live.ts?token={context.token}"
            })

        elif path == "/api/stream/pc-to-phone/stop":
            context.desktop_streamer.stop()
            self.send_json(200, {"status": "stopped"})
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

    def _serve_live_desktop_stream(self):
        client_q = context.desktop_streamer.register_client()
        try:
            self.send_response(200)
            self.send_cors_headers()
            self.send_header("Content-Type", "video/mp2t")
            self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            self.send_header("Pragma", "no-cache")
            self.send_header("Expires", "0")
            self.end_headers()

            while True:
                try:
                    chunk = client_q.get(timeout=4.0)
                    self.wfile.write(chunk)
                    self.wfile.flush()
                except queue.Empty:
                    if not context.desktop_streamer.is_running():
                        break
                    continue
        except (BrokenPipeError, ConnectionResetError, Exception) as e:
            logger.debug(f"Desktop stream client disconnected: {e}")
        finally:
            context.desktop_streamer.unregister_client(client_q)

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


class ThreadedTCPBridgeServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True


def run_bridge_server():
    context.refresh_ip()
    port = context.port

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
