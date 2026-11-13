#!/bin/bash
# Runs the baseline checks plus diagnostics inside the test VM and posts everything to the host receiver
# (host: a listener on 127.0.0.1:8001; the guest reaches the host at 10.0.2.2). The VM has no shared clipboard.
# Usage in the guest:  curl -fsS 10.0.2.2:8000/vm-test/vm-report.sh | bash
{
    curl -fsS http://10.0.2.2:8000/vm-test/vm-baseline.sh | bash
    echo; echo "=== extension info ==="; gnome-extensions info airpods@pulsaros.org
    echo "=== journal ==="; journalctl --user -b --no-pager | grep -i airpods | tail -20
    echo "=== service ==="; systemctl --user status librepods --no-pager | head -15
    echo "=== status.json ==="; cat ~/.local/state/librepods/status.json 2>/dev/null || echo absent
    echo "=== pacman -Qk ==="; pacman -Qk pulsaros-airpods 2>&1 | head -5
} 2>&1 | curl -fsS --data-binary @- http://10.0.2.2:8001/
