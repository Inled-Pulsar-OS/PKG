#!/bin/bash
# ==============================================================================
# Pulsar OS - Finder Package Asset Preparer
# ==============================================================================
set -euo pipefail

STAGE_DIR="$(realpath -m "${1:?usage: prepare-assets.sh <staging-dir>}")"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FORMA_COMMIT="${FORMA_COMMIT:-e0a23a7348f5846e50872b829278d188a4288b5f}"
FORMA_REPO="${FORMA_REPO:-https://github.com/skjsbsnq/Forma.git}"
APP_COMPONENT="finder"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

if [ -n "${FORMA_SRC:-}" ] && [ -d "$FORMA_SRC" ]; then
    echo "📂 Using local Forma checkout: $FORMA_SRC"
    cp -r "$FORMA_SRC" "$WORK_DIR/forma"
elif [ -d "/tmp/forma" ]; then
    echo "📂 Reusing /tmp/forma checkout"
    cp -r "/tmp/forma" "$WORK_DIR/forma"
else
    echo "📥 Fetching Forma at $FORMA_COMMIT..."
    git clone -q --filter=blob:none "$FORMA_REPO" "$WORK_DIR/forma"
    git -C "$WORK_DIR/forma" checkout -q "$FORMA_COMMIT"
fi

FORMA_DIR="$WORK_DIR/forma"

if [ -d "$SCRIPT_DIR/overlay" ]; then
    echo "🎨 Applying Pulsar OS customizations from overlay..."
    cp -rf "$SCRIPT_DIR/overlay/"* "$FORMA_DIR/"
fi

echo "🔨 Compiling the Finder..."
cmake -S "$FORMA_DIR" -B "$WORK_DIR/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/usr \
    -DMAYONNAISE_BUILD_APPS=ON \
    -DMAYONNAISE_BUILD_TESTS=OFF \
    -DMAYONNAISE_USE_SYSTEM_DEPS=ON \
    -Wno-dev
cmake --build "$WORK_DIR/build" --target "$APP_COMPONENT" -j"$(nproc)"

echo "🚀 Installing $APP_COMPONENT into staging..."
DESTDIR="$STAGE_DIR" cmake --install "$WORK_DIR/build" --component "$APP_COMPONENT"

# Install standard desktop entries and aliases
mkdir -p "$STAGE_DIR/usr/share/applications"
if [ -f "$STAGE_DIR/usr/share/applications/io.github.egely1337.Mayonnaise.Finder.desktop" ]; then
    mv "$STAGE_DIR/usr/share/applications/io.github.egely1337.Mayonnaise.Finder.desktop" \
       "$STAGE_DIR/usr/share/applications/finder.desktop"
fi
rm -f "$STAGE_DIR/usr/share/applications/io.github.egely1337.Mayonnaise.Finder.desktop"

# Default file manager associations
if [ -f "$FORMA_DIR/applications/finder/portal/mimeapps.list" ]; then
    install -Dm644 "$FORMA_DIR/applications/finder/portal/mimeapps.list" \
        "$STAGE_DIR/usr/share/applications/mimeapps.list"
fi

# XDG Desktop Portal backend
if [ -f "$FORMA_DIR/applications/finder/portal/pulsar-finder-portal" ]; then
    install -Dm755 "$FORMA_DIR/applications/finder/portal/pulsar-finder-portal" \
        "$STAGE_DIR/usr/lib/pulsar-finder-portal/pulsar-finder-portal"
    install -Dm644 "$FORMA_DIR/applications/finder/portal/org.freedesktop.impl.portal.desktop.finder.service" \
        "$STAGE_DIR/usr/share/dbus-1/services/org.freedesktop.impl.portal.desktop.finder.service"
    install -Dm644 "$FORMA_DIR/applications/finder/portal/finder.portal" \
        "$STAGE_DIR/usr/share/xdg-desktop-portal/portals/finder.portal"
    mkdir -p "$STAGE_DIR/etc/xdg/xdg-desktop-portal"
    install -Dm644 "$FORMA_DIR/applications/finder/portal/portals.conf" \
        "$STAGE_DIR/etc/xdg/xdg-desktop-portal/portals.conf"
    cat << "CONF" > "$STAGE_DIR/etc/xdg/xdg-desktop-portal/gnome-portals.conf"
[preferred]
org.freedesktop.impl.portal.FileChooser=finder;gnome;gtk;
default=gnome;gtk;
org.freedesktop.impl.portal.Secret=oo7-portal;gnome-keyring;
CONF
    mkdir -p "$STAGE_DIR/usr/share/xdg-desktop-portal"
    install -Dm644 "$FORMA_DIR/applications/finder/portal/portals.conf" \
        "$STAGE_DIR/usr/share/xdg-desktop-portal/pulsar-portals.conf"
    install -Dm644 "$STAGE_DIR/etc/xdg/xdg-desktop-portal/gnome-portals.conf" \
        "$STAGE_DIR/usr/share/xdg-desktop-portal/gnome-portals.conf"
fi

# Environment variables for GTK / Chrome / Electron to use portal file chooser
mkdir -p "$STAGE_DIR/etc/profile.d"
cat << "EOF_PROFILE" > "$STAGE_DIR/etc/profile.d/pulsar-portal.sh"
#!/bin/sh
export GTK_USE_PORTAL=1
EOF_PROFILE
chmod 755 "$STAGE_DIR/etc/profile.d/pulsar-portal.sh"

mkdir -p "$STAGE_DIR/etc/environment.d"
echo "GTK_USE_PORTAL=1" > "$STAGE_DIR/etc/environment.d/10-pulsar-portal.conf"

# Systemd user service
mkdir -p "$STAGE_DIR/usr/lib/systemd/user"
cat << "EOF_SVC" > "$STAGE_DIR/usr/lib/systemd/user/pulsar-finder-portal.service"
[Unit]
Description=Pulsar OS Finder Portal Backend
PartOf=graphical-session.target

[Service]
Type=dbus
BusName=org.freedesktop.impl.portal.desktop.finder
ExecStart=/usr/lib/pulsar-finder-portal/pulsar-finder-portal
Restart=on-failure
EOF_SVC

# Icons across standard resolutions
for sz in 16x16 24x24 32x32 48x48 64x64 96x96 128x128 256x256 512x512; do
    if [ -f "$FORMA_DIR/applications/finder/data/icons/${sz}/finder.png" ]; then
        install -Dm644 "$FORMA_DIR/applications/finder/data/icons/${sz}/finder.png" \
            "$STAGE_DIR/usr/share/icons/hicolor/${sz}/apps/finder.png"
        install -Dm644 "$FORMA_DIR/applications/finder/data/icons/${sz}/finder.png" \
            "$STAGE_DIR/usr/share/icons/hicolor/${sz}/apps/io.github.egely1337.Mayonnaise.Finder.png"
    fi
done

# Ensure icon in MacTahoe theme for Spotlight
mkdir -p "$STAGE_DIR/usr/share/icons/MacTahoe-blue/apps/scalable"
if [ -f "$FORMA_DIR/applications/finder/data/icons/512x512/finder.png" ]; then
    install -Dm644 "$FORMA_DIR/applications/finder/data/icons/512x512/finder.png" \
        "$STAGE_DIR/usr/share/icons/MacTahoe-blue/apps/scalable/finder.png"
    ln -sf finder.png "$STAGE_DIR/usr/share/icons/MacTahoe-blue/apps/scalable/io.github.egely1337.Mayonnaise.Finder.png"
fi

if [ -f "$FORMA_DIR/assets/fonts/Inter-LICENSE.txt" ]; then
    install -Dm644 "$FORMA_DIR/assets/fonts/Inter-LICENSE.txt" \
        "$STAGE_DIR/usr/share/doc/finder/Inter-LICENSE.txt"
fi

echo "✅ finder prepared in staging."
