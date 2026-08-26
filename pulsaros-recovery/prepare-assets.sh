#!/bin/bash
# Build Tauri app and place binary + support files in staging
set -e

STAGE_DIR="$(realpath -m "$1")"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAURI_DIR="$SCRIPT_DIR/tauri"

echo "Building Tauri app from $TAURI_DIR..."

# Install frontend deps and build
cd "$TAURI_DIR"
npm install --legacy-peer-deps 2>/dev/null || pnpm install --frozen-lockfile 2>/dev/null || true
npx tauri build 2>/dev/null || pnpm tauri build

# Copy binary to staging
mkdir -p "$STAGE_DIR/usr/bin"
cp "$TAURI_DIR/src-tauri/target/release/pulsaros-recovery" "$STAGE_DIR/usr/bin/pulsaros-recovery"
chmod 755 "$STAGE_DIR/usr/bin/pulsaros-recovery"

# Copy desktop cleanup script
cp "$SCRIPT_DIR/usr/bin/pulsaros-recovery-desktop-setup" "$STAGE_DIR/usr/bin/pulsaros-recovery-desktop-setup"
chmod 755 "$STAGE_DIR/usr/bin/pulsaros-recovery-desktop-setup"

# Copy desktop launcher
mkdir -p "$STAGE_DIR/usr/share/applications"
cp "$SCRIPT_DIR/usr/share/applications/pulsaros-recovery.desktop" "$STAGE_DIR/usr/share/applications/pulsaros-recovery.desktop"

# Copy autostart entry
mkdir -p "$STAGE_DIR/etc/xdg/autostart"
cp "$SCRIPT_DIR/etc/xdg/autostart/pulsaros-recovery-desktop-setup.desktop" "$STAGE_DIR/etc/xdg/autostart/pulsaros-recovery-desktop-setup.desktop"

# Copy postinst
mkdir -p "$STAGE_DIR/DEBIAN"
cp "$SCRIPT_DIR/DEBIAN/postinst" "$STAGE_DIR/DEBIAN/postinst"
chmod 755 "$STAGE_DIR/DEBIAN/postinst"

echo "All files placed in staging: $STAGE_DIR"
