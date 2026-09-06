#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Sayri AI Plugins backend for Pulsar Store."""

import os
import json
import shutil
import zipfile
import tempfile
import urllib.request
from typing import Dict, Any, Optional
from pathlib import Path


class SayriPluginBackend:
    def __init__(self, log_fn=None):
        self.log = log_fn or (lambda msg: None)
        self.plugins_dir = os.path.expanduser("~/.config/sayri/plugins")
        os.makedirs(self.plugins_dir, exist_ok=True)

    def get_plugin_path(self, plugin_id: str) -> Optional[str]:
        target = os.path.join(self.plugins_dir, plugin_id)
        if os.path.isdir(target) and os.path.isfile(os.path.join(target, "manifest.json")):
            return target
        return None

    def is_installed(self, plugin_id: str) -> bool:
        return self.get_plugin_path(plugin_id) is not None

    def get_installed_version(self, plugin_id: str) -> Optional[str]:
        plugin_path = self.get_plugin_path(plugin_id)
        if not plugin_path:
            return None
        try:
            manifest_file = os.path.join(plugin_path, "manifest.json")
            with open(manifest_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            return str(data.get("version", "1.0.0"))
        except Exception:
            return "installed"

    def install(self, plugin_id: str, download_url: str) -> bool:
        self.log(f"[Sayri Plugin] Installing plugin: {plugin_id} from {download_url}...")
        try:
            target_dir = os.path.join(self.plugins_dir, plugin_id)
            with tempfile.TemporaryDirectory() as tmpdir:
                zip_path = os.path.join(tmpdir, "plugin.zip")
                req = urllib.request.Request(download_url, headers={"User-Agent": "PulsarStore/1.0"})
                with urllib.request.urlopen(req, timeout=25) as resp, open(zip_path, "wb") as f:
                    shutil.copyfileobj(resp, f)

                extracted_dir = os.path.join(tmpdir, "extracted")
                os.makedirs(extracted_dir, exist_ok=True)
                with zipfile.ZipFile(zip_path, "r") as zf:
                    zf.extractall(extracted_dir)

                # Look for manifest.json
                actual_plugin_dir = extracted_dir
                if not os.path.isfile(os.path.join(actual_plugin_dir, "manifest.json")):
                    for root, dirs, files in os.walk(extracted_dir):
                        if "manifest.json" in files:
                            actual_plugin_dir = root
                            break

                if os.path.exists(target_dir):
                    shutil.rmtree(target_dir)

                shutil.copytree(actual_plugin_dir, target_dir)

                # Ensure entrypoint is executable if specified
                manifest_file = os.path.join(target_dir, "manifest.json")
                if os.path.isfile(manifest_file):
                    try:
                        with open(manifest_file, "r", encoding="utf-8") as f:
                            m = json.load(f)
                        entrypoint = m.get("entrypoint")
                        if entrypoint:
                            ep_path = os.path.join(target_dir, entrypoint)
                            if os.path.isfile(ep_path):
                                os.chmod(ep_path, 0o755)
                    except Exception:
                        pass

            self.log(f"[Sayri Plugin] Plugin {plugin_id} installed successfully.")
            return True
        except Exception as e:
            self.log(f"[Sayri Plugin] Error installing plugin {plugin_id}: {e}")
            return False

    def uninstall(self, plugin_id: str) -> bool:
        self.log(f"[Sayri Plugin] Removing plugin: {plugin_id}...")
        try:
            # Stop running instances if any
            sayri_conf_dir = os.path.expanduser("~/.config/sayri")
            for filename in os.listdir(sayri_conf_dir):
                if filename.startswith(f"gateway_{plugin_id}") and filename.endswith(".pid"):
                    pid_file = os.path.join(sayri_conf_dir, filename)
                    try:
                        with open(pid_file, "r") as f:
                            pid = int(f.read().strip())
                        os.kill(pid, 15)  # SIGTERM
                        os.remove(pid_file)
                    except Exception:
                        pass

            target_dir = os.path.join(self.plugins_dir, plugin_id)
            if os.path.isdir(target_dir):
                shutil.rmtree(target_dir)
            self.log(f"[Sayri Plugin] Plugin {plugin_id} removed successfully.")
            return True
        except Exception as e:
            self.log(f"[Sayri Plugin] Error removing plugin {plugin_id}: {e}")
            return False

    def update(self, plugin_id: str, download_url: str) -> bool:
        return self.install(plugin_id, download_url)
