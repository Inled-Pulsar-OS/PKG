#!/bin/bash
# Baseline checks B1-B5 for pulsaros-airpods inside the Pulsar OS test VM (no Bluetooth needed).
# Run as the live user, in a terminal inside the GNOME session, after the package is installed:
#   curl -fsS http://10.0.2.2:8000/vm-test/vm-baseline.sh | bash
# with, on the host in pulsaros-airpods/: python -m http.server 8000 --bind 127.0.0.1
# Prints one PASS or FAIL line per check, then observations. Exit 1 when any check fails.
# B6 (fixture UI) and the real-AirPods layer are manual: see docs/vm-test-plan.md.
set -u

PKG=pulsaros-airpods
UUID=airpods@pulsaros.org
EXT=/usr/share/gnome-shell/extensions/$UUID
STATUS="${XDG_STATE_HOME:-$HOME/.local/state}/librepods/status.json"
rc=0

check() { # name, command...
    local n="$1"; shift
    if "$@" >/dev/null 2>&1; then echo "PASS $n"; else echo "FAIL $n"; rc=1; fi
}
no_missing_libs() { test -x /usr/bin/librepods && ! ldd /usr/bin/librepods | grep -q "not found"; }
unit_enabled() { [ -L /usr/lib/systemd/user/graphical-session.target.wants/librepods.service ] \
    && systemctl --user cat librepods.service >/dev/null; }
no_crash_loop() { # restart interval is 5 s: wait for ~3 restarts' worth, then allow at most 2
    systemctl --user cat librepods.service >/dev/null || return 1
    sleep 16
    [ "$(systemctl --user show librepods.service -p NRestarts --value)" -le 2 ]
}
ext_active() { gnome-extensions enable "$UUID" 2>/dev/null; gnome-extensions info "$UUID" | grep -Eq 'State: +(ACTIVE|ENABLED)'; }
ext_no_shell_errors() { ! journalctl --user -b --no-pager _COMM=gnome-shell 2>/dev/null \
    | grep -i "$UUID" | grep -Eiq 'error|exception|failed'; }
no_status_file() { [ ! -e "$STATUS" ]; }

check "B1 install: $PKG installed"                  pacman -Q "$PKG"
check "B1 install: pacman -Qk file integrity"       pacman -Qk "$PKG"
check "B2 binaries: /usr/bin/librepods executable"  test -x /usr/bin/librepods
check "B2 binaries: /usr/bin/librepods-ctl executable" test -x /usr/bin/librepods-ctl
check "B2 binaries: no missing shared libraries"    no_missing_libs
check "B3 unit: librepods.service enabled for the graphical session" unit_enabled
check "B3 unit: no crash loop (NRestarts <= 2 after 16 s)" no_crash_loop
check "B4 extension: listed by gnome-extensions"    bash -c "gnome-extensions list | grep -qx '$UUID'"
check "B4 extension: metadata lists Shell 51"       grep -q '"51"' "$EXT/metadata.json"
check "B4 extension: enabled and active"            ext_active
check "B4 extension: no errors in the Shell journal" ext_no_shell_errors
check "B5 no data: no status.json before any fixture" no_status_file

echo
echo "Observations (recorded, not asserted):"
echo "  librepods.service: $(systemctl --user show librepods.service -p ActiveState,SubState,Result,NRestarts --value 2>/dev/null | tr '\n' ' ')"
echo "  GNOME Shell: $(gnome-shell --version 2>/dev/null)   BlueZ: $(bluetoothd --version 2>/dev/null)"
echo "  Bluetooth controllers: $(bluetoothctl list 2>/dev/null | wc -l)"
echo
echo "MANUAL B5: confirm there is no AirPods icon in the top bar."
echo "MANUAL B6: follow docs/vm-test-plan.md (fixture UI), then the real-AirPods layer R1-R5."
exit $rc
