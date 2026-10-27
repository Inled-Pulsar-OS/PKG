#!/usr/bin/env python3
"""
Application Launcher & Broadway Manager for Pulsar OS
Handles .desktop discovery, icon resolution, and isolated single-instance Broadway streaming.
"""

import base64
import configparser
import glob
import logging
import mimetypes
import os
import re
import shlex
import shutil
import signal
import socket
import subprocess
import time
from typing import Dict, Any, List, Optional

logger = logging.getLogger("PulsarVPN.AppLauncher")

BROADWAY_DISPLAY_NUM = 5
BROADWAY_DEFAULT_PORT = 8085


class AppLauncher:
    def __init__(self, bind_ip: str = "127.0.0.1", port: int = BROADWAY_DEFAULT_PORT):
        self.bind_ip = bind_ip
        self.port = port
        self.display_num = BROADWAY_DISPLAY_NUM
        self.current_app_process: Optional[subprocess.Popen] = None
        self.broadwayd_process: Optional[subprocess.Popen] = None
        self.current_app_id: Optional[str] = None
        self.current_app_name: Optional[str] = None
        self.lock_file = os.path.expanduser("~/.config/pulsaros/active_broadway.pid")
        os.makedirs(os.path.dirname(self.lock_file), exist_ok=True)

    def set_bind_ip(self, ip: str):
        if ip:
            self.bind_ip = ip

    def list_apps(self) -> List[Dict[str, Any]]:
        """
        Scans standard application directories for launchable GUI apps.
        """
        search_dirs = [
            "/usr/share/applications",
            "/usr/local/share/applications",
            os.path.expanduser("~/.local/share/applications")
        ]
        
        apps = []
        seen_ids = set()

        for s_dir in search_dirs:
            if not os.path.exists(s_dir):
                continue
            for fpath in glob.glob(os.path.join(s_dir, "*.desktop")):
                app_id = os.path.basename(fpath)
                if app_id in seen_ids:
                    continue
                
                try:
                    meta = self._parse_desktop_file(fpath)
                    if meta and not meta.get("nodisplay", False) and not meta.get("terminal", False):
                        apps.append(meta)
                        seen_ids.add(app_id)
                except Exception as e:
                    logger.debug(f"Error parsing {fpath}: {e}")

        # Sort alphabetically by name
        apps.sort(key=lambda x: x.get("name", "").lower())
        return apps

    def _parse_desktop_file(self, filepath: str) -> Optional[Dict[str, Any]]:
        app_id = os.path.basename(filepath)
        config = configparser.ConfigParser(interpolation=None, strict=False)
        try:
            with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
                config.read_file(f)
            if "Desktop Entry" not in config:
                return None
            entry = config["Desktop Entry"]
            
            if entry.get("Type", "").strip() != "Application":
                return None

            name = entry.get("Name", "").strip()
            exec_cmd = entry.get("Exec", "").strip()
            if not name or not exec_cmd:
                return None

            nodisplay = entry.getboolean("NoDisplay", fallback=False)
            terminal = entry.getboolean("Terminal", fallback=False)
            icon_name = entry.get("Icon", "").strip()
            comment = entry.get("Comment", "").strip()
            categories = [c.strip() for c in entry.get("Categories", "").split(";") if c.strip()]

            # Clean Exec command field codes (%u, %f, etc.)
            clean_exec = re.sub(r'%[a-zA-Z]', '', exec_cmd).strip()

            # Ensure the executable actually exists on disk so we never list ghost/broken apps
            tokens = shlex.split(clean_exec)
            if not tokens:
                return None
            exe_bin = tokens[0]
            exe_name = os.path.basename(exe_bin)

            # Exclude non-Broadway apps and integrated Android apps (DroidTux / Pulsar Phone)
            # to guarantee that every listed app renders in Broadway and NEVER on the PC screen,
            # and that Android apps from the phone are NOT mirrored back into the phone app list
            if (app_id.startswith("droidtux-") or
                app_id.startswith("dtapp-") or
                app_id.startswith("pulsar-phone-") or
                "X-Android" in categories or
                "X-PulsarPhone" in categories or
                "pulsar-phone-runner" in clean_exec or
                "droidtux" in clean_exec or
                "scrcpy" in clean_exec):
                return None

            non_broadway_bins = {
                "google-chrome", "google-chrome-stable", "chromium", "brave", "firefox",
                "code", "code-oss", "electron", "kdenlive", "scrcpy", "gnome-terminal",
                "xdg-open", "cups", "qv4l2", "qvidcap", "lstopo", "virt-manager",
                "install-macos", "bssh", "bvnc", "droidtux-runner"
            }
            if exe_name in non_broadway_bins or any("gnome-terminal" in t or "xdg-open" in t for t in tokens):
                return None

            if os.path.isabs(exe_bin):
                if not os.path.isfile(exe_bin):
                    return None
            else:
                if not shutil.which(exe_bin):
                    return None

            icon_data = self._resolve_icon_base64(icon_name)
            # If no icon found, generate a clean default app icon so fallback is never used
            if not icon_data:
                icon_data = self._resolve_icon_base64("application-x-executable") or self._resolve_icon_base64("system-run")

            return {
                "id": os.path.basename(filepath),
                "name": name,
                "comment": comment,
                "exec": clean_exec,
                "icon": icon_name,
                "iconData": icon_data,
                "categories": categories,
                "nodisplay": nodisplay,
                "terminal": terminal
            }
        except Exception as e:
            logger.warning(f"Error parsing desktop file {filepath}: {e}")
            return None

    def _resolve_icon_base64(self, icon_name: str) -> Optional[str]:
        if not icon_name:
            return None

        fpath = None
        # 1. Direct file path
        if os.path.isabs(icon_name) and os.path.isfile(icon_name):
            fpath = icon_name

        # 2. Search dirs for icon name
        if not fpath:
            icon_extensions = [".svg", ".png", ".webp", ".xpm"]
            base_dirs = [
                "/usr/share/icons/hicolor/scalable/apps",
                "/usr/share/icons/hicolor/128x128/apps",
                "/usr/share/icons/hicolor/64x64/apps",
                "/usr/share/icons/hicolor/48x48/apps",
                "/usr/share/icons/Adwaita/scalable/apps",
                "/usr/share/pixmaps",
                os.path.expanduser("~/.local/share/icons")
            ]
            for b_dir in base_dirs:
                if not os.path.exists(b_dir):
                    continue
                for ext in icon_extensions:
                    cand = os.path.join(b_dir, icon_name + ext)
                    if os.path.isfile(cand):
                        fpath = cand
                        break
                if fpath:
                    break
                cand_exact = os.path.join(b_dir, icon_name)
                if os.path.isfile(cand_exact):
                    fpath = cand_exact
                    break

        if not fpath:
            return None

        # If already PNG, return directly
        if fpath.endswith(".png"):
            return self._file_to_base64(fpath)

        # For SVG or other formats, convert safely to PNG using rsvg-convert (immune to in-process crashes)
        if shutil.which("rsvg-convert"):
            try:
                res = subprocess.run(
                    ["rsvg-convert", "-w", "128", "-h", "128", "-f", "png", fpath],
                    capture_output=True,
                    timeout=2
                )
                if res.returncode == 0 and res.stdout:
                    b64 = base64.b64encode(res.stdout).decode("utf-8")
                    return f"data:image/png;base64,{b64}"
            except Exception:
                pass

        return self._file_to_base64(fpath)

    def _file_to_base64(self, path: str) -> Optional[str]:
        try:
            mime, _ = mimetypes.guess_type(path)
            if not mime:
                mime = "image/svg+xml" if path.endswith(".svg") else "image/png"
            with open(path, "rb") as f:
                data = f.read()
            b64 = base64.b64encode(data).decode("utf-8")
            return f"data:{mime};base64,{b64}"
        except Exception as e:
            logger.warning(f"Error parsing desktop file {filepath}: {e}")
            return None

    def stop_broadwayd(self):
        if self.broadwayd_process:
            try:
                self.broadwayd_process.kill()
                self.broadwayd_process.wait(timeout=0.5)
            except Exception:
                pass
            self.broadwayd_process = None

        try:
            subprocess.run(["pkill", "-9", "-f", f"gtk4-broadwayd :{self.display_num}"], timeout=1)
            subprocess.run(["pkill", "-9", "-f", f"broadwayd :{self.display_num}"], timeout=1)
        except Exception:
            pass

    def _start_broadwayd(self):
        self.stop_broadwayd()
        time.sleep(0.1)

        broadway_bin = shutil.which("gtk4-broadwayd") or shutil.which("broadwayd") or "broadwayd"
        broadway_cmd = [
            broadway_bin,
            f":{self.display_num}",
            "--port", str(self.port),
            "--address", self.bind_ip
        ]
        logger.info(f"Starting fresh {broadway_bin}: {' '.join(broadway_cmd)}")
        self.broadwayd_process = subprocess.Popen(
            broadway_cmd,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )

        for _ in range(25):
            time.sleep(0.05)
            try:
                with socket.create_connection(("127.0.0.1", self.port), timeout=0.1):
                    logger.info("gtk4-broadwayd is ready and listening on port %d", self.port)
                    return
            except Exception:
                pass

    def _ensure_broadwayd(self):
        self._start_broadwayd()

    def close_current_app(self):
        """
        Closes current active app and resets broadwayd to ensure no windows persist on canvas.
        """
        if self.current_app_process:
            try:
                logger.info(f"Terminating active app {self.current_app_name} (PID: {self.current_app_process.pid})")
                try:
                    os.killpg(os.getpgid(self.current_app_process.pid), signal.SIGKILL)
                except Exception:
                    self.current_app_process.kill()
            except Exception:
                pass
            self.current_app_process = None

        try:
            subprocess.run(["pkill", "-9", "-f", f"BROADWAY_DISPLAY=:{self.display_num}"], timeout=1)
        except Exception:
            pass

        self.stop_broadwayd()

        self.current_app_id = None
        self.current_app_name = None

    def launch_app(self, app_id: str, client_host: Optional[str] = None, dark_mode: bool = True) -> Dict[str, Any]:
        """
        Enforces single active app: Closes previous app, restarts broadwayd clean, and spawns target app.
        """
        # 1. Close previous app and wipe broadwayd canvas
        self.close_current_app()

        # 2. Locate app definition
        apps = self.list_apps()
        app_meta = next((a for a in apps if a["id"] == app_id), None)
        if not app_meta:
            raise ValueError(f"Application {app_id} not found")

        # 3. Start a brand new broadwayd instance with an empty canvas
        self._start_broadwayd()

        # 4. Launch the application with GDK_BACKEND=broadway
        env = os.environ.copy()
        # CRITICAL: Isolate completely from desktop Wayland/X11 displays and user D-Bus session.
        # This prevents single-instance GTK apps (Nautilus, Calendar, etc.) from forwarding
        # activation to desktop processes or opening windows on the PC screen!
        env.pop("WAYLAND_DISPLAY", None)
        env.pop("DISPLAY", None)
        env.pop("DBUS_SESSION_BUS_ADDRESS", None)
        env["QT_QPA_PLATFORM"] = "offscreen"

        env["GDK_BACKEND"] = "broadway"
        env["BROADWAY_DISPLAY"] = f":{self.display_num}"
        env["GDK_CORE_DEVICE_EVENTS"] = "1"
        if dark_mode:
            env["ADW_COLOR_SCHEME"] = "prefer-dark"
            env["GTK_THEME"] = "Adwaita:dark"
        else:
            env["ADW_COLOR_SCHEME"] = "prefer-light"
            env["GTK_THEME"] = "Adwaita"

        raw_tokens = shlex.split(app_meta["exec"])
        if shutil.which("dbus-run-session"):
            exec_cmd = ["dbus-run-session", "--"] + raw_tokens
        else:
            exec_cmd = raw_tokens

        logger.info(f"Launching app in Broadway: {exec_cmd}")
        self.current_app_process = subprocess.Popen(
            exec_cmd,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True
        )

        self.current_app_id = app_meta["id"]
        self.current_app_name = app_meta["name"]

        url_host = client_host or self.bind_ip
        if url_host in ("0.0.0.0", "127.0.0.1", "", None):
            try:
                with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
                    s.connect(("8.8.8.8", 80))
                    url_host = s.getsockname()[0]
            except Exception:
                url_host = "127.0.0.1"

        return {
            "status": "launched",
            "appId": self.current_app_id,
            "appName": self.current_app_name,
            "broadwayUrl": f"http://{url_host}:{self.port}/",
            "port": self.port
        }

    def get_active_state(self) -> Dict[str, Any]:
        running = False
        if self.current_app_process and self.current_app_process.poll() is None:
            running = True
        elif self.current_app_process:
            self.current_app_process = None
            self.current_app_id = None
            self.current_app_name = None

        return {
            "running": running,
            "appId": self.current_app_id,
            "appName": self.current_app_name,
            "broadwayUrl": f"http://{self.bind_ip}:{self.port}/" if running else None
        }
