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

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Tema de GRUB estructurado en staging."
