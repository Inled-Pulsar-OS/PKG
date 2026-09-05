#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Unit and Integration Tests for Pulsar OS Time Machine
"""

import os
import sys
import unittest
import tempfile
import json
from unittest.mock import patch, MagicMock

# Add module to sys.path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "usr", "share", "pulsaros-timemachine"))

from core.config_manager import ConfigManager
from core.btrfs_manager import BtrfsManager
from core.storage_manager import StorageManager
from core.restic_manager import ResticManager
from core.scheduler import SchedulerManager, CALENDAR_MAP
from core.engine import TimeMachineEngine

class TestConfigManager(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.config_file = os.path.join(self.temp_dir, "test_config.json")
        self.mgr = ConfigManager(custom_path=self.config_file)

    def test_default_values(self):
        self.assertEqual(self.mgr.get("source"), "both")
        self.assertEqual(self.mgr.get("destination_type"), "usb")
        self.assertEqual(self.mgr.get("retention_count"), 10)
        self.assertTrue(len(self.mgr.get("repo_password")) > 0)

    def test_save_and_reload(self):
        self.mgr.set("source", "home")
        self.mgr.set("destination_type", "samba")
        self.mgr.set("samba_host", "192.168.1.100")
        self.mgr.set("samba_share", "backups")
        self.mgr.save()

        # Reload fresh instance
        new_mgr = ConfigManager(custom_path=self.config_file)
        self.assertEqual(new_mgr.get("source"), "home")
        self.assertEqual(new_mgr.get("destination_type"), "samba")
        self.assertEqual(new_mgr.get("samba_host"), "192.168.1.100")
        self.assertEqual(new_mgr.get("samba_share"), "backups")

    def test_update_last_backup(self):
        self.mgr.update_last_backup("success", snapshot_id="abc12345", size="125.4 MB")
        self.assertEqual(self.mgr.get("last_backup_status"), "success")
        self.assertEqual(self.mgr.get("last_backup_snapshot_id"), "abc12345")
        self.assertEqual(self.mgr.get("last_backup_size"), "125.4 MB")
        self.assertTrue(len(self.mgr.get("last_backup_time")) > 0)


class TestBtrfsManager(unittest.TestCase):
    def test_btrfs_check(self):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout="btrfs\n", returncode=0)
            self.assertTrue(BtrfsManager.is_btrfs("/"))

            mock_run.return_value = MagicMock(stdout="ext4\n", returncode=0)
            self.assertFalse(BtrfsManager.is_btrfs("/"))

    def test_local_snapshot_listing(self):
        temp_snap_dir = tempfile.mkdtemp()
        BtrfsManager.SNAPSHOT_BASE_DIR = temp_snap_dir
        
        # Create dummy snapshots
        os.makedirs(os.path.join(temp_snap_dir, "pulsar-timemachine-root-20260905-120000"))
        os.makedirs(os.path.join(temp_snap_dir, "pulsar-timemachine-home-20260905-120000"))
        
        snaps = BtrfsManager.list_local_snapshots()
        self.assertEqual(len(snaps), 2)
        types = [s["type"] for s in snaps]
        self.assertIn("root", types)
        self.assertIn("home", types)


class TestStorageManager(unittest.TestCase):
    def test_build_rclone_url(self):
        url = StorageManager.build_rclone_repo_url("gdrive:", "/my-pulsar-backups/")
        self.assertEqual(url, "rclone:gdrive:my-pulsar-backups")

    def test_rclone_remotes_parsing(self):
        with patch("shutil.which", return_value="/usr/bin/rclone"), \
             patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                stdout="gdrive: drive\nonedrive: onedrive\ns3_backup: s3\n",
                returncode=0
            )
            remotes = StorageManager.get_rclone_remotes()
            self.assertEqual(len(remotes), 3)
            self.assertEqual(remotes[0]["name"], "gdrive")
            self.assertEqual(remotes[0]["type"], "drive")
            self.assertEqual(remotes[2]["name"], "s3_backup")

    def test_usb_scanning(self):
        mock_lsblk_output = {
            "blockdevices": [
                {
                    "name": "sdb",
                    "model": "SanDisk Ultra",
                    "tran": "usb",
                    "rm": True,
                    "children": [
                        {
                            "name": "sdb1",
                            "label": "BACKUP_USB",
                            "uuid": "1234-5678",
                            "size": "64G",
                            "fstype": "ext4",
                            "type": "part"
                        }
                    ]
                }
            ]
        }
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=json.dumps(mock_lsblk_output), returncode=0)
            usbs = StorageManager.get_available_usb_disks()
            self.assertEqual(len(usbs), 1)
            self.assertEqual(usbs[0]["label"], "BACKUP_USB")
            self.assertEqual(usbs[0]["uuid"], "1234-5678")
            self.assertEqual(usbs[0]["fstype"], "EXT4")
            self.assertFalse(usbs[0]["needs_formatting"])

    @patch("subprocess.run")
    def test_format_usb_disk(self, mock_run):
        mock_run.return_value = MagicMock(returncode=0, stdout="", stderr="")
        ok, msg = StorageManager.format_usb_disk("/dev/sdz", fs_type="ext4", label="TESTLABEL")
        self.assertTrue(ok)
        self.assertIn("successfully formatted", msg)


class TestResticManager(unittest.TestCase):
    def setUp(self):
        self.mgr = ResticManager(repo_url="/tmp/test-repo", password="testpassword123")

    def test_env_setup(self):
        env = self.mgr._get_env()
        self.assertEqual(env["RESTIC_PASSWORD"], "testpassword123")

    def test_build_cmd(self):
        cmd = self.mgr._build_cmd("snapshots", "--json")
        self.assertEqual(cmd, ["restic", "-r", "/tmp/test-repo", "snapshots", "--json"])

    def test_list_snapshots_json_parsing(self):
        fake_snapshots = [
            {
                "id": "abcdef1234567890",
                "short_id": "abcdef12",
                "time": "2026-09-05T12:00:00Z",
                "paths": ["/.snapshots/pulsar-timemachine-root-20260905-120000"],
                "tags": ["pulsaros", "subvol-root"],
                "hostname": "pulsar-laptop"
            }
        ]
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=json.dumps(fake_snapshots), returncode=0)
            snaps = self.mgr.list_snapshots()
            self.assertEqual(len(snaps), 1)
            self.assertEqual(snaps[0]["short_id"], "abcdef12")
            self.assertIn("pulsaros", snaps[0]["tags"])

    def test_delete_snapshot(self):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="", stderr="")
            ok, msg = self.mgr.delete_snapshot("abcdef12")
            self.assertTrue(ok)
            self.assertIn("deleted successfully", msg)

    def test_cancel_backup(self):
        mock_proc = MagicMock()
        mock_proc.poll.return_value = None
        self.mgr.current_process = mock_proc
        self.assertTrue(self.mgr.cancel_backup())
        mock_proc.terminate.assert_called_once()


class TestSchedulerManager(unittest.TestCase):
    def test_calendar_mappings(self):
        self.assertEqual(CALENDAR_MAP["hourly"], "*-*-* *:00:00")
        self.assertEqual(CALENDAR_MAP["daily"], "*-*-* 03:00:00")
        self.assertEqual(CALENDAR_MAP["weekly"], "Sun *-*-* 03:00:00")
        self.assertEqual(CALENDAR_MAP["monthly"], "*-*-01 03:00:00")


class TestTimeMachineEngine(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.config_file = os.path.join(self.temp_dir, "test_engine_cfg.json")
        self.cfg_mgr = ConfigManager(custom_path=self.config_file)
        self.engine = TimeMachineEngine(self.cfg_mgr)

    def test_resolve_rclone_target(self):
        self.cfg_mgr.set("destination_type", "rclone")
        self.cfg_mgr.set("rclone_remote", "mycloud")
        self.cfg_mgr.set("rclone_path", "backups/pulsar")
        self.cfg_mgr.set("repo_password", "secretpass")
        
        repo_url, passw, rconf = self.engine.resolve_repository_target()
        self.assertEqual(repo_url, "rclone:mycloud:backups/pulsar")
        self.assertEqual(passw, "secretpass")

    def test_resolve_usb_target(self):
        self.cfg_mgr.set("destination_type", "usb")
        self.cfg_mgr.set("usb_mount_path", "/media/user/USB_DRIVE")
        self.cfg_mgr.set("usb_repo_subpath", "my-backups")
        
        with patch("os.path.exists", return_value=True):
            repo_url, passw, _ = self.engine.resolve_repository_target()
            self.assertEqual(repo_url, "/media/user/USB_DRIVE/my-backups")

    def test_delete_backup_delegation(self):
        with patch.object(self.engine, "resolve_repository_target", return_value=("/tmp/repo", "pass", None)), \
             patch("core.engine.ResticManager") as mock_restic_cls:
            mock_inst = MagicMock()
            mock_inst.delete_snapshot.return_value = (True, "Snapshot deleted.")
            mock_restic_cls.return_value = mock_inst
            
            ok, msg = self.engine.delete_backup("testsnap123")
            self.assertTrue(ok)
            mock_inst.delete_snapshot.assert_called_with("testsnap123", log_callback=None)

    def test_custom_path_backup_and_excludes(self):
        with patch.object(self.engine, "resolve_repository_target", return_value=("/tmp/repo", "pass", None)), \
             patch("core.engine.ResticManager") as mock_restic_cls, \
             patch("os.path.exists", return_value=True):
            mock_inst = MagicMock()
            mock_inst.is_repo_initialized.return_value = True
            mock_inst.run_backup.return_value = (True, "Backup successful", {"snapshot_id": "abc1234", "total_bytes_processed": 1024*1024})
            mock_restic_cls.return_value = mock_inst

            self.cfg_mgr.set("source", "custom")
            self.cfg_mgr.set("custom_path", "/tmp/small_folder")
            self.cfg_mgr.set("exclude_patterns", ["*.log", ".cache"])

            ok, msg = self.engine.perform_backup()
            self.assertTrue(ok)
            mock_inst.run_backup.assert_called_once()
            call_kwargs = mock_inst.run_backup.call_args[1]
            self.assertEqual(call_kwargs["paths_to_backup"], ["/tmp/small_folder"])
            self.assertEqual(call_kwargs["excludes"], ["*.log", ".cache"])


if __name__ == "__main__":
    unittest.main()
