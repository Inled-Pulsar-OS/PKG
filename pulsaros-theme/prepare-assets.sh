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
    sed -i 's/MY_USERNAME=.*/MY_USERNAME=\"\${USER:-root}\"/g' libs/lib-core.sh || true
    sed -i 's/MY_HOME=.*/MY_HOME=\"\${HOME:-\/root}\"/g' libs/lib-core.sh || true
    sed -i 's/SUDO_BIN=.*/SUDO_BIN=\"\"/g' libs/lib-core.sh || true
    sed -i 's/! -w "\/root"/false/g' libs/lib-core.sh || true
    sed -i 's/elif \[\[ ! -w .*/elif false; then/g' libs/lib-core.sh || true
fi

mkdir -p "$STAGE_DIR/usr/share/themes"
mkdir -p "$STAGE_DIR/usr/share/icons"

# Ejecutar instalador apuntando al staging (tanto variante clara como oscura)
./install.sh -b -c light -c dark -d "$STAGE_DIR/usr/share/themes" --silent-mode

# Copiar configuración de GTK4 para Skel y Root (Libadwaita Fix)
mkdir -p "$STAGE_DIR/etc/skel/.config/gtk-4.0"
mkdir -p "$STAGE_DIR/root/.config/gtk-4.0"
cp -rf "$STAGE_DIR/usr/share/themes/MacTahoe-Light/gtk-4.0/"* "$STAGE_DIR/etc/skel/.config/gtk-4.0/" 2>/dev/null || true
cp -rf "$STAGE_DIR/usr/share/themes/MacTahoe-Light/gtk-4.0/"* "$STAGE_DIR/root/.config/gtk-4.0/" 2>/dev/null || true
if [ -f "$STAGE_DIR/etc/skel/.config/gtk-4.0/gtk.css" ]; then
    cp -f "$STAGE_DIR/etc/skel/.config/gtk-4.0/gtk.css" "$STAGE_DIR/etc/skel/.config/gtk-4.0/gtk-dark.css" 2>/dev/null || true
fi
if [ -f "$STAGE_DIR/root/.config/gtk-4.0/gtk.css" ]; then
    cp -f "$STAGE_DIR/root/.config/gtk-4.0/gtk.css" "$STAGE_DIR/root/.config/gtk-4.0/gtk-dark.css" 2>/dev/null || true
fi

# Permitir que el sistema nativo de colores de acento de GNOME / Libadwaita controle los botones y temas
echo "Habilitando colores de acento dinámicos en temas MacTahoe..."
find "$STAGE_DIR" -name "*.css" -exec sed -i '/@define-color accent_/d' {} + 2>/dev/null || true
find "$STAGE_DIR" -name "*.css" -exec sed -i '/@define-color theme_selected_/d' {} + 2>/dev/null || true
find "$STAGE_DIR" -name "*.css" -exec sed -i '/@define-color selected_/d' {} + 2>/dev/null || true
find "$STAGE_DIR/usr/share/themes" -name "*.css" -exec sed -i 's/#0088FF/@accent_bg_color/g' {} + 2>/dev/null || true
find "$STAGE_DIR/usr/share/themes" -name "*.css" -exec sed -i 's/#0088ff/@accent_bg_color/g' {} + 2>/dev/null || true
find "$STAGE_DIR/etc/skel" -name "*.css" -exec sed -i 's/#0088FF/@accent_bg_color/g' {} + 2>/dev/null || true
find "$STAGE_DIR/etc/skel" -name "*.css" -exec sed -i 's/#0088ff/@accent_bg_color/g' {} + 2>/dev/null || true
find "$STAGE_DIR/root" -name "*.css" -exec sed -i 's/#0088FF/@accent_bg_color/g' {} + 2>/dev/null || true
find "$STAGE_DIR/root" -name "*.css" -exec sed -i 's/#0088ff/@accent_bg_color/g' {} + 2>/dev/null || true

# Prepend clean fallback accent definitions to ALL gtk-3.0 and gtk-4.0 css files
find "$STAGE_DIR" -type f -name "*.css" | while read -r css_file; do
    if [ -f "$css_file" ]; then
        sed -i '1s/^/@define-color accent_color #3584e4;\n@define-color accent_bg_color #3584e4;\n@define-color accent_fg_color #ffffff;\n@define-color theme_selected_bg_color #3584e4;\n@define-color theme_selected_fg_color #ffffff;\n@define-color selected_bg_color #3584e4;\n@define-color selected_fg_color #ffffff;\n/' "$css_file"
    fi
done

# Añadir estilos explícitos para selección de texto sólida y botones de color de acento
cat <<'ACCENT_BTN_FIX' > /tmp/accent_btn_fix.css

/* ==============================================================================
 * Pulsar OS - Text Selection Fix (Solid, Non-Transparent Selection)
 * ============================================================================== */
selection {
  background-color: @accent_bg_color;
  color: @accent_fg_color;
}

entry selection,
entry:focus selection,
textview text selection,
textview selection,
label:selected,
.view:selected,
.view:selected:focus,
*:selected {
  background-color: @accent_bg_color;
  color: @accent_fg_color;
}

/* ==============================================================================
 * Pulsar OS - Context Menu / Popup Menu Hover & Highlight
 * ============================================================================== */
menu > menuitem:hover,
menu > menuitem:selected,
menuitem.button.flat:hover,
menuitem.button.flat:selected,
modelbutton.flat:hover,
modelbutton.flat:selected {
  background-color: @accent_bg_color;
  color: @accent_fg_color;
}

menu > menuitem:hover label,
menu > menuitem:selected label,
menu > menuitem:hover arrow,
menu > menuitem:selected arrow,
menuitem.button.flat:hover label,
menuitem.button.flat:selected label,
modelbutton.flat:hover label,
modelbutton.flat:selected label {
  color: @accent_fg_color;
}

popover.menu modelbutton:hover,
popover.menu modelbutton:selected,
popover.menu modelbutton:focus:hover {
  background-color: @accent_bg_color;
  color: @accent_fg_color;
}

popover.menu modelbutton:hover label,
popover.menu modelbutton:selected label,
popover.menu modelbutton:hover arrow,
popover.menu modelbutton:selected arrow {
  color: @accent_fg_color;
}

/* ==============================================================================
 * Pulsar OS - Accent Color Selector Previews (GNOME Settings / Libadwaita)
 * ============================================================================== */
button.accent-button {
  min-width: 28px;
  min-height: 28px;
  border-radius: 9999px;
  padding: 0;
  margin: 4px;
  border: 2px solid rgba(255, 255, 255, 0.2);
  box-shadow: 0 2px 5px rgba(0, 0, 0, 0.25);
  transition: all 150ms ease;
}
button.accent-button:hover {
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.35);
}
button.accent-button:checked {
  border: 2.5px solid #ffffff;
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.6), 0 4px 10px rgba(0, 0, 0, 0.4);
}
button.accent-button.blue { background-color: #3584e4; background-image: none; }
button.accent-button.teal { background-color: #2190a4; background-image: none; }
button.accent-button.green { background-color: #3a944a; background-image: none; }
button.accent-button.yellow { background-color: #e5a50a; background-image: none; }
button.accent-button.orange { background-color: #e66100; background-image: none; }
button.accent-button.red { background-color: #e01b24; background-image: none; }
button.accent-button.pink { background-color: #d56199; background-image: none; }
button.accent-button.purple { background-color: #9141ac; background-image: none; }
button.accent-button.slate { background-color: #6f8396; background-image: none; }

/* ==============================================================================
 * Apple Liquid Glass HIG - Specular Rim Highlight & Adaptive Focus Contrast
 * ============================================================================== */
.nautilus-window headerbar .linked,
.nautilus-pathbar,
headerbar box.linked {
  box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.18), 0 2px 8px rgba(0, 0, 0, 0.2);
  border: 1px solid rgba(255, 255, 255, 0.12);
}
ACCENT_BTN_FIX

find "$STAGE_DIR" -path "*/gtk-4.0/gtk.css" -exec sh -c 'cat /tmp/accent_btn_fix.css >> "$1"' _ {} \; 2>/dev/null || true
find "$STAGE_DIR" -path "*/gtk-3.0/gtk.css" -exec sh -c 'cat /tmp/accent_btn_fix.css >> "$1"' _ {} \; 2>/dev/null || true
find "$STAGE_DIR" -path "*/gtk-4.0/gtk-dark.css" -exec sh -c 'cat /tmp/accent_btn_fix.css >> "$1"' _ {} \; 2>/dev/null || true
find "$STAGE_DIR" -path "*/gtk-3.0/gtk-dark.css" -exec sh -c 'cat /tmp/accent_btn_fix.css >> "$1"' _ {} \; 2>/dev/null || true
rm -f /tmp/accent_btn_fix.css

# Copiar también configuración básica de GTK3 a Skel y Root
mkdir -p "$STAGE_DIR/etc/skel/.config/gtk-3.0" "$STAGE_DIR/etc/skel/.config/gtk-4.0"
mkdir -p "$STAGE_DIR/root/.config/gtk-3.0" "$STAGE_DIR/root/.config/gtk-4.0"
cp -rf "$STAGE_DIR/usr/share/themes/MacTahoe-Light/gtk-3.0/gtk.css" "$STAGE_DIR/etc/skel/.config/gtk-3.0/gtk.css" 2>/dev/null || true
cp -rf "$STAGE_DIR/usr/share/themes/MacTahoe-Light/gtk-3.0/gtk.css" "$STAGE_DIR/root/.config/gtk-3.0/gtk.css" 2>/dev/null || true

cat <<'SETTINGS_EOF' > "$STAGE_DIR/etc/skel/.config/gtk-3.0/settings.ini"
[Settings]
gtk-theme-name=MacTahoe-Light
gtk-icon-theme-name=MacTahoe-blue-light
gtk-application-prefer-dark-theme=0
gtk-decoration-layout=close,minimize,maximize:
SETTINGS_EOF
cp -f "$STAGE_DIR/etc/skel/.config/gtk-3.0/settings.ini" "$STAGE_DIR/etc/skel/.config/gtk-4.0/settings.ini"
cp -f "$STAGE_DIR/etc/skel/.config/gtk-3.0/settings.ini" "$STAGE_DIR/root/.config/gtk-3.0/settings.ini"
cp -f "$STAGE_DIR/etc/skel/.config/gtk-3.0/settings.ini" "$STAGE_DIR/root/.config/gtk-4.0/settings.ini"

# 2.2 Aplicar fix para Nautilus moderno (Libadwaita en GNOME 46+)
echo "Aplicando fix de Libadwaita moderno para Nautilus..."
cat <<'NAUTILUS_FIX' > /tmp/nautilus_fix.css

/* ==============================================================================
 * Pulsar OS - Pixel-Perfect macOS Capsules for Nautilus Headerbar
 * ============================================================================== */
/* 1. Headerbars Geometry */
.nautilus-window headerbar,
#NautilusFileChooser headerbar {
    min-height: 44px;
    background-color: transparent;
    background-image: none;
    border-style: none;
    box-shadow: none;
}

/* 2. Universal Linked Button Groups (View Switcher, Navigation) */
.nautilus-window headerbar .linked,
.nautilus-window headerbar box.linked,
.nautilus-window headerbar widget.linked,
.nautilus-window headerbar stackswitcher,
.nautilus-window headerbar viewswitcher,
.nautilus-window headerbar viewswitchertitle,
#NautilusFileChooser headerbar .linked,
#NautilusFileChooser headerbar box.linked,
#NautilusFileChooser headerbar widget.linked,
#NautilusFileChooser headerbar stackswitcher,
#NautilusFileChooser headerbar viewswitcher {
    border-radius: 9999px;
    background-color: alpha(currentColor, 0.08);
    background-image: none;
    border: 1px solid alpha(currentColor, 0.07);
    padding: 2px;
    margin: 0 4px;
    box-shadow: none;
}

/* All buttons inside any linked pill in headerbar */
.nautilus-window headerbar .linked > button,
.nautilus-window headerbar .linked > button:first-child,
.nautilus-window headerbar .linked > button:last-child,
.nautilus-window headerbar .linked > button:not(:first-child):not(:last-child),
.nautilus-window headerbar .linked > menubutton > button,
.nautilus-window headerbar box.linked > button,
.nautilus-window headerbar box.linked > button:first-child,
.nautilus-window headerbar box.linked > button:last-child,
.nautilus-window headerbar box.linked > button:not(:first-child):not(:last-child),
.nautilus-window headerbar box.linked > menubutton > button,
.nautilus-window headerbar widget.linked > button,
.nautilus-window headerbar stackswitcher button,
.nautilus-window headerbar viewswitcher > button.toggle,
.nautilus-window headerbar viewswitcher button,
#NautilusFileChooser headerbar .linked > button,
#NautilusFileChooser headerbar box.linked > button,
#NautilusFileChooser headerbar stackswitcher button,
#NautilusFileChooser headerbar viewswitcher > button.toggle {
    border-radius: 9999px;
    min-height: 24px;
    min-width: 24px;
    padding: 2px 8px;
    margin: 0;
    border-style: none;
    border-image: none;
    background: transparent;
    background-color: transparent;
    background-image: none;
    box-shadow: none;
    font-size: 13px;
    font-weight: 500;
}

/* Hover State inside Linked Capsules */
.nautilus-window headerbar .linked > button:hover,
.nautilus-window headerbar .linked > menubutton > button:hover,
.nautilus-window headerbar box.linked > button:hover,
.nautilus-window headerbar box.linked > menubutton > button:hover,
.nautilus-window headerbar stackswitcher button:hover,
.nautilus-window headerbar viewswitcher > button.toggle:hover {
    background-color: alpha(currentColor, 0.1);
}

/* Active / Checked State inside Linked Capsules (Active View Switcher Tab) */
.nautilus-window headerbar .linked > button:checked,
.nautilus-window headerbar .linked > button:active,
.nautilus-window headerbar box.linked > button:checked,
.nautilus-window headerbar box.linked > button:active,
.nautilus-window headerbar stackswitcher button:checked,
.nautilus-window headerbar stackswitcher button:active,
.nautilus-window headerbar viewswitcher > button.toggle:checked,
.nautilus-window headerbar viewswitcher > button.toggle:active {
    background-color: alpha(currentColor, 0.22);
    color: @accent_fg_color;
    border-radius: 9999px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
}

/* 3. PathBar - The ONE AND ONLY Capsule Background */
.nautilus-pathbar,
.nautilus-window .path-bar,
.nautilus-window headerbar pathbar {
    border-radius: 9999px;
    background-color: alpha(currentColor, 0.08);
    background-image: none;
    border-style: none;
    box-shadow: none;
    padding: 1px 4px;
    margin: 0 4px;
    min-height: 24px;
}

/* ALL Inner Path Elements (Home, ..., etc) - 100% Transparent, ZERO sub-backgrounds */
.nautilus-path-button,
.nautilus-path-button:hover,
.nautilus-path-button:active,
.nautilus-path-button:checked,
.nautilus-path-button.current-dir,
.nautilus-pathbar button,
.nautilus-pathbar button:hover,
.nautilus-pathbar button:active,
.nautilus-pathbar menubutton,
.nautilus-pathbar menubutton > button,
.nautilus-pathbar menubutton > button:hover,
.nautilus-pathbar menubutton > button:active,
.nautilus-pathbar menubutton > button:checked,
.nautilus-pathbar > menubutton,
.nautilus-pathbar > menubutton > button,
.nautilus-pathbar > menubutton > button:hover,
.nautilus-pathbar > menubutton > button:active,
.nautilus-pathbar > menubutton > button:checked,
.nautilus-pathbar > scrolledwindow menubutton > button,
.nautilus-window .path-bar button,
.nautilus-window .path-bar menubutton > button {
    border-radius: 9999px;
    min-height: 22px;
    padding: 1px 6px;
    margin: 0;
    border-style: none;
    border-image: none;
    background: transparent;
    background-color: transparent;
    background-image: none;
    box-shadow: none;
    font-size: 13px;
    font-weight: 500;
}

/* 4. Standalone Buttons (New Folder, Window Options) */
.nautilus-window headerbar > windowhandle > box > button,
.nautilus-window headerbar > windowhandle > box > menubutton > button,
.nautilus-window headerbar > windowhandle > box.start > button,
.nautilus-window headerbar > windowhandle > box.end > button,
.nautilus-window headerbar > windowhandle > box.end > menubutton > button,
#NautilusFileChooser headerbar > windowhandle > box > button,
#NautilusFileChooser headerbar > windowhandle > box > menubutton > button {
    border-radius: 9999px;
    min-height: 28px;
    min-width: 28px;
    padding: 4px;
    margin: 0 2px;
    border-style: none;
    background: transparent;
    box-shadow: none;
}

.nautilus-window headerbar > windowhandle > box > button:hover,
.nautilus-window headerbar > windowhandle > box > menubutton > button:hover,
.nautilus-window headerbar > windowhandle > box.start > button:hover,
.nautilus-window headerbar > windowhandle > box.end > button:hover,
.nautilus-window headerbar > windowhandle > box.end > menubutton > button:hover {
    background-color: alpha(currentColor, 0.1);
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

# 2.3 Pulsar OS - Dock Icon Hover Transparency & Zero Shadow Fix for GNOME Shell Theme
# NOTA: los selectores deben igualar/superar la especificidad del propio tema
# (#dash .dash-item-container ...:hover .overview-icon y la variante con
# #dashtodockContainer), si no, el gris de hover sigue ganando.
cat <<'DOCK_HOVER_FIX' > /tmp/dock_hover_fix.css
/* ==============================================================================
 * Pulsar OS - Dock Icon Hover Transparency & Zero Shadow Fix
 * ============================================================================== */
#dashtodockContainer #dash .dash-item-container .show-apps .overview-icon,
#dashtodockContainer #dash .dash-item-container .show-apps:hover .overview-icon,
#dashtodockContainer #dash .dash-item-container .show-apps:focus .overview-icon,
#dashtodockContainer #dash .dash-item-container .show-apps:active .overview-icon,
#dashtodockContainer #dash .dash-item-container .show-apps:highlighted .overview-icon,
#dashtodockContainer #dash .dash-item-container .show-apps:selected .overview-icon,
#dashtodockContainer #dash .dash-item-container .show-apps:checked .overview-icon,
#dashtodockContainer #dash .dash-item-container .overview-tile .overview-icon,
#dashtodockContainer #dash .dash-item-container .overview-tile:hover .overview-icon,
#dashtodockContainer #dash .dash-item-container .overview-tile:focus .overview-icon,
#dashtodockContainer #dash .dash-item-container .overview-tile:active .overview-icon,
#dashtodockContainer #dash .dash-item-container .overview-tile:highlighted .overview-icon,
#dashtodockContainer #dash .dash-item-container .overview-tile:selected .overview-icon,
#dashtodockContainer #dash .dash-item-container .overview-tile:checked .overview-icon,
#dash .dash-item-container .show-apps .overview-icon,
#dash .dash-item-container .show-apps:hover .overview-icon,
#dash .dash-item-container .show-apps:focus .overview-icon,
#dash .dash-item-container .show-apps:active .overview-icon,
#dash .dash-item-container .show-apps:highlighted .overview-icon,
#dash .dash-item-container .show-apps:selected .overview-icon,
#dash .dash-item-container .show-apps:checked .overview-icon,
#dash .dash-item-container .overview-tile .overview-icon,
#dash .dash-item-container .overview-tile:hover .overview-icon,
#dash .dash-item-container .overview-tile:focus .overview-icon,
#dash .dash-item-container .overview-tile:active .overview-icon,
#dash .dash-item-container .overview-tile:highlighted .overview-icon,
#dash .dash-item-container .overview-tile:selected .overview-icon,
#dash .dash-item-container .overview-tile:checked .overview-icon,
#dashtodockContainer .app-well-app .overview-icon,
#dashtodockContainer .app-well-app:hover .overview-icon,
#dashtodockContainer .app-well-app,
#dashtodockContainer .app-well-app:hover,
#dashtodockContainer .app-well-app:focus,
#dashtodockContainer .app-well-app:active,
#dashtodockContainer .app-well-app:selected,
#dashtodockContainer .app-well-app:checked,
#dashtodockContainer .show-apps,
#dashtodockContainer .show-apps:hover,
#dashtodockContainer .show-apps:focus,
#dashtodockContainer .show-apps:active,
#dashtodockContainer .show-apps:selected,
#dashtodockContainer .show-apps:checked,
#dashtodockContainer .overview-icon,
#dashtodockContainer .overview-icon:hover,
#dashtodockContainer .overview-icon:focus,
#dashtodockContainer .overview-icon:active,
#dashtodockContainer .overview-icon:selected,
#dashtodockContainer .overview-icon:checked,
#dashtodockContainer .dash-item-container > StButton,
#dashtodockContainer .dash-item-container > StButton:hover,
#dashtodockContainer .dash-item-container > StButton:focus,
#dashtodockContainer .dash-item-container > StButton:active,
#dashtodockContainer .dash-item-container > StButton:checked {
    background-color: transparent !important;
    background-image: none !important;
    box-shadow: none !important;
    icon-shadow: none !important;
    border: none !important;
    border-color: transparent !important;
}

/* Nivel tile: el tema tambien pinta .overview-tile:hover/.show-apps:hover
 * directamente (sin prefijo #dash), cubrirlo tambien. */
#dash .dash-item-container .overview-tile,
#dash .dash-item-container .overview-tile:hover,
#dash .dash-item-container .overview-tile:focus,
#dash .dash-item-container .overview-tile:active,
#dash .dash-item-container .overview-tile:highlighted,
#dash .dash-item-container .overview-tile:selected,
#dash .dash-item-container .overview-tile:checked,
#dash .dash-item-container .show-apps,
#dash .dash-item-container .show-apps:hover,
#dash .dash-item-container .show-apps:focus,
#dash .dash-item-container .show-apps:active,
#dash .dash-item-container .show-apps:highlighted,
#dash .dash-item-container .show-apps:selected,
#dash .dash-item-container .show-apps:checked,
#dashtodockContainer #dash .dash-item-container .overview-tile:hover,
#dashtodockContainer #dash .dash-item-container .overview-tile:focus,
#dashtodockContainer #dash .dash-item-container .overview-tile:active,
#dashtodockContainer #dash .dash-item-container .overview-tile:highlighted,
#dashtodockContainer #dash .dash-item-container .overview-tile:selected,
#dashtodockContainer #dash .dash-item-container .overview-tile:checked,
#dashtodockContainer #dash .dash-item-container .show-apps:hover,
#dashtodockContainer #dash .dash-item-container .show-apps:focus,
#dashtodockContainer #dash .dash-item-container .show-apps:active,
#dashtodockContainer #dash .dash-item-container .show-apps:highlighted,
#dashtodockContainer #dash .dash-item-container .show-apps:selected,
#dashtodockContainer #dash .dash-item-container .show-apps:checked {
    background-color: transparent !important;
    background-image: none !important;
    box-shadow: none !important;
}
DOCK_HOVER_FIX

find "$STAGE_DIR" -path "*/gnome-shell/gnome-shell.css" -exec sh -c 'cat /tmp/dock_hover_fix.css >> "$1"' _ {} \; 2>/dev/null || true
rm -f /tmp/dock_hover_fix.css

# 2.4 Pulsar OS - OSD translucido y flecha de submenu sin circulo azul
# NOTA 1: just-perfection deja el OSD opaco con
# (.just-perfection .osd-window { background-color: rgba(0,0,0,1) }), que es
# mas especifico que .osd-window: por eso hace falta !important. El valor
# nuevo conserva el contraste del texto pero deja entrever el fondo.
# NOTA 2: .quick-toggle-menu-button:checked hereda -st-accent-color del tema
# oscuro de GNOME y eso pinta el circulo azul en la flecha de los botones con
# submenu de Ajustes rapidos. Se anula el fondo y se deja un anillo neutro.
cat <<'SHELL_OSD_FIX' > /tmp/shell_osd_fix.css
/* ==========================================================================
 * Pulsar OS - Ajustes locales del tema de GNOME Shell (no vienen de MacTahoe)
 * 1) OSD (volumen/brillo/wifi): translucido y con contraste.
 * 2) Flecha de los botones con submenu de Ajustes rapidos: sin circulo azul.
 * ========================================================================== */

.osd-window {
    background-color: rgba(36, 36, 36, 0.66) !important;
    border: solid rgba(255, 255, 255, 0.14) !important;
    border-width: 1px !important;
    box-shadow: 0 4px 14px 0 rgba(0, 0, 0, 0.45) !important;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.75);
}

.quick-toggle-has-menu .quick-toggle-menu-button:checked,
.quick-toggle-has-menu .quick-toggle-menu-button:checked:hover,
.quick-toggle-has-menu .quick-toggle-menu-button:checked:active {
    background-color: transparent !important;
    color: white !important;
}

.quick-toggle-has-menu .quick-toggle-menu-button:focus,
.quick-toggle-has-menu .quick-toggle-menu-button:focus:hover,
.quick-toggle-has-menu .quick-toggle-menu-button:focus:active,
.quick-toggle-has-menu .quick-toggle-menu-button:checked:focus,
.quick-toggle-has-menu .quick-toggle-menu-button:checked:focus:hover {
    background-color: rgba(255, 255, 255, 0.16) !important;
    box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.55) !important;
    color: white !important;
}
SHELL_OSD_FIX

find "$STAGE_DIR" -path "*/gnome-shell/gnome-shell.css" -exec sh -c 'cat /tmp/shell_osd_fix.css >> "$1"' _ {} \; 2>/dev/null || true
rm -f /tmp/shell_osd_fix.css

# 2.5 Pulsar OS - macOS Rounded Capsule Modal Action Buttons
cat <<'SHELL_MODAL_DARK_FIX' > /tmp/shell_modal_dark_fix.css
/* ==============================================================================
 * Pulsar OS - Clean macOS Rounded Capsule Modal Buttons (Dark)
 * ============================================================================== */

/* Button box layout: macOS spacing between action buttons */
.modal-dialog .modal-dialog-button-box {
  padding-top: 14px !important;
  spacing: 12px !important;
}

/* 1. Base / Secondary / Cancel Button */
.modal-dialog .modal-dialog-button,
.modal-dialog .modal-dialog-linked-button,
.modal-dialog .button,
.modal-dialog-button {
  min-height: 38px !important;
  padding: 0 22px !important;
  margin: 0 !important;
  border-radius: 9999px !important;
  font-size: 13px !important;
  font-weight: 600 !important;
  border: none !important;
  box-shadow: none !important;
  text-shadow: none !important;
  icon-shadow: none !important;
  background-color: rgba(255, 255, 255, 0.12) !important;
  color: #ffffff !important;
  transition-duration: 150ms !important;
}

.modal-dialog .modal-dialog-button:hover,
.modal-dialog .modal-dialog-linked-button:hover,
.modal-dialog .button:hover,
.modal-dialog-button:hover {
  background-color: rgba(255, 255, 255, 0.20) !important;
  border: none !important;
  box-shadow: none !important;
  color: #ffffff !important;
}

.modal-dialog .modal-dialog-button:active,
.modal-dialog .modal-dialog-linked-button:active,
.modal-dialog .button:active,
.modal-dialog-button:active {
  background-color: rgba(255, 255, 255, 0.28) !important;
  border: none !important;
  box-shadow: none !important;
  color: #ffffff !important;
}

.modal-dialog .modal-dialog-button:focus,
.modal-dialog .modal-dialog-linked-button:focus,
.modal-dialog .button:focus,
.modal-dialog-button:focus {
  background-color: rgba(255, 255, 255, 0.18) !important;
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.35) !important;
  border: none !important;
  color: #ffffff !important;
}

.modal-dialog .modal-dialog-button:insensitive,
.modal-dialog .modal-dialog-linked-button:insensitive,
.modal-dialog .button:insensitive,
.modal-dialog-button:insensitive {
  background-color: rgba(255, 255, 255, 0.05) !important;
  border: none !important;
  box-shadow: none !important;
  color: rgba(255, 255, 255, 0.35) !important;
}

/* 2. Primary / Default Action Button (Blue / Accent) */
.modal-dialog .modal-dialog-button:last-child,
.modal-dialog .modal-dialog-linked-button:last-child,
.modal-dialog .button:last-child,
.modal-dialog .modal-dialog-button:default,
.modal-dialog .modal-dialog-linked-button:default,
.modal-dialog .button:default,
.modal-dialog .modal-dialog-button.default,
.modal-dialog .modal-dialog-button:checked,
.modal-dialog .modal-dialog-linked-button:checked {
  color: #ffffff !important;
  background-color: -st-accent-color !important;
  border: none !important;
  box-shadow: none !important;
  background-gradient-direction: none !important;
}

.modal-dialog .modal-dialog-button:hover:last-child,
.modal-dialog .modal-dialog-linked-button:hover:last-child,
.modal-dialog .button:hover:last-child,
.modal-dialog .modal-dialog-button:default:hover,
.modal-dialog .modal-dialog-linked-button:default:hover,
.modal-dialog .button:default:hover,
.modal-dialog .modal-dialog-button.default:hover {
  color: #ffffff !important;
  background-color: st-lighten(-st-accent-color, 8%) !important;
  border: none !important;
  box-shadow: none !important;
}

.modal-dialog .modal-dialog-button:active:last-child,
.modal-dialog .modal-dialog-linked-button:active:last-child,
.modal-dialog .button:active:last-child,
.modal-dialog .modal-dialog-button:default:active,
.modal-dialog .modal-dialog-linked-button:default:active,
.modal-dialog .button:default:active,
.modal-dialog .modal-dialog-button.default:active {
  color: #ffffff !important;
  background-color: st-darken(-st-accent-color, 6%) !important;
  border: none !important;
  box-shadow: none !important;
}

.modal-dialog .modal-dialog-button:focus:last-child,
.modal-dialog .modal-dialog-linked-button:focus:last-child,
.modal-dialog .button:focus:last-child,
.modal-dialog .modal-dialog-button:default:focus,
.modal-dialog .modal-dialog-linked-button:default:focus,
.modal-dialog .button:default:focus {
  box-shadow: 0 0 0 2px st-lighten(-st-accent-color, 25%) !important;
  border: none !important;
  color: #ffffff !important;
}

.modal-dialog .modal-dialog-button:insensitive:last-child,
.modal-dialog .modal-dialog-linked-button:insensitive:last-child,
.modal-dialog .button:insensitive:last-child,
.modal-dialog .modal-dialog-button:default:insensitive {
  background-color: st-transparentize(-st-accent-color, 0.7) !important;
  border: none !important;
  box-shadow: none !important;
  color: rgba(255, 255, 255, 0.4) !important;
}
SHELL_MODAL_DARK_FIX

cat <<'SHELL_MODAL_LIGHT_FIX' > /tmp/shell_modal_light_fix.css
/* ==============================================================================
 * Pulsar OS - Clean macOS Rounded Capsule Modal Buttons (Light)
 * ============================================================================== */

.modal-dialog .modal-dialog-button-box {
  padding-top: 14px !important;
  spacing: 12px !important;
}

/* 1. Base / Secondary / Cancel Button */
.modal-dialog .modal-dialog-button,
.modal-dialog .modal-dialog-linked-button,
.modal-dialog .button,
.modal-dialog-button {
  min-height: 38px !important;
  padding: 0 22px !important;
  margin: 0 !important;
  border-radius: 9999px !important;
  font-size: 13px !important;
  font-weight: 600 !important;
  border: none !important;
  box-shadow: none !important;
  text-shadow: none !important;
  icon-shadow: none !important;
  background-color: rgba(0, 0, 0, 0.08) !important;
  color: #1d1d1f !important;
  transition-duration: 150ms !important;
}

.modal-dialog .modal-dialog-button:hover,
.modal-dialog .modal-dialog-linked-button:hover,
.modal-dialog .button:hover,
.modal-dialog-button:hover {
  background-color: rgba(0, 0, 0, 0.14) !important;
  border: none !important;
  box-shadow: none !important;
  color: #000000 !important;
}

.modal-dialog .modal-dialog-button:active,
.modal-dialog .modal-dialog-linked-button:active,
.modal-dialog .button:active,
.modal-dialog-button:active {
  background-color: rgba(0, 0, 0, 0.20) !important;
  border: none !important;
  box-shadow: none !important;
  color: #000000 !important;
}

.modal-dialog .modal-dialog-button:focus,
.modal-dialog .modal-dialog-linked-button:focus,
.modal-dialog .button:focus,
.modal-dialog-button:focus {
  background-color: rgba(0, 0, 0, 0.12) !important;
  box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.2) !important;
  border: none !important;
  color: #000000 !important;
}

.modal-dialog .modal-dialog-button:insensitive,
.modal-dialog .modal-dialog-linked-button:insensitive,
.modal-dialog .button:insensitive,
.modal-dialog-button:insensitive {
  background-color: rgba(0, 0, 0, 0.04) !important;
  border: none !important;
  box-shadow: none !important;
  color: rgba(0, 0, 0, 0.3) !important;
}

/* 2. Primary / Default Button */
.modal-dialog .modal-dialog-button:last-child,
.modal-dialog .modal-dialog-linked-button:last-child,
.modal-dialog .button:last-child,
.modal-dialog .modal-dialog-button:default,
.modal-dialog .modal-dialog-linked-button:default,
.modal-dialog .button:default,
.modal-dialog .modal-dialog-button.default,
.modal-dialog .modal-dialog-button:checked,
.modal-dialog .modal-dialog-linked-button:checked {
  color: #ffffff !important;
  background-color: -st-accent-color !important;
  border: none !important;
  box-shadow: none !important;
  background-gradient-direction: none !important;
}

.modal-dialog .modal-dialog-button:hover:last-child,
.modal-dialog .modal-dialog-linked-button:hover:last-child,
.modal-dialog .button:hover:last-child,
.modal-dialog .modal-dialog-button:default:hover,
.modal-dialog .modal-dialog-linked-button:default:hover,
.modal-dialog .button:default:hover,
.modal-dialog .modal-dialog-button.default:hover {
  color: #ffffff !important;
  background-color: st-lighten(-st-accent-color, 8%) !important;
  border: none !important;
  box-shadow: none !important;
}

.modal-dialog .modal-dialog-button:active:last-child,
.modal-dialog .modal-dialog-linked-button:active:last-child,
.modal-dialog .button:active:last-child,
.modal-dialog .modal-dialog-button:default:active,
.modal-dialog .modal-dialog-linked-button:default:active,
.modal-dialog .button:default:active,
.modal-dialog .modal-dialog-button.default:active {
  color: #ffffff !important;
  background-color: st-darken(-st-accent-color, 6%) !important;
  border: none !important;
  box-shadow: none !important;
}

.modal-dialog .modal-dialog-button:focus:last-child,
.modal-dialog .modal-dialog-linked-button:focus:last-child,
.modal-dialog .button:focus:last-child,
.modal-dialog .modal-dialog-button:default:focus,
.modal-dialog .modal-dialog-linked-button:default:focus,
.modal-dialog .button:default:focus {
  box-shadow: 0 0 0 2px st-lighten(-st-accent-color, 25%) !important;
  border: none !important;
  color: #ffffff !important;
}
SHELL_MODAL_LIGHT_FIX

find "$STAGE_DIR" -path "*/MacTahoe-Dark*/gnome-shell/gnome-shell.css" -exec sh -c 'cat /tmp/shell_modal_dark_fix.css >> "$1"' _ {} \; 2>/dev/null || true
find "$STAGE_DIR" -path "*/MacTahoe-Light*/gnome-shell/gnome-shell.css" -exec sh -c 'cat /tmp/shell_modal_light_fix.css >> "$1"' _ {} \; 2>/dev/null || true
rm -f /tmp/shell_modal_dark_fix.css /tmp/shell_modal_light_fix.css

# Symlink MacTahoe to MacTahoe-Light for backward compatibility
if [ -d "$STAGE_DIR/usr/share/themes/MacTahoe-Light" ] && [ ! -d "$STAGE_DIR/usr/share/themes/MacTahoe" ]; then
    ln -sf MacTahoe-Light "$STAGE_DIR/usr/share/themes/MacTahoe"
fi




# Ejecutar instalador de iconos
echo "Instalando iconos en staging..."
cd "$TEMP_BUILD/icons"
sed -i 's/gtk-update-icon-cache/true/g' install.sh || true
./install.sh -t blue -d "$STAGE_DIR/usr/share/icons"

# 2.1 Mapear el icono de AppInstall a la App Store y Seafari a Safari
echo "Mapeando iconos de AppInstall y Seafari..."
for theme_dir in "$STAGE_DIR/usr/share/icons/"MacTahoe-blue*; do
    apps_dir="$theme_dir/apps/scalable"
    if [ -d "$apps_dir" ]; then
        ln -sf software-store.svg "$apps_dir/es.inled.AppInstall.svg"
        ln -sf safari.svg "$apps_dir/seafari.svg"
        ln -sf safari.svg "$apps_dir/io.github.seafari.svg"
    fi
done

# 3. Limpiar compilación temporal
rm -rf "$TEMP_BUILD"
echo "✅ Temas e iconos posicionados correctamente."
