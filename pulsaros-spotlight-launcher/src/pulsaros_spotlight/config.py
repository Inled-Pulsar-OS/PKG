"""Configuration management for PulsarOS Spotlight."""

from __future__ import annotations

import json
from pathlib import Path

from gi.repository import GLib

CONFIG_DIR = Path(GLib.get_user_config_dir()) / "pulsaros-spotlight"
CONFIG_FILE = CONFIG_DIR / "config.json"

_DEFAULTS = {
    "is_grid_view": False,
    "clipboard_max_items": 50,
    "clipboard_auto_paste": True,
}


class SpotlightConfig:
    """Persistent app configuration backed by a JSON file."""

    def __init__(self) -> None:
        self.is_grid_view: bool = _DEFAULTS["is_grid_view"]
        self.clipboard_max_items: int = _DEFAULTS["clipboard_max_items"]
        self.clipboard_auto_paste: bool = _DEFAULTS["clipboard_auto_paste"]
        self._load()

    def _load(self) -> None:
        if CONFIG_FILE.exists():
            try:
                data = json.loads(CONFIG_FILE.read_text())
                self.is_grid_view = data.get("is_grid_view", _DEFAULTS["is_grid_view"])
                self.clipboard_max_items = data.get("clipboard_max_items", _DEFAULTS["clipboard_max_items"])
                self.clipboard_auto_paste = data.get("clipboard_auto_paste", _DEFAULTS["clipboard_auto_paste"])
            except (json.JSONDecodeError, OSError):
                pass

    def save(self) -> None:
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        try:
            payload = {
                "is_grid_view": self.is_grid_view,
                "clipboard_max_items": self.clipboard_max_items,
                "clipboard_auto_paste": self.clipboard_auto_paste,
            }
            CONFIG_FILE.write_text(json.dumps(payload, indent=2))
        except OSError:
            pass


Config = SpotlightConfig
