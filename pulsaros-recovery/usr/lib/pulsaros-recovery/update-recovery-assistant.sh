#!/bin/bash
# ==============================================================================
# Pulsar OS — Sincroniza el Recovery Assistant dentro de la partición PULSAR_RECOVERY
# English: Mounts the PULSAR_RECOVERY partition and installs the current
#          /usr/bin/pulsar-recovery-assistant binary into the Debian-based
#          recovery environment stored there (live/filesystem.squashfs),
#          rebuilding the SquashFS only when the assistant has changed.
# Español: Monta la partición PULSAR_RECOVERY e instala la versión actual de
#          /usr/bin/pulsar-recovery-assistant en el entorno Debian de
#          recuperación que contiene (live/filesystem.squashfs), reconstruyendo
#          el SquashFS solo cuando el asistente haya cambiado.
#
# Se ejecuta automáticamente desde el postinst de pulsaros-recovery cada vez
# que el paquete se instala o actualiza en un sistema instalado.
# ==============================================================================

set -uo pipefail

ASYNC_BIN="/usr/bin/pulsar-recovery-assistant"
MOUNT_BASE="/mnt/pulsaros-recovery-sync"
LOG_FILE="/var/log/pulsaros-recovery-sync.log"
MOUNTED_BASE=""

log() {
    local msg="[pulsaros-recovery] $*"
    echo "$msg"
    if [ -d "/var/log" ]; then
        echo "$(date '+%Y-%m-%d %H:%M:%S') $*" >> "$LOG_FILE" 2>/dev/null || true
    fi
}

cleanup_mount() {
    if [ -n "$MOUNTED_BASE" ] && [ -d "$MOUNTED_BASE" ]; then
        umount "$MOUNTED_BASE" 2>/dev/null || true
        rmdir "$MOUNTED_BASE" 2>/dev/null || true
        MOUNTED_BASE=""
    fi
}
trap cleanup_mount EXIT INT TERM

# ----------------------------------------------------------------------------
# Guardas: no tocar la partición de recuperación mientras se está ejecutando
# desde un entorno live/ISO, o desde un chroot de construcción de imagen.
# ----------------------------------------------------------------------------
if [ -d "/run/live/medium" ] || [ -d "/lib/live/mount" ] || [ -d "/run/archiso/bootmnt" ]; then
    log "Entorno live/ISO detectado; no se sincroniza la partición de recuperación."
    exit 0
fi

is_chroot=false
if command -v systemd-detect-virt >/dev/null 2>&1 && systemd-detect-virt --chroot >/dev/null 2>&1; then
    is_chroot=true
fi
# Fallback manual: dentro de un chroot SIN namespace de PID, /proc/1/root apunta
# a la raíz del host y /proc/self/root a la raíz del chroot; si difieren, chroot.
if [ "$is_chroot" = false ] && [ -r "/proc/1/root" ] && [ -r "/proc/self/root" ]; then
    pid1_root="$(readlink -f "/proc/1/root" 2>/dev/null || true)"
    self_root="$(readlink -f "/proc/self/root" 2>/dev/null || true)"
    if [ -n "$pid1_root" ] && [ -n "$self_root" ] && [ "$pid1_root" != "$self_root" ]; then
        is_chroot=true
    fi
fi
if [ "$is_chroot" = true ]; then
    log "Ejecución dentro de chroot detectada; no se sincroniza la partición de recuperación."
    exit 0
fi

if [ ! -f "$ASYNC_BIN" ] || [ ! -x "$ASYNC_BIN" ]; then
    log "No se encontró $ASYNC_BIN; no hay nada que sincronizar."
    exit 0
fi

for _tool in mount umount blkid unsquashfs mksquashfs; do
    if ! command -v "$_tool" >/dev/null 2>&1; then
        log "Falta la herramienta $_tool; no se puede sincronizar la partición de recuperación."
        exit 0
    fi
done

# ----------------------------------------------------------------------------
# Detección de la partición de recuperación (PULSAR_RECOVERY / PulsarRecovery)
# ----------------------------------------------------------------------------
detect_recovery_devices() {
    local dev
    while read -r dev; do
        [ -n "$dev" ] && printf '%s\n' "$dev"
    done < <(blkid -L PULSAR_RECOVERY 2>/dev/null)
    while read -r dev; do
        [ -n "$dev" ] && printf '%s\n' "$dev"
    done < <(blkid -L PulsarRecovery 2>/dev/null)
    if command -v lsblk >/dev/null 2>&1; then
        while read -r dev; do
            [ -n "$dev" ] && printf '%s\n' "$dev"
        done < <(lsblk -rno PATH,LABEL 2>/dev/null | awk '{l=toupper($2); if (l ~ /RECOVERY/ || l ~ /PULSAR_REC/) print $1}')
    fi
}

# Copia directa del binario a la raíz/boot/recovery de la partición como
# respaldo ejecutable manual, además de la versión dentro del SquashFS.
direct_copies() {
    local mnt="$1" d
    for d in "$mnt" "$mnt/boot" "$mnt/recovery"; do
        [ -d "$d" ] || continue
        cp -f "$ASYNC_BIN" "$d/pulsar-recovery-assistant" 2>/dev/null || true
        chmod 755 "$d/pulsar-recovery-assistant" 2>/dev/null || true
    done
}

# ----------------------------------------------------------------------------
# Sincroniza el nuevo assistant en una única partición de recuperación.
# ----------------------------------------------------------------------------
sync_device() {
    local dev="$1" mnt="" mounted_by_us=false cand="" primary="" _tmpdir="" new_sq="" _nproc
    log "Partición de recuperación detectada: $dev"

    if command -v findmnt >/dev/null 2>&1; then
        mnt="$(findmnt -rno TARGET "$dev" 2>/dev/null | head -n1 || true)"
    fi
    if [ -n "$mnt" ]; then
        log "    ya montada en $mnt; se reutiliza el punto de montaje actual."
    else
        mnt="$MOUNT_BASE"
        mkdir -p "$mnt"
        if ! mount -o rw "$dev" "$mnt"; then
            log "    no se pudo montar la partición; se omite."
            rmdir "$mnt" 2>/dev/null || true
            return 0
        fi
        MOUNTED_BASE="$mnt"
        mounted_by_us=true
    fi

    local sq_paths=()
    for cand in "$mnt/live/filesystem.squashfs" "$mnt/filesystem.squashfs" \
                "$mnt/recovery/filesystem.squashfs" "$mnt/images/pulsaros-base.squashfs"; do
        if [ -f "$cand" ]; then
            [ -z "$primary" ] && primary="$cand"
            sq_paths+=("$cand")
        fi
    done

    if [ -z "$primary" ]; then
        log "    no se encontró filesystem.squashfs; solo se copia el binario."
        direct_copies "$mnt"
        return 0
    fi
    log "    SquashFS de recuperación: $primary"

    _tmpdir="$(mktemp -d /tmp/pulsaros-recovery-sync.XXXXXX)"
    chmod 700 "$_tmpdir"

    # Comprobación rápida: si el binario dentro del SquashFS ya es el actual,
    # no hace falta reconstruir nada.
    if unsquashfs -no-progress -f -d "$_tmpdir/one" "$primary" "usr/bin/pulsar-recovery-assistant" >/dev/null 2>&1; then
        if [ -f "$_tmpdir/one/usr/bin/pulsar-recovery-assistant" ] && \
           cmp -s "$_tmpdir/one/usr/bin/pulsar-recovery-assistant" "$ASYNC_BIN"; then
            log "    el Recovery Assistant de la partición ya está al día; se omite la reconstrucción."
            rm -rf "$_tmpdir"
            direct_copies "$mnt"
            return 0
        fi
    fi

    log "    desempaquetando SquashFS (puede tardar unos minutos)…"
    if ! unsquashfs -no-progress -d "$_tmpdir/full" "$primary" >/dev/null 2>&1; then
        log "    FALLO al desempaquetar $primary; se mantiene la versión actual."
        rm -rf "$_tmpdir"
        return 0
    fi

    mkdir -p "$_tmpdir/full/usr/bin"
    cp -f "$ASYNC_BIN" "$_tmpdir/full/usr/bin/pulsar-recovery-assistant"
    chmod 755 "$_tmpdir/full/usr/bin/pulsar-recovery-assistant"

    _nproc="$(nproc 2>/dev/null || echo 2)"
    _nproc=$(( _nproc > 4 ? 4 : _nproc ))
    [ "$_nproc" -lt 1 ] && _nproc=1

    log "    reempaquetando SquashFS (xz, esto puede tardar)…"
    new_sq="$_tmpdir/filesystem-new.squashfs"
    if ! mksquashfs "$_tmpdir/full" "$new_sq" -comp xz -b 1048576 -Xdict-size 100% \
            -processors "$_nproc" -noappend -no-progress >/dev/null 2>&1; then
        # Compatibilidad con versiones antiguas de squashfs-tools
        mksquashfs "$_tmpdir/full" "$new_sq" -comp xz -b 1048576 -processors "$_nproc" \
            -noappend -no-progress >/dev/null 2>&1 || true
    fi

    if [ ! -s "$new_sq" ]; then
        log "    FALLO al reempaquetar el SquashFS; se mantiene la versión actual."
        rm -rf "$_tmpdir"
        return 0
    fi
    log "    nuevo SquashFS generado ($(du -h "$new_sq" 2>/dev/null | cut -f1))."

    # Backup del anterior SquashFS para poder revertir si fuese necesario
    cp -f "$primary" "$primary.prev" 2>/dev/null || true
    for sq in "${sq_paths[@]}"; do
        if cp -f "$new_sq" "$sq"; then
            log "    SquashFS actualizado: $sq"
        else
            log "    FALLO al escribir $sq; existe copia de seguridad en $primary.prev"
        fi
        chmod 644 "$sq" 2>/dev/null || true
    done
    sync
    log "    Recovery Assistant actualizado dentro de la partición $dev."

    rm -rf "$_tmpdir"
    direct_copies "$mnt"
    return 0
}

# ----------------------------------------------------------------------------
# Flujo principal
# ----------------------------------------------------------------------------
raw_devices="$(detect_recovery_devices)"
if [ -z "$raw_devices" ]; then
    log "No se detectó ninguna partición de recuperación (PULSAR_RECOVERY); no hay nada que hacer."
    exit 0
fi

while read -r dev; do
    [ -n "$dev" ] && sync_device "$dev"
done <<< "$raw_devices"

log "Sincronización del Recovery Assistant finalizada."
exit 0