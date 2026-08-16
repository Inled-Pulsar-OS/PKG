"""Configuration management for PulsarOS Spotlight."""

from __future__ import annotations

import json
from pathlib import Path

from gi.repository import GLib

CONFIG_DIR = Path(GLib.get_user_config_dir()) / "pulsaros-spotlight"
CONFIG_FILE = CONFIG_DIR / "config.json"

_DEFAULTS = {
    "is_grid_view": False,
}


class Config:
    """Persistent app configuration backed by a JSON file."""

    def __init__(self) -> None:
        self.is_grid_view: bool = _DEFAULTS["is_grid_view"]
        self._load()

    def _load(self) -> None:
        if CONFIG_FILE.exists():
            try:
                data = json.loads(CONFIG_FILE.read_text())
                self.is_grid_view = data.get("is_grid_view", _DEFAULTS["is_grid_view"])
            except (json.JSONDecodeError, OSError):
                pass

    def save(self) -> None:
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        try:
            CONFIG_FILE.write_text(json.dumps({"is_grid_view": self.is_grid_view}))
        except OSError:
            pass
