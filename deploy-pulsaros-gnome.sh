#!/usr/bin/env bash
# ==============================================================================
# Pulsar OS - Deploy pulsaros-gnome (Debian & Arch) to apt.inled.es
# ==============================================================================
# Builds and deploys pulsaros-gnome for both Debian (.deb) and Arch (.pkg.tar.zst)
# to the Inled repository at apt.inled.es.
#
# Usage:
#   ./deploy-pulsaros-gnome.sh [options]
#
# Options:
#   --deploy, -d          Build and deploy both Debian and Arch packages to apt.inled.es
#   --upload, -u          Build and upload packages to GitHub release without triggering dispatch
#   --onlyupload          Upload already built packages without rebuilding
#   --branch <name>       Target branch (stable, forky, rolling). Default: stable
#   --deb-only            Only build and deploy the Debian package
#   --arch-only           Only build and deploy the Arch package
#   -h, --help            Show this help message
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BRANCH="stable"
ACTION_FLAG="--upload"
DEPLOY_REQUESTED=false
TARGET_DISTRO="both" # "both", "deb", "arch"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --deploy|-d)
            ACTION_FLAG="--upload"
            DEPLOY_REQUESTED=true
            shift
            ;;
        --upload|-u)
            ACTION_FLAG="--upload"
            DEPLOY_REQUESTED=false
            shift
            ;;
        --onlyupload)
            ACTION_FLAG="--onlyupload"
            DEPLOY_REQUESTED=false
            shift
            ;;
        --branch)
            BRANCH="$2"
            shift 2
            ;;
        --deb-only)
            TARGET_DISTRO="deb"
            shift
            ;;
        --arch-only)
            TARGET_DISTRO="arch"
            shift
            ;;
        -h|--help)
            echo "Uso / Usage: $0 [--deploy] [--upload] [--onlyupload] [--branch <stable|forky|rolling>] [--deb-only|--arch-only]"
            exit 0
            ;;
        *)
            echo "❌ Opción desconocida / Unknown option: $1"
            echo "Uso / Usage: $0 [--deploy] [--upload] [--onlyupload] [--branch <stable|forky|rolling>] [--deb-only|--arch-only]"
            exit 1
            ;;
    esac
done

echo "=============================================================================="
echo "🚀 Pulsar OS - Despliegue de pulsaros-gnome (Debian + Arch)"
echo "   Rama / Branch: $BRANCH"
echo "   Acción / Action: $ACTION_FLAG"
echo "   Objetivo / Target: $TARGET_DISTRO"
echo "=============================================================================="

# 1. Comprobación de herramientas necesarias
echo "🔍 Verificando herramientas de compilación..."
MISSING_TOOLS=()
for tool in make sassc msgfmt glib-compile-schemas jq curl git; do
    if ! command -v "$tool" &>/dev/null; then
        MISSING_TOOLS+=("$tool")
    fi
done

if [ "$TARGET_DISTRO" != "arch" ]; then
    if ! command -v dpkg-deb &>/dev/null; then
        MISSING_TOOLS+=("dpkg-deb")
    fi
fi

if [ "$TARGET_DISTRO" != "deb" ]; then
    if ! command -v makepkg &>/dev/null; then
        MISSING_TOOLS+=("makepkg")
    fi
fi

if [ ${#MISSING_TOOLS[@]} -gt 0 ]; then
    echo "❌ Faltan herramientas necesarias: ${MISSING_TOOLS[*]}"
    echo "   Por favor instálalas antes de continuar."
    exit 1
fi
echo "✅ Todas las herramientas necesarias están disponibles."

# 2. Limpieza preventiva de dash-to-dock si tuviera archivos con permisos incorrectos
DASH_TO_DOCK_LOCAL="$SCRIPT_DIR/../dash-to-dock"
if [ -d "$DASH_TO_DOCK_LOCAL/_build" ]; then
    if [ ! -w "$DASH_TO_DOCK_LOCAL/_build" ]; then
        echo "🧹 Limpiando directorio _build con permisos restrictivos en dash-to-dock..."
        sudo rm -rf "$DASH_TO_DOCK_LOCAL/_build" 2>/dev/null || rm -rf "$DASH_TO_DOCK_LOCAL/_build" || true
    fi
fi

# 3. Compilación y subida Debian
if [ "$TARGET_DISTRO" == "both" ] || [ "$TARGET_DISTRO" == "deb" ]; then
    echo ""
    echo "📦 =========================================================="
    echo "📦 [1/2] Compilando y procesando Debian (.deb)..."
    echo "📦 =========================================================="
    
    DEB_ARGS=("pulsaros-gnome" "$ACTION_FLAG" "--branch" "$BRANCH")
    if [ "$DEPLOY_REQUESTED" = true ]; then
        DEB_ARGS=("pulsaros-gnome" "--upload" "--branch" "$BRANCH")
    fi

    (cd "$SCRIPT_DIR" && ./package-and-deploy.sh "${DEB_ARGS[@]}")
    echo "✅ Paquete Debian de pulsaros-gnome procesado con éxito."
fi

# 4. Compilación y subida Arch Linux
if [ "$TARGET_DISTRO" == "both" ] || [ "$TARGET_DISTRO" == "arch" ]; then
    echo ""
    echo "📦 =========================================================="
    echo "📦 [2/2] Compilando y procesando Arch Linux (.pkg.tar.zst)..."
    echo "📦 =========================================================="
    
    ARCH_ARGS=("pulsaros-gnome" "$ACTION_FLAG" "--branch" "$BRANCH")
    if [ "$DEPLOY_REQUESTED" = true ]; then
        ARCH_ARGS=("pulsaros-gnome" "--upload" "--branch" "$BRANCH")
    fi

    (cd "$SCRIPT_DIR/arch" && ./package-and-deploy.sh "${ARCH_ARGS[@]}")
    echo "✅ Paquete Arch Linux de pulsaros-gnome procesado con éxito."
fi

echo ""
echo "=============================================================================="
echo "🎉 ¡Despliegue de pulsaros-gnome completado con éxito para $TARGET_DISTRO!"
echo "   Los paquetes han sido procesados y subidos al repositorio de apt.inled.es."
echo "=============================================================================="
