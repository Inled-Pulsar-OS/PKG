#!/bin/bash
# Pulsar OS — live user cleanup (runs as root before display manager or on login)
if ! id -u live &>/dev/null && [ ! -f /etc/pulsar-need-cleanup ]; then
    exit 0
fi

pkill -9 -u live 2>/dev/null || true
sleep 1
userdel -r -f live 2>/dev/null || true
rm -rf /home/live
rm -f /var/lib/AccountsService/users/live
rm -f /var/lib/AccountsService/icons/live
systemctl restart accounts-daemon 2>/dev/null || true
rm -f /etc/pulsar-need-cleanup
rm -f /etc/sudoers.d/pulsar-ootb-live /etc/sudoers.d/live /etc/sudoers.d/jaime /etc/sudoers.d/pulsar-cleanup-live 2>/dev/null || true
systemctl disable pulsar-cleanup-live.service 2>/dev/null || true
exit 0
