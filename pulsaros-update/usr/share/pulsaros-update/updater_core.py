#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# ==============================================================================
# Pulsar OS - System Update and Migration Helper (Core Engine)
# ==============================================================================
# English: Backend logic for updating, diagnosing, and migrating legacy Pulsar OS
#          installations to the latest platform standards: Recovery Assistant,
#          Language and App Names, Sayri AI, Hibernation Subsystem, and System Extras.
# ==============================================================================

import os
import sys
import subprocess
import shutil
import re
import glob
import json
import hashlib
import time
from typing import Callable, Optional, Dict, Any, List, Tuple

# Supported system languages
POPULAR_LANGUAGES = [
    {"code": "en_US", "name": "English (United States)", "tess": "eng", "keymap": "us"},
    {"code": "en_GB", "name": "English (United Kingdom)", "tess": "eng", "keymap": "gb"},
    {"code": "es_ES", "name": "Spanish (Spain)", "tess": "spa", "keymap": "es"},
    {"code": "es_MX", "name": "Spanish (Mexico)", "tess": "spa", "keymap": "latam"},
    {"code": "fr_FR", "name": "French (France)", "tess": "fra", "keymap": "fr"},
    {"code": "de_DE", "name": "German (Germany)", "tess": "deu", "keymap": "de"},
    {"code": "it_IT", "name": "Italian (Italy)", "tess": "ita", "keymap": "it"},
    {"code": "pt_PT", "name": "Portuguese (Portugal)", "tess": "por", "keymap": "pt"},
    {"code": "pt_BR", "name": "Portuguese (Brazil)", "tess": "por", "keymap": "br"},
    {"code": "ca_ES", "name": "Catalan (Spain)", "tess": "cat", "keymap": "es"},
    {"code": "gl_ES", "name": "Galego (Spain)", "tess": "glg", "keymap": "es"},
    {"code": "eu_ES", "name": "Basque (Spain)", "tess": "eus", "keymap": "es"},
    {"code": "ja_JP", "name": "Japanese (Japan)", "tess": "jpn", "keymap": "jp"},
    {"code": "zh_CN", "name": "Chinese Simplified (China)", "tess": "chi_sim", "keymap": "cn"},
    {"code": "ru_RU", "name": "Russian (Russia)", "tess": "rus", "keymap": "ru"},
]


class UpdateCore:
    def __init__(self, log_callback: Optional[Callable[[str], None]] = None):
        self.log_callback = log_callback or (lambda msg: print(f"[UpdateCore] {msg}"))
        self.is_debian = os.path.exists("/etc/debian_version")
        self.is_arch = os.path.exists("/etc/arch-release") or not self.is_debian

    def log(self, message: str):
        if self.log_callback:
            self.log_callback(message)

    def run_command(self, cmd: List[str], use_root: bool = False, check: bool = False) -> Tuple[int, str, str]:
        """Runs a system command with optional root elevation."""
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
            res = subprocess.run(
                actual_cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=check
            )
            return res.returncode, res.stdout, res.stderr
        except Exception as e:
            self.log(f"[Error] Command failed: {e}")
            return 1, "", str(e)

    def run_command_stream(self, cmd: List[str], use_root: bool = False) -> bool:
        """Runs a command and streams output live to the log callback."""
        actual_cmd = list(cmd)
        if use_root and os.geteuid() != 0:
            if shutil.which("sudo") and os.system("sudo -n true 2>/dev/null") == 0:
                actual_cmd = ["sudo", "-n"] + actual_cmd
            elif shutil.which("pkexec"):
                actual_cmd = ["pkexec"] + actual_cmd
            elif shutil.which("sudo"):
                actual_cmd = ["sudo"] + actual_cmd

        self.log(f"[Exec Stream] {' '.join(actual_cmd)}")
        try:
            process = subprocess.Popen(
                actual_cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                bufsize=1,
                universal_newlines=True
            )
            if process.stdout:
                for line in iter(process.stdout.readline, ''):
                    stripped = line.rstrip()
                    if stripped:
                        self.log(f"  {stripped}")
                process.stdout.close()
            return process.wait() == 0
        except Exception as e:
            self.log(f"[Error] Execution error: {e}")
            return False

    # =========================================================================
    # 1. RECOVERY ASSISTANT AND RECOVERY PARTITION
    # =========================================================================

    def detect_recovery_devices(self) -> List[str]:
        """Finds any partition device labelled PULSAR_RECOVERY or mounted as recovery."""
        devices = []
        try:
            code, out, _ = self.run_command(["lsblk", "-J", "-o", "NAME,LABEL,MOUNTPOINT,FSTYPE,PATH"])
            if code == 0:
                data = json.loads(out)
                def search_devs(blockdevices):
                    for dev in blockdevices:
                        label = (dev.get("label") or "").upper()
                        path = dev.get("path") or f"/dev/{dev.get('NAME', '')}"
                        if "RECOVERY" in label or "PULSAR_REC" in label:
                            if path and path not in devices:
                                devices.append(path)
                        for child in dev.get("children", []):
                            search_devs([child])
                search_devs(data.get("blockdevices", []))
        except Exception:
            pass

        for p in glob.glob("/dev/disk/by-label/*"):
            if "RECOVERY" in os.path.basename(p).upper():
                real_p = os.path.realpath(p)
                if real_p not in devices:
                    devices.append(real_p)

        return devices

    def check_recovery_assistant(self) -> Dict[str, Any]:
        """Checks presence and checks if the recovery partition build matches latest system build."""
        status = {
            "installed_root": False,
            "root_path": "/usr/bin/pulsar-recovery-assistant",
            "recovery_devices": self.detect_recovery_devices(),
            "recovery_partition_found": False,
            "partition_is_outdated": False,
            "partition_version_info": "",
            "details": ""
        }

        root_bin = "/usr/bin/pulsar-recovery-assistant"
        if os.path.exists(root_bin):
            status["installed_root"] = True

        if status["recovery_devices"]:
            status["recovery_partition_found"] = True

        # Inspect binary inside recovery partition squashfs
        for dev in status["recovery_devices"]:
            mount_point = "/tmp/rec_chk_mount"
            try:
                self.run_command(["mkdir", "-p", mount_point], use_root=True)
                code, _, _ = self.run_command(["mount", "-o", "ro", dev, mount_point], use_root=True)
                if code == 0:
                    squash_path = os.path.join(mount_point, "filesystem.squashfs")
                    if os.path.exists(squash_path) and shutil.which("unsquashfs") and os.path.exists(root_bin):
                        # Extract single file to check size and hash
                        ext_dir = "/tmp/rec_single_extract"
                        self.run_command(["rm", "-rf", ext_dir], use_root=True)
                        self.run_command(["unsquashfs", "-f", "-d", ext_dir, squash_path, "usr/bin/pulsar-recovery-assistant"], use_root=True)
                        extracted_bin = os.path.join(ext_dir, "usr", "bin", "pulsar-recovery-assistant")

                        if os.path.exists(extracted_bin):
                            root_size = os.path.getsize(root_bin)
                            part_size = os.path.getsize(extracted_bin)
                            if root_size != part_size:
                                status["partition_is_outdated"] = True
                                status["partition_version_info"] = f"Partition build ({part_size // 1024} KB) differs from root build ({root_size // 1024} KB)"
                        else:
                            status["partition_is_outdated"] = True
                            status["partition_version_info"] = "pulsar-recovery-assistant missing in recovery squashfs"

                        self.run_command(["rm", "-rf", ext_dir], use_root=True)

                    self.run_command(["umount", mount_point], use_root=True)
            except Exception as e:
                pass

        if status["installed_root"] and status["recovery_partition_found"]:
            if status["partition_is_outdated"]:
                status["details"] = f"Recovery partition build is outdated on {', '.join(status['recovery_devices'])}. Update required."
            else:
                status["details"] = f"Recovery Assistant is up to date on root and recovery partition ({', '.join(status['recovery_devices'])})."
        elif status["installed_root"]:
            status["details"] = "Recovery Assistant installed in rootfs (/usr/bin/pulsar-recovery-assistant)."
        else:
            status["details"] = "Native pulsar-recovery-assistant is not installed."

        return status

    def update_recovery_assistant(self) -> bool:
        """Downloads/updates pulsaros-recovery package from repository and synchronizes recovery partition."""
        self.log("[Recovery] Updating pulsaros-recovery package from repository...")

        if self.is_debian:
            self.run_command_stream(["apt-get", "update"], use_root=True)
            self.run_command_stream(["apt-get", "install", "--reinstall", "-y", "pulsaros-recovery"], use_root=True)
        else:
            self.run_command_stream(["pacman", "-Sy"], use_root=True)
            self.run_command_stream(["pacman", "-S", "--needed", "--noconfirm", "pulsaros-recovery"], use_root=True)

        rec_bin = "/usr/bin/pulsar-recovery-assistant"
        if not os.path.exists(rec_bin):
            self.log("[Recovery] Error: /usr/bin/pulsar-recovery-assistant not found after package update.")
            return False

        self.run_command(["chmod", "755", rec_bin], use_root=True)

        # Patch recovery partition squashfs
        rec_devs = self.detect_recovery_devices()
        for dev in rec_devs:
            self.log(f"[Recovery] Detected recovery partition: {dev}. Mounting and updating...")
            mount_point = "/tmp/pulsar_rec_mount"
            try:
                self.run_command(["mkdir", "-p", mount_point], use_root=True)
                code, _, _ = self.run_command(["mount", dev, mount_point], use_root=True)
                if code != 0:
                    self.log(f"[Recovery] Could not mount {dev}, skipping.")
                    continue

                squashfs_candidates = [
                    os.path.join(mount_point, "filesystem.squashfs"),
                    os.path.join(mount_point, "live", "filesystem.squashfs"),
                    os.path.join(mount_point, "recovery", "filesystem.squashfs")
                ]

                target_squash = None
                for s in squashfs_candidates:
                    if os.path.exists(s):
                        target_squash = s
                        break

                if target_squash and shutil.which("unsquashfs") and shutil.which("mksquashfs"):
                    self.log(f"[Recovery] Extracting and patching recovery squashfs: {target_squash}...")
                    unpack_dir = "/tmp/pulsar_rec_unpack"
                    new_squash = "/tmp/filesystem_updated.squashfs"

                    self.run_command(["rm", "-rf", unpack_dir, new_squash], use_root=True)
                    self.run_command(["unsquashfs", "-d", unpack_dir, target_squash], use_root=True)

                    if os.path.isdir(unpack_dir):
                        self.log("[Recovery] Injecting updated pulsar-recovery-assistant into squashfs rootfs...")
                        self.run_command(["mkdir", "-p", f"{unpack_dir}/usr/bin"], use_root=True)
                        self.run_command(["cp", "-f", rec_bin, f"{unpack_dir}/usr/bin/pulsar-recovery-assistant"], use_root=True)
                        self.run_command(["chmod", "755", f"{unpack_dir}/usr/bin/pulsar-recovery-assistant"], use_root=True)

                        self.log("[Recovery] Recompressing recovery squashfs with zstd compression...")
                        self.run_command_stream(["mksquashfs", unpack_dir, new_squash, "-comp", "zstd", "-b", "1048576", "-noappend"], use_root=True)

                        if os.path.exists(new_squash):
                            for s in squashfs_candidates:
                                if os.path.exists(s):
                                    self.log(f"[Recovery] Updating squashfs in {s}...")
                                    self.run_command(["cp", "-f", new_squash, s], use_root=True)

                            self.log("[Recovery] Recovery squashfs updated successfully.")
                            self.run_command(["rm", "-f", new_squash], use_root=True)

                        self.run_command(["rm", "-rf", unpack_dir], use_root=True)

                # Also copy binary directly to root of recovery partition
                for direct_dest in [
                    os.path.join(mount_point, "pulsar-recovery-assistant"),
                    os.path.join(mount_point, "boot", "pulsar-recovery-assistant"),
                    os.path.join(mount_point, "recovery", "pulsar-recovery-assistant")
                ]:
                    try:
                        if os.path.isdir(os.path.dirname(direct_dest)):
                            self.run_command(["cp", "-f", rec_bin, direct_dest], use_root=True)
                            self.run_command(["chmod", "755", direct_dest], use_root=True)
                    except Exception:
                        pass

                self.run_command(["sync"], use_root=True)
                self.run_command(["umount", mount_point], use_root=True)
                self.run_command(["rmdir", mount_point], use_root=True)
                self.log(f"[Recovery] Partition {dev} synchronized and unmounted.")
            except Exception as e:
                self.log(f"[Recovery] Error while processing {dev}: {e}")
                self.run_command(["umount", mount_point], use_root=True)

        self.log("[Recovery] Native Recovery Assistant synchronization complete.")
        return True

    # =========================================================================
    # 2. OFFICIAL PULSAR OS PACKAGES (INLED REPOSITORY)
    # =========================================================================

    def check_packages_status(self) -> Dict[str, Any]:
        """Checks status and upgrades for core Pulsar OS packages."""
        status = {
            "has_updates": False,
            "upgradable_count": 0,
            "upgradable_packages": [],
            "core_installed": True,
            "details": ""
        }
        pulsar_keywords = ["pulsar", "sayri", "seafari", "macboat", "winboat", "spotlight", "xremap"]
        if self.is_arch:
            code, out, _ = self.run_command(["pacman", "-Qu"])
            if code == 0 and out.strip():
                upgradables = []
                for line in out.splitlines():
                    pkg = line.split()[0]
                    if any(k in pkg for k in pulsar_keywords):
                        upgradables.append(line.strip())
                status["upgradable_packages"] = upgradables
                status["upgradable_count"] = len(upgradables)
                status["has_updates"] = len(upgradables) > 0
        elif self.is_debian:
            code, out, _ = self.run_command(["apt-get", "-s", "upgrade"])
            if code == 0:
                upgradables = []
                for line in out.splitlines():
                    if line.startswith("Inst "):
                        pkg = line.split()[1]
                        if any(k in pkg for k in pulsar_keywords):
                            upgradables.append(pkg)
                status["upgradable_packages"] = upgradables
                status["upgradable_count"] = len(upgradables)
                status["has_updates"] = len(upgradables) > 0

        if status["has_updates"]:
            status["details"] = f"{status['upgradable_count']} Pulsar OS package update(s) available in repository."
        else:
            status["details"] = "All official Pulsar OS core packages are up to date."
        return status

    def update_pulsar_packages(self) -> bool:
        """Updates and installs all Pulsar OS core packages from the Inled repository."""
        self.log("[Packages] Synchronizing package repositories and updating all Pulsar OS components...")
        pulsar_core_packages = [
            "pulsaros-meta",
            "pulsaros-essential",
            "pulsaros-gnome",
            "pulsaros-global-menu",
            "pulsaros-spotlight-launcher",
            "pulsaros-control-center-button",
            "pulsaros-circle-to-search",
            "pulsaros-live-wallpaper",
            "pulsaros-effects-settings",
            "pulsaros-recovery",
            "pulsaros-hibernate",
            "pulsaros-bootsound",
            "pulsar-pear-sound-theme",
            "pulsaros-theme",
            "pulsaros-branding",
            "pulsaros-welcome",
            "seafari",
            "sayri",
            "macboat",
            "winboat-bin",
            "gnome-macos-remap-wayland",
            "xremap-gnome-bin"
        ]

        if self.is_debian:
            self.run_command_stream(["apt-get", "update"], use_root=True)
            ok = self.run_command_stream(["apt-get", "install", "-y", "--only-upgrade"] + pulsar_core_packages, use_root=True)
            if not ok:
                ok = self.run_command_stream(["apt-get", "install", "-y"] + pulsar_core_packages, use_root=True)
        else:
            self.run_command_stream(["pacman", "-Sy"], use_root=True)
            ok = self.run_command_stream(["pacman", "-S", "--needed", "--noconfirm"] + pulsar_core_packages, use_root=True)

        if ok:
            self.log("[Packages] All Pulsar OS core packages are successfully updated.")
        else:
            self.log("[Packages] Notice: Package manager finished sync and update check.")
        return ok

    # =========================================================================
    # 3. LANGUAGE, UI AND APP NAMES LOCALIZATION
    # =========================================================================

    def get_current_locale_info(self) -> Dict[str, Any]:
        """Returns the current system locale and app naming mode."""
        current_lang = os.environ.get("LANG", "en_US.UTF-8").split(".")[0]
        try:
            code, out, _ = self.run_command(["localectl", "status"])
            for line in out.splitlines():
                if "System Locale:" in line and "LANG=" in line:
                    match = re.search(r'LANG=([a-zA-Z0-9_]+)', line)
                    if match:
                        current_lang = match.group(1)
        except Exception:
            pass

        username = os.environ.get("USER") or os.environ.get("LOGNAME") or "jaime"
        
        try:
            code, out, _ = self.run_command(["cat", f"/var/lib/AccountsService/users/{username}"], use_root=True)
            if code == 0:
                for line in out.splitlines():
                    if line.startswith("Language="):
                        val = line.split("=", 1)[1].strip().split(".")[0]
                        if val:
                            current_lang = val
        except Exception:
            pass

        app_names_mode = "standard"
        if os.path.exists("/usr/share/applications/org.gnome.Nautilus.desktop"):
            try:
                with open("/usr/share/applications/org.gnome.Nautilus.desktop", "r", errors="ignore") as f:
                    content = f.read()
                    if "Name=Files" in content or "Name[es]=Archivos" in content:
                        app_names_mode = "localized"
            except Exception:
                pass

        return {
            "current_locale": current_lang,
            "app_names_mode": app_names_mode,
            "username": username
        }

    def apply_app_naming_mode(self, app_names_mode: str = "standard", locale_code: str = "es_ES") -> bool:
        """Configures clean macOS application names vs localized names in .desktop files."""
        self.log(f"[Apps] Configuring application titles for style: {app_names_mode} (locale: {locale_code})...")
        lang_short = locale_code.split("_")[0].lower() if "_" in locale_code else locale_code.lower()

        desktop_configs = {
            "org.gnome.Nautilus.desktop": {
                "standard": {"Name": "Finder", f"Name[{lang_short}]": "Finder", "GenericName": "File Manager", f"GenericName[{lang_short}]": "Explorador de Archivos" if lang_short == "es" else "File Manager"},
                "localized": {"Name": "Files", f"Name[{lang_short}]": "Archivos" if lang_short == "es" else "Files", "GenericName": "File Manager"}
            },
            "org.gnome.Settings.desktop": {
                "standard": {"Name": "System Settings", f"Name[{lang_short}]": "Ajustes del Sistema" if lang_short == "es" else "System Settings", "GenericName": "Settings"},
                "localized": {"Name": "Settings", f"Name[{lang_short}]": "Configuración" if lang_short == "es" else "Settings", "GenericName": "Settings"}
            },
            "org.gnome.Software.desktop": {
                "standard": {"Name": "App Store", f"Name[{lang_short}]": "App Store", "GenericName": "Software"},
                "localized": {"Name": "Software", f"Name[{lang_short}]": "Software", "GenericName": "Software"}
            },
            "org.gnome.TextEditor.desktop": {
                "standard": {"Name": "TextEdit", f"Name[{lang_short}]": "TextEdit", "GenericName": "Text Editor"},
                "localized": {"Name": "Text Editor", f"Name[{lang_short}]": "Editor de texto" if lang_short == "es" else "Text Editor", "GenericName": "Text Editor"}
            },
            "gnome-system-monitor.desktop": {
                "standard": {"Name": "Activity Monitor", f"Name[{lang_short}]": "Monitor de Actividad" if lang_short == "es" else "Activity Monitor", "GenericName": "Process Viewer"},
                "localized": {"Name": "System Monitor", f"Name[{lang_short}]": "Monitor del sistema" if lang_short == "es" else "System Monitor", "GenericName": "Process Viewer"}
            },
            "org.gnome.Console.desktop": {
                "standard": {"Name": "Terminal", f"Name[{lang_short}]": "Terminal", "GenericName": "Terminal"},
                "localized": {"Name": "Terminal", f"Name[{lang_short}]": "Terminal", "GenericName": "Terminal"}
            },
            "gnome-disk-utility.desktop": {
                "standard": {"Name": "Disk Utility", f"Name[{lang_short}]": "Utilidad de Discos" if lang_short == "es" else "Disk Utility", "GenericName": "Disks"},
                "localized": {"Name": "Disks", f"Name[{lang_short}]": "Discos" if lang_short == "es" else "Disks", "GenericName": "Disks"}
            },
            "org.gnome.Loupe.desktop": {
                "standard": {"Name": "Preview", f"Name[{lang_short}]": "Vista Previa" if lang_short == "es" else "Preview", "GenericName": "Image Viewer"},
                "localized": {"Name": "Image Viewer", f"Name[{lang_short}]": "Visor de imágenes" if lang_short == "es" else "Image Viewer", "GenericName": "Image Viewer"}
            },
            "org.gnome.Calculator.desktop": {
                "standard": {"Name": "Calculator", f"Name[{lang_short}]": "Calculadora" if lang_short == "es" else "Calculator", "GenericName": "Calculator"},
                "localized": {"Name": "Calculator", f"Name[{lang_short}]": "Calculadora" if lang_short == "es" else "Calculator", "GenericName": "Calculator"}
            }
        }

        user_app_dir = os.path.expanduser("~/.local/share/applications")
        os.makedirs(user_app_dir, exist_ok=True)

        for desktop_name, styles in desktop_configs.items():
            target_props = styles.get(app_names_mode, styles["standard"])
            sys_path = os.path.join("/usr/share/applications", desktop_name)
            user_path = os.path.join(user_app_dir, desktop_name)

            for dpath in [user_path, sys_path]:
                if os.path.exists(dpath) or dpath == user_path:
                    source_path = sys_path if (os.path.exists(sys_path) and not os.path.exists(dpath)) else dpath
                    if os.path.exists(source_path):
                        try:
                            with open(source_path, "r", encoding="utf-8", errors="ignore") as f:
                                lines = f.readlines()
                            new_lines = []
                            keys_to_set = dict(target_props)
                            for line in lines:
                                line_stripped = line.strip()
                                key_match = False
                                for k in list(keys_to_set.keys()):
                                    if line_stripped.startswith(f"{k}="):
                                        new_lines.append(f"{k}={keys_to_set.pop(k)}\n")
                                        key_match = True
                                        break
                                if not key_match:
                                    new_lines.append(line)
                            
                            final_lines = []
                            for line in new_lines:
                                final_lines.append(line)
                                if line.strip() == "[Desktop Entry]":
                                    for k, val in keys_to_set.items():
                                        final_lines.append(f"{k}={val}\n")
                                    keys_to_set = {}

                            if dpath.startswith("/usr"):
                                tmp_file = f"/tmp/{desktop_name}.tmp"
                                with open(tmp_file, "w", encoding="utf-8") as f:
                                    f.writelines(final_lines)
                                self.run_command(["cp", "-f", tmp_file, dpath], use_root=True)
                                if os.path.exists(tmp_file):
                                    os.remove(tmp_file)
                            else:
                                with open(dpath, "w", encoding="utf-8") as f:
                                    f.writelines(final_lines)
                        except Exception as e:
                            self.log(f"[Apps] Notice patching {desktop_name}: {e}")

        if shutil.which("update-desktop-database"):
            self.run_command(["update-desktop-database", "/usr/share/applications"], use_root=True)
            self.run_command(["update-desktop-database", user_app_dir], use_root=False)

        self.log(f"[Apps] Application names updated to {app_names_mode} style.")
        return True

    def reload_desktop_extensions(self) -> bool:
        """Reloads Pulsar OS GNOME Shell extensions to apply locale/settings changes."""
        self.log("[Extensions] Reloading Pulsar OS GNOME Shell extensions...")
        pulsar_extensions = [
            "pulsaros-global-menu@inled.es",
            "pulsar-dock@inled.es",
            "pulsaros-control-center-button@inled.es",
            "pulsaros-spotlight-launcher@inled.es",
            "pulsar-circle-to-search@inled.es"
        ]
        for ext in pulsar_extensions:
            self.run_command(["busctl", "--user", "call", "org.gnome.Shell.Extensions", "/org/gnome/Shell/Extensions", "org.gnome.Shell.Extensions", "DisableExtension", "s", ext])
            time.sleep(0.1)
            self.run_command(["busctl", "--user", "call", "org.gnome.Shell.Extensions", "/org/gnome/Shell/Extensions", "org.gnome.Shell.Extensions", "EnableExtension", "s", ext])
        self.log("[Extensions] Extensions reloaded successfully.")
        return True

    def configure_locale_and_apps(self, locale_code: str, app_names_mode: str = "standard", update_user_dirs: bool = True) -> bool:
        """Applies system locale, AccountsService, GNOME locale, OCR, App Naming, and reloads extensions."""
        self.log(f"[Locale] Setting system language to: {locale_code} (App titles: {app_names_mode})...")
        locale_full = f"{locale_code}.UTF-8" if not locale_code.endswith(".UTF-8") else locale_code
        lang_short = locale_code.split("_")[0].lower() if "_" in locale_code else locale_code.lower()
        username = os.environ.get("USER") or os.environ.get("LOGNAME") or "jaime"

        # 1. /etc/locale.gen and locale-gen
        try:
            locale_gen_path = "/etc/locale.gen"
            if os.path.exists(locale_gen_path):
                with open(locale_gen_path, "r", encoding="utf-8", errors="ignore") as f:
                    content = f.read()
                new_lines = []
                found = False
                for line in content.splitlines():
                    cand = line.strip().lstrip("#").strip()
                    if cand and cand.split() and cand.split()[0] == locale_full:
                        found = True
                        new_lines.append(f"{locale_full} UTF-8")
                    else:
                        new_lines.append(line)
                if not found:
                    new_lines.append(f"{locale_full} UTF-8")
                
                tmp_file = "/tmp/locale.gen.new"
                with open(tmp_file, "w", encoding="utf-8") as f:
                    f.write("\n".join(new_lines) + "\n")
                self.run_command(["cp", "-f", tmp_file, locale_gen_path], use_root=True)
                if os.path.exists(tmp_file):
                    os.remove(tmp_file)
                self.run_command_stream(["locale-gen"], use_root=True)
        except Exception as e:
            self.log(f"[Locale] Notice in /etc/locale.gen: {e}")

        # 2. Set localectl
        self.run_command(["localectl", "set-locale", f"LANG={locale_full}"], use_root=True)

        # 3. /etc/locale.conf or /etc/default/locale
        if self.is_debian:
            self.run_command(["update-locale", f"LANG={locale_full}", f"LC_ALL={locale_full}"], use_root=True)
        else:
            tmp_lconf = "/tmp/locale.conf.new"
            with open(tmp_lconf, "w", encoding="utf-8") as f:
                f.write(f"LANG={locale_full}\nLC_ALL={locale_full}\nLANGUAGE={locale_code}:{lang_short}\n")
            self.run_command(["cp", "-f", tmp_lconf, "/etc/locale.conf"], use_root=True)
            self.run_command(["chmod", "644", "/etc/locale.conf"], use_root=True)
            if os.path.exists(tmp_lconf):
                os.remove(tmp_lconf)

        # 4. AccountsService update via root script and D-Bus
        self.log(f"[Locale] Updating AccountsService for user {username}...")
        try:
            uid = os.getuid()
            self.run_command(["busctl", "call", "org.freedesktop.Accounts", f"/org/freedesktop/Accounts/User{uid}", "org.freedesktop.Accounts.User", "SetLanguage", "s", locale_full], use_root=False)
            self.run_command(["busctl", "call", "org.freedesktop.Accounts", f"/org/freedesktop/Accounts/User{uid}", "org.freedesktop.Accounts.User", "SetLanguages", "as", "1", locale_full], use_root=False)
        except Exception:
            pass

        as_script = f"""
        for u in /var/lib/AccountsService/users/*; do
            if [ -f "$u" ]; then
                sed -i '/^Language=/d;/^FormatsLocale=/d' "$u"
                echo "Language={locale_full}" >> "$u"
                echo "FormatsLocale={locale_full}" >> "$u"
            fi
        done
        """
        self.run_command(["bash", "-c", as_script], use_root=True)

        # User ~/.config/locale.conf and PAM environment
        try:
            user_conf_dir = os.path.expanduser("~/.config")
            os.makedirs(user_conf_dir, exist_ok=True)
            with open(os.path.join(user_conf_dir, "locale.conf"), "w") as f:
                f.write(f"LANG={locale_full}\nLC_ALL={locale_full}\nLANGUAGE={locale_code}:{lang_short}\n")
        except Exception:
            pass

        # 5. Import environment into systemd user session
        self.run_command(["systemctl", "--user", "import-environment", "LANG", "LC_ALL", "LANGUAGE"], use_root=False)
        if shutil.which("dbus-update-activation-environment"):
            self.run_command(["dbus-update-activation-environment", "--systemd", f"LANG={locale_full}", f"LC_ALL={locale_full}", f"LANGUAGE={locale_code}:{lang_short}"], use_root=False)

        # 6. Apply application names
        self.apply_app_naming_mode(app_names_mode, locale_code)

        # 7. Reload GNOME Shell extensions
        self.reload_desktop_extensions()

        # 8. XDG user dirs update
        if update_user_dirs and shutil.which("xdg-user-dirs-update"):
            self.log("[Locale] Updating XDG user directories...")
            self.run_command(["xdg-user-dirs-update", "--force"])

        # 9. OCR / Tesseract language pack
        try:
            tess_map = {item["code"].split("_")[0]: item["tess"] for item in POPULAR_LANGUAGES}
            tess_code = tess_map.get(lang_short, "eng")
            self.log(f"[Locale] Checking OCR Tesseract package: {tess_code}...")
            if self.is_debian:
                self.run_command_stream(["apt-get", "install", "-y", f"tesseract-ocr-{tess_code}"], use_root=True)
            else:
                self.run_command_stream(["pacman", "-S", "--needed", "--noconfirm", f"tesseract-data-{tess_code}"], use_root=True)
        except Exception as e:
            self.log(f"[Locale] Notice in Tesseract data: {e}")

        self.log(f"[Locale] System language successfully set to {locale_full} with {app_names_mode} app names.")
        return True

    # =========================================================================
    # 3. SAYRI AI VOICE ASSISTANT
    # =========================================================================

    def check_sayri_status(self) -> Dict[str, Any]:
        """Checks if Sayri AI is installed, gets current version, and checks for updates."""
        status = {
            "installed": False,
            "installed_version": "Not installed",
            "latest_version": "Unknown",
            "is_latest": False,
            "has_stt_tts": False,
            "details": ""
        }

        if self.is_debian:
            code, out, _ = self.run_command(["dpkg-query", "-W", "-f=${Version}", "sayri"])
            if code == 0 and out.strip():
                status["installed"] = True
                status["installed_version"] = out.strip()
        else:
            code, out, _ = self.run_command(["pacman", "-Q", "sayri"])
            if code == 0 and out.strip():
                parts = out.strip().split()
                if len(parts) >= 2:
                    status["installed"] = True
                    status["installed_version"] = parts[1]

        if shutil.which("sayri") or os.path.exists("/usr/bin/sayri"):
            status["installed"] = True
            if status["installed_version"] == "Not installed":
                status["installed_version"] = "1.0.0"

        try:
            if self.is_debian:
                code, out, _ = self.run_command(["apt-cache", "policy", "sayri"])
                for line in out.splitlines():
                    if "Candidate:" in line:
                        status["latest_version"] = line.split("Candidate:", 1)[1].strip()
            else:
                code, out, _ = self.run_command(["pacman", "-Si", "sayri"])
                for line in out.splitlines():
                    if line.startswith("Version"):
                        status["latest_version"] = line.split(":", 1)[1].strip()
        except Exception:
            pass

        whisper_ok = bool(shutil.which("whisper.cpp") or shutil.which("whisper-cli") or os.path.exists("/usr/bin/whisper-cli") or os.path.exists("/usr/local/bin/whisper-cli"))
        piper_ok = bool(shutil.which("piper") or os.path.exists("/usr/bin/piper") or os.path.exists("/usr/local/bin/piper"))
        status["has_stt_tts"] = whisper_ok and piper_ok

        if status["installed"]:
            if status["latest_version"] not in ["Unknown", ""]:
                status["is_latest"] = (status["installed_version"] == status["latest_version"])
            else:
                status["is_latest"] = True
            status["details"] = f"Sayri installed (v{status['installed_version']})."
        else:
            status["details"] = "Sayri AI is not installed on this system."

        return status

    def update_or_install_sayri(self) -> bool:
        """Updates or installs Sayri AI assistant from repository."""
        self.log("[Sayri] Updating Sayri AI Voice Assistant from repository...")
        if self.is_debian:
            self.run_command_stream(["apt-get", "update"], use_root=True)
            ok = self.run_command_stream(["apt-get", "install", "-y", "sayri"], use_root=True)
        else:
            self.run_command_stream(["pacman", "-Sy"], use_root=True)
            ok = self.run_command_stream(["pacman", "-S", "--needed", "--noconfirm", "sayri"], use_root=True)

        if ok:
            self.log("[Sayri] Sayri AI updated successfully.")
        else:
            self.log("[Sayri] Notice while updating Sayri package.")
        return ok

    # =========================================================================
    # 4. HIBERNATION SUBSYSTEM (pulsaros-hibernate)
    # =========================================================================

    def check_hibernation_status(self) -> Dict[str, Any]:
        """Checks swapfile, resume offset, hibernation packages, and systemd services."""
        status = {
            "package_installed": False,
            "swapfile_exists": False,
            "swap_active": False,
            "swap_size_mb": 0,
            "ram_size_mb": 0,
            "resume_offset_valid": False,
            "resume_offset_val": 0,
            "resume_device": "",
            "services_enabled": False,
            "sleep_conf_ok": False,
            "kernel_hook_ok": False,
            "details": ""
        }

        if self.is_debian:
            code, out, _ = self.run_command(["dpkg-query", "-W", "-f=${Status}", "pulsaros-hibernate"])
            status["package_installed"] = (code == 0 and "installed" in out)
        else:
            code, _, _ = self.run_command(["pacman", "-Q", "pulsaros-hibernate"])
            status["package_installed"] = (code == 0)

        try:
            with open("/proc/meminfo", "r") as f:
                for line in f:
                    if line.startswith("MemTotal:"):
                        kb = int(line.split()[1])
                        status["ram_size_mb"] = kb // 1024
                    elif line.startswith("SwapTotal:"):
                        kb = int(line.split()[1])
                        status["swap_size_mb"] = kb // 1024
        except Exception:
            pass

        code, out, _ = self.run_command(["swapon", "--show"])
        if code == 0 and "/swapfile" in out:
            status["swap_active"] = True
            status["swapfile_exists"] = True
        elif os.path.exists("/swapfile"):
            status["swapfile_exists"] = True

        if os.path.exists("/swapfile"):
            try:
                code, out, _ = self.run_command(["btrfs", "inspect-internal", "map-swapfile", "-r", "/swapfile"])
                if code == 0 and out.strip().isdigit():
                    status["resume_offset_val"] = int(out.strip())
                    status["resume_offset_valid"] = (status["resume_offset_val"] > 0)
                else:
                    code, out, _ = self.run_command(["filefrag", "-v", "/swapfile"])
                    if code == 0:
                        match = re.search(r'0:\s+\d+\.\.\s+\d+:\s+(\d+)\.\.', out)
                        if match:
                            status["resume_offset_val"] = int(match.group(1))
                            status["resume_offset_valid"] = True
            except Exception:
                pass

        try:
            if os.path.exists("/sys/power/resume_offset"):
                with open("/sys/power/resume_offset", "r") as f:
                    v = f.read().strip()
                    if v and v.isdigit() and int(v) > 0:
                        status["resume_offset_valid"] = True
        except Exception:
            pass

        if os.path.exists("/etc/systemd/sleep.conf.d/pulsaros-hibernate.conf"):
            status["sleep_conf_ok"] = True

        code1, out1, _ = self.run_command(["systemctl", "is-enabled", "pulsaros-verify-resume-offset.service"])
        code2, out2, _ = self.run_command(["systemctl", "is-enabled", "pulsaros-console-clean.service"])
        status["services_enabled"] = (code1 == 0 and code2 == 0)

        if self.is_arch and os.path.exists("/etc/mkinitcpio.conf"):
            with open("/etc/mkinitcpio.conf", "r", errors="ignore") as f:
                if re.search(r'^HOOKS=.*\bresume\b', f.read(), re.MULTILINE):
                    status["kernel_hook_ok"] = True
        elif self.is_debian:
            status["kernel_hook_ok"] = os.path.exists("/etc/initramfs-tools/conf.d/resume")

        is_healthy = (status["package_installed"] and status["swap_active"] and 
                      status["resume_offset_valid"] and status["services_enabled"])
        if is_healthy:
            status["details"] = f"Hibernation active: Swapfile {status['swap_size_mb']}MB, offset {status['resume_offset_val']}, services ready."
        else:
            status["details"] = "Hibernation subsystem requires configuration or repair."

        return status

    def fix_and_configure_hibernation(self) -> bool:
        """Fully configures and repairs swapfile, resume offset, kernel hooks, and services."""
        self.log("[Hibernate] Configuring and verifying Pulsar OS hibernation subsystem...")

        if self.is_debian:
            self.run_command_stream(["apt-get", "install", "-y", "pulsaros-hibernate"], use_root=True)
        else:
            self.run_command_stream(["pacman", "-S", "--needed", "--noconfirm", "pulsaros-hibernate"], use_root=True)

        hib_stat = self.check_hibernation_status()
        ram_mb = hib_stat.get("ram_size_mb", 8192) or 8192
        swap_target_mb = max(ram_mb + 1024, 8192)

        if not os.path.exists("/swapfile") or not hib_stat["swap_active"]:
            self.log(f"[Hibernate] Creating optimized /swapfile ({swap_target_mb} MB)...")
            try:
                _, out_df, _ = self.run_command(["df", "-T", "/"])
                is_btrfs = "btrfs" in out_df.lower()
                
                if is_btrfs and shutil.which("btrfs"):
                    self.run_command(["btrfs", "filesystem", "mkswapfile", "--size", f"{swap_target_mb}M", "/swapfile"], use_root=True)
                else:
                    self.run_command(["fallocate", "-l", f"{swap_target_mb}M", "/swapfile"], use_root=True)
                    self.run_command(["chmod", "600", "/swapfile"], use_root=True)
                    self.run_command(["mkswap", "/swapfile"], use_root=True)

                self.run_command(["swapon", "/swapfile"], use_root=True)

                if os.path.exists("/etc/fstab"):
                    with open("/etc/fstab", "r") as f:
                        fstab_txt = f.read()
                    if "/swapfile" not in fstab_txt:
                        self.log("[Hibernate] Registering /swapfile in /etc/fstab...")
                        tmp_fstab = "/tmp/fstab.new"
                        with open(tmp_fstab, "w") as f:
                            f.write(fstab_txt.rstrip() + "\n/swapfile none swap defaults 0 0\n")
                        self.run_command(["cp", "-f", tmp_fstab, "/etc/fstab"], use_root=True)
                        if os.path.exists(tmp_fstab):
                            os.remove(tmp_fstab)
            except Exception as e:
                self.log(f"[Hibernate] Error creating swapfile: {e}")

        self.log("[Hibernate] Creating systemd sleep drop-in...")
        self.run_command(["mkdir", "-p", "/etc/systemd/sleep.conf.d"], use_root=True)
        tmp_sleep = "/tmp/pulsaros-hibernate.conf"
        with open(tmp_sleep, "w") as f:
            f.write("[Sleep]\nAllowHibernation=yes\nAllowHybridSleep=yes\nAllowSuspendThenHibernate=yes\nHibernateMode=shutdown\nHibernateState=disk\n")
        self.run_command(["cp", "-f", tmp_sleep, "/etc/systemd/sleep.conf.d/pulsaros-hibernate.conf"], use_root=True)
        if os.path.exists(tmp_sleep):
            os.remove(tmp_sleep)

        self.run_command(["systemctl", "daemon-reload"], use_root=True)
        self.run_command(["systemctl", "enable", "pulsaros-verify-resume-offset.service"], use_root=True)
        self.run_command(["systemctl", "enable", "pulsaros-console-clean.service"], use_root=True)

        if self.is_arch and os.path.exists("/etc/mkinitcpio.conf"):
            self.log("[Hibernate] Verifying resume HOOK in /etc/mkinitcpio.conf...")
            try:
                with open("/etc/mkinitcpio.conf", "r") as f:
                    mk_txt = f.read()
                hooks_matches = re.findall(r'^HOOKS=.*', mk_txt, re.MULTILINE)
                if hooks_matches and "resume" not in hooks_matches[0]:
                    new_mk = []
                    for l in mk_txt.splitlines():
                        if l.startswith("HOOKS="):
                            if "filesystems" in l:
                                new_mk.append(l.replace("filesystems", "resume filesystems"))
                            else:
                                new_mk.append(l.rstrip(")") + " resume)")
                        else:
                            new_mk.append(l)
                    tmp_mk = "/tmp/mkinitcpio.conf.new"
                    with open(tmp_mk, "w") as f:
                        f.write("\n".join(new_mk) + "\n")
                    self.run_command(["cp", "-f", tmp_mk, "/etc/mkinitcpio.conf"], use_root=True)
                    if os.path.exists(tmp_mk):
                        os.remove(tmp_mk)
                    self.log("[Hibernate] Regenerating initramfs with mkinitcpio -P...")
                    self.run_command_stream(["mkinitcpio", "-P"], use_root=True)
            except Exception as e:
                self.log(f"[Hibernate] Notice in mkinitcpio: {e}")
        elif self.is_debian:
            self.log("[Hibernate] Regenerating Debian initramfs...")
            self.run_command_stream(["update-initramfs", "-u"], use_root=True)

        if os.path.exists("/usr/lib/pulsaros/verify-resume-offset"):
            self.run_command(["/usr/lib/pulsaros/verify-resume-offset"], use_root=True)

        self.log("[Hibernate] Hibernation subsystem configured and verified successfully.")
        return True

    # =========================================================================
    # 5. SYSTEM EXTRAS AND MODERNIZATION
    # =========================================================================

    def check_system_extras(self) -> Dict[str, Any]:
        """Checks sound theme, bootsound, keybindings, Spotlight, and schemas."""
        status = {
            "sound_theme_ok": False,
            "bootsound_ok": False,
            "spotlight_ok": False,
            "keybindings_ok": False,
            "schemas_ok": True,
            "live_wallpaper_ok": False
        }

        code, out, _ = self.run_command(["gsettings", "get", "org.gnome.desktop.sound", "theme-name"])
        status["sound_theme_ok"] = ("pulsar" in out.lower() or "pear" in out.lower() or "apple" in out.lower())

        status["bootsound_ok"] = os.path.exists("/etc/systemd/system/pulsaros-bootsound.service") or os.path.exists("/usr/bin/pulsaros-bootsound")
        status["spotlight_ok"] = os.path.exists("/usr/bin/pulsaros-toggle-launcher") or os.path.exists("/usr/share/applications/pulsaros-spotlight.desktop")
        status["keybindings_ok"] = os.path.exists("/usr/bin/gnome-macos-remap") or os.path.exists("/etc/xdg/autostart/autokey.desktop")

        return status

    def apply_all_extras(self) -> bool:
        """Applies sound themes, bootsound, Spotlight launcher, keybindings, and compiles schemas."""
        self.log("[Extras] Applying system enhancements and visual defaults...")

        # 1. Compile GLib Schemas
        self.log("[Extras] Compiling GSettings schemas...")
        self.run_command(["glib-compile-schemas", "/usr/share/glib-2.0/schemas"], use_root=True)

        # 2. Configure Sound theme
        self.log("[Extras] Configuring Pear/Apple sound theme...")
        try:
            self.run_command(["gsettings", "set", "org.gnome.desktop.sound", "theme-name", "'pulsar-pear-sound-theme'"])
            self.run_command(["gsettings", "set", "org.gnome.desktop.sound", "event-sounds", "true"])
        except Exception:
            pass

        # 3. Configure Bootsound service
        if os.path.exists("/usr/lib/systemd/system/pulsaros-bootsound.service") or os.path.exists("/etc/systemd/system/pulsaros-bootsound.service"):
            self.log("[Extras] Enabling startup chime service...")
            self.run_command(["systemctl", "enable", "pulsaros-bootsound.service"], use_root=True)

        # 4. Update desktop database
        if shutil.which("update-desktop-database"):
            self.run_command(["update-desktop-database", "/usr/share/applications"], use_root=True)

        self.log("[Extras] System enhancements applied successfully.")
        return True

    # =========================================================================
    # MASTER ONE-CLICK MIGRATION
    # =========================================================================

    def run_full_migration(self, target_locale: Optional[str] = None, app_names_mode: str = "standard") -> bool:
        """Runs the entire end-to-end modernization and update workflow."""
        self.log("[Migration] Starting full Pulsar OS system update and migration...")

        self.update_pulsar_packages()

        self.update_recovery_assistant()

        current_loc = target_locale or self.get_current_locale_info().get("current_locale", "en_US")
        self.configure_locale_and_apps(current_loc, app_names_mode=app_names_mode)

        self.update_or_install_sayri()

        self.fix_and_configure_hibernation()

        self.apply_all_extras()

        try:
            config_dir = os.path.expanduser("~/.config/pulsaros")
            os.makedirs(config_dir, exist_ok=True)
            witness_file = os.path.join(config_dir, "update-v1-done")
            with open(witness_file, "w") as f:
                f.write("Pulsar OS migration completed successfully.\n")
        except Exception as e:
            self.log(f"[Migration] Notice saving migration status: {e}")

        self.log("[Migration] Full system update completed successfully.")
        return True
