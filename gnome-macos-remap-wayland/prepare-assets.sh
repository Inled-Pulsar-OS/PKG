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

# Patch install.sh and config.yml so they never clobber the PulsarOS dconf.
# PulsarOS defines the macOS Super<->Ctrl swap (XKB xkb-options), the spotlight
# on <Ctrl>space and all GNOME/Mutter keybindings in the system dconf DB
# (/etc/dconf/db/local). Running the upstream gsettings block here would reset
# xkb-options (killing the swap) and bind <Primary>space to Show Applications
# (stealing the spotlight), so we neutralize it. The modmap section is removed
# because a modmap on top of the XKB swap would double-swap.
python3 -c '
import os
install = "'"$DEST_DIR"'/install.sh"
if os.path.exists(install):
    with open(install, "r") as f:
        content = f.read()
    start = content.find("# Tweak gsettings")
    end = content.find("# Restart is required")
    if start != -1 and end != -1 and end > start:
        replacement = """# PulsarOS: GNOME/Mutter macOS keybindings (incl. the XKB Super<->Ctrl swap,
# spotlight on <Ctrl>space, overlay key, screenshots and terminal shortcuts)
# are provided by the PulsarOS system dconf DB (/etc/dconf/db/local).
# The upstream gsettings block is intentionally disabled: running it here would
# reset xkb-options (killing the swap) and steal <Ctrl>space from the spotlight.
"""
        content = content[:start] + replacement + content[end:]
        with open(install, "w") as f:
            f.write(content)
        print("✅ install.sh gsettings block neutralized (PulsarOS dconf owns keybindings).")
    else:
        print("⚠️ Warning: gsettings block markers not found in install.sh.")

config = "'"$DEST_DIR"'/config.yml"
if os.path.exists(config):
    with open(config, "r") as f:
        content = f.read()
    start = content.find("modmap:")
    end = content.find("RightMeta: RightCtrl")
    if start != -1 and end != -1 and end > start:
        end += len("RightMeta: RightCtrl")
        replacement = """# PulsarOS: the Super<->Ctrl swap is done at the XKB level by the system dconf
# (xkb-options ctrl:swap_lwin_lctl / ctrl:swap_rwin_rctl). A modmap here would
# double-swap the keys, so it is intentionally removed.
"""
        content = content[:start] + replacement + content[end:]
        
        # Translate keymap rules for XKB swap compatibility
        content = content.replace("Ctrl-C: Shift-Super-C", "Super-C: Shift-Super-C")
        content = content.replace("Ctrl-V: Shift-Super-V", "Super-V: Shift-Super-V")
        content = content.replace("Ctrl-T: Shift-Super-T", "Super-T: Shift-Super-T")
        content = content.replace("Ctrl-N: Shift-Super-N", "Super-N: Shift-Super-N")
        content = content.replace("Ctrl-W: Shift-Super-W", "Super-W: Shift-Super-W")
        content = content.replace("Ctrl-Q: Shift-Super-Q", "Super-Q: Shift-Super-Q")
        content = content.replace("Ctrl-F: Shift-Super-F", "Super-F: Shift-Super-F")

        content = content.replace("Super-C: Ctrl-C", "Ctrl-C: Super-C")

        content = content.replace("Ctrl-Left: Home", "Super-Left: Home")
        content = content.replace("Ctrl-Right: End", "Super-Right: End")
        content = content.replace("Shift-Ctrl-Left: Shift-Home", "Shift-Super-Left: Shift-Home")
        content = content.replace("Shift-Ctrl-Right: Shift-End", "Shift-Super-Right: Shift-End")

        content = content.replace("Alt-Backspace: Ctrl-Backspace", "Alt-Backspace: Super-Backspace")
        content = content.replace("Ctrl-K: Ctrl-D", "Ctrl-K: Super-D")

        content = content.replace("Ctrl-Up: Alt-Up", "Super-Up: Alt-Up")
        content = content.replace("Ctrl-Down: Enter", "Super-Down: Enter")
        content = content.replace("Ctrl-Backspace: Delete", "Super-Backspace: Delete")
        content = content.replace("Shift-Ctrl-Dot: Ctrl-H", "Shift-Super-Dot: Super-H")
        content = content.replace("Shift-Ctrl-G: Ctrl-L", "Shift-Super-G: Super-L")

        content = content.replace("Ctrl-Shift-C # Copy text", "Super-Shift-C # Copy text")
        content = content.replace("Ctrl-Shift-V # Paste text", "Super-Shift-V # Paste text")
        content = content.replace("Ctrl-Shift-N # New window", "Super-Shift-N # New window")
        content = content.replace("Ctrl-Shift-Q # Close window", "Super-Shift-Q # Close window")
        content = content.replace("Ctrl-Shift-T # New tab", "Super-Shift-T # New tab")
        content = content.replace("Ctrl-Shift-W # Close tab", "Super-Shift-W # Close tab")
        content = content.replace("Shift-Ctrl-F # Find", "Shift-Super-F # Find")

        content = content.replace("      Ctrl-C: Super-Shift-C", "      Super-C: Super-Shift-C")
        content = content.replace("      Ctrl-V: Super-Shift-V", "      Super-V: Super-Shift-V")
        content = content.replace("      Ctrl-N: Super-Shift-N", "      Super-N: Super-Shift-N")
        content = content.replace("      Ctrl-Q: Super-Shift-Q", "      Super-Q: Super-Shift-Q")
        content = content.replace("      Ctrl-T: Super-Shift-T", "      Super-T: Super-Shift-T")
        content = content.replace("      Ctrl-W: Super-Shift-W", "      Super-W: Super-Shift-W")
        content = content.replace("      Ctrl-F: Shift-Super-F", "      Super-F: Shift-Super-F")

        # Comment out/remove the nano section
        nano_start = content.find("  - name: Terminal and Console - make Ctrl work in `nano` editor")
        next_section = content.find("  - name: Console and Ptyxis shortcuts")
        if nano_start != -1 and next_section != -1 and next_section > nano_start:
            content = content[:nano_start] + content[next_section:]

        with open(config, "w") as f:
            f.write(content)
        print("✅ config.yml modmap removed & keymap translated (XKB swap compatibility).")
    else:
        print("⚠️ Warning: modmap section markers not found in config.yml.")
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
