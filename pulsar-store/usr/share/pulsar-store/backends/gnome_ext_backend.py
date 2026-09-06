#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GNOME Shell Extensions backend for Pulsar Store."""

import os
import json
import shutil
import zipfile
import tempfile
import subprocess
import urllib.request
from typing import Dict, Any, Optional
from pathlib import Path


class GnomeExtensionBackend:
    def __init__(self, log_fn=None):
        self.log = log_fn or (lambda msg: None)
        self.user_ext_dir = os.path.expanduser("~/.local/share/gnome-shell/extensions")
        self.sys_ext_dir = "/usr/share/gnome-shell/extensions"

    def get_extension_path(self, uuid: str) -> Optional[str]:
        user_path = os.path.join(self.user_ext_dir, uuid)
        if os.path.isdir(user_path) and os.path.isfile(os.path.join(user_path, "metadata.json")):
            return user_path
        sys_path = os.path.join(self.sys_ext_dir, uuid)
        if os.path.isdir(sys_path) and os.path.isfile(os.path.join(sys_path, "metadata.json")):
            return sys_path
        return None

    def is_installed(self, uuid: str) -> bool:
        return self.get_extension_path(uuid) is not None

    def get_installed_version(self, uuid: str) -> Optional[str]:
        path = self.get_extension_path(uuid)
        if not path:
            return None
        try:
            meta_path = os.path.join(path, "metadata.json")
            with open(meta_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            return str(data.get("version", "installed"))
        except Exception:
            return "installed"

    def install(self, uuid: str, download_url: str) -> bool:
        self.log(f"[GNOME Extension] Installing extension: {uuid} from {download_url}...")
        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                zip_path = os.path.join(tmpdir, "extension.zip")
                req = urllib.request.Request(download_url, headers={"User-Agent": "PulsarStore/1.0"})
                with urllib.request.urlopen(req, timeout=20) as resp, open(zip_path, "wb") as f:
                    shutil.copyfileobj(resp, f)

                # Extract to temp directory to find uuid if not known
                extracted_dir = os.path.join(tmpdir, "extracted")
                os.makedirs(extracted_dir, exist_ok=True)
                with zipfile.ZipFile(zip_path, 'r') as zf:
                    zf.extractall(extracted_dir)

                target_uuid = uuid
                meta_file = os.path.join(extracted_dir, "metadata.json")
                if os.path.exists(meta_file):
                    try:
                        with open(meta_file, "r", encoding="utf-8") as f:
                            m = json.load(f)
                        target_uuid = m.get("uuid", uuid)
                    except Exception:
                        pass

                dest_dir = os.path.join(self.user_ext_dir, target_uuid)
                if os.path.exists(dest_dir):
                    shutil.rmtree(dest_dir)
                os.makedirs(os.path.dirname(dest_dir), exist_ok=True)
                shutil.copytree(extracted_dir, dest_dir)

                # Compile schemas if present
                schema_dir = os.path.join(dest_dir, "schemas")
                if os.path.isdir(schema_dir) and shutil.which("glib-compile-schemas"):
                    subprocess.run(["glib-compile-schemas", schema_dir], check=False)

                # Enable extension via DBus
                subprocess.run([
                    "busctl", "--user", "call",
                    "org.gnome.Shell.Extensions",
                    "/org/gnome/Shell/Extensions",
                    "org.gnome.Shell.Extensions",
                    "EnableExtension", "s", target_uuid
                ], check=False)

                self.log(f"[GNOME Extension] Extension {target_uuid} installed and enabled successfully.")
                return True
        except Exception as e:
            self.log(f"[GNOME Extension] Error installing extension: {e}")
            return False

    def uninstall(self, uuid: str) -> bool:
        self.log(f"[GNOME Extension] Removing extension: {uuid}...")
        try:
            subprocess.run([
                "busctl", "--user", "call",
                "org.gnome.Shell.Extensions",
                "/org/gnome/Shell/Extensions",
                "org.gnome.Shell.Extensions",
                "DisableExtension", "s", uuid
            ], check=False)

            user_path = os.path.join(self.user_ext_dir, uuid)
            if os.path.isdir(user_path):
                shutil.rmtree(user_path)
            self.log(f"[GNOME Extension] Extension {uuid} removed.")
            return True
        except Exception as e:
            self.log(f"[GNOME Extension] Error removing extension: {e}")
            return False

    def update(self, uuid: str, download_url: str) -> bool:
        return self.install(uuid, download_url)
