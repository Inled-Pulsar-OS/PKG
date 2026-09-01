#!/bin/bash
# ==============================================================================
# Pulsar OS - Welcome Application Asset Preparer
# ==============================================================================
# English: Downloads logos/assets and builds the Tauri binary during packaging.
# Español: Descarga logos/recursos y compila el binario Tauri durante el empaquetado.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
TAURI_DIR="$STAGE_DIR/usr/share/pulsaros-welcome"
UI_DIR="$TAURI_DIR/ui"

# ── 1. Download logos ──
echo "📥 Downloading welcome assets..."
mkdir -p "$UI_DIR"

wget -q --timeout=15 --tries=3 -O "$UI_DIR/macboat.png" "https://hosted.inled.es/macboat.png"
wget -q --timeout=15 --tries=3 -O "$UI_DIR/droidtux.png" "https://hosted.inled.es/droidtux.png"
wget -q --timeout=15 --tries=3 -O "$UI_DIR/winboat.svg" "https://hosted.inled.es/winboat_logo.NqN8dmd9.svg"

echo "✅ Assets downloaded."

# ── 2. Build Tauri binary ──
echo "🔨 Building Tauri binary..."
cd "$TAURI_DIR"

if ! command -v npm &>/dev/null; then
    echo "❌ Error: npm not found. Install Node.js first."
    exit 1
fi

npm ci
npx tauri build

BINARY="$TAURI_DIR/src-tauri/target/release/pulsaros-welcome"
if [ ! -f "$BINARY" ]; then
    echo "❌ Error: Tauri binary not found at $BINARY"
    exit 1
fi

echo "✅ Tauri binary built: $(du -h "$BINARY" | cut -f1)"

# ── 3. Clean build artifacts (keep only release binary) ──
echo "🧹 Cleaning build artifacts..."
rm -rf "$TAURI_DIR/node_modules"
rm -rf "$TAURI_DIR/src-tauri/target/debug"
rm -rf "$TAURI_DIR/src-tauri/target/incremental"
rm -rf "$TAURI_DIR/src-tauri/target/.fingerprint"
rm -rf "$TAURI_DIR/src-tauri/target/build"
rm -rf "$TAURI_DIR/src-tauri/target/deps"
rm -f  "$TAURI_DIR/src-tauri/target/.cargo-lock"
rm -f  "$TAURI_DIR/src-tauri/target/CACHEDIR.TAG"
rm -f  "$TAURI_DIR/src-tauri/target/.rustc_info.json"

echo "✅ Build artifacts cleaned. Release binary ready."
