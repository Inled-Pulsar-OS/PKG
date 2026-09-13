#!/bin/bash
# ==============================================================================
# Pulsar OS - Package Promotion Tool (Unstable -> Stable)
# ==============================================================================
# Rebuilds and deploys the specified package(s) for the official 'stable' branch
# and notifies the central Inled repository.
#
# Usage:
#   ./promote-package.sh <package_name | all> [--local-only]
# ==============================================================================

set -e

PKG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_NAME="$1"
LOCAL_ONLY=false

if [ "$2" == "--local-only" ] || [ "$1" == "--local-only" ]; then
    LOCAL_ONLY=true
    [ "$1" == "--local-only" ] && PACKAGE_NAME="$2"
fi

if [ -z "$PACKAGE_NAME" ]; then
    echo "❌ Error: Especifica el nombre del paquete o 'all'."
    echo "Uso: $0 <package_name | all> [--local-only]"
    exit 1
fi

echo "=============================================================================="
echo "🌟 PROMOCIONANDO PAQUETE(S) A STABLE: $PACKAGE_NAME"
echo "=============================================================================="

DEPLOY_FLAG="--deploy"
if [ "$LOCAL_ONLY" = true ]; then
    DEPLOY_FLAG=""
fi

# 1. Compile & deploy Debian package(s) to stable branch
if [ -f "$PKG_DIR/package-and-deploy.sh" ]; then
    echo "📦 Compilando paquetes Debian para la rama stable..."
    "$PKG_DIR/package-and-deploy.sh" "$PACKAGE_NAME" --branch stable $DEPLOY_FLAG
fi

# 2. Compile & deploy Arch package(s) to stable branch
if [ -f "$PKG_DIR/arch/package-and-deploy.sh" ]; then
    echo "🏛️ Compilando paquetes Arch para la rama stable..."
    (cd "$PKG_DIR/arch" && ./package-and-deploy.sh "$PACKAGE_NAME" --branch stable $DEPLOY_FLAG)
fi

echo "=============================================================================="
echo "🎉 ¡Promoción a la rama stable completada con éxito!"
echo "=============================================================================="
