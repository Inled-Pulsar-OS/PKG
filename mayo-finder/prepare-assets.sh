#!/bin/bash
# ==============================================================================
# Pulsar OS - mayo-finder Package Asset Preparer
# ==============================================================================
# Fetches the Mayonnaise toolkit at the commit pinned below, builds only the
# `finder` target and stages the binary, .desktop entry and icon that its CMake
# install component carries. No Mayonnaise sources end up in the payload: the
# finder links the toolkit statically.
#
# MAYO_SRC=/path/to/mayonnaise reuses an existing checkout (local development).
# ==============================================================================

set -euo pipefail

STAGE_DIR="$(realpath -m "${1:?usage: prepare-assets.sh <staging-dir>}")"

# The Mayonnaise commit this package is built from. Bump it together with the
# version in DEBIAN/control so a release always corresponds to a fixed source.
MAYO_COMMIT="${MAYO_COMMIT:-9eccbfe2f4411ea8766305ef76eaa23afdd112be}"
MAYO_REPO="${MAYO_REPO:-https://github.com/egely1337/mayonnaise.git}"
APP_COMPONENT="finder"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

if [ -n "${MAYO_SRC:-}" ]; then
    echo "📂 Using local Mayonnaise checkout: $MAYO_SRC"
    MAYO_DIR="$MAYO_SRC"
else
    echo "📥 Fetching Mayonnaise at $MAYO_COMMIT..."
    git clone -q --filter=blob:none --no-checkout "$MAYO_REPO" "$WORK_DIR/mayonnaise"
    git -C "$WORK_DIR/mayonnaise" checkout -q "$MAYO_COMMIT"
    MAYO_DIR="$WORK_DIR/mayonnaise"
fi

echo "🔨 Compiling the Mayonnaise finder..."
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
install -Dm644 "$MAYO_DIR/assets/fonts/Inter-LICENSE.txt" \
    "$STAGE_DIR/usr/share/doc/mayo-finder/Inter-LICENSE.txt"

echo "✅ mayo-finder prepared in staging."
