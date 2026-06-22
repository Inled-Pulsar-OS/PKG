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

# Crear symlinks necesarios de fildem en /usr/bin/ si no se crearon
mkdir -p "$STAGE_DIR/usr/bin"
ln -sf /usr/local/bin/fildem "$STAGE_DIR/usr/bin/fildem" || true
ln -sf /usr/local/bin/fildem-hud "$STAGE_DIR/usr/bin/fildem-hud" || true

# Copiar configuración de autostart del HUD
mkdir -p "$STAGE_DIR/etc/xdg/autostart"
cp "$TEMP_BUILD/fildem/fildem-hud.desktop" "$STAGE_DIR/etc/xdg/autostart/"

# Copiar servicio systemd del HUD
mkdir -p "$STAGE_DIR/usr/lib/systemd/user"
cp "$TEMP_BUILD/fildem/fildem.service" "$STAGE_DIR/usr/lib/systemd/user/"

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
