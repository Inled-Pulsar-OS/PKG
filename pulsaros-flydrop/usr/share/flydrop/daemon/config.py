"""
FlyDrop Configuration Manager
Handles persistent settings in ~/.config/flydrop/config.json
"""

import os
import json
import socket
import uuid
import logging

logger = logging.getLogger("FlyDrop.Config")

CONFIG_DIR = os.path.expanduser("~/.config/flydrop")
CONFIG_FILE = os.path.join(CONFIG_DIR, "config.json")


def get_default_download_dir():
    spanish_downloads = os.path.expanduser("~/Descargas")
    if os.path.exists(spanish_downloads):
        return spanish_downloads
    english_downloads = os.path.expanduser("~/Downloads")
    if not os.path.exists(english_downloads):
        try:
            os.makedirs(english_downloads, exist_ok=True)
        except Exception:
            pass
    return english_downloads


def get_default_alias():
    hostname = socket.gethostname()
    if hostname and hostname != "localhost":
        return hostname.capitalize()
    return "Pulsar-PC"


DEFAULT_CONFIG = {
    "alias": get_default_alias(),
    "device_model": "Pulsar OS",
    "device_type": "desktop",  # mobile, desktop, web, headless, server
    "port": 53317,
    "protocol": "http",
    "download_dir": get_default_download_dir(),
    "auto_accept": False,
    "discovery_enabled": True,
    "fingerprint": str(uuid.uuid4()),
    "version": "2.1"
}


class Config:
    _instance = None

    def __init__(self):
        self.data = dict(DEFAULT_CONFIG)
        self.load()

    @classmethod
    def get(cls):
        if cls._instance is None:
            cls._instance = Config()
        return cls._instance

    def load(self):
        if os.path.exists(CONFIG_FILE):
            try:
                with open(CONFIG_FILE, "r", encoding="utf-8") as f:
                    loaded = json.load(f)
                    self.data.update(loaded)
            except Exception as e:
                logger.error(f"Error reading config from {CONFIG_FILE}: {e}")
        else:
            self.save()

    def save(self):
        try:
            os.makedirs(CONFIG_DIR, exist_ok=True)
            with open(CONFIG_FILE, "w", encoding="utf-8") as f:
                json.dump(self.data, f, indent=2, ensure_ascii=False)
        except Exception as e:
            logger.error(f"Error saving config to {CONFIG_FILE}: {e}")

    def get_value(self, key, default=None):
        return self.data.get(key, default)

    def set_value(self, key, value):
        self.data[key] = value
        self.save()

    @property
    def alias(self):
        return self.data.get("alias", get_default_alias())

    @alias.setter
    def alias(self, val):
        self.set_value("alias", str(val))

    @property
    def auto_accept(self):
        return bool(self.data.get("auto_accept", False))

    @auto_accept.setter
    def auto_accept(self, val):
        self.set_value("auto_accept", bool(val))

    @property
    def download_dir(self):
        path = self.data.get("download_dir", get_default_download_dir())
        if not os.path.exists(path):
            os.makedirs(path, exist_ok=True)
        return path

    @download_dir.setter
    def download_dir(self, val):
        self.set_value("download_dir", str(val))

    @property
    def port(self):
        return int(self.data.get("port", 53317))

    @port.setter
    def port(self, val):
        self.set_value("port", int(val))

    @property
    def fingerprint(self):
        if "fingerprint" not in self.data or not self.data["fingerprint"]:
            self.data["fingerprint"] = str(uuid.uuid4())
            self.save()
        return self.data["fingerprint"]

    @property
    def device_model(self):
        return self.data.get("device_model", "Pulsar OS")

    @property
    def device_type(self):
        return self.data.get("device_type", "desktop")

    @property
    def protocol(self):
        return self.data.get("protocol", "http")

    @property
    def version(self):
        return self.data.get("version", "2.1")
