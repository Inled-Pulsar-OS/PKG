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

# Determine partition naming (e.g. /dev/nvme0n1p1 vs /dev/sda1)
if [[ "$DISK" =~ [0-9]$ ]]; then
    PART_EFI="${DISK}p1"
    PART_ROOT="${DISK}p2"
else
    PART_EFI="${DISK}1"
    PART_ROOT="${DISK}2"
fi

log "Starting installation to $DISK (EFI: $PART_EFI, Root: $PART_ROOT)"
log "Partitioning $DISK"

# Unmount existing mounts if any
umount -f "$PART_EFI" 2>/dev/null || true
umount -f "$PART_ROOT" 2>/dev/null || true
umount -f /mnt/boot/efi 2>/dev/null || true
umount -f /mnt 2>/dev/null || true

# Partition
parted -s "$DISK" mklabel gpt
parted -s "$DISK" mkpart ESP fat32 1MiB 513MiB
parted -s "$DISK" set 1 esp on
parted -s "$DISK" mkpart root ext4 513MiB 100%
partprobe "$DISK" 2>/dev/null || true
udevadm settle 2>/dev/null || sleep 2
log "Partitioning done"

# Format
mkfs.vfat -F32 "$PART_EFI"
mkfs.ext4 -F -L tubeos-root "$PART_ROOT"
log "Filesystems created"

# Mount
mount "$PART_ROOT" /mnt
mkdir -p /mnt/boot/efi
mount "$PART_EFI" /mnt/boot/efi
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

run_chroot() {
    if command -v arch-chroot >/dev/null 2>&1; then
        arch-chroot /mnt "$@"
    else
        chroot /mnt "$@"
    fi
}

# Bootloader
if [ -d /sys/firmware/efi ]; then
    run_chroot grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=TubeOS --recheck 2>/dev/null || true
else
    run_chroot grub-install --target=i386-pc "${DISK}" --recheck 2>/dev/null || true
fi
run_chroot update-grub 2>/dev/null || run_chroot grub-mkconfig -o /boot/grub/grub.cfg 2>/dev/null || true
log "Bootloader installed"

# Fstab
if command -v genfstab >/dev/null 2>&1; then
    genfstab -U /mnt >> /mnt/etc/fstab
else
    ROOT_UUID=$(blkid -s UUID -o value "$PART_ROOT" || true)
    EFI_UUID=$(blkid -s UUID -o value "$PART_EFI" || true)
    cat <<EOF >> /mnt/etc/fstab
UUID=$ROOT_UUID / ext4 errors=remount-ro 0 1
UUID=$EFI_UUID /boot/efi vfat umask=0077 0 2
EOF
fi
log "fstab generated"

# Hostname
echo "$HOSTNAME" > /mnt/etc/hostname

# Enable services
run_chroot systemctl enable NetworkManager docker tubeos-gateway tubeos-message-bus tubeos-user-service tubeos-local-storage tubeos-app-management tubeos 2>/dev/null || true
log "Services enabled"

# OOTB flag
mkdir -p /mnt/var/lib/tubeos
touch /mnt/var/lib/tubeos/need-ootb
log "OOTB flag set"

# Cleanup
umount /mnt/dev/pts /mnt/dev /mnt/sys /mnt/proc 2>/dev/null || true
umount /mnt/boot/efi /mnt 2>/dev/null || true

log "INSTALL_DONE"
