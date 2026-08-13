#!/bin/bash
# ==============================================================================
# Pulsar OS - Nautilus GUI Chroot Test Launcher
# ==============================================================================
# Permite probar rápidamente la interfaz de Nautilus (GNOME Files) y sus plugins
# directamente dentro del chroot de Arch Linux en el display actual.
# ==============================================================================

set -e

# Detectar directorio chroot disponible
POSSIBLE_CHROOTS=(
    "/home/jaime/Documentos/pulsar/ISO/build/rootfs-target-stable-arch"
    "/home/jaime/Documentos/pulsar/ISO/build/rootfs-target-stable"
    "../ISO/build/rootfs-target-stable-arch"
    "../ISO/build/rootfs-target-stable"
)

CHROOT_DIR=""
for c in "${POSSIBLE_CHROOTS[@]}"; do
    if [ -d "$c/usr/bin" ]; then
        CHROOT_DIR="$(realpath "$c")"
        break
    fi
done

if [ -z "$CHROOT_DIR" ]; then
    echo "❌ Error: No se encontró un chroot compilado en ISO/build/rootfs-target-*"
    exit 1
fi

echo "================================================================="
echo "  Pulsar OS - Nautilus Chroot GUI Test Launcher"
echo "  Target Chroot: $CHROOT_DIR"
echo "================================================================="

if [ "$EUID" -ne 0 ]; then
    HOST_XAUTH="${XAUTHORITY:-$HOME/.Xauthority}"
    echo "🔑 Solicitando permisos vía pkexec..."
    exec pkexec "$0" "$DISPLAY" "$WAYLAND_DISPLAY" "$XDG_RUNTIME_DIR" "$XDG_SESSION_TYPE" "$HOST_XAUTH"
fi

HOST_DISPLAY="${1:-$DISPLAY}"
HOST_WAYLAND_DISPLAY="${2:-$WAYLAND_DISPLAY}"
HOST_XDG_RUNTIME_DIR="${3:-$XDG_RUNTIME_DIR}"
HOST_XDG_SESSION_TYPE="${4:-$XDG_SESSION_TYPE}"
HOST_XAUTH="${5:-$XAUTHORITY}"

# Detección automática de sockets gráficos si faltan
if [ -z "$HOST_XDG_RUNTIME_DIR" ]; then
    for uid_dir in /run/user/*; do
        [ -d "$uid_dir" ] && { HOST_XDG_RUNTIME_DIR="$uid_dir"; break; }
    done
fi

if [ -z "$HOST_WAYLAND_DISPLAY" ] && [ -d "$HOST_XDG_RUNTIME_DIR" ]; then
    for sock in "$HOST_XDG_RUNTIME_DIR"/wayland-*; do
        [ -S "$sock" ] && { HOST_WAYLAND_DISPLAY=$(basename "$sock"); break; }
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

HOST_DISPLAY="${HOST_DISPLAY:-:0}"
HOST_WAYLAND_DISPLAY="${HOST_WAYLAND_DISPLAY:-wayland-0}"
HOST_XDG_RUNTIME_DIR="${HOST_XDG_RUNTIME_DIR:-/run/user/1000}"
HOST_XDG_SESSION_TYPE="${HOST_XDG_SESSION_TYPE:-wayland}"

echo "Configuración gráfica capturada:"
echo "  - DISPLAY: $HOST_DISPLAY"
echo "  - WAYLAND_DISPLAY: $HOST_WAYLAND_DISPLAY"
echo "  - XDG_RUNTIME_DIR: $HOST_XDG_RUNTIME_DIR"

if command -v xhost &>/dev/null; then
    xhost +local: || true
fi

cleanup() {
    echo "🧹 Desmontando puntos del chroot..."
    umount -l "$CHROOT_DIR/tmp/.Xauthority" 2>/dev/null || true
    umount -l "$CHROOT_DIR/dev/pts" 2>/dev/null || true
    umount -l "$CHROOT_DIR/dev" 2>/dev/null || true
    umount -l "$CHROOT_DIR/proc" 2>/dev/null || true
    umount -l "$CHROOT_DIR/sys" 2>/dev/null || true
    umount -l "$CHROOT_DIR/run" 2>/dev/null || true
    echo "✅ Limpieza completada."
}
trap cleanup EXIT

echo "⚙️ Montando sistemas de archivos virtuales..."
mkdir -p "$CHROOT_DIR/dev/pts"
mount --bind /dev "$CHROOT_DIR/dev"
mount -t devpts devpts "$CHROOT_DIR/dev/pts"
mount --bind /proc "$CHROOT_DIR/proc"
mount --bind /sys "$CHROOT_DIR/sys"
mount --rbind /run "$CHROOT_DIR/run"

if [ -f "$HOST_XAUTH" ]; then
    touch "$CHROOT_DIR/tmp/.Xauthority"
    mount --bind "$HOST_XAUTH" "$CHROOT_DIR/tmp/.Xauthority"
    chown 1000:1000 "$CHROOT_DIR/tmp/.Xauthority" || true
fi

# Inyectar el fix de Nautilus en la configuración GTK del usuario de prueba
echo "🎨 Aplicando el fix de Libadwaita en el chroot para la prueba..."
mkdir -p "$CHROOT_DIR/tmp/testuser/.config/gtk-4.0"
if [ -d "$CHROOT_DIR/etc/skel/.config/gtk-4.0" ]; then
    cp -rf "$CHROOT_DIR/etc/skel/.config/gtk-4.0/"* "$CHROOT_DIR/tmp/testuser/.config/gtk-4.0/" 2>/dev/null || true
fi

cat <<'EOF_NAUTILUS_FIX' >> "$CHROOT_DIR/tmp/testuser/.config/gtk-4.0/gtk.css"
/* Fix Nautilus Libadwaita */
.nautilus-window, #NautilusFileChooser { background-color: @window_bg_color; }
.nautilus-window .sidebar-pane, #NautilusFileChooser .sidebar-pane, .sidebar-pane, .content-pane .sidebar-pane, .sidebar-pane .content-pane {
    border-radius: 0 !important; margin: 0 !important; padding: 0 !important; box-shadow: none !important; background-color: @sidebar_bg_color !important; background-image: none !important;
}
.nautilus-window .sidebar-pane:dir(ltr), .nautilus-window .sidebar-pane:dir(rtl), .nautilus-window .sidebar-pane.end:dir(ltr), .nautilus-window .sidebar-pane.end:dir(rtl), #NautilusFileChooser .sidebar-pane:dir(ltr), #NautilusFileChooser .sidebar-pane:dir(rtl), #NautilusFileChooser .sidebar-pane.end:dir(ltr), #NautilusFileChooser .sidebar-pane.end:dir(rtl) { box-shadow: none !important; }
.nautilus-window headerbar, #NautilusFileChooser headerbar { background-color: transparent !important; box-shadow: none !important; margin: 0 !important; }
.nautilus-window headerbar > windowhandle > box > widget > box.start > stack > widget > box, .nautilus-window headerbar > windowhandle > box > widget > box.start > box > stack > widget > box, #NautilusFileChooser headerbar > windowhandle > box > widget > box.start > stack > widget > box, #NautilusFileChooser headerbar > windowhandle > box > widget > box.start > box > stack > widget > box { margin: 0 !important; padding: 0 !important; border-radius: 0 !important; background: none !important; background-image: none !important; box-shadow: none !important; }
.nautilus-window .content-pane, #NautilusFileChooser .content-pane { border-radius: 0 !important; background-color: @view_bg_color !important; box-shadow: none !important; }
.nautilus-window placessidebar .navigation-sidebar > row, #NautilusFileChooser placessidebar .navigation-sidebar > row { border-radius: 6px; margin: 1px 6px; }
.nautilus-window placessidebar .navigation-sidebar > row:selected, #NautilusFileChooser placessidebar .navigation-sidebar > row:selected { background-color: alpha(@accent_bg_color, 0.25) !important; color: @accent_fg_color !important; }
EOF_NAUTILUS_FIX

chown -R 1000:1000 "$CHROOT_DIR/tmp/testuser" || true

echo "🚀 Iniciando Nautilus desde el chroot..."
echo "-----------------------------------------------------------------"

chroot --userspec=1000:1000 "$CHROOT_DIR" env \
    HOME="/tmp/testuser" \
    DISPLAY="$HOST_DISPLAY" \
    XAUTHORITY="/tmp/.Xauthority" \
    WAYLAND_DISPLAY="$HOST_WAYLAND_DISPLAY" \
    XDG_RUNTIME_DIR="$HOST_XDG_RUNTIME_DIR" \
    XDG_SESSION_TYPE="$HOST_XDG_SESSION_TYPE" \
    XDG_CONFIG_HOME="/tmp/testuser/.config" \
    GTK_THEME="MacTahoe-Dark" \
    GDK_BACKEND="wayland,x11" \
    GSK_RENDERER="cairo" \
    nautilus --new-window /

echo "-----------------------------------------------------------------"
echo "🎉 Prueba de Nautilus finalizada."
