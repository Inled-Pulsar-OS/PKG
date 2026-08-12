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

# Check if the local 'repo' directory is present in the staging folder and contains the theme configuration
# Comprobar si el directorio 'repo' local está presente en la carpeta staging y contiene la configuración del tema
if [ -f "$STAGE_DIR/repo/pulsar-plymouth.plymouth" ]; then
    echo "🎨 Copiando tema Plymouth desde el repositorio local..."
    # Copy theme assets from local repo
    # Copiar recursos del tema desde el repositorio local
    cp -r "$STAGE_DIR/repo"/* "$THEME_DEST/"
    # Remove the repo folder from staging to avoid packing it at the root of the deb package
    # Eliminar la carpeta repo de staging para evitar empaquetarla en la raíz del paquete deb
    rm -rf "$STAGE_DIR/repo"
else
    echo "⚠️ Directorio repo local vacío o no encontrado en staging. Descargando de respaldo desde Github..."
    # If the local repo folder exists but is empty (e.g. submodule not initialized), clean it up first
    # Si la carpeta repo local existe pero está vacía (ej. submódulo no inicializado), limpiarla primero
    rm -rf "$STAGE_DIR/repo"
    
    TEMP_BUILD="/tmp/pulsaros-plymouth-build"
    THEME_REPO="https://github.com/Inled-Pulsar-OS/plymouth-macoslike"
    rm -rf "$TEMP_BUILD"
    mkdir -p "$TEMP_BUILD"
    
    # Clone with depth=1 from GitHub using HTTP/1.1, low speed timeouts, and larger postBuffer
    # Clonar con depth=1 desde GitHub usando HTTP/1.1, límites de velocidad y postBuffer mayor
    git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=20 clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"
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
UseSimpledrm=false
EOF

# 3. Generar la marca de agua transparente (sin texto ni marcas de base)
# 3. Generate transparent watermark (no text, clean look for all distros)
echo "🎨 Generando marca de agua transparente para Plymouth..."
echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=" | base64 -d > "$THEME_DEST/watermark.png"

# 4. Reemplazar los logos y marcas de agua de Debian del sistema por transparencia
# 4. Replace system Debian logos and watermarks with transparency to avoid double branding
echo "Generando reemplazo de logo transparente del sistema..."
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

# English: Force watermark alignment to be centered and at the bottom in the configuration file
# Español: Forzar la alineación del watermark para que esté centrado y abajo en el archivo de configuración
conf_file="$THEME_DEST/pulsar-plymouth.plymouth"
if [ -f "$conf_file" ]; then
    echo "🎨 Forzando alineación del logo watermark en el archivo de configuración del tema..."
    sed -i 's/^Watermark=.*/Watermark=watermark/' "$conf_file"
    # Ensure alignment keys exist or append them / Asegurar que existan las claves de alineación o agregarlas
    if ! grep -q "^WatermarkHorizontalAlignment" "$conf_file"; then
        echo "WatermarkHorizontalAlignment=.5" >> "$conf_file"
    fi
    if ! grep -q "^WatermarkVerticalAlignment" "$conf_file"; then
        echo "WatermarkVerticalAlignment=.96" >> "$conf_file"
    fi
fi

# Ensure the two-step theme renders the last boot message below the animation.
# Plymouth only shows messages when this key is enabled, and the splash log line
# feature (initramfs -> 'plymouth message') depends on it.
# Asegurar que el tema two-step muestre la última línea de log bajo la animación.
if [ -f "$conf_file" ] && ! grep -q "^MessageBelowAnimation=true" "$conf_file"; then
    echo "MessageBelowAnimation=true" >> "$conf_file"
fi

# Clean up temporary build directory if it was created
# Limpiar el directorio temporal de compilación si fue creado
if [ -d "$TEMP_BUILD" ]; then
    rm -rf "$TEMP_BUILD"
fi
echo "✅ Tema Plymouth estructurado correctamente en staging."
