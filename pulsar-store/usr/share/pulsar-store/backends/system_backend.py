#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Native System Package backend (Debian APT / Arch Pacman / .deb / .pkg.tar.zst) for Pulsar Store."""

import os
import sys
import subprocess
import shutil
import tempfile
import urllib.request
from typing import Dict, Any, Optional


class SystemPackageBackend:
    def __init__(self, log_fn=None):
        self.log = log_fn or (lambda msg: None)
        self.is_debian = os.path.exists("/etc/debian_version")
        self.is_arch = os.path.exists("/etc/arch-release") or not self.is_debian

    def run_command(self, cmd, use_root=True) -> bool:
        actual_cmd = list(cmd)
        if use_root and os.geteuid() != 0:
            if shutil.which("sudo") and os.system("sudo -n true 2>/dev/null") == 0:
                actual_cmd = ["sudo", "-n"] + actual_cmd
            elif shutil.which("pkexec"):
                actual_cmd = ["pkexec"] + actual_cmd
            elif shutil.which("sudo"):
                actual_cmd = ["sudo"] + actual_cmd

        self.log(f"[Exec] {' '.join(actual_cmd)}")
        try:
            proc = subprocess.Popen(actual_cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            if proc.stdout:
                for line in iter(proc.stdout.readline, ''):
                    s = line.rstrip()
                    if s:
                        self.log(f"  {s}")
                proc.stdout.close()
            return proc.wait() == 0
        except Exception as e:
            self.log(f"[Error] Command error: {e}")
            return False

    def is_installed(self, package_name: str) -> bool:
        if self.is_arch:
            res = subprocess.run(["pacman", "-Q", package_name], capture_output=True, text=True)
            return res.returncode == 0
        elif self.is_debian:
            res = subprocess.run(["dpkg-query", "-W", "-f=${Status}", package_name], capture_output=True, text=True)
            return res.returncode == 0 and "installed" in res.stdout
        return False

    def get_installed_version(self, package_name: str) -> Optional[str]:
        if self.is_arch:
            res = subprocess.run(["pacman", "-Q", package_name], capture_output=True, text=True)
            if res.returncode == 0 and res.stdout.strip():
                parts = res.stdout.strip().split()
                if len(parts) >= 2:
                    return parts[1]
        elif self.is_debian:
            res = subprocess.run(["dpkg-query", "-W", "-f=${Version}", package_name], capture_output=True, text=True)
            if res.returncode == 0 and res.stdout.strip():
                return res.stdout.strip()
        return None

    def install(self, package_name: str, download_url: Optional[str] = None) -> bool:
        # 1. Direct download URL provided (.deb / .pkg.tar.zst)
        if download_url and (download_url.endswith(".deb") or download_url.endswith(".pkg.tar.zst")):
            return self.install_from_url(download_url)

        # 2. Package name from repository
        self.log(f"[System] Installing repository package: {package_name}...")
        if self.is_arch:
            return self.run_command(["pacman", "-S", "--needed", "--noconfirm", package_name], use_root=True)
        elif self.is_debian:
            self.run_command(["apt-get", "update"], use_root=True)
            return self.run_command(["apt-get", "install", "-y", package_name], use_root=True)
        return False

    def install_from_url(self, url: str) -> bool:
        self.log(f"[System] Downloading package from: {url}...")
        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                filename = url.split("?")[0].split("/")[-1]
                local_path = os.path.join(tmpdir, filename)

                req = urllib.request.Request(url, headers={"User-Agent": "PulsarStore/1.0"})
                with urllib.request.urlopen(req, timeout=30) as resp, open(local_path, "wb") as f:
                    shutil.copyfileobj(resp, f)

                self.log(f"[System] Installing local package: {filename}...")
                if filename.endswith(".pkg.tar.zst") and self.is_arch:
                    return self.run_command(["pacman", "-U", "--noconfirm", "--overwrite", "*", local_path], use_root=True)
                elif filename.endswith(".deb") and self.is_debian:
                    return self.run_command(["apt-get", "install", "-y", local_path], use_root=True)
                elif filename.endswith(".deb") and self.is_arch:
                    # Fallback on Arch if deb-tap or dpkg is available
                    if shutil.which("dpkg"):
                        return self.run_command(["dpkg", "-i", local_path], use_root=True)
                    self.log("[System] Warning: .deb package format on Arch requires dpkg or conversion.")
                    return False
        except Exception as e:
            self.log(f"[System] Error installing from URL: {e}")
            return False
        return False

    def uninstall(self, package_name: str) -> bool:
        self.log(f"[System] Removing package: {package_name}...")
        if self.is_arch:
            return self.run_command(["pacman", "-R", "--noconfirm", package_name], use_root=True)
        elif self.is_debian:
            return self.run_command(["apt-get", "remove", "-y", package_name], use_root=True)
        return False

    def update(self, package_name: str, download_url: Optional[str] = None) -> bool:
        return self.install(package_name, download_url)
