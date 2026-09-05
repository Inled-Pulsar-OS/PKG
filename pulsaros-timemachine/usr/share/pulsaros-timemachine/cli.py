#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Time Machine - CLI & Main Entrypoint
"""

import sys
import os
import argparse
import json

# Add current folder to sys.path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from core.config_manager import ConfigManager
from core.engine import TimeMachineEngine
from core.storage_manager import StorageManager
from core.scheduler import SchedulerManager

def run_cli():
    parser = argparse.ArgumentParser(
        prog="pulsaros-timemachine",
        description="Pulsar OS Time Machine - Btrfs + Restic Backup & Recovery Utility"
    )
    
    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # 1. Backup command
    backup_parser = subparsers.add_parser("backup", help="Trigger a Time Machine backup")
    backup_parser.add_argument("--source", choices=["both", "root", "home", "custom"], help="Override source subvolume(s) or custom")
    backup_parser.add_argument("--path", help="Specific custom directory to back up (when source is custom)")
    backup_parser.add_argument("--exclude", action="append", help="Pattern to exclude (can be specified multiple times)")
    backup_parser.add_argument("--target", choices=["usb", "samba", "rclone"], help="Override storage destination")
    backup_parser.add_argument("--scheduled", action="store_true", help="Flag indicating automatic run from systemd timer")

    # 2. Restore command
    restore_parser = subparsers.add_parser("restore", help="Restore files from a snapshot")
    restore_parser.add_argument("--snapshot", required=True, help="Snapshot ID to restore from")
    restore_parser.add_argument("--target", required=True, help="Target destination directory")
    restore_parser.add_argument("--include", help="Optional specific file or folder path pattern to extract")

    # 3. Delete command (manual snapshot deletion)
    delete_parser = subparsers.add_parser("delete", help="Delete a specific snapshot manually")
    delete_parser.add_argument("--snapshot", required=True, help="Snapshot ID to delete from repository")

    # 4. List command
    list_parser = subparsers.add_parser("list", help="List available snapshots in current backup destination")
    list_parser.add_argument("--json", action="store_true", help="Output raw JSON")

    # 5. Pause & Resume commands
    subparsers.add_parser("pause", help="Pause automatic scheduled backups")
    subparsers.add_parser("resume", help="Resume automatic scheduled backups")

    # 6. Config command
    config_parser = subparsers.add_parser("config", help="Manage Time Machine configuration")
    config_parser.add_argument("--show", action="store_true", help="Print active configuration")
    config_parser.add_argument("--set", nargs="+", help="Set key=value pairs (e.g. schedule_frequency=daily)")

    # 7. GUI command
    gui_parser = subparsers.add_parser("gui", help="Launch the graphical interface")

    args = parser.parse_args()

    # If no command is provided or 'gui', launch the GTK4 application
    if not args.command or args.command == "gui":
        try:
            from ui.app import main_gui
            return main_gui()
        except ImportError as e:
            print(f"Error launching GTK4 GUI: {e}")
            print("Falling back to CLI mode.")
            parser.print_help()
            return 1

    config_mgr = ConfigManager()
    engine = TimeMachineEngine(config_mgr)

    if args.command == "backup":
        if args.target:
            config_mgr.set("destination_type", args.target)
        ok, msg = engine.perform_backup(
            source_override=args.source,
            custom_path_override=args.path,
            excludes_override=args.exclude,
            log_cb=lambda line: print(f"[LOG] {line}")
        )
        print(f"Result: {msg}")
        return 0 if ok else 1

    elif args.command == "restore":
        ok, msg = engine.perform_restore(
            snapshot_id=args.snapshot,
            target_dir=args.target,
            include_path=args.include,
            log_cb=lambda line: print(f"[RESTORE] {line}")
        )
        print(f"Result: {msg}")
        return 0 if ok else 1

    elif args.command == "delete":
        ok, msg = engine.delete_backup(
            snapshot_id=args.snapshot,
            log_cb=lambda line: print(f"[DELETE] {line}")
        )
        print(f"Result: {msg}")
        return 0 if ok else 1

    elif args.command == "pause":
        SchedulerManager.disable()
        print("Automatic backups paused.")
        return 0

    elif args.command == "resume":
        freq = config_mgr.get("schedule_frequency", "daily")
        SchedulerManager.enable(freq)
        print(f"Automatic backups resumed with frequency: {freq}")
        return 0

    elif args.command == "list":
        snaps = engine.get_snapshots_list()
        if args.json:
            print(json.dumps(snaps, indent=2))
        else:
            if not snaps:
                print("No snapshots found in configured destination.")
            else:
                print(f"{'SNAPSHOT ID':<16} {'DATE & TIME':<22} {'HOSTNAME':<15} {'PATHS / TAGS'}")
                print("-" * 75)
                for s in snaps:
                    sid = s.get("short_id", s.get("id", "")[:8])
                    stime = s.get("time", "")[:19].replace("T", " ")
                    shost = s.get("hostname", "")
                    stags = ", ".join(s.get("tags", []))
                    print(f"{sid:<16} {stime:<22} {shost:<15} {stags}")
        return 0

    elif args.command == "config":
        if args.set:
            for item in args.set:
                if "=" in item:
                    k, v = item.split("=", 1)
                    if v.lower() == "true":
                        v = True
                    elif v.lower() == "false":
                        v = False
                    elif v.isdigit():
                        v = int(v)
                    config_mgr.set(k, v)
            config_mgr.save()
            print("Configuration updated successfully.")

        if args.show or not args.set:
            print(json.dumps(config_mgr.config, indent=2))
        return 0

    return 0

if __name__ == "__main__":
    sys.exit(run_cli())
