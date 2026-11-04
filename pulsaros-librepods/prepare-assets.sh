#!/bin/bash
# Pulsar OS package hook: builds the vendored librepods daemon and installs it into the staging dir.
# Debian builds happen in a debian:trixie container so the binary only needs trixie's glibc and Qt 6.8.
# Arch packages are built by arch/pkgbuilds/pulsaros-librepods/PKGBUILD instead.
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Preparing pulsaros-librepods for staging in $STAGE_DIR..."
cd "$SRC_DIR"
mkdir -p "$STAGE_DIR"

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    echo "Building inside debian:trixie..."
    docker run --rm \
        -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
        -v "$SRC_DIR:/src:ro" \
        -v "$STAGE_DIR:/stage" \
        -v "$SRC_DIR/packaging/build-in-container.sh:/build.sh:ro" \
        debian:trixie bash /build.sh
elif pkg-config --exists Qt6Core Qt6Bluetooth libpulse; then
    echo "Docker is not available: building with the local Qt 6 toolchain..."
    BUILD_DIR="$(mktemp -d)"
    cmake -B "$BUILD_DIR" -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF
    cmake --build "$BUILD_DIR"
    DESTDIR="$STAGE_DIR" cmake --install "$BUILD_DIR" --prefix /usr
else
    echo "Error: pulsaros-librepods needs docker (to build in debian:trixie) or the Qt 6 development packages." >&2
    exit 1
fi

# Headless service only: no launcher entry for the tray app, and no unit in the XDG data dirs.
rm -f "$STAGE_DIR/usr/share/applications/me.kavishdevar.librepods.desktop"
rm -f "$STAGE_DIR/usr/share/systemd/user/librepods.service"
rmdir -p --ignore-fail-on-non-empty "$STAGE_DIR/usr/share/systemd/user" 2>/dev/null || true

# The packaged unit runs /usr/bin/librepods and is enabled with the graphical session.
install -Dm644 packaging/librepods.service "$STAGE_DIR/usr/lib/systemd/user/librepods.service"
install -d "$STAGE_DIR/usr/lib/systemd/user/graphical-session.target.wants"
ln -sf ../librepods.service "$STAGE_DIR/usr/lib/systemd/user/graphical-session.target.wants/librepods.service"

# Licence and provenance travel with the binary (GPL-3.0)
install -Dm644 LICENSE "$STAGE_DIR/usr/share/doc/pulsaros-librepods/LICENSE"
install -Dm644 UPSTREAM.md "$STAGE_DIR/usr/share/doc/pulsaros-librepods/UPSTREAM.md"

# Remove staging build remnants: only DEBIAN and usr belong in the package
find "$STAGE_DIR" -mindepth 1 -maxdepth 1 ! -name DEBIAN ! -name usr -exec rm -rf {} +

chmod -R a+rX "$STAGE_DIR"
echo "pulsaros-librepods staging preparation complete."
