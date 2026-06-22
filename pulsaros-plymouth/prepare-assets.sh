#!/bin/bash
# ==============================================================================
# Pulsar OS - Plymouth Theme Asset Preparer
# ==============================================================================
# Descarga e instala en el paquete el tema de Plymouth macOS-like.
# Configura el archivo daemon y oculta logotipos antiguos de Debian.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
TEMP_BUILD="/tmp/pulsaros-plymouth-build"
THEME_REPO="https://github.com/Inled-Pulsar-OS/plymouth-macoslike"

echo "🎨 Descargando tema Plymouth desde Github (depth=1)..."
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Clonar con depth=1
git clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"

# 1. Copiar el tema a staging
THEME_DEST="$STAGE_DIR/usr/share/plymouth/themes/pulsar-plymouth"
mkdir -p "$THEME_DEST"
cp -r "$TEMP_BUILD/theme"/* "$THEME_DEST/"

# Asegurar que el archivo header-image.png esté en la subcarpeta images/ si procede
if [ -f "$THEME_DEST/header-image.png" ]; then
    mkdir -p "$THEME_DEST/images"
    mv "$THEME_DEST/header-image.png" "$THEME_DEST/images/header-image.png"
fi

# 2. Configurar el archivo plymouthd.conf en staging
mkdir -p "$STAGE_DIR/etc/plymouth"
cat <<EOF > "$STAGE_DIR/etc/plymouth/plymouthd.conf"
[Daemon]
Theme=pulsar-plymouth
ShowDelay=0
DeviceTimeout=8
EOF

# 3. Reemplazar los logos de Debian por transparencia para no tener marcas duplicadas
echo "Generando reemplazo de logo transparente..."
mkdir -p "$STAGE_DIR/usr/share/plymouth/themes"
mkdir -p "$STAGE_DIR/usr/share/pixmaps"

# Crear logo transparente de 1x1 usando base64 (nativo en Linux coreutils)
echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=" | base64 -d > "$STAGE_DIR/usr/share/plymouth/debian-logo.png"

# Copiar a las rutas estándar
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/themes/debian-logo.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/plymouth/logo.png"
cp "$STAGE_DIR/usr/share/plymouth/debian-logo.png" "$STAGE_DIR/usr/share/pixmaps/debian-logo.png"

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Tema Plymouth estructurado correctamente en staging."
