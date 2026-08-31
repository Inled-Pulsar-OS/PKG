#!/bin/bash
# Pulsar OS — live user cleanup (runs via pkexec, already root)
userdel -r live 2>/dev/null || true
rm -f /var/lib/AccountsService/users/live
rm -f /var/lib/AccountsService/icons/live
systemctl restart accounts-daemon 2>/dev/null || true
rm -f /etc/pulsar-need-cleanup
rm -f /etc/sudoers.d/pulsar-ootb-live /etc/sudoers.d/live /etc/sudoers.d/jaime 2>/dev/null || true
