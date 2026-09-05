#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Time Machine - Storage Destination Manager
Handles USB auto-detection/mounting (including raw/msdos/unpartitioned USBs), Samba/NAS CIFS, and Rclone.
"""

import os
import subprocess
import json
import shutil
import tempfile
from typing import Dict, List, Optional, Tuple, Any

class StorageManager:
    """Manages storage targets: USB external disks, Samba shares, and Rclone remotes."""

    BASE_MOUNT_DIR = "/run/media/pulsar-timemachine"

    # =========================================================================
    # 1. USB Storage Handling
    # =========================================================================

    @staticmethod
    def get_available_usb_disks() -> List[Dict[str, Any]]:
        """
        Scans system for removable or external USB drives and partitions.
        Returns a list of drive/partition descriptors.
        """
        devices = []
        try:
            cmd = ["lsblk", "-J", "-o", "NAME,PATH,LABEL,UUID,SIZE,FSTYPE,MOUNTPOINT,HOTPLUG,RM,MODEL,TRAN,TYPE"]
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
            data = json.loads(res.stdout)
            
            for root_dev in data.get("blockdevices", []):
                model = root_dev.get("model") or "USB Flash Drive"
                tran = root_dev.get("tran") or ""
                rm = root_dev.get("rm") in (True, 1, "1")
                hotplug = root_dev.get("hotplug") in (True, 1, "1")
                is_usb = tran == "usb" or rm or hotplug
                dev_name = root_dev.get("name", "")

                # Skip loop, zram, cdrom devices
                if dev_name.startswith("loop") or dev_name.startswith("zram") or dev_name.startswith("sr"):
                    continue

                if not is_usb and not dev_name.startswith("sd"):
                    continue

                # Exclude internal system drive if not removable
                root_mp = root_dev.get("mountpoint") or ""
                if root_mp in ("/", "/home", "/boot", "/boot/efi"):
                    continue

                children = root_dev.get("children", [])

                if children:
                    # Device has partitions
                    for child in children:
                        cmp = child.get("mountpoint") or ""
                        if cmp in ("/", "/home", "/boot", "/boot/efi", "[SWAP]"):
                            continue
                        
                        cfstype = (child.get("fstype") or "RAW / MSDOS").upper()
                        if cfstype == "SWAP":
                            continue

                        clabel = child.get("label") or child.get("name")
                        cuuid = child.get("uuid") or ""
                        csize = child.get("size") or ""
                        cpath = child.get("path") or f"/dev/{child.get('name')}"

                        is_raw = cfstype in ("RAW / MSDOS", "MSDOS / RAW", "UNKNOWN", "")
                        display = f"{clabel} ({csize} - {cfstype}) on {model}".strip()
                        if is_raw:
                            display += " [Needs Format]"

                        devices.append({
                            "name": child.get("name"),
                            "path": cpath,
                            "label": clabel,
                            "uuid": cuuid or cpath,
                            "size": csize,
                            "fstype": cfstype,
                            "mountpoint": cmp,
                            "model": model,
                            "display_name": display,
                            "needs_formatting": is_raw
                        })
                else:
                    # Whole disk (e.g. unpartitioned USB or superfloppy /dev/sda)
                    rfstype = (root_dev.get("fstype") or "MSDOS / RAW").upper()
                    rlabel = root_dev.get("label") or model
                    ruuid = root_dev.get("uuid") or ""
                    rsize = root_dev.get("size") or ""
                    rpath = root_dev.get("path") or f"/dev/{dev_name}"

                    is_raw = rfstype in ("RAW / MSDOS", "MSDOS / RAW", "UNKNOWN", "")
                    display = f"{rpath} - {model} ({rsize} {rfstype})".strip()
                    if is_raw:
                        display += " [Needs Format]"

                    devices.append({
                        "name": dev_name,
                        "path": rpath,
                        "label": rlabel,
                        "uuid": ruuid or rpath,
                        "size": rsize,
                        "fstype": rfstype,
                        "mountpoint": root_mp,
                        "model": model,
                        "display_name": display,
                        "needs_formatting": is_raw
                    })

        except Exception as e:
            print(f"[StorageManager] Error detecting USB disks: {e}")

        return devices

    @classmethod
    def format_usb_disk(cls, dev_path: str, fs_type: str = "ext4", label: str = "TIMEMACHINE") -> Tuple[bool, str]:
        """
        Safely formats a USB block device with a GPT partition table and chosen filesystem.
        """
        if not dev_path or not dev_path.startswith("/dev/"):
            return False, "Invalid block device path."

        # Safety check: Prevent formatting system drive
        try:
            cmd = ["lsblk", "-J", "-o", "PATH,MOUNTPOINT"]
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if res.returncode == 0:
                data = json.loads(res.stdout)
                for dev in data.get("blockdevices", []):
                    for part in ([dev] + dev.get("children", [])):
                        if part.get("path") == dev_path and part.get("mountpoint") in ("/", "/home", "/boot", "/boot/efi"):
                            return False, "Cannot format a mounted system drive."
        except Exception:
            pass

        # Unmount if mounted
        cls.unmount_usb(dev_path)

        # Base disk path if a partition was passed (e.g., /dev/sda1 -> /dev/sda)
        base_disk = dev_path.rstrip("0123456789")
        if base_disk.endswith("p") and "nvme" in base_disk:
            base_disk = base_disk[:-1]

        try:
            # Wipe existing signatures
            cmd_wipe = ["wipefs", "-a", base_disk]
            if os.geteuid() != 0:
                cmd_wipe = ["sudo"] + cmd_wipe
            subprocess.run(cmd_wipe, check=True)

            # Create GPT label and single primary partition
            cmd_part = ["parted", "-s", base_disk, "mklabel", "gpt", "mkpart", "primary", fs_type, "1MiB", "100%"]
            if os.geteuid() != 0:
                cmd_part = ["sudo"] + cmd_part
            subprocess.run(cmd_part, check=True)

            # Settle udev
            cmd_settle = ["udevadm", "settle"]
            if os.geteuid() != 0:
                cmd_settle = ["sudo"] + cmd_settle
            subprocess.run(cmd_settle, check=False)

            # Partition path
            part_path = f"{base_disk}1" if not base_disk[-1].isdigit() else f"{base_disk}p1"

            # Format filesystem
            if fs_type == "ext4":
                cmd_mkfs = ["mkfs.ext4", "-F", "-L", label, part_path]
            elif fs_type in ("vfat", "fat32"):
                cmd_mkfs = ["mkfs.vfat", "-F32", "-n", label[:11].upper(), part_path]
            elif fs_type == "exfat":
                cmd_mkfs = ["mkfs.exfat", "-n", label, part_path]
            elif fs_type == "btrfs":
                cmd_mkfs = ["mkfs.btrfs", "-f", "-L", label, part_path]
            else:
                cmd_mkfs = ["mkfs.ext4", "-F", "-L", label, part_path]

            if os.geteuid() != 0:
                cmd_mkfs = ["sudo"] + cmd_mkfs

            res = subprocess.run(cmd_mkfs, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if res.returncode == 0:
                subprocess.run(cmd_settle, check=False)
                # Ensure write permissions for non-root users
                tmp_mp = os.path.join(tempfile.gettempdir(), f"pulsar_fmt_{os.getpid()}")
                try:
                    os.makedirs(tmp_mp, exist_ok=True)
                    cmd_m = ["mount", part_path, tmp_mp]
                    if os.geteuid() != 0:
                        cmd_m = ["sudo"] + cmd_m
                    if subprocess.run(cmd_m).returncode == 0:
                        cmd_perm = ["chmod", "777", tmp_mp]
                        if os.geteuid() != 0:
                            cmd_perm = ["sudo"] + cmd_perm
                        subprocess.run(cmd_perm, check=False)
                        cmd_u = ["umount", tmp_mp]
                        if os.geteuid() != 0:
                            cmd_u = ["sudo"] + cmd_u
                        subprocess.run(cmd_u, check=False)
                except Exception:
                    pass
                finally:
                    shutil.rmtree(tmp_mp, ignore_errors=True)

                return True, f"Device {base_disk} successfully formatted as {fs_type.upper()} ({label})."
            else:
                return False, f"Formatting failed: {res.stderr}"
        except Exception as e:
            return False, f"Exception during formatting: {e}"

    @classmethod
    def get_base_mount_dir(cls) -> str:
        """Returns and ensures a safe writable directory for mountpoints."""
        if os.geteuid() == 0:
            base = cls.BASE_MOUNT_DIR
        else:
            uid = os.getuid()
            user_runtime = f"/run/user/{uid}"
            if os.path.exists(user_runtime):
                base = os.path.join(user_runtime, "pulsar-timemachine")
            else:
                base = "/tmp/pulsar-timemachine"
        os.makedirs(base, exist_ok=True)
        return base

    @classmethod
    def mount_usb(cls, path_or_uuid: str) -> Optional[str]:
        """
        Mounts a USB partition or device by path or UUID.
        Returns the active mountpoint directory path or None.
        """
        active_mp = None
        # First check if already mounted
        try:
            cmd = ["lsblk", "-J", "-o", "PATH,UUID,MOUNTPOINT"]
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if res.returncode == 0:
                data = json.loads(res.stdout)
                for dev in data.get("blockdevices", []):
                    for part in ([dev] + dev.get("children", [])):
                        if (part.get("path") == path_or_uuid or part.get("uuid") == path_or_uuid) and part.get("mountpoint"):
                            active_mp = part.get("mountpoint")
                            break
        except Exception:
            pass

        if not active_mp:
            # Try udisksctl first (user space)
            try:
                target = path_or_uuid if path_or_uuid.startswith("/dev/") else f"/dev/disk/by-uuid/{path_or_uuid}"
                res = subprocess.run(
                    ["udisksctl", "mount", "-b", target],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
                )
                if res.returncode == 0:
                    words = res.stdout.strip().split()
                    if "at" in words:
                        mp_index = words.index("at") + 1
                        if mp_index < len(words):
                            active_mp = words[mp_index].rstrip(".")
            except Exception as e:
                print(f"[StorageManager] udisksctl mount attempt failed: {e}")

        if not active_mp:
            # Fallback to direct mount
            try:
                base_dir = cls.get_base_mount_dir()
                mountpoint = os.path.join(base_dir, "usb_backup")
                os.makedirs(mountpoint, exist_ok=True)
                target = path_or_uuid if path_or_uuid.startswith("/dev/") else f"/dev/disk/by-uuid/{path_or_uuid}"
                
                cmd = ["mount", target, mountpoint]
                if os.geteuid() != 0:
                    cmd = ["sudo"] + cmd
                subprocess.run(cmd, check=True)
                active_mp = mountpoint
            except Exception as e:
                print(f"[StorageManager] Fallback mount failed: {e}")
                return None

        # Ensure active_mp is writable
        if active_mp and not os.access(active_mp, os.W_OK):
            try:
                cmd = ["chmod", "777", active_mp]
                if os.geteuid() != 0:
                    cmd = ["sudo"] + cmd
                subprocess.run(cmd, check=False)
            except Exception:
                pass

        return active_mp

    @classmethod
    def unmount_usb(cls, mountpoint: str) -> bool:
        """Unmounts a USB mountpoint safely."""
        if not mountpoint or not os.path.exists(mountpoint):
            return True
        try:
            subprocess.run(["udisksctl", "unmount", "--mount-point", mountpoint], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
            cmd = ["umount", "-l", mountpoint]
            if os.geteuid() != 0:
                cmd = ["sudo"] + cmd
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            return res.returncode == 0
        except Exception:
            return False

    # =========================================================================
    # 2. Samba / NAS CIFS Handling
    # =========================================================================

    @classmethod
    def mount_samba(cls, host: str, share: str, user: str = "", password: str = "", domain: str = "") -> Optional[str]:
        """
        Mounts a Samba/CIFS share at a dedicated mountpoint.
        Returns the mountpoint path or None on failure.
        """
        host = (host or "").strip()
        share = (share or "").strip()
        user = (user or "").strip()
        password = (password or "").strip()
        domain = (domain or "").strip()

        if not host or not share:
            return None

        share_clean = share.strip("/")
        unc_path = f"//{host}/{share_clean}"
        base_dir = cls.get_base_mount_dir()
        mountpoint = os.path.join(base_dir, f"samba_{host}_{share_clean}".replace("/", "_"))
        
        try:
            os.makedirs(mountpoint, exist_ok=True)
            
            opts = ["vers=3.0", "iocharset=utf8", "rw"]
            if os.geteuid() != 0:
                opts.extend([f"uid={os.getuid()}", f"gid={os.getgid()}", "file_mode=0770", "dir_mode=0770"])

            if user:
                opts.append(f"username={user}")
                if password:
                    opts.append(f"password={password}")
                if domain:
                    opts.append(f"domain={domain}")
            else:
                opts.append("guest")

            cmd = ["mount", "-t", "cifs", unc_path, mountpoint, "-o", ",".join(opts)]
            if os.geteuid() != 0:
                cmd = ["sudo"] + cmd
                
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if res.returncode == 0:
                return mountpoint
            else:
                print(f"[StorageManager] Samba mount error: {res.stderr}")
                return None
        except Exception as e:
            print(f"[StorageManager] Exception mounting Samba share {unc_path}: {e}")
            return None

    @classmethod
    def test_samba_connection(cls, host: str, share: str, user: str = "", password: str = "", domain: str = "") -> Tuple[bool, str]:
        """Tests connectivity and authentication to a Samba share."""
        if not host or not share:
            return False, "Host and share cannot be empty."
        
        if shutil.which("smbclient"):
            share_clean = share.strip("/")
            unc = f"//{host}/{share_clean}"
            cmd = ["smbclient", unc, "-N"]
            if user:
                cmd = ["smbclient", unc, "-U", f"{user}%{password}"]
                if domain:
                    cmd.extend(["-W", domain])
            cmd.extend(["-c", "ls"])
            try:
                res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=10)
                if res.returncode == 0:
                    return True, "Connection and authentication successful."
                else:
                    return False, f"Authentication failed: {res.stderr.strip() or res.stdout.strip()}"
            except Exception as e:
                return False, f"smbclient error: {e}"

        mp = cls.mount_samba(host, share, user, password, domain)
        if mp:
            cls.unmount_samba(mp)
            return True, "Samba share mounted and verified successfully."
        return False, "Unable to mount CIFS share. Verify IP, share name, and credentials."

    @classmethod
    def unmount_samba(cls, mountpoint: str) -> bool:
        """Unmounts a Samba CIFS mountpoint."""
        if not mountpoint or not os.path.exists(mountpoint):
            return True
        try:
            cmd = ["umount", "-l", mountpoint]
            if os.geteuid() != 0:
                cmd = ["sudo"] + cmd
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            return res.returncode == 0
        except Exception:
            return False

    # =========================================================================
    # 3. Rclone / Cloud Storage Handling
    # =========================================================================

    @staticmethod
    def get_rclone_config_path() -> str:
        """Gets the effective rclone config file path."""
        custom_system = "/etc/pulsaros/rclone.conf"
        if os.path.exists(custom_system):
            return custom_system
        user_conf = os.path.expanduser("~/.config/rclone/rclone.conf")
        return user_conf

    @classmethod
    def get_rclone_remotes(cls, custom_config: Optional[str] = None) -> List[Dict[str, str]]:
        """
        Lists all configured Rclone remotes.
        Returns a list of dicts: [{"name": "gdrive", "type": "drive"}, ...]
        """
        remotes = []
        if not shutil.which("rclone"):
            return remotes

        config_file = custom_config or cls.get_rclone_config_path()
        cmd = ["rclone", "listremotes", "--long"]
        if os.path.exists(config_file):
            cmd.extend(["--config", config_file])

        try:
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if res.returncode == 0:
                for line in res.stdout.strip().split("\n"):
                    if not line:
                        continue
                    parts = line.split(":")
                    if len(parts) >= 1:
                        name = parts[0].strip()
                        remote_type = parts[1].strip() if len(parts) > 1 else "cloud"
                        remotes.append({
                            "name": name,
                            "type": remote_type,
                            "display_name": f"{name} ({remote_type})"
                        })
        except Exception as e:
            print(f"[StorageManager] Error listing rclone remotes: {e}")

        return remotes

    @classmethod
    def test_rclone_remote(cls, remote_name: str, custom_config: Optional[str] = None) -> Tuple[bool, str]:
        """Tests if an Rclone remote is reachable and authenticated."""
        if not shutil.which("rclone"):
            return False, "rclone binary not found on system."

        config_file = custom_config or cls.get_rclone_config_path()
        remote_target = f"{remote_name.rstrip(':')}:"
        cmd = ["rclone", "lsd", remote_target, "--max-depth", "1"]
        if os.path.exists(config_file):
            cmd.extend(["--config", config_file])

        try:
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=15)
            if res.returncode == 0:
                return True, f"Successfully connected to cloud remote '{remote_name}'."
            else:
                return False, f"Cloud connection failed: {res.stderr.strip()}"
        except Exception as e:
            return False, f"Rclone error: {e}"

    @classmethod
    def import_rclone_config(cls, source_file: str, target_system: bool = False) -> Tuple[bool, str]:
        """Imports an rclone.conf file from USB or file picker."""
        if not os.path.exists(source_file):
            return False, f"File {source_file} does not exist."

        try:
            if target_system or os.geteuid() == 0:
                target_path = "/etc/pulsaros/rclone.conf"
            else:
                target_path = os.path.expanduser("~/.config/rclone/rclone.conf")
                
            os.makedirs(os.path.dirname(target_path), exist_ok=True)
            shutil.copyfile(source_file, target_path)
            os.chmod(target_path, 0o600)
            return True, f"Configuration successfully imported to {target_path}."
        except Exception as e:
            return False, f"Error importing rclone config: {e}"

    @classmethod
    def build_rclone_repo_url(cls, remote_name: str, subpath: str) -> str:
        """Builds a restic compatible rclone repo URL, e.g. rclone:gdrive:pulsaros-backup"""
        clean_remote = remote_name.rstrip(":")
        clean_path = subpath.strip("/")
        return f"rclone:{clean_remote}:{clean_path}"
