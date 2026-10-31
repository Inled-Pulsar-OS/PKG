#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS - What's New: shared helpers.

Loads the release notes shipped in the package, the per-user state and the
configuration, and exposes a few small utilities used by the watcher and the
viewer.
"""

import glob
import json
import os
import time

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
NOTES_DIR = "/usr/share/pulsaros-whatsnew/releases.d"
ASSETS_DIR = "/usr/share/pulsaros-whatsnew"
SYSTEM_CONFIG = "/etc/pulsaros/whatsnew.conf"
SYSTEM_STATE_DIR = "/var/lib/pulsaros/whatsnew"

APP_ID = "es.inled.PulsarWhatsNew"

# Files that must never be treated as release notes.
IGNORED_NOTES = {"readme.md", "index.md"}


def user_config_path():
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.join(
        os.path.expanduser("~"), ".config"
    )
    return os.path.join(base, "pulsaros", "whatsnew.conf")


def user_state_dir():
    base = os.environ.get("XDG_STATE_HOME") or os.path.join(
        os.path.expanduser("~"), ".local", "state"
    )
    return os.path.join(base, "pulsaros", "whatsnew")


def user_state_path():
    return os.path.join(user_state_dir(), "state.json")


def is_live_or_pre_setup():
    """Returns True if running in a live ISO session or before OOTB setup completes."""
    if os.path.exists("/etc/pulsar-need-setup"):
        return True
    if os.path.exists("/run/archiso") or os.path.exists("/run/live") or os.path.exists("/live"):
        return True
    try:
        with open("/proc/cmdline", "r") as f:
            cmd = f.read()
            if any(k in cmd for k in ("archiso", "pulsar-live", "boot=live")):
                return True
    except OSError:
        pass
    if os.environ.get("USER") == "live":
        return True
    return False


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
def _read_config_file(path, config):
    try:
        with open(path, "r", encoding="utf-8") as fh:
            for raw in fh:
                line = raw.strip()
                if not line or line.startswith("#") or "=" not in line:
                    continue
                key, value = line.split("=", 1)
                config[key.strip()] = value.strip()
    except OSError:
        pass


def load_config():
    """Configuration: system defaults overridden by the user's file."""
    config = {"enabled": "true"}
    _read_config_file(SYSTEM_CONFIG, config)
    _read_config_file(user_config_path(), config)

    def as_bool(key, default=True):
        value = str(config.get(key, str(default))).strip().lower()
        return value in ("1", "true", "yes", "on")

    config["_enabled"] = as_bool("enabled", True)
    config["_autostart"] = as_bool("autostart", True)
    return config


# ---------------------------------------------------------------------------
# State (per user)
# ---------------------------------------------------------------------------
def load_state():
    state = {"seeded": False, "seen": [], "disabled": False, "last_shown": 0}
    try:
        with open(user_state_path(), "r", encoding="utf-8") as fh:
            data = json.load(fh)
        if isinstance(data, dict):
            state.update(data)
    except (OSError, ValueError):
        pass
    if not isinstance(state.get("seen"), list):
        state["seen"] = []
    return state


def save_state(state):
    directory = user_state_dir()
    try:
        os.makedirs(directory, exist_ok=True)
        tmp = user_state_path() + ".tmp"
        with open(tmp, "w", encoding="utf-8") as fh:
            json.dump(state, fh, ensure_ascii=False, indent=2)
        os.replace(tmp, user_state_path())
    except OSError:
        pass


# ---------------------------------------------------------------------------
# Release notes
# ---------------------------------------------------------------------------
def _parse_scalar(value):
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    low = value.lower()
    if low in ("true", "yes", "on"):
        return True
    if low in ("false", "no", "off"):
        return False
    return value


def parse_front_matter(text):
    """Return (metadata dict, markdown body)."""
    meta = {}
    body = text
    if text.startswith("---"):
        end = text.find("\n---", 3)
        if end != -1:
            raw = text[3:end].strip("\n")
            body = text[end + 4:].lstrip("\n")
            meta = _parse_front_matter_block(raw)
    return meta, body


def _parse_front_matter_block(raw):
    meta = {}
    current_key = None
    for line in raw.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        stripped = line.strip()
        if stripped.startswith("- ") and current_key:
            meta.setdefault(current_key, [])
            if isinstance(meta[current_key], list):
                meta[current_key].append(_parse_scalar(stripped[2:]))
            continue
        if ":" in stripped:
            key, value = stripped.split(":", 1)
            key = key.strip()
            value = value.strip()
            current_key = key
            if value == "":
                meta[key] = []
            else:
                meta[key] = _parse_scalar(value)
    return meta


def _as_list(value):
    if value is None:
        return []
    if isinstance(value, list):
        return [str(v).strip() for v in value if str(v).strip()]
    return [part.strip() for part in str(value).split(",") if part.strip()]


def _as_bool(value, default=False):
    if isinstance(value, bool):
        return value
    if value is None:
        return default
    return str(value).strip().lower() in ("1", "true", "yes", "on")


def load_notes(notes_dir=None):
    """All shipped release notes, newest first."""
    notes_dir = notes_dir or NOTES_DIR
    notes = []
    for path in glob.glob(os.path.join(notes_dir, "*.md")):
        name = os.path.basename(path)
        if name.lower() in IGNORED_NOTES:
            continue
        try:
            with open(path, "r", encoding="utf-8") as fh:
                text = fh.read()
        except OSError:
            continue
        meta, body = parse_front_matter(text)
        note_id = str(meta.get("id") or os.path.splitext(name)[0]).strip()
        date = str(meta.get("date") or note_id).strip()
        notes.append(
            {
                "id": note_id,
                "date": date,
                "title": str(meta.get("title") or "").strip(),
                "channel": str(meta.get("channel") or "").strip(),
                "important": _as_bool(meta.get("important")),
                "packages": _as_list(meta.get("packages")),
                "body": body,
            }
        )
    notes.sort(key=lambda n: (n["date"], n["id"]), reverse=True)
    return notes


# ---------------------------------------------------------------------------
# Environment helpers
# ---------------------------------------------------------------------------
def is_graphical():
    """Whether the current process runs inside a graphical session."""
    return bool(
        os.environ.get("WAYLAND_DISPLAY")
        or os.environ.get("DISPLAY")
        or _wayland_socket_present()
    )


def _wayland_socket_present():
    runtime = os.environ.get("XDG_RUNTIME_DIR")
    if not runtime or not os.path.isdir(runtime):
        return False
    try:
        for entry in os.listdir(runtime):
            if entry.startswith("wayland-") and not entry.endswith(".lock"):
                return True
    except OSError:
        return False
    return False


def fresh_install_signal():
    """Heuristic used to silence the very first login of a new image."""
    return os.path.exists("/etc/pulsar-need-setup") or os.path.exists(
        "/etc/pulsar-need-cleanup"
    )


def now():
    return int(time.time())
