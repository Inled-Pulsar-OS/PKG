#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Firewall - Socket Watcher Daemon
Monitors listening sockets and triggers the prompt when an app opens an unallowed port.
Guarantees a single prompt window on screen at any time and prevents prompt flooding.
"""

import logging
import os
import subprocess
import sys
import time

sys.path.insert(0, "/usr/lib/pulsaros-firewall")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from core import (
    get_listening_sockets,
    get_ufw_status,
    load_decisions,
    load_preferences,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [PulsarFirewall.Watcher] %(levelname)s: %(message)s",
)
logger = logging.getLogger("PulsarFirewall.Watcher")


class SocketWatcher:
    def __init__(self):
        self.session_prompted_ports = set()
        self.session_prompted_apps = set()
        self.active_prompt_proc = None

    def is_prompt_open(self) -> bool:
        if self.active_prompt_proc is not None:
            if self.active_prompt_proc.poll() is None:
                return True
            self.active_prompt_proc = None
        return False

    def run_prompt_dialog(self, sock: dict) -> bool:
        # Strict single window check: if a prompt is already visible, do not open another
        if self.is_prompt_open():
            return False

        port = sock["port"]
        proto = sock["proto"]
        pid = sock.get("pid") or 0
        app_name = sock.get("app_name") or "Network Application"
        binary = sock.get("binary") or ""

        cmd = [
            "/usr/bin/pulsar-firewall-prompt",
            "--port", str(port),
            "--proto", proto,
            "--pid", str(pid),
            "--app", app_name,
            "--binary", binary,
        ]

        # Development fallback
        if not os.path.exists(cmd[0]):
            local_prompt = os.path.join(os.path.dirname(os.path.abspath(__file__)), "prompt_dialog.py")
            cmd = [
                sys.executable,
                local_prompt,
                "--port", str(port),
                "--proto", proto,
                "--pid", str(pid),
                "--app", app_name,
                "--binary", binary,
            ]

        try:
            logger.info(f"Opening single security prompt for '{app_name}' on {port}/{proto} (PID: {pid})")
            proc = subprocess.Popen(cmd)
            self.active_prompt_proc = proc
            return True
        except Exception as e:
            logger.error(f"Failed to launch prompt dialog: {e}")
            return False

    def loop(self):
        logger.info("Starting Pulsar Firewall Socket Watcher...")
        while True:
            try:
                prefs = load_preferences()
                if not prefs.get("notify_on_new_ports", True):
                    time.sleep(prefs.get("poll_interval_seconds", 3))
                    continue

                ufw_status = get_ufw_status()
                # If firewall is disabled, everything is open, no need to alert
                if not ufw_status.get("active", False):
                    time.sleep(prefs.get("poll_interval_seconds", 3))
                    continue

                # If a prompt is already open on screen, wait until user closes it
                if self.is_prompt_open():
                    time.sleep(prefs.get("poll_interval_seconds", 3))
                    continue

                listening = get_listening_sockets()
                decisions = load_decisions()

                for sock in listening:
                    port = sock["port"]
                    proto = sock["proto"]
                    key = f"{port}/{proto}"
                    app_name = sock.get("app_name") or ""
                    binary = sock.get("binary") or ""

                    # 1. Skip if already allowed by UFW
                    if sock.get("is_allowed", False):
                        continue

                    # 2. Skip if already decided (by port, app name, or binary)
                    if key in decisions:
                        continue
                    if app_name and f"app:{app_name}" in decisions:
                        continue
                    if binary and f"bin:{binary}" in decisions:
                        continue

                    # 3. Skip if already prompted in this session (by port or app)
                    if key in self.session_prompted_ports:
                        continue
                    if app_name and app_name in self.session_prompted_apps:
                        continue
                    if binary and binary in self.session_prompted_apps:
                        continue

                    # Mark prompted
                    self.session_prompted_ports.add(key)
                    if app_name:
                        self.session_prompted_apps.add(app_name)
                    if binary:
                        self.session_prompted_apps.add(binary)

                    # Show dialog (only one at a time)
                    opened = self.run_prompt_dialog(sock)
                    if opened:
                        # Break out of this pass to avoid opening other prompts simultaneously
                        break

            except Exception as e:
                logger.error(f"Error in watcher loop: {e}", exc_info=True)

            time.sleep(load_preferences().get("poll_interval_seconds", 3))


def main():
    watcher = SocketWatcher()
    watcher.loop()


if __name__ == "__main__":
    main()
