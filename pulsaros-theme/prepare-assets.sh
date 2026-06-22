#!/bin/bash
# ==============================================================================
# Pulsar OS - Theme Asset Preparer
# ==============================================================================
# Este script se ejecuta en caliente durante el empaquetado de pulsaros-theme.
# Descarga los repositorios de temas GTK e iconos y los posiciona en el paquete.
# ==============================================================================

set -e

STAGE_DIR="$1"
TEMP_BUILD="/tmp/pulsaros-theme-build"

THEME_REPO="https://github.com/Inled-Pulsar-OS/MacTahoe-gtk-theme"
ICONS_REPO="https://github.com/Inled-Pulsar-OS/MacTahoe-icon-theme"

echo "🎨 Descargando temas y configuraciones de diseño..."
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# 1. Clonar temas e iconos
echo "Clonando temas GTK..."
git clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"
echo "Clonando iconos..."
git clone --depth=1 "$ICONS_REPO" "$TEMP_BUILD/icons"

# 2. Instalar en la estructura temporal del paquete debian (Staging)
mkdir -p "$STAGE_DIR/usr/share/themes"
mkdir -p "$STAGE_DIR/usr/share/icons"

# Ejecutar instalador del tema GTK apuntando al directorio temporal
echo "Instalando temas GTK en staging..."
cd "$TEMP_BUILD/theme"
./install.sh -b -c dark -l -d "$STAGE_DIR/usr/share/themes" --silent-mode || {
    # Parchear si pide sudo interactivo
    sed -i 's/full_sudo "${1}"; silent_mode/silent_mode/g' tweaks.sh
    sed -i 's/elif \[\[ ! -d "${FIREFOX_DIR_HOME}" && ! -d "${FIREFOX_FLATPAK_DIR_HOME}" && ! -d "${FIREFOX_SNAP_DIR_HOME}" \]\]; then/elif false; then/g' tweaks.sh
    ./install.sh -b -c dark -l -d "$STAGE_DIR/usr/share/themes" --silent-mode
}

# Copiar configuración de GTK4 para Skel y Root (Libadwaita Fix)
mkdir -p "$STAGE_DIR/etc/skel/.config/gtk-4.0"
mkdir -p "$STAGE_DIR/root/.config/gtk-4.0"
cp -rf "$STAGE_DIR/usr/share/themes/MacTahoe-Dark/gtk-4.0/"* "$STAGE_DIR/etc/skel/.config/gtk-4.0/" 2>/dev/null || true
cp -rf "$STAGE_DIR/usr/share/themes/MacTahoe-Dark/gtk-4.0/"* "$STAGE_DIR/root/.config/gtk-4.0/" 2>/dev/null || true

# Ejecutar instalador de iconos
echo "Instalando iconos en staging..."
cd "$TEMP_BUILD/icons"
./install.sh -t blue -d "$STAGE_DIR/usr/share/icons"

# 3. Limpiar compilación temporal
rm -rf "$TEMP_BUILD"
echo "✅ Temas e iconos posicionados correctamente."
