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
# REGLAS DE SEGURIDAD (sudo y polkit sin password para entorno live)
# ==============================================================================

echo "🔐 Estructurando reglas de Sudoers y Polkit sin password para usuario live..."
mkdir -p "$STAGE_DIR/etc/sudoers.d"
echo "live ALL=(ALL) NOPASSWD:ALL" > "$STAGE_DIR/etc/sudoers.d/live"

mkdir -p "$STAGE_DIR/etc/polkit-1/rules.d"
cat <<EOF > "$STAGE_DIR/etc/polkit-1/rules.d/90-pulsaros-nopasswd.rules"
polkit.addRule(function(action, subject) {
    if (action.id == "org.freedesktop.policykit.exec" &&
        (subject.isInGroup("wheel") || subject.isInGroup("sudo") || subject.user == "live")) {
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
# FONDO DE PANTALLA OFICIAL Y COLECCIÓN COMPLETA DE FONDOS APPLE MACOS
# ==============================================================================
echo "📥 Preparando fondos de pantalla oficiales y colección macOS..."
mkdir -p "$STAGE_DIR/usr/share/backgrounds/macos" "$STAGE_DIR/usr/share/gnome-background-properties"

# Copiar desde el sistema anfitrión si están disponibles, o descargar si faltan
if [ -d "/usr/share/backgrounds/macos" ]; then
    cp -rf /usr/share/backgrounds/macos/* "$STAGE_DIR/usr/share/backgrounds/macos/" 2>/dev/null || true
fi

for bg in pulsar-os-tahoe.png pulsaros-golden-gate-background.png pulsaros-golden-gate-bg-oscuro.png; do
    if [ -f "/usr/share/backgrounds/$bg" ]; then
        cp -f "/usr/share/backgrounds/$bg" "$STAGE_DIR/usr/share/backgrounds/"
    fi
done

[ ! -f "$STAGE_DIR/usr/share/backgrounds/pulsar-os-tahoe.png" ] && wget -q --timeout=15 --tries=3 -O "$STAGE_DIR/usr/share/backgrounds/pulsar-os-tahoe.png" "$WALLPAPER_URL" || true
[ ! -f "$STAGE_DIR/usr/share/backgrounds/pulsaros-golden-gate-background.png" ] && wget -q --timeout=15 --tries=3 -O "$STAGE_DIR/usr/share/backgrounds/pulsaros-golden-gate-background.png" "https://hosted.inled.es/pulsaros-golden-gate-background.png" || true
[ ! -f "$STAGE_DIR/usr/share/backgrounds/pulsaros-golden-gate-bg-oscuro.png" ] && wget -q --timeout=15 --tries=3 -O "$STAGE_DIR/usr/share/backgrounds/pulsaros-golden-gate-bg-oscuro.png" "https://hosted.inled.es/pulsaros-golden-gate-bg-oscuro.png" || true

# Copiar archivos de definición de fondos para GNOME Control Center (Ajustes)
if [ -f "/usr/share/gnome-background-properties/pulsar-macos-wallpapers.xml" ]; then
    cp -f "/usr/share/gnome-background-properties/pulsar-macos-wallpapers.xml" "$STAGE_DIR/usr/share/gnome-background-properties/"
fi
if [ -f "/usr/share/gnome-background-properties/pulsar-backgrounds.xml" ]; then
    cp -f "/usr/share/gnome-background-properties/pulsar-backgrounds.xml" "$STAGE_DIR/usr/share/gnome-background-properties/"
fi

# ==============================================================================
# SERVICIO DE COMPATIBILIDAD CON DOCKER Y CONTAINERD EN 9PFS (QEMU TEST)
# ==============================================================================
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

# Configurar Seafari como el navegador web predeterminado en mimeapps.list
mkdir -p "$STAGE_DIR/etc/xdg"
cat <<EOF > "$STAGE_DIR/etc/xdg/mimeapps.list"
[Default Applications]
text/html=seafari.desktop
x-scheme-handler/http=seafari.desktop
x-scheme-handler/https=seafari.desktop
x-scheme-handler/about=seafari.desktop
x-scheme-handler/unknown=seafari.desktop
application/vnd.debian.binary-package=es.inled.AppInstall.desktop
application/x-debian-package=es.inled.AppInstall.desktop
application/x-zstd=es.inled.AppInstall.desktop
EOF

# Configurar modprobe para preservación de VRAM en NVIDIA
mkdir -p "$STAGE_DIR/etc/modprobe.d"
cat <<'CONF_EOF' > "$STAGE_DIR/etc/modprobe.d/pulsaros-nvidia-hibernate.conf"
# Pulsar OS - Preservar memoria VRAM en hibernación / suspensión
options nvidia NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/var/tmp NVreg_DynamicPowerManagement=0x02
CONF_EOF

# Configurar systemd sleep para modo shutdown directo (ACPI S5)
mkdir -p "$STAGE_DIR/etc/systemd/sleep.conf.d"
cat <<'CONF_EOF' > "$STAGE_DIR/etc/systemd/sleep.conf.d/pulsaros-hibernate.conf"
[Sleep]
HibernateMode=shutdown
CONF_EOF

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Configuración de Pulsar OS Essential preparada en staging."
