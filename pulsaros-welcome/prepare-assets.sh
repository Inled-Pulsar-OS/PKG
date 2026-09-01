#!/bin/bash
# ==============================================================================
# Pulsar OS - pulsaros-welcome prepare-assets.sh
# ==============================================================================
# Builds the modern React/Tailwind frontend into dist/ (if needed) and stages
# the Python + WebKitGTK welcome application and OOTB helpers cleanly.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
SCRIPT_DIR="$(dirname "$(realpath "$0")")"
APP_SRC="$SCRIPT_DIR/usr/share/pulsaros-welcome"

echo "🧹 Limpieza inicial del staging..."
find "$STAGE_DIR" -mindepth 1 -maxdepth 1 ! -name DEBIAN ! -name etc -exec rm -rf {} +

# Asegurar que el frontend React esté compilado en dist/
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
mkdir -p "$STAGE_DIR/usr/bin" "$STAGE_DIR/usr/libexec" "$STAGE_DIR/usr/share/pulsaros" "$STAGE_DIR/usr/share/pulsaros-welcome" "$STAGE_DIR/etc"

# Copiar ejecutables y wrappers
cp -f "$SCRIPT_DIR/usr/bin/pulsaros-welcome" "$STAGE_DIR/usr/bin/"
[ -f "$SCRIPT_DIR/usr/bin/pulsar-cleanup-user" ] && cp -f "$SCRIPT_DIR/usr/bin/pulsar-cleanup-user" "$STAGE_DIR/usr/bin/"
[ -f "$SCRIPT_DIR/usr/libexec/pulsar-cleanup-live.sh" ] && cp -f "$SCRIPT_DIR/usr/libexec/pulsar-cleanup-live.sh" "$STAGE_DIR/usr/libexec/"

# Copiar app de bienvenida WebKitGTK y el frontend compilado en dist/
cp -f "$APP_SRC/welcome.py" "$STAGE_DIR/usr/share/pulsaros-welcome/"
[ -d "$APP_SRC/dist" ] && cp -rf "$APP_SRC/dist" "$STAGE_DIR/usr/share/pulsaros-welcome/"
[ -d "$APP_SRC/public" ] && cp -rf "$APP_SRC/public" "$STAGE_DIR/usr/share/pulsaros-welcome/"
[ -f "$APP_SRC/logo.png" ] && cp -f "$APP_SRC/logo.png" "$STAGE_DIR/usr/share/pulsaros-welcome/"

# Copiar helpers OOTB
[ -d "$SCRIPT_DIR/usr/share/pulsaros" ] && cp -rf "$SCRIPT_DIR/usr/share/pulsaros/." "$STAGE_DIR/usr/share/pulsaros/"
[ -d "$SCRIPT_DIR/etc" ] && cp -rf "$SCRIPT_DIR/etc/." "$STAGE_DIR/etc/"

# Establecer permisos
chmod 755 "$STAGE_DIR/usr/bin/pulsaros-welcome"
chmod 755 "$STAGE_DIR/usr/share/pulsaros-welcome/welcome.py"
[ -f "$STAGE_DIR/usr/bin/pulsar-cleanup-user" ] && chmod 755 "$STAGE_DIR/usr/bin/pulsar-cleanup-user"
[ -f "$STAGE_DIR/usr/libexec/pulsar-cleanup-live.sh" ] && chmod 755 "$STAGE_DIR/usr/libexec/pulsar-cleanup-live.sh"

echo "✅ pulsaros-welcome preparado con éxito."
