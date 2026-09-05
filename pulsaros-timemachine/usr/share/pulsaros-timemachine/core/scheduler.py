#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Time Machine - Scheduler Manager
Handles systemd service & timer configuration for automated background backups.
"""

import os
import subprocess
from typing import Optional, Dict

SYSTEMD_SERVICE_FILE = "/etc/systemd/system/pulsaros-timemachine.service"
SYSTEMD_TIMER_FILE = "/etc/systemd/system/pulsaros-timemachine.timer"

CALENDAR_MAP: Dict[str, str] = {
    "hourly": "*-*-* *:00:00",
    "daily": "*-*-* 03:00:00",
    "weekly": "Sun *-*-* 03:00:00",
    "monthly": "*-*-01 03:00:00"
}

class SchedulerManager:
    """Manages systemd timer & service for Pulsar OS Time Machine."""

    @staticmethod
    def is_timer_active() -> bool:
        """Checks if the systemd timer is enabled and active."""
        try:
            res = subprocess.run(
                ["systemctl", "is-active", "pulsaros-timemachine.timer"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            return res.stdout.strip() == "active"
        except Exception:
            return False

    @staticmethod
    def get_next_run_time() -> str:
        """Gets human-readable next execution time from systemctl."""
        try:
            res = subprocess.run(
                ["systemctl", "list-timers", "pulsaros-timemachine.timer", "--no-legend", "--no-pager"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            if res.returncode == 0 and res.stdout.strip():
                parts = res.stdout.strip().split()
                if len(parts) >= 3:
                    return f"{parts[0]} {parts[1]} {parts[2]}"
            return "Scheduled"
        except Exception:
            return "Unknown"

    @classmethod
    def write_systemd_units(cls, frequency: str = "daily") -> bool:
        """Writes the service and timer unit files."""
        calendar_expr = CALENDAR_MAP.get(frequency, "*-*-* 03:00:00")

        service_content = """[Unit]
Description=Pulsar OS Time Machine Automatic Backup Service
Documentation=https://github.com/pulsar-os
After=network-online.target local-fs.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/bin/pulsaros-timemachine backup --scheduled
Nice=19
IOSchedulingClass=2
IOSchedulingPriority=7

[Install]
WantedBy=multi-user.target
"""

        timer_content = f"""[Unit]
Description=Pulsar OS Time Machine Backup Timer
Documentation=https://github.com/pulsar-os

[Timer]
OnCalendar={calendar_expr}
Persistent=true
RandomizedDelaySec=300

[Install]
WantedBy=timers.target
"""
        try:
            # Service
            tmp_svc = f"/tmp/pulsaros-timemachine.service.{os.getpid()}"
            with open(tmp_svc, "w") as f:
                f.write(service_content)
            cmd_svc = ["cp", "-f", tmp_svc, SYSTEMD_SERVICE_FILE]
            if os.geteuid() != 0:
                cmd_svc = ["sudo"] + cmd_svc
            subprocess.run(cmd_svc, check=True)
            if os.path.exists(tmp_svc):
                os.remove(tmp_svc)

            # Timer
            tmp_tmr = f"/tmp/pulsaros-timemachine.timer.{os.getpid()}"
            with open(tmp_tmr, "w") as f:
                f.write(timer_content)
            cmd_tmr = ["cp", "-f", tmp_tmr, SYSTEMD_TIMER_FILE]
            if os.geteuid() != 0:
                cmd_tmr = ["sudo"] + cmd_tmr
            subprocess.run(cmd_tmr, check=True)
            if os.path.exists(tmp_tmr):
                os.remove(tmp_tmr)

            # Reload
            cmd_reload = ["systemctl", "daemon-reload"]
            if os.geteuid() != 0:
                cmd_reload = ["sudo"] + cmd_reload
            subprocess.run(cmd_reload, check=False)
            return True
        except Exception as e:
            print(f"[SchedulerManager] Error creating systemd units: {e}")
            return False

    @classmethod
    def enable(cls, frequency: str = "daily") -> bool:
        """Enables and starts the backup timer."""
        if frequency == "manual":
            return cls.disable()

        if not cls.write_systemd_units(frequency):
            return False

        try:
            cmd = ["systemctl", "enable", "--now", "pulsaros-timemachine.timer"]
            if os.geteuid() != 0:
                cmd = ["sudo"] + cmd
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            return res.returncode == 0
        except Exception as e:
            print(f"[SchedulerManager] Error enabling timer: {e}")
            return False

    @classmethod
    def disable(cls) -> bool:
        """Disables and stops the backup timer."""
        try:
            cmd = ["systemctl", "disable", "--now", "pulsaros-timemachine.timer"]
            if os.geteuid() != 0:
                cmd = ["sudo"] + cmd
            subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
            return True
        except Exception as e:
            print(f"[SchedulerManager] Error disabling timer: {e}")
            return False
