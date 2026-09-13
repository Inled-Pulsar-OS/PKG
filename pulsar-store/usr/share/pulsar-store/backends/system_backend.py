#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Native System Package backend (Debian APT / Arch Pacman / .deb / .pkg.tar.zst) for Pulsar Store."""

import os
import re
import sys
import subprocess
import shutil
import tempfile
import urllib.request
from typing import Dict, Any, List, Optional


class SystemPackageBackend:
    def __init__(self, log_fn=None):
        self.log = log_fn or (lambda msg: None)
        self.is_debian = os.path.exists("/etc/debian_version")
        self.is_arch = os.path.exists("/etc/arch-release") or not self.is_debian
        self.last_error: Optional[str] = None
        self.last_installed_name: Optional[str] = None
        self._assume_list: List[str] = []

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
            lines = []
            if proc.stdout:
                for line in iter(proc.stdout.readline, ''):
                    s = line.rstrip()
                    if s:
                        self.log(f"  {s}")
                        lines.append(s)
                proc.stdout.close()
            ok = proc.wait() == 0
            if not ok:
                self.last_error = "\n".join(lines[-15:]) or f"Command failed: {' '.join(actual_cmd)}"
            return ok
        except Exception as e:
            self.log(f"[Error] Command error: {e}")
            self.last_error = str(e)
            return False

    def list_installed_names(self) -> List[str]:
        """English: Lists installed package names (pacman -Qq / dpkg-query).
        Español: Lista los nombres de paquetes instalados."""
        try:
            if self.is_arch:
                res = subprocess.run(["pacman", "-Qq"], capture_output=True, text=True, timeout=30)
                return [x for x in res.stdout.splitlines() if x]
            elif self.is_debian:
                res = subprocess.run(["dpkg-query", "-W", "-f=${binary:Package}\n"], capture_output=True, text=True, timeout=30)
                return [x for x in res.stdout.splitlines() if x]
        except Exception:
            pass
        return []

    def get_package_name(self, local_path: str) -> Optional[str]:
        """English: Reads the concrete package name of a local Arch package.
        Español: Lee el nombre real del paquete de un .pkg.tar.zst local."""
        try:
            res = subprocess.run(["pacman", "-Qp", "--print-format", "%n", local_path],
                                 capture_output=True, text=True, timeout=30)
            if res.returncode == 0 and res.stdout.strip():
                return res.stdout.strip().splitlines()[0].strip()
        except Exception:
            pass
        try:
            res = subprocess.run(["pacman", "-Qip", local_path],
                                 capture_output=True, text=True, timeout=30,
                                 env={**os.environ, "LC_ALL": "C"})
            for line in res.stdout.splitlines():
                m = re.match(r"^Name\s*:\s*(.+)$", line)
                if m:
                    return m.group(1).strip()
        except Exception:
            pass
        return None

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
        self.last_error = None
        self.last_installed_name = None
        self._assume_list = []
        self.log(f"[System] Downloading package from: {url}...")
        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                filename = url.split("?")[0].split("/")[-1]
                local_path = os.path.join(tmpdir, filename)

                req = urllib.request.Request(url, headers={"User-Agent": "PulsarStore/1.0"})
                with urllib.request.urlopen(req, timeout=120) as resp, open(local_path, "wb") as f:
                    shutil.copyfileobj(resp, f)

                self.log(f"[System] Installing local package: {filename}...")
                if filename.endswith(".pkg.tar.zst") and self.is_arch:
                    pkg_name = self.get_package_name(local_path)
                    if pkg_name:
                        self.log(f"[System] Native package name: {pkg_name}")
                    if not self._ensure_arch_deps(local_path):
                        return False
                    cmd = ["pacman", "-U", "--noconfirm", "--overwrite", "*"]
                    for dep in self._assume_list:
                        cmd.append(f"--assume-installed={dep}")
                    cmd.append(local_path)
                    if self.run_command(cmd, use_root=True):
                        self.last_installed_name = pkg_name
                        return True
                    return False
                elif filename.endswith(".deb") and self.is_debian:
                    if self.run_command(["apt-get", "install", "-y", local_path], use_root=True):
                        self.last_installed_name = filename[: -len(".deb")] or None
                        return True
                    return False
                elif filename.endswith(".deb") and self.is_arch:
                    # Fallback on Arch if deb-tap or dpkg is available
                    if shutil.which("dpkg"):
                        if self.run_command(["dpkg", "-i", local_path], use_root=True):
                            self.last_installed_name = filename[: -len(".deb")] or None
                            return True
                        return False
                    self.last_error = "A .deb package cannot be installed on this Arch system (no dpkg available)."
                    self.log("[System] Warning: .deb package format on Arch requires dpkg or conversion.")
                    return False
        except Exception as e:
            self.log(f"[System] Error installing from URL: {e}")
            self.last_error = str(e)
            return False
        return False

    def _privileged_run(self, args, timeout: int = 600) -> subprocess.CompletedProcess:
        """EN: Run a command as root (sudo -n / pkexec) capturing combined output.
        ES: Ejecuta un comando como root capturando la salida."""
        cmd = list(args)
        if os.geteuid() != 0:
            if shutil.which("sudo") and os.system("sudo -n true 2>/dev/null") == 0:
                cmd = ["sudo", "-n"] + cmd
            elif shutil.which("pkexec"):
                cmd = ["pkexec"] + cmd
            elif shutil.which("sudo"):
                cmd = ["sudo"] + cmd
        return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout,
                              env={**os.environ, "LC_ALL": "C"})

    def _package_depends_on(self, local_path: str) -> List[str]:
        """En: Parse declared dependencies of a local package (Arch).
        Es: Parsea las dependencias declaradas de un paquete local (Arch)."""
        try:
            res = subprocess.run(
                ["pacman", "-Qip", local_path],
                capture_output=True, text=True, timeout=30,
                env={**os.environ, "LC_ALL": "C"},
            )
        except Exception as e:
            self.log(f"[System] Warning: could not inspect package dependencies: {e}")
            return []
        names: List[str] = []
        in_deps = False
        for line in res.stdout.splitlines():
            if re.match(r"^Depends On\s*:", line, re.IGNORECASE):
                in_deps = True
            elif re.match(r"^[A-Za-z][A-Za-z ]*:", line):
                in_deps = False
            if in_deps and ":" in line:
                payload = line.split(":", 1)[1]
                for token in payload.split():
                    name = re.split(r"[<>=]+", token)[0].strip()
                    if name and name not in names:
                        names.append(name)
        return names

    def _ensure_arch_deps(self, local_path: str) -> bool:
        """En: Install native dependencies automatically before pacman -U; names that are
        not real repository packages (e.g. the virtual 'libuuid') are passed via
        --assume-installed so the transaction can still complete.
        Es: Instala dependencias automáticamente; las que no son paquetes reales del repositorio
        (p. ej. la virtual 'libuuid') se pasan con --assume-installed."""
        deps = self._package_depends_on(local_path)
        if not deps:
            return True
        self.log(f"[System] Detected native dependencies: {' '.join(deps)}")
        assume: List[str] = []
        for name in deps:
            if subprocess.run(["pacman", "-Q", name], capture_output=True).returncode == 0:
                continue
            res = self._privileged_run(["pacman", "-S", "--needed", "--noconfirm", name])
            tail = (res.stdout or "") + (res.stderr or "")
            if res.returncode == 0:
                continue
            if "target not found" in tail:
                self.log(f"[System] '{name}' is not a repository package; will be assumed installed.")
                assume.append(name)
            else:
                self.last_error = f"Could not install dependency '{name}':\n" + "\n".join(tail.splitlines()[-8:])
                return False
        self._assume_list = assume
        return True

    def uninstall(self, package_name: str) -> bool:
        self.log(f"[System] Removing package: {package_name}...")
        if self.is_arch:
            return self.run_command(["pacman", "-R", "--noconfirm", package_name], use_root=True)
        elif self.is_debian:
            return self.run_command(["apt-get", "remove", "-y", package_name], use_root=True)
        return False

    def update(self, package_name: str, download_url: Optional[str] = None) -> bool:
        return self.install(package_name, download_url)
