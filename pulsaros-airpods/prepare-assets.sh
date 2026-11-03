#!/bin/bash
# Pulsar OS package hook: installs the sources of this directory into the staging dir.
# Same shape as pulsaros-spotlight-launcher: sources live in src/, cli/, data/ and
# gnome-shell-extension/, and are installed to their final /usr paths here.
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Preparing Pulsar OS AirPods assets for staging in $STAGE_DIR..."

cd "$SRC_DIR"

# Daemon launcher
install -Dm755 cli/pulsaros-airpods-daemon "$STAGE_DIR/usr/bin/pulsaros-airpods-daemon"

# Python package and the D-Bus contract it reads at start
install -d "$STAGE_DIR/usr/share/pulsaros-airpods"
cp -r src/pulsaros_airpods "$STAGE_DIR/usr/share/pulsaros-airpods/"
install -Dm644 data/org.pulsaros.AirPods.xml "$STAGE_DIR/usr/share/pulsaros-airpods/dbus/org.pulsaros.AirPods.xml"

# systemd user service, started with the graphical session
install -Dm644 data/pulsaros-airpods.service "$STAGE_DIR/usr/lib/systemd/user/pulsaros-airpods.service"
install -d "$STAGE_DIR/usr/lib/systemd/user/graphical-session.target.wants"
ln -sf ../pulsaros-airpods.service "$STAGE_DIR/usr/lib/systemd/user/graphical-session.target.wants/pulsaros-airpods.service"

# GNOME Shell extension (code, icons, stylesheet and the i18n catalogs)
EXT_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/airpods@pulsaros.org"
mkdir -p "$EXT_DIR"
cp -r gnome-shell-extension/* "$EXT_DIR/"

# Remove staging build remnants: the sources were installed above
rm -rf "$STAGE_DIR/src" "$STAGE_DIR/cli" "$STAGE_DIR/data" "$STAGE_DIR/gnome-shell-extension" \
    "$STAGE_DIR/scripts" "$STAGE_DIR/tests" "$STAGE_DIR/docs" "$STAGE_DIR/README.md"
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true

# A restrictive umask on the build machine must not leak into the package
chmod -R a+rX "$STAGE_DIR"

echo "Pulsar OS AirPods staging preparation complete."
