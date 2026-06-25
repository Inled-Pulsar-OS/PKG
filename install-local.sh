#!/bin/bash
# ==============================================================================
# Pulsar OS - Global Menu Extension Local Installer
# ==============================================================================
# This script installs the global menu extension locally for testing.
#
# Este script instala la extensión de menú global en local para pruebas.
# ==============================================================================

# Exit on error
# Salir si ocurre un error
set -e

# Target directory for GNOME Shell extensions
# Directorio de destino para las extensiones de GNOME Shell
EXT_UUID="pulsaros-global-menu@inled.es"
LOCAL_EXT_DIR="$HOME/.local/share/gnome-shell/extensions"
TARGET_DIR="$LOCAL_EXT_DIR/$EXT_UUID"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/pulsaros-global-menu/usr/share/gnome-shell/extensions/$EXT_UUID"

echo "🚀 Installing $EXT_UUID locally..."
echo "🚀 Instalando $EXT_UUID localmente..."

# Create local extensions directory if it doesn't exist
# Crear el directorio local de extensiones si no existe
mkdir -p "$LOCAL_EXT_DIR"

# Clean up previous local installations
# Limpiar instalaciones locales anteriores
if [ -d "$TARGET_DIR" ]; then
    echo "🧹 Removing existing local installation..."
    echo "🧹 Eliminando instalación local existente..."
    rm -rf "$TARGET_DIR"
fi

# Copy extension files to target directory
# Copiar los archivos de la extensión al directorio de destino
echo "📂 Copying files to: $TARGET_DIR"
echo "📂 Copiando archivos a: $TARGET_DIR"
cp -r "$SRC_DIR" "$TARGET_DIR"

# Enable the extension via gnome-extensions CLI
# Habilitar la extensión mediante la CLI de gnome-extensions
echo "🔌 Enabling extension..."
echo "🔌 Habilitando la extensión..."
gnome-extensions enable "$EXT_UUID" || {
    echo "⚠️  Could not enable automatically (GNOME Shell might need to reload first)."
    echo "⚠️  No se pudo habilitar automáticamente (puede que GNOME Shell necesite recargarse primero)."
}

echo "=============================================================================="
echo "✅ Installation complete! / ¡Instalación completa!"
echo "=============================================================================="
echo "ℹ️  NOTE (Wayland):"
echo "   Since you are on Wayland, you must log out and log back in to reload GNOME Shell"
echo "   so it detects the new extension."
echo ""
echo "ℹ️  NOTA (Wayland):"
echo "   Dado que estás en Wayland, debes cerrar sesión e iniciar sesión de nuevo para"
echo "   recargar GNOME Shell y que detecte la nueva extensión."
echo "=============================================================================="
