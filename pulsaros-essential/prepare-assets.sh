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
# GESTOR DE HIBERNACIÓN SEGURO CON BARRA DE PROGRESO REAL (PULSAR OS HIBERNATE)
# ==============================================================================
echo "❄️ Configurando gestor de hibernación con progreso real y apagado seguro..."
mkdir -p "$STAGE_DIR/usr/bin"
cat <<'SCRIPT_EOF' > "$STAGE_DIR/usr/bin/pulsaros-hibernate"
#!/bin/bash
# ==============================================================================
# Pulsar OS - Hibernate Manager with Real Progress & Guaranteed Poweroff
# ==============================================================================
set -e

if [ "$EUID" -ne 0 ]; then
    exec pkexec "$0" "$@"
fi

# 1. Asegurar sincronización previa de discos
sync

# 2. Configurar modo de apagado S5 directo (evitar bugs de ACPI S4 de placa)
echo "shutdown" > /sys/power/disk 2>/dev/null || true

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
    plymouth message --text="Pulsar OS: Preparando memoria para hibernación..." 2>/dev/null || true
fi

# 4. Calcular memoria RAM usada a guardar
TOTAL_RAM_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
AVAIL_RAM_KB=$(grep MemAvailable /proc/meminfo | awk '{print $2}')
USED_RAM_KB=$((TOTAL_RAM_KB - AVAIL_RAM_KB))
USED_RAM_MB=$((USED_RAM_KB / 1024))

# Monitor de progreso real en segundo plano
PROGRESS_PID=""
if [ "$HAS_PLYMOUTH" = true ] && [ "$USED_RAM_MB" -gt 0 ]; then
    (
        INITIAL_SWAP_USED=$(grep -v "Filename" /proc/swaps 2>/dev/null | awk '{sum+=$4} END {print sum+0}')
        START_TIME=$(date +%s%N)
        while true; do
            CURRENT_SWAP_USED=$(grep -v "Filename" /proc/swaps 2>/dev/null | awk '{sum+=$4} END {print sum+0}')
            DELTA_KB=$((CURRENT_SWAP_USED - INITIAL_SWAP_USED))
            if [ "$DELTA_KB" -lt 0 ]; then DELTA_KB=0; fi
            DELTA_MB=$((DELTA_KB / 1024))
            
            PCT=$((DELTA_MB * 100 / USED_RAM_MB))
            if [ "$PCT" -gt 100 ]; then PCT=100; fi
            
            NOW=$(date +%s%N)
            ELAPSED_MS=$(( (NOW - START_TIME) / 1000000 ))
            if [ "$ELAPSED_MS" -gt 200 ]; then
                SPEED_MBPS=$(( DELTA_MB * 1000 / ELAPSED_MS ))
            else
                SPEED_MBPS=0
            fi
            
            plymouth message --text="Guardando sesión en disco: ${PCT}% (${DELTA_MB}/${USED_RAM_MB} MB - ${SPEED_MBPS} MB/s)" 2>/dev/null || true
            sleep 0.15
        done
    ) &
    PROGRESS_PID=$!
fi

# 5. Volcar imagen y apagar
if [ "$HAS_PLYMOUTH" = true ]; then
    plymouth message --text="Guardando memoria y apagando equipo..." 2>/dev/null || true
fi

# Ejecutar la hibernación
echo disk > /sys/power/state || true

# Si el proceso sigue vivo tras el intento de suspender
if [ -n "$PROGRESS_PID" ]; then
    kill "$PROGRESS_PID" 2>/dev/null || true
fi

# 6. Fallback de emergencia a lo bruto si el hardware no corta la energía
echo "⚡ Ejecutando apagado forzado de hardware tras volcado de memoria..."
echo 1 > /proc/sys/kernel/sysrq 2>/dev/null || true
sync
echo o > /proc/sysrq-trigger 2>/dev/null || true
systemctl poweroff -f -f 2>/dev/null || true
SCRIPT_EOF

chmod 755 "$STAGE_DIR/usr/bin/pulsaros-hibernate"

# Configurar modprobe para preservación de VRAM en NVIDIA
mkdir -p "$STAGE_DIR/etc/modprobe.d"
cat <<'CONF_EOF' > "$STAGE_DIR/etc/modprobe.d/pulsaros-nvidia-hibernate.conf"
# Pulsar OS - Preservar memoria VRAM en hibernación / suspensión
options nvidia NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/var/tmp
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
ExecStart=/usr/bin/pulsaros-hibernate
CONF_EOF

# Limpieza
rm -rf "$TEMP_BUILD"
echo "✅ Configuración de Pulsar OS Essential y gestor de hibernación preparados en staging."
