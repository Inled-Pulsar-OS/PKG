#!/bin/bash
# ==============================================================================
# Pulsar OS - SDDM Login Theme Chroot GUI Test Launcher
# ==============================================================================

# Strict mode
set -e

CHROOT_DIR="/home/jaime/Documentos/pulsar/ISO/build/rootfs-target-stable"
LOCAL_THEME_PATH="/home/jaime/Documentos/pulsar/PKG/pulsaros-sddm/Apple.Tahoe"
CHROOT_THEME_PATH="/usr/share/sddm/themes/Apple.Tahoe"

echo "================================================================="
echo "  Pulsar OS - SDDM Greeter Tahoe Theme Chroot Test Launcher"
echo "================================================================="

# If not running as root, prepare authorization and escalate via pkexec
if [ "$EUID" -ne 0 ]; then
    echo "🔄 Preparing X11/Wayland authorization on host..."
    if command -v xhost &>/dev/null; then
        xhost +local: || true
    fi
    
    # Locate Xauthority file
    HOST_XAUTH="${XAUTHORITY:-$HOME/.Xauthority}"
    echo "🔑 Requesting root permissions via pkexec..."
    exec pkexec "$0" "$DISPLAY" "$WAYLAND_DISPLAY" "$XDG_RUNTIME_DIR" "$XDG_SESSION_TYPE" "$HOST_XAUTH"
fi

# Recover host environment variables from arguments
HOST_DISPLAY="$1"
HOST_WAYLAND_DISPLAY="$2"
HOST_XDG_RUNTIME_DIR="$3"
HOST_XDG_SESSION_TYPE="$4"
HOST_XAUTH="$5"

# Fallback to local values if not passed
HOST_DISPLAY="${HOST_DISPLAY:-$DISPLAY}"
HOST_WAYLAND_DISPLAY="${HOST_WAYLAND_DISPLAY:-$WAYLAND_DISPLAY}"
HOST_XDG_RUNTIME_DIR="${HOST_XDG_RUNTIME_DIR:-$XDG_RUNTIME_DIR}"
HOST_XDG_SESSION_TYPE="${HOST_XDG_SESSION_TYPE:-$XDG_SESSION_TYPE}"
HOST_XAUTH="${HOST_XAUTH:-$HOME/.Xauthority}"

echo "Display Variables Captured:"
echo "  - DISPLAY: $HOST_DISPLAY"
echo "  - XAUTHORITY File: $HOST_XAUTH"

if [ ! -d "$CHROOT_DIR" ]; then
    echo "❌ Error: Target chroot directory does not exist:"
    echo "   $CHROOT_DIR"
    echo "Please build or bootstrap the target image first."
    exit 1
fi

# Install required Qt6 libraries inside chroot if missing (required for xcb plugin, SVG graphics, and graphical effects)
if [ ! -f "$CHROOT_DIR/usr/lib/x86_64-linux-gnu/libxcb-cursor.so.0" ] || [ ! -f "$CHROOT_DIR/usr/lib/x86_64-linux-gnu/qt6/plugins/imageformats/libqsvg.so" ]; then
    echo "📦 Installing missing Qt6 dependencies (X11 cursor, SVG support, and graphics effects) inside chroot..."
    mkdir -p "$CHROOT_DIR/etc"
    mount --bind /etc/resolv.conf "$CHROOT_DIR/etc/resolv.conf" || true
    chroot "$CHROOT_DIR" apt-get update || true
    chroot "$CHROOT_DIR" apt-get install -y libxcb-cursor0 libqt6svg6 qt6-svg-plugins qml6-module-qt5compat-graphicaleffects || true
    umount -l "$CHROOT_DIR/etc/resolv.conf" || true
fi

# Function to clean up mounts on exit
cleanup() {
    echo "🧹 Cleaning up chroot mounts..."
    umount -l "$CHROOT_DIR/usr/share/themes" 2>/dev/null || true
    umount -l "$CHROOT_DIR/usr/share/icons" 2>/dev/null || true
    umount -l "$CHROOT_DIR/usr/share/fonts" 2>/dev/null || true
    umount -l "$CHROOT_DIR/usr/share/pixmaps" 2>/dev/null || true
    umount -R -l "$CHROOT_DIR$CHROOT_THEME_PATH" 2>/dev/null || true
    umount -l "$CHROOT_DIR/root/.Xauthority" 2>/dev/null || true
    umount -R -l "$CHROOT_DIR/dev" 2>/dev/null || true
    umount -R -l "$CHROOT_DIR/proc" 2>/dev/null || true
    umount -R -l "$CHROOT_DIR/sys" 2>/dev/null || true
    umount -l "$CHROOT_DIR/run" 2>/dev/null || true
    echo "✅ Cleanup complete."
}
trap cleanup EXIT

echo "⚙️ Mounting API virtual filesystems recursively (--rbind)..."
mount --rbind /dev "$CHROOT_DIR/dev"
mount --bind /proc "$CHROOT_DIR/proc"
mount --rbind /sys "$CHROOT_DIR/sys"
mount --rbind /run "$CHROOT_DIR/run"
mount --bind /etc/resolv.conf "$CHROOT_DIR/etc/resolv.conf"

# Bind mount the host's styling environment to guarantee 100% accurate look-and-feel
echo "🎨 Bind mounting host themes, icons, fonts, and pixmaps into chroot..."
mkdir -p "$CHROOT_DIR/usr/share/themes" "$CHROOT_DIR/usr/share/icons" "$CHROOT_DIR/usr/share/fonts" "$CHROOT_DIR/usr/share/pixmaps"
mount --bind /usr/share/themes "$CHROOT_DIR/usr/share/themes"
mount --bind /usr/share/icons "$CHROOT_DIR/usr/share/icons"
mount --bind /usr/share/fonts "$CHROOT_DIR/usr/share/fonts"
mount --bind /usr/share/pixmaps "$CHROOT_DIR/usr/share/pixmaps"

# Share host Xauthority cookie inside chroot
if [ -f "$HOST_XAUTH" ]; then
    echo "🔑 Exposing host Xauthority cookie to chroot..."
    touch "$CHROOT_DIR/root/.Xauthority"
    mount --bind "$HOST_XAUTH" "$CHROOT_DIR/root/.Xauthority"
fi

# Bind mount the local QML theme folder so that local edits are tested directly inside chroot
echo "🔗 Bind mounting local QML theme folder into chroot..."
mkdir -p "$CHROOT_DIR$CHROOT_THEME_PATH"
mount --bind "$LOCAL_THEME_PATH" "$CHROOT_DIR$CHROOT_THEME_PATH"

echo "🚀 Launching SDDM Greeter inside chroot in --test-mode..."
echo "-----------------------------------------------------------------"

# Run SDDM greeter inside chroot using X11 (xcb) with correct cursor libraries and auth
chroot "$CHROOT_DIR" env \
    DISPLAY="$HOST_DISPLAY" \
    XAUTHORITY="/root/.Xauthority" \
    QT_QPA_PLATFORM="xcb" \
    sddm-greeter-qt6 --test-mode --theme "$CHROOT_THEME_PATH"

echo "-----------------------------------------------------------------"
echo "🎉 Chroot execution ended."
