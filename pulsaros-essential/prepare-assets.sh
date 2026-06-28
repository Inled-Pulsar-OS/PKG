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

# ==============================================================================
# FONDO DE PANTALLA OFICIAL Y RED
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
# FONDO DE PANTALLA OFICIAL Y OPCIONES ADICIONALES
# ==============================================================================
echo "📥 Descargando fondo de pantalla oficial y opciones adicionales..."
mkdir -p "$STAGE_DIR/usr/share/backgrounds"
wget -q -O "$STAGE_DIR/usr/share/backgrounds/pulsar-os-tahoe.png" "$WALLPAPER_URL"
wget -q -O "$STAGE_DIR/usr/share/backgrounds/pulsaros-golden-gate-background.png" "https://hosted.inled.es/pulsaros-golden-gate-background.png"
wget -q -O "$STAGE_DIR/usr/share/backgrounds/pulsaros-golden-gate-bg-oscuro.png" "https://hosted.inled.es/pulsaros-golden-gate-bg-oscuro.png"

# Crear el archivo XML de propiedades para registrar los fondos en GNOME Settings
# Create the properties XML file to register wallpapers in GNOME Settings
mkdir -p "$STAGE_DIR/usr/share/gnome-background-properties"
cat <<EOF > "$STAGE_DIR/usr/share/gnome-background-properties/pulsar-backgrounds.xml"
<?xml version="1.0"?>
<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">
<wallpapers>
  <wallpaper deleted="false">
    <name>Pulsar OS Tahoe</name>
    <filename>/usr/share/backgrounds/pulsar-os-tahoe.png</filename>
    <options>zoom</options>
    <shade_type>solid</shade_type>
    <pcolor>#000000</pcolor>
    <scolor>#000000</scolor>
  </wallpaper>
  <wallpaper deleted="false">
    <name>Pulsar OS Golden Gate</name>
    <filename>/usr/share/backgrounds/pulsaros-golden-gate-background.png</filename>
    <filename-dark>/usr/share/backgrounds/pulsaros-golden-gate-bg-oscuro.png</filename-dark>
    <options>zoom</options>
    <shade_type>solid</shade_type>
    <pcolor>#000000</pcolor>
    <scolor>#000000</scolor>
  </wallpaper>
</wallpapers>
EOF

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Configuración y Fildem compilado en staging."
