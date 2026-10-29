#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Firewall - Privileged Helper
Executes UFW commands with root permissions via PolicyKit.
"""

import os
import re
import subprocess
import sys


def sanitize_comment(comment: str) -> str:
    # Allow alphanumeric, spaces, dashes and underscores
    return re.sub(r"[^a-zA-Z0-9_\-\. ]", "", comment)[:40]


def run_ufw(args: list) -> int:
    cmd = ["ufw"] + args
    return subprocess.run(cmd, check=False).returncode


def main():
    if os.geteuid() != 0:
        print("Error: Helper must be run as root", file=sys.stderr)
        sys.exit(1)

    if len(sys.argv) < 2:
        print("Usage: firewall-helper <action> [args...]", file=sys.stderr)
        sys.exit(1)

    action = sys.argv[1].lower()

    if action == "status-raw":
        res_v = subprocess.run(["ufw", "status", "verbose"], stdout=subprocess.PIPE, text=True, check=False)
        res_n = subprocess.run(["ufw", "status", "numbered"], stdout=subprocess.PIPE, text=True, check=False)
        print(res_v.stdout)
        print("--- NUMBERED ---")
        print(res_n.stdout)
        sys.exit(0)

    elif action == "enable":
        code = run_ufw(["--force", "enable"])
        sys.exit(code)

    elif action == "disable":
        code = run_ufw(["disable"])
        sys.exit(code)

    elif action == "init-defaults":
        # Safe default configuration for Pulsar OS
        run_ufw(["default", "deny", "incoming"])
        run_ufw(["default", "allow", "outgoing"])
        # Loopback is critical for local IPC, local webviews, Ollama, etc.
        run_ufw(["allow", "in", "on", "lo"])
        # Pulsar & Sayri standard service ports
        run_ufw(["allow", "8765/tcp", "comment", "Sayri API"])
        run_ufw(["allow", "8989/tcp", "comment", "Pulsar Bridge"])
        run_ufw(["allow", "8085/tcp", "comment", "Pulsar Broadway"])
        run_ufw(["allow", "5353/udp", "comment", "mDNS Avahi"])
        run_ufw(["allow", "53317/tcp", "comment", "FlyDrop"])
        run_ufw(["allow", "53317/udp", "comment", "FlyDrop"])
        run_ufw(["allow", "27183:27199/tcp", "comment", "scrcpy"])
        code = run_ufw(["--force", "enable"])
        sys.exit(code)

    elif action in ("allow", "deny", "reject", "rule"):
        if action == "rule":
            if len(sys.argv) < 5:
                print("Usage: rule <allow|deny|reject> <port> <proto> [comment]", file=sys.stderr)
                sys.exit(1)
            subaction = sys.argv[2].lower()
            port = sys.argv[3]
            proto = sys.argv[4].lower()
            comment = sys.argv[5] if len(sys.argv) > 5 else ""
        else:
            if len(sys.argv) < 4:
                print(f"Usage: {action} <port> <proto> [comment]", file=sys.stderr)
                sys.exit(1)
            subaction = action
            port = sys.argv[2]
            proto = sys.argv[3].lower()
            comment = sys.argv[4] if len(sys.argv) > 4 else ""

        if subaction not in ("allow", "deny", "reject"):
            print("Invalid action (allow/deny/reject)", file=sys.stderr)
            sys.exit(1)
        if not port.isdigit() or not (1 <= int(port) <= 65535):
            print("Invalid port number", file=sys.stderr)
            sys.exit(1)
        if proto not in ("tcp", "udp", "any"):
            print("Invalid protocol (tcp/udp/any)", file=sys.stderr)
            sys.exit(1)

        rule = f"{port}/{proto}" if proto != "any" else str(port)
        cmd_args = [subaction, rule]
        if comment:
            cmd_args.extend(["comment", sanitize_comment(comment)])
        code = run_ufw(cmd_args)
        sys.exit(code)

    elif action == "delete":
        if len(sys.argv) < 4:
            print("Usage: delete <port> <proto>", file=sys.stderr)
            sys.exit(1)
        port = sys.argv[2]
        proto = sys.argv[3].lower()

        rule = f"{port}/{proto}" if proto != "any" else str(port)
        deleted = False
        for act in ("allow", "deny", "reject"):
            res = subprocess.run(["ufw", "delete", act, rule], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False)
            if "Rule deleted" in res.stdout:
                deleted = True
        sys.exit(0 if deleted else 0)

    else:
        print(f"Unknown action: {action}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
