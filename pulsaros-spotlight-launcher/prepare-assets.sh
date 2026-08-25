#!/bin/bash
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Preparing Pulsar OS Spotlight launcher assets for staging in $STAGE_DIR..."

cd "$SRC_DIR"

# Install native Rust binary as the main launcher
mkdir -p "$STAGE_DIR/usr/bin"
if [ ! -f target/release/pulsaros-spotlight ] || [ src/main.rs -nt target/release/pulsaros-spotlight ]; then
    echo "🦀 Building pulsaros-spotlight (release)..."
    cargo build --release
fi
install -m 755 target/release/pulsaros-spotlight "$STAGE_DIR/usr/bin/pulsaros-spotlight"

# Install helper CLI scripts
install -m 755 cli/pulsaros-toggle-remap "$STAGE_DIR/usr/bin/pulsaros-toggle-remap" 2>/dev/null || true
install -m 755 cli/pulsaros-toggle-launcher "$STAGE_DIR/usr/bin/pulsaros-toggle-launcher" 2>/dev/null || true

# Install Desktop file
mkdir -p "$STAGE_DIR/usr/share/applications"
install -m 644 data/pulsaros-spotlight.desktop "$STAGE_DIR/usr/share/applications/pulsaros-spotlight.desktop"

# Install style CSS
mkdir -p "$STAGE_DIR/usr/share/pulsaros-spotlight"
install -m 644 data/base.css "$STAGE_DIR/usr/share/pulsaros-spotlight/base.css"
install -m 644 data/dark.css "$STAGE_DIR/usr/share/pulsaros-spotlight/dark.css"
install -m 644 data/light.css "$STAGE_DIR/usr/share/pulsaros-spotlight/light.css"
install -m 644 data/index.css "$STAGE_DIR/usr/share/pulsaros-spotlight/index.css"

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

# Remove staging build remnants and cache
rm -rf "$STAGE_DIR/src" "$STAGE_DIR/cli" "$STAGE_DIR/data" "$STAGE_DIR/gnome-shell-extension" "$STAGE_DIR/pyproject.toml" "$STAGE_DIR/usr/share/pulsaros-spotlight-launcher" "$STAGE_DIR/build" "$STAGE_DIR/dist" "$STAGE_DIR"/*.egg-info "$STAGE_DIR/.gitignore"
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true

echo "✅ Pulsar OS Spotlight launcher staging preparation complete."
