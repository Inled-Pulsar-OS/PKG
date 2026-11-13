#!/bin/bash
# Quick loop: installs the newest locally built pulsaros-airpods package in the test VM, then logs out so the Shell
# and the user unit pick it up at the next login (SDDM logs `live` straight back in).
# Host: copy the package into pulsaros-airpods/vm-test/ (gitignored) and run the host file server.
# Usage in the guest:  curl -fsS 10.0.2.2:8000/vm-test/vm-install.sh | bash
set -e
H=http://10.0.2.2:8000
PKG=$(curl -fsS "$H/vm-test/" | grep -o 'pulsaros-airpods-[^"<>]*\.pkg\.tar\.zst' | sort -V | tail -1)
[ -n "$PKG" ] || { echo "no package found under $H/vm-test/"; exit 1; }
echo "installing $PKG"
curl -fsS -o "/tmp/$PKG" "$H/vm-test/$PKG"
sudo pacman -U --noconfirm "/tmp/$PKG"
echo "installed. Rebooting in 5 s (logout can land on a lock screen with no known password);"
echo "SDDM autologin brings you straight back to the desktop. Then run:"
echo "  curl -fsS 10.0.2.2:8000/vm-test/vm-report.sh | bash"
sleep 5
systemctl reboot
