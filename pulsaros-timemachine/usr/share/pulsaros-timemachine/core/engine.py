#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Time Machine - Backup & Restore Orchestrator
Coordinates Btrfs snapshots, storage destinations (USB/NAS/Cloud), and Restic repository sync.
"""

import os
import sys
import time
import datetime
from typing import Dict, List, Optional, Tuple, Callable, Any

from .config_manager import ConfigManager
from .btrfs_manager import BtrfsManager
from .storage_manager import StorageManager
from .restic_manager import ResticManager
from .scheduler import SchedulerManager

class TimeMachineEngine:
    """Main Orchestration Engine for Pulsar OS Time Machine."""

    def __init__(self, config_manager: Optional[ConfigManager] = None):
        self.config_mgr = config_manager or ConfigManager()
        self.active_mountpoint: Optional[str] = None
        self.should_unmount: bool = False
        self.active_restic_manager: Optional[ResticManager] = None

    def resolve_repository_target(self) -> Tuple[Optional[str], Optional[str], Optional[str]]:
        """
        Resolves the repository target URL/Path, password, and rclone config.
        Returns: (repo_url, repo_password, rclone_config_path)
        """
        cfg = self.config_mgr.config
        dest_type = cfg.get("destination_type", "usb")
        password = cfg.get("repo_password", "")
        rclone_conf = cfg.get("rclone_config_file") or StorageManager.get_rclone_config_path()

        if dest_type == "usb":
            mount_path = cfg.get("usb_mount_path")
            uuid = cfg.get("usb_uuid")

            # Fallback auto-detection if no UUID is saved or current target is missing
            if not uuid and not mount_path:
                available = StorageManager.get_available_usb_disks()
                ready_disks = [d for d in available if not d.get("needs_formatting")]
                if ready_disks:
                    uuid = ready_disks[0].get("uuid") or ready_disks[0].get("path")
                    self.config_mgr.set("usb_uuid", uuid)
                    self.config_mgr.set("usb_label", ready_disks[0].get("label", ""))
                    self.config_mgr.save()

            if not mount_path and uuid:
                mount_path = StorageManager.mount_usb(uuid)
                self.active_mountpoint = mount_path
                self.should_unmount = True
            elif mount_path and not os.path.exists(mount_path):
                if uuid:
                    mount_path = StorageManager.mount_usb(uuid)
                    self.active_mountpoint = mount_path
                    self.should_unmount = True

            if not mount_path or not os.path.exists(mount_path):
                # Try auto-detecting again from available disks
                available = StorageManager.get_available_usb_disks()
                ready_disks = [d for d in available if not d.get("needs_formatting")]
                if ready_disks:
                    uuid = ready_disks[0].get("uuid") or ready_disks[0].get("path")
                    mount_path = StorageManager.mount_usb(uuid)
                    if mount_path:
                        self.active_mountpoint = mount_path
                        self.should_unmount = True
                        self.config_mgr.set("usb_uuid", uuid)
                        self.config_mgr.set("usb_label", ready_disks[0].get("label", ""))
                        self.config_mgr.save()

            if not mount_path or not os.path.exists(mount_path):
                return None, password, None

            subpath = cfg.get("usb_repo_subpath", "pulsaros-timemachine-backup").strip("/")
            repo_url = os.path.join(mount_path, subpath)
            return repo_url, password, None

        elif dest_type == "samba":
            host = cfg.get("samba_host", "")
            share = cfg.get("samba_share", "")
            user = cfg.get("samba_user", "")
            passw = cfg.get("samba_pass", "")
            domain = cfg.get("samba_domain", "")
            subpath = cfg.get("samba_subpath", "pulsaros-timemachine-backup").strip("/")

            mp = StorageManager.mount_samba(host, share, user, passw, domain)
            if not mp:
                return None, password, None
                
            self.active_mountpoint = mp
            self.should_unmount = True
            repo_url = os.path.join(mp, subpath)
            return repo_url, password, None

        elif dest_type == "rclone":
            remote = cfg.get("rclone_remote", "")
            path = cfg.get("rclone_path", "pulsaros-timemachine-backup")
            if not remote:
                return None, password, None
            repo_url = StorageManager.build_rclone_repo_url(remote, path)
            return repo_url, password, rclone_conf

        return None, password, None

    def cleanup_destination(self) -> None:
        """Unmounts active storage mounts after operations complete."""
        if self.should_unmount and self.active_mountpoint:
            dest_type = self.config_mgr.get("destination_type")
            if dest_type == "usb":
                StorageManager.unmount_usb(self.active_mountpoint)
            elif dest_type == "samba":
                StorageManager.unmount_samba(self.active_mountpoint)
            self.active_mountpoint = None
            self.should_unmount = False

    def cancel_active_backup(self) -> bool:
        """Requests cancellation of running backup."""
        if self.active_restic_manager:
            return self.active_restic_manager.cancel_backup()
        return False

    def perform_backup(
        self,
        source_override: Optional[str] = None,
        custom_path_override: Optional[str] = None,
        excludes_override: Optional[List[str]] = None,
        progress_cb: Optional[Callable[[Dict[str, Any]], None]] = None,
        log_cb: Optional[Callable[[str], None]] = None
    ) -> Tuple[bool, str]:
        """
        Executes a complete Time Machine backup cycle:
        1. Resolve storage destination.
        2. Create atomic Btrfs snapshot(s) or resolve custom path.
        3. Backup to Restic repo with exclude filters applied.
        4. Cleanup snapshots and prune repository according to retention count.
        """
        def log(msg: str):
            if log_cb:
                log_cb(msg)
            print(f"[TimeMachine] {msg}")

        log("Initializing Time Machine backup operation...")
        source = source_override or self.config_mgr.get("source", "both")

        repo_url, password, rclone_conf = self.resolve_repository_target()
        if not repo_url:
            err = "Unable to connect or mount backup destination. Verify storage connection."
            log(f"ERROR: {err}")
            self.config_mgr.update_last_backup("failed", error=err)
            self.cleanup_destination()
            return False, err

        restic = ResticManager(repo_url, password, rclone_conf)
        self.active_restic_manager = restic

        # Ensure repository is initialized
        if not restic.is_repo_initialized():
            log(f"Initializing new Restic repository at {repo_url}...")
            ok, init_msg = restic.init_repo()
            if not ok:
                log(f"ERROR: {init_msg}")
                self.config_mgr.update_last_backup("failed", error=init_msg)
                self.cleanup_destination()
                self.active_restic_manager = None
                return False, init_msg

        snapshots = {}
        paths_to_backup = []
        tags = ["pulsaros", f"source-{source}"]

        if source == "custom":
            cpath = custom_path_override or self.config_mgr.get("custom_path", "")
            if not cpath or not os.path.exists(cpath):
                err = f"Custom directory '{cpath}' not found or invalid."
                log(f"ERROR: {err}")
                self.config_mgr.update_last_backup("failed", error=err)
                self.cleanup_destination()
                self.active_restic_manager = None
                return False, err
            log(f"Backing up custom directory: '{cpath}'...")
            paths_to_backup = [cpath]
            tags.append(f"path-{os.path.basename(cpath)}")
        else:
            # 1. Create Btrfs snapshots
            log(f"Creating read-only Btrfs snapshot(s) for source: '{source}'...")
            snapshots = BtrfsManager.create_snapshots(source)
            
            # Fallback if btrfs snapshots not available (e.g. non-btrfs test or dev env)
            if snapshots:
                for stype, spath in snapshots.items():
                    paths_to_backup.append(spath)
                    tags.append(f"subvol-{stype}")
            else:
                log("Note: Direct paths used (Btrfs subvolume snapshot fallback).")
                if source in ("both", "root"):
                    paths_to_backup.append("/")
                if source in ("both", "home"):
                    paths_to_backup.append("/home")

        excludes = excludes_override if excludes_override is not None else self.config_mgr.get("exclude_patterns", [])
        if excludes:
            log(f"Applying {len(excludes)} exclude pattern(s)...")

        # 2. Run Restic Backup
        log("Streaming data to encrypted restic repository...")
        ok, backup_msg, stats = restic.run_backup(
            paths_to_backup=paths_to_backup,
            tags=tags,
            excludes=excludes,
            progress_callback=progress_cb,
            log_callback=log
        )

        # 3. Clean temporary Btrfs snapshots
        if snapshots:
            log("Cleaning temporary Btrfs staging snapshots...")
            for spath in snapshots.values():
                BtrfsManager.delete_snapshot(spath)

        if ok:
            # 4. Apply retention policy
            retention_count = self.config_mgr.get("retention_count", 10)
            log(f"Applying retention policy (keeping last {retention_count} snapshots)...")
            restic.forget_and_prune(keep_last=retention_count, log_callback=log)
            BtrfsManager.prune_local_snapshots(keep_count=retention_count)

            snap_id = stats.get("snapshot_id", "")
            bytes_total = stats.get("total_bytes_processed", 0) / (1024 * 1024)
            size_str = f"{bytes_total:.1f} MB" if bytes_total > 0 else "N/A"
            self.config_mgr.update_last_backup("success", snapshot_id=snap_id, size=size_str)
            log("Time Machine backup completed successfully.")
        else:
            self.config_mgr.update_last_backup("failed", error=backup_msg)
            log(f"Backup status: {backup_msg}")

        self.cleanup_destination()
        self.active_restic_manager = None
        return ok, backup_msg

    def get_snapshots_list(self) -> List[Dict[str, Any]]:
        """Retrieves all snapshots available in the current destination."""
        repo_url, password, rclone_conf = self.resolve_repository_target()
        if not repo_url:
            self.cleanup_destination()
            return []

        restic = ResticManager(repo_url, password, rclone_conf)
        snaps = restic.list_snapshots()
        self.cleanup_destination()
        return snaps

    def delete_backup(self, snapshot_id: str, log_cb: Optional[Callable[[str], None]] = None) -> Tuple[bool, str]:
        """Manually deletes a single snapshot from the repository."""
        repo_url, password, rclone_conf = self.resolve_repository_target()
        if not repo_url:
            self.cleanup_destination()
            return False, "Could not access backup repository."

        restic = ResticManager(repo_url, password, rclone_conf)
        ok, msg = restic.delete_snapshot(snapshot_id, log_callback=log_cb)
        self.cleanup_destination()
        return ok, msg

    def perform_restore(
        self,
        snapshot_id: str,
        target_dir: str,
        include_path: Optional[str] = None,
        log_cb: Optional[Callable[[str], None]] = None
    ) -> Tuple[bool, str]:
        """Restores a snapshot to a given destination."""
        repo_url, password, rclone_conf = self.resolve_repository_target()
        if not repo_url:
            self.cleanup_destination()
            return False, "Could not access backup repository."

        restic = ResticManager(repo_url, password, rclone_conf)
        ok, msg = restic.restore_snapshot(
            snapshot_id=snapshot_id,
            target_dir=target_dir,
            include_path=include_path,
            log_callback=log_cb
        )
        self.cleanup_destination()
        return ok, msg
