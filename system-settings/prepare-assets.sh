#!/bin/bash
set -euo pipefail

STAGE_DIR="$(realpath -m "${1:?usage: prepare-assets.sh <staging-dir>}")"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FORMA_COMMIT="${FORMA_COMMIT:-e0a23a7348f5846e50872b829278d188a4288b5f}"
FORMA_REPO="${FORMA_REPO:-https://github.com/skjsbsnq/Forma.git}"
APP_COMPONENT="systemsettings"

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

echo "🔨 Compiling System Settings..."
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

mkdir -p "$STAGE_DIR/usr/bin"
if [ -f "$STAGE_DIR/usr/bin/system-settings" ]; then
    ln -sf system-settings "$STAGE_DIR/usr/bin/forma-settings"
elif [ -f "$STAGE_DIR/usr/bin/forma-settings" ]; then
    mv "$STAGE_DIR/usr/bin/forma-settings" "$STAGE_DIR/usr/bin/system-settings"
    ln -sf system-settings "$STAGE_DIR/usr/bin/forma-settings"
fi

mkdir -p "$STAGE_DIR/usr/share/applications"
if [ -f "$STAGE_DIR/usr/share/applications/io.github.egely1337.Mayonnaise.Settings.desktop" ]; then
    mv "$STAGE_DIR/usr/share/applications/io.github.egely1337.Mayonnaise.Settings.desktop" \
       "$STAGE_DIR/usr/share/applications/system-settings.desktop"
fi
rm -f "$STAGE_DIR/usr/share/applications/io.github.egely1337.Mayonnaise.Settings.desktop"

# Compatibility desktop entry with NoDisplay=true so DBus/GNOME Shell shortcuts work without duplicate icon
cat << 'EOF_GCC' > "$STAGE_DIR/usr/share/applications/gnome-control-center.desktop"
[Desktop Entry]
Type=Application
Name=System Settings
Exec=system-settings
Icon=org.gnome.Settings
Terminal=false
NoDisplay=true
Categories=Settings;DesktopSettings;
EOF_GCC

echo "✅ system-settings prepared in staging."
