#!/bin/bash
# ==============================================================================
# Pulsar OS - pulsaros-welcome prepare-assets.sh
# ==============================================================================
# Builds the Tauri binary (primary) and stages the Python+WebKitGTK fallback.
# Tauri build embeds the React frontend in the binary; Python loads dist/ from
# disk, so both dist/ and the binary are staged.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
SCRIPT_DIR="$(dirname "$(realpath "$0")")"
APP_SRC="$SCRIPT_DIR/usr/share/pulsaros-welcome"

echo "🧹 Limpieza inicial del staging..."
find "$STAGE_DIR" -mindepth 1 -maxdepth 1 ! -name DEBIAN ! -name etc -exec rm -rf {} +

# Build Tauri binary (embeds React frontend via beforeBuildCommand)
TAURI_BINARY=""
if command -v npx >/dev/null 2>&1 && [ -f "$APP_SRC/src-tauri/Cargo.toml" ]; then
    echo "🦀 Compilando Tauri binary..."
    cd "$APP_SRC"
    npx tauri build 2>/dev/null || echo "⚠️  Tauri build failed, continuing with Python fallback only"
    cd "$SCRIPT_DIR"
    TAURI_BINARY="$APP_SRC/src-tauri/target/release/pulsaros-welcome"
fi

# Fallback: build React frontend separately if Tauri didn't produce dist/
if [ ! -f "$APP_SRC/dist/index.html" ]; then
    echo "🏗️  Compilando frontend React con pnpm..."
    cd "$APP_SRC"
    if command -v pnpm >/dev/null 2>&1; then
        pnpm build
    elif command -v npm >/dev/null 2>&1; then
        npm run build
    fi
    cd "$SCRIPT_DIR"
fi

echo "📦 Preparando archivos de pulsaros-welcome..."
mkdir -p "$STAGE_DIR/usr/bin" "$STAGE_DIR/usr/lib" "$STAGE_DIR/usr/libexec" \
         "$STAGE_DIR/usr/share/pulsaros" "$STAGE_DIR/usr/share/pulsaros-welcome" "$STAGE_DIR/etc"

# Copy executables and wrappers
cp -f "$SCRIPT_DIR/usr/bin/pulsaros-welcome" "$STAGE_DIR/usr/bin/"
[ -f "$SCRIPT_DIR/usr/bin/pulsar-cleanup-user" ] && cp -f "$SCRIPT_DIR/usr/bin/pulsar-cleanup-user" "$STAGE_DIR/usr/bin/"
[ -f "$SCRIPT_DIR/usr/libexec/pulsar-cleanup-live.sh" ] && cp -f "$SCRIPT_DIR/usr/libexec/pulsar-cleanup-live.sh" "$STAGE_DIR/usr/libexec/"

# Install Tauri binary (primary)
if [ -n "$TAURI_BINARY" ] && [ -f "$TAURI_BINARY" ]; then
    echo "🦀 Instalando Tauri binary..."
    mkdir -p "$STAGE_DIR/usr/lib/pulsaros-welcome"
    cp -f "$TAURI_BINARY" "$STAGE_DIR/usr/lib/pulsaros-welcome/pulsaros-welcome"
    chmod 755 "$STAGE_DIR/usr/lib/pulsaros-welcome/pulsaros-welcome"
fi

# Copy Python+WebKitGTK fallback app and compiled frontend
cp -f "$APP_SRC/welcome.py" "$STAGE_DIR/usr/share/pulsaros-welcome/"
[ -d "$APP_SRC/dist" ] && cp -rf "$APP_SRC/dist" "$STAGE_DIR/usr/share/pulsaros-welcome/"
[ -d "$APP_SRC/public" ] && cp -rf "$APP_SRC/public" "$STAGE_DIR/usr/share/pulsaros-welcome/"
[ -f "$APP_SRC/logo.png" ] && cp -f "$APP_SRC/logo.png" "$STAGE_DIR/usr/share/pulsaros-welcome/"

# Copy OOTB helpers
[ -d "$SCRIPT_DIR/usr/share/pulsaros" ] && cp -rf "$SCRIPT_DIR/usr/share/pulsaros/." "$STAGE_DIR/usr/share/pulsaros/"
[ -d "$SCRIPT_DIR/etc" ] && cp -rf "$SCRIPT_DIR/etc/." "$STAGE_DIR/etc/"

# Set permissions
chmod 755 "$STAGE_DIR/usr/bin/pulsaros-welcome"
chmod 755 "$STAGE_DIR/usr/share/pulsaros-welcome/welcome.py"
[ -f "$STAGE_DIR/usr/bin/pulsar-cleanup-user" ] && chmod 755 "$STAGE_DIR/usr/bin/pulsar-cleanup-user"
[ -f "$STAGE_DIR/usr/libexec/pulsar-cleanup-live.sh" ] && chmod 755 "$STAGE_DIR/usr/libexec/pulsar-cleanup-live.sh"

echo "✅ pulsaros-welcome preparado con éxito."
