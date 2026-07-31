#!/bin/bash
# ==============================================================================
# Pulsar OS - Arch Linux Package Builder and Deployer
# ==============================================================================
# Compiles all PKGBUILD packages into .pkg.tar.zst archives using makepkg
# and optionally deploys them to the Inled Arch Linux repository.
#
# Usage:
#   ./package-and-deploy.sh <package_name | all> [--deploy]
#
# Examples:
#   ./package-and-deploy.sh pulsaros-theme   # Build a single package
#   ./package-and-deploy.sh all              # Build ALL packages
#   ./package-and-deploy.sh all --deploy     # Build and deploy all
# ==============================================================================

set -e

PKG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$PKG_DIR/build"
PKGBUILDS_DIR="$PKG_DIR/pkgbuilds"

PACKAGE_NAME=""
DEPLOY_FLAG=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --deploy|-d)
            DEPLOY_FLAG="--deploy"
            shift
            ;;
        *)
            if [ -z "$PACKAGE_NAME" ]; then
                PACKAGE_NAME="$1"
            else
                echo "❌ Unknown parameter: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

if [ -z "$PACKAGE_NAME" ]; then
    echo "❌ Error: Specify a package name or 'all'."
    echo "Usage: $0 <package_name | all> [--deploy]"
    exit 1
fi

build_single_package() {
    local name="$1"
    local pkgbuild_dir="$PKGBUILDS_DIR/$name"

    echo "=============================================================================="
    echo "📦 BUILDING: $name"
    echo "=============================================================================="

    if [ ! -d "$pkgbuild_dir" ]; then
        echo "❌ Error: PKGBUILD directory not found: $pkgbuild_dir"
        return 1
    fi

    if [ ! -f "$pkgbuild_dir/PKGBUILD" ]; then
        echo "❌ Error: PKGBUILD not found in: $pkgbuild_dir"
        return 1
    fi

    mkdir -p "$BUILD_DIR/packages"

    # Remove stale builds of the same package to avoid 'duplicate target' in pacman -U
    # Eliminar versiones antiguas del mismo paquete para evitar 'duplicate target' en pacman -U
    rm -f "$BUILD_DIR/packages/${name}-"*.pkg.tar.zst

    cd "$pkgbuild_dir"
    PKGDEST="$BUILD_DIR/packages" makepkg -cfd --noconfirm --nosign

    echo "✅ Package built: $name"
    cd "$PKG_DIR"
}

if [ "$PACKAGE_NAME" == "all" ]; then
    echo "🏗️  FULL BUILD: Building all packages..."

    for pkg_dir in "$PKGBUILDS_DIR"/*/; do
        pkg_name=$(basename "$pkg_dir")
        if [ -f "$pkg_dir/PKGBUILD" ]; then
            build_single_package "$pkg_name"
        fi
    done

    echo "=============================================================================="
    echo "🎉 All packages built successfully!"
    echo "=============================================================================="
else
    build_single_package "$PACKAGE_NAME"
fi

if [ "$DEPLOY_FLAG" == "--deploy" ]; then
    echo "🌐 Deployment to Inled Arch repo not yet implemented."
    echo "   Packages are available at: $BUILD_DIR/packages/"
fi
