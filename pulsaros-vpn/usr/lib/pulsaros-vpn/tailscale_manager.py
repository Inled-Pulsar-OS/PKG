#!/usr/bin/env python3
"""
Tailscale Manager for Pulsar VPN
Provides safe queries and controls for Tailscale daemon and status without exposing to public internet.
"""

import json
import logging
import os
import shutil
import subprocess
from typing import Dict, Any, Optional, List

logger = logging.getLogger("PulsarVPN.Tailscale")


class TailscaleManager:
    def __init__(self):
        self.tailscale_bin = shutil.which("tailscale")

    def is_installed(self) -> bool:
        return self.tailscale_bin is not None

    def get_status(self) -> Dict[str, Any]:
        """
        Returns a structured dictionary with Tailscale connection status.
        """
        if not self.is_installed():
            return {
                "installed": False,
                "running": False,
                "backend_state": "NotInstalled",
                "ip": None,
                "hostname": None,
                "magic_dns": None,
                "peers": []
            }

        try:
            res = subprocess.run(
                [self.tailscale_bin, "status", "--json"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=5
            )
            if res.returncode != 0:
                return {
                    "installed": True,
                    "running": False,
                    "backend_state": "Stopped",
                    "error": res.stderr.strip(),
                    "ip": None,
                    "hostname": None,
                    "magic_dns": None,
                    "peers": []
                }

            data = json.loads(res.stdout)
            backend_state = data.get("BackendState", "Unknown")
            self_node = data.get("Self") or {}
            tailscale_ips = self_node.get("TailscaleIPs", [])
            primary_ip = tailscale_ips[0] if tailscale_ips else None

            peers = []
            peers_dict = data.get("Peer") or {}
            for peer_key, peer_data in peers_dict.items():
                p_ips = peer_data.get("TailscaleIPs", [])
                peers.append({
                    "id": peer_key,
                    "hostname": peer_data.get("HostName", "Unknown"),
                    "os": peer_data.get("OS", "unknown"),
                    "ip": p_ips[0] if p_ips else None,
                    "online": peer_data.get("Online", False),
                    "last_seen": peer_data.get("LastSeen", "")
                })

            return {
                "installed": True,
                "running": backend_state == "Running",
                "backend_state": backend_state,
                "ip": primary_ip,
                "all_ips": tailscale_ips,
                "hostname": self_node.get("HostName"),
                "magic_dns": self_node.get("DNSName", "").rstrip("."),
                "auth_url": data.get("AuthURL"),
                "peers": peers
            }
        except Exception as e:
            logger.error(f"Error querying tailscale status: {e}")
            return {
                "installed": True,
                "running": False,
                "backend_state": "Error",
                "error": str(e),
                "ip": None,
                "hostname": None,
                "magic_dns": None,
                "peers": []
            }

    def start_connection(self) -> Optional[str]:
        """
        Invokes tailscale up or login. Returns auth URL if required.
        """
        if not self.is_installed():
            return None
        try:
            import time
            user = os.environ.get("USER", "jaime")
            # Try running with operator first
            cmd = [self.tailscale_bin, "up", f"--operator={user}", "--reset", "--accept-routes=false"]
            proc = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            time.sleep(1)
            status = self.get_status()
            if status.get("auth_url"):
                return status.get("auth_url")

            for _ in range(6):
                time.sleep(0.5)
                status = self.get_status()
                if status.get("auth_url"):
                    return status.get("auth_url")
            return None
        except Exception as e:
            logger.error(f"Failed to start tailscale: {e}")
            return None

    def stop_connection(self) -> bool:
        """
        Disconnects tailscale.
        """
        if not self.is_installed():
            return False
        try:
            res = subprocess.run(
                ["pkexec", self.tailscale_bin, "down"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10
            )
            return res.returncode == 0
        except Exception as e:
            logger.error(f"Failed to stop tailscale: {e}")
            return False
