#!/bin/bash
# ==============================================================================
# Pulsar OS - Spotlight Local Installer
# ==============================================================================
# Installs the Spotlight search package locally for testing:
#   1. Python package (editable via pip)
#   2. CLI scripts to ~/.local/bin
#   3. GNOME Shell extension to ~/.local/share/gnome-shell/extensions
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/pulsaros-spotlight-launcher"
EXT_UUID="pulsaros-spotlight-launcher@inled.es"

echo "==> Installing PulsarOS Spotlight locally..."

# 1. Python package (editable)
echo "==> Installing Python package (pip install -e)..."
pip install -e "$PKG_DIR"

# 2. CLI scripts
echo "==> Installing CLI scripts to ~/.local/bin..."
mkdir -p "$HOME/.local/bin"
install -m 755 "$PKG_DIR/cli/pulsaros-spotlight" "$HOME/.local/bin/"
install -m 755 "$PKG_DIR/cli/pulsaros-toggle-remap" "$HOME/.local/bin/"
install -m 755 "$PKG_DIR/cli/pulsaros-toggle-launcher" "$HOME/.local/bin/"

# 2b. Native Rust binary (overrides the Python wrapper)
echo "==> Building and installing Rust binary (release)..."
(cd "$PKG_DIR" && cargo build --release)
install -m 755 "$PKG_DIR/target/release/pulsaros-spotlight" "$HOME/.local/bin/pulsaros-spotlight"

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
