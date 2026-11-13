#!/bin/bash
# Quick loop: installs the newest locally built pulsaros-airpods package in the test VM.
# Host: copy the package into pulsaros-airpods/vm-test/ (gitignored) and run the host file server.
# Usage in the guest:  curl -fsS 10.0.2.2:8000/vm-test/vm-install.sh | bash
#
# This script never logs out, reboots, or restarts the Shell by itself: the live overlay is
# RAM-only, so either one throws away the install just made (same loss as closing the VM).
# It also found, on this VM, that a scripted logout can land on an unpassable lock screen.
# - librepods.service and the daemon binary pick up the new build right away (restarted below).
# - The GNOME Shell extension (ESM JS) is only reloaded by GNOME on logout/login, not by
#   disable+enable. To see an extension-only fix, log out yourself from the top-right menu
#   (Power icon > Log Out) and let SDDM's autologin bring `live` back; don't reboot the VM.
set -e
H=http://10.0.2.2:8000
PKG=$(curl -fsS "$H/vm-test/" | grep -o 'pulsaros-airpods-[^"<>]*\.pkg\.tar\.zst' | sort -V | tail -1)
[ -n "$PKG" ] || { echo "no package found under $H/vm-test/"; exit 1; }
echo "installing $PKG"
curl -fsS -o "/tmp/$PKG" "$H/vm-test/$PKG"
sudo pacman -U --noconfirm "/tmp/$PKG"
systemctl --user restart librepods.service
echo "installed. librepods.service restarted with the new build."
echo "To pick up an extension-only fix, log out via the top-right menu (not this script) and let autologin bring you back."
echo "Then run:  curl -fsS 10.0.2.2:8000/vm-test/vm-report.sh | bash"
