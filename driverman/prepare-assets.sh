#!/bin/bash
# ==============================================================================
# Pulsar OS - driverman Package Asset Preparer
# ==============================================================================
# Compila el CLI de C++ (driverman), instala la GUI GTK (driverman-gui) y
# limpia las fuentes del payload del paquete .deb.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_TMP="$(mktemp -d)"
trap 'rm -rf "$BUILD_TMP"' EXIT

echo "🔨 Compilando driverman CLI..."
cmake -S "$SRC_DIR" -B "$BUILD_TMP" \
    -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr
cmake --build "$BUILD_TMP" -j"$(nproc)"
cmake --install "$BUILD_TMP" --prefix "$STAGE_DIR/usr"

echo "🚀 Instalando GUI GTK..."
mkdir -p "$STAGE_DIR/usr/bin"
install -m755 "$SRC_DIR/gui/driverman-gui.py" "$STAGE_DIR/usr/bin/driverman-gui"

echo "🧹 Limpiando fuentes del payload..."
rm -rf "$STAGE_DIR/src" "$STAGE_DIR/gui" "$STAGE_DIR/CMakeLists.txt"

echo "✅ driverman preparado en staging."
