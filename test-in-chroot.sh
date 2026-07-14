#!/bin/bash
# ==============================================================================
# Pulsar OS - OOTB Setup Assistant Chroot GUI Test Launcher
# ==============================================================================

# Strict mode
set -e

CHROOT_DIR="/home/jaime/Documentos/pulsar/ISO/build/rootfs-target-stable"
LOCAL_APP_PATH="/home/jaime/Documentos/pulsar/PKG/pulsaros-welcome/usr/share/pulsaros/welcome_ootb.py"
CHROOT_APP_PATH="/usr/share/pulsaros/welcome_ootb.py"

echo "================================================================="
echo "  Pulsar OS - welcome_ootb.py GUI Chroot Test Environment"
echo "================================================================="

# Ask the user for execution mode before escalating permissions
EXEC_MODE="1" # Default to simulation
if [ "$EUID" -ne 0 ]; then
    echo "Select execution mode for chroot testing:"
    echo "  1) Simulation Mode (Safe mock, skips system changes) [Default]"
    echo "  2) Real Mode (Performs actual user creation and configs in chroot)"
    read -r -p "Option (1-2) [1]: " choice
    if [ "$choice" = "2" ]; then
        EXEC_MODE="0"
        echo "⚠️ Running in REAL MODE. System changes will be written inside the chroot."
    else
        echo "ℹ️ Running in SIMULATION MODE. Safe mocks enabled."
    fi
    # Locate Xauthority file
    HOST_XAUTH="${XAUTHORITY:-$HOME/.Xauthority}"
    echo "🔑 Requesting root permissions via pkexec..."
    exec pkexec "$0" "$DISPLAY" "$WAYLAND_DISPLAY" "$XDG_RUNTIME_DIR" "$XDG_SESSION_TYPE" "$HOST_XAUTH" "$EXEC_MODE"
fi

# Recover host environment variables and execution mode from arguments
HOST_DISPLAY="$1"
HOST_WAYLAND_DISPLAY="$2"
HOST_XDG_RUNTIME_DIR="$3"
HOST_XDG_SESSION_TYPE="$4"
HOST_XAUTH="$5"
TEST_MODE_VAL="${6:-1}"

# Fallback to local values if not passed
HOST_DISPLAY="${HOST_DISPLAY:-$DISPLAY}"
HOST_WAYLAND_DISPLAY="${HOST_WAYLAND_DISPLAY:-$WAYLAND_DISPLAY}"
HOST_XDG_RUNTIME_DIR="${HOST_XDG_RUNTIME_DIR:-$XDG_RUNTIME_DIR}"
HOST_XDG_SESSION_TYPE="${HOST_XDG_SESSION_TYPE:-$XDG_SESSION_TYPE}"

# Auto-detect active GUI session sockets if variables are still empty (e.g. run with pkexec directly)
if [ -z "$HOST_XDG_RUNTIME_DIR" ]; then
    for uid_dir in /run/user/*; do
        if [ -d "$uid_dir" ]; then
            HOST_XDG_RUNTIME_DIR="$uid_dir"
            break
        fi
    done
fi

if [ -z "$HOST_WAYLAND_DISPLAY" ] && [ -d "$HOST_XDG_RUNTIME_DIR" ]; then
    for sock in "$HOST_XDG_RUNTIME_DIR"/wayland-*; do
        if [ -S "$sock" ]; then
            HOST_WAYLAND_DISPLAY=$(basename "$sock")
            break
        fi
    done
fi

if [ -z "$HOST_DISPLAY" ]; then
    for x11 in /tmp/.X11-unix/X*; do
        if [ -S "$x11" ]; then
            num=$(basename "$x11" | sed 's/X//')
            HOST_DISPLAY=":$num"
            break
        fi
    done
fi

# Auto-detect Xauthority if empty
if [ -z "$HOST_XAUTH" ]; then
    if [ -n "$XAUTHORITY" ]; then
        HOST_XAUTH="$XAUTHORITY"
    elif [ -f "/home/jaime/.Xauthority" ]; then
        HOST_XAUTH="/home/jaime/.Xauthority"
    elif [ -f "/root/.Xauthority" ]; then
        HOST_XAUTH="/root/.Xauthority"
    fi
fi

# Set ultimate defaults if detection failed
HOST_DISPLAY="${HOST_DISPLAY:-:0}"
HOST_WAYLAND_DISPLAY="${HOST_WAYLAND_DISPLAY:-wayland-0}"
HOST_XDG_RUNTIME_DIR="${HOST_XDG_RUNTIME_DIR:-/run/user/1000}"
HOST_XDG_SESSION_TYPE="${HOST_XDG_SESSION_TYPE:-wayland}"

echo "Display Variables Captured:"
echo "  - DISPLAY: $HOST_DISPLAY"
echo "  - WAYLAND_DISPLAY: $HOST_WAYLAND_DISPLAY"
echo "  - XDG_RUNTIME_DIR: $HOST_XDG_RUNTIME_DIR"
echo "  - XDG_SESSION_TYPE: $HOST_XDG_SESSION_TYPE"
echo "  - TEST_MODE: $TEST_MODE_VAL"

if [ ! -d "$CHROOT_DIR" ]; then
    echo "❌ Error: Target chroot directory does not exist:"
    echo "   $CHROOT_DIR"
    echo "Please build or bootstrap the target image first."
    exit 1
fi

echo "🔄 Preparing X11/Wayland authorization on host..."
if command -v xhost &>/dev/null; then
    xhost +local: || true
fi

# Function to clean up mounts on exit
cleanup() {
    echo "🧹 Cleaning up chroot mounts..."
    umount -l "$CHROOT_DIR/usr/share/themes" 2>/dev/null || true
    umount -l "$CHROOT_DIR/usr/share/icons" 2>/dev/null || true
    umount -l "$CHROOT_DIR/usr/share/fonts" 2>/dev/null || true
    umount -l "$CHROOT_DIR/usr/share/pixmaps" 2>/dev/null || true
    umount -l "$CHROOT_DIR/etc/resolv.conf" 2>/dev/null || true
    umount -l "$CHROOT_DIR/etc/resolv.conf" 2>/dev/null || true
    umount -l "$CHROOT_DIR$CHROOT_APP_PATH" 2>/dev/null || true
    umount -l "$CHROOT_DIR/tmp/.Xauthority" 2>/dev/null || true
    umount -l "$CHROOT_DIR/dev/pts" 2>/dev/null || true
    umount -l "$CHROOT_DIR/dev" 2>/dev/null || true
    umount -l "$CHROOT_DIR/proc" 2>/dev/null || true
    umount -l "$CHROOT_DIR/sys" 2>/dev/null || true
    umount -l "$CHROOT_DIR/run" 2>/dev/null || true
    echo "✅ Cleanup complete."
}
trap cleanup EXIT

echo "⚙️ Mounting API virtual filesystems..."
mkdir -p "$CHROOT_DIR/dev/pts"
mount --bind /dev "$CHROOT_DIR/dev"
mount -t devpts devpts "$CHROOT_DIR/dev/pts"
mount --bind /proc "$CHROOT_DIR/proc"
mount --bind /sys "$CHROOT_DIR/sys"
mount --rbind /run "$CHROOT_DIR/run"
mount --bind /etc/resolv.conf "$CHROOT_DIR/etc/resolv.conf"

# Share host Xauthority cookie inside chroot
if [ -f "$HOST_XAUTH" ]; then
    echo "🔑 Exposing host Xauthority cookie to chroot..."
    touch "$CHROOT_DIR/tmp/.Xauthority"
    mount --bind "$HOST_XAUTH" "$CHROOT_DIR/tmp/.Xauthority"
    chown 1000:1000 "$CHROOT_DIR/tmp/.Xauthority" || true
fi

# Bind mount the host's styling environment to guarantee 100% accurate look-and-feel
echo "🎨 Bind mounting host themes, icons, fonts, and pixmaps into chroot..."
mkdir -p "$CHROOT_DIR/usr/share/themes" "$CHROOT_DIR/usr/share/icons" "$CHROOT_DIR/usr/share/fonts" "$CHROOT_DIR/usr/share/pixmaps"
mount --bind /usr/share/themes "$CHROOT_DIR/usr/share/themes"
mount --bind /usr/share/icons "$CHROOT_DIR/usr/share/icons"
mount --bind /usr/share/fonts "$CHROOT_DIR/usr/share/fonts"
mount --bind /usr/share/pixmaps "$CHROOT_DIR/usr/share/pixmaps"

# Bind mount the local welcome_ootb.py so that local edits are tested directly inside chroot
echo "🔗 Bind mounting local welcome_ootb.py file into chroot..."
mkdir -p "$(dirname "$CHROOT_DIR$CHROOT_APP_PATH")"
touch "$CHROOT_DIR$CHROOT_APP_PATH"
mount --bind "$LOCAL_APP_PATH" "$CHROOT_DIR$CHROOT_APP_PATH"

echo "🚀 Launching OOTB Setup Assistant inside chroot..."
echo "-----------------------------------------------------------------"

# Run inside chroot as UID/GID 1000 (owner of host sockets) with captured display variables and software cairo renderer
chroot --userspec=1000:1000 "$CHROOT_DIR" env \
    HOME="/tmp" \
    DISPLAY="$HOST_DISPLAY" \
    XAUTHORITY="/tmp/.Xauthority" \
    WAYLAND_DISPLAY="$HOST_WAYLAND_DISPLAY" \
    XDG_RUNTIME_DIR="$HOST_XDG_RUNTIME_DIR" \
    XDG_SESSION_TYPE="$HOST_XDG_SESSION_TYPE" \
    XDG_CONFIG_HOME="/etc/skel/.config" \
    GTK_THEME="MacTahoe-Dark" \
    GDK_BACKEND="wayland,x11" \
    GSK_RENDERER="cairo" \
    GDK_GL="disable" \
    TEST_MODE="$TEST_MODE_VAL" \
    python3 "$CHROOT_APP_PATH"

echo "-----------------------------------------------------------------"
echo "🎉 Chroot execution ended."
