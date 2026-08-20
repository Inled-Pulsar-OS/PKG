#!/usr/bin/env python3
"""Tube OS Web Installer & OOTB server.

Usage:
    tubeos-installer              # Live ISO - installation mode
    tubeos-installer --ootb       # Post-reboot - first-time setup mode
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional

from fastapi import FastAPI, Request
from fastapi.responses import HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles

app = FastAPI(title="Tube OS Installer")

ootb_mode = False
STATIC_DIR = Path(__file__).parent / "static"

# ─── Helpers ────────────────────────────────────────────────────────────────

def run(cmd, check=True):
    return subprocess.run(cmd, shell=True, capture_output=True, text=True, check=check)

def get_ip():
    try:
        r = run("ip -4 route get 1.1.1.1 2>/dev/null | awk '{print $7; exit}'")
        ip = r.stdout.strip()
        if ip:
            return ip
    except Exception:
        pass
    try:
        r = run("hostname -I")
        ips = r.stdout.strip().split()
        for ip in ips:
            if not ip.startswith("127."):
                return ip
    except Exception:
        pass
    return "localhost"

def gen_qr_svg(url: str) -> str:
    try:
        import qrcode
        import qrcode.image.svg
        factory = qrcode.image.svg.SvgPathImage
        img = qrcode.make(url, image_factory=factory)
        import io
        buf = io.BytesIO()
        img.save(buf)
        return buf.getvalue().decode("utf-8")
    except Exception:
        return ""

# ─── Disk helpers ───────────────────────────────────────────────────────────

def list_disks():
    disks = []
    try:
        r = run("lsblk -Jdpo NAME,SIZE,MODEL,RM,ROTA,TRAN 2>/dev/null")
        data = json.loads(r.stdout)
        for d in data.get("blockdevices", []):
            removable = d.get("rm", False) or d.get("removable", False)
            if removable and d.get("name", "").startswith("/dev/sr"):
                continue
            disks.append({
                "name": d["name"],
                "size": d.get("size", "?"),
                "model": (d.get("model") or "Unknown").strip(),
                "removable": bool(removable),
                "type": "usb" if removable else ((d.get("tran") or "sata").strip()),
            })
    except Exception:
        pass
    return disks

def install_to_disk(disk: str, hostname_val: str = "tubeos"):
    """Run the actual installation. Writes to disk."""
    steps = [
        ("Creating partition table", f"parted -s {disk} mklabel gpt"),
        ("Creating EFI partition", f"parted -s {disk} mkpart ESP fat32 1MiB 513MiB && parted -s {disk} set 1 esp on"),
        ("Creating root partition", f"parted -s {disk} mkpart root ext4 513MiB 100%"),
        ("Formatting EFI", f"mkfs.vfat -F32 {disk}1"),
        ("Formatting root", f"mkfs.ext4 -F -L tubeos-root {disk}2"),
        ("Mounting root", f"mount {disk}2 /mnt"),
        ("Mounting EFI", f"mkdir -p /mnt/boot/efi && mount {disk}1 /mnt/boot/efi"),
        ("Syncing system", "rsync -ax --exclude=/dev/* --exclude=/proc/* --exclude=/sys/* --exclude=/tmp/* --exclude=/run/* --exclude=/mnt/* /mnt/ /mnt_bak/ 2>/dev/null; rsync -ax / /mnt/ --exclude=/dev/* --exclude=/proc/* --exclude=/sys/* --exclude=/tmp/* --exclude=/run/* --exclude=/mnt/*"),
        ("Installing bootloader", "arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=TubeOS 2>/dev/null || true; arch-chroot /mnt update-grub 2>/dev/null || arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg 2>/dev/null || true"),
        ("Generating fstab", "genfstab -U /mnt >> /mnt/etc/fstab"),
        ("Configuring hostname", f"echo '{hostname_val}' > /mnt/etc/hostname"),
        ("Setting OOTB flag", "mkdir -p /mnt/var/lib/tubeos && touch /mnt/var/lib/tubeos/need-ootb"),
    ]
    total = len(steps)
    for i, (label, cmd) in enumerate(steps):
        yield label, i, total
        r = run(cmd, check=False)
        if r.returncode != 0 and "arch-chroot" not in cmd and "rsync" not in cmd:
            yield f"Error: {r.stderr.strip()}", -1, total
            return
    yield "Installation complete", total, total

# ─── OOTB helpers ───────────────────────────────────────────────────────────

def get_timezones():
    try:
        r = run("timedatectl list-timezones 2>/dev/null")
        return [tz.strip() for tz in r.stdout.strip().split("\n") if tz.strip()]
    except Exception:
        return ["UTC"]

def get_keyboards():
    keyboards = []
    try:
        r = run("localectl list-x11-keymaps 2>/dev/null")
        for line in r.stdout.strip().split("\n"):
            if line.strip():
                keyboards.append(line.strip())
    except Exception:
        pass
    if not keyboards:
        keyboards = ["us", "es", "de", "fr", "gb", "it", "pt", "latam"]
    return keyboards

def apply_timezone(tz: str):
    run(f"timedatectl set-timezone {tz}", check=False)
    run(f"ln -sf /usr/share/zoneinfo/{tz} /etc/localtime", check=False)

def apply_keyboard(kb: str):
    run(f"localectl set-keymap {kb}", check=False)

def create_user(username: str, password: str, fullname: str = ""):
    if not fullname:
        fullname = username
    admin_group = "sudo" if Path("/etc/debian_version").exists() else "wheel"
    run("groupadd -f docker", check=False)
    run("groupadd -f " + admin_group, check=False)
    run(f"useradd -m -s /bin/bash -G {admin_group},docker,audio,video {username}", check=False)
    p = subprocess.Popen(["chpasswd"], stdin=subprocess.PIPE, shell=False)
    p.communicate(input=f"{username}:{password}\n".encode())
    run(f"mkdir -p /etc/sudoers.d && echo '{username} ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/{username} && chmod 0440 /etc/sudoers.d/{username}", check=False)

def finalize_ootb():
    run("rm -f /var/lib/tubeos/need-ootb", check=False)
    run("systemctl enable tubeos-gateway tubeos-message-bus tubeos-user-service tubeos-local-storage tubeos-app-management tubeos 2>/dev/null", check=False)

# ─── Routes: static ────────────────────────────────────────────────────────

app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

@app.get("/", response_class=HTMLResponse)
async def index():
    return (STATIC_DIR / "index.html").read_text()

@app.get("/api/mode")
async def api_mode():
    return {"mode": "ootb" if ootb_mode else "installer"}

# ─── Routes: install mode ──────────────────────────────────────────────────

@app.get("/api/disks")
async def api_disks():
    return {"disks": list_disks()}

@app.get("/api/ip")
async def api_ip():
    ip = get_ip()
    url = f"http://{ip}"
    qr = gen_qr_svg(url)
    return {"ip": ip, "url": url, "qr_svg": qr}

@app.post("/api/install")
async def api_install(request: Request):
    body = await request.json()
    disk = body.get("disk", "")
    hostname_val = body.get("hostname", "tubeos")
    if not disk or not disk.startswith("/dev/"):
        return JSONResponse({"error": "Invalid disk"}, status_code=400)
    # Write a marker so the background install script can pick it up
    marker = Path("/tmp/tubeos-install.json")
    marker.write_text(json.dumps({"disk": disk, "hostname": hostname_val}))
    
    # Spawn install.sh in background if present
    install_sh = Path("/usr/share/tubeos-installer/install.sh")
    if not install_sh.exists():
        install_sh = Path(__file__).parent / "install.sh"
    
    if install_sh.exists():
        Path("/tmp/tubeos-install.log").write_text("")
        subprocess.Popen(["/bin/bash", str(install_sh)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    return {"status": "started"}

@app.get("/api/install/progress")
async def api_install_progress():
    marker = Path("/tmp/tubeos-install.json")
    log = Path("/tmp/tubeos-install.log")
    if not marker.exists():
        return {"state": "idle"}
    if log.exists():
        lines = log.read_text().strip().split("\n")
        last = lines[-1] if lines else ""
        if "INSTALL_DONE" in last:
            return {"state": "done"}
        if "INSTALL_FAIL" in last:
            return {"state": "error", "message": last}
        return {"state": "running", "message": last}
    return {"state": "starting"}

@app.post("/api/reboot")
async def api_reboot():
    run("reboot", check=False)
    return {"status": "rebooting"}

# ─── Routes: OOTB mode ─────────────────────────────────────────────────────

@app.get("/api/timezones")
async def api_timezones():
    return {"timezones": get_timezones()}

@app.get("/api/keyboards")
async def api_keyboards():
    return {"keyboards": get_keyboards()}

@app.post("/api/ootb/timezone")
async def api_ootb_timezone(request: Request):
    body = await request.json()
    tz = body.get("timezone", "UTC")
    apply_timezone(tz)
    return {"status": "ok"}

@app.post("/api/ootb/keyboard")
async def api_ootb_keyboard(request: Request):
    body = await request.json()
    kb = body.get("keyboard", "us")
    apply_keyboard(kb)
    return {"status": "ok"}

@app.post("/api/ootb/user")
async def api_ootb_user(request: Request):
    body = await request.json()
    username = body.get("username", "").strip()
    password = body.get("password", "")
    fullname = body.get("fullname", username)
    if not username or not password:
        return JSONResponse({"error": "Username and password required"}, status_code=400)
    if len(password) < 4:
        return JSONResponse({"error": "Password too short"}, status_code=400)
    create_user(username, password, fullname)
    return {"status": "ok", "dashboard_url": f"http://{get_ip()}"}

@app.post("/api/ootb/finish")
async def api_ootb_finish():
    finalize_ootb()
    return {"status": "ok"}

@app.get("/api/ootb/dockermigrate")
async def api_dockermigrate():
    ip = get_ip()
    return {"url": f"http://{ip}:8070"}

# ─── Main ───────────────────────────────────────────────────────────────────

def main():
    global ootb_mode
    parser = argparse.ArgumentParser(description="Tube OS Installer")
    parser.add_argument("--ootb", action="store_true", help="Run in OOTB mode (post-reboot setup)")
    parser.add_argument("--port", type=int, default=80, help="Listen port")
    parser.add_argument("--host", default="0.0.0.0", help="Listen address")
    args = parser.parse_args()

    ootb_mode = args.ootb

    if ootb_mode:
        print(f"Tube OS OOTB server listening on http://{args.host}:{args.port}")
    else:
        ip = get_ip()
        print(f"Tube OS Installer listening on http://{args.host}:{args.port}")
        print(f"Open http://{ip} from another device to install")

    import uvicorn
    uvicorn.run(app, host=args.host, port=args.port, log_level="info")

if __name__ == "__main__":
    main()
