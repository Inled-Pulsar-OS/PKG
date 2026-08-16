#!/bin/bash
set -e

STAGE_DIR="$(realpath -m "$1")"
DEST_DIR="$STAGE_DIR/usr/share/pulsar-boot-icons"
mkdir -p "$DEST_DIR"

SCRIPT_DIR="$(dirname "$(realpath "$0")")"
cp -f "$SCRIPT_DIR"/*.png "$DEST_DIR/" 2>/dev/null || true
if [ -d "$SCRIPT_DIR/grub" ]; then
    cp -rf "$SCRIPT_DIR/grub" "$DEST_DIR/"
fi

echo "✅ pulsaros-boot-icons assets prepared in $DEST_DIR"
