#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Flatpak package backend for Pulsar Store."""

import subprocess
import shutil
from typing import Dict, Any, Optional


class FlatpakBackend:
    def __init__(self, log_fn=None):
        self.log = log_fn or (lambda msg: None)

    def is_available(self) -> bool:
        return shutil.which("flatpak") is not None

    def get_installed_version(self, app_id: str, flatpak_ref: Optional[str] = None) -> Optional[str]:
        if not self.is_available():
            return None
        # Clean app_id if a URL was passed
        clean_id = app_id.rsplit("/", 1)[-1].replace(".flatpakref", "") if ("/" in app_id) else app_id
        try:
            res = subprocess.run(["flatpak", "list", "--app", "--columns=application,version"], capture_output=True, text=True)
            if res.returncode == 0:
                for line in res.stdout.splitlines():
                    parts = line.split("\t")
                    if parts and parts[0].strip() in (clean_id, app_id):
                        return parts[1].strip() if len(parts) > 1 and parts[1].strip() else "installed"
        except Exception:
            pass
        return None

    def is_installed(self, app_id: str, flatpak_ref: Optional[str] = None) -> bool:
        return self.get_installed_version(app_id, flatpak_ref) is not None

    def install(self, app_id: str, flatpak_ref: Optional[str] = None) -> bool:
        if not self.is_available():
            self.log("[Flatpak] Error: flatpak is not installed on this system.")
            return False

        target = flatpak_ref or app_id
        self.log(f"[Flatpak] Installing Flatpak application: {target}...")

        if target.startswith("http://") or target.startswith("https://"):
            cmd = ["flatpak", "install", "-y", "--noninteractive", "--from", target]
        else:
            cmd = ["flatpak", "install", "-y", "--noninteractive", "flathub", target]

        try:
            proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            if proc.stdout:
                for line in iter(proc.stdout.readline, ''):
                    s = line.rstrip()
                    if s:
                        self.log(f"  {s}")
                proc.stdout.close()
            ret = proc.wait()
            if ret != 0 and "flathub" in cmd:
                # Fallback without explicit remote name
                cmd_fallback = ["flatpak", "install", "-y", "--noninteractive", target]
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

    def uninstall(self, app_id: str, flatpak_ref: Optional[str] = None) -> bool:
        if not self.is_available():
            return False
        clean_id = app_id.rsplit("/", 1)[-1].replace(".flatpakref", "") if ("/" in app_id) else app_id
        self.log(f"[Flatpak] Removing Flatpak application: {clean_id}...")
        cmd = ["flatpak", "uninstall", "-y", "--noninteractive", clean_id]
        try:
            proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            if proc.stdout:
                for line in iter(proc.stdout.readline, ''):
                    s = line.rstrip()
                    if s:
                        self.log(f"  {s}")
                proc.stdout.close()
            return proc.wait() == 0
        except Exception as e:
            self.log(f"[Flatpak] Uninstall error: {e}")
            return False

    def update(self, app_id: str, flatpak_ref: Optional[str] = None) -> bool:
        return self.install(app_id, flatpak_ref)
