#!/bin/bash
# ==============================================================================
# Pulsar OS - SDDM Apple Tahoe Theme Asset Preparer (Offline Version)
# ==============================================================================
# Extrae y configura el tema de SDDM "Apple Tahoe" utilizando el archivo
# Apple.Tahoe.tar.xz ya presente de forma local en la carpeta del paquete.
# Aplica parches de compatibilidad para Qt6/Breeze e instala tipografías.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
TEMP_BUILD="/tmp/pulsaros-sddm-build"

echo "🎨 Preparando tema SDDM Apple.Tahoe desde tarball local..."
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# El tarball ha sido copiado a la carpeta raíz de staging del paquete
LOCAL_TARBALL="$STAGE_DIR/Apple.Tahoe.tar.xz"

if [ ! -f "$LOCAL_TARBALL" ]; then
    echo "❌ Error: No se encontró el archivo $LOCAL_TARBALL en staging."
    exit 1
fi

# Descomprimir tar.xz
echo "Descomprimiendo el tema..."
tar -xf "$LOCAL_TARBALL" -C "$TEMP_BUILD/"

# Estructurar rutas en staging del paquete debian
THEMES_DEST_DIR="$STAGE_DIR/usr/share/sddm/themes/Apple.Tahoe"
mkdir -p "$THEMES_DEST_DIR"

# El tar.xz suele extraer una carpeta llamada 'Apple.Tahoe'
# Copiar el contenido descomprimido a la ruta de staging
if [ -d "$TEMP_BUILD/Apple.Tahoe" ]; then
    cp -r "$TEMP_BUILD/Apple.Tahoe"/* "$THEMES_DEST_DIR/"
else
    # Fallback si el nombre de la carpeta descomprimida varía
    UNPACKED_FOLDER=$(find "$TEMP_BUILD" -mindepth 1 -maxdepth 1 -type d | head -n 1)
    if [ -n "$UNPACKED_FOLDER" ]; then
        cp -r "$UNPACKED_FOLDER"/* "$THEMES_DEST_DIR/"
    else
        echo "❌ Error: No se encontró la carpeta descomprimida del tema en $TEMP_BUILD."
        exit 1
    fi
fi

# Parchear compatibilidad con Qt6 y Breeze (para sistemas basados en Debian 13 / KDE 6)
MAIN_QML="$THEMES_DEST_DIR/Main.qml"
if [ -f "$MAIN_QML" ]; then
    echo "Parcheando Main.qml para compatibilidad con Qt6/Breeze..."
    sed -i 's/import org.kde.breeze.components/import org.kde.breeze/g' "$MAIN_QML"
fi

# Si el tema contiene fuentes tipográficas, instalarlas a nivel de sistema en el paquete
FONTS_SRC_DIR="$THEMES_DEST_DIR/fonts"
if [ -d "$FONTS_SRC_DIR" ]; then
    echo "Instalando fuentes tipográficas asociadas al tema..."
    FONTS_DEST_DIR="$STAGE_DIR/usr/share/fonts/truetype/tahoe-sddm"
    mkdir -p "$FONTS_DEST_DIR"
    find "$FONTS_SRC_DIR" -type f \( -name "*.otf" -o -name "*.ttf" \) -exec cp {} "$FONTS_DEST_DIR/" \;
fi

# ==============================================================================
# LIMPIEZA CRÍTICA:
# Eliminar el archivo tarball comprimido del staging de forma que el paquete .deb
# final de pulsaros-sddm no contenga el archivo .tar.xz gigante e innecesario,
# sino solo los archivos ya descomprimidos en /usr/share/.
# ==============================================================================
echo "🧹 Eliminando tarball de staging para reducir tamaño del paquete..."
rm -f "$LOCAL_TARBALL"
rm -rf "$TEMP_BUILD"

echo "✅ Tema SDDM Apple.Tahoe estructurado y parcheado de forma offline."
