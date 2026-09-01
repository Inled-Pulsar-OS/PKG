#!/usr/bin/env python3
# ==============================================================================
# Pulsar OS - Welcome Application (Python + WebKitGTK Exact React Port)
# ==============================================================================

import os
import re
import sys
import json
import shutil
import subprocess
import cairo

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
gi.require_version("GdkPixbuf", "2.0")

# Support both WebKit2 4.1 and 6.0
try:
    gi.require_version("WebKit2", "4.1")
    from gi.repository import WebKit2
except (ValueError, ImportError):
    try:
        gi.require_version("WebKit2", "6.0")
        from gi.repository import WebKit2
    except (ValueError, ImportError):
        gi.require_version("WebKit2", "4.0")
        from gi.repository import WebKit2

from gi.repository import Gtk, Gdk, GLib, Gio


def is_live_system():
    """Detects if we are running in the Live ISO environment."""
    if os.path.exists("/lib/live/mount") or os.path.exists("/run/archiso/bootmnt") or os.path.exists("/run/live/medium"):
        return True
    if os.environ.get("USER") == "live":
        return True
    try:
        with open("/proc/cmdline", "r") as f:
            cmd = f.read()
            if "boot=live" in cmd or "archisobasedir=live" in cmd or "rootfstype=9p" in cmd:
                return True
    except Exception:
        pass
    return False


def is_arch_system():
    """Detects if running on Arch Linux."""
    try:
        with open("/etc/os-release", "r") as f:
            rel = f.read().lower()
        if "id_like=arch" in rel or "id=arch" in rel:
            return True
        if "id_like=debian" in rel or "id=debian" in rel or "id=ubuntu" in rel:
            return False
    except Exception:
        pass
    return shutil.which("pacman") is not None


def is_ootb_pending():
    """Detects if initial user account setup is required."""
    return os.path.exists("/etc/pulsar-need-setup")


def check_sentinel():
    """Returns True if the user has already completed the welcome onboarding."""
    done_file = os.path.expanduser("~/.config/pulsaros-welcome.done")
    return os.path.isfile(done_file)


def write_sentinel():
    """Marks welcome assistant as completed."""
    done_file = os.path.expanduser("~/.config/pulsaros-welcome.done")
    os.makedirs(os.path.dirname(done_file), exist_ok=True)
    try:
        with open(done_file, "w") as f:
            f.write("done\n")
    except Exception as e:
        print(f"Error writing sentinel: {e}")


def launch_with_fallback(primary, fallback=None):
    candidates = [primary]
    if fallback:
        candidates.append(fallback)
    for cmd in candidates:
        if "/" in cmd:
            if os.path.exists(cmd) and os.access(cmd, os.X_OK):
                subprocess.Popen([cmd])
                return True
        elif shutil.which(cmd):
            subprocess.Popen([cmd])
            return True
    return False


def get_system_resolutions():
    resolutions = []
    try:
        out = subprocess.check_output(["xrandr"], universal_newlines=True)
        for line in out.splitlines():
            m = re.search(r"^\s*(\d+)x(\d+)\s+.*(\*|\+)", line)
            if m:
                w, h = int(m.group(1)), int(m.group(2))
                active = "*" in line
                resolutions.append({"width": w, "height": h, "active": active})
    except Exception:
        resolutions = [{"width": 1920, "height": 1080, "active": True}]
    return resolutions


class WelcomeApp(Gtk.Window):
    def __init__(self, is_live=False, is_arch=False, needs_ootb=False):
        super().__init__(type=Gtk.WindowType.TOPLEVEL)
        self.is_live = is_live
        self.is_arch = is_arch
        self.needs_ootb = needs_ootb

        self.set_title("Pulsar OS Welcome")
        self.set_decorated(False)
        self.maximize()

        # Enable true transparent alpha channel for GNOME blur-my-shell
        screen = self.get_screen()
        visual = screen.get_rgba_visual()
        if visual and screen.is_composited():
            self.set_visual(visual)

        self.set_app_paintable(True)
        self.connect("draw", self._on_draw)
        self.connect("destroy", Gtk.main_quit)

        # Set up WebKit with bridge
        ucm = WebKit2.UserContentManager()
        ucm.register_script_message_handler("welcome")
        ucm.connect("script-message-received::welcome", self._on_script_message)

        # Inject initial runtime state
        init_script = WebKit2.UserScript(
            f"""
                window.IS_LIVE = {'true' if is_live else 'false'};
                window.IS_ARCH = {'true' if is_arch else 'false'};
                window.IS_OOTB = {'true' if needs_ootb else 'false'};
            """,
            WebKit2.UserContentInjectedFrames.ALL_FRAMES,
            WebKit2.UserScriptInjectionTime.START,
            None,
            None
        )
        ucm.add_script(init_script)

        self.webview = WebKit2.WebView.new_with_user_content_manager(ucm)
        bg_color = Gdk.RGBA()
        bg_color.parse("rgba(0, 0, 0, 0)")
        self.webview.set_background_color(bg_color)

        settings = self.webview.get_settings()
        settings.set_allow_file_access_from_file_urls(True)
        settings.set_allow_universal_access_from_file_urls(True)
        settings.set_enable_write_console_messages_to_stdout(True)
        settings.set_enable_developer_extras(True)

        self.add(self.webview)

        # Load exact compiled React/Tailwind frontend from dist/index.html (or fallback to ui/index.html)
        base_dir = os.path.dirname(os.path.abspath(__file__))
        dist_html = os.path.join(base_dir, "dist", "index.html")
        ui_html = os.path.join(base_dir, "ui", "index.html")
        
        target_html = dist_html if os.path.exists(dist_html) else ui_html
        self.webview.load_uri(f"file://{target_html}")

    def _on_draw(self, widget, cr):
        cr.set_source_rgba(0, 0, 0, 0)
        cr.set_operator(cairo.OPERATOR_SOURCE)
        cr.paint()
        return False

    def _send_response(self, req_id, result):
        if not req_id:
            return
        res_json = json.dumps(result)
        js = f"if (window.__handlePyResponse) {{ window.__handlePyResponse('{req_id}', {res_json}); }}"
        self.webview.run_javascript(js, None, None, None)

    def _on_script_message(self, content_manager, js_result):
        try:
            if hasattr(js_result, "get_js_value"):
                js_val = js_result.get_js_value()
                msg_str = js_val.to_string()
            elif hasattr(js_result, "to_string"):
                msg_str = js_result.to_string()
            else:
                msg_str = str(js_result)
            data = json.loads(msg_str)
        except Exception as e:
            print(f"Error parsing script message: {e}")
            return

        cmd = data.get("cmd") or data.get("action")
        args = data.get("args") or {}
        req_id = data.get("id")

        if cmd == "is_live_system":
            self._send_response(req_id, self.is_live)

        elif cmd == "is_arch_system":
            self._send_response(req_id, self.is_arch)

        elif cmd == "is_ootb_pending":
            self._send_response(req_id, self.needs_ootb)

        elif cmd == "check_sentinel":
            self._send_response(req_id, check_sentinel())

        elif cmd == "write_sentinel":
            write_sentinel()
            self._send_response(req_id, None)

        elif cmd == "get_system_mode":
            self._send_response(req_id, "Live" if self.is_live else "Normal")

        elif cmd == "get_resolutions":
            self._send_response(req_id, get_system_resolutions())

        elif cmd == "set_resolution":
            w = args.get("width")
            h = args.get("height")
            if w and h:
                subprocess.Popen(["xrandr", "-s", f"{w}x{h}"])
            self._send_response(req_id, None)

        elif cmd == "get_effects_state":
            self._send_response(req_id, True)

        elif cmd == "set_effects":
            self._send_response(req_id, None)

        elif cmd == "check_adb_devices":
            try:
                out = subprocess.check_output(["adb", "devices"], universal_newlines=True)
            except Exception:
                out = ""
            self._send_response(req_id, out)

        elif cmd == "launch_ootb":
            self.hide()
            candidates = ["/usr/bin/pulsaros-ootb", "/usr/share/pulsaros/welcome_ootb.py"]
            for cand in candidates:
                if os.path.exists(cand):
                    if cand.endswith(".py"):
                        subprocess.Popen(["sudo", "-E", "/usr/bin/python3", cand])
                    else:
                        subprocess.Popen([cand])
                    break
            self._send_response(req_id, None)
            Gtk.main_quit()

        elif cmd == "launch_recovery":
            self.hide()
            launch_with_fallback("pulsaros-recovery", "pulsaros-recovery-window")
            self._send_response(req_id, None)
            Gtk.main_quit()

        elif cmd == "launch_wifi_settings" or cmd == "launch_wifi":
            if not launch_with_fallback("gnome-control-center", None):
                launch_with_fallback("nm-connection-editor")
            else:
                subprocess.Popen(["gnome-control-center", "wifi"])
            self._send_response(req_id, None)

        elif cmd == "launch_bluetooth_settings" or cmd == "launch_bluetooth":
            if not launch_with_fallback("gnome-control-center", None):
                launch_with_fallback("blueman-manager")
            else:
                subprocess.Popen(["gnome-control-center", "bluetooth"])
            self._send_response(req_id, None)

        elif cmd == "launch_display_settings" or cmd == "launch_display":
            subprocess.Popen(["gnome-control-center", "display"])
            self._send_response(req_id, None)

        elif cmd == "launch_appearance_settings" or cmd == "launch_appearance":
            subprocess.Popen(["gnome-control-center", "background"])
            self._send_response(req_id, None)

        elif cmd == "launch_app":
            app = args.get("app") or data.get("app")
            fallback = args.get("fallback") or data.get("fallback")
            if app:
                launch_with_fallback(app, fallback)
            self._send_response(req_id, None)

        elif cmd == "open_url":
            url = args.get("url") or data.get("url")
            if url:
                try:
                    Gio.AppInfo.launch_default_for_uri(url, None)
                except Exception:
                    subprocess.Popen(["xdg-open", url])
            self._send_response(req_id, None)

        elif cmd == "close" or cmd == "close_window":
            self.destroy()
            Gtk.main_quit()


def main():
    force = any(arg in sys.argv for arg in ["--force", "-f"])
    is_live = is_live_system()
    is_arch = is_arch_system()
    needs_ootb = is_ootb_pending()

    # If live user cleanup is pending, launch the cleanup spinner app first
    if os.path.exists("/etc/pulsar-need-cleanup"):
        if os.path.exists("/usr/bin/pulsar-cleanup-user"):
            subprocess.Popen(["/usr/bin/pulsar-cleanup-user"])
            sys.exit(0)

    # If already done and not live/ootb/forced, exit silently
    if not is_live and not needs_ootb and check_sentinel() and not force:
        sys.exit(0)

    # Permit root X11 permissions for OOTB assistant
    if needs_ootb and shutil.which("xhost"):
        subprocess.run(["xhost", "+SI:localuser:root"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    win = WelcomeApp(is_live=is_live, is_arch=is_arch, needs_ootb=needs_ootb)
    win.show_all()
    Gtk.main()


if __name__ == "__main__":
    main()
