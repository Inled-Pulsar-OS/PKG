"""User configuration, stored as JSON in $XDG_CONFIG_HOME/pulsaros-airpods/config.json."""

import json
import logging
import os
import tempfile
from typing import Optional

log = logging.getLogger(__name__)

DEFAULTS = {
    "ear_detection": True,
    "lid_popup": True,
    "low_battery_alerts": True,
    "show_when_disconnected": False,
    "panel_battery": "off",
    "rssi_min": -80,
    "bound_address": None,
}


def default_path() -> str:
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.join(os.path.expanduser("~"), ".config")
    return os.path.join(base, "pulsaros-airpods", "config.json")


class Config:
    def __init__(self, path: Optional[str] = None):
        self.path = path or default_path()
        self._values = dict(DEFAULTS)
        self._load()

    def _load(self) -> None:
        try:
            with open(self.path, encoding="utf-8") as f:
                data = json.load(f)
        except FileNotFoundError:
            return
        except (OSError, ValueError) as e:
            log.warning("ignoring unreadable config %s: %s", self.path, e)
            return
        if not isinstance(data, dict):
            return
        for key, default in DEFAULTS.items():
            value = data.get(key, default)
            # type(True) is not int here: bool is an int subclass, so compare exact types
            # a default of None means "optional string"
            expected = str if default is None else type(default)
            if (default is None and value is None) or type(value) is expected:
                self._values[key] = value

    def get(self, key: str):
        return self._values[key]

    def set(self, key: str, value) -> None:
        if key not in DEFAULTS:
            raise KeyError(key)
        if self._values[key] == value:
            return
        self._values[key] = value
        self._save()

    def _save(self) -> None:
        directory = os.path.dirname(self.path)
        try:
            os.makedirs(directory, exist_ok=True)
            fd, tmp = tempfile.mkstemp(dir=directory, prefix=".config-")
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                json.dump(self._values, f, indent=2)
            os.replace(tmp, self.path)
        except OSError as e:
            log.warning("cannot save config %s: %s", self.path, e)
