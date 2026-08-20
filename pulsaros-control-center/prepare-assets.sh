#!/bin/bash
# ==============================================================================
# Pulsar OS - pulsaros-control-center prepare-assets.sh (Debian build)
# ==============================================================================
# Clones upstream gnome-control-center at the tag matching the Debian/Trixie
# version, applies the Pulsar OS overlay (same as the Arch PKGBUILD), compiles
# the binary with Meson, and installs the result into the package staging tree.
#
# Run by package-and-deploy.sh as:
#   bash prepare-assets.sh <STAGE_DIR>
#
# Requirements on the build host (or in the chroot where this runs):
#   meson, ninja-build, git, gettext, blueprint-compiler,
#   libgtk-4-dev, libadwaita-1-dev, libgnome-desktop-4-dev,
#   libcolord-gtk4-dev, libgoa-1.0-dev, libgtop2-dev,
#   libpwquality-dev, libnm-dev, libsecret-1-dev, libpolkit-gobject-1-dev,
#   libmalcontent-dev, libaccountsservice-dev, libglib2.0-dev,
#   libgsound-dev, libjson-glib-dev, libsoup-3.0-dev, libwacom-dev,
#   libibus-1.0-dev, docbook-xsl xsltproc libkrb5-dev
# ==============================================================================

set -e

STAGE_DIR="$(realpath -m "$1")"
SCRIPT_DIR="$(dirname "$(realpath "$0")")"

# ==============================================================================
# 0. Detect the gnome-control-center version matching the target Debian suite
# ==============================================================================
# Arch PKGBUILD tracks rolling GNOME (currently 50+) — Debian ships older stable
# versions. We must clone the tag that matches what's in the target chroot.
# Priority:
#   1. dpkg database inside STAGE_DIR (if populated — i.e. we're running inside the
#      build chroot after the base was debootstrapped)
#   2. dpkg database on the build host itself
#   3. DEBIAN_VERSION env var → hardcoded suite→version map
#   4. Arch PKGBUILD pkgver (last resort, unlikely to be right for Debian)

GNOME_CC_VER=""

# 1. dpkg in staging / chroot
for status_file in \
    "$STAGE_DIR/var/lib/dpkg/status" \
    "/var/lib/dpkg/status"
do
    if [ -f "$status_file" ]; then
        ver_str=$(grep -A 10 "^Package: gnome-control-center$" "$status_file" \
                  | grep "^Version:" | head -n1 | awk '{print $2}' || true)
        if [ -n "$ver_str" ]; then
            GNOME_CC_VER=$(echo "$ver_str" | cut -d'.' -f1 | cut -d'-' -f1)
            echo "🔍 [ES] Versión de gnome-control-center detectada en dpkg ($status_file): $ver_str → compilando $GNOME_CC_VER.x"
            echo "🔍 [EN] gnome-control-center version detected in dpkg ($status_file): $ver_str → building $GNOME_CC_VER.x"
            break
        fi
    fi
done

# 2. Suite map fallback
if [ -z "$GNOME_CC_VER" ]; then
    SUITE="${DEBIAN_VERSION:-trixie}"
    case "$SUITE" in
        bookworm)  GNOME_CC_VER="43" ;;
        trixie)    GNOME_CC_VER="48" ;;
        forky)     GNOME_CC_VER="48" ;;
        sid|unstable) GNOME_CC_VER="48" ;;
        *)         GNOME_CC_VER="48" ;;
    esac
    echo "🔍 [ES] Suite Debian '$SUITE' → usando versión $GNOME_CC_VER.x"
    echo "🔍 [EN] Debian suite '$SUITE' → using version $GNOME_CC_VER.x"
fi

# Now resolve the full upstream tag (e.g. "48.0", "48.1", …).
# Query the GNOME GitLab tags API to find the latest tag matching our major.
FULL_TAG=""
if command -v curl >/dev/null 2>&1; then
    echo "🌐 [ES] Consultando tags de GNOME GitLab para la versión $GNOME_CC_VER..."
    echo "🌐 [EN] Querying GNOME GitLab tags for version $GNOME_CC_VER..."
    # GitLab REST API — paginated, first page is enough for recent tags
    FULL_TAG=$(curl --max-time 10 -sf \
        "https://gitlab.gnome.org/api/v4/projects/GNOME%2Fgnome-control-center/repository/tags?per_page=100" \
        | grep -o "\"name\":\"${GNOME_CC_VER}[^\"]*\"" \
        | head -n1 \
        | sed 's/"name":"//;s/"//' || true)
fi
# Fallback: use just the major version as the tag (e.g. "48" is a valid git tag upstream)
if [ -z "$FULL_TAG" ]; then
    FULL_TAG="$GNOME_CC_VER"
fi

echo "🏷️ [ES] Tag de compilación: $FULL_TAG"
echo "🏷️ [EN] Build tag: $FULL_TAG"

# ==============================================================================
# 1. Install build dependencies (only if running as root, e.g. inside a chroot)
# ==============================================================================
if [ "$(id -u)" -eq 0 ]; then
    echo "📦 [ES] Instalando dependencias de compilación..."
    echo "📦 [EN] Installing build dependencies..."
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        meson \
        ninja-build \
        git \
        gettext \
        blueprint-compiler \
        libgtk-4-dev \
        libadwaita-1-dev \
        libgnome-desktop-4-dev \
        libcolord-gtk4-dev \
        libgoa-1.0-dev \
        libgtop2-dev \
        libpwquality-dev \
        libnm-dev \
        libsecret-1-dev \
        libpolkit-gobject-1-dev \
        libmalcontent-dev \
        libaccountsservice-dev \
        libglib2.0-dev \
        libgsound-dev \
        libjson-glib-dev \
        libsoup-3.0-dev \
        libwacom-dev \
        libibus-1.0-dev \
        libkrb5-dev \
        libgnutls28-dev \
        libxi-dev \
        libx11-dev \
        docbook-xsl \
        xsltproc \
        modemmanager-dev \
        libmm-glib-dev \
        udisks2 \
        2>/dev/null || true
fi

# ==============================================================================
# 2. Clone upstream gnome-control-center at the target tag
# ==============================================================================
BUILD_ROOT="/tmp/pulsaros-gcc-build"
SRC_DIR="$BUILD_ROOT/gnome-control-center"
rm -rf "$BUILD_ROOT"
mkdir -p "$BUILD_ROOT"

GCC_TAG="${FULL_TAG}"
# Upstream tags use the format "XX.Y" without prefix — try first as-is
echo "📥 [ES] Clonando gnome-control-center $GCC_TAG desde GNOME GitLab..."
echo "📥 [EN] Cloning gnome-control-center $GCC_TAG from GNOME GitLab..."
git -c http.version=HTTP/1.1 -c http.postBuffer=524288000 \
    clone --depth=1 --branch "$GCC_TAG" \
    "https://gitlab.gnome.org/GNOME/gnome-control-center.git" \
    "$SRC_DIR" 2>/dev/null || {
    # Fallback: clone without --branch and checkout the tag
    echo "⚠️ [ES] Fallback: clonando sin tag específico..."
    echo "⚠️ [EN] Fallback: cloning without specific tag..."
    git -c http.version=HTTP/1.1 clone --depth=1 \
        "https://gitlab.gnome.org/GNOME/gnome-control-center.git" \
        "$SRC_DIR"
}

# Clone submodules (libgnome-volume-control, libgxdp)
cd "$SRC_DIR"
git submodule update --init --depth=1 2>/dev/null || true

# ==============================================================================
# 3. Apply Pulsar OS overlay (same files used in the Arch PKGBUILD)
# ==============================================================================
OVERLAY_DIR="$SCRIPT_DIR/../arch/pkgbuilds/gnome-control-center/overlay"
PATCH_FILE="$SCRIPT_DIR/../arch/pkgbuilds/gnome-control-center/pulsar-macos-style.patch"

if [ -d "$OVERLAY_DIR" ]; then
    echo "🎨 [ES] Aplicando overlay de Pulsar OS (archivos source)..."
    echo "🎨 [EN] Applying Pulsar OS overlay (source files)..."
    cp -rf "$OVERLAY_DIR/." "$SRC_DIR/"
else
    echo "⚠️ [ES] Overlay no encontrado, intentando aplicar parche..."
    echo "⚠️ [EN] Overlay not found, trying to apply patch..."
    if [ -f "$PATCH_FILE" ]; then
        patch -Np1 -i "$PATCH_FILE" -d "$SRC_DIR" || true
    fi
fi

# Merge Spanish translations
PO_DIR="$OVERLAY_DIR/po"
if [ -f "$PO_DIR/pulsar-es.po" ] && [ -f "$SRC_DIR/po/es.po" ]; then
    echo "🌐 [ES] Fusionando traducciones al español..."
    msgcat --use-first "$PO_DIR/pulsar-es.po" "$SRC_DIR/po/es.po" \
        > "$SRC_DIR/po/es.po.merged" && mv "$SRC_DIR/po/es.po.merged" "$SRC_DIR/po/es.po" || true
fi

# ==============================================================================
# 4. Meson build & install into STAGE_DIR
# ==============================================================================
BUILD_DIR="$BUILD_ROOT/build"
mkdir -p "$BUILD_DIR"

echo "🔨 [ES] Configurando con Meson..."
echo "🔨 [EN] Configuring with Meson..."

# Locate the distributor logo (deployed by pulsaros-branding)
DISTRIBUTOR_LOGO="/usr/share/pixmaps/pulsaros-logo-text.svg"
if [ ! -f "$DISTRIBUTOR_LOGO" ]; then
    DISTRIBUTOR_LOGO="/usr/share/pixmaps/gnome-logo-text.svg"
fi

meson setup \
    --prefix=/usr \
    --buildtype=release \
    -D documentation=false \
    -D location-services=enabled \
    -D malcontent=true \
    -D distributor_logo="$DISTRIBUTOR_LOGO" \
    -D dark_mode_distributor_logo="$DISTRIBUTOR_LOGO" \
    "$BUILD_DIR" \
    "$SRC_DIR"

echo "⚙️ [ES] Compilando gnome-control-center..."
echo "⚙️ [EN] Building gnome-control-center..."
meson compile -C "$BUILD_DIR" -j "$(nproc)"

echo "📦 [ES] Instalando en staging: $STAGE_DIR"
echo "📦 [EN] Installing into staging: $STAGE_DIR"
DESTDIR="$STAGE_DIR" meson install -C "$BUILD_DIR"

# ==============================================================================
# 5. Install Pulsar OS panel icons (squircle macOS-style)
# ==============================================================================
ICONS_DIR="$OVERLAY_DIR/icons"
if [ -d "$ICONS_DIR" ]; then
    echo "🖼️ [ES] Instalando iconos de paneles de Pulsar OS..."
    echo "🖼️ [EN] Installing Pulsar OS panel icons..."
    mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps"
    cp -rf "$ICONS_DIR/"* "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/"
fi

# ==============================================================================
# 6. Strip keybindings into a separate virtual package
#    (the DEBIAN control already Provides: gnome-control-center, so we keep
#     keybindings here too — no split package needed for Debian)
# ==============================================================================
# Nothing to strip — we ship everything in one .deb for Debian simplicity.

# ==============================================================================
# 7. Cleanup build artifacts
# ==============================================================================
rm -rf "$BUILD_ROOT"
echo "✅ [ES] pulsaros-control-center preparado con éxito."
echo "✅ [EN] pulsaros-control-center prepared successfully."
