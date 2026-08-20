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
wget -q --timeout=15 --tries=3 -O "$STAGE_DIR/usr/share/backgrounds/pulsar-os-tahoe.png" "$WALLPAPER_URL"
wget -q --timeout=15 --tries=3 -O "$STAGE_DIR/usr/share/backgrounds/pulsaros-golden-gate-background.png" "https://hosted.inled.es/pulsaros-golden-gate-background.png"
wget -q --timeout=15 --tries=3 -O "$STAGE_DIR/usr/share/backgrounds/pulsaros-golden-gate-bg-oscuro.png" "https://hosted.inled.es/pulsaros-golden-gate-bg-oscuro.png"

# Crear el archivo XML de propiedades para registrar los fondos en GNOME Settings
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

# ==============================================================================
# GESTOR DE HIBERNACIÓN SEGURO CON PROGRESO Y REANUDACIÓN LIMPIA (PULSAR OS HIBERNATE)
# ==============================================================================
echo "❄️ Configurando gestor de hibernación con progreso real y apagado seguro..."
mkdir -p "$STAGE_DIR/usr/bin"
cat <<'SCRIPT_EOF' > "$STAGE_DIR/usr/bin/pulsaros-hibernate"
#!/bin/bash
# ==============================================================================
# Pulsar OS - Hibernate Manager with Real Progress & Guaranteed Resume
# ==============================================================================
set -e

MODE="${1:-shutdown}"

if [ "$EUID" -ne 0 ]; then
    exec pkexec "$0" "$@"
fi

# 1. Configurar modo de apagado o reinicio según argumento
case "$MODE" in
    reboot)
        echo "reboot" > /sys/power/disk 2>/dev/null || true
        ;;
    suspend)
        echo "suspend" > /sys/power/disk 2>/dev/null || true
        ;;
    *)
        echo "shutdown" > /sys/power/disk 2>/dev/null || true
        ;;
esac

# 2. Asegurar sincronización previa de discos
sync

# 3. Iniciar splash gráfico de Plymouth si está disponible
HAS_PLYMOUTH=false
if command -v plymouth >/dev/null 2>&1; then
    if plymouth --ping 2>/dev/null; then
        HAS_PLYMOUTH=true
    else
        plymouth --show-splash 2>/dev/null && HAS_PLYMOUTH=true || true
    fi
fi

if [ "$HAS_PLYMOUTH" = true ]; then
    plymouth message --text="Pulsar OS: Guardando sesión en disco..." 2>/dev/null || true
fi

# 4. Ejecutar la hibernación del kernel
# El kernel congelará los procesos, volcará la memoria a la swap y apagará/reiniciará el PC.
echo disk > /sys/power/state || true

# ==============================================================================
# RETORNO TRAS DESPERTAR / REANUDACIÓN EXITOSA (RESUME)
# ==============================================================================
# Cuando el kernel reanuda la memoria RAM, la ejecución continúa AQUÍ:
if [ "$HAS_PLYMOUTH" = true ]; then
    plymouth message --text="Sesión de Pulsar OS restaurada con éxito" 2>/dev/null || true
    sleep 0.5
    plymouth --quit 2>/dev/null || true
fi

sync
exit 0
SCRIPT_EOF

chmod 755 "$STAGE_DIR/usr/bin/pulsaros-hibernate"

# Configurar modprobe para preservación de VRAM en NVIDIA
mkdir -p "$STAGE_DIR/etc/modprobe.d"
cat <<'CONF_EOF' > "$STAGE_DIR/etc/modprobe.d/pulsaros-nvidia-hibernate.conf"
# Pulsar OS - Preservar memoria VRAM en hibernación / suspensión
options nvidia NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/var/tmp NVreg_DynamicPowerManagement=0x02
CONF_EOF

# Configurar systemd sleep para modo shutdown directo
mkdir -p "$STAGE_DIR/etc/systemd/sleep.conf.d"
cat <<'CONF_EOF' > "$STAGE_DIR/etc/systemd/sleep.conf.d/pulsaros-hibernate.conf"
[Sleep]
HibernateMode=shutdown
CONF_EOF

# Override de systemd-hibernate para invocar el gestor de Pulsar OS con splash
mkdir -p "$STAGE_DIR/etc/systemd/system/systemd-hibernate.service.d"
cat <<'CONF_EOF' > "$STAGE_DIR/etc/systemd/system/systemd-hibernate.service.d/override.conf"
[Service]
ExecStart=
ExecStart=/usr/bin/pulsaros-hibernate shutdown
CONF_EOF

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Configuración de Pulsar OS Essential y gestor de hibernación preparados en staging."
