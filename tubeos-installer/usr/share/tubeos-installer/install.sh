#!/bin/bash
# Tube OS installer - runs in background, writes progress to log
set -e

LOG="/tmp/tubeos-install.log"
MARKER="/tmp/tubeos-install.json"

log() { echo "$(date +%H:%M:%S) $1" >> "$LOG"; }

if [ ! -f "$MARKER" ]; then
    log "INSTALL_FAIL: No install marker found"
    exit 1
fi

DISK=$(python3 -c "import json; print(json.load(open('$MARKER'))['disk'])")
HOSTNAME=$(python3 -c "import json; print(json.load(open('$MARKER'))['hostname'])")

log "Starting installation to $DISK"
log "Partitioning $DISK"

# Partition
parted -s "$DISK" mklabel gpt
parted -s "$DISK" mkpart ESP fat32 1MiB 513MiB
parted -s "$DISK" set 1 esp on
parted -s "$DISK" mkpart root ext4 513MiB 100%
log "Partitioning done"

# Format
mkfs.vfat -F32 "${DISK}1"
mkfs.ext4 -F -L tubeos-root "${DISK}2"
log "Filesystems created"

# Mount
mount "${DISK}2" /mnt
mkdir -p /mnt/boot/efi
mount "${DISK}1" /mnt/boot/efi
log "Filesystems mounted"

# Copy system
rsync -ax --exclude='/dev/*' --exclude='/proc/*' --exclude='/sys/*' \
    --exclude='/tmp/*' --exclude='/run/*' --exclude='/mnt/*' \
    --exclude='/boot/efi/*' / /mnt/
log "System copied"

# Mount pseudo-filesystems for chroot
mount -t proc proc /mnt/proc
mount -t sysfs sys /mnt/sys
mount --bind /dev /mnt/dev
mount --bind /dev/pts /mnt/dev/pts

# Bootloader
if [ -d /sys/firmware/efi ]; then
    arch-chroot /mnt pacman -S --noconfirm grub efibootmgr 2>/dev/null || true
    arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=TubeOS 2>/dev/null || true
else
    arch-chroot /mnt pacman -S --noconfirm grub 2>/dev/null || true
    arch-chroot /mnt grub-install --target=i386-pc "${DISK}" 2>/dev/null || true
fi
arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg 2>/dev/null || true
log "Bootloader installed"

# Fstab
genfstab -U /mnt >> /mnt/etc/fstab
log "fstab generated"

# Hostname
echo "$HOSTNAME" > /mnt/etc/hostname

# Enable services
arch-chroot /mnt systemctl enable NetworkManager docker tubeos-gateway tubeos-message-bus tubeos-user-service tubeos-local-storage tubeos-app-management tubeos 2>/dev/null || true
log "Services enabled"

# OOTB flag
mkdir -p /mnt/var/lib/tubeos
touch /mnt/var/lib/tubeos/need-ootb
log "OOTB flag set"

# Cleanup
umount /mnt/dev/pts /mnt/dev /mnt/sys /mnt/proc 2>/dev/null || true
umount /mnt/boot/efi /mnt 2>/dev/null || true

log "INSTALL_DONE"
