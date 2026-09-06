#!/bin/bash
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Preparing Pulsar OS Update Assistant assets for staging in $STAGE_DIR..."

cd "$SRC_DIR"

mkdir -p "$STAGE_DIR/usr/bin" \
         "$STAGE_DIR/usr/share/pulsaros-update" \
         "$STAGE_DIR/usr/share/applications" \
         "$STAGE_DIR/etc/xdg/autostart"

cp -f usr/bin/pulsaros-update "$STAGE_DIR/usr/bin/"
chmod 755 "$STAGE_DIR/usr/bin/pulsaros-update"

cp -rf usr/share/pulsaros-update/. "$STAGE_DIR/usr/share/pulsaros-update/"
chmod -R 755 "$STAGE_DIR/usr/share/pulsaros-update"

cp -f usr/share/applications/pulsaros-update.desktop "$STAGE_DIR/usr/share/applications/"
cp -f etc/xdg/autostart/pulsaros-update.desktop "$STAGE_DIR/etc/xdg/autostart/"

if [ -d "usr/share/icons" ]; then
    mkdir -p "$STAGE_DIR/usr/share/icons"
    cp -rf usr/share/icons/. "$STAGE_DIR/usr/share/icons/"
fi

# Clean pycache from staging
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true

echo "✅ Pulsar OS Update Assistant staging complete."
