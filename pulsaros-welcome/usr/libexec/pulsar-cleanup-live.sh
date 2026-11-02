#!/bin/bash
# Pulsar OS — live user cleanup (runs as root on first login of the created user)

# CRITICAL SAFETY: Never delete live user if OOTB setup is still pending!
if [ -f /etc/pulsar-need-setup ]; then
    exit 0
fi

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

# Remove "Install Pulsar OS" desktop shortcuts from all desktops and skel
rm -f /home/*/Desktop/pulsaros-recovery.desktop /home/*/*/pulsaros-recovery.desktop /etc/skel/Desktop/pulsaros-recovery.desktop 2>/dev/null || true
rm -f /etc/xdg/autostart/pulsaros-recovery-desktop-setup.desktop 2>/dev/null || true

# Hide installer icon from application grid on installed system
if [ -f /usr/share/applications/pulsaros-recovery.desktop ]; then
    sed -i '/^NoDisplay=/d' /usr/share/applications/pulsaros-recovery.desktop 2>/dev/null || true
    echo "NoDisplay=true" >> /usr/share/applications/pulsaros-recovery.desktop 2>/dev/null || true
fi

exit 0
