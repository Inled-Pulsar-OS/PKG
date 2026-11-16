#!/bin/bash
# Runs the baseline checks plus diagnostics inside the test VM. Every section is printed and posted to the host
# as soon as it finishes, and each command has a timeout, so a hang shows where it stopped.
# The VM has no shared clipboard; the host listens on 127.0.0.1:8000 (the guest reaches it at 10.0.2.2).
# Usage in the guest:  curl -fsS 10.0.2.2:8000/vm-test/vm-report.sh | bash
H=http://10.0.2.2:8000
section() { # title, command...
    local t="$1"; shift
    echo "=== $t ==="
    local out; out="$(timeout 40 "$@" 2>&1 < /dev/null)"; local rc=$?
    [ $rc -eq 124 ] && out="$out"$'\n'"(timed out after 40 s)"
    echo "$out"
    printf '=== %s (rc=%s) ===\n%s\n' "$t" "$rc" "$out" | curl -sS -m 10 --data-binary @- "$H/post" > /dev/null 2>&1 || echo "(post failed)"
}
section "baseline" bash -c "curl -fsS $H/vm-test/vm-baseline.sh | bash"
section "extension info" gnome-extensions info airpods@pulsaros.org
section "journal" bash -c "journalctl --user -b --no-pager | grep -i airpods | tail -20"
section "service" bash -c "systemctl --user status librepods --no-pager | head -15"
section "status.json" bash -c "cat ~/.local/state/librepods/status.json || echo absent"
section "pacman -Qk" bash -c "pacman -Qk pulsaros-airpods | head -5"
echo "done"
