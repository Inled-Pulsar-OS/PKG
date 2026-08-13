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

# Parchear SCSS para eliminar completamente las deformaciones de Nautilus heredadas de GNOME 40
if [ -f "src/sass/gtk/apps/_gnome-40.0.scss" ]; then
    echo "🎨 Limpiando SCSS heredado de Nautilus en GNOME 40..."
    python3 -c '
with open("src/sass/gtk/apps/_gnome-40.0.scss", "r") as f:
    lines = f.readlines()
# Eliminar las primeras 223 líneas correspondientes al styling obsoleto de Nautilus de GNOME 40
with open("src/sass/gtk/apps/_gnome-40.0.scss", "w") as f:
    f.writelines(lines[223:])
' || true
fi

if [ -f "src/sass/gtk/apps/_libadwaita.scss" ]; then
    echo "🎨 Limpiando bordes y márgenes forzados de splitview en Libadwaita SCSS..."
    python3 -c '
with open("src/sass/gtk/apps/_libadwaita.scss", "r") as f:
    c = f.read()
import re
c = re.sub(r"border-radius:\s*\$wm_radius\s*-\s*\$container_padding;", "border-radius: 0;", c)
c = re.sub(r"border-radius:\s*\$wm_radius;", "border-radius: 0;", c)
c = re.sub(r"margin:\s*\$container_padding;", "margin: 0;", c)
with open("src/sass/gtk/apps/_libadwaita.scss", "w") as f:
    f.write(c)
' || true
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
 * Pulsar OS - Pixel-Perfect macOS Capsules for Nautilus Headerbar
 * ============================================================================== */

/* 1. Base Window & SplitView */
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

.nautilus-window .content-pane,
#NautilusFileChooser .content-pane {
    border-radius: 0 !important;
    background-color: @view_bg_color !important;
    box-shadow: none !important;
}

/* 2. Headerbars Geometry & Alignment */
.nautilus-window headerbar,
#NautilusFileChooser headerbar {
    min-height: 44px !important;
    max-height: 44px !important;
    background-color: transparent !important;
    background-image: none !important;
    border: none !important;
    box-shadow: none !important;
    padding: 0 8px !important;
    margin: 0 !important;
}

/* 3. Traffic Lights Spacing on Top-Left */
.nautilus-window headerbar windowcontrols.start,
#NautilusFileChooser headerbar windowcontrols.start {
    margin-left: 6px !important;
    margin-right: 18px !important;
}

.nautilus-window headerbar windowtitle .title {
    font-weight: bold;
    font-size: 13px;
}

/* 4. Universal macOS Oval Capsules for ALL Linked Button Groups (Navigation, PathBar, ViewSwitcher) */
.nautilus-window headerbar .linked,
.nautilus-window headerbar box.linked,
.nautilus-window headerbar widget.linked,
.nautilus-window headerbar stackswitcher,
.nautilus-window headerbar viewswitcher,
.nautilus-window headerbar viewswitchertitle,
.nautilus-window headerbar .path-bar,
.nautilus-window headerbar pathbar,
.nautilus-window headerbar #NautilusPathBar,
#NautilusFileChooser headerbar .linked,
#NautilusFileChooser headerbar box.linked,
#NautilusFileChooser headerbar widget.linked,
#NautilusFileChooser headerbar stackswitcher,
#NautilusFileChooser headerbar viewswitcher,
#NautilusFileChooser headerbar .path-bar,
#NautilusFileChooser headerbar pathbar,
#NautilusFileChooser headerbar #NautilusPathBar {
    border-radius: 9999px !important;
    background-color: alpha(currentColor, 0.08) !important;
    background-image: none !important;
    border: 1px solid alpha(currentColor, 0.07) !important;
    padding: 2px !important;
    margin: 0 4px !important;
    box-shadow: none !important;
}

/* 5. Buttons and Dropdowns inside ANY Headerbar Linked Capsule */
.nautilus-window headerbar .linked > button,
.nautilus-window headerbar .linked > menubutton,
.nautilus-window headerbar .linked > menubutton > button,
.nautilus-window headerbar box.linked > button,
.nautilus-window headerbar box.linked > menubutton,
.nautilus-window headerbar box.linked > menubutton > button,
.nautilus-window headerbar widget.linked > button,
.nautilus-window headerbar widget.linked > menubutton > button,
.nautilus-window headerbar stackswitcher button,
.nautilus-window headerbar viewswitcher > button.toggle,
.nautilus-window headerbar viewswitcher button,
.nautilus-window headerbar .path-bar button,
.nautilus-window headerbar .path-bar menubutton > button,
.nautilus-window headerbar pathbar button,
.nautilus-window headerbar pathbar menubutton > button,
.nautilus-window headerbar #NautilusPathBar button,
.nautilus-window headerbar #NautilusPathBar menubutton > button,
.nautilus-window headerbar #NautilusPathBar #NautilusPathButton,
#NautilusFileChooser headerbar .linked > button,
#NautilusFileChooser headerbar .linked > menubutton > button,
#NautilusFileChooser headerbar box.linked > button,
#NautilusFileChooser headerbar box.linked > menubutton > button,
#NautilusFileChooser headerbar stackswitcher button,
#NautilusFileChooser headerbar viewswitcher > button.toggle,
#NautilusFileChooser headerbar .path-bar button,
#NautilusFileChooser headerbar .path-bar menubutton > button,
#NautilusFileChooser headerbar #NautilusPathBar button,
#NautilusFileChooser headerbar #NautilusPathBar #NautilusPathButton {
    border-radius: 9999px !important;
    min-height: 24px !important;
    min-width: 24px !important;
    padding: 2px 8px !important;
    margin: 0 !important;
    border: none !important;
    border-image: none !important;
    background: transparent !important;
    background-color: transparent !important;
    background-image: none !important;
    box-shadow: none !important;
    font-size: 13px !important;
    font-weight: 500 !important;
}

/* Hover State inside Linked Capsules */
.nautilus-window headerbar .linked > button:hover,
.nautilus-window headerbar .linked > menubutton > button:hover,
.nautilus-window headerbar box.linked > button:hover,
.nautilus-window headerbar box.linked > menubutton > button:hover,
.nautilus-window headerbar stackswitcher button:hover,
.nautilus-window headerbar viewswitcher > button.toggle:hover,
.nautilus-window headerbar .path-bar button:hover,
.nautilus-window headerbar .path-bar menubutton > button:hover,
.nautilus-window headerbar #NautilusPathBar button:hover,
.nautilus-window headerbar #NautilusPathBar menubutton > button:hover {
    background-color: alpha(currentColor, 0.1) !important;
}

/* Active / Checked State inside Linked Capsules (e.g. Active View Switcher Tab) */
.nautilus-window headerbar .linked > button:checked,
.nautilus-window headerbar .linked > button:active,
.nautilus-window headerbar box.linked > button:checked,
.nautilus-window headerbar box.linked > button:active,
.nautilus-window headerbar stackswitcher button:checked,
.nautilus-window headerbar stackswitcher button:active,
.nautilus-window headerbar viewswitcher > button.toggle:checked,
.nautilus-window headerbar viewswitcher > button.toggle:active,
.nautilus-window headerbar .path-bar button:active,
.nautilus-window headerbar .path-bar menubutton > button:active,
.nautilus-window headerbar #NautilusPathBar button:active,
.nautilus-window headerbar #NautilusPathBar menubutton > button:active {
    background-color: alpha(currentColor, 0.22) !important;
    color: @accent_fg_color !important;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25) !important;
}

/* 6. Standalone Buttons (New Folder, Primary Window Menu) - Circular */
.nautilus-window headerbar > windowhandle > box > button,
.nautilus-window headerbar > windowhandle > box > menubutton > button,
.nautilus-window headerbar > windowhandle > box.start > button,
.nautilus-window headerbar > windowhandle > box.end > button,
.nautilus-window headerbar > windowhandle > box.end > menubutton > button,
#NautilusFileChooser headerbar > windowhandle > box > button,
#NautilusFileChooser headerbar > windowhandle > box > menubutton > button,
.nautilus-window .sidebar-pane headerbar button,
.nautilus-window .sidebar-pane headerbar menubutton > button {
    border-radius: 9999px !important;
    min-height: 28px !important;
    min-width: 28px !important;
    padding: 4px !important;
    margin: 0 2px !important;
    border: none !important;
    background: transparent !important;
    box-shadow: none !important;
}

.nautilus-window headerbar > windowhandle > box > button:hover,
.nautilus-window headerbar > windowhandle > box > menubutton > button:hover,
.nautilus-window headerbar > windowhandle > box.start > button:hover,
.nautilus-window headerbar > windowhandle > box.end > button:hover,
.nautilus-window headerbar > windowhandle > box.end > menubutton > button:hover,
.nautilus-window .sidebar-pane headerbar button:hover,
.nautilus-window .sidebar-pane headerbar menubutton > button:hover {
    background-color: alpha(currentColor, 0.1) !important;
}

/* 8. Places Sidebar Rows */
.nautilus-window placessidebar .navigation-sidebar > row,
#NautilusFileChooser placessidebar .navigation-sidebar > row {
    border-radius: 6px;
    margin: 1px 6px;
    padding: 4px 8px;
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
