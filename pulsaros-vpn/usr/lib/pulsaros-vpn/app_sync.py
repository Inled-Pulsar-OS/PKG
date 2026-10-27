#!/usr/bin/env python3
import os
import re
import sys
import time
import shutil
import logging
import subprocess
from pathlib import Path
from typing import Tuple, List, Dict, Optional

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(levelname)s: %(message)s")
logger = logging.getLogger("PulsarPhone.AppSync")

ICONS_DIR = Path.home() / ".local/share/icons/pulsar_phone_apps"
DESKTOP_DIR = Path.home() / ".local/share/applications"
HELPER_APK = Path("/usr/lib/pulsaros-vpn/pulsar-bridge-helper.apk")
REMOTE_BRIDGE_DIR = "/sdcard/Android/data/com.droidtux.bridge/files"

def resolve_adb_serial() -> Optional[str]:
    try:
        res = subprocess.run(["adb", "devices"], capture_output=True, text=True, timeout=2)
        lines = [l.strip().split() for l in res.stdout.strip().splitlines()[1:] if l.strip()]
        for dev in lines:
            if len(dev) >= 2 and dev[1] == "device":
                return dev[0]
    except Exception as e:
        logger.error(f"Error checking adb devices: {e}")
    return None

def sync_phone_apps(serial: Optional[str] = None) -> Tuple[bool, int, str]:
    if not serial:
        serial = resolve_adb_serial()
    if not serial:
        return False, 0, "No device connected"

    logger.info(f"Syncing apps with device: {serial}")

    # Ensure local dirs exist
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    DESKTOP_DIR.mkdir(parents=True, exist_ok=True)

    # 1. Ensure helper APK is installed on phone if available
    try:
        check_pkg = subprocess.run(["adb", "-s", serial, "shell", "pm path com.droidtux.bridge"], capture_output=True, text=True, timeout=3)
        if "package:" not in check_pkg.stdout:
            # Install helper APK
            apk_path = HELPER_APK if HELPER_APK.exists() else Path("/usr/local/share/droidtux/droidtux-bridge-final.apk")
            if apk_path.exists():
                logger.info("Installing helper APK on device...")
                subprocess.run(["adb", "-s", serial, "install", "-r", "-g", str(apk_path)], capture_output=True, timeout=15)
    except Exception as e:
        logger.warning(f"Error checking/installing helper APK: {e}")

    # 2. Query launcher activities
    try:
        cmd = ["adb", "-s", serial, "shell", "cmd package query-activities --components --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER"]
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
        lines = [l.strip() for l in res.stdout.splitlines() if "/" in l.strip() and not l.strip().startswith("Activity")]
    except Exception as e:
        return False, 0, f"Error querying packages: {e}"

    packages = {}
    for l in lines:
        parts = l.split("/")
        pkg = parts[0].strip()
        act = parts[1].strip()
        if pkg not in packages:
            packages[pkg] = act

    if not packages:
        return False, 0, "No launcher apps found on phone"

    logger.info(f"Found {len(packages)} launcher apps on device")

    # 3. Trigger IconService to generate icons/labels
    try:
        subprocess.run(["adb", "-s", serial, "shell", "am start-foreground-service -n com.droidtux.bridge/.IconService"], capture_output=True, timeout=3)
        time.sleep(1.5)
    except Exception:
        pass

    # 4. Pull all generated icons and labels in batch
    temp_pull_dir = Path("/tmp/pulsar_phone_sync")
    shutil.rmtree(temp_pull_dir, ignore_errors=True)
    temp_pull_dir.mkdir(parents=True, exist_ok=True)

    try:
        subprocess.run(["adb", "-s", serial, "pull", f"{REMOTE_BRIDGE_DIR}/.", str(temp_pull_dir)], capture_output=True, timeout=15)
    except Exception as e:
        logger.warning(f"Batch pull failed: {e}")

    # 5. Generate .desktop entries for each launcher package
    synced_count = 0
    for pkg, act in packages.items():
        # Check label
        label_file = temp_pull_dir / f"{pkg}.label"
        if label_file.exists():
            try:
                name = label_file.read_text(encoding="utf-8", errors="ignore").strip()
            except Exception:
                name = ""
        else:
            name = ""

        if not name:
            # Fallback to pretty package name
            name = pkg.split(".")[-1].capitalize()

        # Check icon
        icon_src = temp_pull_dir / f"{pkg}.png"
        icon_dest = ICONS_DIR / f"{pkg}.png"
        if icon_src.exists() and icon_src.stat().st_size > 0:
            try:
                shutil.copy2(icon_src, icon_dest)
            except Exception:
                pass

        icon_str = str(icon_dest.absolute()) if icon_dest.exists() else "phone"

        desktop_content = (
            f"[Desktop Entry]\n"
            f"Type=Application\n"
            f"Name={name}\n"
            f"Comment=Pulsar Phone ({name})\n"
            f"Exec=/usr/bin/pulsar-phone-runner {pkg} \"{name}\"\n"
            f"Icon={icon_str}\n"
            f"Terminal=false\n"
            f"Categories=X-PulsarPhone;Utility;\n"
            f"StartupWMClass={pkg}\n"
        )

        desktop_path = DESKTOP_DIR / f"pulsar-phone-{pkg}.desktop"
        try:
            desktop_path.write_text(desktop_content, encoding="utf-8")
            synced_count += 1
        except Exception as e:
            logger.error(f"Failed to write desktop file for {pkg}: {e}")

    try:
        subprocess.run(["update-desktop-database", str(DESKTOP_DIR)], capture_output=True, timeout=5)
    except Exception:
        pass

    shutil.rmtree(temp_pull_dir, ignore_errors=True)
    logger.info(f"Successfully synced {synced_count} phone apps")
    return True, synced_count, f"{synced_count} apps sincronizadas correctamente"

if __name__ == "__main__":
    ok, count, msg = sync_phone_apps()
    print(f"Result: ok={ok}, count={count}, msg={msg}")
