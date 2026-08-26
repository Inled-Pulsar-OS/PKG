#!/bin/bash
# Build Tauri app and place binary in staging
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

echo "Tauri binary placed at $STAGE_DIR/usr/bin/pulsaros-recovery"
