#!/bin/bash
# ==============================================================================
# Pulsar OS - Orbit Global Menu Upstream Synchronizer
# ==============================================================================
# Sincroniza modularmente la extensión con upstream (Unmade760/orbit-global-menu)
# aplicando los parches de Pulsar OS (Logo Apple, Modal de Apagado Directo,
# Liquid Glass, etc.) sin necesidad de reescribir manualmente el código.
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UPSTREAM_REPO="https://github.com/Unmade760/orbit-global-menu.git"
WORK_DIR="/tmp/pulsar-orbit-sync"
DEST_EXT_DIR="$SCRIPT_DIR/usr/share/gnome-shell/extensions/pulsaros-global-menu@inled.es"
PATCH_DIR="$SCRIPT_DIR/patches"

echo "🔄 [Pulsar OS] Sincronizando con upstream de Orbit Global Menu..."

# 1. Preparar directorio de trabajo
rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR"
mkdir -p "$PATCH_DIR"

# 2. Clonar última versión upstream
echo "📥 Clonando upstream ($UPSTREAM_REPO)..."
git clone --depth=1 "$UPSTREAM_REPO" "$WORK_DIR/orbit"

# 3. Aplicar overlay y adaptaciones de Pulsar OS
echo "🛠️ Adaptando extension a Pulsar OS..."
EXT_SRC="$WORK_DIR/orbit/global-menu@unmade.space"

if [ ! -d "$EXT_SRC" ]; then
    echo "❌ Error: Directorio de extensión upstream no encontrado."
    exit 1
fi

# Copiar archivos base
mkdir -p "$DEST_EXT_DIR"
cp -rf "$EXT_SRC"/* "$DEST_EXT_DIR/"

# Adaptar metadata.json para Pulsar OS
if [ -f "$DEST_EXT_DIR/metadata.json" ]; then
    sed -i 's/"uuid": "global-menu@unmade.space"/"uuid": "pulsaros-global-menu@inled.es"/g' "$DEST_EXT_DIR/metadata.json"
    sed -i 's/"name": "Orbit Global Menu"/"name": "Pulsar OS Global Menu"/g' "$DEST_EXT_DIR/metadata.json"
fi

# Aplicar parches específicos si existen en patches/
if [ -d "$PATCH_DIR" ] && [ "$(ls -A "$PATCH_DIR"/*.patch 2>/dev/null)" ]; then
    echo "🧩 Aplicando parches de Pulsar OS..."
    for p in "$PATCH_DIR"/*.patch; do
        echo "   Aplicando $p..."
        patch -d "$DEST_EXT_DIR" -p1 < "$p" || echo "   ⚠️ Aviso: Falló parche $p, revisar compatibilidad."
    done
fi

# Limpieza
rm -rf "$WORK_DIR"
echo "✅ Sincronización de Orbit Global Menu completada con éxito."
