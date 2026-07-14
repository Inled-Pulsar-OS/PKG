#!/bin/bash
# ==============================================================================
# Pulsar OS - Bootsound Chroot Test Launcher
# ==============================================================================
set -e

CHROOT_DIR="/home/jaime/Documentos/pulsar/ISO/build/rootfs-target-stable"
SOUND_FILE="/usr/share/extras/boot-sound.wav"

if [ "$EUID" -ne 0 ]; then
    echo "🔑 Requesting root permissions via pkexec..."
    exec pkexec "$0" "$@"
fi

cleanup() {
    echo "🧹 Cleaning up mounts..."
    umount -l "$CHROOT_DIR/dev/pts" 2>/dev/null || true
    umount -l "$CHROOT_DIR/dev" 2>/dev/null || true
    umount -l "$CHROOT_DIR/proc" 2>/dev/null || true
    umount -l "$CHROOT_DIR/sys" 2>/dev/null || true
    echo "✅ Cleanup complete."
}
trap cleanup EXIT

echo "⚙️ Mounting API virtual filesystems..."
mkdir -p "$CHROOT_DIR/dev/pts"
mount --bind /dev "$CHROOT_DIR/dev"
mount -t devpts devpts "$CHROOT_DIR/dev/pts"
mount --bind /proc "$CHROOT_DIR/proc"
mount --bind /sys "$CHROOT_DIR/sys"

echo "🎵 Listing available ALSA playback devices inside chroot:"
chroot "$CHROOT_DIR" aplay -l || echo "⚠️ aplay -l failed. Is alsa-utils installed?"

echo "🎵 Testing smart audio detection and playback inside chroot..."

# Run the detection logic inside the chroot
chroot "$CHROOT_DIR" /bin/bash -c "
# 1. Try to find card by looking for Analog/Speaker/Headphone
target_card=\$(aplay -l | grep -i -E 'analog|speaker|headphone' | awk '{print \$2}' | sed 's/://' | head -n 1)

# 2. Fallback: exclude HDMI, Nvidia, DisplayPort, SPDIF
if [ -z \"\$target_card\" ]; then
    target_card=\$(aplay -l | grep -i 'card' | grep -v -E -i 'hdmi|nvidia|displayport|s/pdif' | awk '{print \$2}' | sed 's/://' | head -n 1)
fi

# 3. Fallback: first card
if [ -z \"\$target_card\" ]; then
    target_card=\$(aplay -l | grep -i 'card' | awk '{print \$2}' | sed 's/://' | head -n 1)
fi

if [ -n \"\$target_card\" ]; then
    echo '🔊 Selected target card: '\$target_card' (plughw:'\$target_card',0)'
    aplay -D plughw:\$target_card,0 -q $SOUND_FILE
else
    echo '🔊 No sound cards detected. Falling back to default.'
    aplay -q $SOUND_FILE
fi
"
