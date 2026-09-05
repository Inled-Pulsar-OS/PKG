#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Time Machine - Configuration Manager
Handles persistent settings, credentials, destinations, and scheduling preferences.
"""

import os
import json
import uuid
import stat
from typing import Dict, Any, Optional

DEFAULT_CONFIG: Dict[str, Any] = {
    "version": 1,
    "source": "both",  # "both" (@ and @home), "root" (@), "home" (@home), "custom"
    "custom_path": "", # Custom directory when source == "custom"
    "exclude_patterns": [
        "**/.cache/*",
        "**/node_modules/*",
        "**/tmp/*",
        "**/.local/share/Trash/*",
        "**/ISO/build/*"
    ],
    "destination_type": "usb",  # "usb", "samba", "rclone"
    
    # USB Storage Destination
    "usb_uuid": "",
    "usb_label": "",
    "usb_mount_path": "",
    "usb_repo_subpath": "pulsaros-timemachine-backup",
    
    # Samba / NAS Destination
    "samba_host": "",
    "samba_share": "",
    "samba_user": "",
    "samba_pass": "",
    "samba_domain": "",
    "samba_subpath": "pulsaros-timemachine-backup",
    
    # Rclone Cloud Destination
    "rclone_remote": "",
    "rclone_path": "pulsaros-timemachine-backup",
    "rclone_config_file": "",
    
    # Encryption & Security
    "repo_password": "",
    "auto_unlock": True,
    
    # Scheduling & Retention
    "schedule_enabled": True,
    "schedule_frequency": "daily",  # "hourly", "daily", "weekly", "monthly", "manual"
    "retention_count": 10,  # Number of recent backups to keep
    
    # Status Metadata
    "last_backup_time": "",
    "last_backup_status": "never",  # "success", "failed", "never"
    "last_backup_snapshot_id": "",
    "last_backup_size": "",
    "last_error_message": "",
}

class ConfigManager:
    """Manages reading and writing Time Machine configuration."""

    def __init__(self, custom_path: Optional[str] = None):
        self.is_custom_path = bool(custom_path)
        if custom_path:
            self.config_path = custom_path
        else:
            if os.geteuid() == 0 or (os.path.exists("/etc/pulsaros/timemachine.json") and os.access("/etc/pulsaros/timemachine.json", os.W_OK)):
                self.config_path = "/etc/pulsaros/timemachine.json"
            else:
                user_home = os.path.expanduser("~")
                self.config_path = os.path.join(user_home, ".config", "pulsaros", "timemachine.json")
                
        self.config: Dict[str, Any] = DEFAULT_CONFIG.copy()
        self.load()

    def load(self) -> Dict[str, Any]:
        """Loads configuration from file or creates defaults."""
        paths_to_try = [self.config_path]
        if not self.is_custom_path:
            user_cfg = os.path.join(os.path.expanduser("~"), ".config", "pulsaros", "timemachine.json")
            sys_cfg = "/etc/pulsaros/timemachine.json"
            for p in (user_cfg, sys_cfg):
                if p not in paths_to_try and os.path.exists(p):
                    paths_to_try.append(p)

        loaded = False
        for p in paths_to_try:
            if os.path.exists(p) and os.access(p, os.R_OK):
                try:
                    with open(p, "r", encoding="utf-8") as f:
                        data = json.load(f)
                        self.config = {**DEFAULT_CONFIG, **data}
                        loaded = True
                        break
                except Exception:
                    pass

        if not loaded:
            self.config = DEFAULT_CONFIG.copy()

        if not self.config.get("repo_password"):
            self.config["repo_password"] = str(uuid.uuid4().hex[:16])
        return self.config

    def save(self) -> bool:
        """Saves current configuration to file with restricted permissions."""
        paths_to_attempt = [self.config_path]
        user_cfg = os.path.join(os.path.expanduser("~"), ".config", "pulsaros", "timemachine.json")
        if user_cfg not in paths_to_attempt:
            paths_to_attempt.append(user_cfg)

        for target_path in paths_to_attempt:
            try:
                config_dir = os.path.dirname(target_path)
                os.makedirs(config_dir, exist_ok=True)
                
                temp_file = f"{target_path}.tmp.{os.getpid()}"
                with open(temp_file, "w", encoding="utf-8") as f:
                    json.dump(self.config, f, indent=4, ensure_ascii=False)
                    
                try:
                    os.chmod(temp_file, stat.S_IRUSR | stat.S_IWUSR | stat.S_IRGRP | stat.S_IWGRP)
                except Exception:
                    pass
                    
                os.replace(temp_file, target_path)
                self.config_path = target_path
                return True
            except Exception as e:
                print(f"[ConfigManager] Notice: Could not save to {target_path}: {e}")
        return False

    def get(self, key: str, default: Any = None) -> Any:
        return self.config.get(key, default)

    def set(self, key: str, value: Any) -> None:
        self.config[key] = value

    def update(self, new_data: Dict[str, Any]) -> None:
        self.config.update(new_data)
        self.save()

    def update_last_backup(self, status: str, snapshot_id: str = "", size: str = "", error: str = "") -> None:
        import datetime
        self.config["last_backup_time"] = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        self.config["last_backup_status"] = status
        self.config["last_backup_snapshot_id"] = snapshot_id
        self.config["last_backup_size"] = size
        self.config["last_error_message"] = error
        self.save()
