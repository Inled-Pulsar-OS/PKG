#!/bin/bash
# Runs the baseline checks plus diagnostics inside the test VM, prints them, and posts them to the host.
# The VM has no shared clipboard; the host listens on 127.0.0.1:8000 (the guest reaches it at 10.0.2.2).
# Usage in the guest:  curl -fsS 10.0.2.2:8000/vm-test/vm-report.sh | bash
OUT=/tmp/vm-report.txt
{
    curl -fsS http://10.0.2.2:8000/vm-test/vm-baseline.sh | bash
    echo; echo "=== extension info ==="; gnome-extensions info airpods@pulsaros.org
    echo "=== journal ==="; journalctl --user -b --no-pager | grep -i airpods | tail -20
    echo "=== service ==="; systemctl --user status librepods --no-pager | head -15
    echo "=== status.json ==="; cat ~/.local/state/librepods/status.json 2>/dev/null || echo absent
    echo "=== pacman -Qk ==="; pacman -Qk pulsaros-airpods 2>&1 | head -5
} > "$OUT" 2>&1 < /dev/null
cat "$OUT"
echo
echo -n "posted to host: "
curl -sS --data-binary @"$OUT" http://10.0.2.2:8000/post || echo "POST FAILED (the output above is still on screen)"
