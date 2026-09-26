"""
Pulsar HBlock - Core
-------------------
Estado, estadísticas y operaciones de hblock (https://github.com/hectorm/hblock).

El estado se lee de /etc/hosts (legible por cualquier usuario); las operaciones
privilegiadas (activar/desactivar/actualizar) se delegan en el helper de root
`/usr/lib/pulsaros-hblock/hblock-helper.sh` vía pkexec.

hblock marca su salida en /etc/hosts con cabeceras como:
    # Generated with hBlock 3.5.1 (https://github.com/hectorm/hblock)
    # Blocked domains: 123456
    # Date: Mon Sep 22 12:00:00 UTC 2026
    # BEGIN HEADER / # BEGIN BLOCKLIST / # BEGIN FOOTER ... # END FOOTER
"""

import os
import re
import subprocess
import time

HOSTS_FILE = "/etc/hosts"
SOURCES_FILE = "/etc/hblock/sources.list"
HELPER = "/usr/lib/pulsaros-hblock/hblock-helper.sh"

_RE_GENERATED = re.compile(r"^#\s*Generated with hBlock", re.MULTILINE)
_RE_BLOCKED = re.compile(r"^#\s*Blocked domains:\s*([0-9]+)\s*$", re.MULTILINE)
_RE_DATE = re.compile(r"^#\s*Date:\s*(.+)$", re.MULTILINE)
_RE_BEGIN_BLOCK = re.compile(r"^#\s*BEGIN BLOCKLIST", re.MULTILINE)
_RE_END_BLOCK = re.compile(r"^#\s*END BLOCKLIST", re.MULTILINE)
_RE_ENTRY = re.compile(r"^(?:0\.0\.0\.0|127\.0\.0\.1)\s+\S")


def hblock_binary():
    """Return the path to the hblock binary, or None if not installed."""
    for p in ("/usr/bin/hblock", "/usr/local/bin/hblock", "/usr/sbin/hblock"):
        if os.path.exists(p):
            return p
    return None


def get_status():
    """Return a dict with the current hblock state (no root privileges needed)."""
    state = {
        "installed": bool(hblock_binary()),
        "enabled": False,
        "blocked": 0,
        "blocked_lines": 0,
        "sources": None,
        "last_update": None,
        "version": None,
        "hosts_mtime": 0,
    }
    try:
        with open(HOSTS_FILE, encoding="utf-8", errors="replace") as f:
            content = f.read()
    except Exception:
        return state

    if not _RE_GENERATED.search(content):
        return state
    state["enabled"] = True

    m = _RE_BLOCKED.search(content)
    if m:
        state["blocked"] = int(m.group(1))

    in_block = False
    for line in content.splitlines():
        if _RE_BEGIN_BLOCK.search(line):
            in_block = True
            continue
        if _RE_END_BLOCK.search(line):
            in_block = False
            continue
        if in_block and _RE_ENTRY.match(line):
            state["blocked_lines"] += 1

    m = _RE_DATE.search(content)
    if m:
        state["last_update"] = m.group(1).strip()
    try:
        state["hosts_mtime"] = os.stat(HOSTS_FILE).st_mtime
    except Exception:
        pass

    if os.path.isfile(SOURCES_FILE):
        try:
            with open(SOURCES_FILE, encoding="utf-8", errors="replace") as f:
                srcs = [
                    line.strip()
                    for line in f
                    if line.strip() and not line.lstrip().startswith("#")
                ]
            if srcs:
                state["sources"] = len(srcs)
        except Exception:
            pass

    # hblock version, best effort from the script banner
    m = re.search(r"hBlock\s+([0-9][0-9a-zA-Z.+-]*)", content)
    if m:
        state["version"] = m.group(1)
    return state


def run_action(action):
    """Run a privileged helper action through pkexec.

    Returns (ok, message).
    """
    if not os.path.exists(HELPER):
        return False, "Helper not found: %s" % HELPER
    if action in ("enable", "disable", "update", "status"):
        pass
    else:
        return False, "Unknown action: %s" % action
    try:
        proc = subprocess.run(
            ["pkexec", HELPER, action],
            capture_output=True,
            text=True,
            timeout=10,
        )
        out = (proc.stdout or "").strip()
        if proc.returncode == 0:
            return True, out or "OK"
        err = (proc.stderr or "").strip()
        return False, err or out or ("Error code %d" % proc.returncode)
    except subprocess.TimeoutExpired:
        return False, "Operation timed out"
    except Exception as exc:  # noqa: BLE001
        return False, str(exc)


if __name__ == "__main__":
    # Quick CLI sanity check for debugging
    import json
    import sys

    if len(sys.argv) > 1 and sys.argv[1] in ("enable", "disable", "update"):
        ok, msg = run_action(sys.argv[1])
        print(msg if ok else "ERROR: " + msg)
        sys.exit(0 if ok else 1)
    print(json.dumps(get_status(), indent=2, default=str))