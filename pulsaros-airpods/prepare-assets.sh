#!/bin/bash
# Pulsar OS package hook: installs the GNOME Shell extension into the staging dir.
# The Bluetooth daemon is a separate package (pulsaros-librepods).
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Preparing Pulsar OS AirPods extension for staging in $STAGE_DIR..."

cd "$SRC_DIR"

EXT_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/airpods@pulsaros.org"
mkdir -p "$EXT_DIR"
cp -r gnome-shell-extension/* "$EXT_DIR/"

# Remove staging build remnants: only DEBIAN and usr belong in the package
find "$STAGE_DIR" -mindepth 1 -maxdepth 1 ! -name DEBIAN ! -name usr -exec rm -rf {} +

# A restrictive umask on the build machine must not leak into the package
chmod -R a+rX "$STAGE_DIR"

echo "Pulsar OS AirPods staging preparation complete."
