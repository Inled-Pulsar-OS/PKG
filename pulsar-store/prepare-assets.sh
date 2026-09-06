#!/bin/bash
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Preparing Pulsar Store assets for staging in $STAGE_DIR..."

cd "$SRC_DIR"

mkdir -p "$STAGE_DIR/usr/bin" \
         "$STAGE_DIR/usr/share/pulsar-store" \
         "$STAGE_DIR/usr/share/applications" \
         "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps"

cp -rf usr/bin/. "$STAGE_DIR/usr/bin/"
chmod 755 "$STAGE_DIR"/usr/bin/*

cp -rf usr/share/pulsar-store/. "$STAGE_DIR/usr/share/pulsar-store/"
chmod -R 755 "$STAGE_DIR/usr/share/pulsar-store"

cp -f usr/share/applications/*.desktop "$STAGE_DIR/usr/share/applications/"
cp -rf usr/share/icons/. "$STAGE_DIR/usr/share/icons/"

# Clean pycache from staging
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true

echo "✅ Pulsar Store staging complete."
