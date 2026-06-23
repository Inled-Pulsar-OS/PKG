#!/bin/bash
# ==============================================================================
# Pulsar OS - Gnome Shell Extensions Asset Preparer
# ==============================================================================
# Descarga e instala localmente las extensiones GNOME Shell seleccionadas
# dentro de la estructura temporal del paquete pulsaros-gnome.
# Evita descargas durante el bootstrap y estabiliza las dependencias.
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"

# --- DYNAMIC GNOME VERSION DETECTION / DETECCIÓN DINÁMICA DE LA VERSIÓN DE GNOME ---
# We try to detect the installed GNOME Shell version to ensure we download compatible extension zips.
# Intentamos detectar la versión instalada de GNOME Shell para descargar zips de extensión compatibles.
GNOME_VER=""

# 1. Search in target/base chroot dpkg database (no root permissions required)
# 1. Buscar en la base de datos dpkg del chroot target/base (no requiere permisos de root)
POSSIBLE_DPKG_STATUS=(
    "$STAGE_DIR/var/lib/dpkg/status"
    "../../ISO/build/rootfs-target/var/lib/dpkg/status"
    "../../ISO/build/rootfs-base/var/lib/dpkg/status"
    "../../build/rootfs-target/var/lib/dpkg/status"
    "../../build/rootfs-base/var/lib/dpkg/status"
)

for status_file in "${POSSIBLE_DPKG_STATUS[@]}"; do
    if [ -f "$status_file" ]; then
        ver_str=$(grep -A 10 "Package: gnome-shell$" "$status_file" | grep "^Version:" | cut -d' ' -f2 || true)
        if [ -n "$ver_str" ]; then
            # Extract major version number (e.g., 48 from 48.7-0+deb13u2)
            # Extraer el número principal de la versión (ej. 48 de 48.7-0+deb13u2)
            GNOME_VER=$(echo "$ver_str" | cut -d'.' -f1 | cut -d'-' -f1)
            echo "🔍 [ES] Detectada versión de GNOME Shell en chroot ($status_file): $GNOME_VER"
            echo "🔍 [EN] Detected GNOME Shell version in chroot ($status_file): $GNOME_VER"
            break
        fi
    fi
done

# 2. Fallback to distribution settings and Madison API if chroot is not found or empty
# 2. Fallback a la configuración de la distribución y Madison API si el chroot no existe o está vacío
if [ -z "$GNOME_VER" ]; then
    DEBIAN_VERSION="trixie" # Default fallback / Fallback por defecto
    
    # Try loading environment variables
    # Intentar cargar variables de entorno
    for env_file in "../../configs/env.sh" "../configs/env.sh" "configs/env.sh"; do
        if [ -f "$env_file" ]; then
            source "$env_file"
            break
        fi
    done
    
    echo "🔍 [ES] No se encontró chroot. Dediciendo según versión de Debian: $DEBIAN_VERSION"
    echo "🔍 [EN] Chroot not found. Inferring version from Debian suite: $DEBIAN_VERSION"
    
    # Optional online query to Debian Madison API (with quick timeout)
    # Consulta opcional en línea a la API Debian Madison (con timeout rápido)
    if command -v curl >/dev/null 2>&1; then
        suite_indicator=""
        case "$DEBIAN_VERSION" in
            trixie) suite_indicator="deb13" ;;
            bookworm) suite_indicator="deb12" ;;
            sid|unstable) suite_indicator="unstable" ;;
        esac
        
        if [ -n "$suite_indicator" ]; then
            madison_ver=$(curl --max-time 3 -s "https://api.ftp-master.debian.org/madison?package=gnome-shell&text=on" | grep "$suite_indicator" | head -n 1 | cut -d'|' -f2 | tr -d '[:space:]' || true)
            if [ -n "$madison_ver" ]; then
                GNOME_VER=$(echo "$madison_ver" | cut -d'.' -f1 | cut -d'-' -f1)
                echo "🌐 [ES] API Debian devolvió versión de GNOME: $GNOME_VER"
                echo "🌐 [EN] Debian API returned GNOME version: $GNOME_VER"
            fi
        fi
    fi
    
    # 3. Final static fallback / Fallback estático final
    if [ -z "$GNOME_VER" ]; then
        case "$DEBIAN_VERSION" in
            bookworm) GNOME_VER="43" ;;
            trixie) GNOME_VER="48" ;;
            *) GNOME_VER="48" ;;
        esac
        echo "🔍 [ES] Usando fallback estático de GNOME para $DEBIAN_VERSION: $GNOME_VER"
        echo "🔍 [EN] Using static GNOME fallback for $DEBIAN_VERSION: $GNOME_VER"
    fi
fi

EGO_EXTENSIONS=(
    "wiggle@mechtifs"
    "search-light@icedman.github.com"
    "kiwimenu@kemma"
    "compiz-alike-magic-lamp-effect@hermes83.github.com"
    "fullscreen-to-empty-workspace2@corgijan.dev"
    "appmenu-is-back@fthx"
    "just-perfection-desktop@just-perfection"
    "notification-position@drugo.dev"
    "wack-lockscreen-clock@rinzler69-wastaken.github.com"
)

echo "🧩 Descargando extensiones de GNOME Shell desde Extensions.gnome.org (EGO)..."

for uuid in "${EGO_EXTENSIONS[@]}"; do
    echo "Descargando: $uuid..."
    
    # --- DOWNLOAD WITH BACKWARD FALLBACK / DESCARGA CON FALLBACK HACIA ATRÁS ---
    # We query EGO starting from the target GNOME version and go backward if not found.
    # Consultamos en EGO empezando por la versión de GNOME objetivo y retrocedemos si no se encuentra.
    download_path="null"
    current_ver=$GNOME_VER
    
    while [ "$download_path" = "null" ] || [ -z "$download_path" ]; do
        info_url="https://extensions.gnome.org/extension-info/?uuid=${uuid}&shell_version=${current_ver}"
        download_path=$(curl -s "$info_url" | jq -r '.download_url')
        
        if [ "$download_path" != "null" ] && [ -n "$download_path" ]; then
            if [ "$current_ver" -ne "$GNOME_VER" ]; then
                echo "⚠️ [ES] Usando fallback compatible con GNOME $current_ver para $uuid"
                echo "⚠️ [EN] Using compatible fallback for GNOME $current_ver for $uuid"
            fi
            break
        fi
        
        # Stop at version 40 to avoid checking infinite/ancient versions
        # Detenerse en la versión 40 para evitar comprobar versiones antiguas/infinitas
        if [ "$current_ver" -le 40 ]; then
            break
        fi
        current_ver=$((current_ver - 1))
    done

    if [ "$download_path" != "null" ] && [ -n "$download_path" ]; then
        full_url="https://extensions.gnome.org${download_path}"
        tmp_zip="/tmp/${uuid}.zip"
        
        if curl -L -s -o "$tmp_zip" "$full_url"; then
            dest_dir="$STAGE_DIR/usr/share/gnome-shell/extensions/${uuid}"
            mkdir -p "$dest_dir"
            unzip -q -o "$tmp_zip" -d "$dest_dir"
            rm "$tmp_zip"
            echo "✅ Extensión $uuid lista."
        fi
    else
        echo "⚠️ [ES] No se encontró ninguna versión compatible en EGO para $uuid"
        echo "⚠️ [EN] No compatible version found on EGO for $uuid"
    fi
done

# Copy all extensions' .gschema.xml files to the global schemas directory so gsettings and dconf can manage them
# Copiar todos los archivos .gschema.xml de las extensiones al directorio global de esquemas para que gsettings y dconf puedan gestionarlos
echo "⚙️ [ES] Copiando esquemas xml de extensiones al directorio global..."
echo "⚙️ [EN] Copying extensions' xml schemas to the global schemas directory..."
mkdir -p "$STAGE_DIR/usr/share/glib-2.0/schemas"
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -name "*.gschema.xml" -exec cp {} "$STAGE_DIR/usr/share/glib-2.0/schemas/" \; 2>/dev/null || true

# Compilar esquemas locales de las extensiones dentro de staging para que estén listos
echo "Compilando esquemas locales de extensiones..."
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -name schemas -type d 2>/dev/null | while read -r schema_path; do
    glib-compile-schemas "$schema_path" || true
done

# Compilar también el directorio global de schemas en staging
echo "Compilando esquemas globales en staging..."
glib-compile-schemas "$STAGE_DIR/usr/share/glib-2.0/schemas" || true

# Ensure correct permissions for all files and folders under extensions and schemas directory (avoiding permission denied in Gnome Shell)
# Asegurar permisos correctos para todos los archivos y carpetas bajo el directorio de extensiones y esquemas (evitando fallos de permisos en Gnome Shell)
echo "⚙️ [ES] Asegurando permisos de lectura y ejecución para las extensiones..."
echo "⚙️ [EN] Ensuring read and execute permissions for the extensions..."
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -type d -exec chmod 755 {} \; 2>/dev/null || true
find "$STAGE_DIR/usr/share/gnome-shell/extensions" -type f -exec chmod 644 {} \; 2>/dev/null || true
find "$STAGE_DIR/usr/share/glib-2.0/schemas" -type f -exec chmod 644 {} \; 2>/dev/null || true

echo "✅ Proceso de extensiones finalizado."
