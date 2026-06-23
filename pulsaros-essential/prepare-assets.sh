#!/bin/bash
# ==============================================================================
# Pulsar OS - Essential Package Asset Preparer
# ==============================================================================
# Descarga y compila Fildem HUD, descarga el fondo de pantalla oficial
# y estructura las reglas de sudoers y polkit sin clave.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
TEMP_BUILD="/tmp/pulsaros-essential-build"
FILDEM_REPO="https://github.com/InledGroup/Fildem"
WALLPAPER_URL="https://raw.githubusercontent.com/Inled-Pulsar-OS/pulsar-art/refs/heads/main/pulsar-os-tahoe.png"

echo "🎨 Descargando y compilando Fildem HUD (depth=1)..."
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Clonar con depth=1
git clone --depth=1 "$FILDEM_REPO" "$TEMP_BUILD/fildem"

# Parchear fallos de wayland en Fildem
sed -i "s/return 'wayland' in (disp or type)/return 'wayland' in (disp or type or '')/" "$TEMP_BUILD/fildem/fildem/utils/wayland.py"
sed -i "s/os.environ\['XDG_SESSION_TYPE'\]/os.environ.get('XDG_SESSION_TYPE', '')/g" "$TEMP_BUILD/fildem/fildem/run.py"

# Compilar e instalar fildem HUD dentro de staging de debian
cd "$TEMP_BUILD/fildem"
python3 setup.py install --root="$STAGE_DIR" --prefix=/usr --install-layout=deb

# Copiar configuración de autostart del HUD
mkdir -p "$STAGE_DIR/etc/xdg/autostart"
cp "$TEMP_BUILD/fildem/fildem-hud.desktop" "$STAGE_DIR/etc/xdg/autostart/"

# Copiar servicio systemd del HUD
mkdir -p "$STAGE_DIR/usr/lib/systemd/user"
cp "$TEMP_BUILD/fildem/fildem.service" "$STAGE_DIR/usr/lib/systemd/user/"

# --- COPY FILDEM SHELL EXTENSION / COPIAR EXTENSIÓN DE SHELL DE FILDEM ---
# Copy the local Gnome Shell extension of Fildem into staging and apply proper permissions.
# Copiar la extensión local de Gnome Shell de Fildem a staging y aplicar permisos correctos.
echo "🧩 [ES] Copiando la extensión de GNOME Shell de Fildem..."
echo "🧩 [EN] Copying Fildem Gnome Shell extension..."
mkdir -p "$STAGE_DIR/usr/share/gnome-shell/extensions/fildem@inled.es"
cp -r "$TEMP_BUILD/fildem/fildem@inled.es/"* "$STAGE_DIR/usr/share/gnome-shell/extensions/fildem@inled.es/"

# Overwrite extension.js with our patched static fallback version
# Sobrescribir extension.js con nuestra versión con fallback estático integrado
echo "🩹 [ES] Aplicando parche de fallback para la extensión de Fildem..."
echo "🩹 [EN] Applying fallback patch for Fildem extension..."
PKG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$PKG_DIR/fildem-patch/extension.js" "$STAGE_DIR/usr/share/gnome-shell/extensions/fildem@inled.es/extension.js"

# Ensure correct permissions for Fildem extension files
# Asegurar permisos correctos para los archivos de la extensión Fildem
find "$STAGE_DIR/usr/share/gnome-shell/extensions/fildem@inled.es" -type d -exec chmod 755 {} \; 2>/dev/null || true
find "$STAGE_DIR/usr/share/gnome-shell/extensions/fildem@inled.es" -type f -exec chmod 644 {} \; 2>/dev/null || true

# Copy Fildem XML Schema to the global schemas directory for gsettings compatibility
# Copiar esquema XML de Fildem al directorio global de esquemas para compatibilidad con gsettings
mkdir -p "$STAGE_DIR/usr/share/glib-2.0/schemas"
cp "$TEMP_BUILD/fildem/fildem@inled.es/schemas/"*.gschema.xml "$STAGE_DIR/usr/share/glib-2.0/schemas/" 2>/dev/null || true
find "$STAGE_DIR/usr/share/glib-2.0/schemas" -type f -exec chmod 644 {} \; 2>/dev/null || true

# Configurar GTK Modules globalmente para Fildem en staging
mkdir -p "$STAGE_DIR/etc/gtk-3.0"
cat <<EOF > "$STAGE_DIR/etc/gtk-3.0/settings.ini"
[Settings]
gtk-modules=appmenu-gtk-module
EOF

mkdir -p "$STAGE_DIR/etc/gtk-2.0"
echo 'gtk-modules="appmenu-gtk-module"' > "$STAGE_DIR/etc/gtk-2.0/gtkrc"

# Replicar configs de GTK en la plantilla de nuevos usuarios (etc/skel)
mkdir -p "$STAGE_DIR/etc/skel/.config/gtk-3.0"
cp "$STAGE_DIR/etc/gtk-3.0/settings.ini" "$STAGE_DIR/etc/skel/.config/gtk-3.0/settings.ini"
cp "$STAGE_DIR/etc/gtk-2.0/gtkrc" "$STAGE_DIR/etc/skel/.gtkrc-2.0"

# ==============================================================================
# REGLAS DE SEGURIDAD (sudo y polkit sin password)
# ==============================================================================
echo "🔐 Estructurando reglas de Sudoers y Polkit sin password..."
mkdir -p "$STAGE_DIR/etc/sudoers.d"
echo "jaime ALL=(ALL) NOPASSWD:ALL" > "$STAGE_DIR/etc/sudoers.d/jaime"
echo "live ALL=(ALL) NOPASSWD:ALL" > "$STAGE_DIR/etc/sudoers.d/live"

mkdir -p "$STAGE_DIR/etc/polkit-1/rules.d"
cat <<EOF > "$STAGE_DIR/etc/polkit-1/rules.d/90-pulsaros-nopasswd.rules"
polkit.addRule(function(action, subject) {
    if (action.id == "org.freedesktop.policykit.exec" &&
        subject.isInGroup("sudo")) {
        return polkit.Result.YES;
    }
});
EOF

# ==============================================================================
# CONFIGURACIÓN DE RED BÁSICA
# ==============================================================================
mkdir -p "$STAGE_DIR/etc/network"
cat <<EOF > "$STAGE_DIR/etc/network/interfaces"
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
EOF

# ==============================================================================
# FONDO DE PANTALLA OFICIAL
# ==============================================================================
echo "📥 Descargando fondo de pantalla oficial de Pulsar OS..."
mkdir -p "$STAGE_DIR/usr/share/backgrounds"
wget -q -O "$STAGE_DIR/usr/share/backgrounds/pulsar-os-tahoe.png" "$WALLPAPER_URL"

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Configuración y Fildem compilado en staging."
