#!/bin/bash
# ==============================================================================
# Pulsar OS - Plymouth Theme Asset Preparer
# ==============================================================================
# Descarga e instala en el paquete el tema de Plymouth macOS-like.
# Configura el archivo daemon y oculta logotipos antiguos de Debian.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
THEME_DEST="$STAGE_DIR/usr/share/plymouth/themes/pulsar-plymouth"
mkdir -p "$THEME_DEST"

# Check if the local 'repo' directory is present in the staging folder
# Comprobar si el directorio 'repo' local está presente en la carpeta staging
if [ -d "$STAGE_DIR/repo" ]; then
    echo "🎨 Copiando tema Plymouth desde el repositorio local..."
    # Copy theme assets from local repo
    # Copiar recursos del tema desde el repositorio local
    cp -r "$STAGE_DIR/repo"/* "$THEME_DEST/"
    # Remove the repo folder from staging to avoid packing it at the root of the deb package
    # Eliminar la carpeta repo de staging para evitar empaquetarla en la raíz del paquete deb
    rm -rf "$STAGE_DIR/repo"
else
    echo "⚠️ Directorio repo local no encontrado en staging. Descargando de respaldo desde Github..."
    TEMP_BUILD="/tmp/pulsaros-plymouth-build"
    THEME_REPO="https://github.com/Inled-Pulsar-OS/plymouth-macoslike"
    rm -rf "$TEMP_BUILD"
    mkdir -p "$TEMP_BUILD"
    
    # Clone with depth=1 from GitHub
    # Clonar con depth=1 desde GitHub
    git clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"
    cp -r "$TEMP_BUILD/theme"/* "$THEME_DEST/"
fi

# Ensure all assets are at the root of the theme directory (not in an images/ subdirectory)
# so that the Debian/Ubuntu initramfs hooks (which only glob *.png in the root theme folder)
# can properly copy them into the ramdisk.
# Asegurar que todos los recursos estén en la raíz del tema (no en la subcarpeta images/)
# para que los hooks de initramfs de Debian/Ubuntu (que solo copian *.png de la raíz)
# puedan incluirlos correctamente en el ramdisk.
if [ -d "$THEME_DEST/images" ]; then
    echo "📂 Aplanando estructura de imágenes del tema..."
    mv "$THEME_DEST/images"/* "$THEME_DEST/" || true
    rm -rf "$THEME_DEST/images"
fi

# 2. Configurar el archivo plymouthd.conf en staging
mkdir -p "$STAGE_DIR/etc/plymouth"
cat <<EOF > "$STAGE_DIR/etc/plymouth/plymouthd.conf"
[Daemon]
Theme=pulsar-plymouth
ShowDelay=0
DeviceTimeout=8
UseFirmwareBackground=false
EOF

# 3. Reemplazar los logos y marcas de agua de Debian por transparencia para no tener marcas duplicadas
echo "Generando reemplazo de logo transparente..."
mkdir -p "$STAGE_DIR/usr/share/plymouth/themes"
mkdir -p "$STAGE_DIR/usr/share/pixmaps"

# Crear logo transparente de 1x1 usando base64 (nativo en Linux coreutils)
echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=" | base64 -d > "$STAGE_DIR/usr/share/plymouth/debian-logo.png"

# Copiar a las rutas estándar
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/debian-logo.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/logo.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/pixmaps/debian-logo.png"

# Sobrescribir las marcas de agua de los temas estándar de Debian (que Plymouth a menudo carga)
mkdir -p "$STAGE_DIR/usr/share/plymouth/themes/spinner"
mkdir -p "$STAGE_DIR/usr/share/plymouth/themes/debian-spinner"
mkdir -p "$STAGE_DIR/usr/share/plymouth/themes/bgrt"

cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/spinner/watermark.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/debian-spinner/watermark.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/bgrt/watermark.png"

# Clean up temporary build directory if it was created
# Limpiar el directorio temporal de compilación si fue creado
if [ -d "$TEMP_BUILD" ]; then
    rm -rf "$TEMP_BUILD"
fi
echo "✅ Tema Plymouth estructurado correctamente en staging."
