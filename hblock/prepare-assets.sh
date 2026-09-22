#!/bin/bash
# =============================================================================
# hblock - prepare-assets
#
# Builds the hblock package FROM SOURCE (hectorm/hblock upstream release):
#   1. Downloads the v3.5.1 source tarball from GitHub.
#   2. Verifies its sha256 (pinned) and the script's own sha256 (published by
#      the author in hblock.sha256), so we never ship tampered code.
#   3. Stages the script, the man page and the systemd units into $STAGE_DIR.
# =============================================================================
set -e

STAGE_DIR="$(realpath -m "$1")"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

HB_VERSION="3.5.1"
HB_TARBALL_SHA256="af98a6753e5de1406b63cd1fabf4b3eae84816168c532dae40c83092acb69941"
TARBALL="hblock-${HB_VERSION}.tar.gz"
URL="https://github.com/hectorm/hblock/archive/refs/tags/v${HB_VERSION}.tar.gz"

echo "🚀 Preparing hblock (v$HB_VERSION, built from source) in $STAGE_DIR..."

BUILD_DIR="/tmp/pulsaros-hblock-src-$$"
mkdir -p "$BUILD_DIR"
trap 'rm -rf "$BUILD_DIR"' EXIT

cd "$BUILD_DIR"
echo "⬇️  Downloading $URL ..."
curl -fsSL -o "$TARBALL" "$URL" || { echo "❌ Failed to download hblock source."; exit 1; }

echo "🔒 Verifying tarball sha256 ($HB_TARBALL_SHA256) ..."
echo "$HB_TARBALL_SHA256  $TARBALL" | sha256sum -c - || { echo "❌ sha256 mismatch — aborting for security."; exit 1; }

tar xzf "$TARBALL"
SRC_FOLDER="$BUILD_DIR/hblock-$HB_VERSION"
[ -d "$SRC_FOLDER" ] || { echo "❌ Source folder 'hblock-$HB_VERSION' not found in tarball."; exit 1; }

# Verifica el checksum del script publicado por el autor (hblock.sha256)
echo "🔒 Verifying hblock script checksum ..."
( cd "$SRC_FOLDER" && sha256sum -c hblock.sha256 ) || { echo "❌ hblock script checksum mismatch."; exit 1; }

mkdir -p "$STAGE_DIR/usr/bin" \
         "$STAGE_DIR/usr/lib/systemd/system" \
         "$STAGE_DIR/usr/share/man/man1"

install -Dm755 "$SRC_FOLDER/hblock" "$STAGE_DIR/usr/bin/hblock"
install -Dm644 "$SRC_FOLDER/hblock.1" "$STAGE_DIR/usr/share/man/man1/hblock.1"
gzip -9f "$STAGE_DIR/usr/share/man/man1/hblock.1"
install -Dm644 "$SRC_FOLDER/resources/systemd/hblock.service" "$STAGE_DIR/usr/lib/systemd/system/hblock.service"
install -Dm644 "$SRC_FOLDER/resources/systemd/hblock.timer" "$STAGE_DIR/usr/lib/systemd/system/hblock.timer"

echo "✅ hblock staging complete:"
find "$STAGE_DIR" -type f -printf '   %p\n' | sort