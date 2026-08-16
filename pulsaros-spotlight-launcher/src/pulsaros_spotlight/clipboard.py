"""Clipboard history manager and auto-paste provider for PulsarOS Spotlight."""

from __future__ import annotations

import json
import logging
import os
import shutil
import subprocess
import time
from pathlib import Path
from typing import TYPE_CHECKING

import gi

gi.require_version("Gdk", "4.0")
gi.require_version("Gtk", "4.0")
from gi.repository import Gdk, GLib, Gtk

from pulsaros_spotlight.config import Config

if TYPE_CHECKING:
    from pulsaros_spotlight.search import SearchResult

logger = logging.getLogger(__name__)

DATA_DIR = Path(GLib.get_user_data_dir()) / "pulsaros-spotlight"
CLIPBOARD_FILE = DATA_DIR / "clipboard_history.json"


class ClipboardManager:
    """Monitors clipboard changes, stores history, and simulates auto-pasting."""

    def __init__(self, config: Config | None = None) -> None:
        self._config = config or Config()
        self._items: list[dict] = []
        self._load()
        self._init_listener()

    def _load(self) -> None:
        if CLIPBOARD_FILE.exists():
            try:
                self._items = json.loads(CLIPBOARD_FILE.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError):
                self._items = []

    def _save(self) -> None:
        DATA_DIR.mkdir(parents=True, exist_ok=True)
        try:
            CLIPBOARD_FILE.write_text(json.dumps(self._items, indent=2), encoding="utf-8")
        except OSError:
            pass

    def _init_listener(self) -> None:
        try:
            display = Gdk.Display.get_default()
            if display:
                clipboard = display.get_clipboard()
                clipboard.connect("changed", self._on_clipboard_changed)
        except Exception:
            logger.warning("Failed to hook Gdk.Clipboard listener", exc_info=True)

    def _on_clipboard_changed(self, clipboard: Gdk.Clipboard) -> None:
        try:
            clipboard.read_text_async(None, self._on_text_read)
        except Exception:
            pass

    def _on_text_read(self, clipboard: Gdk.Clipboard, result) -> None:
        try:
            text = clipboard.read_text_finish(result)
            if text and text.strip():
                self.record_clip(text.strip())
        except Exception:
            pass

    def record_clip(self, text: str) -> None:
        """Add a text snippet to clipboard history."""
        if not text:
            return
        clean = text.strip()
        if not clean:
            return

        # Avoid duplicate at top
        if self._items and self._items[0].get("text") == clean:
            return

        # Remove previous occurrence if exists
        self._items = [item for item in self._items if item.get("text") != clean]

        # Insert at front
        self._items.insert(
            0,
            {
                "text": clean,
                "timestamp": time.time(),
            },
        )

        # Enforce max items
        max_items = getattr(self._config, "clipboard_max_items", 50)
        self._items = self._items[:max_items]
        self._save()

    def search(self, query: str = "") -> list[SearchResult]:
        """Search clipboard history entries."""
        from pulsaros_spotlight.search import SearchResult

        # Also check current clipboard content on search
        try:
            p = subprocess.run(["wl-paste", "--no-newline"], capture_output=True, text=True, timeout=0.2)
            if p.returncode == 0 and p.stdout and p.stdout.strip():
                self.record_clip(p.stdout.strip())
        except Exception:
            pass

        results: list[SearchResult] = []
        q_lower = query.lower().strip()

        for idx, item in enumerate(self._items):
            text = item.get("text", "")
            if not text:
                continue

            if not q_lower or q_lower in text.lower():
                lines = [line.strip() for line in text.splitlines() if line.strip()]
                first_line = lines[0] if lines else text
                if len(first_line) > 60:
                    first_line = first_line[:57] + "..."

                preview = text.replace("\n", " ⏎ ")
                if len(preview) > 100:
                    preview = preview[:97] + "..."

                results.append(
                    SearchResult(
                        url=f"clipboard://{idx}",
                        title=first_line,
                        mime="text/plain-clipboard",
                        snippet=preview,
                        app=None,
                    )
                )

        return results

    def get_clip_by_index(self, index: int) -> str | None:
        if 0 <= index < len(self._items):
            return self._items[index].get("text")
        return None

    def paste_clip(self, text: str) -> None:
        """Set clipboard text and simulate Ctrl+V keystroke into previous window."""
        if not text:
            return

        # 1. Set to Gdk Clipboard and wl-copy
        try:
            display = Gdk.Display.get_default()
            if display:
                display.get_clipboard().set(text)
        except Exception:
            pass

        try:
            p = subprocess.Popen(["wl-copy"], stdin=subprocess.PIPE)
            p.communicate(text.encode("utf-8"))
        except Exception:
            pass

        # 2. Simulate Paste (Ctrl+V) after window hides and focus returns
        if getattr(self._config, "clipboard_auto_paste", True):
            GLib.timeout_add(100, self._simulate_paste)

    @staticmethod
    def _simulate_paste() -> bool:
        """Simulate Ctrl+V keystroke via ydotool, GNOME Shell extension D-Bus, or xdotool."""
        # 1. Native Wayland uinput via ydotool
        if shutil.which("ydotool"):
            try:
                env = os.environ.copy()
                sock = f"/run/user/{os.getuid()}/.ydotool_socket"
                if os.path.exists(sock):
                    env["YDOTOOL_SOCKET"] = sock
                subprocess.Popen(["ydotool", "key", "29:1", "47:1", "47:0", "29:0"], env=env)
                return False
            except Exception:
                pass

        # 2. Native Wayland GNOME Shell Extension D-Bus key injection
        try:
            bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            proxy = Gio.DBusProxy.new_sync(
                bus,
                Gio.DBusProxyFlags.DO_NOT_LOAD_PROPERTIES,
                None,
                "org.gnome.Shell.Extensions.PulsarSpotlight",
                "/org/gnome/Shell/Extensions/PulsarSpotlight",
                "org.gnome.Shell.Extensions.PulsarSpotlight",
                None,
            )
            proxy.call_sync("Paste", None, Gio.DBusCallFlags.NONE, 500, None)
            return False
        except Exception:
            pass

        # 3. External tool fallback
        if shutil.which("xdotool"):
            try:
                subprocess.Popen(["xdotool", "key", "ctrl+v"])
                return False
            except Exception:
                pass

        return False
