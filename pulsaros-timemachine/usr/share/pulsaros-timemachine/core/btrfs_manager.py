#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Time Machine - Btrfs Snapshot Manager
Handles Btrfs subvolume detection, atomic read-only snapshot creation, and local snapshot cleanup.
"""

import os
import subprocess
import datetime
import re
from typing import Dict, List, Optional, Tuple, Any

class BtrfsManager:
    """Manages Btrfs snapshots for root (@) and home (@home) subvolumes."""

    SNAPSHOT_BASE_DIR = "/.snapshots"
    SNAPSHOT_PREFIX = "pulsar-timemachine"

    @staticmethod
    def is_btrfs(path: str = "/") -> bool:
        """Checks if a given path is located on a Btrfs filesystem."""
        try:
            res = subprocess.run(
                ["stat", "-f", "-c", "%T", path],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=True
            )
            return res.stdout.strip().lower() == "btrfs"
        except Exception:
            return False

    @staticmethod
    def get_subvolume_mounts() -> Dict[str, str]:
        """
        Returns a map of standard subvolume mountpoints.
        e.g. {"root": "/", "home": "/home"}
        """
        mounts = {}
        try:
            with open("/proc/mounts", "r") as f:
                for line in f:
                    parts = line.split()
                    if len(parts) >= 4 and parts[2] == "btrfs":
                        mp = parts[1]
                        opts = parts[3]
                        if mp == "/":
                            mounts["root"] = "/"
                        elif mp == "/home":
                            mounts["home"] = "/home"
        except Exception as e:
            print(f"[BtrfsManager] Error reading /proc/mounts: {e}")
            
        # Fallback if both are in root
        if "root" not in mounts and BtrfsManager.is_btrfs("/"):
            mounts["root"] = "/"
        if "home" not in mounts and BtrfsManager.is_btrfs("/home"):
            mounts["home"] = "/home"
            
        return mounts

    @classmethod
    def discover_restore_targets(cls) -> Dict[str, str]:
        """
        Discovers possible restore targets for system recovery:
        - Running OS paths: {"Running System Root (/)": "/", "Running User Home (/home)": "/home"}
        - Recovery mode paths (when booted into live recovery):
          Mounts top-level Btrfs pool and exposes @ and @home directly:
          {"Recovery Target: System Root (@)": "/run/media/pulsar_btrfs_pool/@", "Recovery Target: User Home (@home)": "/run/media/pulsar_btrfs_pool/@home"}
        """
        targets = {}
        # 1. Normal running system
        mounts = cls.get_subvolume_mounts()
        if "root" in mounts:
            targets["Running System Root (/)"] = mounts["root"]
        if "home" in mounts:
            targets["Running User Home (/home)"] = mounts["home"]

        # 2. Recovery Environment: Check for Btrfs pools containing @ and @home
        try:
            import json
            cmd = ["lsblk", "-J", "-o", "NAME,PATH,LABEL,UUID,FSTYPE,MOUNTPOINT"]
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if res.returncode == 0:
                data = json.loads(res.stdout)
                for dev in data.get("blockdevices", []):
                    for part in ([dev] + dev.get("children", [])):
                        if part.get("fstype") == "btrfs":
                            bpath = part.get("path") or f"/dev/{part.get('name')}"
                            pool_mp = "/run/media/pulsar_btrfs_pool"
                            try:
                                os.makedirs(pool_mp, exist_ok=True)
                            except Exception:
                                subprocess.run(["sudo", "mkdir", "-p", pool_mp], check=False)

                            cmd_m = ["mount", "-o", "subvolid=5", bpath, pool_mp]
                            if os.geteuid() != 0:
                                cmd_m = ["sudo"] + cmd_m
                            subprocess.run(cmd_m, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)

                            at_root = os.path.join(pool_mp, "@")
                            at_home = os.path.join(pool_mp, "@home")
                            if os.path.exists(at_root):
                                targets["Recovery Target: System Root (@)"] = at_root
                            if os.path.exists(at_home):
                                targets["Recovery Target: User Home (@home)"] = at_home
        except Exception as e:
            print(f"[BtrfsManager] Notice discovering recovery restore targets: {e}")

        return targets

    @classmethod
    def ensure_snapshot_dir(cls) -> str:
        """Ensures the snapshot holding directory exists."""
        snap_dir = cls.SNAPSHOT_BASE_DIR
        try:
            if not os.path.exists(snap_dir):
                subprocess.run(["sudo", "mkdir", "-p", snap_dir], check=False)
                # If still not existing, try local fallback
                if not os.path.exists(snap_dir):
                    os.makedirs(snap_dir, exist_ok=True)
            return snap_dir
        except Exception:
            fallback = "/tmp/.pulsar-snapshots"
            os.makedirs(fallback, exist_ok=True)
            return fallback

    @classmethod
    def create_snapshots(cls, target_types: str = "both") -> Dict[str, str]:
        """
        Creates read-only snapshots of the requested subvolumes.
        target_types: "both" (@ and @home), "root" (@), or "home" (@home)
        Returns a dictionary of {"root": "/.snapshots/...", "home": "/.snapshots/..."}
        """
        cls.ensure_snapshot_dir()
        timestamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
        created = {}

        mounts = cls.get_subvolume_mounts()
        
        # 1. Root snapshot (@)
        if target_types in ("both", "root") and "root" in mounts:
            root_snap = os.path.join(cls.SNAPSHOT_BASE_DIR, f"{cls.SNAPSHOT_PREFIX}-root-{timestamp}")
            cmd = ["btrfs", "subvolume", "snapshot", "-r", mounts["root"], root_snap]
            if os.geteuid() != 0:
                cmd = ["sudo"] + cmd
            try:
                res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                if res.returncode == 0:
                    created["root"] = root_snap
                else:
                    print(f"[BtrfsManager] Warning creating root snapshot: {res.stderr}")
            except Exception as e:
                print(f"[BtrfsManager] Error running snapshot for root: {e}")

        # 2. Home snapshot (@home)
        if target_types in ("both", "home") and "home" in mounts:
            home_snap = os.path.join(cls.SNAPSHOT_BASE_DIR, f"{cls.SNAPSHOT_PREFIX}-home-{timestamp}")
            cmd = ["btrfs", "subvolume", "snapshot", "-r", mounts["home"], home_snap]
            if os.geteuid() != 0:
                cmd = ["sudo"] + cmd
            try:
                res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                if res.returncode == 0:
                    created["home"] = home_snap
                else:
                    print(f"[BtrfsManager] Warning creating home snapshot: {res.stderr}")
            except Exception as e:
                print(f"[BtrfsManager] Error running snapshot for home: {e}")

        return created

    @classmethod
    def delete_snapshot(cls, snapshot_path: str) -> bool:
        """Deletes a Btrfs subvolume snapshot."""
        if not os.path.exists(snapshot_path):
            return True
        cmd = ["btrfs", "subvolume", "delete", snapshot_path]
        if os.geteuid() != 0:
            cmd = ["sudo"] + cmd
        try:
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            return res.returncode == 0
        except Exception as e:
            print(f"[BtrfsManager] Error deleting snapshot {snapshot_path}: {e}")
            return False

    @classmethod
    def list_local_snapshots(cls) -> List[Dict[str, Any]]:
        """Lists all local Time Machine snapshots found in /.snapshots."""
        results = []
        if not os.path.exists(cls.SNAPSHOT_BASE_DIR):
            return results

        try:
            for entry in os.listdir(cls.SNAPSHOT_BASE_DIR):
                if entry.startswith(cls.SNAPSHOT_PREFIX):
                    full_path = os.path.join(cls.SNAPSHOT_BASE_DIR, entry)
                    subvol_type = "root" if "-root-" in entry else ("home" if "-home-" in entry else "unknown")
                    time_match = re.search(r"(\d{8}-\d{6})", entry)
                    timestamp_str = time_match.group(1) if time_match else "unknown"
                    results.append({
                        "path": full_path,
                        "name": entry,
                        "type": subvol_type,
                        "timestamp": timestamp_str
                    })
        except Exception as e:
            print(f"[BtrfsManager] Error listing snapshots: {e}")
        return sorted(results, key=lambda x: x["name"], reverse=True)

    @classmethod
    def prune_local_snapshots(cls, keep_count: int = 5) -> int:
        """Keeps only the latest keep_count local snapshots and deletes older ones."""
        snaps = cls.list_local_snapshots()
        root_snaps = [s for s in snaps if s["type"] == "root"]
        home_snaps = [s for s in snaps if s["type"] == "home"]
        
        deleted_count = 0
        for collection in (root_snaps, home_snaps):
            if len(collection) > keep_count:
                to_delete = collection[keep_count:]
                for snap in to_delete:
                    if cls.delete_snapshot(snap["path"]):
                        deleted_count += 1
        return deleted_count
