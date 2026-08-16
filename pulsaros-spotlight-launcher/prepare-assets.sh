#!/bin/bash
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Preparing Pulsar OS Spotlight launcher assets for staging in $STAGE_DIR..."

cd "$SRC_DIR"

# Install python modules
mkdir -p "$STAGE_DIR/usr/lib/python3/dist-packages"
cp -r src/pulsaros_spotlight "$STAGE_DIR/usr/lib/python3/dist-packages/"

# Install CLI scripts
mkdir -p "$STAGE_DIR/usr/bin"
install -m 755 cli/pulsaros-spotlight "$STAGE_DIR/usr/bin/pulsaros-spotlight"
install -m 755 cli/pulsaros-toggle-remap "$STAGE_DIR/usr/bin/pulsaros-toggle-remap" 2>/dev/null || true
install -m 755 cli/pulsaros-toggle-launcher "$STAGE_DIR/usr/bin/pulsaros-toggle-launcher" 2>/dev/null || true

# Install Desktop file
mkdir -p "$STAGE_DIR/usr/share/applications"
install -m 644 data/pulsaros-spotlight.desktop "$STAGE_DIR/usr/share/applications/pulsaros-spotlight.desktop"

# Install style CSS
mkdir -p "$STAGE_DIR/usr/share/pulsaros-spotlight"
install -m 644 data/style.css "$STAGE_DIR/usr/share/pulsaros-spotlight/style.css"

# Install Icons
mkdir -p "$STAGE_DIR/usr/share/pulsaros-spotlight/icons"
cp -r data/icons/* "$STAGE_DIR/usr/share/pulsaros-spotlight/icons/" 2>/dev/null || true

mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps"
install -m 644 data/icons/pulsaros-spotlight.svg "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/pulsaros-spotlight.svg" 2>/dev/null || true

mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/symbolic/apps"
install -m 644 data/icons/spotlight-symbolic.svg "$STAGE_DIR/usr/share/icons/hicolor/symbolic/apps/spotlight-symbolic.svg" 2>/dev/null || true

# Install GNOME Shell extension
EXT_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/pulsaros-spotlight-launcher@inled.es"
mkdir -p "$EXT_DIR"
cp -r gnome-shell-extension/* "$EXT_DIR/"

# Remove staging build remnants
rm -rf "$STAGE_DIR/src" "$STAGE_DIR/cli" "$STAGE_DIR/data" "$STAGE_DIR/gnome-shell-extension" "$STAGE_DIR/pyproject.toml" "$STAGE_DIR/usr/share/pulsaros-spotlight-launcher"

echo "✅ Pulsar OS Spotlight launcher staging preparation complete."
