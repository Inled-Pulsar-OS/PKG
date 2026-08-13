#!/bin/bash
# ==============================================================================
# Pulsar OS - Theme Asset Preparer
# ==============================================================================
# Descarga los repositorios de temas GTK e iconos y los compila en la estructura
# temporal de staging del paquete, sin instalar nada en el sistema del host.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
TEMP_BUILD="/tmp/pulsaros-theme-build"

THEME_REPO="https://github.com/Inled-Pulsar-OS/MacTahoe-gtk-theme"
ICONS_REPO="https://github.com/Inled-Pulsar-OS/MacTahoe-icon-theme"

echo "🎨 Descargando temas y configuraciones de diseño..."
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# 1. Clonar temas e iconos
echo "Clonando temas GTK (depth=1, HTTP/1.1 y límites de velocidad)..."
git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=20 clone --depth=1 "$THEME_REPO" "$TEMP_BUILD/theme"
echo "Clonando iconos (depth=1, HTTP/1.1 y límites de velocidad)..."
git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=20 clone --depth=1 "$ICONS_REPO" "$TEMP_BUILD/icons"

# 2. Instalar en la estructura temporal del paquete debian (Staging)
mkdir -p "$STAGE_DIR/usr/share/themes"
mkdir -p "$STAGE_DIR/usr/share/icons"

# Compilar e instalar el tema GTK en staging
echo "Instalando temas GTK en staging..."
cd "$TEMP_BUILD/theme"

# Parche agresivo: Eliminar la validación de root y la llamada a full_sudo de raíz
# en install.sh y tweaks.sh para que no salte en modo silent y no exija root en el host.
sed -i 's/full_sudo "${1}"; //g' install.sh tweaks.sh || true
sed -i 's/full_sudo "${1}"//g' install.sh tweaks.sh || true
sed -i 's/UID -ne 0/false/g' install.sh tweaks.sh || true
sed -i 's/EUID -ne 0/false/g' install.sh tweaks.sh || true
sed -i 's/elif \[\[ ! -d "${FIREFOX_DIR_HOME}" && ! -d "${FIREFOX_FLATPAK_DIR_HOME}" && ! -d "${FIREFOX_SNAP_DIR_HOME}" \]\]; then/elif false; then/g' tweaks.sh || true

# También neutralizar en libs/lib-core.sh por seguridad
if [ -f "libs/lib-core.sh" ]; then
    sed -i 's/! -w "\/root"/false/g' libs/lib-core.sh || true
fi

# Ejecutar instalador apuntando al staging
./install.sh -b -c dark -l -d "$STAGE_DIR/usr/share/themes" --silent-mode

# Copiar configuración de GTK4 para Skel y Root (Libadwaita Fix)
mkdir -p "$STAGE_DIR/etc/skel/.config/gtk-4.0"
mkdir -p "$STAGE_DIR/root/.config/gtk-4.0"
cp -rf "$STAGE_DIR/usr/share/themes/MacTahoe-Dark/gtk-4.0/"* "$STAGE_DIR/etc/skel/.config/gtk-4.0/" 2>/dev/null || true
cp -rf "$STAGE_DIR/usr/share/themes/MacTahoe-Dark/gtk-4.0/"* "$STAGE_DIR/root/.config/gtk-4.0/" 2>/dev/null || true

# 2.2 Aplicar fix para Nautilus moderno (Libadwaita en GNOME 46+)
echo "Aplicando fix de Libadwaita moderno para Nautilus..."
cat <<'NAUTILUS_FIX' > /tmp/nautilus_fix.css

/* ==============================================================================
 * Pulsar OS - Clean Modern Libadwaita Fix for Nautilus (GNOME Files)
 * ============================================================================== */
.nautilus-window,
#NautilusFileChooser {
    background-color: @window_bg_color;
}

.nautilus-window .sidebar-pane,
#NautilusFileChooser .sidebar-pane,
.sidebar-pane,
.content-pane .sidebar-pane,
.sidebar-pane .content-pane {
    border-radius: 0 !important;
    margin: 0 !important;
    padding: 0 !important;
    box-shadow: none !important;
    background-color: @sidebar_bg_color !important;
    background-image: none !important;
}

.nautilus-window .sidebar-pane:dir(ltr),
.nautilus-window .sidebar-pane:dir(rtl),
.nautilus-window .sidebar-pane.end:dir(ltr),
.nautilus-window .sidebar-pane.end:dir(rtl),
#NautilusFileChooser .sidebar-pane:dir(ltr),
#NautilusFileChooser .sidebar-pane:dir(rtl),
#NautilusFileChooser .sidebar-pane.end:dir(ltr),
#NautilusFileChooser .sidebar-pane.end:dir(rtl) {
    box-shadow: none !important;
}

.nautilus-window headerbar,
#NautilusFileChooser headerbar {
    background-color: transparent !important;
    box-shadow: none !important;
    margin: 0 !important;
}

.nautilus-window headerbar > windowhandle > box > widget > box.start > stack > widget > box,
.nautilus-window headerbar > windowhandle > box > widget > box.start > box > stack > widget > box,
#NautilusFileChooser headerbar > windowhandle > box > widget > box.start > stack > widget > box,
#NautilusFileChooser headerbar > windowhandle > box > widget > box.start > box > stack > widget > box {
    margin: 0 !important;
    padding: 0 !important;
    border-radius: 0 !important;
    background: none !important;
    background-image: none !important;
    box-shadow: none !important;
}

.nautilus-window .content-pane,
#NautilusFileChooser .content-pane {
    border-radius: 0 !important;
    background-color: @view_bg_color !important;
    box-shadow: none !important;
}

.nautilus-window placessidebar .navigation-sidebar > row,
#NautilusFileChooser placessidebar .navigation-sidebar > row {
    border-radius: 6px;
    margin: 1px 6px;
}

.nautilus-window placessidebar .navigation-sidebar > row:selected,
#NautilusFileChooser placessidebar .navigation-sidebar > row:selected {
    background-color: alpha(@accent_bg_color, 0.25) !important;
    color: @accent_fg_color !important;
}
NAUTILUS_FIX

for target_dir in \
    "$STAGE_DIR/usr/share/themes/MacTahoe-Dark/gtk-4.0" \
    "$STAGE_DIR/etc/skel/.config/gtk-4.0" \
    "$STAGE_DIR/root/.config/gtk-4.0"; do
    if [ -d "$target_dir" ]; then
        for css_f in "$target_dir"/*.css; do
            if [ -f "$css_f" ]; then
                cat /tmp/nautilus_fix.css >> "$css_f"
            fi
        done
    fi
done
rm -f /tmp/nautilus_fix.css

# Ejecutar instalador de iconos
echo "Instalando iconos en staging..."
cd "$TEMP_BUILD/icons"
./install.sh -t blue -d "$STAGE_DIR/usr/share/icons"

# 2.1 Mapear el icono de AppInstall a la App Store
# AppInstall (del repo de Inled) usa Icon=es.inled.AppInstall en su .desktop, que
# no tiene entrada en el tema MacTahoe. Como no lo empaquetamos, lo mapeamos
# aquí: es.inled.AppInstall -> software-store.svg (la "A" de la App Store).
echo "Mapeando icono de AppInstall a la App Store..."
for theme_dir in "$STAGE_DIR/usr/share/icons/"MacTahoe-blue*; do
    apps_dir="$theme_dir/apps/scalable"
    if [ -d "$apps_dir" ]; then
        ln -sf software-store.svg "$apps_dir/es.inled.AppInstall.svg"
    fi
done

# 3. Limpiar compilación temporal
rm -rf "$TEMP_BUILD"
echo "✅ Temas e iconos posicionados correctamente."
