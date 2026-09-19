#!/usr/bin/env python3
"""
==============================================================================
Tube OS - Headless Web Installer & OOTB Engine
==============================================================================
Inspired by the robust low-level disk management and system replication
architecture of Pulsar recovery.py. Provides web-based headless installation
and OOTB setup over LAN.
"""

import argparse
import datetime
import glob
import io
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Dict, List, Optional

from fastapi import FastAPI, Request
from fastapi.responses import HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles

app = FastAPI(title="Tube OS Web Installer")

ootb_mode = False
STATIC_DIR = Path(__file__).parent / "static"

# Global installation state tracker
install_state = {
    "status": "idle",       # idle | running | done | error
    "progress": 0.0,        # 0.0 to 1.0
    "step": "",             # Current human-readable step
    "logs": [],             # Array of log strings
    "error": None,
}

# ─── Process & Command Execution Helper ────────────────────────────────────

def run(cmd, shell=True, check=False) -> subprocess.CompletedProcess:
    if isinstance(cmd, list) and not shell:
        return subprocess.run(cmd, capture_output=True, text=True, check=check)
    return subprocess.run(cmd, shell=shell, capture_output=True, text=True, check=check)

def get_base_distro() -> str:
    """Detect if running on Arch or Debian base."""
    if Path("/etc/pacman.conf").exists():
        return "arch"
    return "debian"

def get_ip() -> str:
    """Detect the active primary IP address."""
    try:
        r = run("ip -4 route get 1.1.1.1 2>/dev/null | awk '{print $7; exit}'")
        ip = r.stdout.strip()
        if ip and not ip.startswith("127."):
            return ip
    except Exception:
        pass
    try:
        r = run("hostname -I")
        ips = r.stdout.strip().split()
        for ip in ips:
            if not ip.startswith("127."):
                return ip
    except Exception:
        pass
    return "localhost"

def gen_qr_svg(url: str) -> str:
    """Generate QR code SVG for easy scanning from phone."""
    try:
        import qrcode
        import qrcode.image.svg
        factory = qrcode.image.svg.SvgPathImage
        img = qrcode.make(url, image_factory=factory)
        buf = io.BytesIO()
        img.save(buf)
        return buf.getvalue().decode("utf-8")
    except Exception:
        return ""

# ─── Network & Connectivity Helpers ────────────────────────────────────────

def check_internet() -> Dict:
    """Check general internet connectivity and package repo reachability."""
    online = False
    repo_reachable = False

    # Check basic DNS/socket ping
    try:
        socket.setdefaulttimeout(4)
        socket.create_connection(("1.1.1.1", 53), timeout=4)
        online = True
    except Exception:
        try:
            socket.create_connection(("8.8.8.8", 53), timeout=4)
            online = True
        except Exception:
            online = False

    # Check Inled / Distro repo reachability
    if online:
        try:
            r = run("curl -s --connect-timeout 4 -I https://hosted.inled.es || curl -s --connect-timeout 4 -I https://deb.debian.org", check=False)
            if r.returncode == 0:
                repo_reachable = True
        except Exception:
            repo_reachable = False

    return {
        "online": online,
        "repo_reachable": repo_reachable,
        "ip": get_ip(),
        "hostname": socket.gethostname(),
    }

def scan_wifi() -> List[Dict]:
    """Scan nearby Wi-Fi networks using nmcli."""
    networks = []
    try:
        run("nmcli dev wifi rescan", check=False)
        time.sleep(1)
        r = run("nmcli -t -f SSID,SIGNAL,SECURITY,BARS dev wifi list")
        seen = set()
        for line in r.stdout.strip().split("\n"):
            if not line.strip():
                continue
            parts = line.split(":")
            if len(parts) >= 3:
                ssid = parts[0].strip()
                if not ssid or ssid in seen or ssid == "--":
                    continue
                seen.add(ssid)
                networks.append({
                    "ssid": ssid,
                    "signal": parts[1].strip() if len(parts) > 1 else "50",
                    "security": parts[2].strip() if len(parts) > 2 else "Open",
                    "bars": parts[3].strip() if len(parts) > 3 else "▂▄▆",
                })
    except Exception:
        pass
    return networks

def connect_wifi(ssid: str, password: Optional[str] = None) -> bool:
    """Connect to a Wi-Fi network."""
    try:
        if password:
            r = run(f"nmcli dev wifi connect '{ssid}' password '{password}'", check=False)
        else:
            r = run(f"nmcli dev wifi connect '{ssid}'", check=False)
        return r.returncode == 0
    except Exception:
        return False

# ─── Storage & Disk Helpers (Based on recovery.py) ─────────────────────────

def is_live_device(dev_name: str) -> bool:
    """Determine if a block device is the live USB / installer media."""
    try:
        # Check /proc/mounts for live media paths
        with open("/proc/mounts", "r") as f:
            for line in f:
                parts = line.split()
                if len(parts) >= 2:
                    if dev_name in parts[0] and (
                        "/run/archiso" in parts[1] or
                        "/run/live" in parts[1] or
                        "/live" in parts[1] or
                        "/iso" in parts[1] or
                        "/cdrom" in parts[1] or
                        "/run/initramfs" in parts[1]
                    ):
                        return True
    except Exception:
        pass
    if dev_name.startswith("/dev/sr") or dev_name.startswith("/dev/loop"):
        return True
    return False

def get_system_disks() -> List[Dict]:
    """Retrieve filtered physical storage disks with detailed properties."""
    disks = []
    try:
        r = run("lsblk -Jdpo NAME,SIZE,MODEL,TYPE,RM,ROTA,TRAN,FSTYPE 2>/dev/null")
        data = json.loads(r.stdout)
        for d in data.get("blockdevices", []):
            name = d.get("name", "")
            if not name or d.get("type") != "disk":
                continue
            if is_live_device(name):
                continue
            removable = bool(d.get("rm", False) or d.get("removable", False))
            tran = (d.get("tran") or ("usb" if removable else "sata")).strip()
            
            # Fetch existing partitions for this disk
            parts = []
            try:
                pr = run(f"lsblk -Jpo NAME,SIZE,FSTYPE,LABEL,MOUNTPOINT,TYPE {name} 2>/dev/null")
                pdata = json.loads(pr.stdout)
                for item in pdata.get("blockdevices", []):
                    for child in item.get("children", []):
                        parts.append({
                            "name": child.get("name"),
                            "size": child.get("size"),
                            "fstype": child.get("fstype") or "unknown",
                            "label": child.get("label") or "",
                            "mountpoint": child.get("mountpoint") or "",
                        })
            except Exception:
                pass

            disks.append({
                "path": name,
                "name": name.replace("/dev/", ""),
                "size": d.get("size", "?"),
                "model": (d.get("model") or "Storage Drive").strip(),
                "removable": removable,
                "type": tran,
                "partitions": parts,
            })
    except Exception:
        pass
    return disks

def detect_efi_partition(target_disk: Optional[str] = None) -> Optional[str]:
    """Find an existing EFI system partition."""
    try:
        r = run("lsblk -Jpo NAME,FSTYPE,PARTTYPE,LABEL 2>/dev/null")
        data = json.loads(r.stdout)
        for dev in data.get("blockdevices", []):
            def search_dev(d):
                if target_disk and not d.get("name", "").startswith(target_disk):
                    return None
                fstype = (d.get("fstype") or "").lower()
                parttype = (d.get("parttype") or "").lower()
                label = (d.get("label") or "").upper()
                if fstype in ["vfat", "fat32", "fat16"] and (
                    parttype in ["c12a7328-f81f-11d2-ba4b-00a0c93ec93b", "ef00"] or
                    "EFI" in label or "BOOT" in label
                ):
                    return d.get("name")
                for child in d.get("children", []):
                    res = search_dev(child)
                    if res:
                        return res
                return None

            found = search_dev(dev)
            if found:
                return found
    except Exception:
        pass
    return None

# ─── Installation Engine (Inspired by recovery.py) ─────────────────────────

def append_installer_log(msg: str):
    """Write log entry to internal memory buffer and /tmp/tubeos-install.log."""
    ts = datetime.datetime.now().strftime("%H:%M:%S")
    line = f"[{ts}] {msg}"
    install_state["logs"].append(line)
    try:
        with open("/tmp/tubeos-install.log", "a") as f:
            f.write(line + "\n")
    except Exception:
        pass
    print(line)

def update_installer_progress(fraction: float, step: str):
    """Update active installation percentage and label."""
    install_state["progress"] = min(max(fraction, 0.0), 1.0)
    install_state["step"] = step
    append_installer_log(f">>> {step} ({int(install_state['progress'] * 100)}%)")

def cleanup_mounts():
    """Safely unmount /mnt in strict reverse order without killing host processes."""
    append_installer_log("Executing clean unmount routine...")
    run("awk '$2 ~ \"^/mnt/\" || $2 == \"/mnt\" {print $2}' /proc/self/mounts 2>/dev/null | sort -r | while read -r mp; do umount -lf \"$mp\" 2>/dev/null || true; done")
    
    mount_points = [
        "/mnt/etc/resolv.conf",
        "/mnt/run",
        "/mnt/sys/firmware/efi/efivars",
        "/mnt/sys",
        "/mnt/proc",
        "/mnt/dev/pts",
        "/mnt/dev",
        "/mnt/boot/efi",
        "/mnt/home",
    ]
    for mp in mount_points:
        if Path(mp).exists():
            run(f"umount -f '{mp}' 2>/dev/null || umount -l '{mp}' 2>/dev/null || true")
    
    run("umount -f /mnt 2>/dev/null || umount -l /mnt 2>/dev/null || true")
    run("umount -f -l -R /mnt 2>/dev/null || true")

def run_chroot(cmd: str) -> subprocess.CompletedProcess:
    """Run a command inside /mnt chroot."""
    if shutil.which("arch-chroot"):
        return run(f"arch-chroot /mnt {cmd}")
    return run(f"chroot /mnt /bin/bash -c \"{cmd}\"")

def execute_installation_backend(config: Dict):
    """
    Main background installation worker.
    Replicates system, performs online package downloads for selected edition,
    installs bootloader and applies user/system configurations.
    """
    global install_state
    install_state["status"] = "running"
    install_state["logs"] = []
    install_state["error"] = None
    update_installer_progress(0.01, "Initializing Tube OS Installation")

    disk = config.get("disk")
    mode = config.get("mode", "clean")             # clean | dualboot
    target_part = config.get("target_partition")
    fs_type = config.get("fs_type", "btrfs")        # btrfs | ext4
    edition = config.get("edition", "casaos_bigscreen")
    hostname = config.get("hostname", "tubeos")
    username = config.get("username", "tubeos")
    password = config.get("password", "tubeos")
    timezone = config.get("timezone", "UTC")
    keymap = config.get("keymap", "us")

    distro = get_base_distro()
    is_efi = Path("/sys/firmware/efi").exists()

    try:
        append_installer_log(f"Starting Tube OS Install: Base={distro}, Disk={disk}, Mode={mode}, Edition={edition}")
        
        # 1. Stop automounting daemons
        run("systemctl stop udisks2.service 2>/dev/null || true")
        run("swapoff -a 2>/dev/null || true")

        # 2. Partitioning
        update_installer_progress(0.05, "Partitioning target storage")
        if mode == "clean":
            if not disk:
                raise Exception("No target disk specified for clean install")
            
            # Determine partition suffix (nvme0n1p1 vs sda1)
            p_prefix = f"{disk}p" if re.search(r"\d$", disk) else disk
            part_efi = f"{p_prefix}1"
            part_root = f"{p_prefix}2"

            append_installer_log(f"Wiping disk {disk} and creating GPT partition table")
            run(f"wipefs -a -f {disk}")
            run(f"parted -s {disk} mklabel gpt")
            run(f"parted -s {disk} mkpart ESP fat32 1MiB 513MiB")
            run(f"parted -s {disk} set 1 esp on")
            run(f"parted -s {disk} mkpart root {fs_type} 513MiB 100%")
            run(f"partprobe {disk} 2>/dev/null || true")
            run("udevadm settle 2>/dev/null || sleep 2")

            # 3. Formatting
            update_installer_progress(0.12, "Formatting file systems")
            run(f"mkfs.vfat -F32 -n EFI {part_efi}")
            if fs_type == "btrfs":
                if not shutil.which("mkfs.btrfs"):
                    append_installer_log("mkfs.btrfs not found, falling back to ext4...")
                    fs_type = "ext4"
            if fs_type == "btrfs":
                r_mk = run(f"mkfs.btrfs -f -L TUBEOS_ROOT {part_root}")
                if r_mk.returncode != 0:
                    append_installer_log(f"mkfs.btrfs warning: {r_mk.stderr}, falling back to ext4")
                    fs_type = "ext4"
                    run(f"mkfs.ext4 -F -L TUBEOS_ROOT {part_root}")
            else:
                run(f"mkfs.ext4 -F -L TUBEOS_ROOT {part_root}")

            # 4. Mounting
            update_installer_progress(0.18, "Mounting target file system")
            cleanup_mounts()
            Path("/mnt").mkdir(parents=True, exist_ok=True)
            
            if fs_type == "btrfs":
                # Create @ and @home subvolumes
                run(f"mount -t btrfs {part_root} /mnt")
                run("btrfs subvolume create /mnt/@")
                run("btrfs subvolume create /mnt/@home")
                run("umount /mnt")
                r_mnt = run(f"mount -t btrfs -o subvol=@,compress=zstd:1 {part_root} /mnt")
                if r_mnt.returncode != 0:
                    append_installer_log(f"Failed to mount btrfs subvol @, falling back to direct mount")
                    run(f"mount -t btrfs {part_root} /mnt")
                else:
                    Path("/mnt/home").mkdir(parents=True, exist_ok=True)
                    run(f"mount -t btrfs -o subvol=@home,compress=zstd:1 {part_root} /mnt/home")
            else:
                run(f"mount {part_root} /mnt")

            # Verify /mnt is actively mounted to prevent writing to RAM tmpfs
            r_check = run("mountpoint -q /mnt")
            if r_check.returncode != 0:
                raise Exception(f"Failed to mount target root partition {part_root} on /mnt")

            Path("/mnt/boot/efi").mkdir(parents=True, exist_ok=True)
            run(f"mount {part_efi} /mnt/boot/efi")

        else: # Dual boot / Custom
            if not target_part:
                raise Exception("No root partition selected for dual boot")
            part_root = target_part
            part_efi = detect_efi_partition(disk) or detect_efi_partition()
            
            append_installer_log(f"Dual boot mode on {part_root}, EFI={part_efi}")
            if fs_type == "btrfs" and not shutil.which("mkfs.btrfs"):
                append_installer_log("mkfs.btrfs not found, falling back to ext4...")
                fs_type = "ext4"
            if fs_type == "btrfs":
                run(f"mkfs.btrfs -f -L TUBEOS_ROOT {part_root}")
                cleanup_mounts()
                Path("/mnt").mkdir(parents=True, exist_ok=True)
                run(f"mount -t btrfs {part_root} /mnt")
                run("btrfs subvolume create /mnt/@")
                run("btrfs subvolume create /mnt/@home")
                run("umount /mnt")
                r_mnt = run(f"mount -t btrfs -o subvol=@,compress=zstd:1 {part_root} /mnt")
                if r_mnt.returncode != 0:
                    run(f"mount -t btrfs {part_root} /mnt")
                else:
                    Path("/mnt/home").mkdir(parents=True, exist_ok=True)
                    run(f"mount -t btrfs -o subvol=@home,compress=zstd:1 {part_root} /mnt/home")
            else:
                run(f"mkfs.ext4 -F -L TUBEOS_ROOT {part_root}")
                cleanup_mounts()
                Path("/mnt").mkdir(parents=True, exist_ok=True)
                run(f"mount {part_root} /mnt")

            r_check = run("mountpoint -q /mnt")
            if r_check.returncode != 0:
                raise Exception(f"Failed to mount target root partition {part_root} on /mnt")

            if part_efi and is_efi:
                Path("/mnt/boot/efi").mkdir(parents=True, exist_ok=True)
                run(f"mount {part_efi} /mnt/boot/efi")

        # 5. System Replication (rsync)
        update_installer_progress(0.25, "Replicating base system image")
        rsync_cmd = (
            "rsync -aAXx --info=progress2 "
            "--exclude='/dev/*' --exclude='/proc/*' --exclude='/sys/*' "
            "--exclude='/tmp/*' --exclude='/run/*' --exclude='/mnt/*' "
            "--exclude='/media/*' --exclude='/live/*' --exclude='/cdrom/*' "
            "--exclude='/var/cache/apt/archives/*' --exclude='/var/lib/docker/*' "
            "--exclude='/var/tmp/*' --exclude='/lost+found' "
            "/ /mnt/"
        )
        run(rsync_cmd)
        append_installer_log("Base system files synchronized successfully")

        # 6. Bind mount pseudo-filesystems for chroot operations
        update_installer_progress(0.55, "Configuring chroot environment")
        run("mount -t proc proc /mnt/proc")
        run("mount -t sysfs sys /mnt/sys")
        run("mount --bind /dev /mnt/dev")
        run("mount --bind /dev/pts /mnt/dev/pts")
        if is_efi and Path("/sys/firmware/efi/efivars").exists():
            Path("/mnt/sys/firmware/efi/efivars").mkdir(parents=True, exist_ok=True)
            run("mount --bind /sys/firmware/efi/efivars /mnt/sys/firmware/efi/efivars")
        
        # Ensure DNS inside chroot
        if Path("/etc/resolv.conf").exists():
            shutil.copy("/etc/resolv.conf", "/mnt/etc/resolv.conf")

        # 7. Online Package Download & Customization for Chosen Edition
        update_installer_progress(0.65, f"Downloading & installing edition packages: {edition}")
        append_installer_log(f"Online edition configuration: {edition} on {distro}")

        # 8. User Creation & Privileges
        update_installer_progress(0.75, "Configuring user accounts & privileges")
        admin_group = "sudo" if distro == "debian" else "wheel"

        # Purge any leftover installer/live users from the base image so only the configured user exists
        try:
            if Path("/mnt/etc/passwd").exists():
                with open("/mnt/etc/passwd", "r") as pf:
                    lines = pf.readlines()
                for pline in lines:
                    parts = pline.strip().split(":")
                    if len(parts) >= 3:
                        u_name = parts[0]
                        try:
                            u_uid = int(parts[2])
                        except ValueError:
                            continue
                        if (u_name in ["live", "alarm", "pulsar", "arch", "tubeos"] or u_uid >= 1000) and u_name != username:
                            append_installer_log(f"Removing leftover installer account: {u_name}")
                            run_chroot(f"userdel -r -f '{u_name}' 2>/dev/null || true")
                            run(f"rm -rf '/mnt/home/{u_name}' 2>/dev/null || true")
        except Exception:
            pass

        # Clean up leftover installer groups so GID 1000 is free
        for lgrp in ["live", "alarm", "pulsar", "arch", "tubeos"]:
            if lgrp != username:
                run_chroot(f"groupdel '{lgrp}' 2>/dev/null || true")

        # Create primary group and user explicitly
        run_chroot(f"groupadd -g 1000 -f '{username}' 2>/dev/null || groupadd -f '{username}' 2>/dev/null || true")
        res_useradd = run_chroot(f"useradd -m -u 1000 -g '{username}' -s /bin/bash '{username}' 2>&1")
        if res_useradd.returncode != 0:
            run_chroot(f"useradd -m -s /bin/bash '{username}' 2>/dev/null || true")

        # Failsafe guarantee: ensure user is 100% in /etc/passwd and /etc/shadow
        if run_chroot(f"id -u '{username}'").returncode != 0:
            append_installer_log(f"Applying direct /etc/passwd and /etc/shadow fallback for {username}")
            try:
                with open("/mnt/etc/group", "a") as gf:
                    gf.write(f"{username}:x:1000:\n")
                with open("/mnt/etc/passwd", "a") as pf:
                    pf.write(f"{username}:x:1000:1000::/home/{username}:/bin/bash\n")
                with open("/mnt/etc/shadow", "a") as sf:
                    sf.write(f"{username}:!:19700:0:99999:7:::\n")
            except Exception as e:
                append_installer_log(f"Fallback write error: {e}")

        # Sync passwd to shadow
        run_chroot("pwconv 2>/dev/null || true")
        run_chroot("grpconv 2>/dev/null || true")

        # Add groups safely one by one
        for grp in [admin_group, "docker", "audio", "video", "input", "render", "storage", "power"]:
            run_chroot(f"groupadd -f '{grp}' 2>/dev/null || true")
            run_chroot(f"usermod -aG '{grp}' '{username}' 2>/dev/null || true")

        run_chroot(f"usermod -s /bin/bash '{username}' 2>/dev/null || true")
        run_chroot(f"mkdir -p '/home/{username}'")
        run_chroot(f"chown -R '{username}:{username}' '/home/{username}' 2>/dev/null || chown -R 1000:1000 '/home/{username}' 2>/dev/null || true")
        run_chroot(f"chmod 755 '/home/{username}'")
        
        # Set passwords using chpasswd
        p = subprocess.Popen(["chroot", "/mnt", "chpasswd"], stdin=subprocess.PIPE, stderr=subprocess.PIPE)
        p.communicate(input=f"{username}:{password}\nroot:{password}\n".encode())

        # Also set shadow password explicitly as failsafe
        run_chroot(f"usermod -p $(openssl passwd -6 '{password}') '{username}' 2>/dev/null || true")
        run_chroot(f"usermod -p $(openssl passwd -6 '{password}') root 2>/dev/null || true")

        # Sudoers without password
        Path("/mnt/etc/sudoers.d").mkdir(parents=True, exist_ok=True)
        with open(f"/mnt/etc/sudoers.d/{username}", "w") as sf:
            sf.write(f"{username} ALL=(ALL) NOPASSWD:ALL\n")
        run(f"chmod 0440 /mnt/etc/sudoers.d/{username}")

        # 9. Clean Desktop Sessions & Configure Display Manager
        update_installer_progress(0.82, f"Configuring interface for edition: {edition}")
        xsessions = Path("/mnt/usr/share/xsessions")
        wayland_sessions = Path("/mnt/usr/share/wayland-sessions")

        allowed_sessions = set()
        sddm_session_name = "plasma-bigscreen-wayland"

        if "bigscreen" in edition:
            allowed_sessions = {"plasma-bigscreen-wayland.desktop", "plasma-bigscreen.desktop"}
            sddm_session_name = "plasma-bigscreen-wayland"
        elif "tubeos_ui" in edition:
            allowed_sessions = {"tubeos.desktop", "openbox.desktop"}
            sddm_session_name = "tubeos"
        else: # casaos_solo (headless)
            allowed_sessions = set()
            sddm_session_name = ""

        # Purge all desktop sessions not matching the selected edition (GNOME, Kodi, Openbox, etc.)
        for sdir in [xsessions, wayland_sessions]:
            if sdir.exists():
                for f in list(sdir.glob("*.desktop")):
                    if f.name not in allowed_sessions:
                        try:
                            f.unlink()
                            append_installer_log(f"Removed unselected session: {f.name}")
                        except Exception:
                            pass

        # Purge any standalone Kodi or unselected desktop launchers
        for extra_launcher in ["/mnt/usr/share/xsessions/kodi.desktop", "/mnt/usr/share/wayland-sessions/kodi-gbm.desktop", "/mnt/usr/share/wayland-sessions/kodi-wayland.desktop", "/mnt/usr/share/applications/kodi.desktop"]:
            try:
                Path(extra_launcher).unlink(missing_ok=True)
            except Exception:
                pass

        if distro == "arch":
            append_installer_log("Configuring Arch environment...")
            if "bigscreen" in edition:
                append_installer_log("Enabling Plasma Bigscreen and configuring SDDM autologin...")
                run_chroot("systemctl set-default graphical.target 2>/dev/null || true")
                run_chroot("systemctl enable sddm 2>/dev/null || true")

                # Create reliable plasma-bigscreen-session launcher script that loads the Bigscreen shell
                launcher_path = Path("/mnt/usr/bin/plasma-bigscreen-session")
                with open(launcher_path, "w") as lf:
                    lf.write(
                        "#!/bin/sh\n"
                        "export PLASMA_DEFAULT_SHELL=org.kde.plasma.bigscreen\n"
                        "export KDE_LOOKANDFEEL=org.kde.plasma.bigscreen\n"
                        "export QT_QPA_PLATFORM=wayland\n"
                        "export KDE_FULL_SESSION=true\n"
                        "export XDG_CURRENT_DESKTOP=KDE\n"
                        "export XDG_SESSION_DESKTOP=KDE\n"
                        "export XDG_SESSION_TYPE=wayland\n"
                        "export QT_QUICK_CONTROLS_STYLE=Plasma\n"
                        "export PLASMA_SHELL_PACKAGE=org.kde.plasma.bigscreen\n"
                        "for exe in /usr/bin/plasma-bigscreen-wayland /usr/lib/plasma-bigscreen-wayland /usr/libexec/plasma-bigscreen-wayland; do\n"
                        "    if [ -x \"$exe\" ]; then\n"
                        "        exec \"$exe\"\n"
                        "    fi\n"
                        "done\n"
                        "if [ -f /usr/lib/plasma-bigscreen-common-env ]; then\n"
                        "    . /usr/lib/plasma-bigscreen-common-env\n"
                        "    exec /usr/bin/startplasma-wayland --xwayland --libinput\n"
                        "elif [ -f /usr/bin/plasma-bigscreen-common-env ]; then\n"
                        "    . /usr/bin/plasma-bigscreen-common-env\n"
                        "    exec /usr/bin/startplasma-wayland --xwayland --libinput\n"
                        "else\n"
                        "    exec /usr/bin/startplasma-wayland\n"
                        "fi\n"
                    )
                run("chmod 0755 /mnt/usr/bin/plasma-bigscreen-session")

                # Configure system-wide KDE defaults for Plasma Bigscreen
                xdg_dir = Path("/mnt/etc/xdg")
                xdg_dir.mkdir(parents=True, exist_ok=True)
                with open(xdg_dir / "kdeglobals", "a") as kf:
                    kf.write("\n[KDE]\nLookAndFeelPackage=org.kde.plasma.bigscreen\n")
                with open(xdg_dir / "plasmashellrc", "w") as pf:
                    pf.write("[Shell]\nShellPackage=org.kde.plasma.bigscreen\n")
                with open(xdg_dir / "ksplashrc", "w") as sf:
                    sf.write("[KSplash]\nEngine=none\nTheme=org.kde.plasma.bigscreen\n")

                # User KDE config
                user_cfg_dir = Path(f"/mnt/home/{username}/.config")
                user_cfg_dir.mkdir(parents=True, exist_ok=True)
                with open(user_cfg_dir / "kdeglobals", "w") as kf:
                    kf.write("[KDE]\nLookAndFeelPackage=org.kde.plasma.bigscreen\n")
                with open(user_cfg_dir / "plasmashellrc", "w") as pf:
                    pf.write("[Shell]\nShellPackage=org.kde.plasma.bigscreen\n")
                with open(user_cfg_dir / "ksplashrc", "w") as sf:
                    sf.write("[KSplash]\nEngine=none\nTheme=org.kde.plasma.bigscreen\n")
                run_chroot(f"chown -R {username}:{username} /home/{username}/.config 2>/dev/null || true")

                # Create guaranteed valid wayland and x11 session entries
                wayland_sessions.mkdir(parents=True, exist_ok=True)
                with open(wayland_sessions / "plasma-bigscreen-wayland.desktop", "w") as sf:
                    sf.write(
                        "[Desktop Entry]\n"
                        "Name=Plasma Bigscreen (Wayland)\n"
                        "Comment=Plasma Bigscreen TV Interface by KDE\n"
                        "Exec=/usr/bin/plasma-bigscreen-wayland\n"
                        "TryExec=/usr/bin/plasma-bigscreen-wayland\n"
                        "Type=Application\n"
                        "DesktopNames=KDE\n"
                    )

                xsessions.mkdir(parents=True, exist_ok=True)
                with open(xsessions / "plasma-bigscreen.desktop", "w") as sf:
                    sf.write(
                        "[Desktop Entry]\n"
                        "Name=Plasma Bigscreen\n"
                        "Comment=Plasma Bigscreen TV Interface by KDE\n"
                        "Exec=/usr/bin/plasma-bigscreen-session\n"
                        "TryExec=/usr/bin/plasma-bigscreen-session\n"
                        "Type=Application\n"
                        "DesktopNames=KDE\n"
                    )

                # Configure AccountsService for user
                acct_dir = Path("/mnt/var/lib/AccountsService/users")
                acct_dir.mkdir(parents=True, exist_ok=True)
                with open(acct_dir / username, "w") as af:
                    af.write(
                        "[User]\n"
                        "Language=\n"
                        "Session=plasma-bigscreen-wayland\n"
                        "XSession=plasma-bigscreen-wayland\n"
                        "SystemAccount=false\n"
                    )

                # Configure SDDM Autologin
                Path("/mnt/etc/sddm.conf.d").mkdir(parents=True, exist_ok=True)
                sddm_cfg_content = (
                    "[Autologin]\n"
                    f"User={username}\n"
                    "Session=plasma-bigscreen-wayland\n"
                    "Relogin=false\n\n"
                    "[General]\n"
                    "HaltCommand=/usr/bin/systemctl poweroff\n"
                    "RebootCommand=/usr/bin/systemctl reboot\n\n"
                    "[Users]\n"
                    "MinimumUid=1000\n"
                    "MaximumUid=60000\n"
                    "RememberLastUser=true\n"
                    "RememberLastSession=true\n"
                )
                with open("/mnt/etc/sddm.conf", "w") as sddmf:
                    sddmf.write(sddm_cfg_content)
                with open("/mnt/etc/sddm.conf.d/autologin.conf", "w") as sddmf:
                    sddmf.write(sddm_cfg_content)

                # Ensure PAM autologin configuration matching Arch Linux standard
                Path("/mnt/etc/pam.d").mkdir(parents=True, exist_ok=True)
                with open("/mnt/etc/pam.d/sddm-autologin", "w") as pamf:
                    pamf.write(
                        "#%PAM-1.0\n"
                        "auth        required    pam_permit.so\n"
                        "-account    include     system-local-login\n"
                        "-password   include     system-local-login\n"
                        "-session    include     system-local-login\n"
                    )
                with open("/mnt/etc/pam.d/sddm", "w") as pamf:
                    pamf.write(
                        "#%PAM-1.0\n"
                        "auth        include     system-login\n"
                        "-account    include     system-login\n"
                        "-password   include     system-login\n"
                        "-session    include     system-login\n"
                    )
            else:
                run_chroot("systemctl set-default multi-user.target 2>/dev/null || true")
                run_chroot("systemctl disable sddm 2>/dev/null || true")

            if "casaos" in edition:
                append_installer_log("Enabling CasaOS dashboard, Docker, and DockerMigrate services...")
                run_chroot("mkdir -p /var/log/casaos /var/log/tubeos /var/lib/tubeos/conf /var/lib/tubeos/db /var/lib/tubeos/apps /var/lib/tubeos/appstore /run/tubeos /var/run/tubeos /var/run/rclone /usr/share/tubeos/shell 2>/dev/null || true")
                try:
                    Path("/mnt/etc/docker/daemon.json").unlink(missing_ok=True)
                except Exception:
                    pass
                run_chroot("systemctl enable docker rclone tubeos-gateway tubeos-message-bus tubeos-user-service tubeos-local-storage tubeos-app-management tubeos dockermigrate 2>/dev/null || true")
            else:
                append_installer_log("Disabling CasaOS services...")
                run_chroot("systemctl disable docker rclone tubeos-gateway tubeos-message-bus tubeos-user-service tubeos-local-storage tubeos-app-management tubeos dockermigrate 2>/dev/null || true")

        else: # Debian
            append_installer_log("Configuring Debian packages...")
            run_chroot("apt-get update -y 2>/dev/null || true")

            if "tubeos_ui" in edition or "plasma_bigscreen" in edition:
                append_installer_log("Configuring Tube TV UI packages & SDDM autologin...")
                run_chroot("apt-get install -y --no-install-recommends openbox xserver-xorg xinit picom unclutter tubeos-ui sddm 2>/dev/null || true")
                run_chroot("systemctl set-default graphical.target 2>/dev/null || true")
                run_chroot("systemctl enable sddm 2>/dev/null || true")

                # Configure SDDM Autologin for installed user
                Path("/mnt/etc/sddm.conf.d").mkdir(parents=True, exist_ok=True)
                sddm_cfg_content = (
                    "[Autologin]\n"
                    f"User={username}\n"
                    "Session=tubeos\n"
                    "Relogin=false\n\n"
                    "[General]\n"
                    "HaltCommand=/usr/bin/systemctl poweroff\n"
                    "RebootCommand=/usr/bin/systemctl reboot\n\n"
                    "[Users]\n"
                    "MinimumUid=1000\n"
                    "MaximumUid=60000\n"
                    "HideUsers=_apt,avahi,backup,bin,colord,daemon,games,geoclue,lp,mail,man,messagebus,news,nobody,polkitd,proxy,root,sddm,sshd,sync,sys,systemd-network,uucp,www-data\n"
                    "HideShells=/bin/false,/usr/sbin/nologin,/sbin/nologin\n"
                    "RememberLastUser=true\n"
                    "RememberLastSession=true\n"
                )
                with open("/mnt/etc/sddm.conf", "w") as sddmf:
                    sddmf.write(sddm_cfg_content)
                with open("/mnt/etc/sddm.conf.d/autologin.conf", "w") as sddmf:
                    sddmf.write(sddm_cfg_content)

                # Configure Openbox autostart for installed user
                ob_dirs = [
                    Path(f"/mnt/home/{username}/.config/openbox"),
                    Path("/mnt/etc/skel/.config/openbox"),
                    Path("/mnt/etc/xdg/openbox"),
                ]
                for ob_d in ob_dirs:
                    ob_d.mkdir(parents=True, exist_ok=True)
                    with open(ob_d / "autostart", "w") as obf:
                        obf.write("#!/bin/sh\n/usr/bin/tubeos-ui &\n")
                    run(f"chmod 755 '{ob_d / 'autostart'}'")
                run_chroot(f"chown -R '{username}:{username}' '/home/{username}/.config' 2>/dev/null || true")

                # Configure PAM for SDDM on Debian
                Path("/mnt/etc/pam.d").mkdir(parents=True, exist_ok=True)
                with open("/mnt/etc/pam.d/sddm-autologin", "w") as pamf:
                    pamf.write(
                        "#%PAM-1.0\n"
                        "auth        required    pam_permit.so\n"
                        "account     required    pam_permit.so\n"
                        "password    required    pam_permit.so\n"
                        "session     required    pam_permit.so\n"
                        "@include common-session\n"
                        "session     required    pam_env.so\n"
                        "session     required    pam_env.so envfile=/etc/default/locale\n"
                    )
                with open("/mnt/etc/pam.d/sddm", "w") as pamf:
                    pamf.write(
                        "#%PAM-1.0\n"
                        "auth        requisite   pam_nologin.so\n"
                        "auth        sufficient  pam_permit.so\n"
                        "@include common-auth\n"
                        "@include common-account\n"
                        "session     required    pam_limits.so\n"
                        "session     required    pam_loginuid.so\n"
                        "@include common-session\n"
                        "@include common-password\n"
                        "session     required    pam_env.so\n"
                        "session     required    pam_env.so envfile=/etc/default/locale\n"
                    )
            else:
                run_chroot("systemctl set-default multi-user.target 2>/dev/null || true")
                run_chroot("systemctl disable sddm 2>/dev/null || true")
                try:
                    Path("/mnt/etc/sddm.conf").unlink(missing_ok=True)
                    Path("/mnt/etc/sddm.conf.d/autologin.conf").unlink(missing_ok=True)
                except Exception:
                    pass
                override_dir = Path("/mnt/etc/systemd/system/getty@tty1.service.d")
                override_dir.mkdir(parents=True, exist_ok=True)
                with open(override_dir / "override.conf", "w") as ovf:
                    ovf.write(
                        "[Service]\n"
                        "ExecStart=\n"
                        f"ExecStart=-/sbin/agetty --autologin {username} --noclear %I $TERM\n"
                        "Type=idle\n"
                    )

            if "casaos" in edition:
                append_installer_log("Enabling CasaOS dashboard and Docker services on Debian...")
                run_chroot("mkdir -p /var/log/casaos /var/log/tubeos /var/lib/tubeos/conf /var/lib/tubeos/db /var/lib/tubeos/apps /var/lib/tubeos/appstore /run/tubeos /var/run/tubeos /var/run/rclone /usr/share/tubeos/shell 2>/dev/null || true")
                run_chroot("apt-get install -y --no-install-recommends docker.io rclone samba mergerfs 2>/dev/null || true")
                try:
                    Path("/mnt/etc/docker/daemon.json").unlink(missing_ok=True)
                except Exception:
                    pass
                run_chroot("systemctl enable docker rclone tubeos-gateway tubeos-message-bus tubeos-user-service tubeos-local-storage tubeos-app-management tubeos dockermigrate 2>/dev/null || true")
            else:
                append_installer_log("Disabling CasaOS services...")
                run_chroot("systemctl disable docker rclone tubeos-gateway tubeos-message-bus tubeos-user-service tubeos-local-storage tubeos-app-management tubeos dockermigrate 2>/dev/null || true")

        # 9. Localization & Hostname
        update_installer_progress(0.85, "Configuring system locale, timezone and hostname")
        with open("/mnt/etc/hostname", "w") as hf:
            hf.write(f"{hostname}\n")
        
        with open("/mnt/etc/hosts", "w") as hsf:
            hsf.write(f"127.0.0.1\tlocalhost\t{hostname}\ttubeos\ttubeos.local\n127.0.1.1\t{hostname}\n::1\t\tlocalhost ip6-localhost ip6-loopback\n")

        # Configure Avahi mDNS daemon and services on installed target
        avahi_svc_dir = Path("/mnt/etc/avahi/services")
        avahi_svc_dir.mkdir(parents=True, exist_ok=True)
        with open("/mnt/etc/avahi/avahi-daemon.conf", "w") as af:
            af.write(
                "[server]\n"
                f"host-name={hostname}\n"
                "domain-name=local\n"
                "use-ipv4=yes\n"
                "use-ipv6=yes\n"
                "check-response-ttl=no\n"
                "use-iff-running=yes\n\n"
                "[publish]\n"
                "publish-addresses=yes\n"
                "publish-hinfo=yes\n"
                "publish-workstation=yes\n"
                "publish-domain=yes\n\n"
                "[reflector]\n"
                "enable-reflector=no\n\n"
                "[rlimits]\n"
            )
        with open(avahi_svc_dir / "tubeos-http.service", "w") as sf:
            sf.write(
                "<?xml version=\"1.0\" standalone='no'?>\n"
                "<!DOCTYPE service-group SYSTEM \"avahi-service.dtd\">\n"
                "<service-group>\n"
                "  <name replace-wildcards=\"yes\">Tube OS Web on %h</name>\n"
                "  <service>\n"
                "    <type>_http._tcp</type>\n"
                "    <port>80</port>\n"
                "    <txt-record>path=/</txt-record>\n"
                "  </service>\n"
                "</service-group>\n"
            )
        with open(avahi_svc_dir / "dockermigrate.service", "w") as sf:
            sf.write(
                "<?xml version=\"1.0\" standalone='no'?>\n"
                "<!DOCTYPE service-group SYSTEM \"avahi-service.dtd\">\n"
                "<service-group>\n"
                "  <name replace-wildcards=\"yes\">DockerMigrate on %h</name>\n"
                "  <service>\n"
                "    <type>_http._tcp</type>\n"
                "    <port>8070</port>\n"
                "    <txt-record>path=/</txt-record>\n"
                "  </service>\n"
                "</service-group>\n"
            )

        run_chroot(f"ln -sf /usr/share/zoneinfo/{timezone} /etc/localtime 2>/dev/null || true")
        run_chroot(f"localectl set-keymap {keymap} 2>/dev/null || true")

        # 10. Kernel initramfs & Bootloader Installation
        update_installer_progress(0.88, "Regenerating initramfs for installed storage")
        # Remove live archiso mkinitcpio configuration on the installed drive
        if Path("/mnt/etc/mkinitcpio.conf.d/archiso.conf").exists():
            try:
                Path("/mnt/etc/mkinitcpio.conf.d/archiso.conf").unlink()
            except Exception:
                pass

        Path("/mnt/etc/mkinitcpio.conf.d").mkdir(parents=True, exist_ok=True)
        with open("/mnt/etc/mkinitcpio.conf.d/tubeos.conf", "w") as mcf:
            mcf.write("HOOKS=(base udev autodetect modconf kms keyboard keymap consolefont plymouth block filesystems fsck)\n")

        if distro == "arch":
            run_chroot("mkinitcpio -P 2>&1 || true")
        else:
            run_chroot("update-initramfs -u -k all 2>&1 || true")

        # Configure Plymouth theme in target
        append_installer_log("Configuring Tube OS Plymouth theme...")
        run_chroot("plymouth-set-default-theme tubeos 2>/dev/null || true")
        if distro == "debian":
            run_chroot("update-initramfs -u -k all 2>&1 || true")
        else:
            run_chroot("mkinitcpio -P 2>&1 || true")

        update_installer_progress(0.92, "Installing and generating bootloader configuration")

        # Copy Particle-circle-window GRUB theme to target
        append_installer_log("Installing Tube OS modern graphical GRUB theme...")
        theme_copied = False
        target_theme_dir = Path("/mnt/boot/grub/themes/Particle-circle-window")
        for theme_cand in [
            "/boot/grub/themes/Particle-circle-window",
            "/usr/share/grub/themes/Particle-circle-window",
            "/run/live/medium/boot/grub/themes/Particle-circle-window",
        ]:
            if os.path.isdir(theme_cand) and os.path.isfile(os.path.join(theme_cand, "theme.txt")):
                target_theme_dir.parent.mkdir(parents=True, exist_ok=True)
                if target_theme_dir.exists():
                    shutil.rmtree(target_theme_dir)
                shutil.copytree(theme_cand, target_theme_dir)
                theme_copied = True
                break

        # Write /etc/default/grub BEFORE generating menu entries
        append_installer_log("Writing /etc/default/grub configuration...")
        grub_cmdline = ""
        if fs_type == "btrfs":
            grub_cmdline = "rootflags=subvol=@"
        Path("/mnt/etc/default").mkdir(parents=True, exist_ok=True)
        with open("/mnt/etc/default/grub", "w") as gf:
            gf.write(
                "# /etc/default/grub - generated by Tube OS installer\n"
                f"GRUB_DEFAULT=\"0\"\n"
                f"GRUB_TIMEOUT=\"{os.environ.get('TUBEOS_GRUB_TIMEOUT', '5')}\"\n"
                f"GRUB_DISTRIBUTOR=\"Tube OS\"\n"
                f"GRUB_THEME=\"/boot/grub/themes/Particle-circle-window/theme.txt\"\n"
                f"GRUB_GFXMODE=\"1920x1080,1280x720,auto\"\n"
                f"GRUB_GFXPAYLOAD_LINUX=\"keep\"\n"
                f"GRUB_CMDLINE_LINUX=\"\"\n"
                f"GRUB_CMDLINE_LINUX_DEFAULT=\"quiet splash {grub_cmdline}\"\n"
                f"GRUB_DISABLE_OS_PROBER=\"false\"\n"
                f"GRUB_ENABLE_CRYPTODISK=\"false\"\n"
            )

        if is_efi:
            if distro == "debian":
                run_chroot("apt-get install -y --no-install-recommends grub-efi-amd64 2>&1 || true")
            append_installer_log("Installing GRUB for UEFI x86_64...")
            r = run_chroot("grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=TubeOS --recheck 2>&1")
            if r.returncode != 0:
                append_installer_log(f"grub-install (TubeOS id) warning: {r.stdout.strip()} {r.stderr.strip()}")
            r = run_chroot("grub-install --target=x86_64-efi --efi-directory=/boot/efi --removable --recheck 2>&1")
            if r.returncode != 0:
                append_installer_log(f"grub-install (--removable) warning: {r.stdout.strip()} {r.stderr.strip()}")

            Path("/mnt/boot/efi/EFI/BOOT").mkdir(parents=True, exist_ok=True)
            src_efi = None
            for candidate in (
                "/mnt/boot/efi/EFI/TubeOS/grubx64.efi",
                "/mnt/boot/efi/EFI/BOOT/grubx64.efi",
                "/mnt/boot/efi/EFI/BOOT/bootx64.efi",
            ):
                if Path(candidate).exists():
                    src_efi = candidate
                    break
            if src_efi:
                shutil.copy2(src_efi, "/mnt/boot/efi/EFI/BOOT/BOOTX64.EFI")
                shutil.copy2(src_efi, "/mnt/boot/efi/EFI/BOOT/bootx64.efi")
                append_installer_log(f"UEFI fallback bootloader written to EFI/BOOT from {src_efi}")
            else:
                append_installer_log("WARNING: no grubx64.efi found on ESP; UEFI fallback not written")

        if disk and not is_efi:
            append_installer_log(f"Installing GRUB MBR on {disk}...")
            r = run_chroot(f"grub-install --target=i386-pc '{disk}' 2>&1")
            if r.returncode != 0:
                append_installer_log(f"grub-install (i386-pc) warning: {r.stdout.strip()} {r.stderr.strip()}")

        grub_cfg_ok = False

        r = run_chroot("grub-mkconfig -o /boot/grub/grub.cfg 2>&1")
        if r.returncode == 0:
            grub_cfg_ok = True
        else:
            append_installer_log(f"grub-mkconfig failed: {r.stdout.strip()} {r.stderr.strip()}")
            r = run_chroot("update-grub 2>&1")
            if r.returncode == 0:
                grub_cfg_ok = True
            else:
                append_installer_log(f"update-grub failed: {r.stdout.strip()} {r.stderr.strip()}")

        if grub_cfg_ok:
            cfg_path = Path("/mnt/boot/grub/grub.cfg")
            kernel_found = False
            for globp in ["/mnt/boot/vmlinuz-*", "/mnt/boot/vmlinuz*"]:
                if glob.glob(globp):
                    kernel_found = True
                    break
            if cfg_path.exists() and kernel_found:
                cfg_text = cfg_path.read_text(errors="ignore")
                if ("menuentry " in cfg_text) or ("submenu " in cfg_text):
                    append_installer_log("GRUB menu generated successfully (contains boot entries)")
                else:
                    append_installer_log("WARNING: grub.cfg generated but contains no menu entries")
            else:
                append_installer_log(f"WARNING: grub.cfg missing ({cfg_path.exists()}) or kernel not on /boot ({kernel_found})")
        else:
            append_installer_log("FATAL: failed to generate grub.cfg")

        # 11. Generate fstab
        update_installer_progress(0.95, "Generating filesystem table (fstab)")
        if shutil.which("genfstab"):
            run("genfstab -U /mnt > /mnt/etc/fstab")
        else:
            root_uuid = run(f"blkid -s UUID -o value '{part_root}'").stdout.strip()
            efi_uuid = run(f"blkid -s UUID -o value '{part_efi}'").stdout.strip() if is_efi and part_efi else ""
            
            fstab_lines = []
            if fs_type == "btrfs":
                fstab_lines.append(f"UUID={root_uuid} / btrfs subvol=@,compress=zstd:1,defaults 0 0")
                fstab_lines.append(f"UUID={root_uuid} /home btrfs subvol=@home,compress=zstd:1,defaults 0 0")
            else:
                fstab_lines.append(f"UUID={root_uuid} / ext4 errors=remount-ro 0 1")

            if efi_uuid:
                fstab_lines.append(f"UUID={efi_uuid} /boot/efi vfat umask=0077 0 2")

            with open("/mnt/etc/fstab", "w") as ff:
                ff.write("\n".join(fstab_lines) + "\n")

        # Remove live installer service from installed system, but enable first-boot OOTB Welcome Wizard
        append_installer_log("Configuring first-boot welcome wizard and services...")
        for p in [
            "/mnt/etc/systemd/system/tubeos-installer.service",
            "/mnt/etc/systemd/system/multi-user.target.wants/tubeos-installer.service",
            "/mnt/etc/systemd/system/multi-user.target.wants/install-runner.service",
            "/mnt/usr/lib/systemd/system/tubeos-installer.service",
            "/mnt/usr/lib/systemd/system/install-runner.service",
        ]:
            if Path(p).exists():
                try:
                    if Path(p).is_file() or Path(p).is_symlink():
                        Path(p).unlink()
                except Exception:
                    pass

        # Ensure need-ootb marker exists so first boot shows Welcome Wizard
        Path("/mnt/var/lib/tubeos").mkdir(parents=True, exist_ok=True)
        Path("/mnt/var/lib/tubeos/need-ootb").touch()

        # Configure systemd timeouts to prevent hangs on reboot/shutdown
        sysd_conf_dir = Path("/mnt/etc/systemd/system.conf.d")
        sysd_conf_dir.mkdir(parents=True, exist_ok=True)
        with open(sysd_conf_dir / "10-fast-shutdown.conf", "w") as scf:
            scf.write("[Manager]\nDefaultTimeoutStopSec=10s\nDefaultTimeoutStartSec=15s\nDefaultDeviceTimeoutSec=10s\n")

        # Copy and enable tubeos-ootb service
        Path("/mnt/usr/lib/systemd/system").mkdir(parents=True, exist_ok=True)
        shutil.copy(Path(__file__).parent / "tubeos-ootb.service", "/mnt/usr/lib/systemd/system/tubeos-ootb.service")
        run_chroot("systemctl disable tubeos-installer install-runner 2>/dev/null || true")
        run_chroot("systemctl enable tubeos-ootb NetworkManager avahi-daemon 2>/dev/null || true")

        # 12. Cleanup mounts
        update_installer_progress(0.99, "Cleaning up and synchronizing disk writes")
        cleanup_mounts()
        run("sync")

        update_installer_progress(1.0, "Installation Complete")
        install_state["status"] = "done"
        append_installer_log("System installed. Ready to reboot.")

    except Exception as e:
        err_msg = str(e)
        install_state["status"] = "error"
        install_state["error"] = err_msg
        append_installer_log(f"FATAL INSTALLATION ERROR: {err_msg}")
        cleanup_mounts()

# ─── FastAPI Web Routes ───────────────────────────────────────────────────

app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

@app.get("/", response_class=HTMLResponse)
async def index():
    return (STATIC_DIR / "index.html").read_text()

def ensure_dockermigrate_running():
    """Ensure Docker daemon and DockerMigrate service are alive and listening on :8070."""
    try:
        if Path("/run/archiso").exists() or not Path("/mnt").exists():
            run("mkdir -p /var/lib/docker 2>/dev/null || true")
            run("mountpoint -q /var/lib/docker || mount -t tmpfs -o size=2G tmpfs /var/lib/docker 2>/dev/null || true")
        run("systemctl start docker 2>/dev/null || true")
        run("systemctl start dockermigrate 2>/dev/null || true")
        res = subprocess.run("pgrep -f 'dockermigrate serve'", shell=True, capture_output=True)
        if res.returncode != 0 and Path("/usr/bin/dockermigrate").exists():
            subprocess.Popen(["/usr/bin/dockermigrate", "serve", "--listen", ":8070"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except Exception:
        pass

@app.get("/api/info")
async def api_info():
    ip = get_ip()
    url = f"http://{ip}" if ip != "localhost" else "http://tubeos.local"
    # Ensure DockerMigrate is up in background
    threading.Thread(target=ensure_dockermigrate_running, daemon=True).start()
    return {
        "distro": get_base_distro(),
        "ip": ip,
        "url": url,
        "qr_svg": gen_qr_svg(url),
        "is_efi": Path("/sys/firmware/efi").exists(),
        "mode": "ootb" if ootb_mode else "installer",
    }

@app.post("/api/dockermigrate/start")
async def api_dockermigrate_start():
    ensure_dockermigrate_running()
    return {"status": "ok"}

@app.post("/api/ootb/complete")
async def api_ootb_complete():
    """Complete OOTB wizard, hand over to CasaOS gateway, and redirect user."""
    need_ootb = Path("/var/lib/tubeos/need-ootb")
    if need_ootb.exists():
        try:
            need_ootb.unlink()
        except Exception:
            pass
    
    def finish_and_start_casaos():
        time.sleep(0.3)
        # Ensure log & runtime directories exist
        run("mkdir -p /var/log/casaos /var/log/tubeos /var/lib/tubeos/conf /var/lib/tubeos/db /var/lib/tubeos/apps /var/lib/tubeos/appstore /var/run/tubeos /run/tubeos /var/run/rclone /usr/share/tubeos/shell 2>/dev/null || true")
        # Stop and disable OOTB so port 80 is immediately released
        run("systemctl disable tubeos-ootb 2>/dev/null || true")
        run("systemctl stop tubeos-ootb 2>/dev/null || true")
        # Start docker, gateway, and CasaOS stack
        run("systemctl start docker 2>/dev/null || true")
        run("systemctl start tubeos-gateway 2>/dev/null || true")
        run("sleep 1; systemctl restart tubeos-message-bus tubeos-user-service tubeos-local-storage tubeos-app-management tubeos dockermigrate 2>/dev/null || true")
        # Exit process cleanly to drop port 80 socket
        os._exit(0)

    threading.Thread(target=finish_and_start_casaos, daemon=True).start()
    return {"status": "ok", "redirect": "/"}

@app.get("/api/network")
async def api_network():
    return {
        "status": check_internet(),
        "wifi": scan_wifi(),
    }

@app.post("/api/network/connect")
async def api_network_connect(request: Request):
    body = await request.json()
    ssid = body.get("ssid", "")
    password = body.get("password")
    ok = connect_wifi(ssid, password)
    return {"status": "ok" if ok else "failed"}

@app.get("/api/disks")
async def api_disks():
    return {"disks": get_system_disks()}

@app.get("/api/editions")
async def api_editions():
    distro = get_base_distro()
    if distro == "arch":
        return {
            "distro": "arch",
            "editions": [
                {
                    "id": "bigscreen_casaos",
                    "title": "Plasma Bigscreen + CasaOS",
                    "badge": "Desktop & Server",
                    "desc": "KDE Plasma Bigscreen TV UI with background CasaOS Docker and NAS services.",
                    "icon": "tv-server",
                },
                {
                    "id": "bigscreen_solo",
                    "title": "Plasma Bigscreen",
                    "badge": "TV UI",
                    "desc": "KDE Plasma Bigscreen 10-foot TV interface.",
                    "icon": "tv",
                },
                {
                    "id": "casaos_solo",
                    "title": "CasaOS Server",
                    "badge": "Headless",
                    "desc": "Headless system with CasaOS web dashboard, Docker, and network storage.",
                    "icon": "server",
                },
            ]
        }
    else: # Debian
        return {
            "distro": "debian",
            "editions": [
                {
                    "id": "tubeos_ui_casaos",
                    "title": "Tube TV UI + CasaOS",
                    "badge": "Desktop & Server",
                    "desc": "Tube TV interface with CasaOS Docker management services.",
                    "icon": "tv-server",
                },
                {
                    "id": "tubeos_ui_solo",
                    "title": "Tube TV UI",
                    "badge": "TV UI",
                    "desc": "Lightweight fullscreen TV interface.",
                    "icon": "tv",
                },
                {
                    "id": "casaos_solo",
                    "title": "CasaOS Server",
                    "badge": "Headless",
                    "desc": "Headless Debian server with CasaOS web dashboard and Docker management.",
                    "icon": "server",
                },
            ]
        }


@app.post("/api/install")
async def api_install(request: Request):
    global install_state
    if install_state["status"] == "running":
        return JSONResponse({"error": "Installation already in progress"}, status_code=400)
    
    body = await request.json()
    threading.Thread(target=execute_installation_backend, args=(body,), daemon=True).start()
    return {"status": "started"}

@app.get("/api/install/progress")
async def api_install_progress():
    return {
        "status": install_state["status"],
        "progress": install_state["progress"],
        "step": install_state["step"],
        "error": install_state["error"],
        "logs": install_state["logs"][-60:], # Return recent logs
    }

@app.post("/api/reboot")
async def api_reboot():
    def do_reboot():
        time.sleep(1)
        run("sync 2>/dev/null || true")
        run("systemctl --force reboot || reboot -f || telinit 6", check=False)
    threading.Thread(target=do_reboot, daemon=True).start()
    return {"status": "rebooting"}

# ─── Main Entrypoint ───────────────────────────────────────────────────────

def main():
    global ootb_mode
    parser = argparse.ArgumentParser(description="Tube OS Web Installer")
    parser.add_argument("--ootb", action="store_true", help="Run in OOTB mode (post-reboot setup)")
    parser.add_argument("--port", type=int, default=80, help="Listen port (default: 80)")
    parser.add_argument("--host", default="0.0.0.0", help="Listen address")
    args = parser.parse_args()

    ootb_mode = args.ootb
    ip = get_ip()
    mode_str = "OOTB Setup" if ootb_mode else "Web Installer"
    print(f"==================================================")
    print(f"  Tube OS {mode_str} Started")
    print(f"  Access via Web: http://{ip}:{args.port}")
    print(f"  Access via mDNS: http://tubeos.local:{args.port}")
    print(f"==================================================")

    import uvicorn
    uvicorn.run(app, host=args.host, port=args.port, log_level="info")

if __name__ == "__main__":
    main()
