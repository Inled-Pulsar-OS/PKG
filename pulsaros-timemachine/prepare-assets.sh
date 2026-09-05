#!/bin/bash
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Preparing Pulsar OS Time Machine assets for staging in $STAGE_DIR..."

cd "$SRC_DIR"

mkdir -p "$STAGE_DIR/usr/bin" "$STAGE_DIR/usr/share/pulsaros-timemachine" "$STAGE_DIR/usr/share/applications"

cp -f usr/bin/pulsaros-timemachine "$STAGE_DIR/usr/bin/"
chmod 755 "$STAGE_DIR/usr/bin/pulsaros-timemachine"

cp -rf usr/share/pulsaros-timemachine/. "$STAGE_DIR/usr/share/pulsaros-timemachine/"
chmod -R 755 "$STAGE_DIR/usr/share/pulsaros-timemachine"

cp -f usr/share/applications/pulsaros-timemachine.desktop "$STAGE_DIR/usr/share/applications/"

if [ -d "usr/share/icons" ]; then
    mkdir -p "$STAGE_DIR/usr/share/icons"
    cp -rf usr/share/icons/. "$STAGE_DIR/usr/share/icons/"
fi

if [ -d "usr/share/pixmaps" ]; then
    mkdir -p "$STAGE_DIR/usr/share/pixmaps"
    cp -rf usr/share/pixmaps/. "$STAGE_DIR/usr/share/pixmaps/"
fi

# Clean pycache from staging
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true

echo "✅ Pulsar OS Time Machine staging complete."
