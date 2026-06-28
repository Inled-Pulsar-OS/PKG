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

# ==============================================================================
# SERVICIO DE COMPATIBILIDAD CON DOCKER Y CONTAINERD EN 9PFS (QEMU TEST)
# ==============================================================================
# English: Create a dedicated systemd service to mount tmpfs on /var/lib/docker
# and /var/lib/containerd if running on a 9pfs root filesystem (QEMU test environment).
# This bypasses docker.service sandboxing and ensures both docker and containerd
# have a compatible backing store for overlays in RAM, without affecting real hardware.
# Español: Crear un servicio de systemd dedicado para montar tmpfs en /var/lib/docker
# y /var/lib/containerd si se ejecuta sobre un sistema de archivos raíz 9pfs (entorno QEMU).
# Esto evita las restricciones de sandboxing de docker.service y asegura que tanto docker
# como containerd tengan un almacenamiento compatible para overlays en RAM, sin afectar hardware real.

echo "🐳 Configurando servicio docker-9pfs-mount para compatibilidad en QEMU..."
mkdir -p "$STAGE_DIR/etc/systemd/system"
cat <<'EOF' > "$STAGE_DIR/etc/systemd/system/docker-9pfs-mount.service"
[Unit]
Description=Mount tmpfs on /var/lib/docker and /var/lib/containerd for 9pfs compatibility
DefaultDependencies=no
After=local-fs.target
Before=docker.service containerd.service

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'if grep -q " 9p " /proc/mounts; then mkdir -p /var/lib/docker /var/lib/containerd && (mountpoint -q /var/lib/docker || mount -t tmpfs -o size=2G tmpfs /var/lib/docker) && (mountpoint -q /var/lib/containerd || mount -t tmpfs -o size=1G tmpfs /var/lib/containerd); fi'
RemainAfterExit=yes

[Install]
RequiredBy=docker.service containerd.service
WantedBy=multi-user.target
EOF

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Configuración y Fildem compilado en staging."

