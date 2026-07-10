#!/bin/bash
# ==============================================================================
# Pulsar OS - Gnome macOS Remap Wayland Asset Preparer
# ==============================================================================
# Clona el repositorio oficial de gnome-macos-remap-wayland y prepara
# sus archivos dentro del staging del paquete.
# ==============================================================================
set -e

STAGE_DIR="$(realpath -m "$1")"
DEST_DIR="$STAGE_DIR/usr/share/gnome-macos-remap-wayland"
mkdir -p "$DEST_DIR"

echo "📥 Descargando archivos de gnome-macos-remap-wayland desde GitHub..."
TEMP_BUILD="/tmp/gnome-macos-remap-wayland-build"
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Clone the repository
git clone --depth=1 "https://github.com/Inled-Pulsar-OS/gnome-macos-remap-wayland" "$TEMP_BUILD"

# Copy files to destination
cp -r "$TEMP_BUILD"/* "$DEST_DIR/"

# Remove the .git folder from destination
rm -rf "$DEST_DIR/.git"

# Clean up temp
rm -rf "$TEMP_BUILD"

echo "✅ Copiado con éxito."
