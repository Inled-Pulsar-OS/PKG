#!/bin/bash
###############################################################################
# Pulsar HBlock - root helper
#
# Privilaged operations for the Pulsar HBlock app. This script is invoked by
# the app through pkexec, so it always runs as root. It handles:
#
#   status  -> print whether hblock manages /etc/hosts (for debugging)
#   enable  -> back up /etc/hosts (first time), enable hblock.timer, generate
#   disable -> stop timers/services, restore the backup or strip the block
#   update  -> regenerate /etc/hosts from hblock sources right now
###############################################################################
set -u

HOSTS=/etc/hosts
BACKUP=/etc/hosts.pulsar-hblock-backup

need_hblock() {
    if ! command -v hblock >/dev/null 2>&1; then
        echo "ERR: hblock binary not found. Install the 'hblock' package first." >&2
        echo "ERR: el binario de hblock no existe. Instala primero el paquete 'hblock'." >&2
        exit 3
    fi
}

hosts_managed() {
    grep -qE '^# Generated with hBlock|^# *BEGIN BLOCKLIST' "$HOSTS" 2>/dev/null
}

cmd_status() {
    if hosts_managed; then
        echo "managed"
    else
        echo "clean"
    fi
}

cmd_enable() {
    need_hblock
    # Keep the very first /etc/hosts so disabling later restores it exactly.
    if ! hosts_managed && [ ! -f "$BACKUP" ]; then
        cp -a "$HOSTS" "$BACKUP"
    fi
    systemctl enable --now hblock.timer >/dev/null 2>&1 || true
    systemctl start hblock.service >/dev/null 2>&1 || hblock >/dev/null 2>&1 || true
    if hosts_managed; then
        echo "OK enable"
    else
        echo "WARN: hblock ran but /etc/hosts shows no hblock markers." >&2
        echo "OK enable"
    fi
}

cmd_disable() {
    # Stop the scheduled and one-shot work immediately.
    systemctl disable --now hblock.timer hblock.service >/dev/null 2>&1 || true
    systemctl stop hblock.service hblock.timer >/dev/null 2>&1 || true

    if [ -f "$BACKUP" ]; then
        cp -a "$BACKUP" "$HOSTS"
        rm -f "$BACKUP"
    elif hosts_managed; then
        # No backup (hblock was enabled before the app was used): strip the
        # whole hblock-written section from /etc/hosts.
        sed -i '/^# Generated with hBlock/,/^# END FOOTER/d' "$HOSTS" 2>/dev/null \
            || sed -i '/^# Generated with hBlock/,$d' "$HOSTS" 2>/dev/null || true
        # Trim trailing blank lines left behind by the removal.
        sed -i -e :a -e '/^\n*$/{$d;N;ba' -e '}' "$HOSTS" 2>/dev/null || true
        # If nothing is left, put a minimal localhost file back.
        if ! grep -q . "$HOSTS" 2>/dev/null; then
            printf '127.0.0.1\tlocalhost\n::1\t\tlocalhost ip6-localhost ip6-loopback\n' > "$HOSTS"
        fi
    fi
    echo "OK disable"
}

cmd_update() {
    need_hblock
    # Tick the service so the OnCalendar/oneshot unit regenerates /etc/hosts.
    systemctl start hblock.service >/dev/null 2>&1 || hblock >/dev/null 2>&1 || true
    if hosts_managed; then
        echo "OK update"
    else
        echo "WARN: hblock ran but /etc/hosts shows no hblock markers." >&2
        echo "OK update"
    fi
}

case "${1:-}" in
    status)  cmd_status ;;
    enable)  cmd_enable ;;
    disable) cmd_disable ;;
    update)  cmd_update ;;
    *)
        echo "Usage: $0 {status|enable|disable|update}" >&2
        exit 2
        ;;
esac