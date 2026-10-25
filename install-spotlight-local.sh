#!/bin/bash
# ==============================================================================
# Pulsar OS - Spotlight Local Installer
# ==============================================================================
# Installs the Spotlight search package locally for testing:
#   1. Rust binary to ~/.local/bin (cargo build --release)
#   2. CLI scripts to ~/.local/bin, styles and icons to ~/.local/share/pulsaros-spotlight
#   3. GNOME Shell extension to ~/.local/share/gnome-shell/extensions
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/pulsaros-spotlight-launcher"
EXT_UUID="pulsaros-spotlight-launcher@inled.es"

echo "==> Installing PulsarOS Spotlight locally..."

# 1. Native Rust binary
echo "==> Building and installing Rust binary (release)..."
(cd "$PKG_DIR" && cargo build --release --locked)
mkdir -p "$HOME/.local/bin"
install -m 755 "$PKG_DIR/target/release/pulsaros-spotlight" "$HOME/.local/bin/pulsaros-spotlight"

# 2. Helper CLI scripts
echo "==> Installing CLI scripts to ~/.local/bin..."
install -m 755 "$PKG_DIR/cli/pulsaros-toggle-remap" "$HOME/.local/bin/"
install -m 755 "$PKG_DIR/cli/pulsaros-toggle-launcher" "$HOME/.local/bin/"

# 2b. Styles and icons (the app looks in ~/.local/share/pulsaros-spotlight)
echo "==> Installing styles and icons..."
DATA_DIR="$HOME/.local/share/pulsaros-spotlight"
mkdir -p "$DATA_DIR/icons"
install -m 644 "$PKG_DIR"/data/*.css "$DATA_DIR/"
cp -r "$PKG_DIR/data/icons/." "$DATA_DIR/icons/"

# 3. GNOME Shell extension
echo "==> Installing GNOME Shell extension..."
EXT_DIR="$HOME/.local/share/gnome-shell/extensions/$EXT_UUID"
mkdir -p "$EXT_DIR"
cp -r "$PKG_DIR/gnome-shell-extension/"* "$EXT_DIR/"

# 4. Enable extension
echo "==> Enabling extension..."
gnome-extensions enable "$EXT_UUID" 2>/dev/null || true

echo ""
echo "==> Done!"
echo ""
echo "NOTE (Wayland): Log out and back in to reload GNOME Shell."
echo "Make sure ~/.local/bin is in your PATH."
