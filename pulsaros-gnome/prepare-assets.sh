#!/bin/bash
# ==============================================================================
# Pulsar OS - Gnome Shell Extensions Asset Preparer
# ==============================================================================
# Instala localmente las extensiones GNOME Shell desde los submódulos Git de
# Inled-Pulsar-OS ubicados en /PKG, sin descargas externas desde EGO.
# ==============================================================================

set -e

STAGE_DIR="${1:-/tmp/pulsaros-stage}"
STAGE_DIR="$(realpath -m "$STAGE_DIR")" 2>/dev/null || STAGE_DIR="/tmp/pulsaros-stage"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -n "$PKG_ROOT_OVERRIDE" ] && [ -d "$PKG_ROOT_OVERRIDE" ]; then
    PKG_ROOT="$PKG_ROOT_OVERRIDE"
elif [ -d "$SCRIPT_DIR/../Pulsar-CC-27-Gnome" ]; then
    PKG_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
elif [ -d "$SCRIPT_DIR/../../../../Pulsar-CC-27-Gnome" ]; then
    PKG_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
elif [ -d "/home/jaime/Documentos/pulsar/PKG/Pulsar-CC-27-Gnome" ]; then
    PKG_ROOT="/home/jaime/Documentos/pulsar/PKG"
else
    echo "❌ ERROR: No se encontró la raíz de PKG con los submódulos Git!" >&2
    exit 1
fi
echo "ℹ️ Usando PKG_ROOT: $PKG_ROOT"

EXT_DIR="$STAGE_DIR/usr/share/gnome-shell/extensions"
SCHEMA_DIR="$STAGE_DIR/usr/share/glib-2.0/schemas"
mkdir -p "$EXT_DIR" "$SCHEMA_DIR"

echo "=========================================================="
echo "🚀 [Pulsar OS] Preparando extensiones desde submódulos..."
echo "=========================================================="

# 1. Pulsar Control Center 27
echo "📦 Instalando Pulsar Control Center 27..."
CC_SRC="$PKG_ROOT/Pulsar-CC-27-Gnome/macos-control-centre-lg@pulsar.local"
if [ -d "$CC_SRC" ]; then
    rm -rf "$EXT_DIR/macos-control-centre-lg@pulsar.local"
    cp -r "$CC_SRC" "$EXT_DIR/macos-control-centre-lg@pulsar.local"
fi

# 2. Pulsar Notification Center
echo "📦 Instalando Pulsar Notification Center..."
NC_SRC="$PKG_ROOT/Pulsar-Notification-Center/macos-notification-center@pulsar.local"
if [ -d "$NC_SRC" ]; then
    rm -rf "$EXT_DIR/macos-notification-center@pulsar.local"
    cp -r "$NC_SRC" "$EXT_DIR/macos-notification-center@pulsar.local"
fi

# 3. Pulsar Liquid Glass (Agnelo / Inled)
echo "📦 Instalando Pulsar Liquid Glass..."
LG_SRC="$PKG_ROOT/Pulsar-Liquid-Glass"
if [ -d "$LG_SRC" ]; then
    LG_DEST="$EXT_DIR/liquid-glass@thinkingcoding1231.gmail.com"
    rm -rf "$LG_DEST"
    mkdir -p "$LG_DEST"
    cp -r "$LG_SRC"/* "$LG_DEST/"
    rm -rf "$LG_DEST/.git" "$LG_DEST/.github"
fi

# 4. Pulsar Dock (Dash to Dock forked by Inled/Agnelo)
echo "📦 Instalando Pulsar Dock..."
DOCK_SRC="$PKG_ROOT/Pulsar-Dock"
if [ -d "$DOCK_SRC" ]; then
    DOCK_DEST="$EXT_DIR/pulsar-dock@inled.es"
    rm -rf "$DOCK_DEST"
    mkdir -p "$DOCK_DEST"
    cp -r "$DOCK_SRC"/* "$DOCK_DEST/"
    rm -rf "$DOCK_DEST/.git" "$DOCK_DEST/.github"
fi

# 5. AppIndicator Support
echo "📦 Instalando AppIndicator Support..."
APPIND_SRC="$PKG_ROOT/gnome-shell-extension-appindicator"
if [ -d "$APPIND_SRC" ]; then
    APPIND_DEST="$EXT_DIR/appindicatorsupport@rgcjonas.gmail.com"
    rm -rf "$APPIND_DEST"
    mkdir -p "$APPIND_DEST"
    cp -r "$APPIND_SRC"/* "$APPIND_DEST/"
    rm -rf "$APPIND_DEST/.git" "$APPIND_DEST/.github"
fi

# 6. Compiz Alike Magic Lamp Effect
echo "📦 Instalando Compiz Magic Lamp..."
COMPIZ_SRC="$PKG_ROOT/compiz-alike-magic-lamp-effect"
if [ -d "$COMPIZ_SRC" ]; then
    COMPIZ_DEST="$EXT_DIR/compiz-alike-magic-lamp-effect@hermes83.github.com"
    rm -rf "$COMPIZ_DEST"
    mkdir -p "$COMPIZ_DEST"
    cp -r "$COMPIZ_SRC"/* "$COMPIZ_DEST/"
    rm -rf "$COMPIZ_DEST/.git" "$COMPIZ_DEST/.github"
fi

# 7. Pulsar UI Tweaks (Just Perfection + Notification Position + GNOME UI Improvements)
echo "📦 Instalando Pulsar UI Tweaks..."
TWEAKS_SRC="$PKG_ROOT/pulsaros-ui-tweaks"
if [ -d "$TWEAKS_SRC" ]; then
    TWEAKS_DEST="$EXT_DIR/pulsaros-ui-tweaks@inled.es"
    rm -rf "$TWEAKS_DEST"
    mkdir -p "$TWEAKS_DEST"
    cp -r "$TWEAKS_SRC"/* "$TWEAKS_DEST/"
    rm -rf "$TWEAKS_DEST/.git"
fi

# 8. Blur My Shell
echo "📦 Instalando Blur My Shell..."
BMS_SRC="$PKG_ROOT/blur-my-shell"
if [ -d "$BMS_SRC" ]; then
    BMS_DEST="$EXT_DIR/blur-my-shell@aunetx"
    rm -rf "$BMS_DEST"
    mkdir -p "$BMS_DEST"
    make -C "$BMS_SRC" build >/dev/null 2>&1 || true
    if [ -f "$BMS_SRC/build/blur-my-shell@aunetx.shell-extension.zip" ]; then
        unzip -q -o "$BMS_SRC/build/blur-my-shell@aunetx.shell-extension.zip" -d "$BMS_DEST/"
    else
        # Fallback direct copy
        cp -r "$BMS_SRC/src"/* "$BMS_DEST/" 2>/dev/null || true
        cp "$BMS_SRC/metadata.json" "$BMS_DEST/" 2>/dev/null || true
    fi
fi

# 9. No Overview
echo "📦 Instalando No Overview..."
NO_SRC="$PKG_ROOT/no-overview"
if [ -d "$NO_SRC" ]; then
    NO_DEST="$EXT_DIR/no-overview@fthx"
    rm -rf "$NO_DEST"
    mkdir -p "$NO_DEST"
    cp "$NO_SRC/extension.js" "$NO_SRC/metadata.json" "$NO_DEST/" 2>/dev/null || true
fi

# 10. Wiggle
echo "📦 Instalando Wiggle..."
WIGGLE_SRC="$PKG_ROOT/wiggle"
if [ -d "$WIGGLE_SRC" ]; then
    WIGGLE_DEST="$EXT_DIR/wiggle@mechtifs"
    rm -rf "$WIGGLE_DEST"
    mkdir -p "$WIGGLE_DEST"
    cp -r "$WIGGLE_SRC"/* "$WIGGLE_DEST/"
    rm -rf "$WIGGLE_DEST/.git" "$WIGGLE_DEST/.github"
fi

# 11. Pulsar Idle Guard
echo "📦 Instalando Pulsar Idle Guard..."
IDLE_SRC="$PKG_ROOT/pulsaros-gnome/usr/share/gnome-shell/extensions/pulsaros-idle-guard@inled.es"
if [ ! -d "$IDLE_SRC" ]; then
    IDLE_SRC="$SCRIPT_DIR/usr/share/gnome-shell/extensions/pulsaros-idle-guard@inled.es"
fi
if [ -d "$IDLE_SRC" ]; then
    IDLE_DEST="$EXT_DIR/pulsaros-idle-guard@inled.es"
    rm -rf "$IDLE_DEST"
    cp -r "$IDLE_SRC" "$IDLE_DEST"
fi

# 12. User Theme (GNOME Shell User Themes)
echo "📦 Instalando User Theme..."
UT_SRC="$PKG_ROOT/pulsaros-gnome/usr/share/gnome-shell/extensions/user-theme@gnome-shell-extensions.gcampax.github.com"
if [ ! -d "$UT_SRC" ]; then
    UT_SRC="$SCRIPT_DIR/usr/share/gnome-shell/extensions/user-theme@gnome-shell-extensions.gcampax.github.com"
fi
if [ -d "$UT_SRC" ]; then
    UT_DEST="$EXT_DIR/user-theme@gnome-shell-extensions.gcampax.github.com"
    rm -rf "$UT_DEST"
    cp -r "$UT_SRC" "$UT_DEST"
fi

# ------------------------------------------------------------------------------
# PURGAR EXTENSIONES OBSOLETAS Y DESCONTINUADAS DEL DESTINO
# ------------------------------------------------------------------------------
echo "🧹 Purgando extensiones obsoletas de la imagen..."
rm -rf "$EXT_DIR/ding@rastersoft.com" \
       "$EXT_DIR/gsconnect@andyholmes.github.io" \
       "$EXT_DIR/macos-control-centre@pulsar.local" \
       "$EXT_DIR/just-perfection-desktop@just-perfection" \
       "$EXT_DIR/notification-position@drugo.dev" \
       "$EXT_DIR/gnome-ui-tune@itstime.tech" \
       "$EXT_DIR/pulsar-island@inled.es" \
       "$EXT_DIR/pulsaros-control-center-button@inled.es" \
       "$EXT_DIR/gaze@gundulabs.com" \
       "$EXT_DIR"/apps-menu@* \
       "$EXT_DIR"/places-menu@* \
       "$EXT_DIR"/window-list@* \
       "$STAGE_DIR/usr/share/dbus-1/services/org.gnome.Shell.Extensions.GSConnect.service" \
       "$STAGE_DIR/usr/share/applications/org.gnome.Shell.Extensions.GSConnect"* 2>/dev/null || true

# ------------------------------------------------------------------------------
# COMPILAR ESQUEMAS GSETTINGS
# ------------------------------------------------------------------------------
echo "⚙️ Recopilando y compilando esquemas GSettings..."
find "$EXT_DIR" -name "*.gschema.xml" -exec cp {} "$SCHEMA_DIR/" \; 2>/dev/null || true
find "$EXT_DIR" -name schemas -type d 2>/dev/null | while read -r sdir; do
    glib-compile-schemas "$sdir" 2>/dev/null || true
done
glib-compile-schemas "$SCHEMA_DIR" 2>/dev/null || true

# ------------------------------------------------------------------------------
# PERMISOS
# ------------------------------------------------------------------------------
find "$EXT_DIR" -type d -exec chmod 755 {} \; 2>/dev/null || true
find "$EXT_DIR" -type f -exec chmod 644 {} \; 2>/dev/null || true
find "$SCHEMA_DIR" -type f -exec chmod 644 {} \; 2>/dev/null || true

[ -f "$STAGE_DIR/usr/bin/pulsaros-reconcile-extensions" ] && chmod 755 "$STAGE_DIR/usr/bin/pulsaros-reconcile-extensions"
[ -f "$STAGE_DIR/usr/bin/pulsaros-hide-overview" ] && chmod 755 "$STAGE_DIR/usr/bin/pulsaros-hide-overview"
[ -f "$STAGE_DIR/usr/bin/pulsar-sync-accent" ] && chmod 755 "$STAGE_DIR/usr/bin/pulsar-sync-accent"

echo "✅ [Pulsar OS] Todas las extensiones procesadas con éxito desde fuentes oficiales."
