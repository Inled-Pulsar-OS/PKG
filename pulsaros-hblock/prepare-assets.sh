#!/bin/bash
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Preparing Pulsar HBlock assets for staging in $STAGE_DIR..."

cd "$SRC_DIR"

mkdir -p "$STAGE_DIR/usr/bin" \
         "$STAGE_DIR/usr/lib/pulsaros-hblock" \
         "$STAGE_DIR/usr/share/pulsaros-hblock" \
         "$STAGE_DIR/usr/share/applications" \
         "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps"

# Launcher
cp -f usr/bin/pulsaros-hblock "$STAGE_DIR/usr/bin/pulsaros-hblock"
chmod 755 "$STAGE_DIR/usr/bin/pulsaros-hblock"

# Root helper
cp -f usr/lib/pulsaros-hblock/hblock-helper.sh "$STAGE_DIR/usr/lib/pulsaros-hblock/hblock-helper.sh"
chmod 755 "$STAGE_DIR/usr/lib/pulsaros-hblock/hblock-helper.sh"

# App itself
cp -rf usr/share/pulsaros-hblock/. "$STAGE_DIR/usr/share/pulsaros-hblock/"
chmod -R 755 "$STAGE_DIR/usr/share/pulsaros-hblock"

# Desktop + icon
cp -f usr/share/applications/*.desktop "$STAGE_DIR/usr/share/applications/"
cp -rf usr/share/icons/. "$STAGE_DIR/usr/share/icons/"

# Compile translations (PO -> MO) into the staged app when gettext is available
if command -v msgfmt >/dev/null 2>&1; then
    for po in locale/*/LC_MESSAGES/*.po; do
        rel="${po#locale/}"                       # en/LC_MESSAGES/x.po
        target="usr/share/pulsaros-hblock/locale/${rel%.po}.mo"
        mkdir -p "$(dirname "$target")"
        msgfmt -o "$target" "$po" || true
        echo "✅ Compiled $po -> $target"
    done
fi

# Clean pycache from staging
find "$STAGE_DIR" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR" -type f -name "*.pyc" -delete 2>/dev/null || true

echo "✅ Pulsar HBlock staging complete."