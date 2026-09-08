#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Flatpak package backend for Pulsar Store."""

import os
import shutil
import subprocess
import tempfile
import urllib.request
from typing import Dict, Any, Optional, List


def _norm_id(s: str) -> str:
    """Normalizes application identifiers for fuzzy matching (removes hyphens, underscores, dots, lowercase)."""
    if not s:
        return ""
    return s.lower().replace("-", "").replace("_", "").replace(".", "").strip()


class FlatpakBackend:
    def __init__(self, log_fn=None):
        self.log = log_fn or (lambda msg: None)

    def is_available(self) -> bool:
        return shutil.which("flatpak") is not None

    def _get_installed_apps(self) -> List[Dict[str, str]]:
        """Returns list of all installed flatpak applications."""
        if not self.is_available():
            return []
        apps = []
        try:
            res = subprocess.run(
                ["flatpak", "list", "--app", "--columns=application,version,name"],
                capture_output=True,
                text=True
            )
            if res.returncode == 0:
                for line in res.stdout.splitlines():
                    parts = line.split("\t")
                    if parts and parts[0].strip():
                        app_id = parts[0].strip()
                        ver = parts[1].strip() if len(parts) > 1 and parts[1].strip() else "installed"
                        name = parts[2].strip() if len(parts) > 2 else ""
                        apps.append({"id": app_id, "version": ver, "name": name})
        except Exception as e:
            self.log(f"[Flatpak] Error listing installed apps: {e}")
        return apps

    def find_installed_match(self, app_id: str, item: Optional[Dict[str, Any]] = None) -> Optional[Dict[str, str]]:
        installed = self._get_installed_apps()
        if not installed:
            return None

        clean_id = app_id.rsplit("/", 1)[-1].replace(".flatpakref", "").replace(".flatpak", "") if ("/" in app_id) else app_id
        target_norm = _norm_id(clean_id)
        item_name_norm = _norm_id(item.get("name", "")) if item else ""

        # 1. Exact ID match
        for app in installed:
            if app["id"] == clean_id or app["id"] == app_id:
                return app

        # 2. Normalized ID match (e.g. com.ios-notes.app vs com.iosnotes.app)
        for app in installed:
            if _norm_id(app["id"]) == target_norm:
                return app

        # 3. Substring / contains match
        for app in installed:
            app_norm = _norm_id(app["id"])
            if target_norm and (target_norm in app_norm or app_norm in target_norm):
                return app
            if item_name_norm and item_name_norm == _norm_id(app.get("name", "")):
                return app

        return None

    def get_installed_version(self, app_id: str, flatpak_ref: Optional[str] = None, item: Optional[Dict[str, Any]] = None) -> Optional[str]:
        match = self.find_installed_match(app_id, item)
        if match:
            return match.get("version", "installed")
        return None

    def is_installed(self, app_id: str, flatpak_ref: Optional[str] = None, item: Optional[Dict[str, Any]] = None) -> bool:
        return self.find_installed_match(app_id, item) is not None

    def install(self, app_id: str, flatpak_ref: Optional[str] = None, item: Optional[Dict[str, Any]] = None) -> bool:
        if not self.is_available():
            self.log("[Flatpak] Error: flatpak is not installed on this system.")
            return False

        # Ensure Flathub user remote is available
        try:
            subprocess.run(
                ["flatpak", "remote-add", "--user", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"],
                capture_output=True
            )
        except Exception:
            pass

        target = flatpak_ref or (item.get("download_url") if item else None) or app_id
        self.log(f"[Flatpak] Installing Flatpak application from {target}...")

        tmp_bundle = None
        try:
            # Case A: Binary .flatpak single-file bundle
            if target.endswith(".flatpak") or ".flatpak" in target:
                if target.startswith("http://") or target.startswith("https://"):
                    tmp_bundle = os.path.join(tempfile.gettempdir(), f"pulsar-{_norm_id(app_id)}.flatpak")
                    self.log(f"[Flatpak] Downloading Flatpak bundle to {tmp_bundle}...")
                    req = urllib.request.Request(target, headers={"User-Agent": "PulsarStore/1.0"})
                    with urllib.request.urlopen(req, timeout=60) as resp, open(tmp_bundle, "wb") as f:
                        shutil.copyfileobj(resp, f)
                    bundle_path = tmp_bundle
                else:
                    bundle_path = target

                cmd = ["flatpak", "install", "--user", "-y", "--noninteractive", "--bundle", bundle_path]

            # Case B: .flatpakref file
            elif target.startswith("http://") or target.startswith("https://") or target.endswith(".flatpakref"):
                cmd = ["flatpak", "install", "--user", "-y", "--noninteractive", "--from", target]

            # Case C: Flathub application ID
            else:
                cmd = ["flatpak", "install", "--user", "-y", "--noninteractive", "flathub", target]

            self.log(f"[Flatpak] Executing: {' '.join(cmd)}")
            proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            if proc.stdout:
                for line in iter(proc.stdout.readline, ''):
                    s = line.rstrip()
                    if s:
                        self.log(f"  {s}")
                proc.stdout.close()
            ret = proc.wait()

            if ret != 0 and "flathub" in cmd:
                # Fallback without explicit remote
                cmd_fallback = ["flatpak", "install", "--user", "-y", "--noninteractive", target]
                proc2 = subprocess.Popen(cmd_fallback, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                if proc2.stdout:
                    for line in iter(proc2.stdout.readline, ''):
                        s = line.rstrip()
                        if s:
                            self.log(f"  {s}")
                    proc2.stdout.close()
                return proc2.wait() == 0

            return ret == 0

        except Exception as e:
            self.log(f"[Flatpak] Installation error: {e}")
            return False
        finally:
            if tmp_bundle and os.path.exists(tmp_bundle):
                try:
                    os.remove(tmp_bundle)
                except Exception:
                    pass

    def uninstall(self, app_id: str, flatpak_ref: Optional[str] = None, item: Optional[Dict[str, Any]] = None) -> bool:
        if not self.is_available():
            return False

        match = self.find_installed_match(app_id, item)
        real_id = match["id"] if match else (app_id.rsplit("/", 1)[-1].replace(".flatpakref", "") if ("/" in app_id) else app_id)

        self.log(f"[Flatpak] Removing Flatpak application: {real_id}...")
        cmd = ["flatpak", "uninstall", "--user", "-y", "--noninteractive", real_id]
        try:
            proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            if proc.stdout:
                for line in iter(proc.stdout.readline, ''):
                    s = line.rstrip()
                    if s:
                        self.log(f"  {s}")
                proc.stdout.close()
            ret = proc.wait()
            if ret != 0:
                # Try system-wide uninstall fallback
                cmd_sys = ["flatpak", "uninstall", "-y", "--noninteractive", real_id]
                proc2 = subprocess.Popen(cmd_sys, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                if proc2.stdout:
                    for line in iter(proc2.stdout.readline, ''):
                        s = line.rstrip()
                        if s:
                            self.log(f"  {s}")
                    proc2.stdout.close()
                return proc2.wait() == 0
            return ret == 0
        except Exception as e:
            self.log(f"[Flatpak] Uninstall error: {e}")
            return False

    def update(self, app_id: str, flatpak_ref: Optional[str] = None, item: Optional[Dict[str, Any]] = None) -> bool:
        return self.install(app_id, flatpak_ref, item)
