#!/bin/bash
# ==============================================================================
# Pulsar OS - Gnome macOS Remap Wayland Asset Preparer
# ==============================================================================
# Clona el repositorio oficial de gnome-macos-remap-wayland y prepara
# sus archivos dentro del staging del paquete.
# ==============================================================================
set -e

STAGE_DIR="$(realpath -m "$1")"
DEST_DIR="$STAGE_DIR/usr/share/gnome-macos-remap-wayland"
mkdir -p "$DEST_DIR"

echo "📥 Descargando archivos de gnome-macos-remap-wayland desde GitHub..."
TEMP_BUILD="/tmp/gnome-macos-remap-wayland-build"
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Clone the repository
git clone --depth=1 "https://github.com/Inled-Pulsar-OS/gnome-macos-remap-wayland" "$TEMP_BUILD"

# Copy files to destination
cp -r "$TEMP_BUILD"/* "$DEST_DIR/"

# Patch install.sh to support offline installation and Wayland fallback
python3 -c '
import os
path = "'"$DEST_DIR"'/install.sh"
if os.path.exists(path):
    with open(path, "r") as f:
        content = f.read()
    
    old_block = """# Detect compositor type (X11 or Wayland)
if [ "${XDG_SESSION_TYPE}" == "x11" ]; then
  echo "INFO: Detected X11 compositor."
  ARCHIVE_NAME="xremap-linux-${ARCH}-x11.zip"
elif [ "${XDG_SESSION_TYPE}" == "wayland" ]; then
  echo "INFO: Detected Wayland compositor."
  ARCHIVE_NAME="xremap-linux-${ARCH}-gnome.zip"
else
  echo "ERROR: Unsupported compositor."
  exit 1
fi

# Always download latest xremap release from GitHub
wget https://github.com/xremap/xremap/releases/latest/download/$ARCHIVE_NAME"""

    new_block = """# Detect compositor type (X11 or Wayland)
SESSION_TYPE=$(echo "$XDG_SESSION_TYPE" | tr "[:upper:]" "[:lower:]")
if [ "$SESSION_TYPE" != "x11" ] && [ "$SESSION_TYPE" != "wayland" ]; then
  SESSION_TYPE="wayland"
fi

if [ "${SESSION_TYPE}" == "x11" ]; then
  echo "INFO: Detected X11 compositor."
  ARCHIVE_NAME="xremap-linux-${ARCH}-x11.zip"
else
  echo "INFO: Detected Wayland compositor."
  ARCHIVE_NAME="xremap-linux-${ARCH}-gnome.zip"
fi

# Try to use local pre-downloaded archive first, fallback to wget
if [ -f "$BASE_DIR/binaries/$ARCHIVE_NAME" ]; then
  echo "INFO: Using local pre-downloaded archive: $ARCHIVE_NAME"
  cp "$BASE_DIR/binaries/$ARCHIVE_NAME" .
else
  echo "INFO: Downloading $ARCHIVE_NAME from GitHub..."
  wget https://github.com/xremap/xremap/releases/latest/download/$ARCHIVE_NAME
fi"""

    if old_block in content:
        content = content.replace(old_block, new_block)
        with open(path, "w") as f:
            f.write(content)
        print("✅ install.sh successfully patched for offline mode.")
    else:
        print("⚠️ Warning: Could not find exact block in install.sh to patch.")
'

# Remove the .git folder from destination
rm -rf "$DEST_DIR/.git"

# Pre-download xremap binaries for offline installation support
mkdir -p "$DEST_DIR/binaries"
echo "📥 Pre-descargando binarios de Xremap para soporte offline..."
wget -q "https://github.com/xremap/xremap/releases/latest/download/xremap-linux-x86_64-gnome.zip" -O "$DEST_DIR/binaries/xremap-linux-x86_64-gnome.zip" || echo "⚠️ Warning: Failed to pre-download x86_64-gnome"
wget -q "https://github.com/xremap/xremap/releases/latest/download/xremap-linux-x86_64-x11.zip" -O "$DEST_DIR/binaries/xremap-linux-x86_64-x11.zip" || echo "⚠️ Warning: Failed to pre-download x86_64-x11"
wget -q "https://github.com/xremap/xremap/releases/latest/download/xremap-linux-aarch64-gnome.zip" -O "$DEST_DIR/binaries/xremap-linux-aarch64-gnome.zip" || echo "⚠️ Warning: Failed to pre-download aarch64-gnome"
wget -q "https://github.com/xremap/xremap/releases/latest/download/xremap-linux-aarch64-x11.zip" -O "$DEST_DIR/binaries/xremap-linux-aarch64-x11.zip" || echo "⚠️ Warning: Failed to pre-download aarch64-x11"

# Clean up temp
rm -rf "$TEMP_BUILD"

echo "✅ Copiado con éxito."
