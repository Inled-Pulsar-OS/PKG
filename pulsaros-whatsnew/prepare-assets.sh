#!/bin/bash
# ==============================================================================
# Pulsar OS - What's New assets staging
# ==============================================================================
# Called by package-and-deploy.sh with the staging directory as $1. The build
# already copies the package tree (usr/, etc/) to the staging directory, so
# here we only add the release notes coming from the shared releases.d/ folder
# and fix up permissions.
# ==============================================================================
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_ROOT="$(cd "$SRC_DIR/.." && pwd)"
RELEASES_SRC="$PKG_ROOT/releases.d"

echo "📰 Staging the Pulsar OS release notes viewer in $STAGE_DIR..."

install -d -m 0755 "$STAGE_DIR/usr/share/pulsaros-whatsnew/releases.d"

if [ -d "$RELEASES_SRC" ]; then
    found=0
    for md in "$RELEASES_SRC"/*.md; do
        [ -e "$md" ] || continue
        base="$(basename "$md")"
        [ "$base" = "README.md" ] && continue
        install -m 0644 "$md" "$STAGE_DIR/usr/share/pulsaros-whatsnew/releases.d/$base"
        found=$((found + 1))
    done
    echo "   Release notes bundled: $found"
else
    echo "   ⚠️  $RELEASES_SRC not found (packaging without notes)."
fi

# Make sure the entry points are executable.
chmod 0755 "$STAGE_DIR/usr/bin/pulsaros-whatsnew" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/bin/pulsaros-whatsnew-watch" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/bin/pulsaros-whatsnew-hook" 2>/dev/null || true
chmod 0755 "$STAGE_DIR/usr/lib/pulsaros-whatsnew"/*.py 2>/dev/null || true

# Clean pycache from staging.
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true

# The libalpm hook is Arch-only; do not ship it inside the Debian package.
rm -rf "$STAGE_DIR/usr/share/libalpm" 2>/dev/null || true

# Build-time only declaration, never shipped.
rm -f "$STAGE_DIR/EXTRA_PATHS" 2>/dev/null || true

echo "✅ Release notes viewer ready."
