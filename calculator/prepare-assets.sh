#!/bin/bash
# ==============================================================================
# Pulsar OS - Calculator Package Asset Preparer
# ==============================================================================
# Fetches the Mayonnaise toolkit at the commit pinned below, applies the
# Pulsar OS overlay, builds only the `calculator` target and stages the binary,
# .desktop entry, and standard system icons.
#
# MAYO_SRC=/path/to/mayonnaise reuses an existing checkout (local development).
# ==============================================================================

set -euo pipefail

STAGE_DIR="$(realpath -m "${1:?usage: prepare-assets.sh <staging-dir>}")"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

MAYO_COMMIT="${MAYO_COMMIT:-9eccbfe2f4411ea8766305ef76eaa23afdd112be}"
MAYO_REPO="${MAYO_REPO:-https://github.com/egely1337/mayonnaise.git}"
APP_COMPONENT="calculator"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

if [ -n "${MAYO_SRC:-}" ]; then
    echo "📂 Using local Mayonnaise checkout: $MAYO_SRC"
    cp -r "$MAYO_SRC" "$WORK_DIR/mayonnaise"
else
    echo "📥 Fetching Mayonnaise at $MAYO_COMMIT..."
    git clone -q --filter=blob:none --no-checkout "$MAYO_REPO" "$WORK_DIR/mayonnaise"
    git -C "$WORK_DIR/mayonnaise" checkout -q "$MAYO_COMMIT"
fi

MAYO_DIR="$WORK_DIR/mayonnaise"

if [ -d "$SCRIPT_DIR/overlay" ]; then
    echo "🎨 Applying Pulsar OS customizations from overlay..."
    cp -rf "$SCRIPT_DIR/overlay/"* "$MAYO_DIR/"
fi

echo "🔨 Compiling the calculator..."
cmake -S "$MAYO_DIR" -B "$WORK_DIR/build" \
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
if [ -f "$STAGE_DIR/usr/share/applications/io.github.egely1337.Mayonnaise.Calculator.desktop" ]; then
    mv "$STAGE_DIR/usr/share/applications/io.github.egely1337.Mayonnaise.Calculator.desktop" \
       "$STAGE_DIR/usr/share/applications/calculator.desktop"
    cat << 'EOF' > "$STAGE_DIR/usr/share/applications/org.gnome.Calculator.desktop"
[Desktop Entry]
Type=Application
Name=Calculator
Exec=calculator %U
Icon=calculator
Terminal=false
NoDisplay=true
EOF
fi

# Ensure icons are available under standard names
mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps"
if [ -f "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/io.github.egely1337.Mayonnaise.Calculator.svg" ]; then
    cp "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/io.github.egely1337.Mayonnaise.Calculator.svg" \
       "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/calculator.svg"
    ln -sf calculator.svg "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/org.gnome.Calculator.svg"
fi

install -Dm644 "$MAYO_DIR/assets/fonts/Inter-LICENSE.txt" \
    "$STAGE_DIR/usr/share/doc/calculator/Inter-LICENSE.txt"

echo "✅ calculator prepared in staging."
