#!/bin/bash
# ==============================================================================
# Pulsar OS - GRUB Theme Asset Preparer
# ==============================================================================
# Descarga e instala en el paquete el tema de GRUB.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
TEMP_BUILD="/tmp/pulsaros-grub-build"
THEME_REPO="https://github.com/Inled-Pulsar-OS/grub.theme"

echo "🎨 Descargando tema de GRUB desde GitHub (depth=1)..."
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Clonar con depth=1
git clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"

# Copiar el tema a la ruta adecuada de staging para GRUB
DEST_DIR="$STAGE_DIR/boot/grub/themes/grub-theme"
mkdir -p "$DEST_DIR"
cp -r "$TEMP_BUILD/theme"/* "$DEST_DIR/"

# Instalar iconos Lucide personalizados para las entradas de boot de Pulsar OS
SCRIPT_DIR="$(dirname "$(realpath "$0")")"
GRUB_ICONS_DIR="$SCRIPT_DIR/../pulsar-boot-icons/grub"
if [ -d "$GRUB_ICONS_DIR" ]; then
    echo "📦 Instalando iconos Lucide personalizados en tema de GRUB..."
    [ -d "$DEST_DIR/assets/assets-icons/icons-1080p" ] && cp -f "$GRUB_ICONS_DIR"/icons-1080p/*.png "$DEST_DIR/assets/assets-icons/icons-1080p/" 2>/dev/null || true
    [ -d "$DEST_DIR/assets/assets-icons/icons-2k" ] && cp -f "$GRUB_ICONS_DIR"/icons-2k/*.png "$DEST_DIR/assets/assets-icons/icons-2k/" 2>/dev/null || true
    [ -d "$DEST_DIR/assets/assets-icons/icons-4k" ] && cp -f "$GRUB_ICONS_DIR"/icons-4k/*.png "$DEST_DIR/assets/assets-icons/icons-4k/" 2>/dev/null || true
    if [ -d "$DEST_DIR/icons" ]; then
        cp -f "$GRUB_ICONS_DIR"/icons-1080p/*.png "$DEST_DIR/icons/" 2>/dev/null || true
    fi
fi

# Limpieza
rm -rf "$TEMP_BUILD"

# Eliminar directorios .git para evitar inflar el paquete
find "$STAGE_DIR" -name ".git" -type d -prune -exec rm -rf {} + 2>/dev/null || true

echo "✅ Tema de GRUB estructurado en staging."
