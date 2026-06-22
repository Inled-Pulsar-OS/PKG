#!/bin/bash
# ==============================================================================
# Pulsar OS - SDDM Apple Tahoe Theme Asset Preparer
# ==============================================================================
# Configura el tema de SDDM "Apple Tahoe" utilizando la carpeta descomprimida.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
THEME_SRC_DIR="$STAGE_DIR/Apple.Tahoe"
THEMES_DEST_DIR="$STAGE_DIR/usr/share/sddm/themes/Apple.Tahoe"

if [ ! -d "$THEME_SRC_DIR" ]; then
    echo "❌ Error: No se encontró la carpeta del tema $THEME_SRC_DIR."
    exit 1
fi

echo "🎨 Copiando tema SDDM Apple.Tahoe a la ruta de instalación..."
mkdir -p "$THEMES_DEST_DIR"
cp -r "$THEME_SRC_DIR"/* "$THEMES_DEST_DIR/"

# Si el tema contiene fuentes tipográficas, instalarlas a nivel de sistema
FONTS_SRC_DIR="$THEMES_DEST_DIR/fonts"
if [ -d "$FONTS_SRC_DIR" ]; then
    echo "Instalando fuentes tipográficas asociadas al tema..."
    FONTS_DEST_DIR="$STAGE_DIR/usr/share/fonts/truetype/tahoe-sddm"
    mkdir -p "$FONTS_DEST_DIR"
    find "$FONTS_SRC_DIR" -type f \( -name "*.otf" -o -name "*.ttf" \) -exec cp {} "$FONTS_DEST_DIR/" \;
fi

# Limpieza: eliminar la carpeta de origen del tema de la raíz de staging
# para que no se empaquete doble (evita duplicar espacio en el .deb)
echo "🧹 Limpiando archivos fuente del staging..."
rm -rf "$THEME_SRC_DIR"
rm -f "$STAGE_DIR/Apple.Tahoe.tar.xz"

echo "✅ Tema SDDM Apple.Tahoe preparado con éxito."
