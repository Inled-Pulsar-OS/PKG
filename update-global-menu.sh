#!/bin/bash
# ==============================================================================
# Pulsar OS - Global Menu Extension Updater & Installer
# ==============================================================================
# Script to update/install the global menu extension and configure the OS version
# (e.g., 1.1-unstable) displayed in "About Pulsar OS".
#
# Uso:
#   ./update-global-menu.sh [VERSION]
# Ejemplo:
#   ./update-global-menu.sh 1.1-unstable
# ==============================================================================

set -e

# Version parameter (default: 1.1-unstable)
VERSION="${1:-1.1-unstable}"
EXT_UUID="pulsaros-global-menu@inled.es"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -d "$SCRIPT_DIR/pulsaros-global-menu" ]; then
    SRC_DIR="$SCRIPT_DIR/pulsaros-global-menu/usr/share/gnome-shell/extensions/$EXT_UUID"
    PKG_ROOT="$SCRIPT_DIR"
elif [ -d "$SCRIPT_DIR/PKG/pulsaros-global-menu" ]; then
    SRC_DIR="$SCRIPT_DIR/PKG/pulsaros-global-menu/usr/share/gnome-shell/extensions/$EXT_UUID"
    PKG_ROOT="$SCRIPT_DIR/PKG"
else
    SRC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)/pulsaros-global-menu/usr/share/gnome-shell/extensions/$EXT_UUID"
    PKG_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
fi

LOCAL_EXT_DIR="$HOME/.local/share/gnome-shell/extensions"
TARGET_DIR="$LOCAL_EXT_DIR/$EXT_UUID"

echo "=============================================================================="
echo "🍎 Pulsar OS - Actualizador de Global Menu & Versión de Sistema"
echo "   Pulsar OS - Global Menu & OS Version Updater"
echo "=============================================================================="
echo "📌 Versión objetivo / Target version: $VERSION"
echo ""

# 1. Compilar schemas GLib
if [ -d "$SRC_DIR/schemas" ]; then
    echo "⚙️ Compilando GSettings schemas..."
    glib-compile-schemas "$SRC_DIR/schemas"
fi

# 2. Instalar / actualizar en ~/.local/share/gnome-shell/extensions
echo "📂 Instalando extensión en: $TARGET_DIR"
mkdir -p "$LOCAL_EXT_DIR"
rm -rf "$TARGET_DIR"
cp -r "$SRC_DIR" "$TARGET_DIR"

# 3. Actualizar /etc/pulsar-version o /etc/os-release si se tienen permisos
if [ -w /etc/pulsar-version ] 2>/dev/null; then
    echo "$VERSION" > /etc/pulsar-version
    echo "✅ /etc/pulsar-version actualizado a: $VERSION"
elif [ -w /etc/os-release ] 2>/dev/null; then
    sed -i "s/^VERSION_ID=.*/VERSION_ID=\"$VERSION\"/" /etc/os-release
    sed -i "s/^VERSION=.*/VERSION=\"$VERSION\"/" /etc/os-release
    sed -i "s/^PRETTY_NAME=.*/PRETTY_NAME=\"Pulsar OS Bitten Fruit Arch Based $VERSION\"/" /etc/os-release
    echo "✅ /etc/os-release actualizado con versión: $VERSION"
elif command -v pkexec >/dev/null 2>&1 && [ -n "$DISPLAY" -o -n "$WAYLAND_DISPLAY" ]; then
    echo "ℹ️ Configurando versión $VERSION en el sistema (/etc/pulsar-version)..."
    pkexec bash -c "echo '$VERSION' > /etc/pulsar-version" 2>/dev/null || true
fi

# 4. Actualizar copia en /usr/share si tenemos permisos
if [ -d "/usr/share/gnome-shell/extensions/$EXT_UUID" ] && [ -w "/usr/share/gnome-shell/extensions/$EXT_UUID" ]; then
    echo "📂 Actualizando también en /usr/share/gnome-shell/extensions..."
    cp -r "$SRC_DIR"/* "/usr/share/gnome-shell/extensions/$EXT_UUID/" 2>/dev/null || true
fi

# 5. Recargar la extensión en GNOME Shell
echo "🔄 Recargando extensión en GNOME Shell..."
gnome-extensions disable "$EXT_UUID" 2>/dev/null || true
sleep 0.3
gnome-extensions enable "$EXT_UUID" 2>/dev/null || true

# Intentar recarga por DBus si está disponible
gdbus call --session --dest org.gnome.Shell.Extensions --object-path /org/gnome/Shell/Extensions --method org.gnome.Shell.Extensions.ReloadExtension "$EXT_UUID" 2>/dev/null || true

echo ""
echo "=============================================================================="
echo "✅ ¡Actualización completada con éxito!"
echo "   Extensión actualizada y configurada para mostrar versión: $VERSION"
echo ""
echo "ℹ️ Si los cambios de JavaScript no se reflejan de inmediato en Wayland,"
echo "   puedes reiniciar sesión para una recarga completa de GNOME Shell."
echo "=============================================================================="
