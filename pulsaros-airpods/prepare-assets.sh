#!/bin/bash
# Pulsar OS package hook: builds the AirPods daemon (daemon/) and installs it, together with the
# GNOME Shell extension (gnome-shell-extension/), into the staging dir.
#
# The build environment (image, Qt 6 and cmake) comes from the caller, never from this script.
# Arch packages are built by arch/pkgbuilds/pulsaros-airpods/PKGBUILD instead.
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_DIR="$SRC_DIR/daemon"

echo "Preparing pulsaros-airpods for staging in $STAGE_DIR..."
cd "$SRC_DIR"
mkdir -p "$STAGE_DIR"

# -- daemon -------------------------------------------------------------------------------------
# Always built with the local toolchain: the caller (CI job or pulsaros-airpods/build-local.sh) supplies the
# build environment and installs pulsaros-airpods/build-deps.txt first.
if ! pkg-config --exists Qt6Core Qt6Bluetooth libpulse; then
    echo "Error: pulsaros-airpods needs the build dependencies in pulsaros-airpods/build-deps.txt (install-build-deps.sh), or run pulsaros-airpods/build-local.sh." >&2
    exit 1
fi
BUILD_DIR="$(mktemp -d)"
cmake -S "$DAEMON_DIR" -B "$BUILD_DIR" -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF
cmake --build "$BUILD_DIR"
DESTDIR="$STAGE_DIR" cmake --install "$BUILD_DIR" --prefix /usr
rm -r "$BUILD_DIR"

# Headless service only: no launcher entry for the tray app, and no unit in the XDG data dirs.
rm -f "$STAGE_DIR/usr/share/applications/me.kavishdevar.librepods.desktop"
rm -f "$STAGE_DIR/usr/share/systemd/user/librepods.service"
rmdir -p --ignore-fail-on-non-empty "$STAGE_DIR/usr/share/systemd/user" 2>/dev/null || true

# The packaged unit runs /usr/bin/librepods and is enabled with the graphical session.
install -Dm644 "$DAEMON_DIR/packaging/librepods.service" "$STAGE_DIR/usr/lib/systemd/user/librepods.service"
install -d "$STAGE_DIR/usr/lib/systemd/user/graphical-session.target.wants"
ln -sf ../librepods.service "$STAGE_DIR/usr/lib/systemd/user/graphical-session.target.wants/librepods.service"

# The daemon is GPL-3.0: its licence and provenance travel with the binary
install -Dm644 "$DAEMON_DIR/LICENSE" "$STAGE_DIR/usr/share/doc/pulsaros-airpods/librepods/LICENSE"
install -Dm644 "$DAEMON_DIR/UPSTREAM.md" "$STAGE_DIR/usr/share/doc/pulsaros-airpods/librepods/UPSTREAM.md"

# -- GNOME Shell extension --------------------------------------------------------------------------
EXT_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions/airpods@pulsaros.org"
mkdir -p "$EXT_DIR"
cp -r gnome-shell-extension/* "$EXT_DIR/"

# Remove staging build remnants: only DEBIAN and usr belong in the package
find "$STAGE_DIR" -mindepth 1 -maxdepth 1 ! -name DEBIAN ! -name usr -exec rm -rf {} +

chmod -R a+rX "$STAGE_DIR"
echo "pulsaros-airpods staging preparation complete."
