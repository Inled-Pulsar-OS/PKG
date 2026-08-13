#!/bin/bash
# ==============================================================================
# Pulsar OS - rEFInd Theme Asset Preparer
# ==============================================================================
# English: Downloads the macOS rEFInd theme and places it in the package staging.
# Español: Descarga e instala en el paquete el tema rEFInd de macOS.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
TEMP_BUILD="/tmp/pulsaros-refind-build"
THEME_REPO="https://github.com/Inled-Pulsar-OS/refind-mac-theme"

echo "🎨 Descargando tema de rEFInd desde GitHub (depth=1)..."
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Clonar con depth=1, HTTP/1.1 y límites de velocidad
git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=20 clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"

# Copiar el tema a la ruta adecuada de staging para rEFInd
# Debe ser 'rEFInd-Regular-Dark' para coincidir con las rutas internas de theme.conf
DEST_DIR="$STAGE_DIR/usr/share/refind/themes/rEFInd-Regular-Dark"
mkdir -p "$DEST_DIR"
cp -r "$TEMP_BUILD/theme"/* "$DEST_DIR/"

# English: Strip out hardcoded test menuentries from theme.conf to prevent broken options
# Español: Eliminar menuentries de prueba cableados de theme.conf para evitar opciones rotas
sed -i '/#MENUENTRIES/q' "$DEST_DIR/theme.conf"

# Instalar iconos específicos de arranque de Pulsar OS Live
SCRIPT_DIR="$(dirname "$(realpath "$0")")"
BOOT_ICONS_DIR="$SCRIPT_DIR/../pulsar-boot-icons"
if [ -d "$BOOT_ICONS_DIR" ]; then
    echo "📦 Instalando iconos de arranque live personalizados en rEFInd..."
    mkdir -p "$DEST_DIR/icons"
    [ -f "$BOOT_ICONS_DIR/toram.png" ] && cp -f "$BOOT_ICONS_DIR/toram.png" "$DEST_DIR/icons/os_pulsaros_toram.png"
    [ -f "$BOOT_ICONS_DIR/normal.png" ] && cp -f "$BOOT_ICONS_DIR/normal.png" "$DEST_DIR/icons/os_pulsaros_normal.png"
    [ -f "$BOOT_ICONS_DIR/debug-noplymouth.png" ] && cp -f "$BOOT_ICONS_DIR/debug-noplymouth.png" "$DEST_DIR/icons/os_pulsaros_debug.png"
    [ -f "$BOOT_ICONS_DIR/old.png" ] && cp -f "$BOOT_ICONS_DIR/old.png" "$DEST_DIR/icons/os_pulsaros_old.png"
fi

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Tema de rEFInd estructurado en staging."
