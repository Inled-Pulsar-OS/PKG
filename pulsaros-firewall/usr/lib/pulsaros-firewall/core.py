#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Firewall - Core Logic and UFW Backend Wrapper
English fallback default with Spanish translations.
"""

import glob
import json
import locale
import logging
import os
import re
import shutil
import subprocess
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger("PulsarFirewall.Core")

HELPER_BIN = "/usr/lib/pulsaros-firewall/firewall-helper"
CONFIG_DIR = os.path.expanduser("~/.config/pulsaros-firewall")
DECISIONS_FILE = os.path.join(CONFIG_DIR, "decisions.json")
PREFERENCES_FILE = os.path.join(CONFIG_DIR, "preferences.json")

# Internationalization - English is the default fallback
CURRENT_LANG = (os.environ.get("LANG", "") or locale.getdefaultlocale()[0] or "").lower()
IS_SPANISH = CURRENT_LANG.startswith("es")

TRANSLATIONS_ES = {
    "Pulsar Firewall": "Cortafuegos de Pulsar",
    "Status": "Estado",
    "Firewall is Active": "El cortafuegos está activo",
    "Firewall is Disabled": "El cortafuegos está desactivado",
    "Incoming traffic blocked, outgoing allowed": "Tráfico entrante bloqueado, saliente permitido",
    "All network traffic is permitted": "Todo el tráfico de red está permitido",
    "Notify when an app listens on a closed port": "Avisar cuando una app escuche en un puerto cerrado",
    "Shows an interactive prompt to allow or block the application": "Muestra un diálogo interactivo para permitir o bloquear la aplicación",
    "Reset Defaults": "Restablecer valores",
    "Apply recommended Pulsar OS security defaults": "Aplicar valores de seguridad recomendados de Pulsar OS",
    "Pulsar & System Services": "Servicios de Pulsar y del Sistema",
    "Quick toggle for official Pulsar OS features and desktop integration": "Interruptores rápidos para funciones de Pulsar OS e integración del escritorio",
    "Sayri AI Assistant (API)": "Asistente Sayri AI (API)",
    "OpenAI-compatible local server on port 8765": "Servidor local compatible con OpenAI en puerto 8765",
    "Pulsar Phone & Bridge": "Pulsar Phone y Bridge",
    "Phone pairing and streaming on port 8989": "Vinculación y streaming con el teléfono en puerto 8989",
    "Pulsar Broadway Remote Apps": "Pulsar Broadway Apps Remotas",
    "GTK HTML5 application streaming on port 8085": "Streaming de aplicaciones GTK HTML5 en puerto 8085",
    "Network Discovery (mDNS)": "Descubrimiento en red (mDNS)",
    "Local network auto-discovery on port 5353 (UDP)": "Descubrimiento automático en red local en puerto 5353 (UDP)",
    "Remote Access (SSH)": "Acceso Remoto (SSH)",
    "Secure shell terminal access on port 22": "Acceso por terminal seguro en puerto 22",
    "File Sharing (Samba)": "Compartir Archivos (Samba)",
    "Windows file and printer sharing on port 445": "Compartir archivos e impresoras tipo Windows en puerto 445",
    "Active Applications": "Aplicaciones Activas en Red",
    "Applications currently listening on network ports on this computer": "Aplicaciones que actualmente escuchan en puertos de red en este equipo",
    "No external listening applications detected": "No se detectaron aplicaciones externas escuchando",
    "Allowed": "Permitido",
    "Blocked": "Bloqueado",
    "Allow": "Permitir",
    "Deny": "Denegar",
    "Reject": "Rechazar",
    "Rules": "Reglas del Cortafuegos",
    "Custom open ports and inbound firewall rules": "Puertos abiertos y reglas de entrada personalizadas",
    "No custom rules configured": "No hay reglas personalizadas configuradas",
    "Add Rule": "Añadir Regla",
    "Rule Details": "Detalles de la Regla",
    "Port (1-65535)": "Puerto (1-65535)",
    "Protocol": "Protocolo",
    "TCP": "TCP",
    "UDP": "UDP",
    "TCP/UDP (Any)": "TCP/UDP (Cualquiera)",
    "Action": "Acción",
    "ALLOW": "PERMITIR (ALLOW)",
    "DENY": "DENEGAR (DENY)",
    "REJECT": "RECHAZAR (REJECT)",
    "Description / App Name": "Descripción / Nombre de la app",
    "Delete": "Eliminar",
    "Delete Rule": "Eliminar Regla",
    "Close": "Cerrar",
    "Cancel": "Cancelar",
    "Save": "Guardar",
    "Refresh status": "Actualizar estado",
    "Allow incoming connections?": "¿Permitir conexiones entrantes?",
    "is attempting to accept incoming connections on port": "está intentando recibir conexiones de red en el puerto",
    "If you block this port, devices on your network will not be able to connect to this app.": "Si bloqueas este puerto, otros dispositivos en tu red no podrán conectarse a esta aplicación.",
    "Allow Always": "Permitir siempre",
    "Only This Session": "Solo esta sesión",
    "Block": "Bloquear",
    "Executable": "Ejecutable",
    "Process ID": "PID",
    "Technical Details": "Detalles técnicos",
    "Application": "Aplicación",
    "Port": "Puerto",
    "Port %s (%s)": "Puerto %s (%s)",
    "Action: %s • Source: %s": "Acción: %s • Origen: %s",
    "Network Service": "Servicio de Red",
    "OpenSSH Server": "Servidor OpenSSH",
    "mDNS Network Discovery": "Descubrimiento de Red mDNS",
    "FlyDrop File Transfer": "Transferencia FlyDrop",
}


def _(text: str) -> str:
    if IS_SPANISH:
        return TRANSLATIONS_ES.get(text, text)
    return text


def ensure_config_dir():
    try:
        os.makedirs(CONFIG_DIR, exist_ok=True)
    except Exception as e:
        logger.warning(f"Could not create config dir {CONFIG_DIR}: {e}")


def load_preferences() -> Dict[str, Any]:
    ensure_config_dir()
    defaults = {
        "notify_on_new_ports": True,
        "poll_interval_seconds": 3,
    }
    if os.path.exists(PREFERENCES_FILE):
        try:
            with open(PREFERENCES_FILE, "r", encoding="utf-8") as f:
                data = json.load(f)
                defaults.update(data)
        except Exception as e:
            logger.warning(f"Failed to read preferences: {e}")
    return defaults


def save_preferences(prefs: Dict[str, Any]):
    ensure_config_dir()
    try:
        with open(PREFERENCES_FILE, "w", encoding="utf-8") as f:
            json.dump(prefs, f, indent=2)
    except Exception as e:
        logger.warning(f"Failed to save preferences: {e}")


def load_decisions() -> Dict[str, Any]:
    ensure_config_dir()
    if os.path.exists(DECISIONS_FILE):
        try:
            with open(DECISIONS_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception as e:
            logger.warning(f"Failed to read decisions: {e}")
    return {}


def save_decision(key: str, decision: str):
    """Save user decision for an app/port ('allow', 'block', 'temporary')."""
    ensure_config_dir()
    decisions = load_decisions()
    decisions[key] = {
        "decision": decision,
        "updated_at": os.times()[4],
    }
    try:
        with open(DECISIONS_FILE, "w", encoding="utf-8") as f:
            json.dump(decisions, f, indent=2)
    except Exception as e:
        logger.warning(f"Failed to save decision: {e}")


# Helper execution
def get_helper_path() -> str:
    if os.path.exists(HELPER_BIN):
        return HELPER_BIN
    # Local fallback for development / test
    local_dev = os.path.join(os.path.dirname(os.path.abspath(__file__)), "helper.py")
    if os.path.exists(local_dev):
        return local_dev
    return "pulsar-firewall-helper"


def run_helper(action: str, *args: str) -> Tuple[bool, str]:
    """Execute privileged helper via pkexec (or directly if root)."""
    helper = get_helper_path()
    if os.geteuid() == 0:
        cmd = [helper, action, *args]
    else:
        cmd = ["pkexec", helper, action, *args]

    try:
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False)
        if res.returncode == 0:
            return True, res.stdout.strip()
        return False, res.stderr.strip() or res.stdout.strip()
    except Exception as e:
        return False, str(e)


# Desktop App Resolution Cache
_DESKTOP_CACHE: Optional[List[Dict[str, str]]] = None


def _get_desktop_entries() -> List[Dict[str, str]]:
    global _DESKTOP_CACHE
    if _DESKTOP_CACHE is not None:
        return _DESKTOP_CACHE

    entries = []
    dirs = [
        os.path.expanduser("~/.local/share/applications"),
        "/usr/local/share/applications",
        "/usr/share/applications",
    ]
    for d in dirs:
        if not os.path.isdir(d):
            continue
        for fpath in glob.glob(os.path.join(d, "*.desktop")):
            try:
                name = None
                exec_cmd = None
                icon = None
                with open(fpath, "r", encoding="utf-8", errors="ignore") as f:
                    in_desktop_entry = False
                    for line in f:
                        line = line.strip()
                        if line == "[Desktop Entry]":
                            in_desktop_entry = True
                            continue
                        if line.startswith("[") and in_desktop_entry:
                            break
                        if in_desktop_entry:
                            if line.startswith("Name=") and not name:
                                name = line.split("=", 1)[1].strip()
                            elif line.startswith("Exec=") and not exec_cmd:
                                exec_cmd = line.split("=", 1)[1].strip()
                            elif line.startswith("Icon=") and not icon:
                                icon = line.split("=", 1)[1].strip()
                if name and exec_cmd:
                    entries.append({"name": name, "exec": exec_cmd, "icon": icon or "application-x-executable", "file": fpath})
            except Exception:
                pass

    _DESKTOP_CACHE = entries
    return _DESKTOP_CACHE


IGNORED_CLIENT_APPS = {
    "seafari", "firefox", "chrome", "chromium", "brave", "epiphany",
    "zen-browser", "webkitwebprocess", "webkit", "curl", "wget", "git",
    "discord", "telegram-desktop", "slack", "spotify"
}

# System infrastructure network ports (DHCP, NTP, LLMNR, DNS client)
SYSTEM_NETWORK_PORTS = {53, 67, 68, 123, 546, 547, 5355}


def resolve_process_app_info(pid: Optional[int], ss_proc_name: str = "") -> Dict[str, str]:
    """Find friendly app name, binary path and icon for a process ID."""
    info = {
        "name": _("Network Service"),
        "binary": "",
        "cmdline": "",
        "icon": "network-server-symbolic",
    }
    if not pid or not os.path.exists(f"/proc/{pid}"):
        if ss_proc_name:
            info["name"] = ss_proc_name.capitalize()
            info["binary"] = f"/usr/bin/{ss_proc_name}"
            info["icon"] = "application-x-executable"
        return info

    try:
        exe_path = os.path.realpath(f"/proc/{pid}/exe")
        info["binary"] = exe_path
    except Exception:
        exe_path = ""

    try:
        with open(f"/proc/{pid}/cmdline", "rb") as f:
            cmd = f.read().replace(b"\x00", b" ").decode(errors="ignore").strip()
            info["cmdline"] = cmd
    except Exception:
        cmd = ""

    # Known Pulsar / system aliases
    if "sayri" in cmd.lower() or "sayri" in exe_path.lower():
        return {
            "name": _("Sayri AI Assistant (API)"),
            "binary": exe_path,
            "cmdline": cmd,
            "icon": "sayri-tray",
        }
    if "pulsar-bridge" in cmd.lower() or "bridge_daemon" in cmd.lower():
        return {
            "name": _("Pulsar Phone & Bridge"),
            "binary": exe_path,
            "cmdline": cmd,
            "icon": "pulsar-vpn",
        }
    if "broadwayd" in cmd.lower() or "broadway" in cmd.lower():
        return {
            "name": _("Pulsar Broadway Remote Apps"),
            "binary": exe_path,
            "cmdline": cmd,
            "icon": "applications-other-symbolic",
        }
    if "flydrop" in cmd.lower() or "localsend" in cmd.lower():
        return {
            "name": _("FlyDrop File Transfer"),
            "binary": exe_path,
            "cmdline": cmd,
            "icon": "pulsaros-flydrop",
        }
    if "avahi-daemon" in cmd.lower() or "adb" in cmd.lower():
        return {
            "name": _("mDNS Network Discovery"),
            "binary": exe_path,
            "cmdline": cmd,
            "icon": "network-workgroup-symbolic",
        }
    if "sshd" in cmd.lower():
        return {
            "name": _("OpenSSH Server"),
            "binary": exe_path,
            "cmdline": cmd,
            "icon": "terminal-symbolic",
        }

    # Match against .desktop files
    entries = _get_desktop_entries()
    exe_name = os.path.basename(exe_path) if exe_path else (ss_proc_name or "")

    for entry in entries:
        first_arg = entry["exec"].split()[0]
        if os.path.basename(first_arg) == exe_name or first_arg in cmd:
            info["name"] = entry["name"]
            info["icon"] = entry["icon"]
            return info

    if exe_name:
        info["name"] = exe_name.capitalize()
        info["icon"] = "application-x-executable"
    return info


def get_ufw_status() -> Dict[str, Any]:
    """Inspect and parse UFW status and rules."""
    result = {
        "installed": shutil.which("ufw") is not None,
        "active": False,
        "default_incoming": "deny",
        "default_outgoing": "allow",
        "rules": [],
    }
    if not result["installed"]:
        return result

    try:
        p = subprocess.run(["ufw", "status", "verbose"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False)
        out = p.stdout
        if p.returncode != 0 or not out.strip():
            ok, helper_out = run_helper("status-raw")
            if ok:
                out = helper_out
    except Exception as e:
        logger.debug(f"Error querying ufw directly: {e}")
        out = ""

    if not out.strip():
        return result

    result["active"] = "Status: active" in out

    # Defaults
    m_in = re.search(r"Default:\s*(\w+)\s*\(incoming\)", out)
    if m_in:
        result["default_incoming"] = m_in.group(1).lower()
    m_out = re.search(r"(\w+)\s*\(outgoing\)", out)
    if m_out:
        result["default_outgoing"] = m_out.group(1).lower()

    # Query numbered rules
    try:
        p_num = subprocess.run(["ufw", "status", "numbered"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False)
        num_out = p_num.stdout
    except Exception:
        num_out = ""

    lines = num_out.splitlines() if num_out.strip() else out.splitlines()
    for line in lines:
        line_clean = line.strip()
        if not line_clean.startswith("["):
            continue

        comment = ""
        if "#" in line_clean:
            main_part, comment = line_clean.split("#", 1)
            comment = comment.strip()
        else:
            main_part = line_clean

        m = re.match(r"\[\s*(\d+)\]\s+(\S+)\s+(ALLOW IN|DENY IN|REJECT IN|ALLOW|DENY|REJECT)\s+(.+)", main_part.strip())
        if m:
            idx, to_port, action, from_ip = m.groups()
            is_v6 = "(v6)" in to_port or "(v6)" in from_ip
            clean_port = to_port.replace("(v6)", "").strip()
            port_num = 0
            proto = "any"
            if "/" in clean_port:
                parts = clean_port.split("/")
                if parts[0].isdigit():
                    port_num = int(parts[0])
                proto = parts[1].lower()
            elif clean_port.isdigit():
                port_num = int(clean_port)

            result["rules"].append({
                "index": int(idx),
                "to_port": clean_port,
                "port": port_num,
                "proto": proto,
                "action": action.strip(),
                "from_ip": from_ip.strip(),
                "comment": comment,
                "is_v6": is_v6,
            })

    return result


def is_port_allowed(port: int, proto: str, ufw_status: Dict[str, Any]) -> bool:
    """Check if a port/protocol is currently allowed in UFW rules."""
    if not ufw_status.get("active", False):
        return True

    proto = proto.lower()
    for rule in ufw_status.get("rules", []):
        if rule.get("port") == port:
            rule_proto = rule.get("proto", "any").lower()
            if rule_proto in (proto, "any"):
                if "ALLOW" in rule.get("action", ""):
                    return True
    return False


def get_listening_sockets() -> List[Dict[str, Any]]:
    """Scan all active network sockets listening externally."""
    listening = []
    try:
        output = subprocess.check_output(["ss", "-tulpn", "-H"], text=True)
    except Exception as e:
        logger.error(f"Failed to execute ss: {e}")
        return listening

    status = get_ufw_status()

    seen_keys = set()
    for line in output.strip().split("\n"):
        parts = line.split()
        if len(parts) < 5:
            continue
        proto = parts[0].lower()
        state = parts[1]
        local_addr = parts[4]

        # Filter listening sockets
        if proto == "tcp" and state != "LISTEN":
            continue
        if proto == "udp" and state != "UNCONN":
            continue

        # Skip localhost / loopback interfaces (safe, internal)
        if local_addr.startswith("127.") or local_addr.startswith("[::1]") or "127.0.0." in local_addr:
            continue

        port_str = local_addr.rsplit(":", 1)[-1]
        try:
            port = int(port_str)
        except ValueError:
            continue

        # Skip system core networking protocols (DHCP, LLMNR, DNS client, NTP)
        if port in SYSTEM_NETWORK_PORTS:
            continue

        # Skip ephemeral UDP client sockets (WebRTC, DNS queries, QUIC client)
        if proto == "udp" and port >= 32768:
            continue

        # Extract PID and process name from ss line
        pid = None
        m_pid = re.search(r"pid=(\d+)", line)
        if m_pid:
            pid = int(m_pid.group(1))

        ss_proc = ""
        m_proc = re.search(r'users:\(\(\"([^\"]+)\"', line)
        if m_proc:
            ss_proc = m_proc.group(1)

        unique_key = (port, proto)
        if unique_key in seen_keys:
            continue
        seen_keys.add(unique_key)

        app_info = resolve_process_app_info(pid, ss_proc)

        # Skip browser and client outbound traffic
        bin_low = app_info["binary"].lower()
        name_low = app_info["name"].lower()
        if any(ign in bin_low or ign in name_low or ign == ss_proc.lower() for ign in IGNORED_CLIENT_APPS):
            if proto == "udp":
                continue

        allowed = is_port_allowed(port, proto, status)

        listening.append({
            "port": port,
            "proto": proto,
            "local_addr": local_addr,
            "pid": pid,
            "app_name": app_info["name"],
            "binary": app_info["binary"],
            "cmdline": app_info["cmdline"],
            "icon": app_info["icon"],
            "is_allowed": allowed,
        })

    # Sort by port
    listening.sort(key=lambda x: x["port"])
    return listening
